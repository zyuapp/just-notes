import { appApi } from "./app";
import { eventsApi } from "./events";
import { meetingsApi } from "./meetings";
import { legacyImportApi } from "./legacyImport";
import { recordingApi } from "./recording";
import { settingsApi } from "./settings";
import { systemApi } from "./system";
import { threadsApi } from "./threads";
import { transcriptionApi } from "./transcription";

export { getApiErrorMessage, toApiError, ApiError } from "./errors";

export const api = {
  app: appApi,
  events: eventsApi,
  meetings: meetingsApi,
  legacyImport: legacyImportApi,
  recording: recordingApi,
  settings: settingsApi,
  system: systemApi,
  threads: threadsApi,
  transcription: transcriptionApi,
};
