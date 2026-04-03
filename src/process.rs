use std::process::{Child, Command};
use std::sync::{Arc, Mutex};

use crate::config::ServerConfig;
use crate::errors;

pub struct ProcessManager {
    processes: Vec<ManagedProcess>,
}

struct ManagedProcess {
    config: ServerConfig,
    child: Option<Child>,
}

impl ProcessManager {
    pub fn new(servers: Vec<ServerConfig>) -> Self {
        let processes = servers
            .into_iter()
            .map(|config| ManagedProcess {
                config,
                child: None,
            })
            .collect();
        ProcessManager { processes }
    }

    pub fn start(&mut self, id: usize) -> Result<(), String> {
        let proc = self
            .processes
            .get_mut(id)
            .ok_or_else(|| format!("Server {} not found", id))?;

        if proc.child.is_some() {
            return Err(format!("'{}' is already running", proc.config.name));
        }

        let child = spawn_server(&proc.config)?;
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
                // Process has exited — clear the stale handle
                Ok(Some(_)) => {
                    proc.child = None;
                    false
                }
                // Still running
                Ok(None) => true,
                // Error checking — assume still running to be safe
                Err(_) => true,
            }
        } else {
            false
        }
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
}

// No Drop impl — the user chooses whether to stop servers on quit.
// If they click "No", servers keep running independently.

pub type SharedProcessManager = Arc<Mutex<ProcessManager>>;

pub fn new_shared(servers: Vec<ServerConfig>) -> SharedProcessManager {
    Arc::new(Mutex::new(ProcessManager::new(servers)))
}

// Security note: config.cmd is passed to a shell as a command string.
// This is intentional — users need shell features (PATH resolution, pipes, env expansion)
// for commands like "npm run dev" or "cargo run". The config file is local and user-authored,
// so the trust boundary is the filesystem, same as any shell script.
fn spawn_server(config: &ServerConfig) -> Result<Child, String> {
    // Spawn in a PowerShell window with the server name as the window title.
    // Each server gets its own window with visible logs. We keep the process handle
    // so Stop/Restart work from the tray menu.
    // Future: issue #10 will add WT tab mode with PID tracking.
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

    // CREATE_NEW_PROCESS_GROUP + CREATE_NEW_CONSOLE:
    // new console gives the server its own visible window,
    // new process group lets us kill the tree on stop
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x00000200 | 0x00000010); // CREATE_NEW_PROCESS_GROUP | CREATE_NEW_CONSOLE
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
    // If still not exited after 2s, give up — the handle will be dropped
}

/// Restart all PowerShell and Windows Terminal processes.
/// This kills existing ones — the user can reopen them fresh.
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
            .args(["/FI", &format!("IMAGENAME eq {}", proc_name), "/FO", "CSV", "/NH"])
            .output();

        if let Ok(output) = output {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                if line.contains(proc_name) {
                    // Parse PID from CSV: "name","pid","session","session#","mem"
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

    // Try Windows Terminal first, fall back to cmd.exe
    let wt_result = Command::new("cmd")
        .args(["/c", "start", "wt"])
        .spawn();

    if wt_result.is_err() {
        let _ = Command::new("cmd")
            .args(["/c", "start", "cmd.exe"])
            .spawn();
    }
}
