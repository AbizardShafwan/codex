PROMPT AND CONTEXT AUDIT OF THIS FORK
Captured: 2026-10-23

POLICY REVISION: These request captures and their original metrics predate the
latest policy edit. The current source removes failure_recovery and
codex_integration and revises four engineering sections. See
policy-revision-estimate.json for current source sizes and projected totals.
The estimate was initially calculated without rebuilding. Subsequent focused
tests and a native debug test build passed; see ../memyr-command.md for results.
No fresh model request was captured. These original captures remain historical.

Scope
-----
This inventory covers repository-owned prompt assembly and first requests made
by the rebuilt fork to LOCAL mock providers. It does not dump the instructions
of the application hosting this conversation. No live model inference or live
search requests were made. Provider-internal instructions/chat templates are
outside what this client-side capture can establish.

There is no single universal system prompt. The payload depends on provider,
model, settings, selected mode, enabled integrations, AGENTS.md, explicit skill
mentions and retained conversation history. A resumed conversation can also
retain its older base instructions.

Files to read
-------------
1. default-mode-request.txt: full rendered message text and tool schemas from
   a fresh local request with the fork's bundled Default-mode instructions
   explicitly selected, matching the relevant TUI mode selection. The final
   user message is a labeled fixture, not a real user conversation.
2. default-mode-request.json: corresponding outbound Responses JSON body.
3. bundled-prompt-assets.txt: verbatim contents of 50 repository text assets,
   including the canonical engineering policy, mode/permission variants,
   specialist templates and bundled skill bodies. They are alternatives and
   conditional assets, NOT one concatenated runtime prompt.
4. inline-prompt-constants.txt: 32 long constant strings extracted from the
   indexed Rust prompt generators. Format/concat-generated messages require
   their generating source and are represented in rendered captures where active.
5. source-index.json: asset paths, byte sizes, SHA-256 values and 104 relevant
   message/tool generator sources. It also identifies old prompt files that
   ordinary model resolution does not select in this fork.
6. comparison.json: fragment sizes and individual tool sizes for all captures.
7. minimal-*, with-100-skills-* and lean-tools-*: other request captures and
   metrics. Minimal means a fresh app-server request without an explicit mode
   mask, NOT literally zero context or zero skills.

What goes to the model before/alongside the user request
-------------------------------------------------------
A. Base engineering policy.
   Source: codex-rs/models-manager/prompt.md
   Compiled by models-manager/src/model_info.rs. Ordinary bundled, cached,
   discovered and fallback models resolve to this policy. Explicit custom base
   instructions and explicitly supplied rich catalogs retain their override
   paths. The source policy is 9,508 UTF-8 bytes; in these captures its disabled
   Plan-tool section is removed, leaving 9,290 bytes. It is not duplicated with
   all the old gpt_5*.md files.

B. Developer/configuration policy and managed requirements, when supplied.
   Sources: core/src/session/mod.rs and
   core/src/context/world_state/managed_developer_instructions.rs.
   Explicit developer_instructions and administrator requirements may contribute
   additional text. These are installation/configuration inputs, not static
   repository strings that can be universally enumerated.

C. Permissions and environment.
   Sources: prompts/src/permissions_instructions.rs, prompts/templates/permissions,
   core/src/context/world_state/permissions.rs and environment.rs.
   Actual sandbox mode, approvals, network restrictions, working directory,
   shell, date and roots are rendered. Only the applicable permission variant
   is selected, not every template.

D. Repository and global AGENTS.md instructions.
   Sources: core/src/context/world_state/agents_md.rs,
   core/src/context/user_instructions.rs and project documentation loading.
   These are conditional on discoverable instruction files. They can add much
   more text than the compiled base policy. The sandbox capture workspace has
   no project AGENTS.md. Directory-specific instructions in this repository
   are not all blindly concatenated for every task.

