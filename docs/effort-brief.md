# sys-mon implementation brief

Wayfinder map: `wf-48ebf7`. Approved behavior: [specification](wf-48ebf7-spec.md). The map owns the active milestone and execution queue.

Integration is Git branch `main` at `~/code/sys-mon`. Its execution baseline is `01c6ec320115135e1944af0bd76d897ad69fa13a`. Each ticket uses a branch and worktree from current integration and lands only through `wayfinder land`. Do not edit integration directly for ticket work.

The code is a Rust 2024 Cargo package. Use Rust's standard test harness for telemetry and presentation. Run `cargo fmt --check`, `cargo test`, and `cargo clippy --all-targets -- -D warnings`. Native build requirements are in [build.md](build.md). The available host is Fedora 44 x86_64 with an Intel Core i9-13900K, an NVIDIA RTX 4090, `coretemp` package readings, and Btrfs `/` and `/home` subvolumes. Hardware availability is an observation, not a claim of cross-machine coverage.

Durable ticket evidence and reviewer handoffs live under `~/.wayfinder/evidence/wf-48ebf7/`. Ticket preparation and technical contracts are committed in `docs/verification/` or owning code documentation. Qualify evidence against exact candidate and integration SHA, source fixtures and runtime inputs. Record physical-hardware versus fixture-only checks separately.

No repository runtime launcher or database setup exists. Ticket worktrees isolate Rust output by their own `target/` directory. Keep any spawned application or helper owned by the ticket and stop it after QA. There is no existing remote or publication setup; the release ticket will establish those after identifier and package review.
