import { useEffect, useState } from "react";
import { api, getApiErrorMessage } from "../../api";
import type { ThreadSummary } from "../../bindings/ThreadSummary";
import type { AppAction } from "./state";

type AppDispatch = (action: AppAction) => void;

const SEARCH_DEBOUNCE_MS = 200;

export function useThreadSearch(dispatch: AppDispatch) {
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
        .then((threads) => {
          if (!cancelled) setResults(threads);
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

  return { query, setQuery, results };
}
