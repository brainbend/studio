//! A basic terminal for the AI tab, so a tool such as Claude Code or Codex can be used from the side panel.
//!
//! The author runs the tool themselves, in a terminal, exactly as they would in any other terminal window: this is not a chat
//! that drives the tool for them. That matters for a subscription, which belongs to the author's own use of the company's own
//! tool. The tool is started already connected to this app (see [`App::start_terminal`]), so there is nothing to set up.
//!
//! `portable-pty` runs the program, `alacritty_terminal` understands what it prints, and this file draws the result with egui
//! and turns keys into what a terminal sends. It is deliberately plain: colours, scrolling, selecting and copying, pasting, and
//! the usual keys. It does not do mouse reporting, images, or tabs.
use super::*;
use alacritty_terminal::event::{Event as TermEvent, EventListener, WindowSize};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Line, Point, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::{Config, Term, TermMode};
use alacritty_terminal::vte::ansi::{Color, Processor, Rgb};
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::sync::mpsc::{channel, Receiver, Sender};

const FONT_SIZE: f32 = 12.5;
/// The colours of a terminal in the app's dark or light mode.
#[derive(Clone, Copy)]
pub struct Palette {
    pub bg: egui::Color32,
    pub fg: egui::Color32,
    pub selection_bg: egui::Color32,
    pub selection_fg: egui::Color32,
    ansi: [(u8, u8, u8); 16],
}

/// The ANSI colours as a terminal usually shows them: on a dark background, and on a light one (where yellow and white
/// have to be darker to be read).
const ANSI_DARK: [(u8, u8, u8); 16] = [
    (0, 0, 0), (205, 49, 49), (13, 188, 121), (229, 229, 16), (36, 114, 200), (188, 63, 188), (17, 168, 205), (229, 229, 229),
    (102, 102, 102), (241, 76, 76), (35, 209, 139), (245, 245, 67), (59, 142, 234), (214, 112, 214), (41, 184, 219), (255, 255, 255),
];
const ANSI_LIGHT: [(u8, u8, u8); 16] = [
    (0, 0, 0), (205, 49, 49), (0, 140, 0), (148, 152, 0), (4, 81, 165), (188, 5, 188), (5, 152, 188), (85, 85, 85),
    (102, 102, 102), (205, 49, 49), (20, 160, 20), (181, 186, 0), (4, 81, 165), (188, 5, 188), (5, 152, 188), (60, 60, 60),
];

impl Palette {
    pub fn new(dark: bool) -> Palette {
        if dark {
            Palette { bg: egui::Color32::from_rgb(24, 24, 24), fg: egui::Color32::from_rgb(204, 204, 204), selection_bg: egui::Color32::from_rgb(38, 79, 120), selection_fg: egui::Color32::WHITE, ansi: ANSI_DARK }
        } else {
            Palette { bg: egui::Color32::from_rgb(252, 252, 252), fg: egui::Color32::from_rgb(36, 36, 36), selection_bg: egui::Color32::from_rgb(173, 214, 255), selection_fg: egui::Color32::BLACK, ansi: ANSI_LIGHT }
        }
    }

    fn ansi(&self, i: usize) -> egui::Color32 { let (r, g, b) = self.ansi[i.min(15)]; egui::Color32::from_rgb(r, g, b) }
}

/// The tools that can be started in the tab: a name, the program, and whether this app connects it by itself.
pub struct Tool {
    pub name: &'static str,
    /// The program to run; empty for the author's own shell.
    pub program: &'static str,
    pub note: &'static str,
}

pub const TOOLS: [Tool; 4] = [
    Tool { name: "Claude Code", program: "claude", note: "Uses your Claude subscription or Anthropic account" },
    Tool { name: "Codex", program: "codex", note: "Uses your ChatGPT subscription or OpenAI account" },
    Tool { name: "Gemini CLI", program: "gemini", note: "Uses your Google account" },
    Tool { name: "Shell", program: "", note: "A plain terminal" },
];

#[derive(Clone)]
struct Listener(Sender<TermEvent>);

impl EventListener for Listener {
    fn send_event(&self, event: TermEvent) { let _ = self.0.send(event); }
}

struct Size { columns: usize, lines: usize }

impl Dimensions for Size {
    fn total_lines(&self) -> usize { self.lines }
    fn screen_lines(&self) -> usize { self.lines }
    fn columns(&self) -> usize { self.columns }
}

pub struct Terminal {
    /// What it is running, for the toolbar.
    pub name: String,
    pub title: String,
    /// Why it stopped, once it has.
    pub exited: Option<String>,
    term: Term<Listener>,
    parser: Processor,
    bytes: Receiver<Vec<u8>>,
    events: Receiver<TermEvent>,
    writer: Box<dyn Write + Send>,
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    size: (usize, usize),
    cell: (f32, f32),
    /// The app's mode now, and when this was started (the program chose its own colours then).
    dark: bool,
    pub spawned_dark: bool,
    want_focus: bool,
}

