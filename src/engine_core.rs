//! Pure, Win32-independent geometry for the tiling engine.
//!
//! This module holds the *only* platform-independent parts of the engine:
//! zone math, shared-boundary detection, snap guides, percent/pixel conversion
//! and the DWM border-compensation arithmetic. It has no dependency on the
//! `windows` crate and can be compiled and unit-tested on any platform.
//!
//! The remaining engine functions (window enumeration, movement, input, hotkey
//! polling) are Win32-specific and stay in `engine.rs`, which calls into these
//! helpers where useful. Do not move Win32 calls into this module.
//!
//! Several helpers are library-style APIs intended for the UI/API layer, so
//! they are not all called from the binary yet. Allow dead_code here rather
//! than dropping tested logic.

#![allow(dead_code)]

use crate::config::{Profile, Zone};

/// Default snap guides from the design spec (§2d): 25 / 50 / 75 percent.
pub const DEFAULT_SNAP_POINTS: [f32; 3] = [25.0, 50.0, 75.0];

/// An axis-aligned rectangle in screen pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// Convert a percentage (0..100) along an axis of `len` pixels into pixels.
pub fn percent_to_pixel(pct: f32, len: i32) -> i32 {
    (pct / 100.0 * len as f32).round() as i32
}

/// Convert a pixel offset along an axis of `len` pixels back into a percentage.
/// Returns 0.0 for a zero-length axis instead of dividing by zero.
pub fn pixel_to_percent(px: i32, len: i32) -> f32 {
    if len == 0 {
        0.0
    } else {
        px as f32 / len as f32 * 100.0
    }
}

/// Convert a zone's percent rectangle into absolute pixels, applying the gap.
pub fn zone_pixels(zone: &Zone, area: Rect, gap: i32) -> Rect {
    let half = (gap as f32 / 2.0).round() as i32;
    let raw_x = area.x + percent_to_pixel(zone.x, area.w);
    let raw_y = area.y + percent_to_pixel(zone.y, area.h);
    let raw_r = area.x + percent_to_pixel(zone.x + zone.w, area.w);
    let raw_b = area.y + percent_to_pixel(zone.y + zone.h, area.h);

    let x = raw_x + if zone.x > 0.0 { half } else { gap };
    let y = raw_y + if zone.y > 0.0 { half } else { gap };
    let r = raw_r - if zone.x + zone.w < 100.0 { half } else { gap };
    let b = raw_b - if zone.y + zone.h < 100.0 { half } else { gap };
    Rect {
        x,
        y,
        w: (r - x).max(100),
        h: (b - y).max(100),
    }
}

/// Compute pixel rectangles for every zone in a profile, preserving zone ids.
pub fn compute_zones(profile: &Profile, area: Rect, gap: i32) -> Vec<(i32, Rect)> {
    profile
        .zones
        .iter()
        .map(|z| (z.id, zone_pixels(z, area, gap)))
        .collect()
}

/// Look up a computed zone rectangle by id.
pub fn zone_by_id(zones: &[(i32, Rect)], id: i32) -> Option<Rect> {
    zones.iter().find(|(zid, _)| *zid == id).map(|(_, r)| *r)
}

/// Return the id of the computed zone containing the point `(x, y)`, if any.
pub fn zone_at(zones: &[(i32, Rect)], x: i32, y: i32) -> Option<i32> {
    zones
        .iter()
        .find(|(_, z)| x >= z.x && x < z.x + z.w && y >= z.y && y < z.y + z.h)
        .map(|(id, _)| *id)
}

