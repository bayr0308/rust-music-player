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
            // Keep playback controls in one place for both views.
            let transport = Rect::from_min_size(
                pos2(body.left(), body.bottom() - 68.0),
                vec2(body.width(), 36.0),
            );
            let volume = Rect::from_min_size(
                pos2(body.left(), body.bottom() - 24.0),
                vec2(body.width(), 24.0),
            );
            let content = Rect::from_min_max(body.min, pos2(body.right(), transport.top() - 6.0));
            match self.view {
                View::Player => self.now_playing(ui, content, ctx),
                View::Tracks => self.track_list(ui, content, ctx),
            }
            self.transport(ui, transport, ctx, self.display_track().is_some());
            self.volume_control(ui, volume);
            self.footer(ui, footer, ctx);
        });
        self.settings_window(ctx);
        if !self.settings_open {
            self.error_dialog(ctx);
        }
    }

    fn folder_toolbar(&mut self, ui: &mut Ui, r: Rect, ctx: &Context) {
        let selector = Rect::from_min_max(r.min, pos2(r.right() - 40.0, r.bottom()));
        let folder = self.selected.and_then(|i| self.playlists.get(i));
        let name = folder.map_or("Выбрать папку", |f| f.name.as_str());
        let response = fluent_select_button(
            ui,
            selector,
            Id::new("folder-selector"),
            name,
            &format!("Папки: {name}"),
            Some(icons::FOLDER_SIMPLE),
        );
        fluent_popup(&response).show(|ui| {
            Self::scrollbar_gutter(ui);
            ScrollArea::vertical()
                .id_salt("folder-dropdown")
                .max_height(240.0)
                .show(ui, |ui| {
                    for index in 0..self.playlists.len() {
                        let folder = &self.playlists[index];
                        if fluent_option(ui, self.selected == Some(index), &folder.name)
                            .on_hover_text(display_path(&folder.path))
                            .clicked()
                        {
                            self.selected = Some(index);
                            self.filter.clear();
                            self.changed();
                            ui.close();
                        }
                    }
                    if self.playlists.is_empty() {
                        ui.label("Нет добавленных папок");
                    }
                });
            ui.separator();
            if ui
                .add_enabled_ui(!self.picking, |ui| fluent_menu_action(ui, "Добавить"))
                .inner
                .clicked()
            {
                self.pick_folder(ctx);
                ui.close();
            }
        });
        let more = icon_button(
            ui,
            Rect::from_min_size(pos2(r.right() - 34.0, r.top()), vec2(34.0, 32.0)),
            icons::DOTS_THREE,
            "Настройки и действия",
            false,
            false,
        );
        fluent_popup(&more).width(184.0).show(|ui| {
            let refreshable = self
                .selected
                .is_some_and(|index| !self.playlists[index].loading);
            if ui
                .add_enabled_ui(refreshable, |ui| fluent_menu_action(ui, "Обновить папку"))
                .inner
                .on_hover_text("Обновить папку · F5")
                .clicked()
                && let Some(index) = self.selected
            {
                self.scan(index, ctx);
                ui.close();
            }
            if fluent_menu_action(ui, "Настройки").clicked() {
                self.open_settings(SettingsTab::Folders, ctx);
                ui.close();
            }
        });
    }
    fn now_playing(&mut self, ui: &mut Ui, r: Rect, ctx: &Context) {
        let track = self.display_track().cloned();
        let cover_size = r.width().min((r.height() - 72.0).max(88.0));
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
        let artist = track.as_ref().map_or("", |t| t.artist.as_str());
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
        let list_rect = Rect::from_min_max(pos2(r.left(), search.bottom() + 8.0), r.max);
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
    }
    pub(super) fn scrollbar_gutter(ui: &mut Ui) {
        let scroll = &mut ui.spacing_mut().scroll;
        *scroll = egui::style::ScrollStyle::solid();
        scroll.bar_width = 5.0;
        scroll.bar_inner_margin = 8.0;
        scroll.bar_outer_margin = 0.0;
    }
    fn empty_panel(&mut self, ui: &mut Ui, r: Rect, label: &str, ctx: &Context) {
        ui.scope_builder(UiBuilder::new().max_rect(r), |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label(label);
                ui.add_space(14.0);
                if ui.button("Добавить").clicked() {
                    self.pick_folder(ctx);
                }
            });
        });
    }
}
