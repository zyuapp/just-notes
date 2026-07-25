import type { AppInfo } from "../bindings/AppInfo";
import { Button } from "./Button";
import { SettingsRow } from "./SettingsRow";

type AboutSectionProps = {
  appInfo: AppInfo | null;
  onCopyVersion: () => void;
  onRevealDataFolder: () => void;
};

export function AboutSection({ appInfo, onCopyVersion, onRevealDataFolder }: AboutSectionProps) {
  return (
    <section id="settings-about">
      <SettingsRow
        label="Just Notes"
        description={appInfo ? `Version ${appInfo.version}` : "Reading version…"}
      >
        <Button size="compact" variant="quiet" onClick={onCopyVersion} disabled={!appInfo}>
          Copy version
        </Button>
      </SettingsRow>
      <SettingsRow
        label="Application data"
        description={<span className="settings-path">{appInfo?.dataDir ?? "…"}</span>}
      >
        <Button size="compact" onClick={onRevealDataFolder} disabled={!appInfo}>
          Reveal
        </Button>
      </SettingsRow>
    </section>
  );
}
