# Codex CLI LLM Provider Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Codex CLI LLM provider, switch default settings to it, and expose a one-click connection test in settings.

**Architecture:** Extend `LlmConfig` with a provider enum, keep OpenAI-compatible HTTP code intact, and add a Codex CLI execution path inside `LlmClient`. Add a Tauri command for connectivity testing and wire SettingsWindow controls to provider selection and test status.

**Tech Stack:** Rust, Tauri commands, `tokio::process`, Serde, React, TypeScript, Vitest, Cargo tests.

---

## File Structure

- Modify `src-tauri/src/models.rs`: add `LlmProvider` and `LlmConnectionTestResult`.
- Modify `src-tauri/src/llm.rs`: route by provider, build Codex prompts, execute `codex exec`, and test connectivity.
- Modify `src-tauri/src/commands.rs`: add `test_llm_connection`.
- Modify `src-tauri/src/lib.rs`: register the new command.
- Modify `src/lib/types.ts`: mirror provider and connectivity result types.
- Modify `src/lib/tauri.ts`: expose `api.testLlmConnection`.
- Modify `src/lib/fallback.ts`: default to `codex_cli`, provide fallback connection test.
- Modify `src/components/SettingsWindow.tsx`: add provider selector, use-Codex button, and test button.
- Modify `src/components/SettingsWindow.test.tsx`: assert the provider controls render.

### Task 1: Add Provider Settings Types

**Files:**
- Modify: `src-tauri/src/models.rs`
- Modify: `src/lib/types.ts`
- Modify: `src/lib/fallback.ts`

- [ ] **Step 1: Add failing Rust defaults test**

In `src-tauri/src/models.rs`, update the test import:

```rust
use super::{AppSettings, AvatarKind, FullscreenBehavior, LlmProvider, MemorySensitivity};
```

Add:

```rust
#[test]
fn default_llm_provider_uses_codex_cli_without_api_key() {
    let settings = AppSettings::default();

    assert_eq!(settings.llm.provider, LlmProvider::CodexCli);
    assert!(settings.llm.api_key.is_empty());
    assert!(settings.llm.model.is_empty());
}
```

- [ ] **Step 2: Run test and verify failure**

Run:

```powershell
cd src-tauri; cargo test models::tests::default_llm_provider_uses_codex_cli_without_api_key
```

Expected: FAIL because `LlmProvider` and `llm.provider` do not exist.

- [ ] **Step 3: Implement provider type and defaults**

In `src-tauri/src/models.rs`, add before `LlmConfig`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlmProvider {
    OpenAiCompatible,
    CodexCli,
}

fn default_llm_provider() -> LlmProvider {
    LlmProvider::CodexCli
}
```

Add to `LlmConfig`:

```rust
#[serde(default = "default_llm_provider")]
pub provider: LlmProvider,
```

Change `LlmConfig::default()` to:

```rust
Self {
    provider: default_llm_provider(),
    base_url: "https://api.openai.com/v1".to_string(),
    api_key: String::new(),
    model: String::new(),
}
```

- [ ] **Step 4: Run Rust defaults test**

Run:

```powershell
cd src-tauri; cargo test models::tests::default_llm_provider_uses_codex_cli_without_api_key
```

Expected: PASS.

- [ ] **Step 5: Mirror TypeScript types and fallback default**

In `src/lib/types.ts`, add:

```ts
export type LlmProvider = "open_ai_compatible" | "codex_cli";
```

Add to `LlmConfig`:

```ts
provider: LlmProvider;
```

In `src/lib/fallback.ts`, set:

```ts
llm: {
  provider: "codex_cli",
  base_url: "https://api.openai.com/v1",
  api_key: "",
  model: "",
},
```

- [ ] **Step 6: Run frontend build**

Run:

```powershell
npm run build
```

Expected: PASS.

### Task 2: Add Codex CLI LLM Client Path

**Files:**
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/llm.rs`

- [ ] **Step 1: Add failing prompt-builder test**

In `src-tauri/src/llm.rs`, add:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codex_prompt_contains_system_and_user_message() {
        let prompt = codex_exec_prompt("system rules", "hello");

        assert!(prompt.contains("system rules"));
        assert!(prompt.contains("hello"));
        assert!(prompt.contains("Return only QPaw's assistant reply"));
    }
}
```

- [ ] **Step 2: Run test and verify failure**

Run:

```powershell
cd src-tauri; cargo test llm::tests::codex_prompt_contains_system_and_user_message
```

Expected: FAIL because `codex_exec_prompt` does not exist.

- [ ] **Step 3: Implement Codex prompt and route by provider**

In `src-tauri/src/llm.rs`, import:

```rust
use std::process::Stdio;

