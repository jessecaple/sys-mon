//! A current-only snapshot boundary for the native overview.
//!
//! Each provider owns one field group. It publishes a patch rather than a whole
//! snapshot, so independent providers cannot erase one another's readings.
//! Providers must bound each `sample` call. Potentially stuck vendor or sensor
//! calls belong in killable helpers, as the temperature provider demonstrates.

mod linux;

use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub use linux::{
    HostProbe, HostSampler, LinuxHostProbe, TemperatureProvider, run_temperature_helper,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnavailableReason {
    NotYetSampled,
    WarmingUp,
    Missing,
    PermissionDenied,
    InvalidData,
    SourceError,
    TimedOut,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Reading<T> {
    Value(T),
    Unavailable(UnavailableReason),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryAmount {
    pub used_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThresholdKind {
    TargetOrMax,
    Critical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TemperatureThreshold {
    pub millidegrees_celsius: i64,
    pub kind: ThresholdKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Temperature {
    pub millidegrees_celsius: i64,
    pub threshold: Option<TemperatureThreshold>,
    pub source: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CpuSnapshot {
    pub usage_percent: Reading<f64>,
    pub temperature: Reading<Temperature>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GpuSnapshot {
    pub usage_percent: Reading<f64>,
    pub vram: Reading<MemoryAmount>,
    pub temperature: Reading<Temperature>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NetworkSnapshot {
    pub down_bytes_per_second: Reading<f64>,
    pub up_bytes_per_second: Reading<f64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrimaryVolume {
    pub paths: Vec<String>,
    pub available_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StorageSnapshot {
    pub read_bytes_per_second: Reading<f64>,
    pub write_bytes_per_second: Reading<f64>,
    pub primary_volumes: Reading<Vec<PrimaryVolume>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    pub cpu: CpuSnapshot,
    pub ram: Reading<MemoryAmount>,
    pub swap: Reading<MemoryAmount>,
    pub gpu: GpuSnapshot,
    pub network: NetworkSnapshot,
    pub storage: StorageSnapshot,
}

impl Default for Snapshot {
    fn default() -> Self {
        let missing = UnavailableReason::NotYetSampled;
        Self {
            cpu: CpuSnapshot {
                usage_percent: Reading::Unavailable(missing),
                temperature: Reading::Unavailable(missing),
            },
            ram: Reading::Unavailable(missing),
            swap: Reading::Unavailable(missing),
            gpu: GpuSnapshot {
                usage_percent: Reading::Unavailable(missing),
                vram: Reading::Unavailable(missing),
                temperature: Reading::Unavailable(missing),
            },
            network: NetworkSnapshot {
                down_bytes_per_second: Reading::Unavailable(missing),
                up_bytes_per_second: Reading::Unavailable(missing),
            },
            storage: StorageSnapshot {
                read_bytes_per_second: Reading::Unavailable(missing),
                write_bytes_per_second: Reading::Unavailable(missing),
                primary_volumes: Reading::Unavailable(missing),
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct HostPatch {
    pub cpu_usage_percent: Reading<f64>,
    pub ram: Reading<MemoryAmount>,
    pub swap: Reading<MemoryAmount>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GpuPatch {
    pub usage_percent: Reading<f64>,
    pub vram: Reading<MemoryAmount>,
    pub temperature: Reading<Temperature>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IoPatch {
    pub network: NetworkSnapshot,
    pub storage: StorageSnapshot,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Patch {
    Host(HostPatch),
    Temperature(Reading<Temperature>),
    Gpu(GpuPatch),
    Io(IoPatch),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderKind {
    Host,
    Temperature,
    Gpu,
    Io,
}

impl Patch {
    pub fn kind(&self) -> ProviderKind {
        match self {
            Self::Host(_) => ProviderKind::Host,
            Self::Temperature(_) => ProviderKind::Temperature,
            Self::Gpu(_) => ProviderKind::Gpu,
            Self::Io(_) => ProviderKind::Io,
        }
    }

    fn unavailable(kind: ProviderKind, reason: UnavailableReason) -> Self {
        match kind {
            ProviderKind::Host => Self::Host(HostPatch {
                cpu_usage_percent: Reading::Unavailable(reason),
                ram: Reading::Unavailable(reason),
                swap: Reading::Unavailable(reason),
            }),
            ProviderKind::Temperature => Self::Temperature(Reading::Unavailable(reason)),
            ProviderKind::Gpu => Self::Gpu(GpuPatch {
                usage_percent: Reading::Unavailable(reason),
                vram: Reading::Unavailable(reason),
                temperature: Reading::Unavailable(reason),
            }),
            ProviderKind::Io => Self::Io(IoPatch {
                network: NetworkSnapshot {
                    down_bytes_per_second: Reading::Unavailable(reason),
                    up_bytes_per_second: Reading::Unavailable(reason),
                },
                storage: StorageSnapshot {
                    read_bytes_per_second: Reading::Unavailable(reason),
                    write_bytes_per_second: Reading::Unavailable(reason),
                    primary_volumes: Reading::Unavailable(reason),
                },
            }),
        }
    }

    fn apply(self, snapshot: &mut Snapshot) {
        match self {
            Self::Host(patch) => {
                snapshot.cpu.usage_percent = patch.cpu_usage_percent;
                snapshot.ram = patch.ram;
                snapshot.swap = patch.swap;
            }
            Self::Temperature(reading) => snapshot.cpu.temperature = reading,
            Self::Gpu(patch) => {
                snapshot.gpu.usage_percent = patch.usage_percent;
                snapshot.gpu.vram = patch.vram;
                snapshot.gpu.temperature = patch.temperature;
            }
            Self::Io(patch) => {
                snapshot.network = patch.network;
                snapshot.storage = patch.storage;
            }
        }
    }
}

struct StopState {
    stopped: AtomicBool,
    sleep_lock: Mutex<()>,
    wake: Condvar,
}

#[derive(Clone)]
pub struct StopToken(Arc<StopState>);

impl StopToken {
    fn new() -> Self {
        Self(Arc::new(StopState {
            stopped: AtomicBool::new(false),
            sleep_lock: Mutex::new(()),
            wake: Condvar::new(),
        }))
    }

    pub fn is_stopped(&self) -> bool {
        self.0.stopped.load(Ordering::Acquire)
    }

    fn stop(&self) {
        // Use the same mutex as the waiter so notification cannot land
        // between its stopped check and the atomic release inside wait.
        let _guard = self.0.sleep_lock.lock().unwrap_or_else(|e| e.into_inner());
        self.0.stopped.store(true, Ordering::Release);
        self.0.wake.notify_all();
    }

    fn wait(&self, interval: Duration) {
        let guard = self.0.sleep_lock.lock().unwrap_or_else(|e| e.into_inner());
        if !self.is_stopped() {
            let _wait = self
                .0
                .wake
                .wait_timeout(guard, interval)
                .unwrap_or_else(|e| e.into_inner());
        }
    }
}

/// One bounded source of one field group. `sample` must respond to `stop` or
/// have its own bounded/killable boundary; the session cannot abort a Rust thread.
pub trait Provider: Send + 'static {
    fn kind(&self) -> ProviderKind;
    fn sample(&mut self, elapsed: Duration, stop: &StopToken) -> Patch;
    fn interval(&self) -> Duration {
        Duration::from_secs(1)
    }
}

#[derive(Default)]
pub struct SessionBuilder {
    providers: Vec<Box<dyn Provider>>,
}

impl SessionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<P: Provider>(&mut self, provider: P) -> io::Result<&mut Self> {
        if provider.interval() < Duration::from_millis(10) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "provider interval must be at least 10 ms",
            ));
        }
        if self
            .providers
            .iter()
            .any(|existing| existing.kind() == provider.kind())
        {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "provider kind already registered",
            ));
        }
        self.providers.push(Box::new(provider));
        Ok(self)
    }

    pub fn start(self) -> io::Result<TelemetrySession> {
        let state = Arc::new(Mutex::new(SnapshotState {
            active: true,
            generation: 1,
            latest: Snapshot::default(),
        }));
        let stop = StopToken::new();
        let started = Instant::now();
        let mut session = TelemetrySession {
            state,
            stop,
            workers: Vec::new(),
        };
        for mut provider in self.providers {
            let kind = provider.kind();
            let interval = provider.interval();
            let state = Arc::clone(&session.state);
            let stop = session.stop.clone();
            let worker = thread::Builder::new()
                .name(format!("sys-mon-{kind:?}").to_lowercase())
                .spawn(move || {
                    while !stop.is_stopped() {
                        let patch = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            provider.sample(started.elapsed(), &stop)
                        }))
                        .unwrap_or_else(|_| {
                            Patch::unavailable(kind, UnavailableReason::SourceError)
                        });
                        let patch = if patch.kind() == kind {
                            patch
                        } else {
                            Patch::unavailable(kind, UnavailableReason::InvalidData)
                        };
                        publish(&state, 1, patch);
                        stop.wait(interval);
                    }
                });
            match worker {
                Ok(worker) => session.workers.push(worker),
                Err(error) => {
                    session.stop();
                    return Err(error);
                }
            }
        }
        Ok(session)
    }
}

struct SnapshotState {
    active: bool,
    generation: u64,
    latest: Snapshot,
}

fn publish(state: &Mutex<SnapshotState>, generation: u64, patch: Patch) -> bool {
    let mut state = state.lock().unwrap_or_else(|e| e.into_inner());
    if !state.active || state.generation != generation {
        return false;
    }
    patch.apply(&mut state.latest);
    true
}

pub struct TelemetrySession {
    state: Arc<Mutex<SnapshotState>>,
    stop: StopToken,
    workers: Vec<JoinHandle<()>>,
}

impl TelemetrySession {
    pub fn builder() -> SessionBuilder {
        SessionBuilder::new()
    }

    pub fn start_system() -> io::Result<Self> {
        let mut builder = Self::builder();
        builder.register(HostSampler::new(LinuxHostProbe))?;
        builder.register(TemperatureProvider::system()?)?;
        builder.start()
    }

    pub fn snapshot(&self) -> Snapshot {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.latest.clone()
    }

    pub fn stop(&mut self) {
        {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            if state.active {
                state.active = false;
                state.generation = state.generation.wrapping_add(1);
            }
        }
        self.stop.stop();
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

impl Drop for TelemetrySession {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct BlockedProvider {
        entered: std::sync::mpsc::SyncSender<()>,
        release: std::sync::mpsc::Receiver<()>,
    }

    impl Provider for BlockedProvider {
        fn kind(&self) -> ProviderKind {
            ProviderKind::Temperature
        }
        fn sample(&mut self, _elapsed: Duration, _stop: &StopToken) -> Patch {
            self.entered.send(()).unwrap();
            self.release.recv().unwrap();
            Patch::Temperature(Reading::Value(Temperature {
                millidegrees_celsius: 50_000,
                threshold: None,
                source: "late probe".into(),
            }))
        }
    }

    #[test]
    fn in_flight_provider_cannot_commit_after_stop_begins() {
        let (entered_tx, entered_rx) = std::sync::mpsc::sync_channel(0);
        let (release_tx, release_rx) = std::sync::mpsc::sync_channel(0);
        let mut builder = SessionBuilder::new();
        builder
            .register(BlockedProvider {
                entered: entered_tx,
                release: release_rx,
            })
            .unwrap();
        let mut session = builder.start().unwrap();
        entered_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        let state = Arc::clone(&session.state);
        let stopper = thread::spawn(move || {
            session.stop();
            session.snapshot()
        });
        let started = Instant::now();
        loop {
            if !state.lock().unwrap().active {
                break;
            }
            assert!(started.elapsed() < Duration::from_secs(1));
            thread::yield_now();
        }
        release_tx.send(()).unwrap();
        let final_snapshot = stopper.join().unwrap();
        assert_eq!(
            final_snapshot.cpu.temperature,
            Reading::Unavailable(UnavailableReason::NotYetSampled)
        );
    }

    #[test]
    fn late_patch_cannot_publish_after_stop() {
        let state = Mutex::new(SnapshotState {
            active: true,
            generation: 9,
            latest: Snapshot::default(),
        });
        {
            let mut locked = state.lock().unwrap();
            locked.active = false;
            locked.generation += 1;
        }
        let changed = publish(
            &state,
            9,
            Patch::Temperature(Reading::Value(Temperature {
                millidegrees_celsius: 50_000,
                threshold: None,
                source: "test".into(),
            })),
        );
        assert!(!changed);
        assert_eq!(state.lock().unwrap().latest, Snapshot::default());
    }
}
