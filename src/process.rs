use std::collections::HashMap;
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};

use crate::config::ServerConfig;

pub struct ProcessManager {
    processes: HashMap<usize, ManagedProcess>,
}

struct ManagedProcess {
    config: ServerConfig,
    child: Option<Child>,
}

impl ProcessManager {
    pub fn new(servers: Vec<ServerConfig>) -> Self {
        let mut processes = HashMap::new();
        for (i, config) in servers.into_iter().enumerate() {
            processes.insert(
                i,
                ManagedProcess {
                    config,
                    child: None,
                },
            );
        }
        ProcessManager { processes }
    }

    pub fn start(&mut self, id: usize) -> Result<(), String> {
        let proc = self
            .processes
            .get_mut(&id)
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
            .get_mut(&id)
            .ok_or_else(|| format!("Server {} not found", id))?;

        if let Some(child) = proc.child.take() {
            kill_process_tree(child.id());
        }
        Ok(())
    }

    pub fn restart(&mut self, id: usize) -> Result<(), String> {
        self.stop(id)?;
        self.start(id)
    }

    pub fn is_running(&self, id: usize) -> bool {
        self.processes
            .get(&id)
            .map(|p| p.child.is_some())
            .unwrap_or(false)
    }

    pub fn start_all(&mut self) {
        let ids: Vec<usize> = self.processes.keys().copied().collect();
        for id in ids {
            if let Err(e) = self.start(id) {
                eprintln!("Failed to start server {}: {}", id, e);
            }
        }
    }

    pub fn stop_all(&mut self) {
        let ids: Vec<usize> = self.processes.keys().copied().collect();
        for id in ids {
            if let Err(e) = self.stop(id) {
                eprintln!("Failed to stop server {}: {}", id, e);
            }
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
        self.processes.get(&id).map(|p| p.config.name.as_str())
    }
}

impl Drop for ProcessManager {
    fn drop(&mut self) {
        self.stop_all();
    }
}

pub type SharedProcessManager = Arc<Mutex<ProcessManager>>;

pub fn new_shared(servers: Vec<ServerConfig>) -> SharedProcessManager {
    Arc::new(Mutex::new(ProcessManager::new(servers)))
}

fn spawn_server(config: &ServerConfig) -> Result<Child, String> {
    // On Windows, use cmd /c to run the command so that npm/cargo/etc. work
    let mut cmd = Command::new("cmd");
    cmd.args(["/c", &config.cmd]);
    cmd.current_dir(&config.dir);

    // CREATE_NEW_PROCESS_GROUP so we can kill the tree later
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x00000200); // CREATE_NEW_PROCESS_GROUP
    }

    for (key, val) in &config.env {
        cmd.env(key, val);
    }

    let child = cmd
        .spawn()
        .map_err(|e| format!("Failed to start '{}': {}", config.name, e))?;

    Ok(child)
}

/// Kill a process and all its children on Windows using taskkill /T
fn kill_process_tree(pid: u32) {
    let _ = Command::new("taskkill")
        .args(["/F", "/T", "/PID", &pid.to_string()])
        .output();
}

/// Restart all PowerShell and Windows Terminal processes.
/// This kills existing ones — the user can reopen them fresh.
pub fn restart_terminals() {
    // Kill all powershell/pwsh processes (except our own parent)
    let our_pid = std::process::id();

    // Use taskkill to kill terminal processes
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

    // Launch a fresh Windows Terminal
    let _ = Command::new("cmd")
        .args(["/c", "start", "wt"])
        .spawn();
}
