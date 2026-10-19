//! Newline-delimited, versioned presentation frames for the GNOME panel.

use std::io::{self, Read, Write};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::presentation::{Cell, SnapshotAverager, present};
use crate::telemetry::{MemoryAmount, Reading, Snapshot, TelemetrySession};

fn write_frame(output: &mut impl Write, snapshot: &Snapshot) -> io::Result<()> {
    #[derive(serde::Serialize)]
    struct PanelCell<'a> {
        #[serde(flatten)]
        cell: &'a Cell,
        #[serde(skip_serializing_if = "Option::is_none")]
        amount: Option<&'a MemoryAmount>,
        #[serde(skip_serializing_if = "Option::is_none")]
        percentages: Option<Vec<f64>>,
    }
    let cells = present(snapshot);
    let cells: Vec<_> = cells
        .iter()
        .map(|cell| {
            let amount = match cell.label {
                "RAM" => Some(&snapshot.ram),
                "Swap" => Some(&snapshot.swap),
                "VRAM" => Some(&snapshot.gpu.vram),
                "/tmp" => Some(&snapshot.storage.tmp),
                _ => None,
            }
            .and_then(|reading| match reading {
                Reading::Value(amount) => Some(amount),
                Reading::Unavailable(_) => None,
            });
            let percentages = if cell.label == "Storage" {
                match &snapshot.storage.primary_volumes {
                    Reading::Value(volumes) if !volumes.is_empty() => volumes
                        .iter()
                        .map(|volume| {
                            (volume.total_bytes > 0 && volume.available_bytes <= volume.total_bytes)
                                .then(|| {
                                    volume.available_bytes as f64 / volume.total_bytes as f64
                                        * 100.0
                                })
                        })
                        .collect::<Option<Vec<_>>>(),
                    Reading::Value(_) | Reading::Unavailable(_) => None,
                }
            } else {
                None
            };
            PanelCell {
                cell,
                amount,
                percentages,
            }
        })
        .collect();
    let frame = serde_json::json!({ "version": 1, "cells": cells });
    serde_json::to_writer(&mut *output, &frame)?;
    output.write_all(b"\n")?;
    output.flush()
}

/// Keep one telemetry session alive. Closing stdin requests a clean shutdown;
/// a closed stdout also terminates the session rather than keeping it orphaned.
pub fn run_stream() -> io::Result<()> {
    let (closed_tx, closed_rx) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let mut byte = [0];
        let mut input = io::stdin().lock();
        while matches!(input.read(&mut byte), Ok(1)) {}
        let _ = closed_tx.send(());
    });
    let mut session = TelemetrySession::start_system()?;
    let mut output = io::stdout().lock();
    let result: io::Result<()> = (|| {
        write_frame(&mut output, &session.snapshot())?;
        let started = Instant::now();
        let mut averager = SnapshotAverager::default();
        while let Err(mpsc::RecvTimeoutError::Timeout) =
            closed_rx.recv_timeout(Duration::from_secs(1))
        {
            if let Some(snapshot) = averager.update(started.elapsed(), session.snapshot()) {
                write_frame(&mut output, &snapshot)?;
            }
        }
        Ok(())
    })();
    session.stop();
    match result {
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_frames_supply_exact_memory_bounds_without_parsing_descriptions() {
        let mut snapshot = Snapshot {
            ram: Reading::Value(MemoryAmount {
                used_bytes: 30 << 30,
                total_bytes: 64 << 30,
            }),
            swap: Reading::Value(MemoryAmount {
                used_bytes: 7 << 30,
                total_bytes: 8 << 30,
            }),
            ..Snapshot::default()
        };
        snapshot.gpu.vram = Reading::Value(MemoryAmount {
            used_bytes: 640 << 20,
            total_bytes: 24 << 30,
        });
        let mut output = Vec::new();
        write_frame(&mut output, &snapshot).unwrap();
        let frame: serde_json::Value = serde_json::from_slice(&output).unwrap();
        let cells = frame["cells"].as_array().unwrap();
        for (label, used, total) in [
            ("RAM", 30_u64 << 30, 64_u64 << 30),
            ("Swap", 7 << 30, 8 << 30),
            ("VRAM", 640 << 20, 24 << 30),
        ] {
            let cell = cells.iter().find(|cell| cell["label"] == label).unwrap();
            assert_eq!(cell["amount"]["used_bytes"], used);
            assert_eq!(cell["amount"]["total_bytes"], total);
        }
        let missing = cells.iter().find(|cell| cell["label"] == "/tmp").unwrap();
        assert!(missing.get("amount").is_none());
    }
    #[test]
    fn storage_percentage_uses_available_space_without_counting_reserved_blocks_as_free() {
        let mut snapshot = Snapshot::default();
        snapshot.storage.primary_volumes = Reading::Value(vec![crate::telemetry::PrimaryVolume {
            paths: vec!["/".into(), "/home".into()],
            available_bytes: 20,
            used_bytes: 70,
            total_bytes: 100,
        }]);
        let mut output = Vec::new();
        write_frame(&mut output, &snapshot).unwrap();
        let frame: serde_json::Value = serde_json::from_slice(&output).unwrap();
        let cell = frame["cells"]
            .as_array()
            .unwrap()
            .iter()
            .find(|cell| cell["label"] == "Storage")
            .unwrap();
        assert_eq!(cell["percentages"], serde_json::json!([20.0]));
    }
}
