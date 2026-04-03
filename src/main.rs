// Hide console window on Windows release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod errors;
mod process;

use config::{Config, OutputMode};
use process::{new_shared, SharedProcessManager};
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::WindowId;

const MENU_ID_START_ALL: &str = "start_all";
const MENU_ID_STOP_ALL: &str = "stop_all";
const MENU_ID_RESTART_ALL: &str = "restart_all";
const MENU_ID_RESTART_TERMINALS: &str = "restart_terminals";
const MENU_ID_RELOAD_CONFIG: &str = "reload_config";
const MENU_ID_OPEN_CONFIG: &str = "open_config";
const MENU_ID_QUIT: &str = "quit";

fn main() {
    let config = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            errors::show_error(
                "Config Error",
                &format!("{}\n\nPath: {}", e, Config::config_path().display()),
            );
            Config {
                output: OutputMode::default(),
                server: Vec::new(),
            }
        }
    };

    if config.server.is_empty() {
        let config_path = Config::config_path();
        if !config_path.exists() {
            let sample = r#"# Server Start Configuration
# Uncomment and edit the blocks below. Every block MUST start with [[server]].
# The name inside the brackets is always "server" — it does NOT change per project.

# ─── Global output mode (applies to all servers unless overridden) ───
# "terminal" = PowerShell window with visible logs (default)
# "logfile"  = hidden, logs to %APPDATA%/server-start/logs/<name>.log
# "hidden"   = hidden, no windows, no output captured
# output = "terminal"

# ─── Example: typical full-stack setup ───
# [[server]]
# name = "Frontend"
# dir = "C:/dev/my-app"
# cmd = "npm run dev"
#
# [[server]]
# name = "Backend API"
# dir = "C:/dev/my-api"
# cmd = "cargo run"
# env = { RUST_LOG = "debug" }
# output = "logfile"       # per-server override
#
# [[server]]
# name = "Tauri Dev"
# dir = "C:/dev/my-app"
# cmd = "cargo tauri dev"
# env = { RUST_LOG = "debug", RUST_BACKTRACE = "1" }

# ─── Common env vars ───
# env = { RUST_LOG = "debug" }
# env = { RUST_LOG = "debug", RUST_BACKTRACE = "1" }
# env = { NODE_ENV = "development", PORT = "3001" }
# env = { DEBUG = "*" }
"#;
            std::fs::write(&config_path, sample).ok();
        }
        errors::show_error(
            "No Servers Configured",
            &format!(
                "No servers found in config. Add [[server]] blocks to:\n\n{}",
                Config::config_path().display()
            ),
        );
    }

    let manager = new_shared(config.server, config.output);
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let mut app = App::new(manager);

    event_loop.run_app(&mut app).expect("Event loop failed");
}

struct App {
    manager: SharedProcessManager,
    _tray: Option<TrayIcon>,
    server_count: usize,
}

impl App {
    fn new(manager: SharedProcessManager) -> Self {
        let server_count = manager.lock().unwrap().server_count();
        App {
            manager,
            _tray: None,
            server_count,
        }
    }

