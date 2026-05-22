use async_trait::async_trait;

use crate::error::{QPawError, QPawResult};

#[async_trait]
pub trait IdleProvider: Send + Sync {
    async fn idle_seconds(&self) -> QPawResult<u64>;
}

#[derive(Default)]
pub struct SystemIdleProvider;

#[cfg(target_os = "windows")]
#[async_trait]
impl IdleProvider for SystemIdleProvider {
    async fn idle_seconds(&self) -> QPawResult<u64> {
        use windows_sys::Win32::System::SystemInformation::GetTickCount;
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};

        let mut info = LASTINPUTINFO {
            cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
            dwTime: 0,
        };

        let ok = unsafe { GetLastInputInfo(&mut info) };
        if ok == 0 {
            return Err(QPawError::Message(
                "failed to read Windows idle state".to_string(),
            ));
        }

        let tick = unsafe { GetTickCount() };
        Ok(tick.saturating_sub(info.dwTime) as u64 / 1000)
    }
}

#[cfg(target_os = "macos")]
#[async_trait]
impl IdleProvider for SystemIdleProvider {
    async fn idle_seconds(&self) -> QPawResult<u64> {
        // The platform boundary is intentionally isolated for the macOS follow-up.
        Ok(0)
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
#[async_trait]
impl IdleProvider for SystemIdleProvider {
    async fn idle_seconds(&self) -> QPawResult<u64> {
        Ok(0)
    }
}
