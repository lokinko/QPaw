use chrono::Utc;

use super::DocumentStore;
use crate::debug;
use crate::error::QPawResult;
use crate::models::{HabitEvent, ReminderEvent, ReminderFeedbackPayload};

impl DocumentStore {
    pub async fn append_habit_event(&self, active: bool, idle_seconds: u64) -> QPawResult<()> {
        debug::log(
            "storage:append_habit_event",
            format!("active={active} idle_seconds={idle_seconds}"),
        );
        let event = HabitEvent {
            active,
            idle_seconds,
            created_at: Utc::now(),
        };
        let _: Option<HabitEvent> = self.db.create("habit_event").content(event).await?;
        Ok(())
    }

    pub async fn append_reminder_event(&self, event: &ReminderEvent) -> QPawResult<()> {
        debug::log(
            "storage:append_reminder_event",
            format!(
                "reminder_id={} kind={:?} idle_seconds={}",
                event.reminder_id, event.kind, event.idle_seconds
            ),
        );
        let _: Option<ReminderEvent> = self
            .db
            .create("reminder_event")
            .content(event.clone())
            .await?;
        Ok(())
    }

    pub async fn set_reminder_feedback(&self, payload: &ReminderFeedbackPayload) -> QPawResult<()> {
        debug::log(
            "storage:set_reminder_feedback",
            format!(
                "reminder_id={} kind={:?} feedback={:?}",
                payload.reminder_id, payload.kind, payload.feedback
            ),
        );
        self.db
            .query(
                "UPDATE reminder_event
                 SET feedback = $feedback
                 WHERE reminder_id = $reminder_id;",
            )
            .bind(("feedback", payload.feedback.clone()))
            .bind(("reminder_id", payload.reminder_id.clone()))
            .await?;
        Ok(())
    }
}