    fn build_menu(&self) -> Menu {
        let menu = Menu::new();
        let mut mgr = self.manager.lock().unwrap();

        // Individual server controls
        for id in 0..self.server_count {
            let name = mgr.server_name(id).unwrap_or("Unknown").to_string();
            let running = mgr.is_running(id);
            let current_mode = mgr.server_output_mode(id).cloned().unwrap_or_default();
            let status = if running { " [running]" } else { " [stopped]" };

            let submenu = Submenu::new(format!("{}{}", name, status), true);

            let start_item = MenuItem::with_id(format!("start_{}", id), "Start", !running, None);
            let stop_item = MenuItem::with_id(format!("stop_{}", id), "Stop", running, None);
            let restart_item =
                MenuItem::with_id(format!("restart_{}", id), "Restart", running, None);

            submenu.append(&start_item).ok();
            submenu.append(&stop_item).ok();
            submenu.append(&restart_item).ok();

            // Output mode selector
            submenu.append(&PredefinedMenuItem::separator()).ok();
            let mode_sub = Submenu::new("Mode", true);
            let check = |m: &OutputMode| if *m == current_mode { " *" } else { "" };
            let terminal_item = MenuItem::with_id(
                format!("mode_terminal_{}", id),
                format!("Terminal{}", check(&OutputMode::Terminal)),
                current_mode != OutputMode::Terminal,
                None,
            );
            let logfile_item = MenuItem::with_id(
                format!("mode_logfile_{}", id),
                format!("Logfile{}", check(&OutputMode::Logfile)),
                current_mode != OutputMode::Logfile,
                None,
            );
            let hidden_item = MenuItem::with_id(
                format!("mode_hidden_{}", id),
                format!("Hidden{}", check(&OutputMode::Hidden)),
                current_mode != OutputMode::Hidden,
                None,
            );
            mode_sub.append(&terminal_item).ok();
            mode_sub.append(&logfile_item).ok();
            mode_sub.append(&hidden_item).ok();
            submenu.append(&mode_sub).ok();

            // View Log option for logfile-mode servers
            if current_mode == OutputMode::Logfile {
                let view_log =
                    MenuItem::with_id(format!("viewlog_{}", id), "View Log", true, None);
                submenu.append(&view_log).ok();
            }

            menu.append(&submenu).ok();
        }

        if self.server_count > 0 {
            menu.append(&PredefinedMenuItem::separator()).ok();
        }

        // Bulk actions
        let start_all = MenuItem::with_id(MENU_ID_START_ALL, "Start All Servers", true, None);
        let stop_all = MenuItem::with_id(MENU_ID_STOP_ALL, "Stop All Servers", true, None);
        let restart_all =
            MenuItem::with_id(MENU_ID_RESTART_ALL, "Restart All Servers", true, None);

        menu.append(&start_all).ok();
        menu.append(&stop_all).ok();
        menu.append(&restart_all).ok();

        menu.append(&PredefinedMenuItem::separator()).ok();

        // Terminal restart
        let restart_terms = MenuItem::with_id(
            MENU_ID_RESTART_TERMINALS,
            "Restart Terminals",
            true,
            None,
        );
        menu.append(&restart_terms).ok();

        menu.append(&PredefinedMenuItem::separator()).ok();

        // Config
        let open_config = MenuItem::with_id(MENU_ID_OPEN_CONFIG, "Open Config", true, None);
        let reload_config = MenuItem::with_id(MENU_ID_RELOAD_CONFIG, "Reload Config", true, None);
        menu.append(&open_config).ok();
        menu.append(&reload_config).ok();

        menu.append(&PredefinedMenuItem::separator()).ok();

        let quit = MenuItem::with_id(MENU_ID_QUIT, "Quit", true, None);
        menu.append(&quit).ok();

        menu
    }

    fn rebuild_tray(&mut self) {
        let menu = self.build_menu();
        let icon = create_icon();

        match TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Server Start")
            .with_icon(icon)
            .build()
        {
            Ok(tray) => {
                self._tray = Some(tray);
            }
            Err(e) => {
                errors::show_error("Tray Error", &format!("Failed to rebuild tray icon: {}", e));
            }
        }
    }

