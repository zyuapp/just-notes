import type { AppInfo } from "../bindings/AppInfo";
import type { AppSettings } from "../bindings/AppSettings";
import { FolderInput } from "lucide-react";
import { Button } from "./Button";
import { SettingsToggle } from "./SettingsControls";

type StorageSettingsSectionProps = {
  appInfo: AppInfo | null;
  settings: AppSettings;
  busy: boolean;
  onRevealFolder: () => void;
  onImportLegacyData: () => void;
  onToggleRawAudio: () => void;
  onToggleMarkdownCopy: () => void;
};

export function StorageSettingsSection(props: StorageSettingsSectionProps) {
  return <section id="settings-storage">
    <div className="settings-row">
      <div>
        <strong>Transcripts folder</strong>
        <p className="settings-path">{props.appInfo?.threadsDir ?? "…"}</p>
      </div>
      <div className="settings-row-actions">
        <Button size="compact" onClick={props.onRevealFolder}>Reveal</Button>
      </div>
    </div>
    <div className="legacy-import-card">
      <div className="legacy-import-card-copy">
        <div className="legacy-import-card-icon" aria-hidden="true"><FolderInput size={16} /></div>
        <div>
          <strong>Recordings from an earlier version</strong>
          <p>Merge recordings from the previous version. Current recordings and the downloaded transcription model will be preserved.</p>
        </div>
      </div>
      <Button disabled={props.busy} onClick={props.onImportLegacyData}>
        Import previous recordings…
      </Button>
    </div>
    <SettingsToggle
      label="Save raw audio"
      description="Keep mic.wav and system.wav after the transcript is polished."
      checked={props.settings.saveRawAudio}
      onToggle={props.onToggleRawAudio}
    />
    <SettingsToggle
      label="Create Markdown copies"
      description="Write a transcript.md beside every recording when it finishes."
      checked={props.settings.markdownCopy}
      onToggle={props.onToggleMarkdownCopy}
    />
  </section>;
}
