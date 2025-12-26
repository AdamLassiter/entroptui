use crate::analysis::Analyzer;

/// Average entropy of each bit-plane (0..1), where each plane’s entropy is:
/// \(H(p) = -p\log_2(p) - (1-p)\log_2(1-p)\), max 1 when p=0.5.
#[derive(Default)]
pub struct BitPlaneEntropyAnalyzer;

impl BitPlaneEntropyAnalyzer {
    fn h01(p: f64) -> f64 {
        if p <= 0.0 || p >= 1.0 {
            return 0.0;
        }
        -(p * p.log2() + (1.0 - p) * (1.0 - p).log2())
    }
}

impl Analyzer for BitPlaneEntropyAnalyzer {
    fn name(&self) -> &'static str {
        "Bit-plane entropy"
    }

    fn metric_label(&self) -> &'static str {
        "bit entropy (0..1)"
    }

    fn value_norm(&self, data: &[u8]) -> f64 {
        if data.is_empty() {
            return 0.0;
        }
        let n_bits = (data.len() as f64).max(1.0);
        let mut sum = 0.0;
        for bit in 0..8u8 {
            let mut ones = 0u64;
            for &b in data {
                ones += ((b >> bit) & 1) as u64;
            }
            let p = ones as f64 / n_bits;
            sum += Self::h01(p);
        }
        (sum / 8.0).clamp(0.0, 1.0)
    }

    fn value_norm_sparse(&self, data: &[u8]) -> f64 {
        self.value_norm(data)
    }
}
