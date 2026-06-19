import type { TranscriptionModelStatus } from "../bindings/TranscriptionModelStatus";
import type { TranscriptionStatusPayload } from "../bindings/TranscriptionStatusPayload";
import { downloadPercent } from "../lib/transcriptionModel";
import { DownloadingLabel } from "./DownloadingLabel";

type TranscriptionSettingsSectionProps = {
  transcriptionStatus: TranscriptionStatusPayload | null;
  onStartModelDownload: () => void;
  onCancelModelDownload: () => void;
};

export function TranscriptionSettingsSection({
  transcriptionStatus,
  onStartModelDownload,
  onCancelModelDownload,
}: TranscriptionSettingsSectionProps) {
  const parakeet = transcriptionStatus?.availableModels[0];
  return (
    <section>
      <h3>Local transcription</h3>
      <p className="settings-hint">{transcriptionStatus?.message ?? "Checking model status..."}</p>
      <ul className="settings-models">
        {transcriptionStatus?.availableModels.map((model) => (
          <li key={model.filename} className={model.selected ? "selected" : ""}>
            <i aria-hidden="true" />
            <span className="model-name">{model.name}</span>
            <code>{model.filename}</code>
            <ModelStatusText model={model} />
            <ModelAction
              model={model}
              onStartModelDownload={onStartModelDownload}
              onCancelModelDownload={onCancelModelDownload}
            />
          </li>
        ))}
      </ul>
      <p className="settings-hint">
        Models live in the local models folder{parakeet ? ` at ${parakeet.path}` : ""}. Audio never
        leaves this Mac: transcription runs locally, there is no account, and no meeting bots join
        your calls.
      </p>
    </section>
  );
}

function ModelStatusText({ model }: { model: TranscriptionModelStatus }) {
  if (model.downloadState === "downloading") {
    return (
      <em>
        <DownloadingLabel percent={downloadPercent(model)} />
      </em>
    );
  }
  if (model.downloadState === "installing") return <em>Installing</em>;
  if (model.downloadState === "failed") return <em>{model.errorMessage ?? "Download failed"}</em>;
  if (model.downloadState === "cancelled") return <em>Cancelled</em>;
  if (model.installed) return <em>{model.selected ? "Installed - selected" : "Installed"}</em>;
  return <em>{model.displaySize ? `Not installed · ${model.displaySize}` : "Not installed"}</em>;
}

function ModelAction({
  model,
  onStartModelDownload,
  onCancelModelDownload,
}: {
  model: TranscriptionModelStatus;
  onStartModelDownload: () => void;
  onCancelModelDownload: () => void;
}) {
  if (!model.downloadable || model.installed) return null;
  if (model.canCancel) {
    return (
      <button type="button" onClick={onCancelModelDownload}>
        Cancel
      </button>
    );
  }
  if (model.canDownload) {
    return (
      <button type="button" onClick={onStartModelDownload}>
        {model.downloadState === "failed" || model.downloadState === "cancelled" ? "Retry" : "Download"}
      </button>
    );
  }
  return null;
}
