use ahash::AHashMap;

use crate::analysis::Analyzer;

#[derive(Default)]
pub struct EntropyAnalyzer {
    pub bucket: BucketSize,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub enum BucketSize {
    #[default]
    B1,
    B2,
    B4,
}

impl BucketSize {
    pub fn bytes(self) -> usize {
        match self {
            BucketSize::B1 => 1,
            BucketSize::B2 => 2,
            BucketSize::B4 => 4,
        }
    }
}

impl EntropyAnalyzer {
    /// Normalized Shannon entropy for a slice, in range 0..1.
    /// - 0 means perfectly predictable
    /// - 1 means maximally uniform for the chosen bucket size
    pub fn entropy_norm(&self, data: &[u8]) -> f64 {
        let k = self.bucket.bytes();
        if data.len() < k || k == 0 {
            return 0.0;
        }

        // Count symbols of size k (ignore trailing partial).
        let n_syms = data.len() / k;
        if n_syms == 0 {
            return 0.0;
        }

        let h_bits_per_symbol = match self.bucket {
            BucketSize::B1 => Self::entropy_u8(&data[..n_syms]),
            BucketSize::B2 => Self::entropy_u16(data, n_syms),
            BucketSize::B4 => Self::entropy_u32(data, n_syms),
        };

        let max_bits_per_symbol = (8.0 * k as f64).max(1.0);
        (h_bits_per_symbol / max_bits_per_symbol).clamp(0.0, 1.0)
    }

    /// Like `entropy_norm`, but avoids large dense count tables.
    /// This is especially important when computing *many* entropies (e.g. Hilbert maps),
    /// where per-cell allocations of big tables would dominate runtime.
    pub fn entropy_norm_sparse(&self, data: &[u8]) -> f64 {
        let k = self.bucket.bytes();
        if data.len() < k || k == 0 {
            return 0.0;
        }

        let n_syms = data.len() / k;
        if n_syms == 0 {
            return 0.0;
        }

        let h_bits_per_symbol = match self.bucket {
            BucketSize::B1 => Self::entropy_u8(&data[..n_syms]),
            BucketSize::B2 => {
                let mut counts: AHashMap<u16, usize> = AHashMap::with_capacity(n_syms);
                for i in 0..n_syms {
                    let j = i * 2;
                    let sym = u16::from_le_bytes([data[j], data[j + 1]]);
                    *counts.entry(sym).or_insert(0) += 1;
                }
                Self::entropy_from_counts(counts.values().copied(), n_syms)
            }
            BucketSize::B4 => {
                let mut counts: AHashMap<u32, usize> = AHashMap::with_capacity(n_syms);
                for i in 0..n_syms {
                    let j = i * 4;
                    let sym = u32::from_le_bytes([data[j], data[j + 1], data[j + 2], data[j + 3]]);
                    *counts.entry(sym).or_insert(0) += 1;
                }
                Self::entropy_from_counts(counts.values().copied(), n_syms)
            }
        };

        let max_bits_per_symbol = (8.0 * k as f64).max(1.0);
        (h_bits_per_symbol / max_bits_per_symbol).clamp(0.0, 1.0)
    }

    fn entropy_from_counts(counts: impl Iterator<Item = usize>, total: usize) -> f64 {
        if total == 0 {
            return 0.0;
        }
        let total_f = total as f64;
        let mut h = 0.0;
        for c in counts {
            if c == 0 {
                continue;
            }
            let p = c as f64 / total_f;
            h -= p * p.log2();
        }
        h
    }

    fn entropy_u8(data: &[u8]) -> f64 {
        let mut counts = [0usize; 256];
        for &b in data {
            counts[b as usize] += 1;
        }
        Self::entropy_from_counts(counts.into_iter(), data.len())
    }

    fn entropy_u16(data: &[u8], n_syms: usize) -> f64 {
        // 65,536 self.buckets is fine for a TUI-sized window.
        let mut counts = vec![0usize; 65536];
        for i in 0..n_syms {
            let j = i * 2;
            let sym = u16::from_le_bytes([data[j], data[j + 1]]) as usize;
            counts[sym] += 1;
        }
        Self::entropy_from_counts(counts.into_iter(), n_syms)
    }

    fn entropy_u32(data: &[u8], n_syms: usize) -> f64 {
        // Too many possible symbols for a dense table; use a fast hash map.
        let mut counts: AHashMap<u32, usize> = AHashMap::with_capacity(n_syms);
        for i in 0..n_syms {
            let j = i * 4;
            let sym = u32::from_le_bytes([data[j], data[j + 1], data[j + 2], data[j + 3]]);
            *counts.entry(sym).or_insert(0) += 1;
        }
        Self::entropy_from_counts(counts.values().copied(), n_syms)
    }
}

impl Analyzer for EntropyAnalyzer {
    fn name(&self) -> &'static str {
        "Shannon Entropy"
    }

    fn metric_label(&self) -> &'static str {
        match self.bucket {
            BucketSize::B1 => "entropy-1 (0..1)",
            BucketSize::B2 => "entropy-2 (0..1)",
            BucketSize::B4 => "entropy-4 (0..1)",
        }
    }

    fn value_norm(&self, data: &[u8]) -> f64 {
        self.entropy_norm(data)
    }

    fn value_norm_sparse(&self, data: &[u8]) -> f64 {
        self.entropy_norm_sparse(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper for repeating patterns.
    fn repeat_pattern(pattern: &[u8], total_len: usize) -> Vec<u8> {
        pattern.iter().cloned().cycle().take(total_len).collect()
    }

    #[test]
    fn entropy_analyzer_b1_basic() {
        let a = EntropyAnalyzer {
            bucket: BucketSize::B1,
        };
        let constant = vec![42u8; 1024];
        let noisy: Vec<u8> = (0..1024).map(|_| rand::random::<u8>()).collect();
        assert_eq!(a.value_norm(&constant), 0.0);
        let v_noisy = a.value_norm(&noisy);
        assert!(
            v_noisy > 0.9 && v_noisy <= 1.0,
            "noisy bytes should approach 1"
        );
    }

    #[test]
    fn entropy_analyzer_b2_and_b4_match_sparse() {
        let data = repeat_pattern(&[1, 2, 3, 4, 5, 6, 7, 8], 2048);

        for bucket in [BucketSize::B2, BucketSize::B4] {
            let a = EntropyAnalyzer { bucket };
            let dense = a.value_norm(&data);
            let sparse = a.value_norm_sparse(&data);
            assert!((dense - sparse).abs() < 1e-9);
        }
    }

    #[test]
    fn entropy_analyzer_zero_length() {
        let a = EntropyAnalyzer::default();
        assert_eq!(a.value_norm(&[]), 0.0);
    }

    #[test]
    fn larger_buckets_can_change_entropy_level() {
        let pattern = (0..=255u8).cycle().take(4096).collect::<Vec<_>>();
        let e1 = EntropyAnalyzer {
            bucket: BucketSize::B1,
        };
        let e2 = EntropyAnalyzer {
            bucket: BucketSize::B4,
        };
        let h1 = e1.value_norm(&pattern);
        let h2 = e2.value_norm(&pattern);
        assert!(h2 <= 1.0 && h1 <= 1.0);
    }
}
