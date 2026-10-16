//! A visible, self-contained confirmation window, independent of native hosts
//! launched with hidden startup flags or Linux's optional dialog utilities.
use anyhow::Result;
use eframe::egui;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub fn show(name: String, source: String, destination: String) -> Result<bool> {
    let accepted = Arc::new(AtomicBool::new(false));
    let result = accepted.clone();
    let options = eframe::NativeOptions {
        persist_window: false,
        viewport: egui::ViewportBuilder::default()
            .with_app_id("org.stratafetch.confirmation")
            .with_title("Download with StrataFetch")
            .with_inner_size([560.0, 330.0])
            .with_min_inner_size([460.0, 300.0])
            .with_active(true)
            .with_visible(true)
            .with_always_on_top()
            .with_icon(Arc::new(crate::branding::icon())),
        ..Default::default()
    };
    eframe::run_native(
        "Download with StrataFetch",
        options,
        Box::new(move |cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::new(Confirmation {
                name,
                source,
                destination,
                accepted,
                first_frame: true,
                screenshot: std::env::var_os("STRATAFETCH_CONFIRM_CAPTURE").map(Into::into),
                started: std::time::Instant::now(),
                screenshot_requested: false,
            }))
        }),
    )
    .map_err(|error| anyhow::anyhow!("Cannot display download confirmation: {error}"))?;
    Ok(result.load(Ordering::Relaxed))
}

struct Confirmation {
    name: String,
    source: String,
    destination: String,
    accepted: Arc<AtomicBool>,
    first_frame: bool,
    screenshot: Option<std::path::PathBuf>,
    started: std::time::Instant,
    screenshot_requested: bool,
}
impl eframe::App for Confirmation {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.draw(root);
        // A framebuffer capture can only decline. There is no environment
        // variable or unattended path that approves a browser download.
        if let Some(path) = &self.screenshot {
            let ctx = root.ctx();
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
            if self.started.elapsed().as_secs() >= 2 && !self.screenshot_requested {
                self.screenshot_requested = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            }
            if let Some(image) = ctx.input(|input| {
                input.events.iter().find_map(|event| match event {
                    egui::Event::Screenshot { image, .. } => Some(image.clone()),
                    _ => None,
                })
            }) {
                let pixels: Vec<u8> = image
                    .pixels
                    .iter()
                    .flat_map(|pixel| pixel.to_array())
                    .collect();
                if let Err(error) = image::save_buffer(
                    path,
                    &pixels,
                    image.size[0] as u32,
                    image.size[1] as u32,
                    image::ColorType::Rgba8,
                ) {
                    eprintln!("Confirmation capture failed: {error}");
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }
}
impl Confirmation {
    fn draw(&mut self, root: &mut egui::Ui) -> [egui::Rect; 2] {
        let ctx = root.ctx().clone();
        let buttons = egui::CentralPanel::default()
            .show(root, |ui| {
                ui.add_space(10.0);
                ui.heading(egui::RichText::new("Download this file?").color(egui::Color32::WHITE));
                ui.add_space(12.0);
                egui::ScrollArea::vertical()
                    .max_height((ui.available_height() - 100.0).max(100.0))
                    .show(ui, |ui| {
                        ui.strong(&self.name);
                        ui.add_space(10.0);
                        ui.label(format!("From: {}", self.source));
                        ui.label(format!("Save to: {}", self.destination));
                    });
                ui.add_space(12.0);
                ui.label("The browser download stays paused while you decide.");
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    let download = ui.add_sized(
                        [240.0, 34.0],
                        egui::Button::new(
                            egui::RichText::new("Download with StrataFetch")
                                .color(egui::Color32::WHITE),
                        )
                        .fill(egui::Color32::from_rgb(35, 125, 103)),
                    );
                    if download.clicked() {
                        self.accepted.store(true, Ordering::Relaxed);
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    let keep = ui.add_sized([180.0, 34.0], egui::Button::new("Keep in browser"));
                    if self.first_frame {
                        keep.request_focus();
                    }
                    if keep.clicked() || ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    [download.rect, keep.rect]
                })
                .inner
            })
            .inner;
        if self.first_frame {
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            self.first_frame = false;
        }
        buttons
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn visible_buttons_require_an_explicit_download_click() {
        for approve in [false, true] {
            let accepted = Arc::new(AtomicBool::new(false));
            let mut dialog = Confirmation {
                name: "file.zip".into(),
                source: "localhost".into(),
                destination: "/tmp".into(),
                accepted: accepted.clone(),
                first_frame: true,
                screenshot: None,
                started: std::time::Instant::now(),
                screenshot_requested: false,
            };
            let ctx = egui::Context::default();
            let mut buttons = [egui::Rect::NOTHING; 2];
            for frame in 0..4 {
                let mut input = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(560.0, 330.0),
                    )),
                    ..Default::default()
                };
                if frame == 3 {
                    let pos = buttons[if approve { 0 } else { 1 }].center();
                    input.events = vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed: true,
                            modifiers: Default::default(),
                        },
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed: false,
                            modifiers: Default::default(),
                        },
                    ];
                }
                let mut output = ctx.run_ui(input, |root| {
                    buttons = dialog.draw(root);
                });
                output.textures_delta.clear();
                assert!(
                    buttons
                        .iter()
                        .all(|rect| rect.is_finite() && rect.min.y > 0.0 && rect.max.y <= 330.0)
                );
                if frame < 3 {
                    assert!(!accepted.load(Ordering::Relaxed));
                }
            }
            assert_eq!(accepted.load(Ordering::Relaxed), approve);
        }
    }
}
