# Logic prototype contract

Produce one double-clickable HTML file unless the existing workspace has a clearly simpler runnable harness. Keep the decision logic isolated in pure functions or a small reducer so scenarios are deterministic and easy to inspect.

Use the project's domain vocabulary in state and action names. Show current state and the event log after every action. Provide:

1. **Free play** controls for every meaningful transition.
2. **Guided happy path** proving the normal sequence.
3. **Edge scenarios** for boundaries, repetition, empty state, and recovery.
4. **Illegal scenarios** that attempt invalid transitions and visibly demonstrate rejection or error handling.

Include a reset control and deterministic initial state. Avoid persistence unless persistence itself is the question. Hand the artifact to the human with the exact scenarios to try, then revise from their feedback. The prototype's state model and verdict are evidence; the HTML is not production code.
