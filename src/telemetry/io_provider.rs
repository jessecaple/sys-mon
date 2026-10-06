//! Current network, block and primary-filesystem readings. All retained
//! counters belong to sources discovered on the immediately preceding tick.

use super::{
    IoPatch, NetworkSnapshot, Patch, PrimaryVolume, Provider, ProviderKind, Reading, StopToken,
    StorageSnapshot, UnavailableReason,
};
use std::collections::{HashMap, HashSet};
use std::ffi::CString;
use std::fs;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::path::PathBuf;
use std::time::Duration;

const MAX_INTERVAL: Duration = Duration::from_secs(10);

#[derive(Clone, Copy)]
struct Counters {
    down: Option<u64>,
    up: Option<u64>,
}

#[cfg(test)]
impl Counters {
    fn both(down: u64, up: u64) -> Self {
        Self {
            down: Some(down),
            up: Some(up),
        }
    }
}

#[derive(Clone)]
struct Link {
    name: String,
    ifindex: u32,
    up: bool,
}

#[derive(Clone)]
struct NetworkFrame {
    routes4: String,
    routes6: String,
    dev: String,
    links: Vec<Link>,
}

struct BlockDevice {
    name: String,
    identity: String,
    parent: Option<String>,
    slaves: Vec<String>,
    held: bool,
    synthetic: bool,
    counters: Result<Counters, UnavailableReason>,
}

struct FsNumbers {
    blocks: u64,
    free: u64,
    available: u64,
    fragment_size: u64,
}

struct OpenVolume {
    mount_id: u64,
    numbers: Option<FsNumbers>,
}

struct VolumeFrame {
    mountinfo: String,
    root: Result<OpenVolume, UnavailableReason>,
    home: Result<OpenVolume, UnavailableReason>,
}

trait IoProbe: Send + 'static {
    fn network(&mut self) -> io::Result<NetworkFrame>;
    fn block(&mut self) -> io::Result<Vec<BlockDevice>>;
    fn volumes(&mut self) -> io::Result<VolumeFrame>;
}

struct LinuxIoProbe;

impl IoProbe for LinuxIoProbe {
    fn network(&mut self) -> io::Result<NetworkFrame> {
        let routes4 = match fs::read_to_string("/proc/net/route") {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
            Err(error) => return Err(error),
        };
        // IPv6 can be disabled; an absent file simply leaves the IPv4 or
        // active-link fallback available.
        let routes6 = match fs::read_to_string("/proc/net/ipv6_route") {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
            Err(error) => return Err(error),
        };
        let dev = fs::read_to_string("/proc/net/dev")?;
        let mut links = Vec::new();
        for entry in fs::read_dir("/sys/class/net")? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let ifindex = read_number(entry.path().join("ifindex"))?;
            let flags = fs::read_to_string(entry.path().join("flags"))?;
            let flags = u32::from_str_radix(flags.trim().trim_start_matches("0x"), 16)
                .map_err(|_| invalid_input("invalid interface flags"))?;
            links.push(Link {
                name,
                ifindex,
                up: flags & (libc::IFF_UP as u32) != 0,
            });
        }
        Ok(NetworkFrame {
            routes4,
            routes6,
            dev,
            links,
        })
    }

    fn block(&mut self) -> io::Result<Vec<BlockDevice>> {
        let mut devices = Vec::new();
        for entry in fs::read_dir("/sys/class/block")? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            let identity = fs::read_to_string(path.join("dev"))?.trim().to_owned();
            let major: u32 = identity
                .split(':')
                .next()
                .and_then(|part| part.parse().ok())
                .ok_or_else(|| invalid_input("invalid block device identity"))?;
            let parent = if path.join("partition").try_exists()? {
                fs::canonicalize(&path)?
                    .parent()
                    .and_then(|path| path.file_name())
                    .map(|name| name.to_string_lossy().into_owned())
            } else {
                None
            };
            let held = match fs::read_dir(path.join("holders"))?.next() {
                Some(Ok(_)) => true,
                Some(Err(error)) => return Err(error),
                None => false,
            };
            let slaves = match fs::read_dir(path.join("slaves")) {
                Ok(entries) => entries
                    .map(|entry| {
                        entry.map(|entry| entry.file_name().to_string_lossy().into_owned())
                    })
                    .collect::<io::Result<Vec<_>>>()?,
                Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
                Err(error) => return Err(error),
            };
            let synthetic = major == 1
                || path.join("loop").try_exists()?
                || path.join("comp_algorithm").try_exists()?;
            let counters = fs::read_to_string(path.join("stat"))
                .map_err(|error| reason_for_io(&error))
                .and_then(|text| parse_block_stat(&text).ok_or(UnavailableReason::InvalidData));
            devices.push(BlockDevice {
                name,
                identity,
                parent,
                slaves,
                held,
                synthetic,
                counters,
            });
        }
        Ok(devices)
    }

    fn volumes(&mut self) -> io::Result<VolumeFrame> {
        let mountinfo = fs::read_to_string("/proc/self/mountinfo")?;
        let root = open_volume("/", &mountinfo).map_err(|error| reason_for_io(&error));
        let home = open_volume("/home", &mountinfo).map_err(|error| reason_for_io(&error));
        Ok(VolumeFrame {
            mountinfo,
            root,
            home,
        })
    }
}

fn read_number(path: PathBuf) -> io::Result<u32> {
    fs::read_to_string(path)?
        .trim()
        .parse()
        .map_err(|_| invalid_input("invalid numeric sysfs value"))
}

