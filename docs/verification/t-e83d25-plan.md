# M2 dashboard delivery plan

Wayfinder: `wf-48ebf7`, planning ticket `t-e83d25`. Source baseline:
`e1315418bfbf2fc445d6675e030f9c956d1435e4` on `main`.
Authority: [approved specification](../wf-48ebf7-spec.md), M2 description in
Wayfinder, and the [dark](../prototypes/visual-overview-approved-dark.png) and
[light](../prototypes/visual-overview-approved-light.png) visual references.

M1 has delivered `TelemetrySession::start_system`, `snapshot`, and `stop`,
plus the current `Snapshot` value model in `src/telemetry/mod.rs`.
The integrated M1 completion review is
[t-b25f7c-completion.md](t-b25f7c-completion.md). The executable currently
prints a placeholder except for its diagnostic and internal helper modes.
There is no GTK application yet. At planning time, `pkg-config` cannot find
`gtk4` or `libadwaita-1` development packages on this host; the dashboard
ticket owns its build setup and must record the resulting runtime requirements.

## One complete delivery outcome

Filed one ordinary work ticket, `t-abb4ba`, **Deliver the native one-screen
dashboard**.
The whole window, its presentation model, accessible cells, telemetry
connection and close lifecycle form one user-facing screen. There is no
independent shared subsystem with another named consumer in M2. Do not split
ordinary cells, styling, formatting or lifecycle wiring into separate tickets.
The M1 telemetry contract is an integrated prerequisite, so the new ticket
needs no open-ticket dependency.

| M2 obligation | Delivery witness |
| --- | --- |
| Native GNOME Wayland/X11 window, normal lifecycle, resizable always-two-column overview | Launch the executable; inspect minimum and wider windows on available display backends. Close the sole window and verify application and telemetry helpers exit. |
| Exact six-row metric order and separate network/storage directions | Snapshot-driven UI check of all 12 named cells against the specification and approved visual references. |
| Current values, units, zero and unavailable states | Focused presentation tests using injected snapshots; rendered checks with real telemetry and representative unavailable readings. |
| RAM/swap/VRAM used amount only, capacity rails, CPU/GPU percentage, temperature thresholds, remaining storage with used rail | Presentation tests for valid, absent and zero totals; threshold absent/present; distinct primary volumes; clamp rails without changing text or inventing scales. |
| System light/dark, monochrome labels and glyphs, square integrated rails, aligned values | Rendered screenshots and visual inspection in both themes at minimum and wider widths, including supported text scaling. |
| Accessible descriptions and status without color or rail interpretation | Inspect accessible names/values for ordinary, zero, unavailable and capacity states; keyboard and text-scaling checks. |
| Live M1 telemetry and responsive, bounded updates | Real host run with snapshots outside the main loop; verify changed readings, independent unavailable fields, no queued history, minimized behavior and stop during in-flight work. |
| Excluded surfaces | Inspect the sole window for no internal heading, detail view, tabs, graphs or expansion controls. |

The dashboard ticket should retain exact screenshots, source candidate,
hardware/backend availability, commands, observable results and any remaining
unverified paths in its own durable evidence. The later M3 milestone owns
extended reliability verification, packaging and public release.
