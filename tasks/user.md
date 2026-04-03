# Testing: Code Review Fixes (Issues #1-#8)

## What changed
- **Error dialogs**: All errors now show Windows MessageBox popups instead of invisible `eprintln!`
- **Process health**: Menu now detects crashed/exited servers and shows `[stopped]` correctly
- **Stop/restart reliability**: App waits up to 2s for processes to actually exit before restarting (prevents port conflicts)
- **Restart Terminals**: Now shows a YES/NO confirmation dialog before killing anything. Falls back to `cmd.exe` if Windows Terminal isn't installed.
- **Server stdio**: Spawned processes redirect stdout/stderr to null (prevents blocking on invalid handles)
- **Tray icon**: No longer panics if rebuild fails; icon alpha anti-aliasing fixed
- **Open Config**: Handles non-UTF-8 paths correctly
- **Server ordering**: Start/stop order now matches config file order (was random)

## How to test

### 1. Run the exe
```
target\release\server-start.exe
```
(Double-click or run from explorer — it's a tray app, no console window)

### 2. Test error dialog (config parse error)
- Open config: right-click tray → Open Config
- Break the TOML syntax (e.g., add `!!!` somewhere)
- Right-click tray → Reload Config
- **Expected**: A Windows error dialog pops up saying "Config Reload Failed"

### 3. Test server start/stop
- Fix config, add a test server:
  ```toml
  [[server]]
  name = "Test"
  dir = "C:/"
  cmd = "ping -t localhost"
  ```
- Reload Config
- Start the "Test" server from the tray menu
- **Expected**: Shows `[running]` in menu

### 4. Test crash detection
- While "Test" is running, open Task Manager and kill the `ping` process
- Right-click the tray icon again
- **Expected**: Shows `[stopped]` (not stuck on `[running]`)

### 5. Test Restart Terminals
- Right-click tray → Restart Terminals
- **Expected**: A YES/NO confirmation dialog appears BEFORE anything happens
- Click No → nothing happens
- Click Yes → terminals close, fresh one opens

### 6. Test restart reliability  
- Start a server, then Restart it from the menu
- **Expected**: No "port already in use" errors, clean restart
