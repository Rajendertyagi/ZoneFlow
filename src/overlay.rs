use crate::engine::Rect as ZoneRect;
use eframe::egui;
use std::sync::{Arc, RwLock};

/// Shared state between the main app and the overlay viewport.
#[derive(Default)]
pub struct OverlayState {
    pub active: bool,
    pub zones: Vec<(i32, ZoneRect)>,
    pub hover_zone: i32,
    pub work: ZoneRect,
}

pub type SharedOverlay = Arc<RwLock<OverlayState>>;

/// Alphas used by [`show`] to preserve the original overlay look until the
/// editor wires `settings.overlay_inactive_alpha` / `overlay_active_alpha`
/// through [`show_with_alpha`].
const DEFAULT_INACTIVE_ALPHA: u8 = 90;
const DEFAULT_ACTIVE_ALPHA: u8 = 200;

/// Neutral fallback for a malformed zone-tint hex string. A bad config entry
/// can never panic; it simply renders as neutral grey.
const FALLBACK_TINT: egui::Color32 = egui::Color32::from_rgb(128, 128, 128);

/// Overlay zone border: translucent white, preserving the original overlay
/// look. Named rather than inline so the painter code stays colour-literal free.
const OVERLAY_BORDER: egui::Color32 = egui::Color32::from_rgba_premultiplied(255, 255, 255, 180);

/// Parse a `#RRGGBB` (or `RRGGBB`) hex string into an egui colour. Invalid
/// values fall back to [`FALLBACK_TINT`] so a bad config entry can never panic.
fn parse_tint(s: &str) -> egui::Color32 {
    let hex = s.trim().trim_start_matches('#');
    if hex.len() == 6 && hex.is_ascii() {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
        ) {
            return egui::Color32::from_rgb(r, g, b);
        }
    }
    FALLBACK_TINT
}

/// Build the drawing palette from config tint strings. Never returns an empty
/// slice, so indexing it in the draw loop cannot divide by zero. When no tints
/// are supplied, the config's default zone tints are used (single source of
/// truth) instead of a hardcoded palette.
fn palette_from<S: AsRef<str>>(tints: &[S]) -> Vec<egui::Color32> {
    if tints.is_empty() {
        let defaults = crate::config::default_config().settings.zone_tints;
        return defaults.iter().map(|s| parse_tint(s)).collect();
    }
    tints.iter().map(|s| parse_tint(s.as_ref())).collect()
}

/// Draw the overlay viewport (called each frame while dragging) using the
/// config's default tints and alphas. Kept for compatibility; the editor always
/// calls [`show_with_theme`] with the live config, so this is currently unused.
#[allow(dead_code)]
pub fn show(ctx: &egui::Context, state: &SharedOverlay) {
    let tints = crate::config::default_config().settings.zone_tints;
    show_with_theme(
        ctx,
        state,
        &tints,
        DEFAULT_INACTIVE_ALPHA,
        DEFAULT_ACTIVE_ALPHA,
    );
}

/// Like [`show`], but with configurable overlay alphas.
///
/// Exposed for the editor to pass `settings.overlay_inactive_alpha` /
/// `overlay_active_alpha` once that call site is wired; unused until then.
#[allow(dead_code)]
pub fn show_with_alpha(ctx: &egui::Context, state: &SharedOverlay, inactive: u8, active: u8) {
    let tints = crate::config::default_config().settings.zone_tints;
    show_with_theme(ctx, state, &tints, inactive, active);
}

/// Draw the overlay with a palette derived from `tints` (hex strings from
/// `settings.zone_tints`) and configurable `inactive` / `active` alphas.
pub fn show_with_theme(
    ctx: &egui::Context,
    state: &SharedOverlay,
    tints: &[String],
    inactive: u8,
    active: u8,
) {
    let colors = palette_from(tints);
    draw(ctx, state, colors, inactive, active);
}

fn draw(
    ctx: &egui::Context,
    state: &SharedOverlay,
    colors: Vec<egui::Color32>,
    inactive: u8,
    active: u8,
) {
    if colors.is_empty() {
        return;
    }
    // Snapshot under the lock and release it before opening the viewport. A
    // poisoned lock simply skips this frame instead of panicking.
    let (zones, hover, work) = {
        let Ok(s) = state.read() else {
            return;
        };
        if !s.active {
            return;
        }
        (s.zones.clone(), s.hover_zone, s.work)
    };

    let id = egui::ViewportId::from_hash_of("screenlayout_overlay");
    let builder = egui::ViewportBuilder::default()
        .with_transparent(true)
        .with_always_on_top()
        .with_mouse_passthrough(true)
        .with_decorations(false)
        .with_taskbar(false)
        .with_position(egui::pos2(work.x as f32, work.y as f32))
        .with_inner_size(egui::vec2(work.w as f32, work.h as f32));

    let state2 = state.clone();
    ctx.show_viewport_deferred(id, builder, move |ui, _class| {
        // Re-check inside the callback: the overlay may have been deactivated
        // between the snapshot above and this frame. A poisoned lock is ignored.
        let Ok(st) = state2.read() else {
            return;
        };
        if !st.active {
            return;
        }
        let rect = ui.max_rect();
        let painter = ui.painter();
        for (idx, (zid, z)) in zones.iter().enumerate() {
            let zr = egui::Rect::from_min_max(
                rect.min
                    + egui::vec2(
                        (z.x - work.x) as f32 / work.w as f32 * rect.width(),
                        (z.y - work.y) as f32 / work.h as f32 * rect.height(),
                    ),
                rect.min
                    + egui::vec2(
                        (z.x - work.x + z.w) as f32 / work.w as f32 * rect.width(),
                        (z.y - work.y + z.h) as f32 / work.h as f32 * rect.height(),
                    ),
            );
            let base = colors[idx % colors.len()];
            let color = if *zid == hover {
                egui::Color32::from_rgba_premultiplied(base.r(), base.g(), base.b(), active)
            } else {
                egui::Color32::from_rgba_premultiplied(base.r(), base.g(), base.b(), inactive)
            };
            painter.rect_filled(zr, 6.0, color);
            painter.rect_stroke(
                zr,
                6.0,
                egui::Stroke::new(2.0, OVERLAY_BORDER),
                egui::StrokeKind::Inside,
            );
            painter.text(
                zr.center(),
                egui::Align2::CENTER_CENTER,
                zid.to_string(),
                egui::FontId::proportional(48.0),
                egui::Color32::WHITE,
            );
        }
    });
}