E. Active collaboration mode, persistence and multi-agent context.
   Sources: collaboration-mode-templates/templates/default.md and plan.md;
   prompts/templates/persistent_mode.md;
   prompts/src/model_messages/multi_agent.rs; core/src/session/world_state.rs.
   Default and Plan are alternatives. Persistence, role/roster/model-choice
   hints and delegation policy depend on feature flags and session state.
   The TUI selects a mode; a bare app-server request need not carry that same
   mode mask. default-mode-request.* explicitly includes the bundled Default
   instructions. Do not compare its size to an interactive session as though
   all settings/history were identical.

F. Skills catalog and skill instructions.
   Sources: ext/skills/src/catalog_prompt.rs, render.rs, fragments.rs,
   extension.rs and world_state_catalogs.rs.
   Enabled skills can contribute names, descriptions and source paths BEFORE
   invocation. A common how-to-use block is model-metadata-dependent; public
   provider fallback metadata currently leaves that extra usage block off.
   Explicitly selected skills can have their body injected as a <skill> user
   fragment. A model may also read a relevant SKILL.md via tools, making its
   content part of later context. References/scripts/templates are loaded
   separately when needed. They are not all eagerly concatenated.

G. Plugins, apps and MCP tools.
   Sources: core/src/context/available_plugins_instructions.rs,
   apps_instructions.rs, recommended_plugins_instructions.rs;
   core/src/plugins/injection.rs and render.rs; core/src/tools/spec_plan.rs.
   A plugin is a bundle, not one uniform prompt. Its enabled skills join the
   skills catalog. Generic integration guidance is conditional on availability
   and model flags. Explicit plugin mentions add a capability hint bounded to
   4 KiB per rendered hint; this is not an automatic dump of every skill body.
   Optional recommendations may list up to 50 not-installed plugins.
   Available MCP/app tool schemas can be model-visible before any explicit
   invocation. Discovery-capable models may receive deferred namespace metadata
   and load schemas on demand. Public-provider model descriptors currently have
   supports_search_tool=false and tool_mode=Direct, so registered tools generally
   need direct exposure; do NOT assume OpenAI-style lazy tool discovery applies
   to Ollama/DeepSeek. Disabled/denied tools still pass their normal policy gates.
   There were no installed plugins or MCP servers in the isolated capture home.

H. Tool definitions and output contracts.
   Sources: core/src/tools/spec_plan.rs and tools/handlers; extension tool specs;
   core/src/session/turn.rs and core/src/client.rs.
   Function names, descriptions and JSON schemas count as model context even
   though they are structurally separate from ordinary system-message prose.
   These include terminal tools, questions, viewing images, optional delegation,
   goals and web.run. Tool/tool-result and structured-output contracts vary with
   capabilities/settings. Native-provider namespace aliases are restored by
   client_public_history.rs; aliases do not eliminate schema/context cost.

I. Conditional runtime context and specialist model tasks.
   Sources: core/src/context and world_state; ext/goal/templates;
   ext/memories/templates; memories/write/templates; prompts/templates.
   Memory-read guidance, token/window budget hints, changed instructions,
   hooks' additional context, notices and queued agent messages can be injected
   when their conditions occur. Review, compaction, security/approval review,
   memory extraction/consolidation and realtime have separate specialized
   prompts; they are not all prepended to every ordinary engineering request.
   An extension can contribute additional per-thread/per-turn/world-state text.

Actual payload roles and assembly
--------------------------------
This fork normally constructs a base-instruction prefix plus contextual message
items and a separate tools array. For ordinary public-provider requests, the
client converts developer roles to system roles for provider compatibility.
The capture contains system engineering policy; system skills/permissions/mode
fragments; user-role environment context; then the actual user fixture. Tools
are supplied separately. AGENTS.md and selected-skill fragments use user-role
messages in the internal representation, although they still provide behavioral
context. 'Everything before my prompt' is broader than only role=system.

Measured sizes (UTF-8 bytes; not provider tokenizer counts)
---------------------------------------------------------
Default-mode local capture:
  Engineering policy                9,290
  Available-skills catalog           3,176  (6 entries, no full bodies)
  Permissions                           341
  Default-mode instructions          2,005
  Environment                           481
  13 tool definitions               18,668  (compact JSON)
  Combined                          33,961  (~34 KB)

