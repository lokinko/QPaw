import { renderToString } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { SettingsWindow } from "./SettingsWindow";

describe("SettingsWindow", () => {
  it("renders personal memory assistant settings", () => {
    vi.stubGlobal("window", {});
    const markup = renderToString(<SettingsWindow />);

    expect(markup).toContain("个人记忆助理");
    expect(markup).toContain("每日主动上限");
    expect(markup).toContain("空闲阈值");
  });

  it("renders llm provider controls and connectivity test", () => {
    vi.stubGlobal("window", {});
    const markup = renderToString(<SettingsWindow />);

    expect(markup).toContain("Provider");
    expect(markup).toContain("Codex CLI");
    expect(markup).toContain("测试连通性");
  });
});
