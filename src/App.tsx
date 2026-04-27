import { ThreadSidebar } from "./components/ThreadSidebar";
import { TranscriptPanel } from "./components/TranscriptPanel";
import {
  appReducer,
  getActiveThreadId,
  getStatusLabel,
  initialAppState,
} from "./features/app/state";
import { useJustNotesController } from "./features/app/useJustNotesController";
import { useLiveTranscriptEvents } from "./features/app/useLiveTranscriptEvents";
import { useMemo, useReducer } from "react";

export default function App() {
  const [state, dispatch] = useReducer(appReducer, initialAppState);
  const actions = useJustNotesController(state, dispatch);
  useLiveTranscriptEvents(dispatch);

  const activeThreadId = getActiveThreadId(state);
  const statusLabel = useMemo(() => getStatusLabel(state), [state]);

  return (
    <main className="app-shell">
      <ThreadSidebar
        activeThreadId={activeThreadId}
        appInfo={state.appInfo}
        selectedThreadId={state.selectedThreadId}
        threads={state.threads}
        onCreateThread={actions.createThread}
        onSelectThread={(threadId) => void actions.selectThread(threadId)}
      />
      <TranscriptPanel
        error={state.error}
        liveStatus={state.liveStatus}
        meters={state.meters}
        onCreateThread={actions.createThread}
        onStartFixtureRecording={actions.startFixtureRecording}
        onStartRecording={actions.startRecording}
        onStopRecording={actions.stopRecording}
        recorderState={state.recorderState}
        selectedThread={state.selectedThread}
        statusLabel={statusLabel}
        transcriptionStatus={state.transcriptionStatus}
        fixtureMode={state.appInfo?.fixtureMode ?? false}
      />
    </main>
  );
}
