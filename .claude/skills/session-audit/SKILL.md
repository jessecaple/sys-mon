---
name: session-audit
description: Audit past agent sessions in this repository and propose a few evidence-backed edits to its agent instructions and the Wayfinder workflow, applied only with the owner's approval.
disable-model-invocation: true
---

# Session audit

A one-time review of past agent sessions, run when the owner asks. It reads what agents actually did, measures that against the instructions they had, and proposes a small set of edits. The owner accepts or rejects each one. Nothing runs between audits, and nothing changes without the owner's approval.

## 1. Collect the sessions

Audit the sessions run in this repository since the last audit recorded in the audit log (step 5). With no log, ask the owner for the period; two weeks is a sensible default.

- Claude Code keeps one JSONL transcript per session in `~/.claude/projects/<working directory with each / replaced by ->/`, with subagent transcripts in a directory named after the session. Worktrees of this repository have their own project directories; include them.
- Codex keeps sessions in `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl`. Keep only those whose session metadata names a working directory inside this repository.

Read transcripts; never change them. Distill each one with a script into the owner's messages, the agent's decisions and one-line tool summaries, and open the raw transcript only to verify a quote. Redact secrets (keys, tokens, passwords, connection strings, personal data) from anything you quote. With many sessions, audit them in parallel subagents that each return findings only.

## 2. List the instructions under audit

Give every instruction a short stable id (`AG-12`, `implement-7`) so findings can cite it:

- the repository's AGENTS.md and CLAUDE.md, one id per rule;
- project skills, by their description, which is their trigger;
- the Wayfinder skills and role definitions installed by `wayfinder init` (`implement`, `wayfinder`, the `wayfinder-*` agents).

Mark each source's owner: the repository, or the Wayfinder bundle. `wayfinder init` overwrites installed Wayfinder files, so they are never edited in place.

## 3. Audit each session

Record only what the transcript shows, each finding with a verbatim quote:

- **Harm:** the agent followed an instruction, and following it caused damage or wasted work. This is evidence against the instruction.
- **Non-compliance:** the agent ignored or broke an instruction. This is evidence the instruction failed to steer: reinforce it, move it or sharpen it; never delete it for being ignored.
- **Gap:** a mistake no instruction covers, with one imperative sentence that would have prevented it and its owner:
  - *repository*, when this codebase caused it;
  - *workflow*, when the Wayfinder process, a role or the harness caused it;
  - *failed trigger*, when a skill already covers it but did not load; the fix is that skill's description.

The owner's corrections are the strongest gap signal: "stop doing X", "too verbose", reverted work, a repeated question. Do not credit an instruction for an outcome that would have happened anyway. A session with no findings is a valid result.

## 4. Fold the findings and propose edits

Merge findings that one change would fix, and count the distinct sessions behind each. Propose at most five edits, the highest-signal first:

- Every edit needs evidence from at least two distinct sessions. One bad session never changes the instructions.
- Removing an instruction needs harm in at least two sessions.
- Amend the instruction that was in play before adding a new one. When a new rule contradicts an old one, rewrite the old one in the same edit, so the instructions never contradict themselves.
- Place each rule where it costs least. Rules that apply broadly, or protect safety, stay in always-loaded instructions (AGENTS.md, role definitions). A narrow rule with a clear trigger belongs in a skill, whose description is the trigger. A narrow rule with no detectable trigger is a deletion candidate. Every always-loaded line is paid for in every future session.
- Skip edits the owner rejected in an earlier audit unless new evidence changes the case.

For each edit give the target file, the text before and after, the finding type, the number of sessions and one or two quotes. Edits to Wayfinder-owned files are proposals for the Wayfinder bundle's source: write them out for the owner, and do not touch the installed copies.

## 5. Apply and record

Present the edits and let the owner accept or reject each one. Apply the accepted repository edits, and hand over accepted Wayfinder edits as text. Then append to the audit log: the date, the sessions covered, and each edit with its decision and a one-line reason. Keep the log at `.local/session-audit-log.md` when `.local/` is git-ignored; otherwise ask the owner where it belongs. The log is the only file this audit creates.
