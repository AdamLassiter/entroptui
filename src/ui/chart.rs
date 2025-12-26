use std::cmp::max;

use crate::{
    App,
    cache::{ChartCache, MetricCache},
};

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    symbols,
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType},
};

impl App {
    pub fn ensure_chart(&mut self, bins: u16) {
        let needs = match &self.chart {
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
        let report = self.analyzers[self.analyzer_idx].analyze_bins(&self.window_data, bins_usize);

        self.chart = Some(ChartCache {
            bins,
            offset: self.offset,
            window_len: self.window_len,
            analyzer_idx: self.analyzer_idx,
            values: report.values_norm,
            mean: report.mean,
            std: report.std,
        });
    }
}

pub fn draw_chart(
    f: &mut Frame,
    area: Rect,
    metric: Option<&MetricCache>,
    chart: Option<&ChartCache>,
) {
    let (title, y_title) = match metric {
        Some(MetricCache::Single {
            analyzer_name,
            analyzer_label,
            ..
        }) => (format!("{} ({})", analyzer_name, analyzer_label), "0..1"),
        Some(MetricCache::Multi {
            analyzer_name,
            analyzer_label,
            ..
        }) => (format!("{} ({})", analyzer_name, analyzer_label), "0..1"),
        None => ("Metric".to_string(), "0..1"),
    };

    let block = Block::default().title(title).borders(Borders::ALL);

    let Some(metric) = metric else {
        f.render_widget(block, area);
        return;
    };

    let mut datasets = Vec::new();
    let x_max;

    match metric {
        MetricCache::Single { values, .. } => {
            let points: Vec<(f64, f64)> = values
                .iter()
                .enumerate()
                .map(|(i, &v)| (i as f64, v))
                .collect();
            x_max = (values.len().saturating_sub(1)).max(1) as f64;

            datasets.push(
                Dataset::default()
                    .name("value")
                    .marker(symbols::Marker::Braille)
                    .style(Style::default().fg(Color::Yellow))
                    .graph_type(GraphType::Line)
                    .data(points.as_slice()),
            );

            render_chart(f, area, y_title, block, datasets, x_max);
        }
        MetricCache::Multi { series, .. } => {
            // Palette for up to 8 bit-planes.
            let colors = [
                Color::LightBlue,
                Color::Cyan,
                Color::LightGreen,
                Color::Green,
                Color::Yellow,
                Color::LightRed,
                Color::Red,
                Color::Magenta,
            ];

            let max_len = series.iter().map(|s| s.values.len()).max().unwrap_or(0);
            x_max = (max_len.saturating_sub(1)).max(1) as f64;

            // First collect all point vectors (so we don't push after borrowing).
            let mut points_store: Vec<Vec<(f64, f64)>> = Vec::with_capacity(series.len());
            for s in series.iter() {
                let pts: Vec<(f64, f64)> = s
                    .values
                    .iter()
                    .enumerate()
                    .map(|(j, &v)| (j as f64, v))
                    .collect();
                points_store.push(pts);
            }

            // Now build datasets borrowing from points_store.
            let mut datasets = Vec::with_capacity(series.len());
            for (i, s) in series.iter().enumerate() {
                let color = colors[i % colors.len()];
                datasets.push(
                    Dataset::default()
                        .name(s.name)
                        .marker(symbols::Marker::Braille)
                        .style(Style::default().fg(color))
                        .graph_type(GraphType::Line)
                        .data(points_store[i].as_slice()),
                );
            }

            render_chart(f, area, y_title, block, datasets, x_max);
        }
    }
}

fn render_chart(
    f: &mut Frame<'_>,
    area: Rect,
    y_title: &str,
    block: Block<'_>,
    datasets: Vec<Dataset<'_>>,
    x_max: f64,
) {
    let chart = Chart::new(datasets)
        .block(block)
        .x_axis(
            Axis::default()
                .title("bin")
                .bounds([0.0, x_max])
                .style(Style::default().fg(Color::Gray)),
        )
        .y_axis(
            Axis::default()
                .title(y_title)
                .bounds([0.0, 1.0])
                .style(Style::default().fg(Color::Gray)),
        );

    f.render_widget(chart, area);
}
