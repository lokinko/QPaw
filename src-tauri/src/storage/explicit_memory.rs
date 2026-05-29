use super::{DocumentStore, ExplicitMemoryRecord};
use crate::debug;
use crate::error::QPawResult;
use crate::models::{ExplicitMemoryItem, ExplicitMemoryStatus};

fn normalize_body(body: &str) -> String {
    body.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

impl DocumentStore {
    pub async fn upsert_explicit_memory(&self, item: &ExplicitMemoryItem) -> QPawResult<()> {
        let normalized_body = normalize_body(&item.body);
        debug::log(
            "storage:upsert_explicit_memory",
            format!(
                "id={} body_len={} keywords={}",
                item.id,
                item.body.chars().count(),
                item.keywords.len()
            ),
        );
        self.db
            .query("DELETE explicit_memory WHERE normalized_body = $normalized_body OR uid = $uid;")
            .bind(("normalized_body", normalized_body.clone()))
            .bind(("uid", item.id.clone()))
            .await?;
        let _: Option<ExplicitMemoryRecord> = self
            .db
            .create("explicit_memory")
            .content(ExplicitMemoryRecord::from_item(item, normalized_body))
            .await?;
        Ok(())
    }

    pub async fn list_active_explicit_memories(&self) -> QPawResult<Vec<ExplicitMemoryItem>> {
        let records: Vec<ExplicitMemoryRecord> = self.db.select("explicit_memory").await?;
        let mut items = records
            .into_iter()
            .filter(|record| record.status == ExplicitMemoryStatus::Active)
            .map(ExplicitMemoryItem::from)
            .collect::<Vec<_>>();
        items.sort_by(|a, b| b.last_used_at.cmp(&a.last_used_at));
        debug::log(
            "storage:list_active_explicit_memories",
            format!("count={}", items.len()),
        );
        Ok(items)
    }
}
