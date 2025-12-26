use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::suggest::Suggestion;

pub fn draw_suggestions(f: &mut Frame, area: Rect, suggestions: &[Suggestion]) {
    let block = Block::default().title("Suggestions").borders(Borders::ALL);

    let mut suggestion_lines = Vec::new();
    if suggestions.is_empty() {
        suggestion_lines.push(Line::from(Span::styled(
            "- (no suggestions)",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        suggestion_lines.push(Line::from(Span::styled(
            "Likely content (heuristic)",
            Style::default().fg(Color::Cyan),
        )));
        for s in suggestions.iter().take(6) {
            let pct = (s.confidence as f64 * 100.0).round() as i64;
            suggestion_lines.push(Line::from(Span::styled(
                format!("{:>3}% {}", pct, s.label),
                Style::default().fg(Color::Green),
            )));
            for r in &s.reasons {
                suggestion_lines.push(Line::from(Span::styled(
                    format!("- {}", r),
                    Style::default().fg(Color::Gray),
                )));
            }
        }
    }

    f.render_widget(
        Paragraph::new(suggestion_lines)
            .block(block)
            .wrap(Wrap { trim: true }),
        area,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::suggest::Suggestion;
    use ratatui::layout::Rect;

    #[test]
    fn draw_suggestions_handles_empty_and_nonempty() {
        let backend = ratatui::backend::TestBackend::new(80, 12);
        let mut term = ratatui::Terminal::new(backend).unwrap();

        let empty: Vec<Suggestion> = Vec::new();
        term.draw(|f| {
            draw_suggestions(f, Rect::new(0, 0, 80, 12), &empty);
        })
        .unwrap();

        let populated = vec![Suggestion {
            label: "Test suggestion",
            confidence: 0.8,
            reasons: vec!["reason1".to_string(), "reason2".to_string()],
        }];
        term.draw(|f| {
            draw_suggestions(f, Rect::new(0, 0, 80, 12), &populated);
        })
        .unwrap();
    }
}
