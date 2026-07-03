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
    /// Named groups of servers for bulk start/stop/restart
    #[serde(default)]
    pub group: Vec<GroupConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GroupConfig {
    pub name: String,
    /// Server names belonging to this group (must match [[server]] name fields)
    pub servers: Vec<String>,
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
    /// Optional: TCP port the server listens on. Enables detection of
    /// externally-started instances (shown as [external] in the tray).
    pub port: Option<u16>,
}

impl ServerConfig {
    /// Returns the effective output mode, preferring per-server override over global default.
    pub fn effective_output<'a>(&'a self, global: &'a OutputMode) -> &'a OutputMode {
        self.output.as_ref().unwrap_or(global)
    }

    /// True when the fields that affect process spawning are equal.
    /// `port` is intentionally excluded: it only affects status detection,
    /// so changing it must not restart a running server on config reload.
    pub fn spawn_fields_eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.dir == other.dir
            && self.cmd == other.cmd
            && self.env == other.env
            && self.output == other.output
    }
}

impl Config {
    pub fn load() -> Result<Self, String> {
        let path = Self::config_path();
        if !path.exists() {
            return Ok(Config {
                output: OutputMode::default(),
                server: Vec::new(),
                group: Vec::new(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_server_with_port() {
        let cfg: Config = toml::from_str(
            r#"
            [[server]]
            name = "web"
            dir = "C:/x"
            cmd = "npm run dev"
            port = 5173
        "#,
        )
        .unwrap();
        assert_eq!(cfg.server[0].port, Some(5173));
    }

    #[test]
    fn port_is_optional() {
        let cfg: Config = toml::from_str(
            r#"
            [[server]]
            name = "web"
            dir = "C:/x"
            cmd = "npm run dev"
        "#,
        )
        .unwrap();
        assert_eq!(cfg.server[0].port, None);
    }

    #[test]
    fn rejects_out_of_range_port() {
        let result: Result<Config, _> = toml::from_str(
            r#"
            [[server]]
            name = "web"
            dir = "C:/x"
            cmd = "npm run dev"
            port = 70000
        "#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn spawn_fields_eq_ignores_port() {
        let cfg: Config = toml::from_str(
            r#"
            [[server]]
            name = "web"
            dir = "C:/x"
            cmd = "npm run dev"
            port = 5173

            [[server]]
            name = "web"
            dir = "C:/x"
            cmd = "npm run dev"
            port = 8080
        "#,
        )
        .unwrap();
        assert!(cfg.server[0].spawn_fields_eq(&cfg.server[1]));
        assert_ne!(cfg.server[0], cfg.server[1]);
    }
}
