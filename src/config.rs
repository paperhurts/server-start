use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// How a server's output is displayed.
#[derive(Debug, Deserialize, Clone, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OutputMode {
    /// PowerShell window per server with named title and visible logs
    #[default]
    Terminal,
    /// Hidden process, stdout/stderr redirected to a log file
    Logfile,
    /// Hidden process, no output captured
    Hidden,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    /// Global default output mode for all servers
    #[serde(default)]
    pub output: OutputMode,
    #[serde(default)]
    pub server: Vec<ServerConfig>,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct ServerConfig {
    pub name: String,
    pub dir: String,
    pub cmd: String,
    /// Optional: environment variables for this server
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Per-server output mode override (uses global default if not set)
    pub output: Option<OutputMode>,
}

impl ServerConfig {
    /// Returns the effective output mode, preferring per-server override over global default.
    pub fn effective_output<'a>(&'a self, global: &'a OutputMode) -> &'a OutputMode {
        self.output.as_ref().unwrap_or(global)
    }
}

impl Config {
    pub fn load() -> Result<Self, String> {
        let path = Self::config_path();
        if !path.exists() {
            return Ok(Config {
                output: OutputMode::default(),
                server: Vec::new(),
            });
        }
        let contents = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read config at {}: {}", path.display(), e))?;
        let config: Config =
            toml::from_str(&contents).map_err(|e| format!("Failed to parse config: {}", e))?;
        Ok(config)
    }

    /// The config directory: `%APPDATA%/server-start/`
    pub fn config_dir() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("server-start");
        fs::create_dir_all(&path).ok();
        path
    }

    pub fn config_path() -> PathBuf {
        let mut path = Self::config_dir();
        path.push("config.toml");
        path
    }

    /// Log directory: `%APPDATA%/server-start/logs/`
    pub fn logs_dir() -> PathBuf {
        let mut path = Self::config_dir();
        path.push("logs");
        fs::create_dir_all(&path).ok();
        path
    }

    /// Log file path for a given server name
    pub fn log_path(server_name: &str) -> PathBuf {
        let mut path = Self::logs_dir();
        // Sanitize server name for use as filename
        let safe_name: String = server_name
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect();
        path.push(format!("{}.log", safe_name));
        path
    }
}
