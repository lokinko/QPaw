use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use uuid::Uuid;

use crate::debug;
use crate::error::{QPawError, QPawResult};
use crate::models::{
    AppSettings, ChatMessage, ChatRole, LlmConnectionTestResult, LlmProvider, MemoryDocument,
};

const CODEX_EXEC_TIMEOUT: Duration = Duration::from_secs(90);
const CODEX_CONNECTIVITY_TEST_TIMEOUT: Duration = Duration::from_secs(60);
const OPENAI_COMPATIBLE_CONNECTIVITY_TEST_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Default)]
pub struct LlmClient {
    client: reqwest::Client,
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest<'a> {
    model: &'a str,
    messages: Vec<LlmMessage<'a>>,
    temperature: f32,
}

#[derive(Debug, Serialize)]
struct LlmMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

#[derive(Debug, Deserialize)]
struct ChoiceMessage {
    content: String,
}

impl LlmClient {
    pub async fn test_connection(&self, settings: &AppSettings) -> LlmConnectionTestResult {
        let provider = settings.llm.provider.clone();
        let result = match settings.llm.provider {
            LlmProvider::CodexCli => {
                let model = settings.llm.model.trim();
                let prompt = codex_exec_prompt(
                    "You are a connectivity probe. Reply with exactly: QPaw connection OK",
                    "Test the current LLM connection.",
                );
                run_codex_exec_with_timeout(prompt, model, CODEX_CONNECTIVITY_TEST_TIMEOUT).await
            }
            LlmProvider::OpenAiCompatible => {
                if !settings.has_openai_compatible_llm_config() {
                    Err(QPawError::Message(
                        "OpenAI-compatible provider requires Base URL, Model, and API Key"
                            .to_string(),
                    ))
                } else {
                    match tokio::time::timeout(
                        OPENAI_COMPATIBLE_CONNECTIVITY_TEST_TIMEOUT,
                        self.reply_with_context(
                            settings,
                            "Test the current LLM connection. Reply with exactly: QPaw connection OK",
                            "You are a connectivity probe.",
                        ),
                    )
                    .await
                    {
                        Ok(result) => result,
                        Err(_) => Err(QPawError::Message(llm_connectivity_timeout_message(
                            OPENAI_COMPATIBLE_CONNECTIVITY_TEST_TIMEOUT,
                        ))),
                    }
                }
            }
        };

        match result {
            Ok(content) => LlmConnectionTestResult {
                provider,
                success: true,
                message: "LLM connectivity test succeeded".to_string(),
                detail: Some(content.chars().take(240).collect()),
            },
            Err(error) => LlmConnectionTestResult {
                provider,
                success: false,
                message: "LLM connectivity test failed".to_string(),
                detail: Some(error.to_string()),
            },
        }
    }

