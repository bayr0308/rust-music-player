use crate::{
    equalizer::{Equalized, EqualizerControl, EqualizerSettings},
    gapless::{QueueOutput, TrackSource},
};
use rodio::{
    Decoder, OutputStream, OutputStreamBuilder, Sink, Source,
    cpal::{
        self,
        traits::{DeviceTrait, HostTrait},
    },
    source::UniformSourceIterator,
};
use std::{
    collections::VecDeque,
    fs::File,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, Sender},
    },
    time::{Duration, Instant},
};

#[derive(Default)]
pub struct Audio {
    stream: Option<OutputStream>,
    sink: Option<Sink>,
    device: Option<cpal::Device>,
    output_name: Option<String>,
    preferred_output: Option<String>,
    path: Option<PathBuf>,
    errors: Option<Receiver<String>>,
    checked_at: Option<Instant>,
    reported_error: Option<String>,
    failed_stream: bool,
    equalizer: EqualizerControl,
    events_tx: Option<Sender<u64>>,
    events_rx: Option<Receiver<u64>>,
    scheduled: VecDeque<Scheduled>,
    transitions: Vec<usize>,
    serial: u64,
    current_id: usize,
}
#[derive(Clone)]
struct Scheduled {
    serial: u64,
    id: usize,
    path: PathBuf,
    initial: bool,
}

