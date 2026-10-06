---
name: "grilling"
description: "Grill the user relentlessly about a plan, decision, or idea. Use when the user wants to stress-test their thinking, or uses any 'grill' trigger phrases."
---

Interview the user relentlessly until you reach a shared understanding. Map this as a **design tree**: every decision branches into the decisions that hang off it.

Work the tree in **rounds**. The **frontier** is every decision whose prerequisites are already settled: the questions you can ask _now_ without guessing at answers you haven't heard yet. Ask the whole frontier in one round: number each question and give your recommended answer. Then wait for the user's answers before the next round.

Ask each round directly, one question at a time; use the Interview delivery section below for question fields and confirmation.

Each round the user answers reshapes the tree: settled decisions push the frontier outward and unblock questions that depended on them. Recompute the frontier and ask the next round. A question whose answer depends on another question still open in this round belongs to a _later_ round, not this one.

Finding _facts_ is your job, never the user's. When a frontier question needs a fact from the environment (filesystem, tools, etc.), dispatch a sub-agent to find it; don't ask the user for anything you could look up yourself. Don't block on it: a running exploration is an unsettled prerequisite, so only the questions downstream of it wait for the sub-agent to report; ask the rest of the frontier now. The _decisions_ are the user's: put each to them and wait.

The session is done when the frontier is empty: every branch of the design tree visited, nothing left silently assumed. Do not act on it until the user confirms you have reached a shared understanding.

## Interview delivery

Put interview questions to the human directly and wait for their answers.

## Interview rounds

1. Send each round as ONE message: one entry per frontier question, in the round's Q1/Q2 order.
2. Every entry must be self-contained. Put the complete question — including whatever context the human needs to answer it — in the `question` field, with a short `header` as its title. Never split a question so the human must combine the chat message and the dialog to understand what is being asked.
3. Put recommended answers in `options`: put your recommended choice first with "(Recommended)" appended to its label, and give each option a one-line tradeoff description. The human can always type a custom answer, so options never box them in.
4. The assistant message before the call may carry analysis, exploration results, or findings that motivate the round — as supplements the human may skip, never as the missing half of a question.

## Final confirmation

Confirming a document you just presented is the one place a short question may point at chat content:

1. First present the shared understanding in the ordinary assistant response, organized under short headings with at most 12 concise bullets. Group larger summaries into product, architecture, behavior, operations, and deferred work rather than one giant paragraph.
2. Then ask a short confirming question such as **Does this capture the agreed v1 boundary?**, offering **Yes, confirm** and **Needs corrections** while leaving room for a free-text answer.
3. If corrections are needed, ask only about the disputed section through the tool; do not repeat the whole summary in another question.
