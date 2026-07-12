import { getApiErrorMessage } from "../../api";
import type { RecordingPayload } from "../../bindings/RecordingPayload";
import type { AppAction } from "./state";

type AppDispatch = (action: AppAction) => void;

type RecordingStartOptions = {
  dispatch: AppDispatch;
  start: () => Promise<RecordingPayload>;
  refreshThreads: (nextSelectedId?: string) => Promise<void>;
};

export async function runRecordingStart({
  dispatch,
  start,
  refreshThreads,
}: RecordingStartOptions) {
  dispatch({ type: "recordingStarting" });
  let payload: RecordingPayload;
  try {
    payload = await start();
  } catch (error) {
    dispatch({ type: "recordingStartFailed", message: getApiErrorMessage(error) });
    return;
  }
  dispatch({ type: "recordingStarted", payload });
  try {
    await refreshThreads(payload.thread.summary.id);
  } catch (error) {
    dispatch({ type: "failed", message: getApiErrorMessage(error) });
  }
}
