# Provider-agnostic login and model selection

## Goal

Support ChatGPT's existing login alongside isolated API-key login for Ollama
Cloud and DeepSeek. Let users select provider models and hide unsupported
commands without removing provider-independent engineering features. Preserve
the built-in engineering policy and existing user changes.

## Plan

1. Trace authentication, model discovery/selection, request capabilities and
   command gating. Verify provider API contracts against official documentation.
2. Reuse existing Responses transport where supported. Add provider-scoped
   credential storage and login without overwriting ChatGPT credentials.
3. Connect provider selection and authenticated model discovery to the TUI.
4. Gate provider-dependent commands and request features consistently. Verify
   actual plugin prompt loading rather than assuming it is entirely lazy.
5. Add focused tests for auth isolation, provider catalogs, request compatibility,
   selection and command visibility; run checks and document remaining gaps.

## Evidence and decisions

- The current `WireApi` supports only Responses, not Chat Completions.
- Official Ollama Cloud documentation now describes stateless Responses at
  `https://ollama.com/v1`, Bearer API-key auth, function tools and streaming.
  Stateful Responses, hosted Responses web search and freeform tool replay are
  unsupported. Source: https://docs.ollama.com/api/openai-compatibility
- DeepSeek documents Responses and a Codex integration. Exact capability and
  catalog handling are being checked before implementation. Sources:
  https://api-docs.deepseek.com/guides/responses_api/ and
  https://api-docs.deepseek.com/quick_start/agent_integrations/codex/
- Preserve ChatGPT login; never send its credentials to other providers.
- Prefer capabilities over model-name prefixes for feature gating.
- The user explicitly chose to continue the same conversation across providers,
  including sending its history to the selected backend. Do not replace the
  thread with an empty conversation on provider selection.
- Inspect the local DeepSeek Harness reference for cache-prefix handling and
  reasoning replay before finalizing the DeepSeek request adapter. Reference
  code is evidence, not a runtime dependency; never import its credentials.
- The user authorized a minimal live Ollama Cloud smoke test after building,
  using the existing `OLLAMA_API_KEY`. Use an isolated temporary home/workspace;
  do not print the key, send project history or change user-global configuration.

## Progress

Provider definitions, isolated encrypted credentials, public model discovery and
provider-specific login/model-list RPCs are wired to onboarding and `/login`.
The TUI reconnects an idle, unshared conversation using its existing thread ID.
Busy/shared conversations are not interrupted. Provider-independent commands
remain available; voice and ChatGPT account controls are gated by capability.

### Cache reference audit

Inspected the reference checkout's official upstream identity and its current
DeepSeek adapter, request serializer/replay handling, system-prompt projection,
agent-loop request construction and key-gated cache integration test. Its current
native adapter uses the Messages API, not the Responses API used by this fork.

Relevant principles:

- Reuse unchanged instructions, tool declarations and historical content exactly;
  append new context instead of rewriting the front of each request.
- Append system updates only on verified capable routes. Clearing or replacing
  instructions must actually remove their previous effect; cache reuse cannot
  preserve stale authority or permissions.
- Keep native thinking metadata scoped to its source model. Foreign signatures
  are not portable, while visible conversation/tool content must survive switches.
- Tool updates and image offloading change cache boundaries; do not freeze stale
  tool capabilities or silently discard content to improve a cache metric.
- Its current Messages usage adapter reads `cache_read_input_tokens` and
  `cache_creation_input_tokens`. Some cache-test comments still mention older
  Chat Completions field names; use actual wire code and current docs as evidence.
- Its live cache test uses paid requests and was not run. Its old fixed-block
  comment is not an assumption to copy; current DeepSeek docs describe persisted
  prefix units and best-effort asynchronous cache construction.

Official Responses docs specify `usage.input_tokens_details.cached_tokens`,
already decoded by this fork. OpenAI cache keys/retention are unsupported by
DeepSeek and do not activate its automatic cache. Added request-copy compatibility
handling for public catalogs: system roles instead of unsupported developer-role
semantics, no first-party cache/replay/request-metadata extensions. History order
and persistent session content remain unchanged. Focused tests verify request
compatibility, history preservation and stable prefix construction.

This is harness-inspired prefix stability, not a port of its complete Messages
adapter. Unchanged instruction IDs and namespaced tool aliases are deterministic;
the history test verifies that appending a user turn leaves the earlier request
prefix identical. Instruction, capability or tool changes must still take effect,
even when they invalidate a cached prefix. No Messages-only update policy or
client-side KV cache was added, and direct DeepSeek cache hits remain unmeasured.

### Final verification

