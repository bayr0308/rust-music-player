use rodio::{Source, source::SeekError};
use serde::{Deserialize, Serialize};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

pub const FREQUENCIES: [f32; 5] = [60.0, 250.0, 1000.0, 4000.0, 12000.0];
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EqualizerSettings {
    pub enabled: bool,
    pub gains: [f32; 5],
}
impl Default for EqualizerSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            gains: [0.0; 5],
        }
    }
}
impl EqualizerSettings {
    pub fn normalize(&mut self) {
        for gain in &mut self.gains {
            *gain = if gain.is_finite() {
                gain.clamp(-12.0, 12.0)
            } else {
                0.0
            };
        }
    }
}
pub type EqualizerControl = Arc<Mutex<EqualizerSettings>>;
#[derive(Clone, Copy)]
struct Coefficients {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
}
impl Coefficients {
    fn peak(frequency: f32, gain: f32, rate: u32) -> Self {
        // RBJ peaking EQ, Q=0.8; coefficients normalized by a0.
        // https://www.w3.org/TR/audio-eq-cookbook/#formulae
        let w = 2.0 * std::f64::consts::PI * frequency.min(rate as f32 * 0.45) as f64 / rate as f64;
        let a = 10.0_f64.powf(gain as f64 / 40.0);
        let alpha = w.sin() / 1.6;
        let a0 = 1.0 + alpha / a;
        Self {
            b0: (1.0 + alpha * a) / a0,
            b1: -2.0 * w.cos() / a0,
            b2: (1.0 - alpha * a) / a0,
            a1: -2.0 * w.cos() / a0,
            a2: (1.0 - alpha / a) / a0,
        }
    }
}
#[derive(Clone, Copy, Default)]
struct State {
    z1: f64,
    z2: f64,
}
impl State {
    fn process(&mut self, x: f64, c: Coefficients) -> f64 {
        let y = c.b0 * x + self.z1;
        self.z1 = c.b1 * x - c.a1 * y + self.z2;
        self.z2 = c.b2 * x - c.a2 * y;
        y
    }
}
/// One DSP instance wraps the entire normalized playlist stream, so filter
/// history continues across track boundaries and channels stay independent.
pub struct Equalized<S> {
    source: S,
    control: EqualizerControl,
    settings: EqualizerSettings,
    coefficients: [Coefficients; 5],
    states: Vec<[State; 5]>,
    channel: usize,
    samples_until_refresh: usize,
    headroom: f64,
}
impl<S: Source<Item = f32>> Equalized<S> {
    pub fn new(source: S, control: EqualizerControl) -> Self {
        let rate = source.sample_rate();
        let channels = source.channels().max(1) as usize;
        let mut result = Self {
            source,
            control,
            settings: EqualizerSettings::default(),
            coefficients: FREQUENCIES.map(|f| Coefficients::peak(f, 0.0, rate)),
            states: vec![[State::default(); 5]; channels],
            channel: 0,
            samples_until_refresh: 0,
            headroom: 1.0,
        };
        result.refresh();
        result
    }
    fn refresh(&mut self) {
        // UI writes never block the realtime audio callback.
        let Ok(settings) = self.control.try_lock() else {
            return;
        };
        if *settings == self.settings {
            return;
        }
        self.settings = settings.clone();
        self.coefficients = std::array::from_fn(|i| {
            Coefficients::peak(FREQUENCIES[i], settings.gains[i], self.source.sample_rate())
        });
        let boost = settings.gains.iter().copied().fold(0.0_f32, f32::max);
        self.headroom = 10.0_f64.powf(-boost as f64 / 20.0);
        self.states.fill([State::default(); 5]);
    }
}
impl<S: Source<Item = f32>> Iterator for Equalized<S> {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        let x = self.source.next()?;
        if self.samples_until_refresh == 0 {
            self.refresh();
            self.samples_until_refresh = 256 * self.states.len();
        }
        self.samples_until_refresh -= 1;
        let channel = self.channel;
        self.channel = (self.channel + 1) % self.states.len();
        if !self.settings.enabled || self.settings.gains == [0.0; 5] {
            return Some(x);
        }
        let mut y = x as f64 * self.headroom;
        for i in 0..5 {
            y = self.states[channel][i].process(y, self.coefficients[i]);
        }
        Some(y.clamp(-1.0, 1.0) as f32)
    }
}
impl<S: Source<Item = f32>> Source for Equalized<S> {
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
        self.source.try_seek(pos)?;
        self.states.fill([State::default(); 5]);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rodio::buffer::SamplesBuffer;
    fn control(settings: EqualizerSettings) -> EqualizerControl {
        Arc::new(Mutex::new(settings))
    }
    #[test]
    fn bypass_is_exact_and_stereo_channels_do_not_leak() {
        let data: Vec<f32> = (0..12000)
            .flat_map(|i| [((i as f32) * 0.13).sin() * 0.2, 0.0])
            .collect();
        let original = SamplesBuffer::new(2, 48000, data.clone());
        assert_eq!(
            Equalized::new(original, control(EqualizerSettings::default())).collect::<Vec<_>>(),
            data
        );
        let eq = Equalized::new(
            SamplesBuffer::new(2, 48000, data),
            control(EqualizerSettings {
                enabled: true,
                gains: [3.0, 0.0, -6.0, 0.0, 2.0],
            }),
        );
        for (i, s) in eq.enumerate() {
            assert!(s.is_finite());
            if i % 2 == 1 {
                assert_eq!(s, 0.0);
            }
        }
    }
    #[test]
    fn one_khz_cut_attenuates_the_requested_band() {
        let data: Vec<f32> = (0..48000)
            .map(|i| (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / 48000.0).sin() * 0.25)
            .collect();
        let output: Vec<_> = Equalized::new(
            SamplesBuffer::new(1, 48000, data.clone()),
            control(EqualizerSettings {
                enabled: true,
                gains: [0.0, 0.0, -12.0, 0.0, 0.0],
            }),
        )
        .collect();
        let rms = |v: &[f32]| (v.iter().map(|x| x * x).sum::<f32>() / v.len() as f32).sqrt();
        let ratio = rms(&output[4800..]) / rms(&data[4800..]);
        assert!((ratio - 0.2512).abs() < 0.015, "ratio={ratio}");
    }
}
