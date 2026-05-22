import { Check, Clock, X } from "lucide-react";
import { api } from "../lib/tauri";
import type { ReminderFeedback, ReminderPayload } from "../lib/types";

interface ReminderBubbleProps {
  reminder: ReminderPayload;
  onClose: () => void;
}

export function ReminderBubble({ reminder, onClose }: ReminderBubbleProps) {
  const submit = async (feedback: ReminderFeedback) => {
    await api.setReminderFeedback({
      reminder_id: reminder.id,
      kind: reminder.kind,
      feedback,
    });
    onClose();
  };

  return (
    <section className="reminder-bubble" aria-live="polite">
      <div>
        <h2>{reminder.title}</h2>
        <p>{reminder.message}</p>
      </div>
      <div className="reminder-bubble__actions">
        <button title={reminder.action_label} onClick={() => void submit("done")}>
          <Check size={17} />
        </button>
        <button title="稍后提醒" onClick={() => void submit("snoozed")}>
          <Clock size={17} />
        </button>
        <button title="关闭" onClick={() => void submit("dismissed")}>
          <X size={17} />
        </button>
      </div>
    </section>
  );
}
