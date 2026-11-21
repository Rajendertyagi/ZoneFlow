use crate::config::AppConfig;
use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager};
use std::collections::HashMap;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

#[derive(Clone, Debug)]
pub enum Action {
    LaunchApp(usize),
    Retile,
    /// Retained for compatibility with existing `handle_action` arms; the
    /// arrow-key bindings now emit [`Action::CycleAxis`] instead (§2c).
    #[allow(dead_code)]
    CycleNext,
    #[allow(dead_code)]
    CyclePrev,
    /// Move the active window one zone along `(dx, dy)` in the zone grid
    /// (§2c): left `(-1,0)`, right `(1,0)`, up `(0,-1)`, down `(0,1)`.
    CycleAxis(i32, i32),
    Maximize,
    Restore,
    Reload,
    ShowSettings,
    Exit,
}

pub struct System {
    // Keep these alive — dropping them unregisters hotkeys / removes tray icon
    manager: GlobalHotKeyManager,
    _tray: TrayIcon,
    hotkey_map: HashMap<u32, Action>,
    /// Successfully registered hotkeys, tracked so `register_from_config` can
    /// unregister them before applying a new binding set (config reload).
    registered: Vec<HotKey>,
    /// Bindings that could not be parsed or registered (shown in the UI log).
    pub failures: Vec<String>,
}

impl System {
    /// Constructs the system, loading hotkey bindings from the on-disk config.
    ///
    /// Kept with the original zero-argument signature for call-site compatibility
    /// (`editor.rs::with_system`). Use [`System::with_config`] when a config is
    /// already in memory.
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let cfg = crate::config::load_config();
        Self::with_config(&cfg)
    }

    /// Constructs the tray icon and hotkey manager, registering hotkeys from
    /// `cfg.hotkeys` (§2c). Tray creation is fatal; individual hotkey failures
    /// are logged and recorded in [`System::failures`] but never abort startup.
    pub fn with_config(cfg: &AppConfig) -> Result<Self, Box<dyn std::error::Error>> {
        // Tray icon first — always available even if hotkeys fail.
        let menu = Menu::new();
        let i_settings = MenuItem::new("Settings", true, None);
        let i_retile = MenuItem::new("Re-tile now", true, None);
        let i_reload = MenuItem::new("Reload config", true, None);
        let i_exit = MenuItem::new("Exit", true, None);
        menu.append(&i_settings)?;
        menu.append(&i_retile)?;
        menu.append(&i_reload)?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&i_exit)?;

        let icon = make_icon()?;
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("ScreenLayout")
            .with_icon(icon)
            .build()?;

        let manager = GlobalHotKeyManager::new()?;

        // Apply the config's autostart preference at startup so the registry
        // value stays in sync with the (authoritative) config file.
        crate::autostart::set_auto_start(cfg.auto_start);

        let mut system = Self {
            manager,
            _tray: tray,
            hotkey_map: HashMap::new(),
            registered: Vec::new(),
            failures: Vec::new(),
        };
        system.register_from_config(cfg)?;
        Ok(system)
    }

    /// (Re-)register global hotkeys from a config's `hotkeys` map.
    ///
    /// Binding strings use the AHK-style format from `default_hotkeys()`, e.g.
    /// `"Ctrl+Shift+1"`, `"Ctrl+Alt+Left"`, `"Win+Up"`. `"Win"` is normalised to
    /// `"Meta"` because `global-hotkey` does not recognise the former. Unknown
    /// action names, unparseable bindings and registration conflicts are logged
    /// with `tracing::warn!` and recorded in [`System::failures`]; processing
    /// continues so one bad binding never prevents the others from registering.
    pub fn register_from_config(&mut self, cfg: &AppConfig) -> Result<(), String> {
        // Drop any previous registrations first so this can be called again
        // after a config reload without leaving stale hotkeys behind.
        for hk in self.registered.drain(..) {
            if let Err(e) = self.manager.unregister(hk) {
                tracing::warn!("Failed to unregister hotkey {}: {}", hk, e);
            }
        }
        self.hotkey_map.clear();
        self.failures.clear();

        for (name, binding) in &cfg.hotkeys {
            let Some(action) = action_for_name(name) else {
                tracing::warn!("Unknown hotkey action '{}' in config; skipping", name);
                continue;
            };

            let normalized = normalize_binding(binding);
            let hk = match normalized.parse::<HotKey>() {
                Ok(hk) => hk,
                Err(e) => {
                    tracing::warn!(
                        "Invalid hotkey binding '{}' for '{}': {}; skipping",
                        binding,
                        name,
                        e
                    );
                    self.failures.push(format!("{} ({})", name, binding));
                    continue;
                }
            };

            let id = hk.id();
            match self.manager.register(hk) {
                Ok(()) => {
                    self.hotkey_map.insert(id, action);
                    self.registered.push(hk);
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to register hotkey '{}' for '{}': {}; skipping",
                        binding,
                        name,
                        e
                    );
                    self.failures.push(format!("{} ({})", name, binding));
                }
            }
        }

        Ok(())
    }

    pub fn poll(&self) -> Vec<Action> {
        let mut actions = Vec::new();

        while let Ok(ev) = GlobalHotKeyEvent::receiver().try_recv() {
            if let Some(action) = self.hotkey_map.get(&ev.id) {
                actions.push(action.clone());
            }
        }

        while let Ok(ev) = MenuEvent::receiver().try_recv() {
            match ev.id.0.as_str() {
                "Settings" => actions.push(Action::ShowSettings),
                "Re-tile now" => actions.push(Action::Retile),
                "Reload config" => actions.push(Action::Reload),
                "Exit" => actions.push(Action::Exit),
                _ => {}
            }
        }

        actions
    }
}