- Built `codex-rs/target/debug/codex` with
  `cargo build --locked -p codex-cli --bin codex`. The installed CLI and global
  user configuration were not replaced.
- 202 targeted tests passed: provider-info 34, provider 114, request/history 3,
  login UI 17, command visibility/popup 29, schema freshness 4 and same-thread
  provider handoff 1. The handoff test preserved thread ID and visible turns.
- Stable/experimental protocol exports and the config schema were regenerated.
  Changed Rust files pass rustfmt checks; `git diff --check` passes. The full
  workspace suite was not run.
- With the user's installation approval, installed locked Python code-generation
  and test dependencies into `sdk/python/.venv` only, without the packaged CLI
  runtime or global package changes. Regenerated the Python protocol models for
  provider login, optional provider-scoped logout and model-list provider selection.
  Python contract freshness, client RPC and public API signature checks passed
  (91 tests), including five new provider serialization/backward-compatibility
  cases. Generation warned that unsigned integer formats fall back to Python
  integers; generation completed successfully.
- A command-auth refresh regression was caught and corrected. The expanded login
  menu's expected snapshot was updated after inspecting its actual output; the
  ordinary test rerun passed. No assertion was removed or weakened.
- Live Ollama Cloud smoke passed for `deepseek-v4.1-flash:1017`: one read-only
  `printf` tool command succeeded and the model replied `OLLAMA_PROVIDER_OK`.
  Elapsed time was 3.8 seconds. Reported usage: 14,021 input tokens, 6,912 cached
  input tokens and 75 output tokens. These are observed metrics for this run,
  not a promised future cache-hit rate.
- The smoke used a temporary `CODEX_HOME` and workspace, disabled analytics and
  feedback, inherited the API key without printing it, and cleaned up afterward.
  No project history was sent. DeepSeek's direct API path has mocked/local
  coverage, not a live inference test.
- Postman discovery was attempted; integration returned 403, and the available
  `postman` executable is a desktop launcher rather than the CLI. No CLI was
  installed; provider HTTP checks used the repository's wiremock harness.

### Limits and deferred work

- Opaque provider-encrypted compaction cannot be transferred to another provider.
  Such histories are rejected explicitly rather than silently truncated. A
  plaintext handoff to a new conversation is required for those histories.
- Public routes reject HTTP redirects and do not use an ambient ChatGPT identity
  for credentials or 401 recovery. Saved-key logout leaves ChatGPT intact;
  environment credentials must be unset separately.

## Usage

### Follow-up: agents hint and thinking-level selection

- User reports the `← for agents` hint is missing and cannot select thinking
  levels for Ollama Cloud `deepseek-v4.1-flash:1017` (`/status`: reasoning none).
- Source evidence: agents navigation is deliberately limited to local-daemon
  sessions and an empty composer; this is not provider capability gating.
- Root cause: public catalog parsing imported IDs only. Exact direct DeepSeek
  model IDs had advisory presets, but Ollama and newer aliases lacked controls.
  Old capability caches could also survive development rebuilds at version 0.0.0.
- Added native DeepSeek `effort` parsing and Ollama `/api/show` thinking discovery,
  including named levels and on/off-only models. Only validated thinking metadata
  is read; provider system prompts and templates remain ignored. Missing or
  malformed metadata does not hide otherwise usable models.
- Ollama detail requests reuse provider auth, transport/proxy routing, retries,
  redirect rejection and response-size limits. Four concurrent requests and an
  overall best-effort deadline bound resource use; unfinished tasks are cancelled.
  Public cache identities now invalidate the older ID-only capability projection.
- Live read-only Ollama metadata for `deepseek-v4.1-flash:1017` advertised
  `[false, "low", "high", "max"]` with default `"high"`. No inference was run.
  DeepSeek's official `/models` contract also advertises native effort metadata;
  its live endpoint requires credentials and was not used for inference.
- The agents hint is not provider-gated: command-line configuration overrides
  deliberately select embedded mode because a shared server cannot silently adopt
  arbitrary invocation configuration. No navigation guard was weakened.
- Request regression tests confirm that selected levels reach Responses requests,
  not merely the UI. The test-drive binary has been rebuilt and its catalog RPC
  verified against local mocks and live read-only Ollama metadata.

Additional findings in the same test drive:

- Reasoning-summary-only history fell through to the generic historical-record
  adapter, which made it ordinary assistant text. Changed replay to keep all
  visible reasoning on typed reasoning items instead. Added the reported scratch-
  page text as a regression fixture; no durable conversation history is rewritten.
- Added a separate `provider/account/read` RPC for provider-native account status,
  rather than pretending third-party balances are ChatGPT quotas. `/status` starts
  an on-demand read, updates the original status card and never polls in the
  background. It uses only the selected provider's credentials, rejects redirects,
  limits response bytes and deadlines, and does not automatically retry 429s.
