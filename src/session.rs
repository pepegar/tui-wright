use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};

use crate::error::{Error, Result};
use crate::input::{self, Key};
use crate::screen::{self, ScreenSnapshot};
use crate::trace::TraceRecorder;

type TraceSink = Arc<Mutex<Option<TraceRecorder>>>;

pub struct Session {
    parser: Arc<Mutex<vt100::Parser>>,
    writer: Box<dyn Write + Send>,
    pty: portable_pty::PtyPair,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    _reader_handle: thread::JoinHandle<()>,
    trace: TraceSink,
    cols: u16,
    rows: u16,
}

impl Session {
    pub fn spawn(command: &str, args: &[String], cols: u16, rows: u16, cwd: &Path) -> Result<Self> {
        let pty_system = NativePtySystem::default();
        let pty = pty_system.openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let mut cmd = CommandBuilder::new(command);
        cmd.args(args);
        cmd.cwd(cwd);

        let child = pty.slave.spawn_command(cmd)?;
        let writer = pty.master.take_writer()?;
        let mut reader = pty.master.try_clone_reader()?;

        let parser = Arc::new(Mutex::new(vt100::Parser::new(rows, cols, 0)));
        let trace: TraceSink = Arc::new(Mutex::new(None));

        let parser_clone = Arc::clone(&parser);
        let trace_clone = Arc::clone(&trace);
        let reader_handle = thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if let Ok(mut t) = trace_clone.lock() {
                            if let Some(ref mut recorder) = *t {
                                let _ = recorder.record_output(&buf[..n]);
                            }
                        }
                        let mut p = parser_clone.lock().unwrap();
                        p.process(&buf[..n]);
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Session {
            parser,
            writer,
            pty,
            child,
            _reader_handle: reader_handle,
            trace,
            cols,
            rows,
        })
    }

    pub fn screen_text(&self) -> String {
        let parser = self.parser.lock().unwrap();
        screen::screen_text(parser.screen())
    }

    pub fn screen_snapshot(&self) -> ScreenSnapshot {
        let parser = self.parser.lock().unwrap();
        screen::from_screen(parser.screen())
    }

    pub fn cursor_position(&self) -> (u16, u16) {
        let parser = self.parser.lock().unwrap();
        parser.screen().cursor_position()
    }

    pub fn type_text(&mut self, text: &str) -> Result<()> {
        self.trace_input(text.as_bytes());
        self.writer.write_all(text.as_bytes())?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn send_key(&mut self, key: &Key) -> Result<()> {
        let seq = key.to_escape_sequence();
        self.trace_input(&seq);
        self.writer.write_all(&seq)?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn send_key_by_name(&mut self, name: &str) -> Result<()> {
        let key = input::parse_key_name(name)?;
        self.send_key(&key)
    }

    pub fn send_mouse(&mut self, action: &str, col: u16, row: u16) -> Result<()> {
        let mouse_action = input::parse_mouse_action(action)?;
        let seq = input::mouse_sgr_sequence(&mouse_action, col, row);
        self.trace_input(&seq);
        self.writer.write_all(&seq)?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        self.pty.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        let mut parser = self.parser.lock().unwrap();
        parser.set_size(rows, cols);
        if let Ok(mut t) = self.trace.lock() {
            if let Some(ref mut recorder) = *t {
                let _ = recorder.record_resize(cols, rows);
            }
        }
        Ok(())
    }

    pub fn kill(&mut self) -> Result<()> {
        self.child.kill().map_err(|e| Error::Pty(e.to_string()))?;
        Ok(())
    }

    pub fn is_alive(&mut self) -> bool {
        self.child
            .try_wait()
            .ok()
            .map(|status| status.is_none())
            .unwrap_or(false)
    }

    pub fn trace_start(&self, output_path: PathBuf, title: Option<String>) -> Result<()> {
        let recorder = TraceRecorder::new(output_path, self.cols, self.rows, title)?;
        let mut t = self.trace.lock().unwrap();
        *t = Some(recorder);
        Ok(())
    }

    pub fn trace_stop(&self) -> Result<()> {
        let mut t = self.trace.lock().unwrap();
        if let Some(recorder) = t.take() {
            recorder.finish()?;
        }
        Ok(())
    }

    pub fn trace_marker(&self, label: &str) {
        if let Ok(mut t) = self.trace.lock() {
            if let Some(ref mut recorder) = *t {
                let _ = recorder.record_marker(label);
            }
        }
    }

    fn trace_input(&self, raw_bytes: &[u8]) {
        if let Ok(mut t) = self.trace.lock() {
            if let Some(ref mut recorder) = *t {
                let _ = recorder.record_input(raw_bytes);
            }
        }
    }
}
