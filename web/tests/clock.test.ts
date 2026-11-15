import { describe, expect, it } from "vitest";
import { clockFor, hhmm, localHours, parseDate, todayAt, utcLabel } from "../src/lib/clock";
import { normalisePlace } from "../src/lib/places";

describe("clock helpers", () => {
  it("reads local hours from a Julian Day", () => {
    // JD 2451545.0 is 2000-01-01 12:00 UT (the J2000 epoch, a known fact).
    // Julian Days near 2.45 million carry ~1e-8 h of rounding, so 6 places.
    expect(localHours(2451545.0, 0)).toBeCloseTo(12, 6);
    expect(localHours(2451545.0, 5.5)).toBeCloseTo(17.5, 6);
    expect(localHours(2451545.0, -8)).toBeCloseTo(4, 6);
  });

  it("formats and wraps clock times", () => {
    expect(hhmm(18 + 5 / 60)).toBe("18:05");
    expect(hhmm(23.999)).toBe("00:00");
    expect(hhmm(-0.5)).toBe("23:30");
  });

  it("only accepts real calendar dates", () => {
    expect(parseDate("2026-11-15")).toEqual({ year: 2026, month: 11, day: 15 });
    expect(parseDate("2028-02-29")).not.toBeNull(); // 2028 is a leap year
    expect(parseDate("2026-02-29")).toBeNull();
    expect(parseDate("2026-13-01")).toBeNull();
    expect(parseDate("15/11/2026")).toBeNull();
  });

  it("finds today on another clock", () => {
    // 2026-11-15 20:00 UTC is already 16 November in India (UTC+5.5).
    const t = new Date(Date.UTC(2026, 10, 15, 20, 0));
    expect(todayAt(0, t)).toEqual({ year: 2026, month: 11, day: 15 });
    expect(todayAt(5.5, t)).toEqual({ year: 2026, month: 11, day: 16 });
  });

  it("labels UTC offsets", () => {
    expect(utcLabel(0)).toBe("UTC");
    expect(utcLabel(5.5)).toBe("UTC+5:30");
    expect(utcLabel(-3)).toBe("UTC-3");
    expect(utcLabel(5.75)).toBe("UTC+5:45"); // Nepal
  });

  it("chooses standard, daylight or guessed clocks", () => {
    const ny = normalisePlace({ name: "NY", lat: 40.7, lon: -74, tz: -5, dst_tz: -4, tz_name: "EST", dst_tz_name: "EDT" });
    expect(clockFor(ny, false)).toMatchObject({ tz: -5, label: "EST", guessed: false });
    expect(clockFor(ny, true)).toMatchObject({ tz: -4, label: "EDT", guessed: false });
    // No offset known: 76.27 E / 15 degrees per hour rounds to UTC+5.
    const bare = normalisePlace({ name: "x", lat: 9.93, lon: 76.27 });
    expect(clockFor(bare, true)).toMatchObject({ tz: 5, guessed: true });
  });
});
