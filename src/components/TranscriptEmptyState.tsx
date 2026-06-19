import { Mic } from "lucide-react";
import type { TranscriptionModelStatus } from "../bindings/TranscriptionModelStatus";
import type { TranscriptionStatusPayload } from "../bindings/TranscriptionStatusPayload";
import { downloadActionLabel, downloadPercent } from "../lib/transcriptionModel";
import { DownloadingLabel } from "./DownloadingLabel";

type TranscriptEmptyStateProps = {
  hasThread: boolean;
  canStart: boolean;
  transcriptionStatus: TranscriptionStatusPayload | null;
  onStartModelDownload: () => void;
  onStartRecording: () => void;
};

export function TranscriptEmptyState({
  hasThread,
  canStart,
  transcriptionStatus,
  onStartModelDownload,
  onStartRecording,
}: TranscriptEmptyStateProps) {
  const selectedModel = transcriptionStatus?.availableModels.find((model) => model.selected);
  const missingSelectedModel = Boolean(transcriptionStatus && !transcriptionStatus.ready);
  const primaryAction =
    missingSelectedModel && selectedModel?.canDownload ? onStartModelDownload : onStartRecording;

  return (
    <div className="empty-state">
      <div className="empty-mark">
        <Mic size={20} aria-hidden="true" />
      </div>
      <div>
        <h2>{hasThread ? "Ready when you are" : "Ready to capture"}</h2>
        <p>
          {hasThread
            ? "Start recording to add the first transcript segment to this thread."
            : "Start recording to capture your first transcript."}
        </p>
      </div>
      <div className="empty-actions">
        <button
          type="button"
          className="empty-primary"
          onClick={primaryAction}
          disabled={!canStart || (missingSelectedModel && !selectedModel?.canDownload)}
        >
          <span className="record-glyph" aria-hidden="true" />
          <span>{missingSelectedModel ? emptyDownloadLabel(selectedModel) : "Start recording"}</span>
        </button>
      </div>
    </div>
  );
}

function emptyDownloadLabel(model: TranscriptionModelStatus | undefined) {
  if (!model) return "Model required";
  if (model.downloadState === "downloading") return <DownloadingLabel percent={downloadPercent(model)} />;
  if (model.downloadState === "installing") return "Installing model";
  return downloadActionLabel();
}
