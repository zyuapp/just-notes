import type { LiveTranscriptStatusPayload } from "../bindings/LiveTranscriptStatusPayload";
import type { ThreadDetail } from "../bindings/ThreadDetail";

type PanelFooterProps = {
  liveStatus: LiveTranscriptStatusPayload | null;
  selectedThread: ThreadDetail | null;
};

export function PanelFooter({ liveStatus, selectedThread }: PanelFooterProps) {
  return (
    <footer className="panel-foot">
      <span>{liveStatus?.message ?? "Idle"}</span>
      <span>{selectedThread?.transcriptMarkdownPath ?? ""}</span>
    </footer>
  );
}
