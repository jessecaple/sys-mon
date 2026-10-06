---
name: "handoff"
description: "Compact the current conversation into a handoff document for another agent to pick up."
---

Write a handoff document summarising the current conversation so a fresh agent can continue the work. Save it in the current workspace at the requested path, or HANDOFF.md by default.

Include a "suggested skills" section in the document, naming which skills the next agent should call the Skill tool for.

Do not duplicate content already captured in other artifacts (specs, plans, ADRs, issues, commits, diffs). Reference them by path or URL instead.

Redact any sensitive information, such as API keys, passwords, or personally identifiable information.

If the user passed arguments, treat them as a description of what the next session will focus on and tailor the doc accordingly.

# Invocation input

Treat the direct user's invocation text as: What will the next session be used for?

## Save the handoff

A handoff must be durable and visible to whoever picks the work up next. Write it in the current workspace at a user-approved path (default HANDOFF.md or a project-established handoff directory), never an ephemeral /tmp path. Give the user the exact path. Starting the next agent is the operator's call; do it only when asked.
