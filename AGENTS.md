# AI Coding Guidelines

These guidelines apply to AI-assisted work in this repository.

## Workflow
1. Scan every file you plan to change and directly related modules.
2. Summarize current behavior and invariants.
3. Propose a minimal patch plan (diff and rationale).
4. Scope actions to the approval tier:
   1. **Free** — reads, searches, web docs, `cargo check`/`fmt`/`clippy`/`test`, local binary runs, scratch files outside `src/`.
   2. **Task-approved** (covered by the user's initial request) — edits under `src/`, gate fixes, iteration within the agreed plan.
   3. **Ask each time** — `Cargo.toml` deps, cross-module or public-API refactors, edits beyond ~3 files or ~200 lines, file deletions, CI or release changes.
   4. **Always ask** — `git commit`, `git push`, PRs, tags, force ops, anything visible outside the local repo.
5. Affirm all `Completion Gates` are met.

## Code Design
- Prioritize correctness then idiomatic and reviewable code.
- Prefer clarity over cleverness.
- Write small single-purpose functions with clear names.
- Expand to single-purpose modules composed of concise functions.
- Prefer decomposition over accretion: extract helpers as behavior grows.
- Prefer canonical, widely understood solutions.
- Keep diffs focused; avoid idiosyncratic churn.
- Write comments that explain enduring intent or constraints, no editorial comments.

## Naming
- Naming must be semantic.
- Do not encode type or structural primitives in names (int, object, string, etc).
- Avoid namespace prefixes or suffixes. If everything starts with or ends with `_name_`, nothing should.
- Use names like `State`, `Context`, or `Manager` only if a clear abstraction requires it at a systemic level.

## Abstraction
- Abstract to remove duplication or enforce invariants.
- Prefer concrete types over generic wrappers.
- Avoid `unwrap`/`expect` outside of tests; truly-infallible uses with a justifying comment are acceptable.
- Use effective error handling patterns including `Result` and `Option`.

## Dependencies and Imports
- Prefer the standard library.
- Add external crates only with user approval.
- Declare imports at the top of each module; keep them explicit and organized so dependencies are clear.

## Tests
- Test project behavior and contracts, not language or dependency internals.
- Avoid vacuous tests: removing or breaking target code must cause a test to fail.
- Unit tests are required to be hermetic: no network or external assets.
- Add or update tests for every behavior change.

## Completion Gates

Before marking work complete, run and report:

1. `cargo check`
2. `cargo fmt --check`
3. `cargo clippy --all-targets --all-features --no-deps -- -D warnings`
4. All tests pass (unit, doc, and integration)

Do not mark work complete until all gates pass.

## Release

All commits land on a branch; `main` only ever fast-forwards.

1. Work on a branch (`claude/<topic>`); never commit to `main` directly.
2. Add an `X.Y.Z` entry to `CHANGELOG.md` and commit — this is the release
   commit. `Cargo.toml`/`Cargo.lock` are not touched at this step (the version
   was already staged to `X.Y.Z` by the previous release's step 4).
3. Create a signed, annotated tag `X.Y.Z` (no `v` prefix — matches existing
   tags) on the release commit.
4. Bump `Cargo.toml` to `X.Y.(Z+1)`, then run `cargo check` so `Cargo.lock`
   refreshes. Commit both as `docs: X.Y.(Z+1)` to prepare the next release.
5. FF-merge the branch into `main` (`git merge --ff-only`); push `main` and
   the tag — the `deploy-crate` workflow publishes to crates.io on tag push.
6. Delete the feature branch (local and remote).

The tag version matches the code version *at the tagged commit*; the
`docs: X.Y.(Z+1)` commit prepares the *next* release.
