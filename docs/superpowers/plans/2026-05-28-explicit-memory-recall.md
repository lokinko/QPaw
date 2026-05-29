# Explicit Memory Recall Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make explicit user memories immediately recallable in the next chat turn and shared across Codex CLI and OpenAI-compatible providers.

**Architecture:** Add a local `ExplicitMemoryItem` model and SurrealDB-backed storage methods. Import explicit save-worthy chat messages into this immediate recall layer, retrieve matching or recent active explicit memories in `MemoryRetriever::context_for_chat`, and include them in a distinct `Immediate user memories` prompt section before invoking either provider.

**Tech Stack:** Rust, Tauri backend, SurrealDB local engine, existing `DocumentStore`, `MemoryService`, `MemoryRetriever`, and Cargo tests.

---

## File Structure

- `src-tauri/src/models.rs`: add explicit memory domain types.
- `src-tauri/src/storage/records.rs`: add `ExplicitMemoryRecord` conversion.
- `src-tauri/src/storage/explicit_memory.rs`: create focused storage functions for upsert and active-list retrieval.
- `src-tauri/src/storage.rs`: register the new storage module and clear explicit memory in full data reset.
- `src-tauri/src/memory/explicit.rs`: create deterministic normalization, keyword extraction, and matching helpers.
- `src-tauri/src/memory/mod.rs`: expose `import_explicit_memory`.
- `src-tauri/src/chat.rs`: import explicit memories when the chat decision says `save` or the legacy detector matches.
- `src-tauri/src/memory/retriever.rs`: retrieve explicit memories and add an `Immediate user memories` section to chat context.
- `src-tauri/src/memory/consolidator.rs`: include explicit memories as consolidation input without removing them on failure.

---

### Task 1: Add Explicit Memory Domain Types

**Files:**
- Modify: `src-tauri/src/models.rs`

- [ ] **Step 1: Write the failing model serialization test**

Add these imports to the existing `#[cfg(test)] mod tests` import list in `src-tauri/src/models.rs`:

```rust
use super::{ExplicitMemoryItem, ExplicitMemoryStatus};
```

Add this test inside the existing tests module:

```rust
#[test]
fn explicit_memory_status_serializes_as_snake_case() {
    let encoded = serde_json::to_value(ExplicitMemoryStatus::Consolidated)
        .expect("serialize explicit memory status");

    assert_eq!(encoded, serde_json::json!("consolidated"));
}

#[test]
fn explicit_memory_item_preserves_original_body_and_keywords() {
    let now = chrono::Utc::now();
    let item = ExplicitMemoryItem {
        id: "explicit_1".to_string(),
        body: "记住我喜欢简洁回答".to_string(),
        source: "chat".to_string(),
        tags: vec!["explicit_memory_request".to_string()],
        keywords: vec!["简洁".to_string(), "回答".to_string()],
        created_at: now,
        last_used_at: now,
        status: ExplicitMemoryStatus::Active,
    };

    assert_eq!(item.body, "记住我喜欢简洁回答");
    assert_eq!(item.keywords, vec!["简洁", "回答"]);
}
```

- [ ] **Step 2: Run the model tests and verify failure**

Run:

```powershell
cd src-tauri
cargo test models::tests::explicit_memory
```

Expected: FAIL because `ExplicitMemoryItem` and `ExplicitMemoryStatus` do not exist.

- [ ] **Step 3: Add the domain types**

Add this near the other memory model types in `src-tauri/src/models.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExplicitMemoryStatus {
    Active,
    Consolidated,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplicitMemoryItem {
    pub id: String,
    pub body: String,
    pub source: String,
    pub tags: Vec<String>,
    pub keywords: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: DateTime<Utc>,
    pub status: ExplicitMemoryStatus,
}
```

- [ ] **Step 4: Run the model tests and verify pass**

Run:

```powershell
cd src-tauri
cargo test models::tests::explicit_memory
```

Expected: PASS.

---

### Task 2: Add Explicit Memory Storage

**Files:**
- Modify: `src-tauri/src/storage/records.rs`
- Create: `src-tauri/src/storage/explicit_memory.rs`
- Modify: `src-tauri/src/storage.rs`

- [ ] **Step 1: Write failing storage tests**

Add this test to the existing `#[cfg(test)] mod tests` in `src-tauri/src/storage.rs`:

