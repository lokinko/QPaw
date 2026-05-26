# Storage Working Memory Split Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move working-memory persistence methods out of `src-tauri/src/storage.rs` into a focused child module without changing the public `DocumentStore` API.

**Architecture:** Keep `DocumentStore`, database connection setup, schema migration, raw interaction events, layered memory, consolidation, and storage tests in `storage.rs` for this step. Add `src-tauri/src/storage/working_memory.rs` with an `impl DocumentStore` block for `WorkingMemoryItem` persistence and cleanup methods. The new module will use existing `records::*` conversions and keep behavior identical.

**Tech Stack:** Rust, Tauri, SurrealDB local engine, existing `QPawResult` error type.

---

### Task 1: Create Working Memory Storage Module

**Files:**
- Create: `src-tauri/src/storage/working_memory.rs`
- Modify: `src-tauri/src/storage.rs`

- [ ] **Step 1: Add module declaration**

Add the child module beside the other storage modules:

```rust
mod conversation;
mod records;
mod reminder_events;
mod settings;
mod working_memory;
```

- [ ] **Step 2: Move working-memory imports**

Remove `WorkingMemoryItem` from the top-level `crate::models` import in `storage.rs`. The new child module should import:

```rust
use chrono::{NaiveDate, Utc};

use super::{records::*, DocumentStore};
use crate::debug;
use crate::error::QPawResult;
use crate::models::WorkingMemoryItem;
```

- [ ] **Step 3: Move methods unchanged**

Move these methods from `impl DocumentStore` in `storage.rs` into `src-tauri/src/storage/working_memory.rs`:

```rust
pub async fn save_working_memory(&self, item: &WorkingMemoryItem) -> QPawResult<()>
pub async fn list_working_memory(&self) -> QPawResult<Vec<WorkingMemoryItem>>
pub async fn list_active_working_memory(&self) -> QPawResult<Vec<WorkingMemoryItem>>
pub async fn list_working_memory_for_date(&self, date: NaiveDate) -> QPawResult<Vec<WorkingMemoryItem>>
pub async fn clear_working_memory(&self) -> QPawResult<()>
pub async fn clear_working_memory_for_date(&self, date: NaiveDate) -> QPawResult<()>
pub async fn cleanup_expired_working_memory(&self) -> QPawResult<usize>
async fn delete_working_memory_by_id(&self, id: &str) -> QPawResult<()>
```

Expected behavior remains unchanged:

- `save_working_memory` deletes an existing row by business `uid` before inserting.
- `list_working_memory` sorts by `updated_at` descending.
- `list_active_working_memory` filters `expires_at > Utc::now()`.
- `cleanup_expired_working_memory` deletes expired rows and returns the count.

- [ ] **Step 4: Verify storage tests**

Run:

```powershell
cargo test storage::tests
```

Expected: all storage tests pass.

- [ ] **Step 5: Verify full project**

Run:

```powershell
cargo test
npm test
npm run build
```

Expected: all commands exit 0.

- [ ] **Step 6: Commit only this split**

Stage only the plan and backend storage files:

```powershell
git add docs/superpowers/plans/2026-05-26-storage-working-memory-split.md src-tauri/src/storage.rs src-tauri/src/storage/working_memory.rs
git commit -m "refactor: split working memory storage"
```

Do not stage the pre-existing frontend/avatar working tree changes in this commit.
