# sys-mon

sys-mon is a native GTK4/libadwaita resource monitor for GNOME on Linux. Its
single window shows current CPU, memory, GPU, network and storage readings in
two columns. It keeps no history. Closing the window stops collection.

Sensors vary by machine. A missing driver, unreadable sensor or unsupported
reading appears as unavailable while the other readings continue updating.

## Build and run

The Rust bindings require GTK 4.12 or newer and libadwaita 1.4 or newer. You
also need Rust, a C compiler, `pkg-config` and the GTK/libadwaita development
files. Rust 1.98.1 was used for verification; the oldest usable Rust version
has not been established.

On Fedora 44, install the build dependencies and run from the source checkout:

```sh
sudo dnf install gtk4-devel libadwaita-devel pkgconf-pkg-config gcc
cargo run --locked
```

The executable loads GTK4 and libadwaita at runtime. It uses the desktop's
light or dark theme. NVIDIA NVML and Intel Level Zero are optional facilities;
their absence does not prevent the app from opening. AMD readings use amdgpu
interfaces exposed by the host.

## Install from source

These commands install for the current user after a release-mode source build:

```sh
cargo build --release --locked
install -Dm755 target/release/sys-mon "$HOME/.local/bin/sys-mon"
install -Dm644 data/io.github.jessecaple.SysMon.desktop "$HOME/.local/share/applications/io.github.jessecaple.SysMon.desktop"
install -Dm644 data/io.github.jessecaple.SysMon.svg "$HOME/.local/share/icons/hicolor/scalable/apps/io.github.jessecaple.SysMon.svg"
```

Launch `sys-mon` from a terminal or the application menu. The menu entry
requires `$HOME/.local/bin` in the desktop session's `PATH`; sign out and back
in if you just added that directory. To remove this installation, delete the
three files named in the `install` commands. The app itself runs as an ordinary
user and does not need a privileged service.

## Metric availability

CPU, memory, network and storage data come from Linux kernel interfaces. The
GPU provider discovers NVIDIA, AMD and Intel devices where their drivers expose
the required readings. A GPU can report usage while its temperature or VRAM is
unavailable. Shared system memory is not presented as dedicated VRAM.

Some `/proc`, `/sys`, render-node and sensor files may be inaccessible to an
ordinary user. sys-mon reports affected readings as unavailable and retries
without hiding unrelated values. Primary storage means local capacity backing
`/` and `/home`; a shared filesystem is shown once. Remote or unclassified
mounts are excluded. The visible amount is space available to the ordinary
user.

## Verified here

Development and live checks used Fedora 44 x86_64, Rust 1.98.1, GTK 4.22.5,
libadwaita 1.9.4, an Intel Core i9-13900K, NVIDIA RTX 4090 and Btrfs `/` and
`/home` subvolumes. The native X11 window was exercised on a GNOME Wayland
host, including launch, resizing, sensor updates and close. A Wayland launch
was observed earlier; the full live stress and close pass used X11. AMD and
Intel GPU behavior was checked with fixtures, not physical hardware. Remote
primary mounts and distinct local primary filesystems were also fixture-tested.
Other distributions, architectures and physical GPU configurations have not
been verified here. A weeks-long run was not performed.

Run the source checks with:

```sh
cargo fmt --check
cargo test --locked
cargo clippy --all-targets -- -D warnings
cargo build --locked
```

The [build guide](docs/build.md) and [specification](docs/wf-48ebf7-spec.md)
record the source requirements and behavior in more detail.

## License

The sys-mon product source, icon and documentation are MIT licensed; see
[LICENSE](LICENSE). Some tracked development skills carry their own license
files in their directories. Rust dependencies and system libraries retain
their respective licenses.
