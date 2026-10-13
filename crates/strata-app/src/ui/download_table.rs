use super::{
    icons::{self, Icon},
    theme,
    ui_state::*,
};

use eframe::egui::{self, Color32};

use std::cmp::Ordering;

use strata_core::{Job, Status};

pub fn rebuild(state: &mut State, jobs: &[Job]) {
    if !state.dirty {
        return;
    }

    state.dirty = false;

    let search = state.search.to_lowercase();

    state.visible = jobs
        .iter()
        .enumerate()
        .filter(|(_, j)| {
            state.filter.matches(j)
                && (search.is_empty()
                    || format!("{} {} {}", name(j), j.url, j.category)
                        .to_lowercase()
                        .contains(&search))
        })
        .map(|(i, _)| i)
        .collect();

    let column = state.sort;

    let descending = state.descending;

    state.visible.sort_by(|a, b| {
        let (a, b) = (&jobs[*a], &jobs[*b]);

        let order = match column {
            Column::Name => name(a).to_lowercase().cmp(&name(b).to_lowercase()),

            Column::Size => a.size.cmp(&b.size),

            Column::Status => status(a.status).cmp(status(b.status)),

            Column::Progress => percent(a).total_cmp(&percent(b)),

            Column::Speed => a.speed.total_cmp(&b.speed),

            Column::Eta => eta(a)
                .unwrap_or(f64::INFINITY)
                .total_cmp(&eta(b).unwrap_or(f64::INFINITY)),

            Column::Connections => a.connections.cmp(&b.connections),

            Column::Added => a.created.cmp(&b.created),

            Column::Category => a.category.cmp(&b.category),

            Column::Source => a.url.cmp(&b.url),

            Column::Downloaded => a.downloaded.cmp(&b.downloaded),
        };

        let order = if descending { order.reverse() } else { order };

        if order == Ordering::Equal {
            if descending {
                b.sequence.cmp(&a.sequence)
            } else {
                a.sequence.cmp(&b.sequence)
            }
        } else {
            order
        }
    });

    let visible: std::collections::HashSet<_> =
        state.visible.iter().map(|i| jobs[*i].id.as_str()).collect();

    state
        .selection
        .ids
        .retain(|id| visible.contains(id.as_str()));

    if state
        .selection
        .cursor
        .as_ref()
        .is_some_and(|id| !visible.contains(id.as_str()))
    {
        state.selection.cursor = None;

        state.selection.anchor = None;
    }
}

