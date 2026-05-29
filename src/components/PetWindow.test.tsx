import { renderToString } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { PetWindow, replyPreviewClassName } from "./PetWindow";

describe("PetWindow", () => {
  it("renders a scrollable reply button and full reply preview region", () => {
    const markup = renderToString(<PetWindow />);

    expect(markup).toContain("pet-chat-dock__reply");
    expect(markup).toContain("pet-reply-preview");
    expect(markup).toContain('aria-label="查看完整回复"');
  });

  it("marks preview as hover-dismissed after the close button is used", () => {
    expect(replyPreviewClassName(false, true)).toBe("pet-reply-wrap is-hover-dismissed");
    expect(replyPreviewClassName(true, false)).toBe("pet-reply-wrap is-pinned");
  });
});