- Ollama's documented `/api/balance` returns legacy session/weekly remaining
  percentages and reset times, or included/purchased dollar balances for newer
  plans. DeepSeek's `/user/balance` returns currency-specific decimal balances.
  These fields are displayed as reported, not synthesized from token usage.
  A read-only live Ollama account check returned HTTP 200 and confirmed the
  session/weekly `remaining_percent` and `resets_at` fields for this account;
  neither credentials nor actual balance/quota values were printed.
- Initial web-search finding: this repository does contain the
  `web.run` executor, compiled into the CLI through the app-server extension
  registry, but it calls the active provider's `alpha/search` backend.
  It cannot simply be enabled against Ollama's different API. No third-party
  search-service connection or paid live search was authorized or run at that
  stage. The subsequent native-provider implementation is documented below.
- User's launch command sets workspace-write network access but does not select
  workspace-write, so the previously observed Read Only mode remains relevant.
- Regression verification: provider tests passed (124), replay/request tests
  passed (5), native reasoning-picker/status UI tests passed, and Python SDK
  checks passed (92). An unrelated agents-navigation test hit Rust's default test
  stack limit, then passed with the exact repository CI test-thread stack setting,
  `RUST_MIN_STACK=8388608`. No
  core dump was available, no global stack setting was changed, and no assertions
  or navigation guards were weakened.
- Stable and experimental schema fixtures were regenerated and freshness tests
  passed (four, one ignored). Native account reset timestamps export as ordinary
  TypeScript numbers, matching JSON and existing protocol timestamp conventions.
- The final `cargo build --locked -p codex-cli --bin codex -j3` succeeded. The
  binary remains `codex-rs/target/debug/codex`; neither the installed CLI nor the
  user's configuration was replaced. Restart existing CLI processes to use it.
- End-to-end JSON-RPC checks of that binary passed against local HTTP mocks:
  both provider catalogs expose None/Low/High/Max with default High, native account
  reads return balances/windows, and all five requests use only their own scoped
  credentials. The initial smoke incorrectly tried to override reserved built-in
  provider IDs, which existing configuration deliberately ignores. Using custom
  provider IDs for the mock endpoints corrected the fixture; assertions stayed
  intact and no production routing change was needed.
- A live read-only JSON-RPC smoke of the rebuilt binary confirmed Ollama model
  `deepseek-v4.1-flash:1017` exposes None/Low/High/Max (default High), and `/status`
  data includes the 5-hour/weekly window labels, both reset timestamps, and a
  purchased-balance row. No account values or credentials were printed; no
  inference or search requests were made. Direct DeepSeek remains locally mocked,
  not live-verified. Existing leaked assistant messages in saved history are not
  rewritten; only future request adaptation is fixed.

### Follow-up: native web tools

The user requested official-provider documentation research and implementation
of browsing for both public providers. Current official documents now establish
native search/fetch endpoints for both, so no cross-provider search service or
additional credential is needed:

| Provider | Search | Read a page | Authentication |
| --- | --- | --- | --- |
| Ollama Cloud | `POST /api/web_search` (`query`, `max_results`) | `POST /api/web_fetch` (`url`) | Selected provider's Bearer API key |
| DeepSeek | `POST /v1/tools/search` (`query`, `count`, `need_content`) | `POST /v1/tools/fetch` (`url`) | Selected provider's Bearer API key |

Official sources read on 2026-10-23:

- https://docs.ollama.com/capabilities/web-search
- https://api-docs.deepseek.com/guides/search/
- https://api-docs.deepseek.com/api/search/
- https://api-docs.deepseek.com/api/fetch/
- https://api-docs.deepseek.com/guides/responses_api/

DeepSeek's hosted Responses `web_search` is Pro-specific; the separate search and
fetch APIs allow the same client-executed tool interface across its models.
Ollama's separate search service likewise does not require hosted Responses tools.
Provider-native search/fetch may consume service quotas or charges under the
provider's terms. No live search or paid model request was used for verification.

Implementation reuses the existing `web.run` extension and browsing UI events,
with a restricted schema for `search_query`, `open` and `find`. Search results
carry explicit URLs; page operations use those URLs, preserving usability across
provider switches and restarts without inventing portable opaque search IDs.
Fetched text is untrusted external context. Pages are bounded, locally cached
per thread/provider and expose numbered windows with explicit truncation markers.
Image search, screenshots, numbered-link clicking and specialized data commands
are not advertised by the native schema.

