# Rust desktop and graph stack for the GNOME resource monitor

Research for Wayfinder `wf-48ebf7/t-529845`, 2026-10-05. This report is a stack recommendation, not an implementation or a measured performance result.

> Later owner decision (`wf-48ebf7/t-67afd4`): the delivered monitor is a current-status overview without history graphs or secondary detail views. The native Rust GTK4/libadwaita recommendation still applies; the Cairo `DrawingArea` graph proposal below is historical research, not a current implementation requirement.

## Recommendation

Use the matched `gtk4` and `libadwaita` Rust bindings (`adw::Application` and `adw::ApplicationWindow`) for the single native GNOME window. Build the overview from standard GTK/libadwaita widgets and a small number of custom `gtk4::DrawingArea` graphs drawn with the bundled Cairo binding. Keep labels, values, controls, focus, and device-detail affordances as ordinary widgets outside the drawing surface. Use bounded per-series sample buffers and redraw graphs only when new samples, size, style, or visible selection changes. This is a proposed choice, based on the APIs below; no idle CPU or memory comparison has yet been measured. [gtk-rs libadwaita guide](https://gtk-rs.org/gtk4-rs/stable/latest/book/libadwaita.html), [GTK DrawingArea](https://gtk-rs.org/gtk4-rs/stable/latest/docs/gtk4/struct.DrawingArea.html), [GTK accessibility guidance](https://docs.gtk.org/gtk4/section-accessibility.html).

## Verified facts

- GTK4's Rust binding is the maintained GTK4 API for Rust. It runs GTK objects on the main thread; its documentation says GTK types are not `Send`/`Sync`. The gtk-rs book recommends moving CPU-bound work to another thread and returning results through a channel so the main loop remains responsive. [gtk4 crate](https://gtk-rs.org/gtk4-rs/stable/latest/docs/gtk4/), [gtk-rs event-loop guide](https://gtk-rs.org/gtk4-rs/stable/latest/book/main_event_loop.html).
- Libadwaita augments GTK4 with GNOME widgets, adaptive layout and Adwaita styling, including dark-style integration. The gtk-rs guide explicitly warns that `gtk4` and `libadwaita` crate versions must be matched. Version feature flags enable APIs tied to minimum installed GTK/libadwaita versions. [gtk-rs libadwaita guide](https://gtk-rs.org/gtk4-rs/stable/latest/book/libadwaita.html), [gtk4 crate features](https://gtk-rs.org/gtk4-rs/stable/latest/docs/gtk4/).
- `gtk4::DrawingArea` accepts a Cairo draw function and can be invalidated with `queue_draw()`. The draw function also runs for exposure and resize, so the model must retain enough recent samples to reconstruct the graph. GTK documents event controllers for pointer input and focus handling for keyboard interaction on a drawing area. [DrawingArea API](https://gtk-rs.org/gtk4-rs/stable/latest/docs/gtk4/struct.DrawingArea.html).
- GTK's standard controls expose accessibility information by default. A custom widget/drawing area needs deliberate semantic properties; a visual graph alone does not supply a screen reader with current values or controls. [GTK accessibility guidance](https://docs.gtk.org/gtk4/section-accessibility.html).
- GDK abstracts Wayland and X11. GTK's Linux builds normally include both backends, and the runtime backend follows the display environment (or `GDK_BACKEND` if set). No backend-specific UI API is needed for this window. [GTK overview](https://docs.gtk.org/gtk4/overview.html), [GTK build backends](https://docs.gtk.org/gtk4/building.html), [GDK backend selection](https://docs.gtk.org/gdk4/func.set_allowed_backends.html).
- GTK recommends saving `Window::get_default_size()` for later `set_default_size()` restoration; this avoids allocation-based size drift. Maximization is exposed as window state and is asynchronous. Exact coordinate restoration is outside the approved destination. [GTK default size](https://docs.gtk.org/gtk4/method.Window.set_default_size.html), [GTK maximized property](https://docs.gtk.org/gtk4/property.Window.maximized.html).
- An application window keeps `GtkApplication` alive; removal of its last window may let the application stop. A `glib` timeout returns a `SourceId` and repeats until its closure returns `ControlFlow::Break`; GTK's shutdown signal runs when the primary instance's main loop terminates. [GTK application window association](https://docs.gtk.org/gtk4/method.Window.set_application.html), [GLib timeout API](https://gtk-rs.org/gtk-rs-core/stable/latest/docs/glib/source/fn.timeout_add_local.html), [AdwApplication lifecycle](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/class.Application.html).

## Graph options and tradeoffs

| Option | Evidence | Fit for this dashboard |
| --- | --- | --- |
| Cairo inside `DrawingArea` | Direct draw callback and explicit `queue_draw()` are supported by GTK. [API](https://gtk-rs.org/gtk4-rs/stable/latest/docs/gtk4/struct.DrawingArea.html) | Recommended. Few compact time-series graphs need custom strokes, fills and markers, with no chart framework required. Low idle cost is plausible if redraw follows sampling, but must be measured. |
| Plotters with `plotters-cairo` | The project supplies a Cairo drawing backend for Plotters; Plotters supports charts and multiple backends. [backend](https://github.com/plotters-rs/plotters-cairo), [Plotters](https://github.com/plotters-rs/plotters) | Viable if later requirements demand rich axes, legends or export. It adds a plotting abstraction and dependency for the initial small sparklines. No benchmark was found here that proves it faster or slower. |
| Custom GSK/widget snapshot | GTK's scene graph has renderers for OpenGL, Vulkan and Cairo. [GTK overview](https://docs.gtk.org/gtk4/overview.html) | Possible later optimization if Cairo profiling shows a bottleneck. It requires a custom widget and more accessibility work, so it is unnecessary as the starting point. |

## Proposed implementation constraints

1. Keep metric collection off the GTK main thread when it can block; pass immutable sample snapshots or bounded updates to the UI. This follows the gtk-rs threading guidance, while the exact collector design belongs to the telemetry ticket. [gtk-rs event-loop guide](https://gtk-rs.org/gtk4-rs/stable/latest/book/main_event_loop.html).
2. Bound history by sample count and time horizon, including per-core and per-device series. Cap rendered points to the available graph width or downsample before drawing. These are design proposals to satisfy the map's bounded-history requirement, not guarantees provided by GTK.
3. Keep a stable scale and clear unavailable state per metric. Use text/value widgets as accessible equivalents to the graphical reading and ensure graph-only gestures have keyboard-accessible controls. [GTK accessibility guidance](https://docs.gtk.org/gtk4/section-accessibility.html), [DrawingArea API](https://gtk-rs.org/gtk4-rs/stable/latest/docs/gtk4/struct.DrawingArea.html).
4. Own timers, worker cancellation, and channels with the window/application lifecycle. On close, stop collection, remove repeating sources or make them break, drain/join workers with a bounded shutdown path, then allow the last window to disappear. The API enables this; correct cleanup still requires tests. [GLib timeout API](https://gtk-rs.org/gtk-rs-core/stable/latest/docs/glib/source/fn.timeout_add_local.html), [GTK window lifecycle](https://docs.gtk.org/gtk4/method.Window.set_application.html).

## Build and compatibility implications

The Rust crates bind system GTK4/libadwaita libraries. The gtk-rs Linux guide lists development packages (`libgtk-4-dev`, `libadwaita-1-dev` on Debian-like systems; corresponding `-devel` packages on Fedora) plus build tools. Choose the oldest GTK/libadwaita API version supported by the intended release targets, document that runtime minimum, and keep crate feature levels aligned. GTK's default Linux build supports both Wayland and X11, but distribution packages must actually contain those backends. [gtk-rs Linux installation](https://gtk-rs.org/gtk4-rs/stable/latest/book/installation_linux.html), [gtk-rs libadwaita guide](https://gtk-rs.org/gtk4-rs/stable/latest/book/libadwaita.html), [GTK build backends](https://docs.gtk.org/gtk4/building.html).

## Unresolved questions for implementation

- Which minimum distro/runtime versions to publish and package against. The decision should follow actual target distro versions; this ticket does not choose an unrequested support floor.
- What sampling cadence and graph horizon meet the owner's glanceability goal within measured idle CPU and memory budgets. Benchmark on the available machine under load and at narrow window sizes.
- Whether Cairo rendering remains within that budget at high CPU-core counts and HiDPI. Measure before adding Plotters or custom GSK rendering.