/// Index of the zone one step away from `from_idx` along `(dx, dy)`.
///
/// `zones` are percent rectangles (see [`Zone`]). The current zone's centre is
/// shifted by one zone-width / zone-height along the requested axis to form a
/// target point; among the zones whose centre lies strictly in that direction,
/// the one nearest the target wins. This makes Ctrl+Alt+Up/Down move a row and
/// Ctrl+Alt+Left/Right move a column (§2c) instead of cycling linearly.
///
/// Returns `None` at the edge of the layout or when the direction is `(0, 0)`.
pub fn nearest_zone_in_direction(
    zones: &[Zone],
    from_idx: usize,
    dx: i32,
    dy: i32,
) -> Option<usize> {
    let cur = zones.get(from_idx)?;
    let cur_cx = cur.x + cur.w / 2.0;
    let cur_cy = cur.y + cur.h / 2.0;
    let target_cx = cur_cx + dx as f32 * cur.w;
    let target_cy = cur_cy + dy as f32 * cur.h;

    let mut best: Option<(usize, f32)> = None;
    for (i, z) in zones.iter().enumerate() {
        if i == from_idx {
            continue;
        }
        let zcx = z.x + z.w / 2.0;
        let zcy = z.y + z.h / 2.0;
        // The candidate must lie strictly in the requested direction on the
        // movement axis; otherwise we could wrap to the opposite edge.
        let along = if dx != 0 {
            (zcx - cur_cx) * dx as f32
        } else {
            (zcy - cur_cy) * dy as f32
        };
        if along <= f32::EPSILON {
            continue;
        }
        let dcx = zcx - target_cx;
        let dcy = zcy - target_cy;
        let score = dcx * dcx + dcy * dcy;
        let replace = match best {
            None => true,
            Some((_, best_score)) => score < best_score,
        };
        if replace {
            best = Some((i, score));
        }
    }
    best.map(|(i, _)| i)
}

/// Snap `value` (a percentage) to the nearest point in `points` within
/// `tolerance`. Values outside every tolerance window are returned unchanged.
pub fn snap_percent(value: f32, points: &[f32], tolerance: f32) -> f32 {
    points
        .iter()
        .copied()
        .filter_map(|p| {
            let distance = (value - p).abs();
            (distance <= tolerance).then_some((distance, p))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, p)| p)
        .unwrap_or(value)
}

/// Snap to the default 25 / 50 / 75 guides.
pub fn snap_to_default_points(value: f32, tolerance: f32) -> f32 {
    snap_percent(value, &DEFAULT_SNAP_POINTS, tolerance)
}

/// A shared edge between adjacent zones, in percent coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SharedBoundary {
    /// Vertical divider at `at` percent x, spanning y from `start` to `end`.
    Vertical { at: f32, start: f32, end: f32 },
    /// Horizontal divider at `at` percent y, spanning x from `start` to `end`.
    Horizontal { at: f32, start: f32, end: f32 },
}

impl SharedBoundary {
    /// Position of the divider along its own axis, in percent.
    pub fn position(&self) -> f32 {
        match self {
            Self::Vertical { at, .. } | Self::Horizontal { at, .. } => *at,
        }
    }

    /// Span of the divider along the perpendicular axis, in percent.
    pub fn span(&self) -> (f32, f32) {
        match self {
            Self::Vertical { start, end, .. } | Self::Horizontal { start, end, .. } => {
                (*start, *end)
            }
        }
    }
}

fn same_boundary(a: &SharedBoundary, b: &SharedBoundary, tolerance: f32) -> bool {
    match (a, b) {
        (
            SharedBoundary::Vertical {
                at: a1,
                start: s1,
                end: e1,
            },
            SharedBoundary::Vertical {
                at: a2,
                start: s2,
                end: e2,
            },
        )
        | (
            SharedBoundary::Horizontal {
                at: a1,
                start: s1,
                end: e1,
            },
            SharedBoundary::Horizontal {
                at: a2,
                start: s2,
                end: e2,
            },
        ) => {
            (a1 - a2).abs() <= tolerance
                && (s1 - s2).abs() <= tolerance
                && (e1 - e2).abs() <= tolerance
        }
        _ => false,
    }
}

