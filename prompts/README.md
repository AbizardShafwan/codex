# Engineering policy for this Memyr build

The built-in policy lives in
[`codex-rs/models-manager/prompt.md`](../codex-rs/models-manager/prompt.md).
It is self-contained and compiled into the application. No other repository,
project configuration or runtime prompt file is required.

This README is documentation for people, not a model instruction.

The current policy uses mandatory engineering directives and leaves repository
procedures to project AGENTS.md files. The former `failure_recovery` and
`codex_integration` sections have been removed. See the current
[verification status](../docs/memyr-command.md#verification-status) for checks
performed after prompt review. Installing updated instructions still requires
rebuilding/reinstalling the executable and starting a new session.

## Behavior

Ordinary model resolution uses the compiled policy for bundled, cached,
provider-discovered and unknown models. Provider metadata still supplies model
capabilities, tool contracts and security messages, but cannot silently replace
the base policy or the local mode, delegation and persistence defaults.

Supporting prompts apply the same engineering principles within their specific
jobs. Plan mode remains non-mutating, review preserves its JSON contract,
compaction produces a handoff rather than executing instructions, and
delegation still requires explicit user permission. Security review and
approval policies are not weakened.

Explicit base instructions, including `model_instructions_file`, remain
literal overrides. An explicitly supplied `model_catalog_json` also retains its
authored messages as an advanced customization.

## Build and use

From `codex-rs`, with the required Rust toolchain and build dependencies:

```bash
cargo build --locked -p codex-cli --bin memyr
./target/debug/memyr
```

Run that executable in any project to use this build's defaults. The previous
repository-only prompt override has been removed. Editing the source does not
change a separately installed executable; installation is a separate step.
Use a new session to verify defaults, because resumed conversations can retain
their saved base instructions.

To customize the built-in policy, edit the bundled Markdown and rebuild. For a
temporary override without rebuilding, pass an absolute path to another prompt:

```bash
/path/to/custom/memyr -c 'model_instructions_file="/absolute/path/to/custom-prompt.md"'
```

See [implementation and verification notes](../docs/engineering-policy.md) for
the loading path, preserved contracts and actual check results.
