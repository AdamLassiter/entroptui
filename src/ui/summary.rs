use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span}, widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::{PlotCache, suggest::Features};

fn color_for_entropy_bpb(v: f64) -> Color {
    // 0..8, higher == "bigger"
    if v < 2.0 {
        Color::Blue
    } else if v < 5.5 {
        Color::Yellow
    } else if v < 7.8 {
        Color::LightRed
    } else {
        Color::Red
    }
}

fn color_for_std_bpb(v: f64) -> Color {
    // higher stddev == more mixed/variable
    if v < 0.3 {
        Color::Green
    } else if v < 1.2 {
        Color::Yellow
    } else {
        Color::Red
    }
}

fn color_for_ratio_high_good(pct: f64) -> Color {
    // pct in 0..100
    if pct >= 95.0 {
        Color::Green
    } else if pct >= 80.0 {
        Color::Yellow
    } else {
        Color::Red
    }
}

fn color_for_ratio_low_good(pct: f64) -> Color {
    // pct in 0..100
    if pct <= 1.0 {
        Color::Green
    } else if pct <= 10.0 {
        Color::Yellow
    } else {
        Color::Red
    }
}

fn num_span(text: String, color: Color) -> Span<'static> {
    Span::styled(text, Style::default().fg(color))
}

pub fn draw_summary<'a>(
    f: &mut Frame,
    area: Rect,
    window_data: &'a [u8],
    plot: Option<&PlotCache>,
    features: &Features,
) {
    let block = Block::default().title("Summary").borders(Borders::ALL);

    let (mean_bpb, std_bpb) = plot
        .map(|p| (p.mean_bits_per_byte, p.std_bits_per_byte))
        .unwrap_or((0.0, 0.0));

    let mut summary_lines = Vec::new();

    // Not really "good/bad" in a generic way → cyan.
    summary_lines.push(Line::from(vec![
        Span::raw("window bytes read: "),
        num_span(window_data.len().to_string(), Color::Cyan),
    ]));
    summary_lines.push(Line::from(vec![
        Span::raw("mean entropy: "),
        num_span(format!("{:.3}", mean_bpb), color_for_entropy_bpb(mean_bpb)),
        Span::raw(" bits/byte (0..8)"),
    ]));

    summary_lines.push(Line::from(vec![
        Span::raw("stddev entropy: "),
        num_span(format!("{:.3}", std_bpb), color_for_std_bpb(std_bpb)),
        Span::raw(" bits/byte"),
    ]));

    let printable_pct = features.printable_ratio * 100.0;
    summary_lines.push(Line::from(vec![
        Span::raw("printable ratio: "),
        num_span(
            format!("{:.3}%", printable_pct),
            color_for_ratio_high_good(printable_pct),
        ),
    ]));

    let zero_pct = features.zero_ratio * 100.0;
    summary_lines.push(Line::from(vec![
        Span::raw("zero ratio: "),
        num_span(
            format!("{:.3}%", zero_pct),
            color_for_ratio_low_good(zero_pct),
        ),
    ]));

    f.render_widget(
        Paragraph::new(summary_lines)
            .block(block)
            .wrap(Wrap { trim: true }),
        area,
    );
}
