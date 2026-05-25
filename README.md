# QPaw

QPaw is a Windows-first, low-interruption desktop pet built with Tauri, React,
TypeScript, and a Rust backend. It runs as a transparent always-on-top pet
window, keeps reminders and memory data local, and can connect to an
OpenAI-compatible LLM endpoint for more natural conversation.

The current app is focused on three things:

- Staying present on the desktop without becoming a full-screen app.
- Nudging the user with configurable health and habit reminders.
- Saving local chat, interaction events, working memory, and layered long-term
  memory so QPaw can reuse context later.

## Current Feature Set

### Desktop Pet Window

The primary window is the `pet` Tauri window. It is transparent, decoration-free,
always on top, resizable, skipped from the taskbar, and intended to sit quietly on
the desktop.

Implemented behavior:

- Transparent pet stage with drag support through Tauri window drag regions.
- Resize handles around the pet window.
- Pet window size persistence. The frontend watches resize events and saves the
  logical width and height through the Rust command `save_pet_window_size`.
- Restores saved pet window size on application startup when the stored size is
  within configured bounds.
- Bottom quick-chat dock for sending one short message to QPaw.
- Displays the most recent assistant reply in the pet window.
- Receives reminder events from the backend and renders reminder bubbles in the
  pet window.

The default pet reply is intentionally quiet:

```text
我在旁边，不会打断你。
```

### Settings Window

The secondary `settings` Tauri window opens at `/?view=settings`. Closing it hides
the window instead of destroying it. It can be reopened from the tray menu.

The settings UI currently includes:

- LLM configuration.
- Reminder configuration and live runtime status.
- Avatar import and scale control.
- Local data export and clear controls.
- Conversation history panel.
- Layered memory audit panel.

The app route is selected by `src/App.tsx`:

- `/?view=settings` renders `SettingsWindow`.
- Any other route renders `PetWindow`.

### System Tray

QPaw creates a tray icon with these menu actions:

- Show or hide the pet window.
- Open the settings window.
- Pause or resume all reminders.
- Quit the application.

The tray is configured in the Rust startup path in `src-tauri/src/lib.rs`.

## Chat And LLM

QPaw can talk to an OpenAI-compatible `/chat/completions` endpoint.

The LLM settings are:

- `base_url`, defaulting to `https://api.openai.com/v1`.
- `model`, defaulting to `gpt-4.1-mini`.
- `api_key`, stored locally in app settings.

The Rust LLM client is implemented in `src-tauri/src/llm.rs`. It sends a system
message plus the user's message, with a low temperature for calmer replies.

If the LLM is not configured, QPaw does not block chat. It returns a local
fallback reply:

```text
我会先用本地模式记住这件事。配置 LLM 后，我可以更自然地回应。
```

Chat behavior currently includes:

- User messages are persisted to the local document store.
- Assistant messages are persisted to the local document store.
- Both user and assistant messages are recorded as interaction events.
- Chat history can be loaded from the settings window.
- The backend tries to build local memory context before calling the LLM.
- Messages containing memory-like trigger phrases such as `记住`, `记得`,
  `remember`, or `note that` are also saved to the legacy memory table.

The current system prompt explicitly keeps QPaw in minimal privacy mode:

- QPaw should be calm and low-interruption.
- Replies should be concise.
- QPaw must not claim access to window titles, app names, keystrokes, or unseen
  context.

## Local Memory

QPaw has both short-lived working memory and layered long-term memory.

### Interaction Events

The memory system records interaction events for:

- Chat messages.
- Reminder feedback.
- Task events.
- Assistant reflections.
- App actions.

Each event stores:

- A generated ID.
- Event kind.
- Actor.
- Summary.
- Structured JSON content.
- Tags.
- Creation time.

These events are the raw material for working memory and long-term
consolidation.

### Working Memory

Working memory is short-lived context used during chat. It is stored locally and
expires after a configurable number of hours.

Current settings:

- `working_memory_enabled`: enabled by default.
- `working_memory_retention_hours`: defaults to 36 hours.

