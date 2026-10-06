# Native dashboard preparation

Ticket `t-abb4ba`; integration prerequisite `e7089f0387ed649fe70fa9d5f0e08007c2113f3e`.
Authority: [specification](../wf-48ebf7-spec.md), Wayfinder M2 description and
ticket brief, [M2 plan](t-e83d25-plan.md), approved
[dark](../prototypes/visual-overview-approved-dark.png) and
[light](../prototypes/visual-overview-approved-light.png) captures.
The HTML prototype is disposable design evidence, not application code.

## Behavior inventory and witnesses

| Source and consumer | Required behavior and consequences | Witness |
| --- | --- | --- |
| M2 and specification visual contract; sole window | Six rows in exact order: CPU/CPU temp, RAM/Swap, GPU/GPU temp, VRAM/Storage, Network down/up, Storage read/write. Two columns remain visible at the supported minimum and wider widths. No heading, tabs, graphs, details or expansion. | Rendered minimum/wide window, inspect all 12 labels and values. |
| Approved captures and prototype; each metric cell | Monochrome surface, quiet labels, strong aligned current values, distinct glyphs, square bottom fill rails only for bounded metrics. Follow system light/dark preference. Glyphs supplement text. | Light/dark captures and accessibility inspection. |
| `Snapshot` fields in `src/telemetry/mod.rs`; presenter | CPU/GPU usage as percent; CPU/GPU temperature in °C; RAM/swap/VRAM current used amount only; network and storage directions in bytes per second; storage available bytes. Real zero remains zero. Each `Reading::Unavailable` affects only its field and recovers when a later value arrives. Startup and warm-up show Waiting; missing/unsupported shows Unavailable; permission denial shows No access; invalid data shows Invalid reading; source failure shows Source error; timeout shows Timed out. Each accessible description names the status and metric. | Injected normal/zero and all seven unavailable reasons, startup-to-value and failure-to-recovery transitions without stale text; live values and recovery. |
| `MemoryAmount`; three memory cells | A valid positive total drives a used/total rail and accessible description. Unknown or zero total never creates a fraction. Visible text never appends total. | Injected zero and unknown capacity cases. |
| `Temperature.threshold`; temperature cells | Valid temperature text survives an absent or invalid threshold. A positive hardware target/max or critical limit drives a clamped rail, with no visible endpoint label or universal fallback. Accessible scale identifies a target/max versus critical limit. | Both threshold kinds, absent/zero threshold and over-threshold snapshots. |
| `PrimaryVolume`; Storage cell | One shared `/` and `/home` volume appears once. Separate volumes remain individually named by path, with separate available amounts and used/total rails. Each displayed path's accessible description says its visible amount is remaining capacity and its rail is used capacity. `/`-only and `/home`-only subsets show only the observed path; all-unavailable stays unavailable. Available bytes are shown as reported, not derived from `total - used`, which may include reserved capacity. A zero/invalid total suppresses the rail. | Shared, distinct, each partial subset, all-unavailable, reserved-space and zero-total fixtures; subset recovery and rendered state. |
| `TelemetrySession::snapshot`; GTK window | Collection starts once; UI reads only latest snapshot on bounded timer without accumulating queued updates or running source work on GTK loop. Widgets update only when a displayed value or status changes. Minimized windows continue bounded collection but cause no continuous animation or redraw. Provider failures retain independent cells. | Live activity, failing fields, unchanged snapshots, minimized-window process/paint check, responsiveness and no growing queue. |
| `TelemetrySession::stop`; sole window close | Closing stops refresh and all owned collection/helpers, including in-flight work, before the process exits. A slow stop does not freeze the visible desktop. | Close in normal/in-flight/failure states and process check. |
| Desktop user and accessibility APIs | Meaningful value/status and rail scale descriptions without reliance on color; supported text scaling retains readable two columns. No metric cell requires pointer interaction. | Accessibility readback, keyboard, scaled text, minimum width. |

Telemetry owns discovery, permissions, provider recovery and field status. The UI
does not request extra privileges or persist readings. There is no UI preference
or saved window position. Runtime source and hardware variants are covered by M1
fixtures; this ticket checks their visible consequences.

## Technical and UX design

`src/presentation.rs` will convert a coherent `Snapshot` into twelve display
cells. It owns formatting, visible text, accessible descriptions and optional
rail fractions; it will not retain history. The presenter preserves reported
storage availability even when filesystem reserve makes it differ from total
minus used. The sole UI consumer is
`src/dashboard.rs`, called by `src/main.rs`. The existing diagnostic and helper
arguments remain dispatched before GTK startup. `TelemetrySession` remains the
sole collection owner and its API is unchanged.

Use matched `gtk4`/`libadwaita` Rust bindings and an `adw::ApplicationWindow`.
Build a fixed six-row, two-column grid. Each cell aligns a distinct decorative
glyph, label and value, with a narrow square rail along the bottom when its
quantity is bounded. Use Adwaita foreground/surface colors and theme-aware CSS
without color encoding. Header bar provides only native window controls. A
minimum logical width derived from label/value measurements keeps both columns
readable; verify with larger text. Distinct volumes use two lines inside the
Storage cell, each named `/` or `/home`, with a corresponding rail. A larger
minimum width or taller Storage row is acceptable when required to avoid
truncating truthful values.

The GTK main loop polls `snapshot()` once per second. It neither waits for
providers nor queues snapshots. It changes widget properties only when the
presented state changes; GTK/compositor handles minimized-window painting.
Close requests hide the window, start one
background stop operation, and quit the application when that operation
finishes. Duplicate close requests cannot start duplicate stops. Session
startup failure leaves the window usable with unavailable cells and reports
the error to stderr. The telemetry session does not modify GTK widgets.

Test the presenter with injected snapshots and the close controller with an
in-flight provider. Inspect the actual window on the available display backend
at minimum/wide width, light/dark, normal/unknown/zero data and text scaling.