struct OpenedOutput {
    stream: OutputStream,
    device: cpal::Device,
    name: String,
    errors: Receiver<String>,
}
impl Audio {
    pub fn set_equalizer(&self, mut settings: EqualizerSettings) {
        settings.normalize();
        if let Ok(mut current) = self.equalizer.lock() {
            *current = settings;
        }
    }
    fn connect_sink(stream: &OutputStream, equalizer: EqualizerControl) -> Sink {
        let (sink, source) = Sink::new();
        let config = stream.config();
        let source = QueueOutput::new(source, config.channel_count(), config.sample_rate());
        stream.mixer().add(Equalized::new(source, equalizer));
        sink
    }
    fn reset_events(&mut self) {
        let (tx, rx) = mpsc::channel();
        self.events_tx = Some(tx);
        self.events_rx = Some(rx);
        self.scheduled.clear();
        self.serial = 0;
    }
    fn tracked<S: Source<Item = f32>>(
        &mut self,
        source: S,
        path: &Path,
        id: usize,
        initial: bool,
    ) -> TrackSource<UniformSourceIterator<S>> {
        self.serial += 1;
        self.scheduled.push_back(Scheduled {
            serial: self.serial,
            id,
            path: path.to_owned(),
            initial,
        });
        let config = self.stream.as_ref().unwrap().config();
        TrackSource::new(
            UniformSourceIterator::new(source, config.channel_count(), config.sample_rate()),
            self.serial,
            self.events_tx.as_ref().unwrap().clone(),
        )
    }
    fn sync_events(&mut self) {
        let events: Vec<_> = self
            .events_rx
            .as_ref()
            .map(|rx| rx.try_iter().collect())
            .unwrap_or_default();
        for serial in events {
            if let Some(index) = self.scheduled.iter().position(|s| s.serial == serial) {
                self.scheduled.drain(..index);
                let active = self.scheduled.front_mut().unwrap();
                self.path = Some(active.path.clone());
                self.current_id = active.id;
                if !active.initial {
                    self.transitions.push(active.id);
                    active.initial = true;
                }
            }
        }
    }
    pub fn take_transitions(&mut self) -> Vec<usize> {
        self.sync_events();
        std::mem::take(&mut self.transitions)
    }
    pub fn upcoming_count(&self) -> usize {
        self.scheduled.len().saturating_sub(1)
    }
    pub fn enqueue(&mut self, path: &Path, id: usize) -> Result<(), String> {
        let source = Decoder::try_from(File::open(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        let source = self.tracked(source, path, id, false);
        self.sink
            .as_ref()
            .ok_or("Нет активного воспроизведения")?
            .append(source);
        Ok(())
    }
    pub fn clear_upcoming(&mut self) -> Result<(), String> {
        if let Some(device) = self.device.clone() {
            self.reconnect(device)?;
        }
        Ok(())
    }
    pub fn with_output(preferred_output: Option<String>) -> Self {
        Self {
            preferred_output,
            equalizer: Arc::new(Mutex::new(EqualizerSettings::default())),
            ..Self::default()
        }
    }
    pub fn available_outputs() -> Result<Vec<String>, String> {
        cpal::default_host()
            .output_devices()
            .map_err(|e| format!("Не удалось получить аудиоустройства: {e}"))?
            .map(|d| d.name().map_err(|e| e.to_string()))
            .collect()
    }
    pub fn output_name(&self) -> Option<&str> {
        self.output_name.as_deref()
    }
    fn resolve_output(preferred: Option<&str>) -> Result<cpal::Device, String> {
        let host = cpal::default_host();
        if let Some(name) = preferred {
            host.output_devices()
                .map_err(|e| e.to_string())?
                .find(|d| d.name().is_ok_and(|n| n == name))
                .ok_or_else(|| {
                    format!(
                        "Аудиоустройство «{name}» недоступно. Выберите другой выход в меню «…»."
                    )
                })
        } else {
            host.default_output_device()
                .ok_or_else(|| "В Windows нет устройства вывода по умолчанию.".into())
        }
    }
    fn open_output(device: cpal::Device) -> Result<OpenedOutput, String> {
        let name = device.name().map_err(|e| e.to_string())?;
        let (tx, errors) = mpsc::channel();
        // Retry formats only on this endpoint. Rodio's open_default_stream can
        // silently fall back to another physical device, which is surprising.
        let mut stream = OutputStreamBuilder::from_device(device.clone())
            .map_err(|e| format!("Не удалось открыть «{name}»: {e}"))?
            .with_error_callback(move |e| {
                let _ = tx.send(format!("Ошибка аудиовыхода: {e}"));
            })
            .open_stream_or_fallback()
            .map_err(|e| format!("Не удалось открыть «{name}»: {e}"))?;
        stream.log_on_drop(false);
        Ok(OpenedOutput {
            stream,
            device,
            name,
            errors,
        })
    }
    fn install_output(&mut self, output: OpenedOutput) {
        self.stream = Some(output.stream);
        self.device = Some(output.device);
        self.output_name = Some(output.name);
        self.errors = Some(output.errors);
        self.reported_error = None;
        self.failed_stream = false;
    }
    pub fn play(&mut self, path: &Path, volume: f32) -> Result<(), String> {
        self.play_track(path, volume, 0)
    }
    pub fn play_track(&mut self, path: &Path, volume: f32, id: usize) -> Result<(), String> {
        let file = File::open(path).map_err(|e| format!("Не удалось открыть файл: {e}"))?;
        let source =
            Decoder::try_from(file).map_err(|e| format!("Не удалось воспроизвести файл: {e}"))?;
        let device = Self::resolve_output(self.preferred_output.as_deref())?;
        if self.stream.is_none()
            || self
                .device
                .as_ref()
                .is_none_or(|old| !same_device(old, &device))
        {
            let output = Self::open_output(device)?;
            // Only retire the old stream after the desired endpoint opens.
            self.stop();
            self.install_output(output);
        }
        self.stop();
        self.reset_events();
        let sink = Self::connect_sink(self.stream.as_ref().unwrap(), self.equalizer.clone());
        sink.set_volume(volume.clamp(0.0, 1.0));
        sink.append(self.tracked(source, path, id, true));
        self.current_id = id;
        self.sink = Some(sink);
        self.path = Some(path.to_owned());
        self.reported_error = None;
        Ok(())
    }
    /// Switch output transactionally, preserving playback position, volume and pause.
    pub fn select_output(&mut self, preferred: Option<String>) -> Result<(), String> {
        let device = Self::resolve_output(preferred.as_deref())?;
        self.reconnect(device)?;
        self.preferred_output = preferred;
        Ok(())
    }
    fn reconnect(&mut self, device: cpal::Device) -> Result<(), String> {
        self.sync_events();
        let output = Self::open_output(device)?;
        let (tx, rx) = mpsc::channel();
        let replacement = if let (Some(path), Some(old)) = (&self.path, &self.sink) {
            if !old.empty() {
                let source = Decoder::try_from(File::open(path).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
                let sink = Self::connect_sink(&output.stream, self.equalizer.clone());
                sink.pause();
                sink.set_volume(old.volume());
                let config = output.stream.config();
                let source = UniformSourceIterator::new(
                    source,
                    config.channel_count(),
                    config.sample_rate(),
                );
                sink.append(TrackSource::new(source, 1, tx.clone()));
                sink.try_seek(old.get_pos())
                    .map_err(|e| format!("Не удалось сохранить позицию: {e}"))?;
                Some((sink, old.is_paused()))
            } else {
                None
            }
        } else {
            None
        };
        let path = self.path.clone();
        let id = self.current_id;
        let transitions = std::mem::take(&mut self.transitions);
        self.stop();
        self.install_output(output);
        self.path = path.clone();
        self.current_id = id;
        self.transitions = transitions;
        self.events_tx = Some(tx);
        self.events_rx = Some(rx);
        self.serial = 1;
        if let Some((sink, paused)) = replacement {
            self.scheduled.push_back(Scheduled {
                serial: 1,
                id,
                path: path.unwrap(),
                initial: true,
            });
            if !paused {
                sink.play();
            }
            self.sink = Some(sink);
        }
        Ok(())
    }
    /// Poll outside the realtime callback; follows Windows default while playing.
    pub fn check_output(&mut self) -> Option<String> {
        self.sync_events();
        if self.sink.as_ref().is_none_or(Sink::empty)
            || self
                .checked_at
                .is_some_and(|t| t.elapsed() < Duration::from_secs(1))
        {
            return None;
        }
        self.checked_at = Some(Instant::now());
        let stream_error = self.errors.as_ref().and_then(|rx| rx.try_recv().ok());
        self.failed_stream |= stream_error.is_some();
        let result = Self::resolve_output(self.preferred_output.as_deref()).and_then(|device| {
            if self.failed_stream
                || self
                    .device
                    .as_ref()
                    .is_none_or(|old| !same_device(old, &device))
            {
                self.reconnect(device)
            } else {
                Ok(())
            }
        });
        match result {
            Ok(()) => {
                self.reported_error = None;
                None
            }
            Err(error) => {
                if let Some(sink) = &self.sink {
                    sink.pause();
                }
                if self.reported_error.as_ref() == Some(&error) {
                    return None;
                }
                self.reported_error = Some(error.clone());
                Some(error)
            }
        }
    }
    pub fn stop(&mut self) {
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }
        self.path = None;
        self.scheduled.clear();
        self.transitions.clear();
        self.events_tx = None;
        self.events_rx = None;
    }
    pub fn toggle(&self) {
        if let Some(sink) = &self.sink {
            if sink.is_paused() {
                sink.play();
            } else {
                sink.pause();
            }
        }
    }
    pub fn paused(&self) -> bool {
        self.sink.as_ref().is_none_or(Sink::is_paused)
    }
    pub fn finished(&self) -> bool {
        self.sink.as_ref().is_some_and(Sink::empty)
    }
    pub fn position(&self) -> f64 {
        self.sink
            .as_ref()
            .map_or(0.0, |s| s.get_pos().as_secs_f64())
    }
    pub fn volume(&self, value: f32) {
        if let Some(s) = &self.sink {
            s.set_volume(value.clamp(0.0, 1.0));
        }
    }
    pub fn seek(&self, seconds: f64) -> Result<(), String> {
        if let Some(s) = &self.sink {
            s.try_seek(Duration::from_secs_f64(seconds.max(0.0)))
                .map_err(|e| format!("Перемотка недоступна: {e}"))?;
        }
        Ok(())
    }
}
fn same_device(left: &cpal::Device, right: &cpal::Device) -> bool {
    #[cfg(windows)]
    {
        // CPAL's WASAPI equality compares endpoint IDs, including identically named devices.
        let (cpal::platform::DeviceInner::Wasapi(left), cpal::platform::DeviceInner::Wasapi(right)) =
            (left.as_inner(), right.as_inner());
        left == right
    }
    #[cfg(not(windows))]
    {
        left.name()
            .ok()
            .zip(right.name().ok())
            .is_some_and(|(a, b)| a == b)
    }
}