fn invalid_input(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn open_volume(path: &str, mountinfo: &str) -> io::Result<OpenVolume> {
    let path = CString::new(path).map_err(|_| invalid_input("invalid volume path"))?;
    // O_PATH needs search permission on ancestors, but not read permission on
    // the mounted directory itself. Keep one descriptor for identity and size.
    let raw_fd = unsafe {
        libc::open(
            path.as_ptr(),
            libc::O_PATH | libc::O_DIRECTORY | libc::O_CLOEXEC,
        )
    };
    if raw_fd < 0 {
        return Err(io::Error::last_os_error());
    }
    let file = unsafe { OwnedFd::from_raw_fd(raw_fd) };
    let fdinfo = fs::read_to_string(format!("/proc/self/fdinfo/{}", file.as_raw_fd()))?;
    let mount_id = fdinfo
        .lines()
        .find_map(|line| {
            line.strip_prefix("mnt_id:")
                .and_then(|value| value.trim().parse().ok())
        })
        .ok_or_else(|| invalid_input("missing mount ID"))?;
    let identity = mount_identity(mountinfo, mount_id)
        .ok_or_else(|| invalid_input("missing effective mount identity"))?;
    // Network and unclassified filesystems may block inside fstatvfs. Only
    // ask eligible local storage for its capacity.
    let numbers = if mount_class(&identity.fs_type) == MountClass::Storage {
        let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
        if unsafe { libc::fstatvfs(file.as_raw_fd(), &mut stat) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Some(FsNumbers {
            blocks: stat.f_blocks,
            free: stat.f_bfree,
            available: stat.f_bavail,
            fragment_size: stat.f_frsize,
        })
    } else {
        None
    };
    Ok(OpenVolume { mount_id, numbers })
}

fn reason_for_io(error: &io::Error) -> UnavailableReason {
    match error.kind() {
        io::ErrorKind::NotFound => UnavailableReason::Missing,
        io::ErrorKind::PermissionDenied => UnavailableReason::PermissionDenied,
        io::ErrorKind::InvalidData => UnavailableReason::InvalidData,
        _ => UnavailableReason::SourceError,
    }
}

fn parse_net_dev(text: &str, name: &str) -> Option<Counters> {
    let (_, fields) = text.lines().find_map(|line| {
        let (candidate, fields) = line.rsplit_once(':')?;
        (candidate.trim() == name).then_some((candidate, fields))
    })?;
    let fields: Vec<_> = fields.split_whitespace().collect();
    if fields.len() < 16 {
        return None;
    }
    Some(Counters {
        down: fields[0].parse().ok(),
        up: fields[8].parse().ok(),
    })
}

fn route4(text: &str) -> Vec<(u64, String)> {
    text.lines()
        .skip(1)
        .filter_map(|line| {
            let fields: Vec<_> = line.split_whitespace().collect();
            if fields.len() < 8 || fields[1] != "00000000" || fields[7] != "00000000" {
                return None;
            }
            let flags = u32::from_str_radix(fields[3], 16).ok()?;
            if flags & 1 == 0 {
                return None;
            }
            Some((fields[6].parse().ok()?, fields[0].to_owned()))
        })
        .collect()
}

fn route6(text: &str) -> Vec<(u64, String)> {
    text.lines()
        .filter_map(|line| {
            let fields: Vec<_> = line.split_whitespace().collect();
            if fields.len() < 10 || fields[0] != "0".repeat(32) || fields[1] != "00" {
                return None;
            }
            Some((
                u64::from_str_radix(fields[5], 16).ok()?,
                fields[9].to_owned(),
            ))
        })
        .collect()
}

fn selected_link(frame: &NetworkFrame) -> Option<&Link> {
    let eligible = |name: &str| {
        frame
            .links
            .iter()
            .find(|link| link.name == name && link.name != "lo" && link.up)
    };
    for routes in [route4(&frame.routes4), route6(&frame.routes6)] {
        if let Some((_, link)) = routes
            .into_iter()
            .filter(|(_, name)| eligible(name).is_some())
            .min_by(|a, b| a.cmp(b))
        {
            return eligible(&link);
        }
    }
    frame
        .links
        .iter()
        .filter(|link| link.name != "lo" && link.up)
        .min_by(|a, b| a.name.cmp(&b.name))
}

fn parse_block_stat(text: &str) -> Option<Counters> {
    let fields: Vec<_> = text.split_whitespace().collect();
    if fields.len() < 7 {
        return None;
    }
    Some(Counters {
        down: fields[2]
            .parse::<u64>()
            .ok()
            .and_then(|value| value.checked_mul(512)),
        up: fields[6]
            .parse::<u64>()
            .ok()
            .and_then(|value| value.checked_mul(512)),
    })
}

fn selected_blocks(devices: &[BlockDevice]) -> Vec<&BlockDevice> {
    let partitioned: HashSet<&str> = devices
        .iter()
        .filter_map(|device| device.parent.as_deref())
        .collect();
    let by_name: HashMap<&str, &BlockDevice> = devices
        .iter()
        .map(|device| (device.name.as_str(), device))
        .collect();
    let mut selected: Vec<_> = devices
        .iter()
        .filter(|device| {
            !device.held
                && !partitioned.contains(device.name.as_str())
                && !has_synthetic_ancestor(device, &by_name, &mut HashSet::new())
        })
        .collect();
    selected.sort_by(|a, b| a.identity.cmp(&b.identity).then(a.name.cmp(&b.name)));
    selected
}

fn mixed_synthetic_stack(devices: &[BlockDevice]) -> bool {
    let by_name: HashMap<&str, &BlockDevice> = devices
        .iter()
        .map(|device| (device.name.as_str(), device))
        .collect();
    devices.iter().any(|device| {
        !device.held
            && !device.slaves.is_empty()
            && has_synthetic_ancestor(device, &by_name, &mut HashSet::new())
            && has_real_backing(device, &by_name, &mut HashSet::new())
    })
}

fn has_real_backing<'a>(
    device: &'a BlockDevice,
    by_name: &HashMap<&str, &'a BlockDevice>,
    visited: &mut HashSet<&'a str>,
) -> bool {
    if device.synthetic || !visited.insert(&device.name) {
        return false;
    }
    if device.slaves.is_empty() {
        return true;
    }
    device
        .slaves
        .iter()
        .filter_map(|name| by_name.get(name.as_str()))
        .any(|slave| has_real_backing(slave, by_name, visited))
}

fn has_synthetic_ancestor<'a>(
    device: &'a BlockDevice,
    by_name: &HashMap<&str, &'a BlockDevice>,
    visited: &mut HashSet<&'a str>,
) -> bool {
    if device.synthetic {
        return true;
    }
    if !visited.insert(&device.name) {
        return false;
    }
    device
        .parent
        .iter()
        .chain(device.slaves.iter())
        .filter_map(|name| by_name.get(name.as_str()))
        .any(|ancestor| has_synthetic_ancestor(ancestor, by_name, visited))
}

