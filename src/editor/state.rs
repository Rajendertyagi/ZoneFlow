//! Editor state types and data structures.
//!
//! Pure data holders (no UI rendering). Kept `pub(super)` so `mod.rs` and the
//! sibling editor modules can use them.

use eframe::egui;

/* ============================================================================
 * RESOLUTION PRESETS (§2d item 11) — values live in config Settings.
 * ========================================================================== */

/// UI selection key for the resolution selector. The actual preset values
/// (label / width / work height) are user-editable in `Settings::resolutions`;
/// these methods read them from the config snapshot loaded at startup.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum ResKey {
    FourK,
    Qhd,
    Fhd,
}

impl ResKey {
    pub(super) fn idx(self) -> usize {
        match self {
            Self::FourK => 0,
            Self::Qhd => 1,
            Self::Fhd => 2,
        }
    }
    fn preset(self) -> Option<&'static crate::config::ResolutionPreset> {
        crate::config::resolutions().get(self.idx())
    }
    pub(super) fn label(self) -> &'static str {
        self.preset().map(|p| p.label.as_str()).unwrap_or("Unknown")
    }
    pub(super) fn width(self) -> i32 {
        self.preset().map(|p| p.width as i32).unwrap_or(1920)
    }
    pub(super) fn work_height(self) -> i32 {
        self.preset().map(|p| p.work_height as i32).unwrap_or(1032)
    }
    pub(super) fn all() -> [Self; 3] {
        [Self::FourK, Self::Qhd, Self::Fhd]
    }
}

/* ============================================================================
 * TEMPLATES / PRESETS / PLACEHOLDER APP LISTS — all data-driven.
 * ========================================================================== */

pub(super) type RectSpec = (f32, f32, f32, f32);

pub(super) const TEMPLATES: [(&str, &[RectSpec]); 5] = [
    ("Duo", &[(0.0, 0.0, 50.0, 100.0), (50.0, 0.0, 50.0, 100.0)]),
    (
        "Debug",
        &[
            (0.0, 0.0, 60.0, 100.0),
            (60.0, 0.0, 40.0, 55.0),
            (60.0, 55.0, 40.0, 45.0),
        ],
    ),
    (
        "3 Columns",
        &[
            (0.0, 0.0, 33.333, 100.0),
            (33.333, 0.0, 33.334, 100.0),
            (66.667, 0.0, 33.333, 100.0),
        ],
    ),
    ("Focus", &[(0.0, 0.0, 100.0, 100.0)]),
    ("Empty", &[]),
];

pub(super) const PRESETS: [(&str, &[RectSpec]); 7] = [
    ("Left half", &[(0.0, 0.0, 50.0, 100.0)]),
    ("Right half", &[(50.0, 0.0, 50.0, 100.0)]),
    ("Top half", &[(0.0, 0.0, 100.0, 50.0)]),
    ("Bottom half", &[(0.0, 50.0, 100.0, 50.0)]),
    ("Left third", &[(0.0, 0.0, 33.333, 100.0)]),
    ("Top-left quarter", &[(0.0, 0.0, 50.0, 50.0)]),
    ("Center / Focus", &[(20.0, 15.0, 60.0, 70.0)]),
];

/* ============================================================================
 * APP ENUMERATION (§2a / §2.2) — real system data, no baked-in app lists.
 * ========================================================================== */

/// A discovered app. Reuses the drag payload so the library and picker share
/// one data shape: label + best-effort exe name + full path when known.
pub(super) type AppEntry = DragApp;

/* ============================================================================
 * STATE TYPES
 * ========================================================================== */

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum InspectorTab {
    #[default]
    Process,
    Browser,
    Automation,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum RailTab {
    #[default]
    Apps,
    Profiles,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum View {
    Canvas,
    Hotkeys,
    Settings,
}

#[derive(Clone, PartialEq, Eq)]
pub(super) struct Snapshot {
    pub(super) cfg_json: String,
    pub(super) selected_zone: i32,
    pub(super) res: ResKey,
}

#[derive(Clone)]
pub(super) struct Boundary {
    pub(super) vertical: bool,
    pub(super) pos: f32,
    pub(super) span_min: f32,
    pub(super) span_max: f32,
    pub(super) before: Vec<i32>,
    pub(super) after: Vec<i32>,
}

pub(super) struct BoundaryDrag {
    pub(super) boundary: Boundary,
    pub(super) start_pointer: f32,
    pub(super) start_pos: f32,
    pub(super) before_orig: Vec<(i32, f32, f32)>, // (id, coord, size) along the axis
    pub(super) after_orig: Vec<(i32, f32, f32)>,
}

pub(super) struct ZoneDrag {
    pub(super) id: i32,
    pub(super) grab: egui::Vec2,
    pub(super) orig: (f32, f32, f32, f32),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum WizardOption {
    Template,
    Grid,
    Single,
    Draw,
}

pub(super) struct WizardState {
    pub(super) option: WizardOption,
    pub(super) template: usize,
    pub(super) cols: u32,
    pub(super) rows: u32,
    pub(super) preset: usize,
}

impl Default for WizardState {
    fn default() -> Self {
        Self {
            option: WizardOption::Template,
            template: 0,
            cols: 3,
            rows: 2,
            preset: 0,
        }
    }
}

impl WizardState {
    pub(super) fn zones(&self) -> Vec<RectSpec> {
        match self.option {
            WizardOption::Template => TEMPLATES
                .get(self.template)
                .map(|(_, z)| z.to_vec())
                .unwrap_or_default(),
            WizardOption::Grid => {
                let mut out = Vec::new();
                let c = self.cols.max(1);
                let r = self.rows.max(1);
                for ry in 0..r {
                    for cx in 0..c {
                        out.push((
                            cx as f32 / c as f32 * 100.0,
                            ry as f32 / r as f32 * 100.0,
                            100.0 / c as f32,
                            100.0 / r as f32,
                        ));
                    }
                }
                out
            }
            WizardOption::Single => PRESETS
                .get(self.preset)
                .map(|(_, z)| z.to_vec())
                .unwrap_or_default(),
            WizardOption::Draw => Vec::new(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum PickerTab {
    Installed,
    Running,
    Browse,
}

pub(super) struct PickerState {
    pub(super) tab: PickerTab,
    pub(super) search: String,
    pub(super) path: String,
}

impl Default for PickerState {
    fn default() -> Self {
        Self {
            tab: PickerTab::Installed,
            search: String::new(),
            path: String::new(),
        }
    }
}

#[derive(Default)]
pub(super) struct EditBuffers {
    pub(super) label: String,
    pub(super) exe: String,
    pub(super) path: String,
    pub(super) url: String,
    pub(super) title: String,
    pub(super) trigger: String,
    pub(super) macro_text: String,
    pub(super) delay: f32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum StrField {
    Label,
    Exe,
    Path,
    Url,
    Title,
    Trigger,
    Macro,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum LogClass {
    Normal,
    Good,
    Accent,
    Bad,
}

pub(super) struct LogLine {
    pub(super) class: LogClass,
    pub(super) text: String,
}

pub(super) enum TextDialogKind {
    RenameZone(i32),
    NewProfile,
    RenameProfile,
}

pub(super) struct TextDialogState {
    pub(super) kind: TextDialogKind,
    pub(super) value: String,
}

#[derive(Clone)]
pub(super) struct DragApp {
    pub(super) label: String,
    pub(super) exe: String,
    pub(super) path: String,
}

#[derive(Clone, Copy)]
pub(super) struct DragZoneApp {
    pub(super) id: i32,
}
