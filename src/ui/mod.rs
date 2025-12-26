mod chart;
mod hex;
mod hilbert;
mod shortcuts;
mod suggestions;
mod summary;

use crate::analysis::BucketSize;
use crate::suggest::{Features, Suggestion};
use crate::ui::chart::draw_chart;
use crate::ui::hex::draw_hex_viewer;
use crate::ui::hilbert::draw_hilbert;
use crate::ui::shortcuts::draw_shortcuts;
use crate::ui::suggestions::draw_suggestions;
use crate::ui::summary::draw_summary;
use crate::{App, HilbertCache, PlotCache};

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use std::cmp::{max, min};
use std::path::Path;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ViewMode {
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

pub fn ensure_views(app: &mut App, size: Rect) {
    // For the Hilbert map: pick the largest power-of-two square that fits.
    let max_side = min(size.width.saturating_sub(2), size.height.saturating_sub(2));
    let side = hilbert::best_pow2_side(max_side as u16);
    if side >= 2 {
        hilbert::ensure_hilbert(app, side);
    }

    let bins = max(10, size.width.saturating_sub(2)) as u16;
    chart::ensure_plot(app, bins);
}

#[allow(clippy::too_many_arguments)]
pub fn draw(
    f: &mut Frame,
    path: &Path,
    file_len: u64,
    offset: u64,
    window_len: u64,
    bucket: BucketSize,
    view: ViewMode,
    window_data: &[u8],
    plot: Option<&PlotCache>,
    hilbert: Option<&HilbertCache>,
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
        bucket,
        view.label(),
        status,
    );
    draw_main(f, root[1], offset, view, plot, hilbert, window_data);
    draw_footer(f, root[2], window_data, plot, features, suggestions);
}

#[allow(clippy::too_many_arguments)]
fn draw_header(
    f: &mut Frame,
    area: Rect,
    path: &Path,
    file_len: u64,
    offset: u64,
    window_len: u64,
    bucket: BucketSize,
    view_label: &str,
    status: &str,
) {
    let title = format!(
        "Entropy TUI | {} | view={} | file={} bytes | offset={} window={} bucket={}",
        path.display(),
        view_label,
        file_len,
        offset,
        window_len,
        bucket.label()
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
            "Controls: ←/→ scroll  PgUp/PgDn page  +/- zoom  b bucket  v view  q quit",
        ));
    }

    let p = Paragraph::new(lines).block(Block::default().borders(Borders::ALL));
    f.render_widget(p, area);
}

fn draw_main(
    f: &mut Frame,
    area: Rect,
    offset: u64,
    view: ViewMode,
    plot: Option<&PlotCache>,
    hilbert: Option<&HilbertCache>,
    window_data: &[u8],
) {
    match view {
        ViewMode::Hilbert => draw_hilbert(f, area, hilbert),
        ViewMode::Chart => draw_chart(f, area, plot),
        ViewMode::Hex => draw_hex_viewer(f, area, offset, window_data),
    }
}

fn draw_footer(
    f: &mut Frame,
    area: Rect,
    window_data: &[u8],
    plot: Option<&PlotCache>,
    features: &Features,
    suggestions: &[Suggestion],
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .split(area);

    draw_summary(f, chunks[0], window_data, plot, features);
    draw_suggestions(f, chunks[1], suggestions);
    draw_shortcuts(f, chunks[2]);
}
