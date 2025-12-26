use std::cmp::max;

use crate::{App, HilbertCursor, cache::HilbertCache, ui::ViewMode};

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use rayon::prelude::*;

impl App {
    pub fn ensure_hilbert(&mut self, side: u16) {
        let needs = match &self.hilbert {
            None => true,
            Some(h) => {
                h.side != side
                    || h.offset != self.offset
                    || h.window_len != self.window_len
                    || h.analyzer_idx != self.analyzer_idx
            }
        };
        if !needs {
            return;
        }

        let side_usize = max(2, side as usize);
        let cells = side_usize * side_usize;

        if self.window_data.is_empty() {
            self.hilbert = Some(HilbertCache {
                side,
                offset: self.offset,
                window_len: self.window_len,
                analyzer_idx: self.analyzer_idx,
                xy_to_d: vec![0; cells],
                values_row_major: vec![0.0; cells],
            });
            return;
        }

        // Compute entropy values per Hilbert-distance cell in parallel.
        // Then remap into row-major for rendering.
        let data = &self.window_data;
        let analyzer = &self.analyzers[self.analyzer_idx];
        let chunk = (data.len() / cells).max(1);

        let mut values_d = vec![0.0f64; cells];
        values_d.par_iter_mut().enumerate().for_each(|(d, slot)| {
            let start = d * chunk;
            if start >= data.len() {
                *slot = 0.0;
                return;
            }
            let end = if d == cells - 1 {
                data.len()
            } else {
                (start + chunk).min(data.len())
            };

            // Prefer sparse counting for Hilbert to avoid huge per-cell allocations.
            *slot = analyzer.value_norm_sparse(&data[start..end]);
        });

        let mut values_row_major = vec![0.0f64; cells];
        let mut xy_to_d = vec![0u32; cells];
        for (d, &val) in values_d.iter().enumerate() {
            let (x, y) = d2xy(side, d as u32);
            let x = x as usize;
            let y = y as usize;
            if x < side_usize && y < side_usize {
                values_row_major[y * side_usize + x] = val;
                xy_to_d[y * side_usize + x] = d as u32;
            }
        }

        self.hilbert = Some(HilbertCache {
            side,
            offset: self.offset,
            window_len: self.window_len,
            analyzer_idx: self.analyzer_idx,
            xy_to_d,
            values_row_major,
        });

        // Clamp cursor to new side if needed.
        if let Some(mut c) = self.hilbert_cursor {
            c.x = c.x.min(side.saturating_sub(1));
            c.y = c.y.min(side.saturating_sub(1));
            self.hilbert_cursor = Some(c);
        }
    }

    pub fn move_hilbert_cursor(&mut self, dx: i16, dy: i16) {
        if self.view != ViewMode::Hilbert {
            return;
        }
        let Some(h) = self.hilbert.as_ref() else {
            return;
        };
        let side = h.side;
        let Some(mut c) = self.hilbert_cursor else {
            self.hilbert_cursor = Some(HilbertCursor { x: 0, y: 0 });
            return;
        };

        let nx = (c.x as i32 + dx as i32).clamp(0, side.saturating_sub(1) as i32);
        let ny = (c.y as i32 + dy as i32).clamp(0, side.saturating_sub(1) as i32);
        c.x = nx as u16;
        c.y = ny as u16;
        self.hilbert_cursor = Some(c);
    }

    pub fn hilbert_cursor_range(&self) -> Option<(u64, u64, f64)> {
        // Returns (abs_start, abs_end_exclusive, value_norm)
        let h = self.hilbert.as_ref()?;
        let c = self.hilbert_cursor?;

        let side = h.side as usize;
        if side == 0 {
            return None;
        }

        let x = c.x as usize;
        let y = c.y as usize;
        let idx = y * side + x;
        if idx >= h.xy_to_d.len() {
            return None;
        }
        let d = h.xy_to_d[idx] as usize;
        let cells = side * side;
        let chunk = (self.window_data.len() / cells).max(1);
        let start = d * chunk;
        if start >= self.window_data.len() {
            return None;
        }
        let end = if d == cells - 1 {
            self.window_data.len()
        } else {
            ((d + 1) * chunk).min(self.window_data.len())
        };

        let abs_start = self.offset + start as u64;
        let abs_end = self.offset + end as u64;
        let value = h.values_row_major.get(idx).copied().unwrap_or(0.0);
        Some((abs_start, abs_end, value))
    }
}

