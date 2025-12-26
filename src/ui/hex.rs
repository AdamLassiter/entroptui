use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::App;

impl App {
    pub fn ensure_hex(&mut self, size: Rect) {
        self.hex_page_bytes = compute_hex_page_bytes(size);
    }
}

fn compute_hex_page_bytes(size: Rect) -> u64 {
    // Compute how many bytes the hex viewer can display (one "page").
    // This mirrors the layout in ui.rs: root vertical split (3, main, 6)
    // then main split horizontally (68% left, 32% right), and hex uses
    // (height-2) lines at 16 bytes/line.
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(6),
        ])
        .split(size);
    let main = root[1];
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(68), Constraint::Percentage(32)])
        .split(main);
    let hex_area: Rect = cols[1];
    let inner_h = hex_area.height.saturating_sub(2) as u64;
    inner_h.saturating_mul(16)
}

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

#[cfg(test)]
mod tests {
    use ratatui::layout::Rect;

    use super::*;

    #[test]
    fn compute_hex_page_bytes_reasonable() {
        let r = Rect::new(0, 0, 80, 25);
        let b = compute_hex_page_bytes(r);
        // 25 height minus top/bottom 3+6=9 => main≈16 lines, ×16 bytes ≈ 256, within range.
        assert!(b >= 16 && b < 1000);
    }

    #[test]
    fn empty_hex_handles_no_data_without_panic() {
        let area = Rect::new(0, 0, 80, 10);
        let buf = ratatui::backend::TestBackend::new(80, 10);
        let mut term = ratatui::Terminal::new(buf).unwrap();
        term.draw(|f| draw_hex_viewer(f, area, 0, &[])).unwrap();
        // No assertion on visuals; just ensure it draws safely.
    }

    #[test]
    fn computes_byte_count_small_area() {
        let r = Rect::new(0, 0, 20, 8);
        let b = compute_hex_page_bytes(r);
        assert!(b <= 200);
    }
}
