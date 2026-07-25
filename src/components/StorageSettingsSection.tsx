import type { AppInfo } from "../bindings/AppInfo";
import type { AppSettings } from "../bindings/AppSettings";
import type { StorageUsagePayload } from "../bindings/StorageUsagePayload";
import { formatBytes } from "../lib/format";
import { Button } from "./Button";
import { SettingsRow } from "./SettingsRow";
import { SettingsToggle } from "./SettingsControls";
import { useConfirmAction } from "./useConfirmAction";
import type { StorageActions } from "./settingsViewTypes";

type StorageSettingsSectionProps = StorageActions & {
  appInfo: AppInfo | null;
  settings: AppSettings;
  usage: StorageUsagePayload | null;
};

export function StorageSettingsSection(props: StorageSettingsSectionProps) {
  const { usage } = props;
  return (
    <section id="settings-storage">
      <h3>Location</h3>
      <SettingsRow
        label="Transcripts folder"
        description={
          <>
            <span className="settings-path">{props.appInfo?.threadsDir ?? "…"}</span>
            {usage && (
              <span className="settings-subhint">
                {formatBytes(usage.totalBytes)} · {usage.threadCount}{" "}
                {usage.threadCount === 1 ? "recording" : "recordings"} ·{" "}
                {formatBytes(usage.rawAudioBytes)} of that is raw audio
              </span>
            )}
          </>
        }
      >
        <Button size="compact" onClick={props.onRevealFolder}>
          Reveal
        </Button>
        <Button size="compact" variant="quiet" onClick={props.onCopyFolderPath}>
          Copy path
        </Button>
      </SettingsRow>

      <h3 className="settings-group-heading">What gets kept</h3>
      <SettingsToggle
        label="Save raw audio"
        description={
          props.settings.saveRawAudio
            ? "Keep mic.wav and system.wav after the transcript is polished."
            : "Off: raw audio is deleted once each recording finishes transcribing. Transcripts are unaffected."
        }
        checked={props.settings.saveRawAudio}
        onToggle={props.onToggleRawAudio}
      />
      <SettingsToggle
        label="Create Markdown copies"
        description="Write a transcript.md beside every recording when it finishes."
        checked={props.settings.markdownCopy}
        onToggle={props.onToggleMarkdownCopy}
      />
      <ReclaimSpaceRow
        reclaimableBytes={usage?.reclaimableBytes ?? 0}
        clearing={props.clearing}
        onDelete={props.onDeleteReclaimableRawAudio}
      />
    </section>
  );
}

type ReclaimSpaceRowProps = {
  reclaimableBytes: number;
  clearing: boolean;
  onDelete: () => void;
};

function ReclaimSpaceRow({ reclaimableBytes, clearing, onDelete }: ReclaimSpaceRowProps) {
  const confirmDelete = useConfirmAction(onDelete);
  const nothingToReclaim = reclaimableBytes === 0;
  return (
    <SettingsRow
      label="Reclaim space"
      description="Delete raw audio for recordings that already finished. Transcripts are kept."
    >
      <Button
        size="compact"
        variant="danger"
        className={confirmDelete.armed ? "armed" : undefined}
        disabled={nothingToReclaim || clearing}
        onClick={confirmDelete.trigger}
        onBlur={confirmDelete.reset}
      >
        {reclaimLabel(confirmDelete.armed, clearing, reclaimableBytes)}
      </Button>
    </SettingsRow>
  );
}

function reclaimLabel(armed: boolean, clearing: boolean, reclaimableBytes: number): string {
  if (clearing) return "Deleting…";
  if (reclaimableBytes === 0) return "Nothing to delete";
  if (armed) return `Confirm · ${formatBytes(reclaimableBytes)}`;
  return `Delete ${formatBytes(reclaimableBytes)}…`;
}
