// The only file that talks to the WebAssembly bridge (wasm/src/lib.rs).
// Everything else in the web app calls these typed helpers.
//
// The bridge works in JSON text: we stringify the place, pass the pack's JSON
// text through untouched, and parse what comes back.

import init, * as wasm from "../wasm-pkg/skyglow_wasm.js";
import type { ConditionsReading, ConditionsSeries, EventName, Glow, GlowMoment, LocalDate, Pack, Place, SkyState, Timeline } from "./types";

let ready: Promise<void> | null = null;

/** Load the engine once. Pass the .wasm bytes in tests (Node has no fetch of local files). */
export function loadEngine(wasmBytes?: BufferSource): Promise<void> {
  if (!ready) {
    ready = wasmBytes
      ? Promise.resolve(void wasm.initSync({ module: wasmBytes }))
      : init().then(() => undefined);
  }
  return ready;
}

const placeJson = (p: Place) => JSON.stringify(p);

export function defaultPackText(): string {
  return wasm.defaultPack();
}

/** Validate a pack and return it parsed. Throws with the core's message if it is bad. */
export function packInfo(packText: string): Pack {
  return JSON.parse(wasm.packInfo(packText));
}

export function timeline(
  place: Place,
  packText: string,
  date: LocalDate,
  tz: number,
  event: EventName,
): Timeline {
  return JSON.parse(wasm.timeline(placeJson(place), packText, date.year, date.month, date.day, tz, event));
}

export function skyAt(place: Place, packText: string, jd: number): SkyState {
  return JSON.parse(wasm.skyAt(placeJson(place), packText, jd));
}

export function skySeries(
  place: Place,
  packText: string,
  jdStart: number,
  jdEnd: number,
  stepMinutes: number,
): SkyState[] {
  return JSON.parse(wasm.skySeries(placeJson(place), packText, jdStart, jdEnd, stepMinutes));
}

/**
 * Merge the text of Open-Meteo answers (forecast, air quality, or both) into
 * one conditions series. Throws with the core's message if an answer is bad.
 */
export function conditionsFromOpenMeteo(answers: string[], source: string): ConditionsSeries {
  return JSON.parse(wasm.conditionsFromOpenMeteo(JSON.stringify(answers), source));
}

/**
 * The conditions at one instant, or null when the series has nothing close
 * enough. Takes the series as JSON text so a caller asking minute after
 * minute stringifies it once.
 */
export function conditionsAt(seriesText: string, jd: number): ConditionsReading | null {
  return JSON.parse(wasm.conditionsAt(seriesText, jd));
}

/** The glow of every layer at one instant. `seriesText` "" means no conditions (potential only). */
export function glowAt(place: Place, packText: string, seriesText: string, jd: number): Glow {
  return JSON.parse(wasm.glowAt(placeJson(place), packText, seriesText, jd));
}

/** The glow at evenly spaced instants; same grid as `skySeries` for the same arguments. */
export function glowSeries(place: Place, packText: string, seriesText: string, jdStart: number, jdEnd: number, stepMinutes: number): Glow[] {
  return JSON.parse(wasm.glowSeries(placeJson(place), packText, seriesText, jdStart, jdEnd, stepMinutes));
}

/** The best glow moment in a window, or null when no score is known there. */
export function bestGlow(place: Place, packText: string, seriesText: string, jdStart: number, jdEnd: number, stepMinutes: number): GlowMoment | null {
  return JSON.parse(wasm.bestGlow(placeJson(place), packText, seriesText, jdStart, jdEnd, stepMinutes));
}

/** The `hourly=` lists to ask Open-Meteo for; the core owns them so asking and reading stay in step. */
export function openMeteoHourly(): { forecast: string; airQuality: string } {
  return { forecast: wasm.openMeteoForecastHourly(), airQuality: wasm.openMeteoAirQualityHourly() };
}

export function jdFromLocal(date: LocalDate, hours: number, tz: number): number {
  return wasm.jdFromLocal(date.year, date.month, date.day, hours, tz);
}

export function formatClock(jd: number, tz: number): string {
  return wasm.formatClock(jd, tz);
}

export function solarNoon(date: LocalDate, lon: number, tz: number): number {
  return wasm.solarNoon(date.year, date.month, date.day, lon, tz);
}
