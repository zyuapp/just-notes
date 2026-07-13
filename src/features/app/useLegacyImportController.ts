import { useCallback, useState } from "react";
import { api, getApiErrorMessage } from "../../api";
import { openLegacyImportState, type LegacyImportViewState } from "./legacyImportState";

export function useLegacyImportController(
  onError: (error: unknown) => void,
  onImported: () => Promise<void>,
) {
  const [state, setState] = useState<LegacyImportViewState | null>(null);

  const open = useCallback(() => setState(openLegacyImportState), []);

  const close = useCallback(async () => {
    setState(null);
    try {
      await api.legacyImport.cancel();
    } catch (error) {
      onError(error);
    }
  }, [onError]);

  const locate = useCallback(async () => {
    setState({ stage: "locating" });
    try {
      const preview = await api.legacyImport.prepare();
      setState(preview ? { stage: "preview", preview } : { stage: "intro" });
    } catch (error) {
      setState({ stage: "intro", error: getApiErrorMessage(error) });
    }
  }, []);

  const confirm = useCallback(async () => {
    setState((current) =>
      current?.stage === "preview" ? { stage: "importing", preview: current.preview } : current,
    );
    try {
      const result = await api.legacyImport.confirm();
      setState({ stage: "complete", result });
      try {
        await onImported();
      } catch (error) {
        onError(error);
      }
    } catch (error) {
      setState({ stage: "intro", error: getApiErrorMessage(error) });
    }
  }, [onError, onImported]);

  return { state, open, close, locate, confirm };
}
