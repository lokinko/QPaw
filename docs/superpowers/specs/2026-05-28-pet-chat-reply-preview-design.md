# Pet Chat Reply Preview Design

## Goal

Improve the pet window quick-chat reply display so longer assistant replies can
be read without turning the compact desktop pet into a full chat window.

## Behavior

The reply text in the pet chat dock should become a compact scrollable region.
Short replies keep the current lightweight feel. Longer replies wrap to multiple
lines and can be scrolled inside the reply area instead of being truncated to a
single ellipsis line.

When the user hovers over the reply region, QPaw should show a larger preview
above the chat dock with the complete current reply. The preview should support
scrolling for very long replies. It should not appear automatically after every
response; the user must intentionally hover.

When the user clicks the reply region, the same preview should stay pinned open.
Clicking the reply region again or pressing a close button should collapse the
pinned preview. Hover can still show the preview when it is not pinned.

## Constraints

- Keep the default pet chat dock compact.
- Do not add a full chat history view to the pet window.
- Do not change backend chat behavior.
- Preserve the existing quick input and send behavior.
- Keep the preview above the dock and inside the pet window bounds as much as
  practical.
- Preserve newline formatting in replies, including memory confirmation prompts.

## Implementation Notes

This should be a focused frontend change in `PetWindow.tsx` and `styles.css`.
The component needs local pinned-preview state and accessible labels on the
reply region and close button. Static render tests should verify the scrollable
reply region and preview affordance are present.

## Verification

Run the frontend tests, production build, and relevant backend tests if shared
types are touched. If only `PetWindow.tsx` and CSS change, frontend tests and
build are sufficient.
