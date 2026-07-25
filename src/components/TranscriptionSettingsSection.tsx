import type { TranscriptionModelStatus } from "../bindings/TranscriptionModelStatus";
import type { TranscriptionStatusPayload } from "../bindings/TranscriptionStatusPayload";
import { currentModel } from "../lib/transcriptionModel";
import { Button } from "./Button";
import { ModelDownloadProgress } from "./ModelDownloadProgress";
import { ModelStatusRow } from "./ModelStatusRow";
import { SettingsBanner } from "./SettingsBanner";
import { SettingsRow } from "./SettingsRow";
import { useConfirmAction } from "./useConfirmAction";
import type { ModelActions } from "./settingsViewTypes";

type TranscriptionSettingsSectionProps = ModelActions & {
  transcriptionStatus: TranscriptionStatusPayload | null;
};

export function TranscriptionSettingsSection({
  transcriptionStatus,
  onStartModelDownload,
  onCancelModelDownload,
  onDeleteModel,
}: TranscriptionSettingsSectionProps) {
  const model = currentModel(transcriptionStatus?.availableModels);
  const failed = model?.downloadState === "failed";
  const missing = Boolean(model) && !model?.installed && !model?.canCancel;

  return (
    <section>
      {/* The banner states the consequence; the model row below owns the action. */}
      {missing && (
        <SettingsBanner
          tone={failed ? "danger" : "warning"}
          title="Recordings are not being transcribed"
          description="Audio is still captured and saved. Install the model to turn it into text."
        />
      )}

      <ModelStatusRow
        model={model}
        onStartModelDownload={onStartModelDownload}
        onCancelModelDownload={onCancelModelDownload}
      />

      {model?.downloadState === "downloading" && <ModelDownloadProgress model={model} />}

      {model?.installed && <DeleteModelRow onDeleteModel={onDeleteModel} />}

      <p className="settings-callout">
        Audio never leaves this Mac: transcription runs locally, there is no account, and no meeting
        bots join your calls.
      </p>
    </section>
  );
}

function DeleteModelRow({ onDeleteModel }: { onDeleteModel: () => void }) {
  const confirmDelete = useConfirmAction(onDeleteModel);
  return (
    <SettingsRow
      label="Remove downloaded model"
      description="Recordings keep working but stop being transcribed until you download the model again."
    >
      <Button
        size="compact"
        variant="danger"
        className={confirmDelete.armed ? "armed" : undefined}
        onClick={confirmDelete.trigger}
        onBlur={confirmDelete.reset}
      >
        {confirmDelete.armed ? "Confirm delete" : "Delete"}
      </Button>
    </SettingsRow>
  );
}
