# Memyr command and identity

The primary executable is `memyr`. The Rust package/library names remain
`codex-cli`/`codex_cli`, and the existing `codex` target is retained as a
compatibility entry point for clients and internal development tooling. Both
entry points share the same CLI implementation; Cargo defaults to `memyr`.

The main bundled policy identifies the assistant as Memyr without claiming it
runs in Codex CLI. The realtime policy, protocol fallback and older prompt
assets use the same identity. Explicit user-authored prompt/catalog overrides
remain literal overrides, and existing saved sessions can retain old prompts.

CLI help, completion scripts, session titles and resume hints use the Memyr
name. Tool definitions were not shortened or otherwise changed in this task.
Existing storage paths, environment variable names, service protocols, ChatGPT
authentication and provider identifiers retain their compatibility names.

## Local use

From `codex-rs`:

```bash
cargo build --locked -p codex-cli --bin memyr
./target/debug/memyr
```

For a command on PATH, explicitly install your build from that directory:

```bash
cargo install --locked --path cli --bin memyr
memyr
```

`just memyr` and `just exec` use the new binary. The private local npm package
exposes `memyr` through `codex-cli/bin/memyr.js`; it resolves only a local native
build or a vendor tree containing this fork's native binary. Private npm staging
includes a local vendor tree when supplied instead of upstream platform packages.
The native package assembler now defaults to its new `memyr` variant while
retaining the existing compatibility variants. Bazel exposes the new native
binary and a sibling `memyr-release` archive target.

## Verification status

Focused testing was authorized after prompt review. Results on 2026-10-23:

- 127 Rust library tests passed: model manager (63), prompts (43), CLI utilities
  including resume commands (21).
- 69 TUI tests passed: status (57), session headers (10), initial header (1),
  and clearing a long transcript (1). Snapshot differences were reviewed before
  acceptance: branding, account login command text and the shorter title's
  spacing/style positions. No unrelated snapshot differences were accepted.
- 27 native packaging tests and 2 private npm staging tests passed.
- 4 Node launcher tests passed: arguments/environment/exit codes, missing binary
  handling, bundled binary precedence and SIGTERM forwarding.

Testing exposed a private npm staging bug: explicit `--vendor-src` content was
ignored. The builder now includes that vendor tree and preserves executable
permissions; the new regression test passes. Package filename expectations and
the update-command test were adjusted for the public Memyr name.

- 312 CLI unit tests and the update-command integration test passed. One
  intentionally ignored subprocess fixture remains ignored by the normal test
  harness (`doctor::filesystem_paths::tests::blocked_probe_fixture`).
- The rebuilt native `target/debug/memyr` passed six isolated command checks:
  version, top-level help, exec help, and Bash/Zsh/Fish completions.

Total: **542 passed, 1 intentionally ignored**, plus six native command smoke
checks. Rust/Node syntax and whitespace checks passed. Stable rustfmt emits the
existing warning that nightly-only `imports_granularity` is unavailable.

Commands used from `codex-rs`:

```bash
cargo test --locked --offline -p codex-models-manager -p codex-prompts -p codex-utils-cli --lib -j 2
RUST_MIN_STACK=8388608 cargo test --locked --offline -p codex-tui --lib status::tests -j 2 -- --test-threads=2
RUST_MIN_STACK=8388608 cargo test --locked --offline -p codex-cli --bin codex --test update -j 2 -- --test-threads=2
```

After compiling the TUI test binary, the same binary was also run directly with
filters for `history_cell::tests::session_`,
`initial_session_header_starts_at_the_top_of_the_viewport`, and
`clear_ui_after_long_transcript_snapshots_fresh_header_only`. This avoided waiting
on the concurrent CLI build's Cargo lock; all selected cases passed.

Commands used from the repository root:

```bash
node --test codex-cli/tests/memyr-launcher.test.mjs
python -m unittest discover -s codex-cli/tests -p 'test_*.py'
CODEX_REPO_ROOT="$PWD" python -m unittest discover -s scripts/codex_package -p 'test_*.py'
```

Native debug binaries were compiled by Cargo's test build, but no release build,
installation or live provider calls ran. These checks used installed Rust
1.98.1; the pinned Rust 1.99.1 toolchain was not installed. The full workspace,
Bazel release targets and runtime behavior on other operating systems have not
been tested. A fresh model request was not captured during this verification.
