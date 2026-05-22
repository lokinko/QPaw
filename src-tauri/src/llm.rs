use serde::{Deserialize, Serialize};

use crate::debug;
use crate::error::{QPawError, QPawResult};
use crate::models::{AppSettings, ChatMessage, ChatRole, MemoryDocument};

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
    pub async fn reply(
        &self,
        settings: &AppSettings,
        message: &str,
        memories: &[MemoryDocument],
    ) -> QPawResult<String> {
        if settings.llm.api_key.trim().is_empty() || settings.llm.model.trim().is_empty() {
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
        if settings.llm.api_key.trim().is_empty() || settings.llm.model.trim().is_empty() {
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
        if settings.llm.api_key.trim().is_empty() || settings.llm.model.trim().is_empty() {
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
