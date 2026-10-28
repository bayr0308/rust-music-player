use super::*;

impl PlayerApp {
    pub(super) fn interface(&mut self, ctx: &Context) {
        CentralPanel::default().frame(Frame::NONE).show(ctx, |ui| {
            let whole = ui.max_rect();
            ui.painter().rect_filled(
                whole,
                0.0,
                if self.acrylic {
                    Color32::from_rgba_unmultiplied(
                        self.cover_tint[0],
                        self.cover_tint[1],
                        self.cover_tint[2],
                        155,
                    )
                } else {
                    Color32::from_rgb(self.cover_tint[0], self.cover_tint[1], self.cover_tint[2])
                },
            );
            let r = whole;
            const MARGIN: f32 = 16.0;
            let toolbar = Rect::from_min_size(
                pos2(r.left() + MARGIN, r.top() + 4.0),
                vec2(r.width() - 2.0 * MARGIN, 32.0),
            );
            self.folder_toolbar(ui, toolbar, ctx);
            let footer = Rect::from_min_max(
                pos2(r.left() + MARGIN, r.bottom() - 36.0),
                pos2(r.right() - MARGIN, r.bottom() - 4.0),
            );
            let body = Rect::from_min_max(
                pos2(r.left() + MARGIN, r.top() + 44.0),
                pos2(r.right() - MARGIN, footer.top() - 6.0),
            );
            match self.view {
                View::Player => self.now_playing(ui, body, ctx),
                View::Tracks => self.track_list(ui, body, ctx),
                View::Folders => self.folder_list(ui, body, ctx),
                View::Equalizer => self.equalizer_panel(ui, body, ctx),
            }
            self.footer(ui, footer, ctx);
        });
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

    fn folder_toolbar(&mut self, ui: &mut Ui, r: Rect, ctx: &Context) {
        let selector = Rect::from_min_max(r.min, pos2(r.right() - 40.0, r.bottom()));
        let folder = self.selected.and_then(|i| self.playlists.get(i));
        let name = folder.map_or("Выбрать папку", |f| f.name.as_str());
        let response = ui.interact(selector, Id::new("folder-selector"), Sense::click());
        response.widget_info(|| {
            WidgetInfo::labeled(WidgetType::Button, true, format!("Папки: {name}"))
        });
        if response.hovered() || response.has_focus() || self.view == View::Folders {
            ui.painter().rect_filled(selector, 5.0, LINE);
        }
        icon(
            ui.painter(),
            pos2(selector.left() + 10.0, selector.center().y),
            icons::FOLDER_SIMPLE,
            20.0,
            FG,
        );
        ellipsis(
            ui.painter(),
            Rect::from_min_max(
                pos2(selector.left() + 30.0, selector.top()),
                pos2(selector.right() - 26.0, selector.bottom()),
            ),
            name,
            15.0,
            FG,
        );
        icon(
            ui.painter(),
            pos2(selector.right() - 12.0, selector.center().y),
            icons::CARET_DOWN,
            12.0,
            MUTED,
        );
        if response
            .on_hover_cursor(CursorIcon::PointingHand)
            .on_hover_text(name)
            .clicked()
        {
            self.view = if self.view == View::Folders {
                View::Player
            } else {
                View::Folders
            };
        }
        let more = icon_button(
            ui,
            Rect::from_min_size(pos2(r.right() - 34.0, r.top()), vec2(34.0, 32.0)),
            icons::DOTS_THREE,
            "Настройки и действия",
            false,
            false,
        );
        Popup::menu(&more).show(|ui| {
            if ui.button("Эквалайзер").clicked() {
                self.view = View::Equalizer;
                ui.close();
            }
            ui.separator();
            ui.menu_button("Устройство вывода звука", |ui| {
                if ui
                    .selectable_label(
                        self.settings.output_device.is_none(),
                        "По умолчанию Windows",
                    )
                    .clicked()
                {
                    self.select_output(None);
                    ui.close();
                }
                ui.separator();
                match Audio::available_outputs() {
                    Ok(outputs) => {
                        for name in outputs {
                            if ui
                                .selectable_label(
                                    self.settings.output_device.as_ref() == Some(&name),
                                    &name,
                                )
                                .clicked()
                            {
                                self.select_output(Some(name));
                                ui.close();
                            }
                        }
                    }
                    Err(error) => {
                        ui.label(error);
                    }
                }
            });
            if let Some(name) = self.audio.output_name() {
                ui.add(
                    Label::new(
                        RichText::new(format!("Сейчас: {name}"))
                            .size(11.0)
                            .color(MUTED),
                    )
                    .wrap(),
                );
            }
            ui.separator();
            if ui.button("Добавить папку…  Ctrl+O").clicked() {
                self.pick_folder(ctx);
                ui.close();
            }
            if let Some(index) = self.selected {
                if ui.button("Обновить папку  F5").clicked() {
                    self.scan(index, ctx);
                    ui.close();
                }
                if ui.button("Убрать папку из плеера").clicked() {
                    self.remove_folder(index);
                    ui.close();
                }
                ui.separator();
                ui.label(
                    RichText::new(display_path(
                        &self
                            .playlists
                            .get(index)
                            .map(|p| p.path.clone())
                            .unwrap_or_default(),
                    ))
                    .size(10.0)
                    .color(MUTED),
                );
            }
        });
    }
    fn now_playing(&mut self, ui: &mut Ui, r: Rect, ctx: &Context) {
        let track = self.display_track().cloned();
        let cover_size = r.width().min((r.height() - 146.0).max(88.0));
        let art = Rect::from_min_size(
            pos2(r.center().x - cover_size / 2.0, r.top()),
            vec2(cover_size, cover_size),
        );
        if let Some(texture) = &self.cover {
            ui.put(
                art,
                egui::Image::new(texture)
                    .fit_to_exact_size(art.size())
                    .corner_radius(6.0),
            );
        } else {
            ui.painter()
                .rect_filled(art, 6.0, Color32::from_white_alpha(10));
            icon(
                ui.painter(),
                art.center() - vec2(0.0, if self.playlists.is_empty() { 14.0 } else { 0.0 }),
                icons::MUSIC_NOTES,
                42.0,
                MUTED,
            );
            if self.playlists.is_empty() {
                text(
                    ui.painter(),
                    art.center() + vec2(0.0, 32.0),
                    Align2::CENTER_CENTER,
                    "Добавьте папку с музыкой",
                    12.0,
                    MUTED,
                );
                let response = ui.interact(art, Id::new("empty-add"), Sense::click());
                response.widget_info(|| {
                    WidgetInfo::labeled(WidgetType::Button, true, "Добавить папку с музыкой")
                });
                if response.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                    self.pick_folder(ctx);
                }
            }
        }
        let title_y = art.bottom() + 8.0;
        let title_rect = Rect::from_min_size(pos2(r.left(), title_y), vec2(r.width(), 20.0));
        let folder = self.selected.and_then(|i| self.playlists.get(i));
        let empty_title = match folder {
            Some(f) if f.loading => "Читаем папку…",
            Some(f) if f.error.is_some() => "Папка недоступна",
            Some(_) => "В папке нет треков",
            None => "Ваша музыка здесь",
        };
        let title = track.as_ref().map_or(empty_title, |t| t.title.as_str());
        let artist =
            track.as_ref().map_or("Одна папка — один плейлист", |t| {
                t.artist.as_str()
            });
        heading(ui.painter(), title_rect, title, 17.0);
        ellipsis(
            ui.painter(),
            title_rect.translate(vec2(0.0, 20.0)),
            artist,
            13.0,
            MUTED,
        );
        ui.interact(
            title_rect.expand2(vec2(0.0, 12.0)),
            Id::new("track-label"),
            Sense::hover(),
        )
        .on_hover_text(format!("{title}\n{artist}"));
        let duration = track.as_ref().map_or(0.0, |t| t.duration);
        let mut position = if self.current.is_none() {
            0.0
        } else if self.ended {
            duration
        } else {
            self.audio.position().min(duration.max(0.0))
        };
        let seek_y = title_y + 48.0;
        ui.add_enabled_ui(
            self.current.is_some() && duration > 0.0 && !self.ended,
            |ui| {
                if slider(
                    ui,
                    Rect::from_min_size(pos2(r.left(), seek_y - 10.0), vec2(r.width(), 20.0)),
                    &mut position,
                    duration,
                    "Позиция воспроизведения",
                )
                .changed()
                {
                    self.seek(position);
                }
            },
        );
        text(
            ui.painter(),
            pos2(r.left(), seek_y + 13.0),
            Align2::LEFT_CENTER,
            time_label(position),
            10.0,
            MUTED,
        );
        text(
            ui.painter(),
            pos2(r.right(), seek_y + 13.0),
            Align2::RIGHT_CENTER,
            format!("−{}", time_label((duration - position).max(0.0))),
            10.0,
            MUTED,
        );
        self.transport(
            ui,
            Rect::from_min_size(pos2(r.left(), seek_y + 22.0), vec2(r.width(), 36.0)),
            ctx,
            track.is_some(),
        );
        self.volume_control(
            ui,
            Rect::from_min_size(pos2(r.left(), seek_y + 66.0), vec2(r.width(), 24.0)),
        );
    }
    fn transport(&mut self, ui: &mut Ui, r: Rect, ctx: &Context, enabled: bool) {
        ui.add_enabled_ui(enabled, |ui| {
            for (fraction, symbol, label, active, primary) in [
                (
                    0.03,
                    icons::SHUFFLE,
                    "Случайный порядок",
                    self.settings.shuffle,
                    false,
                ),
                (
                    0.25,
                    icons::SKIP_BACK,
                    "Предыдущий трек · Ctrl+←",
                    false,
                    false,
                ),
                (
                    0.50,
                    if self.audio.paused() || self.ended || self.current.is_none() {
                        icons::PLAY
                    } else {
                        icons::PAUSE
                    },
                    if self.audio.paused() || self.ended || self.current.is_none() {
                        "Воспроизвести · Пробел"
                    } else {
                        "Пауза · Пробел"
                    },
                    false,
                    true,
                ),
                (
                    0.75,
                    icons::SKIP_FORWARD,
                    "Следующий трек · Ctrl+→",
                    false,
                    false,
                ),
                (
                    0.97,
                    if self.settings.repeat == Repeat::One {
                        icons::REPEAT_ONCE
                    } else {
                        icons::REPEAT
                    },
                    self.settings.repeat.label(),
                    self.settings.repeat != Repeat::Off,
                    false,
                ),
            ] {
                let x = r.left() + 16.0 + fraction * (r.width() - 32.0);
                let button = Rect::from_center_size(
                    pos2(x, r.center().y),
                    vec2(if primary { 40.0 } else { 32.0 }, 36.0),
                );
                let can_use = self.current.is_some() || (fraction != 0.25 && fraction != 0.75);
                let clicked = ui
                    .add_enabled_ui(can_use, |ui| {
                        icon_button(ui, button, symbol, label, active, primary).clicked()
                    })
                    .inner;
                if clicked {
                    if fraction < 0.1 {
                        self.toggle_shuffle();
                    } else if fraction < 0.3 {
                        self.previous(ctx);
                    } else if fraction < 0.6 {
                        self.toggle(ctx);
                    } else if fraction < 0.8 {
                        self.next(true, ctx);
                    } else {
                        self.cycle_repeat();
                    }
                }
            }
        });
    }
    fn volume_control(&mut self, ui: &mut Ui, r: Rect) {
        if icon_button(
            ui,
            Rect::from_min_size(r.min, vec2(28.0, 24.0)),
            if self.settings.volume == 0.0 {
                icons::SPEAKER_SLASH
            } else {
                icons::SPEAKER_LOW
            },
            "Выключить / включить звук",
            false,
            false,
        )
        .clicked()
        {
            if self.settings.volume > 0.0 {
                self.last_volume = self.settings.volume;
                self.set_volume(0.0);
            } else {
                self.set_volume(self.last_volume.max(0.05));
            }
        }
        icon(
            ui.painter(),
            pos2(r.right() - 12.0, r.center().y),
            icons::SPEAKER_HIGH,
            16.0,
            MUTED,
        );
        let mut volume = self.settings.volume as f64;
        if slider(
            ui,
            Rect::from_min_max(
                pos2(r.left() + 36.0, r.top()),
                pos2(r.right() - 34.0, r.bottom()),
            ),
            &mut volume,
            1.0,
            "Громкость",
        )
        .changed()
        {
            self.set_volume(volume as f32);
        }
    }
    fn footer(&mut self, ui: &mut Ui, r: Rect, ctx: &Context) {
        if icon_button(
            ui,
            Rect::from_min_size(r.min, vec2(32.0, 32.0)),
            icons::FOLDER_SIMPLE_PLUS,
            "Добавить папку · Ctrl+O",
            false,
            false,
        )
        .clicked()
        {
            self.pick_folder(ctx);
        }
        let folder = self.selected.and_then(|i| self.playlists.get(i));
        let count = folder.map_or_else(
            || "Нет папок".into(),
            |f| {
                if f.loading {
                    "Загрузка…".into()
                } else {
                    track_count(f.tracks.len())
                }
            },
        );
        text(
            ui.painter(),
            r.center(),
            Align2::CENTER_CENTER,
            count,
            11.0,
            MUTED,
        );
        let label = if self.view == View::Player {
            "Список треков"
        } else {
            "К проигрывателю"
        };
        if icon_button(
            ui,
            Rect::from_min_size(pos2(r.right() - 32.0, r.top()), vec2(32.0, 32.0)),
            if self.view == View::Player {
                icons::LIST
            } else {
                icons::MUSIC_NOTES
            },
            label,
            self.view != View::Player,
            false,
        )
        .clicked()
        {
            self.view = if self.view == View::Player {
                View::Tracks
            } else {
                View::Player
            };
            ctx.memory_mut(|m| m.surrender_focus(Id::new("search")));
        }
    }
    fn track_list(&mut self, ui: &mut Ui, r: Rect, ctx: &Context) {
        let Some(index) = self.selected else {
            self.empty_panel(ui, r, "Добавьте папку с музыкой", ctx);
            return;
        };
        let folder = &self.playlists[index];
        let loading = folder.loading;
        let error = folder.error.clone();
        let mut selected_track = None;
        let search = Rect::from_min_size(r.min, vec2(r.width(), 32.0));
        ui.put(
            search,
            TextEdit::singleline(&mut self.filter)
                .id(Id::new("search"))
                .hint_text("Поиск в папке")
                .font(FontId::proportional(13.0))
                .margin(vec2(10.0, 7.0)),
        );
        let list_rect = Rect::from_min_max(
            pos2(r.left(), search.bottom() + 8.0),
            pos2(r.right(), r.bottom() - 52.0),
        );
        let tracks: Vec<_> = folder
            .tracks
            .iter()
            .enumerate()
            .filter(|(_, t)| t.matches(&self.filter))
            .collect();
        let current_path = self.current_track().map(|t| &t.path);
        ui.scope_builder(UiBuilder::new().max_rect(list_rect), |ui| {
            ui.set_clip_rect(list_rect);
            if let Some(error) = error {
                ui.label(RichText::new(error).color(MUTED));
            } else if loading && tracks.is_empty() {
                ui.label("Читаем музыку…");
            } else if tracks.is_empty() {
                ui.label(if self.filter.is_empty() {
                    "В этой папке пока нет аудиофайлов."
                } else {
                    "Ничего не найдено."
                });
            } else {
                ui.spacing_mut().item_spacing.y = 0.0;
                Self::scrollbar_gutter(ui);
                ScrollArea::vertical()
                    .id_salt("tracks-scroll")
                    .max_height(list_rect.height())
                    .auto_shrink([false, false])
                    .show_rows(ui, 54.0, tracks.len(), |ui, range| {
                        for row_index in range {
                            let (track_index, track) = tracks[row_index];
                            let playing = current_path == Some(&track.path);
                            let (row, response) = ui.allocate_exact_size(
                                vec2(ui.available_width(), 54.0),
                                Sense::click(),
                            );
                            response.widget_info(|| {
                                WidgetInfo::selected(
                                    WidgetType::SelectableLabel,
                                    true,
                                    playing,
                                    format!("{} — {}", track.title, track.artist),
                                )
                            });
                            if playing || response.hovered() || response.has_focus() {
                                ui.painter().rect_filled(
                                    row,
                                    5.0,
                                    if playing { SELECT } else { LINE },
                                );
                            }
                            if playing {
                                ui.painter().rect_filled(
                                    Rect::from_min_size(row.min + vec2(0.0, 10.0), vec2(3.0, 34.0)),
                                    2.0,
                                    ACCENT,
                                );
                            }
                            let left = row.left() + 10.0;
                            ellipsis(
                                ui.painter(),
                                Rect::from_min_size(
                                    pos2(left, row.top() + 6.0),
                                    vec2(row.width() - 62.0, 21.0),
                                ),
                                &track.title,
                                13.0,
                                if playing { ACCENT } else { FG },
                            );
                            ellipsis(
                                ui.painter(),
                                Rect::from_min_size(
                                    pos2(left, row.top() + 27.0),
                                    vec2(row.width() - 62.0, 19.0),
                                ),
                                &track.artist,
                                11.0,
                                MUTED,
                            );
                            text(
                                ui.painter(),
                                pos2(row.right() - 8.0, row.center().y),
                                Align2::RIGHT_CENTER,
                                time_label(track.duration),
                                10.0,
                                MUTED,
                            );
                            if response
                                .on_hover_text(format!("{}\n{}", track.title, track.artist))
                                .on_hover_cursor(CursorIcon::PointingHand)
                                .clicked()
                            {
                                selected_track = Some(track_index);
                            }
                        }
                    });
            }
        });
        if let Some(i) = selected_track {
            if self
                .current_track()
                .is_some_and(|t| t.path == self.playlists[index].tracks[i].path)
            {
                self.toggle(ctx);
            } else {
                self.start_track(i, ctx);
            }
            if self.message.is_none() {
                self.view = View::Player;
            }
        }
        self.transport(
            ui,
            Rect::from_min_size(pos2(r.left(), r.bottom() - 46.0), vec2(r.width(), 46.0)),
            ctx,
            self.display_track().is_some(),
        );
    }
    fn folder_list(&mut self, ui: &mut Ui, r: Rect, ctx: &Context) {
        if self.playlists.is_empty() {
            self.empty_panel(ui, r, "Одна папка — один плейлист", ctx);
            return;
        }
        let mut select = None;
        let mut remove = None;
        let mut refresh = None;
        ui.scope_builder(UiBuilder::new().max_rect(r), |ui| {
            Self::scrollbar_gutter(ui);
            ui.label(RichText::new("Ваши папки").size(12.0).color(MUTED));
            ui.add_space(8.0);
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for (i, folder) in self.playlists.iter().enumerate() {
                        let (row, response) = ui
                            .allocate_exact_size(vec2(ui.available_width(), 58.0), Sense::click());
                        response.widget_info(|| {
                            WidgetInfo::selected(
                                WidgetType::SelectableLabel,
                                true,
                                self.selected == Some(i),
                                &folder.name,
                            )
                        });
                        if self.selected == Some(i) || response.hovered() || response.has_focus() {
                            ui.painter().rect_filled(row, 5.0, SELECT);
                        }
                        icon(
                            ui.painter(),
                            pos2(row.left() + 18.0, row.center().y),
                            icons::FOLDER_SIMPLE,
                            20.0,
                            FG,
                        );
                        ellipsis(
                            ui.painter(),
                            Rect::from_min_size(
                                pos2(row.left() + 38.0, row.top() + 7.0),
                                vec2(row.width() - 52.0, 23.0),
                            ),
                            &folder.name,
                            14.0,
                            FG,
                        );
                        ellipsis(
                            ui.painter(),
                            Rect::from_min_size(
                                pos2(row.left() + 38.0, row.top() + 31.0),
                                vec2(row.width() - 52.0, 18.0),
                            ),
                            &if folder.loading {
                                "Читаем папку…".into()
                            } else {
                                track_count(folder.tracks.len())
                            },
                            11.0,
                            MUTED,
                        );
                        if response.clicked() {
                            select = Some(i);
                        }
                        response.context_menu(|ui| {
                            if ui.button("Обновить").clicked() {
                                refresh = Some(i);
                                ui.close();
                            }
                            if ui.button("Убрать из плеера").clicked() {
                                remove = Some(i);
                                ui.close();
                            }
                        });
                        response.on_hover_text(display_path(&folder.path));
                    }
                });
        });
        if let Some(i) = select {
            self.selected = Some(i);
            self.filter.clear();
            self.view = View::Tracks;
            self.changed();
        }
        if let Some(i) = refresh {
            self.scan(i, ctx);
        }
        if let Some(i) = remove {
            self.remove_folder(i);
        }
    }
    fn scrollbar_gutter(ui: &mut Ui) {
        let scroll = &mut ui.spacing_mut().scroll;
        *scroll = egui::style::ScrollStyle::solid();
        scroll.bar_width = 5.0;
        scroll.bar_inner_margin = 8.0;
        scroll.bar_outer_margin = 0.0;
    }
    fn equalizer_panel(&mut self, ui: &mut Ui, r: Rect, ctx: &Context) {
        let mut updated = false;
        heading(
            ui.painter(),
            Rect::from_min_size(r.min, vec2(r.width(), 24.0)),
            "Эквалайзер",
            18.0,
        );
        ui.scope_builder(
            UiBuilder::new().max_rect(Rect::from_min_size(
                r.min + vec2(0.0, 34.0),
                vec2(r.width(), 24.0),
            )),
            |ui| {
                updated |= ui
                    .checkbox(&mut self.settings.equalizer.enabled, "Включён")
                    .changed();
            },
        );
        ui.add_enabled_ui(self.settings.equalizer.enabled, |ui| {
            for i in 0..5 {
                let x = r.left() + (i as f32 + 0.5) * r.width() / 5.0;
                text(
                    ui.painter(),
                    pos2(x, r.top() + 80.0),
                    Align2::CENTER_CENTER,
                    format!("{:+.0}", self.settings.equalizer.gains[i]),
                    12.0,
                    FG,
                );
                let name = format!("{} Гц", mellow::equalizer::FREQUENCIES[i]);
                updated |= equalizer_slider(
                    ui,
                    Rect::from_min_size(pos2(x - 16.0, r.top() + 96.0), vec2(32.0, 150.0)),
                    &mut self.settings.equalizer.gains[i],
                    &name,
                )
                .changed();
                text(
                    ui.painter(),
                    pos2(x, r.top() + 260.0),
                    Align2::CENTER_CENTER,
                    ["60", "250", "1k", "4k", "12k"][i],
                    11.0,
                    MUTED,
                );
            }
        });
        text(
            ui.painter(),
            pos2(r.left(), r.top() + 286.0),
            Align2::LEFT_CENTER,
            "Частота · Гц     Усиление · ±12 дБ",
            11.0,
            MUTED,
        );
        ui.scope_builder(
            UiBuilder::new().max_rect(Rect::from_min_size(
                r.min + vec2(0.0, 310.0),
                vec2(r.width(), 28.0),
            )),
            |ui| {
                ui.horizontal(|ui| {
                    for (name, gains) in [
                        ("Ровно", [0.0; 5]),
                        ("Бас", [6.0, 3.0, 0.0, 0.0, 0.0]),
                        ("Голос", [-2.0, 0.0, 3.0, 2.0, -1.0]),
                    ] {
                        if ui.button(name).clicked() {
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
        self.transport(
            ui,
            Rect::from_min_size(pos2(r.left(), r.bottom() - 36.0), vec2(r.width(), 36.0)),
            ctx,
            self.display_track().is_some(),
        );
    }
    fn empty_panel(&mut self, ui: &mut Ui, r: Rect, label: &str, ctx: &Context) {
        ui.scope_builder(UiBuilder::new().max_rect(r), |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label(label);
                ui.add_space(14.0);
                if ui.button("Добавить папку…").clicked() {
                    self.pick_folder(ctx);
                }
            });
        });
    }
}
