---
name: "code-review"
description: "Review current files or an explicitly requested Git comparison against project standards and specification sources."
---

# Code review

Review both engineering quality and requirement coverage. Small, low-risk changes may be self-reviewed by their implementer with targeted checks. Substantial implementations, meaningful correctness risks, or an explicit request for independent review use one independent read-only reviewer. An agent already assigned independent review performs it directly; it does not spawn another layer. Split review among specialists only when distinct risks or scope justify it.

- **Standards:** repository guidance and relevant design/engineering risks, using the smell baseline below as judgment aids.
- **Specification:** behavior against the direct request, approved specification, settled discussion or Wayfinder decisions.

Reuse the implementer's recorded verification; run additional checks only for a specific gap or risk you found, limited to the tests that close it — never a full suite or a rerun "just in case". Scale depth to the change: a small fix needs its diff, surrounding behavior and targeted evidence; a feature needs its contracts, failure modes and affected consumers. High-risk permission, security or migration changes may need deeper review regardless of line count.

## Select and freeze the scope

### Current-files mode

This is the default for generic review requests and plain directories. Identify the implementation's relevant changed/current files from the current session, task graph, and workspace. Freeze the file list and source description before review. Pause writer edits during review or use a frozen snapshot so the candidate stays fixed. Do not ask for repository initialization.

### Explicit Git-comparison mode

Use only when the human explicitly names a commit, tag, branch, merge base, or revision, or when an approved orchestration skill supplies the exact comparison revision. For an `implement` review, the writer supplies the qualified integration base SHA, frozen candidate SHA and owned scope; no additional human confirmation is needed. Verify both revisions. Freeze `git diff <base>...<candidate>` and the relevant commit list once. Review the ticket change and affected consumers without re-reviewing earlier tickets. Reject invalid revisions or an empty claimed change. Review the same frozen scope throughout; never review moving HEAD.

## Specification source priority

1. Direct request and approved plan in the current session.
2. Named Wayfinder map and its decisions: `wayfinder open <map>` for the projection, `wayfinder open <map>/<ticket>` for one ticket, `wayfinder export <map>` for the whole document.
3. Named workspace specification or design document.
4. Another local source explicitly linked to the work.

If none exists, report that requirement coverage cannot be assessed and review Standards only. Never bootstrap an external tracker.

## Standards sources and smell baseline

Apply repository instructions already supplied in context. Read additional referenced standards, CONTRIBUTING, domain documents, and applicable ADRs as needed. Read AGENTS.md or CLAUDE.md itself only when editing or auditing that file, or investigating missing or truncated guidance. Documented repository rules override this baseline. Every smell is a labelled judgment call, never a hard violation; skip concerns tooling already enforces.

Block on a concrete consequence for approved behavior, data integrity, authorization, usability or an explicit engineering requirement. For edge-case findings, identify a reachable situation, a concrete consequence and its connection to the changed behavior or affected consumers. Trace it from actual contracts and callers. Keep optional refinements separate from clearance.

Inspect public ownership and affected consumers: duplicated semantics, cross-capability persistence access, inconsistent contracts, unsupported abstractions and missing generation can create real delivery defects. Scope corrections to the ticket's acceptance and its affected consumers.

## Navigation and common-change probes

For a changed owner, start at the repository guide and locate its public entry,
contract, implementation and meaningful test without the writer explaining the
path. Identify where a plausible extension or fix belongs. Flag a broken or
misleading path with its concrete consequence; file count alone is not a finding.

For recurring behavior or composition, trace one common change through its owner
to every intended consumer. Check for handwritten peers that would miss that
change, including layout frames, async request lifetimes and retry identity.
Confirm any retained similar code has differing rules recorded at its owner.
Check public runtime exports for actual outside callers when the export surface
changes; internal tests alone do not justify a production public export.

## Verification evidence

Read the implementer's candidate, commands and verdicts. A role handoff alone does not justify rerunning tests, lint, formatting or typecheck. Check structural acceptance as well as behavior and test coverage. Passing checks or promised cleanup do not satisfy required ownership, typing, consolidation or code generation; report the unmet criterion as a finding.

Return findings to the writer. On revision, reuse the same independent reviewer for corrections that affect the original review’s correctness assumptions; retain unaffected clearance and refresh targeted evidence. Review remains read-only unless fixes are separately authorized.

## Report

Lead with actionable findings, ordered by severity, with file/hunk evidence and the requirement or rule involved. Distinguish documented violations from judgment calls; keep improvements outside the accepted scope separate from blockers. Assess both Standards and Specification without requiring two reports or ceremonial empty sections. If there are no findings, say so and state any material coverage limits. Identify the reviewed candidate and whether existing verification evidence was reused or supplemented.
