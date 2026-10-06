use super::{
    HostPatch, MemoryAmount, Patch, Provider, ProviderKind, Reading, StopToken, Temperature,
    TemperatureThreshold, ThresholdKind, UnavailableReason,
};
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub trait HostProbe: Send + 'static {
    fn read_stat(&mut self) -> io::Result<String>;
    fn read_meminfo(&mut self) -> io::Result<String>;
}

pub struct LinuxHostProbe;

impl HostProbe for LinuxHostProbe {
    fn read_stat(&mut self) -> io::Result<String> {
        fs::read_to_string("/proc/stat")
    }

    fn read_meminfo(&mut self) -> io::Result<String> {
        fs::read_to_string("/proc/meminfo")
    }
}

#[derive(Clone, Copy)]
struct CpuCounters {
    fields: [u64; 8],
}

impl CpuCounters {
    fn parse(stat: &str) -> Option<Self> {
        let mut parts = stat
            .lines()
            .find(|line| line.starts_with("cpu "))?
            .split_whitespace();
        if parts.next()? != "cpu" {
            return None;
        }
        let mut fields = [0; 8];
        for field in &mut fields {
            *field = parts.next()?.parse().ok()?;
        }
        // Guest and guest_nice follow these fields, but are already counted in
        // user and nice. We deliberately do not add them again.
        Some(Self { fields })
    }

    fn usage_since(&self, previous: &Self) -> Option<f64> {
        let mut deltas = [0_u64; 8];
        for (index, (current, prior)) in self.fields.iter().zip(previous.fields).enumerate() {
            deltas[index] = current.checked_sub(prior)?;
        }
        let busy: u128 = [0, 1, 2, 5, 6].iter().map(|&i| u128::from(deltas[i])).sum();
        let inactive: u128 = [3, 4, 7].iter().map(|&i| u128::from(deltas[i])).sum();
        let total = busy + inactive;
        (total > 0).then_some((busy as f64 / total as f64) * 100.0)
    }
}

pub struct HostSampler<P: HostProbe> {
    probe: P,
    previous_cpu: Option<(CpuCounters, Duration)>,
}

impl<P: HostProbe> HostSampler<P> {
    pub fn new(probe: P) -> Self {
        Self {
            probe,
            previous_cpu: None,
        }
    }

    /// One synchronous host sample. This returns only the fields owned by the
    /// host provider, so it cannot overwrite temperature or future providers.
    pub fn tick(&mut self, elapsed: Duration) -> HostPatch {
        let cpu_usage_percent = match self.probe.read_stat() {
            Ok(stat) => match CpuCounters::parse(&stat) {
                Some(current) => {
                    let value = match self.previous_cpu {
                        Some((prior, prior_at))
                            if elapsed > prior_at
                                && elapsed - prior_at <= Duration::from_secs(10) =>
                        {
                            current
                                .usage_since(&prior)
                                .map(Reading::Value)
                                .unwrap_or(Reading::Unavailable(UnavailableReason::InvalidData))
                        }
                        Some(_) => Reading::Unavailable(UnavailableReason::InvalidData),
                        None => Reading::Unavailable(UnavailableReason::WarmingUp),
                    };
                    self.previous_cpu = Some((current, elapsed));
                    value
                }
                None => {
                    self.previous_cpu = None;
                    Reading::Unavailable(UnavailableReason::InvalidData)
                }
            },
            Err(error) => {
                self.previous_cpu = None;
                Reading::Unavailable(reason_for_io(&error))
            }
        };
        let (ram, swap) = match self.probe.read_meminfo() {
            Ok(meminfo) => (
                parse_memory_amount(&meminfo, "MemTotal", "MemAvailable"),
                parse_memory_amount(&meminfo, "SwapTotal", "SwapFree"),
            ),
            Err(error) => {
                let reason = reason_for_io(&error);
                (Reading::Unavailable(reason), Reading::Unavailable(reason))
            }
        };
        HostPatch {
            cpu_usage_percent,
            ram,
            swap,
        }
    }
}

