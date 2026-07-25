import { Button } from "./Button";
import { SettingsRow } from "./SettingsRow";

type PrivacyLegalSectionProps = {
  onOpenExternalUrl: (url: string) => void;
  onOpenLegalDocument: (document: "privacy" | "notices") => void;
};

const PRIVACY_POLICY_URL = import.meta.env.VITE_PRIVACY_POLICY_URL;
const SUPPORT_URL = import.meta.env.VITE_SUPPORT_URL;
const MODEL_URL = "https://huggingface.co/nvidia/parakeet-tdt-0.6b-v2";

export function PrivacyLegalSection({
  onOpenExternalUrl,
  onOpenLegalDocument,
}: PrivacyLegalSectionProps) {
  return (
    <section id="settings-privacy">
      <h3>What stays on this Mac</h3>
      <SettingsRow
        label="Processed locally"
        description="Microphone audio, system audio, calendar titles and times, and transcripts. No account, no analytics, no tracking, and nothing uploaded."
      />
      <SettingsRow
        label="Kept until you delete it"
        description="Recordings and transcripts. Raw audio is removed after transcription while Save raw audio is off."
      />
      <SettingsRow
        label="Calendar access is read-only"
        description="Revoke it in Permissions or macOS System Settings at any time."
      />
      <SettingsRow
        label="One routine network request"
        description="The transcription model download from GitHub, which you approve. GitHub and your network provider may see ordinary connection data such as an IP address."
      />
      <div className="settings-row-actions settings-legal-actions">
        {PRIVACY_POLICY_URL && (
          <Button size="compact" onClick={() => onOpenExternalUrl(PRIVACY_POLICY_URL)}>
            Read the privacy policy
          </Button>
        )}
        <Button size="compact" variant="quiet" onClick={() => onOpenLegalDocument("privacy")}>
          Bundled offline copy
        </Button>
        {SUPPORT_URL && (
          <Button size="compact" variant="quiet" onClick={() => onOpenExternalUrl(SUPPORT_URL)}>
            Support
          </Button>
        )}
      </div>

      <h3 className="settings-group-heading">Model and open-source software</h3>
      <SettingsRow
        label="NVIDIA Parakeet TDT 0.6B v2"
        description={
          <>
            CC BY 4.0, converted for local use through sherpa-onnx.
            <button
              type="button"
              className="settings-link"
              onClick={() => onOpenExternalUrl(MODEL_URL)}
            >
              huggingface.co/nvidia/parakeet-tdt-0.6b-v2
            </button>
          </>
        }
      >
        <Button size="compact" onClick={() => onOpenLegalDocument("notices")}>
          Third-party notices
        </Button>
      </SettingsRow>
    </section>
  );
}
