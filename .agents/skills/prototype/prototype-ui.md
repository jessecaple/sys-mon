# UI prototype contract

Prototype inside the real page, route, and data context when practical; use a standalone artifact only when integration would obscure the decision. Default to **three structurally different variants**, not cosmetic variations. Each variant must make a distinct product or interaction tradeoff.

Provide a visible switcher that:

- has a URL-stable selection when routing is available;
- is keyboard accessible with clear focus and labels;
- preserves representative data and surrounding context;
- exposes the states needed to judge loading, empty, error, normal, and dense content where relevant;
- is clearly marked as prototype-only and cannot leak into production unnoticed.

Explain the decision each variant tests and the tradeoff it embodies. Give the human a short evaluation script, collect reactions to each variant, and iterate until one direction or a new hybrid answers the question. Record the selected direction and rejected alternatives; do not treat implementation polish as evidence.
