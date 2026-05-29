use chrono::Utc;
use uuid::Uuid;

use crate::models::{ExplicitMemoryItem, ExplicitMemoryStatus};

const STOP_WORDS: &[&str] = &[
    "remember",
    "that",
    "please",
    "prefer",
    "记住",
    "记得",
    "记一下",
    "我",
    "喜欢",
];

const KNOWN_TERMS: &[&str] = &[
    "简洁", "回答", "提醒", "安静", "上午", "下午", "晚上", "睡眠", "喝水", "肩颈", "项目", "生日",
    "上海", "小王", "咖啡", "名字", "称呼", "住在", "城市", "茶",
];

const CJK_VALUE_MARKERS: &[&str] = &["生日是", "住在", "叫我", "喜欢", "名字是", "称呼"];

pub fn normalize_explicit_body(body: &str) -> String {
    body.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn is_cjk(ch: char) -> bool {
    ('\u{4e00}'..='\u{9fff}').contains(&ch)
}

fn push_keyword(keywords: &mut Vec<String>, keyword: &str) {
    if !keyword.is_empty() && !STOP_WORDS.contains(&keyword) {
        keywords.push(keyword.to_string());
    }
}

fn first_cjk_chunk(text: &str) -> Option<String> {
    let chunk: String = text.chars().take_while(|ch| is_cjk(*ch)).collect();
    let len = chunk.chars().count();
    if (2..=6).contains(&len) && !STOP_WORDS.contains(&chunk.as_str()) {
        Some(chunk)
    } else {
        None
    }
}

pub fn extract_keywords(text: &str) -> Vec<String> {
    let mut keywords = Vec::new();
    for token in text
        .split(|ch: char| {
            ch.is_whitespace()
                || matches!(
                    ch,
                    ',' | '.' | '。' | '，' | '!' | '?' | '！' | '？' | ':' | '：' | ';' | '；'
                )
        })
        .map(str::trim)
        .filter(|token| !token.is_empty())
    {
        let lowered = token.to_lowercase();
        if lowered.chars().all(|ch| ch.is_ascii_alphanumeric()) {
            if lowered.len() >= 2 {
                push_keyword(&mut keywords, &lowered);
            }
            continue;
        }

        for term in KNOWN_TERMS {
            if lowered.contains(term) {
                push_keyword(&mut keywords, term);
            }
        }

        if STOP_WORDS.contains(&lowered.as_str()) {
            continue;
        }

        for marker in CJK_VALUE_MARKERS {
            if let Some((_, value)) = token.split_once(marker) {
                if let Some(chunk) = first_cjk_chunk(value) {
                    push_keyword(&mut keywords, &chunk);
                }
            }
        }
    }
    keywords.sort();
    keywords.dedup();
    keywords
}

pub fn explicit_memory_from_message(
    body: &str,
    source: &str,
    tags: Vec<String>,
) -> ExplicitMemoryItem {
    let now = Utc::now();
    let normalized_body = normalize_explicit_body(body);
    ExplicitMemoryItem {
        id: format!("explicit_{}", Uuid::new_v4()),
        body: body.trim().to_string(),
        source: source.to_string(),
        tags,
        keywords: extract_keywords(&normalized_body),
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
    fn extracts_keywords_from_common_chinese_profile_memories() {
        let keywords =
            extract_keywords("记住我的生日是5月1日，我住在上海，以后叫我小王，我喜欢咖啡");

        assert!(keywords.contains(&"生日".to_string()));
        assert!(keywords.contains(&"上海".to_string()));
        assert!(keywords.contains(&"小王".to_string()));
        assert!(keywords.contains(&"咖啡".to_string()));
    }

    #[test]
    fn extracts_short_meaningful_english_keywords() {
        let keywords = extract_keywords("remember that I prefer tea SQL AI");

        assert!(keywords.contains(&"tea".to_string()));
        assert!(keywords.contains(&"sql".to_string()));
        assert!(keywords.contains(&"ai".to_string()));
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
