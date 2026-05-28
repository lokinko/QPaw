import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { StaticAvatar } from "./StaticAvatar";

describe("StaticAvatar", () => {
  it("does not visually scale the image outside layout bounds", () => {
    vi.stubGlobal("window", {});

    const markup = renderToStaticMarkup(<StaticAvatar imagePath="/avatar.png" scale={0.75} />);

    expect(markup).toContain('class="static-avatar"');
    expect(markup).not.toContain("transform");
  });

  it("passes the configured scale into avatar layout", () => {
    vi.stubGlobal("window", {});

    const markup = renderToStaticMarkup(<StaticAvatar imagePath="/avatar.png" scale={0.35} />);

    expect(markup).toContain("--avatar-scale:0.35");
  });

  it("marks the rendered image as part of the draggable window region", () => {
    vi.stubGlobal("window", {});

    const markup = renderToStaticMarkup(<StaticAvatar imagePath="/avatar.png" scale={1} />);

    expect(markup).toMatch(/<img[^>]*class="static-avatar"[^>]*data-tauri-drag-region="true"/);
  });
});
