mod avatar;
mod commands;
mod debug;
mod error;
mod idle;
mod llm;
mod memory;
mod models;
mod notification;
mod reminders;
mod storage;

use std::sync::Arc;

use avatar::AvatarStore;
use commands::{
    clear_memory, clear_working_memory, delete_memory_item, get_memory_stats, get_reminder_status,
    get_settings, import_avatar, list_chat_history, list_memories, list_memory_items,
    list_working_memory, query_memory, record_task_event, run_memory_consolidation,
    save_pet_window_size, save_settings, send_chat_message, set_reminder_feedback,
    trigger_test_reminder,
};
use error::QPawError;
use idle::SystemIdleProvider;
use llm::LlmClient;
use memory::{start_memory_loop, MemoryService};
use models::{
    AppSettings, PET_WINDOW_MAX_HEIGHT, PET_WINDOW_MAX_WIDTH, PET_WINDOW_MIN_HEIGHT,
    PET_WINDOW_MIN_WIDTH,
};
use reminders::{start_reminder_loop, ReminderRuntime};
use storage::DocumentStore;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{image::Image, LogicalSize, Manager, WebviewWindowBuilder, WindowEvent};

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<DocumentStore>,
    pub llm: Arc<LlmClient>,
    pub memory: Arc<MemoryService>,
    pub avatars: Arc<AvatarStore>,
    pub reminders: Arc<ReminderRuntime>,
}

impl AppState {
    async fn new(app: &tauri::AppHandle) -> Result<Self, QPawError> {
        let app_dir = app.path().app_data_dir()?;
        std::fs::create_dir_all(&app_dir)?;

        let store = Arc::new(DocumentStore::connect(app_dir.join("db")).await?);
        let avatars = Arc::new(AvatarStore::new(app_dir.join("avatars")));
        let llm = Arc::new(LlmClient::default());
        let memory = Arc::new(MemoryService::new(Arc::clone(&store), Arc::clone(&llm)));
        let reminders = Arc::new(ReminderRuntime::new(
            Arc::clone(&store),
            Arc::new(SystemIdleProvider::default()),
        ));

        Ok(Self {
            store,
            llm,
            memory,
            avatars,
            reminders,
        })
    }
}

fn open_settings_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }

    let app = app.clone();
    std::thread::spawn(move || {
        let Some(config) = app
            .config()
            .app
            .windows
            .iter()
            .find(|config| config.label == "settings")
            .cloned()
        else {
            eprintln!("settings window config not found");
            return;
        };

        match WebviewWindowBuilder::from_config(&app, &config).and_then(|builder| builder.build()) {
            Ok(window) => {
                let _ = window.show();
                let _ = window.set_focus();
            }
            Err(error) => eprintln!("failed to open settings window: {error}"),
        }
    });
}

fn saved_pet_window_size(settings: &AppSettings) -> Option<LogicalSize<f64>> {
    let width = settings.window.pet_width?;
    let height = settings.window.pet_height?;
    if !(PET_WINDOW_MIN_WIDTH..=PET_WINDOW_MAX_WIDTH).contains(&width)
        || !(PET_WINDOW_MIN_HEIGHT..=PET_WINDOW_MAX_HEIGHT).contains(&height)
    {
        return None;
    }

    Some(LogicalSize::new(f64::from(width), f64::from(height)))
}

fn restore_pet_window_size(app: &tauri::App, state: &AppState) {
    let Some(window) = app.get_webview_window("pet") else {
        return;
    };

    match tauri::async_runtime::block_on(state.store.get_settings()) {
        Ok(settings) => {
            if let Some(size) = saved_pet_window_size(&settings) {
                if let Err(error) = window.set_size(size) {
                    eprintln!("failed to restore pet window size: {error}");
                }
            }
        }
        Err(error) => eprintln!("failed to load settings for pet window size: {error}"),
    }
}

fn setup_tray(app: &tauri::App, state: AppState) -> tauri::Result<()> {
    let show_hide = MenuItem::with_id(app, "toggle_pet", "显示/隐藏桌宠", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "open_settings", "打开设置", true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "toggle_pause", "暂停/恢复提醒", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_hide, &settings, &pause, &quit])?;

    let icon = app
        .default_window_icon()
        .cloned()
        .or_else(|| Image::from_bytes(include_bytes!("../icons/icon.png")).ok());

    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("QPaw")
        .menu(&menu)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "toggle_pet" => {
                if let Some(window) = app.get_webview_window("pet") {
                    let visible = window.is_visible().unwrap_or(false);
                    let _ = if visible {
                        window.hide()
                    } else {
                        window.show()
                    };
                }
            }
            "open_settings" => {
                open_settings_window(app);
            }
            "toggle_pause" => {
                let state = state.clone();
                tauri::async_runtime::spawn(async move {
                    if let Ok(mut settings) = state.store.get_settings().await {
                        settings.reminders.paused = !settings.reminders.paused;
                        let _ = state.store.save_settings(&settings).await;
                    }
                });
            }
            "quit" => app.exit(0),
            _ => {}
        });

    if let Some(icon) = icon {
        tray = tray.icon(icon);
    }

    tray.build(app)?;
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .on_window_event(|window, event| {
            if window.label() == "settings" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .setup(|app| {
            let handle = app.handle().clone();
            let state = tauri::async_runtime::block_on(AppState::new(&handle))?;
            setup_tray(app, state.clone())?;
            start_reminder_loop(handle, Arc::clone(&state.reminders));
            start_memory_loop(Arc::clone(&state.memory));
            app.manage(state);
            let state = app.state::<AppState>().inner().clone();
            restore_pet_window_size(app, &state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            save_pet_window_size,
            import_avatar,
            send_chat_message,
            list_chat_history,
            list_working_memory,
            clear_working_memory,
            query_memory,
            list_memory_items,
            delete_memory_item,
            run_memory_consolidation,
            get_memory_stats,
            record_task_event,
            list_memories,
            clear_memory,
            trigger_test_reminder,
            get_reminder_status,
            set_reminder_feedback,
        ])
        .run(tauri::generate_context!())
        .expect("error while running QPaw");
}
