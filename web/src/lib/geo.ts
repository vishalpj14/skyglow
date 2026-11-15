// The two features that can leave the device, both only when the user asks:
//
// * GPS: the browser asks the user's permission; the position stays in the page.
// * Online name search: sends ONLY the typed search text to Photon
//   (https://photon.komoot.io, OpenStreetMap data). Nothing else is sent.
//
// Everything else in Skyglow works offline.

export function gpsPosition(): Promise<{ lat: number; lon: number; elevation_m: number | null }> {
  return new Promise((resolve, reject) => {
    if (!("geolocation" in navigator)) {
      reject(new Error("This browser has no GPS / location support."));
      return;
    }
    navigator.geolocation.getCurrentPosition(
      (pos) =>
        resolve({
          lat: round(pos.coords.latitude, 4),
          lon: round(pos.coords.longitude, 4),
          // Phones sometimes report altitude; it is rough, so round to 10 m.
          elevation_m: pos.coords.altitude == null ? null : Math.max(0, Math.round(pos.coords.altitude / 10) * 10),
        }),
      (err) => reject(new Error(err.message || "Location was not available.")),
      { enableHighAccuracy: false, timeout: 15000, maximumAge: 600000 },
    );
  });
}

export interface SearchHit {
  /** Short name, e.g. "Quito". */
  name: string;
  /** Name with region and country, e.g. "Quito, Pichincha, Ecuador". */
  label: string;
  lat: number;
  lon: number;
}

interface PhotonFeature {
  properties: { name?: string; city?: string; county?: string; state?: string; country?: string; osm_value?: string };
  geometry: { coordinates: [number, number] };
}

/** Turn Photon's answer into search hits, dropping nameless or duplicate entries. */
export function photonHits(features: PhotonFeature[]): SearchHit[] {
  const seen = new Set<string>();
  const out: SearchHit[] = [];
  for (const f of features) {
    const p = f.properties;
    if (!p.name) continue;
    const parts = [p.name, p.city, p.county, p.state, p.country].filter((x, i, a): x is string => !!x && a.indexOf(x) === i);
    const label = parts.join(", ");
    if (seen.has(label)) continue;
    seen.add(label);
    const [lon, lat] = f.geometry.coordinates;
    out.push({ name: p.name, label, lat: round(lat, 4), lon: round(lon, 4) });
  }
  return out;
}

/**
 * Search places by name as you type, using Photon (https://photon.komoot.io),
 * an OpenStreetMap search made for type-ahead. Sends only the typed text.
 * Pass an AbortSignal so an older, slower search cannot overwrite a newer one.
 */
export async function searchOnline(query: string, signal?: AbortSignal): Promise<SearchHit[]> {
  const q = query.trim();
  if (q.length < 2) return [];
  const url = new URL("https://photon.komoot.io/api/");
  url.searchParams.set("q", q);
  url.searchParams.set("limit", "7");
  const limit = AbortSignal.timeout(8000);
  const res = await fetch(url, { signal: signal ? AbortSignal.any([signal, limit]) : limit, headers: { Accept: "application/json" } });
  if (!res.ok) throw new Error(`Search failed (${res.status}).`);
  const body = (await res.json()) as { features?: PhotonFeature[] };
  return photonHits(body.features ?? []);
}

export function round(x: number, places: number): number {
  const f = 10 ** places;
  return Math.round(x * f) / f;
}
