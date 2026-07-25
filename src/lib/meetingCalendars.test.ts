import { describe, expect, test } from "bun:test";
import {
  bulkToggle,
  filterCalendars,
  groupByAccount,
  hasWatchedCalendar,
  toggleCalendarSelection,
} from "./meetingCalendars";

describe("toggleCalendarSelection", () => {
  test("adds and removes a calendar", () => {
    expect(toggleCalendarSelection([], "work")).toEqual(["work"]);
    expect(toggleCalendarSelection(["work"], "work")).toEqual([]);
  });

  test("a burst of toggles accumulates when each sees the previous result", () => {
    const ids = ["work", "personal", "family"].reduce(toggleCalendarSelection, [] as string[]);
    expect(ids).toEqual(["work", "personal", "family"]);
  });

  test("does not duplicate an already-selected calendar", () => {
    expect(toggleCalendarSelection(["work", "work"], "personal")).toEqual(["work", "personal"]);
  });
});

describe("hasWatchedCalendar", () => {
  test("is false with nothing selected", () => {
    expect(hasWatchedCalendar([], [{ id: "work" }])).toBe(false);
  });

  test("is true when a selected calendar still exists", () => {
    expect(hasWatchedCalendar(["work"], [{ id: "work" }, { id: "personal" }])).toBe(true);
  });

  test("ignores a selection whose calendar has disappeared", () => {
    expect(hasWatchedCalendar(["retired"], [{ id: "work" }])).toBe(false);
  });
});

const CALENDARS = [
  { id: "work", title: "Work", account: "Google" },
  { id: "us", title: "US Holidays", account: "Subscribed" },
  { id: "uk", title: "UK Holidays", account: "Subscribed" },
];

describe("filterCalendars", () => {
  test("matches on title or account", () => {
    expect(filterCalendars(CALENDARS, "holiday").map((c) => c.id)).toEqual(["us", "uk"]);
    expect(filterCalendars(CALENDARS, "google").map((c) => c.id)).toEqual(["work"]);
  });

  test("blank filter keeps everything", () => {
    expect(filterCalendars(CALENDARS, "   ")).toHaveLength(3);
  });
});

describe("bulkToggle", () => {
  test("selects all when nothing visible is selected", () => {
    const bulk = bulkToggle(CALENDARS, []);
    expect(bulk.selectingAll).toBe(true);
    expect(bulk.idsToToggle).toEqual(["work", "us", "uk"]);
  });

  test("deselects only what is not yet off", () => {
    const bulk = bulkToggle(CALENDARS, ["work", "us"]);
    expect(bulk.selectingAll).toBe(false);
    expect(bulk.idsToToggle).toEqual(["work", "us"]);
  });

  test("a filter hiding the selected calendars still selects the visible ones", () => {
    const visible = filterCalendars(CALENDARS, "holiday");
    const bulk = bulkToggle(visible, ["work"]);
    expect(bulk.selectingAll).toBe(true);
    expect(bulk.idsToToggle).toEqual(["us", "uk"]);
  });
});

describe("groupByAccount", () => {
  test("keeps accounts in first-seen order with their calendars", () => {
    expect(groupByAccount(CALENDARS).map(([account, group]) => [account, group.length])).toEqual([
      ["Google", 1],
      ["Subscribed", 2],
    ]);
  });
});