pub fn draw_hilbert(
    f: &mut Frame,
    area: Rect,
    hilbert: Option<&HilbertCache>,
    cursor: Option<HilbertCursor>,
) {
    let block = Block::default()
        .title("Hilbert entropy map (heatmap: low blue → red high)")
        .borders(Borders::ALL);

    let Some(h) = hilbert else {
        f.render_widget(block, area);
        return;
    };

    let side = h.side as usize;
    if side < 2 {
        f.render_widget(block, area);
        return;
    }

    // Build a side x side grid of colored cells using truecolor RGB heatmap.
    // Use truecolor RGB: low entropy -> blue, high entropy -> red.
    let mut lines: Vec<Line> = Vec::with_capacity(side);
    for y in 0..side {
        let mut spans: Vec<Span> = Vec::with_capacity(side);
        for x in 0..side {
            let v = h.values_row_major[y * side + x].clamp(0.0, 1.0);
            let (r, g, b) = heatmap_rgb(v);
            let is_cursor = cursor.is_some_and(|c| c.x as usize == x && c.y as usize == y);
            let style = if is_cursor {
                // Visible highlight independent of underlying heatmap.
                Style::default().fg(Color::Black).bg(Color::White)
            } else {
                Style::default().fg(Color::Rgb(r, g, b))
            };

            spans.push(Span::styled("██", style));
        }
        lines.push(Line::from(spans));
    }

    let p = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    f.render_widget(p, area);
}

fn lerp_u8(a: u8, b: u8, t: f64) -> u8 {
    let t = t.clamp(0.0, 1.0);
    let af = a as f64;
    let bf = b as f64;
    (af + (bf - af) * t).round().clamp(0.0, 255.0) as u8
}

/// Heatmap ramp:
/// 0.00 blue -> 0.25 cyan -> 0.50 green -> 0.75 yellow -> 1.00 red
fn heatmap_rgb(v: f64) -> (u8, u8, u8) {
    let v = v.clamp(0.0, 1.0);
    let stops: [(u8, u8, u8); 5] = [
        (0, 0, 255),   // blue
        (0, 255, 255), // cyan
        (0, 255, 0),   // green
        (255, 255, 0), // yellow
        (255, 0, 0),   // red
    ];

    let t = v * 4.0; // 0..4
    let i = t.floor() as usize;
    if i >= 4 {
        return stops[4];
    }
    let frac = t - i as f64;
    let (r0, g0, b0) = stops[i];
    let (r1, g1, b1) = stops[i + 1];
    (
        lerp_u8(r0, r1, frac),
        lerp_u8(g0, g1, frac),
        lerp_u8(b0, b1, frac),
    )
}

/// Pick the largest power-of-two side length <= max_side.
pub fn best_pow2_side(max_side: u16) -> u16 {
    if max_side < 2 {
        return 0;
    }
    let mut s: u16 = 1;
    while s.saturating_mul(2) <= max_side {
        s = s.saturating_mul(2);
    }
    if s < 2 { 0 } else { s }
}

/// Map distance `d` along a Hilbert curve to (x,y) in an `side x side` grid.
/// `side` must be a power of two.
///
/// Based on the classic iterative algorithm (Wikipedia).
pub fn d2xy(side: u16, d: u32) -> (u16, u16) {
    let n = side as u32;
    let mut x: u32 = 0;
    let mut y: u32 = 0;
    let mut t = d;
    let mut s: u32 = 1;

    while s < n {
        let rx = (t / 2) & 1;
        let ry = (t ^ rx) & 1;
        let (nx, ny) = rot(s, x, y, rx, ry);
        x = nx + s * rx;
        y = ny + s * ry;
        t /= 4;
        s *= 2;
    }

    (x as u16, y as u16)
}

fn rot(s: u32, mut x: u32, mut y: u32, rx: u32, ry: u32) -> (u32, u32) {
    if ry == 0 {
        if rx == 1 {
            x = s - 1 - x;
            y = s - 1 - y;
        }
        // Swap x and y
        (y, x)
    } else {
        (x, y)
    }
}
