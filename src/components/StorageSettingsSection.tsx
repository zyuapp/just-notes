import type { AppInfo } from "../bindings/AppInfo";
import type { AppSettings } from "../bindings/AppSettings";
import { SettingsToggle } from "./SettingsControls";

type StorageSettingsSectionProps = {
  appInfo: AppInfo | null;
  settings: AppSettings;
  busy: boolean;
  onRevealFolder: () => void;
  onChooseFolder: () => void;
  onUseDefaultFolder: () => void;
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
        <button type="button" onClick={props.onRevealFolder}>Reveal</button>
        <button type="button" disabled={props.busy} onClick={props.onChooseFolder}>Change</button>
        {(props.settings.transcriptsDir || props.settings.transcriptsFolderUnavailable) && (
          <button type="button" disabled={props.busy} onClick={props.onUseDefaultFolder}>Use Default</button>
        )}
      </div>
    </div>
    {props.busy && (
      <p className="settings-hint">Storage can be changed after recording and transcription finish.</p>
    )}
    {props.settings.transcriptsFolderUnavailable && (
      <p className="settings-hint">The saved custom folder is unavailable. Reconnect it, choose another folder, or use default storage.</p>
    )}
    <div className="settings-row">
      <div>
        <strong>Data from an earlier version</strong>
        <p className="settings-hint">Import recordings from the old ~/.just-notes folder once.</p>
      </div>
      <button type="button" disabled={props.busy} onClick={props.onImportLegacyData}>Import…</button>
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
