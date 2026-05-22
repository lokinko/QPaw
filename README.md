# QPaw

QPaw is a low-interruption desktop pet for Windows-first workflows, built with
Tauri, React, TypeScript, and a Rust backend. The app keeps reminders local,
supports an OpenAI-compatible LLM endpoint, and stores conversations and habit
signals in an embedded document database.

## Current Scope

- Transparent, draggable desktop pet window.
- Settings window with LLM, reminders, avatar import, memory export, and clear controls.
- System tray for show/hide, settings, pause/resume reminders, and quit.
- Live2D `.model3.json` import flow. The official Cubism Core runtime is not bundled.
- Local SurrealDB embedded document store for settings, conversations, memories, habit events, and reminder events.
- Windows idle detection through `GetLastInputInfo`, with a platform abstraction prepared for macOS.

## Development

Install prerequisites:

- Node.js 20+
- Rust 1.89+ stable with the MSVC toolchain on Windows
- Microsoft C++ Build Tools and WebView2 Runtime for Tauri development

Then run:

```powershell
npm install
npm run tauri:dev
```

Frontend-only development is also available:

```powershell
npm run dev
```

## Live2D Runtime Note

QPaw supports importing a Live2D Cubism `.model3.json` model package. To render
Cubism 4/5 models, place the official `live2dcubismcore.min.js` file from the
Live2D Cubism SDK for Web at:

```text
public/vendor/live2dcubismcore.min.js
```

The app will still run without that file and will show a placeholder avatar until
the runtime is available.
