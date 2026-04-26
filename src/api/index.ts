import { appApi } from "./app";
import { eventsApi } from "./events";
import { recordingApi } from "./recording";
import { threadsApi } from "./threads";
import { transcriptionApi } from "./transcription";

export { getApiErrorMessage, toApiError, ApiError } from "./errors";

export const api = {
  app: appApi,
  events: eventsApi,
  recording: recordingApi,
  threads: threadsApi,
  transcription: transcriptionApi,
};
