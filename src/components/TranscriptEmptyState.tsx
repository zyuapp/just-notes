import { Mic } from "lucide-react";
import type { TranscriptionStatusPayload } from "../bindings/TranscriptionStatusPayload";
import type { RecorderState } from "../features/app/state";

type TranscriptEmptyStateProps = {
  hasThread: boolean;
  recorderState: RecorderState;
  transcriptionStatus: TranscriptionStatusPayload | null;
};

export function TranscriptEmptyState({
  hasThread,
  recorderState,
  transcriptionStatus,
}: TranscriptEmptyStateProps) {
  const capturing = recorderState !== "idle";
  const modelMissing = !capturing && Boolean(transcriptionStatus && !transcriptionStatus.ready);

  return (
    <div className="empty-state">
      <div className="empty-mark">
        <Mic size={20} aria-hidden="true" />
      </div>
      <div>
        <h2>{capturing ? "Listening…" : hasThread ? "Ready when you are" : "Ready to capture"}</h2>
        <p>{emptyHint(capturing, modelMissing, hasThread)}</p>
      </div>
    </div>
  );
}

function emptyHint(capturing: boolean, modelMissing: boolean, hasThread: boolean) {
  if (capturing) return "Your transcript will appear here as you speak.";
  if (modelMissing) return "Download the transcription model below to start.";
  return hasThread
    ? "Press Record below to add the first transcript segment to this thread."
    : "Press Record below to capture your first transcript.";
}
