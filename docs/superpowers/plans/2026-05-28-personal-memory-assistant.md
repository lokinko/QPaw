# Personal Memory Assistant Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the first personal memory assistant slice: typed settings, a pure interruptibility policy, local rule-based memory decisions, and chat integration that can save, ignore, or ask before saving.

**Architecture:** Keep the first slice backend-heavy and testable. Add focused Rust modules for `interruptibility` and `memory_decision`, extend existing shared models, then wire `memory_decision` into `chat::send_chat_message` without replacing the existing memory service or reminder runtime. Frontend changes are limited to typed settings/fallback defaults and displaying an optional memory-confirmation prompt from chat responses.

**Tech Stack:** Rust, Tauri 2, Serde, Chrono, React, TypeScript, Vitest, Cargo tests.

---

## File Structure

- Create `src-tauri/src/interruptibility.rs`: pure policy types and `evaluate_interruptibility`.
- Create `src-tauri/src/memory_decision.rs`: local rule-based memory decision types and `decide_memory`.
- Modify `src-tauri/src/models.rs`: settings enums/structs, chat memory decision response payload, and defaults.
- Modify `src-tauri/src/lib.rs`: register new Rust modules.
- Modify `src-tauri/src/chat.rs`: call `memory_decision` and expose the result in `SendChatResponse`.
- Modify `src/lib/types.ts`: mirror new settings and chat response types.
- Modify `src/lib/fallback.ts`: add default personal memory assistant settings and fallback chat memory-decision behavior.
- Modify `src/components/ChatPanel.tsx`: display memory confirmation prompt copy when returned.
- Modify `src/components/PetWindow.tsx`: include confirmation prompt in quick chat reply text.
- Modify `src/components/SettingsWindow.tsx`: add minimal personal memory assistant controls.

The first implementation does not add the proactive loop or Windows fullscreen API. It reserves the settings and policy shape, then implements the policy as pure logic so fullscreen integration can be added safely in the next slice.

### Task 1: Add Personal Memory Assistant Settings

**Files:**
- Modify: `src-tauri/src/models.rs`
- Modify: `src/lib/types.ts`
- Modify: `src/lib/fallback.ts`

- [ ] **Step 1: Write the failing Rust settings default test**

Add these imports and tests in `src-tauri/src/models.rs` inside the existing `#[cfg(test)] mod tests`:

```rust
use super::{
    AppSettings, AvatarKind, FullscreenBehavior, MemorySensitivity,
};
```

Replace the current `use super::{AppSettings, AvatarKind};` line with the import above, then add:

