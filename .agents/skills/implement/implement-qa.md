# Hands-on QA

The implementation session owns QA in its ticket worktree. Combine visual inspection and person-operable journey checks in one pass. Use the ticket's requirements, verified inventory, UX design and repository contracts to choose cases. A local UI change needs its affected states; a new screen or meaningful journey needs its main task, plausible failures and recovery. Include permissions, persistence, cancellation, keyboard use and responsive behavior where the change can affect them.

## Candidate and runtime

Use the repository launcher's documented freshness checks when available; require ready owned processes and qualified runtime inputs for the candidate. Keep the full candidate identity in the evidence even when the launcher tracks runtime inputs separately. Until it supports that distinction, satisfy its existing source check. Identify the tested source, worktree, origin, process, API source/health and cleanup owner once. Recheck after a runtime change or failure. Qualify backend behavior as well as frontend source. Use the supported test-session setup and designated local or test data. Required states need real controls or an established production-component harness. Record an unavailable runtime as an infrastructure blocker and an unreachable required state as a coverage hold.

Perform QA before independent review where practical. Carry passing evidence forward under [Evidence and resumption](SKILL.md#evidence-and-resumption); final acceptance reconciles the reviewed commit with that evidence. A runtime restart alone does not require repeating otherwise applicable journeys.

## One focused browser pass

Use the repository's browser-check helper, or one Playwright script per journey/set of states. Interact through visible, enabled controls by role/label and ordinary pointer/keyboard actions. Wait for observable state or focus settlement. Use actual user-visible results and contract-defined persistence outcomes as assertions.

Choose meaningful checkpoints such as loaded, editing, validation, saved/reopened and recovery. At these states inspect clipping, overlap, overflow, focus, control reachability and the applicable visual rules. Use the helper's page checks and screenshots together; inspect contact sheets and enlarge flagged or unclear captures. Qualify intentional layout reports through the repository browser tool's documented procedure, with the governing design rule and a person-operable witness. Retain the raw verdict and unresolved problems. Measure changed geometry or suspected violations with computed styles; confirm viewport and saved-image dimensions for responsive evidence.

Cover affected visual states at the required widths and themes. Exercise theme-independent business behavior once on qualified inputs; repeat across layouts, roles or themes where they change the behavior or expose a specific risk. Existing focused component fixtures can establish rare visual states. Record their applicability. Judge composition, legibility and interaction as well as automated assertions.

## Native and desktop clients

Verify changed native or Electron journeys in the actual application on the
affected target platform. Record the application build, operating system and
runtime with the candidate. Exercise affected host interactions such as launch,
clipboard, file dialogs, drag/drop, keyboard shortcuts and local persistence
through the controls a person uses. Check saved preferences after reopening and
relevant context changes when their behavior changes. Retain the observed result
and focused evidence. Record an unavailable target runtime as a coverage hold.

## Findings and evidence

Fix defects found while developing the ticket and retain relevant failure/correction evidence. Record unresolved defects and coverage holds on the ticket with candidate, state, expected/actual behavior, reproduction, impact and requirement or violated design rule. Use `wayfinder finding add --source qa` for behavior or `--source design` for a visual violation; check for an existing finding first. Close your own findings after verification. Independent reviewers close theirs.

Keep one durable ticket evidence record containing candidate, exact commands/verdicts, selected journeys and states, essential readbacks/measurements, representative captures and remaining gaps. Keep scripts and disposable captures outside candidate worktrees; preserve essential evidence outside temporary storage and exclude credentials. Reference this record from the checkpoint and final resolution. A pass covers its named scope; qualify all required acceptance evidence before landing.

For milestone closeout, reuse qualified ticket evidence and exercise missing combined journeys or invalidated assumptions on integration. File gaps against their owning scope. Clean up your unused runtimes and credentials after acceptance, preserving resources still in use.
