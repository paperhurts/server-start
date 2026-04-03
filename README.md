# Server Start

A lightweight Windows system tray app for managing dev servers across multiple projects. Start, stop, and restart things like `npm run dev`, `cargo tauri dev`, or any command — all from your task tray.

## Features

- **Tray menu** with per-server Start / Stop / Restart controls
- **Bulk actions** — Start All, Stop All, Restart All
- **Restart Terminals** — kills all PowerShell/pwsh/Windows Terminal sessions and opens a fresh one (great after installing new CLI tools)
- **Hot-reload config** — edit your config and reload without restarting the app
- **Open Config** — jump straight to your config file from the tray menu
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
[[server]]
name = "Frontend"
dir = "C:/dev/my-app"
cmd = "npm run dev"

[[server]]
name = "Backend API"
dir = "C:/dev/my-api"
cmd = "cargo run"

[[server]]
name = "Tauri Dev"
dir = "C:/dev/my-app"
cmd = "cargo tauri dev"
env = { RUST_LOG = "debug" }
```

Each `[[server]]` block defines one process:

| Field  | Required | Description                        |
|--------|----------|------------------------------------|
| `name` | yes      | Display name in the tray menu      |
| `dir`  | yes      | Working directory for the command   |
| `cmd`  | yes      | The command to run (via `cmd /c`)   |
| `env`  | no       | Extra environment variables to set  |

## Usage

Right-click the green tray icon to see your servers and controls:

- Each server has a submenu with **Start**, **Stop**, and **Restart**
- Status shows as `[running]` or `[stopped]`
- **Start/Stop/Restart All Servers** for bulk control
- **Restart Terminals** to kill and reopen all terminal windows
- **Open Config** to edit your server list
- **Reload Config** to pick up changes without restarting

## Built With

Rust, using [tray-icon](https://crates.io/crates/tray-icon) and [winit](https://crates.io/crates/winit).