/// Find every shared edge between zones (design spec §2b).
///
/// A vertical boundary exists at `x` when at least one zone's right edge
/// (`x + w`) and at least one other zone's left edge (`x`) are within
/// `tolerance` of `x`; its span covers the y-range of all touching zones.
/// Horizontal boundaries are the analogous case for top/bottom edges.
pub fn find_shared_boundaries(zones: &[Zone], tolerance: f32) -> Vec<SharedBoundary> {
    let mut found: Vec<SharedBoundary> = Vec::new();

    for z in zones {
        // Vertical candidates: this zone's left and right edges.
        for at in [z.x, z.x + z.w] {
            let before: Vec<&Zone> = zones
                .iter()
                .filter(|o| ((o.x + o.w) - at).abs() <= tolerance)
                .collect();
            let after: Vec<&Zone> = zones
                .iter()
                .filter(|o| (o.x - at).abs() <= tolerance)
                .collect();
            if before.is_empty() || after.is_empty() {
                continue;
            }
            let start = before
                .iter()
                .chain(after.iter())
                .map(|o| o.y)
                .fold(f32::INFINITY, f32::min);
            let end = before
                .iter()
                .chain(after.iter())
                .map(|o| o.y + o.h)
                .fold(f32::NEG_INFINITY, f32::max);
            let boundary = SharedBoundary::Vertical { at, start, end };
            if !found.iter().any(|e| same_boundary(e, &boundary, tolerance)) {
                found.push(boundary);
            }
        }

        // Horizontal candidates: this zone's top and bottom edges.
        for at in [z.y, z.y + z.h] {
            let above: Vec<&Zone> = zones
                .iter()
                .filter(|o| ((o.y + o.h) - at).abs() <= tolerance)
                .collect();
            let below: Vec<&Zone> = zones
                .iter()
                .filter(|o| (o.y - at).abs() <= tolerance)
                .collect();
            if above.is_empty() || below.is_empty() {
                continue;
            }
            let start = above
                .iter()
                .chain(below.iter())
                .map(|o| o.x)
                .fold(f32::INFINITY, f32::min);
            let end = above
                .iter()
                .chain(below.iter())
                .map(|o| o.x + o.w)
                .fold(f32::NEG_INFINITY, f32::max);
            let boundary = SharedBoundary::Horizontal { at, start, end };
            if !found.iter().any(|e| same_boundary(e, &boundary, tolerance)) {
                found.push(boundary);
            }
        }
    }

    found
}

/// CamelCase alias matching the name used in the design spec (`design-v3.html` §2b).
#[allow(non_snake_case)]
pub fn findSharedBoundaries(zones: &[Zone], tolerance: f32) -> Vec<SharedBoundary> {
    find_shared_boundaries(zones, tolerance)
}

/// Compute the invisible DWM border widths from a Win32 window rect and the
/// DWM extended-frame-bounds rect. Each component is clamped to be non-negative.
///
/// Returns `(left, top, right, bottom)` in pixels.
pub fn border_offsets(win: Rect, dwm: Rect) -> (i32, i32, i32, i32) {
    (
        (dwm.x - win.x).max(0),
        (dwm.y - win.y).max(0),
        ((win.x + win.w) - (dwm.x + dwm.w)).max(0),
        ((win.y + win.h) - (dwm.y + dwm.h)).max(0),
    )
}

/// Expand a target zone rect by the invisible borders so that the *visible*
/// frame lands exactly on the zone. This is the arithmetic `move_window` uses.
pub fn compensate_rect(zone: Rect, borders: (i32, i32, i32, i32)) -> Rect {
    let (left, top, right, bottom) = borders;
    Rect {
        x: zone.x - left,
        y: zone.y - top,
        w: zone.w + left + right,
        h: zone.h + top + bottom,
    }
}

/// A single keyboard injection instruction, decoupled from Win32 so it can be
/// unit-tested. `send_text`/`send_keys` in `engine.rs` turn these into SendInput.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyEvent {
    /// Inject a UTF-16 code unit via KEYEVENTF_UNICODE (text, layout-independent).
    Unicode(u16),
    /// Tap a virtual-key code (special keys like ENTER/F5).
    Vk(u16),
    /// Press modifier VK, tap key VK, release modifier (e.g. Ctrl+T).
    Chord(u16, u16),
}

