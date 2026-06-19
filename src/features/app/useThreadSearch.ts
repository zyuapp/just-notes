import { useEffect, useState } from "react";
import { api, getApiErrorMessage } from "../../api";
import type { ThreadSummary } from "../../bindings/ThreadSummary";
import type { AppAction } from "./state";

type AppDispatch = (action: AppAction) => void;

const SEARCH_DEBOUNCE_MS = 200;

export function useThreadSearch(dispatch: AppDispatch, threads: ThreadSummary[]) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<ThreadSummary[] | null>(null);

  useEffect(() => {
    const trimmed = query.trim();
    if (!trimmed) {
      setResults(null);
      return;
    }

    let cancelled = false;
    const handle = setTimeout(() => {
      api.threads
        .search(trimmed)
        .then((matches) => {
          if (!cancelled) setResults(matches);
        })
        .catch((error) => {
          if (!cancelled) dispatch({ type: "failed", message: getApiErrorMessage(error) });
        });
    }, SEARCH_DEBOUNCE_MS);

    return () => {
      cancelled = true;
      clearTimeout(handle);
    };
  }, [dispatch, query]);

  // Drop results for threads that no longer exist (e.g. deleted) so the filtered
  // list can't show stale rows that error when clicked.
  useEffect(() => {
    setResults((current) => {
      if (current === null) return null;
      const live = current.filter((result) => threads.some((thread) => thread.id === result.id));
      return live.length === current.length ? current : live;
    });
  }, [threads]);

  return { query, setQuery, results };
}
