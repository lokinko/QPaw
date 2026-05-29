# Codex CLI LLM Provider Design

## Goal

Allow QPaw to use the user's existing Codex CLI login as an LLM provider, while
keeping the current OpenAI-compatible HTTP provider available.

## Provider Model

Add `llm.provider` with two values:

- `openai_compatible`: current behavior, using `base_url`, `api_key`, and
  `model` to call `/chat/completions`.
- `codex_cli`: local Codex CLI execution through the authenticated `codex`
  command.

The default local/fallback configuration should switch to `codex_cli`. Existing
saved settings that do not have `provider` should also deserialize into Codex
CLI mode unless the user explicitly selects OpenAI-compatible mode.

## Codex CLI Behavior

QPaw should call Codex non-interactively:

```powershell
codex exec --ephemeral --skip-git-repo-check --sandbox read-only -
```

The prompt is passed through stdin. If the settings model is non-empty, append
`-m <model>`; otherwise let Codex use its configured default model.

The stdin prompt should include the QPaw system prompt and the user message, and
should ask Codex to return only QPaw's assistant reply.

## Safety Boundary

Codex CLI mode must be read-only by default:

- `--sandbox read-only`
- `--ephemeral`
- `--skip-git-repo-check`

This keeps QPaw chat from writing files, mutating the repository, or prompting
for write access during normal conversation.

## Connectivity Test

Add a backend command `test_llm_connection`.

For OpenAI-compatible mode, send a short HTTP chat completion request and require
a non-empty response.

For Codex CLI mode, run a short prompt that asks Codex to reply with `OK`, then
require non-empty stdout. The command should return a structured status payload
with provider, success, message, and optional detail.

## Settings UI

In the LLM service settings card:

- Add a provider selector.
- Add a "Use Codex CLI" button that switches provider to `codex_cli`, clears the
  API key, and leaves model editable.
- Add a "Test Connection" button that calls `test_llm_connection` and displays
  the result.

When provider is `codex_cli`, Base URL and API Key are not required. The UI can
keep the fields visible for OpenAI-compatible use, but status readiness should
treat Codex CLI mode as configured when Codex CLI is available.

## Out Of Scope

- Streaming Codex output.
- Codex app-server protocol integration.
- Reading or parsing Codex auth file contents.
- Allowing write-capable Codex sandbox modes from QPaw chat.