pub fn show(ui: &mut egui::Ui, state: &mut State, jobs: &[Job]) -> Vec<Action> {
    rebuild(state, jobs);

    let mut actions = Vec::new();

    let ordered: Vec<&str> = state.visible.iter().map(|i| jobs[*i].id.as_str()).collect();

    let ctx = ui.ctx().clone();

    if state.table_keyboard(&ctx) {
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::A)) {
            state.selection.all(&ordered);
        }

        for (key, delta) in [
            (egui::Key::ArrowDown, 1isize),
            (egui::Key::ArrowUp, -1),
            (egui::Key::PageDown, 12),
            (egui::Key::PageUp, -12),
        ] {
            let key_modifiers = ctx.input(|i| {
                i.events.iter().find_map(|event| match event {
                    egui::Event::Key {
                        key: pressed,

                        pressed: true,

                        modifiers,
                        ..
                    } if *pressed == key => Some(*modifiers),

                    _ => None,
                })
            });

            if let Some(key_modifiers) = key_modifiers
                && !ordered.is_empty()
            {
                let current = state
                    .selection
                    .cursor
                    .as_ref()
                    .and_then(|id| ordered.iter().position(|other| *other == id))
                    .unwrap_or(if delta > 0 { 0 } else { ordered.len() - 1 });

                let next = (current as isize + delta).clamp(0, ordered.len() as isize - 1) as usize;

                state.selection.click(
                    ordered[next],
                    &ordered,
                    key_modifiers.ctrl || key_modifiers.command,
                    key_modifiers.shift,
                );

                state.scroll_to = Some(next);
            }
        }

        for (key, index) in [
            (egui::Key::Home, 0),
            (egui::Key::End, ordered.len().saturating_sub(1)),
        ] {
            let shift = ctx.input(|i| {
                i.events.iter().find_map(|event| match event {
                    egui::Event::Key {
                        key: pressed,

                        pressed: true,

                        modifiers,
                        ..
                    } if *pressed == key => Some(modifiers.shift),

                    _ => None,
                })
            });

            if let Some(shift) = shift
                && !ordered.is_empty()
            {
                state
                    .selection
                    .click(ordered[index], &ordered, false, shift);

                state.scroll_to = Some(index);
            }
        }
    }

    let columns = state.prefs.columns.clone();

    let total_width: f32 = columns.iter().map(|column| column.width() + 7.0).sum();

    egui::ScrollArea::horizontal()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;

            // Row interactions own clicks; text selection would intercept double/right clicks.
            ui.style_mut().interaction.selectable_labels = false;

            ui.set_min_width(total_width.max(ui.available_width()));

            let height = (ui.available_height() - 28.0).max(0.0);

            let mut table = egui_extras::TableBuilder::new(ui)
                .id_salt(("downloads", &columns))
                .striped(true)
                .resizable(true)
                .max_scroll_height(height)
                .sense(egui::Sense::click())
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .auto_shrink([false, false]);

            if let Some(row) = state.scroll_to.take() {
                table = table.scroll_to_row(row, None);
            }

            for column in &columns {
                table = table.column(if *column == Column::Name {
                    egui_extras::Column::remainder().at_least(120.0).clip(true)
                } else {
                    egui_extras::Column::initial(column.width())
                        .at_least(58.0)
                        .clip(true)
                });
            }

            table
                .header(28.0, |mut header| {
                    for column in &columns {
                        header.col(|ui| {
                            let label = if state.sort == *column {
                                format!(
                                    "{} {}",
                                    text(state.prefs.swedish, column.title()),
                                    if state.descending { "↓" } else { "↑" }
                                )
                            } else {
                                text(state.prefs.swedish, column.title()).into()
                            };

                            if ui
                                .selectable_label(state.sort == *column, label)
                                .on_hover_text("Click to sort. Drag the column boundary to resize.")
                                .clicked()
                            {
                                if state.sort == *column {
                                    state.descending = !state.descending;
                                } else {
                                    state.sort = *column;

                                    state.descending = false;
                                }

                                state.dirty = true;
                            }
                        });
                    }
                })
                .body(|body| {
                    body.rows(32.0, state.visible.len(), |mut row| {
                        let job = &jobs[state.visible[row.index()]];

                        row.set_selected(state.selection.ids.contains(&job.id));

                        for column in &columns {
                            row.col(|ui| {
                                let rect = ui.max_rect();

                                match column {
                                    Column::Name => {
                                        let (rect, _) = ui.allocate_exact_size(
                                            egui::vec2(17.0, 17.0),
                                            egui::Sense::hover(),
                                        );

                                        let category = strata_core::category_for(&name(job));

                                        let color = match category {
                                            "Archives" => Color32::from_rgb(177, 146, 79),

                                            "Video" => Color32::from_rgb(138, 150, 191),

                                            "Audio" => Color32::from_rgb(129, 163, 137),

                                            "Images" => Color32::from_rgb(122, 163, 184),

                                            "Applications" => Color32::from_rgb(151, 152, 161),

                                            _ => ui.visuals().text_color(),
                                        };

                                        let icon = match category {
                                            "Archives" => Icon::Archive,
                                            "Audio" => Icon::Audio,
                                            "Video" => Icon::Video,
                                            "Images" => Icon::Image,
                                            "Applications" => Icon::Application,
                                            "Documents" => Icon::Document,
                                            _ => Icon::File,
                                        };
                                        icons::paint(ui, rect, icon, color);

                                        ui.add(egui::Label::new(name(job)).truncate())
                                            .on_hover_text(job.destination.to_string_lossy());
                                    }

                                    Column::Size => {
                                        ui.label(
                                            job.size
                                                .map(|n| bytes(n as f64))
                                                .unwrap_or_else(|| "Unknown".into()),
                                        );
                                    }

                                    Column::Status => {
                                        ui.colored_label(
                                            if job.status == Status::Failed {
                                                Color32::from_rgb(199, 111, 104)
                                            } else {
                                                ui.visuals().text_color()
                                            },
                                            text(state.prefs.swedish, status(job.status)),
                                        )
                                        .on_hover_text(job.error.as_deref().unwrap_or(""));
                                    }

                                    Column::Progress => {
                                        let width = (ui.available_width() - 8.0).max(15.0);

                                        let (r, _) = ui.allocate_exact_size(
                                            egui::vec2(width, 24.0),
                                            egui::Sense::hover(),
                                        );

                                        ui.painter().text(
                                            r.left_top(),
                                            egui::Align2::LEFT_TOP,
                                            format!("{:.1}%", percent(job) * 100.0),
                                            egui::FontId::proportional(12.0),
                                            ui.visuals().text_color(),
                                        );

                                        let rail = egui::Rect::from_min_size(
                                            egui::pos2(r.left(), r.bottom() - 4.0),
                                            egui::vec2(width, 3.0),
                                        );

                                        ui.painter().rect_filled(
                                            rail,
                                            1.0,
                                            ui.visuals().widgets.noninteractive.bg_stroke.color,
                                        );

                                        ui.painter().rect_filled(
                                            egui::Rect::from_min_size(
                                                rail.min,
                                                egui::vec2(width * percent(job), 3.0),
                                            ),
                                            1.0,
                                            theme::ACCENT,
                                        );
                                    }

                                    Column::Speed => {
                                        ui.label(if job.status == Status::Downloading {
                                            format!("{}/s", bytes(job.speed))
                                        } else {
                                            "—".into()
                                        });
                                    }

                                    Column::Eta => {
                                        ui.label(
                                            eta(job).map(duration).unwrap_or_else(|| "—".into()),
                                        );
                                    }

                                    Column::Connections => {
                                        ui.label(if job.connections > 0 {
                                            job.connections.to_string()
                                        } else {
                                            "—".into()
                                        });
                                    }

                                    Column::Added => {
                                        ui.label(added(job.created));
                                    }

                                    Column::Category => {
                                        ui.add(egui::Label::new(&job.category).truncate());
                                    }

                                    Column::Source => {
                                        ui.add(egui::Label::new(&job.url).truncate())
                                            .on_hover_text(&job.url);
                                    }

                                    Column::Downloaded => {
                                        ui.label(bytes(job.downloaded as f64));
                                    }
                                }

                                ui.painter().line_segment(
                                    [rect.right_top(), rect.right_bottom()],
                                    egui::Stroke::new(
                                        0.5,
                                        ui.visuals().widgets.noninteractive.bg_stroke.color,
                                    ),
                                );
                            });
                        }

                        let response = row.response();

                        if response.clicked() {
                            let (ctrl, shift) = modifiers(&ctx);

                            state.selection.click(&job.id, &ordered, ctrl, shift);

                            response.request_focus();
                        }

                        if response.secondary_clicked() && !state.selection.ids.contains(&job.id) {
                            state.selection.click(&job.id, &ordered, false, false);
                        }

                        if response.double_clicked()
                            || (response.hovered()
                                && ctx.input(|i| {
                                    i.pointer
                                        .button_double_clicked(egui::PointerButton::Primary)
                                }))
                        {
                            actions.push(
                                if job.status == Status::Completed && state.prefs.open_completed {
                                    Action::OpenFile(job.id.clone())
                                } else {
                                    Action::Details(job.id.clone())
                                },
                            );
                        }

                        response.context_menu(|ui| {
                            for (label, action) in [
                                ("Download details", Action::Details(job.id.clone())),
                                ("Resume selected", Action::Resume),
                                ("Pause selected", Action::Pause),
                                ("Stop selected", Action::Stop),
                                ("Remove selected…", Action::Remove),
                            ] {
                                if ui.button(label).clicked() {
                                    actions.push(action);

                                    ui.close();
                                }
                            }

                            ui.separator();

                            if ui
                                .add_enabled(
                                    job.status == Status::Completed,
                                    egui::Button::new("Open file"),
                                )
                                .clicked()
                            {
                                actions.push(Action::OpenFile(job.id.clone()));

                                ui.close();
                            }

                            if ui.button("Open folder").clicked() {
                                actions.push(Action::OpenFolder(job.id.clone()));

                                ui.close();
                            }
                        });
                    });
                });
        });

    if state.visible.is_empty() {
        let rect = ui.max_rect();

        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            if jobs.is_empty() {
                "No downloads yet. Choose Add URL or press Ctrl+N."
            } else {
                "No downloads match this view."
            },
            egui::FontId::proportional(13.0),
            ui.visuals().weak_text_color(),
        );
    }

    actions
}