Current extraction behavior is intentionally small. It can recognize when the
user tells QPaw what it should be called, using patterns such as:

- `你叫 ...`
- `你名字是 ...`
- `你的名字是 ...`
- `QPaw 叫 ...`
- `它叫 ...`
- `名字是 ...`

That fact is saved as a working memory item with kind `identity`.

### Layered Long-Term Memory

The long-term memory model uses these layers:

- `L0`: high-level user-facing memory items.
- `L1 concept`: concepts such as person, project, preference, habit, topic, or
  task.
- `L1 relation`: relationships between entities.
- `L2`: events.
- `L3`: reflections, observations, lessons, successes, or failures.

The app exposes these categories for L0 memory:

- Preference.
- Person or relationship.
- Task or project.
- Health habit.
- Interaction style.
- Lesson.

The memory panel can:

- Show active working memory.
- Show long-term layered memory.
- Filter by layer.
- Filter by category.
- Search by query text.
- Include or hide archived memory.
- Delete individual long-term memory items.
- Clear current working memory.
- Trigger manual memory consolidation.
- Show memory stats, including raw events, L0, L1, L2, L3, pending jobs, and last
  consolidation time.

### Memory Retrieval

Before a chat request reaches the LLM, QPaw asks the memory retriever for context
related to the current user message.

The retriever currently:

- Loads active working memory.
- Queries each long-term memory layer.
- Scores memory items by query term matches in title, summary, and tags.
- Keeps the newest relevant items.
- Builds a compact text context for the LLM system prompt.

### Memory Consolidation

QPaw has a memory consolidation service that can turn raw events and working
memory into layered long-term memory.

Consolidation behavior:

- Runs a startup backfill when the app starts.
- Runs a periodic loop every 60 seconds.
- At local midnight, consolidates the previous day once.
- Can be run manually from the memory panel.
- Uses the configured LLM to produce strict JSON drafts.
- Saves generated L0, L1, L2, and L3 records.
- Can archive old memory.
- Cleans up raw events according to retention settings.
- Clears working memory for the consolidated date.

Memory consolidation is disabled if memory is disabled in settings. If the LLM is
not configured, manual consolidation reports that the LLM is unavailable.

## Reminders

QPaw includes a local reminder runtime that does not depend on the LLM.

Default reminder items:

- Eye rest every 45 minutes, with a 10 minute idle grace window.
- Hydration every 60 minutes, with a 10 minute idle grace window.

Users can configure reminders from the settings window:

- Pause or resume all reminders.
- Add custom reminder items.
- Change reminder title.
- Change interval in minutes.
- Change idle grace in minutes.
- Pause or resume a single reminder item.
- Delete a reminder item.

Runtime behavior:

- The reminder loop ticks every second.
- Active time is counted only while the user is not naturally idle.
- Natural idle currently means at least 60 seconds idle.
- When a reminder reaches its interval, it becomes pending.
- A pending reminder is emitted when the user is idle or when the item's grace
  time expires.
- After a reminder is emitted, that reminder's active counter resets.
- Reminder events are saved locally.
- Reminder feedback is recorded as both reminder data and memory interaction
  events.

Reminder messages are generated from the reminder title. Hydration and eye-rest
titles get warmer built-in copy; other titles get a generic care-oriented
message.

### Reminder Feedback

Reminder bubbles support three feedback actions:

- Done.
- Snoozed.
- Dismissed.

Feedback is stored locally through the `set_reminder_feedback` command and also
recorded as an interaction event for the memory system.

## Avatar Support

QPaw supports two avatar types:

- Live2D Cubism `.model3.json` packages.
- Static `png`, `jpg`, `jpeg`, and `webp` images.

Avatar import behavior:

- Static images are copied into the app data avatar directory.
- Live2D imports can start from either a `.model3.json` file or a directory.
- Directory imports search up to four levels deep for a `.model3.json` file.
- The entire Live2D model directory is copied into app data.
- The selected avatar is saved into settings.
- The settings window exposes an avatar scale slider from `0.6` to `1.6`.