/// Map arbitrary text to Unicode key events (one per UTF-16 code unit).
/// Unlike the old ASCII-as-VK approach this preserves case, punctuation and
/// non-ASCII characters and is independent of the keyboard layout.
pub fn text_to_events(text: &str) -> Vec<KeyEvent> {
    text.encode_utf16().map(KeyEvent::Unicode).collect()
}

/// Map an AHK-style token string to key events.
/// Recognised tokens: ^t, ^l, {ENTER}, {F5}, and generic ^X (X an ASCII letter/digit).
/// Unrecognised tokens are ignored (preserving the old parser's behavior).
pub fn keys_to_events(keys: &str) -> Vec<KeyEvent> {
    const VK_CONTROL: u16 = 0x11;
    const VK_RETURN: u16 = 0x0D;
    const VK_F5: u16 = 0x74;
    keys.split_whitespace()
        .filter_map(|token| match token {
            "^t" => Some(KeyEvent::Chord(VK_CONTROL, b'T' as u16)),
            "^l" => Some(KeyEvent::Chord(VK_CONTROL, b'L' as u16)),
            "{ENTER}" => Some(KeyEvent::Vk(VK_RETURN)),
            "{F5}" => Some(KeyEvent::Vk(VK_F5)),
            _ if token.starts_with('^') && token.chars().count() == 2 => token
                .chars()
                .nth(1)
                .and_then(|c| {
                    // Only letters/digits are valid generic chord keys. Punctuation
                    // would map to an unrelated VK (e.g. '!' -> VK_PRIOR).
                    if c.is_ascii_alphanumeric() {
                        Some(c.to_ascii_uppercase() as u16)
                    } else {
                        None
                    }
                })
                .map(|vk| KeyEvent::Chord(VK_CONTROL, vk)),
            _ => None,
        })
        .collect()
}

/// Choose the executable to launch: the stored full path when present,
/// otherwise the exe name (resolved via PATH). AHK parity (§2c).
pub fn launch_target(zone: &Zone) -> &str {
    if zone.path.trim().is_empty() {
        &zone.exe
    } else {
        &zone.path
    }
}

