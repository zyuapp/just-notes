import type { AppSettings } from "../bindings/AppSettings";
import type { TranscriptionProvider } from "../bindings/TranscriptionProvider";
import type { TranscriptionProviderPreference } from "../bindings/TranscriptionProviderPreference";
import type { TranscriptionModelStatus } from "../bindings/TranscriptionModelStatus";
import type { TranscriptionStatusPayload } from "../bindings/TranscriptionStatusPayload";

type TranscriptionSettingsSectionProps = {
  settings: AppSettings;
  transcriptionStatus: TranscriptionStatusPayload | null;
  onTranscriptionProviderChange: (provider: TranscriptionProviderPreference) => void;
  onStartModelDownload: (provider: TranscriptionProvider) => void;
  onCancelModelDownload: (provider: TranscriptionProvider) => void;
};

export function TranscriptionSettingsSection({
  settings,
  transcriptionStatus,
  onTranscriptionProviderChange,
  onStartModelDownload,
  onCancelModelDownload,
}: TranscriptionSettingsSectionProps) {
  const parakeet = transcriptionStatus?.availableModels.find(
    (model) => model.provider === "parakeet",
  );
  return (
    <section>
      <h3>Local transcription</h3>
      <div className="settings-row">
        <div>
          <strong>Transcript model</strong>
          <p className="settings-hint">Parakeet is used for new polished transcripts by default.</p>
        </div>
        <div className="settings-segmented" role="group" aria-label="Transcript model">
          <button
            type="button"
            className={settings.transcriptionProvider === "parakeet" ? "selected" : ""}
            onClick={() => onTranscriptionProviderChange("parakeet")}
          >
            Parakeet
          </button>
          <button
            type="button"
            className={settings.transcriptionProvider === "whisper" ? "selected" : ""}
            onClick={() => onTranscriptionProviderChange("whisper")}
          >
            Whisper
          </button>
        </div>
      </div>
      <p className="settings-hint">{transcriptionStatus?.message ?? "Checking model status..."}</p>
      <ul className="settings-models">
        {transcriptionStatus?.availableModels.map((model) => (
          <li
            key={`${model.provider}:${model.filename}`}
            className={model.selected ? "selected" : ""}
          >
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
    return <em>{`Downloading ${downloadPercent(model)}%`}</em>;
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
  onStartModelDownload: (provider: TranscriptionProvider) => void;
  onCancelModelDownload: (provider: TranscriptionProvider) => void;
}) {
  if (!model.downloadable || model.installed) return null;
  if (model.canCancel) {
    return (
      <button type="button" onClick={() => onCancelModelDownload(model.provider)}>
        Cancel
      </button>
    );
  }
  if (model.canDownload) {
    return (
      <button type="button" onClick={() => onStartModelDownload(model.provider)}>
        {model.downloadState === "failed" || model.downloadState === "cancelled" ? "Retry" : "Download"}
      </button>
    );
  }
  return null;
}

function downloadPercent(model: TranscriptionModelStatus) {
  if (model.totalBytes === 0) return 0;
  return Math.min(100, Math.floor((model.progressBytes * 100) / model.totalBytes));
}
