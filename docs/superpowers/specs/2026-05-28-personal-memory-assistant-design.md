# Personal Memory Assistant Design

## Goal

QPaw's next product direction is a personal memory assistant. The focus is not
to collect structured health check-in forms. The focus is to notice which
natural user expressions, preferences, recurring states, and life events are
worth remembering, then reuse those memories in later conversations, gentle
follow-ups, and retrospectives.

The first implementation should make QPaw better at four things:

- Judging whether an expression is worth remembering.
- Asking for confirmation when a memory is plausible but uncertain.
- Following up at low-interruption times.
- Reviewing remembered fragments to find patterns and associations.

## Product Behavior

QPaw should proactively check in only when it is unlikely to interrupt the user.
The default cadence is low frequency: one or two proactive prompts per day. The
preferred windows are configurable, with afternoon and evening as the intended
defaults. If recent conversations mention fatigue, anxiety, pain, poor sleep, or
similar recurring personal-state signals, QPaw may temporarily increase
follow-up attention, but every proactive prompt must still pass the
interruptibility checks.

There are two proactive prompt types:

- Daily light check-in: a broad, natural prompt such as "现在状态还好吗？" or
  "这会儿适合简单复盘一下吗？"
- Context follow-up: a gentle reference to a recent remembered signal, such as
  "你前两天提到肩颈不舒服，今天有好一点吗？"

If the user ignores or closes a proactive prompt, QPaw should not immediately
retry. That interaction can be recorded for throttling, but it should not become
a long-term personal memory by itself.

Retrospectives can be user-triggered or gently proposed during a suitable
evening window. A retrospective summarizes patterns and possible associations
from remembered expressions. It must not make medical claims or present
uncertain correlations as facts.

## Privacy And Interruption Boundaries

QPaw may use local idle duration, configured time windows, and fullscreen state
to decide whether it can appear. Fullscreen has the highest priority. When the
foreground app appears fullscreen, QPaw should default to hiding completely. The
setting should later allow alternatives such as "silent only" or disabling the
fullscreen behavior.

The privacy boundary is explicit:

- Fullscreen detection is used only to hide or silence QPaw.
- Idle detection is used only as elapsed time since user input.
- QPaw must not read window titles, app names, web page contents, document
  contents, keystroke contents, or claim awareness of unseen context.

## Recommended Approach

Build the first iteration as a personal-state memory loop with a small shared
interruptibility core.

This is better than only extending reminders because the product goal is not
just scheduled prompts. The memory assistant needs to decide what is worth
remembering, ask confirmation when uncertain, connect related fragments, and
support later review.

This is smaller than a broad context-aware companion system. The first
implementation should avoid a large cross-product orchestration layer and focus
on personal memory behavior that the user can feel quickly.

## Backend Modules

### interruptibility

`interruptibility` answers one question: can QPaw interrupt the user now?

Inputs:

- Current fullscreen state.
- Current idle seconds.
- Configured allowed check-in windows.
- Configured idle threshold.
- Configured fullscreen behavior.

Outputs:

- `Available`
- `FullscreenHidden`
- `OutsideAllowedWindow`
- `RecentlyActive`
- `Disabled`

This module should not generate prompt copy, write memories, or know about
personal-state logic. It exists so proactive personal prompts, ordinary
reminders, and future work-related prompts can share the same non-interruption
rules.

### memory_decision

`memory_decision` decides whether a user expression should become a memory.

Inputs:

- The current user text.
- The source of the text, such as quick chat, settings chat, proactive check-in
  response, or explicit confirmation.
- Recent chat context when available.
- Relevant existing memories when available.
- Memory sensitivity setting.

Outputs:

- `ignore`: keep only normal chat history.
- `save`: write a memory directly.
- `ask`: ask the user whether this should be remembered.

Each decision should preserve a human-readable reason. Saved memories should
preserve the user's original wording, source, suggested tags or associations,
and enough metadata to explain why QPaw kept the memory. They should not be
forced into a fixed health schema.

The first implementation should work without an LLM:

- Explicit phrases such as "记住", "记得", "记一下", "以后提醒我", "remember", or
  "note that" should directly save.
- Clear preference statements should usually save.
- Repeated personal-state signals such as tiredness, poor sleep, anxiety, pain,
  or low energy should save or ask, depending on confidence and sensitivity.
- Short greetings, generic acknowledgements, and one-off low-context remarks
  should usually ignore.

When an LLM is configured, it can enhance the decision with reasons, tags,
associations, and follow-up suggestions. The LLM must not be the only path for
basic memory capture.

### personal_state_loop

`personal_state_loop` controls proactive personal memory behavior.

Responsibilities:

- Enforce daily proactive prompt limits.
- Ask `interruptibility` whether a prompt can appear.
- Choose between a daily light check-in, a context follow-up, and a
  retrospective proposal.
