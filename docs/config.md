# Configuration

This custom build uses the [engineering policy](../prompts/README.md) as
its built-in default across projects. Explicit prompt-file and custom model
catalog overrides remain available.

For basic configuration instructions, see [this documentation](https://developers.openai.com/codex/config-basic).

For advanced configuration instructions, see [this documentation](https://developers.openai.com/codex/config-advanced).

For a full configuration reference, see [this documentation](https://developers.openai.com/codex/config-reference).

## Browsing with Ollama Cloud and DeepSeek

Launch this fork with `--search` to enable the native `web.run` tool for either
provider. To keep it enabled across launches, set this at the top level of your
`config.toml`:

```toml
web_search = "live"
```

Search and page reads use the selected provider's existing login or API key;
they do not require ChatGPT login or an MCP server. Ollama Cloud uses its
[search/fetch APIs](https://docs.ollama.com/capabilities/web-search), and DeepSeek
uses its [search/fetch APIs](https://api-docs.deepseek.com/guides/search/).
The tool supports searching, opening HTTP(S) pages and finding text within them.
Results include source URLs for citations. Images, PDF screenshots and specialized
finance/weather commands are not exposed by these adapters.

Native services perform live requests and may consume provider quotas or charges.
Cached-only and indexed-only modes do not expose these adapters. Configured domain
or location filters currently produce an explicit unsupported-settings error;
they are never silently ignored. Provider capability opt-outs remain effective.

## Lifecycle hooks

Admins can set top-level `allow_managed_hooks_only = true` in
`requirements.toml` to ignore user, project, and session hook configs while
still allowing managed hooks from requirements and managed config layers. This
setting is only supported in `requirements.toml`; putting it in `config.toml`
does not enable managed-hooks-only mode.
