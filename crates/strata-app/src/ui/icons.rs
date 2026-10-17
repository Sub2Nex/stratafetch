use eframe::egui::{self, Color32, Stroke};

#[derive(Clone, Copy)]
pub enum Icon {
    Add,

    Resume,

    Pause,

    Stop,

    Remove,

    Clock,

    Settings,

    Queue,

    File,
    Archive,
    Audio,
    Video,
    Image,
    Application,
    Document,

    Folder,
}

pub fn paint(ui: &egui::Ui, rect: egui::Rect, icon: Icon, color: Color32) {
    let p = ui.painter();

    let stroke = Stroke::new(1.6, color);

    let point = |x: f32, y: f32| {
        egui::pos2(
            rect.left() + x * rect.width(),
            rect.top() + y * rect.height(),
        )
    };

    let line =
        |a: (f32, f32), b: (f32, f32)| p.line_segment([point(a.0, a.1), point(b.0, b.1)], stroke);

    match icon {
        Icon::Add => {
            line((0.5, 0.12), (0.5, 0.88));

            line((0.12, 0.5), (0.88, 0.5));
        }

        Icon::Resume => {
            p.add(egui::Shape::convex_polygon(
                vec![point(0.25, 0.15), point(0.85, 0.5), point(0.25, 0.85)],
                color,
                Stroke::NONE,
            ));
        }

        Icon::Pause => {
            for x in [0.25, 0.62] {
                p.rect_filled(
                    egui::Rect::from_min_max(point(x, 0.18), point(x + 0.13, 0.82)),
                    1.0,
                    color,
                );
            }
        }

        Icon::Stop => {
            p.rect_stroke(
                egui::Rect::from_min_max(point(0.2, 0.2), point(0.8, 0.8)),
                1.0,
                stroke,
                egui::StrokeKind::Inside,
            );
        }

        Icon::Remove => {
            line((0.15, 0.25), (0.85, 0.25));

            line((0.35, 0.1), (0.65, 0.1));

            line((0.25, 0.3), (0.32, 0.9));

            line((0.75, 0.3), (0.68, 0.9));

            line((0.32, 0.9), (0.68, 0.9));
        }

        Icon::Clock => {
            p.circle_stroke(rect.center(), rect.width() * 0.4, stroke);

            line((0.5, 0.24), (0.5, 0.5));

            line((0.5, 0.5), (0.72, 0.6));
        }

        Icon::Settings => {
            p.circle_stroke(rect.center(), rect.width() * 0.22, stroke);

            for i in 0..8 {
                let a = i as f32 * std::f32::consts::TAU / 8.0;

                line(
                    (0.5 + a.cos() * 0.28, 0.5 + a.sin() * 0.28),
                    (0.5 + a.cos() * 0.43, 0.5 + a.sin() * 0.43),
                );
            }
        }

        Icon::Queue => {
            for y in [0.25, 0.5, 0.75] {
                line((0.15, y), (0.22, y));

                line((0.35, y), (0.88, y));
            }
        }

        Icon::File => {
            line((0.22, 0.1), (0.62, 0.1));

            line((0.62, 0.1), (0.8, 0.3));

            line((0.8, 0.3), (0.8, 0.9));

            line((0.8, 0.9), (0.22, 0.9));

            line((0.22, 0.9), (0.22, 0.1));

            line((0.62, 0.1), (0.62, 0.3));

            line((0.62, 0.3), (0.8, 0.3));
        }
        Icon::Archive => {
            paint(ui, rect, Icon::File, color);
            for y in [0.35, 0.5, 0.65, 0.8] {
                line((0.46, y), (0.55, y));
            }
        }
        Icon::Audio => {
            line((0.4, 0.2), (0.8, 0.12));
            line((0.4, 0.2), (0.4, 0.72));
            line((0.8, 0.12), (0.8, 0.62));
            p.circle_filled(point(0.3, 0.75), rect.width() * 0.13, color);
            p.circle_filled(point(0.7, 0.65), rect.width() * 0.13, color);
        }
        Icon::Video => {
            p.rect_stroke(
                egui::Rect::from_min_max(point(0.08, 0.2), point(0.92, 0.8)),
                1.0,
                stroke,
                egui::StrokeKind::Inside,
            );
            paint(ui, rect.shrink(4.0), Icon::Resume, color);
        }
        Icon::Image => {
            p.rect_stroke(rect.shrink(2.0), 1.0, stroke, egui::StrokeKind::Inside);
            p.circle_filled(point(0.65, 0.3), rect.width() * 0.08, color);
            line((0.15, 0.8), (0.45, 0.48));
            line((0.45, 0.48), (0.8, 0.8));
        }
        Icon::Application => {
            p.rect_stroke(rect.shrink(2.0), 1.0, stroke, egui::StrokeKind::Inside);
            line((0.15, 0.35), (0.85, 0.35));
            line((0.4, 0.5), (0.25, 0.63));
            line((0.25, 0.63), (0.4, 0.76));
        }
        Icon::Document => {
            paint(ui, rect, Icon::File, color);
            for y in [0.45, 0.6, 0.75] {
                line((0.35, y), (0.68, y));
            }
        }

        Icon::Folder => {
            line((0.1, 0.25), (0.38, 0.25));

            line((0.38, 0.25), (0.48, 0.4));

            line((0.48, 0.4), (0.9, 0.4));

            line((0.9, 0.4), (0.82, 0.85));

            line((0.82, 0.85), (0.14, 0.85));

            line((0.14, 0.85), (0.1, 0.25));
        }
    }
}

pub fn button(ui: &mut egui::Ui, icon: Icon, label: &str, enabled: bool, tooltip: &str) -> bool {
    let label = strata_app::i18n::tr(label);
    let color = if enabled {
        ui.visuals().text_color()
    } else {
        ui.visuals().weak_text_color()
    };

    let text = ui
        .painter()
        .layout_no_wrap(label.into(), egui::FontId::proportional(13.0), color);

    let size = egui::vec2(text.size().x + 36.0, 30.0);

    let (rect, response) = ui.allocate_exact_size(
        size,
        if enabled {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        },
    );

    let visuals = ui.style().interact(&response);

    if enabled
        && (response.hovered() || response.is_pointer_button_down_on() || response.has_focus())
    {
        ui.painter().rect_filled(rect, 4.0, visuals.bg_fill);
    }

    let color = if enabled {
        ui.visuals().text_color()
    } else {
        ui.visuals().weak_text_color()
    };

    paint(
        ui,
        egui::Rect::from_min_size(rect.min + egui::vec2(6.0, 7.0), egui::vec2(16.0, 16.0)),
        icon,
        color,
    );

    ui.painter()
        .galley(rect.min + egui::vec2(28.0, 8.0), text, color);

    response.on_hover_text(strata_app::i18n::tr(tooltip)).clicked() && enabled
}
