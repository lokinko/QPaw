import type { AppSettings, LlmConfig, LlmProvider, LlmProviderConfig, LlmProviderConfigs } from "./types";

const defaultProviderConfigs: LlmProviderConfigs = {
  codex_cli: {
    base_url: "",
    api_key: "",
    model: "",
  },
  open_ai_compatible: {
    base_url: "https://api.openai.com/v1",
    api_key: "",
    model: "gpt-4.1-mini",
  },
};

function activeProviderConfig(llm: LlmConfig): LlmProviderConfig {
  return {
    base_url: llm.base_url,
    api_key: llm.api_key,
    model: llm.model,
  };
}

function rememberActiveProvider(llm: LlmConfig): LlmProviderConfigs {
  return {
    codex_cli: {
      ...defaultProviderConfigs.codex_cli,
      ...llm.provider_configs?.codex_cli,
    },
    open_ai_compatible: {
      ...defaultProviderConfigs.open_ai_compatible,
      ...llm.provider_configs?.open_ai_compatible,
    },
    [llm.provider]: activeProviderConfig(llm),
  };
}

export function switchLlmProvider(settings: AppSettings, provider: LlmProvider): AppSettings {
  const providerConfigs = rememberActiveProvider(settings.llm);
  const restoredConfig = providerConfigs[provider];

  return {
    ...settings,
    llm: {
      ...settings.llm,
      ...restoredConfig,
      provider,
      provider_configs: providerConfigs,
    },
  };
}

export function updateActiveLlmConfig(settings: AppSettings, patch: Partial<LlmProviderConfig>): AppSettings {
  const providerConfigs = rememberActiveProvider(settings.llm);
  const nextConfig = {
    ...providerConfigs[settings.llm.provider],
    ...patch,
  };

  return {
    ...settings,
    llm: {
      ...settings.llm,
      ...nextConfig,
      provider_configs: {
        ...providerConfigs,
        [settings.llm.provider]: nextConfig,
      },
    },
  };
}
