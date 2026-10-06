---
name: "to-spec"
description: "Synthesize settled discussion or a completed Wayfinder map into a written specification."
---

# Spec synthesis

Turn settled discussion or a completed Wayfinder map into an implementation-ready specification. This is synthesis, not another interview.

## The session is the destination

The session itself is the default spec artifact: deliver the specification in your reply. Never require a Git repository, branch, remote, external tracker, or hosting account. Write a workspace Markdown file only when requested, when the effort's working knowledge requires one, or when completing a Wayfinder effort (see the final section). Do not publish externally without explicit authorization.

## Ground the synthesis

1. For a Wayfinder reference, open the map — `wayfinder export <map>` dumps it whole — and inspect resolved tickets needed to recover decisions, evidence, prototypes, fog disposition, and out-of-scope boundaries.
2. Otherwise use settled discussion from the current session.
3. Inspect the relevant codebase shape before specifying implementation: public modules, domain glossary or CONTEXT documents, applicable ADRs, repository guidance, and similar tests. Use established domain vocabulary.
4. Identify the highest observable testing seam that can prove the behavior. Prefer an existing public seam and existing test harness; add a new seam only when no adequate one exists. Minimize seam count—one coherent seam is ideal.
5. Treat already agreed testing seams as settled. If a genuinely unresolved seam choice materially changes the product or architecture, ask one focused confirmation; otherwise record the best-supported choice and continue.
6. Do not reopen settled decisions. Put minor uncertainty in Further Notes; ask only when contradictory requirements prevent an honest specification.

## Output

Produce:

1. **Problem Statement** — the problem from the user's perspective.
2. **Solution** — intended outcome and behavior.
3. **User Stories** — an extensive numbered set covering actors, normal flows, boundaries, errors, accessibility, operations, and migration where relevant. Format each as “As an …, I want …, so that …”.
4. **Implementation Decisions** — settled architecture, modules, interfaces, domain model, schemas, contracts, constraints, migration, and prototype-derived decisions. Avoid brittle file paths and full code snippets unless a compact prototype state machine or type shape is more precise than prose.
5. **Testing Decisions** — observable behavior, selected highest seam, harness and prior art, failure cases, and what makes a good external-behavior test.
6. **Out of Scope** — explicit boundaries.
7. **Further Notes** — remaining non-blocking uncertainty and handoff context.

For a Wayfinder effort, the roadmap's milestones live in the map's milestone list (`wayfinder milestones`), not in the specification. Describe route policies and acceptance rules here and reference milestones by key. At the decision handoff, derive the complete ordered milestone roadmap from the settled decisions, compare it with the map, and propose every missing or inaccurate milestone for the human's approval under the Wayfinder milestone rules. Record approved milestones in the map rather than restating the list in the specification.

When invoked automatically at Wayfinder decision completion, synthesize immediately in the same session without asking for a command or publication destination, and write the specification to a workspace Markdown file: the location the effort's working knowledge names, otherwise `docs/<map-id>-spec.md`. This file is the durable baseline a later fresh execution-planning session reads. Then finish the Wayfinder skill's milestone-roadmap reconciliation before stopping. Implementation requires a separate execution-planning session and the owner's approval.
