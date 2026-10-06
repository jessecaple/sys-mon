# sys-mon specification

Decision baseline for Wayfinder `wf-48ebf7`, completed 2026-10-05.

This document synthesizes the current destination and settled decisions. The delivery roadmap and milestone scope live in Wayfinder; use `wayfinder milestones wf-48ebf7` and `wayfinder milestone show wf-48ebf7 --milestone <key>`. This document does not duplicate that roadmap.

## Problem Statement

The owner wants a small native GNOME resource monitor that can remain open during heavy work and long sessions. It should answer what the machine is doing now with a readable, compact overview. Missing sensors must be apparent, and the monitor must not become a source of accumulating work, memory growth or frozen windows.

## Solution

Build **sys-mon**, a native Rust GTK4/libadwaita application for Linux GNOME on Wayland and X11. Its only window is a resizable, always-two-column current-status overview. It follows the system light/dark theme and uses monochrome text, distinct glyphs and integrated fill rails.

The approved visual references are:

- [Dark overview](prototypes/visual-overview-approved-dark.png).
- [Light overview](prototypes/visual-overview-approved-light.png).
- [Disposable HTML prototype](prototypes/visual-overview.html), whose controls and sample values are design evidence only.

The ordinary overview follows this row order:

| Left column | Right column |
| --- | --- |
| CPU usage | CPU temperature |
| RAM used | Swap used |
| GPU usage | GPU temperature |
| VRAM used | Primary storage remaining |
| Network down | Network up |
| Storage read | Storage write |

Values update periodically from discovered hardware and kernel interfaces. Where a source is absent, unsupported, inaccessible or failing, show an unavailable reading without interrupting the other cells. Close the window to end the application and all its collection.

Efficiency and reliability are implementation requirements. The owner rejected a separate tuning exercise: “I don't think we need to tune this thing... just build it correctly the first time.” Verification must catch defects in collection, memory retention, failure handling and shutdown. No invented percentage or memory ceiling, preliminary benchmarking project or prescribed soak duration is part of this baseline.

## User Stories

1. As a desktop user, I want one overview of current system activity, so that I can check my machine while working.
2. As a user doing heavy work, I want the monitor to remain responsive, so that it remains useful when resources are busy.
3. As a user, I want CPU usage and attributable CPU temperature, so that activity and thermal readings are clear.
4. As a user, I want current RAM and swap use with capacity-based rails, so that I can judge memory pressure without extra text.
5. As a user with an NVIDIA, AMD or Intel GPU, I want the readings my hardware exposes, so that missing capabilities do not prevent the rest of the monitor working.
6. As a user, I want GPU usage, temperature and actual device-local memory use distinguished correctly, so that shared system memory is not mislabeled as dedicated VRAM.
7. As a user, I want separate network down/up and storage read/write cells, so that each direction is easy to find.
8. As a user, I want the capacity remaining on primary storage, so that I can see usable space without double-counting a filesystem shared by `/` and `/home`.
9. As a user, I want unavailable readings distinguished from real zeroes, so that I can trust the displayed values.
10. As a user, I want readings to recover after a source returns, so that I do not have to restart the application after a transient failure.
11. As a user with changing interfaces or devices, I want discovery and rate baselines to adapt, so that removal, replacement or reset does not produce fictitious spikes.
12. As a user, I want two readable columns at the smallest supported window size, so that the layout stays familiar as I resize it.
13. As a user with color-vision differences, I want explicit labels and distinct glyph shapes in a monochrome design, so that identifying a metric never depends on color.
14. As a user who relies on accessible text or larger fonts, I want meaningful values and status descriptions, so that the overview remains understandable without interpreting a rail visually.
15. As a user, I want system light/dark appearance, so that the window remains legible in my desktop theme.
16. As a user leaving the monitor open, I want retained work and memory to stay bounded, so that the monitor does not degrade a long session.
17. As a user closing the window, I want all collection and application processes to stop, so that there is no hidden background monitor.
18. As someone installing sys-mon, I want native installation instructions and accurate runtime requirements, so that I can run it on a supported system.
19. As someone evaluating a release, I want an honest record of tested hardware and unverified paths, so that fixture coverage is not confused with physical hardware validation.
20. As someone using or contributing to the project, I want its source published under MIT in the owner's personal public repository, so that its implementation and license are available.

## Implementation Decisions

### Native application and visual contract

Use matched Rust GTK4/libadwaita bindings. The repository currently contains a minimal Cargo package named `sys-mon` and a hello-world entry point; there is no existing application architecture or test harness to preserve. Keep the public telemetry boundary small and hide provider-specific parsing, discovery and driver handling behind it. The native UI consumes coherent current snapshots.

The final prototype amendments take precedence over earlier design proposals: monochrome replaces component colors; current status replaces graphs; the overview has no internal heading, secondary views, expansion controls or tabs. Native window decorations remain normal desktop behavior.

