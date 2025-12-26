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
        Line::from("cycle: b (bucket size 1 → 2 → 4 bytes per symbol)"),
        Line::from("view: v (cycle Chart → Hilbert → Hex)"),
        Line::from("quit: q | Esc | Ctrl+C"),
    ];

    f.render_widget(
        Paragraph::new(notes).block(block).wrap(Wrap { trim: true }),
        area,
    );
}
