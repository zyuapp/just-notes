import { useEffect } from "react";
import { api } from "../../api";
import type { AppAction } from "./state";

type AppDispatch = (action: AppAction) => void;

export function useLiveTranscriptEvents(dispatch: AppDispatch) {
  useEffect(() => {
    const unlistenMeter = api.events.onMeter((payload) => {
      dispatch({ type: "meterReceived", payload });
    });
    const unlistenSegment = api.events.onLiveTranscriptSegment((payload) => {
      dispatch({ type: "liveSegmentReceived", payload, updatedAtMs: Date.now() });
    });
    const unlistenStatus = api.events.onLiveTranscriptStatus((payload) => {
      dispatch({ type: "liveStatusReceived", payload });
    });
    const unlistenError = api.events.onLiveTranscriptError((payload) => {
      dispatch({ type: "liveErrorReceived", payload });
    });

    return () => {
      unlistenMeter.then((dispose) => dispose()).catch(() => undefined);
      unlistenSegment.then((dispose) => dispose()).catch(() => undefined);
      unlistenStatus.then((dispose) => dispose()).catch(() => undefined);
      unlistenError.then((dispose) => dispose()).catch(() => undefined);
    };
  }, [dispatch]);
}
