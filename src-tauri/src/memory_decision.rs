#![allow(dead_code)]

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
        if sensitivity == MemorySensitivity::Active
            || contains_any(trimmed, &["最近", "总是", "连续", "这几天"])
        {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_memory_request_is_saved() {
        let decision = decide_memory(
            "请记住我下午容易忘记喝水",
            MemorySensitivity::Balanced,
            true,
        );

        assert_eq!(decision.action, MemoryDecisionAction::Save);
        assert!(decision.reason.contains("explicit"));
        assert!(decision
            .tags
            .contains(&"explicit_memory_request".to_string()));
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
        let decision = decide_memory(
            "我喜欢你提醒的时候短一点",
            MemorySensitivity::Balanced,
            true,
        );

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
