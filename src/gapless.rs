use rodio::{Source, source::SeekError};
use std::{sync::mpsc::Sender, time::Duration};

/// Every enqueued track is normalized before entering the queue. Expose that
/// fixed output format even while Rodio's queue is empty or between sources:
/// its own format getters otherwise lag one sample behind the transition.
pub struct QueueOutput {
    source: rodio::queue::SourcesQueueOutput,
    channels: u16,
    sample_rate: u32,
}
impl QueueOutput {
    pub fn new(source: rodio::queue::SourcesQueueOutput, channels: u16, sample_rate: u32) -> Self {
        Self {
            source,
            channels,
            sample_rate,
        }
    }
}
impl Iterator for QueueOutput {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        self.source.next()
    }
}
impl Source for QueueOutput {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        self.channels
    }
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        self.source.try_seek(pos)
    }
}
/// Announces the first actual sample, not a UI timer or estimated duration.
pub struct TrackSource<S> {
    source: S,
    serial: u64,
    started: bool,
    events: Sender<u64>,
}
impl<S> TrackSource<S> {
    pub fn new(source: S, serial: u64, events: Sender<u64>) -> Self {
        Self {
            source,
            serial,
            started: false,
            events,
        }
    }
}
impl<S: Source<Item = f32>> Iterator for TrackSource<S> {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        let sample = self.source.next()?;
        if !self.started {
            self.started = true;
            let _ = self.events.send(self.serial);
        }
        Some(sample)
    }
}
impl<S: Source<Item = f32>> Source for TrackSource<S> {
    fn current_span_len(&self) -> Option<usize> {
        self.source.current_span_len()
    }
    fn channels(&self) -> u16 {
        self.source.channels()
    }
    fn sample_rate(&self) -> u32 {
        self.source.sample_rate()
    }
    fn total_duration(&self) -> Option<Duration> {
        self.source.total_duration()
    }
    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        self.source.try_seek(pos)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::equalizer::{Equalized, EqualizerSettings};
    use rodio::{buffer::SamplesBuffer, queue};
    use std::sync::{Arc, Mutex};
    #[test]
    fn queued_sources_have_no_inserted_silence_at_boundary() {
        let (tx, rx) = std::sync::mpsc::channel();
        let (input, output) = queue::queue(false);
        input.append(TrackSource::new(
            SamplesBuffer::new(2, 48000, vec![0.25; 960]),
            1,
            tx.clone(),
        ));
        input.append(TrackSource::new(
            SamplesBuffer::new(2, 48000, vec![-0.5; 960]),
            2,
            tx,
        ));
        let output = QueueOutput::new(output, 2, 48000);
        let samples: Vec<f32> =
            Equalized::new(output, Arc::new(Mutex::new(EqualizerSettings::default()))).collect();
        assert_eq!(samples.len(), 1920);
        assert!(samples[..960].iter().all(|x| *x == 0.25));
        assert!(samples[960..].iter().all(|x| *x == -0.5));
        assert_eq!(rx.try_iter().collect::<Vec<_>>(), vec![1, 2]);
    }

    #[test]
    fn equalizer_history_is_continuous_across_tracks() {
        let data: Vec<f32> = (0..19200).map(|i| (i as f32 * 0.13).sin() * 0.1).collect();
        let control = Arc::new(Mutex::new(EqualizerSettings {
            enabled: true,
            gains: [4.0, -2.0, 3.0, 0.0, -1.0],
        }));
        let reference: Vec<_> =
            Equalized::new(SamplesBuffer::new(2, 48000, data.clone()), control.clone()).collect();
        let (input, output) = queue::queue(false);
        input.append(SamplesBuffer::new(2, 48000, data[..9600].to_vec()));
        input.append(SamplesBuffer::new(2, 48000, data[9600..].to_vec()));
        let output = QueueOutput::new(output, 2, 48000);
        let actual: Vec<_> = Equalized::new(output, control).collect();
        assert_eq!(actual.len(), reference.len());
        assert!(actual.iter().zip(&reference).all(|(a, b)| a == b));
    }
}