Use strong readable values, quieter labels, distinct decorative glyphs and aligned value starts within each column. Fill rails are square and integrated along the bottom edge of their cells. The approved prototype's minimum width is **640 CSS pixels**; it is evidence for the layout, not a command to force 640 native logical pixels regardless of fonts. Enforce a native minimum that preserves both columns, all labels and values under supported text scaling. Verify both minimum and wider widths in light and dark themes.

RAM, swap and VRAM display only their current used amounts with abbreviated units; their totals inform rails and accessible descriptions. CPU/GPU usage is a percentage. Temperature shows the current value and uses a hardware-reported max or target threshold for its rail when available. Clamp the visual rail without falsifying the text. Do not display endpoint labels. A valid temperature without a valid threshold still has valid text; do not invent a universal scale. The prototype's 80 °C CPU and 84 °C GPU thresholds describe the owner's hardware, not all systems.

The visible **Storage** value means remaining capacity. Its rail represents used capacity, as in the approved prototype, and its accessible description makes both meanings clear. Do not append a visible “free” qualifier. Throughput uses separate **Network down**, **Network up**, **Storage read** and **Storage write** cells; rates do not receive invented maximum-capacity rails. The owner approved the later compact UI as shown on 2026-10-06, including the gray bottom tracks under Network down and Network up. Those tracks are decorative: they have no fill, value or implied maximum. Storage read and Storage write have no track.

Use accessible text for unavailable states and meaningful descriptions of values, units and rail scales. A glyph supplements its text label. Preserve true zero readings, and never divide by zero or claim a capacity percentage when total capacity is unknown or zero.

### Metric sources and semantics

The researched source route is kernel interfaces for host metrics, NVML for NVIDIA, amdgpu sysfs/hwmon for AMD, and capability-checked Intel Level Zero Sysman where available. Load optional vendor facilities so missing drivers or libraries do not prevent the application starting. Support remains conditional on what the running Linux system exposes to the ordinary user.

| Reading | Contract |
| --- | --- |
| CPU usage | Derive aggregate activity from cumulative CPU counters with a valid previous baseline. Document the idle/iowait convention and verify the calculation. |
| RAM and swap | Use the kernel's memory semantics, including available memory; do not equate free RAM alone with all usable RAM. |
| CPU temperature | Match sensor driver/labels to the CPU package or die; do not select an arbitrary temperature channel and call it CPU temperature. |
| GPU usage, memory and temperature | Normalize the provider's distinct fields and capabilities. Device-local memory occupancy is distinct from memory bandwidth activity. |
| Intel/shared memory | Do not invent dedicated VRAM from shared system memory or a whole-device percentage by summing unrelated engine percentages. Use unavailable status when the overview's metric cannot be established honestly. |
| Network rates | Use monotonic counter deltas and avoid overlapping physical, bridge and VPN accounting. |
| Storage rates | Normalize cumulative sectors/bytes and choose a nonoverlapping block layer; never sum both a partition and its parent or a mapped device and its backing device. |
| Remaining capacity | Use space available to the ordinary user and deduplicate shared primary-storage capacity by filesystem identity. |

Discover CPUs, GPU identities, sensors, interfaces, block devices and mounts at runtime. Avoid fixed device names such as `card0` or a particular `hwmon` number. Handle multiple devices internally without introducing an extra-device screen. Record deterministic selection or valid aggregation rules during telemetry preparation; never sum overlapping counters or present an arbitrary sum of percentages as a meaningful aggregate.

**Primary storage** has the meaning in [CONTEXT.md](../CONTEXT.md): storage backing `/` and `/home`, with shared capacity shown once. Distinct primary filesystems must remain separately identifiable in the overview; shared Btrfs subvolumes must not produce duplicate capacity readings or a summed total.

A newly discovered or reset counter needs a fresh baseline before reporting a rate. Missing counters, invalid elapsed intervals and suspend/resume discontinuities must not become negative values or synthetic spikes. Keep unavailable status separate from a valid measurement of zero.

### Collection, bounded state and shutdown

Use a one-second refresh for changing readings as the initial engineering default. Slow-changing discovery or capacity checks may run less often on a bounded schedule. This default does not introduce a polling-settings surface.

Sleep between collection opportunities. Prevent overlapping work for a provider, catch-up polling queues and unbounded retry loops. Deliver current snapshots through bounded/coalescing transport so a busy UI does not accumulate obsolete updates. Redraw in response to current data and actual display changes, without continuous decorative animation.

Retain current readings, previous counters needed for rates, and current discovery state. Remove obsolete device state. Work, diagnostics and memory must not grow with elapsed runtime or repeated device churn; state may scale with devices currently discovered. There are no historical series to retain.

Run potentially blocking work outside the GTK main loop. Failure of one source must leave unrelated sources and the window usable. Replace invalid readings with unavailable status rather than keeping stale values presented as current. Recover automatically at a bounded retry pace; rate recovery requires a fresh baseline.

Handle slow or stuck optional providers so they cannot block shutdown, accumulate workers or freeze unrelated metrics. Merely setting a cancellation flag around an unbounded foreign call is insufficient. Select a practical bounded failure/cancellation design during implementation.

