import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { PixiNightCatAvatar } from "./PixiNightCatAvatar";

describe("PixiNightCatAvatar", () => {
  it("renders a draggable static fallback image before Pixi mounts", () => {
    const markup = renderToStaticMarkup(<PixiNightCatAvatar />);

    expect(markup).toContain('class="pixi-night-cat__fallback"');
    expect(markup).toContain('src="/avatars/star-lantern-cat-sprite-trimmed.png"');
    expect(markup).toContain('data-tauri-drag-region="true"');
  });
});
