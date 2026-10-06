---
name: "wayfinder"
description: "Chart a new persistent Wayfinder map for an effort too large or uncertain for one agent session, or summarize an existing one."
---

# Wayfinder

Use Wayfinder for an effort too large or uncertain for one agent session. One schema-v2 effort keeps its decisions and its execution under one durable map id. Decision tickets settle the route; execution tickets deliver it. The phase makes the active plane explicit.

This skill opens efforts. Two verbs carry them forward, one ticket per session:

- `decide` works the next decision ticket.
- `implement` delivers the next execution ticket.

## Invocation

Interpret the direct user's text around the invocation:

- `/wayfinder <idea>`: interview and chart a new effort.
- `/wayfinder status <map>`: open and summarize it, naming the verb that carries it forward (`decide` in `decision`, `implement` in `decision_ready` or `execution`).
- `/wayfinder continue` or `/wayfinder work <map>/<ticket>`: these moved to `decide`. Load and follow `decide` with the same target.

## Driving the CLI

The main session runs `wayfinder session start` before charting. Never edit files under the Wayfinder home (`~/.wayfinder` or `$WAYFINDER_HOME`: maps, sessions, locks, backups) or map JSON by hand. Address tickets as `wf-xxxxxx/t-xxxxxx`; omit the map ID only when the workspace has exactly one active map.
Before the first map write, read [map rules](map-rules.md), installed beside this skill: how commands answer, phases, decision ticket types, milestones, subtasks, owner questions, scope changes, ticket sizing and what goes where. They apply in every phase.

## Subagents

The decision-only Research Analyst claims, researches and resolves an assigned decision research ticket under the explicit session ID you supply, and returns a receipt of at most ten lines.

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

## Chart a new effort

1. Load `grilling` and `domain-modeling`. Establish the destination, terms, observable completion state, and consciously excluded scope with the human.
2. Follow `grilling` to settle currently answerable decisions and expose dependent fog.
3. Obtain shared-understanding confirmation through `grilling` before charting; keep the confirmation question under 160 characters.
4. If the full journey fits this session and no fog remains, stop before persistence and ask how the user wants to proceed.
5. Run `wayfinder chart` once, with a short `name` (a few words, at most 80 characters, used as the map's label), the full destination, the milestone list, initial tickets (each typed per [Decision ticket types](map-rules.md#decision-ticket-types), with its `milestone`, `eli5` and up-front `subtasks` checklist), local-key dependencies, fog, and out-of-scope records in a single stdin payload. The effort starts in `decision` phase.
6. Report the destination and frontier. Start a Research Analyst subagent for each unblocked research ticket, in parallel, with the complete ownership protocol.
7. End by directing the user to `decide` for the first decision ticket.
