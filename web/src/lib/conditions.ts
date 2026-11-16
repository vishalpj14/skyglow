// Live conditions: cloud cover, moisture and haze for a place and date, from
// the free Open-Meteo services (https://open-meteo.com, CC BY 4.0):
//
//   forecast service      cloud cover (low, middle, high), humidity, water vapour
//   air-quality service   aerosol optical depth (the European CAMS forecast)
//
// This file does the going-online part: build the two addresses, fetch them,
// and keep the answer in this browser so it is still there offline. Reading
// the answers is the Rust core's job (core/src/openmeteo.rs, through
// engine.ts), so the web app and the command line translate identically.
//
// What is sent: the place's coordinates rounded to two decimals (about 1 km;
// the forecast grid is coarser than that anyway) and three dates. Nothing else.

import * as engine from "./engine";
import { round } from "./geo";
import type { ConditionsSeries, LocalDate } from "./types";

/** Shown next to the numbers. */
export const SOURCE = "Open-Meteo forecast";

/** A saved answer younger than this is used without asking again. Forecasts update about hourly. */
export const FRESH_MS = 60 * 60 * 1000;

/** How many place-and-date answers to keep in this browser. */
const KEEP = 12;
const CACHE = "skyglow.conditions.v1";
/** Give up on a service after this long, so a stalled connection never hangs the app. */
const TIMEOUT_MS = 8000;

export interface StoredConditions {
  series: ConditionsSeries;
  /** When it was fetched, milliseconds since 1970 on this device's clock. */
  fetchedAt: number;
  /** Which of the two services answered usefully. */
  parts: { weather: boolean; aerosols: boolean };
}

export interface Loaded {
  stored: StoredConditions | null;
  /** Where `stored` came from this time. */
  from: "network" | "saved" | "none";
  /** Why a fresh answer could not be had, in words, if one was tried. */
  problem?: string;
}

