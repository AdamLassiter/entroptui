use ratatui::{
    Frame,
    layout::Rect,
    text::Line,
    widgets::{Block, Borders, Paragraph, Wrap},
};

pub fn draw_shortcuts(f: &mut Frame<'_>, area: Rect) {
    let block = Block::default().title("Shortcuts").borders(Borders::ALL);

    let notes = vec![
        Line::from("scroll: ←/→ (page) | PgUp/PgDown (window) | Home/End (file)"),
        Line::from("zoom: +/-"),
        Line::from("cycle: c (Entropy [1/2/4 bytes] → Spectral → Bitplane)"),
        Line::from("view: v (Chart → Hilbert → Hex)"),
        Line::from("quit: q | Esc | Ctrl+C"),
    ];

    f.render_widget(
        Paragraph::new(notes).block(block).wrap(Wrap { trim: true }),
        area,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;

    #[test]
    fn draw_shortcuts_safe_to_call() {
        let backend = ratatui::backend::TestBackend::new(60, 6);
        let mut term = ratatui::Terminal::new(backend).unwrap();
        term.draw(|f| {
            draw_shortcuts(f, Rect::new(0, 0, 60, 6));
        })
        .unwrap();
    }
}
