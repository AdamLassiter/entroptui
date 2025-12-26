use std::cmp::max;

use crate::{
    App,
    cache::{MetricCache, MetricSeriesCache},
};

pub mod bitplane;
pub mod entropy;
pub mod spectral;

#[derive(Clone, Debug)]
pub struct BinsReport {
    pub values_norm: Vec<f64>, // per-bin entropy normalized to 0..1
    pub mean: f64,             // 0..8 (approx)
    pub std: f64,              // variability indicator
}

#[derive(Clone, Debug)]
pub struct SeriesBinsReport {
    pub name: &'static str,
    pub values_norm: Vec<f64>,
}

#[derive(Clone, Debug)]
pub struct MultiBinsReport {
    pub series: Vec<SeriesBinsReport>,
}

impl App {
    pub fn ensure_metric(&mut self, bins: u16) {
        let needs = match &self.metric {
            None => true,
            Some(p) => match p {
                MetricCache::Single {
                    bins: pb,
                    offset,
                    window_len,
                    analyzer_idx,
                    ..
                } => {
                    *pb != bins
                        || *offset != self.offset
                        || *window_len != self.window_len
                        || *analyzer_idx != self.analyzer_idx
                }
                MetricCache::Multi {
                    bins: pb,
                    offset,
                    window_len,
                    analyzer_idx,
                    ..
                } => {
                    *pb != bins
                        || *offset != self.offset
                        || *window_len != self.window_len
                        || *analyzer_idx != self.analyzer_idx
                }
            },
        };
        if !needs {
            return;
        }

        let bins_usize = max(4, bins as usize);
        let analyzer = &self.analyzers[self.analyzer_idx];

        if let Some(multi) = analyzer.analyze_bins_multi(&self.window_data, bins_usize) {
            let series = multi
                .series
                .into_iter()
                .map(|s| MetricSeriesCache {
                    name: s.name,
                    values: s.values_norm,
                })
                .collect::<Vec<_>>();

            self.metric = Some(MetricCache::Multi {
                bins,
                offset: self.offset,
                window_len: self.window_len,
                analyzer_idx: self.analyzer_idx,
                analyzer_name: analyzer.name(),
                analyzer_label: analyzer.metric_label(),
                series,
            });
        } else {
            let report = analyzer.analyze_bins(&self.window_data, bins_usize);
            self.metric = Some(MetricCache::Single {
                bins,
                offset: self.offset,
                window_len: self.window_len,
                analyzer_idx: self.analyzer_idx,
                analyzer_name: analyzer.name(),
                analyzer_label: analyzer.metric_label(),
                values: report.values_norm,
            });
        }
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

    /// Optional multi-series output for analyzers that naturally produce
    /// multiple curves (e.g. per-bit-plane entropy).
    fn analyze_bins_multi(&self, _data: &[u8], _bins: usize) -> Option<MultiBinsReport> {
        None
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mean_and_stddev_basic() {
        let xs = [1.0, 2.0, 3.0, 4.0];
        let m = super::mean(&xs);
        assert!((m - 2.5).abs() < 1e-9);
        let s = super::stddev(&xs, m);
        assert!(s > 0.0);
    }

    #[test]
    fn analyzer_bins_match_value_norm() {
        struct Dummy;
        impl Analyzer for Dummy {
            fn name(&self) -> &'static str {
                "dummy"
            }
            fn value_norm(&self, data: &[u8]) -> f64 {
                data.iter().map(|&b| b as f64).sum::<f64>() / 255.0
            }
            fn value_norm_sparse(&self, data: &[u8]) -> f64 {
                self.value_norm(data)
            }
        }

        let d = Dummy;
        let data = vec![1u8, 2, 3, 4, 5, 6, 7, 8];
        let r = d.analyze_bins(&data, 4);
        assert_eq!(r.values_norm.len(), 4);
        assert!(r.mean >= 0.0);
        assert!(r.std >= 0.0);
    }
}
