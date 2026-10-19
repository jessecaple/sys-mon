//! Nonoverlapping display averages; raw telemetry remains unchanged.

use std::collections::VecDeque;
use std::time::Duration;

use crate::telemetry::{MemoryAmount, Reading, Snapshot, Temperature};

const WINDOW: Duration = Duration::from_secs(3);
const MAX_SAMPLES: usize = 3;

#[derive(Default)]
pub(crate) struct SnapshotAverager {
    samples: VecDeque<(Duration, Snapshot)>,
    window_start: Duration,
}

impl SnapshotAverager {
    /// Collect on the existing one-second UI timer, publishing every three seconds.
    pub(crate) fn update(&mut self, now: Duration, snapshot: Snapshot) -> Option<Snapshot> {
        if now < self.window_start || self.samples.back().is_some_and(|(time, _)| now < *time) {
            self.samples.clear();
            self.window_start = now;
        }
        // Discard pre-suspend history and bound memory even if called too often.
        while self
            .samples
            .front()
            .is_some_and(|(time, _)| now.saturating_sub(*time) >= WINDOW)
            || self.samples.len() >= MAX_SAMPLES
        {
            self.samples.pop_front();
        }
        self.samples.push_back((now, snapshot.clone()));
        if now.saturating_sub(self.window_start) < WINDOW {
            return None;
        }

        let mut display = snapshot;
        display.cpu.usage_percent = self.average(|s| &s.cpu.usage_percent);
        display.cpu.temperature = self.average(|s| &s.cpu.temperature);
        display.ram = self.average(|s| &s.ram);
        display.swap = self.average(|s| &s.swap);
        display.gpu.usage_percent = self.average(|s| &s.gpu.usage_percent);
        display.gpu.temperature = self.average(|s| &s.gpu.temperature);
        display.gpu.vram = self.average(|s| &s.gpu.vram);
        display.network.down_bytes_per_second = self.average(|s| &s.network.down_bytes_per_second);
        display.network.up_bytes_per_second = self.average(|s| &s.network.up_bytes_per_second);
        display.storage.read_bytes_per_second = self.average(|s| &s.storage.read_bytes_per_second);
        display.storage.write_bytes_per_second =
            self.average(|s| &s.storage.write_bytes_per_second);
        // Capacity is sampled less often; keep its latest actual value and paths.
        self.samples.clear();
        self.window_start = now;
        Some(display)
    }

    fn average<T: Sample>(&self, field: impl Fn(&Snapshot) -> &Reading<T>) -> Reading<T> {
        let latest = field(&self.samples.back().unwrap().1);
        let Reading::Value(current) = latest else {
            return latest.clone();
        };
        if !current.valid() {
            return latest.clone();
        }
        let mut mean = 0.0;
        let mut count = 0;
        for (_, snapshot) in self.samples.iter().rev() {
            match field(snapshot) {
                Reading::Value(sample) if sample.valid() && current.compatible(sample) => {
                    count += 1;
                    mean += (sample.number() - mean) / f64::from(count);
                }
                // Never bridge a failure, sensor change, or capacity change.
                _ => break,
            }
        }
        Reading::Value(current.with_number(mean))
    }
}

trait Sample: Clone {
    fn number(&self) -> f64;
    fn with_number(&self, number: f64) -> Self;
    fn compatible(&self, _other: &Self) -> bool {
        true
    }
    fn valid(&self) -> bool {
        true
    }
}

impl Sample for f64 {
    fn number(&self) -> f64 {
        *self
    }
    fn with_number(&self, number: f64) -> Self {
        number
    }
    fn valid(&self) -> bool {
        self.is_finite() && *self >= 0.0
    }
}

impl Sample for MemoryAmount {
    fn number(&self) -> f64 {
        self.used_bytes as f64
    }
    fn with_number(&self, number: f64) -> Self {
        Self {
            used_bytes: number.round() as u64,
            ..*self
        }
    }
    fn compatible(&self, other: &Self) -> bool {
        self.total_bytes == other.total_bytes
    }
}

