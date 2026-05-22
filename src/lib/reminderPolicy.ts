import type { ReminderKind, ReminderSettings } from "./types";

export function nextReminderKind(
  settings: ReminderSettings,
  activeMinutes: Record<ReminderKind, number>,
): ReminderKind | null {
  if (settings.paused) return null;
  return (
    settings.items.find(
      (item) => !item.paused && (activeMinutes[item.id] ?? 0) >= item.interval_minutes,
    )?.id ?? null
  );
}
