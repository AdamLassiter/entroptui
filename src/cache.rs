use crate::{
    suggest::{Features, Suggestion},
    ui::ViewMode,
};

#[derive(Clone, Debug)]
pub struct MetricSeriesCache {
    pub name: &'static str,
    pub values: Vec<f64>, // 0..1
    pub mean: f64,
    pub std: f64,
}

#[derive(Clone, Debug)]
pub enum MetricCache {
    Single {
        bins: u16,
        offset: u64,
        window_len: u64,
        analyzer_idx: usize,
        analyzer_name: &'static str,
        analyzer_label: &'static str,
        values: Vec<f64>, // 0..1
        mean: f64,
        std: f64,
    },
    Multi {
        bins: u16,
        offset: u64,
        window_len: u64,
        analyzer_idx: usize,
        analyzer_name: &'static str,
        analyzer_label: &'static str,
        series: Vec<MetricSeriesCache>,
    },
}
impl MetricCache {
    fn analyzer_name(&self) -> &'static str {
        match self {
            Self::Single { analyzer_name, .. } => analyzer_name,
            Self::Multi { analyzer_name, .. } => analyzer_name,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ChartCache {
    pub bins: u16,
    pub offset: u64,
    pub window_len: u64,
    pub analyzer_idx: usize,
    pub values: Vec<f64>, // 0..1
    pub mean: f64,
    pub std: f64,
}

#[derive(Clone, Debug)]
pub struct HilbertCache {
    pub side: u16, // grid is side x side, side must be power-of-two
    pub offset: u64,
    pub window_len: u64,
    pub analyzer_idx: usize,
    pub values_row_major: Vec<f64>, // length = side*side, indexed by y*side+x
}

#[derive(Clone, Debug)]
pub struct SuggestCache {
    pub offset: u64,
    pub window_len: u64,
    pub view: ViewMode,
    pub feature_len: u64,
    pub features: Features,
    pub suggestions: Vec<Suggestion>,
}
