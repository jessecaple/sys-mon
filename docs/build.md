# Build and install sys-mon

sys-mon is a native GTK4/libadwaita application for GNOME on Linux. Building
requires Rust, a C compiler, `pkg-config`, GTK 4.12 or newer and libadwaita
1.4 or newer, including their development files. Verification used Rust
1.98.1; the oldest usable Rust version has not been established. The built
executable needs the GTK4 and libadwaita runtime libraries.

On Fedora 44, install the build dependencies and run from the source checkout:

```sh
sudo dnf install gtk4-devel libadwaita-devel pkgconf-pkg-config gcc
cargo run --locked
```

## Install for your user

```sh
cargo build --release --locked
install -Dm755 target/release/sys-mon "$HOME/.local/bin/sys-mon"
install -Dm644 data/io.github.jessecaple.SysMon.desktop "$HOME/.local/share/applications/io.github.jessecaple.SysMon.desktop"
install -Dm644 data/io.github.jessecaple.SysMon.svg "$HOME/.local/share/icons/hicolor/scalable/apps/io.github.jessecaple.SysMon.svg"
```

Launch `sys-mon` from a terminal or the application menu. The menu entry needs
`$HOME/.local/bin` in the desktop session's `PATH`; sign out and back in if you
just added it. To uninstall, delete the three installed files above. The app
runs as an ordinary user and needs no privileged service.

## Hardware and limits

CPU, memory, network and storage readings come from Linux kernel interfaces.
NVIDIA NVML and Intel Level Zero are optional; AMD uses amdgpu interfaces.
Missing drivers, unreadable sensors and unsupported readings appear as
unavailable while other readings continue updating. Access to `/proc`,
`/sys`, render nodes and sensors depends on the host. Shared system memory is
not shown as dedicated VRAM; GPU usage can remain available when temperature
or VRAM is not. Primary storage is local capacity backing `/` and `/home`, with
a shared filesystem shown once. It reports space available to the ordinary
user and excludes remote or unclassified mounts.

Live checks used Fedora 44 x86_64, Rust 1.98.1, GTK 4.22.5, libadwaita 1.9.4,
an Intel Core i9-13900K, NVIDIA RTX 4090 and Btrfs `/` and `/home` subvolumes.
The native X11 window was exercised on a GNOME Wayland host; a Wayland launch
was observed separately. AMD and Intel GPU behavior, remote primary mounts and
distinct local primary filesystems were tested with fixtures. Other
distributions, architectures and physical GPU configurations have not been
verified here. A weeks-long run was not performed.

## Check the source

```sh
cargo fmt --check
cargo test --locked
cargo clippy --all-targets -- -D warnings
cargo build --locked
```

The [specification](wf-48ebf7-spec.md) gives the metric and window contract.
The product source, icon and documentation are MIT licensed under the root
[LICENSE](../LICENSE). Tracked development skills retain the license terms in
their own directories; Rust dependencies and system libraries have their own
licenses. No binary packages are published.
