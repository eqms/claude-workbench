use anyhow::Result;
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

pub struct PseudoTerminal {
    pub parser: Arc<Mutex<vt100::Parser>>,
    pub writer: Box<dyn Write + Send>,
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

        let parser = Arc::new(Mutex::new(vt100::Parser::new(rows, cols, 1000)));
        let exited = Arc::new(AtomicBool::new(false));
        let mut reader = pair.master.try_clone_reader()?;
        let parser_clone = parser.clone();
        let exited_clone = exited.clone();

        // Spawn a background thread to read from PTY and update the parser
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
                        let mut parser = parser_clone.lock().unwrap();
                        parser.process(&buffer[..n]);
                    }
                    Err(_) => {
                        // Error - process probably exited
                        exited_clone.store(true, Ordering::SeqCst);
                        break;
                    }
                }
            }
        });

        // Spawn the command
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

        let writer = pair.master.take_writer()?;

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
        self.writer.write_all(input)?;
        self.writer.flush()?;
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
                    line.push_str(cell.contents());
                }
            }
            // Trim trailing whitespace but preserve leading
            lines.push(line.trim_end().to_string());
        }

        lines
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

    /// Check if the PTY process has exited
    pub fn has_exited(&self) -> bool {
        self.exited.load(Ordering::SeqCst)
    }
}
