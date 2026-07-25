import { useCallback, useState } from "react";
import { api, getApiErrorMessage } from "../../api";
import type { AppAction } from "./state";

type AppDispatch = (action: AppAction) => void;

export function useStorageController(dispatch: AppDispatch) {
  const [clearing, setClearing] = useState(false);

  const refreshUsage = useCallback(() => {
    api.system
      .getStorageUsage()
      .then((storageUsage) => dispatch({ type: "storageUsageLoaded", storageUsage }))
      .catch(() => undefined);
  }, [dispatch]);

  const deleteReclaimableRawAudio = useCallback(async () => {
    setClearing(true);
    try {
      const storageUsage = await api.system.deleteReclaimableRawAudio();
      dispatch({ type: "storageUsageLoaded", storageUsage });
    } catch (error) {
      dispatch({ type: "failed", message: getApiErrorMessage(error) });
    } finally {
      setClearing(false);
    }
  }, [dispatch]);

  return { clearing, deleteReclaimableRawAudio, refreshUsage };
}
