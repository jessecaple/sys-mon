//! Optional GPU sources. Vendor reads run in a killable child process.

mod intel;

use super::{
    GpuPatch, MemoryAmount, Patch, Provider, ProviderKind, Reading, StopToken, Temperature,
    TemperatureThreshold, ThresholdKind, UnavailableReason,
};
use nvml_wrapper::Nvml;
use nvml_wrapper::enum_wrappers::device::{
    TemperatureSensor, TemperatureThreshold as NvmlThreshold,
};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const MAX_DEVICES: usize = 16;
const MAX_OUTPUT: u64 = 65_536;

fn unavailable(reason: UnavailableReason) -> GpuPatch {
    GpuPatch {
        usage_percent: Reading::Unavailable(reason),
        vram: Reading::Unavailable(reason),
        temperature: Reading::Unavailable(reason),
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct DeviceFrame {
    vendor: Vendor,
    pci: Option<String>,
    identity: String,
    ordinal: usize,
    patch: GpuPatch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
enum Vendor {
    Nvidia,
    Amd,
    Intel,
}

fn selected_patch(mut frames: Vec<DeviceFrame>) -> GpuPatch {
    frames.retain(|frame| frame.identity.len() <= 128);
    frames.sort_by(|a, b| {
        (
            a.vendor,
            a.pci.is_none(),
            a.pci.as_deref(),
            &a.identity,
            a.ordinal,
        )
            .cmp(&(
                b.vendor,
                b.pci.is_none(),
                b.pci.as_deref(),
                &b.identity,
                b.ordinal,
            ))
    });
    let Some(frame) = frames.into_iter().next() else {
        return unavailable(UnavailableReason::Missing);
    };
    validate_patch(frame.patch)
}

fn validate_patch(mut patch: GpuPatch) -> GpuPatch {
    if let Reading::Value(percent) = patch.usage_percent
        && (!percent.is_finite() || !(0.0..=100.0).contains(&percent))
    {
        patch.usage_percent = Reading::Unavailable(UnavailableReason::InvalidData);
    }
    if let Reading::Value(amount) = patch.vram
        && (amount.total_bytes == 0 || amount.used_bytes > amount.total_bytes)
    {
        patch.vram = Reading::Unavailable(UnavailableReason::InvalidData);
    }
    if let Reading::Value(value) = &patch.temperature
        && (value.source.is_empty()
            || value.source.len() > 128
            || !(-100_000..=250_000).contains(&value.millidegrees_celsius))
    {
        patch.temperature = Reading::Unavailable(UnavailableReason::InvalidData);
    }
    patch
}

pub struct GpuProvider {
    executable: PathBuf,
    timeout: Duration,
    interval: Duration,
    unreaped: Option<Child>,
}

impl GpuProvider {
    pub fn system() -> io::Result<Self> {
        Ok(Self::with_executable(
            std::env::current_exe()?,
            Duration::from_millis(650),
        ))
    }

    /// The executable must support `--internal-gpu-probe` and emit a JSON array
    /// of current DeviceFrame values. This is also the process-boundary test seam.
    pub fn with_executable(executable: PathBuf, timeout: Duration) -> Self {
        Self {
            executable,
            timeout,
            interval: Duration::from_secs(1),
            unreaped: None,
        }
    }

    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
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

    fn sample_child(&mut self, stop: &StopToken) -> GpuPatch {
        if !self.reap_previous() {
            return unavailable(UnavailableReason::TimedOut);
        }
        let mut child = match Command::new(&self.executable)
            .arg("--internal-gpu-probe")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => child,
            Err(error) => return unavailable(io_reason(&error)),
        };
        let started = Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    if !status.success() {
                        return unavailable(UnavailableReason::SourceError);
                    }
                    let mut output = Vec::new();
                    let read = child
                        .stdout
                        .take()
                        .map(|stdout| stdout.take(MAX_OUTPUT + 1).read_to_end(&mut output))
                        .unwrap_or_else(|| Err(io::Error::other("missing GPU helper output")));
                    if read.is_err() || output.len() as u64 > MAX_OUTPUT {
                        return unavailable(UnavailableReason::InvalidData);
                    }
                    let frames: Vec<DeviceFrame> =
                        match serde_json::from_slice::<Vec<DeviceFrame>>(&output) {
                            Ok(frames) if frames.len() <= MAX_DEVICES * 3 => frames,
                            _ => return unavailable(UnavailableReason::InvalidData),
                        };
                    return selected_patch(frames);
                }
                Ok(None) => {
                    if stop.is_stopped() || started.elapsed() >= self.timeout {
                        let _ = child.kill();
                        let reap_started = Instant::now();
                        loop {
                            if matches!(child.try_wait(), Ok(Some(_))) {
                                break;
                            }
                            if reap_started.elapsed() >= Duration::from_millis(100) {
                                self.unreaped = Some(child);
                                break;
                            }
                            thread::sleep(Duration::from_millis(5));
                        }
                        return unavailable(UnavailableReason::TimedOut);
                    }
                    thread::sleep(Duration::from_millis(5));
                }
                Err(_) => {
                    let _ = child.kill();
                    self.unreaped = Some(child);
                    return unavailable(UnavailableReason::SourceError);
                }
            }
        }
    }
}

