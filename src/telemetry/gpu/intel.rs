//! Minimal runtime-loaded Level Zero Sysman ABI. All calls stay in the GPU child.

use super::{DeviceFrame, MAX_DEVICES, Vendor};
use crate::telemetry::{
    GpuPatch, MemoryAmount, Reading, Temperature, TemperatureThreshold, ThresholdKind,
    UnavailableReason,
};
use libloading::Library;
use std::ffi::c_void;
use std::ptr;
use std::thread;
use std::time::Duration;

type Handle = *mut c_void;
type ResultCode = u32;
type Enumerate = unsafe extern "C" fn(Handle, *mut u32, *mut Handle) -> ResultCode;
type Properties<T> = unsafe extern "C" fn(Handle, *mut T) -> ResultCode;
type EngineActivity = unsafe extern "C" fn(Handle, *mut EngineStats) -> ResultCode;
type TempState = unsafe extern "C" fn(Handle, *mut f64) -> ResultCode;

const SUCCESS: ResultCode = 0;
const INIT_GPU_ONLY: u32 = 1;
const ENGINE_PROPERTIES: u32 = 0x5;
const MEM_PROPERTIES: u32 = 0xb;
const TEMP_PROPERTIES: u32 = 0x14;
const MEM_STATE: u32 = 0x1e;
const PCI_PROPERTIES: u32 = 0x2;
const GROUP_ALL: u32 = 0;
const MEM_LOC_DEVICE: u32 = 1;
const TEMP_GLOBAL: u32 = 0;
const TEMP_GPU: u32 = 1;

