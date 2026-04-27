import { FileText, AudioLines } from "lucide-react";
import type { LiveTranscriptStatusPayload } from "../bindings/LiveTranscriptStatusPayload";
import type { ThreadDetail } from "../bindings/ThreadDetail";

type PanelFooterProps = {
  liveStatus: LiveTranscriptStatusPayload | null;
  selectedThread: ThreadDetail | null;
};

export function PanelFooter({ liveStatus, selectedThread }: PanelFooterProps) {
  return (
    <footer className="panel-foot">
      <span>
        <AudioLines size={22} aria-hidden="true" />
        <strong>Idle</strong>
        <span>{liveStatus?.message ?? "Live transcription is listening with small.en"}</span>
      </span>
      <span>
        <FileText size={22} aria-hidden="true" />
        <span>{selectedThread?.transcriptMarkdownPath ? `Markdown: ${selectedThread.transcriptMarkdownPath}` : ""}</span>
      </span>
    </footer>
  );
}
