use anyhow::Result;
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

/// Callbacks for handling terminal queries (DSR, DA) that require responses
/// back to the child process running inside the PTY.
pub struct PtyCallbacks {
    pending_responses: Vec<Vec<u8>>,
}

impl PtyCallbacks {
    fn new() -> Self {
        Self {
            pending_responses: Vec::new(),
        }
    }

    fn drain_responses(&mut self) -> Vec<Vec<u8>> {
        std::mem::take(&mut self.pending_responses)
    }
}

impl vt100::Callbacks for PtyCallbacks {
    fn unhandled_csi(
        &mut self,
        screen: &mut vt100::Screen,
        i1: Option<u8>,
        _i2: Option<u8>,
        params: &[&[u16]],
        c: char,
    ) {
        let param = params.first().and_then(|p| p.first()).copied().unwrap_or(0);
        match (i1, c) {
            // DSR/CPR: \x1b[6n or \x1b[?6n → respond \x1b[{row+1};{col+1}R
            (None, 'n') | (Some(b'?'), 'n') if param == 6 => {
                let (row, col) = screen.cursor_position();
                self.pending_responses
                    .push(format!("\x1b[{};{}R", row + 1, col + 1).into_bytes());
            }
            // DA: \x1b[c / \x1b[0c → respond \x1b[?1;2c (VT100 with AVO)
            (None, 'c') if param == 0 => {
                self.pending_responses.push(b"\x1b[?1;2c".to_vec());
            }
            // DA2: \x1b[>c / \x1b[>0c → respond \x1b[>0;0;0c
            (Some(b'>'), 'c') if param == 0 => {
                self.pending_responses.push(b"\x1b[>0;0;0c".to_vec());
            }
            _ => {}
        }
    }
}

pub struct PseudoTerminal {
    pub parser: Arc<Mutex<vt100::Parser<PtyCallbacks>>>,
    pub writer: Arc<Mutex<Box<dyn Write + Send>>>,
    pub master: Box<dyn portable_pty::MasterPty + Send>,
    /// Flag indicating if the PTY process has exited (reader thread got EOF)
    pub exited: Arc<AtomicBool>,
}

impl PseudoTerminal {
    pub fn new(command: &[String], rows: u16, cols: u16, cwd: &std::path::Path) -> Result<Self> {
        let pty_system = NativePtySystem::default();
        let pair = pty_system.openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let parser = Arc::new(Mutex::new(vt100::Parser::new_with_callbacks(
            rows,
            cols,
            1000,
            PtyCallbacks::new(),
        )));
        let exited = Arc::new(AtomicBool::new(false));
        let mut reader = pair.master.try_clone_reader()?;
        let parser_clone = parser.clone();
        let exited_clone = exited.clone();

        // Spawn the command before taking the writer, so we have the slave end
        let cmd_str = &command[0];
        let args = &command[1..];
        let mut cmd = CommandBuilder::new(cmd_str);
        cmd.args(args);
        cmd.cwd(cwd);

        // Inherit all environment variables from parent process
        // This is critical for Claude CLI which needs LANG, LC_ALL, HOME, PATH, etc.
        for (key, value) in std::env::vars() {
            cmd.env(key, value);
        }

        // Override specific settings for terminal compatibility
        cmd.env("TERM", "xterm-256color");
        cmd.env("fish_features", "no-query-term");
        // Ensure UTF-8 encoding for proper character handling
        cmd.env("LANG", "en_US.UTF-8");
        cmd.env("LC_ALL", "en_US.UTF-8");

        let _child = pair.slave.spawn_command(cmd)?;

        // We drop _child here, it runs in background.
        // In a real app we might want to keep it to check exit status.

        let writer: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(pair.master.take_writer()?));
        let writer_clone = writer.clone();

