const isDebugEnabled = () =>
  Boolean((import.meta as ImportMeta & { env?: { DEV?: boolean } }).env?.DEV) ||
  localStorage.getItem("qpaw:debug") === "1";

export function debugLog(scope: string, details?: Record<string, unknown>) {
  if (!isDebugEnabled()) return;
  console.debug(`[QPaw debug][${scope}]`, details ?? {});
}

export function debugError(scope: string, error: unknown, details?: Record<string, unknown>) {
  if (!isDebugEnabled()) return;
  console.error(`[QPaw error][${scope}]`, { ...details, error: formatError(error) });
}

export function formatError(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}
