type PrivacyLegalSectionProps = {
  onOpenExternalUrl: (url: string) => void;
  onOpenLegalDocument: (document: "privacy" | "notices") => void;
};

const PRIVACY_POLICY_URL = import.meta.env.VITE_PRIVACY_POLICY_URL;
const SUPPORT_URL = import.meta.env.VITE_SUPPORT_URL;

export function PrivacyLegalSection({
  onOpenExternalUrl,
  onOpenLegalDocument,
}: PrivacyLegalSectionProps) {
  return (
    <section id="settings-privacy">
      <h3>Privacy</h3>
      <div className="settings-legal-copy">
        <p>
          Just Notes processes microphone audio, system audio, calendar meeting titles and times,
          and transcripts locally on this Mac. It has no accounts, analytics, advertising, or
          tracking, and it does not upload recordings or transcripts.
        </p>
        <p>
          Recordings and transcripts remain until you delete them. Raw audio is removed after
          transcription when Save raw audio is disabled. Calendar access is read-only and can be
          revoked with the controls in Permissions or macOS System Settings.
        </p>
        <p>
          The only routine network request is the user-approved download of the local transcription
          model from GitHub. GitHub and the network provider may receive ordinary connection data,
          such as an IP address, while serving that file.
        </p>
      </div>
      <div className="settings-row-actions settings-legal-actions">
        {PRIVACY_POLICY_URL && (
          <button type="button" onClick={() => onOpenExternalUrl(PRIVACY_POLICY_URL)}>
            Privacy Policy
          </button>
        )}
        <button type="button" onClick={() => onOpenLegalDocument("privacy")}>
          Offline Policy
        </button>
        {SUPPORT_URL && (
          <button type="button" onClick={() => onOpenExternalUrl(SUPPORT_URL)}>
            Support
          </button>
        )}
      </div>

      <h3>Model and open-source software</h3>
      <div className="settings-legal-copy">
        <p>
          Transcription uses NVIDIA Parakeet TDT 0.6B v2 under CC BY 4.0, converted for local use
          through sherpa-onnx. Full license and third-party notices are included with the app.
        </p>
        <p className="settings-hint">
          Model source: huggingface.co/nvidia/parakeet-tdt-0.6b-v2
        </p>
        <button type="button" onClick={() => onOpenLegalDocument("notices")}>
          Open Third-Party Notices
        </button>
      </div>
    </section>
  );
}
