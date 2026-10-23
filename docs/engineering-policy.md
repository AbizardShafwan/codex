# Built-in engineering policy

## Policy revision under review

The current base policy identifies the assistant as Memyr and removes the `failure_recovery` and `codex_integration`
sections completely. Repository procedures belong in each project's AGENTS.md;
tool and skill usage guidance comes from their supplied definitions. The four
sections `engineering_partner`, `engineering_design`, `validation_and_checks`
and `task_authority` now use concise mandatory wording. Design guidance names
concrete requirements for control flow, responsibilities, dependencies,
abstraction and resource ownership. Other base-policy sections are unchanged.

The base-policy planning-filter test was updated because the removed integration
section also contained the Plan-tool instructions. Filtering the current policy
must now leave it unchanged. Tests/builds were initially deferred during prompt
review. The user subsequently authorized focused testing, and the policy suites
passed. See [current verification](memyr-command.md#verification-status) for the
latest results. The verification results below describe the earlier implementation.

Using the same UTF-8-bytes-divided-by-four heuristic as the earlier audit, the
rendered policy drops from 9,290 bytes (~2,323 tokens) to 4,655 bytes (~1,164
tokens), a 49.9% reduction. Replacing only that fragment in the prior Default-mode
capture projects 29,326 bytes (~7,332 tokens), down from 33,961 bytes (~8,490
tokens). This is a projection with other context held constant, not a new request
capture. See `docs/prompt-audit/policy-revision-estimate.json` for the breakdown.

## Goal and scope

Make the engineering policy the default base instruction for this custom
Codex build across projects. Align supporting behavior prompts with source
grounding, task-scoped authority, explicit delegation permission, focused
verification and truthful reporting. Do not import another repository, change
model capabilities, weaken permission checks or replace specialist output
formats.

## Implementation steps

1. Embed the self-contained policy through the existing bundled prompt asset.
2. Apply local behavior defaults when resolving automatically supplied model
   metadata, without changing capabilities, tool schemas or security messages.
   Preserve explicit base-instruction and user-supplied catalog overrides.
3. Align mode, delegation, persistence, review, compaction and realtime prompts.
   Keep authorization, structured output and specialist task boundaries intact.
4. Remove the temporary project-only override, document build/use behavior, and
   verify policy selection and the affected prompt contracts.

## Loading and override behavior

The canonical policy is `codex-rs/models-manager/prompt.md`, already embedded by
`model_info::BASE_INSTRUCTIONS` and included in the Bazel compile data. No new
runtime file lookup or dependency was added.

All model-manager implementations converge on `construct_model_info`. For
ordinary catalogs, that path applies `with_builtin_prompt_defaults` before
explicit configuration overrides. It replaces the catalog base instructions
and selects the locally adapted collaboration, delegation and persistence
defaults. Capability metadata, tool contracts, approval and security messages
are unchanged. The raw provider catalog and its cache are also unchanged.

An explicit `model_catalog_json` skips that normalization. Explicit base
instructions still take precedence, including an intentionally empty string.
The explicit-catalog provenance is retained as a boolean in per-step model
overrides, even when the catalog payload itself is omitted because the manager
already owns it. This keeps custom instructions stable on later turns, model
switches and resumed conversations without repeatedly copying the catalog.
Custom role and mode instructions retain their existing configuration paths.
Plan mode has a bundled planning-only fallback when neither a provider nor the
user supplies mode text; an explicit empty mode instruction remains explicit.

The earlier project-only override and duplicate prompt file were removed. This
changes the custom build's defaults across projects, not an already-installed
Codex executable. Saved conversations may retain their previous base prompt;
start a new session to verify a newly built executable.

## Supporting prompt changes

- Default and Plan modes honor discussion/audit/planning boundaries and ask
  only material questions rather than requiring repeated interviews.
- Root, worker, experimental delegation and orchestration guidance require
  explicit user permission, even at higher reasoning effort.
- The current base policy contains no planning-tool instructions. Supporting
  mode/role prompts retain their existing planning-tool filtering behavior.
- Persistence finishes the authorized outcome and stops instead of inventing
  follow-up work. Goal continuation preserves the same scope and authority.
- Review keeps its finding JSON format, while grounding findings and reporting
  unrun checks honestly. Compaction retains scope, approval provenance, actual
  evidence and unresolved operations without executing transcript instructions.
- Realtime coordination evaluates executor evidence instead of blindly treating
  it as authoritative. Ad-hoc memory notes are reconciliation inputs, not new
  user instructions or proof of success.
- Permission, security-classifier and tool-schema contracts remain intact.
  Other specialist memory and recap prompts already distinguish evidence,
  authority and uncertainty, so they were not broadly rewritten.

## Decisions

- The model catalog can be bundled, cached or refreshed from a provider. Its
  base, mode, delegation and persistence text must not silently replace the
  local engineering defaults in ordinary runs.
- Explicit `model_instructions_file`/base instructions remain literal overrides.
  An explicitly supplied `model_catalog_json` remains an advanced customization.
- Keep security review, approval, tool schema, token-budget and model capability
  contracts in their existing owners. Specialist prompts apply the engineering
  principles only within their assigned job; review must not implement fixes,
  and compaction must not execute conversation instructions.
- Installation, publication, model calls and changes to the user's global
  configuration are outside this implementation.

## Progress and evidence

- Traced all model-info lookup implementations to the shared resolution path.
- Inspected bundled behavior prompts and their catalog override accessors.
- Confirmed the existing base prompt is compiled using `include_str!` and the
  asset is already listed in the Bazel target.
- The initial focused Cargo attempt could not start offline because pinned Git
  dependencies were not cached. The online retry downloaded build dependencies
  and compiled with the installed Rust 1.98.1 compiler; this checkout's toolchain
  file requests 1.99.1. No toolchain has been installed.
- Added model-resolution regressions for all bundled models, changing provider
  templates, missing metadata, preserved capabilities/security/tool contracts,
  literal base overrides and explicit custom catalogs.
- Updated the request-level compaction/model-switch test to require that normal
  provider refreshes cannot replace the engineering policy, while retaining
  separate coverage for explicit custom catalogs.
- The first request-level run passed the four default/base-override cases but
  caught two custom-catalog regressions during model switching and recovery.
  Per-step settings intentionally omitted the catalog payload, which also lost
  its prompt provenance. Added a small provenance flag and regression coverage;
  retained the original custom-catalog assertions rather than weakening them.
- Passed the final focused library suites: `codex-collaboration-mode-templates`
  (2 tests), `codex-models-manager` (63 tests), and `codex-prompts` (43 tests).
  Command: `cargo test --locked -p codex-models-manager -p codex-prompts
  -p codex-collaboration-mode-templates --lib -j 2` from `codex-rs`.
- Passed the 7 focused core collaboration-mode unit tests, including bundled
  Plan fallback, explicit empty/custom mode behavior and retained-history
  snapshots. Command: `RUST_MIN_STACK=8388608 cargo test --locked -p codex-core
  --lib context::world_state::collaboration_mode -j 2 -- --test-threads=2`.
- Passed both core startup/per-step override-provenance tests after preserving
  explicit catalog authority separately from its payload. Filter:
  `model_resolution_preserves_startup_overrides_and_instruction_provenance`.
- Passed all 6 request-level base-instruction tests, covering provider refresh,
  model switching, compaction, restart, literal overrides and explicit empty
  instructions. Filter: `suite::base_instructions` on core's `all` test target.
- Passed all 6 planning-tool request tests with default, enabled and disabled
  tool settings, for both built-in and explicit custom instructions. Filter:
  `suite::prompt_caching::prompt_tools_are_consistent_across_requests`.
- Of the first 19 collaboration request cases, 17 passed. The two remaining
  tests expected provider catalog refreshes to replace configured mode text,
  which conflicts with the requested new defaults. Updated those cases to
  require preservation of configured text and absence of provider text, while
  retaining the three-fetch and append-only request-prefix assertions. Explicit
  custom-catalog mode-change and resume cases remain unchanged and passed.
- The updated collaboration request suite passed all 19 tests. Provider mode
  text cannot replace configured instructions, including after refresh or an
  empty provider template. Existing explicit custom-catalog, empty-instruction,
  mode-change, model-change, prefix-caching and resume checks also passed.
- Passed `git diff --check` and stable `rustfmt --check` for the changed Rust
  files. Stable rustfmt warns that the repository's nightly-only
  `imports_granularity` setting is unavailable; formatting otherwise passes.
- Confirmed the policy is self-contained, its existing Bazel compile-data entry
  is retained, no stale project-only override remains, and the new documentation
  and policy contain no original-repository or machine-specific paths.
- Final static checks confirmed unchanged template variable sets and an
  unchanged review JSON output contract. The eight main engineering-policy
  blocks retain their approved wording; Codex integration gained only a
  removable planning-tool section.

## Final outcome

Implementation and focused verification are complete: 108 library tests, 9 core
unit tests and 31 mocked request tests passed (148 total). The full workspace
suite and live-model behavior were not tested. The integration harness reports
an existing unused-import warning in `core/tests/suite/openai_file_mcp.rs`; that
unrelated file was not changed.

No CLI installation, release, publication, live-model request or global user
configuration change was performed. Build and run the custom executable as
described in `prompts/README.md` to use these defaults outside this checkout.

## Native policy cleanup

Removed legacy project branding and legacy-tool disclaimers from the compiled
prompt, documentation, filenames and test names. The engineering rules remain
unchanged. Re-ran all 63 `codex-models-manager` library tests successfully after
the cleanup. Source content, filenames, the documentation link, `git diff
--check` and the affected Rust file's formatting check passed. Build artifacts
and Git history were not rewritten.