impl Drop for Terminal {
    fn drop(&mut self) { let _ = self.child.kill(); }
}

/// Single-quotes a word for a POSIX shell.
fn quote(word: &str) -> String { format!("'{}'", word.replace('\'', r"'\''")) }

impl Terminal {
    /// Runs `program` with `args` in a terminal. The program is found the way the author's own shell would find it (their
    /// login shell reads their profile, so the PATH set up for the command line is used even though this app was opened from
    /// the desktop). With no program, it is the shell itself.
    pub fn spawn(name: &str, program: &str, args: &[String], cwd: Option<std::path::PathBuf>, env: &[(String, String)], dark: bool, ctx: &egui::Context) -> Result<Terminal, String> {
        let mut cmd = if cfg!(windows) {
            let mut c = CommandBuilder::new("cmd");
            if !program.is_empty() { c.arg("/C"); c.arg(program); c.args(args); }
            c
        } else {
            let shell = std::env::var("SHELL").ok().filter(|s| !s.is_empty()).unwrap_or_else(|| "/bin/zsh".into());
            let mut c = CommandBuilder::new(&shell);
            c.arg("-l");
            if !program.is_empty() {
                let words = std::iter::once(program).chain(args.iter().map(|a| a.as_str())).map(quote).collect::<Vec<_>>().join(" ");
                c.arg("-i");
                c.arg("-c");
                c.arg(format!("command -v {p} >/dev/null 2>&1 || {{ echo \"{name} was not found. Install it, then choose Restart.\"; exit 127; }}; exec {words}", p = quote(program)));
            } else {
                c.arg("-i");
            }
            c
        };
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        // The old, widely read way for a program to learn whether the background is light (foreground;background).
        cmd.env("COLORFGBG", if dark { "15;0" } else { "0;15" });
        for (k, v) in env { cmd.env(k, v); }
        if let Some(dir) = cwd { cmd.cwd(dir); }
        let (cols, rows) = (80usize, 24usize);
        let pair = native_pty_system().openpty(PtySize { rows: rows as u16, cols: cols as u16, pixel_width: 0, pixel_height: 0 }).map_err(|e| e.to_string())?;
        let child = pair.slave.spawn_command(cmd).map_err(|e| format!("Could not start {name}: {e}"))?;
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
        let writer = pair.master.take_writer().map_err(|e| e.to_string())?;
        let (tx, bytes) = channel::<Vec<u8>>();
        let wake = ctx.clone();
        std::thread::spawn(move || {
            let mut buf = [0u8; 16 * 1024];
            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 || tx.send(buf[..n].to_vec()).is_err() { break; }
                wake.request_repaint();
            }
            wake.request_repaint();
        });
        let (etx, events) = channel();
        let term = Term::new(Config::default(), &Size { columns: cols, lines: rows }, Listener(etx));
        Ok(Terminal { name: name.to_string(), title: String::new(), exited: None, term, parser: Processor::new(), bytes, events, writer, master: pair.master, child, size: (cols, rows), cell: (8.0, 16.0), dark, spawned_dark: dark, want_focus: true })
    }

    /// Takes in what the program has printed and answers what the terminal itself is asked.
    pub fn pump(&mut self) {
        while let Ok(chunk) = self.bytes.try_recv() { self.parser.advance(&mut self.term, &chunk); }
        while let Ok(event) = self.events.try_recv() {
            match event {
                TermEvent::PtyWrite(text) => self.send(text.as_bytes()),
                TermEvent::Title(t) => self.title = t,
                TermEvent::ResetTitle => self.title.clear(),
                // A program may ask what colour the terminal is, to choose its own; it gets the answer a dark one gives.
                TermEvent::ColorRequest(index, format) => {
                    let pal = Palette::new(self.dark);
                    let c = match index { 257 => pal.bg, _ => pal.fg };
                    self.send(format(Rgb { r: c.r(), g: c.g(), b: c.b() }).as_bytes());
                }
                TermEvent::TextAreaSizeRequest(format) => {
                    let (w, h) = self.cell;
                    let size = WindowSize { num_lines: self.size.1 as u16, num_cols: self.size.0 as u16, cell_width: w as u16, cell_height: h as u16 };
                    self.send(format(size).as_bytes());
                }
                _ => {}
            }
        }
        if self.exited.is_none() {
            if let Ok(Some(status)) = self.child.try_wait() { self.exited = Some(format!("{status}")); }
        }
    }

    pub fn send(&mut self, bytes: &[u8]) {
        if self.exited.is_some() { return; }
        let _ = self.writer.write_all(bytes);
        let _ = self.writer.flush();
    }

    /// What is on the screen, as text, line by line (for tests).
    #[cfg(test)]
    pub fn screen_text(&self) -> String {
        let grid = self.term.grid();
        (0..grid.screen_lines()).map(|l| (0..grid.columns()).map(|c| grid[Line(l as i32)][Column(c)].c).collect::<String>().trim_end().to_string()).collect::<Vec<_>>().join("\n")
    }

    pub fn resize(&mut self, cols: usize, rows: usize) {
        let (cols, rows) = (cols.max(8), rows.max(2));
        if (cols, rows) == self.size { return; }
        self.size = (cols, rows);
        self.term.resize(Size { columns: cols, lines: rows });
        let _ = self.master.resize(PtySize { rows: rows as u16, cols: cols as u16, pixel_width: 0, pixel_height: 0 });
    }

    /// Pasted text, in the form the program asked for.
    pub fn paste(&mut self, text: &str) {
        self.term.scroll_display(Scroll::Bottom);
        if self.term.mode().contains(TermMode::BRACKETED_PASTE) {
            self.send(format!("\x1b[200~{}\x1b[201~", text.replace('\x1b', "")).as_bytes());
        } else {
            self.send(text.replace("\r\n", "\r").replace('\n', "\r").as_bytes());
        }
    }

    /// Draws the terminal in `ui`, filling it, and takes the keyboard while it has focus.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        self.dark = ui.visuals().dark_mode;
        let pal = Palette::new(self.dark);
        self.pump();
        let font = egui::FontId::monospace(FONT_SIZE);
        let (cw, ch) = ui.fonts_mut(|f| { let g = f.layout_no_wrap("M".repeat(16), font.clone(), egui::Color32::WHITE); (g.size().x / 16.0, g.size().y) });
        self.cell = (cw, ch);
        let avail = ui.available_size();
        let cols = ((avail.x - 4.0) / cw).floor().max(8.0) as usize;
        let rows = (avail.y / ch).floor().max(2.0) as usize;
        let (rect, response) = ui.allocate_exact_size(egui::vec2(avail.x, rows as f32 * ch), egui::Sense::click_and_drag());
        self.resize(cols, rows);
        if std::mem::take(&mut self.want_focus) { response.request_focus(); }
        if response.clicked() || response.drag_started() { response.request_focus(); }
        let focused = response.has_focus();
        let origin = rect.min + egui::vec2(2.0, 0.0);
        let offset = self.term.grid().display_offset();

        // Selecting with the mouse.
        let point_at = |pos: egui::Pos2| -> (Point, Side) {
            let x = (pos.x - origin.x).max(0.0);
            let col = ((x / cw) as usize).min(cols - 1);
            let row = (((pos.y - origin.y) / ch).max(0.0) as usize).min(rows - 1);
            (Point::new(Line(row as i32 - offset as i32), Column(col)), if x - col as f32 * cw < cw / 2.0 { Side::Left } else { Side::Right })
        };
        if let Some(pos) = response.interact_pointer_pos() {
            if response.drag_started() { let (p, s) = point_at(pos); self.term.selection = Some(Selection::new(SelectionType::Simple, p, s)); }
            else if response.dragged() { let (p, s) = point_at(pos); if let Some(sel) = self.term.selection.as_mut() { sel.update(p, s); } }
            else if response.clicked() { self.term.selection = None; }
        }
        if response.hovered() {
            let dy = ui.input(|i| i.smooth_scroll_delta.y);
            let lines = (dy / ch).round() as i32;
            if lines != 0 { self.term.scroll_display(Scroll::Delta(lines)); }
        }

        // Drawing.
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, pal.bg);
        let content = self.term.renderable_content();
        let selection = content.selection;
        let mut grid = vec![vec![(' ', pal.fg, pal.bg, Flags::empty()); cols]; rows];
        for cell in content.display_iter {
            let row = cell.point.line.0 + content.display_offset as i32;
            let col = cell.point.column.0;
            if row < 0 || row as usize >= rows || col >= cols { continue; }
            let (mut fg, mut bg) = (resolve(cell.fg, &pal), resolve(cell.bg, &pal));
            if cell.flags.contains(Flags::INVERSE) { std::mem::swap(&mut fg, &mut bg); }
            if selection.is_some_and(|s| s.contains(cell.point)) { bg = pal.selection_bg; fg = pal.selection_fg; }
            if cell.flags.contains(Flags::DIM) { fg = fg.gamma_multiply(0.6); }
            let c = if cell.flags.contains(Flags::HIDDEN) || cell.flags.contains(Flags::WIDE_CHAR_SPACER) || cell.flags.contains(Flags::LEADING_WIDE_CHAR_SPACER) { ' ' } else { cell.c };
            grid[row as usize][col] = (c, fg, bg, cell.flags);
        }
        for (r, line) in grid.iter().enumerate() {
            let y = origin.y + r as f32 * ch;
            // Backgrounds first, in runs.
            let mut c = 0;
            while c < cols {
                let bg = line[c].2;
                let mut end = c + 1;
                while end < cols && line[end].2 == bg { end += 1; }
                if bg != pal.bg { painter.rect_filled(egui::Rect::from_min_size(egui::pos2(origin.x + c as f32 * cw, y), egui::vec2((end - c) as f32 * cw, ch)), 0.0, bg); }
                c = end;
            }
            // Then text, in runs of one colour; a wide character is drawn alone so the columns stay in step.
            let mut c = 0;
            while c < cols {
                let (ch0, fg, _, flags) = line[c];
                if ch0 == ' ' && !flags.contains(Flags::UNDERLINE) { c += 1; continue; }
                let wide = flags.contains(Flags::WIDE_CHAR);
                let mut end = c + 1;
                let mut text = String::from(ch0);
                if !wide {
                    while end < cols && line[end].1 == fg && !line[end].3.contains(Flags::WIDE_CHAR) && line[end].3.contains(Flags::UNDERLINE) == flags.contains(Flags::UNDERLINE) { text.push(line[end].0); end += 1; }
                } else { end = c + 2; }
                let x = origin.x + c as f32 * cw;
                painter.text(egui::pos2(x, y), egui::Align2::LEFT_TOP, text.trim_end(), font.clone(), fg);
                if flags.contains(Flags::UNDERLINE) { painter.line_segment([egui::pos2(x, y + ch - 1.5), egui::pos2(x + (end - c) as f32 * cw, y + ch - 1.5)], egui::Stroke::new(1.0, fg)); }
                if flags.contains(Flags::STRIKEOUT) { painter.line_segment([egui::pos2(x, y + ch / 2.0), egui::pos2(x + (end - c) as f32 * cw, y + ch / 2.0)], egui::Stroke::new(1.0, fg)); }
                c = end;
            }
        }
        if content.mode.contains(TermMode::SHOW_CURSOR) && self.exited.is_none() {
            let row = content.cursor.point.line.0 + content.display_offset as i32;
            let col = content.cursor.point.column.0;
            if row >= 0 && (row as usize) < rows && col < cols {
                let r = egui::Rect::from_min_size(egui::pos2(origin.x + col as f32 * cw, origin.y + row as f32 * ch), egui::vec2(cw, ch));
                if focused { painter.rect_filled(r, 0.0, pal.fg.gamma_multiply(0.7)); } else { painter.rect_stroke(r, 0.0, egui::Stroke::new(1.0, pal.fg), egui::StrokeKind::Inside); }
            }
        }
        if let Some(why) = &self.exited {
            let r = egui::Rect::from_min_size(egui::pos2(rect.left(), rect.bottom() - ch * 1.6), egui::vec2(rect.width(), ch * 1.6));
            painter.rect_filled(r, 0.0, pal.bg.gamma_multiply(0.85));
            painter.text(r.center(), egui::Align2::CENTER_CENTER, format!("{} has finished ({why})", self.name), font.clone(), pal.fg);
        }

        // The keyboard, while it has focus. Tab, Escape and the arrows belong to the program, not to egui's focus moving.
        if focused {
            ui.memory_mut(|m| m.set_focus_lock_filter(response.id, egui::EventFilter { tab: true, horizontal_arrows: true, vertical_arrows: true, escape: true }));
            let events = ui.input(|i| i.events.clone());
            let app_cursor = self.term.mode().contains(TermMode::APP_CURSOR);
            for event in events {
                match event {
                    egui::Event::Text(t) => { self.term.scroll_display(Scroll::Bottom); self.send(t.as_bytes()); }
                    egui::Event::Key { key, pressed: true, modifiers, .. } => {
                        if let Some(bytes) = key_bytes(key, modifiers, app_cursor) { self.term.scroll_display(Scroll::Bottom); self.send(&bytes); }
                    }
                    egui::Event::Paste(t) => self.paste(&t),
                    // Copy is Ctrl+C on Windows and Linux, where egui turns the key into this event; with nothing selected
                    // it is what the program expects from Ctrl+C.
                    egui::Event::Copy | egui::Event::Cut => match self.term.selection_to_string().filter(|s| !s.is_empty()) {
                        Some(text) => { ui.ctx().copy_text(text); self.term.selection = None; }
                        None => self.send(&[3]),
                    },
                    _ => {}
                }
            }
            ui.input_mut(|i| i.events.retain(|e| !matches!(e, egui::Event::Text(_) | egui::Event::Key { .. } | egui::Event::Paste(_) | egui::Event::Copy | egui::Event::Cut)));
        }
    }
}

