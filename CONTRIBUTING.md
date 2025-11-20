# Contributing to exclusive-display-mirror

Thanks for your interest in contributing! This document explains how to get started, the workflow we expect contributors to follow, and our policy for AI-assisted contributions.

Please read the whole document before opening a pull request.

Table of contents
- Getting started
- Development environment
- Branching & commits
- Pull request process
- AI-assisted contributions (important)
- Tests & quality checks
- Labels and metadata
- Review expectations
- Getting help


1. Getting started

- Prefer opening an Issue first for non-trivial work or design discussions.
- For small fixes or docs improvements, you may open a PR directly.

Clone the repository and set up dependencies:

```bash
# replace OWNER/REPO with your repository path
git clone https://github.com/OWNER/REPO.git
cd REPO

# install frontend deps
yarn install

# ensure Rust toolchain is installed
rustup install stable
rustup default stable
```


2. Development environment

Recommended commands (adjust if you use npm/pnpm):

```bash
# Frontend dev server (Svelte + Vite)
yarn dev

# Build frontend assets
yarn build

# Tauri dev (from project root)
yarn tauri dev
# or
cargo tauri dev

# Run Rust tests
cargo test --workspace
```

Notes:
- If you need platform-specific tools (Visual Studio Build Tools on Windows, etc.), install them before attempting native builds.
- Running `yarn tauri dev` or `cargo tauri dev` may require elevated privileges on Windows depending on local native components.


3. Branching & commits

- Use descriptive branch names, e.g. `feat/preview-mode`, `fix/display-edid`, `chore/deps`.
- Follow Conventional Commits where possible: `feat: ...`, `fix: ...`, `docs: ...`.
- Keep changes small and focused. Large refactors should be discussed in an Issue first.


4. Pull Request process

When opening a PR, include:
- A clear title and short description of what the change does and why.
- Reference to any related issue (`Fixes #123`).
- A short summary of testing performed locally.
- Disclosure if AI was used to author or assist the changes (see the AI section below).

PR checklist (author must complete before requesting review):
- [ ] Code builds without errors locally.
- [ ] Relevant tests pass locally (see Tests section).
- [ ] Lint/formatters run (prettier / eslint / rustfmt / clippy as applicable).
- [ ] Documentation updated where appropriate (README, docs, etc.).
- [ ] No secrets or credentials accidentally committed.
- [ ] Added labels: `type:*` and, if applicable, `ai-assisted`.
- [ ] Declared if AI assistance was used (Yes/No). If Yes, included tool name and prompt excerpt(s).


5. AI-assisted contributions (critical)

We allow AI-assisted contributions, but only under strict rules to ensure safety, correctness, and maintainability.

Requirements for any PR that includes AI-generated code, text, or configuration:
- Disclosure: The PR description must include an "AI-assisted" field with the tool name, model/version, and a short description of where it was used.
- Provenance: Provide relevant prompt excerpts and indicate which files or hunks were generated or edited by AI. Prefer to include this as a short file `AI_PROMPTS.md` in the PR if many prompts are used.
- Human review & testing: At least one human reviewer (not the PR author) must run the project's test/build commands locally, verify the behavior, and leave an explicit approval comment listing the commands they executed.
- Labeling: Add the `ai-assisted` label to the PR. Maintainers will also add `needs-human-approval` where deeper review is required.
- No auto-merge: PRs flagged `ai-assisted` must not be merged automatically by bots; a human approval is required.

Failure to follow these rules will result in the PR being blocked until compliance is demonstrated.


6. Tests & quality checks

Minimal checks to run before requesting review:

```bash
# frontend build (verifies build-time errors)
yarn build

# run Rust tests
cargo test --workspace

# run linters / formatters (example)
yarn lint || true
# run rustfmt / clippy
cargo fmt --all -- --check || true
cargo clippy --all-targets --all-features -- -D warnings || true
```

If your change introduces new functionality, add tests that cover the behavior. For native or platform-specific changes, include clear manual smoke-test steps in the PR.


7. Labels and metadata

Please add the following labels where appropriate:
- `type:bug`, `type:feat`, `type:docs`, `type:chore`
- `ai-assisted` — MUST be added if AI tools were used
- `needs-human-approval` — for AI-assisted PRs or risky changes
- `risk:low`, `risk:medium`, `risk:high`


8. Review expectations

- Aim for at least one reviewer to respond within 48–72 hours.
- A human reviewer must not rely solely on CI; they must run the test and smoke-test commands locally for `ai-assisted` PRs.
- Reviewers should verify style, correctness, security implications, and licensing concerns for any generated code.


9. Getting help

If you need assistance, open an issue describing the problem and what you've tried. For private/security issues, contact the maintainers via the reporting contact in `CODE_OF_CONDUCT.md`.


Thank you for contributing — your help makes this project better!
