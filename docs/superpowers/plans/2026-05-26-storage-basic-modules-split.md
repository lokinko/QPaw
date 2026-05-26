# Storage Basic Modules Split Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move non-layered storage methods out of `src-tauri/src/storage.rs` while preserving the `DocumentStore` API.

**Architecture:** Keep `DocumentStore`, connection setup, migrations, layered memory, and tests in `storage.rs`. Add focused child modules under `src-tauri/src/storage/` that implement methods on `DocumentStore`: `settings.rs`, `conversation.rs`, and `reminder_events.rs`.

**Tech Stack:** Rust, SurrealDB local engine, existing backend storage tests.

---

### Task 1: Move Settings And Avatar Storage

**Files:**
- Create: `src-tauri/src/storage/settings.rs`
- Modify: `src-tauri/src/storage.rs`

- [ ] **Step 1: Run baseline storage tests**

Run: `cd src-tauri; cargo test storage::tests`

Expected: PASS.

- [ ] **Step 2: Move these methods into `storage/settings.rs`**

Move exact existing implementations:

```rust
DocumentStore::get_settings
DocumentStore::save_settings
DocumentStore::save_avatar
```

The module imports `super::{records::*, DocumentStore}`, `crate::debug`, `crate::error::QPawResult`, and `crate::models::{AppSettings, AvatarManifest}`.

- [ ] **Step 3: Register module**

Add `mod settings;` near the top of `storage.rs`.

- [ ] **Step 4: Run storage tests**

Run: `cd src-tauri; cargo test storage::tests`

Expected: PASS.

### Task 2: Move Conversation And Legacy Memory Storage

**Files:**
- Create: `src-tauri/src/storage/conversation.rs`
- Modify: `src-tauri/src/storage.rs`

- [ ] **Step 1: Move these methods into `storage/conversation.rs`**

Move exact existing implementations:

```rust
DocumentStore::append_chat
DocumentStore::list_chat_history
DocumentStore::append_memory
DocumentStore::list_memories
DocumentStore::clear_memory
```

The module imports `super::DocumentStore`, `crate::debug`, `crate::error::QPawResult`, and `crate::models::{ChatMessage, MemoryDocument}`.

- [ ] **Step 2: Register module**

Add `mod conversation;` near the top of `storage.rs`.

- [ ] **Step 3: Run storage tests**

Run: `cd src-tauri; cargo test storage::tests`

Expected: PASS.

### Task 3: Move Habit And Reminder Event Storage

**Files:**
- Create: `src-tauri/src/storage/reminder_events.rs`
- Modify: `src-tauri/src/storage.rs`

- [ ] **Step 1: Move these methods into `storage/reminder_events.rs`**

Move exact existing implementations:

```rust
DocumentStore::append_habit_event
DocumentStore::append_reminder_event
DocumentStore::set_reminder_feedback
```

The module imports `super::DocumentStore`, `chrono::Utc`, `crate::debug`, `crate::error::QPawResult`, and `crate::models::{HabitEvent, ReminderEvent, ReminderFeedbackPayload}`.

- [ ] **Step 2: Register module**

Add `mod reminder_events;` near the top of `storage.rs`.

- [ ] **Step 3: Run full verification**

Run: `cd src-tauri; cargo test`

Expected: PASS.

Run: `npm test`

Expected: PASS.

Run: `npm run build`

Expected: PASS.
