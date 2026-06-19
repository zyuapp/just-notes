import { FilePlus2, Mic } from "lucide-react";
import type { TranscriptionModelStatus } from "../bindings/TranscriptionModelStatus";
import type { TranscriptionStatusPayload } from "../bindings/TranscriptionStatusPayload";
import { downloadActionLabel, downloadPercent } from "../lib/transcriptionModel";
import { DownloadingLabel } from "./DownloadingLabel";

type TranscriptEmptyStateProps = {
  hasThread: boolean;
  canStart: boolean;
  transcriptionStatus: TranscriptionStatusPayload | null;
  onCreateThread: () => void;
  onStartModelDownload: () => void;
  onStartRecording: () => void;
};

export function TranscriptEmptyState({
  hasThread,
  canStart,
  transcriptionStatus,
  onCreateThread,
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
            : "Record a conversation now, or create a blank thread for notes you will fill in later."}
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
        {!hasThread && (
          <button type="button" className="empty-secondary" onClick={onCreateThread}>
            <FilePlus2 size={14} aria-hidden="true" />
            <span>New blank thread</span>
          </button>
        )}
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
