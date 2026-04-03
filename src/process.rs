use std::fs::File;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

use crate::config::{Config, OutputMode, ServerConfig};
use crate::errors;

pub struct ProcessManager {
    processes: Vec<ManagedProcess>,
    global_output: OutputMode,
}

struct ManagedProcess {
    config: ServerConfig,
    child: Option<Child>,
}

impl ProcessManager {
    pub fn new(servers: Vec<ServerConfig>, global_output: OutputMode) -> Self {
        let processes = servers
            .into_iter()
            .map(|config| ManagedProcess {
                config,
                child: None,
            })
            .collect();
        ProcessManager {
            processes,
            global_output,
        }
    }

    pub fn start(&mut self, id: usize) -> Result<(), String> {
        let proc = self
            .processes
            .get_mut(id)
            .ok_or_else(|| format!("Server {} not found", id))?;

        if proc.child.is_some() {
            return Err(format!("'{}' is already running", proc.config.name));
        }

        let mode = proc.config.effective_output(&self.global_output);
        let child = spawn_server(&proc.config, mode)?;
        proc.child = Some(child);
        Ok(())
    }

    pub fn stop(&mut self, id: usize) -> Result<(), String> {
        let proc = self
            .processes
            .get_mut(id)
            .ok_or_else(|| format!("Server {} not found", id))?;

        if let Some(mut child) = proc.child.take() {
            kill_process_tree(child.id());
            // Wait for the process to actually exit (up to ~2 seconds)
            // to avoid port-already-in-use races on restart
            wait_for_exit(&mut child);
        }
        Ok(())
    }

    pub fn restart(&mut self, id: usize) -> Result<(), String> {
        self.stop(id)?;
        self.start(id)
    }

    /// Check if a server is actually running by polling the child process.
    /// Clears stale handles for processes that have exited.
    pub fn is_running(&mut self, id: usize) -> bool {
        let Some(proc) = self.processes.get_mut(id) else {
            return false;
        };

        if let Some(ref mut child) = proc.child {
            match child.try_wait() {
                Ok(Some(_)) => {
                    proc.child = None;
                    false
                }
                Ok(None) => true,
                Err(_) => true,
            }
        } else {
            false
        }
    }

    /// Returns the effective output mode for a server
    pub fn server_output_mode(&self, id: usize) -> Option<&OutputMode> {
        self.processes
            .get(id)
            .map(|p| p.config.effective_output(&self.global_output))
    }

    /// Change a server's output mode. Stops and restarts it if running.
    pub fn set_output_mode(&mut self, id: usize, mode: OutputMode) -> Result<(), String> {
        let was_running = self.is_running(id);
        if was_running {
            self.stop(id)?;
        }

        let proc = self
            .processes
            .get_mut(id)
            .ok_or_else(|| format!("Server {} not found", id))?;
        proc.config.output = Some(mode);

        if was_running {
            self.start(id)?;
        }
        Ok(())
    }

    pub fn start_all(&mut self) {
        let mut failures = Vec::new();
        for id in 0..self.processes.len() {
            if let Err(e) = self.start(id) {
                failures.push(e);
            }
        }
        if !failures.is_empty() {
            errors::show_error("Start All Failed", &failures.join("\n"));
        }
    }

    pub fn stop_all(&mut self) {
        let mut failures = Vec::new();
        for id in 0..self.processes.len() {
            if let Err(e) = self.stop(id) {
                failures.push(e);
            }
        }
        if !failures.is_empty() {
            errors::show_error("Stop All Failed", &failures.join("\n"));
        }
    }

    pub fn restart_all(&mut self) {
        self.stop_all();
        self.start_all();
    }

    pub fn server_count(&self) -> usize {
        self.processes.len()
    }

    pub fn server_name(&self, id: usize) -> Option<&str> {
        self.processes.get(id).map(|p| p.config.name.as_str())
    }

    /// Reload with a new config, preserving running servers whose config hasn't changed.
    /// - Unchanged servers: keep running, transfer child handle
    /// - Changed servers: stop, update config (left stopped)
    /// - New servers: added as stopped
    /// - Removed servers: stopped and removed
    pub fn reload(&mut self, new_servers: Vec<ServerConfig>, new_global_output: OutputMode) {
        let mut new_processes: Vec<ManagedProcess> = Vec::with_capacity(new_servers.len());

        for new_config in new_servers {
            // Find existing server by name
            let existing_idx = self
                .processes
                .iter()
                .position(|p| p.config.name == new_config.name);

            if let Some(idx) = existing_idx {
                let old = &mut self.processes[idx];
                if old.config == new_config {
                    // Config unchanged — transfer the child handle
                    new_processes.push(ManagedProcess {
                        config: new_config,
                        child: old.child.take(),
                    });
                } else {
                    // Config changed — stop the old one
                    if let Some(mut child) = old.child.take() {
                        kill_process_tree(child.id());
                        wait_for_exit(&mut child);
                    }
                    new_processes.push(ManagedProcess {
                        config: new_config,
                        child: None,
                    });
                }
            } else {
                // New server — add as stopped
                new_processes.push(ManagedProcess {
                    config: new_config,
                    child: None,
                });
            }
        }

        // Any remaining old processes not matched by name are removed — stop them
        for old in &mut self.processes {
            if let Some(mut child) = old.child.take() {
                kill_process_tree(child.id());
                wait_for_exit(&mut child);
            }
        }

        self.processes = new_processes;
        self.global_output = new_global_output;
    }
}

