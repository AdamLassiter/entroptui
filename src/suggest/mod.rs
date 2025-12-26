mod b64;
mod compress_encrypt;
mod hex;
mod low_entropy;
mod magic;
mod mixed_region;
mod structured_bin;
mod text;

use crate::{App, SuggestCache, suggest::magic::MagicHit, ui::ViewMode};

#[derive(Clone, Debug)]
pub struct Suggestion {
    pub label: &'static str,
    pub confidence: f32, // 0..1
    pub reasons: Vec<String>,
}

impl Suggestion {
    fn new(label: &'static str, confidence: f32) -> Self {
        Self {
            label,
            confidence: confidence.clamp(0.0, 1.0),
            reasons: Vec::new(),
        }
    }

    fn reason(mut self, r: impl Into<String>) -> Self {
        self.reasons.push(r.into());
        self
    }
}

pub struct SuggestInput<'a> {
    pub data: &'a [u8],
    pub offset: u64,
    pub entropy_mean_bpb: f64, // 0..8
    pub entropy_std_bpb: f64,
    pub entropy_bins_norm: Option<&'a [f64]>, // 0..1, optional
}

#[derive(Clone, Debug, Default)]
pub struct Features {
    pub sample_len: usize,

    pub zero_ratio: f64,
    pub ff_ratio: f64,
    pub printable_ratio: f64,
    pub whitespace_ratio: f64,
    pub newline_ratio: f64,
    pub utf8_valid: bool,

    pub base64_ratio: f64,
    pub base64_padding_ratio: f64,
    pub hex_ratio: f64,
    pub hex_digits_even: bool,

    pub top1_ratio: f64,
    pub chi_square_256: f64,
    pub adjacent_equal_ratio: f64,

    pub entropy_min_norm: f64,
    pub entropy_max_norm: f64,

    pub magic_hits: Vec<MagicHit>,
}

impl Features {
    fn compute(input: &SuggestInput) -> Self {
        let data = input.data;
        let sample_len = data.len().min(64 * 1024);
        let sample = &data[..sample_len];

        let mut hist = [0u32; 256];
        let mut zeros = 0u32;
        let mut ffs = 0u32;
        let mut printable = 0u32;
        let mut whitespace = 0u32;
        let mut newlines = 0u32;

        let mut base64_ok = 0u32;
        let mut base64_pad = 0u32;
        let mut hex_ok = 0u32;
        let mut hex_digits = 0u32;

        let mut adj_eq = 0u32;

        for (i, &b) in sample.iter().enumerate() {
            hist[b as usize] += 1;
            if b == 0x00 {
                zeros += 1;
            }
            if b == 0xFF {
                ffs += 1;
            }

            let is_ws = matches!(b, b' ' | b'\t' | b'\r' | b'\n');
            let is_nl = b == b'\n';
            let is_print = (0x20..=0x7E).contains(&b) || is_ws;

            if is_print {
                printable += 1;
            }
            if is_ws {
                whitespace += 1;
            }
            if is_nl {
                newlines += 1;
            }

            let is_b64 =
                matches!(b, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'+' | b'/' | b'=') || is_ws;
            if is_b64 {
                base64_ok += 1;
            }
            if b == b'=' {
                base64_pad += 1;
            }

            let is_hex = b.is_ascii_hexdigit() || is_ws;
            if is_hex {
                hex_ok += 1;
            }
            if b.is_ascii_hexdigit() {
                hex_digits += 1;
            }

            if i + 1 < sample.len() && sample[i + 1] == b {
                adj_eq += 1;
            }
        }

        let n = sample_len.max(1) as f64;
        let zero_ratio = zeros as f64 / n;
        let ff_ratio = ffs as f64 / n;
        let printable_ratio = printable as f64 / n;
        let whitespace_ratio = whitespace as f64 / n;
        let newline_ratio = newlines as f64 / n;

        let base64_ratio = base64_ok as f64 / n;
        let base64_padding_ratio = base64_pad as f64 / n;
        let hex_ratio = hex_ok as f64 / n;
        let hex_digits_even = hex_digits.is_multiple_of(2);

        let top1 = hist.iter().copied().max().unwrap_or(0) as f64;
        let top1_ratio = top1 / n;
        let chi_square_256 = chi_square_uniform_256(&hist, sample_len);
        let adjacent_equal_ratio = adj_eq as f64 / n;

        let (entropy_min_norm, entropy_max_norm) =
            input.entropy_bins_norm.map(min_max).unwrap_or((0.0, 0.0));

        let utf8_valid = std::str::from_utf8(sample).is_ok();
        let magic_hits = magic::scan_magic(sample);

        Self {
            sample_len,
            zero_ratio,
            ff_ratio,
            printable_ratio,
            whitespace_ratio,
            newline_ratio,
            utf8_valid,
            base64_ratio,
            base64_padding_ratio,
            hex_ratio,
            hex_digits_even,
            top1_ratio,
            chi_square_256,
            adjacent_equal_ratio,
            entropy_min_norm,
            entropy_max_norm,
            magic_hits,
        }
    }
}

