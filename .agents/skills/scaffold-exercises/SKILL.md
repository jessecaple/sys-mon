---
name: "scaffold-exercises"
description: "Create exercise directory structures with sections, problems, solutions, and explainers that pass linting. Use when user wants to scaffold exercises, create exercise stubs, or set up a new course section."
---

# Exercise scaffolding

Create exercise structures that are useful even when nearby examples are sparse and that pass the project's structural checks.

## Discover conventions

Inspect repository guidance and adjacent sections first. Preserve established naming when present. Otherwise use deterministic numeric prefixes for sections and exercises and lowercase kebab-case names.

## Minimum portable contract

Each exercise contains at least one variant directory: `problem/`, `solution/`, or `explainer/`. When the request does not name a variant, default to `explainer/`.

Every variant contains a non-empty `readme.md` that explains its purpose and links only to files that exist. Add `main.ts` or the repository's established entry point when the exercise requires runnable code. Problem and solution variants should mirror required entry points so comparisons are meaningful.

Section and exercise indexes must link to their children when the surrounding course convention uses indexes. Never create empty placeholder directories that structural lint rejects.

## Process

1. Translate the requested plan into ordered section/exercise/variant paths and show ambiguities before writing.
2. Create the smallest complete stubs satisfying both nearby conventions and the portable contract.
3. Use ordinary filesystem moves when renumbering so staging is untouched. Use `git mv` only when the direct user explicitly requests that Git action.
4. Validate numeric ordering, duplicate prefixes, required variants, non-empty readmes, link targets, and entry-point symmetry.
5. Run the repository's exercise lint, formatting, and type checks; report any unavailable check.

A plain directory is valid. Never initialize Git, stage, commit, push, or publish unless separately requested.
