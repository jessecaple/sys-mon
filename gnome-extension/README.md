# sys-mon for the GNOME top bar

A GNOME Shell 50 extension. It sits in the panel's left area, after the
workspace switcher, and inherits the panel's background.
Source Code Pro numbers and monochrome icons give the panel a technical feel.
Other systems fall back to their monospace font if Source Code Pro is absent.
Temperatures, swap, VRAM, network upload, disk I/O and inodes use quieter text.
Slashes remain faint, and the background stays native. Each group starts with a bundled 16px symbolic icon.

Capped resources use two-digit whole percentages capped at 99 on the panel:
RAM, swap, VRAM, `/tmp` space and `/tmp` inodes show percent used; primary
storage shows percent remaining. Temperatures remain Celsius, with values
above 99 °C shown as `99+°`. Unknown capacities show a dash. Full amounts and actual percentages remain in accessible descriptions.

Percentages and temperatures are zero-padded to two digits (`03%`, `07°`);
speeds use three (`009K`, `000B`). Tiny positive readings retain `<1%` or
`<01B` so they remain distinct from zero. Padding zeros use half the reading's
opacity; the significant zero in `00%` or `000B` keeps its normal brightness.

Every value has a fixed slot, so changing readings do not move neighboring
values or groups. Network and disk speeds use up to three whole digits and a
compact binary unit. Values center on fixed anchors, balancing spare space
around slashes with no added padding. Slots closely fit three-character
percentages/temperatures or four-character speeds; monospace units occupy the
same width. Equal gaps separate groups; lighter rate
text keeps the storage group from dominating. Unusually long readings are
clipped to their slot; accessible descriptions retain the complete reading.

| Icon | Values, left to right |
| --- | --- |
| CPU chip | CPU % / CPU temperature |
| Graphics card | GPU % / VRAM % used / GPU temperature |
| Memory module | RAM % used / swap % used |
| Storage disk | Primary storage % remaining / disk read / disk write |
| Folder with clock | /tmp filesystem % used / /tmp inodes % used |
| Transfer arrows | Network download / upload |

Speeds use binary units: `G` means GiB/s, `M` means MiB/s and `K` means KiB/s,
with `/s` omitted from the strip. The strip is display-only: clicks and hovering
do not open popups or change its appearance. A dash means waiting or unavailable.
Accessible descriptions retain full labels, units, totals and status details. Separate storage volumes retain path
labels, separated by a middle dot.

The strip keeps the clock in GNOME's center area. It is intended for a wide
panel like the owner's desktop; all fourteen panel readings remain visible, so it
may compete for space on small monitors or with large text scaling.

## Install

From the repository root:

```sh
./scripts/install-gnome-extension.sh
gnome-extensions enable sys-mon@jcaple.dev
```

A newly installed extension may need a sign-out/sign-in before GNOME discovers
it. Changes to already loaded JavaScript need a new Shell session too. The
installer bundles the built Rust executable inside the extension; no service,
root access, Node runtime or separate dashboard window is needed. Repeat the
installer and start a new session when updating it.

Disable or remove:

```sh
gnome-extensions disable sys-mon@jcaple.dev
```

After disabling, remove
`~/.local/share/gnome-shell/extensions/sys-mon@jcaple.dev` to uninstall.

## Backend and checks

The extension starts one `sys-mon --panel-stream` subprocess. It collects the
existing telemetry and publishes versioned JSON lines every three seconds,
using the same display averaging as the dashboard. Closing its stdin requests
shutdown and joins the telemetry workers. Shell reads stdout asynchronously.
A failed process clears the values and is retried after five seconds; stale
readings are cleared after ten seconds. Disabling removes actors, timers,
and both process pipes, including cancellation of pending reads.
Shell reads in 4 KiB chunks and rejects a frame larger than 64 KiB before JSON
decoding, including unfinished lines. GPU, temperature and capacity helper
output is drained with bounded nonblocking reads so pipe EOF cannot prevent
timeout or worker shutdown.

```sh
cargo fmt --check
cargo test --locked
cargo clippy --all-targets -- -D warnings
node --test gnome-extension/tests/*.test.mjs
```

The JavaScript tests run under Node for convenience; the installed extension
runs inside GNOME Shell's GJS. The protocol and process ownership are covered
by the Rust process-boundary test. Verify placement, display-only behavior, disable /
re-enable, and backend recovery in a running Shell.

Implementation follows the [GNOME extension guide](https://gjs.guide/extensions/development/creating.html)
and [asynchronous subprocess guide](https://gjs.guide/guides/gio/subprocesses.html).

Verified on the owner's GNOME Shell 50.5 desktop at 3440×1440: the six groups
fit after Space Bar with the clock centered, showing `/tmp` space and inode usage. An isolated GNOME Shell 50 smoke test also
passed real readings, display-only behavior, backend crash recovery, disable, clean
backend exit and re-enable. Run that test with a bundle containing the built
backend using `gnome-shell-test-tool --headless --extension <bundle>
<absolute-path-to-tests/shell-smoke.js>` inside `dbus-run-session`.
