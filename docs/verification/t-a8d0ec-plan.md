# M3 reliability and public source plan

Wayfinder: `wf-48ebf7`, planning ticket `t-a8d0ec`. Integration source at
planning: `454cebddfb551b66b3e7745f514d306274b63dc3` on `main`.
Authority: the active M3 description in Wayfinder and the
[approved specification](../wf-48ebf7-spec.md). The
[effort brief](../effort-brief.md) records the available host and build gates.

M1 delivered the bounded `TelemetrySession` and optional GPU/temperature
helpers. Its [completion review](t-b25f7c-completion.md) covers fixture
failures, recovery, churn, in-flight stop and one live diagnostic, while
explicitly declining a long soak. M2 delivered the native window and
close-to-stop wiring. Its [completion audit](t-f012be-completion.md) covers
current X11 renders, a successful window close and focused lifecycle tests.
Those contracts and checks were a starting point for M3, not M3 stress or
publication evidence. At planning time there was no README, license, desktop
file or Git remote. The owner narrowed the publication outcome on 2026-10-06
to a public source repository without a published release or binary packages.

## Delivery outcomes

1. **Harden and verify long-session reliability** (`t-579f33`). One coherent capability:
   practical local CPU/GPU/storage activity, bounded resource behavior,
   recoverable source failures and clean process/helper shutdown under normal,
   failure and in-flight conditions. Inspect and repair any observed defect
   within the existing telemetry or window owner. Automated witnesses should
   exercise many ticks and device churn without wall-clock soak, bounded worker
   and retained state, independent source recovery, stale-value replacement,
   and stop after a slow or failed source. Measure and record the local run's
   duration, workload, CPU/memory and responsiveness observations, child count,
   shutdown result and limits of available hardware. A real weeks-long run and
   physical AMD/Intel hardware are not gates. This ticket precedes publication
   so the public source notes describe the verified candidate.

2. **Publish sys-mon source in a public repository** (`t-a38bf6`). Check final
   product, executable, Cargo, application, desktop and repository identifiers.
   Add MIT license and Cargo metadata, desktop entry and icon, README, source
   build and local install guidance. Document runtime libraries, ordinary-user
   permissions, optional vendors, tested hardware and unverified paths. Run
   final source checks and a source-built launch/close journey. Verify the
   owner's personal GitHub destination, publish the source repository publicly,
   and read back its URL, visibility, license and exact commit. This ticket
   depends on the reliability ticket; the public source reflects the hardened
   integration source. No GitHub Release, tag or native binary package is part
   of this outcome.

## M3 obligation mapping

| Milestone obligation | Owning outcome and witness |
| --- | --- |
| Bounded memory, CPU and work over prolonged heavy use | Reliability: stress run observations and long-tick/churn assertions; repair observed leaks, queues or runaway retries. |
| Recoverable sensor failures and source independence | Reliability: injected failures and return, stale reading invalidation, live unavailable behavior where reproducible. |
| Lifecycle and clean shutdown | Reliability: normal, failed and in-flight stop tests plus process and child readback after native close. |
| `sys-mon` identifiers | Publication: inspect Cargo, binary, window, application ID, desktop entry and repository name before publishing. |
| Source installation and runtime dependencies | Publication: README, source guide, desktop entry, source-build and launch evidence, with only verified compatibility claims. |
| Supported and unverified hardware paths | Reliability observations feed public source documentation; distinguish physical NVIDIA and local host from AMD/Intel fixtures and untested environments. |
| MIT and public personal repository | Publication: license and Cargo metadata, verified personal account/repository, public source commit and readback. |

The publication ticket includes source documentation and desktop metadata;
the reliability ticket includes repairs found by its checks. Neither needs a
new shared subsystem or a separate test ticket on current evidence. M3
closeout later combines the two deliveries and independently reviews the
active milestone's full acceptance; this plan does not claim completion.