struct MountIdentity {
    device: String,
    fs_type: String,
    source: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MountClass {
    Storage,
    Excluded,
    Unknown,
}

fn mount_class(fs_type: &str) -> MountClass {
    match fs_type {
        "ext2" | "ext3" | "ext4" | "btrfs" | "xfs" | "f2fs" | "zfs" | "bcachefs" | "vfat"
        | "exfat" | "ntfs" | "ntfs3" | "erofs" | "squashfs" | "iso9660" | "udf" => {
            MountClass::Storage
        }
        "nfs" | "nfs4" | "cifs" | "smb3" | "9p" | "ceph" | "glusterfs" | "afs" | "coda"
        | "orangefs" | "lustre" | "fuse.sshfs" | "fuse.rclone" | "fuse.davfs"
        | "fuse.glusterfs" | "fuse.ceph" | "tmpfs" | "ramfs" | "devtmpfs" | "hugetlbfs"
        | "proc" | "sysfs" => MountClass::Excluded,
        _ => MountClass::Unknown,
    }
}

fn mount_identity(text: &str, id: u64) -> Option<MountIdentity> {
    text.lines().find_map(|line| {
        let (left, right) = line.split_once(" - ")?;
        let fields: Vec<_> = left.split_whitespace().collect();
        let other: Vec<_> = right.split_whitespace().collect();
        if fields.len() < 5 || other.len() < 2 || fields[0].parse::<u64>().ok()? != id {
            return None;
        }
        Some(MountIdentity {
            device: fields[2].to_owned(),
            fs_type: other[0].to_owned(),
            source: other[1].to_owned(),
        })
    })
}

fn volume_numbers(numbers: &FsNumbers) -> Option<(u64, u64, u64)> {
    if numbers.fragment_size == 0
        || numbers.blocks == 0
        || numbers.available > numbers.free
        || numbers.free > numbers.blocks
    {
        return None;
    }
    Some((
        numbers.available.checked_mul(numbers.fragment_size)?,
        numbers
            .blocks
            .checked_sub(numbers.free)?
            .checked_mul(numbers.fragment_size)?,
        numbers.blocks.checked_mul(numbers.fragment_size)?,
    ))
}

fn selected_volume(
    mountinfo: &str,
    path: &str,
    opened: Result<OpenVolume, UnavailableReason>,
) -> Result<Option<(MountIdentity, PrimaryVolume)>, UnavailableReason> {
    let opened = opened?;
    let identity =
        mount_identity(mountinfo, opened.mount_id).ok_or(UnavailableReason::InvalidData)?;
    match mount_class(&identity.fs_type) {
        MountClass::Excluded => return Ok(None),
        MountClass::Unknown => return Err(UnavailableReason::InvalidData),
        MountClass::Storage => {}
    }
    let numbers = opened.numbers.ok_or(UnavailableReason::InvalidData)?;
    let (available_bytes, used_bytes, total_bytes) =
        volume_numbers(&numbers).ok_or(UnavailableReason::InvalidData)?;
    Ok(Some((
        identity,
        PrimaryVolume {
            paths: vec![path.to_owned()],
            available_bytes,
            used_bytes,
            total_bytes,
        },
    )))
}

fn primary_volumes(frame: VolumeFrame) -> Reading<Vec<PrimaryVolume>> {
    let mut volumes: Vec<(MountIdentity, PrimaryVolume)> = Vec::new();
    let mut first_failure = None;
    for (path, opened) in [("/", frame.root), ("/home", frame.home)] {
        match selected_volume(&frame.mountinfo, path, opened) {
            Ok(Some((identity, volume))) => {
                if let Some((_, prior)) = volumes.iter_mut().find(|(prior_id, _)| {
                    prior_id.device == identity.device
                        && prior_id.fs_type == identity.fs_type
                        && prior_id.source == identity.source
                }) {
                    prior.paths.push(path.to_owned());
                } else {
                    volumes.push((identity, volume));
                }
            }
            Ok(None) => {}
            Err(reason) => {
                first_failure.get_or_insert(reason);
            }
        }
    }
    if volumes.is_empty() {
        Reading::Unavailable(first_failure.unwrap_or(UnavailableReason::Missing))
    } else {
        Reading::Value(volumes.into_iter().map(|(_, volume)| volume).collect())
    }
}

fn rate(current: u64, prior: u64, elapsed: Duration, prior_at: Duration) -> Reading<f64> {
    if elapsed <= prior_at || elapsed - prior_at > MAX_INTERVAL {
        return Reading::Unavailable(UnavailableReason::InvalidData);
    }
    match current.checked_sub(prior) {
        Some(delta) => Reading::Value(delta as f64 / (elapsed - prior_at).as_secs_f64()),
        None => Reading::Unavailable(UnavailableReason::InvalidData),
    }
}

fn direction_rate(
    current: Option<u64>,
    previous: Option<(u64, Duration)>,
    elapsed: Duration,
) -> Reading<f64> {
    let Some(current) = current else {
        return Reading::Unavailable(UnavailableReason::InvalidData);
    };
    match previous {
        Some((prior, prior_at)) => rate(current, prior, elapsed, prior_at),
        None => Reading::Unavailable(UnavailableReason::WarmingUp),
    }
}

pub(super) struct IoSampler {
    probe: Box<dyn IoProbe>,
    network_previous: Option<(String, Counters, Duration)>,
    block_previous: HashMap<String, (Counters, Duration)>,
}

impl IoSampler {
    pub(super) fn system() -> Self {
        Self::new(LinuxIoProbe)
    }

    fn new(probe: impl IoProbe) -> Self {
        Self {
            probe: Box::new(probe),
            network_previous: None,
            block_previous: HashMap::new(),
        }
    }

    fn tick(&mut self, elapsed: Duration) -> IoPatch {
        let network = self.network(elapsed);
        let (read_bytes_per_second, write_bytes_per_second) = self.block(elapsed);
        let primary_volumes = match self.probe.volumes() {
            Ok(frame) => primary_volumes(frame),
            Err(error) => Reading::Unavailable(reason_for_io(&error)),
        };
        IoPatch {
            network,
            storage: StorageSnapshot {
                read_bytes_per_second,
                write_bytes_per_second,
                primary_volumes,
            },
        }
    }

    fn network(&mut self, elapsed: Duration) -> NetworkSnapshot {
        let unavailable = |reason| NetworkSnapshot {
            down_bytes_per_second: Reading::Unavailable(reason),
            up_bytes_per_second: Reading::Unavailable(reason),
        };
        let frame = match self.probe.network() {
            Ok(frame) => frame,
            Err(error) => {
                self.network_previous = None;
                return unavailable(reason_for_io(&error));
            }
        };
        let Some(link) = selected_link(&frame) else {
            self.network_previous = None;
            return unavailable(UnavailableReason::Missing);
        };
        let Some(current) = parse_net_dev(&frame.dev, &link.name) else {
            self.network_previous = None;
            return unavailable(UnavailableReason::InvalidData);
        };
        let identity = format!("{}:{}", link.name, link.ifindex);
        let previous = self
            .network_previous
            .as_ref()
            .and_then(|(id, counters, at)| (id == &identity).then_some((counters, *at)));
        let result = NetworkSnapshot {
            down_bytes_per_second: direction_rate(
                current.down,
                previous.and_then(|(counters, at)| counters.down.map(|value| (value, at))),
                elapsed,
            ),
            up_bytes_per_second: direction_rate(
                current.up,
                previous.and_then(|(counters, at)| counters.up.map(|value| (value, at))),
                elapsed,
            ),
        };
        self.network_previous = Some((identity, current, elapsed));
        result
    }

