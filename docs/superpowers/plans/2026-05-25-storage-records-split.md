# Storage Records Split Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reduce `src-tauri/src/storage.rs` size by moving SurrealDB record DTOs and conversion implementations into a focused storage records module.

**Architecture:** Keep `DocumentStore` and its public API in `storage.rs`. Add `src-tauri/src/storage/records.rs` as an internal submodule declared from `storage.rs`; storage methods continue to use the same record type names through `use records::*`.

**Tech Stack:** Rust, SurrealDB local engine, existing backend storage tests.

---

### Task 1: Extract Record Types

**Files:**
- Create: `src-tauri/src/storage/records.rs`
- Modify: `src-tauri/src/storage.rs`

- [ ] **Step 1: Run baseline storage tests**

Run: `cd src-tauri; cargo test storage::tests`

Expected: PASS before the refactor starts.

- [ ] **Step 2: Move record DTOs and conversion impls**

Move these definitions from `storage.rs` into `storage/records.rs`:

```rust
SchemaMigrationRecord
AvatarManifestRecord
InteractionEventRecord
WorkingMemoryRecord
MemoryL0Record
MemoryL1ConceptRecord
MemoryL1RelationRecord
MemoryL2EventRecord
MemoryL3ReflectionRecord
MemoryConsolidationJobRecord
impl From / TryFrom blocks for those record types
impl From<MemoryL0 | MemoryL1Concept | MemoryL1Relation | MemoryL2Event | MemoryL3Reflection> for LayeredMemoryItem
```

Each moved type must be `pub(super)` so `storage.rs` can use it but the rest of the crate does not grow a new public API.

- [ ] **Step 3: Wire the module**

At the top of `storage.rs`, add:

```rust
mod records;

use records::*;
```

Remove imports from `storage.rs` that are only needed by `records.rs`.

- [ ] **Step 4: Run storage tests**

Run: `cd src-tauri; cargo test storage::tests`

Expected: PASS.

- [ ] **Step 5: Run full verification**

Run: `cd src-tauri; cargo test`

Expected: PASS.

Run: `npm test`

Expected: PASS.

Run: `npm run build`

Expected: PASS.
