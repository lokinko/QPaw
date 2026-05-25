# Chat Service Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move the chat send workflow out of the Tauri command layer into a focused backend service without changing user-visible behavior.

**Architecture:** Create `src-tauri/src/chat.rs` as the home for chat orchestration. `commands.rs` remains the Tauri boundary and delegates `send_chat_message` to the service. Tests first lock down small extracted behavior before production code changes.

**Tech Stack:** Rust, Tauri commands, existing `DocumentStore`, `MemoryService`, `LlmClient`, `cargo test`.

---

### Task 1: Extract Memory Trigger Policy

**Files:**
- Create: `src-tauri/src/chat.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/commands.rs`

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/chat.rs` with:

```rust
pub fn should_store_legacy_memory(message: &str) -> bool {
    let lowered = message.to_lowercase();
    message.contains("记住")
        || message.contains("记得")
        || lowered.contains("remember")
        || lowered.contains("note that")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_memory_policy_detects_explicit_memory_requests() {
        assert!(should_store_legacy_memory("请记住我下午容易忘记喝水"));
        assert!(should_store_legacy_memory("remember that I prefer concise replies"));
        assert!(should_store_legacy_memory("note that I work best in the morning"));
        assert!(!should_store_legacy_memory("今天只是随便聊聊"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd src-tauri; cargo test chat::tests::legacy_memory_policy_detects_explicit_memory_requests`

Expected: FAIL because `chat` is not yet registered as a crate module.

- [ ] **Step 3: Register module**

Add `mod chat;` to `src-tauri/src/lib.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cd src-tauri; cargo test chat::tests::legacy_memory_policy_detects_explicit_memory_requests`

Expected: PASS.

- [ ] **Step 5: Switch command to use policy**

In `src-tauri/src/commands.rs`, import `crate::chat::should_store_legacy_memory`, replace `should_store_memory(&clean)` with `should_store_legacy_memory(&clean)`, and remove the private `should_store_memory` function.

- [ ] **Step 6: Run backend tests**

Run: `cd src-tauri; cargo test`

Expected: PASS.

### Task 2: Move Chat Send Workflow Into Service

**Files:**
- Modify: `src-tauri/src/chat.rs`
- Modify: `src-tauri/src/commands.rs`

- [ ] **Step 1: Move orchestration code**

Create a public async function in `chat.rs`:

```rust
pub async fn send_chat_message(message: String, state: &crate::AppState) -> crate::error::QPawResult<crate::models::SendChatResponse>
```

Move the existing command body into this function, preserving logging scopes and behavior.

- [ ] **Step 2: Delegate command**

In `commands.rs`, reduce the Tauri command to:

```rust
#[tauri::command]
pub async fn send_chat_message(
    message: String,
    state: State<'_, AppState>,
) -> QPawResult<SendChatResponse> {
    crate::chat::send_chat_message(message, state.inner()).await
}
```

- [ ] **Step 3: Run backend tests**

Run: `cd src-tauri; cargo test`

Expected: PASS.

- [ ] **Step 4: Run frontend tests and build**

Run: `npm test`

Expected: PASS.

Run: `npm run build`

Expected: PASS.
