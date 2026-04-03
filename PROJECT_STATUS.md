# Project Status

## Overview
Windows system tray app for managing dev servers. Single Rust binary, no runtime deps.

## Current State (2026-04-03)
- **v0.1.0 released** — https://github.com/paperhurts/server-start/releases/tag/v0.1.0
- All code review issues resolved (#1-#8)
- Output modes shipped (#10): terminal, logfile, hidden
- Mode toggle UI from tray (#14)
- Smart config reload preserving running servers (#13)
- Synthwave icon (#11)

## Architecture
- `src/main.rs` — tray icon, menu building, event loop (winit + tray-icon)
- `src/config.rs` — TOML config parsing, OutputMode enum, log path helpers
- `src/process.rs` — process spawning (3 modes), kill trees, config reload diffing
- `src/errors.rs` — MessageBoxW wrapper for user-visible error dialogs

## Open Issues
- No open GitHub issues
- No automated tests
- No CI pipeline
