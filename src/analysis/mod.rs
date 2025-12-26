use ahash::AHashMap;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum BucketSize {
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

    pub fn next(self) -> Self {
        match self {
            BucketSize::B1 => BucketSize::B2,
            BucketSize::B2 => BucketSize::B4,
            BucketSize::B4 => BucketSize::B1,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            BucketSize::B1 => "1 byte",
            BucketSize::B2 => "2 bytes",
            BucketSize::B4 => "4 bytes",
        }
    }
}

#[derive(Clone, Debug)]
pub struct EntropyBinsReport {
    pub values_norm: Vec<f64>,   // per-bin entropy normalized to 0..1
    pub mean_bits_per_byte: f64, // 0..8 (approx)
    pub std_bits_per_byte: f64,  // variability indicator
}

pub trait Analyzer: Send + Sync {
    fn name(&self) -> &'static str;

    /// For now we expose a dedicated entropy-binning method, but you can
    /// generalize this later to a common "analyze bins" interface for
    /// multiple metrics.
    fn analyze_entropy_bins(
        &self,
        data: &[u8],
        bucket: BucketSize,
        bins: usize,
    ) -> EntropyBinsReport;
}

#[derive(Default)]
pub struct EntropyAnalyzer;

/// Normalized Shannon entropy for a slice, in range 0..1.
/// - 0 means perfectly predictable
/// - 1 means maximally uniform for the chosen bucket size
pub fn entropy_norm(data: &[u8], bucket: BucketSize) -> f64 {
    let k = bucket.bytes();
    if data.len() < k || k == 0 {
        return 0.0;
    }

    // Count symbols of size k (ignore trailing partial).
    let n_syms = data.len() / k;
    if n_syms == 0 {
        return 0.0;
    }

    let h_bits_per_symbol = match bucket {
        BucketSize::B1 => entropy_u8(&data[..n_syms]),
        BucketSize::B2 => entropy_u16(data, n_syms),
        BucketSize::B4 => entropy_u32(data, n_syms),
    };

    let max_bits_per_symbol = (8.0 * k as f64).max(1.0);
    (h_bits_per_symbol / max_bits_per_symbol).clamp(0.0, 1.0)
}

/// Like `entropy_norm`, but avoids large dense count tables.
/// This is especially important when computing *many* entropies (e.g. Hilbert maps),
/// where per-cell allocations of big tables would dominate runtime.
pub fn entropy_norm_sparse(data: &[u8], bucket: BucketSize) -> f64 {
    let k = bucket.bytes();
    if data.len() < k || k == 0 {
        return 0.0;
    }

    let n_syms = data.len() / k;
    if n_syms == 0 {
        return 0.0;
    }

    let h_bits_per_symbol = match bucket {
        BucketSize::B1 => entropy_u8(&data[..n_syms]),
        BucketSize::B2 => {
            let mut counts: AHashMap<u16, usize> = AHashMap::with_capacity(n_syms);
            for i in 0..n_syms {
                let j = i * 2;
                let sym = u16::from_le_bytes([data[j], data[j + 1]]);
                *counts.entry(sym).or_insert(0) += 1;
            }
            entropy_from_counts(counts.values().copied(), n_syms)
        }
        BucketSize::B4 => {
            let mut counts: AHashMap<u32, usize> = AHashMap::with_capacity(n_syms);
            for i in 0..n_syms {
                let j = i * 4;
                let sym = u32::from_le_bytes([data[j], data[j + 1], data[j + 2], data[j + 3]]);
                *counts.entry(sym).or_insert(0) += 1;
            }
            entropy_from_counts(counts.values().copied(), n_syms)
        }
    };

    let max_bits_per_symbol = (8.0 * k as f64).max(1.0);
    (h_bits_per_symbol / max_bits_per_symbol).clamp(0.0, 1.0)
}

impl Analyzer for EntropyAnalyzer {
    fn name(&self) -> &'static str {
        "Entropy"
    }

    fn analyze_entropy_bins(
        &self,
        data: &[u8],
        bucket: BucketSize,
        bins: usize,
    ) -> EntropyBinsReport {
        let bins = bins.max(1);
        if data.is_empty() {
            return EntropyBinsReport {
                values_norm: vec![0.0; bins],
                mean_bits_per_byte: 0.0,
                std_bits_per_byte: 0.0,
            };
        }

        // Split by bytes (not by symbols); entropy() will internally ignore
        // trailing partial symbols per bin.
        let bin_size = (data.len() / bins).max(1);
        let mut values = Vec::with_capacity(bins);
        for i in 0..bins {
            let start = i * bin_size;
            if start >= data.len() {
                values.push(0.0);
                continue;
            }
            let end = if i == bins - 1 {
                data.len()
            } else {
                (start + bin_size).min(data.len())
            };
            values.push(entropy_norm(&data[start..end], bucket));
        }

        // Convert normalized entropy to bits/byte:
        // normalized = H / (8*k); H per symbol. Per byte bits ~= normalized*8.
        let bits: Vec<f64> = values.iter().map(|v| v * 8.0).collect();
        let mean = mean(&bits);
        let std = stddev(&bits, mean);

        EntropyBinsReport {
            values_norm: values,
            mean_bits_per_byte: mean,
            std_bits_per_byte: std,
        }
    }
}

fn mean(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        return 0.0;
    }
    xs.iter().sum::<f64>() / xs.len() as f64
}

fn stddev(xs: &[f64], mean: f64) -> f64 {
    if xs.len() < 2 {
        return 0.0;
    }
    let var = xs
        .iter()
        .map(|x| {
            let d = x - mean;
            d * d
        })
        .sum::<f64>()
        / (xs.len() as f64);
    var.sqrt()
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
    entropy_from_counts(counts.into_iter(), data.len())
}

fn entropy_u16(data: &[u8], n_syms: usize) -> f64 {
    // 65,536 buckets is fine for a TUI-sized window.
    let mut counts = vec![0usize; 65536];
    for i in 0..n_syms {
        let j = i * 2;
        let sym = u16::from_le_bytes([data[j], data[j + 1]]) as usize;
        counts[sym] += 1;
    }
    entropy_from_counts(counts.into_iter(), n_syms)
}

fn entropy_u32(data: &[u8], n_syms: usize) -> f64 {
    // Too many possible symbols for a dense table; use a fast hash map.
    let mut counts: AHashMap<u32, usize> = AHashMap::with_capacity(n_syms);
    for i in 0..n_syms {
        let j = i * 4;
        let sym = u32::from_le_bytes([data[j], data[j + 1], data[j + 2], data[j + 3]]);
        *counts.entry(sym).or_insert(0) += 1;
    }
    entropy_from_counts(counts.values().copied(), n_syms)
}
