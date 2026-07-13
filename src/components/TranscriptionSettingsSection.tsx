import type { TranscriptionModelStatus } from "../bindings/TranscriptionModelStatus";
import type { TranscriptionStatusPayload } from "../bindings/TranscriptionStatusPayload";
import { downloadPercent } from "../lib/transcriptionModel";
import { Button } from "./Button";
import { DownloadingLabel } from "./DownloadingLabel";
import { useConfirmAction } from "./useConfirmAction";

type TranscriptionSettingsSectionProps = {
  transcriptionStatus: TranscriptionStatusPayload | null;
  onStartModelDownload: () => void;
  onCancelModelDownload: () => void;
  onDeleteModel: () => void;
};

export function TranscriptionSettingsSection({
  transcriptionStatus,
  onStartModelDownload,
  onCancelModelDownload,
  onDeleteModel,
}: TranscriptionSettingsSectionProps) {
  const model = transcriptionStatus?.availableModels[0];
  return (
    <section>
      <h3>Local transcription</h3>
      <div className="settings-row">
        <div>
          <strong>{model?.name ?? "Local model"}</strong>
          <p className="settings-hint">
            <ModelStatusText model={model} />
          </p>
        </div>
        <div className="settings-row-actions">
          <ModelAction
            model={model}
            onStartModelDownload={onStartModelDownload}
            onCancelModelDownload={onCancelModelDownload}
            onDeleteModel={onDeleteModel}
          />
        </div>
      </div>
      <p className="settings-hint">
        Audio never leaves this Mac: transcription runs locally, there is no account, and no meeting
        bots join your calls.
      </p>
    </section>
  );
}

function ModelStatusText({ model }: { model: TranscriptionModelStatus | undefined }) {
  if (!model) return <>Checking model status…</>;
  if (model.downloadState === "downloading") {
    return <DownloadingLabel percent={downloadPercent(model)} />;
  }
  if (model.downloadState === "installing") return <>Installing…</>;
  if (model.downloadState === "failed") return <>{model.errorMessage ?? "Download failed"}</>;
  if (model.downloadState === "cancelled") return <>Download cancelled</>;
  if (model.installed) return <>Installed</>;
  return <>{model.displaySize ? `Not installed · ${model.displaySize}` : "Not installed"}</>;
}

type ModelActionProps = {
  model: TranscriptionModelStatus | undefined;
  onStartModelDownload: () => void;
  onCancelModelDownload: () => void;
  onDeleteModel: () => void;
};

function ModelAction({
  model,
  onStartModelDownload,
  onCancelModelDownload,
  onDeleteModel,
}: ModelActionProps) {
  const confirmDelete = useConfirmAction(onDeleteModel);
  if (!model) return null;
  if (model.installed) {
    return (
      <Button
        size="compact"
        variant="danger"
        className={confirmDelete.armed ? "armed" : undefined}
        onClick={confirmDelete.trigger}
        onBlur={confirmDelete.reset}
      >
        {confirmDelete.armed ? "Confirm delete" : "Delete"}
      </Button>
    );
  }
  if (model.canCancel) {
    return (
      <Button size="compact" onClick={onCancelModelDownload}>
        Cancel
      </Button>
    );
  }
  if (model.canDownload) {
    return (
      <Button size="compact" onClick={onStartModelDownload}>
        {model.downloadState === "failed" || model.downloadState === "cancelled"
          ? "Retry"
          : "Download"}
      </Button>
    );
  }
  return null;
}
