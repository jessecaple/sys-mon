# M2 one-screen dashboard completion audit

Ticket `t-f012be`; integration source `45a84b5846f6f8389dcd4935fc0afef4b86df096`.
The active M2 description and [specification](../wf-48ebf7-spec.md) are the
acceptance authority. This report qualifies the delivery evidence after the
additional compact-layout commits `0cf038d` and `45a84b5`. Raw captures and
the independent review are stored under
`~/.wayfinder/evidence/wf-48ebf7/`.

| M2 obligation | Current source and qualified witness |
| --- | --- |
| Native GNOME Wayland/X11 sole window | `src/dashboard.rs` creates one GTK4/libadwaita application window. Earlier Wayland launch and X11 use are recorded in `t-abb4ba-evidence.md`; the layout commits changed neither backend selection nor telemetry startup. The current X11 build launched and exited normally. |
| Six exact rows, twelve key metrics and concise values | `src/presentation.rs::present` fixes row order and field formatting. The current live X11 compact and wide captures show all twelve readings. Existing focused presenter tests cover units and all fields. |
| Current-only RAM, swap and VRAM; capacity-based rails | `src/presentation.rs::memory` keeps total out of visible text and uses it only for the accessible description and rail. Existing tests cover positive and unknown totals; live captures show current amounts. |
| Separate network and storage directions without invented maxima | Four separate cells are in the current live captures and presenter output. Rate display lines have no rail fraction. The compact layout adds visible gray tracks under Network down/up. The owner confirmed on 2026-10-06, “Yes Keep. THE UI IS HOW I WANT IT.” Question `q-b6d473` is closed; the specification now states that these tracks are decorative, without a fill, value or implied maximum. |
| Temperature thresholds with no visible endpoints | The presenter uses only reported target/max or critical thresholds and retains text without one. Existing tests cover both threshold kinds, missing scale and clamping; live captures show temperature fills without endpoint labels. |
| Primary storage remaining and distinct volumes | Presenter tests cover shared, distinct, partial, reserved and zero-total cases. The current distinct-storage capture shows separately labeled `/` and `/home` values and rails; live Btrfs shows one remaining amount. |
| Available, unavailable and real zero states | Existing presenter and dashboard tests cover field-local reasons, startup failure, zero and recovery. The current unavailable fixture capture shows mixed statuses and real zero alongside them. |
| Always two columns and readable minimum/wide layouts | `src/dashboard.rs` uses a homogeneous two-column grid and GTK natural minimum. Current X11 compact captures measure 570 pixels wide with ordinary or unavailable data and 656 pixels with distinct storage. A 1.5× scaled distinct-volume view reported a 920-pixel natural minimum; its captured view remained readable. Current live wide and scaled wide captures show both columns. |
| System light/dark, monochrome and non-color cues | Current light/dark captures show system color-scheme changes, matching glyph shapes and explicit labels. The dark preference and text scaling were restored to `default` and `1.0`. Earlier design review and approved visual references remain the comparison basis. |
| Accessible descriptions and status encoding | `src/dashboard.rs` binds presenter descriptions to labels and rails; the earlier AT-SPI readback remains applicable because the compact commits did not change those bindings. The new decorative network tracks are presentation-only. |
| No internal header, detail screen, tabs or graph | Current captures and window source show only native window controls and the metric grid. No secondary view or history storage was added. The owner confirmed the visible product title should be `sys-mon`; the corrected title appears in the final native capture and X11 `WM_NAME`. |
| Live snapshots, bounded UI updates and shutdown | The existing `TelemetrySession` integration and focused lifecycle tests remain unchanged. Current physical X11 readings updated, and an X11 WM delete closed the live process with exit code 0. Earlier in-flight and failure tests remain applicable. |

## Refreshed QA and limits

Fedora 44 x86_64, GNOME Wayland host with an X11 app backend, rustc 1.98.1,
GTK 4.22.5 and libadwaita 1.9.4. `cargo test -q` passed 37 library and 13
integration tests; `cargo fmt --check` and `cargo clippy --all-targets --
-D warnings` passed after the compact grid index correction. The current
source uses the build environment and hardware described in
`docs/effort-brief.md`.

Current captures in `t-f012be/`: `x11-unavailable-compact.png`,
`x11-unavailable-dark-compact.png`, `x11-distinct-dark-compact.png`,
`x11-distinct-dark-scaled.png`, `x11-distinct-dark-wide-scaled.png`,
`x11-live-light-compact.png`, `x11-live-light-wide.png` and
`x11-live-light-final-title.png`. The live session
reported CPU, memory, NVIDIA GPU, network, storage and temperature readings.
The unavailable and distinct-storage states use the established debug fixture
inside the production widget tree.

The changed source is layout and window chrome. Existing presenter,
telemetry-provider, runtime discovery, lifecycle and Wayland launch evidence
remain qualified for their named behavior. Wayland screenshot capture remains
unavailable on this GNOME session. Physical AMD and Intel GPU coverage and a
weeks-long soak are outside the M2 gate; M1 fixture and later M3 reliability
evidence describe those limits.

Independent completion review: the first pass is in
`~/.wayfinder/evidence/wf-48ebf7/t-f012be-independent-review.md`.
The reviewer closed the network-track finding after verifying the owner's
approval and the updated specification. The visible title and Clippy failure
were corrected in this closeout branch. Completion awaits the same reviewer's
final source and evidence readback.