    pub async fn reply(
        &self,
        settings: &AppSettings,
        message: &str,
        memories: &[MemoryDocument],
    ) -> QPawResult<String> {
        if matches!(settings.llm.provider, LlmProvider::OpenAiCompatible)
            && !settings.has_openai_compatible_llm_config()
        {
            debug::log("llm:reply", "llm not configured; using local fallback");
            return Ok("我会先用本地模式记住这件事。配置 LLM 后，我可以更自然地回应。".to_string());
        }

        let memory_context = memories
            .iter()
            .rev()
            .take(6)
            .map(|memory| memory.body.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        let system = format!(
            "You are QPaw, a calm low-interruption desktop pet. Keep replies concise. \
             Privacy mode is minimal: never claim access to window titles, app names, or keystrokes. \
             Local memory snippets:\n{}",
            memory_context
        );

        self.reply_with_context(settings, message, &system).await
    }

    pub async fn reply_with_context(
        &self,
        settings: &AppSettings,
        message: &str,
        system: &str,
    ) -> QPawResult<String> {
        match settings.llm.provider {
            LlmProvider::CodexCli => {
                let model = settings.llm.model.trim();
                let prompt = codex_exec_prompt(system, message);
                debug::log(
                    "llm:reply_with_context",
                    format!(
                        "codex exec request model={} user_len={} system_len={}",
                        if model.is_empty() { "<default>" } else { model },
                        message.chars().count(),
                        system.chars().count()
                    ),
                );
                let content = run_codex_exec(prompt, model).await?;
                debug::log(
                    "llm:reply_with_context",
                    format!("codex exec content_len={}", content.chars().count()),
                );
                return Ok(content);
            }
            LlmProvider::OpenAiCompatible => {}
        }

        if !settings.has_openai_compatible_llm_config() {
            debug::log(
                "llm:reply_with_context",
                "llm not configured; using local fallback",
            );
            return Ok("我会先用本地模式记住这件事。配置 LLM 后，我可以更自然地回应。".to_string());
        }

        let body = ChatCompletionRequest {
            model: settings.llm.model.trim(),
            messages: vec![
                LlmMessage {
                    role: "system",
                    content: &system,
                },
                LlmMessage {
                    role: "user",
                    content: message,
                },
            ],
            temperature: 0.4,
        };

        let url = format!(
            "{}/chat/completions",
            settings.llm.base_url.trim().trim_end_matches('/')
        );
        debug::log(
            "llm:reply_with_context",
            format!(
                "request url={} model={} user_len={} system_len={}",
                url,
                settings.llm.model.trim(),
                message.chars().count(),
                system.chars().count()
            ),
        );
        let response = self
            .client
            .post(url)
            .bearer_auth(settings.llm.api_key.trim())
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        debug::log(
            "llm:reply_with_context",
            format!("response status={status}"),
        );
        if !response.status().is_success() {
            return Err(QPawError::Message(format!(
                "LLM request failed with status {}",
                status
            )));
        }

        let response: ChatCompletionResponse = response.json().await?;
        let content = response
            .choices
            .into_iter()
            .next()
            .map(|choice| choice.message.content)
            .filter(|content| !content.trim().is_empty())
            .ok_or_else(|| QPawError::Message("LLM response was empty".to_string()))?;
        debug::log(
            "llm:reply_with_context",
            format!("response parsed content_len={}", content.chars().count()),
        );
        Ok(content)
    }

    pub async fn complete_json(
        &self,
        settings: &AppSettings,
        system: &str,
        user: &str,
    ) -> QPawResult<String> {
        if matches!(settings.llm.provider, LlmProvider::CodexCli) {
            let model = settings.llm.model.trim();
            let prompt = codex_exec_prompt(
                system,
                &format!("{user}\n\nReturn strict JSON only. Do not include markdown fences."),
            );
            debug::log(
                "llm:complete_json",
                format!(
                    "codex exec request model={} user_len={} system_len={}",
                    if model.is_empty() { "<default>" } else { model },
                    user.chars().count(),
                    system.chars().count()
                ),
            );
            return run_codex_exec(prompt, model).await;
        }

        if !settings.has_openai_compatible_llm_config() {
            debug::log("llm:complete_json", "llm not configured");
            return Err(QPawError::Message("LLM is not configured".to_string()));
        }

        let body = ChatCompletionRequest {
            model: settings.llm.model.trim(),
            messages: vec![
                LlmMessage {
                    role: "system",
                    content: system,
                },
                LlmMessage {
                    role: "user",
                    content: user,
                },
            ],
            temperature: 0.2,
        };

        let url = format!(
            "{}/chat/completions",
            settings.llm.base_url.trim().trim_end_matches('/')
        );
        debug::log(
            "llm:complete_json",
            format!(
                "request url={} model={} user_len={} system_len={}",
                url,
                settings.llm.model.trim(),
                user.chars().count(),
                system.chars().count()
            ),
        );
        let response = self
            .client
            .post(url)
            .bearer_auth(settings.llm.api_key.trim())
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        debug::log("llm:complete_json", format!("response status={status}"));
        if !response.status().is_success() {
            return Err(QPawError::Message(format!(
                "LLM request failed with status {}",
                status
            )));
        }

        let response: ChatCompletionResponse = response.json().await?;
        let content = response
            .choices
            .into_iter()
            .next()
            .map(|choice| choice.message.content)
            .filter(|content| !content.trim().is_empty())
            .ok_or_else(|| QPawError::Message("LLM response was empty".to_string()))?;
        debug::log(
            "llm:complete_json",
            format!("response parsed content_len={}", content.chars().count()),
        );
        Ok(content)
    }

    pub fn fallback_assistant_message(content: String) -> ChatMessage {
        ChatMessage {
            role: ChatRole::Assistant,
            content,
            created_at: chrono::Utc::now(),
        }
    }
}

impl AppSettings {
    fn has_openai_compatible_llm_config(&self) -> bool {
        !self.llm.base_url.trim().is_empty()
            && !self.llm.api_key.trim().is_empty()
            && !self.llm.model.trim().is_empty()
    }
}

fn codex_exec_prompt(system: &str, user: &str) -> String {
    format!("System instructions:\n{system}\n\nUser message:\n{user}\n\nRespond as QPaw.")
}

async fn run_codex_exec(prompt: String, model: &str) -> QPawResult<String> {
    run_codex_exec_with_timeout(prompt, model, CODEX_EXEC_TIMEOUT).await
}

async fn run_codex_exec_with_timeout(
    prompt: String,
    model: &str,
    timeout: Duration,
) -> QPawResult<String> {
    let model = model.to_string();
    tokio::task::spawn_blocking(move || run_codex_exec_blocking(&prompt, &model, timeout))
        .await
        .map_err(|error| QPawError::Message(format!("Codex CLI task failed: {error}")))?
}

fn run_codex_exec_blocking(prompt: &str, model: &str, timeout: Duration) -> QPawResult<String> {
    let output_path =
        std::env::temp_dir().join(format!("qpaw-codex-last-message-{}.txt", Uuid::new_v4()));
    let mut args = vec![
        "exec".to_string(),
        "--ephemeral".to_string(),
        "--skip-git-repo-check".to_string(),
        "--sandbox".to_string(),
        "read-only".to_string(),
        "--color".to_string(),
        "never".to_string(),
        "--output-last-message".to_string(),
        output_path.to_string_lossy().to_string(),
    ];
    if !model.trim().is_empty() {
        args.push("--model".to_string());
        args.push(model.trim().to_string());
    }
    args.push("-".to_string());

    let mut child = Command::new(codex_command_name())
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(prompt.as_bytes())?;
    }

    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }

        if start.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            let _ = fs::remove_file(&output_path);
            return Err(QPawError::Message(codex_timeout_message(timeout)));
        }

        thread::sleep(Duration::from_millis(100));
    };

    let mut stdout = String::new();
    if let Some(mut handle) = child.stdout.take() {
        handle.read_to_string(&mut stdout)?;
    }

    let mut stderr = String::new();
    if let Some(mut handle) = child.stderr.take() {
        handle.read_to_string(&mut stderr)?;
    }

    let last_message = fs::read_to_string(&output_path).unwrap_or_default();
    let _ = fs::remove_file(&output_path);

    if !status.success() {
        let detail = stderr
            .trim()
            .lines()
            .last()
            .or_else(|| stdout.trim().lines().last())
            .unwrap_or("Codex CLI exited with a non-zero status");
        return Err(QPawError::Message(format!(
            "Codex CLI request failed: {detail}"
        )));
    }

    let content = last_message.trim();
    if content.is_empty() {
        let detail = summarize_codex_output(&stdout, &stderr)
            .unwrap_or_else(|| "Codex CLI response was empty".to_string());
        return Err(QPawError::Message(format!(
            "Codex CLI response was empty: {detail}"
        )));
    }

    Ok(content.to_string())
}