/// What a terminal sends for a key that is not text, or `None` when it sends nothing (the text events carry the letters).
pub fn key_bytes(key: egui::Key, m: egui::Modifiers, app_cursor: bool) -> Option<Vec<u8>> {
    use egui::Key::*;
    // Cmd on a Mac is for the app, not the program (Cmd+C and Cmd+V arrive as copy and paste events).
    if m.mac_cmd { return None; }
    let alt = if m.alt { "\x1b" } else { "" };
    let modifier = 1 + m.shift as u8 + 2 * m.alt as u8 + 4 * m.ctrl as u8;
    let arrow = |c: char| -> Vec<u8> {
        if modifier > 1 { format!("\x1b[1;{modifier}{c}").into_bytes() } else if app_cursor { format!("\x1bO{c}").into_bytes() } else { format!("\x1b[{c}").into_bytes() }
    };
    let tilde = |n: u8| -> Vec<u8> { if modifier > 1 { format!("\x1b[{n};{modifier}~").into_bytes() } else { format!("\x1b[{n}~").into_bytes() } };
    Some(match key {
        Enter => if m.shift || m.alt { b"\x1b\r".to_vec() } else { b"\r".to_vec() },
        Backspace => format!("{alt}\x7f").into_bytes(),
        Tab => if m.shift { b"\x1b[Z".to_vec() } else { b"\t".to_vec() },
        Escape => b"\x1b".to_vec(),
        ArrowUp => arrow('A'),
        ArrowDown => arrow('B'),
        ArrowRight => arrow('C'),
        ArrowLeft => arrow('D'),
        Home => if modifier > 1 { format!("\x1b[1;{modifier}H").into_bytes() } else { b"\x1b[H".to_vec() },
        End => if modifier > 1 { format!("\x1b[1;{modifier}F").into_bytes() } else { b"\x1b[F".to_vec() },
        Insert => tilde(2),
        Delete => tilde(3),
        PageUp => tilde(5),
        PageDown => tilde(6),
        // Ctrl with a letter is the control character (Ctrl+C is 3). The letter itself arrives as text only without Ctrl.
        k if m.ctrl && !m.alt => {
            let name = k.name();
            let c = name.chars().next().filter(|c| name.len() == 1 && c.is_ascii_alphabetic())?;
            vec![c.to_ascii_uppercase() as u8 - b'A' + 1]
        }
        k if m.alt && !m.ctrl => {
            // Alt+letter is Escape then the letter (what shells call Meta).
            let name = k.name();
            let c = name.chars().next().filter(|_| name.len() == 1)?;
            let c = if m.shift { c.to_ascii_uppercase() } else { c.to_ascii_lowercase() };
            format!("\x1b{c}").into_bytes()
        }
        _ => return None,
    })
}

