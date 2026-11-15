// Small date and clock helpers. Pure functions, no engine, no browser state
// (except browserTzHours, which only reads the device's clock setting).

import type { LocalDate, Place } from "./types";

/** Local clock hours (0..24) of a Julian Day on a clock `tz` hours from UTC. */
export function localHours(jd: number, tz: number): number {
  const local = jd + tz / 24 + 0.5;
  return (local - Math.floor(local)) * 24;
}

/** "18:05" from 18.083 hours. Rounds to the nearest minute. */
export function hhmm(hours: number): string {
  let m = Math.round(hours * 60);
  m = ((m % 1440) + 1440) % 1440;
  return `${String(Math.floor(m / 60)).padStart(2, "0")}:${String(m % 60).padStart(2, "0")}`;
}

/** "2026-11-15" -> {year, month, day}, or null if it is not a real date. */
export function parseDate(text: string): LocalDate | null {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(text.trim());
  if (!m) return null;
  const [year, month, day] = [Number(m[1]), Number(m[2]), Number(m[3])];
  const check = new Date(Date.UTC(year, month - 1, day));
  if (check.getUTCFullYear() !== year || check.getUTCMonth() !== month - 1 || check.getUTCDate() !== day) {
    return null;
  }
  return { year, month, day };
}

export function formatDate(d: LocalDate): string {
  return `${d.year}-${String(d.month).padStart(2, "0")}-${String(d.day).padStart(2, "0")}`;
}

/** Today's date on a clock `tz` hours from UTC. */
export function todayAt(tz: number, now: Date = new Date()): LocalDate {
  const shifted = new Date(now.getTime() + tz * 3_600_000);
  return { year: shifted.getUTCFullYear(), month: shifted.getUTCMonth() + 1, day: shifted.getUTCDate() };
}

/** This device's UTC offset in hours on a date (includes daylight saving if the device uses it). */
export function browserTzHours(d: LocalDate): number {
  return -new Date(d.year, d.month - 1, d.day, 12).getTimezoneOffset() / 60;
}

/**
 * The clock to show times in for a place: its daylight-saving offset if the
 * user asked for it and the place has one, else its standard offset, else a
 * rough guess from longitude (15 degrees per hour). The guess is labelled so
 * the user knows to set it.
 */
export function clockFor(place: Place, useDst: boolean): { tz: number; label: string; guessed: boolean } {
  if (useDst && place.dst_tz != null) {
    return { tz: place.dst_tz, label: place.dst_tz_name || utcLabel(place.dst_tz), guessed: false };
  }
  if (place.tz != null) {
    return { tz: place.tz, label: place.tz_name || utcLabel(place.tz), guessed: false };
  }
  const tz = Math.round(place.lon / 15);
  return { tz, label: `${utcLabel(tz)} (guessed from longitude)`, guessed: true };
}

export function utcLabel(tz: number): string {
  if (tz === 0) return "UTC";
  const sign = tz > 0 ? "+" : "-";
  const a = Math.abs(tz);
  const h = Math.floor(a);
  const m = Math.round((a - h) * 60);
  return `UTC${sign}${h}${m ? ":" + String(m).padStart(2, "0") : ""}`;
}

// ---- time zones -------------------------------------------------------------

/** The device's own time-zone name, e.g. "Asia/Kolkata" (empty if unknown). */
export function deviceZoneName(): string {
  try {
    return Intl.DateTimeFormat().resolvedOptions().timeZone ?? "";
  } catch {
    return "";
  }
}

/**
 * UTC offset in hours of an IANA time zone (e.g. "America/New_York") at
 * local noon on a date, daylight saving included. `null` if the browser does
 * not know the zone.
 */
export function offsetForZone(zone: string, d: LocalDate): number | null {
  try {
    const noonUtc = new Date(Date.UTC(d.year, d.month - 1, d.day, 12));
    const name = new Intl.DateTimeFormat("en-US", { timeZone: zone, timeZoneName: "longOffset" })
      .formatToParts(noonUtc)
      .find((p) => p.type === "timeZoneName")?.value;
    if (!name) return null;
    if (name === "GMT") return 0;
    const m = /^GMT([+-])(\d{1,2})(?::(\d{2}))?$/.exec(name);
    if (!m) return null;
    const h = Number(m[2]) + Number(m[3] ?? 0) / 60;
    return m[1] === "-" ? -h : h;
  } catch {
    return null;
  }
}

