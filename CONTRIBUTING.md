# Contributing to `parsm`

Contributions are welcome: a bug report, a fix, input parsm reads or prints wrong, or a feature.

## Pull requests

1. Fork the repo and branch from `main`.
2. Add tests for anything that changes behavior.
3. Update README.md, `parsm --examples` (`print_usage_examples` in `src/bin/parsm.rs`), and CHANGELOG.md when you change what they describe.
4. Make the checks below pass.
5. Open the pull request.

`main` only ever fast-forwards. Rebase rather than merge.

## Before you push

```sh
cargo fmt --check
cargo clippy --all-targets --all-features --no-deps -- -D warnings
cargo test
bin/quick_test.sh
```

`bin/quick_test.sh` builds the release binary and checks its output end to end.

## Tests

Add tests for behavior changes, and prove each one fails when its target breaks: break the code on purpose, watch the test go red, put it back. A vacuous test covers nothing. Unit tests are hermetic: no network, no files outside the checked-in tree. Integration tests may access external files.

Assert on output, not on a parse succeeding.

## Grammar

`pest/parsm.pest` is the one parser. A syntax change comes with a test for the new form and for what it must still reject.

## Commits

[Conventional Commits](https://www.conventionalcommits.org), signed and in lower case: `feat(dsl): …`, `fix(grammar): …`, `docs(readme): …`. [commitlint](commitlint.config.js) checks them on pull requests.

## Style

The tree is `rustfmt` clean and `clippy` clean with warnings as errors. Otherwise match the file you are in: semantic names with no type or namespace affixes, small single-purpose functions, `Result` and `Option` rather than `unwrap` outside tests, and source files under about 2,500 lines.

Ask in an issue before adding a dependency.

## Bug reports

The smallest input plus expression is the reproduction. Include the command, actual output, expected output and `parsm --version`.

```sh
echo '{"name":"Alice","age":30}' | parsm 'age > 25 [${name}]'
```

This prints `Alice`.

## Working with an AI agent

`AGENTS.md` is the brief for AI agents working in this repo: the conventions at length and the completion gates. Point your agent at it.

## License

`parsm` is distributed under the MIT License (`LICENSE`). By contributing you agree that your contributions are licensed under the same terms.

---

Adapted from the open-source contribution guidelines for
[Facebook's Draft](https://github.com/facebook/draft-js).
