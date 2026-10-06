# Wayfinder map rules

The rules for writing to a Wayfinder map in any phase. The `wayfinder`, `decide` and `implement` skills each install this file beside their own.

## Commands

Every command prints JSON on stdout — exit 0 success, exit 1 domain failure (`{ok:false,code,message,detail}`), exit 2 usage error — Reads (`open`, `milestone show`, `subtasks`, `findings`) show map state. A write returns only what changed, the new revision and what you could not otherwise know: created ids, tickets it unblocked, a newly active milestone. Read `wayfinder open` when you need the frontier; a refusal names the fix. Write verbs read one JSON object: write the payload to a temp file and pass it with `--input` (`wayfinder resolve wf-x/t-y --input <temp file>`), because real resolutions are multi-kilobyte Markdown and hand-quoting them into argv corrupts the durable record. `--input` works in every shell. Piping the file on stdin also works, except in Windows PowerShell 5.1, which re-encodes piped text as ASCII and turns non-ASCII characters into `?`. `--expect <revision>` is the compare-and-swap; knowledge writes and `activate` require it, and ticket-scoped commands accept either the map revision or that ticket's revision.

## Phases

A unified effort moves one way through `decision → decision_ready → execution → complete`. No phase returns to an earlier one.

- **decision:** decision tickets own the frontier and fog; the `decide` skill works them one per session. Ask the owner directly in the interactive session; the owner-question inbox is for execution only.
- **decision_ready:** decision work is settled and the session that completed it has synthesized a specification. Implementation activation belongs to a different, fresh session.
- **execution:** the `implement` skill drives the execution tickets. Execution holds only task tickets. A question for the human is an owner question (see [Owner questions](#owner-questions)); a HITL task is work the human must do.
- **complete:** execution is finished and the effort is terminal. Chart a new effort for later work.

## Decision ticket types

Choose by the question being unblocked:

- **Grilling is the default:** HITL decision conversation; load `grilling` and `domain-modeling`.
- **Research is AFK only:** facts outside the workspace; a Research Analyst subagent claims, researches, reports, and resolves it.
- **Prototype:** HITL disposable evidence plus a human reaction.
- **Task:** prerequisite-only work before a decision; explicitly choose `afk` or `hitl`.

A ticket's brief names companion skills, constraints, or a precise prerequisite checklist. A decision task never delivers the destination itself. `type`, `interaction`, and `artifacts` are descriptive metadata that makes a ticket self-explanatory to whoever picks it up; none of them is checked when you resolve.

## Milestones

The map owns the effort's milestone master list: ordered delivery stages, each with a short `key` (such as `M5`), a `name`, a `description` of its contents, and a status: `planned`, `active`, `paused` or `completed`. It is the single roadmap record — keep milestone definitions in the map, not in specification tables or other documents, and point documents at `wayfinder milestones`. Milestone keys are `M` plus a number with at most one decimal part (`M4`, `M8.5`) and increase in list order, so a new milestone slots between neighbours without renumbering; `M0` is reserved for the standing milestone.

- **Descriptions are big-picture but comprehensive.** Name every capability, surface, integration and acceptance obligation the milestone delivers — screens, capabilities and gates, not buttons or files — so its completion can be audited from the description alone. **They hold scope, not status:** change one only when scope changes, with the owner's approval; progress and requirements belong on tickets.
- **Every ticket draft** — in `chart`, `add`, `activate`, a resolve's `newTickets`, a `supersede` replacement or a `bring-in-scope` ticket — carries a `milestone`, an `eli5` and a `subtasks` checklist of at least one item; the CLI refuses a draft missing any of them.
- **Milestone:** `milestone:"<key>"` names a milestone that is not completed. A `supersede` replacement inherits the replaced ticket's milestone unless it names another.
- **ELI5:** `eli5:"…"` is shown only to the owner in the viewer and never returned to agents. Explain it like I'm 5 (ELI5), for a non-technical owner who has not read the ticket:
  1. **Lead with why it matters:** what changes for the people who use the product, or what would go wrong without it.
  2. **Then what gets done, in everyday words.** Avoid technical terms (mapping, migration, endpoint, engine, schema, health check, records, refactor and the like); describe their effect instead.
  3. **Name bundled pieces plainly.** If the ticket combines unrelated parts, say so and name each, rather than blurring them into one vague sentence.
  4. Two to four short sentences, at most 500 characters; no file paths, ticket ids, code names or legacy screen codes.

  Weak: "We make sure every old permission is mapped to its new version, with tests that catch any gaps." Better: "Customers moving from the old system must keep exactly the same access — nobody should gain or lose the ability to see or change things. We check every old permission against the new ones and add tests so a gap can't slip through."

- **Milestone scope changes need the owner.** Adding, editing, shifting back and reassigning tickets (`wayfinder milestone add|edit|activate|assign`) need the owner's explicit yes first; run them with `--basis owner --reason "…"` quoting or summarizing their words (see Changing scope). Completing a milestone after a clean independent review needs no owner approval.
- **One active milestone.** During execution exactly one delivery milestone is `active`: the earliest not yet completed. The CLI activates it at `activate` and when the previous one completes. Execution tickets belong only in M0 or the active milestone; choose M0 first for new work claims while existing active-milestone claims continue. Record genuine M0 prerequisites as dependencies. Planned milestones hold scope in their descriptions, not execution tickets. Record a deferred obligation there with owner approval if it changes scope; plan its work tickets when that milestone becomes active. Pulling later work forward is an owner decision.
- **Plan before delivering; never skip.** When the active milestone has no open execution tickets, its claimable slot grants one session the planning or closeout decision. `claim --next` takes ready M0 work and milestone tickets before the slot. The slot owner compares the full description with delivered evidence, then files one planning or closeout ticket; `add` transfers the claim to that ticket atomically. With no resolved ordinary delivery work, plan the description into work outcomes and checklists; cancelled, ruled-out and superseded history does not count. After delivery leaves no ordinary work open, uncovered obligations need another planning ticket and more work; only apparent full coverage proceeds to independent closeout. A resolved planning ticket and an empty queue are not delivery evidence. `milestone complete` refuses a milestone with no resolved work tickets.
- **Completion needs an independent review.** When ordinary milestone work is terminal and coverage appears complete, the slot owner creates one `executionRole: closeout` ticket for combined QA and independent completion review. A requirements-reviewer subagent that neither implemented the work nor authored its inventories checks every milestone criterion against source and evidence. Keep the report in durable verification storage; the ticket owner may commit it on the closeout branch. Gaps become work tickets and closeout dependencies. Once clear, land/resolve closeout; its claim returns to the slot until the same session runs `wayfinder milestone complete --milestone <key> --expect <current map revision> --reason "Independent completion review cleared"` with `{"summary":"…","artifacts":[{"kind":"file","label":"completion review","value":"<durable report path>"}]}`. The CLI refuses non-active milestones, open tickets or a missing review. Completion activates the next milestone; continue without another owner approval.
- **Shifting back is the owner's call.** To return to an earlier completed milestone (say, a regression in what it delivered that must be fixed before moving on), get the human's explicit approval, then run `wayfinder milestone activate --milestone <key> --basis owner --reason "…"` with their words and why the project is shifting back. That milestone becomes active and the current one is `paused`; completing the earlier one again (with a fresh review) resumes the paused one.
- **Decision to execution requires milestones.** Before decision work hands off to execution, the map's milestone list must cover the settled roadmap. `activate` refuses a map with no planned delivery milestone, and every activation draft names one.
- **Charting** includes the milestone list in the `chart` payload (`milestones:[{key,name,description}]`); those initial milestones are approved as part of the charting confirmation. `chart` adds the standing milestone `M0` first on its own.
- **Standing milestone (M0).** M0 holds priority work no roadmap milestone owns: cross-cutting tech debt, tooling and upgrades, test infrastructure, and owner requests that fit no milestone. Work that concerns a milestone goes there; a small fix to a completed milestone's delivery goes in M0, and substantial rework of one is a shift back (`milestone activate`, owner approval). M0 never completes, so keep it small and extend an existing M0 ticket (`edit`, `subtask add`) before opening another. Every M0 ticket enters with a basis and reason in every phase: `owner` when the owner actually asked (their words in the reason), `discovery` when you found it (say why). Ready M0 tickets lead the frontier: claim them first, finish that one ticket, then claim the next available ticket in M0 or the active milestone. Actual prerequisites are recorded in `blockedBy`.

`wayfinder milestones` lists the master list with status, open/resolved execution ticket counts and progress. `wayfinder milestone show --milestone <key>` gives one milestone in full: description, review, tickets and history. Ticket briefs carry `milestone`; the execution projection lists every milestone by key, name and status, with the description and open tickets of the active one.

## Subtasks

Subtasks are the required way to track work inside a ticket. Each is a short checklist line (at most 120 characters) owned by one ticket; the ticket's done/total count is how the owner and implementation session see its progress and notice scope creep.

- **Plan them up front.** Every ticket draft needs a `subtasks:[...]` array of at least one item, and the CLI refuses a draft without one. Enumerate everything you can already see: each behavior, screen section, migration, integration, test obligation and verification step; a complete list is what makes percent-complete meaningful.
- **Add as you discover.** New required work found mid-ticket goes on as another subtask (`wayfinder subtask add <map>/<ticket> --text "…"`, or `{subtasks:[…]}` on stdin). Adding is always allowed on an open ticket and needs no claim, but every late addition is visible scope growth in the viewer's Changes tab — prefer finding it during planning. A subtask may only refine delivery of approved scope; a new deliverable is an `owner` change (see Changing scope).
- **Check off as you go.** `wayfinder subtask done <ticket> s-xxxxxx…` when an item is actually delivered and verified; `undo` if it turns out not to be. `delete` an item that is no longer needed; it disappears from the checklist but stays in the map's history.
- **Resolve only a finished checklist.** `resolve` refuses while any live subtask is unchecked.

`wayfinder subtasks <map>/<ticket>` lists the live checklist in creation order with `{done,total}`; `wayfinder open <map>/<ticket>` includes the same list. Ticket briefs in projections carry the `subtasks:{done,total}` count.

## Owner questions

The owner-question inbox is for autonomous execution only. `question ask` and `question close` require execution phase. During `decision` and `decision_ready`, ask the owner directly in the interactive session and record the resulting decision or approval where it belongs. Existing questions remain readable in every phase.

During execution the owner is away and nobody watches session transcripts, so the ticket owner reaches the owner only through the map's owner questions, which the owner sees in the viewer header. `wayfinder question ask --text "Should …?" [t-xxxxxx…]` (or `{question,context?,ticketIds?}` on stdin) records one; each implementation session asks for its owned work, and the text must ask something: status, progress and completion notices are refused. When the owner answers in a session, it runs `wayfinder question close q-xxxxxx --text "<their answer>"`, which removes the question from the outstanding list and keeps the answer as the record; the decision itself still goes where it belongs (ticket, scope, milestone). `wayfinder questions` lists what is open (`--status closed|all` for the rest), and `wayfinder open` carries the open questions.

## Worktree record

A ticket's implementation session records where it works with `wayfinder worktree <map>/<ticket> --path <worktree> --branch <branch> --base <starting SHA>` on an open ticket; recording again replaces it. `wayfinder open <map>/<ticket>`, ticket briefs and the viewer show it, so reviewers and QA work from the same location.

## Changing scope

The approved destination, milestones and out-of-scope list define scope. Every change to it carries a basis and a reason:

- **`discovery`** — work required to deliver what is already approved. Agents may add tickets and subtasks on this basis on their own.
- **`owner`** — anything that adds or removes deliverables, moves the boundary (`rule-out`, `bring-in-scope`, `scope add|edit|remove`), edits the destination or milestones, or reverses a settled decision. It needs the human's explicit yes; record their words in the reason. Never infer it.

Where the basis goes: `add` takes `{tickets,basis,reason}` (required during execution, recorded as each ticket's origin); a resolve's `newTickets` during execution needs `newTicketsBasis` and `newTicketsReason`; `rule-out`, every `milestone` write and `scope add|edit|remove` take `--basis`; `bring-in-scope` and a destination change through `update` take `basis` and `reason` in the payload. Out-of-scope records not tied to a ticket are managed directly, with `--basis` and `--reason` saying why the boundary moved (`owner`: their words): `wayfinder scope add` with `{items:[{text,reason}]}` records excluded work on the working plane, `scope edit --id o-xxxxxx` with `{text?,reason?}` rewrites a record in place, and `scope remove --id o-xxxxxx` deletes one. A record created by `rule-out` is linked to its ticket and returns only through `bring-in-scope`; a record already brought back is history. Discovery additions need no extra reporting — they appear in the viewer's Changes tab and its scope-change count.

## Major changes during execution

When the destination, architecture, dependency graph, or already-integrated work needs substantial reconsideration, hold affected work and bring the change to the human with a concrete reason (during execution, as an owner question). With their direct approval, reshape the map in place: `update` the destination, edit milestones (`wayfinder milestone add|edit|assign`), `bring-in-scope` or `rule-out` work, adjust out-of-scope records (`scope add|edit|remove`), `supersede` settled tickets whose answers no longer hold, `cancel` open work that no longer serves the destination, `edit` open tickets whose scope changed, and `add` the new task tickets. Each scope change carries basis `owner` with the approval in its reason.

## Supersede, amend, and scope

**Supersede** a resolved, cancelled, or ruled-out ticket whose answer no longer holds. Supply `basis` (`discovery` or `preference`), the trigger, and a replacement draft. The transaction preserves history, opens a replacement on the same plane, inherits blockers, rewires dependents, and releases dependent claims whose prerequisite reopened. Preference basis needs a fresh owner decision.

**Amend** a resolved ticket only when its answer, dependencies, destination boundary, and downstream decisions remain valid. Amendments are ordered and append-only. Artifact items add evidence, or identify an exact old `{kind,label,value}` through `replaces` to repair the effective artifact projection. Missing, ambiguous, duplicate, unchanged, colliding, and over-limit corrections fail rather than guess.

**Restate cleanly** when the owner wants a settled decision to read as if it had been decided that way — no amendment or supersession trail on the ticket. Use `wayfinder edit <map>/<ticket> --basis owner --reason "…"` with their words and `{title?,question?,brief?,resolution?,decisionSummary?}` on stdin instead of `amend` or `supersede`. The rewrite replaces the resolution and decision summary in place (same 65,536 / 500 limits); artifacts stay as they are, so an execution ticket keeps its one handoff artifact. The prior text goes only to the ticket's edit history and the Changes tab. Use it only on the owner's explicit request; correct your own mistakes with `amend` or `supersede`.

**Bring in scope** with `wayfinder bring-in-scope` and `{scopeId,basis:"owner",reason,ticket?,destination?}`, only after the human moves the boundary. It works in decision and execution phases; during execution the ticket is a task. The old scope record remains history; a replacement ticket or destination redraw enters the working plane atomically.

## Execution ticket sizing

A complete screen is usually one execution ticket. For work without a screen, use a comparably substantial, coherent workflow or system capability. Include the controls, domain, persistence, API, permissions, validation, consumer integration and meaningful tests needed to deliver it.

Create a separate prerequisite ticket for a substantial shared subsystem that warrants its own implementation and verification, with named real or approved planned consumers. Examples are the shared effective-dating system built before the screens that use it, or the Daily Production grid with its coordinated editing and calculation behavior. Keep each subsystem's supporting mechanics together; being shared, reusable or independently testable alone does not justify a ticket.

Ordinary buttons, fields, tables, dialogs, screen sections, helpers and technical layers stay within the screen or subsystem ticket. “Deliver the Route screen” includes its member-selection grid, search, Add/Remove actions, saving, permissions and tests. A complex shared grid can be a subsystem ticket; an ordinary grid is part of its screen.

Within a ticket, the subtask checklist — not more tickets — enumerates the sections, controls, layers and tests. Before splitting, name the complete screen, comparably substantial capability or supporting subsystem each ticket will deliver and explain why separate delivery is warranted. Keep work with an existing ticket that owns it. Use subtasks and substantial review chunks to manage implementation size; file counts, review size and agent boundaries do not determine ticket boundaries. Preserve section-level inventories, ownership and verification within the owning ticket.

## Execution dependencies

Each behavior ticket owns its meaningful tests. Add a separate ordinary test ticket only when combined behavior cannot be tested in any one prerequisite ticket; block it on those prerequisites and scope it to that test. `blockedBy` records actual prerequisites and deliberate owner approval gates, never chronology. The owner tests deliveries on their own schedule, outside the ticket graph. Plan shared mechanics before their named consumers, within the owning ticket or a separate subsystem that meets the ticket-sizing rule. Capture structural as well as functional acceptance before dispatch. Use `implement` for the execution queue and proportional review.

To correct obsolete dependencies on an open, unclaimed execution ticket, pipe `{blockedBy:[ticketIds],reason}` to `wayfinder dependencies <map>/<ticket> --expect <revision>`; it records old and new edges with the reason and protects graph validity and live claims. A ticket owner can add `--release` to record blockers and release its own claim atomically; invalid dependencies or another affected live claim leave the original claim intact. Use that operation when handing off blocked work, rather than releasing before setting blockers.

## What goes where

Every piece of information has one owning home. Check the rows in order and use the first that fits; working knowledge is last.

<!-- prettier-ignore -->
| Information | Home |
| --- | --- |
| Work inside a ticket: its progress and checkable items | that ticket's subtasks — check off as delivered, add items discovered mid-ticket |
| Requirements, acceptance, constraints and links to documents for one ticket | that ticket's brief (`wayfinder edit` with `{brief}`; replaced, at most 4,000 characters) |
| A ticket's technical or UX design | its design (`wayfinder design --kind technical\|ux`; replaced, at most 4,000 characters), or a link to the committed document |
| Review, design review, QA and verification findings | the ticket's findings (`wayfinder finding add --source review\|design\|qa\|verification`), closed by whoever raised them once the fix is confirmed |
| Where an open ticket stands and how to resume it | its checkpoint (`wayfinder checkpoint`; overwritten, at most 1,000 characters), linking the durable handoff for reviewer identities and detailed evidence |
| Unanswered, not-yet-ticketable question | fog |
| Known work for a planned milestone | that milestone's description, with source references; create execution tickets after it becomes active |
| What a ticket delivers and why it matters, for the owner | that ticket's `eli5`, written by the ELI5 rules under Milestones |
| Work consciously excluded from the destination | an out-of-scope record (see Changing scope) |
| Priority work that belongs to no open roadmap milestone, including small corrections to completed deliveries | a ticket in the standing milestone M0; work within an open milestone stays there, and substantial rework of a completed milestone requires the approved shift back (see Milestones) |
| A milestone's overall scope | that milestone's description (scope only; see Milestones) |
| Milestone completion evidence | the milestone's independent completion report in durable verification storage or `docs/verification/`, attached on `milestone complete` |
| Delivered outcome and evidence: commit or handoff, verification commands and verdicts, what-to-test notes | the ticket's resolution (corrections via `amend`; see Restate cleanly) |
| Where delivery stands and remaining gaps | the map itself — active milestone, frontier, open tickets and their subtasks; read it with `wayfinder open` and `wayfinder milestone show`, never a separate summary |
| Where a ticket's implementation session works | its worktree record (`wayfinder worktree`) |
| Reviewer identities, candidates and continuation/evidence pointers | durable ticket handoff referenced by the checkpoint; reuse the same reviewer for corrections, record replacements when unavailable |
| Engineering or repository rule, including a workflow rule | AGENTS.md, or an edit to the owning skill |
| Product behavior, contracts, owner rulings | the repository's contract or decision documents |
| What the effort is | the map's name and destination |
| Why scope moved | the basis and reason on that change: ticket origin, scope record, scope history (`scope add\|edit\|remove`), milestone history or destination history |
| Short current fact needed across tickets that no home above owns, or a workflow lesson worth keeping | working knowledge, if it passes the [admission test](working-knowledge.md) |

Each ticket field above has one purpose and is replaced when it changes. When information moves to a better home, remove it from the old one. Maintain working-knowledge entries, and move misclassified fog, per [Working knowledge](working-knowledge.md).

## Editing tickets

Change an open ticket's title, question, brief or eli5 with `wayfinder edit <map>/<ticket> --reason "…"` and `{title?,question?,brief?,eli5?}` on stdin. Set a design with `wayfinder design <map>/<ticket> --kind technical|ux`, findings with `wayfinder finding add|close` and the resume point with `wayfinder checkpoint`. The edit history records when, who and why, and any title or question replaced, but never copies the other text; `wayfinder open <map>/<ticket> --history` shows it. A ticket written before these fields keeps its old notes read-only (`wayfinder open <map>/<ticket> --legacy-notes`); rebuild its brief and checkpoint from them the first time you work it. Settled tickets change only through `amend`, `supersede` or an owner restatement (see Supersede, amend, and scope). Put requirements on the ticket that owns them.

## Durable semantics

Text that is kept — subtasks, briefs, designs, findings, checkpoints, resolutions and owner questions — keeps every word that changes meaning when condensed (not, never, only, except) and every number and unit exact, spells words out rather than inventing abbreviations, and uses one term for each concept throughout.

Write the least the next reader needs to act: the current state and what to do next, with links to documents that hold the detail. Evidence goes in the resolution or the commit, and the edit history keeps who changed what. When text runs long, cut content and keep every word whole.

The map is an index. In every phase `wayfinder open` lists the ID, title and queue position of open tickets without embedding full tickets; during execution it covers M0 and the active milestone (other milestones: `wayfinder milestone show`). Closed history and other context samples have fixed limits, with counts for omitted items. Open the exact ticket for full evidence or export the canonical map. Full answers live on tickets; summaries and effective artifacts are projections. Superseded tickets and settled decision tickets remain readable but never re-enter the frontier. A claimed ticket keeps `status: open`: the claim is the ticket's `claim` object (`sessionId`, `claimedAt`), its observable effect is leaving the frontier, and re-claiming from the same session is an idempotent no-op. Claims and phase transitions are atomic and revision-guarded, so refresh stale assumptions before writing.

Wayfinder owns persistence. Maps are keyed to the git repository root and fall back to the working directory outside a repository. Do not require external trackers or side databases for decision work.

## Independent execution sessions

Implementation sessions use `session start --isolated` and pass the returned `--session` on every map command and reviewer assignment; the legacy ambient session is not a concurrent-chat identity. `claim <map> --next` atomically resumes the caller's claim or selects one ready AFK ticket, M0 first; `claim: null` means none available. A session finishes or checkpoints/releases its execution ticket before claiming another. No preparation runs before claiming.

All participating Git sessions use `land <ticket> --candidate <full SHA> --base <qualified integration SHA>` with the resolution payload to fast-forward and resolve under workspace serialization. The command supplies commit evidence and refuses open findings, unfinished subtasks, changed candidates and stale integration. Review and QA happen before landing and refresh when their assumptions change. Direct Git writes to integration bypass this protocol.
