# Server Start

A lightweight Windows system tray app for managing dev servers across multiple projects. Start, stop, and restart things like `npm run dev`, `cargo tauri dev`, or any command — all from your task tray.

## Features

- **Tray menu** with per-server Start / Stop / Restart controls
- **Output modes** — `terminal` (PowerShell windows with logs), `logfile` (hidden + log file), or `hidden` (no output)
- **Bulk actions** — Start All, Stop All, Restart All
- **Restart Terminals** — kills all PowerShell/pwsh/Windows Terminal sessions and opens a fresh one (asks for confirmation first)
- **Hot-reload config** — edit your config and reload without restarting the app
- **Open Config** — jump straight to your config file from the tray menu
- **Error dialogs** — config errors and server failures show Windows message boxes (no silent failures)
- **Crash detection** — automatically detects when a server exits and updates the menu status
- **Zero runtime dependencies** — single `.exe`, no Node/Python/etc. required

## Install

```
cargo install --path .
```

Or build manually:

```
cargo build --release
```

The binary will be at `target/release/server-start.exe`. Drop it in your startup folder or pin it however you like.

## Configuration

On first run, a sample config is created at:

```
%APPDATA%/server-start/config.toml
```

Add your dev servers:

```toml
# Global output mode (optional, defaults to "terminal")
# output = "terminal"

[[server]]
name = "Frontend"
dir = "C:/dev/my-app"
cmd = "npm run dev"

[[server]]
name = "Backend API"
dir = "C:/dev/my-api"
cmd = "cargo run"
output = "logfile"   # override: logs to %APPDATA%/server-start/logs/Backend_API.log

[[server]]
name = "Tauri Dev"
dir = "C:/dev/my-app"
cmd = "cargo tauri dev"
env = { RUST_LOG = "debug" }
```

Each `[[server]]` block defines one process:

| Field    | Required | Description                                      |
|----------|----------|--------------------------------------------------|
| `name`   | yes      | Display name in the tray menu                    |
| `dir`    | yes      | Working directory for the command                 |
| `cmd`    | yes      | The command to run                                |
| `env`    | no       | Extra environment variables to set                |
| `output` | no       | `"terminal"`, `"logfile"`, or `"hidden"` (overrides global) |

### Output Modes

| Mode       | Behavior                                                                 |
|------------|--------------------------------------------------------------------------|
| `terminal` | Opens a PowerShell window per server with the server name as the title. Logs are visible. This is the default. |
| `logfile`  | Server runs hidden. Output is written to `%APPDATA%/server-start/logs/<name>.log`. A "View Log" option appears in the server's tray submenu. |
| `hidden`   | Server runs hidden with no output captured. Use for servers where you don't need logs. |

Set a global default with `output = "terminal"` at the top of your config, and override per-server with the `output` field inside a `[[server]]` block.

## Usage

Right-click the tray icon to see your servers and controls:

- Each server has a submenu with **Start**, **Stop**, and **Restart**
- Logfile-mode servers also have a **View Log** option
- Status shows as `[running]` or `[stopped]` (auto-detects crashes)
- **Start/Stop/Restart All Servers** for bulk control
- **Restart Terminals** to kill and reopen all terminal windows (shows confirmation first — this kills *all* open terminals, not just ones managed by the app)
- **Open Config** to edit your server list
- **Reload Config** to pick up changes without restarting
- **Quit** asks whether to stop running servers or leave them running

> **Note:** Stopping/restarting a server kills its entire process tree. If you're running Claude or another tool inside a terminal that a configured server spawned, it will be killed when that server is stopped.

## Built With

Rust, using [tray-icon](https://crates.io/crates/tray-icon) and [winit](https://crates.io/crates/winit).