```rust
#[tokio::test]
async fn explicit_memory_upsert_deduplicates_normalized_body() {
    let path = test_db_path("explicit-memory-dedupe");
    {
        let store = DocumentStore::connect(path.clone()).await.unwrap();
        let first = crate::models::ExplicitMemoryItem {
            id: "explicit_first".to_string(),
            body: "记住我喜欢简洁回答".to_string(),
            source: "chat".to_string(),
            tags: vec!["explicit_memory_request".to_string()],
            keywords: vec!["简洁".to_string(), "回答".to_string()],
            created_at: chrono::Utc::now(),
            last_used_at: chrono::Utc::now(),
            status: crate::models::ExplicitMemoryStatus::Active,
        };
        let second = crate::models::ExplicitMemoryItem {
            id: "explicit_second".to_string(),
            body: "  记住我喜欢简洁回答  ".to_string(),
            source: "chat".to_string(),
            tags: vec!["explicit_memory_request".to_string()],
            keywords: vec!["简洁".to_string(), "回答".to_string()],
            created_at: chrono::Utc::now(),
            last_used_at: chrono::Utc::now(),
            status: crate::models::ExplicitMemoryStatus::Active,
        };

        store.upsert_explicit_memory(&first).await.unwrap();
        store.upsert_explicit_memory(&second).await.unwrap();
        let items = store.list_active_explicit_memories().await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].body.trim(), "记住我喜欢简洁回答");
    }
    let _ = std::fs::remove_dir_all(path);
}

#[tokio::test]
async fn explicit_memory_round_trips_active_items() {
    let path = test_db_path("explicit-memory-roundtrip");
    {
        let store = DocumentStore::connect(path.clone()).await.unwrap();
        let item = crate::models::ExplicitMemoryItem {
            id: "explicit_roundtrip".to_string(),
            body: "remember that I prefer concise replies".to_string(),
            source: "chat".to_string(),
            tags: vec!["explicit_memory_request".to_string()],
            keywords: vec!["prefer".to_string(), "concise".to_string(), "replies".to_string()],
            created_at: chrono::Utc::now(),
            last_used_at: chrono::Utc::now(),
            status: crate::models::ExplicitMemoryStatus::Active,
        };

        store.upsert_explicit_memory(&item).await.unwrap();
        let items = store.list_active_explicit_memories().await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "explicit_roundtrip");
        assert_eq!(items[0].keywords, vec!["prefer", "concise", "replies"]);
    }
    let _ = std::fs::remove_dir_all(path);
}
```

- [ ] **Step 2: Run storage tests and verify failure**

Run:

```powershell
cd src-tauri
cargo test storage::tests::explicit_memory
```

Expected: FAIL because explicit memory storage methods do not exist.

- [ ] **Step 3: Add storage record conversion**

In `src-tauri/src/storage/records.rs`, add imports:

```rust
use crate::models::{ExplicitMemoryItem, ExplicitMemoryStatus};
```

Add this record type:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ExplicitMemoryRecord {
    pub(super) uid: String,
    pub(super) normalized_body: String,
    pub(super) body: String,
    pub(super) source: String,
    pub(super) tags: Vec<String>,
    pub(super) keywords: Vec<String>,
    pub(super) created_at: DateTime<Utc>,
    pub(super) last_used_at: DateTime<Utc>,
    pub(super) status: ExplicitMemoryStatus,
}

impl ExplicitMemoryRecord {
    pub(super) fn from_item(item: &ExplicitMemoryItem, normalized_body: String) -> Self {
        Self {
            uid: item.id.clone(),
            normalized_body,
            body: item.body.clone(),
            source: item.source.clone(),
            tags: item.tags.clone(),
            keywords: item.keywords.clone(),
            created_at: item.created_at,
            last_used_at: item.last_used_at,
            status: item.status.clone(),
        }
    }
}

