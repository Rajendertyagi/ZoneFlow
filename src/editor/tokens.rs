//! Shared UI layout tokens for the editor modules.
//!
//! Every spacing, margin, panel size, scroll height and control width used by
//! the editor lives here. These are pure aliases for the values that used to be
//! written inline at the call sites — centralising them means a value is changed
//! in exactly one place, with no visual or behavioural change.
//!
//! Import from a sibling module with `use super::tokens::*;` (or from `mod.rs`
//! with `use tokens::*;`).

/* --------------------------------------------------------------------------
 * SPACING SCALE (logical pixels)
 *
 * Used by `add_space` and the shadcn `layout::{row, wrap_row, row_top, vstack}`
 * helpers. Named by value where the semantic scale has no matching rung so a
 * call site always states the exact gap it wants.
 * ------------------------------------------------------------------------ */

/// Tightest gap (4px): label -> control, section sub-gaps.
pub(crate) const SPACE_XS: f32 = 4.0;
/// Small gap (6px): the default `add_space` and the tools row.
pub(crate) const SPACE_SM: f32 = 6.0;
/// 8px gap: the common row / vstack gap.
pub(crate) const SPACE_8: f32 = 8.0;
/// Medium gap (10px): top bar row and console vstack.
pub(crate) const SPACE_MD: f32 = 10.0;
/// 12px gap: card / section vstack.
pub(crate) const SPACE_12: f32 = 12.0;
/// 16px gap: outer vstack between cards.
pub(crate) const SPACE_16: f32 = 16.0;
/// 20px gap: wizard top row (options | preview).
pub(crate) const SPACE_20: f32 = 20.0;

/* --------------------------------------------------------------------------
 * PANEL GEOMETRY
 * ------------------------------------------------------------------------ */

/// Default width of the left (library / profiles) panel.
pub(crate) const PANEL_LEFT_W: f32 = 212.0;
/// Minimum width of the left panel.
pub(crate) const PANEL_LEFT_MIN_W: f32 = 170.0;
/// Maximum width of the left panel.
pub(crate) const PANEL_LEFT_MAX_W: f32 = 360.0;
/// Default width of the right (inspector) panel.
pub(crate) const PANEL_RIGHT_W: f32 = 290.0;
/// Minimum width of the right panel.
pub(crate) const PANEL_RIGHT_MIN_W: f32 = 230.0;
/// Maximum width of the right panel.
pub(crate) const PANEL_RIGHT_MAX_W: f32 = 440.0;

/* --------------------------------------------------------------------------
 * SCROLL AREA HEIGHTS
 * ------------------------------------------------------------------------ */

/// Max height of the profiles list in the left panel. Also the space reserved
/// at the bottom of the app library so the profiles panel is never pushed off.
pub(crate) const PROFILES_H: f32 = 150.0;
/// Minimum height of the app library scroll area.
pub(crate) const LIB_MIN_H: f32 = 80.0;
/// Max height of the assigned-apps strip under the canvas.
pub(crate) const APPS_STRIP_H: f32 = 105.0;
/// Max height of the console log in the inspector.
pub(crate) const CONSOLE_H: f32 = 96.0;
/// Max height of the app list in the picker dialog.
pub(crate) const DIALOG_LIST_H: f32 = 260.0;

/* --------------------------------------------------------------------------
 * CONTROL WIDTHS
 * ------------------------------------------------------------------------ */

/// Width of the resolution / close-behaviour selects.
pub(crate) const SELECT_W: f32 = 180.0;
/// Minimum width of the inspector's inline selects.
pub(crate) const SELECT_MIN_W: f32 = 80.0;

/* --------------------------------------------------------------------------
 * FRAME MARGINS
 *
 * `egui::Margin` stores i8 values, so these tuples are i8 as well.
 * ------------------------------------------------------------------------ */

/// Inner margin of the left side panel: (horizontal, vertical).
pub(crate) const MARGIN_PANEL_LEFT: (i8, i8) = (10, 8);
/// Inner margin of the right side panel: (horizontal, vertical).
pub(crate) const MARGIN_PANEL_RIGHT: (i8, i8) = (8, 8);
/// Inner margin of the inspector frame (all sides).
pub(crate) const MARGIN_INSPECTOR: i8 = 12;
/// Inner margin of the toast dialog: (horizontal, vertical).
pub(crate) const MARGIN_DIALOG: (i8, i8) = (14, 8);