The app does not bundle the official Live2D Cubism Core runtime. To render
Cubism 4/5 models, place the official runtime file here:

```text
public/vendor/live2dcubismcore.min.js
```

Static image avatars do not require that runtime.

## Data And Privacy Model

QPaw is designed around a minimal local data model.

Stored locally:

- Settings.
- LLM configuration.
- Chat history.
- Legacy memory snippets.
- Interaction events.
- Working memory.
- Layered long-term memory.
- Reminder events and feedback.
- Habit activity events.
- Imported avatar files.

The Rust backend stores data in an embedded SurrealDB database under the app data
directory. Avatar files are also copied into the app data directory.

Important privacy notes:

- Reminder logic works locally and does not require network access.
- Chat only calls the configured LLM endpoint when an API key and model are set.
- The current settings strategy is development-oriented and stores configuration
  in local plaintext.
- Minimal privacy mode is explicit: QPaw should not claim to know window titles,
  app names, keystrokes, or context it was not given.
- Windows idle detection uses `GetLastInputInfo`; it measures idle time, not
  keystroke contents.

## Frontend Structure

The frontend is a Vite React app.

Key files:

- `src/main.tsx`: React entrypoint.
- `src/App.tsx`: chooses between pet and settings views.
- `src/components/PetWindow.tsx`: transparent pet window, avatar, reminder
  bubble, resize handles, and quick chat.
- `src/components/SettingsWindow.tsx`: settings dashboard.
- `src/components/ChatPanel.tsx`: full chat history and chat input in settings.
- `src/components/MemoryPanel.tsx`: working memory and layered memory audit UI.
- `src/components/ReminderBubble.tsx`: reminder popup and feedback buttons.
- `src/components/Live2DAvatar.tsx`: Live2D rendering path.
- `src/components/StaticAvatar.tsx`: static image avatar rendering path.
- `src/components/ResizeHandles.tsx`: pet window resize controls.
- `src/components/ControlButton.tsx`: shared button component.
- `src/lib/tauri.ts`: typed frontend API wrapper for Tauri commands.
- `src/lib/fallback.ts`: browser-only fallback implementation for frontend
  development without Tauri.
- `src/lib/types.ts`: TypeScript models shared by the UI.
- `src/lib/reminderPolicy.ts`: frontend reminder-policy helper used by tests.
- `src/styles.css`: application styling.

When the app is opened in a plain browser instead of Tauri, `src/lib/fallback.ts`
provides in-memory fallback behavior for settings, chat, memory, reminders, and
avatar import. This is useful for UI development, but it is not the real desktop
runtime.

## Backend Structure

The backend is a Tauri 2 Rust application.

Key files:

- `src-tauri/src/lib.rs`: application startup, state creation, tray setup,
  command registration, window behavior, reminder loop, and memory loop.
- `src-tauri/src/commands.rs`: Tauri command handlers exposed to the frontend.
- `src-tauri/src/models.rs`: shared Rust data models and defaults.
- `src-tauri/src/storage.rs`: SurrealDB document store and persistence methods.
- `src-tauri/src/llm.rs`: OpenAI-compatible chat completion client.
- `src-tauri/src/reminders.rs`: reminder runtime and reminder message policy.
- `src-tauri/src/idle.rs`: platform idle-time abstraction.
- `src-tauri/src/notification.rs`: reminder event emission abstraction.
- `src-tauri/src/avatar.rs`: avatar import and copy logic.
- `src-tauri/src/error.rs`: application error type.
- `src-tauri/src/debug.rs`: logging helpers.
- `src-tauri/src/memory/mod.rs`: memory service facade and background loop.
- `src-tauri/src/memory/working.rs`: working memory extraction and upsert logic.
- `src-tauri/src/memory/retriever.rs`: memory search and chat context builder.
- `src-tauri/src/memory/consolidator.rs`: daily long-term memory consolidation.
- `src-tauri/src/memory/prompts.rs`: LLM prompts for consolidation.
- `src-tauri/src/memory/store.rs`: long-term memory storage facade.

