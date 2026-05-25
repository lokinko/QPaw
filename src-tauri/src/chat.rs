use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::debug;
use crate::error::QPawResult;
use crate::models::{
    ChatMessage, ChatRole, InteractionEventKind, MemoryDocument, SendChatResponse,
};
use crate::AppState;

pub async fn send_chat_message(
    message: String,
    state: &AppState,
) -> QPawResult<SendChatResponse> {
    let clean = message.trim().to_string();
    let trace_id = Uuid::new_v4().to_string();
    debug::log(
        "chat:send",
        format!(
            "trace_id={trace_id} start message_len={}",
            clean.chars().count()
        ),
    );
    let user = ChatMessage {
        role: ChatRole::User,
        content: clean.clone(),
        created_at: Utc::now(),
    };
    if let Err(error) = state.store.append_chat(&user).await {
        debug::err(
            "chat:send",
            format!("trace_id={trace_id} failed to persist user chat message: {error}"),
        );
    } else {
        debug::log(
            "chat:send",
            format!("trace_id={trace_id} user chat persisted"),
        );
    }
    let user_event = match state
        .memory
        .record_event(
            InteractionEventKind::ChatMessage,
            "user",
            clean.clone(),
            json!({ "role": "user", "content": clean.clone() }),
            vec!["chat".to_string()],
        )
        .await
    {
        Ok(event) => {
            debug::log(
                "chat:send",
                format!("trace_id={trace_id} user interaction event persisted"),
            );
            Some(event)
        }
        Err(error) => {
            debug::err(
                "chat:send",
                format!("trace_id={trace_id} failed to persist user interaction event: {error}"),
            );
            None
        }
    };

    let settings = state.store.get_settings().await.unwrap_or_default();
    if settings.memory.working_memory_enabled {
        if let Some(user_event) = user_event.as_ref() {
            match state
                .memory
                .update_working_memory_from_user_event(
                    user_event,
                    settings.memory.working_memory_retention_hours,
                )
                .await
            {
                Ok(items) => debug::log(
                    "chat:send",
                    format!(
                        "trace_id={trace_id} working_memory_updated count={}",
                        items.len()
                    ),
                ),
                Err(error) => debug::err(
                    "chat:send",
                    format!("trace_id={trace_id} working_memory_update_failed: {error}"),
                ),
            }
        }
    }
    let mut memories = state.store.list_memories().await.unwrap_or_default();
    debug::log(
        "chat:send",
        format!(
            "trace_id={trace_id} settings_loaded llm_configured={} legacy_memories={}",
            !settings.llm.api_key.trim().is_empty() && !settings.llm.model.trim().is_empty(),
            memories.len()
        ),
    );
    let layered_context = match state.memory.context_for_chat(&clean).await {
        Ok(context) => {
            debug::log(
                "chat:send",
                format!(
                    "trace_id={trace_id} layered_context_loaded context_len={}",
                    context.len()
                ),
            );
            context
        }
        Err(error) => {
            debug::err(
                "chat:send",
                format!("trace_id={trace_id} layered_context_failed: {error}"),
            );
            String::new()
        }
    };
    let system = format!(
        "You are QPaw, a calm low-interruption desktop pet. Keep replies concise. \
         Privacy mode is minimal: never claim access to window titles, app names, or keystrokes.\n\
         Layered local memory context:\n{}",
        layered_context
    );
    let assistant_content = match state
        .llm
        .reply_with_context(&settings, &clean, &system)
        .await
    {
        Ok(reply) => {
            debug::log(
                "chat:send",
                format!(
                    "trace_id={trace_id} llm_reply_ok reply_len={}",
                    reply.chars().count()
                ),
            );
            reply
        }
        Err(error) => {
            debug::err(
                "chat:send",
                format!("trace_id={trace_id} llm_reply_failed: {error}"),
            );
            format!("我暂时连不上 LLM，但已经在本地记下了。错误：{error}")
        }
    };
    let assistant = ChatMessage {
        role: ChatRole::Assistant,
        content: assistant_content,
        created_at: Utc::now(),
    };
    if let Err(error) = state.store.append_chat(&assistant).await {
        debug::err(
            "chat:send",
            format!("trace_id={trace_id} failed to persist assistant chat message: {error}"),
        );
    } else {
        debug::log(
            "chat:send",
            format!("trace_id={trace_id} assistant chat persisted"),
        );
    }
    if let Err(error) = state
        .memory
        .record_event(
            InteractionEventKind::ChatMessage,
            "assistant",
            assistant.content.clone(),
            json!({ "role": "assistant", "content": assistant.content.clone() }),
            vec!["chat".to_string()],
        )
        .await
    {
        debug::err(
            "chat:send",
            format!("trace_id={trace_id} failed to persist assistant interaction event: {error}"),
        );
    } else {
        debug::log(
            "chat:send",
            format!("trace_id={trace_id} assistant interaction event persisted"),
        );
    }

    if should_store_legacy_memory(&clean) {
        let memory = MemoryDocument {
            body: clean.clone(),
            source: "chat".to_string(),
            created_at: Utc::now(),
        };
        if let Err(error) = state.store.append_memory(&memory).await {
            debug::err(
                "chat:send",
                format!("trace_id={trace_id} failed to persist legacy memory: {error}"),
            );
        } else {
            debug::log(
                "chat:send",
                format!("trace_id={trace_id} legacy memory persisted"),
            );
        }
        memories.push(memory);
    }

    debug::log(
        "chat:send",
        format!(
            "trace_id={trace_id} done returned_memories={}",
            memories.len()
        ),
    );
    Ok(SendChatResponse {
        user,
        assistant,
        memories,
    })
}

pub fn should_store_legacy_memory(message: &str) -> bool {
    let lowered = message.to_lowercase();
    message.contains("记住")
        || message.contains("记得")
        || message.contains("记一下")
        || lowered.contains("remember")
        || lowered.contains("note that")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_memory_policy_detects_explicit_memory_requests() {
        assert!(should_store_legacy_memory("请记住我下午容易忘记喝水"));
        assert!(should_store_legacy_memory("帮我记一下我喜欢安静提醒"));
        assert!(should_store_legacy_memory("remember that I prefer concise replies"));
        assert!(should_store_legacy_memory("note that I work best in the morning"));
        assert!(!should_store_legacy_memory("今天只是随便聊聊"));
    }
}
