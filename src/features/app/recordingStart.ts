import { getApiErrorMessage } from "../../api";
import type { RecordingPayload } from "../../bindings/RecordingPayload";
import type { AppAction } from "./state";

type AppDispatch = (action: AppAction) => void;

type RecordingStartOptions = {
  dispatch: AppDispatch;
  start: () => Promise<RecordingPayload>;
  refreshThreads: (nextSelectedId?: string) => Promise<void>;
  recoverFailure?: () => Promise<void>;
};

export async function runRecordingStart({
  dispatch,
  start,
  refreshThreads,
  recoverFailure,
}: RecordingStartOptions) {
  dispatch({ type: "recordingStarting" });
  try {
    const payload = await start();
    dispatch({ type: "recordingStarted", payload });
    await refreshThreads(payload.thread.summary.id);
  } catch (error) {
    dispatch({ type: "recordingStartFailed", message: getApiErrorMessage(error) });
    await recoverFailure?.();
  }
}
