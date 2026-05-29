import { renderToString } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { ChatPanel } from "./ChatPanel";

describe("ChatPanel", () => {
  it("has a region for memory confirmation status", () => {
    const markup = renderToString(<ChatPanel />);

    expect(markup).toContain("chat-memory-decision");
  });
});
