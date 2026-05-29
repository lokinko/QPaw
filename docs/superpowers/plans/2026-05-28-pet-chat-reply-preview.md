# Pet Chat Reply Preview Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the pet-window quick-chat reply readable by adding a scrollable reply region plus hover and click-pinned full-reply preview.

**Architecture:** Keep this as a focused frontend change. `PetWindow.tsx` owns the pinned-preview state and accessible controls; `styles.css` owns the compact scrollable region and preview positioning. No backend or shared type changes are needed.

**Tech Stack:** React, TypeScript, CSS, Vitest static render tests, Vite build.

---

## File Structure

- Create `src/components/PetWindow.test.tsx`: static render coverage for the reply scroll region and preview affordance.
- Modify `src/components/PetWindow.tsx`: add pinned preview state, reply button semantics, preview markup, and close button.
- Modify `src/styles.css`: replace single-line ellipsis with compact multiline scrolling and add preview styles.

### Task 1: Add Pet Reply Preview Test

**Files:**
- Create: `src/components/PetWindow.test.tsx`

- [ ] **Step 1: Write the failing test**

Create `src/components/PetWindow.test.tsx`:

```tsx
import { renderToString } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { PetWindow } from "./PetWindow";

describe("PetWindow", () => {
  it("renders a scrollable reply button and full reply preview region", () => {
    const markup = renderToString(<PetWindow />);

    expect(markup).toContain("pet-chat-dock__reply");
    expect(markup).toContain("pet-reply-preview");
    expect(markup).toContain("aria-label=\"查看完整回复\"");
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```powershell
npm test -- src/components/PetWindow.test.tsx
```

Expected: FAIL because `pet-chat-dock__reply` and `pet-reply-preview` are not rendered.

### Task 2: Add Reply Preview Markup

**Files:**
- Modify: `src/components/PetWindow.tsx`

- [ ] **Step 1: Implement the minimal component change**

In `src/components/PetWindow.tsx`, add pinned state after `reply` state:

```tsx
const [replyPreviewPinned, setReplyPreviewPinned] = useState(false);
```

Replace the current reply paragraph:

```tsx
<p data-tauri-drag-region>{reply}</p>
```

with:

```tsx
<div className={replyPreviewPinned ? "pet-reply-wrap is-pinned" : "pet-reply-wrap"}>
  <button
    className="pet-chat-dock__reply"
    type="button"
    aria-label="查看完整回复"
    aria-expanded={replyPreviewPinned}
    onClick={() => setReplyPreviewPinned((current) => !current)}
  >
    {reply}
  </button>
  <div className="pet-reply-preview" role="dialog" aria-label="完整回复预览">
    <button
      className="pet-reply-preview__close"
      type="button"
      aria-label="关闭完整回复预览"
      onClick={() => setReplyPreviewPinned(false)}
    >
      ×
    </button>
    <p>{reply}</p>
  </div>
</div>
```

In `sendQuickMessage`, after `setChatInput("");`, add:

```tsx
setReplyPreviewPinned(false);
```

- [ ] **Step 2: Run the PetWindow test**

Run:

```powershell
npm test -- src/components/PetWindow.test.tsx
```

Expected: PASS.

### Task 3: Add Scroll And Preview Styles

**Files:**
- Modify: `src/styles.css`

- [ ] **Step 1: Replace single-line reply CSS and add preview styles**

In `src/styles.css`, replace the existing `.pet-chat-dock p` rule with:

```css
.pet-reply-wrap {
  position: relative;
  min-width: 0;
  margin: 0 0 9px;
}

.pet-chat-dock__reply {
  display: block;
  width: 100%;
  max-height: 58px;
  padding: 0;
  color: #31474c;
  font-size: 12px;
  line-height: 1.4;
  text-align: left;
  white-space: pre-wrap;
  overflow: auto;
  background: transparent;
  border: 0;
  border-radius: 6px;
  box-shadow: none;
  cursor: text;
  scrollbar-width: thin;
}

.pet-chat-dock__reply:focus-visible {
  outline: 2px solid rgba(39, 113, 95, 0.32);
  outline-offset: 3px;
}

.pet-reply-preview {
  position: absolute;
  left: 0;
  right: 0;
  bottom: calc(100% + 8px);
  z-index: 35;
  display: none;
  max-height: min(220px, 50vh);
  padding: 12px 34px 12px 12px;
  color: #23363b;
  background: rgba(255, 255, 255, 0.98);
  border: 1px solid rgba(38, 67, 72, 0.16);
  border-radius: 8px;
  box-shadow: 0 18px 52px rgba(16, 33, 37, 0.18);
  overflow: auto;
}

.pet-reply-wrap:hover .pet-reply-preview,
.pet-reply-wrap.is-pinned .pet-reply-preview,
.pet-reply-wrap:focus-within .pet-reply-preview {
  display: block;
}

.pet-reply-preview p {
  margin: 0;
  color: inherit;
  font-size: 13px;
  line-height: 1.5;
  white-space: pre-wrap;
}

.pet-reply-preview__close {
  position: absolute;
  top: 7px;
  right: 7px;
  display: grid;
  place-items: center;
  width: 22px;
  height: 22px;
  padding: 0;
  color: #4a6268;
  background: #edf4f2;
  border: 1px solid #d4e1df;
  border-radius: 999px;
  box-shadow: none;
}
```

- [ ] **Step 2: Run frontend tests**

Run:

```powershell
npm test
```

Expected: PASS.

### Task 4: Verify Build And Diff

**Files:**
- All files from Tasks 1-3.

- [ ] **Step 1: Run production build**

Run:

```powershell
npm run build
```

Expected: PASS.

- [ ] **Step 2: Review diff**

Run:

```powershell
git diff -- src/components/PetWindow.tsx src/components/PetWindow.test.tsx src/styles.css docs/superpowers/plans/2026-05-28-pet-chat-reply-preview.md
```

Expected: Diff contains only the reply preview implementation plan, PetWindow markup/state, PetWindow test, and CSS for reply scrolling/preview.
