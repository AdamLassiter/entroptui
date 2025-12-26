use std::cmp::max;

use crate::{App, ChartCache, MetricCache};

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
    let title = metric
        .map(|p| format!("{} ({})", p.analyzer_name, p.analyzer_label))
        .unwrap_or_else(|| "Metric".to_string());
    let block = Block::default().title(title).borders(Borders::ALL);

    let Some(plot) = chart else {
        f.render_widget(block, area);
        return;
    };

    let points: Vec<(f64, f64)> = plot
        .values
        .iter()
        .enumerate()
        .map(|(i, &v)| (i as f64, v))
        .collect();

    let ds = Dataset::default()
        .name("entropy")
        .marker(symbols::Marker::Braille)
        .style(Style::default().fg(Color::Yellow))
        .graph_type(GraphType::Line)
        .data(&points);

    let x_max = (plot.values.len().saturating_sub(1)).max(1) as f64;

    let chart = Chart::new(vec![ds])
        .block(block)
        .x_axis(
            Axis::default()
                .title("bin")
                .bounds([0.0, x_max])
                .style(Style::default().fg(Color::Gray)),
        )
        .y_axis(
            Axis::default()
                .title("0..1")
                .bounds([0.0, 1.0])
                .style(Style::default().fg(Color::Gray)),
        );

    f.render_widget(chart, area);
}
