use super::{settings, transfer_dialog, ui_state::State};
use eframe::egui;
#[test]
fn local_browser_setup_renders_buttons_and_approval_steps() {
    let ctx = egui::Context::default();
    let mut state = State::new(Default::default());
    state.show_settings();
    state.settings_page = 7;
    for frame in 0..3 {
        let mut output = ctx.run_ui(
            egui::RawInput {
                time: Some(frame as f64),
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 720.0),
                )),
                ..Default::default()
            },
            |root| {
                egui::CentralPanel::default().show(root, |_| {});
                settings::show(root.ctx(), &mut state);
            },
        );
        output.textures_delta.clear();
        if frame == 2 {
            for text in [
                "Set up Chrome",
                "Set up Edge",
                "Set up Firefox",
                "Load unpacked",
                "Temporary Add-on",
            ] {
                assert!(
                    output
                        .shapes
                        .iter()
                        .any(|shape| has_text(&shape.shape, text)),
                    "missing browser setup control: {text}"
                );
            }
        }
    }
}
#[test]
fn restarting_a_transfer_resets_its_average_speed_baseline() {
    use std::time::{Duration, Instant};
    let temp = tempfile::tempdir().unwrap();
    let manager =
        strata_core::Manager::open(&temp.path().join("state"), Default::default()).unwrap();
    let mut job = manager
        .add("https://example.org/file", temp.path(), None)
        .unwrap();
    let started = Instant::now();
    let mut session = super::Session {
        initial: 10_000,
        downloaded: 20_000,
        elapsed: 10.0,
        last: started,
        last_status: strata_core::Status::Paused,
    };
    job.status = strata_core::Status::Downloading;
    session.observe(&job, started + Duration::from_secs(1));
    assert_eq!(session.initial, 0);
    assert_eq!(session.elapsed, 0.0);
    job.downloaded = 1024;
    session.observe(&job, started + Duration::from_secs(2));
    assert_eq!(session.downloaded - session.initial, 1024);
    assert_eq!(session.elapsed, 1.0);
}
fn has_text(shape: &egui::Shape, needle: &str) -> bool {
    match shape {
        egui::Shape::Text(text) => text.pos.is_finite() && text.galley.job.text.contains(needle),
        egui::Shape::Vec(shapes) => shapes.iter().any(|shape| has_text(shape, needle)),
        _ => false,
    }
}
#[test]
fn transfer_window_and_settings_footer_are_visible() {
    let dir = tempfile::tempdir().unwrap();
    let manager =
        strata_core::Manager::open(&dir.path().join("state"), Default::default()).unwrap();
    let job = manager
        .add(
            "https://example.com/file.zip",
            &dir.path().join("out"),
            Some("file.zip"),
        )
        .unwrap();
    let jobs = vec![job.clone()];
    let ctx = egui::Context::default();
    let mut state = State::new(Default::default());
    state.transfer = Some(job.id);
    for frame in 0..3 {
        let mut output = ctx.run_ui(
            egui::RawInput {
                time: Some(frame as f64),
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 720.0),
                )),
                ..Default::default()
            },
            |root| {
                egui::CentralPanel::default().show(root, |_| {});
                transfer_dialog::show(root.ctx(), &mut state, &jobs, |_| None);
            },
        );
        output.textures_delta.clear();
        if frame == 2 {
            assert!(
                output
                    .shapes
                    .iter()
                    .any(|shape| has_text(&shape.shape, "Transfer details")),
                "transfer window must render finite visible text"
            );
            assert!(
                output.shapes.iter().any(|shape| shape
                    .clip_rect
                    .intersects(shape.shape.visual_bounding_rect())
                    && has_text(&shape.shape, "Open folder")),
                "transfer controls must stay visible"
            );
        }
    }
    state.prefs.descriptions.insert(
        jobs[0].id.clone(),
        "A long description that must scroll without hiding transfer controls. ".repeat(100),
    );
    for frame in 3..6 {
        let mut output = ctx.run_ui(
            egui::RawInput {
                time: Some(frame as f64),
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(640.0, 360.0),
                )),
                ..Default::default()
            },
            |root| {
                egui::CentralPanel::default().show(root, |_| {});
                transfer_dialog::show(root.ctx(), &mut state, &jobs, |_| None);
            },
        );
        output.textures_delta.clear();
        if frame == 5 {
            assert!(
                output.shapes.iter().any(|shape| shape
                    .clip_rect
                    .intersects(shape.shape.visual_bounding_rect())
                    && has_text(&shape.shape, "Open folder")),
                "small/scaled viewports and long descriptions must retain transfer controls"
            );
        }
    }
    transfer_dialog::show(&ctx, &mut state, &[], |_| None);
    assert!(
        state.transfer.is_none(),
        "removing a viewed job must release the dialog and keyboard guard"
    );
    state.settings_open = true;
    for frame in 6..9 {
        let mut output = ctx.run_ui(
            egui::RawInput {
                time: Some(frame as f64),
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 720.0),
                )),
                ..Default::default()
            },
            |root| {
                egui::CentralPanel::default().show(root, |_| {});
                settings::show(root.ctx(), &mut state);
            },
        );
        output.textures_delta.clear();
        if frame == 8 {
            assert!(
                output.shapes.iter().any(|shape| shape
                    .clip_rect
                    .intersects(shape.shape.visual_bounding_rect())
                    && has_text(&shape.shape, "Apply engine settings")),
                "settings footer must be on screen"
            );
        }
    }
}
#[test]
fn single_stream_transfers_do_not_claim_verified_resume() {
    let dir = tempfile::tempdir().unwrap();
    let manager =
        strata_core::Manager::open(&dir.path().join("state"), Default::default()).unwrap();
    let mut job = manager
        .add("https://example.org/file", dir.path(), None)
        .unwrap();
    job.validator = Some("\"etag-without-ranges\"".into());
    job.size = Some(100);
    job.downloaded = 50;
    let ctx = egui::Context::default();
    let mut state = State::new(Default::default());
    state.transfer = Some(job.id.clone());
    for frame in 0..3 {
        let mut output = ctx.run_ui(
            egui::RawInput {
                time: Some(frame as f64),
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 720.0),
                )),
                ..Default::default()
            },
            |root| {
                egui::CentralPanel::default().show(root, |_| {});
                transfer_dialog::show(root.ctx(), &mut state, std::slice::from_ref(&job), |_| None);
            },
        );
        output.textures_delta.clear();
        if frame == 2 {
            assert!(
                output
                    .shapes
                    .iter()
                    .any(|shape| has_text(&shape.shape, "Restart required if interrupted"))
            );
            assert!(
                !output
                    .shapes
                    .iter()
                    .any(|shape| has_text(&shape.shape, "Validated byte ranges"))
            );
        }
    }
}
