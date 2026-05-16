# Testing: New App Icon (Issue #18)

## What Changed
The hand-drawn pixel "~/" tray icon is replaced with a cyan rounded-square power-button design rendered fresh from an SVG. The executable also gains a proper multi-resolution icon so File Explorer, Alt-Tab, the taskbar, and Task Manager all show the brand mark instead of the default Rust cog.

Single source of truth: `assets/server-start-icon.svg`. `build.rs` calls `resvg` to render the vector at every needed size — 256×256 for the tray (raw RGBA, embedded via `include_bytes!`) and a multi-resolution `.ico` (16/32/48/64/128/256) embedded as a Win32 resource. Each ICO frame is rasterised directly at its target size, not downsampled from a larger raster, so 16×16 stays legible.

## How to Test

Before starting: confirm `target\release\server-start.exe` from any previous session is **not running**. Right-click the tray icon → Quit if it is.

### 1. Build
```
cargo build --release
```
Expect: clean build, no warnings. Binary at `target\release\server-start.exe` (~1.3 MB).

### 2. Tray icon
```
target\release\server-start.exe
```
- Look at the system tray (notification area, bottom-right)
- **Expected:** cyan rounded-square icon with a magenta power button glyph
- If your taskbar is set to hide overflow icons, click the up-arrow to see it

### 3. File Explorer icon
- Navigate to `target\release\` in Explorer
- Find `server-start.exe`
- **Expected:** the file shows the new icon as its thumbnail

  **If Explorer shows the old icon**, it's a Windows shell cache, not a build problem. Force a refresh with `ie4uinit.exe -show` from any cmd/PowerShell. If still stale, `taskkill /f /im explorer.exe && del /a %localappdata%\IconCache.db && start explorer.exe` (brief screen flicker, nothing destroyed beyond regenerable caches).

### 4. Task Manager
- Open Task Manager → Details tab
- Find `server-start.exe`
- **Expected:** the new icon shows in the leftmost column

### 5. Right-click → Properties on the exe
- Right-click `server-start.exe` → Properties
- The top-left should show the new icon

### 6. Sanity check the tray menu still works
- Right-click the tray icon → menu opens normally
- Each existing menu item should still work (start/stop a server, switch a mode, Reload Config)
- Nothing about menu behavior changed — this PR only touches icon generation

## What to Look For
- ✅ Tray icon matches the cyan power-button SVG
- ✅ Exe icon in File Explorer matches the SVG (after cache clear if needed)
- ✅ Icon stays crisp at 16×16 (Task Manager Details column) — no downsample mush
- ✅ No crashes on launch
- ✅ All existing menu functionality preserved

## If Something's Wrong
- **Tray looks chunky on a high-DPI display:** the tray RGBA is 256×256. tray-icon scales internally; if it's still off, the next iteration would be to load the tray icon from the embedded multi-res `.ico` resource via `Icon::from_resource(1, None)` so Windows picks the perfect-fit frame. ~5 LOC change, no new deps. Not done in this PR.
- **Build error mentioning rc.exe / windres:** `winresource` needs a resource compiler. On MSVC toolchain it uses `rc.exe` from the Windows SDK; should already be installed if `cargo build` has ever worked here.
- **Build error parsing the SVG:** the source has to be valid SVG that `resvg`/`usvg` can parse. Pure paths/shapes are fine; embedded raster `<image>` tags or system-font `<text>` would need extra setup.

## After Confirming It Works
Already confirmed visually. Branch pushed to origin; PR pending.
