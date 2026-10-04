use crate::model::{Effects, SAMPLE_RATE, Track};
use std::f32::consts::PI;

pub fn db(value: f32) -> f32 {
    10f32.powf(value / 20.)
}
#[derive(Clone)]
struct Filter {
    b: [f32; 3],
    a: [f32; 2],
    state: [[f32; 2]; 2],
}
impl Filter {
    fn new(kind: u8, frequency: f32, gain: f32, q: f32) -> Self {
        let w = 2. * PI * frequency / SAMPLE_RATE as f32;
        let alpha = w.sin() / (2. * q);
        let cos = w.cos();
        let amp = 10f32.powf(gain / 40.);
        let (b, a) = match kind {
            0 => (
                [(1. + cos) / 2., -(1. + cos), (1. + cos) / 2.],
                [1. + alpha, -2. * cos, 1. - alpha],
            ),
            1 => (
                [(1. - cos) / 2., 1. - cos, (1. - cos) / 2.],
                [1. + alpha, -2. * cos, 1. - alpha],
            ),
            _ => (
                [1. + alpha * amp, -2. * cos, 1. - alpha * amp],
                [1. + alpha / amp, -2. * cos, 1. - alpha / amp],
            ),
        };
        Self {
            b: [b[0] / a[0], b[1] / a[0], b[2] / a[0]],
            a: [a[1] / a[0], a[2] / a[0]],
            state: [[0.; 2]; 2],
        }
    }
    fn sample(&mut self, x: f32, channel: usize) -> f32 {
        let state = &mut self.state[channel];
        let y = self.b[0] * x + state[0];
        state[0] = self.b[1] * x - self.a[0] * y + state[1];
        state[1] = self.b[2] * x - self.a[1] * y;
        y
    }
}
pub struct Processor {
    settings: Effects,
    filters: [Filter; 4],
    envelope: f32,
    compression: f32,
    gate: f32,
}
impl Processor {
    pub fn new(settings: &Effects) -> Self {
        Self {
            settings: settings.clone(),
            filters: [
                Filter::new(0, settings.highpass, 0., 0.707),
                Filter::new(2, 250., settings.low_mid, 0.8),
                Filter::new(2, 3200., settings.presence, 0.7),
                Filter::new(1, settings.lowpass, 0., 0.707),
            ],
            envelope: 0.,
            compression: 1.,
            gate: 1.,
        }
    }
    pub fn process(&mut self, track: &Track, samples: &mut [[f32; 2]]) -> f32 {
        if self.settings != track.effects {
            let mut next = Self::new(&track.effects);
            for i in 0..4 {
                next.filters[i].state = self.filters[i].state;
            }
            next.envelope = self.envelope;
            next.compression = self.compression;
            next.gate = self.gate;
            *self = next;
        }
        let settings = &self.settings;
        let attack = (-1. / (SAMPLE_RATE as f32 * settings.attack / 1000.)).exp();
        let release = (-1. / (SAMPLE_RATE as f32 * settings.release / 1000.)).exp();
        let left_gain = db(track.volume) * (1. - track.pan.max(0.)).sqrt();
        let right_gain = db(track.volume) * (1. + track.pan.min(0.)).sqrt();
        let mut peak: f32 = 0.;
        for sample in samples {
            if !track.fx_bypass {
                for filter in &mut self.filters {
                    for (channel, x) in sample.iter_mut().enumerate() {
                        *x = filter.sample(*x, channel);
                    }
                }
                let level = sample[0].abs().max(sample[1].abs());
                let smoothing = if level > self.envelope {
                    attack
                } else {
                    release
                };
                self.envelope = smoothing * self.envelope + (1. - smoothing) * level;
                let level_db = 20. * self.envelope.max(1e-9).log10();
                let over = level_db - settings.threshold;
                let knee = 6.;
                let reduction = if over < -knee / 2. {
                    0.
                } else if over > knee / 2. {
                    (1. - 1. / settings.ratio) * over
                } else {
                    (1. - 1. / settings.ratio) * (over + knee / 2.).powi(2) / (2. * knee)
                };
                let target = db(-reduction);
                let speed = if target < self.compression {
                    attack
                } else {
                    release
                };
                self.compression = speed * self.compression + (1. - speed) * target;
                let gate_target = db(-settings.gate_reduction
                    * ((settings.gate_threshold - level_db) / 12.).clamp(0., 1.));
                let gate_speed = if gate_target > self.gate {
                    attack
                } else {
                    release
                };
                self.gate = gate_speed * self.gate + (1. - gate_speed) * gate_target;
                let gain = self.compression * self.gate * db(settings.makeup);
                sample[0] *= gain;
                sample[1] *= gain;
            }
            sample[0] *= left_gain;
            sample[1] *= right_gain;
            peak = peak.max(sample[0].abs()).max(sample[1].abs());
        }
        peak
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pan_and_soft_noise_reduction_preserve_finite_audio() {
        let mut track = Track::new("voice", "Voice");
        track.pan = 1.;
        track.effects.gate_reduction = 6.;
        let mut processor = Processor::new(&track.effects);
        let mut samples = vec![[0.001, 0.001]; 48000];
        processor.process(&track, &mut samples);
        assert!(samples.iter().all(|s| s[0] == 0. && s[1].is_finite()));
        assert!(samples[47999][1].abs() < 0.001);
    }
}
