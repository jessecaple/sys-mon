# M3 reliability and public release plan

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
Those contracts and checks are a starting point for M3, not M3 stress or
release evidence. There is no README, license, desktop file, package recipe,
Git remote or release at this source. `docs/build.md` is source-build guidance
and states that release packages and a tested distribution matrix remain.

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
   so the release evidence describes the verified candidate.

2. **Package and publish the first public release** (`t-a38bf6`). One end-to-end release
   workflow: check final `sys-mon` product, executable, Cargo, desktop,
   package and repository identifiers; add MIT license, desktop metadata,
   native RPM/DEB package recipes and source-build/install instructions; build
   and install what this host can actually verify; document runtime libraries,
   ordinary-user permissions, optional vendor facilities, tested hardware and
   unverified paths without inventing a distro matrix. Run final source checks
   and a package launch/close journey. Publish the source and release to the
   owner's personal public GitHub repository after checking its account and
   repository identity. Record URLs, tags, package checksums and exact release
   artifacts. This ticket depends on the reliability ticket; packaging may
   be prepared early within its branch, but the public release qualifies the
   hardened integration source.

## M3 obligation mapping

| Milestone obligation | Owning outcome and witness |
| --- | --- |
| Bounded memory, CPU and work over prolonged heavy use | Reliability: stress run observations and long-tick/churn assertions; repair observed leaks, queues or runaway retries. |
| Recoverable sensor failures and source independence | Reliability: injected failures and return, stale reading invalidation, live unavailable behavior where reproducible. |
| Lifecycle and clean shutdown | Reliability: normal, failed and in-flight stop tests plus process and child readback after native close. |
| `sys-mon` identifiers | Release: inspect Cargo, binary, window, desktop, packages, repository and release naming immediately before publication. |
| Installation, native packaging, runtime dependencies | Release: package recipes, source guide, built RPM/DEB as feasible, install/launch evidence and honest support matrix. |
| Supported and unverified hardware paths | Reliability observations feed release documentation; distinguish physical NVIDIA and local host from AMD/Intel fixtures and untested environments. |
| MIT, public personal repository and release | Release: license, verified personal account/repository, public source and tagged downloadable artifacts with readback. |

The release ticket includes ordinary packaging mechanics and documentation;
the reliability ticket includes repairs found by its checks. Neither needs a
new shared subsystem or a separate test ticket on current evidence. M3
closeout later combines the two deliveries and independently reviews the
active milestone's full acceptance; this plan does not claim completion.
