const BYTE_UNITS = ["B", "KB", "MB", "GB", "TB"];

// Binary steps, matching the model catalogue's own display_size so one download
// is not described as two different sizes.
export function formatBytes(bytes: number): string {
  if (bytes <= 0) return "0 MB";
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < BYTE_UNITS.length - 1) {
    value /= 1024;
    unit += 1;
  }
  const precision = value < 10 && unit > 1 ? 1 : 0;
  return `${value.toFixed(precision)} ${BYTE_UNITS[unit]}`;
}

export function formatDuration(ms: number) {
  const totalSeconds = Math.floor(ms / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes.toString().padStart(2, "0")}:${seconds.toString().padStart(2, "0")}`;
}

export function formatCompactDuration(ms: number) {
  const totalSeconds = Math.floor(ms / 1000);
  if (totalSeconds < 60) return `${totalSeconds} sec`;
  const totalMinutes = Math.floor(totalSeconds / 60);
  if (totalMinutes < 60) return `${totalMinutes} min`;
  const hours = Math.floor(totalMinutes / 60);
  const minutes = totalMinutes % 60;
  return minutes === 0 ? `${hours} hr` : `${hours} hr ${minutes} min`;
}

export function formatTimeOfDay(ms: number) {
  return new Intl.DateTimeFormat(undefined, {
    hour: "numeric",
    minute: "2-digit",
  }).format(new Date(ms));
}

export function formatThreadDate(ms: number) {
  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  }).format(new Date(ms));
}

export function formatMeetingTiming(startAtMs: number, nowMs: number) {
  const deltaMinutes = Math.ceil((startAtMs - nowMs) / 60_000);
  if (deltaMinutes > 1) return `starts in ${deltaMinutes} min`;
  if (deltaMinutes >= 0) return "starts now";
  const minutesAgo = Math.abs(deltaMinutes);
  return `started ${minutesAgo} min ago`;
}