impl From<ExplicitMemoryRecord> for ExplicitMemoryItem {
    fn from(item: ExplicitMemoryRecord) -> Self {
        Self {
            id: item.uid,
            body: item.body,
            source: item.source,
            tags: item.tags,
            keywords: item.keywords,
            created_at: item.created_at,
            last_used_at: item.last_used_at,
            status: item.status,
        }
    }
}
```

- [ ] **Step 4: Create focused explicit memory storage module**

Create `src-tauri/src/storage/explicit_memory.rs`:

```rust
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
            .query("DELETE explicit_memory WHERE normalized_body = $normalized_body;")
            .bind(("normalized_body", normalized_body.clone()))
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
```

- [ ] **Step 5: Register module and clear table on reset**

In `src-tauri/src/storage.rs`, add:

```rust
mod explicit_memory;
```

In `DocumentStore::clear_memory`, add `DELETE explicit_memory;` to the query:

```rust
DELETE explicit_memory;
```

In `DocumentStore::clear_legacy_structured_memory_once`, add `DELETE explicit_memory;` to the migration query because the immediate layer is derived from chat memory behavior and can be rebuilt from future interactions:

```rust
DELETE explicit_memory;
```

- [ ] **Step 6: Run storage tests and verify pass**

Run:

```powershell
cd src-tauri
cargo test storage::tests::explicit_memory
```

Expected: PASS.

---

### Task 3: Add Import And Matching Helpers

**Files:**
- Create: `src-tauri/src/memory/explicit.rs`
- Modify: `src-tauri/src/memory/mod.rs`

- [ ] **Step 1: Write failing helper tests**

Create `src-tauri/src/memory/explicit.rs` with tests first:

```rust
#[cfg(test)]
mod tests {
    use super::{extract_keywords, memory_matches_query, normalize_explicit_body};
    use crate::models::{ExplicitMemoryItem, ExplicitMemoryStatus};

    #[test]
    fn normalizes_explicit_memory_body() {
        assert_eq!(
            normalize_explicit_body("  Remember   THAT I Prefer Concise Replies  "),
            "remember that i prefer concise replies"
        );
    }

    #[test]
    fn extracts_keywords_from_chinese_and_english_text() {
        let keywords = extract_keywords("记住我喜欢简洁回答 and concise replies");

        assert!(keywords.contains(&"简洁".to_string()));
        assert!(keywords.contains(&"回答".to_string()));
        assert!(keywords.contains(&"concise".to_string()));
        assert!(keywords.contains(&"replies".to_string()));
    }

    #[test]
    fn explicit_memory_matches_query_by_keyword() {
        let now = chrono::Utc::now();
        let item = ExplicitMemoryItem {
            id: "explicit_1".to_string(),
            body: "记住我喜欢简洁回答".to_string(),
            source: "chat".to_string(),
            tags: vec!["explicit_memory_request".to_string()],
            keywords: vec!["简洁".to_string(), "回答".to_string()],
            created_at: now,
            last_used_at: now,
            status: ExplicitMemoryStatus::Active,
        };

        assert!(memory_matches_query(&item, "请简洁说明一下"));
        assert!(!memory_matches_query(&item, "今天几点了"));
    }
}
```

- [ ] **Step 2: Run helper tests and verify failure**

Run:

```powershell
cd src-tauri
cargo test memory::explicit::tests
```

Expected: FAIL because the helper functions are not implemented and the module is not registered.

- [ ] **Step 3: Register module**

In `src-tauri/src/memory/mod.rs`, add:

```rust
pub mod explicit;
```

- [ ] **Step 4: Implement helpers**

Replace `src-tauri/src/memory/explicit.rs` with:

```rust
use chrono::Utc;
use uuid::Uuid;

use crate::models::{ExplicitMemoryItem, ExplicitMemoryStatus};

const STOP_WORDS: &[&str] = &[
    "remember", "that", "please", "prefer", "记住", "记得", "记一下", "我", "喜欢",
];

pub fn normalize_explicit_body(body: &str) -> String {
    body.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

pub fn extract_keywords(text: &str) -> Vec<String> {
    let mut keywords = Vec::new();
    for token in text
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | '.' | '。' | '，' | '!' | '?' | '！' | '？' | ':' | '：' | ';' | '；'))
        .map(str::trim)
        .filter(|token| !token.is_empty())
    {
        let lowered = token.to_lowercase();
        if lowered.chars().all(|ch| ch.is_ascii_alphanumeric()) {
            if lowered.len() >= 4 && !STOP_WORDS.contains(&lowered.as_str()) {
                keywords.push(lowered);
            }
            continue;
        }

        for term in ["简洁", "回答", "提醒", "安静", "上午", "下午", "晚上", "睡眠", "喝水", "肩颈", "项目"] {
            if token.contains(term) {
                keywords.push(term.to_string());
            }
        }
    }
    keywords.sort();
    keywords.dedup();
    keywords
}

pub fn explicit_memory_from_message(body: &str, source: &str, tags: Vec<String>) -> ExplicitMemoryItem {
    let now = Utc::now();
    ExplicitMemoryItem {
        id: format!("explicit_{}", Uuid::new_v4()),
        body: body.trim().to_string(),
        source: source.to_string(),
        tags,
        keywords: extract_keywords(body),
        created_at: now,
        last_used_at: now,
        status: ExplicitMemoryStatus::Active,
    }
}

