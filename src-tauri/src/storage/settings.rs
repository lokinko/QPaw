use super::{records::*, DocumentStore};
use crate::debug;
use crate::error::QPawResult;
use crate::models::{AppSettings, AvatarManifest};

impl DocumentStore {
    pub async fn get_settings(&self) -> QPawResult<AppSettings> {
        let settings: Option<AppSettings> = self.db.select(("app_settings", "default")).await?;
        debug::log(
            "storage:get_settings",
            format!("found_existing={}", settings.is_some()),
        );
        Ok(settings.unwrap_or_default())
    }

    pub async fn save_settings(&self, settings: &AppSettings) -> QPawResult<AppSettings> {
        debug::log(
            "storage:save_settings",
            format!(
                "llm_configured={} memory_enabled={} reminders_paused={}",
                !settings.llm.api_key.trim().is_empty() && !settings.llm.model.trim().is_empty(),
                settings.memory.enabled,
                settings.reminders.paused
            ),
        );
        let mut response = self
            .db
            .query("UPSERT app_settings:default CONTENT $settings;")
            .bind(("settings", settings.clone()))
            .await?;
        let saved: Option<AppSettings> = response.take(0)?;
        Ok(saved.unwrap_or_else(|| settings.clone()))
    }

    pub async fn save_avatar(&self, manifest: &AvatarManifest) -> QPawResult<()> {
        debug::log(
            "storage:save_avatar",
            format!(
                "avatar_id={} path_len={} kind={:?}",
                manifest.id,
                manifest.path.len(),
                manifest.kind
            ),
        );
        let _: Option<AvatarManifestRecord> = self
            .db
            .create("avatar")
            .content(AvatarManifestRecord::from(manifest))
            .await?;
        Ok(())
    }
}