impl<P: HostProbe> Provider for HostSampler<P> {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Host
    }

    fn sample(&mut self, elapsed: Duration, _stop: &StopToken) -> Patch {
        Patch::Host(self.tick(elapsed))
    }
}

fn parse_memory_amount(
    text: &str,
    total_name: &str,
    available_name: &str,
) -> Reading<MemoryAmount> {
    let total = parse_meminfo_value(text, total_name);
    let available = parse_meminfo_value(text, available_name);
    match (total, available) {
        (Some(total_kib), Some(available_kib)) => {
            let values = total_kib
                .checked_sub(available_kib)
                .and_then(|used| Some((used.checked_mul(1024)?, total_kib.checked_mul(1024)?)));
            match values {
                Some((used_bytes, total_bytes)) => Reading::Value(MemoryAmount {
                    used_bytes,
                    total_bytes,
                }),
                None => Reading::Unavailable(UnavailableReason::InvalidData),
            }
        }
        _ => Reading::Unavailable(UnavailableReason::InvalidData),
    }
}

fn parse_meminfo_value(text: &str, name: &str) -> Option<u64> {
    for line in text.lines() {
        let Some((key, rest)) = line.split_once(':') else {
            continue;
        };
        if key == name {
            let mut parts = rest.split_whitespace();
            let value = parts.next()?.parse().ok()?;
            return (parts.next() == Some("kB") && parts.next().is_none()).then_some(value);
        }
    }
    None
}

fn reason_for_io(error: &io::Error) -> UnavailableReason {
    match error.kind() {
        io::ErrorKind::NotFound => UnavailableReason::Missing,
        io::ErrorKind::PermissionDenied => UnavailableReason::PermissionDenied,
        _ => UnavailableReason::SourceError,
    }
}

pub struct TemperatureProvider {
    executable: PathBuf,
    timeout: Duration,
    unreaped: Option<Child>,
}

impl TemperatureProvider {
    pub fn system() -> io::Result<Self> {
        Ok(Self::with_executable(
            std::env::current_exe()?,
            Duration::from_millis(400),
        ))
    }

    /// The executable must support `--internal-temperature-probe` and write the
    /// small versioned line produced by `run_temperature_helper`.
    pub fn with_executable(executable: PathBuf, timeout: Duration) -> Self {
        Self {
            executable,
            timeout,
            unreaped: None,
        }
    }

    fn reap_previous(&mut self) -> bool {
        let Some(child) = self.unreaped.as_mut() else {
            return true;
        };
        let _ = child.kill();
        match child.try_wait() {
            Ok(Some(_)) => {
                self.unreaped = None;
                true
            }
            _ => false,
        }
    }

    fn read(&mut self, stop: &StopToken) -> Reading<Temperature> {
        if !self.reap_previous() {
            return Reading::Unavailable(UnavailableReason::TimedOut);
        }
        let mut child = match Command::new(&self.executable)
            .arg("--internal-temperature-probe")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => child,
            Err(error) => return Reading::Unavailable(reason_for_io(&error)),
        };
        let started = Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    if !status.success() {
                        return Reading::Unavailable(UnavailableReason::SourceError);
                    }
                    let mut output = String::new();
                    let read = child
                        .stdout
                        .take()
                        .map(|stdout| stdout.take(4096).read_to_string(&mut output))
                        .unwrap_or_else(|| Err(io::Error::other("missing helper output")));
                    if read.is_err() {
                        return Reading::Unavailable(UnavailableReason::SourceError);
                    }
                    return parse_temperature_line(&output)
                        .unwrap_or(Reading::Unavailable(UnavailableReason::InvalidData));
                }
                Ok(None) => {
                    if stop.is_stopped() || started.elapsed() >= self.timeout {
                        let _ = child.kill();
                        // Reap a normally killable helper without an unbounded
                        // wait. A driver stuck in uninterruptible kernel sleep
                        // cannot be reaped promptly; retain only one handle.
                        let reap_start = Instant::now();
                        loop {
                            if matches!(child.try_wait(), Ok(Some(_))) {
                                break;
                            }
                            if reap_start.elapsed() >= Duration::from_millis(100) {
                                self.unreaped = Some(child);
                                break;
                            }
                            thread::sleep(Duration::from_millis(5));
                        }
                        return Reading::Unavailable(UnavailableReason::TimedOut);
                    }
                    thread::sleep(Duration::from_millis(5));
                }
                Err(_) => {
                    let _ = child.kill();
                    self.unreaped = Some(child);
                    return Reading::Unavailable(UnavailableReason::SourceError);
                }
            }
        }
    }
}

