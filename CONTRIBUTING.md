# Contributing to Aether

Thank you for your interest in contributing to Aether — the AI-centric programming language.

## How to Contribute

1. **Fork** the repository and create a feature branch from `main`.
2. **Make your changes** — keep commits focused and atomic.
3. **Test** your changes: `cargo build --release && cargo test`
4. **Sign your commits** with a DCO sign-off (see below).
5. **Open a pull request** against `main` with a clear description of what and why.

## Developer Certificate of Origin (DCO)

All contributions must be signed with the [Developer Certificate of Origin](https://developercertificate.org/). This certifies that you have the right to submit the contribution under the Apache 2.0 license.

Sign each commit with `-s` (or `--signoff`):

```
git commit -s -m "feat: add new intent to ASL registry"
```

This adds a `Signed-off-by: Your Name <your@email.com>` trailer to the commit message. Pull requests without DCO sign-offs will not be merged.

## Contribution Areas

- **Grammar / Parser** (`src/aether.pest`, `src/parser.rs`) — new syntax proposals should include spec updates in `spec/aether.ebnf`
- **ASL Registry** (`asl/registry.json`) — new standard intents should include description, safety level, recommended language, and rationale
- **Executor** (`src/executor.rs`) — performance improvements, new guest language support
- **Documentation** (`docs/`) — corrections, examples, and clarifications always welcome
- **Examples** (`examples/`) — real-world `.ae` programs demonstrating Aether capabilities

## Code Style

- Run `cargo clippy` before submitting — zero warnings expected
- Run `cargo fmt` to format your code
- Public functions should have doc comments (`///`)

## Reporting Bugs

Open a [GitHub Issue](https://github.com/baiers/aether/issues) with:
- Aether version (`aether --version`)
- The `.ae` or `.as` program that triggers the bug
- Expected vs. actual behavior
- Full error output

## Security Issues

Please **do not** open public issues for security vulnerabilities. See [SECURITY.md](SECURITY.md) for the responsible disclosure process.

## License

By contributing, you agree that your contributions are licensed under the [Apache License 2.0](LICENSE).
