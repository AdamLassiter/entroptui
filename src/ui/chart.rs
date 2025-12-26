use std::cmp::max;

use crate::{App, PlotCache};

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    symbols,
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType},
};

pub fn ensure_plot(app: &mut App, bins: u16) {
    let needs = match &app.plot {
        None => true,
        Some(p) => {
            p.bins != bins
                || p.bucket != app.bucket
                || p.offset != app.offset
                || p.window_len != app.window_len
        }
    };
    if !needs {
        return;
    }

    let bins_usize = max(4, bins as usize);
    let report = app
        .analyzer
        .analyze_entropy_bins(&app.window_data, app.bucket, bins_usize);

    app.plot = Some(PlotCache {
        bins,
        bucket: app.bucket,
        offset: app.offset,
        window_len: app.window_len,
        values: report.values_norm,
        mean_bits_per_byte: report.mean_bits_per_byte,
        std_bits_per_byte: report.std_bits_per_byte,
    });
}

pub fn draw_chart(f: &mut Frame, area: Rect, plot: Option<&PlotCache>) {
    let block = Block::default()
        .title("Entropy (normalized 0..1)")
        .borders(Borders::ALL);

    let Some(plot) = plot else {
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
                .title("H")
                .bounds([0.0, 1.0])
                .style(Style::default().fg(Color::Gray)),
        );

    f.render_widget(chart, area);
}