use tokio::io::AsyncWriteExt;
use tokio::process::Command;
```

Update models import:

```rust
use crate::models::{AppSettings, ChatMessage, ChatRole, LlmProvider, MemoryDocument};
```

At the start of `reply_with_context`, route:

```rust
if settings.llm.provider == LlmProvider::CodexCli {
    return self.reply_with_codex_cli(settings, message, system).await;
}
```

Add methods:

```rust
async fn reply_with_codex_cli(
    &self,
    settings: &AppSettings,
    message: &str,
    system: &str,
) -> QPawResult<String> {
    let prompt = codex_exec_prompt(system, message);
    run_codex_exec(settings.llm.model.trim(), &prompt).await
}
```

Add helper functions after `impl LlmClient`:

```rust
fn codex_exec_prompt(system: &str, message: &str) -> String {
    format!(
        "You are being used as QPaw's local LLM backend.\n\nSystem instructions:\n{system}\n\nUser message:\n{message}\n\nReturn only QPaw's assistant reply. Do not describe tool use, files, or implementation steps."
    )
}

async fn run_codex_exec(model: &str, prompt: &str) -> QPawResult<String> {
    let mut command = Command::new("codex");
    command
        .arg("exec")
        .arg("--ephemeral")
        .arg("--skip-git-repo-check")
        .arg("--sandbox")
        .arg("read-only");

    if !model.is_empty() {
        command.arg("-m").arg(model);
    }

    command.arg("-");
    command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = command.spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(prompt.as_bytes()).await?;
    }

    let output = child.wait_with_output().await?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(QPawError::Message(format!(
            "Codex CLI request failed{}",
            if stderr.is_empty() { String::new() } else { format!(": {stderr}") }
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        return Err(QPawError::Message("Codex CLI response was empty".to_string()));
    }
    Ok(stdout)
}
```

- [ ] **Step 4: Run prompt-builder test**

Run:

```powershell
cd src-tauri; cargo test llm::tests::codex_prompt_contains_system_and_user_message
```

Expected: PASS.

### Task 3: Add Connectivity Test Command

**Files:**
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/llm.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add result model and serialization test**

In `src-tauri/src/models.rs`, add:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConnectionTestResult {
    pub provider: LlmProvider,
    pub ok: bool,
    pub message: String,
    pub detail: Option<String>,
}
```

Add test:

```rust
#[test]
fn llm_connection_test_result_serializes_provider() {
    let value = super::LlmConnectionTestResult {
        provider: LlmProvider::CodexCli,
        ok: true,
        message: "Codex CLI connection OK".to_string(),
        detail: None,
    };

    let encoded = serde_json::to_value(value).expect("serialize result");

    assert_eq!(encoded["provider"], serde_json::json!("codex_cli"));
    assert_eq!(encoded["ok"], serde_json::json!(true));
}
```

- [ ] **Step 2: Run serialization test**

Run:

```powershell
cd src-tauri; cargo test models::tests::llm_connection_test_result_serializes_provider
```

Expected: PASS.

- [ ] **Step 3: Implement LlmClient::test_connection**

In `src-tauri/src/llm.rs`, import `LlmConnectionTestResult` and add:

```rust
pub async fn test_connection(&self, settings: &AppSettings) -> LlmConnectionTestResult {
    let provider = settings.llm.provider.clone();
    let result = match provider {
        LlmProvider::CodexCli => {
            run_codex_exec(settings.llm.model.trim(), "Reply with exactly OK.").await
        }
        LlmProvider::OpenAiCompatible => {
            self.reply_with_context(settings, "Reply with exactly OK.", "Connection test.").await
        }
    };

    match result {
        Ok(reply) => LlmConnectionTestResult {
            provider,
            ok: true,
            message: "LLM connection OK".to_string(),
            detail: Some(reply.chars().take(200).collect()),
        },
        Err(error) => LlmConnectionTestResult {
            provider,
            ok: false,
            message: "LLM connection failed".to_string(),
            detail: Some(error.to_string()),
        },
    }
}
```

- [ ] **Step 4: Add Tauri command and register it**

In `src-tauri/src/commands.rs`, import `LlmConnectionTestResult` and add:

```rust
#[tauri::command]
pub async fn test_llm_connection(
    settings: AppSettings,
    state: State<'_, AppState>,
) -> QPawResult<LlmConnectionTestResult> {
    debug::log("command:test_llm_connection", "testing llm connection");
    Ok(state.llm.test_connection(&settings).await)
}
```

In `src-tauri/src/lib.rs`, add `test_llm_connection` to imports and `.invoke_handler(...)`.

- [ ] **Step 5: Run backend tests**

Run:

```powershell
cd src-tauri; cargo test models::tests::llm_connection_test_result_serializes_provider llm::tests::codex_prompt_contains_system_and_user_message
```

Expected: Cargo accepts one filter at a time; if this command errors, run the two tests separately and confirm both pass.

### Task 4: Add Frontend Controls

**Files:**
- Modify: `src/lib/types.ts`
- Modify: `src/lib/tauri.ts`
- Modify: `src/lib/fallback.ts`
- Modify: `src/components/SettingsWindow.tsx`
- Modify: `src/components/SettingsWindow.test.tsx`

- [ ] **Step 1: Add failing SettingsWindow test assertions**

In `src/components/SettingsWindow.test.tsx`, add assertions:

```tsx
expect(markup).toContain("Provider");
expect(markup).toContain("Codex CLI");
expect(markup).toContain("测试连通性");
```

- [ ] **Step 2: Run test and verify failure**

Run:

```powershell
npm test -- src/components/SettingsWindow.test.tsx
```

Expected: FAIL because controls do not exist.

- [ ] **Step 3: Add frontend types and API**

In `src/lib/types.ts`, add:

```ts
export interface LlmConnectionTestResult {
  provider: LlmProvider;
  ok: boolean;
  message: string;
  detail: string | null;
}
```

In `src/lib/tauri.ts`, import the type and add:

```ts
testLlmConnection: (settings: AppSettings) =>
  call<LlmConnectionTestResult>("test_llm_connection", { settings }),
```

In `src/lib/fallback.ts`, add a `test_llm_connection` case returning OK for Codex CLI fallback.

- [ ] **Step 4: Add SettingsWindow controls**

In `SettingsWindow`, add status state:

```tsx
const [llmTestStatus, setLlmTestStatus] = useState<string | null>(null);
```

Add helper:

```tsx
const useCodexCliProvider = () =>
  save({
    ...settings,
    llm: {
      ...settings.llm,
      provider: "codex_cli",
      api_key: "",
    },
  });

const testLlmConnection = async () => {
  setLlmTestStatus("正在测试 LLM 连通性...");
  const result = await api.testLlmConnection(settings);
  setLlmTestStatus(result.ok ? `连通性正常：${result.message}` : `连通性失败：${result.detail ?? result.message}`);
};
```

In the LLM card, add provider select and buttons:

```tsx
<label>
  Provider
  <select
    value={settings.llm.provider}
    onChange={(event) =>
      save({
        ...settings,
        llm: {
          ...settings.llm,
          provider: event.target.value as AppSettings["llm"]["provider"],
        },
      })
    }
  >
    <option value="open_ai_compatible">OpenAI Compatible</option>
    <option value="codex_cli">Codex CLI</option>
  </select>
</label>
<div className="button-row">
  <ControlButton onClick={() => void useCodexCliProvider()}>使用 Codex CLI</ControlButton>
  <ControlButton variant="primary" onClick={() => void testLlmConnection()}>
    测试连通性
  </ControlButton>
</div>
{llmTestStatus ? <p className="status-line">{llmTestStatus}</p> : null}
```

- [ ] **Step 5: Run SettingsWindow test**

Run:

```powershell
npm test -- src/components/SettingsWindow.test.tsx
```

Expected: PASS.

### Task 5: Full Verification

**Files:**
- All modified files from Tasks 1-4.

- [ ] **Step 1: Run backend tests**

Run:

```powershell
cd src-tauri; cargo test
```

Expected: PASS.

- [ ] **Step 2: Run frontend tests**

Run:

```powershell
npm test
```

Expected: PASS.

- [ ] **Step 3: Run build**

Run:

```powershell
npm run build
```

Expected: PASS.

- [ ] **Step 4: Test real Codex CLI connectivity**

Run:

```powershell
"Reply with exactly OK." | codex exec --ephemeral --skip-git-repo-check --sandbox read-only -
```

Expected: command exits 0 and prints a non-empty response.
