# M3 reliability preparation

Ticket `t-579f33`; integration prerequisite `63adc09449c289b3fc3f7a02eb311044766412ab`.
Authority: [specification](../wf-48ebf7-spec.md), active M3 description,
[M3 plan](t-a8d0ec-plan.md), and [effort brief](../effort-brief.md).

## Existing contract and scope

M1's [completion review](t-b25f7c-completion.md) verified the telemetry
inventory and fixture behavior. M2's [completion audit](t-f012be-completion.md)
verified the dashboard and close wiring. Their source and requirements have not
changed at this starting revision. This ticket adds reliability evidence and
repairs observed failures; it does not reinterpret a source or add a screen.

`TelemetrySession` owns one worker per registered provider and one latest
snapshot. Workers publish field-scoped patches after sampling, then wait an
interval. `stop` invalidates the generation, wakes workers and joins them.
Host and I/O sources read kernel files. CPU and I/O rate baselines are replaced
on each tick, removed on failure, and reset after invalid time. GPU and CPU
temperature vendor/sensor calls run in bounded, killable child processes;
their providers retain at most one unreaped child. GTK reads the latest
snapshot once per second, updates changed cells, and on close hides the window
while a background task stops telemetry before application quit. The native
app and helper modes share the executable.

The public consumers are `src/dashboard.rs` and the telemetry snapshot tests.
The preparation targets their established contracts. Any change to the
sampling or child lifecycle will be traced against these consumers and
reviewed before implementation if it materially changes the protocol.

## Verification mapping

| Required behavior | Source and witness |
| --- | --- |
| Bounded work, CPU, memory and helper count | Inspect session scheduling and provider state; many-tick/churn tests plus measured live process samples under representative activity. |
| Independent source failure and recovery | Public session tests with injected failures, GPU/temperature helper timeout and return; check stale values disappear. |
| Slow or stuck optional source | Existing killable-helper tests; add a witness only for an uncovered failure exposed by audit or stress. |
| Responsive native window | Launch the ticket binary on the available desktop, observe updates and usable close during local activity. |
| Normal, error and in-flight close | Existing session and dashboard tests plus native process/child exit checks; distinguish fixture from live evidence. |
| General source hygiene | `cargo fmt --check`, `cargo test --locked`, `cargo clippy --all-targets -- -D warnings`, `cargo build --locked`. |

The live run uses Fedora 44, this host's Intel CPU, NVIDIA GPU and Btrfs
storage. Physical AMD/Intel GPU compatibility, remote mounts, another desktop
backend and an actual weeks-long soak remain explicitly unverified unless
separately exercised. No fixed resource ceiling is acceptance; measurements
must instead expose sustained growth, accumulating workers or runaway CPU.