/**
 * Which clock to show times in.
 * - `device`: whatever this phone or computer's clock says (the default).
 * - `place`: the place's own local time (its time zone if known, else its file's offsets).
 * - `custom`: a fixed UTC offset the user typed.
 */
export type ClockChoice = { mode: "device" } | { mode: "place" } | { mode: "custom"; tz: number };

export interface ResolvedClock {
  tz: number;
  label: string;
  /** True when the offset is only a guess from longitude. */
  guessed: boolean;
}

export function resolveClock(
  choice: ClockChoice,
  place: Place,
  useDst: boolean,
  d: LocalDate,
  device: { tz: number; zone: string } = { tz: browserTzHours(d), zone: deviceZoneName() },
): ResolvedClock {
  if (choice.mode === "device") {
    return { tz: device.tz, label: `this device's clock${device.zone ? ` (${device.zone})` : ""}, ${utcLabel(device.tz)}`, guessed: false };
  }
  if (choice.mode === "custom") {
    return { tz: choice.tz, label: utcLabel(choice.tz), guessed: false };
  }
  if (place.zone) {
    const tz = offsetForZone(place.zone, d);
    if (tz != null) return { tz, label: `${place.zone}, ${utcLabel(tz)}`, guessed: false };
  }
  const c = clockFor(place, useDst);
  return { ...c, label: c.guessed ? c.label : `${c.label}, ${utcLabel(c.tz)}` };
}

// ---- comparing time zones ----------------------------------------------------

/**
 * The place's own UTC offset on a date: its time zone if known (daylight
 * saving included), else the offsets from its place file, else a guess from
 * longitude (15 degrees per hour), flagged as approximate.
 */
export function placeOffset(place: Place, useDst: boolean, d: LocalDate): { tz: number; zone: string; approx: boolean } {
  if (place.zone) {
    const tz = offsetForZone(place.zone, d);
    if (tz != null) return { tz, zone: place.zone, approx: false };
  }
  const c = clockFor(place, useDst);
  const name = useDst && place.dst_tz != null ? place.dst_tz_name : place.tz_name;
  return { tz: c.tz, zone: c.guessed ? "" : name || "", approx: c.guessed };
}

export interface ZoneRow {
  who: "place" | "utc" | "you";
  label: string;
  /** "HH:MM" on that clock. */
  time: string;
  /** e.g. "Asia/Tokyo, UTC+9" or "about UTC+5". */
  zone: string;
  /** Days ahead (+1) or behind (-1) of the date shown on screen. */
  dayShift: number;
  /** True for the clock the big time on screen uses. */
  shown: boolean;
}

/** Local calendar day number of a Julian Day on a clock `tz` hours from UTC. */
function dayNumber(jd: number, tz: number): number {
  return Math.floor(jd + tz / 24 + 0.5);
}

/**
 * When the place keeps a different time from this device, list the same
 * instant on three clocks: the place's, UTC, and yours. `null` when the place
 * is on the device's own time (nothing useful to compare).
 */
export function compareZones(
  place: Place,
  useDst: boolean,
  d: LocalDate,
  jd: number,
  shownTz: number,
  device: { tz: number; zone: string } = { tz: browserTzHours(d), zone: deviceZoneName() },
): ZoneRow[] | null {
  const p = placeOffset(place, useDst, d);
  if (Math.abs(p.tz - device.tz) < 1e-9) return null;
  const shownDay = dayNumber(jd, shownTz);
  const row = (who: ZoneRow["who"], label: string, tz: number, zone: string): ZoneRow => ({
    who,
    label,
    time: hhmm(localHours(jd, tz)),
    zone,
    dayShift: dayNumber(jd, tz) - shownDay,
    shown: Math.abs(tz - shownTz) < 1e-9,
  });
  const rows = [
    row("place", place.name || "There", p.tz, p.approx ? `about ${utcLabel(p.tz)}, guessed from longitude` : [p.zone, utcLabel(p.tz)].filter(Boolean).join(", ")),
    row("you", "You", device.tz, [device.zone, utcLabel(device.tz)].filter(Boolean).join(", ")),
  ];
  // UTC only adds something when it is neither the place's clock nor yours.
  if (Math.abs(p.tz) > 1e-9 && Math.abs(device.tz) > 1e-9) rows.splice(1, 0, row("utc", "UTC", 0, "world time"));
  return rows;
}
