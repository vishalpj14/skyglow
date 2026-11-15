// The only file that talks to the WebAssembly bridge (wasm/src/lib.rs).
// Everything else in the web app calls these typed helpers.
//
// The bridge works in JSON text: we stringify the place, pass the pack's JSON
// text through untouched, and parse what comes back.

import init, * as wasm from "../wasm-pkg/skyglow_wasm.js";
import type { EventName, LocalDate, Pack, Place, SkyState, Timeline } from "./types";

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

export function jdFromLocal(date: LocalDate, hours: number, tz: number): number {
  return wasm.jdFromLocal(date.year, date.month, date.day, hours, tz);
}

export function formatClock(jd: number, tz: number): string {
  return wasm.formatClock(jd, tz);
}

export function solarNoon(date: LocalDate, lon: number, tz: number): number {
  return wasm.solarNoon(date.year, date.month, date.day, lon, tz);
}
