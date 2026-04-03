// Hide console window on Windows release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod process;

use config::Config;
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
            eprintln!("Config error: {}", e);
            eprintln!("Config path: {}", Config::config_path().display());
            Config {
                server: Vec::new(),
            }
        }
    };

    if config.server.is_empty() {
        let config_path = Config::config_path();
        if !config_path.exists() {
            let sample = r#"# Server Start Configuration
# Add your dev servers here. Each [[server]] block is one process.

# [[server]]
# name = "Frontend"
# dir = "C:/dev/my-app"
# cmd = "npm run dev"
#
# [[server]]
# name = "Backend API"
# dir = "C:/dev/my-api"
# cmd = "cargo run"
#
# [[server]]
# name = "Tauri Dev"
# dir = "C:/dev/my-app"
# cmd = "cargo tauri dev"
# env = { RUST_LOG = "debug" }
"#;
            std::fs::write(&config_path, sample).ok();
            eprintln!(
                "No servers configured. Sample config created at: {}",
                config_path.display()
            );
        }
    }

    let manager = new_shared(config.server);
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
        let mgr = self.manager.lock().unwrap();

        // Individual server controls
        for id in 0..self.server_count {
            let name = mgr.server_name(id).unwrap_or("Unknown").to_string();
            let running = mgr.is_running(id);
            let status = if running { " [running]" } else { " [stopped]" };

            let submenu = Submenu::new(format!("{}{}", name, status), true);

            let start_item = MenuItem::with_id(format!("start_{}", id), "Start", !running, None);
            let stop_item = MenuItem::with_id(format!("stop_{}", id), "Stop", running, None);
            let restart_item =
                MenuItem::with_id(format!("restart_{}", id), "Restart", running, None);

            submenu.append(&start_item).ok();
            submenu.append(&stop_item).ok();
            submenu.append(&restart_item).ok();

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
        self._tray.take();

        let menu = self.build_menu();
        let icon = create_icon();

        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Server Start")
            .with_icon(icon)
            .build()
            .expect("Failed to build tray icon");

        self._tray = Some(tray);
    }

    fn handle_menu_event(&mut self, event: MenuEvent, event_loop: &ActiveEventLoop) {
        let id = event.id().0.as_str();

        match id {
            MENU_ID_QUIT => {
                self.manager.lock().unwrap().stop_all();
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
                let _ = std::process::Command::new("cmd")
                    .args(["/c", "start", "", path.to_str().unwrap_or("")])
                    .spawn();
            }
            MENU_ID_RELOAD_CONFIG => {
                match Config::load() {
                    Ok(config) => {
                        self.manager.lock().unwrap().stop_all();
                        let new_manager = new_shared(config.server.clone());
                        self.server_count = config.server.len();
                        self.manager = new_manager;
                        self.rebuild_tray();
                    }
                    Err(e) => {
                        eprintln!("Failed to reload config: {}", e);
                    }
                }
            }
            other => {
                if let Some(id_str) = other.strip_prefix("start_") {
                    if let Ok(server_id) = id_str.parse::<usize>() {
                        if let Err(e) = self.manager.lock().unwrap().start(server_id) {
                            eprintln!("{}", e);
                        }
                        self.rebuild_tray();
                    }
                } else if let Some(id_str) = other.strip_prefix("stop_") {
                    if let Ok(server_id) = id_str.parse::<usize>() {
                        if let Err(e) = self.manager.lock().unwrap().stop(server_id) {
                            eprintln!("{}", e);
                        }
                        self.rebuild_tray();
                    }
                } else if let Some(id_str) = other.strip_prefix("restart_") {
                    if let Ok(server_id) = id_str.parse::<usize>() {
                        if let Err(e) = self.manager.lock().unwrap().restart(server_id) {
                            eprintln!("{}", e);
                        }
                        self.rebuild_tray();
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

/// Create a simple green circle icon programmatically
fn create_icon() -> Icon {
    let size = 32u32;
    let mut rgba = vec![0u8; (size * size * 4) as usize];

    let center = size as f32 / 2.0;
    let radius = center - 2.0;

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let dist = (dx * dx + dy * dy).sqrt();

            let idx = ((y * size + x) * 4) as usize;

            if dist <= radius {
                let brightness = 1.0 - (dist / radius) * 0.3;
                rgba[idx] = (50.0 * brightness) as u8;
                rgba[idx + 1] = (200.0 * brightness) as u8;
                rgba[idx + 2] = (80.0 * brightness) as u8;
                rgba[idx + 3] = 255;
            } else if dist <= radius + 1.0 {
                let alpha = (1.0 - (dist - radius)) * 255.0;
                rgba[idx] = 50;
                rgba[idx + 1] = 200;
                rgba[idx + 2] = 80;
                rgba[idx + 3] = alpha as u8;
            }
        }
    }

    Icon::from_rgba(rgba, size, size).expect("Failed to create icon")
}
