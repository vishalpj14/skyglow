// "Online extras": details Skyglow can fill in for a place when the user has
// switched them on and the device is online. Each one sends ONLY the place's
// coordinates (rounded to about 100 m) to a free public service:
//
//   time zone     Open-Meteo forecast API   (https://open-meteo.com, CC BY 4.0)
//   ground height Open-Meteo elevation API  (Copernicus 90 m terrain model)
//   place name    OpenStreetMap Nominatim   (https://nominatim.org, ODbL)
//
// Answers are cached in this browser, so the same spot is never looked up twice.
// Everything here is optional: if a lookup fails, the app carries on without it.

import { round } from "./geo";

export interface PlaceDetails {
  /** IANA time zone, e.g. "Asia/Kolkata". */
  zone?: string;
  /** Ground height above sea level, metres (terrain model, not your phone's altitude). */
  elevation_m?: number;
  /** Short local name, e.g. "Fort Kochi". */
  name?: string;
  /** Long name with region and country. */
  long_name?: string;
}

const CACHE = "skyglow.placeDetails.v1";

function cacheKey(lat: number, lon: number): string {
  return `${round(lat, 3)},${round(lon, 3)}`;
}
function readCache(): Record<string, PlaceDetails> {
  try {
    return JSON.parse(localStorage.getItem(CACHE) ?? "{}");
  } catch {
    return {};
  }
}
function writeCache(key: string, d: PlaceDetails) {
  try {
    const c = readCache();
    c[key] = d;
    // Keep the cache small: the 200 most recent spots.
    const keys = Object.keys(c);
    for (const k of keys.slice(0, Math.max(0, keys.length - 200))) delete c[k];
    localStorage.setItem(CACHE, JSON.stringify(c));
  } catch {
    /* storage blocked: no cache, which is fine */
  }
}

/** Give up on any one service after this long, so a stalled connection never hangs the app. */
const TIMEOUT_MS = 8000;

async function getJson(url: URL): Promise<unknown> {
  const res = await fetch(url, { headers: { Accept: "application/json" }, signal: AbortSignal.timeout(TIMEOUT_MS) });
  if (!res.ok) throw new Error(`${url.host} answered ${res.status}`);
  return res.json();
}

/** Pick a short, human name from a Nominatim reverse-geocoding answer. */
export function shortName(r: { name?: string; address?: Record<string, string> }): string | undefined {
  const a = r.address ?? {};
  return r.name || a.village || a.town || a.suburb || a.city || a.county || a.state || undefined;
}

export async function lookupDetails(lat: number, lon: number): Promise<PlaceDetails> {
  const key = cacheKey(lat, lon);
  const cached = readCache()[key];
  if (cached) return cached;

  const qLat = String(round(lat, 3));
  const qLon = String(round(lon, 3));

  const zoneUrl = new URL("https://api.open-meteo.com/v1/forecast");
  zoneUrl.search = new URLSearchParams({ latitude: qLat, longitude: qLon, timezone: "auto", forecast_days: "1" }).toString();
  const heightUrl = new URL("https://api.open-meteo.com/v1/elevation");
  heightUrl.search = new URLSearchParams({ latitude: qLat, longitude: qLon }).toString();
  const nameUrl = new URL("https://nominatim.openstreetmap.org/reverse");
  nameUrl.search = new URLSearchParams({ format: "jsonv2", lat: qLat, lon: qLon, zoom: "14" }).toString();

  const [zone, height, name] = await Promise.allSettled([getJson(zoneUrl), getJson(heightUrl), getJson(nameUrl)]);
  const out: PlaceDetails = {};
  if (zone.status === "fulfilled") {
    const z = (zone.value as { timezone?: string }).timezone;
    if (z && z !== "GMT") out.zone = z;
  }
  if (height.status === "fulfilled") {
    const e = (height.value as { elevation?: number[] }).elevation?.[0];
    if (typeof e === "number" && Number.isFinite(e)) out.elevation_m = Math.max(0, Math.round(e));
  }
  if (name.status === "fulfilled") {
    const r = name.value as { name?: string; display_name?: string; address?: Record<string, string> };
    out.name = shortName(r);
    out.long_name = r.display_name;
  }
  if (Object.keys(out).length) writeCache(key, out);
  return out;
}
