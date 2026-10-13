#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod ui;
use eframe::egui;
fn main() -> eframe::Result {
    let smoke = std::env::var_os("STRATAFETCH_SMOKE_OUTPUT").is_some();
    let width = std::env::var("STRATAFETCH_SMOKE_WIDTH")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .filter(|_| smoke)
        .unwrap_or(1280.0)
        .clamp(880.0, 2560.0);
    let height = std::env::var("STRATAFETCH_SMOKE_HEIGHT")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .filter(|_| smoke)
        .unwrap_or(720.0)
        .clamp(500.0, 1440.0);
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        persist_window: !smoke,
        viewport: egui::ViewportBuilder::default()
            .with_icon(std::sync::Arc::new(strata_app::branding::icon()))
            .with_inner_size([width, height])
            .with_clamp_size_to_monitor_size(!smoke)
            .with_min_inner_size([880.0, 500.0]),
        persistence_path: std::env::var_os("STRATAFETCH_STATE_DIR").map(|path| {
            std::path::PathBuf::from(path).join(if smoke {
                format!("desktop-smoke-{}.ron", std::process::id())
            } else {
                "desktop-preferences.ron".into()
            })
        }),
        ..Default::default()
    };
    eframe::run_native(
        "StrataFetch",
        options,
        Box::new(|cc| Ok(Box::new(ui::App::new(cc)))),
    )
}
