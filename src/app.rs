use crate::{
    Options,
    theme::{self, *},
};
use eframe::egui::{self, *};
use mellow::{
    audio::Audio,
    library::{self, Playlist, Track, time_label, track_count},
    queue::{Queue, Repeat},
    settings::Settings,
};
use std::{
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    time::{Duration, Instant},
};

mod ui;

#[derive(Clone, Copy, PartialEq)]
enum View {
    Player,
    Tracks,
    Folders,
    Equalizer,
}

enum Loaded {
    Scan {
        path: PathBuf,
        generation: u64,
        result: Result<Vec<Track>, String>,
    },
    Folders(Vec<PathBuf>),
    Cover {
        path: PathBuf,
        bytes: Option<Vec<u8>>,
    },
}

pub struct PlayerApp {
    view: View,
    acrylic: bool,
    preview_path: Option<PathBuf>,
    playlists: Vec<Playlist>,
    selected: Option<usize>,
    filter: String,
    settings: Settings,
    settings_path: PathBuf,
    settings_ok: bool,
    dirty: Option<Instant>,
    audio: Audio,
    queue: Queue,
    queue_tracks: Vec<Track>,
    queue_folder: Option<PathBuf>,
    current: Option<usize>,
    ended: bool,
    cover: Option<TextureHandle>,
    tx: Sender<Loaded>,
    rx: Receiver<Loaded>,
    picking: bool,
    message: Option<String>,
    last_volume: f32,
    cover_tint: [u8; 3],
    preload_blocked: Option<usize>,
}

impl PlayerApp {
    pub fn new(cc: &eframe::CreationContext<'_>, options: Options) -> Self {
        theme::setup(&cc.egui_ctx);
        let custom_root = options.data_dir.is_some();
        let root = options.data_dir.unwrap_or_else(|| {
            directories::ProjectDirs::from("", "", "Mellow")
                .map(|p| p.config_dir().to_path_buf())
                .unwrap_or_else(|| PathBuf::from(".mellow"))
        });
        let settings_path = root.join("settings.json");
        let legacy = directories::ProjectDirs::from("", "", "Tiho")
            .map(|d| d.config_dir().join("settings.json"));
        let load_path = if !custom_root && !settings_path.exists() {
            legacy
                .as_ref()
                .filter(|p| p.exists())
                .unwrap_or(&settings_path)
        } else {
            &settings_path
        };
        let (settings, message, settings_ok) = match Settings::load(load_path) {
            Ok(settings) => (settings, None, true),
            Err(error) => (
                Settings::default(),
                Some(format!(
                    "{error}. Исходный файл настроек оставлен без изменений."
                )),
                false,
            ),
        };
        let (tx, rx) = mpsc::channel();
        let playlists: Vec<_> = settings
            .folders
            .iter()
            .map(|p| Playlist::new(p.clone()))
            .collect();
        let selected = settings
            .selected
            .as_ref()
            .and_then(|p| playlists.iter().position(|f| &f.path == p))
            .or_else(|| (!playlists.is_empty()).then_some(0));
        let mut app = Self {
            view: View::Player,
            acrylic: crate::window::acrylic(cc),
            preview_path: None,
            playlists,
            selected,
            filter: String::new(),
            audio: Audio::with_output(settings.output_device.clone()),
            settings,
            settings_path,
            settings_ok,
            dirty: None,
            queue: Queue::default(),
            queue_tracks: vec![],
            queue_folder: None,
            current: None,
            ended: false,
            cover: None,
            tx,
            rx,
            picking: false,
            message,
            last_volume: 0.65,
            cover_tint: [32, 38, 50],
            preload_blocked: None,
        };
        app.audio.set_equalizer(app.settings.equalizer.clone());
        for index in 0..app.playlists.len() {
            app.scan(index, &cc.egui_ctx);
        }
        for path in options.folders {
            app.add_folder(path, &cc.egui_ctx);
        }
        app
    }

