---
name: "wizard"
description: "Generate a provider-neutral interactive wizard only for steps a human must perform."
---

# Wizard

Generate a provider-neutral interactive Bash wizard only for steps a human must perform. Complete every agent-capable step yourself first.

## Discover and confirm

1. Inspect existing configuration, environment examples, scripts, and current state before asking questions.
2. Identify the provider only from the direct request or existing configuration; never assume hosting, CI, secret storage, repository layout, or account type.
3. For an unfamiliar dashboard or API, research current primary documentation before writing click instructions. Record the documentation URL and do not invent labels or navigation.
4. Present an ordered inventory with one row per value/action: stage, human action, value captured, destination, secret/non-secret, verification, and whether it is reversible. Confirm this inventory with the human before generating the script.
5. Minimize the manual boundary. Credentials, approvals, captchas, and irreversible dashboard actions belong in the wizard; file edits and verifiable local setup usually do not.

## Generate

Start from [wizard-template.sh](./wizard-template.sh) and keep only needed helpers. Set `TOTAL_STAGES` correctly. Every stage must be restartable and safe to rerun.

- Explain each action immediately before prompting.
- Print every URL and open it only as a convenience.
- Read secrets without echo; never print, log, interpolate into command lines, or persist them anywhere except the confirmed destination.
- Validate formats and reject placeholders.
- Confirm immediately before each irreversible or externally mutating action, even when the overall inventory was approved.
- Detect existing destinations and confirm before overwriting.
- Verify outcomes automatically where possible; otherwise require a simple human confirmation.
- Never require Git, a remote, or an external tracker.

## Validate

Run `bash -n`, run `shellcheck` when available, verify the executable bit, inspect every captured value's data flow from prompt to destination, and dry-run non-mutating paths. Ensure cancellation and rerun behavior preserve prior state. Return the script path, prerequisites, manual boundaries, and resume instructions.

Do not commit, upload, publish, or mutate an external service beyond the actions explicitly confirmed in the wizard.
