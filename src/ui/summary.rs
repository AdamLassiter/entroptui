use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::{cache::ChartCache, suggest::Features};

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

pub fn draw_summary(
    f: &mut Frame,
    area: Rect,
    window_data: &[u8],
    plot: Option<&ChartCache>,
    features: &Features,
    hilbert_cursor_info: Option<(u64, u64, f64)>,
) {
    let block = Block::default().title("Summary").borders(Borders::ALL);

    let (mean_bpb, std_bpb) = plot.map(|p| (p.mean, p.std)).unwrap_or((0.0, 0.0));

    let mut summary_lines = Vec::new();

    // Not really "good/bad" in a generic way → cyan.
    summary_lines.push(Line::from(vec![
        Span::raw("window bytes read: "),
        num_span(window_data.len().to_string(), Color::Cyan),
        Span::raw(" ["),
        num_span(features.sample_len.to_string(), Color::Cyan),
        Span::raw("]"),
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

    if let Some((abs_start, abs_end, v)) = hilbert_cursor_info {
        summary_lines.push(Line::from(Span::styled(
            format!(
                "hilbert cursor: [0x{:X}..0x{:X}) len={} value={:.3}",
                abs_start,
                abs_end,
                abs_end.saturating_sub(abs_start),
                v
            ),
            Style::default().fg(Color::Cyan),
        )));
    }

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

    let ff_pct = features.ff_ratio * 100.0;
    summary_lines.push(Line::from(vec![
        Span::raw("0xFF ratio: "),
        num_span(format!("{:.3}%", ff_pct), color_for_ratio_low_good(ff_pct)),
    ]));

    let whitespace_pct = features.whitespace_ratio * 100.0;
    summary_lines.push(Line::from(vec![
        Span::raw("whitespace ratio: "),
        num_span(
            format!("{:.3}%", whitespace_pct),
            color_for_ratio_low_good(whitespace_pct),
        ),
    ]));

    let newline_pct = features.newline_ratio * 100.0;
    summary_lines.push(Line::from(vec![
        Span::raw("newline ratio: "),
        num_span(
            format!("{:.3}%", newline_pct),
            color_for_ratio_low_good(newline_pct),
        ),
    ]));

    f.render_widget(
        Paragraph::new(summary_lines)
            .block(block)
            .wrap(Wrap { trim: true }),
        area,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;

    #[test]
    fn color_for_entropy_bpb_ranges() {
        assert_eq!(color_for_entropy_bpb(0.0), Color::Blue);
        assert_eq!(color_for_entropy_bpb(3.0), Color::Yellow);
        assert_eq!(color_for_entropy_bpb(6.0), Color::LightRed);
        assert_eq!(color_for_entropy_bpb(8.0), Color::Red);
    }

    #[test]
    fn color_for_std_bpb_ranges() {
        assert_eq!(color_for_std_bpb(0.1), Color::Green);
        assert_eq!(color_for_std_bpb(0.8), Color::Yellow);
        assert_eq!(color_for_std_bpb(2.0), Color::Red);
    }

    #[test]
    fn ratio_color_functions_high_low_good() {
        assert_eq!(color_for_ratio_high_good(99.0), Color::Green);
        assert_eq!(color_for_ratio_high_good(85.0), Color::Yellow);
        assert_eq!(color_for_ratio_high_good(10.0), Color::Red);

        assert_eq!(color_for_ratio_low_good(0.1), Color::Green);
        assert_eq!(color_for_ratio_low_good(5.0), Color::Yellow);
        assert_eq!(color_for_ratio_low_good(50.0), Color::Red);
    }

    #[test]
    fn num_span_applies_color() {
        let s = num_span("123".into(), Color::Cyan);
        let style = s.style;
        assert_eq!(style.fg.unwrap(), Color::Cyan);
    }

    #[test]
    fn draw_summary_executes_safely() {
        let backend = ratatui::backend::TestBackend::new(80, 10);
        let mut term = ratatui::Terminal::new(backend).unwrap();
        let plot = Some(ChartCache {
            bins: 10,
            offset: 0,
            window_len: 10,
            analyzer_idx: 0,
            values: vec![0.1; 10],
            mean: 0.1,
            std: 0.1,
        });
        let features = Features::default();
        term.draw(|f| {
            draw_summary(
                f,
                Rect::new(0, 0, 80, 10),
                &[0u8; 10],
                plot.as_ref(),
                &features,
                None,
            )
        })
        .unwrap();
    }

    #[test]
    fn color_helpers_never_produce_unknowns() {
        for f in [color_for_entropy_bpb, color_for_std_bpb] {
            for &val in &[0.0, 1.0, 3.0, 6.0, 8.0] {
                match f(val) {
                    Color::Blue
                    | Color::Yellow
                    | Color::LightRed
                    | Color::Red
                    | Color::Green
                    | Color::Cyan => {}
                    other => panic!("unexpected color {:?}", other),
                }
            }
        }
    }
}
