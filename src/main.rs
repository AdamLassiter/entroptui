use anyhow::{Context, Result};
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::layout::Rect;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{
    cmp::{max, min},
    fs::File,
    io::{self, Read, Seek, SeekFrom},
    path::PathBuf,
    time::{Duration, Instant},
};

mod analysis;
mod cache;
mod suggest;
mod ui;

use analysis::Analyzer;
use suggest::SuggestEngine;

use crate::{
    analysis::{
        bitplane::BitPlaneEntropyAnalyzer,
        entropy::{BucketSize, EntropyAnalyzer},
        spectral::SpectralFlatnessAnalyzer,
    },
    cache::{ChartCache, HilbertCache, MetricCache, SuggestCache},
    suggest::Features,
    ui::ViewMode,
};

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Input file path
    path: PathBuf,

    /// Initial window size in bytes
    #[arg(long, default_value_t = 1024 * 1024)]
    window: u64,

    /// Initial offset in bytes
    #[arg(long, default_value_t = 0)]
    offset: u64,
}

fn main() -> Result<()> {
    let args = Args::parse();

    enable_raw_mode().context("enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).context("enter alt screen")?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("create terminal")?;

    let res = run_app(&mut terminal, args);

    disable_raw_mode().ok();
    execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();
    terminal.show_cursor().ok();

    res
}

struct FileWindow {
    file: File,
    len: u64,
}

impl FileWindow {
    fn open(path: &PathBuf) -> Result<Self> {
        let file = File::open(path).with_context(|| format!("open {}", path.display()))?;
        let len = file
            .metadata()
            .with_context(|| format!("metadata {}", path.display()))?
            .len();
        Ok(Self { file, len })
    }

    fn read_window(&mut self, offset: u64, window_len: u64) -> Result<Vec<u8>> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        let off = min(offset, self.len.saturating_sub(1));
        self.file.seek(SeekFrom::Start(off)).context("seek")?;

        let max_read = min(window_len, self.len.saturating_sub(off));
        let mut buf = vec![0u8; max_read as usize];
        let mut total = 0usize;
        while total < buf.len() {
            let n = self.file.read(&mut buf[total..]).context("read")?;
            if n == 0 {
                break;
            }
            total += n;
        }
        buf.truncate(total);
        Ok(buf)
    }
}

struct App {
    path: PathBuf,
    file_len: u64,

    offset: u64,
    window_len: u64,
    view: ViewMode,
    hex_page_bytes: u64,

    window_data: Vec<u8>,

    analyzers: Vec<Box<dyn Analyzer>>,
    analyzer_idx: usize,

    suggest_engine: SuggestEngine,
    suggest_cache: Option<SuggestCache>,

    metric: Option<MetricCache>,
    chart: Option<ChartCache>,
    hilbert: Option<HilbertCache>,

    status: String,
    dirty: bool,
}

impl std::fmt::Debug for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("App")
            .field("path", &self.path)
            .field("file_len", &self.file_len)
            .field("offset", &self.offset)
            .field("window_len", &self.window_len)
            .field("window_data", &self.window_data)
            .field("plot", &self.metric)
            .field("status", &self.status)
            .field("dirty", &self.dirty)
            .finish()
    }
}

impl App {
    fn new(path: PathBuf, file_len: u64, offset: u64, window_len: u64) -> Self {
        let analyzers: Vec<Box<dyn Analyzer>> = vec![
            Box::new(EntropyAnalyzer {
                bucket: BucketSize::B1,
            }),
            Box::new(EntropyAnalyzer {
                bucket: BucketSize::B2,
            }),
            Box::new(EntropyAnalyzer {
                bucket: BucketSize::B4,
            }),
            Box::new(SpectralFlatnessAnalyzer::default()),
            Box::new(BitPlaneEntropyAnalyzer),
        ];
        Self {
            path,
            file_len,
            offset: min(offset, file_len),
            window_len: max(4 * 1024, window_len),
            view: ViewMode::Chart,
            hex_page_bytes: 0,
            suggest_engine: SuggestEngine::default(),
            suggest_cache: None,
            analyzers,
            analyzer_idx: 0,
            window_data: Vec::new(),
            metric: None,
            chart: None,
            hilbert: None,
            status: String::new(),
            dirty: true,
        }
    }

    fn clamp_offset(&mut self) {
        if self.file_len == 0 {
            self.offset = 0;
            return;
        }
        self.offset = min(self.offset, self.file_len.saturating_sub(1));
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
        self.suggest_cache = None;
        self.metric = None;
    }