pub fn memory_matches_query(item: &ExplicitMemoryItem, query: &str) -> bool {
    let query = query.to_lowercase();
    item.keywords
        .iter()
        .any(|keyword| !keyword.trim().is_empty() && query.contains(&keyword.to_lowercase()))
}
```

- [ ] **Step 5: Expose import method on MemoryService**

In `src-tauri/src/memory/mod.rs`, add to imports:

```rust
use self::explicit::explicit_memory_from_message;
```

Add this method inside `impl MemoryService`:

```rust
pub async fn import_explicit_memory(
    &self,
    body: &str,
    source: &str,
    tags: Vec<String>,
) -> QPawResult<crate::models::ExplicitMemoryItem> {
    let item = explicit_memory_from_message(body, source, tags);
    debug::log(
        "memory:import_explicit_memory",
        format!("id={} body_len={}", item.id, item.body.chars().count()),
    );
    self.store.upsert_explicit_memory(&item).await?;
    Ok(item)
}
```

- [ ] **Step 6: Run helper tests and verify pass**

Run:

```powershell
cd src-tauri
cargo test memory::explicit::tests
```

Expected: PASS.

---

### Task 4: Import Explicit Chat Memories

**Files:**
- Modify: `src-tauri/src/chat.rs`

- [ ] **Step 1: Write failing chat import test**

Add this test to `src-tauri/src/chat.rs` tests module:

```rust
#[tokio::test]
async fn explicit_memory_detector_marks_messages_for_immediate_import() {
    assert!(should_store_legacy_memory("记住我喜欢简洁回答"));
    assert!(should_store_legacy_memory("remember that I prefer concise replies"));
}
```

This confirms the import trigger remains aligned with the legacy detector before changing the service wiring.

- [ ] **Step 2: Run chat tests**

Run:

```powershell
cd src-tauri
cargo test chat::tests::explicit_memory_detector_marks_messages_for_immediate_import
```

Expected: PASS because the detector already exists. This is a characterization test.

- [ ] **Step 3: Add explicit import call in chat path**

In `src-tauri/src/chat.rs`, replace this block:

```rust
    if should_store_legacy_memory(&clean) || memory_decision.action == MemoryDecisionAction::Save {
```

with:

```rust
    let should_save_memory =
        should_store_legacy_memory(&clean) || memory_decision.action == MemoryDecisionAction::Save;

    if should_save_memory {
```

Inside the `if should_save_memory` block, after the legacy `append_memory` logging block, add:

```rust
        if let Err(error) = state
            .memory
            .import_explicit_memory(
                &clean,
                "chat",
                vec!["explicit_memory_request".to_string()],
            )
            .await
        {
            debug::err(
                "chat:send",
                format!("trace_id={trace_id} failed to import explicit memory: {error}"),
            );
        } else {
            debug::log(
                "chat:send",
                format!("trace_id={trace_id} explicit memory imported"),
            );
        }
```

- [ ] **Step 4: Run chat tests**

Run:

```powershell
cd src-tauri
cargo test chat::tests
```

Expected: PASS.

---

### Task 5: Include Explicit Memories In Chat Context

**Files:**
- Modify: `src-tauri/src/memory/retriever.rs`

- [ ] **Step 1: Write failing context builder test**

In `src-tauri/src/memory/retriever.rs`, add a tests module:

```rust
#[cfg(test)]
mod tests {
    use super::build_context;
    use crate::models::{ExplicitMemoryItem, ExplicitMemoryStatus};

    #[test]
    fn context_includes_immediate_user_memories_section() {
        let now = chrono::Utc::now();
        let explicit = vec![ExplicitMemoryItem {
            id: "explicit_1".to_string(),
            body: "记住我喜欢简洁回答".to_string(),
            source: "chat".to_string(),
            tags: vec!["explicit_memory_request".to_string()],
            keywords: vec!["简洁".to_string(), "回答".to_string()],
            created_at: now,
            last_used_at: now,
            status: ExplicitMemoryStatus::Active,
        }];

        let context = build_context(&explicit, &[], &[]);

        assert!(context.contains("Immediate user memories:"));
        assert!(context.contains("记住我喜欢简洁回答"));
    }
}
```

- [ ] **Step 2: Run retriever test and verify failure**

Run:

```powershell
cd src-tauri
cargo test memory::retriever::tests::context_includes_immediate_user_memories_section
```

Expected: FAIL because `build_context` does not accept explicit memories yet.

- [ ] **Step 3: Add explicit memory imports and selection**

In `src-tauri/src/memory/retriever.rs`, update imports:

```rust
use crate::models::{
    ExplicitMemoryItem, LayeredMemoryItem, MemoryLayer, MemoryLayerFilter, MemoryQueryRequest,
    MemoryQueryResponse,
};
```

Add:

```rust
use super::explicit::memory_matches_query;
```

In `context_for_chat`, after loading working memory, add:

```rust
        let explicit = self.store.list_active_explicit_memories().await?;
        let selected_explicit = select_explicit_memories(message, explicit);
```

Change:

```rust
        let context = build_context(&working, &selected);
```

to:

```rust
        let context = build_context(&selected_explicit, &working, &selected);
```

Update the debug log format to include explicit count:

```rust
                "working_items={} explicit_items={} selected_items={} context_len={}",
                working.len(),
                selected_explicit.len(),
                selected.len(),
                context.len()
```

- [ ] **Step 4: Update context builder**

Replace:

```rust
fn build_layered_context(items: &[LayeredMemoryItem]) -> String {
    build_context(&[], items)
}
```

with:

```rust
fn build_layered_context(items: &[LayeredMemoryItem]) -> String {
    build_context(&[], &[], items)
}
```

Replace the current `build_context` signature and first section with:

```rust
fn build_context(
    explicit: &[ExplicitMemoryItem],
    working: &[crate::models::WorkingMemoryItem],
    items: &[LayeredMemoryItem],
) -> String {
    let mut sections = Vec::new();
    if !explicit.is_empty() {
        sections.push("Immediate user memories:".to_string());
        sections.extend(explicit.iter().map(|item| {
            format!("- {} [{}]", item.body.trim(), item.keywords.join(", "))
        }));
    }
    if !working.is_empty() {
        sections.push("Today's working memory:".to_string());
        sections.extend(working.iter().map(|item| {
            format!(
                "- {:?} {}: {} [{}]",
                item.kind,
                item.title.trim(),
                item.summary.trim(),
                item.keywords.join(", ")
            )
        }));
    }
    if !items.is_empty() {
        sections.push("Layered long-term memory:".to_string());
    }
    sections.extend(
        items
            .iter()
            .map(|item| {
                format!(
                    "- {:?} {}: {}",
                    item.layer,
                    item.title.trim(),
                    item.summary.trim()
                )
            })
            .collect::<Vec<_>>(),
    );
    sections.join("\n")
}
```

Add the selector below `score`:

```rust
fn select_explicit_memories(message: &str, mut items: Vec<ExplicitMemoryItem>) -> Vec<ExplicitMemoryItem> {
    let mut matched = items
        .iter()
        .filter(|item| memory_matches_query(item, message))
        .cloned()
        .collect::<Vec<_>>();
    if matched.is_empty() {
        items.sort_by(|a, b| b.last_used_at.cmp(&a.last_used_at));
        matched = items.into_iter().take(3).collect();
    } else {
        matched.sort_by(|a, b| b.last_used_at.cmp(&a.last_used_at));
        matched.truncate(6);
    }
    matched
}
```

- [ ] **Step 5: Run retriever tests**

Run:

```powershell
cd src-tauri
cargo test memory::retriever::tests
```

Expected: PASS.

---

### Task 6: Feed Explicit Memories Into Consolidation Input

**Files:**
- Modify: `src-tauri/src/memory/consolidator.rs`
- Modify: `src-tauri/src/memory/prompts.rs`

- [ ] **Step 1: Write failing prompt test**

In `src-tauri/src/memory/prompts.rs`, add or extend tests with:

```rust
#[test]
fn consolidation_prompt_includes_explicit_memories() {
    let date = chrono::NaiveDate::from_ymd_opt(2026, 5, 28).unwrap();
    let now = chrono::Utc::now();
    let explicit = vec![crate::models::ExplicitMemoryItem {
        id: "explicit_1".to_string(),
        body: "记住我喜欢简洁回答".to_string(),
        source: "chat".to_string(),
        tags: vec!["explicit_memory_request".to_string()],
        keywords: vec!["简洁".to_string(), "回答".to_string()],
        created_at: now,
        last_used_at: now,
        status: crate::models::ExplicitMemoryStatus::Active,
    }];

    let prompt = consolidation_user_prompt(date, &[], &[], &explicit, &[]);

    assert!(prompt.contains("Explicit immediate memories"));
    assert!(prompt.contains("记住我喜欢简洁回答"));
}
```

- [ ] **Step 2: Run prompt test and verify failure**

Run:

```powershell
cd src-tauri
cargo test memory::prompts::tests::consolidation_prompt_includes_explicit_memories
```

Expected: FAIL because `consolidation_user_prompt` does not accept explicit memories yet.

- [ ] **Step 3: Update prompt signature and body**

In `src-tauri/src/memory/prompts.rs`, update the function signature from:

```rust
pub fn consolidation_user_prompt(
    date: NaiveDate,
    events: &[InteractionEvent],
    working: &[WorkingMemoryItem],
    existing: &[LayeredMemoryItem],
) -> String {
```

to:

```rust
pub fn consolidation_user_prompt(
    date: NaiveDate,
    events: &[InteractionEvent],
    working: &[WorkingMemoryItem],
    explicit: &[ExplicitMemoryItem],
    existing: &[LayeredMemoryItem],
) -> String {
```

Add `ExplicitMemoryItem` to the model imports.

In the prompt body, add this section before existing layered memories:

```rust
    if explicit.is_empty() {
        sections.push("Explicit immediate memories:\n- none".to_string());
    } else {
        sections.push(format!(
            "Explicit immediate memories:\n{}",
            explicit
                .iter()
                .map(|item| format!(
                    "- id={} source={} body={} tags=[{}] keywords=[{}]",
                    item.id,
                    item.source,
                    item.body,
                    item.tags.join(", "),
                    item.keywords.join(", ")
                ))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
```

- [ ] **Step 4: Pass explicit memories from consolidator**

In `src-tauri/src/memory/consolidator.rs`, after loading working memory:

```rust
        let explicit = self.store.list_active_explicit_memories().await?;
```

Update the log:

```rust
                "date={date} existing_memory_count={} working_memory_count={} explicit_memory_count={}",
                existing.len(),
                working.len(),
                explicit.len()
```

Update the prompt call:

```rust
let user_prompt = consolidation_user_prompt(date, &events, &working, &explicit, &existing);
```

- [ ] **Step 5: Run prompt and consolidation tests**

Run:

```powershell
cd src-tauri
cargo test memory::prompts
cargo test memory::consolidator
```

Expected: PASS.

---

### Task 7: Final Verification

**Files:**
- No new files.

- [ ] **Step 1: Run Rust formatting**

Run:

```powershell
cd src-tauri
cargo fmt
```

Expected: command exits 0.

- [ ] **Step 2: Run Rust format check**

Run:

```powershell
cd src-tauri
cargo fmt --check
```

Expected: command exits 0 with no diff output.

- [ ] **Step 3: Run Rust tests**

Run:

```powershell
cd src-tauri
cargo test
```

Expected: all tests pass.

- [ ] **Step 4: Run frontend tests**

Run:

```powershell
npm test
```

Expected: all Vitest suites pass. No frontend code is expected to change, but this catches shared type regressions.

- [ ] **Step 5: Run production build**

Run:

```powershell
npm run build
```

Expected: TypeScript and Vite build complete successfully.

- [ ] **Step 6: Inspect changed files**

Run:

```powershell
git diff -- src-tauri/src/models.rs src-tauri/src/storage.rs src-tauri/src/storage/records.rs src-tauri/src/storage/explicit_memory.rs src-tauri/src/memory/explicit.rs src-tauri/src/memory/mod.rs src-tauri/src/memory/retriever.rs src-tauri/src/memory/consolidator.rs src-tauri/src/memory/prompts.rs src-tauri/src/chat.rs
```

Expected: diff only contains explicit memory model, storage, import, retrieval, consolidation prompt, and focused tests.

---

## Spec Coverage Self-Review

- Immediate next-turn recall: covered by Tasks 3, 4, and 5.
- Provider-independent context: covered by Task 5 because both providers use `reply_with_context`.
- Local matching without LLM: covered by Task 3.
- Long-term consolidation input: covered by Task 6.
- Prompt section separation: covered by Task 5.
- Error handling for import failure: covered by Task 4 logging and non-fatal behavior.
- Consolidation failure keeps immediate memories: covered by design because Task 6 reads explicit memories but does not delete or archive them.

No embedding retrieval, provider-specific sync, Codex session reading, memory editor work, or complex conflict resolution is included.