fn codex_timeout_message(timeout: Duration) -> String {
    format!(
        "Codex CLI connectivity timed out after {} seconds",
        timeout.as_secs()
    )
}

fn llm_connectivity_timeout_message(timeout: Duration) -> String {
    format!(
        "LLM connectivity test timed out after {} seconds",
        timeout.as_secs()
    )
}

#[cfg(target_os = "windows")]
fn codex_command_name() -> &'static str {
    "codex.cmd"
}

#[cfg(not(target_os = "windows"))]
fn codex_command_name() -> &'static str {
    "codex"
}

fn summarize_codex_output(stdout: &str, stderr: &str) -> Option<String> {
    stderr
        .trim()
        .lines()
        .rev()
        .chain(stdout.trim().lines().rev())
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(ToString::to_string)
}

#[cfg(test)]
mod tests {
    use super::{codex_command_name, codex_exec_prompt, summarize_codex_output};

    #[test]
    fn codex_exec_prompt_contains_system_and_user_message() {
        let prompt = codex_exec_prompt("keep it short", "hello");

        assert!(prompt.contains("System instructions:\nkeep it short"));
        assert!(prompt.contains("User message:\nhello"));
        assert!(prompt.contains("Respond as QPaw."));
    }

    #[test]
    fn codex_command_uses_windows_cmd_shim_on_windows() {
        #[cfg(target_os = "windows")]
        assert_eq!(codex_command_name(), "codex.cmd");

        #[cfg(not(target_os = "windows"))]
        assert_eq!(codex_command_name(), "codex");
    }

    #[test]
    fn summarize_codex_output_prefers_last_non_empty_error_line() {
        let detail =
            summarize_codex_output("stdout ok", "first\nusage limit\n").expect("summarize output");

        assert_eq!(detail, "usage limit");
    }
}

#[cfg(test)]
#[path = "llm_timeout_tests.rs"]
mod llm_timeout_tests;
