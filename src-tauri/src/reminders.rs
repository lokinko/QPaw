use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::debug;
use crate::error::QPawResult;
use crate::idle::IdleProvider;
use crate::models::{
    ReminderDefinition, ReminderEvent, ReminderItemRuntimeStatus, ReminderKind, ReminderPayload,
    ReminderRuntimeStatus, ReminderSettings,
};
use crate::notification::{NotificationProvider, TauriEventNotificationProvider};
use crate::storage::DocumentStore;

const TICK_SECONDS: u64 = 1;
const NATURAL_IDLE_SECONDS: u64 = 60;

#[derive(Debug, Clone)]
struct PendingReminder {
    kind: ReminderKind,
    waited_seconds: u64,
}

#[derive(Debug, Default)]
struct ReminderState {
    active_seconds: HashMap<String, u64>,
    pending: Option<PendingReminder>,
    habit_tick: u64,
}

pub struct ReminderRuntime {
    store: Arc<DocumentStore>,
    idle: Arc<dyn IdleProvider>,
    state: Mutex<ReminderState>,
}

impl ReminderRuntime {
    pub fn new(store: Arc<DocumentStore>, idle: Arc<dyn IdleProvider>) -> Self {
        Self {
            store,
            idle,
            state: Mutex::new(ReminderState::default()),
        }
    }

    pub async fn tick(&self) -> QPawResult<Option<ReminderPayload>> {
        let settings = self.store.get_settings().await?.reminders;
        let idle_seconds = self.idle.idle_seconds().await.unwrap_or(0);
        let mut state = self.state.lock().await;

        if settings.paused {
            if state.pending.is_some() {
                debug::log("reminders:tick", "paused; clearing pending reminder");
            }
            state.pending = None;
            return Ok(None);
        }

        let active = idle_seconds < NATURAL_IDLE_SECONDS;
        if active {
            for item in active_reminders(&settings) {
                *state.active_seconds.entry(item.id.clone()).or_default() += TICK_SECONDS;
            }
        }

        state.habit_tick += TICK_SECONDS;
        if state.habit_tick >= 300 {
            state.habit_tick = 0;
            let _ = self.store.append_habit_event(active, idle_seconds).await;
        }

        if let Some(pending) = state.pending.as_ref() {
            let still_valid = settings
                .items
                .iter()
                .any(|item| item.id == pending.kind && !item.paused);
            if !still_valid {
                debug::log(
                    "reminders:tick",
                    format!("pending reminder removed_or_paused kind={}", pending.kind),
                );
                state.pending = None;
            }
        }

        if state.pending.is_none() {
            state.pending = due_kind(&settings, &state).map(|kind| {
                let active_seconds = state.active_seconds.get(&kind).copied().unwrap_or(0);
                debug::log(
                    "reminders:tick",
                    format!(
                        "due kind={kind} idle_seconds={idle_seconds} active_seconds={active_seconds}"
                    ),
                );
                PendingReminder {
                    kind,
                    waited_seconds: 0,
                }
            });
        }

        if let Some(pending) = state.pending.as_mut() {
            pending.waited_seconds += TICK_SECONDS;
            let Some(item) = settings.items.iter().find(|item| item.id == pending.kind) else {
                state.pending = None;
                return Ok(None);
            };
            let grace_seconds = item.idle_grace_minutes * 60;
            if idle_seconds >= NATURAL_IDLE_SECONDS || pending.waited_seconds >= grace_seconds {
                let kind = pending.kind.clone();
                debug::log(
                    "reminders:tick",
                    format!(
                        "emitting kind={kind} idle_seconds={idle_seconds} waited_seconds={}",
                        pending.waited_seconds
                    ),
                );
                state.pending = None;
                state.active_seconds.insert(kind.clone(), 0);
                return self.build_payload(item, idle_seconds).await.map(Some);
            }
        }

        Ok(None)
    }

    pub async fn status(&self) -> QPawResult<ReminderRuntimeStatus> {
        let settings = self.store.get_settings().await?.reminders;
        let idle_seconds = self.idle.idle_seconds().await.unwrap_or(0);
        let active = idle_seconds < NATURAL_IDLE_SECONDS;
        let state = self.state.lock().await;
        let pending = state.pending.clone();

        Ok(ReminderRuntimeStatus {
            paused: settings.paused,
            idle_seconds,
            active,
            items: settings
                .items
                .iter()
                .map(|item| ReminderItemRuntimeStatus {
                    id: item.id.clone(),
                    title: item.title.clone(),
                    paused: item.paused,
                    active_seconds: state.active_seconds.get(&item.id).copied().unwrap_or(0),
                    interval_seconds: item.interval_minutes * 60,
                    idle_grace_seconds: item.idle_grace_minutes * 60,
                    pending_waited_seconds: pending
                        .as_ref()
                        .filter(|pending| pending.kind == item.id)
                        .map(|pending| pending.waited_seconds),
                })
                .collect(),
        })
    }