#[cfg(test)]
mod tests {

    use super::*;

    fn records() -> (tempfile::TempDir, Vec<Job>) {
        let dir = tempfile::tempdir().unwrap();

        let manager =
            strata_core::Manager::open(&dir.path().join("state"), Default::default()).unwrap();

        let jobs = ["z.zip", "a.pdf", "b.zip"]
            .into_iter()
            .map(|name| {
                manager
                    .add(
                        "https://example.com/file",
                        &dir.path().join("out"),
                        Some(name),
                    )
                    .unwrap()
            })
            .collect();

        (dir, jobs)
    }

    #[test]
    fn sorting_and_filtering_clear_hidden_selection() {
        let (_dir, mut jobs) = records();

        jobs[0].size = Some(900);

        jobs[1].size = Some(100);

        jobs[2].size = Some(30);

        let mut state = State::new(Default::default());

        state.sort = Column::Size;

        state.descending = false;

        rebuild(&mut state, &jobs);

        assert_eq!(state.visible, vec![2, 1, 0]);

        state
            .selection
            .click(&jobs[1].id, &[&jobs[1].id], false, false);

        state.filter = Filter::Category("Archives".into());

        state.dirty = true;

        rebuild(&mut state, &jobs);

        assert_eq!(state.visible, vec![2, 0]);

        assert!(state.selection.ids.is_empty());

        assert!(state.selection.cursor.is_none());

        state.search = "z.zip".into();

        state.dirty = true;

        rebuild(&mut state, &jobs);

        assert_eq!(state.visible, vec![0]);
    }