    fn changed(&mut self) {
        self.dirty = Some(Instant::now());
    }
    fn persist(&mut self) {
        if !self.settings_ok {
            return;
        }
        self.settings.folders = self.playlists.iter().map(|f| f.path.clone()).collect();
        self.settings.selected = self
            .selected
            .and_then(|i| self.playlists.get(i))
            .map(|f| f.path.clone());
        if let Err(e) = self.settings.save(&self.settings_path) {
            self.message = Some(format!("Не удалось сохранить настройки: {e}"));
        }
        self.dirty = None;
    }
    fn scan(&mut self, index: usize, ctx: &Context) {
        let playlist = &mut self.playlists[index];
        playlist.generation += 1;
        playlist.loading = true;
        playlist.error = None;
        let (path, generation, tx, ctx) = (
            playlist.path.clone(),
            playlist.generation,
            self.tx.clone(),
            ctx.clone(),
        );
        std::thread::spawn(move || {
            let result = library::scan_folder(&path);
            let _ = tx.send(Loaded::Scan {
                path,
                generation,
                result,
            });
            ctx.request_repaint();
        });
    }
    fn add_folder(&mut self, path: PathBuf, ctx: &Context) {
        if !path.is_dir() {
            self.message =
                Some("Перетащите папку с музыкой или выберите её через «Добавить папку».".into());
            return;
        }
        let path = path.canonicalize().unwrap_or(path);
        if let Some(index) = self.playlists.iter().position(|p| p.path == path) {
            self.selected = Some(index);
            self.filter.clear();
            self.changed();
            return;
        }
        self.playlists.push(Playlist::new(path));
        let index = self.playlists.len() - 1;
        self.selected = Some(index);
        self.filter.clear();
        self.scan(index, ctx);
        self.changed();
    }
    fn pick_folder(&mut self, ctx: &Context) {
        if self.picking {
            return;
        }
        self.picking = true;
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        std::thread::spawn(move || {
            let folders = rfd::FileDialog::new()
                .set_title("Добавить папки с музыкой")
                .pick_folders()
                .unwrap_or_default();
            let _ = tx.send(Loaded::Folders(folders));
            ctx.request_repaint();
        });
    }
    fn remove_folder(&mut self, index: usize) {
        let removed = self.playlists.remove(index);
        if self.queue_folder.as_ref() == Some(&removed.path) {
            self.audio.stop();
            self.queue_tracks.clear();
            self.current = None;
            self.cover = None;
            self.queue_folder = None;
        }
        self.selected = self.selected.and_then(|selected| {
            if self.playlists.is_empty() {
                None
            } else if selected > index {
                Some(selected - 1)
            } else {
                Some(selected.min(self.playlists.len() - 1))
            }
        });
        self.filter.clear();
        self.changed();
    }
    fn poll(&mut self, ctx: &Context) {
        if let Some(error) = self.audio.check_output() {
            self.message = Some(error);
        }
        while let Ok(loaded) = self.rx.try_recv() {
            match loaded {
                Loaded::Scan {
                    path,
                    generation,
                    result,
                } => {
                    if let Some(folder) = self
                        .playlists
                        .iter_mut()
                        .find(|p| p.path == path && p.generation == generation)
                    {
                        folder.loading = false;
                        match result {
                            Ok(tracks) => {
                                folder.tracks = tracks;
                                folder.error = None;
                            }
                            Err(e) => {
                                folder.tracks.clear();
                                folder.error = Some(e);
                            }
                        }
                    }
                }
                Loaded::Folders(paths) => {
                    self.picking = false;
                    if !paths.is_empty() {
                        self.view = View::Tracks;
                    }
                    for path in paths {
                        self.add_folder(path, ctx);
                    }
                }
                Loaded::Cover { path, bytes } => {
                    if self.display_track().is_some_and(|t| t.path == path) {
                        self.cover = bytes.and_then(|bytes| {
                            let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
                                .with_guessed_format()
                                .ok()?;
                            let mut limits = image::Limits::default();
                            limits.max_image_width = Some(8192);
                            limits.max_image_height = Some(8192);
                            reader.limits(limits);
                            let image = reader
                                .decode()
                                .ok()?
                                .resize_to_fill(600, 600, image::imageops::FilterType::Lanczos3)
                                .to_rgba8();
                            self.cover_tint = mellow::artwork::cover_tint(&image);
                            let size = [image.width() as usize, image.height() as usize];
                            Some(ctx.load_texture(
                                "album-cover",
                                ColorImage::from_rgba_unmultiplied(size, image.as_raw()),
                                TextureOptions::LINEAR,
                            ))
                        });
                    }
                }
            }
        }
        self.consume_transitions();
        self.prime_gapless();
        if self.current.is_some() && !self.ended && self.audio.finished() {
            self.next(false, ctx);
        }
        if self
            .dirty
            .is_some_and(|t| t.elapsed() > Duration::from_millis(500))
        {
            self.persist();
        }
    }
    fn consume_transitions(&mut self) {
        for index in self.audio.take_transitions() {
            // Empty tracks can finish without emitting a sample. Align to the
            // actual playing track rather than counting UI notifications.
            if let Some(cursor) = self.queue.order.iter().position(|&id| id == index) {
                self.queue.cursor = cursor;
            }
            self.current = Some(index);
            self.ended = false;
            self.preload_blocked = None;
        }
    }
    fn prime_gapless(&mut self) {
        if self.current.is_none() || self.ended || self.preload_blocked.is_some() {
            return;
        }
        let mut planned = self.queue.clone();
        let pending = self.audio.upcoming_count();
        for _ in 0..pending {
            planned.advance(self.settings.repeat, false);
        }
        for _ in pending..8 {
            let Some(index) = planned.advance(self.settings.repeat, false) else {
                break;
            };
            let Some(track) = self.queue_tracks.get(index) else {
                break;
            };
            if self.audio.enqueue(&track.path, index).is_err() {
                // Keep playing; normal end-of-track handling reports an unreadable next file.
                self.preload_blocked = Some(index);
                break;
            }
        }
    }
    fn toggle_shuffle(&mut self) {
        if let Err(error) = self.audio.clear_upcoming() {
            self.message = Some(error);
            return;
        }
        self.consume_transitions();
        self.settings.shuffle = !self.settings.shuffle;
        if let Some(index) = self.current {
            self.queue
                .start(self.queue_tracks.len(), index, self.settings.shuffle);
        }
        self.preload_blocked = None;
        self.changed();
    }
    fn cycle_repeat(&mut self) {
        if let Err(error) = self.audio.clear_upcoming() {
            self.message = Some(error);
            return;
        }
        self.consume_transitions();
        self.settings.repeat = self.settings.repeat.cycle();
        self.preload_blocked = None;
        self.changed();
    }
    fn display_track(&self) -> Option<&Track> {
        self.current_track().or_else(|| {
            self.selected
                .and_then(|i| self.playlists.get(i))
                .and_then(|p| p.tracks.first())
        })
    }
    fn refresh_cover(&mut self, ctx: &Context) {
        let path = self.display_track().map(|t| t.path.clone());
        if self.preview_path == path {
            return;
        }
        self.preview_path = path.clone();
        self.cover = None;
        self.cover_tint = [32, 38, 50];
        if let Some(path) = path {
            let (tx, ctx) = (self.tx.clone(), ctx.clone());
            std::thread::spawn(move || {
                let bytes = library::cover_bytes(&path);
                let _ = tx.send(Loaded::Cover { path, bytes });
                ctx.request_repaint();
            });
        }
    }
    fn current_track(&self) -> Option<&Track> {
        self.current.and_then(|i| self.queue_tracks.get(i))
    }
    fn start_track(&mut self, index: usize, ctx: &Context) {
        let Some(folder) = self.selected.and_then(|i| self.playlists.get(i)) else {
            return;
        };
        if index >= folder.tracks.len() {
            return;
        }
        let old_tracks = self.queue_tracks.clone();
        let old_folder = self.queue_folder.clone();
        self.queue_tracks = folder.tracks.clone();
        self.queue_folder = Some(folder.path.clone());
        if self.play_index(index, ctx) {
            self.queue
                .start(self.queue_tracks.len(), index, self.settings.shuffle);
        } else {
            self.queue_tracks = old_tracks;
            self.queue_folder = old_folder;
        }
    }
    fn play_index(&mut self, index: usize, _ctx: &Context) -> bool {
        let Some(track) = self.queue_tracks.get(index) else {
            return false;
        };
        let path = track.path.clone();
        match self.audio.play_track(&path, self.settings.volume, index) {
            Ok(()) => {
                self.current = Some(index);
                self.ended = false;
                self.preload_blocked = None;
                self.message = None;
                true
            }
            Err(e) => {
                self.message = Some(e);
                false
            }
        }
    }
    fn next(&mut self, manual: bool, ctx: &Context) {
        let old_cursor = self.queue.cursor;
        if let Some(index) = self.queue.advance(self.settings.repeat, manual) {
            if !self.play_index(index, ctx) {
                self.queue.cursor = old_cursor;
                self.ended = true;
                self.audio.stop();
            }
        } else {
            self.ended = true;
            self.audio.stop();
        }
    }
    fn previous(&mut self, ctx: &Context) {
        if self.audio.position() > 3.0 {
            self.seek(0.0);
        } else {
            let cursor = self.queue.cursor;
            if let Some(index) = self.queue.previous()
                && !self.play_index(index, ctx)
            {
                self.queue.cursor = cursor;
            }
        }
    }
    fn toggle(&mut self, ctx: &Context) {
        if let Some(index) = self.current {
            if self.ended {
                self.play_index(index, ctx);
            } else {
                self.audio.toggle();
            }
        } else if let Some(index) = self.selected.and_then(|i| {
            self.playlists[i]
                .tracks
                .iter()
                .position(|t| t.matches(&self.filter))
        }) {
            self.start_track(index, ctx);
        }
    }
    fn seek(&mut self, value: f64) {
        if let Err(e) = self.audio.seek(value) {
            self.message = Some(e);
        }
    }
    fn set_volume(&mut self, value: f32) {
        self.settings.volume = value.clamp(0.0, 1.0);
        self.audio.volume(self.settings.volume);
        self.changed();
    }

