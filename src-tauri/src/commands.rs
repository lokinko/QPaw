use chrono::{NaiveDate, Utc};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use crate::debug;
use crate::error::QPawResult;
use crate::models::{
    AppSettings, AvatarManifest, ChatMessage, ChatRole, InteractionEventKind, LayeredMemoryItem,
    MemoryConsolidationReport, MemoryDocument, MemoryLayer, MemoryLayerFilter, MemoryQueryRequest,
    MemoryQueryResponse, MemoryStats, ReminderEvent, ReminderFeedbackPayload, ReminderKind,
    ReminderPayload, ReminderRuntimeStatus, SendChatResponse, WorkingMemoryItem,
    PET_WINDOW_MAX_HEIGHT, PET_WINDOW_MAX_WIDTH, PET_WINDOW_MIN_HEIGHT, PET_WINDOW_MIN_WIDTH,
};
use crate::AppState;

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> QPawResult<AppSettings> {
    debug::log("command:get_settings", "loading settings");
    state.store.get_settings().await
}

#[tauri::command]
pub async fn save_settings(
    settings: AppSettings,
    state: State<'_, AppState>,
) -> QPawResult<AppSettings> {
    debug::log("command:save_settings", "saving settings");
    state.store.save_settings(&settings).await
}

#[tauri::command]
pub async fn save_pet_window_size(
    width: u32,
    height: u32,
    state: State<'_, AppState>,
) -> QPawResult<()> {
    let width = width.clamp(PET_WINDOW_MIN_WIDTH, PET_WINDOW_MAX_WIDTH);
    let height = height.clamp(PET_WINDOW_MIN_HEIGHT, PET_WINDOW_MAX_HEIGHT);

    debug::log(
        "command:save_pet_window_size",
        format!("width={width} height={height}"),
    );

    let mut settings = state.store.get_settings().await?;
    if settings.window.pet_width == Some(width) && settings.window.pet_height == Some(height) {
        return Ok(());
    }

    settings.window.pet_width = Some(width);
    settings.window.pet_height = Some(height);
    let _ = state.store.save_settings(&settings).await?;
    Ok(())
}

#[tauri::command]
pub async fn import_avatar(path: String, state: State<'_, AppState>) -> QPawResult<AvatarManifest> {
    debug::log(
        "command:import_avatar",
        format!("importing avatar path_len={}", path.len()),
    );
    let manifest = state.avatars.import_model(path.into())?;
    state.store.save_avatar(&manifest).await?;

    let mut settings = state.store.get_settings().await?;
    settings.avatar.current_avatar_id = Some(manifest.id.clone());
    settings.avatar.model_json_path = Some(manifest.model_json_path.clone());
    let _ = state.store.save_settings(&settings).await?;

    Ok(manifest)
}