fn min_max(xs: &[f64]) -> (f64, f64) {
    let mut minv = f64::INFINITY;
    let mut maxv = f64::NEG_INFINITY;
    for &x in xs {
        minv = minv.min(x);
        maxv = maxv.max(x);
    }
    if !minv.is_finite() || !maxv.is_finite() {
        (0.0, 0.0)
    } else {
        (minv, maxv)
    }
}

fn chi_square_uniform_256(hist: &[u32; 256], n: usize) -> f64 {
    if n == 0 {
        return 0.0;
    }
    let expected = n as f64 / 256.0;
    if expected <= 0.0 {
        return 0.0;
    }
    let mut chi2 = 0.0;
    for &c in hist {
        let obs = c as f64;
        let d = obs - expected;
        chi2 += (d * d) / expected;
    }
    chi2
}

pub trait Heuristic: Send + Sync {
    fn name(&self) -> &'static str;
    fn apply(&self, input: &SuggestInput, feats: &Features, out: &mut Vec<Suggestion>);
}

pub struct SuggestEngine {
    heuristics: Vec<Box<dyn Heuristic>>,
    pub max_results: usize,
}

impl SuggestEngine {
    pub fn new() -> Self {
        Self {
            heuristics: Vec::new(),
            max_results: 6,
        }
    }

    pub fn with(mut self, h: impl Heuristic + 'static) -> Self {
        self.heuristics.push(Box::new(h));
        self
    }

    pub fn suggest(&self, input: SuggestInput) -> (Features, Vec<Suggestion>) {
        let feats = Features::compute(&input);
        let mut raw: Vec<Suggestion> = Vec::new();
        for h in &self.heuristics {
            h.apply(&input, &feats, &mut raw);
        }

        let mut merged: Vec<Suggestion> = Vec::new();
        for s in raw {
            if let Some(existing) = merged.iter_mut().find(|x| x.label == s.label) {
                if s.confidence > existing.confidence {
                    existing.confidence = s.confidence;
                }
                for r in s.reasons {
                    if !existing.reasons.iter().any(|e| e == &r) {
                        existing.reasons.push(r);
                    }
                }
            } else {
                merged.push(s);
            }
        }

        merged.sort_by(|a, b| b.confidence.total_cmp(&a.confidence));
        merged.truncate(self.max_results.max(1));
        (feats, merged)
    }
}

impl Default for SuggestEngine {
    fn default() -> Self {
        SuggestEngine::new()
            .with(magic::MagicBytesHeuristic)
            .with(b64::Base64Heuristic)
            .with(hex::HexHeuristic)
            .with(text::TextHeuristic)
            .with(compress_encrypt::CompressedVsEncryptedHeuristic)
            .with(low_entropy::LowEntropyHeuristic)
            .with(mixed_region::MixedRegionsHeuristic)
            .with(structured_bin::StructuredBinaryHeuristic)
    }
}

pub fn ensure_suggestions(app: &mut App) {
    let Some(plot) = app.metric.as_ref() else {
        return;
    };

    // In Hex view mode, compute features on the bytes actually visible in the
    // hex viewer (the "hex window"), not the entire analysis window.
    let (feature_data, feature_len) = if app.view == ViewMode::Hex {
        let n = (app.hex_page_bytes as usize).max(1);
        let end = app.window_data.len().min(n);
        (&app.window_data[..end], end as u64)
    } else {
        (&app.window_data[..], app.window_data.len() as u64)
    };

    let needs = match &app.suggest_cache {
        None => true,
        Some(c) => {
            c.offset != app.offset
                || c.window_len != app.window_len
                || c.view != app.view
                || c.feature_len != feature_len
        }
    };
    if !needs {
        return;
    }

    let input = SuggestInput {
        data: feature_data,
        offset: app.offset,
        entropy_mean_bpb: plot.mean,
        entropy_std_bpb: plot.std,
        entropy_bins_norm: Some(&plot.values),
    };

    let (features, suggestions) = app.suggest_engine.suggest(input);
    app.suggest_cache = Some(SuggestCache {
        offset: app.offset,
        window_len: app.window_len,
        view: app.view,
        feature_len,
        features,
        suggestions,
    });
}
