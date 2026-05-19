# Project Status

## Overview
Windows system tray app for managing dev servers. Single Rust binary, no runtime deps.

## Current State (2026-05-16)
- **v0.1.0 released** — https://github.com/paperhurts/server-start/releases/tag/v0.1.0
- All code review issues resolved (#1-#8)
- Output modes shipped (#10): terminal, logfile, hidden
- Mode toggle UI from tray (#14)
- Smart config reload preserving running servers (#13)
- Original hand-drawn synthwave icon (#11) — replaced by #18
- Server groups shipped (#16): PR #17, includes group validation
- **New cyan power-button icon (#18): MERGED via PR #20.** SVG-rendered at every target size at build time via `resvg`; ships both the tray RGBA and a multi-res Win32 .ico resource for the exe.
- **"Start with Windows" (#19): implemented on `issue-19-start-with-windows` (commit `fa21f76`), verified after a real reboot on 2026-05-16.** Tray checkbox toggles HKCU Run-key registration; app launches automatically at logon when enabled. Awaiting merge to main as part of the v0.2.0 release.
- Workspace cleanup (PR #21): `tasks/` and `.claude/` are gitignored. They were AI-assistant workspace dirs (handoff notes, introspection lessons) that shouldn't have been published. `tasks/` files are untracked but stay on disk for whoever's mid-test on them.

## Architecture
- `assets/server-start-icon.svg` — single source of truth for both tray and exe icons
- `build.rs` — at build time: renders SVG via resvg at each target size; writes raw-RGBA blob for tray + multi-res `.ico` embedded as Win32 resource
- `src/main.rs` — tray icon, menu building, event loop (winit + tray-icon)
- `src/autostart.rs` — HKCU Run-key toggle for "Start with Windows" (Windows-only)
- `src/config.rs` — TOML config parsing, OutputMode enum, GroupConfig, log path helpers
- `src/process.rs` — process spawning (3 modes), kill trees, config reload diffing, group operations
- `src/errors.rs` — MessageBoxW wrapper for user-visible error dialogs

## Open Issues
- No automated tests
- No CI pipeline