/// Return groups of zone ids in `profile` with ambiguous duplicate assignments.
///
/// Zones are first grouped by case-insensitive exe name (the window matcher's
/// primary identity). Within an exe group, two zones with distinct non-empty
/// full paths are *not* duplicates — exact path matching resolves them to
/// different windows. A group is ambiguous only when at least one zone has no
/// path (it can fall back to the first exe-name match and collide with any
/// other zone in the group) or when multiple zones share the same path.
///
/// Used to warn about ambiguous assignments; changes no runtime behavior.
pub fn duplicate_app_assignments(profile: &Profile) -> Vec<(String, Vec<i32>)> {
    use std::collections::{BTreeMap, BTreeSet};
    let mut by_exe: BTreeMap<String, Vec<&Zone>> = BTreeMap::new();
    for z in &profile.zones {
        if z.exe.trim().is_empty() {
            continue;
        }
        by_exe
            .entry(z.exe.trim().to_lowercase())
            .or_default()
            .push(z);
    }
    let mut out = Vec::new();
    for (exe, zones) in by_exe {
        if zones.len() < 2 {
            continue;
        }
        let mut paths = BTreeSet::new();
        let mut has_exe_only = false;
        for z in &zones {
            if z.path.trim().is_empty() {
                has_exe_only = true;
            } else {
                paths.insert(z.path.trim().replace('/', "\\").to_lowercase());
            }
        }
        // Distinct full paths with no exe-only zone resolve to different
        // windows — not a duplicate. Otherwise the whole exe group is ambiguous.
        if !has_exe_only && paths.len() > 1 {
            continue;
        }
        out.push((exe, zones.iter().map(|z| z.id).collect()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zone(id: i32, x: f32, y: f32, w: f32, h: f32) -> Zone {
        Zone::new(id, "", "", x, y, w, h)
    }

    #[test]
    fn zone_pixels_applies_gap_symmetrically() {
        let area = Rect {
            x: 0,
            y: 0,
            w: 1000,
            h: 800,
        };
        let left = zone_pixels(&zone(1, 0.0, 0.0, 50.0, 100.0), area, 10);
        let right = zone_pixels(&zone(2, 50.0, 0.0, 50.0, 100.0), area, 10);
        assert_eq!(
            left,
            Rect {
                x: 10,
                y: 10,
                w: 485,
                h: 780
            }
        );
        assert_eq!(
            right,
            Rect {
                x: 505,
                y: 10,
                w: 485,
                h: 780
            }
        );
        // The gap between the two zones is exactly 10 px (495 -> 505).
        assert_eq!(left.x + left.w + 10, right.x);
    }

    #[test]
    fn zone_pixels_clamps_to_minimum_size() {
        let area = Rect {
            x: 0,
            y: 0,
            w: 1000,
            h: 800,
        };
        let tiny = zone_pixels(&zone(1, 0.0, 0.0, 1.0, 1.0), area, 0);
        assert_eq!(tiny.w, 100);
        assert_eq!(tiny.h, 100);
    }

    #[test]
    fn zone_pixels_respects_area_offset() {
        let area = Rect {
            x: 100,
            y: 50,
            w: 1000,
            h: 800,
        };
        let r = zone_pixels(&zone(1, 0.0, 0.0, 100.0, 100.0), area, 0);
        assert_eq!(
            r,
            Rect {
                x: 100,
                y: 50,
                w: 1000,
                h: 800
            }
        );
    }

    #[test]
    fn compute_zones_preserves_ids_and_order() {
        let profile = Profile {
            name: "test".into(),
            hotkey: String::new(),
            zones: vec![
                zone(7, 0.0, 0.0, 50.0, 100.0),
                zone(3, 50.0, 0.0, 50.0, 100.0),
            ],
        };
        let area = Rect {
            x: 0,
            y: 0,
            w: 1000,
            h: 800,
        };
        let computed = compute_zones(&profile, area, 0);
        assert_eq!(computed.len(), 2);
        assert_eq!(computed[0].0, 7);
        assert_eq!(computed[1].0, 3);
        assert_eq!(computed[0].1, zone_pixels(&profile.zones[0], area, 0));
        assert_eq!(zone_by_id(&computed, 3), Some(computed[1].1));
        assert_eq!(zone_by_id(&computed, 99), None);
    }

    #[test]
    fn zone_at_finds_containing_zone() {
        let zones = vec![
            (
                1,
                Rect {
                    x: 0,
                    y: 0,
                    w: 100,
                    h: 100,
                },
            ),
            (
                2,
                Rect {
                    x: 100,
                    y: 0,
                    w: 100,
                    h: 100,
                },
            ),
        ];
        assert_eq!(zone_at(&zones, 50, 50), Some(1));
        assert_eq!(zone_at(&zones, 150, 50), Some(2));
        // The shared edge belongs to the right-hand zone (half-open intervals).
        assert_eq!(zone_at(&zones, 100, 50), Some(2));
        assert_eq!(zone_at(&zones, 500, 500), None);
    }

    #[test]
    fn find_shared_boundaries_detects_vertical_divider() {
        let zones = vec![
            zone(1, 0.0, 0.0, 50.0, 100.0),
            zone(2, 50.0, 0.0, 50.0, 100.0),
        ];
        let boundaries = find_shared_boundaries(&zones, 1.5);
        assert_eq!(boundaries.len(), 1);
        assert_eq!(
            boundaries[0],
            SharedBoundary::Vertical {
                at: 50.0,
                start: 0.0,
                end: 100.0
            }
        );
    }

    #[test]
    fn find_shared_boundaries_detects_grid_dividers() {
        let zones = vec![
            zone(1, 0.0, 0.0, 50.0, 50.0),
            zone(2, 50.0, 0.0, 50.0, 50.0),
            zone(3, 0.0, 50.0, 50.0, 50.0),
            zone(4, 50.0, 50.0, 50.0, 50.0),
        ];
        let boundaries = find_shared_boundaries(&zones, 1.5);
        assert_eq!(boundaries.len(), 2);
        assert!(boundaries.contains(&SharedBoundary::Vertical {
            at: 50.0,
            start: 0.0,
            end: 100.0
        }));
        assert!(boundaries.contains(&SharedBoundary::Horizontal {
            at: 50.0,
            start: 0.0,
            end: 100.0
        }));
    }

    #[test]
    fn find_shared_boundaries_tolerates_small_gaps() {
        let zones = vec![
            zone(1, 0.0, 0.0, 49.5, 100.0),
            zone(2, 50.0, 0.0, 50.0, 100.0),
        ];
        let boundaries = find_shared_boundaries(&zones, 1.5);
        assert_eq!(boundaries.len(), 1);
        assert_eq!(boundaries[0].position(), 49.5);
        assert_eq!(boundaries[0].span(), (0.0, 100.0));
    }

    #[test]
    fn snapping_uses_25_50_75_guides() {
        assert_eq!(snap_to_default_points(24.2, 1.5), 25.0);
        assert_eq!(snap_to_default_points(51.0, 1.5), 50.0);
        assert_eq!(snap_to_default_points(74.0, 1.5), 75.0);
        assert_eq!(snap_to_default_points(40.0, 1.5), 40.0);
        assert_eq!(snap_to_default_points(50.0, 0.0), 50.0);
    }

    #[test]
    fn percent_and_pixel_conversion_round_trip() {
        assert_eq!(percent_to_pixel(25.0, 1920), 480);
        assert_eq!(percent_to_pixel(33.333, 1000), 333);
        assert_eq!(pixel_to_percent(480, 1920), 25.0);
        assert_eq!(pixel_to_percent(0, 0), 0.0);
    }

    #[test]
    fn dwm_border_offsets_and_compensation() {
        let win = Rect {
            x: 100,
            y: 100,
            w: 800,
            h: 600,
        };
        let dwm = Rect {
            x: 104,
            y: 103,
            w: 792,
            h: 594,
        };
        let borders = border_offsets(win, dwm);
        assert_eq!(borders, (4, 3, 4, 3));
        let target = compensate_rect(
            Rect {
                x: 200,
                y: 150,
                w: 400,
                h: 300,
            },
            borders,
        );
        assert_eq!(
            target,
            Rect {
                x: 196,
                y: 147,
                w: 408,
                h: 306
            }
        );
    }

    #[test]
    fn dwm_border_offsets_clamp_negative_values() {
        let win = Rect {
            x: 0,
            y: 0,
            w: 100,
            h: 100,
        };
        let dwm = Rect {
            x: -5,
            y: -5,
            w: 110,
            h: 110,
        };
        assert_eq!(border_offsets(win, dwm), (0, 0, 0, 0));
    }

    #[test]
    fn nearest_zone_in_direction_moves_across_a_2x2_grid() {
        let zones = vec![
            zone(1, 0.0, 0.0, 50.0, 50.0),   // top-left
            zone(2, 50.0, 0.0, 50.0, 50.0),  // top-right
            zone(3, 0.0, 50.0, 50.0, 50.0),  // bottom-left
            zone(4, 50.0, 50.0, 50.0, 50.0), // bottom-right
        ];
        assert_eq!(nearest_zone_in_direction(&zones, 0, 1, 0), Some(1)); // right
        assert_eq!(nearest_zone_in_direction(&zones, 1, -1, 0), Some(0)); // left
        assert_eq!(nearest_zone_in_direction(&zones, 0, 0, 1), Some(2)); // down
        assert_eq!(nearest_zone_in_direction(&zones, 2, 0, -1), Some(0)); // up
    }

    #[test]
    fn nearest_zone_in_direction_respects_edges_and_zero_direction() {
        let zones = vec![
            zone(1, 0.0, 0.0, 50.0, 100.0),
            zone(2, 50.0, 0.0, 50.0, 100.0),
        ];
        // No zone to the left of the first one, nor above/below either one.
        assert_eq!(nearest_zone_in_direction(&zones, 0, -1, 0), None);
        assert_eq!(nearest_zone_in_direction(&zones, 1, 1, 0), None);
        assert_eq!(nearest_zone_in_direction(&zones, 0, 0, -1), None);
        assert_eq!(nearest_zone_in_direction(&zones, 0, 0, 0), None);
    }

    #[test]
    fn nearest_zone_in_direction_picks_closest_of_several() {
        // Left half plus two right-hand quarters: moving right from the left
        // zone should land on the nearer (upper) quarter.
        let zones = vec![
            zone(1, 0.0, 0.0, 50.0, 100.0),
            zone(2, 50.0, 0.0, 50.0, 50.0),
            zone(3, 50.0, 50.0, 50.0, 50.0),
        ];
        assert_eq!(nearest_zone_in_direction(&zones, 0, 1, 0), Some(1));
    }

    // --- WS-A regression tests ---

    #[test]
    fn text_to_events_uses_unicode_not_ascii_as_vk() {
        // Lowercase 'a' (0x61) must be a Unicode code unit, never VK 0x61
        // (which is VK_NUMPAD1). This is the core of bug B1.
        assert_eq!(text_to_events("a"), vec![KeyEvent::Unicode(0x61)]);
        // Uppercase and punctuation are preserved as their code units.
        assert_eq!(
            text_to_events("A!"),
            vec![KeyEvent::Unicode(0x41), KeyEvent::Unicode(0x21)]
        );
    }

    #[test]
    fn text_to_events_preserves_non_ascii() {
        // Non-ASCII was silently dropped by the old `if c.is_ascii()` filter.
        let events = text_to_events("é中");
        assert_eq!(
            events,
            vec![KeyEvent::Unicode(0x00E9), KeyEvent::Unicode(0x4E2D)]
        );
    }

    #[test]
    fn keys_to_events_maps_lowercase_chord_to_uppercase_vk() {
        // `^a` must produce Ctrl+VK_A (0x41), not Ctrl+0x61. Bug B1 chord half.
        assert_eq!(keys_to_events("^a"), vec![KeyEvent::Chord(0x11, 0x41)]);
        assert_eq!(keys_to_events("^t"), vec![KeyEvent::Chord(0x11, 0x54)]);
        assert_eq!(keys_to_events("{ENTER}"), vec![KeyEvent::Vk(0x0D)]);
        assert_eq!(keys_to_events("{F5}"), vec![KeyEvent::Vk(0x74)]);
    }

    #[test]
    fn keys_to_events_ignores_unrecognised_tokens() {
        assert!(keys_to_events("not a token").is_empty());
        assert!(keys_to_events("^é").is_empty()); // non-ASCII chord ignored
    }

    #[test]
    fn keys_to_events_rejects_punctuation_chords() {
        // Doc says generic ^X is a letter/digit; punctuation must not become a
        // bogus VK ('!' = 0x21 is VK_PRIOR, '-' = 0x2D is VK_INSERT).
        assert!(keys_to_events("^!").is_empty());
        assert!(keys_to_events("^-").is_empty());
        // Digits remain valid.
        assert_eq!(
            keys_to_events("^1"),
            vec![KeyEvent::Chord(0x11, b'1' as u16)]
        );
    }

    #[test]
    fn launch_target_prefers_full_path_then_falls_back_to_exe() {
        let mut z = Zone::new(1, "", "app.exe", 0.0, 0.0, 50.0, 100.0);
        assert_eq!(launch_target(&z), "app.exe");
        z.path = r"C:\Program Files\Vendor\app.exe".into();
        assert_eq!(launch_target(&z), r"C:\Program Files\Vendor\app.exe");
        z.path = "   ".into(); // whitespace-only treated as empty
        assert_eq!(launch_target(&z), "app.exe");
    }

    #[test]
    fn duplicate_app_assignments_groups_same_exe_and_path() {
        let profile = Profile {
            name: "test".into(),
            hotkey: String::new(),
            zones: vec![
                Zone::new(1, "", "chrome.exe", 0.0, 0.0, 50.0, 100.0),
                Zone::new(2, "", "CHROME.EXE", 50.0, 0.0, 50.0, 100.0), // case-insensitive dup
                Zone::new(3, "", "code.exe", 0.0, 0.0, 50.0, 100.0),
            ],
        };
        let dups = duplicate_app_assignments(&profile);
        assert_eq!(dups.len(), 1);
        assert_eq!(dups[0].0, "chrome.exe");
        assert_eq!(dups[0].1, vec![1, 2]);
    }

    #[test]
    fn duplicate_app_assignments_empty_for_distinct_apps() {
        let profile = Profile {
            name: "test".into(),
            hotkey: String::new(),
            zones: vec![
                Zone::new(1, "", "chrome.exe", 0.0, 0.0, 50.0, 100.0),
                Zone::new(2, "", "code.exe", 50.0, 0.0, 50.0, 100.0),
            ],
        };
        assert!(duplicate_app_assignments(&profile).is_empty());
    }

    #[test]
    fn duplicate_app_assignments_detects_mixed_path_and_exe_zones() {
        // One zone stores a full path, the other only the exe name: both refer
        // to the same app and must be grouped (the old path-keyed helper missed this).
        let mut with_path = Zone::new(1, "", "chrome.exe", 0.0, 0.0, 50.0, 100.0);
        with_path.path = r"C:\Program Files\Google\Chrome\chrome.exe".into();
        let exe_only = Zone::new(2, "", "chrome.exe", 50.0, 0.0, 50.0, 100.0);
        let profile = Profile {
            name: "test".into(),
            hotkey: String::new(),
            zones: vec![with_path, exe_only],
        };
        let dups = duplicate_app_assignments(&profile);
        assert_eq!(dups.len(), 1);
        assert_eq!(dups[0].0, "chrome.exe");
        assert_eq!(dups[0].1, vec![1, 2]);
    }

    #[test]
    fn duplicate_app_assignments_ignores_distinct_installations() {
        // Two genuinely different installations sharing an exe name resolve to
        // different windows via exact path matching — no warning should fire.
        let mut a = Zone::new(1, "", "chrome.exe", 0.0, 0.0, 50.0, 100.0);
        a.path = r"C:\Program Files\Google\Chrome\chrome.exe".into();
        let mut b = Zone::new(2, "", "chrome.exe", 50.0, 0.0, 50.0, 100.0);
        b.path = r"D:\Portable\Chrome\chrome.exe".into();
        let profile = Profile {
            name: "test".into(),
            hotkey: String::new(),
            zones: vec![a, b],
        };
        assert!(duplicate_app_assignments(&profile).is_empty());
    }

    #[test]
    fn duplicate_app_assignments_groups_identical_paths() {
        // Same path expressed with different case and separators is still a
        // duplicate (both zones resolve to the same window).
        let mut a = Zone::new(1, "", "chrome.exe", 0.0, 0.0, 50.0, 100.0);
        a.path = r"C:\Chrome\chrome.exe".into();
        let mut b = Zone::new(2, "", "chrome.exe", 50.0, 0.0, 50.0, 100.0);
        b.path = "c:/chrome/chrome.exe".into();
        let profile = Profile {
            name: "test".into(),
            hotkey: String::new(),
            zones: vec![a, b],
        };
        let dups = duplicate_app_assignments(&profile);
        assert_eq!(dups.len(), 1);
        assert_eq!(dups[0].1, vec![1, 2]);
    }
}
