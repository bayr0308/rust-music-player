use super::*;

const SETTINGS_VIEWPORT: &str = "mellow-settings";
const SETTINGS_BG: Color32 = Color32::from_rgb(32, 30, 37);
const SETTINGS_CARD: Color32 = Color32::from_rgb(43, 40, 48);
const SETTINGS_MUTED: Color32 = Color32::from_rgb(193, 190, 199);
const SETTINGS_ACCENT: Color32 = Color32::from_rgb(96, 205, 255);

impl PlayerApp {
    pub(super) fn open_settings(&mut self, tab: SettingsTab, ctx: &Context) {
        self.settings_tab = tab;
        self.output_devices = Audio::available_outputs();
        if self.settings_open {
            ctx.send_viewport_cmd_to(
                ViewportId::from_hash_of(SETTINGS_VIEWPORT),
                ViewportCommand::Focus,
            );
        }
        self.settings_open = true;
    }

    pub(super) fn settings_window(&mut self, ctx: &Context) {
        if !self.settings_open {
            return;
        }
        ctx.show_viewport_immediate(
            ViewportId::from_hash_of(SETTINGS_VIEWPORT),
            ViewportBuilder::default()
                .with_title("Настройки — Mellow")
                .with_inner_size([720.0, 580.0])
                .with_min_inner_size([620.0, 460.0])
                .with_transparent(false),
            |ctx, class| {
                if ctx.input(|i| i.viewport().close_requested()) {
                    self.settings_open = false;
                    return;
                }
                if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
                    if Popup::is_any_open(ctx) {
                        Popup::close_all(ctx);
                    } else if self.message.is_some() {
                        self.message = None;
                    } else {
                        self.settings_open = false;
                        return;
                    }
                }
                if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::O)) {
                    self.pick_folder(ctx);
                }
                if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::F5))
                    && let Some(index) = self.selected
                {
                    self.scan(index, ctx);
                }
                for file in ctx.input(|i| i.raw.dropped_files.clone()) {
                    if let Some(path) = file.path {
                        self.add_folder(path, ctx);
                    }
                }
                if class == ViewportClass::Embedded {
                    let mut open = self.settings_open;
                    egui::Window::new("Настройки")
                        .open(&mut open)
                        .default_size([720.0, 580.0])
                        .show(ctx, |ui| self.settings_content(ui, ctx));
                    self.settings_open = open;
                } else {
                    CentralPanel::default()
                        .frame(Frame::new().fill(SETTINGS_BG))
                        .show(ctx, |ui| self.settings_content(ui, ctx));
                }
                self.error_dialog(ctx);
            },
        );
    }

    fn settings_content(&mut self, ui: &mut Ui, ctx: &Context) {
        let style = ui.style_mut();
        style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(57, 54, 62);
        style.visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(57, 54, 62);
        style.visuals.widgets.inactive.bg_stroke =
            Stroke::new(1.0_f32, Color32::from_white_alpha(14));
        style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(65, 62, 71);
        style.visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(65, 62, 71);
        style.visuals.widgets.hovered.bg_stroke = style.visuals.widgets.inactive.bg_stroke;
        style.visuals.widgets.active.bg_stroke = style.visuals.widgets.inactive.bg_stroke;
        style.visuals.selection.bg_fill = Color32::from_rgb(65, 62, 71);
        style.spacing.button_padding = vec2(12.0, 7.0);
        style.spacing.interact_size.y = 32.0;
        for widget in [
            &mut style.visuals.widgets.inactive,
            &mut style.visuals.widgets.hovered,
            &mut style.visuals.widgets.active,
            &mut style.visuals.widgets.open,
        ] {
            widget.corner_radius = CornerRadius::same(4);
        }
        let r = ui.max_rect();
        let sidebar = Rect::from_min_max(
            r.min + vec2(12.0, 24.0),
            pos2(r.left() + 172.0, r.bottom() - 20.0),
        );
        ui.scope_builder(UiBuilder::new().max_rect(sidebar), |ui| {
            for (tab, label, symbol) in [
                (SettingsTab::Folders, "Папки", icons::FOLDER_SIMPLE),
                (SettingsTab::Sound, "Звук", icons::SPEAKER_HIGH),
            ] {
                let (row, response) =
                    ui.allocate_exact_size(vec2(sidebar.width(), 40.0), Sense::click());
                let selected = self.settings_tab == tab;
                response.widget_info(|| {
                    WidgetInfo::selected(WidgetType::SelectableLabel, true, selected, label)
                });
                if selected || response.hovered() || response.has_focus() {
                    ui.painter().rect_filled(row, 4.0, SETTINGS_CARD);
                }
                if selected {
                    ui.painter().rect_filled(
                        Rect::from_center_size(
                            pos2(row.left() + 2.0, row.center().y),
                            vec2(3.0, 18.0),
                        ),
                        2.0,
                        SETTINGS_ACCENT,
                    );
                }
                if response.has_focus() {
                    ui.painter().rect_stroke(
                        row,
                        4.0,
                        Stroke::new(1.0_f32, SETTINGS_ACCENT),
                        StrokeKind::Inside,
                    );
                }
                icon(
                    ui.painter(),
                    pos2(row.left() + 24.0, row.center().y),
                    symbol,
                    18.0,
                    FG,
                );
                text(
                    ui.painter(),
                    pos2(row.left() + 45.0, row.center().y),
                    Align2::LEFT_CENTER,
                    label,
                    14.0,
                    FG,
                );
                if response.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                    self.settings_tab = tab;
                }
            }
        });
        let content = Rect::from_min_max(r.min + vec2(196.0, 24.0), r.max - vec2(24.0, 20.0));
        ui.scope_builder(UiBuilder::new().max_rect(content), |ui| {
            ui.set_clip_rect(content);
            let (header, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), 36.0), Sense::hover());
            heading(
                ui.painter(),
                header,
                if self.settings_tab == SettingsTab::Sound {
                    "Звук"
                } else {
                    "Папки"
                },
                26.0,
            );
            if self.settings_tab == SettingsTab::Folders {
                let add = Rect::from_min_size(
                    pos2(header.right() - 90.0, header.top()),
                    vec2(90.0, 32.0),
                );
                let response = ui
                    .add_enabled_ui(!self.picking, |ui| ui.put(add, Button::new("Добавить")))
                    .inner;
                if response.has_focus() {
                    ui.painter().rect_stroke(
                        add,
                        4.0,
                        Stroke::new(1.0_f32, SETTINGS_ACCENT),
                        StrokeKind::Inside,
                    );
                }
                if response.clicked() {
                    self.pick_folder(ctx);
                }
            }
            ui.add_space(16.0);
            Self::scrollbar_gutter(ui);
            ScrollArea::vertical()
                .id_salt((
                    "settings-content",
                    self.settings_tab == SettingsTab::Folders,
                ))
                .auto_shrink([false, false])
                .show(ui, |ui| match self.settings_tab {
                    SettingsTab::Sound => self.sound_settings(ui),
                    SettingsTab::Folders => self.folder_settings(ui, ctx),
                });
        });
    }

    fn settings_card() -> Frame {
        Frame::new()
            .fill(SETTINGS_CARD)
            .corner_radius(8)
            .stroke(Stroke::new(1.0_f32, Color32::from_white_alpha(8)))
            .inner_margin(16)
    }

    fn sound_settings(&mut self, ui: &mut Ui) {
        Self::settings_card().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(RichText::new("Устройство вывода").size(14.0));
            ui.add_space(6.0);
            let mut selected = self.settings.output_device.clone();
            ui.horizontal(|ui| {
                let width = ui.available_width() - 40.0;
                let (selector, _) = ui.allocate_exact_size(vec2(width, 32.0), Sense::hover());
                let response = fluent_select_button(
                    ui,
                    selector,
                    ui.id().with("output-device"),
                    selected.as_deref().unwrap_or("По умолчанию Windows"),
                    "Устройство вывода",
                    None,
                );
                fluent_popup(&response).show(|ui| {
                    Self::scrollbar_gutter(ui);
                    ScrollArea::vertical()
                        .id_salt("outputs-scroll")
                        .max_height(280.0)
                        .show(ui, |ui| {
                            if fluent_option(ui, selected.is_none(), "По умолчанию Windows")
                                .clicked()
                            {
                                selected = None;
                                ui.close();
                            }
                            if let Ok(outputs) = &self.output_devices {
                                for name in outputs {
                                    if fluent_option(ui, selected.as_ref() == Some(name), name)
                                        .on_hover_text(name)
                                        .clicked()
                                    {
                                        selected = Some(name.clone());
                                        ui.close();
                                    }
                                }
                            }
                        });
                });
                let (r, _) = ui.allocate_exact_size(vec2(32.0, 32.0), Sense::hover());
                if icon_button(
                    ui,
                    r,
                    icons::ARROWS_CLOCKWISE,
                    "Обновить список устройств",
                    false,
                    false,
                )
                .clicked()
                {
                    self.output_devices = Audio::available_outputs();
                }
            });
            if selected != self.settings.output_device {
                self.select_output(selected);
            }
            if let Err(error) = &self.output_devices {
                ui.add(Label::new(RichText::new(error).color(SETTINGS_MUTED)).wrap());
            }
        });
        ui.add_space(6.0);
        Self::settings_card().show(ui, |ui| {
            let (equalizer, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), 310.0), Sense::hover());
            self.equalizer_panel(ui, equalizer);
        });
    }

    fn folder_settings(&mut self, ui: &mut Ui, ctx: &Context) {
        if self.playlists.is_empty() {
            Self::settings_card().show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.label("Добавьте папку с музыкой или перетащите её в это окно.");
            });
        }
        let mut select = None;
        let mut refresh = None;
        let mut remove = None;
        for (index, folder) in self.playlists.iter().enumerate() {
            ui.push_id(&folder.path, |ui| {
                let (card, _) =
                    ui.allocate_exact_size(vec2(ui.available_width(), 104.0), Sense::hover());
                ui.painter().rect_filled(card, 8.0, SETTINGS_CARD);
                ui.painter().rect_stroke(
                    card,
                    8.0,
                    Stroke::new(1.0_f32, Color32::from_white_alpha(8)),
                    StrokeKind::Inside,
                );
                let selected = self.selected == Some(index);
                let actions_left = card.right() - 88.0;
                let response = ui.interact(
                    Rect::from_min_max(card.min, pos2(actions_left - 8.0, card.bottom())),
                    ui.id().with("select"),
                    Sense::click(),
                );
                response.widget_info(|| {
                    WidgetInfo::selected(WidgetType::SelectableLabel, true, selected, &folder.name)
                });
                if response.has_focus() {
                    ui.painter().rect_stroke(
                        response.rect.shrink(2.0),
                        6.0,
                        Stroke::new(1.0_f32, SETTINGS_ACCENT),
                        StrokeKind::Inside,
                    );
                }
                icon(
                    ui.painter(),
                    pos2(card.left() + 28.0, card.center().y),
                    icons::FOLDER_SIMPLE,
                    22.0,
                    if selected { SETTINGS_ACCENT } else { FG },
                );
                let left = card.left() + 52.0;
                let width = (actions_left - 12.0 - left).max(1.0);
                heading(
                    ui.painter(),
                    Rect::from_min_size(pos2(left, card.top() + 16.0), vec2(width, 22.0)),
                    &folder.name,
                    14.0,
                );
                let path = display_path(&folder.path);
                ellipsis(
                    ui.painter(),
                    Rect::from_min_size(pos2(left, card.top() + 42.0), vec2(width, 18.0)),
                    &path,
                    11.0,
                    SETTINGS_MUTED,
                );
                let status = folder.error.clone().unwrap_or_else(|| {
                    if folder.loading {
                        "Читаем папку…".into()
                    } else {
                        track_count(folder.tracks.len())
                    }
                });
                ellipsis(
                    ui.painter(),
                    Rect::from_min_size(pos2(left, card.top() + 66.0), vec2(width, 18.0)),
                    &status,
                    11.0,
                    SETTINGS_MUTED,
                );
                if response
                    .on_hover_text(format!("{}\n{}\n{}", folder.name, path, status))
                    .clicked()
                {
                    select = Some(index);
                }
                let refresh_rect = Rect::from_min_size(
                    pos2(actions_left, card.center().y - 16.0),
                    vec2(32.0, 32.0),
                );
                if ui
                    .add_enabled_ui(!folder.loading, |ui| {
                        icon_button(
                            ui,
                            refresh_rect,
                            icons::ARROWS_CLOCKWISE,
                            "Обновить папку",
                            false,
                            false,
                        )
                    })
                    .inner
                    .clicked()
                {
                    refresh = Some(index);
                }
                if icon_button(
                    ui,
                    refresh_rect.translate(vec2(40.0, 0.0)),
                    icons::TRASH,
                    "Убрать из плеера",
                    false,
                    false,
                )
                .clicked()
                {
                    remove = Some(index);
                }
            });
            ui.add_space(4.0);
        }
        if let Some(index) = select {
            self.selected = Some(index);
            self.filter.clear();
            self.changed();
        }
        if let Some(index) = refresh {
            self.scan(index, ctx);
        }
        if let Some(index) = remove {
            self.remove_folder(index);
        }
    }

    pub(super) fn error_dialog(&mut self, ctx: &Context) {
        if let Some(message) = self.message.clone() {
            egui::Window::new("Не удалось выполнить действие")
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .default_width((ctx.content_rect().width() - 48.0).min(320.0))
                .show(ctx, |ui| {
                    ui.label(message);
                    if ui.button("Понятно").clicked() {
                        self.message = None;
                    }
                });
        }
    }
    fn equalizer_panel(&mut self, ui: &mut Ui, r: Rect) {
        let mut updated = false;
        heading(
            ui.painter(),
            Rect::from_min_size(r.min, vec2(r.width() - 60.0, 24.0)),
            "Эквалайзер",
            16.0,
        );
        let toggle = Rect::from_min_size(pos2(r.right() - 40.0, r.top() + 2.0), vec2(40.0, 22.0));
        let response = ui.interact(
            toggle.expand(4.0),
            ui.id().with("equalizer-enabled"),
            Sense::click(),
        );
        response.widget_info(|| {
            WidgetInfo::selected(
                WidgetType::Checkbox,
                true,
                self.settings.equalizer.enabled,
                "Эквалайзер",
            )
        });
        if response.clicked() {
            self.settings.equalizer.enabled = !self.settings.equalizer.enabled;
            updated = true;
        }
        let enabled = self.settings.equalizer.enabled;
        ui.painter().rect_filled(
            toggle,
            11.0,
            if enabled {
                SETTINGS_ACCENT
            } else {
                Color32::from_rgb(60, 57, 65)
            },
        );
        ui.painter().rect_stroke(
            toggle,
            11.0,
            Stroke::new(
                1.0_f32,
                if response.has_focus() {
                    FG
                } else {
                    SETTINGS_MUTED
                },
            ),
            StrokeKind::Inside,
        );
        ui.painter().circle_filled(
            pos2(
                if enabled {
                    toggle.right() - 11.0
                } else {
                    toggle.left() + 11.0
                },
                toggle.center().y,
            ),
            6.0,
            if enabled { SETTINGS_BG } else { FG },
        );
        response
            .on_hover_cursor(CursorIcon::PointingHand)
            .on_hover_text(if enabled {
                "Выключить эквалайзер"
            } else {
                "Включить эквалайзер"
            });
        ui.add_enabled_ui(enabled, |ui| {
            for i in 0..5 {
                let x = r.left() + (i as f32 + 0.5) * r.width() / 5.0;
                text(
                    ui.painter(),
                    pos2(x, r.top() + 56.0),
                    Align2::CENTER_CENTER,
                    format!("{:+.0}", self.settings.equalizer.gains[i]),
                    12.0,
                    FG,
                );
                updated |= equalizer_slider(
                    ui,
                    Rect::from_min_size(pos2(x - 16.0, r.top() + 76.0), vec2(32.0, 148.0)),
                    &mut self.settings.equalizer.gains[i],
                    &format!("{} Гц", mellow::equalizer::FREQUENCIES[i]),
                )
                .changed();
                text(
                    ui.painter(),
                    pos2(x, r.top() + 240.0),
                    Align2::CENTER_CENTER,
                    ["60 Гц", "250 Гц", "1 кГц", "4 кГц", "12 кГц"][i],
                    11.0,
                    SETTINGS_MUTED,
                );
            }
        });
        ui.scope_builder(
            UiBuilder::new().max_rect(Rect::from_min_size(
                r.min + vec2(0.0, 274.0),
                vec2(r.width(), 32.0),
            )),
            |ui| {
                ui.horizontal(|ui| {
                    for (name, gains) in [
                        ("Ровно", [0.0; 5]),
                        ("Бас", [6.0, 3.0, 0.0, 0.0, 0.0]),
                        ("Голос", [-2.0, 0.0, 3.0, 2.0, -1.0]),
                    ] {
                        if ui
                            .add(Button::selectable(
                                enabled && self.settings.equalizer.gains == gains,
                                name,
                            ))
                            .clicked()
                        {
                            self.settings.equalizer.gains = gains;
                            self.settings.equalizer.enabled = true;
                            updated = true;
                        }
                    }
                });
            },
        );
        if updated {
            self.audio.set_equalizer(self.settings.equalizer.clone());
            self.changed();
        }
    }
}
