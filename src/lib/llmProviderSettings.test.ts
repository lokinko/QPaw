import { describe, expect, it } from "vitest";
import { fallbackSettings } from "./fallback";
import { switchLlmProvider } from "./llmProviderSettings";

describe("switchLlmProvider", () => {
  it("remembers OpenAI-compatible model and API key when switching away and back", () => {
    const openAiSettings = {
      ...fallbackSettings,
      llm: {
        ...fallbackSettings.llm,
        provider: "open_ai_compatible" as const,
        base_url: "https://llm.example.test/v1",
        model: "custom-model",
        api_key: "sk-test",
      },
    };

    const codexSettings = switchLlmProvider(openAiSettings, "codex_cli");
    const restored = switchLlmProvider(codexSettings, "open_ai_compatible");

    expect(restored.llm.base_url).toBe("https://llm.example.test/v1");
    expect(restored.llm.model).toBe("custom-model");
    expect(restored.llm.api_key).toBe("sk-test");
  });

  it("restores the previously configured Codex model", () => {
    const settings = {
      ...fallbackSettings,
      llm: {
        ...fallbackSettings.llm,
        provider: "codex_cli" as const,
        model: "gpt-5-codex",
      },
    };

    const openAiSettings = switchLlmProvider(settings, "open_ai_compatible");
    const restored = switchLlmProvider(openAiSettings, "codex_cli");

    expect(restored.llm.model).toBe("gpt-5-codex");
  });
});