    fn select_output(&mut self, name: Option<String>) {
        match self.audio.select_output(name.clone()) {
            Ok(()) => {
                self.settings.output_device = name;
                self.message = None;
                self.changed();
            }
            Err(error) => self.message = Some(error),
        }
    }
    fn keyboard(&mut self, ctx: &Context) {
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            self.view = View::Player;
            self.message = None;
            ctx.memory_mut(|m| m.surrender_focus(Id::new("search")));
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::O)) {
            self.pick_folder(ctx);
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::F)) {
            self.view = View::Tracks;
            ctx.memory_mut(|m| m.request_focus(Id::new("search")));
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::F5))
            && let Some(index) = self.selected
        {
            self.scan(index, ctx);
        }
        if !ctx.wants_keyboard_input() && !ctx.memory(|m| m.focused().is_some()) {
            if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Space)) {
                self.toggle(ctx);
            }
            if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::ArrowRight)) {
                self.next(true, ctx);
            }
            if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::ArrowLeft)) {
                self.previous(ctx);
            }
            if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowRight)) {
                self.seek(
                    (self.audio.position() + 5.0)
                        .min(self.current_track().map_or(0.0, |t| t.duration)),
                );
            }
            if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowLeft)) {
                self.seek((self.audio.position() - 5.0).max(0.0));
            }
        }
    }
}

impl eframe::App for PlayerApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.poll(ctx);
        self.refresh_cover(ctx);
        self.keyboard(ctx);
        let drops = ctx.input(|i| i.raw.dropped_files.clone());
        for file in drops {
            if let Some(path) = file.path {
                self.add_folder(path, ctx);
            }
        }
        self.interface(ctx);
        self.prime_gapless();
        if ctx.input(|i| !i.raw.hovered_files.is_empty()) {
            let painter =
                ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("drop-overlay")));
            painter.rect_filled(ctx.content_rect(), 0.0, Color32::from_black_alpha(220));
            text(
                &painter,
                ctx.content_rect().center(),
                Align2::CENTER_CENTER,
                "Отпустите папку",
                18.0,
                FG,
            );
        }
        ctx.request_repaint_after(if self.current.is_some() && !self.audio.paused() {
            Duration::from_millis(200)
        } else {
            Duration::from_millis(500)
        });
    }
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.persist();
    }
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy()
        .trim_start_matches(r"\\?\")
        .to_owned()
}