This excludes transport metadata, framing and the 45-byte user-message fixture.
Any bytes/4 token estimates in metrics are a rough size proxy only; they are
NOT measured Ollama, DeepSeek or OpenAI token usage. HTTP mocks' response usage
fields are fixtures and are not token measurements.

The minimal bare-app-server example omits the explicit Default-mode block:
  Context text + tool schemas         31,956 bytes

A second isolated home added 100 synthetic skills, each with a long body and a
unique body marker. Catalog text increased from 3,176 to 8,494 bytes, an increase
of 5,318 bytes. All 100 synthetic names appeared, their descriptions were
shortened to fit the catalog budget, and NONE of their bodies/unique body markers
appeared in the initial request. Existing bundled and ordinary global host skills
were still eligible; the profile means 100 ADDITIONAL synthetic skills.

A third scratch configuration disabled unused delegation and goal tools:
  Tools:                            13 -> 5
  Tool-schema bytes:            18,668 -> 5,787
  Savings:                         12,881 bytes (~69% of tool-schema bytes)
The remaining tools were terminal execution, terminal-session input, user
questions, image viewing and web.run. This changed only the isolated test profile,
not the user's configuration or compiled defaults. A selected Default-mode block
would add the same mode text independently of this tool-count comparison.

Where the size protection works, and where it does not
------------------------------------------------------
- Skills catalog metadata normally targets 2% of known context size; an unknown
  context size uses an 8,000-character fallback. An explicitly configured
  skills.max_context_tokens is clamped to 10,000 by the renderer. Introductory
  text, wrappers and optional usage instructions are additional overhead; the
  budget is not a whole-session prompt cap.
- Explicitly auto-injected skill main bodies are truncated to 8,000 bytes each.
  A model reading SKILL.md or reference files through a tool is not bounded by
  the catalog budget; normal tool-result/history handling applies instead.
- Deferred namespace context is bounded to 4 KiB when that feature applies.
  This does not bound all eager tool schemas or all installed MCP capabilities.
- World-state diffs try not to append unchanged guidance again. Existing guidance,
  loaded skill/tool text and tool results can still remain in retained history.
  Prefix caching does not make that logical context disappear.

Practical reductions to consider
--------------------------------
Preserve the main engineering policy, permission facts and applicable project
instructions. The largest demonstrated avoidable cost here is unused tools.
If you do not use delegation or explicit persistent goals, this configuration
is the tested tool-removal combination (not applied to your settings):

  [features]
  multi_agent = false
  multi_agent_v2 = false
  goals = false

For a larger skill installation, a catalog-only budget can be set separately:

  [skills]
  max_context_tokens = 1500

Lower budgets shorten descriptions and may omit some entries. Disable unused
individual skills/plugins/MCP servers to remove their advertised capabilities.
skills.include_instructions=false hides automatic catalog instructions; it is
not a universal off switch for explicit skills, tool reads or plugin tools.
Do not simply mark a public provider as supporting hosted OpenAI tool_search;
a compatible client-side discovery implementation would be needed to keep those
providers functional while lazily exposing arbitrary tools.

How to inspect later
--------------------
The binary has `codex debug prompt-input`, which renders contextual input JSON
without a user inference turn. Source: cli/src/main.rs run_debug_prompt_input_command
and core/src/prompt_debug.rs. It is useful but NOT a complete outbound request
dump: the non-incremental base prefix and tools are assembled later by client.rs,
and that debug command uses a smaller extension registry than app-server. Startup
may still discover environments/skills or refresh metadata; it is not a universal
zero-network command. Use a controlled local request capture for exact full wire
inspection. Captures in this directory used that full app-server route.

Official documentation cross-check
-----------------------------------
https://developers.openai.com/plugins/concepts/skills
https://learn.chatgpt.com/docs/build-skills
Official documentation describes metadata-first skill discovery and loading the
body when relevant/invoked. The budget details, eager-tool behavior and exact byte
measurements above come from this fork's current source and local captures.

No production Rust source, built-in prompts, installed integrations, credentials
or user configuration were changed by this audit. Only this report/export folder
was added. These exported files are documentation and are not prompt-loading
inputs. Synthetic workspaces and mock processes were cleaned up.
