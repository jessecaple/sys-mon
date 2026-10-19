//! Current-only text and rail semantics for the native dashboard.

use crate::telemetry::{
    InodeAmount, MemoryAmount, PrimaryVolume, Reading, Snapshot, Temperature, ThresholdKind,
    UnavailableReason,
};

mod averaging;
pub(crate) use averaging::SnapshotAverager;

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct DisplayLine {
    pub value: String,
    pub description: String,
    pub rail: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Cell {
    pub label: &'static str,
    pub glyph: &'static str,
    pub lines: Vec<DisplayLine>,
}

impl Cell {
    fn one(label: &'static str, glyph: &'static str, line: DisplayLine) -> Self {
        Self {
            label,
            glyph,
            lines: vec![line],
        }
    }
}

fn unavailable(label: &str, reason: UnavailableReason) -> DisplayLine {
    let (value, detail) = match reason {
        UnavailableReason::NotYetSampled => ("Waiting", "waiting for the first reading"),
        UnavailableReason::WarmingUp => ("Waiting", "calculating the first reading"),
        UnavailableReason::Missing => ("Unavailable", "source unavailable"),
        UnavailableReason::PermissionDenied => ("No access", "permission denied"),
        UnavailableReason::InvalidData => ("Invalid reading", "source returned invalid data"),
        UnavailableReason::SourceError => ("Source error", "source failed"),
        UnavailableReason::TimedOut => ("Timed out", "source timed out"),
    };
    DisplayLine {
        value: value.into(),
        description: format!("{label}: {detail}"),
        rail: None,
    }
}

fn value_line(value: String, description: String, rail: Option<f64>) -> DisplayLine {
    DisplayLine {
        value,
        description,
        rail,
    }
}

fn percent(label: &str, reading: &Reading<f64>) -> DisplayLine {
    match reading {
        Reading::Value(n) if n.is_finite() && *n >= 0.0 => {
            let visible = if *n > 0.0 && *n < 0.1 {
                "<0.1%".to_string()
            } else if *n > 0.0 && *n < 1.0 {
                format!("{n:.1}%")
            } else {
                format!("{n:.0}%")
            };
            value_line(
                visible,
                format!("{label}: {n} percent"),
                Some((n / 100.0).clamp(0.0, 1.0)),
            )
        }
        Reading::Value(_) => unavailable(label, UnavailableReason::InvalidData),
        Reading::Unavailable(reason) => unavailable(label, *reason),
    }
}

fn bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut quantity = bytes as f64;
    let mut unit = 0;
    while quantity >= 1024.0 && unit < UNITS.len() - 1 {
        quantity /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else if quantity < 100.0 {
        format!("{quantity:.1} {}", UNITS[unit])
    } else {
        format!("{quantity:.0} {}", UNITS[unit])
    }
}

fn rate(label: &str, reading: &Reading<f64>) -> DisplayLine {
    match reading {
        Reading::Value(n) if n.is_finite() && *n >= 0.0 => {
            let amount = if *n > 0.0 && *n < 0.5 {
                "<1 B".to_string()
            } else {
                bytes(n.round() as u64)
            };
            value_line(
                format!("{amount}/s"),
                format!("{label}: {amount} per second"),
                None,
            )
        }
        Reading::Value(_) => unavailable(label, UnavailableReason::InvalidData),
        Reading::Unavailable(reason) => unavailable(label, *reason),
    }
}

fn memory(label: &str, reading: &Reading<MemoryAmount>) -> DisplayLine {
    match reading {
        Reading::Value(amount) => {
            let used = bytes(amount.used_bytes);
            let rail = (amount.total_bytes > 0)
                .then(|| (amount.used_bytes as f64 / amount.total_bytes as f64).clamp(0.0, 1.0));
            let description = if amount.total_bytes > 0 {
                format!(
                    "{label}: {used} used of {} total",
                    bytes(amount.total_bytes)
                )
            } else {
                format!("{label}: {used} used; total capacity unknown")
            };
            value_line(used, description, rail)
        }
        Reading::Unavailable(reason) => unavailable(label, *reason),
    }
}

fn inodes(reading: &Reading<InodeAmount>) -> DisplayLine {
    match reading {
        Reading::Value(amount) if amount.total > 0 && amount.used <= amount.total => {
            let value = if amount.used < 1000 {
                amount.used.to_string()
            } else if amount.used < 1_000_000 {
                format!("{:.1}K", amount.used as f64 / 1000.0)
            } else {
                format!("{:.1}M", amount.used as f64 / 1_000_000.0)
            };
            value_line(
                value,
                format!(
                    "/tmp inodes: {} used of {} total",
                    amount.used, amount.total
                ),
                Some(amount.used as f64 / amount.total as f64),
            )
        }
        Reading::Value(_) => unavailable("/tmp inodes", UnavailableReason::InvalidData),
        Reading::Unavailable(reason) => unavailable("/tmp inodes", *reason),
    }
}

fn temperature(label: &str, reading: &Reading<Temperature>) -> DisplayLine {
    match reading {
        Reading::Value(temp) => {
            let celsius = temp.millidegrees_celsius as f64 / 1000.0;
            let visible = format!("{celsius:.0} °C");
            let scale = temp
                .threshold
                .as_ref()
                .filter(|t| t.millidegrees_celsius > 0);
            let (description, rail) = if let Some(scale) = scale {
                let threshold = scale.millidegrees_celsius as f64 / 1000.0;
                let kind = match scale.kind {
                    ThresholdKind::TargetOrMax => "target or maximum",
                    ThresholdKind::Critical => "critical limit",
                };
                (
                    format!("{label}: {visible}; {kind} {threshold:.0} °C"),
                    Some((celsius / threshold).clamp(0.0, 1.0)),
                )
            } else {
                (format!("{label}: {visible}; threshold unknown"), None)
            };
            value_line(visible, description, rail)
        }
        Reading::Unavailable(reason) => unavailable(label, *reason),
    }
}

fn volume_line(volume: &PrimaryVolume, include_path: bool) -> DisplayLine {
    let paths = if volume.paths.is_empty() {
        "Primary storage".to_string()
    } else {
        volume.paths.join(" and ")
    };
    let available = bytes(volume.available_bytes);
    let value = if include_path {
        format!(
            "{} {available}",
            volume.paths.first().map_or("Storage", String::as_str)
        )
    } else {
        available.clone()
    };
    let rail = (volume.total_bytes > 0)
        .then(|| (volume.used_bytes as f64 / volume.total_bytes as f64).clamp(0.0, 1.0));
    let description = if volume.total_bytes > 0 {
        format!(
            "Storage {paths}: {available} remaining; {} used of {} total",
            bytes(volume.used_bytes),
            bytes(volume.total_bytes)
        )
    } else {
        format!(
            "Storage {paths}: {available} remaining; {} used; total capacity unknown",
            bytes(volume.used_bytes)
        )
    };
    value_line(value, description, rail)
}

fn storage(reading: &Reading<Vec<PrimaryVolume>>) -> Vec<DisplayLine> {
    match reading {
        Reading::Value(volumes) if !volumes.is_empty() => {
            let include_path = volumes.len() > 1 || volumes[0].paths.len() == 1;
            volumes
                .iter()
                .map(|volume| volume_line(volume, include_path))
                .collect()
        }
        Reading::Value(_) => vec![unavailable("Storage", UnavailableReason::Missing)],
        Reading::Unavailable(reason) => vec![unavailable("Storage", *reason)],
    }
}

/// Row-major order for the two-column overview.
pub fn present(snapshot: &Snapshot) -> [Cell; 14] {
    [
        Cell::one("CPU", "cpu", percent("CPU", &snapshot.cpu.usage_percent)),
        Cell::one(
            "CPU temp",
            "temperature",
            temperature("CPU temperature", &snapshot.cpu.temperature),
        ),
        Cell::one("RAM", "ram", memory("RAM", &snapshot.ram)),
        Cell::one("Swap", "swap", memory("Swap", &snapshot.swap)),
        Cell::one("GPU", "gpu", percent("GPU", &snapshot.gpu.usage_percent)),
        Cell::one(
            "GPU temp",
            "temperature",
            temperature("GPU temperature", &snapshot.gpu.temperature),
        ),
        Cell::one("VRAM", "vram", memory("VRAM", &snapshot.gpu.vram)),
        Cell {
            label: "Storage",
            glyph: "storage",
            lines: storage(&snapshot.storage.primary_volumes),
        },
        Cell::one(
            "/tmp",
            "tmp",
            memory("/tmp filesystem", &snapshot.storage.tmp),
        ),
        Cell::one("Inodes", "inodes", inodes(&snapshot.storage.tmp_inodes)),
        Cell::one(
            "Network down",
            "network-down",
            rate("Network down", &snapshot.network.down_bytes_per_second),
        ),
        Cell::one(
            "Network up",
            "network-up",
            rate("Network up", &snapshot.network.up_bytes_per_second),
        ),
        Cell::one(
            "Storage read",
            "storage-read",
            rate("Storage read", &snapshot.storage.read_bytes_per_second),
        ),
        Cell::one(
            "Storage write",
            "storage-write",
            rate("Storage write", &snapshot.storage.write_bytes_per_second),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::{TemperatureThreshold, UnavailableReason};

    #[test]
    fn zero_missing_and_recovery_are_distinct() {
        let mut snapshot = Snapshot::default();
        assert_eq!(present(&snapshot)[0].lines[0].value, "Waiting");
        snapshot.cpu.usage_percent = Reading::Value(0.0);
        assert_eq!(present(&snapshot)[0].lines[0].value, "0%");
        snapshot.cpu.usage_percent = Reading::Unavailable(UnavailableReason::TimedOut);
        let timed_out = &present(&snapshot)[0].lines[0];
        assert_eq!(timed_out.value, "Timed out");
        assert!(timed_out.rail.is_none());
        snapshot.cpu.usage_percent = Reading::Value(32.0);
        assert_eq!(present(&snapshot)[0].lines[0].value, "32%");
    }

    #[test]
    fn all_unavailable_reasons_have_field_local_descriptions() {
        let cases = [
            (UnavailableReason::NotYetSampled, "Waiting", "first reading"),
            (UnavailableReason::WarmingUp, "Waiting", "calculating"),
            (UnavailableReason::Missing, "Unavailable", "unavailable"),
            (
                UnavailableReason::PermissionDenied,
                "No access",
                "permission denied",
            ),
            (
                UnavailableReason::InvalidData,
                "Invalid reading",
                "invalid data",
            ),
            (
                UnavailableReason::SourceError,
                "Source error",
                "source failed",
            ),
            (UnavailableReason::TimedOut, "Timed out", "timed out"),
        ];
        for (reason, value, description) in cases {
            let mut snapshot = Snapshot::default();
            snapshot.gpu.usage_percent = Reading::Unavailable(reason);
            let cells = present(&snapshot);
            assert_eq!(cells[4].lines[0].value, value);
            assert!(cells[4].lines[0].description.contains(description));
            assert!(cells[4].lines[0].rail.is_none());
            assert_eq!(cells[0].lines[0].value, "Waiting");
        }
    }

    #[test]
    fn memory_text_omits_total_but_description_and_rail_use_it() {
        let mut snapshot = Snapshot {
            ram: Reading::Value(MemoryAmount {
                used_bytes: 11 * 1024 * 1024 * 1024,
                total_bytes: 32 * 1024 * 1024 * 1024,
            }),
            ..Snapshot::default()
        };
        let ram = &present(&snapshot)[2].lines[0];
        assert_eq!(ram.value, "11.0 GiB");
        assert!(ram.description.contains("32.0 GiB total"));
        assert!(ram.rail.unwrap() > 0.3);
        snapshot.ram = Reading::Value(MemoryAmount {
            used_bytes: 1,
            total_bytes: 0,
        });
        assert!(present(&snapshot)[2].lines[0].rail.is_none());
    }

    #[test]
    fn temperatures_keep_text_without_scale_and_distinguish_critical() {
        let mut snapshot = Snapshot::default();
        snapshot.cpu.temperature = Reading::Value(Temperature {
            millidegrees_celsius: 61_000,
            threshold: None,
            source: "test".into(),
        });
        assert_eq!(present(&snapshot)[1].lines[0].value, "61 °C");
        assert!(present(&snapshot)[1].lines[0].rail.is_none());
        if let Reading::Value(temp) = &mut snapshot.cpu.temperature {
            temp.threshold = Some(TemperatureThreshold {
                millidegrees_celsius: 80_000,
                kind: ThresholdKind::Critical,
            });
        }
        let line = &present(&snapshot)[1].lines[0];
        assert!(line.description.contains("critical limit 80 °C"));
        assert_eq!(line.rail, Some(61.0 / 80.0));
    }

    #[test]
    fn separate_volumes_preserve_reported_available_space() {
        let mut snapshot = Snapshot::default();
        snapshot.storage.primary_volumes = Reading::Value(vec![
            PrimaryVolume {
                paths: vec!["/".into()],
                available_bytes: 20,
                used_bytes: 70,
                total_bytes: 100,
            },
            PrimaryVolume {
                paths: vec!["/home".into()],
                available_bytes: 30,
                used_bytes: 50,
                total_bytes: 100,
            },
        ]);
        let storage = &present(&snapshot)[7];
        assert_eq!(storage.lines.len(), 2);
        assert_eq!(storage.lines[0].value, "/ 20 B");
        assert!(storage.lines[0].description.contains("20 B remaining"));
        assert!(storage.lines[0].description.contains("70 B used"));
        assert_eq!(storage.lines[0].rail, Some(0.7));
        assert_eq!(storage.lines[1].value, "/home 30 B");
    }

    #[test]
    fn shared_and_partial_storage_keep_path_truth_and_suppress_unknown_rail() {
        let mut snapshot = Snapshot::default();
        snapshot.storage.primary_volumes = Reading::Value(vec![PrimaryVolume {
            paths: vec!["/".into(), "/home".into()],
            available_bytes: 10,
            used_bytes: 90,
            total_bytes: 100,
        }]);
        let storage = &present(&snapshot)[7];
        assert_eq!(storage.lines.len(), 1);
        assert_eq!(storage.lines[0].value, "10 B");
        assert!(storage.lines[0].description.contains("/ and /home"));
        assert_eq!(storage.lines[0].rail, Some(0.9));

        snapshot.storage.primary_volumes = Reading::Value(vec![PrimaryVolume {
            paths: vec!["/home".into()],
            available_bytes: 0,
            used_bytes: 10,
            total_bytes: 0,
        }]);
        let partial = &present(&snapshot)[7].lines[0];
        assert_eq!(partial.value, "/home 0 B");
        assert!(partial.description.contains("remaining"));
        assert!(
            partial
                .description
                .contains("10 B used; total capacity unknown")
        );
        assert!(partial.rail.is_none());
    }

    #[test]
    fn row_order_and_unbounded_rates_match_overview() {
        let mut snapshot = Snapshot::default();
        snapshot.network.down_bytes_per_second = Reading::Value(0.0);
        snapshot.storage.read_bytes_per_second = Reading::Value(1024.0 * 1024.0);
        let cells = present(&snapshot);
        assert_eq!(
            cells.iter().map(|cell| cell.label).collect::<Vec<_>>(),
            vec![
                "CPU",
                "CPU temp",
                "RAM",
                "Swap",
                "GPU",
                "GPU temp",
                "VRAM",
                "Storage",
                "/tmp",
                "Inodes",
                "Network down",
                "Network up",
                "Storage read",
                "Storage write",
            ]
        );
        assert_eq!(cells[10].lines[0].value, "0 B/s");
        assert!(cells[10].lines[0].rail.is_none());
        assert_eq!(cells[12].lines[0].value, "1.0 MiB/s");
        assert!(cells[12].lines[0].rail.is_none());
    }

    #[test]
    fn small_nonzero_activity_does_not_look_like_zero() {
        let mut snapshot = Snapshot::default();
        snapshot.cpu.usage_percent = Reading::Value(0.04);
        snapshot.network.down_bytes_per_second = Reading::Value(0.2);
        let cells = present(&snapshot);
        assert_eq!(cells[0].lines[0].value, "<0.1%");
        assert!(cells[0].lines[0].description.contains("0.04 percent"));
        assert_eq!(cells[10].lines[0].value, "<1 B/s");
        snapshot.cpu.usage_percent = Reading::Value(0.5);
        assert_eq!(present(&snapshot)[0].lines[0].value, "0.5%");
    }
    #[test]
    fn tmp_space_and_inodes_show_used_amounts_with_independent_rails() {
        let mut snapshot = Snapshot::default();
        snapshot.storage.tmp = Reading::Value(MemoryAmount {
            used_bytes: 2 * 1024 * 1024 * 1024,
            total_bytes: 8 * 1024 * 1024 * 1024,
        });
        snapshot.storage.tmp_inodes = Reading::Value(InodeAmount {
            used: 2500,
            total: 10_000,
        });
        let cells = present(&snapshot);
        assert_eq!(cells[8].label, "/tmp");
        assert_eq!(cells[8].lines[0].value, "2.0 GiB");
        assert_eq!(cells[8].lines[0].rail, Some(0.25));
        assert_eq!(cells[9].label, "Inodes");
        assert_eq!(cells[9].lines[0].value, "2.5K");
        assert_eq!(cells[9].lines[0].rail, Some(0.25));
        assert_eq!(
            cells[9].lines[0].description,
            "/tmp inodes: 2500 used of 10000 total"
        );

        snapshot.storage.tmp_inodes = Reading::Unavailable(UnavailableReason::Missing);
        let cells = present(&snapshot);
        assert_eq!(cells[8].lines[0].value, "2.0 GiB");
        assert_eq!(cells[9].lines[0].value, "Unavailable");
        assert!(cells[9].lines[0].rail.is_none());
    }

    #[test]
    fn inode_zero_full_and_invalid_values_remain_distinct() {
        for (used, total, text, rail) in [
            (0, 100, "0", Some(0.0)),
            (100, 100, "100", Some(1.0)),
            (1, 0, "Invalid reading", None),
            (101, 100, "Invalid reading", None),
        ] {
            let line = inodes(&Reading::Value(InodeAmount { used, total }));
            assert_eq!(line.value, text);
            assert_eq!(line.rail, rail);
        }
    }
}
