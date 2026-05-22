import { describe, expect, it } from "vitest";
import { nextReminderKind } from "./reminderPolicy";

const settings = {
  paused: false,
  items: [
    {
      id: "eye_rest",
      title: "闭眼休息",
      message: "闭眼休息 20 秒，让眼睛缓一下。",
      action_label: "休息了",
      interval_minutes: 45,
      idle_grace_minutes: 10,
      paused: false,
    },
    {
      id: "hydration",
      title: "喝水时间",
      message: "喝口水，然后继续。",
      action_label: "喝过了",
      interval_minutes: 60,
      idle_grace_minutes: 10,
      paused: false,
    },
  ],
};

describe("nextReminderKind", () => {
  it("does not emit when reminders are paused", () => {
    expect(
      nextReminderKind({ ...settings, paused: true }, { hydration: 80, eye_rest: 80 }),
    ).toBeNull();
  });

  it("prioritizes eye rest when both reminders are due", () => {
    expect(nextReminderKind(settings, { hydration: 70, eye_rest: 50 })).toBe("eye_rest");
  });

  it("emits hydration when only water is due", () => {
    expect(nextReminderKind(settings, { hydration: 61, eye_rest: 10 })).toBe("hydration");
  });

  it("skips a paused custom reminder", () => {
    expect(
      nextReminderKind(
        {
          ...settings,
          items: [{ ...settings.items[0], paused: true }],
        },
        { eye_rest: 50 },
      ),
    ).toBeNull();
  });
});
