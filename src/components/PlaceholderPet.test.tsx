import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { PlaceholderPet } from "./PlaceholderPet";

describe("PlaceholderPet", () => {
  it("renders the Pixi night cat host as the default avatar", () => {
    const markup = renderToStaticMarkup(<PlaceholderPet />);

    expect(markup).toContain('class="pixi-night-cat"');
    expect(markup).toContain('aria-label="QPaw star lantern cat avatar"');
  });

  it("keeps fallback status details visible", () => {
    const markup = renderToStaticMarkup(<PlaceholderPet detail="缺少官方 Cubism Core" />);

    expect(markup).toContain("缺少官方 Cubism Core");
    expect(markup).toContain('class="avatar-status"');
  });
});