impl Provider for TemperatureProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Temperature
    }

    fn sample(&mut self, _elapsed: Duration, stop: &StopToken) -> Patch {
        Patch::Temperature(self.read(stop))
    }
}

impl Drop for TemperatureProvider {
    fn drop(&mut self) {
        if let Some(child) = self.unreaped.as_mut() {
            let _ = child.kill();
            let _ = child.try_wait();
        }
    }
}

fn parse_temperature_line(line: &str) -> Option<Reading<Temperature>> {
    let line = line.trim_end_matches(['\n', '\r']);
    if line == "U" {
        return Some(Reading::Unavailable(UnavailableReason::Missing));
    }
    let mut parts = line.splitn(5, '\t');
    if parts.next()? != "V1" {
        return None;
    }
    let millidegrees_celsius = parts.next()?.parse().ok()?;
    let threshold_kind = parts.next()?;
    let threshold_milli = parts.next()?;
    let source = parts.next()?.to_string();
    if source.is_empty() || source.len() > 1000 {
        return None;
    }
    let threshold = match threshold_kind {
        "none" if threshold_milli == "-" => None,
        "max" => Some(TemperatureThreshold {
            millidegrees_celsius: threshold_milli
                .parse::<i64>()
                .ok()?
                .checked_sub(0)
                .filter(|value| *value > 0)?,
            kind: ThresholdKind::TargetOrMax,
        }),
        "crit" => Some(TemperatureThreshold {
            millidegrees_celsius: threshold_milli
                .parse::<i64>()
                .ok()?
                .checked_sub(0)
                .filter(|value| *value > 0)?,
            kind: ThresholdKind::Critical,
        }),
        _ => return None,
    };
    Some(Reading::Value(Temperature {
        millidegrees_celsius,
        threshold,
        source,
    }))
}

/// Internal helper entry point. The parent runs this in a separate process so
/// an unresponsive hwmon driver does not block the host worker or window close.
pub fn run_temperature_helper() -> io::Result<()> {
    let reading = scan_cpu_temperature(Path::new("/sys/class/hwmon"))?;
    let mut stdout = io::stdout().lock();
    match reading {
        Some(value) => {
            let (kind, threshold) = match value.threshold {
                Some(TemperatureThreshold {
                    millidegrees_celsius,
                    kind: ThresholdKind::TargetOrMax,
                }) => ("max", millidegrees_celsius.to_string()),
                Some(TemperatureThreshold {
                    millidegrees_celsius,
                    kind: ThresholdKind::Critical,
                }) => ("crit", millidegrees_celsius.to_string()),
                None => ("none", "-".to_string()),
            };
            writeln!(
                stdout,
                "V1\t{}\t{kind}\t{threshold}\t{}",
                value.millidegrees_celsius, value.source
            )?;
        }
        None => writeln!(stdout, "U")?,
    }
    Ok(())
}

