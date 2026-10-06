use std::collections::VecDeque;
use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use sys_mon::telemetry::{
    GpuPatch, HostPatch, HostProbe, HostSampler, IoPatch, MemoryAmount, NetworkSnapshot, Patch,
    PrimaryVolume, Provider, ProviderKind, Reading, SessionBuilder, StopToken, StorageSnapshot,
    TemperatureProvider, UnavailableReason,
};

struct Frames {
    stats: VecDeque<io::Result<String>>,
    memory: VecDeque<io::Result<String>>,
}

impl HostProbe for Frames {
    fn read_stat(&mut self) -> io::Result<String> {
        self.stats.pop_front().expect("stat fixture exhausted")
    }
    fn read_meminfo(&mut self) -> io::Result<String> {
        self.memory.pop_front().expect("meminfo fixture exhausted")
    }
}

fn stat(user: u64, idle: u64, steal: u64, guest: u64) -> String {
    format!("cpu  {user} 0 0 {idle} 0 0 0 {steal} {guest} 0\ncpu0 1 1 1 1 1 1 1 1 0 0\n")
}

fn mem(total: u64, available: u64, swap_total: u64, swap_free: u64) -> String {
    format!(
        "MemTotal: {total} kB\nMemFree: 1 kB\nMemAvailable: {available} kB\nSwapTotal: {swap_total} kB\nSwapFree: {swap_free} kB\n"
    )
}

#[test]
fn public_host_patch_preserves_units_guest_accounting_and_valid_zero() {
    let mut sampler = HostSampler::new(Frames {
        stats: VecDeque::from(vec![
            Ok(stat(100, 100, 0, 50)),
            Ok(stat(150, 150, 50, 100)),
            Ok(stat(150, 250, 50, 100)),
        ]),
        memory: VecDeque::from(vec![
            Ok(mem(100, 40, 0, 0)),
            Ok(mem(100, 40, 0, 0)),
            Ok(mem(100, 40, 0, 0)),
        ]),
    });
    let first = sampler.tick(Duration::ZERO);
    assert_eq!(
        first.cpu_usage_percent,
        Reading::Unavailable(UnavailableReason::WarmingUp)
    );
    assert_eq!(
        first.ram,
        Reading::Value(MemoryAmount {
            used_bytes: 60 * 1024,
            total_bytes: 100 * 1024
        })
    );
    assert_eq!(
        first.swap,
        Reading::Value(MemoryAmount {
            used_bytes: 0,
            total_bytes: 0
        })
    );
    // Guest is already included in user; steal is explicitly inactive.
    let usage = sampler.tick(Duration::from_secs(1)).cpu_usage_percent;
    assert!(matches!(usage, Reading::Value(value) if (value - 100.0 / 3.0).abs() < 1e-10));
    assert_eq!(
        sampler.tick(Duration::from_secs(2)).cpu_usage_percent,
        Reading::Value(0.0)
    );
}

#[test]
fn host_errors_resets_and_resume_gaps_rebaseline_without_stale_values() {
    let mut sampler = HostSampler::new(Frames {
        stats: VecDeque::from(vec![
            Ok(stat(100, 100, 0, 0)),
            Ok(stat(120, 120, 0, 0)),
            Ok(stat(5, 5, 0, 0)), // reset
            Ok(stat(15, 5, 0, 0)),
            Err(io::Error::from(io::ErrorKind::PermissionDenied)),
            Ok(stat(30, 20, 0, 0)),
            Ok(stat(40, 20, 0, 0)),
            Ok(stat(50, 20, 0, 0)), // long elapsed gap
            Ok(stat(60, 20, 0, 0)),
        ]),
        memory: (0..9).map(|_| Ok(mem(10, 5, 10, 3))).collect(),
    });
    let times = [0, 1, 2, 3, 4, 5, 6, 30, 31];
    let patches: Vec<_> = times
        .into_iter()
        .map(|s| sampler.tick(Duration::from_secs(s)))
        .collect();
    assert_eq!(patches[1].cpu_usage_percent, Reading::Value(50.0));
    assert_eq!(
        patches[2].cpu_usage_percent,
        Reading::Unavailable(UnavailableReason::InvalidData)
    );
    assert_eq!(patches[3].cpu_usage_percent, Reading::Value(100.0));
    assert_eq!(
        patches[4].cpu_usage_percent,
        Reading::Unavailable(UnavailableReason::PermissionDenied)
    );
    assert_eq!(
        patches[5].cpu_usage_percent,
        Reading::Unavailable(UnavailableReason::WarmingUp)
    );
    assert_eq!(patches[6].cpu_usage_percent, Reading::Value(100.0));
    assert_eq!(
        patches[7].cpu_usage_percent,
        Reading::Unavailable(UnavailableReason::InvalidData)
    );
    assert_eq!(patches[8].cpu_usage_percent, Reading::Value(100.0));
    assert_eq!(
        patches[4].ram,
        Reading::Value(MemoryAmount {
            used_bytes: 5 * 1024,
            total_bytes: 10 * 1024
        })
    );
}

