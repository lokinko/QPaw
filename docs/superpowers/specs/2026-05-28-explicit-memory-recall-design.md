# Explicit Memory Recall Design

## Goal

Make explicit user memories feel immediately remembered by QPaw. When the user
says "记住", "记一下", "remember", or otherwise triggers a `save` memory
decision, the fact should become available to the next chat turn without waiting
for long-term memory consolidation.

This recall must be provider-independent. Codex CLI and OpenAI-compatible API
mode should both receive the same local memory context because the memory lives
in QPaw's local store, not in either provider.

## Current Gap

The chat path already records user and assistant interaction events. It also
stores explicit "记住" messages in the legacy `memory` table. However, the
runtime prompt context for normal chat is built by `context_for_chat`, which
currently reads active working memory and layered long-term memory only. Legacy
explicit memories are not imported into that retrieval path.

As a result, a user can ask QPaw to remember something and see it saved in local
data, but later chat turns may not include that fact in the LLM prompt. The
product then feels like it did not actually remember.

## Product Behavior

The first iteration should prioritize a simple, observable loop:

- Explicit memories are available on the next chat turn.
- The recall result is shared by Codex CLI and OpenAI-compatible providers.
- Local matching works without calling an LLM.
- Long-term consolidation can still refine memories into layered structures
  later.

Example:

1. User: "记住我喜欢简洁回答。"
2. QPaw saves the memory as an explicit immediate memory.
3. Next user message: "给我讲一下这个功能。"
4. The prompt includes an `Immediate user memories` section containing the
   concise-reply preference.
5. The selected provider responds with that preference available in context.

## Recommended Approach

Use a two-stage memory path:

1. Immediate recall layer for explicit memory.
2. Existing layered memory consolidation for long-term structure.

This gives good short-term experience while preserving the existing layered
memory architecture. It avoids relying on provider-specific memory, and it does
not require LLM embeddings or semantic search for the first pass.

## Data Model

Add a lightweight explicit memory record, either as a new table or as a clearly
typed extension around the existing legacy memory table. The implementation
should prefer the smaller migration surface, but the behavior must expose these
fields internally:

- `id`
- `body`
- `source`, such as `chat`
- `tags`
- `keywords`
- `created_at`
- `last_used_at`
- `status`: `active`, `consolidated`, or `archived`

For the first implementation, `body` should preserve the user's original
wording. `keywords` can be extracted with deterministic local rules from the
message, tags, and simple Chinese/English tokenization. The system should not
rewrite explicit memories through an LLM before they become recallable.

## Import Flow

When chat memory decision returns `save`, or the legacy explicit-memory detector
matches, the chat service should write the memory into the immediate recall
layer in addition to the existing legacy path.

The import function should be idempotent enough to avoid obvious duplicates. A
simple normalized-body key is sufficient for the first iteration:

- Trim whitespace.
- Normalize ASCII case.
- Collapse repeated whitespace.
- Reuse an existing active record when the normalized body matches.

If the user repeats the same memory, update `last_used_at` instead of creating
multiple prompt entries.

## Matching Flow

Before each LLM call, `context_for_chat` should gather:

- Active working memory.
- Matching active explicit memories.
- Matching layered long-term memories.

Explicit memory matching should be local and deterministic:

- If the current message contains terms from a memory's keywords, include it.
- If no keyword match exists, include a small number of recent active explicit
  memories so brand-new preferences still affect the next turn.
- Apply a hard cap to avoid prompt bloat.

Recommended first-pass caps:

- Up to 6 explicit memories.
- Up to 16 layered memories, preserving the current layered cap.
- Working memory remains uncapped unless it already has an existing cap.

The prompt context should separate sections:

```text
Immediate user memories:
- Preference: 用户喜欢简洁回答。

Today's working memory:
- ...

Layered long-term memory:
- ...
```

Section names help the model treat explicit user memories as current user
preferences rather than background chat transcript.

## Consolidation Flow

Memory consolidation should include active explicit memories alongside raw
interaction events and working memory. When consolidation successfully creates
or updates layered memory from an explicit memory, the original explicit record
can be marked `consolidated`.

Consolidated explicit records should not disappear immediately from recall.
They can remain eligible for a short grace period or until the layered memory is
available in retrieval. The first implementation may keep consolidated records
active in recall to avoid losing context, then rely on prompt caps to prevent
overload.

## Provider Behavior

Provider choice must not change recall behavior:

- Codex CLI receives the same prompt context via `codex exec`.
- OpenAI-compatible API receives the same prompt context through chat
  completions.
- No memory is read from Codex CLI's own sessions.
- No memory is assumed to exist inside the remote API provider.

This keeps memory ownership local to QPaw and makes provider switching
predictable.

## Error Handling

If explicit memory import fails, chat should still proceed. The assistant can
reply normally, and the failure should be logged with a trace id.

If matching fails, chat should continue with working and layered memory only.
The user should not see a failure unless the save action itself is being
surfaced in the UI.

If consolidation fails, immediate memories should remain active so the user's
explicitly saved facts are still recallable.

## Testing

Backend tests should cover:

- Explicit "记住" messages are imported into immediate recall.
- A repeated explicit memory updates the existing record instead of duplicating
  prompt context.
- `context_for_chat` includes matching explicit memories.
- `context_for_chat` includes recent explicit memories when keyword matching is
  weak.
- Codex CLI and OpenAI-compatible paths both receive memory context through the
  same `reply_with_context` input.
- Consolidation failure does not remove immediate memories.

Frontend tests are not required for the first implementation unless the UI
shows explicit memory import status.

## Out Of Scope

- Embedding-based semantic retrieval.
- Provider-specific memory synchronization.
- Reading Codex CLI session history.
- User-facing memory editor changes.
- Complex conflict resolution between contradictory memories.
