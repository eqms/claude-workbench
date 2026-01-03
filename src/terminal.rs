use anyhow::Result;
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;

pub struct PseudoTerminal {
    pub parser: Arc<Mutex<vt100::Parser>>,
    pub writer: Box<dyn Write + Send>,
    pub master: Box<dyn portable_pty::MasterPty + Send>,
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

        let parser = Arc::new(Mutex::new(vt100::Parser::new(rows, cols, 0)));
        let mut reader = pair.master.try_clone_reader()?;
        let parser_clone = parser.clone();

        // Spawn a background thread to read from PTY and update the parser
        thread::spawn(move || {
            let mut buffer = [0u8; 4096];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        let mut parser = parser_clone.lock().unwrap();
                        parser.process(&buffer[..n]);
                    }
                    Err(_) => break, // Error
                }
            }
        });

        // Spawn the command
        let cmd_str = &command[0];
        let args = &command[1..];
        let mut cmd = CommandBuilder::new(cmd_str);
        cmd.args(args);
        cmd.cwd(cwd);
        
        // Create environment that suppresses Fish's DA query but keeps colors
        cmd.env("TERM", "xterm-256color");
        cmd.env("fish_features", "no-query-term");
        
        let _child = pair.slave.spawn_command(cmd)?;
        
        // We drop _child here, it runs in background. 
        // In a real app we might want to keep it to check exit status.

        let writer = pair.master.take_writer()?;

        Ok(Self {
            parser,
            writer,
            master: pair.master,
        })
    }

    pub fn resize(&mut self, rows: u16, cols: u16) -> Result<()> {
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
        self.writer.write_all(input)?;
        self.writer.flush()?;
        Ok(())
    }
}
