# Testing: Start with Windows (Issue #19)

## What Changed
The tray menu has a new **"Start with Windows"** checkbox above "Open Config". Toggling it writes/deletes a value at:

```
HKCU\Software\Microsoft\Windows\CurrentVersion\Run
  ServerStart = "C:\path\to\server-start.exe"
```

When set, Windows launches server-start at user logon. Servers stay stopped at launch (same behavior as a normal launch).

The registry is the **single source of truth** — there's no shadow state in the config file. Toggling externally (Task Manager > Startup, regedit) is detected because the menu re-queries the registry every time it's rebuilt.

## How to Test

Before starting: confirm any existing `server-start.exe` is **not running** (right-click tray → Quit).

### 1. Build
```
cargo build --release
```
Expect: clean build. Single new runtime dep added: `winreg` (Windows-only).

### 2. Verify the toggle exists
```
target\release\server-start.exe
```
Right-click the tray icon. Above "Open Config" you should see a checkbox: **☐ Start with Windows**.

### 3. Enable
- Click "Start with Windows"
- Right-click the tray again — the box should now be **☑ Start with Windows**
- In a regular PowerShell, run:
  ```
  reg query "HKCU\Software\Microsoft\Windows\CurrentVersion\Run" /v ServerStart
  ```
  Expected output contains:
  ```
  ServerStart    REG_SZ    "C:\dev\server-start\target\release\server-start.exe"
  ```
  (Path quoted, matches your build location.)

### 4. Verify in Task Manager
- Open Task Manager → **Startup apps** tab
- Find an entry whose Name is "ServerStart" with status **Enabled**
- The publisher will be empty (we don't sign the binary). Disregard.

### 5. Disable
- Click "Start with Windows" again in the tray menu
- Box should be unchecked
- Run the `reg query` from step 3 again — expected: **"ERROR: ... unable to find the specified registry key or value"**
- Task Manager Startup apps tab: the entry should be gone

### 6. Reboot test (the real one)
- Re-enable from the tray menu
- **Quit** server-start from the tray (Quit confirms if servers are running)
- Sign out and back in, or reboot
- After logon: server-start should appear in the tray automatically

### 7. External-edit handling
- With autostart enabled, open Task Manager → Startup apps → right-click "ServerStart" → **Disable**
- Right-click the tray icon → the checkbox should now show **unchecked** (menu re-queried registry)
- Click it to re-enable through our UI — it should work cleanly

### 8. Edge case: moved binary
- Enable autostart
- Quit server-start
- Rename `target\release\server-start.exe` to `server-start-2.exe`
- Run `server-start-2.exe`
- Click "Start with Windows" twice (off, then on)
- `reg query ... /v ServerStart` should show the **new** path. `enable()` overwrites — no manual cleanup needed.
- Rename it back afterwards.

## What to Look For
- ✅ Checkbox appears in correct position (above "Open Config", with separators)
- ✅ Registry value writes/deletes correctly
- ✅ Task Manager Startup tab reflects state
- ✅ App actually launches at logon after a reboot
- ✅ External changes (via Task Manager) propagate to the menu on next right-click
- ✅ No error dialogs unless something genuinely went wrong

## If Something's Wrong
- **"Failed to write registry value: Access is denied"**: unlikely under HKCU which is per-user; would indicate a corrupted profile or a Group Policy override. Check `gpresult /h policy.html` for restrictive policies on the Run key.
- **App doesn't appear at logon**: verify the registry value exists with `reg query`. If yes but it still doesn't launch, the path may have a parsing problem — copy the value and try running it manually from cmd.
- **Checkbox stays unchecked after clicking**: an error must have triggered (dialog should pop). If no dialog, an `is_enabled()` query is returning false right after a successful `enable()` — would suggest a permission or registry virtualization quirk. Report back.

## After Confirming It Works
Tell me and I'll:
1. Push `issue-19-start-with-windows` to origin
2. Open a PR against `main`
3. Close issue #19 once merged

## Still Pending (Separate Branches)
- `issue-18-app-icons` — PNG-pipeline icon work committed locally, awaiting redesign before pushing