        // Spawn a background thread to read from PTY, update the parser,
        // and write back responses for terminal queries (DSR/CPR, DA)
        thread::spawn(move || {
            let mut buffer = [0u8; 4096];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => {
                        // EOF - process has exited
                        exited_clone.store(true, Ordering::SeqCst);
                        break;
                    }
                    Ok(n) => {
                        let responses = {
                            let mut parser = parser_clone.lock().unwrap();
                            parser.process(&buffer[..n]);
                            parser.callbacks_mut().drain_responses()
                        };
                        if !responses.is_empty() {
                            if let Ok(mut w) = writer_clone.lock() {
                                for response in responses {
                                    let _ = w.write_all(&response);
                                }
                                let _ = w.flush();
                            }
                        }
                    }
                    Err(_) => {
                        // Error - process probably exited
                        exited_clone.store(true, Ordering::SeqCst);
                        break;
                    }
                }
            }
        });

        Ok(Self {
            parser,
            writer,
            master: pair.master,
            exited,
        })
    }

    pub fn resize(&mut self, rows: u16, cols: u16) -> Result<()> {
        {
            let parser = self.parser.lock().unwrap();
            let screen = parser.screen();
            let (curr_rows, curr_cols) = screen.size();
            if curr_rows == rows && curr_cols == cols {
                return Ok(());
            }
        }

        self.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        let mut parser = self.parser.lock().unwrap();
        parser.screen_mut().set_size(rows, cols);
        Ok(())
    }

    pub fn write_input(&mut self, input: &[u8]) -> Result<()> {
        // If typing, reset scrollback
        {
            let mut parser = self.parser.lock().unwrap();
            let screen = parser.screen_mut();
            if screen.scrollback() > 0 {
                screen.set_scrollback(0);
            }
        }
        let mut writer = self.writer.lock().unwrap();
        writer.write_all(input)?;
        writer.flush()?;
        Ok(())
    }

    pub fn scroll_up(&self, lines: usize) {
        let mut parser = self.parser.lock().unwrap();
        let screen = parser.screen_mut();
        let current = screen.scrollback();
        screen.set_scrollback(current + lines);
    }

    pub fn scroll_down(&self, lines: usize) {
        let mut parser = self.parser.lock().unwrap();
        let screen = parser.screen_mut();
        let current = screen.scrollback();
        if current >= lines {
            screen.set_scrollback(current - lines);
        } else {
            screen.set_scrollback(0);
        }
    }

    /// Push cell content to line, treating empty cells as spaces.
    /// The vt100 crate returns "" for space cells (len=0), so we
    /// must fall back to ' ' to preserve whitespace between words.
    fn push_cell_content(line: &mut String, cell: &vt100::Cell) {
        let text = cell.contents();
        if text.is_empty() {
            line.push(' ');
        } else {
            line.push_str(text);
        }
    }

    /// Extract text content from specified line range (screen-relative, 0-based)
    /// Returns lines as strings with trailing whitespace trimmed
    pub fn extract_lines(&self, start: usize, end: usize) -> Vec<String> {
        let parser = self.parser.lock().unwrap();
        let screen = parser.screen();
        let (rows, cols) = screen.size();

        let mut lines = Vec::new();
        for row in start..=end {
            if row >= rows as usize {
                break;
            }

            let mut line = String::new();
            for col in 0..cols {
                if let Some(cell) = screen.cell(row as u16, col) {
                    Self::push_cell_content(&mut line, cell);
                }
            }
            // Trim trailing whitespace but preserve leading
            lines.push(line.trim_end().to_string());
        }

        lines
    }

    /// Extract the last `count` lines from the most recent terminal output.
    /// Temporarily sets scrollback to 0 to access the bottom of the buffer,
    /// then restores the original scrollback position.
    pub fn extract_last_n_lines(&self, count: usize) -> Vec<String> {
        let mut parser = self.parser.lock().unwrap();
        let screen = parser.screen_mut();

        // Save and reset scrollback to see most recent output
        let saved = screen.scrollback();
        screen.set_scrollback(0);

        let (rows, cols) = screen.size();
        let start_row = if rows as usize > count {
            rows as usize - count
        } else {
            0
        };

        let mut lines = Vec::new();
        for row in start_row..rows as usize {
            let mut line = String::new();
            for col in 0..cols {
                if let Some(cell) = screen.cell(row as u16, col) {
                    Self::push_cell_content(&mut line, cell);
                }
            }
            lines.push(line.trim_end().to_string());
        }

        // Restore original scrollback position
        screen.set_scrollback(saved);
        lines
    }

    /// Extract text content from character-level selection range
    /// (start_row, start_col) to (end_row, end_col) - all 0-based
    /// For multi-line selections:
    /// - First line: from start_col to end of line
    /// - Middle lines: entire line
    /// - Last line: from start to end_col
    pub fn extract_char_range(
        &self,
        start_row: usize,
        start_col: usize,
        end_row: usize,
        end_col: usize,
    ) -> String {
        let parser = self.parser.lock().unwrap();
        let screen = parser.screen();
        let (rows, cols) = screen.size();

        let mut result = String::new();

        for row in start_row..=end_row {
            if row >= rows as usize {
                break;
            }

            let col_start = if row == start_row { start_col } else { 0 };
            let col_end = if row == end_row {
                end_col.min(cols as usize)
            } else {
                cols as usize
            };

            let mut line = String::new();
            for col in col_start..col_end {
                if col >= cols as usize {
                    break;
                }
                if let Some(cell) = screen.cell(row as u16, col as u16) {
                    Self::push_cell_content(&mut line, cell);
                }
            }

            // Trim trailing whitespace for middle/last lines, but not leading
            if row == end_row {
                result.push_str(line.trim_end());
            } else {
                result.push_str(line.trim_end());
                result.push('\n');
            }
        }

        result
    }

    /// Get the current visible cursor row (0-based)
    pub fn cursor_row(&self) -> u16 {
        let parser = self.parser.lock().unwrap();
        let screen = parser.screen();
        screen.cursor_position().0
    }

    /// Get current scrollback offset
    pub fn scrollback(&self) -> usize {
        let parser = self.parser.lock().unwrap();
        parser.screen().scrollback()
    }

    /// Set scrollback position by ratio (0.0 = top/max scrollback, 1.0 = bottom/current)
    pub fn set_scrollback_position(&self, ratio: f64) {
        let max_scrollback = 1000usize;
        let target = ((1.0 - ratio) * max_scrollback as f64) as usize;
        let mut parser = self.parser.lock().unwrap();
        let screen = parser.screen_mut();
        screen.set_scrollback(target);
    }

    /// Check if the PTY process has exited
    pub fn has_exited(&self) -> bool {
        self.exited.load(Ordering::SeqCst)
    }
}
