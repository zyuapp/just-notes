import { AudioLines } from "lucide-react";
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
    <div className={capturing ? "empty-state capturing" : "empty-state"}>
      <div className="empty-mark" aria-hidden="true">
        <span className="empty-mark-core"><AudioLines size={32} /></span>
      </div>
      <div>
        <h2>{capturing ? "Recording" : "New recording"}</h2>
        <p>{emptyHint(capturing, modelMissing, hasThread)}</p>
      </div>
    </div>
  );
}

function emptyHint(capturing: boolean, modelMissing: boolean, hasThread: boolean) {
  if (capturing) return "Your transcript will appear here as you speak.";
  if (modelMissing) return "Download the transcription model below to start.";
  return hasThread
    ? "Press Record to add audio to this note."
    : "Record your microphone and system audio.";
}
