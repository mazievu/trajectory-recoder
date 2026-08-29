use core_types::event::RawEvent;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Crash-resilient append-only NDJSON event writer with periodic sync.
pub struct NdjsonWriter {
    path: PathBuf,
    writer: BufWriter<File>,
    last_flush: Instant,
    flush_interval: Duration,
    records_written: usize,
}

impl NdjsonWriter {
    pub fn open(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let p = path.as_ref().to_path_buf();
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = OpenOptions::new().create(true).append(true).open(&p)?;
        let writer = BufWriter::with_capacity(64 * 1024, file);

        Ok(Self {
            path: p,
            writer,
            last_flush: Instant::now(),
            flush_interval: Duration::from_secs(2), // Max 2s flush interval
            records_written: 0,
        })
    }

    pub fn write_event(&mut self, event: &RawEvent) -> std::io::Result<()> {
        self.write_record(event)
    }

    pub fn write_record<T: serde::Serialize>(&mut self, record: &T) -> std::io::Result<()> {
        let json_line = serde_json::to_string(record)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        self.writer.write_all(json_line.as_bytes())?;
        self.writer.write_all(b"\n")?;
        self.records_written += 1;

        if self.last_flush.elapsed() >= self.flush_interval {
            self.flush_sync()?;
        }

        Ok(())
    }

    pub fn flush_sync(&mut self) -> std::io::Result<()> {
        self.writer.flush()?;
        self.writer.get_ref().sync_data()?;
        self.last_flush = Instant::now();
        Ok(())
    }

    pub fn records_written(&self) -> usize {
        self.records_written
    }
}

impl Drop for NdjsonWriter {
    fn drop(&mut self) {
        let _ = self.flush_sync();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_types::event::{EventSource, RawEventPayload, RawMouseEvent};
    use core_types::id::GlobalEventId;
    use core_types::timestamp::DualTimestamp;
    use tempfile::tempdir;

    #[test]
    fn test_ndjson_writer_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("events.raw.ndjson");

        let mut writer = NdjsonWriter::open(&path).unwrap();

        let event1 = RawEvent::new(
            1,
            GlobalEventId::new(1),
            DualTimestamp::now(),
            "M1".to_string(),
            1,
            "U1".to_string(),
            EventSource::Win32Hook,
            1,
            RawEventPayload::Mouse(RawMouseEvent::default()),
        );

        let event2 = RawEvent::new(
            2,
            GlobalEventId::new(2),
            DualTimestamp::now(),
            "M1".to_string(),
            1,
            "U1".to_string(),
            EventSource::Win32Hook,
            2,
            RawEventPayload::Mouse(RawMouseEvent::default()),
        );

        writer.write_event(&event1).unwrap();
        writer.write_event(&event2).unwrap();
        writer.flush_sync().unwrap();

        assert_eq!(writer.records_written(), 2);

        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);

        let read_ev1: RawEvent = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(read_ev1.event_id, 1);
    }
}