impl Provider for GpuProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Gpu
    }

    fn sample(&mut self, _elapsed: Duration, stop: &StopToken) -> Patch {
        Patch::Gpu(self.sample_child(stop))
    }

    fn interval(&self) -> Duration {
        self.interval
    }
}

impl Drop for GpuProvider {
    fn drop(&mut self) {
        if let Some(child) = self.unreaped.as_mut() {
            let _ = child.kill();
            let _ = child.try_wait();
        }
    }
}

pub fn run_gpu_helper() -> io::Result<()> {
    let mut frames = scan_nvidia();
    frames.extend(scan_amd(Path::new("/sys/class/drm"), classify_amd));
    frames.extend(intel::scan_intel());
    frames.sort_by_key(|frame| frame.vendor);
    frames.truncate(MAX_DEVICES);
    serde_json::to_writer(io::stdout().lock(), &frames)?;
    io::stdout().flush()
}

fn scan_nvidia() -> Vec<DeviceFrame> {
    let Ok(nvml) = Nvml::init() else {
        return Vec::new();
    };
    let Ok(count) = nvml.device_count() else {
        return Vec::new();
    };
    (0..count.min(MAX_DEVICES as u32))
        .filter_map(|ordinal| {
            let device = nvml.device_by_index(ordinal).ok()?;
            let pci = device
                .pci_info()
                .ok()
                .map(|info| info.bus_id.to_lowercase());
            let identity = device
                .uuid()
                .ok()
                .unwrap_or_else(|| format!("nvidia-{ordinal}"));
            let usage_percent = device
                .utilization_rates()
                .map(|value| Reading::Value(f64::from(value.gpu)))
                .unwrap_or(Reading::Unavailable(UnavailableReason::SourceError));
            let vram = device
                .memory_info()
                .map(|value| {
                    Reading::Value(MemoryAmount {
                        used_bytes: value.used,
                        total_bytes: value.total,
                    })
                })
                .unwrap_or(Reading::Unavailable(UnavailableReason::SourceError));
            let temperature = device
                .temperature(TemperatureSensor::Gpu)
                .map(|celsius| {
                    let threshold = device
                        .temperature_threshold(NvmlThreshold::GpuMax)
                        .ok()
                        .filter(|value| *value > 0)
                        .map(|value| TemperatureThreshold {
                            millidegrees_celsius: i64::from(value) * 1000,
                            kind: ThresholdKind::TargetOrMax,
                        })
                        .or_else(|| {
                            device
                                .temperature_threshold(NvmlThreshold::Slowdown)
                                .ok()
                                .filter(|value| *value > 0)
                                .map(|value| TemperatureThreshold {
                                    millidegrees_celsius: i64::from(value) * 1000,
                                    kind: ThresholdKind::Critical,
                                })
                        });
                    Reading::Value(Temperature {
                        millidegrees_celsius: i64::from(celsius) * 1000,
                        threshold,
                        source: "NVIDIA GPU".into(),
                    })
                })
                .unwrap_or(Reading::Unavailable(UnavailableReason::SourceError));
            Some(DeviceFrame {
                vendor: Vendor::Nvidia,
                pci,
                identity,
                ordinal: ordinal as usize,
                patch: GpuPatch {
                    usage_percent,
                    vram,
                    temperature,
                },
            })
        })
        .collect()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AmdMemoryKind {
    Discrete,
    Integrated,
    Unknown,
}

fn classify_amd(card: &Path) -> AmdMemoryKind {
    // AMD's public DEV_INFO ioctl exposes AMDGPU_IDS_FLAGS_FUSION. Query only
    // the prefix through ids_flags; the kernel honors return_size.
    const DRM_IOCTL_AMDGPU_INFO: libc::c_ulong = 0x4020_6445;
    const AMDGPU_INFO_DEV_INFO: u32 = 0x16;
    const IDS_FLAGS_OFFSET: usize = 136;
    #[repr(C)]
    struct Query {
        return_pointer: u64,
        return_size: u32,
        query: u32,
        payload: [u8; 16],
    }
    let drm_dir = card.join("device/drm");
    let Ok(entries) = fs::read_dir(drm_dir) else {
        return AmdMemoryKind::Unknown;
    };
    let Some(render_name) = entries
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .find(|name| name.starts_with("renderD"))
    else {
        return AmdMemoryKind::Unknown;
    };
    // A fixture may put a regular render node here. Production uses /dev/dri.
    let node = if card.starts_with("/sys/") {
        PathBuf::from("/dev/dri").join(render_name)
    } else {
        card.parent().unwrap_or(Path::new("/")).join(render_name)
    };
    let Ok(file) = File::open(node) else {
        return AmdMemoryKind::Unknown;
    };
    let mut result = [0_u8; IDS_FLAGS_OFFSET + 8];
    let mut query = Query {
        return_pointer: result.as_mut_ptr() as u64,
        return_size: result.len() as u32,
        query: AMDGPU_INFO_DEV_INFO,
        payload: [0; 16],
    };
    // SAFETY: the file is an open DRM render node, query has the public UAPI
    // layout, and return_pointer addresses a live output buffer of return_size.
    let status = unsafe { libc::ioctl(file.as_raw_fd(), DRM_IOCTL_AMDGPU_INFO, &mut query) };
    if status != 0 {
        return AmdMemoryKind::Unknown;
    }
    let flags = u64::from_ne_bytes(result[IDS_FLAGS_OFFSET..].try_into().unwrap());
    if flags & 1 != 0 {
        AmdMemoryKind::Integrated
    } else {
        AmdMemoryKind::Discrete
    }
}

fn scan_amd(root: &Path, classify: impl Fn(&Path) -> AmdMemoryKind) -> Vec<DeviceFrame> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut cards: Vec<_> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.strip_prefix("card").is_some_and(|suffix| {
                        !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit())
                    })
                })
        })
        .collect();
    cards.sort();
    cards
        .into_iter()
        .take(MAX_DEVICES)
        .enumerate()
        .filter_map(|(ordinal, card)| {
            let device = card.join("device");
            let vendor = fs::read_to_string(device.join("vendor")).ok()?;
            if vendor.trim() != "0x1002" {
                return None;
            }
            let driver = fs::read_link(device.join("driver")).ok()?;
            if driver.file_name()?.to_str()? != "amdgpu" {
                return None;
            }
            let pci = fs::canonicalize(&device).ok().and_then(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(str::to_owned)
            });
            let identity = pci.clone().unwrap_or_else(|| card.display().to_string());
            let usage_percent = read_u64(device.join("gpu_busy_percent"))
                .and_then(|value| {
                    (value <= 100)
                        .then_some(Reading::Value(value as f64))
                        .ok_or(UnavailableReason::InvalidData)
                })
                .unwrap_or_else(Reading::Unavailable);
            let vram = match classify(&card) {
                AmdMemoryKind::Discrete => match (
                    read_u64(device.join("mem_info_vram_used")),
                    read_u64(device.join("mem_info_vram_total")),
                ) {
                    (Ok(used_bytes), Ok(total_bytes)) => Reading::Value(MemoryAmount {
                        used_bytes,
                        total_bytes,
                    }),
                    (Err(reason), _) | (_, Err(reason)) => Reading::Unavailable(reason),
                },
                AmdMemoryKind::Integrated | AmdMemoryKind::Unknown => {
                    Reading::Unavailable(UnavailableReason::Missing)
                }
            };
            Some(DeviceFrame {
                vendor: Vendor::Amd,
                pci,
                identity,
                ordinal,
                patch: GpuPatch {
                    usage_percent,
                    vram,
                    temperature: scan_amd_temperature(&device),
                },
            })
        })
        .collect()
}

