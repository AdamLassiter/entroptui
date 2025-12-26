use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

pub fn draw_hex_viewer(f: &mut Frame, area: Rect, offset: u64, window_data: &[u8]) {
    let block = Block::default()
        .title("Hex (window start)")
        .borders(Borders::ALL);

    // Inside the block, render up to (height - borders) lines.
    let inner_h = area.height.saturating_sub(2) as usize;
    let bytes_per_line = 16usize;
    let max_bytes = inner_h.saturating_mul(bytes_per_line);
    let data = &window_data[..window_data.len().min(max_bytes)];

    let mut lines: Vec<Line> = Vec::with_capacity(inner_h.max(1));
    if data.is_empty() {
        lines.push(Line::from(Span::styled(
            "(no data)",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for (li, chunk) in data.chunks(bytes_per_line).enumerate() {
            let addr = offset + (li as u64) * (bytes_per_line as u64);

            let mut hex = String::with_capacity(3 * bytes_per_line + 1);
            for i in 0..bytes_per_line {
                if i < chunk.len() {
                    hex.push_str(&format!("{:02X} ", chunk[i]));
                } else {
                    hex.push_str("   ");
                }
                if i == 7 {
                    hex.push(' ');
                }
            }

            let mut ascii = String::with_capacity(bytes_per_line);
            for &b in chunk {
                let c = if (0x20..=0x7E).contains(&b) {
                    b as char
                } else {
                    '.'
                };
                ascii.push(c);
            }
            for _ in chunk.len()..bytes_per_line {
                ascii.push(' ');
            }

            let s = format!("{addr:010X}  {hex}|{ascii}|");
            lines.push(Line::from(Span::styled(
                s,
                Style::default().fg(Color::Gray),
            )));
        }
    }

    let p = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    f.render_widget(p, area);
}