#[test]
fn missing_memory_fields_do_not_erase_other_readings() {
    let mut sampler = HostSampler::new(Frames {
        stats: VecDeque::from(vec![Ok(stat(1, 1, 0, 0)), Ok(stat(2, 1, 0, 0))]),
        memory: VecDeque::from(vec![
            Ok(mem(10, 5, 10, 3)),
            Ok("MemTotal: 10 kB\nMemAvailable: 3 kB\nSwapTotal: 10 kB\n".into()),
        ]),
    });
    sampler.tick(Duration::ZERO);
    let patch = sampler.tick(Duration::from_secs(1));
    assert_eq!(patch.cpu_usage_percent, Reading::Value(100.0));
    assert_eq!(
        patch.ram,
        Reading::Value(MemoryAmount {
            used_bytes: 7 * 1024,
            total_bytes: 10 * 1024
        })
    );
    assert_eq!(
        patch.swap,
        Reading::Unavailable(UnavailableReason::InvalidData)
    );
}

struct RepeatingProvider {
    kind: ProviderKind,
    patch: Patch,
    ticks: Arc<AtomicUsize>,
    interval: Duration,
    work: Duration,
}

impl Provider for RepeatingProvider {
    fn kind(&self) -> ProviderKind {
        self.kind
    }
    fn sample(&mut self, _elapsed: Duration, _stop: &StopToken) -> Patch {
        self.ticks.fetch_add(1, Ordering::SeqCst);
        thread::sleep(self.work);
        self.patch.clone()
    }
    fn interval(&self) -> Duration {
        self.interval
    }
}

