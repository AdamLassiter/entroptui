use crate::analysis::{Analyzer, MultiBinsReport, SeriesBinsReport};

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
            for (i, bit) in ones.iter_mut().enumerate() {
                *bit += ((b >> i) & 1) as u64;
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
                });
            }
            return Some(MultiBinsReport { series });
        }

        let bin_size = (data.len() / bins).max(1);
        let mut per_bit: [Vec<f64>; 8] = std::array::from_fn(|_| Vec::with_capacity(bins));

        for i in 0..bins {
            let start = i * bin_size;
            if start >= data.len() {
                for b in &mut per_bit {
                    b.push(0.0);
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
        for (i, bit) in per_bit.iter().enumerate() {
            series.push(SeriesBinsReport {
                name: bit_name(i),
                values_norm: bit.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitplane_entropy_empty() {
        let a = BitPlaneEntropyAnalyzer::default();
        let data: [u8; 0] = [];
        let v = a.value_norm(&data);
        assert_eq!(v, 0.0);
    }

    #[test]
    fn bitplane_entropy_random_vs_constant() {
        let a = BitPlaneEntropyAnalyzer::default();
        let random_bytes: Vec<u8> = (0..4096).map(|_| rand::random::<u8>()).collect();
        let zeros = vec![0u8; 4096];
        let h_rand = a.value_norm(&random_bytes);
        let h_zeros = a.value_norm(&zeros);
        assert!(h_rand > h_zeros, "random should have higher entropy");
        assert!((0.0..=1.0).contains(&h_rand));
    }
}
