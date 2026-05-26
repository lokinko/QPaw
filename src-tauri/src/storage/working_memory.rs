use chrono::{NaiveDate, Utc};

use super::{records::*, DocumentStore};
use crate::debug;
use crate::error::QPawResult;
use crate::models::WorkingMemoryItem;

impl DocumentStore {
    pub async fn save_working_memory(&self, item: &WorkingMemoryItem) -> QPawResult<()> {
        debug::log(
            "storage:save_working_memory",
            format!(
                "id={} kind={:?} title_len={} keywords={}",
                item.id,
                item.kind,
                item.title.chars().count(),
                item.keywords.len()
            ),
        );
        self.delete_working_memory_by_id(&item.id).await?;
        let _: Option<WorkingMemoryRecord> = self
            .db
            .create("working_memory")
            .content(WorkingMemoryRecord::from(item))
            .await?;
        Ok(())
    }

    pub async fn list_working_memory(&self) -> QPawResult<Vec<WorkingMemoryItem>> {
        let records: Vec<WorkingMemoryRecord> = self.db.select("working_memory").await?;
        let mut items = records
            .into_iter()
            .map(WorkingMemoryItem::from)
            .collect::<Vec<_>>();
        items.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        debug::log(
            "storage:list_working_memory",
            format!("count={}", items.len()),
        );
        Ok(items)
    }

    pub async fn list_active_working_memory(&self) -> QPawResult<Vec<WorkingMemoryItem>> {
        let now = Utc::now();
        let mut items = self
            .list_working_memory()
            .await?
            .into_iter()
            .filter(|item| item.expires_at > now)
            .collect::<Vec<_>>();
        items.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        debug::log(
            "storage:list_active_working_memory",
            format!("count={}", items.len()),
        );
        Ok(items)
    }

    pub async fn list_working_memory_for_date(
        &self,
        date: NaiveDate,
    ) -> QPawResult<Vec<WorkingMemoryItem>> {
        let items = self
            .list_working_memory()
            .await?
            .into_iter()
            .filter(|item| {
                item.created_at.date_naive() == date || item.updated_at.date_naive() == date
            })
            .collect::<Vec<_>>();
        debug::log(
            "storage:list_working_memory_for_date",
            format!("date={date} count={}", items.len()),
        );
        Ok(items)
    }

    pub async fn clear_working_memory(&self) -> QPawResult<()> {
        debug::log(
            "storage:clear_working_memory",
            "deleting all working memory",
        );
        self.db.query("DELETE working_memory;").await?;
        Ok(())
    }

    pub async fn clear_working_memory_for_date(&self, date: NaiveDate) -> QPawResult<()> {
        let items = self.list_working_memory_for_date(date).await?;
        debug::log(
            "storage:clear_working_memory_for_date",
            format!("date={date} count={}", items.len()),
        );
        for item in items {
            self.delete_working_memory_by_id(&item.id).await?;
        }
        Ok(())
    }

    pub async fn cleanup_expired_working_memory(&self) -> QPawResult<usize> {
        let now = Utc::now();
        let items = self
            .list_working_memory()
            .await?
            .into_iter()
            .filter(|item| item.expires_at <= now)
            .collect::<Vec<_>>();
        let count = items.len();
        for item in items {
            self.delete_working_memory_by_id(&item.id).await?;
        }
        debug::log(
            "storage:cleanup_expired_working_memory",
            format!("deleted={count}"),
        );
        Ok(count)
    }

    async fn delete_working_memory_by_id(&self, id: &str) -> QPawResult<()> {
        self.db
            .query("DELETE working_memory WHERE uid = $id;")
            .bind(("id", id.to_string()))
            .await?;
        Ok(())
    }
}
