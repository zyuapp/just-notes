import { useCallback, useEffect, useState } from "react";
import { api, getApiErrorMessage } from "../../api";
import type { ThreadSummary } from "../../bindings/ThreadSummary";

// Loads the archived-thread store while the archive view is open. Kept out of
// the render component so components never reach for the app API directly.
export function useArchivedThreads(active: boolean) {
  const [items, setItems] = useState<ThreadSummary[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(async () => {
    try {
      setItems(await api.threads.listArchived());
    } catch (err) {
      setError(getApiErrorMessage(err));
    }
  }, []);

  useEffect(() => {
    if (!active) {
      setItems(null);
      setError(null);
      return;
    }
    void reload();
  }, [active, reload]);

  return { items, error, reload };
}
