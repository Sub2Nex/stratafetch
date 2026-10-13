use eframe::egui::{self, Color32, FontId, Stroke};

pub const ACCENT: Color32 = Color32::from_rgb(44, 151, 158);

pub fn apply(ctx: &egui::Context, dark: bool) {
    let mut style = egui::Style {
        visuals: if dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        },
        ..Default::default()
    };

    style.visuals.panel_fill = if dark {
        Color32::from_rgb(36, 39, 43)
    } else {
        Color32::from_rgb(246, 247, 249)
    };

    style.visuals.window_fill = if dark {
        Color32::from_rgb(43, 46, 51)
    } else {
        Color32::from_rgb(252, 252, 253)
    };

    style.visuals.extreme_bg_color = if dark {
        Color32::from_rgb(31, 34, 38)
    } else {
        Color32::WHITE
    };

    style.visuals.faint_bg_color = if dark {
        Color32::from_rgb(41, 44, 48)
    } else {
        Color32::from_rgb(243, 246, 248)
    };

    style.visuals.override_text_color = Some(if dark {
        Color32::from_rgb(226, 230, 235)
    } else {
        Color32::from_rgb(35, 43, 51)
    });

    style.visuals.selection.bg_fill = if dark {
        Color32::from_rgb(49, 80, 87)
    } else {
        Color32::from_rgb(209, 234, 237)
    };

    style.visuals.selection.stroke = Stroke::new(
        1.0,
        if dark {
            Color32::WHITE
        } else {
            Color32::from_rgb(21, 58, 63)
        },
    );

    style.visuals.widgets.hovered.bg_fill = if dark {
        Color32::from_rgb(57, 62, 68)
    } else {
        Color32::from_rgb(225, 231, 236)
    };

    style.visuals.widgets.active.bg_fill = if dark {
        Color32::from_rgb(68, 76, 82)
    } else {
        Color32::from_rgb(211, 223, 230)
    };
    style.visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);

    style.spacing.item_spacing = egui::vec2(7.0, 6.0);

    style.spacing.button_padding = egui::vec2(8.0, 5.0);

    style.spacing.interact_size = egui::vec2(28.0, 26.0);

    for (text, size) in [
        (egui::TextStyle::Body, 13.0),
        (egui::TextStyle::Button, 13.0),
        (egui::TextStyle::Small, 11.0),
        (egui::TextStyle::Heading, 17.0),
    ] {
        style.text_styles.insert(text, FontId::proportional(size));
    }

    let theme = if dark {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    };

    ctx.set_theme(theme);

    ctx.set_style_of(theme, style);
}
