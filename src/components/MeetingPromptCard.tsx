import { CalendarClock } from "lucide-react";
import { useEffect, useState } from "react";
import type { MeetingPromptPayload } from "../bindings/MeetingPromptPayload";
import { formatMeetingTiming, formatTimeOfDay } from "../lib/format";

type MeetingPromptCardProps = {
  prompt: MeetingPromptPayload;
  onStart: () => void;
  onDismiss: () => void;
};

export function MeetingPromptCard({ prompt, onStart, onDismiss }: MeetingPromptCardProps) {
  const [now, setNow] = useState(Date.now);

  useEffect(() => {
    setNow(Date.now());
    const timer = window.setInterval(() => setNow(Date.now()), 15_000);
    return () => window.clearInterval(timer);
  }, [prompt.requestId]);

  return (
    <aside className="meeting-prompt-card" aria-label="Upcoming meeting">
      <span className="meeting-prompt-icon" aria-hidden="true">
        <CalendarClock size={18} />
      </span>
      <div className="meeting-prompt-copy">
        <span>
          Meeting {formatMeetingTiming(prompt.startAtMs, now)} ·{" "}
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
