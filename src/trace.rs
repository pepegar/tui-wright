use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;

#[derive(Serialize)]
pub struct AsciicastHeader {
    pub version: u8,
    pub width: u16,
    pub height: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

pub struct TraceRecorder {
    writer: BufWriter<File>,
    start: Instant,
}

impl TraceRecorder {
    pub fn new(path: PathBuf, cols: u16, rows: u16, title: Option<String>) -> std::io::Result<Self> {
        let file = File::create(&path)?;
        let mut writer = BufWriter::new(file);

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .map(|d| d.as_secs());

        let header = AsciicastHeader {
            version: 2,
            width: cols,
            height: rows,
            timestamp,
            title,
        };

        let header_json = serde_json::to_string(&header)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        writeln!(writer, "{}", header_json)?;
        writer.flush()?;

        Ok(TraceRecorder {
            writer,
            start: Instant::now(),
        })
    }

    fn elapsed_seconds(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    fn write_event(&mut self, code: &str, data: &str) -> std::io::Result<()> {
        let event = serde_json::to_string(&(self.elapsed_seconds(), code, data))
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        writeln!(self.writer, "{}", event)?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn record_output(&mut self, raw_bytes: &[u8]) -> std::io::Result<()> {
        let data = String::from_utf8_lossy(raw_bytes);
        self.write_event("o", &data)
    }

    pub fn record_input(&mut self, raw_bytes: &[u8]) -> std::io::Result<()> {
        let data = String::from_utf8_lossy(raw_bytes);
        self.write_event("i", &data)
    }

    pub fn record_marker(&mut self, label: &str) -> std::io::Result<()> {
        self.write_event("m", label)
    }

    pub fn record_resize(&mut self, cols: u16, rows: u16) -> std::io::Result<()> {
        let data = format!("{}x{}", cols, rows);
        self.write_event("r", &data)
    }

    pub fn finish(mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_trace_header() {
        let dir = std::env::temp_dir();
        let path = dir.join("test-trace-header.cast");
        let recorder = TraceRecorder::new(path.clone(), 80, 24, Some("test".to_string())).unwrap();
        recorder.finish().unwrap();

        let content = fs::read_to_string(&path).unwrap();
        let header: serde_json::Value = serde_json::from_str(content.lines().next().unwrap()).unwrap();
        assert_eq!(header["version"], 2);
        assert_eq!(header["width"], 80);
        assert_eq!(header["height"], 24);
        assert_eq!(header["title"], "test");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_trace_output_event() {
        let dir = std::env::temp_dir();
        let path = dir.join("test-trace-output.cast");
        let mut recorder = TraceRecorder::new(path.clone(), 80, 24, None).unwrap();
        recorder.record_output(b"hello world").unwrap();
        recorder.finish().unwrap();

        let content = fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.trim().lines().collect();
        assert_eq!(lines.len(), 2);

        let event: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(event[1], "o");
        assert_eq!(event[2], "hello world");
        assert!(event[0].as_f64().unwrap() >= 0.0);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_trace_all_event_types() {
        let dir = std::env::temp_dir();
        let path = dir.join("test-trace-all-events.cast");
        let mut recorder = TraceRecorder::new(path.clone(), 80, 24, None).unwrap();
        recorder.record_output(b"output").unwrap();
        recorder.record_input(b"input").unwrap();
        recorder.record_marker("checkpoint").unwrap();
        recorder.record_resize(120, 40).unwrap();
        recorder.finish().unwrap();

        let content = fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.trim().lines().collect();
        assert_eq!(lines.len(), 5);

        let e1: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(e1[1], "o");
        assert_eq!(e1[2], "output");

        let e2: serde_json::Value = serde_json::from_str(lines[2]).unwrap();
        assert_eq!(e2[1], "i");
        assert_eq!(e2[2], "input");

        let e3: serde_json::Value = serde_json::from_str(lines[3]).unwrap();
        assert_eq!(e3[1], "m");
        assert_eq!(e3[2], "checkpoint");

        let e4: serde_json::Value = serde_json::from_str(lines[4]).unwrap();
        assert_eq!(e4[1], "r");
        assert_eq!(e4[2], "120x40");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_trace_timestamps_increase() {
        let dir = std::env::temp_dir();
        let path = dir.join("test-trace-timestamps.cast");
        let mut recorder = TraceRecorder::new(path.clone(), 80, 24, None).unwrap();
        recorder.record_output(b"first").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        recorder.record_output(b"second").unwrap();
        recorder.finish().unwrap();

        let content = fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.trim().lines().collect();
        let e1: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
        let e2: serde_json::Value = serde_json::from_str(lines[2]).unwrap();
        assert!(e2[0].as_f64().unwrap() > e1[0].as_f64().unwrap());

        let _ = fs::remove_file(&path);
    }
}
