# M3 reliability and public source completion audit

Wayfinder `wf-48ebf7`, closeout ticket `t-8aeab6`. Integration source audited:
`6b1ecf2c8c43ffae63d68bce22230e4cd3378e76` on `main`. The active M3
description and [specification](../wf-48ebf7-spec.md) govern acceptance. The
closeout branch adds this report only. Detailed measurements, commands, live
captures and reviewer receipts are retained under
`/home/jcaple/.wayfinder/evidence/wf-48ebf7/`.

| M3 obligation | Integrated owner and qualified evidence | Assessment |
| --- | --- | --- |
| Bounded work, memory and CPU during prolonged heavy use | `src/telemetry/mod.rs` owns one latest snapshot and bounded workers. `src/telemetry/io_provider.rs` replaces device baselines each tick and confines capacity lookup to one bounded helper. [Reliability receipt](/home/jcaple/.wayfinder/evidence/wf-48ebf7/t-579f33.md) records the 10,000-tick churn assertion, helper timeout/recovery witnesses and a 75-second native CPU/GPU/storage run: 145 samples, post-warm-up RSS 159,616–164,264 KiB, 10–11 threads, 37–39 descriptors and about 0.8% of one core. | Covered by tests and representative local stress; no numeric product ceiling or weeks-long soak claimed. |
| Recoverable sensor failure and independent readings | `tests/telemetry.rs` exercises injected elapsed time, source errors, device churn and recovery through public snapshots. `src/telemetry/io_provider.rs` tests capacity timeout, malformed output and recovery while network/storage rates remain valid. Earlier [M1 completion review](t-b25f7c-completion.md) qualifies CPU, GPU, temperature, network, storage and mount fixtures. | Covered on fixtures and available Intel CPU/NVIDIA GPU/Btrfs host. Physical AMD/Intel GPUs and remote mounts remain unverified. |
| Slow or stuck source and clean normal, failed and in-flight shutdown | Capacity, GPU and temperature helpers have killable bounded process boundaries; `TelemetrySession::stop` invalidates the generation and joins workers. `src/dashboard.rs` hides the window, stops telemetry off the GTK loop, then quits. [Reliability receipt](/home/jcaple/.wayfinder/evidence/wf-48ebf7/t-579f33.md) records native normal and unavailable-fixture close at 0.114 seconds, and paused capacity-helper in-flight close at 0.716 seconds with helper PID absent. Session and dashboard tests cover failure stop and late-patch rejection. | Covered on the available X11 desktop and focused tests. An uninterruptible kernel task cannot be guaranteed to reap immediately; its helper count stays capped. |
| Name and final identifiers | `Cargo.toml` names the package and binary `sys-mon`; `src/dashboard.rs` titles the window `sys-mon`; `src/dashboard.rs` and the desktop entry use `io.github.jessecaple.SysMon`; Cargo and the Git remote name `jessecaple/sys-mon`. The [publication receipt](/home/jcaple/.wayfinder/evidence/wf-48ebf7/t-a38bf6.md) records source-built and installed desktop launch/close with matching WM class. | Covered on source and live desktop. |
| MIT license and accurate source build/local installation | Root [LICENSE](../../LICENSE) and Cargo metadata record MIT. [README](../../README.md) gives a short Fedora source route; [build guide](../build.md) gives the current-user install files, GTK/libadwaita development and runtime dependencies, permission limits, optional vendor facilities, verification commands and unverified hardware paths. The publication receipt records fmt, 41 library + 13 integration tests, Clippy, debug/release builds, desktop validation and a license/dependency audit. The [README trim receipt](/home/jcaple/.wayfinder/evidence/wf-48ebf7/t-e7f68c.md) verifies its docs-only change and local links. | Covered for the documented Fedora 44 x86_64 host; no untested distro or architecture promise. |
| Personal public source repository | `https://github.com/jessecaple/sys-mon` is public, owned by personal user `jessecaple`, has `main` as default and GitHub's MIT license recognition. Current GitHub API and `git ls-remote` readbacks return `6b1ecf2c8c43ffae63d68bce22230e4cd3378e76` for `main`, matching integrated source. The publication and README trim receipts record creation and the single later push. | Covered by current remote readback. No GitHub Release, tag or binary package is required or claimed. |

## Evidence qualification and combined journeys

The reliability source landed at `406201c4f496c914884560a8480387b255d7bcd6`.
The publication commit `337ed1bfa66288161b5b5bfce7debaddba7d63b7` changed
metadata, desktop assets and documentation, then passed the full source checks
and native direct/installed launch-close journeys. The subsequent integration
commit `6b1ecf2` changed only `README.md` and `docs/build.md`; its link and
command review passed and it did not change executable source, fixtures,
desktop assets or runtime configuration. The earlier M1 fixture and M2
dashboard evidence remains qualified for unchanged contracts. The M3 native
stress and shutdown evidence remains qualified for the identical production
source. No combined native journey was invalidated by the docs-only change.

At this audit, GitHub's repository API returned owner `jessecaple`, public
visibility, default branch `main`, MIT license and the expected URL. Its
`main` ref and `git ls-remote origin refs/heads/main` both returned the
integrated SHA above; no remote tags were returned. The source still routes
one twelve-cell GTK window through current `TelemetrySession` snapshots and
stops the session on window close. Public documentation links and commands are
the ones verified in the README trim receipt. This readback completes the
public-source portion of the combined journey on current integration.

The tested desktop was Fedora 44 x86_64 with an Intel Core i9-13900K, NVIDIA
RTX 4090, Btrfs `/` and `/home`, GTK 4.22.5 and libadwaita 1.9.4. Native X11
was exercised on a GNOME Wayland host; an earlier Wayland launch was observed.
Physical AMD/Intel GPU paths, remote primary mounts, other distributions or
architectures, and a weeks-long soak are unverified and are disclosed in the
build guide. The owner narrowed M3 to a public source repository, so release
artifacts and packages are outside acceptance.

## Independent completion review

Pending review by a `wayfinder-requirements-reviewer` who did not implement
M3 work or author its inventories. The reviewer must inspect the milestone
description, source, approved exception and qualified evidence for every row.
