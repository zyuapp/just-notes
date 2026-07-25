import { useMemo, useState } from "react";
import type { MeetingCalendarPayload } from "../bindings/MeetingCalendarPayload";
import { bulkToggle, filterCalendars, groupByAccount } from "../lib/meetingCalendars";
import { Button } from "./Button";
import { CalendarOption } from "./CalendarOption";
import { SearchField } from "./SearchField";

// Below this a filter and a bulk action are more chrome than help.
const FILTER_THRESHOLD = 6;

type CalendarPickerProps = {
  calendars: MeetingCalendarPayload[];
  selectedIds: string[];
  pendingIds: string[];
  onToggle: (calendarId: string) => void;
};

export function CalendarPicker({
  calendars,
  selectedIds,
  pendingIds,
  onToggle,
}: CalendarPickerProps) {
  const [filter, setFilter] = useState("");
  const selected = new Set(selectedIds);
  const visible = useMemo(() => filterCalendars(calendars, filter), [calendars, filter]);
  const selectedCount = calendars.filter((calendar) => selected.has(calendar.id)).length;
  const bulk = bulkToggle(visible, selectedIds);
  const crowded = calendars.length > FILTER_THRESHOLD;

  if (calendars.length === 0) {
    return <p className="settings-hint">No event calendars are available in macOS Calendar.</p>;
  }

  return (
    <>
      {crowded && (
        <div className="calendar-toolbar">
          <SearchField
            className="calendar-filter"
            value={filter}
            placeholder="Filter calendars"
            onChange={setFilter}
          />
          <Button
            size="compact"
            variant="quiet"
            disabled={bulk.idsToToggle.length === 0}
            // Writes are queued, so flipping many calendars at once is safe.
            onClick={() => bulk.idsToToggle.forEach(onToggle)}
          >
            {bulk.selectingAll ? "Select all" : "Deselect all"}
          </Button>
        </div>
      )}

      <div className={crowded ? "calendar-list scrolls" : "calendar-list"}>
        {groupByAccount(visible).map(([account, group]) => (
          <div key={account}>
            <div className="calendar-account">{account}</div>
            {group.map((calendar) => (
              <CalendarOption
                key={calendar.id}
                calendar={calendar}
                checked={selected.has(calendar.id)}
                pending={pendingIds.includes(calendar.id)}
                onToggle={onToggle}
              />
            ))}
          </div>
        ))}
        {visible.length === 0 && <p className="settings-hint">No calendars match “{filter}”.</p>}
      </div>

      <p className="settings-hint settings-note">
        {selectedCount} of {calendars.length} calendars watched.
      </p>
    </>
  );
}