    fn frame(
        ctx: &egui::Context,

        state: &mut State,

        jobs: &[Job],

        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1100.0, 600.0),
                )),

                events,

                ..Default::default()
            },
            |root| {
                egui::CentralPanel::default().show(root, |ui| {
                    show(ui, state, jobs);
                });
            },
        );

        output.textures_delta.clear();

        output
    }

    fn key(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
        egui::Event::Key {
            key,

            physical_key: None,

            pressed: true,

            repeat: false,

            modifiers,
        }
    }

    #[test]
    fn focused_row_accepts_keyboard_but_dialog_and_search_do_not() {
        let (_dir, jobs) = records();

        let ctx = egui::Context::default();

        let mut state = State::new(Default::default());

        frame(&ctx, &mut state, &jobs, vec![]);

        ctx.memory_mut(|memory| memory.request_focus(egui::Id::new("focused_row")));

        frame(
            &ctx,
            &mut state,
            &jobs,
            vec![key(egui::Key::A, egui::Modifiers::CTRL)],
        );

        assert_eq!(state.selection.ids.len(), 3);
        let ordered: Vec<_> = state.visible.iter().map(|i| jobs[*i].id.as_str()).collect();
        state.selection.click(ordered[0], &ordered, false, false);
        frame(
            &ctx,
            &mut state,
            &jobs,
            vec![key(egui::Key::ArrowDown, egui::Modifiers::SHIFT)],
        );
        assert_eq!(
            state.selection.ids.len(),
            2,
            "Shift uses the key event's modifiers, even after native key release"
        );

        state.selection.ids.clear();

        state.add.open = true;

        frame(
            &ctx,
            &mut state,
            &jobs,
            vec![key(egui::Key::A, egui::Modifiers::CTRL)],
        );

        assert!(state.selection.ids.is_empty());

        state.add.open = false;

        state.search_focused = true;

        frame(
            &ctx,
            &mut state,
            &jobs,
            vec![key(egui::Key::ArrowDown, egui::Modifiers::NONE)],
        );

        assert!(state.selection.ids.is_empty());
    }

    #[test]
    fn five_thousand_rows_are_virtualized() {
        let (_dir, sample) = records();

        let jobs: Vec<_> = (0..5000)
            .map(|i| {
                let mut j = sample[0].clone();

                j.id = format!("unit-{i}");

                j.sequence = i;

                j
            })
            .collect();

        let ctx = egui::Context::default();

        let mut state = State::new(Default::default());

        frame(&ctx, &mut state, &jobs, vec![]);

        let output = frame(&ctx, &mut state, &jobs, vec![]);

        assert_eq!(state.visible.len(), 5000);

        assert!(
            output.shapes.len() < 1000,
            "offscreen rows must not render: {} shapes",
            output.shapes.len()
        );
    }
    #[test]
    fn actual_row_pointer_events_select_range_open_details_and_context_menu() {
        let (_dir, jobs) = records();
        let ctx = egui::Context::default();
        let mut state = State::new(Default::default());
        let mut render = |time, events| {
            let mut actions = Vec::new();
            let mut output = ctx.run_ui(
                egui::RawInput {
                    time: Some(time),
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1100.0, 600.0),
                    )),
                    events,
                    ..Default::default()
                },
                |root| {
                    egui::CentralPanel::default().show(root, |ui| {
                        actions = show(ui, &mut state, &jobs);
                    });
                },
            );
            output.textures_delta.clear();
            (actions, state.selection.ids.len())
        };
        render(0.0, vec![]);
        let event = |y, button, pressed, modifiers| egui::Event::PointerButton {
            pos: egui::pos2(200.0, y),
            button,
            pressed,
            modifiers,
        };
        let primary = egui::PointerButton::Primary;
        render(
            0.1,
            vec![
                egui::Event::PointerMoved(egui::pos2(200.0, 50.0)),
                event(50.0, primary, true, egui::Modifiers::NONE),
            ],
        );
        assert_eq!(
            render(
                0.12,
                vec![event(50.0, primary, false, egui::Modifiers::NONE)]
            )
            .1,
            1
        );
        render(0.2, vec![event(50.0, primary, true, egui::Modifiers::NONE)]);
        assert!(
            render(
                0.22,
                vec![event(50.0, primary, false, egui::Modifiers::NONE)]
            )
            .0
            .iter()
            .any(|a| matches!(a, Action::Details(_)))
        );
        render(
            1.0,
            vec![
                egui::Event::PointerMoved(egui::pos2(200.0, 114.0)),
                event(114.0, primary, true, egui::Modifiers::SHIFT),
            ],
        );
        assert_eq!(
            render(
                1.02,
                vec![event(114.0, primary, false, egui::Modifiers::SHIFT)]
            )
            .1,
            3
        );
        render(
            1.5,
            vec![
                egui::Event::PointerMoved(egui::pos2(200.0, 82.0)),
                event(82.0, primary, true, egui::Modifiers::CTRL),
            ],
        );
        assert_eq!(
            render(
                1.52,
                vec![event(82.0, primary, false, egui::Modifiers::CTRL)]
            )
            .1,
            2
        );
        render(
            2.0,
            vec![event(
                82.0,
                egui::PointerButton::Secondary,
                true,
                egui::Modifiers::NONE,
            )],
        );
        render(
            2.02,
            vec![event(
                82.0,
                egui::PointerButton::Secondary,
                false,
                egui::Modifiers::NONE,
            )],
        );
        assert!(egui::Popup::is_any_open(&ctx));
    }
}