fn read_u64(path: PathBuf) -> Result<u64, UnavailableReason> {
    fs::read_to_string(path)
        .map_err(|error| io_reason(&error))?
        .trim()
        .parse()
        .map_err(|_| UnavailableReason::InvalidData)
}

fn io_reason(error: &io::Error) -> UnavailableReason {
    match error.kind() {
        io::ErrorKind::NotFound => UnavailableReason::Missing,
        io::ErrorKind::PermissionDenied => UnavailableReason::PermissionDenied,
        _ => UnavailableReason::SourceError,
    }
}

fn scan_amd_temperature(device: &Path) -> Reading<Temperature> {
    let Ok(chips) = fs::read_dir(device.join("hwmon")) else {
        return Reading::Unavailable(UnavailableReason::Missing);
    };
    let mut candidates = Vec::new();
    for chip in chips.flatten().map(|entry| entry.path()) {
        if fs::read_to_string(chip.join("name"))
            .ok()
            .is_none_or(|name| name.trim() != "amdgpu")
        {
            continue;
        }
        let Ok(entries) = fs::read_dir(&chip) else {
            continue;
        };
        for entry in entries.flatten() {
            let Some(index) = entry.file_name().to_str().and_then(|name| {
                name.strip_prefix("temp")?
                    .strip_suffix("_label")
                    .map(str::to_owned)
            }) else {
                continue;
            };
            if index.parse::<u32>().is_err() {
                continue;
            }
            let Ok(label) = fs::read_to_string(entry.path()) else {
                continue;
            };
            let rank = match label.trim().to_ascii_lowercase().as_str() {
                "edge" => 0,
                "junction" => 1,
                _ => continue,
            };
            let prefix = format!("temp{index}");
            if !status_allows(chip.join(format!("{prefix}_fault")), "0")
                || !status_allows(chip.join(format!("{prefix}_enable")), "1")
            {
                continue;
            }
            let Ok(value) = fs::read_to_string(chip.join(format!("{prefix}_input"))) else {
                continue;
            };
            let Ok(millidegrees_celsius) = value.trim().parse::<i64>() else {
                continue;
            };
            let threshold = read_positive_i64(chip.join(format!("{prefix}_max")))
                .map(|millidegrees_celsius| TemperatureThreshold {
                    millidegrees_celsius,
                    kind: ThresholdKind::TargetOrMax,
                })
                .or_else(|| {
                    read_positive_i64(chip.join(format!("{prefix}_crit"))).map(
                        |millidegrees_celsius| TemperatureThreshold {
                            millidegrees_celsius,
                            kind: ThresholdKind::Critical,
                        },
                    )
                });
            candidates.push((
                rank,
                Temperature {
                    millidegrees_celsius,
                    threshold,
                    source: format!("AMD GPU {}", label.trim()),
                },
            ));
        }
    }
    candidates.sort_by_key(|(rank, _)| *rank);
    match candidates.as_slice() {
        [] => Reading::Unavailable(UnavailableReason::Missing),
        [(rank, value), ..]
            if candidates.iter().filter(|(other, _)| other == rank).count() == 1 =>
        {
            Reading::Value(value.clone())
        }
        _ => Reading::Unavailable(UnavailableReason::InvalidData),
    }
}

