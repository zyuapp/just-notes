import { CalendarClock } from "lucide-react";
import type { MeetingPromptPayload } from "../bindings/MeetingPromptPayload";
import { formatMeetingTiming, formatTimeOfDay } from "../lib/format";

type MeetingPromptCardProps = {
  prompt: MeetingPromptPayload;
  onStart: () => void;
  onDismiss: () => void;
};

export function MeetingPromptCard({ prompt, onStart, onDismiss }: MeetingPromptCardProps) {
  return (
    <aside className="meeting-prompt-card" aria-label="Upcoming meeting">
      <span className="meeting-prompt-icon" aria-hidden="true">
        <CalendarClock size={18} />
      </span>
      <div className="meeting-prompt-copy">
        <span>
          Meeting {formatMeetingTiming(prompt.startAtMs, Date.now())} ·{" "}
          {formatTimeOfDay(prompt.startAtMs)}
        </span>
        <strong>{prompt.title}</strong>
      </div>
      <div className="meeting-prompt-actions">
        <button type="button" className="meeting-prompt-dismiss" onClick={onDismiss}>
          Dismiss
        </button>
        <button type="button" className="meeting-prompt-start" onClick={onStart}>
          Start recording
        </button>
      </div>
    </aside>
  );
}
