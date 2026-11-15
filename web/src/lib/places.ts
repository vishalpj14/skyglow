// Places the app knows without a network: the public example cities in
// examples/places/ (bundled at build time), plus places the user saved on
// this device. Saved places live only in this browser's storage and are
// never sent anywhere.

import type { Place } from "./types";
import type { ClockChoice } from "./clock";

const files = import.meta.glob<Place>("../../../examples/places/*.json", {
  eager: true,
  import: "default",
});

/** Fill in the optional fields so every Place has the same shape. */
export function normalisePlace(p: Partial<Place> & { lat: number; lon: number }, key = ""): Place {
  return {
    name: p.name || key,
    lat: p.lat,
    lon: p.lon,
    tz: p.tz ?? null,
    tz_name: p.tz_name ?? "",
    dst_tz: p.dst_tz ?? null,
    dst_tz_name: p.dst_tz_name ?? "",
    dst_note: p.dst_note ?? "",
    elevation_m: p.elevation_m ?? 0,
    horizon: (p.horizon ?? []).map((h) => ({ ...h })),
    peaks: (p.peaks ?? []).map((k) => ({ ...k })),
    note: p.note ?? "",
    ...(p.zone ? { zone: p.zone } : {}),
  };
}

export interface NamedPlace {
  key: string;
  place: Place;
  saved: boolean;
  /** The clock the user chose for this saved place; none means "this device's clock". */
  clock?: ClockChoice;
}

export const examplePlaces: NamedPlace[] = Object.entries(files)
  .map(([path, place]) => {
    const key = path.split("/").pop()!.replace(/\.json$/, "");
    return { key, place: normalisePlace(place, key), saved: false };
  })
  .sort((a, b) => a.place.name.localeCompare(b.place.name));

/** Case-insensitive match on name or key, best (prefix) matches first. */
export function searchPlaces(list: NamedPlace[], query: string): NamedPlace[] {
  const q = query.trim().toLowerCase();
  if (!q) return list;
  const hits = list.filter((p) => p.place.name.toLowerCase().includes(q) || p.key.includes(q));
  const starts = (p: NamedPlace) => p.place.name.toLowerCase().startsWith(q) || p.key.startsWith(q);
  return [...hits.filter(starts), ...hits.filter((p) => !starts(p))];
}

const STORE_V1 = "skyglow.savedPlaces.v1";
const STORE = "skyglow.savedPlaces.v2";

/** Saved places on this device. Storage can be blocked (private mode), so never throw. */
export function loadSaved(): NamedPlace[] {
  try {
    const raw = localStorage.getItem(STORE);
    if (raw) {
      const obj = JSON.parse(raw) as Record<string, { place: Place; clock?: ClockChoice }>;
      return Object.entries(obj).map(([key, v]) => ({ key, place: normalisePlace(v.place, key), saved: true, clock: v.clock }));
    }
    // Version 1 stored the bare place.
    const old = localStorage.getItem(STORE_V1);
    if (!old) return [];
    const obj = JSON.parse(old) as Record<string, Place>;
    return Object.entries(obj).map(([key, p]) => ({ key, place: normalisePlace(p, key), saved: true }));
  } catch {
    return [];
  }
}

export function saveAll(list: NamedPlace[]): boolean {
  try {
    const obj: Record<string, { place: Place; clock?: ClockChoice }> = {};
    for (const p of list) obj[p.key] = p.clock ? { place: p.place, clock: p.clock } : { place: p.place };
    localStorage.setItem(STORE, JSON.stringify(obj));
    return true;
  } catch {
    return false;
  }
}

/** A storage key from a display name, same rule as the CLI's --place names. */
export function keyFor(name: string): string {
  const k = name
    .toLowerCase()
    .replace(/[^a-z0-9_-]+/g, "_")
    .replace(/^_+|_+$/g, "");
  return k || "place";
}
