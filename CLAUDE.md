# CLAUDE

## About
Windows system tray app for managing dev servers. Right-click the tray icon to start/stop/restart servers defined in a TOML config, or restart all terminals.

## Build
```
cargo build          # debug
cargo build --release  # release (hides console window)
```

## Config
Config lives at `%APPDATA%/server-start/config.toml`. First run creates a sample.

