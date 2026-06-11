import { X } from "lucide-react";
import { useEffect } from "react";
import type { AppInfo } from "../bindings/AppInfo";
import type { AppSettings } from "../bindings/AppSettings";
import type { PermissionsPayload } from "../bindings/PermissionsPayload";
import type { TranscriptionStatusPayload } from "../bindings/TranscriptionStatusPayload";
import { permissionLabel, SettingsToggle } from "./SettingsControls";

type SettingsViewProps = {
  settings: AppSettings;
  appInfo: AppInfo | null;
  transcriptionStatus: TranscriptionStatusPayload | null;
  permissions: PermissionsPayload | null;
  onClose: () => void;
  onChooseFolder: () => void;
  onUseDefaultFolder: () => void;
  onRevealFolder: () => void;
  onToggleRawAudio: () => void;
  onToggleMarkdownCopy: () => void;
  onOpenPrivacy: (pane: "microphone" | "system-audio") => void;
};

export function SettingsView({
  settings,
  appInfo,
  transcriptionStatus,
  permissions,
  onClose,
  onChooseFolder,
  onUseDefaultFolder,
  onRevealFolder,
  onToggleRawAudio,
  onToggleMarkdownCopy,
  onOpenPrivacy,
}: SettingsViewProps) {
  // macOS webviews never deliver keydown for Escape (tauri#5790); keyup does.
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    document.addEventListener("keydown", onKey);
    document.addEventListener("keyup", onKey);
    return () => {
      document.removeEventListener("keydown", onKey);
      document.removeEventListener("keyup", onKey);
    };
  }, [onClose]);

  return (
    <div className="settings-overlay" role="dialog" aria-label="Settings">
      <div className="settings-panel">
        <header>
          <h2>Settings</h2>
          <button type="button" className="icon-button" onClick={onClose} aria-label="Close settings">
            <X size={15} aria-hidden="true" />
          </button>
        </header>

        <section>
          <h3>Storage</h3>
          <div className="settings-row">
            <div>
              <strong>Transcripts folder</strong>
              <p className="settings-path">{appInfo?.threadsDir ?? "…"}</p>
            </div>
            <div className="settings-row-actions">
              <button type="button" onClick={onChooseFolder}>Change…</button>
              <button type="button" onClick={onRevealFolder}>Reveal</button>
              {settings.transcriptsDir !== null && (
                <button type="button" onClick={onUseDefaultFolder}>Use default</button>
              )}
            </div>
          </div>
          <SettingsToggle
            label="Save raw audio"
            description="Keep mic.wav and system.wav with each recording, and use them to polish the transcript after you stop."
            checked={settings.saveRawAudio}
            onToggle={onToggleRawAudio}
          />
          <SettingsToggle
            label="Create Markdown copies"
            description="Write a transcript.md beside every recording when it finishes."
            checked={settings.markdownCopy}
            onToggle={onToggleMarkdownCopy}
          />
        </section>

        <section>
          <h3>Local transcription</h3>
          <p className="settings-hint">{transcriptionStatus?.message ?? "Checking model status…"}</p>
          <ul className="settings-models">
            {transcriptionStatus?.availableModels.map((model) => (
              <li key={model.filename} className={model.selected ? "selected" : ""}>
                <i aria-hidden="true" />
                <span className="model-name">{model.name}</span>
                <code>{model.filename}</code>
                <em>
                  {model.installed
                    ? model.selected
                      ? "Installed · selected"
                      : "Installed"
                    : "Not installed"}
                </em>
              </li>
            ))}
          </ul>
          <p className="settings-hint">
            Models live in the local models folder. Audio never leaves this Mac: transcription runs
            locally, there is no account, and no meeting bots join your calls.
          </p>
        </section>

        <section>
          <h3>Permissions</h3>
          <div className="settings-row">
            <div>
              <strong>Microphone</strong>
              <p className="settings-hint">{permissionLabel(permissions?.microphone)}</p>
            </div>
            <div className="settings-row-actions">
              <button type="button" onClick={() => onOpenPrivacy("microphone")}>
                Open System Settings
              </button>
            </div>
          </div>
          <div className="settings-row">
            <div>
              <strong>System audio</strong>
              <p className="settings-hint">
                macOS asks on first recording. Manage it under Screen &amp; System Audio Recording.
              </p>
            </div>
            <div className="settings-row-actions">
              <button type="button" onClick={() => onOpenPrivacy("system-audio")}>
                Open System Settings
              </button>
            </div>
          </div>
        </section>
      </div>
    </div>
  );
}

