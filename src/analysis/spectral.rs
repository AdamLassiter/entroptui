use std::sync::Arc;

use rustfft::{Fft, FftPlanner, num_complex::Complex};

use crate::analysis::Analyzer;

/// Spectral flatness (Wiener entropy) of the byte stream treated as a signal.
/// Returns 0..1 where:
/// - 0 => tonal/periodic/peaky spectrum
/// - 1 => flat/noise-like spectrum
pub struct SpectralFlatnessAnalyzer {
    plans: Vec<(usize, Arc<dyn Fft<f64>>)>,
    max_n: usize,
}

impl SpectralFlatnessAnalyzer {
    pub fn new(max_n: usize) -> Self {
        let max_n = max_n.clamp(64, 16384);
        let mut planner = FftPlanner::<f64>::new();

        let mut plans = Vec::new();
        let mut n = 64usize;
        while n <= max_n {
            let fft = planner.plan_fft_forward(n);
            plans.push((n, fft));
            n *= 2;
        }

        Self { plans, max_n }
    }

    fn pick_n(&self, len: usize) -> Option<usize> {
        if len < 64 {
            return None;
        }
        let mut n = len.next_power_of_two();
        if n > self.max_n {
            n = self.max_n;
        }
        if n < 64 {
            return None;
        }
        Some(n)
    }

    fn fft_for(&self, n: usize) -> Option<&Arc<dyn Fft<f64>>> {
        self.plans.iter().find(|(k, _)| *k == n).map(|(_, f)| f)
    }

    fn hann(i: usize, n: usize) -> f64 {
        if n <= 1 {
            return 1.0;
        }
        // 0.5 * (1 - cos(2pi*i/(n-1)))
        let x = (2.0 * std::f64::consts::PI * i as f64) / (n as f64 - 1.0);
        0.5 * (1.0 - x.cos())
    }

    fn spectral_flatness(sample: &[u8], fft: &Arc<dyn Fft<f64>>, n: usize) -> f64 {
        // Resample evenly into N points and window it.
        let len = sample.len();
        let mut buf: Vec<Complex<f64>> = Vec::with_capacity(n);
        for i in 0..n {
            let idx = (i * len) / n;
            let x = sample[idx] as f64 - 127.5;
            let w = Self::hann(i, n);
            buf.push(Complex { re: x * w, im: 0.0 });
        }

        fft.process(&mut buf);

        // Power spectrum excluding DC.
        let eps = 1e-12;
        let mut sum = 0.0;
        let mut sum_log = 0.0;
        let mut count = 0usize;

        // Only use the positive frequencies (1..n/2).
        let kmax = (n / 2).max(2);
        for c in &buf[1..kmax] {
            let p = c.re * c.re + c.im * c.im + eps;
            sum += p;
            sum_log += p.ln();
            count += 1;
        }

        if count == 0 || sum <= 0.0 {
            return 0.0;
        }

        let am = sum / count as f64;
        let gm = (sum_log / count as f64).exp();
        (gm / am).clamp(0.0, 1.0)
    }
}

impl Default for SpectralFlatnessAnalyzer {
    fn default() -> Self {
        // Keep this modest; we may compute this per-bin and in Hilbert cells.
        Self::new(2048)
    }
}

impl Analyzer for SpectralFlatnessAnalyzer {
    fn name(&self) -> &'static str {
        "Spectral flatness"
    }

    fn metric_label(&self) -> &'static str {
        "flatness (0..1)"
    }

    fn value_norm(&self, data: &[u8]) -> f64 {
        let Some(n) = self.pick_n(data.len()) else {
            return 0.0;
        };
        let Some(fft) = self.fft_for(n) else {
            return 0.0;
        };
        Self::spectral_flatness(data, fft, n)
    }

    fn value_norm_sparse(&self, data: &[u8]) -> f64 {
        self.value_norm(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spectral_flatness_empty_small() {
        let a = SpectralFlatnessAnalyzer::default();
        assert_eq!(a.value_norm(&[]), 0.0);
        assert_eq!(a.value_norm(&[1u8; 10]), 0.0);
    }

    #[test]
    fn spectral_flatness_noisy_vs_sine() {
        let n = 1024;
        // Simulated noise
        let noise: Vec<u8> = (0..n).map(|_| rand::random::<u8>()).collect();
        // Simulated periodic signal: full sinewave in byte space
        let sine: Vec<u8> = (0..n)
            .map(|i| (127.5 + 127.5 * (2.0 * std::f64::consts::PI * i as f64 / 32.0).sin()) as u8)
            .collect();

        let a = SpectralFlatnessAnalyzer::default();
        let flat_noise = a.value_norm(&noise);
        let flat_sine = a.value_norm(&sine);

        assert!(flat_noise > flat_sine, "noise spectrum should be flatter");
        assert!((0.0..=1.0).contains(&flat_noise));
        assert!((0.0..=1.0).contains(&flat_sine));
    }

    #[test]
    fn fft_plan_selection_works() {
        let s = SpectralFlatnessAnalyzer::new(2048);
        let n = s.pick_n(300).expect("should pick 512");
        assert!(n.is_power_of_two());
        let fft = s.fft_for(n);
        assert!(fft.is_some());
    }
}
