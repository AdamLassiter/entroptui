mod chart;
mod hex;
mod hilbert;
mod shortcuts;
mod suggestions;
mod summary;

use std::{
    cmp::{max, min},
    path::Path,
};

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::{
    App,
    HilbertCursor,
    cache::{ChartCache, HilbertCache, MetricCache},
    suggest::{Features, Suggestion},
    ui::{
        chart::draw_chart,
        hex::draw_hex_viewer,
        hilbert::draw_hilbert,
        shortcuts::draw_shortcuts,
        suggestions::draw_suggestions,
        summary::draw_summary,
    },
};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub enum ViewMode {
    #[default]
    Chart,
    Hilbert,
    Hex,
}

impl ViewMode {
    pub fn next(self) -> Self {
        match self {
            ViewMode::Chart => ViewMode::Hilbert,
            ViewMode::Hilbert => ViewMode::Hex,
            ViewMode::Hex => ViewMode::Chart,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ViewMode::Chart => "Chart",
            ViewMode::Hilbert => "Hilbert",
            ViewMode::Hex => "Hex",
        }
    }
}

impl App {
    pub fn ensure_views(&mut self, size: Rect) -> u16 {
        // For the Hilbert map: pick the largest power-of-two square that fits.
        let max_side = min(size.width.saturating_sub(2), size.height.saturating_sub(2));
        let side = hilbert::best_pow2_side(max_side as u16);
        let bins = max(10, size.width.saturating_sub(2)) as u16;

        if side >= 2 {
            self.ensure_hilbert(side);
        }
        self.ensure_chart(bins);
        self.ensure_hex(size);

        bins
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw(
    f: &mut Frame,
    path: &Path,
    file_len: u64,
    offset: u64,
    window_len: u64,
    view: ViewMode,
    analyzer_name: &str,
    window_data: &[u8],
    metric: Option<&MetricCache>,
    chart: Option<&ChartCache>,
    hilbert: Option<&HilbertCache>,
    hilbert_cursor: Option<HilbertCursor>,
    hilbert_cursor_info: Option<(u64, u64, f64)>,
    hex_base_offset: u64,
    hex_data: &[u8],
    features: &Features,
    suggestions: &[Suggestion],
    status: &str,
) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(10),
        ])
        .split(f.area());

    draw_header(
        f,
        root[0],
        path,
        file_len,
        offset,
        window_len,
        view.label(),
        analyzer_name,
        status,
    );
    draw_main(
        f,
        root[1],
        view,
        metric,
        hilbert,
        hilbert_cursor,
        hex_base_offset,
        hex_data,
    );
    draw_footer(
        f,
        root[2],
        window_data,
        chart,
        features,
        suggestions,
        hilbert_cursor_info,
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_header(
    f: &mut Frame,
    area: Rect,
    path: &Path,
    file_len: u64,
    offset: u64,
    window_len: u64,
    view_label: &str,
    analyzer_name: &str,
    status: &str,
) {
    let title = format!(
        "Entropy TUI | {} | view={} | analyzer={} | file={} bytes | offset={} window={}",
        path.display(),
        view_label,
        analyzer_name,
        file_len,
        offset,
        window_len,
    );

    let mut lines = vec![Line::from(vec![Span::styled(
        title,
        Style::default().fg(Color::Cyan),
    )])];

    if !status.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            status.to_string(),
            Style::default().fg(Color::Red),
        )]));
    } else {
        lines.push(Line::from(
            "Controls: ←/→ scroll | PgUp/PgDn page | +/- zoom | v view | h/j/k/l/Enter move/jump-to cursor (Hilbert) | q quit",
        ));
    }

    let p = Paragraph::new(lines).block(Block::default().borders(Borders::ALL));
    f.render_widget(p, area);
}

#[allow(clippy::too_many_arguments)]
fn draw_main(
    f: &mut Frame,
    area: Rect,
    view: ViewMode,
    metric: Option<&MetricCache>,
    hilbert: Option<&HilbertCache>,
    hilbert_cursor: Option<HilbertCursor>,
    hex_base_offset: u64,
    hex_data: &[u8],
) {
    match view {
        ViewMode::Hilbert => draw_hilbert(f, area, hilbert, hilbert_cursor),
        ViewMode::Chart => draw_chart(f, area, metric),
        ViewMode::Hex => draw_hex_viewer(f, area, hex_base_offset, hex_data),
    }
}

fn draw_footer(
    f: &mut Frame,
    area: Rect,
    window_data: &[u8],
    plot: Option<&ChartCache>,
    features: &Features,
    suggestions: &[Suggestion],
    hilbert_cursor_info: Option<(u64, u64, f64)>,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .split(area);

    draw_summary(
        f,
        chunks[0],
        window_data,
        plot,
        features,
        hilbert_cursor_info,
    );
    draw_suggestions(f, chunks[1], suggestions);
    draw_shortcuts(f, chunks[2]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewmode_next_cycles_correctly() {
        assert_eq!(ViewMode::Chart.next(), ViewMode::Hilbert);
        assert_eq!(ViewMode::Hilbert.next(), ViewMode::Hex);
        assert_eq!(ViewMode::Hex.next(), ViewMode::Chart);
    }

    #[test]
    fn viewmode_label_returns_expected() {
        assert_eq!(ViewMode::Chart.label(), "Chart");
        assert_eq!(ViewMode::Hilbert.label(), "Hilbert");
        assert_eq!(ViewMode::Hex.label(), "Hex");
    }

    #[test]
    fn ensure_views_doesnt_panic() {
        use ratatui::layout::Rect;

        use crate::analysis::Analyzer;

        struct Dummy;
        impl Analyzer for Dummy {
            fn name(&self) -> &'static str {
                "dummy"
            }

            fn value_norm(&self, _: &[u8]) -> f64 {
                0.5
            }

            fn value_norm_sparse(&self, _: &[u8]) -> f64 {
                0.5
            }
        }

        let mut app = App {
            analyzers: vec![Box::new(Dummy)],
            analyzer_idx: 0,
            window_data: vec![0u8; 64],
            offset: 0,
            window_len: 64,
            ..Default::default()
        };

        let size = Rect::new(0, 0, 40, 20);
        let bins = app.ensure_views(size);
        assert!(bins >= 10);
    }
}
