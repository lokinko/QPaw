import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { fitNightCatCanvasSize, PixiNightCatAvatar } from "./PixiNightCatAvatar";

describe("PixiNightCatAvatar", () => {
  it("renders a draggable static fallback image before Pixi mounts", () => {
    const markup = renderToStaticMarkup(<PixiNightCatAvatar />);

    expect(markup).toContain('class="pixi-night-cat__fallback"');
    expect(markup).toContain('src="/avatars/star-lantern-cat-sprite-trimmed.png"');
    expect(markup).toContain('data-tauri-drag-region="true"');
  });

  it("fits the Pixi canvas to compact host bounds without a large minimum", () => {
    expect(fitNightCatCanvasSize({ width: 96.4, height: 118.6 })).toEqual({
      width: 96,
      height: 119,
    });
  });
});