## Tauri Windows And Permissions

The app defines two windows in `src-tauri/tauri.conf.json`:

- `pet`: transparent always-on-top desktop pet window.
- `settings`: normal settings window, hidden by default.

Enabled Tauri permissions include:

- Core app, event, menu, resource, tray, and window defaults.
- Window drag and resize dragging.
- Dialog plugin defaults for avatar file selection.

The asset protocol is enabled for app data, local app data, and resources so
imported avatar files can be rendered by the frontend.

## Commands Exposed To The Frontend

The frontend talks to Rust through typed wrappers in `src/lib/tauri.ts`.

Current commands:

- `get_settings`
- `save_settings`
- `save_pet_window_size`
- `import_avatar`
- `send_chat_message`
- `list_chat_history`
- `list_working_memory`
- `clear_working_memory`
- `query_memory`
- `list_memory_items`
- `delete_memory_item`
- `run_memory_consolidation`
- `get_memory_stats`
- `record_task_event`
- `list_memories`
- `clear_memory`
- `trigger_test_reminder`
- `get_reminder_status`
- `set_reminder_feedback`

## Development Requirements

Install these before running the app:

- Node.js 20 or newer.
- Rust 1.89 or newer.
- Windows MSVC Rust toolchain.
- Microsoft C++ Build Tools.
- Microsoft Edge WebView2 Runtime.

The app is Windows-first today. The idle provider has an isolated macOS boundary,
but macOS currently returns `0` idle seconds and is not a complete implementation.

## Install

```powershell
npm install
```

## Run

Run the full Tauri desktop app:

```powershell
npm run tauri:dev
```

Run the frontend only:

```powershell
npm run dev
```

Frontend-only development starts Vite at:

```text
http://127.0.0.1:1420
```

The browser-only mode uses the fallback API in `src/lib/fallback.ts`. It can
preview UI behavior, but it does not start the real Rust reminder loop, tray, idle
detection, embedded database, or native avatar import.

## Build

Build frontend assets:

```powershell
npm run build
```

Build the Tauri app:

```powershell
npm run tauri:build
```

## Test

Run frontend tests:

```powershell
npm test
```

Run Rust tests:

```powershell
cd src-tauri
cargo test
```

Existing test coverage includes:

- Frontend reminder policy behavior.
- Reminder due-kind prioritization and pause handling.
- Avatar import for static images and Live2D `.model3.json`.
- Working memory extraction and repeated identity updates.
- Storage round trips for chat, memory, layered memory, and working memory.
- Memory consolidation helper behavior.

## Debugging Notes

The frontend wrapper logs Tauri command calls, runtime type, argument keys,
elapsed time, and failures through `src/lib/debug.ts`.

The Rust backend uses `debug::log` and `debug::err` across command handlers,
storage, reminders, memory, and LLM calls. Many log messages include a trace ID or
record counts so chat and memory operations can be followed across the backend.

## Known Current Limitations

- The quick chat dock is still a compact single-line input, not a full venting or
  journaling experience.
- Working memory extraction is currently narrow and mostly recognizes QPaw naming
  facts.
- Richer automatic extraction of user preferences, habits, relationships, and
  emotional context is not implemented yet.
- LLM-based memory consolidation requires a configured LLM endpoint.
- LLM settings are stored locally in plaintext.
- The official Live2D Cubism Core runtime is not bundled.
- Browser-only Vite preview is a development fallback, not the native runtime.
- macOS idle detection is stubbed.
- There is no account system or cloud sync.

## Repository Layout

```text
.
├── index.html
├── package.json
├── vite.config.ts
├── tsconfig.json
├── public/
│   └── vendor/
├── src/
│   ├── App.tsx
│   ├── main.tsx
│   ├── styles.css
│   ├── components/
│   └── lib/
└── src-tauri/
    ├── Cargo.toml
    ├── tauri.conf.json
    ├── capabilities/
    ├── icons/
    └── src/
```
