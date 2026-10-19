# sys-mon

A resource monitor for GNOME. A compact top-bar extension groups CPU, memory,
GPU, storage, `/tmp`, inode, network and disk I/O readings behind six glyphs.
The strip is display-only, with no click menu or hover popups. A native dashboard is also
available.

The display updates every three seconds, showing the average of fast-changing
readings collected since the previous update.

## Install in the GNOME top bar

On GNOME Shell 50, after installing the build dependencies below:

```sh
./scripts/install-gnome-extension.sh
gnome-extensions enable sys-mon@jcaple.dev
```

A new extension may need a sign-out/sign-in before GNOME discovers it. See
the [extension guide](gnome-extension/README.md) for grouping, units and removal.

## Run the dashboard from source

On Fedora, install the build dependencies and run:

```sh
sudo dnf install gtk4-devel libadwaita-devel pkgconf-pkg-config gcc
cargo run --locked
```

See the [build guide](docs/build.md) for installation and verified hardware,
and the [specification](docs/wf-48ebf7-spec.md) for metric details. sys-mon is
MIT licensed; see [LICENSE](LICENSE).