/// Map a config hotkey name (the keys of `AppConfig.hotkeys`) to an [`Action`].
/// `profile_N` maps to `LaunchApp(N-1)` for N in 1..=9.
fn action_for_name(name: &str) -> Option<Action> {
    if let Some(n) = name.strip_prefix("profile_") {
        if let Ok(n) = n.parse::<usize>() {
            if (1..=9).contains(&n) {
                return Some(Action::LaunchApp(n - 1));
            }
        }
    }
    match name {
        "clear_layout" => Some(Action::Retile),
        // §2c: the arrow keys move a zone in their own direction, not linearly
        // through the zone list. CycleNext/CyclePrev remain for compatibility.
        "move_left" => Some(Action::CycleAxis(-1, 0)),
        "move_right" => Some(Action::CycleAxis(1, 0)),
        "move_up" => Some(Action::CycleAxis(0, -1)),
        "move_down" => Some(Action::CycleAxis(0, 1)),
        "maximize" => Some(Action::Maximize),
        "restore" => Some(Action::Restore),
        "reload" => Some(Action::Reload),
        _ => None,
    }
}

/// Normalise an AHK-style binding for `global-hotkey`'s parser.
///
/// `global-hotkey` accepts `Ctrl`, `Alt`, `Shift`, `Meta`/`Cmd`/`Super` and
/// key names such as `1`, `Left`, `Up`, `R`, but not `Win`. Map `Win`/`Windows`
/// to `Meta` so the config defaults (`"Win+Up"`, `"Win+Down"`) parse correctly.
fn normalize_binding(binding: &str) -> String {
    let mut out = String::with_capacity(binding.len());
    for token in binding.split('+') {
        let t = token.trim();
        let mapped = if t.eq_ignore_ascii_case("win") || t.eq_ignore_ascii_case("windows") {
            "Meta"
        } else {
            t
        };
        if !out.is_empty() {
            out.push('+');
        }
        out.push_str(mapped);
    }
    out
}

/// Display-change debounce in milliseconds, sourced from config (§2.2).
///
/// The editor reads `cfg.settings.display_debounce_ms` directly for its
/// work-area polling loop; this helper is kept as a convenience accessor.
#[allow(dead_code)]
pub fn display_debounce_ms(cfg: &AppConfig) -> u64 {
    cfg.settings.display_debounce_ms
}

/// Tray icon brand colours. These are the fixed app-icon identity (a raster
/// asset), not UI theme colours, so they are intentionally not sourced from the
/// shadcn palette or config. Named constants keep the literals out of the
/// drawing loop.
const TRAY_BORDER_RGB: (u8, u8, u8) = (120, 180, 240);
const TRAY_FILL_RGB: (u8, u8, u8) = (31, 78, 121);

fn make_icon() -> Result<Icon, Box<dyn std::error::Error>> {
    let size = 32u32;
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let border = x < 2 || y < 2 || x >= size - 2 || y >= size - 2;
            let (r, g, b) = if border {
                TRAY_BORDER_RGB
            } else {
                TRAY_FILL_RGB
            };
            rgba.extend_from_slice(&[r, g, b, 255]);
        }
    }
    Ok(Icon::from_rgba(rgba, size, size)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::default_hotkeys;

    #[test]
    fn all_default_bindings_have_actions_and_parse() {
        for (name, binding) in default_hotkeys() {
            assert!(action_for_name(&name).is_some(), "no action for '{}'", name);
            let normalized = normalize_binding(&binding);
            assert!(
                normalized.parse::<HotKey>().is_ok(),
                "binding '{}' (normalised '{}') for '{}' failed to parse",
                binding,
                normalized,
                name
            );
        }
    }

    #[test]
    fn win_is_normalised_to_meta() {
        assert_eq!(normalize_binding("Win+Up"), "Meta+Up");
        assert_eq!(normalize_binding("Ctrl+Shift+1"), "Ctrl+Shift+1");
    }
}
