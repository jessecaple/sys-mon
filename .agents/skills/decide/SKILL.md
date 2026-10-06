---
name: "decide"
description: "Work one Wayfinder decision ticket per invocation, the next ready one or a named one, then stop; finish decision work into a specification when none remain."
---

# Decide

`decide` works one Wayfinder decision ticket per invocation, then stops. Decision tickets settle an effort's route before `implement` delivers it; the `wayfinder` skill charts new efforts.

## Invocation

Interpret the direct user's text around the invocation:

- `decide`: run `wayfinder list`, then open the sole active effort or ask which one, and continue as for `decide <map>`.
- `decide <map>`: by phase:
  - **decision:** work the first ready decision ticket on its frontier without requiring the user to supply its id. If none is ready, report the blocker, or finish decision work when all decisions and fog are settled.
  - **decision_ready** or **execution:** report the phase and direct the user to `implement` rather than starting a decision ticket.
  - **complete:** report it and direct the user to `wayfinder <idea>` to chart a new effort.
- `decide <map>/<ticket>`: work that decision ticket.

## Driving the CLI

The main session runs `wayfinder session start` for single-session decision work. Concurrent sessions use `session start --isolated` and pass the returned `--session <id>` on every map command, including reviewer assignments, without replacing shared ambient attribution. Never mint a replacement while another session is using it. Never edit files under the Wayfinder home (`~/.wayfinder` or `$WAYFINDER_HOME`: maps, sessions, locks, backups) or map JSON by hand. Address tickets as `wf-xxxxxx/t-xxxxxx`; omit the map ID only when the workspace has exactly one active map.
Before the first map write, read [map rules](map-rules.md), installed beside this skill: how commands answer, phases, decision ticket types, milestones, subtasks, scope changes and what goes where.

## Subagents

The decision-only Research Analyst claims, researches and resolves an assigned decision research ticket under the explicit session ID you supply, and returns a receipt of at most ten lines. Independent reviewers read their ticket with `wayfinder open`, use the explicit session ID supplied by its owner, and return a receipt of at most ten lines. They add and close only their own permitted findings and never author the candidate. Reuse the same reviewer for a ticket's corrections.

## Writing for the owner

Shape everything you write for the owner, your replies in this session and every owner question, for a reader with ADHD:

1. Lead with what the owner needs to do or decide; context comes after, if at all.
2. Number steps and options, one action per step, recommendation first, at most five items per list.
3. Restate where the work stands every time (milestone, ticket, step); the owner holds nothing between messages.
4. Give concrete time estimates ("about two hours"), never "some work".
5. Show what now works in concrete terms instead of burying it in a recap.
6. Keep to one topic; offer a second issue as a separate question.
7. State problems as cause and fix, in a flat tone.
8. No preamble, recap, closing pleasantry or idiom; end with the one next action.

These rules do not apply to subagent assignments or to Wayfinder records other than owner questions.

## Work one decision ticket

1. Open the map at low resolution, then inspect only the chosen frontier ticket (`wayfinder open <map>/<ticket>`) and its brief.
2. The main session resolves at most **one decision ticket per session across all Wayfinder efforts**. Nothing enforces this; a session that settles several tickets fragments the reasoning behind each one. For research, **delegate before claiming** and stop — the Research Analyst subagent claims and resolves the ticket. Every other owner claims first; resolving without a claim is refused.
3. Gather evidence by type:
   - **grilling:** load and follow `grilling` and `domain-modeling`; resolve after a post-claim human response.
   - **research:** the Research Analyst loads `research`, writes a workspace Markdown report, and resolves with that report as a file artifact.
   - **prototype:** load `prototype`, present disposable evidence, ask the human directly for their reaction, and include the artifact.
   - **task/AFK:** perform only the prerequisite and record the facts later decisions need in its resolution.
   - **task/HITL:** give the human a precise checklist, confirm completion with them directly, and record the result in its resolution.
4. Keep the ticket's subtasks current while you work, then resolve once — every live subtask done — with the full answer, one-line decision summary, evidence artifacts, newly sharp tickets (typed per [Decision ticket types](map-rules.md#decision-ticket-types)), and stable fog ids to remove. The write is atomic. `decisionSummary` caps at 500 characters and `resolution` at 65,536; an over-limit write is refused with the actual length.
5. Close work honestly: `wayfinder rule-out --basis … --reason …` beyond the destination (see [Changing scope](map-rules.md#changing-scope)), `cancel` when no longer needed, `supersede` when a settled answer is wrong, and `amend` only when the answer and graph still stand; an owner-requested clean restatement uses `edit --basis owner` (see Restate cleanly under [Supersede, amend, and scope](map-rules.md#supersede-amend-and-scope)).
6. Report what was decided and the new frontier, then start a Research Analyst subagent for each newly unblocked research ticket, in parallel, with the complete ownership protocol. Stop; the next decision ticket needs a new `decide` invocation.

## Finish decision work

When no decision tickets or fog remain open, run `wayfinder finish`, then load `to-spec` and synthesize the settled route in this session. Write the specification where `to-spec` prescribes. Derive the complete ordered delivery-milestone roadmap from the settled decisions: give each milestone a key, name and description covering its capabilities, surfaces, integrations and acceptance obligations. Compare that full roadmap with `wayfinder milestones`, propose every missing or inaccurate milestone to the human, and record approved additions or edits with `wayfinder milestone add|edit`. Check that the map's milestone list covers the whole settled roadmap before stopping; the automatically created M0 does not stand in for delivery milestones. Then **stop** and direct the user to `implement` in a fresh session, which seeks approval for the execution plan and activates execution. Do not ask the human to run `/to-spec`; completing decision work already authorizes synthesis and the file write.
