use async_trait::async_trait;
use tauri::{AppHandle, Emitter};

use crate::error::QPawResult;
use crate::models::ReminderPayload;

#[async_trait]
pub trait NotificationProvider: Send + Sync {
    async fn reminder_due(&self, payload: ReminderPayload) -> QPawResult<()>;
}

pub struct TauriEventNotificationProvider {
    app: AppHandle,
}

impl TauriEventNotificationProvider {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

#[async_trait]
impl NotificationProvider for TauriEventNotificationProvider {
    async fn reminder_due(&self, payload: ReminderPayload) -> QPawResult<()> {
        self.app.emit("reminder_due", payload)?;
        Ok(())
    }
}
