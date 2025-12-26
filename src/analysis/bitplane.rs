use crate::analysis::{Analyzer, MultiBinsReport, SeriesBinsReport, mean, stddev};

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

    fn bit_plane_entropies(data: &[u8]) -> [f64; 8] {
        if data.is_empty() {
            return [0.0; 8];
        }
        let n = data.len() as f64;
        let mut ones = [0u64; 8];
        for &b in data {
            for bit in 0..8 {
                ones[bit] += ((b >> bit) & 1) as u64;
            }
        }
        let mut hs = [0.0f64; 8];
        for bit in 0..8 {
            let p = ones[bit] as f64 / n;
            hs[bit] = Self::h01(p).clamp(0.0, 1.0);
        }
        hs
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
        let hs = Self::bit_plane_entropies(data);
        (hs.into_iter().sum::<f64>() / 8.0).clamp(0.0, 1.0)
    }

    fn value_norm_sparse(&self, data: &[u8]) -> f64 {
        self.value_norm(data)
    }

    fn analyze_bins_multi(&self, data: &[u8], bins: usize) -> Option<MultiBinsReport> {
        let bins = bins.max(1);
        if data.is_empty() {
            let mut series = Vec::with_capacity(8);
            for bit in 0..8 {
                series.push(SeriesBinsReport {
                    name: bit_name(bit),
                    values_norm: vec![0.0; bins],
                    mean: 0.0,
                    std: 0.0,
                });
            }
            return Some(MultiBinsReport { series });
        }

        let bin_size = (data.len() / bins).max(1);
        let mut per_bit: [Vec<f64>; 8] = std::array::from_fn(|_| Vec::with_capacity(bins));

        for i in 0..bins {
            let start = i * bin_size;
            if start >= data.len() {
                for b in 0..8 {
                    per_bit[b].push(0.0);
                }
                continue;
            }
            let end = if i == bins - 1 {
                data.len()
            } else {
                (start + bin_size).min(data.len())
            };
            let hs = Self::bit_plane_entropies(&data[start..end]);
            for b in 0..8 {
                per_bit[b].push(hs[b]);
            }
        }

        let mut series = Vec::with_capacity(8);
        for bit in 0..8 {
            let meanv = mean(&per_bit[bit]);
            let stdv = stddev(&per_bit[bit], meanv);
            series.push(SeriesBinsReport {
                name: bit_name(bit),
                values_norm: per_bit[bit].clone(),
                mean: meanv,
                std: stdv,
            });
        }

        Some(MultiBinsReport { series })
    }
}

fn bit_name(bit: usize) -> &'static str {
    match bit {
        0 => "bit 0 (LSB)",
        1 => "bit 1",
        2 => "bit 2",
        3 => "bit 3",
        4 => "bit 4",
        5 => "bit 5",
        6 => "bit 6",
        7 => "bit 7 (MSB)",
        _ => "bit ?",
    }
}