    async fn build_payload(
        &self,
        item: &ReminderDefinition,
        idle_seconds: u64,
    ) -> QPawResult<ReminderPayload> {
        let id = Uuid::new_v4().to_string();
        let message = reminder_message_for_title(&item.title);
        let payload = ReminderPayload {
            id: id.clone(),
            kind: item.id.clone(),
            title: item.title.clone(),
            message: message.clone(),
            action_label: item.action_label.clone(),
            due_at: Utc::now(),
        };
        self.store
            .append_reminder_event(&ReminderEvent {
                reminder_id: id,
                kind: item.id.clone(),
                message,
                feedback: None,
                idle_seconds,
                created_at: Utc::now(),
            })
            .await?;
        Ok(payload)
    }
}

fn active_reminders(settings: &ReminderSettings) -> impl Iterator<Item = &ReminderDefinition> {
    settings.items.iter().filter(|item| !item.paused)
}

fn due_kind(settings: &ReminderSettings, state: &ReminderState) -> Option<ReminderKind> {
    active_reminders(settings)
        .find(|item| {
            state.active_seconds.get(&item.id).copied().unwrap_or(0) >= item.interval_minutes * 60
        })
        .map(|item| item.id.clone())
}

pub fn reminder_message(settings: &ReminderSettings, kind: &str) -> String {
    settings
        .items
        .iter()
        .find(|item| item.id == kind)
        .map(|item| reminder_message_for_title(&item.title))
        .unwrap_or_else(|| "到时间了。先照顾一下自己，身体会记得每一次被认真对待。".to_string())
}

pub fn reminder_message_for_title(title: &str) -> String {
    let title = title.trim();
    let label = if title.is_empty() {
        "休息一下"
    } else {
        title
    };
    if label.contains('水') || label.to_ascii_lowercase().contains("drink") {
        return "该喝点水了。慢慢喝几口，让身体从安静的地方重新亮起来。".to_string();
    }
    if label.contains('眼') || label.contains("休息") || label.to_ascii_lowercase().contains("eye")
    {
        return "眼睛也陪你努力很久了。闭上眼休息一小会儿，把清亮还给自己。".to_string();
    }
    format!("到「{label}」的时间了。先把自己照顾好一点，身体会记得每一次温柔的停顿。")
}

pub fn start_reminder_loop(app: tauri::AppHandle, runtime: Arc<ReminderRuntime>) {
    let notifications = TauriEventNotificationProvider::new(app);
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(TICK_SECONDS));
        loop {
            interval.tick().await;
            match runtime.tick().await {
                Ok(Some(payload)) => {
                    let _ = notifications.reminder_due(payload).await;
                }
                Ok(None) => {}
                Err(error) => eprintln!("reminder tick failed: {error}"),
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ReminderDefinition, ReminderSettings};

    fn settings() -> ReminderSettings {
        ReminderSettings {
            paused: false,
            items: vec![
                ReminderDefinition {
                    id: "eye_rest".to_string(),
                    title: "闭眼休息".to_string(),
                    message: "闭眼休息 20 秒，让眼睛缓一下。".to_string(),
                    action_label: "休息了".to_string(),
                    interval_minutes: 45,
                    idle_grace_minutes: 10,
                    paused: false,
                },
                ReminderDefinition {
                    id: "hydration".to_string(),
                    title: "喝水时间".to_string(),
                    message: "喝口水，然后继续。".to_string(),
                    action_label: "喝过了".to_string(),
                    interval_minutes: 60,
                    idle_grace_minutes: 10,
                    paused: false,
                },
            ],
        }
    }

    #[test]
    fn eye_rest_is_prioritized_when_both_are_due() {
        let state = ReminderState {
            active_seconds: HashMap::from([
                ("hydration".to_string(), 3600),
                ("eye_rest".to_string(), 2700),
            ]),
            pending: None,
            habit_tick: 0,
        };
        assert_eq!(due_kind(&settings(), &state), Some("eye_rest".to_string()));
    }

    #[test]
    fn hydration_is_due_after_interval() {
        let state = ReminderState {
            active_seconds: HashMap::from([
                ("hydration".to_string(), 3600),
                ("eye_rest".to_string(), 300),
            ]),
            pending: None,
            habit_tick: 0,
        };
        assert_eq!(due_kind(&settings(), &state), Some("hydration".to_string()));
    }

    #[test]
    fn no_due_kind_before_threshold() {
        let state = ReminderState {
            active_seconds: HashMap::from([
                ("hydration".to_string(), 900),
                ("eye_rest".to_string(), 900),
            ]),
            pending: None,
            habit_tick: 0,
        };
        assert_eq!(due_kind(&settings(), &state), None);
    }

    #[test]
    fn paused_item_is_not_due() {
        let mut settings = settings();
        settings.items[0].paused = true;
        let state = ReminderState {
            active_seconds: HashMap::from([
                ("hydration".to_string(), 900),
                ("eye_rest".to_string(), 2700),
            ]),
            pending: None,
            habit_tick: 0,
        };
        assert_eq!(due_kind(&settings, &state), None);
    }
}