impl Sample for Temperature {
    fn number(&self) -> f64 {
        self.millidegrees_celsius as f64
    }
    fn with_number(&self, number: f64) -> Self {
        Self {
            millidegrees_celsius: number.round() as i64,
            ..self.clone()
        }
    }
    fn compatible(&self, other: &Self) -> bool {
        self.source == other.source && self.threshold == other.threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presentation::present;
    use crate::telemetry::{PrimaryVolume, UnavailableReason};

    fn snapshot(value: f64) -> Snapshot {
        let mut snapshot = Snapshot::default();
        snapshot.cpu.usage_percent = Reading::Value(value);
        snapshot.gpu.usage_percent = Reading::Value(value);
        snapshot.network.down_bytes_per_second = Reading::Value(value);
        snapshot.network.up_bytes_per_second = Reading::Value(value);
        snapshot.storage.read_bytes_per_second = Reading::Value(value);
        snapshot.storage.write_bytes_per_second = Reading::Value(value);
        snapshot
    }

    #[test]
    fn updates_every_three_seconds_using_only_the_current_interval() {
        let mut averager = SnapshotAverager::default();
        let values = [0.0, 90.0, 0.0, 0.0, 0.0, 0.0, 60.0, 60.0, 60.0];
        for (index, value) in values.into_iter().enumerate() {
            let second = index + 1;
            let shown = averager.update(Duration::from_secs(second as u64), snapshot(value));
            if second % 3 != 0 {
                assert!(
                    shown.is_none(),
                    "must keep displayed values steady between updates"
                );
                continue;
            }
            let shown = shown.unwrap();
            let expected = [30.0, 0.0, 60.0][second / 3 - 1];
            assert_eq!(shown.cpu.usage_percent, Reading::Value(expected));
            assert_eq!(shown.gpu.usage_percent, Reading::Value(expected));
            assert_eq!(
                shown.network.down_bytes_per_second,
                Reading::Value(expected)
            );
            assert_eq!(shown.network.up_bytes_per_second, Reading::Value(expected));
            assert_eq!(
                shown.storage.read_bytes_per_second,
                Reading::Value(expected)
            );
            assert_eq!(
                shown.storage.write_bytes_per_second,
                Reading::Value(expected)
            );
            assert_eq!(present(&shown)[0].lines[0].rail, Some(expected / 100.0));
        }
    }

    #[test]
    fn latest_failure_is_shown_at_the_next_display_update() {
        let mut averager = SnapshotAverager::default();
        averager.update(Duration::from_secs(1), snapshot(90.0));
        averager.update(Duration::from_secs(2), snapshot(90.0));
        let mut failed = snapshot(0.0);
        failed.cpu.usage_percent = Reading::Unavailable(UnavailableReason::TimedOut);
        let shown = averager.update(Duration::from_secs(3), failed).unwrap();
        assert_eq!(present(&shown)[0].lines[0].value, "Timed out");
        assert_eq!(shown.gpu.usage_percent, Reading::Value(60.0));
    }

    #[test]
    fn recovery_within_an_interval_does_not_average_across_the_failure() {
        let mut averager = SnapshotAverager::default();
        averager.update(Duration::from_secs(1), snapshot(90.0));
        let mut failed = snapshot(0.0);
        failed.cpu.usage_percent = Reading::Unavailable(UnavailableReason::TimedOut);
        averager.update(Duration::from_secs(2), failed);
        let shown = averager
            .update(Duration::from_secs(3), snapshot(0.0))
            .unwrap();
        assert_eq!(shown.cpu.usage_percent, Reading::Value(0.0));
        assert_eq!(shown.gpu.usage_percent, Reading::Value(30.0));
    }

    #[test]
    fn invalid_data_is_not_hidden_or_carried_into_recovery() {
        for invalid in [f64::NAN, f64::INFINITY, -1.0] {
            let mut averager = SnapshotAverager::default();
            averager.update(Duration::from_secs(1), snapshot(90.0));
            let shown = averager
                .update(Duration::from_secs(3), snapshot(invalid))
                .unwrap();
            assert_eq!(present(&shown)[0].lines[0].value, "Invalid reading");
            let recovered = averager
                .update(Duration::from_secs(6), snapshot(0.0))
                .unwrap();
            assert_eq!(recovered.cpu.usage_percent, Reading::Value(0.0));
        }
    }

    #[test]
    fn elapsed_gap_and_clock_reset_discard_old_samples() {
        let mut averager = SnapshotAverager::default();
        averager.update(Duration::from_secs(1), snapshot(90.0));
        let shown = averager
            .update(Duration::from_secs(30), snapshot(0.0))
            .unwrap();
        assert_eq!(shown.cpu.usage_percent, Reading::Value(0.0));
        assert!(averager.update(Duration::ZERO, snapshot(12.0)).is_none());
        averager.update(Duration::from_secs(1), snapshot(12.0));
        averager.update(Duration::from_secs(2), snapshot(12.0));
        let shown = averager
            .update(Duration::from_secs(3), snapshot(12.0))
            .unwrap();
        assert_eq!(shown.cpu.usage_percent, Reading::Value(12.0));
    }

    #[test]
    fn memory_and_temperature_average_without_blending_changed_sources() {
        let mut averager = SnapshotAverager::default();
        let mut input = snapshot(0.0);
        input.ram = Reading::Value(MemoryAmount {
            used_bytes: 100,
            total_bytes: 1000,
        });
        input.swap = input.ram.clone();
        input.gpu.vram = input.ram.clone();
        input.cpu.temperature = Reading::Value(Temperature {
            millidegrees_celsius: 40_000,
            threshold: None,
            source: "package".into(),
        });
        input.gpu.temperature = input.cpu.temperature.clone();
        averager.update(Duration::from_secs(1), input.clone());
        input.ram = Reading::Value(MemoryAmount {
            used_bytes: 300,
            total_bytes: 1000,
        });
        input.swap = input.ram.clone();
        input.gpu.vram = input.ram.clone();
        if let Reading::Value(temp) = &mut input.cpu.temperature {
            temp.millidegrees_celsius = 60_000;
        }
        input.gpu.temperature = input.cpu.temperature.clone();
        let shown = averager
            .update(Duration::from_secs(3), input.clone())
            .unwrap();
        let expected = Reading::Value(MemoryAmount {
            used_bytes: 200,
            total_bytes: 1000,
        });
        assert_eq!(shown.ram, expected);
        assert_eq!(shown.swap, expected);
        assert_eq!(shown.gpu.vram, expected);
        assert_eq!(present(&shown)[1].lines[0].value, "50 °C");
        assert_eq!(shown.gpu.temperature, shown.cpu.temperature);
        averager.update(Duration::from_secs(4), input.clone());
        if let Reading::Value(amount) = &mut input.ram {
            amount.total_bytes = 2000;
        }
        if let Reading::Value(temp) = &mut input.cpu.temperature {
            temp.source = "new sensor".into();
        }
        input.storage.primary_volumes = Reading::Value(vec![PrimaryVolume {
            paths: vec!["/".into()],
            available_bytes: 500,
            used_bytes: 1500,
            total_bytes: 2000,
        }]);
        let shown = averager
            .update(Duration::from_secs(6), input.clone())
            .unwrap();
        assert_eq!(shown.ram, input.ram);
        assert_eq!(shown.cpu.temperature, input.cpu.temperature);
        assert_eq!(shown.storage.primary_volumes, input.storage.primary_volumes);
    }

    #[test]
    fn retained_state_is_bounded_and_cleared_after_publication() {
        let mut averager = SnapshotAverager::default();
        for _ in 0..10_000 {
            assert!(averager.update(Duration::ZERO, snapshot(42.0)).is_none());
        }
        assert_eq!(averager.samples.len(), MAX_SAMPLES);
        averager
            .update(Duration::from_secs(3), snapshot(42.0))
            .unwrap();
        assert!(averager.samples.is_empty());
    }
}
