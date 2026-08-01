import { agentAccessApi } from "./agentAccess";
import { appApi } from "./app";
import { eventsApi } from "./events";
import { meetingsApi } from "./meetings";
import { recordingApi } from "./recording";
import { settingsApi } from "./settings";
import { systemApi } from "./system";
import { threadsApi } from "./threads";
import { transcriptionApi } from "./transcription";
import { windowApi } from "./window";

export { getApiErrorMessage, toApiError, ApiError } from "./errors";

export const api = {
  agentAccess: agentAccessApi,
  app: appApi,
  events: eventsApi,
  meetings: meetingsApi,
  recording: recordingApi,
  settings: settingsApi,
  system: systemApi,
  threads: threadsApi,
  transcription: transcriptionApi,
  window: windowApi,
};