fn scan_cpu_temperature(root: &Path) -> io::Result<Option<Temperature>> {
    let mut chips: Vec<_> = fs::read_dir(root)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    chips.sort();
    let mut candidates = Vec::new();
    for chip in chips {
        let Ok(driver) = fs::read_to_string(chip.join("name")) else {
            continue;
        };
        let driver = driver.trim();
        let Ok(entries) = fs::read_dir(&chip) else {
            continue;
        };
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let Some(file_name) = file_name.to_str() else {
                continue;
            };
            let Some(index) = file_name
                .strip_prefix("temp")
                .and_then(|s| s.strip_suffix("_label"))
            else {
                continue;
            };
            if index.parse::<u32>().is_err() {
                continue;
            }
            let Ok(label) = fs::read_to_string(entry.path()) else {
                continue;
            };
            let label = label.trim();
            let eligible = (driver == "coretemp"
                && label
                    .strip_prefix("Package id ")
                    .is_some_and(|n| n.parse::<u32>().is_ok()))
                || (driver == "k10temp" && label == "Tdie")
                || (driver.starts_with("peci_cputemp") && label == "Die");
            if !eligible {
                continue;
            }
            let prefix = format!("temp{index}");
            let fault_path = chip.join(format!("{prefix}_fault"));
            match fs::read_to_string(&fault_path) {
                Ok(fault) if fault.trim() != "0" => continue,
                Err(error) if error.kind() != io::ErrorKind::NotFound => continue,
                _ => {}
            }
            let Ok(input) = fs::read_to_string(chip.join(format!("{prefix}_input"))) else {
                continue;
            };
            let Ok(millidegrees_celsius) = input.trim().parse::<i64>() else {
                continue;
            };
            let max = read_positive_i64(&chip.join(format!("{prefix}_max")));
            let critical = read_positive_i64(&chip.join(format!("{prefix}_crit")));
            let threshold = max
                .map(|millidegrees_celsius| TemperatureThreshold {
                    millidegrees_celsius,
                    kind: ThresholdKind::TargetOrMax,
                })
                .or_else(|| {
                    critical.map(|millidegrees_celsius| TemperatureThreshold {
                        millidegrees_celsius,
                        kind: ThresholdKind::Critical,
                    })
                });
            candidates.push((
                Temperature {
                    millidegrees_celsius,
                    threshold,
                    source: format!("{driver} {label}"),
                },
                entry.path(),
            ));
        }
    }
    candidates.sort_by(|(a, a_path), (b, b_path)| {
        b.millidegrees_celsius
            .cmp(&a.millidegrees_celsius)
            .then_with(|| a.source.cmp(&b.source))
            .then_with(|| a_path.cmp(b_path))
    });
    Ok(candidates.into_iter().next().map(|(value, _)| value))
}