/// A cell's colour: one of the 16, one of the 256, a true colour, or the terminal's own.
fn resolve(color: Color, pal: &Palette) -> egui::Color32 {
    match color {
        Color::Spec(Rgb { r, g, b }) => egui::Color32::from_rgb(r, g, b),
        Color::Indexed(i) => indexed(i as usize, pal),
        Color::Named(n) => match n as usize {
            i @ 0..=15 => pal.ansi(i),
            257 => pal.bg,
            259..=266 => pal.ansi(n as usize - 259).gamma_multiply(0.6),
            268 => pal.fg.gamma_multiply(0.6),
            _ => pal.fg,
        },
    }
}

/// The 256-colour palette: the 16, a 6x6x6 cube, and 24 greys.
fn indexed(i: usize, pal: &Palette) -> egui::Color32 {
    match i {
        0..=15 => pal.ansi(i),
        16..=231 => {
            let n = i - 16;
            let level = |v: usize| if v == 0 { 0 } else { (55 + 40 * v) as u8 };
            egui::Color32::from_rgb(level(n / 36), level(n / 6 % 6), level(n % 6))
        }
        _ => { let v = (8 + 10 * (i.min(255) - 232)) as u8; egui::Color32::from_rgb(v, v, v) }
    }
}

/// What a tool started in the AI tab is told about where it is and what it is for. Without it, "make a presentation" is
/// answered with a web page or a file, since nothing says it means slides in this app.
pub fn instructions() -> String {
    format!("You are running inside BrainBend Studio, an app for building interactive lessons, in its AI tab. The author watches the open lesson on screen beside you.\n\n\
Treat every request to make, design, write, change or improve something to teach, present or show as a request about the open lesson, whatever it is called: a presentation, slide deck, deck, slideshow, slides, pitch, \
lesson, course, module, training, tutorial, e-learning, workshop, class, curriculum, walkthrough, onboarding, quiz, test, assessment, flashcards, storyboard, or no name at all (\"make something about X\"). \
Build it in the open lesson with the brainbend tools (the MCP server named brainbend). Do not make a web page, HTML, a document, a PowerPoint or other file, an artifact, or anything in a browser: \
the result must be slides in BrainBend Studio. Only when the author clearly asks for something that is not the lesson, such as help with code or a question, do something else. \
If they ask for a file or an export, make the lesson first, then say that publishing is done from Studio's Publish button. \
If a brainbend tool fails, tell the author what it said instead of switching to something else.\n\n{}{}", assistant::SYSTEM, mcp_tools::INSTRUCTIONS)
}