fn read_positive_i64(path: PathBuf) -> Option<i64> {
    fs::read_to_string(path)
        .ok()?
        .trim()
        .parse::<i64>()
        .ok()
        .filter(|value| *value > 0)
}

fn status_allows(path: PathBuf, expected: &str) -> bool {
    match fs::read_to_string(path) {
        Ok(value) => value.trim() == expected,
        Err(error) if error.kind() == io::ErrorKind::NotFound => true,
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture_root() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "sys-mon-gpu-fixture-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn amd_fixture(root: &Path) -> PathBuf {
        let pci = root.join("0000:03:00.0");
        fs::create_dir_all(pci.join("hwmon/hwmon7")).unwrap();
        fs::create_dir_all(root.join("card2")).unwrap();
        symlink(&pci, root.join("card2/device")).unwrap();
        fs::write(pci.join("vendor"), "0x1002\n").unwrap();
        symlink("amdgpu", pci.join("driver")).unwrap();
        fs::write(pci.join("gpu_busy_percent"), "0\n").unwrap();
        fs::write(pci.join("mem_info_vram_used"), "100\n").unwrap();
        fs::write(pci.join("mem_info_vram_total"), "200\n").unwrap();
        let hwmon = pci.join("hwmon/hwmon7");
        fs::write(hwmon.join("name"), "amdgpu\n").unwrap();
        fs::write(hwmon.join("temp1_label"), "edge\n").unwrap();
        fs::write(hwmon.join("temp1_input"), "41000\n").unwrap();
        fs::write(hwmon.join("temp1_max"), "85000\n").unwrap();
        fs::write(hwmon.join("temp2_label"), "junction\n").unwrap();
        fs::write(hwmon.join("temp2_input"), "61000\n").unwrap();
        fs::write(hwmon.join("temp2_crit"), "110000\n").unwrap();
        pci
    }

    #[test]
    fn amd_apu_does_not_become_vram_and_temperature_keeps_its_channel_limit() {
        let root = fixture_root();
        let pci = amd_fixture(&root);
        let discrete = scan_amd(&root, |_| AmdMemoryKind::Discrete);
        assert_eq!(discrete.len(), 1);
        assert_eq!(discrete[0].patch.usage_percent, Reading::Value(0.0));
        assert_eq!(
            discrete[0].patch.vram,
            Reading::Value(MemoryAmount {
                used_bytes: 100,
                total_bytes: 200
            })
        );
        assert_eq!(
            discrete[0].patch.temperature,
            Reading::Value(Temperature {
                millidegrees_celsius: 41_000,
                threshold: Some(TemperatureThreshold {
                    millidegrees_celsius: 85_000,
                    kind: ThresholdKind::TargetOrMax,
                }),
                source: "AMD GPU edge".into(),
            })
        );
        for kind in [AmdMemoryKind::Integrated, AmdMemoryKind::Unknown] {
            let frame = scan_amd(&root, |_| kind).pop().unwrap();
            assert!(matches!(frame.patch.vram, Reading::Unavailable(_)));
            assert_eq!(frame.patch.usage_percent, Reading::Value(0.0));
        }
        let hwmon = pci.join("hwmon/hwmon7");
        fs::write(hwmon.join("temp1_fault"), "1\n").unwrap();
        let frame = scan_amd(&root, |_| AmdMemoryKind::Discrete).pop().unwrap();
        assert!(matches!(
            frame.patch.temperature,
            Reading::Value(Temperature {
                millidegrees_celsius: 61_000,
                threshold: Some(TemperatureThreshold {
                    millidegrees_celsius: 110_000,
                    kind: ThresholdKind::Critical
                }),
                ..
            })
        ));
        fs::remove_file(hwmon.join("temp1_fault")).unwrap();
        fs::create_dir(hwmon.join("temp1_fault")).unwrap();
        let frame = scan_amd(&root, |_| AmdMemoryKind::Discrete).pop().unwrap();
        assert!(matches!(
            frame.patch.temperature,
            Reading::Value(Temperature {
                millidegrees_celsius: 61_000,
                ..
            })
        ));
        fs::remove_dir(hwmon.join("temp1_fault")).unwrap();
        fs::create_dir(hwmon.join("temp1_enable")).unwrap();
        let frame = scan_amd(&root, |_| AmdMemoryKind::Discrete).pop().unwrap();
        assert!(matches!(
            frame.patch.temperature,
            Reading::Value(Temperature {
                millidegrees_celsius: 61_000,
                ..
            })
        ));
        fs::remove_dir(hwmon.join("temp1_enable")).unwrap();
        fs::write(hwmon.join("temp1_fault"), "1\n").unwrap();
        fs::write(hwmon.join("temp3_label"), "junction\n").unwrap();
        fs::write(hwmon.join("temp3_input"), "62000\n").unwrap();
        let frame = scan_amd(&root, |_| AmdMemoryKind::Discrete).pop().unwrap();
        assert_eq!(
            frame.patch.temperature,
            Reading::Unavailable(UnavailableReason::InvalidData)
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn selection_does_not_mix_devices_or_prefer_a_later_healthy_field() {
        let frame = |vendor, pci: Option<&str>, identity: &str, percent| DeviceFrame {
            vendor,
            pci: pci.map(str::to_owned),
            identity: identity.into(),
            ordinal: 0,
            patch: GpuPatch {
                usage_percent: percent,
                vram: Reading::Unavailable(UnavailableReason::Missing),
                temperature: Reading::Unavailable(UnavailableReason::Missing),
            },
        };
        let selected = selected_patch(vec![
            frame(
                Vendor::Amd,
                Some("0000:01:00.0"),
                "amd",
                Reading::Value(90.0),
            ),
            frame(Vendor::Nvidia, None, "nv-no-bdf", Reading::Value(60.0)),
            frame(
                Vendor::Nvidia,
                Some("0000:02:00.0"),
                "nv-bdf",
                Reading::Unavailable(UnavailableReason::PermissionDenied),
            ),
        ]);
        assert_eq!(
            selected.usage_percent,
            Reading::Unavailable(UnavailableReason::PermissionDenied)
        );
        assert_eq!(
            selected.vram,
            Reading::Unavailable(UnavailableReason::Missing)
        );
        assert_eq!(
            selected_patch(Vec::new()).usage_percent,
            Reading::Unavailable(UnavailableReason::Missing)
        );
    }
}
