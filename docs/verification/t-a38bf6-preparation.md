# Public source publication preparation

Ticket `wf-48ebf7/t-a38bf6`; session `wfs-b4eb4635c5bbb789`. Integration
prerequisite `t-579f33` is present at `406201c4f496c914884560a8480387b255d7bcd6`.
The owner corrected the destination on 2026-10-06: publish a public source
repository, without a GitHub Release, tag, binary packages or downloadable
artifacts. The current Wayfinder ticket and M3 description govern this work;
the older release wording in the specification and M3 plan must be corrected.

## Existing behavior and publication inventory

| Source and consumer | Current behavior and variants | Permission, recovery and lasting effect |
| --- | --- | --- |
| `Cargo.toml`, `src/main.rs`, `src/dashboard.rs`; source builders and desktop launcher | Cargo package and executable are `sys-mon` at version `0.1.0`; the window title is `sys-mon`. The application ID is `io.github.sys_mon.SysMon`. Cargo has no license or repository metadata. There is no desktop entry or icon declaration. | Building needs GTK4/libadwaita development metadata and a Rust/C toolchain. Launching needs a graphical session. The application ID and desktop filename must agree for desktop launch and window association. Add `license = "MIT"` and the verified repository URL to Cargo metadata. |
| `docs/build.md`; people building from source | It lists Fedora development packages, `cargo run`, and verification commands. It still promises later release packages and an untested distro matrix. There is no root README, license or local installation recipe. | Source installs must describe ordinary-user paths and runtime libraries without claiming unsupported distributions. Unavailable sensors remain visible as unavailable; no privileged UI service is required. |
| `src/telemetry/linux.rs`, `src/telemetry/mod.rs`; CPU, RAM, swap and temperature cells | CPU and memory use procfs; CPU package/die temperature uses attributable hwmon channels. Existing host snapshots and source-failure tests are described in the M1 [completion review](t-b25f7c-completion.md). | Ordinary-user read access to procfs and hwmon varies by host. Missing, invalid and denied readings become unavailable rather than zero; source recovery publishes a fresh reading. Only the current snapshot and needed CPU baseline persist in the process. |
| `src/telemetry/gpu.rs`, `src/telemetry/gpu/intel.rs`; GPU usage, temperature and VRAM cells | NVIDIA uses optional NVML, AMD uses amdgpu sysfs and a render-node classification ioctl, and Intel uses optional Level Zero Sysman. The verified field and device rules are in [GPU preparation](t-f0e7ff-preparation.md). Physical NVIDIA was compared with `nvidia-smi`; AMD/Intel paths have fixtures only. | Driver libraries, render nodes, sysfs and sensor permissions can be absent for ordinary users. Each field may be unavailable independently; rediscovery allows recovery. No stale selected-device value or history is retained. |
| `src/telemetry/io_provider.rs`; network down/up and storage read/write cells | Selected logical network link and nonoverlapping block devices supply separate counter rates. Source selection, reset and overlap rules are in [I/O preparation](t-62c8b3-preparation.md); long-session evidence is in the [reliability receipt](~/.wayfinder/evidence/wf-48ebf7/t-579f33.md). | Procfs, routing and block sysfs normally allow ordinary-user reads. New, removed, reset or unreadable counters warm up or become unavailable without fabricated spikes. Network and storage errors remain independent; only current identity baselines persist. |
| `src/telemetry/io_provider.rs`; primary-storage remaining cell | Local `/` and `/home` capacity uses ordinary-user available space and deduplicates shared Btrfs subvolumes. Remote or unclassified mounts are excluded. See [capacity correction](t-cf4caa-preparation.md) and the [bounded helper design](t-579f33-preparation.md). This Fedora host's shared Btrfs volume was checked live; remote and distinct-volume behavior have fixtures. | Each path needs search permission and its mount/capacity information must be readable. One valid local path survives failure of the other; no verified local path is unavailable. The bounded helper prevents a slow pathname lookup from stalling the window; later valid samples recover. No mount history persists. |
| `src/dashboard.rs`, `src/presentation.rs`; desktop user and README readers | The native two-column window consumes the current snapshot. X11 native close and stress have live evidence in the [reliability receipt](~/.wayfinder/evidence/wf-48ebf7/t-579f33.md); M2 recorded a Wayland launch in its [completion audit](t-f012be-completion.md). | An unavailable sensor leaves other cells usable. Closing the window stops collection and owned helpers. This ticket changes desktop launch identity, not the metric layout or telemetry contracts. |
| Git checkout and GitHub; people reading or cloning source | `main` has no remote and `jessecaple/sys-mon` did not exist when checked. The authenticated GitHub account is the personal user `jessecaple`. Current `.gitignore` excludes `target/` only. | The public repository is the durable external result. Before publication, inspect tracked files for credentials and private material; verify account, repository name, visibility, license and exact pushed commit afterward. Do not create a GitHub Release or tag. |
| Root `LICENSE`, Cargo metadata, vendored-looking skills and prototype assets; public readers | No root license exists. Git history uses the GitHub account's no-reply address. Some tracked skill bundles carry their own license files; prototypes and research are tracked project material. | Add the standard MIT text with the verified profile name and year. Audit bundled assets and third-party notices before representing the repository as MIT-licensed; do not overwrite their separate terms. Git publication permanently exposes tracked history and files. |

## Design and verification

Keep the native dashboard and telemetry contracts unchanged. Align the GTK
application ID with the verified personal GitHub namespace and a matching
desktop entry. Install the source-built executable, desktop file and a small
monochrome SVG icon into a user-owned prefix using documented commands. No
package manager integration is added.
The live X11 window reports `sys-mon` as its WM class; the desktop entry names
that class for window association.

The README is the public entry: what the app shows, Fedora 44 x86_64 source
build and local installation, GTK/libadwaita version floors, optional hardware
paths, permissions, checks and verification limits. Cargo metadata must identify
MIT and the final repository. Audit the included images, skill bundles and
dependency notices separately; the root MIT grant covers owner-authored product
source and documentation. Correct the settled
specification, M3 plan, effort brief and build guide to remove obsolete release
requirements. Existing prototype and research files remain historical evidence
and need not be rewritten to imply a new decision.

Witnesses: inspect all final identifiers; validate the desktop entry and
launcher association; run `cargo fmt --check`, `cargo test --locked`, Clippy and
`cargo build --locked` with the established local GTK metadata root; launch and
close the source-built application on the available desktop; audit tracked
files; then read back the public repository URL, visibility, license and pushed
commit. The M1/M2/M3 telemetry evidence remains applicable because source
collection and dashboard behavior are not changed by this ticket.
