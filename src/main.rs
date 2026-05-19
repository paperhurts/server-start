// Hide console window on Windows release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(windows)]
mod autostart;
mod config;
mod errors;
mod process;

use config::{Config, GroupConfig, OutputMode, ServerConfig};
use process::{new_shared, SharedProcessManager};
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
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
#[cfg(windows)]
const MENU_ID_AUTOSTART: &str = "autostart";
const MENU_ID_QUIT: &str = "quit";

fn warn_unmatched_group_servers(groups: &[GroupConfig], servers: &[ServerConfig]) {
    let server_names: Vec<&str> = servers.iter().map(|s| s.name.as_str()).collect();
    let mut warnings = Vec::new();

    for group in groups {
        let unmatched: Vec<&str> = group
            .servers
            .iter()
            .filter(|name| !server_names.contains(&name.as_str()))
            .map(|s| s.as_str())
            .collect();

        if !unmatched.is_empty() {
            warnings.push(format!(
                "Group \"{}\" references unknown servers: {}",
                group.name,
                unmatched.join(", ")
            ));
        }
    }

    if !warnings.is_empty() {
        errors::show_error("Group Config Warning", &warnings.join("\n\n"));
    }
}

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
                group: Vec::new(),
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

# ─── Groups: start/stop/restart multiple servers at once ───
# [[group]]
# name = "Reader"
# servers = ["Frontend", "Backend API"]

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

    warn_unmatched_group_servers(&config.group, &config.server);

    let manager = new_shared(config.server, config.output);
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let mut app = App::new(manager, config.group);

    event_loop.run_app(&mut app).expect("Event loop failed");
}

struct App {
    manager: SharedProcessManager,
    _tray: Option<TrayIcon>,
    server_count: usize,
    groups: Vec<GroupConfig>,
}

impl App {
    fn new(manager: SharedProcessManager, groups: Vec<GroupConfig>) -> Self {
        let server_count = manager.lock().unwrap().server_count();
        App {
            manager,
            _tray: None,
            server_count,
            groups,
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

        // Group controls
        if !self.groups.is_empty() {
            for (gidx, group) in self.groups.iter().enumerate() {
                let resolved: Vec<_> = group.servers.iter()
                    .filter(|name| mgr.server_id_by_name(name).is_some())
                    .collect();
                let total = resolved.len();
                let running = resolved.iter().filter(|name| {
                    mgr.server_id_by_name(name)
                        .map(|id| mgr.is_running(id))
                        .unwrap_or(false)
                }).count();

                let label = format!("{} [{}/{}]", group.name, running, total);
                let submenu = Submenu::new(label, true);

                let start = MenuItem::with_id(
                    format!("grp_start_{}", gidx), "Start Group", true, None,
                );
                let stop = MenuItem::with_id(
                    format!("grp_stop_{}", gidx), "Stop Group", true, None,
                );
                let restart = MenuItem::with_id(
                    format!("grp_restart_{}", gidx), "Restart Group", true, None,
                );

                submenu.append(&start).ok();
                submenu.append(&stop).ok();
                submenu.append(&restart).ok();
                menu.append(&submenu).ok();
            }

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

        // Start with Windows (HKCU Run-key toggle). Windows-only — the
        // registry is the source of truth, so we re-query its state every
        // time the menu is rebuilt.
        #[cfg(windows)]
        {
            let autostart_item = CheckMenuItem::with_id(
                MENU_ID_AUTOSTART,
                "Start with Windows",
                true,
                autostart::is_enabled(),
                None,
            );
            menu.append(&autostart_item).ok();
            menu.append(&PredefinedMenuItem::separator()).ok();
        }

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
            #[cfg(windows)]
            MENU_ID_AUTOSTART => {
                // Toggle based on the registry's current state, not on the
                // CheckMenuItem's checked flag — the menu reflects state at
                // build time, but the user could have changed it externally
                // (Task Manager > Startup, regedit) between rebuilds.
                let result = if autostart::is_enabled() {
                    autostart::disable()
                } else {
                    autostart::enable()
                };
                if let Err(e) = result {
                    errors::show_error("Start with Windows", &e);
                }
                self.rebuild_tray();
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
                        warn_unmatched_group_servers(&config.group, &config.server);
                        self.manager
                            .lock()
                            .unwrap()
                            .reload(config.server.clone(), config.output.clone());
                        self.server_count = config.server.len();
                        self.groups = config.group;
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
                } else if let Some(id_str) = other.strip_prefix("grp_start_") {
                    if let Ok(gidx) = id_str.parse::<usize>() {
                        if let Some(group) = self.groups.get(gidx) {
                            self.manager.lock().unwrap().start_group(&group.servers);
                            self.rebuild_tray();
                        }
                    }
                } else if let Some(id_str) = other.strip_prefix("grp_stop_") {
                    if let Ok(gidx) = id_str.parse::<usize>() {
                        if let Some(group) = self.groups.get(gidx) {
                            self.manager.lock().unwrap().stop_group(&group.servers);
                            self.rebuild_tray();
                        }
                    }
                } else if let Some(id_str) = other.strip_prefix("grp_restart_") {
                    if let Ok(gidx) = id_str.parse::<usize>() {
                        if let Some(group) = self.groups.get(gidx) {
                            self.manager.lock().unwrap().restart_group(&group.servers);
                            self.rebuild_tray();
                        }
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

/// Raw RGBA bytes for the tray icon, generated by `build.rs` from
/// `assets/server-start-icon.png` at `TRAY_ICON_SIZE` × `TRAY_ICON_SIZE`.
const TRAY_ICON_RGBA: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/tray-icon.rgba"));

/// Side length of the tray-icon RGBA blob. The const assertion below makes
/// this a compile-time error to drift out of sync with build.rs.
const TRAY_ICON_SIZE: u32 = 256;

const _: () = assert!(
    TRAY_ICON_RGBA.len() == (TRAY_ICON_SIZE as usize) * (TRAY_ICON_SIZE as usize) * 4,
    "TRAY_ICON_SIZE does not match the embedded RGBA blob; update build.rs::TRAY_SIZE or this constant",
);

fn create_icon() -> Icon {
    Icon::from_rgba(TRAY_ICON_RGBA.to_vec(), TRAY_ICON_SIZE, TRAY_ICON_SIZE)
        .expect("Failed to create tray icon")
}