```rust
#[test]
fn default_personal_memory_assistant_settings_are_low_interruption() {
    let settings = AppSettings::default();

    assert!(settings.personal_memory.enabled);
    assert_eq!(settings.personal_memory.daily_prompt_limit, 2);
    assert_eq!(settings.personal_memory.idle_threshold_seconds, 180);
    assert_eq!(
        settings.personal_memory.fullscreen_behavior,
        FullscreenBehavior::Hide
    );
    assert_eq!(
        settings.personal_memory.memory_sensitivity,
        MemorySensitivity::Balanced
    );
    assert!(settings.personal_memory.allow_confirmation_questions);
    assert!(!settings.personal_memory.allow_low_confidence_in_review);
    assert_eq!(settings.personal_memory.allowed_windows.len(), 2);
    assert_eq!(settings.personal_memory.allowed_windows[0].start, "13:30");
    assert_eq!(settings.personal_memory.allowed_windows[0].end, "16:30");
    assert_eq!(settings.personal_memory.allowed_windows[1].start, "20:00");
    assert_eq!(settings.personal_memory.allowed_windows[1].end, "23:00");
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```powershell
cd src-tauri; cargo test models::tests::default_personal_memory_assistant_settings_are_low_interruption
```

Expected: FAIL because `personal_memory`, `FullscreenBehavior`, and `MemorySensitivity` are not defined.

- [ ] **Step 3: Add Rust settings types and defaults**

In `src-tauri/src/models.rs`, add these types after `MemorySettings`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FullscreenBehavior {
    Hide,
    Silent,
    Off,
}

fn default_fullscreen_behavior() -> FullscreenBehavior {
    FullscreenBehavior::Hide
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemorySensitivity {
    Conservative,
    Balanced,
    Active,
}

fn default_memory_sensitivity() -> MemorySensitivity {
    MemorySensitivity::Balanced
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersonalMemoryWindow {
    pub start: String,
    pub end: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalMemorySettings {
    pub enabled: bool,
    #[serde(default = "default_daily_prompt_limit")]
    pub daily_prompt_limit: u8,
    #[serde(default = "default_personal_memory_windows")]
    pub allowed_windows: Vec<PersonalMemoryWindow>,
    #[serde(default = "default_idle_threshold_seconds")]
    pub idle_threshold_seconds: u64,
    #[serde(default = "default_fullscreen_behavior")]
    pub fullscreen_behavior: FullscreenBehavior,
    #[serde(default = "default_memory_sensitivity")]
    pub memory_sensitivity: MemorySensitivity,
    #[serde(default = "default_allow_confirmation_questions")]
    pub allow_confirmation_questions: bool,
    #[serde(default)]
    pub allow_low_confidence_in_review: bool,
}

impl Default for PersonalMemorySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            daily_prompt_limit: default_daily_prompt_limit(),
            allowed_windows: default_personal_memory_windows(),
            idle_threshold_seconds: default_idle_threshold_seconds(),
            fullscreen_behavior: default_fullscreen_behavior(),
            memory_sensitivity: default_memory_sensitivity(),
            allow_confirmation_questions: default_allow_confirmation_questions(),
            allow_low_confidence_in_review: false,
        }
    }
}

fn default_daily_prompt_limit() -> u8 {
    2
}

fn default_personal_memory_windows() -> Vec<PersonalMemoryWindow> {
    vec![
        PersonalMemoryWindow {
            start: "13:30".to_string(),
            end: "16:30".to_string(),
        },
        PersonalMemoryWindow {
            start: "20:00".to_string(),
            end: "23:00".to_string(),
        },
    ]
}

fn default_idle_threshold_seconds() -> u64 {
    180
}

fn default_allow_confirmation_questions() -> bool {
    true
}
```

Add the field to `AppSettings`:

```rust
#[serde(default)]
pub personal_memory: PersonalMemorySettings,
```

Add the default value in `impl Default for AppSettings`:

```rust
personal_memory: PersonalMemorySettings::default(),
```

- [ ] **Step 4: Run the Rust settings test to verify it passes**

Run:

```powershell
cd src-tauri; cargo test models::tests::default_personal_memory_assistant_settings_are_low_interruption
```

Expected: PASS.

- [ ] **Step 5: Mirror settings types in TypeScript**

In `src/lib/types.ts`, add:

```ts
export type FullscreenBehavior = "hide" | "silent" | "off";
export type MemorySensitivity = "conservative" | "balanced" | "active";

export interface PersonalMemoryWindow {
  start: string;
  end: string;
}

export interface PersonalMemorySettings {
  enabled: boolean;
  daily_prompt_limit: number;
  allowed_windows: PersonalMemoryWindow[];
  idle_threshold_seconds: number;
  fullscreen_behavior: FullscreenBehavior;
  memory_sensitivity: MemorySensitivity;
  allow_confirmation_questions: boolean;
  allow_low_confidence_in_review: boolean;
}
```

Add to `AppSettings`:

```ts
personal_memory: PersonalMemorySettings;
```

- [ ] **Step 6: Add fallback defaults**

In `src/lib/fallback.ts`, add `personal_memory` to `fallbackSettings`:

```ts
personal_memory: {
  enabled: true,
  daily_prompt_limit: 2,
  allowed_windows: [
    { start: "13:30", end: "16:30" },
    { start: "20:00", end: "23:00" },
  ],
  idle_threshold_seconds: 180,
  fullscreen_behavior: "hide",
  memory_sensitivity: "balanced",
  allow_confirmation_questions: true,
  allow_low_confidence_in_review: false,
},
```

- [ ] **Step 7: Run typecheck build**

Run:

```powershell
npm run build
```

Expected: PASS.

### Task 2: Add Interruptibility Policy

**Files:**
- Create: `src-tauri/src/interruptibility.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write the failing policy tests**

Create `src-tauri/src/interruptibility.rs` with tests first:

```rust
use chrono::NaiveTime;

