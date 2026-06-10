import { AudioLines, FileText } from "lucide-react";
import type { FinalizationStatusPayload } from "../bindings/FinalizationStatusPayload";
import type { LiveTranscriptStatusPayload } from "../bindings/LiveTranscriptStatusPayload";
import type { ThreadDetail } from "../bindings/ThreadDetail";

type PanelFooterProps = {
  liveStatus: LiveTranscriptStatusPayload | null;
  finalization: FinalizationStatusPayload | null;
  selectedThread: ThreadDetail | null;
  onRevealMarkdown: (path: string) => void;
};

export function PanelFooter({
  liveStatus,
  finalization,
  selectedThread,
  onRevealMarkdown,
}: PanelFooterProps) {
  const finalizationForThread =
    finalization && finalization.threadId === selectedThread?.summary.id ? finalization : null;
  const statusLabel = finalizationForThread?.state === "running"
    ? "Transcribing"
    : liveStatus?.active
      ? "Live"
      : "Idle";
  const message = finalizationForThread?.message ?? liveStatus?.message ?? "Local transcription";
  const markdownPath = selectedThread?.transcriptMarkdownPath;

  return (
    <footer className="panel-foot">
      <span>
        <AudioLines size={22} aria-hidden="true" />
        <strong>{statusLabel}</strong>
        <span>{message}</span>
      </span>
      <span>
        <FileText size={22} aria-hidden="true" />
        {markdownPath ? (
          <button
            type="button"
            className="footer-path"
            onClick={() => onRevealMarkdown(markdownPath)}
            title="Reveal Markdown in Finder"
          >
            {markdownPath}
          </button>
        ) : (
          <span />
        )}
      </span>
    </footer>
  );
}