    fn next_analyzer(&mut self) {
        if self.analyzers.is_empty() {
            return;
        }
        self.analyzer_idx = (self.analyzer_idx + 1) % self.analyzers.len();
        self.metric = None;
        self.hilbert = None;
    }

    fn next_view(&mut self) {
        self.view = self.view.next();
        self.suggest_cache = None;
    }

    fn zoom_in(&mut self) {
        self.window_len = max(4 * 1024, self.window_len / 2);
        self.mark_dirty();
    }

    fn zoom_out(&mut self) {
        let max_len = max(4 * 1024, self.file_len);
        self.window_len = min(max_len, self.window_len.saturating_mul(2));
        self.mark_dirty();
    }

    fn scroll_by(&mut self, delta: i64) {
        let new_off = if delta.is_negative() {
            self.offset.saturating_sub(delta.unsigned_abs())
        } else {
            self.offset.saturating_add(delta as u64)
        };
        self.offset = new_off;
        self.clamp_offset();
        self.mark_dirty();
    }

    fn jump_start(&mut self) {
        self.offset = 0;
        self.mark_dirty();
    }

    fn jump_end(&mut self) {
        if self.file_len == 0 {
            self.offset = 0;
        } else {
            self.offset = self.file_len.saturating_sub(1);
        }
        self.mark_dirty();
    }

    fn refresh_window(&mut self, fw: &mut FileWindow) -> Result<()> {
        self.clamp_offset();
        self.window_data = fw.read_window(self.offset, self.window_len)?;
        Ok(())
    }

    fn ensure_size(&mut self, size: Rect) {
        let bins = self.ensure_views(size);
        self.ensure_suggestions();
        self.ensure_metric(bins);
    }
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, args: Args) -> Result<()> {
    let mut fw = FileWindow::open(&args.path)?;
    let mut app = App::new(args.path.clone(), fw.len, args.offset, args.window);

    app.refresh_window(&mut fw)?;
    app.dirty = false;

    let tick_rate = Duration::from_millis(33);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| {
            app.ensure_size(f.area());
            let analyzer = &app.analyzers[app.analyzer_idx];

            ui::draw(
                f,
                &app.path,
                app.file_len,
                app.offset,
                app.window_len,
                app.view,
                analyzer.name(),
                &app.window_data,
                app.metric.as_ref(),
                app.chart.as_ref(),
                app.hilbert.as_ref(),
                app.suggest_cache
                    .as_ref()
                    .map(|c| &c.features)
                    .unwrap_or(&Features::default()),
                app.suggest_cache
                    .as_ref()
                    .map(|c| c.suggestions.as_slice())
                    .unwrap_or(&[]),
                &app.status,
            );
        })?;

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)?
            && let Event::Key(k) = event::read()?
        {
            if k.kind != KeyEventKind::Press {
                continue;
            }

            match (k.code, k.modifiers) {
                (KeyCode::Char('q'), _)
                | (KeyCode::Esc, _)
                | (KeyCode::Char('c'), KeyModifiers::CONTROL) => break,

                (KeyCode::Char('v'), _) => {
                    app.next_view();
                }

                (KeyCode::Char('c'), _) => {
                    app.next_analyzer();
                }

                (KeyCode::Char('+'), _) | (KeyCode::Char('='), _) => {
                    app.zoom_in();
                }
                (KeyCode::Char('-'), _) => {
                    app.zoom_out();
                }

                (KeyCode::Left, _) => {
                    let step = max(1, app.hex_page_bytes) as i64;
                    app.scroll_by(-step);
                }
                (KeyCode::Right, _) => {
                    let step = max(1, app.hex_page_bytes) as i64;
                    app.scroll_by(step);
                }

                (KeyCode::PageUp, _) => {
                    let step = max(1, app.window_len) as i64;
                    app.scroll_by(-step);
                }
                (KeyCode::PageDown, _) => {
                    let step = max(1, app.window_len) as i64;
                    app.scroll_by(step);
                }

                (KeyCode::Home, _) => app.jump_start(),
                (KeyCode::End, _) => app.jump_end(),
                _ => {}
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }

        if app.dirty {
            if let Err(e) = app.refresh_window(&mut fw) {
                app.status = format!("read error: {e:#}");
            } else {
                app.status.clear();
            }
            app.dirty = false;
        }
    }

    Ok(())
}