    fn handle_menu_event(&mut self, event: MenuEvent, event_loop: &ActiveEventLoop) {
        let id = event.id().0.as_str();

        match id {
            MENU_ID_QUIT => {
                let has_running = {
                    let mut mgr = self.manager.lock().unwrap();
                    (0..mgr.server_count()).any(|id| mgr.is_running(id))
                };
                if has_running
                    && errors::confirm(
                        "Quit Server Start",
                        "Stop all running servers before quitting?",
                    )
                {
                    self.manager.lock().unwrap().stop_all();
                }
                event_loop.exit();
            }
            MENU_ID_START_ALL => {
                self.manager.lock().unwrap().start_all();
                self.rebuild_tray();
            }
            MENU_ID_STOP_ALL => {
                self.manager.lock().unwrap().stop_all();
                self.rebuild_tray();
            }
            MENU_ID_RESTART_ALL => {
                self.manager.lock().unwrap().restart_all();
                self.rebuild_tray();
            }
            MENU_ID_RESTART_TERMINALS => {
                process::restart_terminals();
            }
            MENU_ID_OPEN_CONFIG => {
                let path = Config::config_path();
                let path_str = path.to_string_lossy();
                let _ = std::process::Command::new("cmd")
                    .args(["/c", "start", "", &path_str])
                    .spawn();
            }
            MENU_ID_RELOAD_CONFIG => {
                match Config::load() {
                    Ok(config) => {
                        if config.server.is_empty() {
                            errors::show_error(
                                "No Servers Configured",
                                &format!(
                                    "No servers found in config. Add [[server]] blocks to:\n\n{}",
                                    Config::config_path().display()
                                ),
                            );
                        }
                        self.manager
                            .lock()
                            .unwrap()
                            .reload(config.server.clone(), config.output.clone());
                        self.server_count = config.server.len();
                        self.rebuild_tray();
                    }
                    Err(e) => {
                        errors::show_error("Config Reload Failed", &e);
                    }
                }
            }
            other => {
                if let Some(id_str) = other.strip_prefix("start_") {
                    if let Ok(server_id) = id_str.parse::<usize>() {
                        if let Err(e) = self.manager.lock().unwrap().start(server_id) {
                            errors::show_error("Start Failed", &e);
                        }
                        self.rebuild_tray();
                    }
                } else if let Some(id_str) = other.strip_prefix("stop_") {
                    if let Ok(server_id) = id_str.parse::<usize>() {
                        if let Err(e) = self.manager.lock().unwrap().stop(server_id) {
                            errors::show_error("Stop Failed", &e);
                        }
                        self.rebuild_tray();
                    }
                } else if let Some(id_str) = other.strip_prefix("restart_") {
                    if let Ok(server_id) = id_str.parse::<usize>() {
                        if let Err(e) = self.manager.lock().unwrap().restart(server_id) {
                            errors::show_error("Restart Failed", &e);
                        }
                        self.rebuild_tray();
                    }
                } else if let Some(id_str) = other.strip_prefix("mode_terminal_") {
                    if let Ok(server_id) = id_str.parse::<usize>() {
                        if let Err(e) = self.manager.lock().unwrap().set_output_mode(server_id, OutputMode::Terminal) {
                            errors::show_error("Mode Change Failed", &e);
                        }
                        self.rebuild_tray();
                    }
                } else if let Some(id_str) = other.strip_prefix("mode_logfile_") {
                    if let Ok(server_id) = id_str.parse::<usize>() {
                        if let Err(e) = self.manager.lock().unwrap().set_output_mode(server_id, OutputMode::Logfile) {
                            errors::show_error("Mode Change Failed", &e);
                        }
                        self.rebuild_tray();
                    }
                } else if let Some(id_str) = other.strip_prefix("mode_hidden_") {
                    if let Ok(server_id) = id_str.parse::<usize>() {
                        if let Err(e) = self.manager.lock().unwrap().set_output_mode(server_id, OutputMode::Hidden) {
                            errors::show_error("Mode Change Failed", &e);
                        }
                        self.rebuild_tray();
                    }
                } else if let Some(id_str) = other.strip_prefix("viewlog_") {
                    if let Ok(server_id) = id_str.parse::<usize>() {
                        if let Some(name) = self
                            .manager
                            .lock()
                            .unwrap()
                            .server_name(server_id)
                            .map(|s| s.to_string())
                        {
                            let log_path = Config::log_path(&name);
                            let path_str = log_path.to_string_lossy();
                            let _ = std::process::Command::new("cmd")
                                .args(["/c", "start", "", &path_str])
                                .spawn();
                        }
                    }
                }
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        if self._tray.is_none() {
            self.rebuild_tray();
        }
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        _event: WindowEvent,
    ) {
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            self.handle_menu_event(event, event_loop);
        }
    }
}

/// Synthwave-styled tray icon: dark circle with cyan/magenta glow and ~/ text
fn create_icon() -> Icon {
    let size = 32u32;
    let mut rgba = vec![0u8; (size * size * 4) as usize];

    let center = size as f32 / 2.0;
    let radius = center - 1.0;

    // Hardcoded 11x7 bitmap for "~/" — each byte is a row, bits are pixels
    // Tilde is ~6px wide, slash is ~3px wide, 2px gap
    #[rustfmt::skip]
    const GLYPH_TILDE_SLASH: [[u8; 11]; 7] = [
        //  ~ ~ ~ ~ ~ ~   / / /
        [0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0],  // row 0:           /
        [0, 1, 1, 0, 0, 1, 0, 0, 1, 0, 0],  // row 1:  ~~  ~   /
        [1, 0, 0, 1, 1, 0, 0, 0, 1, 0, 0],  // row 2: ~  ~~   /
        [0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0],  // row 3:        /
        [0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0],  // row 4:        /
        [0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0],  // row 5:       /
        [0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0],  // row 6:       /
    ];

    let glyph_w = 11u32;
    let glyph_h = 7u32;
    let glyph_x = (size - glyph_w) / 2; // center horizontally
    let glyph_y = (size - glyph_h) / 2; // center vertically

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let dist = (dx * dx + dy * dy).sqrt();
            let idx = ((y * size + x) * 4) as usize;

            if dist <= radius {
                // Angle for cyan→magenta gradient (0 at top, wraps around)
                let angle = dy.atan2(dx); // -PI to PI
                let t = (angle + std::f32::consts::PI) / (2.0 * std::f32::consts::PI); // 0..1

                // Background: dark purple base with gradient glow at edges
                let edge_factor = (dist / radius).powi(3); // stronger glow near edge

                // Cyan (#00fff0) to magenta (#ff00ff) based on angle
                let cyan_r = 0.0_f32;
                let cyan_g = 255.0;
                let cyan_b = 240.0;
                let mag_r = 255.0;
                let mag_g = 0.0;
                let mag_b = 255.0;

                let glow_r = cyan_r + (mag_r - cyan_r) * t;
                let glow_g = cyan_g + (mag_g - cyan_g) * t;
                let glow_b = cyan_b + (mag_b - cyan_b) * t;

                // Dark base: #1a1a2e
                let base_r = 26.0;
                let base_g = 26.0;
                let base_b = 46.0;

                let r = base_r + (glow_r - base_r) * edge_factor * 0.7;
                let g = base_g + (glow_g - base_g) * edge_factor * 0.7;
                let b = base_b + (glow_b - base_b) * edge_factor * 0.7;

                rgba[idx] = r.clamp(0.0, 255.0) as u8;
                rgba[idx + 1] = g.clamp(0.0, 255.0) as u8;
                rgba[idx + 2] = b.clamp(0.0, 255.0) as u8;
                rgba[idx + 3] = 255;

                // Draw ~/ glyph on top
                let gx = x.wrapping_sub(glyph_x);
                let gy = y.wrapping_sub(glyph_y);
                if gx < glyph_w
                    && gy < glyph_h
                    && GLYPH_TILDE_SLASH[gy as usize][gx as usize] == 1
                {
                    // Bright cyan text with slight glow
                    rgba[idx] = 0;
                    rgba[idx + 1] = 255;
                    rgba[idx + 2] = 240;
                    rgba[idx + 3] = 255;
                }
            } else if dist <= radius + 1.0 {
                // Anti-aliased edge
                let alpha = ((1.0 - (dist - radius)) * 255.0).clamp(0.0, 255.0);
                let angle = dy.atan2(dx);
                let t = (angle + std::f32::consts::PI) / (2.0 * std::f32::consts::PI);

                let r = (255.0 * t).clamp(0.0, 255.0);
                let g = (255.0 * (1.0 - t) * 0.5).clamp(0.0, 255.0);
                let b = (240.0 + 15.0 * t).clamp(0.0, 255.0);

                rgba[idx] = r as u8;
                rgba[idx + 1] = g as u8;
                rgba[idx + 2] = b as u8;
                rgba[idx + 3] = alpha as u8;
            }
        }
    }

    Icon::from_rgba(rgba, size, size).expect("Failed to create icon")
}
