---
name: "resolving-merge-conflicts"
description: "Use when you need to resolve an in-progress git merge/rebase conflict."
---

# Merge-conflict resolution

Use only for a direct request to resolve an already-active Git merge or rebase.

1. Verify the worktree and active operation. If absent, report that the skill is not applicable; never initialize Git or start an operation merely to satisfy it.
2. Inspect status, local history, conflict stages, the merge/rebase goal, the current session, and user-provided context. Reconstruct why each side changed before editing. External sources are optional only when explicitly requested.
3. Resolve every hunk while preserving both intents where compatible. When intents conflict, choose the behavior matching the operation's stated goal and record the tradeoff. Do not invent unrelated behavior and do not abort the requested operation.
4. Preserve unrelated changes. Stage only resolved conflict paths.
5. Run the project's complete relevant checks—not only tests for manually edited hunks—because cleanly merged interactions can break integration. Fix failures introduced by combining the histories.
6. Continue the active merge or rebase. For a rebase, repeat inspect → resolve → stage → check → continue for every conflicted commit until the operation completes.
7. Do not create extra commits, push, publish, force-clean, or rewrite unrelated history. If signing or another human-only boundary blocks continuation, report it exactly and wait.

Finish with resolved paths, reconstructed intents, tradeoffs, checks, and final operation state.
