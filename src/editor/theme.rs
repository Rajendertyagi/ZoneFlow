//! Canvas palette / theme token helpers.
//!
/* ============================================================================
 * CANVAS PALETTE — token colors for the custom Painter canvas only.
 *
 * The shadcn theme is applied once in main.rs via `apply_with_fonts`. This
 * struct READS that theme's palette (it never writes to the egui Context) and
 * combines it with the config's zone tints. All shadcn widgets read the live
 * theme themselves; only the hand-painted monitor canvas needs these colors.
 * ========================================================================== */

use crate::config::AppConfig;
use eframe::egui;

/// Amber fallback used when a configured zone-tint hex string is malformed.
/// Mirrors the default first zone tint (`#F5B544`) so a bad config entry still
/// renders in the app's accent colour rather than a bare literal at the call site.
const FALLBACK_TINT_RGB: (u8, u8, u8) = (245, 181, 68);

/// Success / "Tiled" status green. shadcn's Zinc+Amber palette has no `success`
/// role, and this preset uses the default Neutral chart theme (so `chart[2]` is
/// grey, not green). The green is kept as a named token — it also matches the
/// default `zone_tints[2]` (`#5BD6A0`) — to avoid a visual regression while
/// keeping colour literals out of the painter code.
const SUCCESS_GREEN_RGB: (u8, u8, u8) = (91, 214, 160);

fn hex_color(hex: &str, alpha: u8) -> egui::Color32 {
    let h = hex.trim().trim_start_matches('#');
    if h.len() >= 6 {
        let r = u8::from_str_radix(&h[0..2], 16).unwrap_or(255);
        let g = u8::from_str_radix(&h[2..4], 16).unwrap_or(255);
        let b = u8::from_str_radix(&h[4..6], 16).unwrap_or(255);
        egui::Color32::from_rgba_unmultiplied(r, g, b, alpha)
    } else {
        egui::Color32::from_rgba_unmultiplied(
            FALLBACK_TINT_RGB.0,
            FALLBACK_TINT_RGB.1,
            FALLBACK_TINT_RGB.2,
            alpha,
        )
    }
}

/// Same RGB as `c`, with a new alpha.
fn with_alpha(c: egui::Color32, alpha: u8) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha)
}

#[derive(Clone)]
pub(super) struct CanvasTheme {
    pub(super) bg: egui::Color32,
    pub(super) surface2: egui::Color32,
    pub(super) raised: egui::Color32,
    pub(super) line: egui::Color32,
    pub(super) line2: egui::Color32,
    pub(super) ink: egui::Color32,
    pub(super) muted: egui::Color32,
    pub(super) faint: egui::Color32,
    pub(super) accent: egui::Color32,
    pub(super) accent2: egui::Color32,
    pub(super) accent_bg: egui::Color32,
    pub(super) accent_line: egui::Color32,
    pub(super) good: egui::Color32,
    pub(super) good_bg: egui::Color32,
    pub(super) r_sm: f32,
    pub(super) r_md: f32,
    pub(super) r_lg: f32,
    pub(super) r_xl: f32,
    pub(super) f_small: f32,
    pub(super) tints: Vec<egui::Color32>,
}

impl CanvasTheme {
    pub(super) fn new(ctx: &egui::Context, cfg: &AppConfig) -> Self {
        use shadcn_egui::Theme;
        let p = *Theme::current(ctx).palette();
        let tints: Vec<egui::Color32> = if cfg.settings.zone_tints.is_empty() {
            vec![egui::Color32::from_rgba_unmultiplied(
                FALLBACK_TINT_RGB.0,
                FALLBACK_TINT_RGB.1,
                FALLBACK_TINT_RGB.2,
                255,
            )]
        } else {
            cfg.settings
                .zone_tints
                .iter()
                .map(|h| hex_color(h, 255))
                .collect()
        };
        let accent = tints[0];
        let good = egui::Color32::from_rgb(
            SUCCESS_GREEN_RGB.0,
            SUCCESS_GREEN_RGB.1,
            SUCCESS_GREEN_RGB.2,
        );
        Self {
            bg: p.background,
            surface2: p.muted,
            raised: p.secondary,
            line: p.border,
            line2: p.border,
            ink: p.foreground,
            muted: p.muted_foreground,
            faint: with_alpha(p.muted_foreground, 150),
            accent,
            accent2: accent,
            accent_bg: with_alpha(accent, 40),
            accent_line: with_alpha(accent, 115),
            good,
            good_bg: with_alpha(good, 40),
            r_sm: 6.0,
            r_md: 9.0,
            r_lg: 12.0,
            r_xl: 16.0,
            f_small: 10.5,
            tints,
        }
    }

    pub(super) fn tint(&self, idx: usize) -> egui::Color32 {
        self.tints[idx % self.tints.len()]
    }

    pub(super) fn tint_bg(&self, idx: usize) -> egui::Color32 {
        with_alpha(self.tint(idx), 36)
    }

    pub(super) fn tint_line(&self, idx: usize) -> egui::Color32 {
        with_alpha(self.tint(idx), 82)
    }
}
