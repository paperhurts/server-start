# Project Status

## Overview
Windows system tray app for managing dev servers. Single Rust binary, no runtime deps.

## Current State (2026-04-03)
- Initial codebase committed (2 commits on main)
- Branch `issue-1-8-code-review-fixes` has all fixes from code review (issues #1-#8)
- **Awaiting user testing** before push
- No tests, no CI

## Architecture
- `src/main.rs` — tray icon, menu building, event loop (winit + tray-icon)
- `src/config.rs` — TOML config parsing from `%APPDATA%/server-start/config.toml`
- `src/process.rs` — process spawning, kill trees, terminal restart
- `src/errors.rs` — MessageBoxW wrapper for user-visible error dialogs

## Known Issues
- No automated tests
- No CI pipeline
- No logging to file (errors are dialogs only)
