use std::cmp::max;

use crate::{App, MetricCache};

pub mod bitplane;
pub mod entropy;
pub mod spectral;

#[derive(Clone, Debug)]
pub struct BinsReport {
    pub values_norm: Vec<f64>, // per-bin entropy normalized to 0..1
    pub mean: f64,             // 0..8 (approx)
    pub std: f64,              // variability indicator
}

impl App {
    pub fn ensure_metric(&mut self, bins: u16) {
        let needs = match &self.metric {
            None => true,
            Some(p) => {
                p.bins != bins
                    || p.offset != self.offset
                    || p.window_len != self.window_len
                    || p.analyzer_idx != self.analyzer_idx
            }
        };
        if !needs {
            return;
        }

        let bins_usize = max(4, bins as usize);
        let a = &self.analyzers[self.analyzer_idx];
        let report = a.analyze_bins(&self.window_data, bins_usize);

        self.metric = Some(MetricCache {
            bins,
            offset: self.offset,
            window_len: self.window_len,
            analyzer_idx: self.analyzer_idx,
            analyzer_name: a.name(),
            analyzer_label: a.metric_label(),
            values: report.values_norm,
            mean: report.mean,
            std: report.std,
        });
    }
}

pub trait Analyzer: Send + Sync {
    fn name(&self) -> &'static str;

    /// Short label for the chart title / axis context.
    fn metric_label(&self) -> &'static str {
        "value (0..1)"
    }

    /// Compute a single value for a slice (0..1).
    fn value_norm(&self, data: &[u8]) -> f64;

    /// Compute a single value for a sparse slice (0..1).
    fn value_norm_sparse(&self, data: &[u8]) -> f64;

    /// Default binning: split by bytes and call `value_norm` per bin.
    fn analyze_bins(&self, data: &[u8], bins: usize) -> BinsReport {
        let bins = bins.max(1);
        if data.is_empty() {
            return BinsReport {
                values_norm: vec![0.0; bins],
                mean: 0.0,
                std: 0.0,
            };
        }

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
            values.push(self.value_norm(&data[start..end]));
        }

        let mean = mean(&values);
        let std = stddev(&values, mean);
        BinsReport {
            values_norm: values,
            mean,
            std,
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