// No Drop impl — the user chooses whether to stop servers on quit.
// If they click "No", servers keep running independently.

pub type SharedProcessManager = Arc<Mutex<ProcessManager>>;

pub fn new_shared(servers: Vec<ServerConfig>, global_output: OutputMode) -> SharedProcessManager {
    Arc::new(Mutex::new(ProcessManager::new(servers, global_output)))
}

// Security note: config.cmd is passed to a shell as a command string.
// This is intentional — users need shell features (PATH resolution, pipes, env expansion)
// for commands like "npm run dev" or "cargo run". The config file is local and user-authored,
// so the trust boundary is the filesystem, same as any shell script.
fn spawn_server(config: &ServerConfig, mode: &OutputMode) -> Result<Child, String> {
    match mode {
        OutputMode::Terminal => spawn_terminal(config),
        OutputMode::Logfile => spawn_logfile(config),
        OutputMode::Hidden => spawn_hidden(config),
    }
}

/// Terminal mode: PowerShell window with named title and visible logs
fn spawn_terminal(config: &ServerConfig) -> Result<Child, String> {
    let mut cmd = Command::new("powershell");
    cmd.args([
        "-NoExit",
        "-Command",
        &format!(
            "$Host.UI.RawUI.WindowTitle = '{}'; Set-Location '{}'; {}",
            config.name, config.dir, config.cmd
        ),
    ]);

    for (key, val) in &config.env {
        cmd.env(key, val);
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NEW_PROCESS_GROUP | CREATE_NEW_CONSOLE
        cmd.creation_flags(0x00000200 | 0x00000010);
    }

    cmd.spawn()
        .map_err(|e| format!("Failed to start '{}': {}", config.name, e))
}

/// Logfile mode: hidden process with stdout/stderr redirected to a log file
fn spawn_logfile(config: &ServerConfig) -> Result<Child, String> {
    let log_path = Config::log_path(&config.name);

    // Truncate log on each start so it doesn't grow forever
    let log_file = File::create(&log_path)
        .map_err(|e| format!("Failed to create log file for '{}': {}", config.name, e))?;
    let log_err = log_file
        .try_clone()
        .map_err(|e| format!("Failed to clone log handle for '{}': {}", config.name, e))?;

    let mut cmd = Command::new("cmd");
    cmd.args(["/c", &config.cmd]);
    cmd.current_dir(&config.dir);
    cmd.stdout(Stdio::from(log_file));
    cmd.stderr(Stdio::from(log_err));

    for (key, val) in &config.env {
        cmd.env(key, val);
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW
        cmd.creation_flags(0x00000200 | 0x08000000);
    }

    cmd.spawn()
        .map_err(|e| format!("Failed to start '{}': {}", config.name, e))
}

/// Hidden mode: no window, no output captured
fn spawn_hidden(config: &ServerConfig) -> Result<Child, String> {
    let mut cmd = Command::new("cmd");
    cmd.args(["/c", &config.cmd]);
    cmd.current_dir(&config.dir);
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());

    for (key, val) in &config.env {
        cmd.env(key, val);
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW
        cmd.creation_flags(0x00000200 | 0x08000000);
    }

    cmd.spawn()
        .map_err(|e| format!("Failed to start '{}': {}", config.name, e))
}

/// Kill a process and all its children on Windows using taskkill /T
fn kill_process_tree(pid: u32) {
    let _ = Command::new("taskkill")
        .args(["/F", "/T", "/PID", &pid.to_string()])
        .output();
}

/// Wait for a child process to exit after kill, with a timeout.
/// Polls try_wait() up to ~2 seconds to avoid blocking the UI thread forever.
fn wait_for_exit(child: &mut Child) {
    for _ in 0..20 {
        match child.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(100)),
            Err(_) => return,
        }
    }
}

/// Restart all PowerShell and Windows Terminal processes.
/// Shows a confirmation dialog first since this is destructive.
pub fn restart_terminals() {
    if !errors::confirm(
        "Restart Terminals",
        "This will kill ALL open PowerShell and Windows Terminal windows and open a fresh one.\n\nContinue?",
    ) {
        return;
    }

    let our_pid = std::process::id();

    for proc_name in &["powershell.exe", "pwsh.exe", "WindowsTerminal.exe"] {
        let output = Command::new("tasklist")
            .args([
                "/FI",
                &format!("IMAGENAME eq {}", proc_name),
                "/FO",
                "CSV",
                "/NH",
            ])
            .output();

        if let Ok(output) = output {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                if line.contains(proc_name) {
                    let parts: Vec<&str> = line.split(',').collect();
                    if parts.len() >= 2 {
                        if let Ok(pid) = parts[1].trim_matches('"').parse::<u32>() {
                            if pid != our_pid {
                                let _ = Command::new("taskkill")
                                    .args(["/F", "/PID", &pid.to_string()])
                                    .output();
                            }
                        }
                    }
                }
            }
        }
    }

    let wt_result = Command::new("cmd")
        .args(["/c", "start", "wt"])
        .spawn();

    if wt_result.is_err() {
        let _ = Command::new("cmd")
            .args(["/c", "start", "cmd.exe"])
            .spawn();
    }
}