Closing the sole window ends scheduling, collection, owned helpers and the application. Verify this with normal operation, errors and in-flight collection. Minimized-window behavior requires no new product feature; avoid unnecessary painting and keep any continuing collection bounded. Exact desktop placement is outside scope.

### Publication and installation

Use lowercase **sys-mon** for the displayed product and intended executable, Cargo package and repository basename. Recheck final repository, application and package identifiers immediately before publication without reopening the rejected acronym exercise.

Publish source and a release in the owner's personal public GitHub account with the MIT license. Supply native installation/release packaging and source-build instructions, appropriate desktop metadata, and explicit GTK/libadwaita/runtime dependencies. The packaging research recommends native RPM/DEB packages; the actual supported distro versions and architectures must be established from build and installation evidence during release preparation. Do not claim an untested support matrix.

Document metric availability and ordinary-user permission limits. The release evidence must distinguish live hardware checks, fixture checks and unverified paths, including display backends actually exercised. Include reproducible verification commands and observed outcomes. Publication and packaging have not occurred merely because this specification exists.

## Testing Decisions

Use the **telemetry session's snapshot-and-stop contract** as the main automated testing seam. Inject source responses and elapsed time behind that boundary so tests can drive public behavior without real vendor hardware or wall-clock waiting. Rust's standard test harness is the initial choice; this repository has no prior harness. Keep parser fixtures behind that same subsystem boundary where practical, with focused parser tests only where they add coverage of difficult input semantics.

Prove observable results: normalized values, unavailable status, truthful rate baselines, independent source failure, recovery, and termination. Avoid tests that merely mirror private helper calls or exact internal structures.

Required coverage includes:

- CPU, memory, swap, GPU and sensor units; real zero versus missing or unsupported readings; unknown/zero capacities and absent temperature thresholds.
- Counter warm-up, reset, removal, invalid intervals and resume; network/block overlap; shared and distinct primary filesystems.
- Multiple devices, changed identities, source errors and permission failure; AMD/Intel fixtures cover complete, partial and unavailable responses.
- Slow sources, repeated failure and recovery, backpressure, many scheduled ticks and repeated device churn, proving no expanding work queue or retained obsolete state.
- Stop during ordinary collection, failures and in-flight work; verify application/helper termination at the process boundary as well as the sampling boundary.

Verify the native window against the approved screenshots with live telemetry and representative unavailable states. Check minimum/wide sizes, system text scaling, themes, accessible labels, aligned values, current-only memory text, correct storage meaning and hardware-scaled temperature rails. Exercise the complete window on available Wayland/X11 environments and disclose any backend not actually tested.

Perform a practical local run under ordinary use and representative CPU/GPU/storage activity. Check responsiveness and resource behavior for busy loops, leaked state, growing queues, repeated worker creation or runaway retrying. Fix any observed defects. The live NVIDIA path is checked on the available host; AMD/Intel fixtures establish software behavior, not physical compatibility.

No earlier proposed CPU/RAM number, fixed growth allowance, two-hour soak, 30-minute stress duration or two-second exit deadline was approved. The acceptance gate is correct, bounded behavior supported by meaningful tests and practical live verification. An actual weeks-long run and physical unavailable GPUs are not release gates. Test harness timeouts are engineering safeguards, not product performance promises.

## Out of Scope

- History graphs, retained historical series, per-core/extra-device detail views, expansion controls, Processes/File Systems tabs and an internal overview heading.
- A collector or service that remains active after the window closes.
- Exact window-position restoration or control of GNOME Wayland placement.
- A dedicated tuning or preliminary benchmarking project and arbitrary numeric resource-budget acceptance gates.
- Renaming the product through another acronym exercise.

Flatpak/AppImage distribution is not part of the settled first-release route; do not add it from historical research alone. Unavailable physical hardware and a weeks-long soak are verification limitations to disclose, rather than prerequisites to publication.

## Further Notes

The authoritative decisions are `t-1aa5ed` (metric sources), `t-529845` (native stack, with its graph proposal superseded), `t-414b23` (name), `t-67afd4` and its final amendments (approved appearance), and `t-ae35e5` (reliability). Research reports may retain superseded naming, graphs, color or extra-detail proposals; current map decisions and this specification take precedence.

Evidence references:

- [Linux metric sources](research/linux-metric-sources.md).
- [Rust desktop stack research](research/rust-desktop-graph-stack.md).
- [Release packaging research](research/name-and-release-route.md); its proposed name is obsolete.

Provider/runtime availability for Intel, CPU sensor attribution on other machines, exact supported dependency versions, and distro/architecture packaging coverage remain engineering checks during implementation. Resolve them using capability detection, accurate unavailable states and documented evidence. They do not authorize claims of universal hardware support or additional product surfaces.

The complete settled scope was compared with the existing M1–M3 descriptions at handoff. Those descriptions cover telemetry and tests, the integrated overview and lifecycle, and reliability verification plus publication; no milestone addition or scope edit is needed. No execution work has been delivered by this decision phase. Continue with `implement wf-48ebf7` in a fresh session to prepare and approve the execution plan.
