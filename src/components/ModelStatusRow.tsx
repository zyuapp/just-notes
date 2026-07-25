import type { TranscriptionModelStatus } from "../bindings/TranscriptionModelStatus";
import { Button } from "./Button";
import { SettingsRow } from "./SettingsRow";
import type { StatusTone } from "../lib/statusTone";
import { StatusPill } from "./StatusPill";

type ModelStatusRowProps = {
  model: TranscriptionModelStatus | undefined;
  onStartModelDownload: () => void;
  onCancelModelDownload: () => void;
};

export function ModelStatusRow({
  model,
  onStartModelDownload,
  onCancelModelDownload,
}: ModelStatusRowProps) {
  if (!model) {
    return <SettingsRow label="Local model" description="Checking model status…" />;
  }
  const status = modelStatus(model);
  return (
    <SettingsRow
      label={model.name}
      description={status.description}
      tone={status.tone === "danger" ? "danger" : "normal"}
    >
      <StatusPill tone={status.tone} label={status.pill} />
      {model.canCancel && (
        <Button size="compact" onClick={onCancelModelDownload}>
          Cancel
        </Button>
      )}
      {model.canDownload && (
        <Button size="compact" variant="primary" onClick={onStartModelDownload}>
          {model.downloadState === "failed" || model.downloadState === "cancelled"
            ? "Retry"
            : "Download"}
        </Button>
      )}
    </SettingsRow>
  );
}

function modelStatus(model: TranscriptionModelStatus): {
  tone: StatusTone;
  pill: string;
  description: string;
} {
  if (model.downloadState === "downloading") {
    return { tone: "idle", pill: "Downloading", description: "English transcription, runs entirely on this Mac." };
  }
  if (model.downloadState === "installing") {
    return { tone: "idle", pill: "Installing", description: "Unpacking the model." };
  }
  if (model.downloadState === "failed") {
    return {
      tone: "danger",
      pill: "Failed",
      description: model.errorMessage ?? "The download did not finish. Retrying resumes it.",
    };
  }
  if (model.downloadState === "cancelled") {
    return { tone: "idle", pill: "Cancelled", description: "The download was stopped." };
  }
  if (model.installed) {
    return {
      tone: "ok",
      pill: "Installed",
      description: "The only model Just Notes uses.",
    };
  }
  return {
    tone: "idle",
    pill: model.displaySize ? `Not installed · ${model.displaySize}` : "Not installed",
    description: "English transcription, runs entirely on this Mac.",
  };
}
