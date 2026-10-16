#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod ui;
use eframe::egui;
fn main() -> eframe::Result {
    if let Some(result) = strata_app::browser_integration::confirmation_if_requested() {
        std::process::exit(match result {
            Ok(true) => strata_app::browser_integration::CONFIRM_ACCEPTED,
            Ok(false) => strata_app::browser_integration::CONFIRM_DECLINED,
            Err(error) => {
                eprintln!("StrataFetch confirmation: {error:#}");
                2
            }
        });
    }
    if let Some(result) = strata_app::updates::apply_update_if_requested() {
        if let Err(error) = result {
            rfd::MessageDialog::new()
                .set_title("StrataFetch update failed")
                .set_description(format!("{error:#}\nThe previous executable is retained in the application or update directory."))
                .set_level(rfd::MessageLevel::Error)
                .show();
        }
        return Ok(());
    }
    let setup = std::env::args().any(|arg| arg == "--browser-setup");
    let _desktop_lock =
        match strata_app::browser_integration::desktop_lock(&strata_app::state_dir(), setup) {
            Ok(Some(file)) => file,
            Ok(None) => return Ok(()),
            Err(error) => {
                rfd::MessageDialog::new()
                    .set_title("Cannot open StrataFetch")
                    .set_description(format!("{error:#}"))
                    .set_level(rfd::MessageLevel::Error)
                    .show();
                return Ok(());
            }
        };
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
            .with_app_id("org.stratafetch.StrataFetch")
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