#[tauri::command]
pub async fn send_chat_message(
    message: String,
    state: State<'_, AppState>,
) -> QPawResult<SendChatResponse> {
    let clean = message.trim().to_string();
    let trace_id = Uuid::new_v4().to_string();
    debug::log(
        "command:send_chat_message",
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
            "command:send_chat_message",
            format!("trace_id={trace_id} failed to persist user chat message: {error}"),
        );
    } else {
        debug::log(
            "command:send_chat_message",
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
                "command:send_chat_message",
                format!("trace_id={trace_id} user interaction event persisted"),
            );
            Some(event)
        }
        Err(error) => {
            debug::err(
                "command:send_chat_message",
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
                    "command:send_chat_message",
                    format!(
                        "trace_id={trace_id} working_memory_updated count={}",
                        items.len()
                    ),
                ),
                Err(error) => debug::err(
                    "command:send_chat_message",
                    format!("trace_id={trace_id} working_memory_update_failed: {error}"),
                ),
            }
        }
    }
    let mut memories = state.store.list_memories().await.unwrap_or_default();
    debug::log(
        "command:send_chat_message",
        format!(
            "trace_id={trace_id} settings_loaded llm_configured={} legacy_memories={}",
            !settings.llm.api_key.trim().is_empty() && !settings.llm.model.trim().is_empty(),
            memories.len()
        ),
    );
    let layered_context = match state.memory.context_for_chat(&clean).await {
        Ok(context) => {
            debug::log(
                "command:send_chat_message",
                format!(
                    "trace_id={trace_id} layered_context_loaded context_len={}",
                    context.len()
                ),
            );
            context
        }
        Err(error) => {
            debug::err(
                "command:send_chat_message",
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
                "command:send_chat_message",
                format!(
                    "trace_id={trace_id} llm_reply_ok reply_len={}",
                    reply.chars().count()
                ),
            );
            reply
        }
        Err(error) => {
            debug::err(
                "command:send_chat_message",
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
            "command:send_chat_message",
            format!("trace_id={trace_id} failed to persist assistant chat message: {error}"),
        );
    } else {
        debug::log(
            "command:send_chat_message",
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
            "command:send_chat_message",
            format!("trace_id={trace_id} failed to persist assistant interaction event: {error}"),
        );
    } else {
        debug::log(
            "command:send_chat_message",
            format!("trace_id={trace_id} assistant interaction event persisted"),
        );
    }

    if should_store_memory(&clean) {
        let memory = MemoryDocument {
            body: clean.clone(),
            source: "chat".to_string(),
            created_at: Utc::now(),
        };
        if let Err(error) = state.store.append_memory(&memory).await {
            debug::err(
                "command:send_chat_message",
                format!("trace_id={trace_id} failed to persist legacy memory: {error}"),
            );
        } else {
            debug::log(
                "command:send_chat_message",
                format!("trace_id={trace_id} legacy memory persisted"),
            );
        }
        memories.push(memory);
    }

    debug::log(
        "command:send_chat_message",
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

#[tauri::command]
pub async fn list_chat_history(state: State<'_, AppState>) -> QPawResult<Vec<ChatMessage>> {
    debug::log("command:list_chat_history", "loading chat history");
    state.store.list_chat_history().await
}

#[tauri::command]
pub async fn list_working_memory(state: State<'_, AppState>) -> QPawResult<Vec<WorkingMemoryItem>> {
    debug::log(
        "command:list_working_memory",
        "loading active working memory",
    );
    state.memory.list_working_memory().await
}

#[tauri::command]
pub async fn clear_working_memory(state: State<'_, AppState>) -> QPawResult<()> {
    debug::log("command:clear_working_memory", "clearing working memory");
    state.memory.clear_working_memory().await
}

#[tauri::command]
pub async fn query_memory(
    request: MemoryQueryRequest,
    state: State<'_, AppState>,
) -> QPawResult<MemoryQueryResponse> {
    debug::log(
        "command:query_memory",
        format!(
            "query_len={} layer={:?} category={:?} limit={:?}",
            request.query.chars().count(),
            request.layer,
            request.category,
            request.limit
        ),
    );
    state.memory.query(request).await
}

#[tauri::command]
pub async fn list_memory_items(
    filter: MemoryLayerFilter,
    state: State<'_, AppState>,
) -> QPawResult<Vec<LayeredMemoryItem>> {
    debug::log(
        "command:list_memory_items",
        format!(
            "layer={:?} category={:?} query_len={} include_archived={}",
            filter.layer,
            filter.category,
            filter.query.as_deref().unwrap_or_default().chars().count(),
            filter.include_archived
        ),
    );
    state.memory.list(filter).await
}

#[tauri::command]
pub async fn delete_memory_item(
    layer: MemoryLayer,
    id: String,
    state: State<'_, AppState>,
) -> QPawResult<()> {
    debug::log(
        "command:delete_memory_item",
        format!("layer={layer:?} id={id}"),
    );
    state.memory.delete(layer, id).await
}

#[tauri::command]
pub async fn run_memory_consolidation(
    date: Option<String>,
    state: State<'_, AppState>,
) -> QPawResult<MemoryConsolidationReport> {
    debug::log(
        "command:run_memory_consolidation",
        format!("date_arg={date:?}"),
    );
    let date = date
        .as_deref()
        .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok());
    state.memory.consolidate_date(date).await
}

#[tauri::command]
pub async fn get_memory_stats(state: State<'_, AppState>) -> QPawResult<MemoryStats> {
    debug::log("command:get_memory_stats", "loading memory stats");
    state.memory.stats().await
}

#[tauri::command]
pub async fn record_task_event(
    summary: String,
    status: Option<String>,
    content: Option<Value>,
    state: State<'_, AppState>,
) -> QPawResult<()> {
    debug::log(
        "command:record_task_event",
        format!(
            "summary_len={} status={status:?} has_content={}",
            summary.chars().count(),
            content.is_some()
        ),
    );
    let mut tags = vec!["task".to_string()];
    if let Some(status) = status.as_deref().filter(|value| !value.trim().is_empty()) {
        tags.push(status.trim().to_string());
    }

    state
        .memory
        .record_event(
            InteractionEventKind::TaskEvent,
            "user",
            summary.trim().to_string(),
            json!({
                "status": status,
                "content": content.unwrap_or_else(|| json!({}))
            }),
            tags,
        )
        .await?;
    Ok(())
}

#[tauri::command]
pub async fn list_memories(state: State<'_, AppState>) -> QPawResult<Vec<MemoryDocument>> {
    debug::log("command:list_memories", "loading legacy memories");
    state.store.list_memories().await
}

#[tauri::command]
pub async fn clear_memory(state: State<'_, AppState>) -> QPawResult<()> {
    debug::log("command:clear_memory", "clearing all memory tables");
    state.store.clear_memory().await
}

#[tauri::command]
pub async fn trigger_test_reminder(
    kind: ReminderKind,
    app: AppHandle,
    state: State<'_, AppState>,
) -> QPawResult<ReminderPayload> {
    debug::log("command:trigger_test_reminder", format!("kind={kind}"));
    let settings = state.store.get_settings().await?.reminders;
    let item = settings
        .items
        .iter()
        .find(|item| item.id == kind)
        .cloned()
        .unwrap_or_else(|| crate::models::ReminderDefinition {
            id: kind.clone(),
            title: "测试提醒".to_string(),
            message: crate::reminders::reminder_message(&settings, &kind),
            action_label: "照顾好了".to_string(),
            interval_minutes: 1,
            idle_grace_minutes: 1,
            paused: false,
        });
    let message = crate::reminders::reminder_message_for_title(&item.title);
    let payload = ReminderPayload {
        id: Uuid::new_v4().to_string(),
        kind: item.id.clone(),
        title: item.title.clone(),
        message: format!("测试提醒：{message}"),
        action_label: item.action_label.clone(),
        due_at: Utc::now(),
    };

    state
        .store
        .append_reminder_event(&ReminderEvent {
            reminder_id: payload.id.clone(),
            kind: payload.kind.clone(),
            message: payload.message.clone(),
            feedback: None,
            idle_seconds: 0,
            created_at: Utc::now(),
        })
        .await?;

    app.emit("reminder_due", payload.clone()).ok();
    Ok(payload)
}

#[tauri::command]
pub async fn get_reminder_status(state: State<'_, AppState>) -> QPawResult<ReminderRuntimeStatus> {
    debug::log(
        "command:get_reminder_status",
        "loading reminder runtime status",
    );
    state.reminders.status().await
}

#[tauri::command]
pub async fn set_reminder_feedback(
    payload: ReminderFeedbackPayload,
    state: State<'_, AppState>,
) -> QPawResult<()> {
    debug::log(
        "command:set_reminder_feedback",
        format!(
            "reminder_id={} kind={} feedback={:?}",
            payload.reminder_id, payload.kind, payload.feedback
        ),
    );
    state.store.set_reminder_feedback(&payload).await?;
    state
        .memory
        .record_event(
            InteractionEventKind::ReminderFeedback,
            "user",
            format!("{} reminder feedback: {:?}", payload.kind, payload.feedback),
            json!({
                "reminder_id": payload.reminder_id,
                "kind": payload.kind,
                "feedback": payload.feedback
            }),
            vec!["reminder".to_string(), "feedback".to_string()],
        )
        .await?;
    Ok(())
}

fn should_store_memory(message: &str) -> bool {
    let lowered = message.to_lowercase();
    message.contains("记住")
        || message.contains("记得")
        || lowered.contains("remember")
        || lowered.contains("note that")
}