fn wait_until(mut condition: impl FnMut() -> bool) {
    let started = Instant::now();
    while !condition() {
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "condition did not become true"
        );
        thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn provider_interleaving_preserves_disjoint_fields_and_stop_ends_all_workers() {
    let ticks = Arc::new(AtomicUsize::new(0));
    let host = RepeatingProvider {
        kind: ProviderKind::Host,
        patch: Patch::Host(HostPatch {
            cpu_usage_percent: Reading::Value(25.0),
            ram: Reading::Value(MemoryAmount {
                used_bytes: 5,
                total_bytes: 10,
            }),
            swap: Reading::Value(MemoryAmount {
                used_bytes: 0,
                total_bytes: 0,
            }),
        }),
        ticks: Arc::clone(&ticks),
        interval: Duration::from_millis(3),
        work: Duration::ZERO,
    };
    let gpu = RepeatingProvider {
        kind: ProviderKind::Gpu,
        patch: Patch::Gpu(GpuPatch {
            usage_percent: Reading::Value(40.0),
            vram: Reading::Value(MemoryAmount {
                used_bytes: 7,
                total_bytes: 12,
            }),
            temperature: Reading::Unavailable(UnavailableReason::Missing),
        }),
        ticks: Arc::new(AtomicUsize::new(0)),
        interval: Duration::from_millis(7),
        work: Duration::ZERO,
    };
    let io = RepeatingProvider {
        kind: ProviderKind::Io,
        patch: Patch::Io(IoPatch {
            network: NetworkSnapshot {
                down_bytes_per_second: Reading::Value(1.0),
                up_bytes_per_second: Reading::Value(2.0),
            },
            storage: StorageSnapshot {
                read_bytes_per_second: Reading::Value(3.0),
                write_bytes_per_second: Reading::Value(4.0),
                primary_volumes: Reading::Value(vec![PrimaryVolume {
                    paths: vec!["/".into(), "/home".into()],
                    available_bytes: 5,
                    total_bytes: 10,
                }]),
            },
        }),
        ticks: Arc::new(AtomicUsize::new(0)),
        interval: Duration::from_millis(7),
        work: Duration::ZERO,
    };
    let mut builder = SessionBuilder::new();
    builder
        .register(host)
        .unwrap()
        .register(gpu)
        .unwrap()
        .register(io)
        .unwrap();
    let mut session = builder.start().unwrap();
    wait_until(|| {
        session.snapshot().gpu.usage_percent == Reading::Value(40.0)
            && session.snapshot().storage.read_bytes_per_second == Reading::Value(3.0)
            && ticks.load(Ordering::SeqCst) >= 4
    });
    let snapshot = session.snapshot();
    assert_eq!(snapshot.cpu.usage_percent, Reading::Value(25.0));
    assert_eq!(
        snapshot.gpu.vram,
        Reading::Value(MemoryAmount {
            used_bytes: 7,
            total_bytes: 12
        })
    );
    assert_eq!(snapshot.network.up_bytes_per_second, Reading::Value(2.0));
    session.stop();
    let stopped_at = ticks.load(Ordering::SeqCst);
    thread::sleep(Duration::from_millis(20));
    assert_eq!(ticks.load(Ordering::SeqCst), stopped_at);
    session.stop();
}

#[test]
fn slow_provider_never_overlaps_or_builds_a_catch_up_queue() {
    let ticks = Arc::new(AtomicUsize::new(0));
    let provider = RepeatingProvider {
        kind: ProviderKind::Host,
        patch: Patch::Host(HostPatch {
            cpu_usage_percent: Reading::Value(1.0),
            ram: Reading::Unavailable(UnavailableReason::Missing),
            swap: Reading::Unavailable(UnavailableReason::Missing),
        }),
        ticks: Arc::clone(&ticks),
        interval: Duration::from_millis(5),
        work: Duration::from_millis(25),
    };
    let mut builder = SessionBuilder::new();
    builder.register(provider).unwrap();
    let mut session = builder.start().unwrap();
    thread::sleep(Duration::from_millis(120));
    session.stop();
    let count = ticks.load(Ordering::SeqCst);
    assert!(
        (3..=5).contains(&count),
        "expected one worker with delay after work, got {count} ticks"
    );
}

#[test]
fn unreleased_temperature_helper_does_not_stall_host_or_shutdown() {
    let temp_root = temp_path("stuck-helper");
    fs::create_dir_all(&temp_root).unwrap();
    let script = temp_root.join("helper.sh");
    let pid_path = temp_root.join("pid");
    fs::write(
        &script,
        format!(
            "#!/bin/sh\nprintf '%s' \"$$\" > '{}'\nexec sleep 30\n",
            pid_path.display()
        ),
    )
    .unwrap();
    let mut permissions = fs::metadata(&script).unwrap().permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&script, permissions).unwrap();
    let ticks = Arc::new(AtomicUsize::new(0));
    let host = RepeatingProvider {
        kind: ProviderKind::Host,
        patch: Patch::Host(HostPatch {
            cpu_usage_percent: Reading::Value(11.0),
            ram: Reading::Unavailable(UnavailableReason::Missing),
            swap: Reading::Unavailable(UnavailableReason::Missing),
        }),
        ticks: Arc::clone(&ticks),
        interval: Duration::from_millis(5),
        work: Duration::ZERO,
    };
    let mut builder = SessionBuilder::new();
    builder.register(host).unwrap();
    builder
        .register(TemperatureProvider::with_executable(
            script,
            Duration::from_millis(80),
        ))
        .unwrap();
    let mut session = builder.start().unwrap();
    wait_until(|| {
        session.snapshot().cpu.temperature == Reading::Unavailable(UnavailableReason::TimedOut)
    });
    assert!(
        ticks.load(Ordering::SeqCst) >= 3,
        "host must keep sampling while sensor is stuck"
    );
    let start = Instant::now();
    session.stop();
    assert!(
        start.elapsed() < Duration::from_secs(1),
        "stop waited for stuck helper"
    );
    let pid = fs::read_to_string(&pid_path).unwrap();
    assert!(
        !PathBuf::from(format!("/proc/{pid}")).exists(),
        "owned helper was not reaped"
    );
    fs::remove_dir_all(temp_root).unwrap();
}

fn temp_path(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("sys-mon-{label}-{}-{nanos}", std::process::id()))
}
