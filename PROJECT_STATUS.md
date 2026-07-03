# Project Status

## Overview
Windows system tray app for managing dev servers. Single Rust binary, no runtime deps.

## Current State (2026-07-03)
- **External server detection (#24):** optional `port` field per `[[server]]`; servers started outside the app (terminal, AI dev session) show as `[external]` via the Windows TCP listener table (`GetExtendedTcpTable`, IPv4+IPv6). Stop kills the detected PID tree, Restart adopts the server under app management, Start refuses when the port is externally occupied. New "Refresh Status" menu item + auto-refresh on tray-icon hover (menu swapped in place via `set_menu`, no icon flicker). Config reload ignores port-only changes. First unit tests added (config parsing, port probe, detection logic) — `cargo test`. Manually verified against the reader-app prod frontend on 2026-07-03.
- **v0.2.0 released** — https://github.com/paperhurts/server-start/releases/tag/v0.2.0
- **v0.1.0** — https://github.com/paperhurts/server-start/releases/tag/v0.1.0
- All code review issues resolved (#1-#8)
- Output modes shipped (#10): terminal, logfile, hidden
- Mode toggle UI from tray (#14)
- Smart config reload preserving running servers (#13)
- Original hand-drawn synthwave icon (#11) — replaced by #18

### Shipped in v0.2.0
- **Server groups (#16, PR #17):** start/stop/restart multiple servers at once; includes group validation.
- **New cyan power-button icon (#18, PR #20):** single SVG (`assets/server-start-icon.svg`) rendered at every target size at build time via `resvg`; ships both the tray RGBA and a multi-res Win32 `.ico` resource for the exe.
- **"Start with Windows" (#19, PR #22):** tray checkbox toggles HKCU Run-key registration; app launches automatically at logon when enabled. Registry is the single source of truth — the menu re-queries on rebuild so external toggles (Task Manager > Startup, regedit) are reflected. Verified after a real reboot on 2026-05-16.
- Workspace cleanup (PR #21): `tasks/` and `.claude/` are gitignored. They were AI-assistant workspace dirs (handoff notes, introspection lessons) that shouldn't have been published.

## Architecture
- `assets/server-start-icon.svg` — single source of truth for both tray and exe icons
- `build.rs` — at build time: renders SVG via resvg at each target size; writes raw-RGBA blob for tray + multi-res `.ico` embedded as Win32 resource
- `src/main.rs` — tray icon, menu building, event loop (winit + tray-icon)
- `src/autostart.rs` — HKCU Run-key toggle for "Start with Windows" (Windows-only)
- `src/config.rs` — TOML config parsing, OutputMode enum, GroupConfig, log path helpers
- `src/ports.rs` — Windows TCP listener table probe (port → owning PID) for external-server detection
- `src/process.rs` — process spawning (3 modes), kill trees, RunState (running/external/stopped), config reload diffing, group operations
- `src/errors.rs` — MessageBoxW wrapper for user-visible error dialogs

## Open Issues
- Test coverage is minimal (unit tests exist for config parsing, port probe, and external detection; nothing for spawning/menu logic)
- No CI pipeline
