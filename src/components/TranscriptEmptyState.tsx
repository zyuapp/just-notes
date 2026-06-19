import { FilePlus2, Mic } from "lucide-react";
import type { TranscriptionProvider } from "../bindings/TranscriptionProvider";
import type { TranscriptionModelStatus } from "../bindings/TranscriptionModelStatus";
import type { TranscriptionStatusPayload } from "../bindings/TranscriptionStatusPayload";

type TranscriptEmptyStateProps = {
  hasThread: boolean;
  canStart: boolean;
  transcriptionStatus: TranscriptionStatusPayload | null;
  onCreateThread: () => void;
  onStartModelDownload: (provider: TranscriptionProvider) => void;
  onStartRecording: () => void;
  onUseWhisper: () => void;
};

export function TranscriptEmptyState({
  hasThread,
  canStart,
  transcriptionStatus,
  onCreateThread,
  onStartModelDownload,
  onStartRecording,
  onUseWhisper,
}: TranscriptEmptyStateProps) {
  const selectedModel = transcriptionStatus?.availableModels.find((model) => model.selected);
  const missingSelectedModel = Boolean(transcriptionStatus && !transcriptionStatus.ready);
  const whisperInstalled = transcriptionStatus?.availableModels.some(
    (model) => model.provider === "whisper" && model.installed,
  );
  const primaryAction =
    missingSelectedModel && selectedModel?.canDownload
      ? () => onStartModelDownload(selectedModel.provider)
      : onStartRecording;

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
        {missingSelectedModel && selectedModel?.provider === "parakeet" && whisperInstalled && (
          <button type="button" className="empty-secondary" onClick={onUseWhisper}>
            <span>Use Whisper for now</span>
          </button>
        )}
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
  if (model.downloadState === "downloading") return `Downloading ${downloadPercent(model)}%`;
  if (model.downloadState === "installing") return "Installing model";
  if (model.provider === "parakeet") return "Download Parakeet";
  return `Download ${model.name}`;
}

function downloadPercent(model: TranscriptionModelStatus) {
  if (model.totalBytes === 0) return 0;
  return Math.min(100, Math.floor((model.progressBytes * 100) / model.totalBytes));
}
