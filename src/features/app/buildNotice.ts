import type { Notice } from "../../components/NoticeBar";
import { downloadActionLabel } from "../../lib/transcriptionModel";
import type { AppState } from "./state";

export function buildNotice(
  state: AppState,
  openSettings: () => void,
  openPrivacy: (pane: "microphone" | "system-audio") => Promise<void>,
  startModelDownload: () => Promise<void>,
): Notice | null {
  if (state.permissions && ["denied", "restricted"].includes(state.permissions.microphone)) {
    return {
      message: "Microphone access is blocked, so recordings will miss your voice.",
      actionLabel: "Open System Settings",
      onAction: () => void openPrivacy("microphone"),
    };
  }
  if (state.transcriptionStatus && !state.transcriptionStatus.ready) {
    const selectedModel = state.transcriptionStatus.availableModels.find((model) => model.selected);
    return {
      message: "Install the selected local transcription model before recording.",
      actionLabel: selectedModel?.canDownload ? downloadActionLabel(selectedModel) : "Model status",
      onAction: selectedModel?.canDownload ? () => void startModelDownload() : openSettings,
    };
  }
  return null;
}
