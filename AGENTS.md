# Repository instructions

This is Memyr, a personal coding-agent CLI. Use `memyr` for the public command
and Memyr for the assistant identity. Internal `codex-*` crate names, protocols,
environment variables and storage paths remain compatibility contracts; do not
rename them as cosmetic cleanup.

Read applicable nested AGENTS.md files before editing their directories.
Preserve unrelated tracked, staged and untracked work.

## Source map

- `codex-rs/cli`: native entry points; Cargo defaults to `memyr`. The `codex`
  binary remains a compatibility entry point sharing the same implementation.
- `codex-rs/models-manager/prompt.md`: canonical compiled engineering policy.
  Supporting mode and specialist prompts live in their owning Rust crates.
- `codex-rs/model-provider*`, `codex-rs/login`: provider capabilities and auth.
- `codex-rs/core`, `codex-rs/ext`, `codex-rs/tui`: sessions, tools and interface.
- `codex-cli/bin/memyr.js`: private local Node launcher for this fork's binary.

## Change boundaries

- Keep project procedures here and general engineering behavior in the compiled
  policy. Do not duplicate the full system policy into AGENTS.md.
- Preserve explicit prompt/catalog overrides and provider capability metadata.
  Do not imply that editing prompts changes an installed or resumed session.
- Preserve provider-scoped credentials, stable cache prefixes, reasoning replay
  boundaries and conversation continuity when switching providers.
- Never route a provider's credentials to another provider. Do not print secrets
  or send repository history in live provider checks without authorization.
- Keep approval and sandbox enforcement intact. The tool definitions are under
  review; do not shorten or change them as incidental cleanup.
- Keep the policy self-contained; do not add a runtime dependency on another
  checkout. Preserve LICENSE and NOTICE attribution.

## Commands and verification

Follow README.md for local build, install, update and uninstall commands. The
inherited installers and built-in updater are not adapted for this fork.

Use focused checks for the affected crate; do not default to the full workspace
or `--all-features`. Honor explicit user requests to defer tests or builds.
Review behavior changes before accepting snapshot updates. Installation and live
provider calls require the user's authorization.

Treat `docs/prompt-audit` captures and older verification notes as historical
evidence. Label source-based projections separately from fresh request captures.
Report unrun checks and pending rebuilds explicitly.
