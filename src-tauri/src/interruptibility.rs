#![allow(dead_code)]

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
