# Testing: Server Groups (Issue #16)

## What Changed
Server groups let you start/stop/restart a named subset of servers at once. Groups appear in the tray menu between individual servers and the bulk "Start All" actions.

## How to Test

### 1. Build & Launch
```
cargo build && target\debug\server-start.exe
```
(Or kill the running release exe first and `cargo build --release`)

### 2. Add groups to config
Open Config from the tray menu, add groups that reference your existing servers:
```toml
[[group]]
name = "Reader"
servers = ["Reader Frontend", "Reader Backend"]
```
Server names in `servers` must exactly match the `name` field in your `[[server]]` blocks.

### 3. Reload Config
Click "Reload Config" in the tray menu.

### 4. Verify group menu
- Right-click the tray icon
- You should see a "Reader [0/2]" submenu between your individual servers and "Start All Servers"
- The submenu should have: Start Group, Stop Group, Restart Group

### 5. Test Start Group
- Click "Start Group" inside the Reader submenu
- **Expected:** Only the servers in that group start, others stay stopped
- Menu should update to "Reader [2/2]"

### 6. Test Stop Group
- Click "Stop Group"
- **Expected:** Only the group's servers stop
- Menu should update to "Reader [0/2]"

### 7. Test Restart Group
- Start the group, then click "Restart Group"
- **Expected:** Servers stop then start, still running after

### 8. Verify other servers unaffected
- Start some servers NOT in the group
- Restart the group
- **Expected:** Non-group servers keep running, unaffected

### 9. Config reload preserves groups
- Edit config to add/remove a group, Reload Config
- **Expected:** Menu updates to reflect the new group definitions
