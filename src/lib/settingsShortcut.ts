type ShortcutEvent = Pick<KeyboardEvent, "metaKey" | "key" | "altKey" | "ctrlKey">;

export function isSettingsShortcut(event: ShortcutEvent): boolean {
  return event.metaKey && !event.altKey && !event.ctrlKey && event.key === ",";
}