use crate::models::{FullscreenBehavior, PersonalMemorySettings, PersonalMemoryWindow};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Interruptibility {
    Available,
    Disabled,
    FullscreenHidden,
    OutsideAllowedWindow,
    RecentlyActive,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::MemorySensitivity;

    fn settings() -> PersonalMemorySettings {
        PersonalMemorySettings {
            enabled: true,
            daily_prompt_limit: 2,
            allowed_windows: vec![PersonalMemoryWindow {
                start: "13:30".to_string(),
                end: "16:30".to_string(),
            }],
            idle_threshold_seconds: 180,
            fullscreen_behavior: FullscreenBehavior::Hide,
            memory_sensitivity: MemorySensitivity::Balanced,
            allow_confirmation_questions: true,
            allow_low_confidence_in_review: false,
        }
    }

    #[test]
    fn disabled_settings_block_prompts() {
        let mut settings = settings();
        settings.enabled = false;

        assert_eq!(
            evaluate_interruptibility(&settings, true, 300, time("14:00")),
            Interruptibility::Disabled
        );
    }

    #[test]
    fn fullscreen_hide_blocks_before_idle_or_time_checks() {
        assert_eq!(
            evaluate_interruptibility(&settings(), true, 0, time("09:00")),
            Interruptibility::FullscreenHidden
        );
    }

    #[test]
    fn outside_allowed_window_blocks_prompts() {
        assert_eq!(
            evaluate_interruptibility(&settings(), false, 300, time("09:00")),
            Interruptibility::OutsideAllowedWindow
        );
    }

    #[test]
    fn recent_activity_blocks_inside_allowed_window() {
        assert_eq!(
            evaluate_interruptibility(&settings(), false, 60, time("14:00")),
            Interruptibility::RecentlyActive
        );
    }

    #[test]
    fn allowed_window_and_idle_threshold_make_qpaw_available() {
        assert_eq!(
            evaluate_interruptibility(&settings(), false, 180, time("14:00")),
            Interruptibility::Available
        );
    }

    fn time(value: &str) -> NaiveTime {
        NaiveTime::parse_from_str(value, "%H:%M").unwrap()
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```powershell
cd src-tauri; cargo test interruptibility::tests
```

Expected: FAIL because `interruptibility` is not registered and `evaluate_interruptibility` does not exist.

- [ ] **Step 3: Register module and implement minimal policy**

Add to `src-tauri/src/lib.rs` near the other modules:

```rust
mod interruptibility;
```

In `src-tauri/src/interruptibility.rs`, add above the test module:

```rust
pub fn evaluate_interruptibility(
    settings: &PersonalMemorySettings,
    is_fullscreen: bool,
    idle_seconds: u64,
    now: NaiveTime,
) -> Interruptibility {
    if !settings.enabled {
        return Interruptibility::Disabled;
    }

    if is_fullscreen && settings.fullscreen_behavior == FullscreenBehavior::Hide {
        return Interruptibility::FullscreenHidden;
    }

    if !inside_any_allowed_window(&settings.allowed_windows, now) {
        return Interruptibility::OutsideAllowedWindow;
    }

    if idle_seconds < settings.idle_threshold_seconds {
        return Interruptibility::RecentlyActive;
    }

    Interruptibility::Available
}

fn inside_any_allowed_window(windows: &[PersonalMemoryWindow], now: NaiveTime) -> bool {
    windows.iter().any(|window| {
        let Ok(start) = NaiveTime::parse_from_str(&window.start, "%H:%M") else {
            return false;
        };
        let Ok(end) = NaiveTime::parse_from_str(&window.end, "%H:%M") else {
            return false;
        };

        if start <= end {
            now >= start && now <= end
        } else {
            now >= start || now <= end
        }
    })
}
```

- [ ] **Step 4: Run policy tests**

Run:

```powershell
cd src-tauri; cargo test interruptibility::tests
```

Expected: PASS.

### Task 3: Add Local Memory Decision Rules

**Files:**
- Create: `src-tauri/src/memory_decision.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing memory decision tests**

Create `src-tauri/src/memory_decision.rs` with:

```rust
use crate::models::MemorySensitivity;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryDecisionAction {
    Ignore,
    Save,
    Ask,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryDecision {
    pub action: MemoryDecisionAction,
    pub reason: String,
    pub tags: Vec<String>,
    pub confirmation_prompt: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_memory_request_is_saved() {
        let decision = decide_memory("请记住我下午容易忘记喝水", MemorySensitivity::Balanced, true);

        assert_eq!(decision.action, MemoryDecisionAction::Save);
        assert!(decision.reason.contains("explicit"));
        assert!(decision.tags.contains(&"explicit_memory_request".to_string()));
        assert_eq!(decision.confirmation_prompt, None);
    }

    #[test]
    fn ordinary_greeting_is_ignored() {
        let decision = decide_memory("你好", MemorySensitivity::Balanced, true);

        assert_eq!(decision.action, MemoryDecisionAction::Ignore);
        assert!(decision.tags.is_empty());
    }

    #[test]
    fn preference_statement_is_saved() {
        let decision = decide_memory("我喜欢你提醒的时候短一点", MemorySensitivity::Balanced, true);

        assert_eq!(decision.action, MemoryDecisionAction::Save);
        assert!(decision.tags.contains(&"preference".to_string()));
    }

    #[test]
    fn uncertain_body_state_asks_for_confirmation_when_allowed() {
        let decision = decide_memory("今天肩颈有点不舒服", MemorySensitivity::Balanced, true);

        assert_eq!(decision.action, MemoryDecisionAction::Ask);
        assert!(decision.tags.contains(&"personal_state".to_string()));
        assert_eq!(
            decision.confirmation_prompt.as_deref(),
            Some("这件事以后可能有用，要我记一下吗？")
        );
    }

    #[test]
    fn active_sensitivity_saves_personal_state_signal() {
        let decision = decide_memory("最近总是睡不好", MemorySensitivity::Active, true);

        assert_eq!(decision.action, MemoryDecisionAction::Save);
        assert!(decision.tags.contains(&"personal_state".to_string()));
    }

    #[test]
    fn confirmation_disabled_ignores_uncertain_state() {
        let decision = decide_memory("今天肩颈有点不舒服", MemorySensitivity::Balanced, false);

        assert_eq!(decision.action, MemoryDecisionAction::Ignore);
        assert_eq!(decision.confirmation_prompt, None);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```powershell
cd src-tauri; cargo test memory_decision::tests
```

Expected: FAIL because `memory_decision` is not registered and `decide_memory` does not exist.

- [ ] **Step 3: Register module and implement local rules**

Add to `src-tauri/src/lib.rs`:

```rust
mod memory_decision;
```

Add this implementation above the tests in `src-tauri/src/memory_decision.rs`:

```rust
pub fn decide_memory(
    text: &str,
    sensitivity: MemorySensitivity,
    allow_confirmation_questions: bool,
) -> MemoryDecision {
    let trimmed = text.trim();
    let lowered = trimmed.to_lowercase();

    if trimmed.is_empty() || is_low_context_message(trimmed) {
        return MemoryDecision {
            action: MemoryDecisionAction::Ignore,
            reason: "low_context".to_string(),
            tags: vec![],
            confirmation_prompt: None,
        };
    }

    if contains_any(trimmed, &["记住", "记得", "记一下", "以后提醒我"])
        || contains_any(&lowered, &["remember", "note that"])
    {
        return MemoryDecision {
            action: MemoryDecisionAction::Save,
            reason: "explicit_memory_request".to_string(),
            tags: vec!["explicit_memory_request".to_string()],
            confirmation_prompt: None,
        };
    }

    if contains_any(trimmed, &["我喜欢", "我不喜欢", "我希望", "不要", "别"]) {
        return MemoryDecision {
            action: MemoryDecisionAction::Save,
            reason: "preference_statement".to_string(),
            tags: vec!["preference".to_string()],
            confirmation_prompt: None,
        };
    }

    if contains_any(
        trimmed,
        &[
            "睡不好",
            "睡眠",
            "疲惫",
            "很累",
            "没精神",
            "焦虑",
            "压力",
            "疼",
            "不舒服",
            "肩颈",
            "头痛",
            "胃",
        ],
    ) {
        if sensitivity == MemorySensitivity::Active || contains_any(trimmed, &["最近", "总是", "连续", "这几天"]) {
            return MemoryDecision {
                action: MemoryDecisionAction::Save,
                reason: "recurring_personal_state_signal".to_string(),
                tags: vec!["personal_state".to_string()],
                confirmation_prompt: None,
            };
        }

        if allow_confirmation_questions {
            return MemoryDecision {
                action: MemoryDecisionAction::Ask,
                reason: "possible_personal_state_signal".to_string(),
                tags: vec!["personal_state".to_string()],
                confirmation_prompt: Some("这件事以后可能有用，要我记一下吗？".to_string()),
            };
        }
    }

    MemoryDecision {
        action: MemoryDecisionAction::Ignore,
        reason: "no_memory_signal".to_string(),
        tags: vec![],
        confirmation_prompt: None,
    }
}

fn is_low_context_message(text: &str) -> bool {
    matches!(
        text,
        "你好" | "早" | "早上好" | "嗯" | "哦" | "好" | "谢谢" | "hi" | "hello"
    )
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}
```

- [ ] **Step 4: Run memory decision tests**

Run:

```powershell
cd src-tauri; cargo test memory_decision::tests
```

Expected: PASS.

### Task 4: Integrate Memory Decisions Into Chat

**Files:**
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/chat.rs`
- Modify: `src/lib/types.ts`
- Modify: `src/lib/fallback.ts`

- [ ] **Step 1: Write failing response serialization test**

In `src-tauri/src/models.rs`, add these types near `SendChatResponse`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMemoryDecision {
    pub action: String,
    pub reason: String,
    pub tags: Vec<String>,
    pub confirmation_prompt: Option<String>,
}
```

Then add this field to `SendChatResponse`:

```rust
#[serde(default)]
pub memory_decision: Option<ChatMemoryDecision>,
```

Add this test to `models` tests:

```rust
#[test]
fn chat_memory_decision_serializes_confirmation_prompt() {
    let decision = super::ChatMemoryDecision {
        action: "ask".to_string(),
        reason: "possible_personal_state_signal".to_string(),
        tags: vec!["personal_state".to_string()],
        confirmation_prompt: Some("这件事以后可能有用，要我记一下吗？".to_string()),
    };

    let encoded = serde_json::to_value(decision).expect("serialize memory decision");

    assert_eq!(encoded["action"], "ask");
    assert_eq!(
        encoded["confirmation_prompt"],
        serde_json::json!("这件事以后可能有用，要我记一下吗？")
    );
}
```

- [ ] **Step 2: Run model test**

Run:

```powershell
cd src-tauri; cargo test models::tests::chat_memory_decision_serializes_confirmation_prompt
```

Expected: PASS after adding the type and field. If it fails due to missing `super::ChatMemoryDecision`, fix the import path by using `use super::ChatMemoryDecision;` in the test module.

- [ ] **Step 3: Wire decision into chat**

In `src-tauri/src/chat.rs`, update imports:

```rust
use crate::memory_decision::{decide_memory, MemoryDecisionAction};
use crate::models::{
    ChatMemoryDecision, ChatMessage, ChatRole, InteractionEventKind, MemoryDocument,
    SendChatResponse,
};
```

After settings are loaded and before legacy memory handling, add:

```rust
let memory_decision = decide_memory(
    &clean,
    settings.personal_memory.memory_sensitivity.clone(),
    settings.personal_memory.allow_confirmation_questions,
);
debug::log(
    "chat:send",
    format!(
        "trace_id={trace_id} memory_decision={:?} reason={}",
        memory_decision.action, memory_decision.reason
    ),
);
```

Change legacy memory saving from:

```rust
if should_store_legacy_memory(&clean) {
```

to:

```rust
if should_store_legacy_memory(&clean) || memory_decision.action == MemoryDecisionAction::Save {
```

Add a `ChatMemoryDecision` value before the final `Ok`:

```rust
let chat_memory_decision = ChatMemoryDecision {
    action: match memory_decision.action {
        MemoryDecisionAction::Ignore => "ignore",
        MemoryDecisionAction::Save => "save",
        MemoryDecisionAction::Ask => "ask",
    }
    .to_string(),
    reason: memory_decision.reason,
    tags: memory_decision.tags,
    confirmation_prompt: memory_decision.confirmation_prompt,
};
```

Return it:

```rust
Ok(SendChatResponse {
    user,
    assistant,
    memories,
    memory_decision: Some(chat_memory_decision),
})
```

- [ ] **Step 4: Run focused backend tests**

Run:

```powershell
cd src-tauri; cargo test chat::tests memory_decision::tests models::tests::chat_memory_decision_serializes_confirmation_prompt
```

Expected: PASS.

- [ ] **Step 5: Mirror chat response types in TypeScript**

In `src/lib/types.ts`, add:

```ts
export type ChatMemoryDecisionAction = "ignore" | "save" | "ask";

export interface ChatMemoryDecision {
  action: ChatMemoryDecisionAction;
  reason: string;
  tags: string[];
  confirmation_prompt: string | null;
}
```

Add to `SendChatResponse`:

```ts
memory_decision?: ChatMemoryDecision | null;
```

- [ ] **Step 6: Add fallback decision response**

In `src/lib/fallback.ts`, in the `send_chat_message` case, add a small local decision object:

```ts
const memoryDecision =
  /记住|记得|记一下|以后提醒我|remember|note that/i.test(message)
    ? {
        action: "save" as const,
        reason: "explicit_memory_request",
        tags: ["explicit_memory_request"],
        confirmation_prompt: null,
      }
    : /睡不好|睡眠|疲惫|很累|没精神|焦虑|压力|疼|不舒服|肩颈|头痛|胃/.test(message)
      ? {
          action: "ask" as const,
          reason: "possible_personal_state_signal",
          tags: ["personal_state"],
          confirmation_prompt: "这件事以后可能有用，要我记一下吗？",
        }
      : {
          action: "ignore" as const,
          reason: "no_memory_signal",
          tags: [],
          confirmation_prompt: null,
        };
```

Return it in the fallback `SendChatResponse`:

```ts
memory_decision: memoryDecision,
```

- [ ] **Step 7: Run frontend typecheck build**

Run:

```powershell
npm run build
```

Expected: PASS.

### Task 5: Show Memory Confirmation In Chat UI

**Files:**
- Modify: `src/components/ChatPanel.tsx`
- Modify: `src/components/PetWindow.tsx`

- [ ] **Step 1: Write failing ChatPanel test**

If no `ChatPanel` test exists, create `src/components/ChatPanel.test.tsx`:

```tsx
import { renderToString } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { ChatPanel } from "./ChatPanel";

describe("ChatPanel", () => {
  it("has a region for memory confirmation status", () => {
    const markup = renderToString(<ChatPanel />);

    expect(markup).toContain("chat-memory-decision");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```powershell
npm test -- src/components/ChatPanel.test.tsx
```

Expected: FAIL because `chat-memory-decision` does not exist in the rendered markup.

- [ ] **Step 3: Add confirmation status rendering**

In `src/components/ChatPanel.tsx`, add state:

```ts
const [memoryPrompt, setMemoryPrompt] = useState<string | null>(null);
```

In `send`, after `const response = await api.sendChatMessage(message);`, add:

```ts
setMemoryPrompt(response.memory_decision?.confirmation_prompt ?? null);
```

In the catch block, add:

```ts
setMemoryPrompt(null);
```

Render after the existing status line:

```tsx
<p className="chat-memory-decision" aria-live="polite">
  {memoryPrompt ?? ""}
</p>
```

- [ ] **Step 4: Run ChatPanel test**

Run:

```powershell
npm test -- src/components/ChatPanel.test.tsx
```

Expected: PASS.

- [ ] **Step 5: Add quick chat confirmation prompt**

In `src/components/PetWindow.tsx`, after `const response = await api.sendChatMessage(message);`, replace:

```ts
setReply(response.assistant.content);
```

with:

```ts
setReply(
  response.memory_decision?.confirmation_prompt
    ? `${response.assistant.content}\n${response.memory_decision.confirmation_prompt}`
    : response.assistant.content,
);
```

- [ ] **Step 6: Run frontend tests**

Run:

```powershell
npm test
```

Expected: PASS.

### Task 6: Add Minimal Settings Controls

**Files:**
- Modify: `src/components/SettingsWindow.tsx`
- Modify: `src/styles.css`

- [ ] **Step 1: Write failing static settings test**

Create `src/components/SettingsWindow.test.tsx` if it does not exist:

```tsx
import { renderToString } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { SettingsWindow } from "./SettingsWindow";

describe("SettingsWindow", () => {
  it("renders personal memory assistant settings", () => {
    const markup = renderToString(<SettingsWindow />);

    expect(markup).toContain("个人记忆助理");
    expect(markup).toContain("每日主动上限");
    expect(markup).toContain("空闲阈值");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```powershell
npm test -- src/components/SettingsWindow.test.tsx
```

Expected: FAIL because those labels are not rendered.

- [ ] **Step 3: Add settings section**

In `src/components/SettingsWindow.tsx`, add a settings card near the memory section:

```tsx
<section className="settings-card">
  <header>
    <h2>个人记忆助理</h2>
    <p>QPaw 会在低打扰时机询问近况，并判断哪些表达值得记住。</p>
  </header>
  <label className="toggle-row">
    <input
      type="checkbox"
      checked={settings.personal_memory.enabled}
      onChange={(event) =>
        save({
          ...settings,
          personal_memory: {
            ...settings.personal_memory,
            enabled: event.target.checked,
          },
        })
      }
    />
    启用主动关心
  </label>
  <label>
    每日主动上限
    <input
      type="number"
      min={0}
      max={6}
      value={settings.personal_memory.daily_prompt_limit}
      onChange={(event) =>
        save({
          ...settings,
          personal_memory: {
            ...settings.personal_memory,
            daily_prompt_limit: Number(event.target.value),
          },
        })
      }
    />
  </label>
  <label>
    空闲阈值（秒）
    <input
      type="number"
      min={30}
      step={30}
      value={settings.personal_memory.idle_threshold_seconds}
      onChange={(event) =>
        save({
          ...settings,
          personal_memory: {
            ...settings.personal_memory,
            idle_threshold_seconds: Number(event.target.value),
          },
        })
      }
    />
  </label>
  <label>
    记忆敏感度
    <select
      value={settings.personal_memory.memory_sensitivity}
      onChange={(event) =>
        save({
          ...settings,
          personal_memory: {
            ...settings.personal_memory,
            memory_sensitivity: event.target.value as typeof settings.personal_memory.memory_sensitivity,
          },
        })
      }
    >
      <option value="conservative">保守</option>
      <option value="balanced">平衡</option>
      <option value="active">积极</option>
    </select>
  </label>
</section>
```

- [ ] **Step 4: Add small CSS only if layout needs it**

If `select` has no matching width, add this to `src/styles.css` near existing input styles:

```css
select {
  width: 100%;
  border: 1px solid #cad8d6;
  border-radius: 8px;
  padding: 9px 10px;
  color: #243438;
  background: #ffffff;
}
```

- [ ] **Step 5: Run settings test**

Run:

```powershell
npm test -- src/components/SettingsWindow.test.tsx
```

Expected: PASS.

### Task 7: Full Verification

**Files:**
- All modified files from Tasks 1-6.

- [ ] **Step 1: Run backend tests**

Run:

```powershell
cd src-tauri; cargo test
```

Expected: PASS.

- [ ] **Step 2: Run frontend tests**

Run:

```powershell
npm test
```

Expected: PASS.

- [ ] **Step 3: Run frontend build**

Run:

```powershell
npm run build
```

Expected: PASS.

- [ ] **Step 4: Review git diff**

Run:

```powershell
git diff --stat
git diff -- docs/superpowers/plans/2026-05-28-personal-memory-assistant.md src-tauri/src/models.rs src-tauri/src/interruptibility.rs src-tauri/src/memory_decision.rs src-tauri/src/chat.rs src-tauri/src/lib.rs src/lib/types.ts src/lib/fallback.ts src/components/ChatPanel.tsx src/components/PetWindow.tsx src/components/SettingsWindow.tsx src/styles.css
```

Expected: Diff only contains the implementation plan and personal memory assistant changes. Pre-existing Codex status and avatar work may still appear in `git status`, but should not be staged or reverted.

- [ ] **Step 5: Commit only this implementation slice when the user asks for a commit**

Stage only files touched for this slice:

```powershell
git add docs/superpowers/plans/2026-05-28-personal-memory-assistant.md src-tauri/src/models.rs src-tauri/src/interruptibility.rs src-tauri/src/memory_decision.rs src-tauri/src/chat.rs src-tauri/src/lib.rs src/lib/types.ts src/lib/fallback.ts src/components/ChatPanel.tsx src/components/ChatPanel.test.tsx src/components/PetWindow.tsx src/components/SettingsWindow.tsx src/components/SettingsWindow.test.tsx src/styles.css
git commit -m "feat: add personal memory decision slice"
```

Do not stage unrelated pre-existing files such as `src-tauri/src/codex_dev.rs` unless they are already part of the current user-approved work.