impl App {
    /// Starts a tool in the AI tab, connected to this app: the controller is turned on if it is off, and the tool is told how
    /// to reach it, so there is nothing to set up. Replaces a terminal that is already open.
    pub(super) fn start_terminal(&mut self, tool: usize, ctx: &egui::Context) {
        let tool = &TOOLS[tool.min(TOOLS.len() - 1)];
        if !self.mcp.running() { self.mcp.start(ctx); }
        let Some(url) = self.mcp.url() else { self.assistant.terminal_error = Some(self.mcp.error.clone().unwrap_or_else(|| "Could not turn on the connection.".into())); return };
        let token = self.mcp.settings.token.clone();
        let mut args: Vec<String> = vec![];
        let mut env: Vec<(String, String)> = vec![];
        match tool.program {
            "claude" => {
                // The connection goes in a file only the author can read, so the token is not in the command line.
                let path = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(|h| std::path::PathBuf::from(h).join(".brainbend-studio").join("claude-mcp.json"));
                if let Some(path) = path {
                    let config = serde_json::json!({ "mcpServers": { "brainbend": { "type": "http", "url": url, "headers": { "Authorization": format!("Bearer {token}") } } } });
                    mcp::write_private(&path, &config.to_string());
                    // Only this connection is loaded: with the author's other connectors (a browser, documents) in reach, a request
                    // for a presentation was answered in a browser. The instructions say what the tab is for, and the artifact
                    // tool, which would make a web page, is off.
                    args = vec!["--mcp-config".into(), path.to_string_lossy().into_owned(), "--strict-mcp-config".into(), "--allowedTools".into(), "mcp__brainbend".into(),
                        "--disallowedTools".into(), "Artifact".into(), "--append-system-prompt".into(), instructions()];
                }
            }
            "codex" => {
                args = vec!["-c".into(), format!("mcp_servers.brainbend.url=\"{url}\""), "-c".into(), "mcp_servers.brainbend.bearer_token_env_var=\"BRAINBEND_TOKEN\"".into()];
                env.push(("BRAINBEND_TOKEN".into(), token));
            }
            "gemini" => {
                // Gemini CLI reads the servers it should use from a settings file in the folder it starts in, so the
                // connection is made there, and the author's own settings are left alone.
                let dir = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(|h| std::path::PathBuf::from(h).join(".brainbend-studio").join("ai-workspace").join(".gemini"));
                if let Some(dir) = dir {
                    let config = serde_json::json!({ "mcpServers": { "brainbend": { "httpUrl": url, "headers": { "Authorization": format!("Bearer {token}") }, "trust": true } } });
                    mcp::write_private(&dir.join("settings.json"), &config.to_string());
                }
            }
            _ => {}
        }
        // The tool works in a folder of its own, not the author's home, so that what it can read and change by itself is
        // only what it makes here; the lesson is changed through the connection, which has its own switches.
        let workspace = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(|h| std::path::PathBuf::from(h).join(".brainbend-studio").join("ai-workspace"));
        if let Some(dir) = &workspace {
            let _ = std::fs::create_dir_all(dir);
            // The tools that read a notes file from their folder (Claude Code, Codex and Gemini CLI each have their own name
            // for it) are told the same thing.
            for file in ["CLAUDE.md", "AGENTS.md", "GEMINI.md"] { let _ = std::fs::write(dir.join(file), instructions()); }
        }
        self.assistant.terminal = None;
        match Terminal::spawn(tool.name, tool.program, &args, workspace, &env, ctx.global_style().visuals.dark_mode, ctx) {
            Ok(t) => { self.assistant.terminal = Some(t); self.assistant.terminal_error = None; self.assistant.view_terminal = true; self.assistant.term_tool = TOOLS.iter().position(|t| t.name == tool.name).unwrap_or(0); }
            Err(e) => self.assistant.terminal_error = Some(e),
        }
    }