/** The slice of browser storage this file needs, so tests can pass a plain object. */
export interface Store {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

/** The calendar date `days` after `d`, as YYYY-MM-DD. */
export function shiftDate(d: LocalDate, days: number): string {
  return new Date(Date.UTC(d.year, d.month - 1, d.day + days)).toISOString().slice(0, 10);
}

/**
 * The two addresses to fetch. Times come back in UTC. The day before and the
 * day after are included, because the local day of a place far from Greenwich
 * spills over both ends of the UTC day.
 */
export function conditionsUrls(lat: number, lon: number, date: LocalDate): { weather: URL; aerosols: URL } {
  const hourly = engine.openMeteoHourly();
  const common = {
    latitude: String(round(lat, 2)),
    longitude: String(round(lon, 2)),
    start_date: shiftDate(date, -1),
    end_date: shiftDate(date, 1),
    timezone: "GMT",
    timeformat: "unixtime",
  };
  const weather = new URL("https://api.open-meteo.com/v1/forecast");
  weather.search = new URLSearchParams({ ...common, hourly: hourly.forecast }).toString();
  const aerosols = new URL("https://air-quality-api.open-meteo.com/v1/air-quality");
  aerosols.search = new URLSearchParams({ ...common, hourly: hourly.airQuality }).toString();
  return { weather, aerosols };
}

/** One saved answer per rounded spot and date. */
export function cacheKey(lat: number, lon: number, date: LocalDate): string {
  return `${round(lat, 2)},${round(lon, 2)},${shiftDate(date, 0)}`;
}

function browserStore(): Store | null {
  try {
    return typeof localStorage === "undefined" ? null : localStorage;
  } catch {
    return null;
  }
}

function readAll(store: Store | null): Record<string, StoredConditions> {
  try {
    return JSON.parse(store?.getItem(CACHE) ?? "{}") ?? {};
  } catch {
    return {};
  }
}

export function readSaved(key: string, store: Store | null = browserStore()): StoredConditions | null {
  const hit = readAll(store)[key];
  return hit && Array.isArray(hit.series?.samples) && typeof hit.fetchedAt === "number" ? hit : null;
}

function writeSaved(key: string, value: StoredConditions, store: Store | null) {
  try {
    const all = readAll(store);
    all[key] = value;
    // Keep only the most recently fetched answers.
    const newestFirst = Object.keys(all).sort((a, b) => all[b].fetchedAt - all[a].fetchedAt);
    for (const old of newestFirst.slice(KEEP)) delete all[old];
    store?.setItem(CACHE, JSON.stringify(all));
  } catch {
    /* storage blocked or full: no saved copy, which is fine */
  }
}

/** The text of an answer, whatever its status: Open-Meteo explains refusals in the body. */
async function fetchText(url: URL): Promise<string> {
  const res = await fetch(url, { headers: { Accept: "application/json" }, signal: AbortSignal.timeout(TIMEOUT_MS) });
  return res.text();
}

/** The core's own message, without its "answer 1:" prefix. */
function plain(e: unknown): string {
  const text = e instanceof Error ? e.message : String(e);
  return text.replace(/^Open-Meteo answer \d+: /, "");
}

export interface LoadOptions {
  /** May we go online right now (device online and online extras on)? */
  online: boolean;
  /** Ask again even if the saved answer is fresh (the Refresh button). */
  force?: boolean;
  now?: number;
  fetchText?: (url: URL) => Promise<string>;
  store?: Store | null;
}

/**
 * Conditions for a place and date: a fresh saved answer if there is one,
 * otherwise a new one from Open-Meteo, otherwise whatever older copy is
 * saved, otherwise nothing. Never throws: a failure is reported in `problem`.
 */
export async function loadConditions(lat: number, lon: number, date: LocalDate, opts: LoadOptions): Promise<Loaded> {
  const store = opts.store === undefined ? browserStore() : opts.store;
  const now = opts.now ?? Date.now();
  const key = cacheKey(lat, lon, date);
  const saved = readSaved(key, store);
  const fallback = (problem?: string): Loaded => ({ stored: saved, from: saved ? "saved" : "none", problem });

  if (saved && !opts.force && now - saved.fetchedAt < FRESH_MS) return fallback();
  if (!opts.online) return fallback();

  const get = opts.fetchText ?? fetchText;
  const urls = conditionsUrls(lat, lon, date);
  const answers = await Promise.allSettled([get(urls.weather), get(urls.aerosols)]);

  // Keep each answer the core can read on its own; remember why the others failed.
  const good: string[] = [];
  const problems: string[] = [];
  const usable = answers.map((a) => {
    if (a.status === "rejected") {
      problems.push("Open-Meteo could not be reached");
      return false;
    }
    try {
      engine.conditionsFromOpenMeteo([a.value], SOURCE);
      good.push(a.value);
      return true;
    } catch (e) {
      problems.push(plain(e));
      return false;
    }
  });
  if (!good.length) return fallback(problems[0]);

  const stored: StoredConditions = {
    series: engine.conditionsFromOpenMeteo(good, SOURCE),
    fetchedAt: now,
    parts: { weather: usable[0], aerosols: usable[1] },
  };
  writeSaved(key, stored, store);
  return { stored, from: "network" };
}

/** Julian Day to UTC text like "2026-11-15T12:00", to the nearest minute. */
export function utcText(jd: number): string {
  const ms = Math.round((jd - 2440587.5) * 1440) * 60_000;
  return new Date(ms).toISOString().slice(0, 16);
}

/**
 * A saved forecast as a skyglow conditions file (docs/CONDITIONS.md): one
 * reading per line, times in UTC. Put it next to an observation and point the
 * observation's `"conditions": { "file": ... }` at it. No coordinates are
 * written into it.
 */
export function conditionsFileText(stored: StoredConditions): string {
  const fetched = new Date(stored.fetchedAt).toISOString().slice(0, 16).replace("T", " ");
  const source = `${stored.series.source}, fetched ${fetched} UTC`;
  const lines = stored.series.samples.map(({ jd, ...values }) => "    " + JSON.stringify({ utc: utcText(jd), ...values }));
  return `{\n  "source": ${JSON.stringify(source)},\n  "samples": [\n${lines.join(",\n")}\n  ]\n}\n`;
}

/** File name for a saved forecast, e.g. "skyglow-conditions-2026-11-15.json". */
export function conditionsFileName(date: LocalDate): string {
  return `skyglow-conditions-${shiftDate(date, 0)}.json`;
}

/** Hand the file to the browser's normal download. */
export function downloadText(name: string, text: string) {
  const url = URL.createObjectURL(new Blob([text], { type: "application/json" }));
  const a = document.createElement("a");
  a.href = url;
  a.download = name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

/** "just now", "12 min ago", "3 h ago", "2 days ago". */
export function ageText(ms: number): string {
  const min = Math.floor(ms / 60_000);
  if (min < 1) return "just now";
  if (min < 60) return `${min} min ago`;
  const h = Math.floor(min / 60);
  if (h < 48) return `${h} h ago`;
  return `${Math.floor(h / 24)} days ago`;
}
