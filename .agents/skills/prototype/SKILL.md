---
name: "prototype"
description: "Build a throwaway prototype to answer a design question. Use when the user wants to sanity-check whether a state model or logic feels right, or explore what a UI should look like."
---

# Prototyping

Build a disposable artifact to answer one explicit design question. A prototype is evidence, not production delivery.

## Route before building

Classify the question:

- **Logic or state-model question:** load and follow [prototype-logic.md](./prototype-logic.md).
- **Visual, interaction, or layout question:** load and follow [prototype-ui.md](./prototype-ui.md).
- If both are entangled, isolate the uncertain logic first, then expose it through the smallest UI that lets a human evaluate it.

Restate the question and observable success signal. Confirm the target before writing outside the current workspace. Mark files as prototype material, avoid unrelated refactors, and make the artifact trivial to run.

## Human evaluation loop

Present the runnable artifact to the human, explain what to try, and ask for concrete reactions. Iterate until the question is answered; do not substitute the agent's own taste for human evaluation. Report the exact local path, run command, observed evidence, human reaction, and verdict.

Keep the prototype as disposable local primary evidence. Do not fold it into production code, initialize Git, create a branch or commit, publish remotely, or create external records unless separately requested.

## When loaded from Wayfinder

The prototype ticket remains human-in-the-loop. Wait for the human's reaction before resolving it. Record their reaction, artifact path, and resulting decision as resolution evidence. Later production implementation uses the decision, not prototype code blindly.
