type CalendarLike = { id: string };

export function toggleCalendarSelection(selectedIds: string[], calendarId: string): string[] {
  const selected = new Set(selectedIds);
  if (selected.has(calendarId)) selected.delete(calendarId);
  else selected.add(calendarId);
  return [...selected];
}

// A selection can outlive the calendar it names — an account signs out, a shared
// calendar is revoked — so "watching something" means watching something that
// still exists.
export function hasWatchedCalendar(selectedIds: string[], available: CalendarLike[]): boolean {
  const availableIds = new Set(available.map((calendar) => calendar.id));
  return selectedIds.some((id) => availableIds.has(id));
}

type CalendarRow = { id: string; title: string; account: string };

export function filterCalendars<T extends CalendarRow>(calendars: T[], filter: string): T[] {
  const needle = filter.trim().toLowerCase();
  if (!needle) return calendars;
  return calendars.filter(
    (calendar) =>
      calendar.title.toLowerCase().includes(needle)
      || calendar.account.toLowerCase().includes(needle),
  );
}

// Describes and acts on the rows actually on screen, so the bulk action can
// never be labelled for one direction and then do nothing because the selected
// calendars are hidden behind a filter.
export function bulkToggle<T extends CalendarRow>(
  visible: T[],
  selectedIds: string[],
): { selectingAll: boolean; idsToToggle: string[] } {
  const selected = new Set(selectedIds);
  const selectingAll = !visible.some((calendar) => selected.has(calendar.id));
  return {
    selectingAll,
    idsToToggle: visible
      .filter((calendar) => selected.has(calendar.id) !== selectingAll)
      .map((calendar) => calendar.id),
  };
}

export function groupByAccount<T extends CalendarRow>(calendars: T[]): [string, T[]][] {
  const groups = new Map<string, T[]>();
  for (const calendar of calendars) {
    const group = groups.get(calendar.account);
    if (group) group.push(calendar);
    else groups.set(calendar.account, [calendar]);
  }
  return [...groups.entries()];
}
