# sys-mon

A native resource monitor for GNOME. One window shows current CPU, memory,
GPU, network and storage readings.

## Run from source

On Fedora, install the build dependencies and run:

```sh
sudo dnf install gtk4-devel libadwaita-devel pkgconf-pkg-config gcc
cargo run --locked
```

See the [build guide](docs/build.md) for installation and verified hardware,
and the [specification](docs/wf-48ebf7-spec.md) for metric details. sys-mon is
MIT licensed; see [LICENSE](LICENSE).
