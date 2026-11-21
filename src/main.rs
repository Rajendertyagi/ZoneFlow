mod autostart;
mod config;
mod editor;
mod engine;
mod engine_core;
mod overlay;
mod system;

use eframe::egui;

fn main() -> eframe::Result<()> {
    // Logging to a file next to the exe; level via RUST_LOG, default "info".
    let log_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("zoneflow.log")));
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_ansi(false)
        .with_writer(move || -> Box<dyn std::io::Write> {
            if let Some(ref p) = log_path {
                if let Ok(f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(p)
                {
                    return Box::new(f);
                }
            }
            Box::new(std::io::sink())
        })
        .finish();
    // Only fails if a global subscriber was already installed (e.g. by a test
    // harness or embedding host); in that case the existing subscriber is kept,
    // which is harmless, so the result is intentionally ignored.
    let _ = tracing::subscriber::set_global_default(subscriber);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1500.0, 850.0])
            .with_title("ScreenLayout Studio"),
        ..Default::default()
    };
    eframe::run_native(
        "ScreenLayout Studio",
        options,
        Box::new(|cc| {
            // Apply shadcn theme once at startup: Zinc base + Amber accent,
            // with JetBrains Mono installed as egui's Monospace font.
            use shadcn_egui::{BaseColor, FontName, Preset, ThemeColor};
            let mut fonts = egui::FontDefinitions::default();
            // JetBrains Mono is bundled via the `font-jetbrains-mono` feature, but
            // handle its absence gracefully rather than panicking at startup: if
            // the font is unavailable, egui's default monospace font is used.
            if let Some(jb) = shadcn_egui::fonts::files(FontName::JetbrainsMono) {
                fonts.font_data.insert(
                    "zf-mono".into(),
                    std::sync::Arc::new(egui::FontData::from_static(jb.face(400).data)),
                );
                fonts
                    .families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .insert(0, "zf-mono".into());
            } else {
                tracing::warn!(
                    "JetBrains Mono font unavailable; falling back to egui's default monospace font"
                );
            }
            Preset::default()
                .base_color(BaseColor::Zinc)
                .theme_color(ThemeColor::Amber)
                .theme()
                .apply_with_fonts(&cc.egui_ctx, fonts);
            cc.egui_ctx.set_theme(egui::Theme::Dark);
            Ok(Box::new(editor::Editor::default().with_system()))
        }),
    )
}