    /// The terminal and its toolbar, filling the AI tab.
    pub(super) fn terminal_ui(&mut self, ui: &mut egui::Ui) {
        let mut restart = false;
        let mut close = false;
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new(self.assistant.terminal.as_ref().map(|t| t.name.as_str()).unwrap_or("")).strong());
            if ui.small_button("Restart").on_hover_text("Start it again").clicked() { restart = true; }
            if ui.small_button("Close").on_hover_text("Stop it and close the terminal").clicked() { close = true; }
            if ui.small_button("Settings").clicked() { self.assistant.setup_open = true; }
        });
        if let Some(e) = &self.assistant.terminal_error { ui.colored_label(egui::Color32::from_rgb(220, 70, 70), e); }
        // The tool chooses its own colours when it starts, so a change of mode after that reaches only the plain text.
        if self.assistant.terminal.as_ref().is_some_and(|t| t.spawned_dark != ui.visuals().dark_mode) { ui.weak("The theme changed. Restart the tool so its colors match."); }
        if let Some(t) = self.assistant.terminal.as_mut() { t.ui(ui); }
        if restart { let ctx = ui.ctx().clone(); let tool = self.assistant.term_tool; self.start_terminal(tool, &ctx); }
        if close { self.assistant.terminal = None; self.assistant.view_terminal = false; }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(k: egui::Key, mods: egui::Modifiers) -> Option<Vec<u8>> { key_bytes(k, mods, false) }
    const NONE: egui::Modifiers = egui::Modifiers::NONE;

    #[test]
    fn keys_become_what_a_terminal_sends() {
        assert_eq!(key(egui::Key::Enter, NONE).unwrap(), b"\r");
        assert_eq!(key(egui::Key::Enter, egui::Modifiers::SHIFT).unwrap(), b"\x1b\r", "a new line without sending");
        assert_eq!(key(egui::Key::Backspace, NONE).unwrap(), b"\x7f");
        assert_eq!(key(egui::Key::Tab, egui::Modifiers::SHIFT).unwrap(), b"\x1b[Z", "Claude Code changes mode with Shift+Tab");
        assert_eq!(key(egui::Key::Escape, NONE).unwrap(), b"\x1b");
        assert_eq!(key(egui::Key::C, egui::Modifiers::CTRL).unwrap(), vec![3], "Ctrl+C");
        assert_eq!(key(egui::Key::D, egui::Modifiers::CTRL).unwrap(), vec![4]);
        assert_eq!(key(egui::Key::ArrowUp, NONE).unwrap(), b"\x1b[A");
        assert_eq!(key_bytes(egui::Key::ArrowUp, NONE, true).unwrap(), b"\x1bOA", "the program asked for application cursor keys");
        assert_eq!(key(egui::Key::ArrowLeft, egui::Modifiers::CTRL).unwrap(), b"\x1b[1;5D");
        assert_eq!(key(egui::Key::Delete, NONE).unwrap(), b"\x1b[3~");
        assert_eq!(key(egui::Key::PageUp, NONE).unwrap(), b"\x1b[5~");
        assert_eq!(key(egui::Key::B, egui::Modifiers::ALT).unwrap(), b"\x1bb", "Alt is Escape first");
        assert_eq!(key(egui::Key::A, NONE), None, "a letter arrives as text, not as a key");
        assert_eq!(key(egui::Key::C, egui::Modifiers::MAC_CMD), None, "Cmd is the app's, for copying");
    }

    #[test]
    fn colours_are_the_usual_ones() {
        use alacritty_terminal::vte::ansi::NamedColor;
        let dark = Palette::new(true);
        assert_eq!(resolve(Color::Named(NamedColor::Red), &dark), dark.ansi(1));
        assert_eq!(resolve(Color::Named(NamedColor::Foreground), &dark), dark.fg);
        assert_eq!(resolve(Color::Named(NamedColor::Background), &dark), dark.bg);
        assert_eq!(resolve(Color::Spec(Rgb { r: 1, g: 2, b: 3 }), &dark), egui::Color32::from_rgb(1, 2, 3));
        assert_eq!(indexed(16, &dark), egui::Color32::from_rgb(0, 0, 0));
        assert_eq!(indexed(231, &dark), egui::Color32::from_rgb(255, 255, 255));
        assert_eq!(indexed(232, &dark), egui::Color32::from_rgb(8, 8, 8));
        assert_eq!(indexed(9, &dark), dark.ansi(9));
    }

    #[test]
    fn the_terminal_has_a_palette_for_each_mode_and_the_text_can_be_read_in_both() {
        let (dark, light) = (Palette::new(true), Palette::new(false));
        let lum = |c: egui::Color32| 0.2126 * c.r() as f32 + 0.7152 * c.g() as f32 + 0.0722 * c.b() as f32;
        assert!(lum(dark.bg) < 60.0 && lum(dark.fg) > 150.0, "dark mode: light text on a dark page");
        assert!(lum(light.bg) > 200.0 && lum(light.fg) < 80.0, "light mode: dark text on a light page");
        assert_ne!(resolve(Color::Named(alacritty_terminal::vte::ansi::NamedColor::Foreground), &dark), resolve(Color::Named(alacritty_terminal::vte::ansi::NamedColor::Foreground), &light));
        for (name, pal) in [("dark", dark), ("light", light)] {
            // Every colour a program might print with has to stand out from the page it is printed on.
            for i in 1..16 {
                if i == 7 && name == "dark" { continue; }
                let gap = (lum(pal.ansi(i)) - lum(pal.bg)).abs();
                assert!(gap > 35.0, "{name} mode: colour {i} is too close to the page ({gap})");
            }
            assert!((lum(pal.selection_bg) - lum(pal.selection_fg)).abs() > 80.0, "{name} mode: selected text can be read");
        }
    }

    // These run /bin/sh, printf and stty, which Windows does not have.
    #[cfg(unix)]
    #[test]
    fn a_program_is_told_whether_the_background_is_light_or_dark() {
        let _guard = SHELL_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("SHELL", "/bin/sh");
        let ctx = egui::Context::default();
        for (dark, expect) in [(true, "15;0"), (false, "0;15")] {
            let mut t = Terminal::spawn("Test", "", &[], None, &[], dark, &ctx).unwrap();
            assert_eq!(t.spawned_dark, dark);
            t.send(b"echo mode=$COLORFGBG\r");
            wait_for(&mut t, &format!("mode={expect}"));
        }
    }

    #[test]
    fn a_tool_is_told_a_presentation_means_slides_in_this_app() {
        let text = instructions();
        for word in ["presentation", "slide deck", "slideshow", "course", "lesson", "training", "tutorial", "quiz", "pitch", "e-learning", "PowerPoint"] { assert!(text.contains(word), "it should name {word}"); }
        assert!(text.contains("no name at all") && text.contains("brainbend tools") && text.contains("Do not make a web page"), "{text}");
        assert!(text.contains("get_project"), "it carries the app's own advice on how to work");
    }

    #[test]
    fn a_word_is_quoted_for_the_shell() {
        assert_eq!(quote("claude"), "'claude'");
        assert_eq!(quote("it's"), r"'it'\''s'");
    }

    /// Runs `sh` with a script in a terminal, without the login shell.
    #[cfg(unix)]
    fn sh(script: &str) -> Terminal {
        let ctx = egui::Context::default();
        let mut t = Terminal::spawn("Test", "", &[], None, &[], true, &ctx).unwrap();
        // The real thing runs the author's shell; a test runs a script of its own.
        t.send(format!("{script}\n").as_bytes());
        t
    }

    #[cfg(unix)]
    fn wait_for(t: &mut Terminal, what: &str) -> String {
        for _ in 0..400 {
            t.pump();
            let s = t.screen_text();
            if s.contains(what) { return s; }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("never saw {what:?}; the screen was:\n{}", t.screen_text());
    }

    // These run /bin/sh, printf and stty, which Windows does not have.
    #[cfg(unix)]
    #[test]
    fn what_a_program_prints_reaches_the_screen_and_what_is_typed_reaches_it() {
        let _guard = SHELL_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("SHELL", "/bin/sh");
        let mut t = sh("printf 'hello \\033[31mred\\033[0m world\\n'; echo $((6*7))");
        let screen = wait_for(&mut t, "42");
        assert!(screen.contains("hello red world"), "{screen}");
        t.send(b"echo typed-by-us\r");
        wait_for(&mut t, "typed-by-us");
        t.paste("echo pasted-text\n");
        wait_for(&mut t, "pasted-text");
    }

    // These run /bin/sh, printf and stty, which Windows does not have.
    #[cfg(unix)]
    #[test]
    fn it_follows_the_size_and_notices_the_program_ending() {
        let _guard = SHELL_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("SHELL", "/bin/sh");
        let mut t = sh("stty size");
        wait_for(&mut t, "24 80");
        t.resize(50, 10);
        t.send(b"stty size\r");
        wait_for(&mut t, "10 50");
        t.send(b"exit 3\r");
        for _ in 0..400 {
            t.pump();
            if t.exited.is_some() { break; }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(t.exited.as_deref().is_some_and(|s| s.contains('3')), "{:?}", t.exited);
    }

    // These run /bin/sh, printf and stty, which Windows does not have.
    #[cfg(unix)]
    #[test]
    fn a_tool_that_is_not_installed_says_so_instead_of_leaving_a_blank_terminal() {
        let _guard = SHELL_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("SHELL", "/bin/sh");
        let ctx = egui::Context::default();
        let mut t = Terminal::spawn("Nothing", "definitely-not-a-real-program-xyz", &[], None, &[], true, &ctx).unwrap();
        wait_for(&mut t, "was not found");
    }

    #[cfg(unix)]
    static SHELL_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
}
