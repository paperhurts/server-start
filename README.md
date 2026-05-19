# Server Start

A lightweight Windows system tray app for managing dev servers across multiple projects. Start, stop, and restart things like `npm run dev`, `cargo tauri dev`, or any command — all from your task tray.

## Features

- **Tray menu** with per-server Start / Stop / Restart controls
- **Output modes** — `terminal` (PowerShell windows with logs), `logfile` (hidden + log file), or `hidden` (no output)
- **Mode toggle** — switch any server's output mode on the fly from the tray menu
- **Server groups** — group related servers (e.g., frontend + backend) and start/stop/restart them together
- **Bulk actions** — Start All, Stop All, Restart All
- **Start with Windows** — tray checkbox that registers the app to launch automatically at user logon
- **Restart Terminals** — kills all PowerShell/pwsh/Windows Terminal sessions and opens a fresh one (asks for confirmation first)
- **Hot-reload config** — edit your config and reload without restarting the app (running servers with unchanged config stay running)
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

The binary will be at `target/release/server-start.exe`. Drop it wherever you like — once it's running, the tray menu has a **Start with Windows** checkbox that handles launching it at logon for you (see [Start with Windows](#start-with-windows) below).

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

### Groups

Group related servers to start/stop/restart them together without affecting all servers:

```toml
[[group]]
name = "Reader"
servers = ["Reader Frontend", "Reader Backend"]

[[group]]
name = "Auth Stack"
servers = ["Auth API", "Redis"]
```

Each `[[group]]` block defines a named collection:

| Field     | Required | Description                                                |
|-----------|----------|------------------------------------------------------------|
| `name`    | yes      | Display name in the tray menu                              |
| `servers` | yes      | List of server names (must match `name` in `[[server]]` blocks) |

Groups appear in the tray menu between individual servers and the bulk actions. Each group shows a running count (e.g., `Reader [1/2]`) and has **Start Group**, **Stop Group**, and **Restart Group** actions.

### Output Modes

| Mode       | Behavior                                                                 |
|------------|--------------------------------------------------------------------------|
| `terminal` | Opens a PowerShell window per server with the server name as the title. Logs are visible. This is the default. |
| `logfile`  | Server runs hidden. Output is written to `%APPDATA%/server-start/logs/<name>.log`. A "View Log" option appears in the server's tray submenu. |
| `hidden`   | Server runs hidden with no output captured. Use for servers where you don't need logs. |

Set a global default with `output = "terminal"` at the top of your config, and override per-server with the `output` field inside a `[[server]]` block. You can also switch modes on the fly from each server's tray submenu — no config editing needed.

## Usage

Right-click the tray icon to see your servers and controls:

- Each server has a submenu with **Start**, **Stop**, **Restart**, and **Mode** (Terminal/Logfile/Hidden)
- Logfile-mode servers also have a **View Log** option
- Status shows as `[running]` or `[stopped]` (auto-detects crashes)
- **Groups** show running count and have **Start/Stop/Restart Group** actions
- **Start/Stop/Restart All Servers** for bulk control
- **Restart Terminals** to kill and reopen all terminal windows (shows confirmation first — this kills *all* open terminals, not just ones managed by the app)
- **Start with Windows** checkbox toggles whether the app launches at user logon (see below)
- **Open Config** to edit your server list
- **Reload Config** to pick up changes without restarting
- **Quit** asks whether to stop running servers or leave them running

### Start with Windows

Toggling the **Start with Windows** checkbox in the tray menu writes (or removes) a value at:

```
HKCU\Software\Microsoft\Windows\CurrentVersion\Run\ServerStart
```

When set, Windows launches `server-start.exe` automatically at user logon. The value points at the exe you're currently running, so if you move the binary, toggle it off and back on to update the path.

The registry is the single source of truth — there's no shadow state in `config.toml`. If you toggle the entry externally (Task Manager > Startup, or `regedit`), the tray menu reflects the current state the next time you open it.

Servers themselves do not auto-start on launch; the app comes up with everything stopped, same as a normal launch.

> **Note:** Stopping/restarting a server kills its entire process tree. If you're running Claude or another tool inside a terminal that a configured server spawned, it will be killed when that server is stopped.

## Built With

Rust, using [tray-icon](https://crates.io/crates/tray-icon) and [winit](https://crates.io/crates/winit).