fn read_positive_i64(path: &Path) -> Option<i64> {
    fs::read_to_string(path)
        .ok()?
        .trim()
        .parse::<i64>()
        .ok()
        .filter(|value| *value > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_fields_are_independent() {
        let text = "MemTotal: 100 kB\nMemAvailable: 40 kB\nSwapTotal: 0 kB\nSwapFree: 0 kB\n";
        assert_eq!(
            parse_memory_amount(text, "MemTotal", "MemAvailable"),
            Reading::Value(MemoryAmount {
                used_bytes: 60 * 1024,
                total_bytes: 100 * 1024
            })
        );
        assert_eq!(
            parse_memory_amount(text, "SwapTotal", "SwapFree"),
            Reading::Value(MemoryAmount {
                used_bytes: 0,
                total_bytes: 0
            })
        );
    }

    fn fixture_root(label: &str) -> PathBuf {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "sys-mon-hwmon-{label}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[allow(clippy::too_many_arguments)]
    fn channel(
        root: &Path,
        chip: &str,
        driver: &str,
        number: u32,
        label: &str,
        input: i64,
        max: Option<i64>,
        crit: Option<i64>,
    ) {
        let path = root.join(chip);
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("name"), driver).unwrap();
        fs::write(path.join(format!("temp{number}_label")), label).unwrap();
        fs::write(path.join(format!("temp{number}_input")), input.to_string()).unwrap();
        if let Some(max) = max {
            fs::write(path.join(format!("temp{number}_max")), max.to_string()).unwrap();
        }
        if let Some(crit) = crit {
            fs::write(path.join(format!("temp{number}_crit")), crit.to_string()).unwrap();
        }
    }

    #[test]
    fn attributed_temperature_ignores_control_and_unrelated_sensors() {
        let root = fixture_root("attribution");
        channel(
            &root,
            "hwmon0",
            "coretemp",
            1,
            "Package id 0",
            47_000,
            Some(80_000),
            Some(100_000),
        );
        channel(
            &root,
            "hwmon1",
            "k10temp",
            1,
            "Tctl",
            95_000,
            Some(90_000),
            None,
        );
        channel(
            &root,
            "hwmon1",
            "k10temp",
            2,
            "Tdie",
            53_000,
            None,
            Some(95_000),
        );
        channel(
            &root,
            "hwmon2",
            "peci_cputemp.cpu0",
            1,
            "Die",
            49_000,
            Some(78_000),
            None,
        );
        channel(
            &root,
            "hwmon3",
            "nvme",
            1,
            "Composite",
            90_000,
            Some(100_000),
            None,
        );
        let chosen = scan_cpu_temperature(&root).unwrap().unwrap();
        assert_eq!(chosen.millidegrees_celsius, 53_000);
        assert_eq!(chosen.source, "k10temp Tdie");
        assert_eq!(
            chosen.threshold,
            Some(TemperatureThreshold {
                millidegrees_celsius: 95_000,
                kind: ThresholdKind::Critical
            })
        );
        fs::write(root.join("hwmon1/temp2_fault"), "1").unwrap();
        let chosen = scan_cpu_temperature(&root).unwrap().unwrap();
        assert_eq!(chosen.source, "peci_cputemp.cpu0 Die");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn hottest_package_uses_its_own_scale_and_invalid_fault_is_unavailable() {
        let root = fixture_root("packages");
        channel(
            &root,
            "hwmon0",
            "coretemp",
            1,
            "Package id 0",
            47_000,
            Some(80_000),
            Some(100_000),
        );
        channel(
            &root,
            "hwmon0",
            "coretemp",
            2,
            "Package id 1",
            61_000,
            None,
            Some(96_000),
        );
        let chosen = scan_cpu_temperature(&root).unwrap().unwrap();
        assert_eq!(chosen.source, "coretemp Package id 1");
        assert_eq!(chosen.threshold.unwrap().kind, ThresholdKind::Critical);
        fs::write(root.join("hwmon0/temp2_fault"), "bad").unwrap();
        let chosen = scan_cpu_temperature(&root).unwrap().unwrap();
        assert_eq!(chosen.source, "coretemp Package id 0");
        assert_eq!(
            chosen.threshold,
            Some(TemperatureThreshold {
                millidegrees_celsius: 80_000,
                kind: ThresholdKind::TargetOrMax
            })
        );
        fs::remove_file(root.join("hwmon0/temp1_max")).unwrap();
        let chosen = scan_cpu_temperature(&root).unwrap().unwrap();
        assert_eq!(chosen.threshold.unwrap().kind, ThresholdKind::Critical);
        fs::remove_file(root.join("hwmon0/temp1_crit")).unwrap();
        assert!(
            scan_cpu_temperature(&root)
                .unwrap()
                .unwrap()
                .threshold
                .is_none()
        );
        fs::remove_file(root.join("hwmon0/temp1_input")).unwrap();
        assert!(scan_cpu_temperature(&root).unwrap().is_none());
        fs::write(root.join("hwmon0/temp1_input"), "51000").unwrap();
        assert_eq!(
            scan_cpu_temperature(&root)
                .unwrap()
                .unwrap()
                .millidegrees_celsius,
            51_000
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn repeated_sensor_arrival_and_removal_never_reuses_an_old_reading() {
        let root = fixture_root("churn");
        for cycle in 0..128 {
            let value = 40_000 + cycle;
            channel(
                &root,
                "hwmon0",
                "coretemp",
                1,
                "Package id 0",
                value,
                None,
                None,
            );
            assert_eq!(
                scan_cpu_temperature(&root)
                    .unwrap()
                    .unwrap()
                    .millidegrees_celsius,
                value
            );
            fs::remove_dir_all(root.join("hwmon0")).unwrap();
            assert!(scan_cpu_temperature(&root).unwrap().is_none());
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn guest_fields_are_not_double_counted() {
        let first = CpuCounters::parse("cpu 100 0 0 100 0 0 0 0 50 0\n").unwrap();
        let second = CpuCounters::parse("cpu 150 0 0 150 0 0 0 0 100 0\n").unwrap();
        assert_eq!(second.usage_since(&first), Some(50.0));
    }
}
