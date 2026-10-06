---
name: "setup-pre-commit"
description: "Set up Husky pre-commit hooks with lint-staged (Prettier), type checking, and tests in the current repo. Use when user wants to add pre-commit hooks, set up Husky, configure lint-staged, or add commit-time formatting/typechecking/testing."
---

# Pre-commit setup

Configure requested commit-time checks in an existing Git worktree while preserving the project's package manager and conventions.

## Preflight

1. Verify this is a Git worktree; otherwise explain that commit hooks are not applicable and stop without initializing Git.
2. Detect the package manager, Node project shape, existing hook manager, formatter, typecheck/test scripts, staged state, and custom hooks. Never replace a working hook system silently.
3. Prefer the existing manager. When none exists and the request names Husky, install the current compatible Husky plus lint-staged and Prettier as development dependencies using the detected package manager.

## Husky default recipe

1. Ensure package.json has a `prepare` script that initializes Husky without overwriting unrelated prepare work; compose or ask when an existing script is incompatible.
2. Initialize `.husky/` using the installed version's documented command and create an executable `.husky/pre-commit`.
3. Order checks from fast to expensive: staged formatting first, then typecheck, then focused or requested tests. Invoke existing package scripts rather than duplicating commands.
4. Configure lint-staged for the repository's actual file types. Use Prettier only where supported, preserve existing lint-staged configuration, and create a minimal Prettier config only when none exists and formatting was requested.
5. If a requested typecheck or test script is missing, report it rather than inventing an incorrect command.

## Verify

- Confirm the hook and configuration files exist and the hook is executable.
- Confirm the prepare script and dependency versions are coherent.
- Run lint-staged on safe fixture or currently staged files without altering the user's staged set.
- Invoke the hook through Git in a reversible test when practical, restoring the exact staged state afterward; otherwise run the hook body directly and explain the limitation.
- Run package-manager install consistency and relevant checks.

Do not create a commit, stage files, push, or alter history unless separately requested. Passing concrete hook verification is completion.
