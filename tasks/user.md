# Testing: Output Modes + Synthwave Icon (Issues #10, #11)

## What Changed
1. **Output modes** — three modes: `terminal` (PowerShell windows, default), `logfile` (hidden + log file), `hidden` (no output)
2. **Global + per-server config** — set `output = "logfile"` globally or per `[[server]]` block
3. **View Log menu item** — logfile-mode servers get a "View Log" option in their submenu
4. **Synthwave icon** — dark purple circle with cyan/magenta gradient glow and `~/` text

## How to Test

### 1. Launch
```
target\release\server-start.exe
```

### 2. Check the icon
- Look at the system tray — should be a dark circle with cyan/magenta glow and ~/ in the center
- Check it looks OK on your taskbar

### 3. Test terminal mode (default)
- Start a server with no `output` setting → PowerShell window with named title and visible logs

### 4. Test logfile mode
- Add `output = "logfile"` to one server in config, e.g.:
  ```toml
  [[server]]
  name = "Frontend"
  dir = "C:/dev/reader/frontend"
  cmd = "npm run dev"
  output = "logfile"
  ```
- Reload Config → Start that server
- **Expected:** No window appears, server runs hidden
- Check the menu — should have a "View Log" option in the server's submenu
- Click "View Log" — should open the log file
- Check `%APPDATA%/server-start/logs/Frontend.log` has content

### 5. Test hidden mode
- Set `output = "hidden"` on a server → Start it
- **Expected:** No window, no log file, server runs silently

### 6. Test global default
- Add `output = "logfile"` at the top of config (before any [[server]])
- **Expected:** All servers default to logfile mode unless they have their own `output` override

### 7. Quit behavior
- With servers running, Quit → should ask "Stop all running servers?"