Enable with `--search` or top-level `web_search = "live"`. Cached-only, indexed-only
and disabled modes never silently become live requests. Existing provider
capability opt-outs remain effective. Configured domain/location restrictions
are rejected explicitly because their semantics are not portable to both APIs.
Public providers never receive a hosted Responses web-search declaration.
OpenAI's existing `alpha/search` path and schemas remain in use for its models.

Requests retain same-origin provider routing and proxy prefixes/query parameters,
reject redirects, use scoped credentials and never attach ChatGPT recovery or
conversation history. Each operation has one HTTP attempt, a response byte limit
and an overall tool deadline. Provider errors are sanitized and returned to the
model so ordinary failures do not abort the conversation.

Postman API discovery was attempted; Orbit failed certificate-chain validation
in both Python and curl. TLS verification was retained. The existing Postman
CLI availability limitation remains, so HTTP regression tests use the repository's
wiremock transport with the official contracts above.

Verification completed:

- 165 targeted Rust tests passed: the 125-test provider suite, 25 core browsing/
  tool-registration checks and all 15 web-extension tests. This includes nine new
  native-adapter/capability tests, with actual HTTP coverage of auth, request
  formats, reverse-proxy paths, bad responses, body limits, redirect rejection,
  rate limits, page reuse and a hanging service's 30-second deadline. The final
  rerun also verifies extracted links are returned with page windows. The initial
  compile caught a missing import in a new test fixture; it was corrected before
  the ordinary rerun. No assertions were weakened.
- Built `codex-rs/target/debug/codex` successfully with
  `cargo build --locked --offline -p codex-cli --bin codex -j3` (5m 41s).
  `--version` reports `codex-cli 0.0.0`. No installed executable or user-global
  configuration was replaced.
- End-to-end JSON-RPC smoke of that executable passed for both providers against
  local HTTP mocks: six real harness tool calls (search/open/find per provider),
  four native-service HTTP requests (find reuses the page), eight mocked Responses
  requests and six completed browsing UI items. Tool declarations, alias restoration,
  result replay and isolated provider credentials were checked. No hosted Responses
  web declaration or `alpha/search` request was produced.
- Available stable rustfmt checks and `git diff --check` passed. The installed
  formatter warns that nightly-only import granularity is unavailable. The lockfile
  adds only existing workspace dependency edges for serde, tokio and wiremock in
  the web extension; no dependency versions or global tools were installed.
- Full workspace tests and live provider search/inference were not run. Native
  remote search access, current service quotas and returned content remain live
  verification limits. Test processes used temporary homes/workspaces and fake
  credentials, with no user conversation history sent to a live service.

After rebuilding this fork, run `/login`, select ChatGPT, DeepSeek or Ollama
Cloud, then choose a model with `/model`. Existing ChatGPT authentication stays
separate from provider keys. A native provider's empty key field reconnects using
server-side saved credentials or its environment variable; it does not transfer
the client's environment to a remote server. Explicit environment keys take
precedence over saved keys. Saved keys use the existing encrypted local secrets
backend under the server's `CODEX_HOME/provider-auth`, scoped by provider/endpoint.
`/logout` removes the selected public provider's saved key without logging out
ChatGPT. It does not unset or revoke an environment API key.

To start the built CLI with an environment key:

```sh
./codex-rs/target/debug/memyr -c 'model_provider="ollama-cloud"' \
  -m 'deepseek-v4.1-flash:1017'
./codex-rs/target/debug/memyr -c 'model_provider="deepseek"' -m deepseek-flash
```

Set `OLLAMA_API_KEY` or `DEEPSEEK_API_KEY` in the environment of the **server**
process. Both providers use stateless Responses and resend visible conversation
history. Changing providers therefore shares that history with the selected
provider. Provider-encrypted compaction cannot be decrypted by another service;
use the original service to prepare a plaintext handoff for a new conversation
in that case.

Public discovery imports model IDs and validated provider-specific thinking
controls, not trusted system instructions. It cannot replace the built-in
engineering policy. Use `/model` to select a model and its advertised reasoning
level; Max is under the existing “More reasoning…” choice. Boolean-only Ollama thinking
uses None for off and High for on. Advanced custom providers may
opt into public-ID discovery with `model_catalog_format = "openai"`; the existing
default `"codex"` retains rich catalog metadata behavior.

Sources: [Ollama compatibility](https://docs.ollama.com/api/openai-compatibility),
[DeepSeek Responses](https://api-docs.deepseek.com/guides/responses_api/),
[DeepSeek caching](https://api-docs.deepseek.com/guides/kv_cache/).
Thinking metadata: [Ollama](https://docs.ollama.com/capabilities/thinking),
[DeepSeek](https://api-docs.deepseek.com/api/list-models/).
