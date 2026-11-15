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