#[repr(C)]
#[derive(Default)]
struct EngineProperties {
    stype: u32,
    p_next: *mut c_void,
    group: u32,
    on_subdevice: u32,
    subdevice_id: u32,
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct EngineStats {
    active_time: u64,
    timestamp: u64,
}

#[repr(C)]
#[derive(Default)]
struct MemProperties {
    stype: u32,
    p_next: *mut c_void,
    kind: u32,
    on_subdevice: u32,
    subdevice_id: u32,
    location: u32,
    physical_size: u64,
    bus_width: i32,
    num_channels: i32,
}

#[repr(C)]
#[derive(Default)]
struct MemState {
    stype: u32,
    p_next: *const c_void,
    health: u32,
    free: u64,
    size: u64,
}

#[repr(C)]
#[derive(Default)]
struct TempProperties {
    stype: u32,
    p_next: *mut c_void,
    kind: u32,
    on_subdevice: u32,
    subdevice_id: u32,
    max_temperature: f64,
    is_critical_supported: u32,
    is_threshold1_supported: u32,
    is_threshold2_supported: u32,
}

#[repr(C)]
#[derive(Default)]
struct PciAddress {
    domain: u32,
    bus: u32,
    device: u32,
    function: u32,
}

#[repr(C)]
#[derive(Default)]
struct PciSpeed {
    generation: i32,
    width: i32,
    max_bandwidth: i64,
}

#[repr(C)]
#[derive(Default)]
struct PciProperties {
    stype: u32,
    p_next: *mut c_void,
    address: PciAddress,
    max_speed: PciSpeed,
    have_bandwidth_counters: u32,
    have_packet_counters: u32,
    have_replay_counters: u32,
}

struct Api {
    _library: Option<Library>,
    init: unsafe extern "C" fn(u32) -> ResultCode,
    drivers: unsafe extern "C" fn(*mut u32, *mut Handle) -> ResultCode,
    devices: Enumerate,
    pci_properties: Option<Properties<PciProperties>>,
    engines: Option<Enumerate>,
    engine_properties: Option<Properties<EngineProperties>>,
    engine_activity: Option<EngineActivity>,
    memories: Option<Enumerate>,
    memory_properties: Option<Properties<MemProperties>>,
    memory_state: Option<Properties<MemState>>,
    temperatures: Option<Enumerate>,
    temperature_properties: Option<Properties<TempProperties>>,
    temperature_state: Option<TempState>,
}

impl Api {
    fn load() -> Option<Self> {
        // SAFETY: the loader remains owned by Api while its copied symbols are used.
        let library = unsafe { Library::new("libze_loader.so.1").ok()? };
        // SAFETY: names and signatures follow the public zes_api.h C ABI.
        unsafe {
            Some(Self {
                init: symbol(&library, b"zesInit\0")?,
                drivers: symbol(&library, b"zesDriverGet\0")?,
                devices: symbol(&library, b"zesDeviceGet\0")?,
                pci_properties: symbol(&library, b"zesDevicePciGetProperties\0"),
                engines: symbol(&library, b"zesDeviceEnumEngineGroups\0"),
                engine_properties: symbol(&library, b"zesEngineGetProperties\0"),
                engine_activity: symbol(&library, b"zesEngineGetActivity\0"),
                memories: symbol(&library, b"zesDeviceEnumMemoryModules\0"),
                memory_properties: symbol(&library, b"zesMemoryGetProperties\0"),
                memory_state: symbol(&library, b"zesMemoryGetState\0"),
                temperatures: symbol(&library, b"zesDeviceEnumTemperatureSensors\0"),
                temperature_properties: symbol(&library, b"zesTemperatureGetProperties\0"),
                temperature_state: symbol(&library, b"zesTemperatureGetState\0"),
                _library: Some(library),
            })
        }
    }
}

unsafe fn symbol<T: Copy>(library: &Library, name: &[u8]) -> Option<T> {
    // SAFETY: the caller supplies a symbol name with its C ABI signature.
    unsafe { library.get::<T>(name).ok().map(|symbol| *symbol) }
}

fn enumerate(mut call: impl FnMut(*mut u32, *mut Handle) -> ResultCode) -> Option<Vec<Handle>> {
    let mut count = 0_u32;
    if call(&mut count, ptr::null_mut()) != SUCCESS || count > MAX_DEVICES as u32 {
        return None;
    }
    let mut handles = vec![ptr::null_mut(); count as usize];
    if count > 0 && call(&mut count, handles.as_mut_ptr()) != SUCCESS {
        return None;
    }
    handles.truncate(count as usize);
    (!handles.iter().any(|handle| handle.is_null())).then_some(handles)
}

pub(super) fn scan_intel() -> Vec<DeviceFrame> {
    let Some(api) = Api::load() else {
        return Vec::new();
    };
    scan_with_api(api)
}

fn scan_with_api(api: Api) -> Vec<DeviceFrame> {
    // SAFETY: Api owns a loaded Sysman library and the flag is defined by its ABI.
    if unsafe { (api.init)(INIT_GPU_ONLY) } != SUCCESS {
        return Vec::new();
    }
    let Some(drivers) = enumerate(|count, handles| {
        // SAFETY: pointers refer to bounded local enumeration buffers.
        unsafe { (api.drivers)(count, handles) }
    }) else {
        return Vec::new();
    };
    let mut frames = Vec::new();
    for (driver_ordinal, driver) in drivers.into_iter().enumerate() {
        let Some(devices) = enumerate(|count, handles| {
            // SAFETY: driver is a Sysman handle from the current enumeration.
            unsafe { (api.devices)(driver, count, handles) }
        }) else {
            continue;
        };
        for (device_ordinal, device) in devices.into_iter().enumerate() {
            let pci = pci_address(&api, device);
            frames.push(DeviceFrame {
                vendor: Vendor::Intel,
                pci,
                identity: format!("intel-{driver_ordinal}-{device_ordinal}"),
                ordinal: frames.len(),
                patch: GpuPatch {
                    usage_percent: engine_usage(&api, device),
                    vram: device_memory(&api, device),
                    temperature: gpu_temperature(&api, device),
                },
            });
            if frames.len() >= MAX_DEVICES {
                return frames;
            }
        }
    }
    frames
}

fn pci_address(api: &Api, device: Handle) -> Option<String> {
    let call = api.pci_properties?;
    let mut properties = PciProperties {
        stype: PCI_PROPERTIES,
        ..Default::default()
    };
    // SAFETY: properties is ABI-compatible and lives through the call.
    (unsafe { call(device, &mut properties) } == SUCCESS).then(|| {
        format!(
            "{:08x}:{:02x}:{:02x}.{}",
            properties.address.domain,
            properties.address.bus,
            properties.address.device,
            properties.address.function
        )
    })
}

fn engine_usage(api: &Api, device: Handle) -> Reading<f64> {
    let Some((enumerate_engines, properties_call, activity_call)) = api
        .engines
        .zip(api.engine_properties)
        .zip(api.engine_activity)
        .map(|((a, b), c)| (a, b, c))
    else {
        return Reading::Unavailable(UnavailableReason::Missing);
    };
    let Some(engines) = enumerate(|count, handles| {
        // SAFETY: device and enumeration pointers follow Sysman ABI.
        unsafe { enumerate_engines(device, count, handles) }
    }) else {
        return Reading::Unavailable(UnavailableReason::SourceError);
    };
    let mut roots = Vec::new();
    for engine in engines {
        let mut properties = EngineProperties {
            stype: ENGINE_PROPERTIES,
            ..Default::default()
        };
        // SAFETY: properties is initialized with the required structure type.
        if unsafe { properties_call(engine, &mut properties) } == SUCCESS
            && properties.group == GROUP_ALL
            && properties.on_subdevice == 0
        {
            roots.push(engine);
        }
    }
    if roots.len() != 1 {
        return Reading::Unavailable(UnavailableReason::Missing);
    }
    let mut first = EngineStats::default();
    let mut second = EngineStats::default();
    // SAFETY: root handle was returned by this Sysman instance.
    if unsafe { activity_call(roots[0], &mut first) } != SUCCESS {
        return Reading::Unavailable(UnavailableReason::SourceError);
    }
    thread::sleep(Duration::from_millis(25));
    // SAFETY: same root handle and an initialized output buffer.
    if unsafe { activity_call(roots[0], &mut second) } != SUCCESS {
        return Reading::Unavailable(UnavailableReason::SourceError);
    }
    match usage_delta(first, second) {
        Some(percent) => Reading::Value(percent),
        None => Reading::Unavailable(UnavailableReason::InvalidData),
    }
}

fn usage_delta(first: EngineStats, second: EngineStats) -> Option<f64> {
    let active = second.active_time.checked_sub(first.active_time)?;
    let elapsed = second.timestamp.checked_sub(first.timestamp)?;
    (elapsed > 0 && active <= elapsed).then_some(active as f64 * 100.0 / elapsed as f64)
}

fn device_memory(api: &Api, device: Handle) -> Reading<MemoryAmount> {
    let Some((enumerate_memory, properties_call, state_call)) = api
        .memories
        .zip(api.memory_properties)
        .zip(api.memory_state)
        .map(|((a, b), c)| (a, b, c))
    else {
        return Reading::Unavailable(UnavailableReason::Missing);
    };
    let Some(modules) = enumerate(|count, handles| {
        // SAFETY: device and enumeration pointers follow Sysman ABI.
        unsafe { enumerate_memory(device, count, handles) }
    }) else {
        return Reading::Unavailable(UnavailableReason::SourceError);
    };
    let mut used = 0_u64;
    let mut total = 0_u64;
    let mut found = false;
    for module in modules {
        let mut properties = MemProperties {
            stype: MEM_PROPERTIES,
            ..Default::default()
        };
        // SAFETY: output buffer follows Sysman ABI.
        if unsafe { properties_call(module, &mut properties) } != SUCCESS {
            return Reading::Unavailable(UnavailableReason::SourceError);
        }
        if properties.location != MEM_LOC_DEVICE || properties.on_subdevice != 0 {
            continue;
        }
        found = true;
        if properties.physical_size == 0 {
            return Reading::Unavailable(UnavailableReason::Missing);
        }
        let mut state = MemState {
            stype: MEM_STATE,
            ..Default::default()
        };
        // SAFETY: output buffer follows Sysman ABI.
        if unsafe { state_call(module, &mut state) } != SUCCESS {
            return Reading::Unavailable(UnavailableReason::SourceError);
        }
        let Some(module_used) = properties.physical_size.checked_sub(state.free) else {
            return Reading::Unavailable(UnavailableReason::InvalidData);
        };
        let Some(next_used) = used.checked_add(module_used) else {
            return Reading::Unavailable(UnavailableReason::InvalidData);
        };
        let Some(next_total) = total.checked_add(properties.physical_size) else {
            return Reading::Unavailable(UnavailableReason::InvalidData);
        };
        used = next_used;
        total = next_total;
    }
    if !found || total == 0 {
        Reading::Unavailable(UnavailableReason::Missing)
    } else {
        Reading::Value(MemoryAmount {
            used_bytes: used,
            total_bytes: total,
        })
    }
}

fn gpu_temperature(api: &Api, device: Handle) -> Reading<Temperature> {
    let Some((enumerate_temperature, properties_call, state_call)) = api
        .temperatures
        .zip(api.temperature_properties)
        .zip(api.temperature_state)
        .map(|((a, b), c)| (a, b, c))
    else {
        return Reading::Unavailable(UnavailableReason::Missing);
    };
    let Some(sensors) = enumerate(|count, handles| {
        // SAFETY: device and enumeration pointers follow Sysman ABI.
        unsafe { enumerate_temperature(device, count, handles) }
    }) else {
        return Reading::Unavailable(UnavailableReason::SourceError);
    };
    let mut candidates = Vec::new();
    for sensor in sensors {
        let mut properties = TempProperties {
            stype: TEMP_PROPERTIES,
            ..Default::default()
        };
        // SAFETY: output buffer follows Sysman ABI.
        if unsafe { properties_call(sensor, &mut properties) } != SUCCESS
            || properties.on_subdevice != 0
        {
            continue;
        }
        let rank = match properties.kind {
            TEMP_GPU => 0,
            TEMP_GLOBAL => 1,
            _ => continue,
        };
        candidates.push((rank, sensor, properties));
    }
    candidates.sort_by_key(|(rank, _, _)| *rank);
    let Some((rank, sensor, properties)) = candidates.first() else {
        return Reading::Unavailable(UnavailableReason::Missing);
    };
    if candidates
        .iter()
        .filter(|(other, _, _)| other == rank)
        .count()
        != 1
    {
        return Reading::Unavailable(UnavailableReason::InvalidData);
    }
    let mut celsius = 0_f64;
    // SAFETY: selected root sensor and initialized output.
    if unsafe { state_call(*sensor, &mut celsius) } != SUCCESS {
        return Reading::Unavailable(UnavailableReason::SourceError);
    }
    let Some(millidegrees_celsius) = degrees_to_milli(celsius) else {
        return Reading::Unavailable(UnavailableReason::InvalidData);
    };
    let threshold = degrees_to_milli(properties.max_temperature)
        .filter(|value| *value > 0)
        .map(|millidegrees_celsius| TemperatureThreshold {
            millidegrees_celsius,
            kind: ThresholdKind::TargetOrMax,
        });
    Reading::Value(Temperature {
        millidegrees_celsius,
        threshold,
        source: if *rank == 0 {
            "Intel GPU".into()
        } else {
            "Intel GPU global".into()
        },
    })
}

fn degrees_to_milli(value: f64) -> Option<i64> {
    (value.is_finite() && (-100.0..=250.0).contains(&value))
        .then_some((value * 1000.0).round() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static INIT_FLAGS: AtomicUsize = AtomicUsize::new(0);
    static TEST_LOCK: Mutex<()> = Mutex::new(());
    unsafe extern "C" fn no_init(flags: u32) -> ResultCode {
        INIT_FLAGS.store(flags as usize, Ordering::SeqCst);
        SUCCESS
    }
    unsafe extern "C" fn no_drivers(count: *mut u32, handles: *mut Handle) -> ResultCode {
        // SAFETY: scan_with_api supplies the ABI's count and one-handle buffer.
        unsafe {
            *count = 1;
            if !handles.is_null() {
                *handles = 3 as Handle;
            }
        }
        SUCCESS
    }
    unsafe extern "C" fn no_devices(
        _device: Handle,
        count: *mut u32,
        handles: *mut Handle,
    ) -> ResultCode {
        // The mock Sysman driver exposes a GPU and a VPU; GPU_ONLY filters VPU.
        let gpu_only = INIT_FLAGS.load(Ordering::SeqCst) == INIT_GPU_ONLY as usize;
        // SAFETY: scan_with_api supplies the ABI's count and two-handle buffer.
        unsafe {
            *count = if gpu_only { 1 } else { 2 };
            if !handles.is_null() {
                *handles = 1 as Handle;
                if !gpu_only {
                    *handles.add(1) = 9 as Handle;
                }
            }
        }
        SUCCESS
    }
    unsafe extern "C" fn both(
        _device: Handle,
        count: *mut u32,
        handles: *mut Handle,
    ) -> ResultCode {
        // SAFETY: the test caller supplies the ABI's count and two-handle buffer.
        unsafe {
            if handles.is_null() {
                *count = 2;
            } else {
                *count = 2;
                *handles = 1 as Handle;
                *handles.add(1) = 2 as Handle;
            }
        }
        SUCCESS
    }
    unsafe extern "C" fn subonly(
        _device: Handle,
        count: *mut u32,
        handles: *mut Handle,
    ) -> ResultCode {
        // SAFETY: the test caller supplies the ABI's count and one-handle buffer.
        unsafe {
            if handles.is_null() {
                *count = 1;
            } else {
                *count = 1;
                *handles = 2 as Handle;
            }
        }
        SUCCESS
    }
    unsafe extern "C" fn known_and_unknown_roots(
        _device: Handle,
        count: *mut u32,
        handles: *mut Handle,
    ) -> ResultCode {
        // SAFETY: the test caller supplies the ABI's count and three-handle buffer.
        unsafe {
            *count = 3;
            if !handles.is_null() {
                *handles = 1 as Handle;
                *handles.add(1) = 2 as Handle;
                *handles.add(2) = 3 as Handle;
            }
        }
        SUCCESS
    }
    unsafe extern "C" fn memory_properties(
        handle: Handle,
        output: *mut MemProperties,
    ) -> ResultCode {
        // SAFETY: device_memory passes a live initialized output buffer.
        unsafe {
            (*output).location = MEM_LOC_DEVICE;
            (*output).on_subdevice = (handle as usize == 2) as u32;
            (*output).physical_size = if handle as usize == 3 { 0 } else { 100 };
        }
        SUCCESS
    }
    unsafe extern "C" fn memory_state(handle: Handle, output: *mut MemState) -> ResultCode {
        // SAFETY: device_memory passes a live initialized output buffer.
        unsafe {
            (*output).free = if handle as usize == 1 {
                40
            } else if handle as usize == 2 {
                20
            } else {
                0
            };
        }
        SUCCESS
    }
    unsafe extern "C" fn engine_properties(
        handle: Handle,
        output: *mut EngineProperties,
    ) -> ResultCode {
        // SAFETY: engine_usage passes a live initialized output buffer.
        unsafe {
            (*output).group = GROUP_ALL;
            (*output).on_subdevice = (handle as usize == 2) as u32;
        }
        SUCCESS
    }
    static ACTIVITY_CALLS: AtomicUsize = AtomicUsize::new(0);
    unsafe extern "C" fn engine_activity(_handle: Handle, output: *mut EngineStats) -> ResultCode {
        let second = ACTIVITY_CALLS.fetch_add(1, Ordering::SeqCst) % 2 == 1;
        // SAFETY: engine_usage passes a live initialized output buffer.
        unsafe {
            *output = if second {
                EngineStats {
                    active_time: 30,
                    timestamp: 140,
                }
            } else {
                EngineStats {
                    active_time: 10,
                    timestamp: 100,
                }
            };
        }
        SUCCESS
    }
    unsafe extern "C" fn temperature_properties(
        handle: Handle,
        output: *mut TempProperties,
    ) -> ResultCode {
        // SAFETY: gpu_temperature passes a live initialized output buffer.
        unsafe {
            (*output).kind = TEMP_GPU;
            (*output).on_subdevice = (handle as usize == 2) as u32;
            (*output).max_temperature = 85.0;
        }
        SUCCESS
    }
    unsafe extern "C" fn temperature_state(_handle: Handle, output: *mut f64) -> ResultCode {
        // SAFETY: gpu_temperature passes a live output buffer.
        unsafe {
            *output = 45.0;
        }
        SUCCESS
    }

    fn fake_api() -> Api {
        Api {
            _library: None,
            init: no_init,
            drivers: no_drivers,
            devices: no_devices,
            pci_properties: None,
            engines: Some(both),
            engine_properties: Some(engine_properties),
            engine_activity: Some(engine_activity),
            memories: Some(both),
            memory_properties: Some(memory_properties),
            memory_state: Some(memory_state),
            temperatures: Some(both),
            temperature_properties: Some(temperature_properties),
            temperature_state: Some(temperature_state),
        }
    }

    #[test]
    fn sysman_adapter_ignores_subdevice_overlap_and_refuses_subdevice_only() {
        let _guard = TEST_LOCK.lock().unwrap();
        ACTIVITY_CALLS.store(0, Ordering::SeqCst);
        let mut api = fake_api();
        assert_eq!(
            device_memory(&api, 1 as Handle),
            Reading::Value(MemoryAmount {
                used_bytes: 60,
                total_bytes: 100
            })
        );
        assert_eq!(engine_usage(&api, 1 as Handle), Reading::Value(50.0));
        assert!(matches!(
            gpu_temperature(&api, 1 as Handle),
            Reading::Value(Temperature {
                millidegrees_celsius: 45_000,
                threshold: Some(TemperatureThreshold {
                    millidegrees_celsius: 85_000,
                    ..
                }),
                ..
            })
        ));
        api.memories = Some(subonly);
        api.engines = Some(subonly);
        api.temperatures = Some(subonly);
        assert_eq!(
            device_memory(&api, 1 as Handle),
            Reading::Unavailable(UnavailableReason::Missing)
        );
        assert_eq!(
            engine_usage(&api, 1 as Handle),
            Reading::Unavailable(UnavailableReason::Missing)
        );
        assert_eq!(
            gpu_temperature(&api, 1 as Handle),
            Reading::Unavailable(UnavailableReason::Missing)
        );
        api.memories = Some(known_and_unknown_roots);
        assert_eq!(
            device_memory(&api, 1 as Handle),
            Reading::Unavailable(UnavailableReason::Missing)
        );
    }

    #[test]
    fn sysman_initialization_filters_vpu_before_gpu_selection() {
        let _guard = TEST_LOCK.lock().unwrap();
        ACTIVITY_CALLS.store(0, Ordering::SeqCst);
        let frames = scan_with_api(fake_api());
        assert_eq!(INIT_FLAGS.load(Ordering::SeqCst), INIT_GPU_ONLY as usize);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].vendor, Vendor::Intel);
        assert_eq!(frames[0].patch.usage_percent, Reading::Value(50.0));
    }

    #[test]
    fn aggregate_counter_refuses_reset_and_impossible_activity() {
        assert_eq!(
            usage_delta(
                EngineStats {
                    active_time: 10,
                    timestamp: 100
                },
                EngineStats {
                    active_time: 35,
                    timestamp: 150
                },
            ),
            Some(50.0)
        );
        assert_eq!(
            usage_delta(
                EngineStats {
                    active_time: 10,
                    timestamp: 100
                },
                EngineStats {
                    active_time: 5,
                    timestamp: 150
                }
            ),
            None
        );
        assert_eq!(
            usage_delta(
                EngineStats {
                    active_time: 10,
                    timestamp: 100
                },
                EngineStats {
                    active_time: 11,
                    timestamp: 100
                }
            ),
            None
        );
        assert_eq!(
            usage_delta(
                EngineStats {
                    active_time: 10,
                    timestamp: 100
                },
                EngineStats {
                    active_time: 70,
                    timestamp: 150
                }
            ),
            None
        );
    }
}