    fn block(&mut self, elapsed: Duration) -> (Reading<f64>, Reading<f64>) {
        let devices = match self.probe.block() {
            Ok(devices) => devices,
            Err(error) => {
                self.block_previous.clear();
                let reason = reason_for_io(&error);
                return (Reading::Unavailable(reason), Reading::Unavailable(reason));
            }
        };
        if mixed_synthetic_stack(&devices) {
            self.block_previous.clear();
            return (
                Reading::Unavailable(UnavailableReason::InvalidData),
                Reading::Unavailable(UnavailableReason::InvalidData),
            );
        }
        let selected = selected_blocks(&devices);
        if selected.is_empty() {
            self.block_previous.clear();
            return (
                Reading::Unavailable(UnavailableReason::Missing),
                Reading::Unavailable(UnavailableReason::Missing),
            );
        }
        let mut next = HashMap::with_capacity(selected.len());
        let mut down = 0.0;
        let mut up = 0.0;
        let mut down_error = None;
        let mut up_error = None;
        for device in selected {
            let current = match device.counters {
                Ok(counters) => counters,
                Err(reason) => {
                    down_error.get_or_insert(reason);
                    up_error.get_or_insert(reason);
                    continue;
                }
            };
            let identity = format!("{}:{}", device.name, device.identity);
            let previous = self.block_previous.get(&identity);
            match direction_rate(
                current.down,
                previous.and_then(|(prior, at)| prior.down.map(|value| (value, *at))),
                elapsed,
            ) {
                Reading::Value(value) => down += value,
                Reading::Unavailable(reason) => {
                    down_error.get_or_insert(reason);
                }
            }
            match direction_rate(
                current.up,
                previous.and_then(|(prior, at)| prior.up.map(|value| (value, *at))),
                elapsed,
            ) {
                Reading::Value(value) => up += value,
                Reading::Unavailable(reason) => {
                    up_error.get_or_insert(reason);
                }
            }
            next.insert(identity, (current, elapsed));
        }
        self.block_previous = next;
        (
            down_error.map_or(Reading::Value(down), Reading::Unavailable),
            up_error.map_or(Reading::Value(up), Reading::Unavailable),
        )
    }
}

impl Provider for IoSampler {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Io
    }

