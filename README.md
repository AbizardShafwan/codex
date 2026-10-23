# Memyr

Memyr is a personal coding agent for working in your terminal. It supports
ChatGPT sign-in, Ollama Cloud and DeepSeek, with a shared engineering policy.

## Install, update and uninstall

Install this checkout as the `memyr` command. From the repository root:

```bash
cd codex-rs
cargo install --locked --path cli --bin memyr
memyr
```

This compiles a release build and installs only the `memyr` executable. Rust and
the native build dependencies are required; the Rust toolchain is specified in
[`codex-rs/rust-toolchain.toml`](codex-rs/rust-toolchain.toml).

Cargo installs into `~/.cargo/bin` by default. If `memyr` is not found, add that
directory to your current shell's PATH (use your configured Cargo bin directory
if it differs):

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

To update after changing this checkout, run the following from the repository
root. It rebuilds and reinstalls the local source; it does not pull Git changes:

```bash
cd codex-rs
cargo install --locked --path cli --bin memyr --force
```

To uninstall only the Memyr executable:

```bash
cargo uninstall --bin memyr codex-cli
```

`codex-cli` is the internal Rust package name. Uninstalling the executable leaves
your configuration, credentials and conversation history intact. If you used a
custom Cargo installation root, use the same `--root` for uninstalling.

There is no dedicated Memyr install/update/uninstall script yet. The inherited
`scripts/install/install.sh`, `scripts/install/install.ps1` and built-in
`memyr update` still target the original distribution. Use the local Cargo
commands above to maintain this fork.

## Build and run without installing

From the repository root:

```bash
cd codex-rs
cargo build --locked -p codex-cli --bin memyr
./target/debug/memyr
```

The repository also provides `just memyr` and `just exec` from the repository
root for running from source.

The commands above are instructions for your use. See the current
[verification status](docs/memyr-command.md#verification-status) for completed
checks and remaining limitations.

## Providers

Use `/login` and `/model` in the interactive CLI. Once installed, you can also
select a provider at launch:

```bash
memyr -c 'model_provider="ollama-cloud"' -m 'deepseek-v4.1-flash:1017'
memyr -c 'model_provider="deepseek"' -m deepseek-flash
```

Existing configuration, credentials and sessions retain their current storage
paths and `CODEX_HOME`/`CODEX_SQLITE_HOME` settings. The command rename does not
migrate or delete that data. A compatibility `codex` binary target remains for
existing clients; `memyr` is the default Cargo entry point.

## Policy and local launcher

The built-in identity is Memyr. The engineering policy is compiled from
[`codex-rs/models-manager/prompt.md`](codex-rs/models-manager/prompt.md).
Rebuild and start a new session after editing it; resumed sessions can retain
previously saved instructions. Explicit custom prompt overrides retain their
configured text.

[`codex-cli/bin/memyr.js`](codex-cli/bin/memyr.js) launches a local debug/release
build or a bundled `vendor/<target>/bin/memyr` executable. Its private local npm
package exposes the `memyr` command and does not download an upstream CLI.

See [policy notes](prompts/README.md), [provider notes](docs/provider-agnostic-implementation.md)
and [rename notes](docs/memyr-command.md). Repository development instructions
are in [AGENTS.md](AGENTS.md).

Licensed under [Apache-2.0](LICENSE). Existing copyright and attribution notices
remain in [NOTICE](NOTICE).