- Record prompt attempts and user responses for throttling.
- Pass user responses into `memory_decision`.

This module should not replace the existing reminder runtime. Hydration,
eye-rest, and custom reminder behavior stays in the reminders subsystem.

## Existing Module Integration

The existing chat service remains the main path for user conversation. After a
user message is persisted and recorded as an interaction event, the chat service
should call `memory_decision`. The result controls whether a memory is written
immediately, ignored, or represented as a lightweight confirmation prompt in the
assistant reply.

The existing memory service remains responsible for persistence, retrieval, and
long-term consolidation. New memory decisions should feed into the current
interaction event and layered-memory flow rather than creating a separate
fixed-format personal-state database.

The existing reminders runtime continues to handle scheduled health and habit
reminders. It may later use `interruptibility`, but it should not become the
home of personal memory logic.

## Frontend Changes

The pet window should respect fullscreen behavior. In the default mode, QPaw
hides completely while fullscreen is active and restores only after fullscreen
ends. Restoration does not automatically mean a proactive prompt appears; normal
time-window and idle checks still apply.

The chat surfaces should support confirmation-style memory prompts. The first
version may use natural language confirmation, for example: "这件事以后可能有用，
要我记一下吗？" Later versions can add explicit buttons.

Settings should expose:

- Enable or disable personal memory assistant behavior.
- Daily proactive prompt limit, default 2.
- Allowed check-in windows, default afternoon and evening.
- Idle threshold, default 180 seconds.
- Fullscreen behavior, default hide completely.
- Memory sensitivity: conservative, balanced, or active.
- Whether QPaw may ask confirmation before saving uncertain memories.
- Whether retrospectives may cite lower-confidence memories.

The memory panel should evolve from an audit panel into a reviewable memory
list. It should show why an item was saved, its source, suggested tags or
associations, and whether it came from explicit user confirmation.

## Data Shape

The design intentionally avoids a rigid `PersonalStateEntry` with fixed fields
such as mood, energy, sleep, and stress. Personal-state memories should be
natural memory records with flexible metadata.

At minimum, a saved memory should support:

- Stable id.
- Original text.
- Summary.
- Source.
- Created time.
- Decision reason.
- Suggested tags or associations.
- Confidence or sensitivity result.
- Whether the user explicitly confirmed it.
- Optional follow-up suggestion.
- Optional links to related memories or events.

This can be implemented by extending the existing layered memory and interaction
event flow, or by introducing a small memory-candidate table if needed for
pending confirmations. The implementation plan should choose the smallest change
that keeps pending confirmation state clear and testable.

## Error Handling

If LLM-enhanced memory decision fails, QPaw should fall back to local rules and
continue the chat. LLM failure should not prevent storing explicit user memory
requests.

If fullscreen or idle detection fails, QPaw should choose the least disruptive
safe behavior. For proactive prompts, that means delaying the prompt. For normal
manual chat, detection failure should not block the user.

If memory persistence fails, chat should continue with a visible but concise
error path in logs and status text. QPaw should not claim that it remembered
something if persistence failed.

## Testing Strategy

Backend tests should cover:

- `interruptibility` result priority: fullscreen wins, disabled wins, time
  window blocks, idle threshold blocks, available passes.
- `memory_decision` for explicit memory requests, ordinary greetings, preference
  statements, recurring personal-state signals, and uncertain one-off
  statements.
- `personal_state_loop` daily limits, fullscreen blocking, idle gating,
  time-window gating, and elevated attention after recent saved signals.
- Chat integration: save, ask, and ignore decisions do not break existing chat
  persistence.

Frontend tests should cover:

- Settings controls render and map to typed settings.
- Confirmation prompt copy appears when the backend asks for confirmation.
- Pet visibility responds to fullscreen-hidden state.

Existing storage, chat, reminder, avatar, and build tests should continue to
pass.

## First Implementation Slice

The first implementation should be narrow:

1. Add typed settings for personal memory assistant behavior.
2. Add `interruptibility` with testable pure policy logic.
3. Add `memory_decision` with local rule-based decisions.
4. Integrate memory decisions into chat send.
5. Add pending confirmation support in chat copy or lightweight state.
6. Add the first proactive loop only after the decision and interruptibility
   pieces are stable.

Fullscreen detection can be implemented in the first or second slice depending
on Windows API risk. If it is deferred, the settings and policy shape should
still reserve the behavior so the public design does not change.

## Out Of Scope

- Medical diagnosis or treatment advice.
- Reading app names, titles, documents, web pages, or keyboard contents.
- Cloud sync or account systems.
- A fixed health dashboard requiring structured daily form entry.
- Broad work-project integrations beyond personal memory behavior.