    fn sample(&mut self, elapsed: Duration, _stop: &StopToken) -> Patch {
        Patch::Io(self.tick(elapsed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::{SessionBuilder, Snapshot};
    use std::collections::VecDeque;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Instant;

    struct Frames {
        network: VecDeque<io::Result<NetworkFrame>>,
        block: VecDeque<io::Result<Vec<BlockDevice>>>,
        volumes: VecDeque<io::Result<VolumeFrame>>,
    }

    impl IoProbe for Frames {
        fn network(&mut self) -> io::Result<NetworkFrame> {
            self.network.pop_front().expect("network frame")
        }
        fn block(&mut self) -> io::Result<Vec<BlockDevice>> {
            self.block.pop_front().expect("block frame")
        }
        fn volumes(&mut self) -> io::Result<VolumeFrame> {
            self.volumes.pop_front().expect("volume frame")
        }
    }

    fn network_frame(default: &str, eno: (u64, u64), vpn: (u64, u64)) -> NetworkFrame {
        NetworkFrame {
            routes4: format!(
                "Iface Destination Gateway Flags RefCnt Use Metric Mask MTU Window IRTT\n{default} 00000000 00000000 0001 0 0 100 00000000 0 0 0\n"
            ),
            routes6: String::new(),
            dev: format!(
                "Inter-| Receive | Transmit\nface |bytes packets errs drop fifo frame compressed multicast|bytes packets errs drop fifo colls carrier compressed\n eno2: {} 0 0 0 0 0 0 0 {} 0 0 0 0 0 0 0\n wg0: {} 0 0 0 0 0 0 0 {} 0 0 0 0 0 0 0\n",
                eno.0, eno.1, vpn.0, vpn.1
            ),
            links: vec![
                Link {
                    name: "eno2".into(),
                    ifindex: 2,
                    up: true,
                },
                Link {
                    name: "wg0".into(),
                    ifindex: 3,
                    up: true,
                },
            ],
        }
    }

    fn block_device(
        name: &str,
        id: &str,
        parent: Option<&str>,
        held: bool,
        down: u64,
        up: u64,
    ) -> BlockDevice {
        BlockDevice {
            name: name.into(),
            identity: id.into(),
            parent: parent.map(str::to_owned),
            slaves: Vec::new(),
            held,
            synthetic: false,
            counters: Ok(Counters::both(down, up)),
        }
    }

    fn mounts(root_id: u64, home_id: u64, home_source: &str, home_dev: &str) -> VolumeFrame {
        VolumeFrame {
            mountinfo: format!(
                "43 1 0:35 /root / rw - btrfs /dev/nvme0n1p3 rw\n63 43 {home_dev} /home /home rw - btrfs {home_source} rw\n80 43 0:50 / /home2 rw - ext4 /dev/other rw\n"
            ),
            root: Ok(OpenVolume {
                mount_id: root_id,
                numbers: Some(FsNumbers {
                    blocks: 100,
                    free: 30,
                    available: 20,
                    fragment_size: 4096,
                }),
            }),
            home: Ok(OpenVolume {
                mount_id: home_id,
                numbers: Some(FsNumbers {
                    blocks: 100,
                    free: 30,
                    available: 20,
                    fragment_size: 4096,
                }),
            }),
        }
    }

    fn numbers_mut(opened: &mut Result<OpenVolume, UnavailableReason>) -> &mut FsNumbers {
        opened.as_mut().unwrap().numbers.as_mut().unwrap()
    }

    fn mount_type(frame: &mut VolumeFrame, id: u64, fs_type: &str, source: &str) {
        frame.mountinfo = frame
            .mountinfo
            .lines()
            .map(|line| {
                if line.starts_with(&format!("{id} ")) {
                    let (left, _) = line.split_once(" - ").unwrap();
                    format!("{left} - {fs_type} {source} rw")
                } else {
                    line.to_owned()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
    }

    fn minimal_blocks(down: u64, up: u64) -> Vec<BlockDevice> {
        vec![block_device("nvme0n1", "259:0", None, false, down, up)]
    }

    fn frames(
        network: Vec<NetworkFrame>,
        block: Vec<Vec<BlockDevice>>,
        volume_count: usize,
    ) -> Frames {
        Frames {
            network: network.into_iter().map(Ok).collect(),
            block: block.into_iter().map(Ok).collect(),
            volumes: (0..volume_count)
                .map(|_| Ok(mounts(43, 63, "/dev/nvme0n1p3", "0:35")))
                .collect(),
        }
    }

    #[test]
    fn selected_route_is_one_logical_link_and_rebaselines_on_change_reset_and_gap() {
        let mut sampler = IoSampler::new(frames(
            vec![
                network_frame("eno2", (100, 200), (1000, 2000)),
                network_frame("eno2", (150, 230), (9000, 9000)),
                network_frame("wg0", (200, 300), (9100, 9050)),
                network_frame("wg0", (250, 350), (9150, 9050)),
                network_frame("wg0", (300, 400), (10, 10)),
                network_frame("wg0", (350, 450), (20, 20)),
                network_frame("wg0", (400, 500), (30, 30)),
                network_frame("wg0", (450, 550), (40, 40)),
            ],
            (0..8).map(|i| minimal_blocks(i * 100, i * 100)).collect(),
            8,
        ));
        assert_eq!(
            sampler.tick(Duration::ZERO).network.down_bytes_per_second,
            Reading::Unavailable(UnavailableReason::WarmingUp)
        );
        let second = sampler.tick(Duration::from_secs(1));
        assert_eq!(second.network.down_bytes_per_second, Reading::Value(50.0));
        assert_eq!(second.network.up_bytes_per_second, Reading::Value(30.0));
        assert_eq!(
            sampler
                .tick(Duration::from_secs(2))
                .network
                .down_bytes_per_second,
            Reading::Unavailable(UnavailableReason::WarmingUp)
        );
        let vpn = sampler.tick(Duration::from_secs(3));
        assert_eq!(vpn.network.down_bytes_per_second, Reading::Value(50.0));
        assert_eq!(vpn.network.up_bytes_per_second, Reading::Value(0.0));
        assert_eq!(
            sampler
                .tick(Duration::from_secs(4))
                .network
                .down_bytes_per_second,
            Reading::Unavailable(UnavailableReason::InvalidData)
        );
        assert_eq!(
            sampler
                .tick(Duration::from_secs(5))
                .network
                .down_bytes_per_second,
            Reading::Value(10.0)
        );
        assert_eq!(
            sampler
                .tick(Duration::from_secs(30))
                .network
                .down_bytes_per_second,
            Reading::Unavailable(UnavailableReason::InvalidData)
        );
        assert_eq!(
            sampler
                .tick(Duration::from_secs(31))
                .network
                .down_bytes_per_second,
            Reading::Value(10.0)
        );
    }

    #[test]
    fn ipv6_default_and_deterministic_fallback_do_not_sum_links() {
        let mut frame = network_frame("eno2", (10, 10), (100, 100));
        frame.routes4.clear();
        frame.routes6 = format!(
            "{} 00 {} 00 {} 00000014 00000000 00000000 00000001 wg0\n",
            "0".repeat(32),
            "0".repeat(32),
            "0".repeat(32)
        );
        assert_eq!(selected_link(&frame).unwrap().name, "wg0");
        frame.routes6.clear();
        assert_eq!(selected_link(&frame).unwrap().name, "eno2");
        frame.links[0].up = false;
        assert_eq!(selected_link(&frame).unwrap().name, "wg0");
        frame.links[1].up = false;
        assert!(selected_link(&frame).is_none());
    }

    #[test]
    fn lowest_default_metric_wins_with_name_tie_break_and_bad_rows_are_not_zero() {
        let mut frame = network_frame("wg0", (10, 10), (100, 100));
        frame
            .routes4
            .push_str("eno2 00000000 00000000 0001 0 0 50 00000000 0 0 0\n");
        assert_eq!(selected_link(&frame).unwrap().name, "eno2");
        frame
            .routes4
            .push_str("wg0 00000000 00000000 0001 0 0 50 00000000 0 0 0\n");
        assert_eq!(selected_link(&frame).unwrap().name, "eno2");
        assert_eq!(parse_net_dev(&frame.dev, "eno2").unwrap().down, Some(10));
        frame.dev = "eno2: broken\n".into();
        assert!(parse_net_dev(&frame.dev, "eno2").is_none());
        assert!(parse_block_stat("1 2 3 4 5 6 7").is_some());
        let partial = parse_block_stat("1 2 nope 4 5 6 7").unwrap();
        assert_eq!(partial.down, None);
        assert_eq!(partial.up, Some(7 * 512));
        let overflow = parse_block_stat(&format!("1 2 {} 4 5 6 7", u64::MAX)).unwrap();
        assert_eq!(overflow.down, None);
        assert_eq!(overflow.up, Some(7 * 512));
    }

    #[test]
    fn leaf_block_layer_excludes_parent_and_held_backing_but_keeps_independent_disk() {
        let make = |offset: u64| {
            vec![
                block_device("sda", "8:0", None, false, 1000 + offset, 2000 + offset),
                block_device(
                    "sda1",
                    "8:1",
                    Some("sda"),
                    true,
                    800 + offset,
                    1800 + offset,
                ),
                block_device("dm-0", "253:0", None, false, 700 + offset, 1700 + offset),
                block_device("sdb", "8:16", None, false, 300 + offset, 400 + offset),
            ]
        };
        let mut sampler = IoSampler::new(frames(
            vec![network_frame("eno2", (0, 0), (0, 0)); 2],
            vec![make(0), make(100)],
            2,
        ));
        let first = sampler.tick(Duration::ZERO);
        assert_eq!(
            first.storage.read_bytes_per_second,
            Reading::Unavailable(UnavailableReason::WarmingUp)
        );
        let next = sampler.tick(Duration::from_secs(1));
        assert_eq!(next.storage.read_bytes_per_second, Reading::Value(200.0));
        assert_eq!(next.storage.write_bytes_per_second, Reading::Value(200.0));
    }

    #[test]
    fn loop_backed_partition_and_mapper_are_not_counted_with_host_storage() {
        let mut loop_disk = block_device("loop0", "7:0", None, false, 100, 100);
        loop_disk.synthetic = true;
        let partition = block_device("loop0p1", "7:1", Some("loop0"), true, 100, 100);
        let mut mapper = block_device("dm-0", "253:0", None, false, 100, 100);
        mapper.slaves.push("loop0p1".into());
        let physical = block_device("nvme0n1", "259:0", None, false, 100, 100);
        let devices = vec![loop_disk, partition, mapper, physical];
        let selected = selected_blocks(&devices);
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].name, "nvme0n1");
    }

    #[test]
    fn mixed_loop_and_real_backing_is_unavailable_instead_of_undercounted() {
        let mut loop_disk = block_device("loop0", "7:0", None, true, 100, 100);
        loop_disk.synthetic = true;
        let real_disk = block_device("sda", "8:0", None, false, 100, 100);
        let real_partition = block_device("sda1", "8:1", Some("sda"), true, 100, 100);
        let mut mapper = block_device("dm-0", "253:0", None, false, 100, 100);
        mapper.slaves = vec!["loop0".into(), "sda1".into()];
        let devices = vec![loop_disk, real_disk, real_partition, mapper];
        assert!(mixed_synthetic_stack(&devices));
        let mut sampler = IoSampler::new(frames(
            vec![network_frame("eno2", (0, 0), (0, 0))],
            vec![devices],
            1,
        ));
        let patch = sampler.tick(Duration::ZERO);
        assert_eq!(
            patch.storage.read_bytes_per_second,
            Reading::Unavailable(UnavailableReason::InvalidData)
        );
        assert!(matches!(patch.storage.primary_volumes, Reading::Value(_)));
    }

    #[test]
    fn block_churn_and_error_are_unavailable_then_recover_without_old_state() {
        let mut broken = block_device("sdb", "8:16", None, false, 0, 0);
        broken.counters = Err(UnavailableReason::PermissionDenied);
        let mut sampler = IoSampler::new(frames(
            vec![network_frame("eno2", (0, 0), (0, 0)); 7],
            vec![
                minimal_blocks(0, 0),
                minimal_blocks(512, 1024),
                vec![block_device("sdb", "8:16", None, false, 100, 100)],
                vec![block_device("sdb", "8:16", None, false, 100, 100)],
                vec![broken],
                vec![block_device("sdb", "8:16", None, false, 200, 200)],
                vec![block_device("sdb", "8:16", None, false, 300, 300)],
            ],
            7,
        ));
        sampler.tick(Duration::ZERO);
        assert_eq!(
            sampler
                .tick(Duration::from_secs(1))
                .storage
                .write_bytes_per_second,
            Reading::Value(1024.0)
        );
        assert_eq!(
            sampler
                .tick(Duration::from_secs(2))
                .storage
                .read_bytes_per_second,
            Reading::Unavailable(UnavailableReason::WarmingUp)
        );
        assert_eq!(
            sampler
                .tick(Duration::from_secs(3))
                .storage
                .read_bytes_per_second,
            Reading::Value(0.0)
        );
        assert_eq!(
            sampler
                .tick(Duration::from_secs(4))
                .storage
                .read_bytes_per_second,
            Reading::Unavailable(UnavailableReason::PermissionDenied)
        );
        assert_eq!(
            sampler
                .tick(Duration::from_secs(5))
                .storage
                .read_bytes_per_second,
            Reading::Unavailable(UnavailableReason::WarmingUp)
        );
        assert_eq!(
            sampler
                .tick(Duration::from_secs(6))
                .storage
                .read_bytes_per_second,
            Reading::Value(100.0)
        );
        assert_eq!(sampler.block_previous.len(), 1);
    }

    #[test]
    fn thousands_of_device_replacements_and_failures_do_not_retain_old_baselines() {
        struct ChangingDevices {
            sample: u64,
        }

        impl IoProbe for ChangingDevices {
            fn network(&mut self) -> io::Result<NetworkFrame> {
                Ok(network_frame("eno2", (self.sample, self.sample), (0, 0)))
            }

            fn block(&mut self) -> io::Result<Vec<BlockDevice>> {
                let sample = self.sample;
                self.sample += 1;
                if sample % 1000 == 999 {
                    return Err(io::ErrorKind::PermissionDenied.into());
                }
                Ok(vec![block_device(
                    "disk",
                    &format!("8:{}", sample / 2),
                    None,
                    false,
                    sample * 100,
                    sample * 200,
                )])
            }

            fn volumes(&mut self) -> io::Result<VolumeFrame> {
                Ok(mounts(43, 63, "/dev/nvme0n1p3", "0:35"))
            }
        }

        let mut sampler = IoSampler::new(ChangingDevices { sample: 0 });
        for sample in 0..10_000 {
            let patch = sampler.tick(Duration::from_secs(sample));
            let expected = if sample % 1000 == 999 {
                Reading::Unavailable(UnavailableReason::PermissionDenied)
            } else if sample % 2 == 0 {
                Reading::Unavailable(UnavailableReason::WarmingUp)
            } else {
                Reading::Value(100.0)
            };
            assert_eq!(patch.storage.read_bytes_per_second, expected);
            assert!(sampler.block_previous.len() <= 1);
            assert!(matches!(patch.storage.primary_volumes, Reading::Value(_)));
        }
        assert!(sampler.block_previous.is_empty());
    }

    #[test]
    fn source_failure_clears_only_its_baseline_and_invalid_time_never_spikes() {
        let mut source = frames(
            vec![
                network_frame("eno2", (100, 100), (0, 0)),
                network_frame("eno2", (200, 200), (0, 0)),
                network_frame("eno2", (300, 300), (0, 0)),
                network_frame("eno2", (400, 400), (0, 0)),
            ],
            vec![
                minimal_blocks(0, 0),
                minimal_blocks(100, 100),
                minimal_blocks(200, 200),
                minimal_blocks(300, 300),
                minimal_blocks(400, 400),
            ],
            5,
        );
        source
            .network
            .insert(1, Err(io::ErrorKind::PermissionDenied.into()));
        source
            .volumes
            .insert(3, Err(io::ErrorKind::PermissionDenied.into()));
        let mut sampler = IoSampler::new(source);
        sampler.tick(Duration::ZERO);
        let failed = sampler.tick(Duration::from_secs(1));
        assert_eq!(
            failed.network.down_bytes_per_second,
            Reading::Unavailable(UnavailableReason::PermissionDenied)
        );
        assert_eq!(failed.storage.read_bytes_per_second, Reading::Value(100.0));
        assert!(matches!(failed.storage.primary_volumes, Reading::Value(_)));
        assert_eq!(
            sampler
                .tick(Duration::from_secs(2))
                .network
                .down_bytes_per_second,
            Reading::Unavailable(UnavailableReason::WarmingUp)
        );
        let bad_time = sampler.tick(Duration::from_secs(2));
        assert_eq!(
            bad_time.network.down_bytes_per_second,
            Reading::Unavailable(UnavailableReason::InvalidData)
        );
        assert_eq!(
            bad_time.storage.read_bytes_per_second,
            Reading::Unavailable(UnavailableReason::InvalidData)
        );
        assert_eq!(
            bad_time.storage.primary_volumes,
            Reading::Unavailable(UnavailableReason::PermissionDenied)
        );
        let recovered = sampler.tick(Duration::from_secs(3));
        assert_eq!(
            recovered.network.down_bytes_per_second,
            Reading::Value(100.0)
        );
        assert_eq!(
            recovered.storage.read_bytes_per_second,
            Reading::Value(100.0)
        );
        assert!(matches!(
            recovered.storage.primary_volumes,
            Reading::Value(_)
        ));
    }

    #[test]
    fn primary_capacity_deduplicates_btrfs_and_preserves_reserved_blocks() {
        let Reading::Value(shared) = primary_volumes(mounts(43, 63, "/dev/nvme0n1p3", "0:35"))
        else {
            panic!("shared volume");
        };
        assert_eq!(shared.len(), 1);
        assert_eq!(shared[0].paths, ["/", "/home"]);
        assert_eq!(shared[0].available_bytes, 20 * 4096);
        assert_eq!(shared[0].used_bytes, 70 * 4096);
        assert_eq!(shared[0].total_bytes, 100 * 4096);
        let Reading::Value(distinct) = primary_volumes(mounts(43, 63, "/dev/sdb1", "8:17")) else {
            panic!("distinct volumes");
        };
        assert_eq!(distinct.len(), 2);
        assert_eq!(distinct[0].paths, ["/"]);
        assert_eq!(distinct[1].paths, ["/home"]);
        let mut ext4_home = mounts(43, 63, "/dev/sdb1", "8:17");
        mount_type(&mut ext4_home, 63, "ext4", "/dev/sdb1");
        assert!(
            matches!(primary_volumes(ext4_home), Reading::Value(ref volumes) if volumes.len() == 2 && volumes[1].paths == ["/home"])
        );
        let mut absent = mounts(43, 99, "/dev/sdb1", "8:17");
        assert!(
            matches!(primary_volumes(absent), Reading::Value(ref volumes) if volumes.len() == 1 && volumes[0].paths == ["/"])
        );
        absent = mounts(43, 63, "/dev/nvme0n1p3", "0:35");
        numbers_mut(&mut absent.home).available = 31;
        assert!(
            matches!(primary_volumes(absent), Reading::Value(ref volumes) if volumes.len() == 1 && volumes[0].paths == ["/"])
        );
        let mut full = mounts(43, 63, "/dev/nvme0n1p3", "0:35");
        numbers_mut(&mut full.root).free = 0;
        numbers_mut(&mut full.root).available = 0;
        numbers_mut(&mut full.home).free = 0;
        numbers_mut(&mut full.home).available = 0;
        let Reading::Value(full) = primary_volumes(full) else {
            panic!("full volume");
        };
        assert_eq!(full[0].available_bytes, 0);
        assert_eq!(full[0].used_bytes, full[0].total_bytes);
        let mut overflow = mounts(43, 63, "/dev/nvme0n1p3", "0:35");
        numbers_mut(&mut overflow.root).blocks = u64::MAX;
        assert!(
            matches!(primary_volumes(overflow), Reading::Value(ref volumes) if volumes.len() == 1 && volumes[0].paths == ["/home"])
        );
    }

    #[test]
    fn primary_capacity_uses_only_verified_disk_storage_from_either_path() {
        for (fs_type, source) in [
            ("nfs4", "server:/home"),
            ("cifs", "//server/share"),
            ("fuse.sshfs", "user@server:/home"),
            ("tmpfs", "tmpfs"),
            ("fuse", "unspecified"),
        ] {
            let mut frame = mounts(43, 63, "/dev/sdb1", "8:17");
            mount_type(&mut frame, 63, fs_type, source);
            assert!(
                matches!(primary_volumes(frame), Reading::Value(ref volumes) if volumes.len() == 1 && volumes[0].paths == ["/"])
            );
        }
        for fs_type in ["nfs", "overlay", "tmpfs"] {
            let mut frame = mounts(43, 63, "/dev/sdb1", "8:17");
            mount_type(&mut frame, 43, fs_type, "server:/root");
            assert!(
                matches!(primary_volumes(frame), Reading::Value(ref volumes) if volumes.len() == 1 && volumes[0].paths == ["/home"])
            );
        }
        let mut both_remote = mounts(43, 63, "/dev/sdb1", "8:17");
        mount_type(&mut both_remote, 43, "nfs", "server:/root");
        mount_type(&mut both_remote, 63, "cifs", "//server/home");
        assert_eq!(
            primary_volumes(both_remote),
            Reading::Unavailable(UnavailableReason::Missing)
        );
        let mut both_unknown = mounts(43, 63, "/dev/sdb1", "8:17");
        mount_type(&mut both_unknown, 43, "overlay", "overlay");
        mount_type(&mut both_unknown, 63, "fuse", "local-or-remote");
        assert_eq!(
            primary_volumes(both_unknown),
            Reading::Unavailable(UnavailableReason::InvalidData)
        );
    }

    #[test]
    fn independent_path_failures_preserve_verified_capacity_and_recover() {
        let mut home_fails = mounts(43, 63, "/dev/sdb1", "8:17");
        mount_type(&mut home_fails, 63, "nfs4", "server:/home");
        home_fails.home = Err(UnavailableReason::SourceError);
        assert!(
            matches!(primary_volumes(home_fails), Reading::Value(ref volumes) if volumes.len() == 1 && volumes[0].paths == ["/"])
        );
        let mut root_fails = mounts(43, 63, "/dev/sdb1", "8:17");
        root_fails.root = Err(UnavailableReason::PermissionDenied);
        assert!(
            matches!(primary_volumes(root_fails), Reading::Value(ref volumes) if volumes.len() == 1 && volumes[0].paths == ["/home"])
        );
        let mut both_fail = mounts(43, 63, "/dev/sdb1", "8:17");
        both_fail.root = Err(UnavailableReason::PermissionDenied);
        both_fail.home = Err(UnavailableReason::SourceError);
        assert_eq!(
            primary_volumes(both_fail),
            Reading::Unavailable(UnavailableReason::PermissionDenied)
        );
        assert!(
            matches!(primary_volumes(mounts(43, 63, "/dev/sdb1", "8:17")), Reading::Value(ref volumes) if volumes.len() == 2)
        );
    }

    #[test]
    fn searchable_only_directory_still_supplies_mount_identity_and_capacity() {
        let path = std::env::temp_dir().join(format!(
            "sys-mon-io-search-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o111)).unwrap();
        let mountinfo = fs::read_to_string("/proc/self/mountinfo").unwrap();
        let result = open_volume(path.to_str().unwrap(), &mountinfo);
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        fs::remove_dir(&path).unwrap();
        let volume = result.unwrap();
        assert!(volume.mount_id > 0);
        if mount_class(&mount_identity(&mountinfo, volume.mount_id).unwrap().fs_type)
            == MountClass::Storage
        {
            assert!(volume.numbers.unwrap().blocks > 0);
        }
    }

    #[test]
    fn a_malformed_direction_clears_only_its_own_rate_baseline() {
        let mut frames = frames(
            vec![
                network_frame("eno2", (100, 100), (0, 0)),
                network_frame("eno2", (150, 150), (0, 0)),
                network_frame("eno2", (200, 200), (0, 0)),
                network_frame("eno2", (250, 250), (0, 0)),
            ],
            vec![
                minimal_blocks(0, 0),
                minimal_blocks(100, 100),
                minimal_blocks(200, 200),
                minimal_blocks(300, 300),
            ],
            4,
        );
        frames.network[1].as_mut().unwrap().dev =
            "eno2: bad 0 0 0 0 0 0 0 150 0 0 0 0 0 0 0\n".into();
        frames.block[1].as_mut().unwrap()[0].counters = Ok(Counters {
            down: None,
            up: Some(100),
        });
        let mut sampler = IoSampler::new(frames);
        sampler.tick(Duration::ZERO);
        let partial = sampler.tick(Duration::from_secs(1));
        assert_eq!(
            partial.network.down_bytes_per_second,
            Reading::Unavailable(UnavailableReason::InvalidData)
        );
        assert_eq!(partial.network.up_bytes_per_second, Reading::Value(50.0));
        assert_eq!(
            partial.storage.read_bytes_per_second,
            Reading::Unavailable(UnavailableReason::InvalidData)
        );
        assert_eq!(
            partial.storage.write_bytes_per_second,
            Reading::Value(100.0)
        );
        let warm = sampler.tick(Duration::from_secs(2));
        assert_eq!(
            warm.network.down_bytes_per_second,
            Reading::Unavailable(UnavailableReason::WarmingUp)
        );
        assert_eq!(warm.network.up_bytes_per_second, Reading::Value(50.0));
        assert_eq!(
            warm.storage.read_bytes_per_second,
            Reading::Unavailable(UnavailableReason::WarmingUp)
        );
        assert_eq!(warm.storage.write_bytes_per_second, Reading::Value(100.0));
        let recovered = sampler.tick(Duration::from_secs(3));
        assert_eq!(
            recovered.network.down_bytes_per_second,
            Reading::Value(50.0)
        );
        assert_eq!(
            recovered.storage.read_bytes_per_second,
            Reading::Value(100.0)
        );
    }

    struct SharedProbe {
        ticks: Arc<Mutex<u64>>,
        volume_mode: Arc<AtomicUsize>,
    }

    impl IoProbe for SharedProbe {
        fn network(&mut self) -> io::Result<NetworkFrame> {
            let mut number = self.ticks.lock().unwrap();
            *number += 1;
            Ok(network_frame(
                "eno2",
                (*number * 100, *number * 200),
                (0, 0),
            ))
        }
        fn block(&mut self) -> io::Result<Vec<BlockDevice>> {
            let number = *self.ticks.lock().unwrap();
            Ok(minimal_blocks(number * 512, number * 1024))
        }
        fn volumes(&mut self) -> io::Result<VolumeFrame> {
            let mode = self.volume_mode.load(Ordering::SeqCst);
            let mut frame = if mode == 0 || mode == 4 {
                mounts(43, 63, "/dev/nvme0n1p3", "0:35")
            } else {
                mounts(43, 63, "/dev/sdb1", "8:17")
            };
            if mode == 1 || mode == 3 {
                mount_type(&mut frame, 63, "nfs4", "server:/home");
            }
            if mode == 2 || mode == 3 {
                mount_type(&mut frame, 43, "nfs4", "server:/root");
            }
            if mode == 5 {
                mount_type(&mut frame, 43, "overlay", "overlay");
                numbers_mut(&mut frame.home).available = 10;
            }
            if mode == 6 {
                frame.root = Err(UnavailableReason::PermissionDenied);
                numbers_mut(&mut frame.home).available = 5;
            }
            Ok(frame)
        }
    }

    #[test]
    fn session_publishes_io_without_erasing_other_fields_and_stops() {
        struct FastIo(IoSampler);
        impl Provider for FastIo {
            fn kind(&self) -> ProviderKind {
                ProviderKind::Io
            }
            fn sample(&mut self, elapsed: Duration, stop: &StopToken) -> Patch {
                self.0.sample(elapsed, stop)
            }
            fn interval(&self) -> Duration {
                Duration::from_millis(10)
            }
        }
        let ticks = Arc::new(Mutex::new(0));
        let volume_mode = Arc::new(AtomicUsize::new(0));
        let mut builder = SessionBuilder::new();
        builder
            .register(FastIo(IoSampler::new(SharedProbe {
                ticks: Arc::clone(&ticks),
                volume_mode: Arc::clone(&volume_mode),
            })))
            .unwrap();
        let mut session = builder.start().unwrap();
        let started = Instant::now();
        let snapshot: Snapshot = loop {
            let snapshot = session.snapshot();
            if matches!(snapshot.network.down_bytes_per_second, Reading::Value(_))
                && matches!(snapshot.storage.read_bytes_per_second, Reading::Value(_))
            {
                break snapshot;
            }
            assert!(started.elapsed() < Duration::from_secs(1));
            thread::sleep(Duration::from_millis(2));
        };
        assert!(matches!(
            snapshot.cpu.usage_percent,
            Reading::Unavailable(UnavailableReason::NotYetSampled)
        ));
        assert!(
            matches!(snapshot.storage.primary_volumes, Reading::Value(ref volumes) if volumes.len() == 1 && volumes[0].paths == ["/", "/home"])
        );
        let wait_for_capacity = |expected: Reading<Vec<PrimaryVolume>>| {
            let started = Instant::now();
            loop {
                let current = session.snapshot().storage.primary_volumes;
                if current == expected {
                    break;
                }
                assert!(
                    started.elapsed() < Duration::from_secs(1),
                    "capacity did not update: {current:?}"
                );
                thread::sleep(Duration::from_millis(2));
            }
        };
        let root_only = primary_volumes({
            let mut frame = mounts(43, 63, "/dev/sdb1", "8:17");
            mount_type(&mut frame, 63, "nfs4", "server:/home");
            frame
        });
        volume_mode.store(1, Ordering::SeqCst);
        wait_for_capacity(root_only);
        let home_only = primary_volumes({
            let mut frame = mounts(43, 63, "/dev/sdb1", "8:17");
            mount_type(&mut frame, 43, "nfs4", "server:/root");
            frame
        });
        volume_mode.store(2, Ordering::SeqCst);
        wait_for_capacity(home_only);
        let unknown_root = primary_volumes({
            let mut frame = mounts(43, 63, "/dev/sdb1", "8:17");
            mount_type(&mut frame, 43, "overlay", "overlay");
            numbers_mut(&mut frame.home).available = 10;
            frame
        });
        volume_mode.store(5, Ordering::SeqCst);
        wait_for_capacity(unknown_root);
        let failed_root = primary_volumes({
            let mut frame = mounts(43, 63, "/dev/sdb1", "8:17");
            frame.root = Err(UnavailableReason::PermissionDenied);
            numbers_mut(&mut frame.home).available = 5;
            frame
        });
        volume_mode.store(6, Ordering::SeqCst);
        wait_for_capacity(failed_root);
        volume_mode.store(3, Ordering::SeqCst);
        wait_for_capacity(Reading::Unavailable(UnavailableReason::Missing));
        volume_mode.store(4, Ordering::SeqCst);
        wait_for_capacity(primary_volumes(mounts(43, 63, "/dev/nvme0n1p3", "0:35")));
        session.stop();
        let stopped_at = *ticks.lock().unwrap();
        thread::sleep(Duration::from_millis(25));
        assert_eq!(*ticks.lock().unwrap(), stopped_at);
    }
}
