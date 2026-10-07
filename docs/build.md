# Build and run sys-mon

sys-mon is a native GTK4/libadwaita application for GNOME on Linux. The Rust
bindings in this repository require GTK 4.12 or newer and libadwaita 1.4 or
newer. A C toolchain and `pkg-config` are needed to build. Verification here
used Rust 1.98.1; the oldest usable Rust version has not been established.

On Fedora, install the development libraries and run from the source checkout:

```sh
sudo dnf install gtk4-devel libadwaita-devel pkgconf-pkg-config gcc
cargo run
```

The executable needs the GTK4 and libadwaita runtime libraries. GNOME supplies
the normal system theme. NVIDIA NVML and Intel Level Zero facilities are
optional; missing hardware or driver libraries leave their readings unavailable
while the rest of the window works. Sensor and device permissions depend on the
host. No privileged service is required by the UI.

Run `cargo fmt --check`, `cargo test --locked`,
`cargo clippy --all-targets -- -D warnings` and `cargo build --locked` to verify
a source build. The [README](../README.md) gives current-user installation and
removal steps, including the desktop entry and icon. Only the environments
listed there have been checked; no binary packages are published.
