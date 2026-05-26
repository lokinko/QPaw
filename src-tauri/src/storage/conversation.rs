use super::DocumentStore;
use crate::debug;
use crate::error::QPawResult;
use crate::models::{ChatMessage, MemoryDocument};

impl DocumentStore {
    pub async fn append_chat(&self, message: &ChatMessage) -> QPawResult<()> {
        debug::log(
            "storage:append_chat",
            format!(
                "role={:?} content_len={}",
                message.role,
                message.content.chars().count()
            ),
        );
        let _: Option<ChatMessage> = self
            .db
            .create("conversation")
            .content(message.clone())
            .await?;
        Ok(())
    }

    pub async fn list_chat_history(&self) -> QPawResult<Vec<ChatMessage>> {
        let mut messages: Vec<ChatMessage> = self.db.select("conversation").await?;
        messages.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        debug::log(
            "storage:list_chat_history",
            format!("count={}", messages.len()),
        );
        Ok(messages)
    }

    pub async fn append_memory(&self, memory: &MemoryDocument) -> QPawResult<()> {
        debug::log(
            "storage:append_memory",
            format!(
                "source={} body_len={}",
                memory.source,
                memory.body.chars().count()
            ),
        );
        let _: Option<MemoryDocument> = self.db.create("memory").content(memory.clone()).await?;
        Ok(())
    }

    pub async fn list_memories(&self) -> QPawResult<Vec<MemoryDocument>> {
        let memories: Vec<MemoryDocument> = self.db.select("memory").await?;
        debug::log("storage:list_memories", format!("count={}", memories.len()));
        Ok(memories)
    }

    pub async fn clear_memory(&self) -> QPawResult<()> {
        debug::log(
            "storage:clear_memory",
            "deleting conversation and memory tables",
        );
        self.db
            .query(
                "DELETE conversation;
                 DELETE memory;
                 DELETE habit_event;
                 DELETE reminder_event;
                 DELETE interaction_event;
                 DELETE working_memory;
                 DELETE memory_l0;
                 DELETE memory_l1_concept;
                 DELETE memory_l1_relation;
                 DELETE memory_l2_event;
                 DELETE memory_l3_reflection;
                 DELETE memory_consolidation_job;",
            )
            .await?;
        Ok(())
    }
}
