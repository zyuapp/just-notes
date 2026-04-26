export class ApiError extends Error {
  readonly cause: unknown;

  constructor(message: string, cause?: unknown) {
    super(message);
    this.name = "ApiError";
    this.cause = cause;
  }
}

export function toApiError(error: unknown): ApiError {
  if (error instanceof ApiError) {
    return error;
  }

  if (error instanceof Error) {
    return new ApiError(error.message, error);
  }

  if (typeof error === "string") {
    return new ApiError(error, error);
  }

  return new ApiError("An unexpected app error occurred.", error);
}

export function getApiErrorMessage(error: unknown): string {
  return toApiError(error).message;
}
