// The real Rust engine, compiled to WebAssembly, loaded in Node. These check
// the bridge end to end against known facts, not against its own output.
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { beforeAll, describe, expect, it } from "vitest";
import * as engine from "../src/lib/engine";
import { localHours } from "../src/lib/clock";
import { describeLayer, describeSun } from "../src/lib/describe";
import { buildStrip, CLOUD_DAY, NIGHT } from "../src/lib/strip";
import { examplePlaces, searchPlaces } from "../src/lib/places";
import { bundledPacks } from "../src/lib/packs";

const kochi = examplePlaces.find((p) => p.key === "kochi")!.place;
const nov15 = { year: 2026, month: 11, day: 15 };

beforeAll(async () => {
  const bytes = readFileSync(fileURLToPath(new URL("../src/wasm-pkg/skyglow_wasm_bg.wasm", import.meta.url)));
  await engine.loadEngine(bytes);
});

describe("engine bridge", () => {
  it("gives the almanac sunset and sunrise for Kochi", () => {
    // Almanac, Kochi mid-November: sunrise about 06:19, sunset about 17:59-18:00 IST.
    const set = engine.timeline(kochi, "", nov15, 5.5, "sunset");
    const rise = engine.timeline(kochi, "", nov15, 5.5, "sunrise");
    const hs = localHours(set.event_jd!, 5.5);
    const hr = localHours(rise.event_jd!, 5.5);
    expect(hs).toBeGreaterThan(17 + 56 / 60);
    expect(hs).toBeLessThan(18 + 3 / 60);
    expect(hr).toBeGreaterThan(6 + 16 / 60);
    expect(hr).toBeLessThan(6 + 23 / 60);
  });

  it("matches the CLI wording for the sky at an instant", () => {
    // Step until the sun is just past 3 degrees down (it drops ~0.25 deg a
    // minute here). Overhead clouds must then be above R(1/cos d - 1), worked
    // here independently: about 8.7 km at 3.0 deg, 9.6 km at 3.14 deg.
    const noon = engine.solarNoon(nov15, kochi.lon, 5.5);
    let jd = noon;
    while (engine.skyAt(kochi, "", jd).elevation_deg > -3) jd += 1 / 1440;
    const s = engine.skyAt(kochi, "", jd);
    expect(s.rising).toBe(false);
    const lines = describeSun(s);
    expect(lines[0]).toMatch(/below the horizon \(setting\)/);
    const expected = 6371 * (1 / Math.cos((s.depression_deg * Math.PI) / 180) - 1);
    expect(s.depression_deg).toBeGreaterThan(3);
    expect(s.depression_deg).toBeLessThan(3.3);
    expect(lines[1]).toContain(`higher than ${expected.toFixed(1)} km`);
    expect(s.layers.map(describeLayer).every((t) => t.length > 10)).toBe(true);
  });

  it("builds a day of sky states and a colour strip", () => {
    const start = engine.jdFromLocal(nov15, 0, 5.5);
    const day = engine.skySeries(kochi, "", start, start + 1, 10);
    expect(day).toHaveLength(145); // 24 h / 10 min + 1
    const pack = engine.packInfo("");
    const strip = buildStrip(day, pack);
    // Midnight: night sky. Noon: daylit white clouds.
    expect(strip[0].sky.colour).toBe(NIGHT);
    expect(strip[72].layers.every((c) => c.colour === CLOUD_DAY)).toBe(true);
    // Some time in the evening a layer takes a pack swatch.
    const swatches = new Set(pack.grazing_colours.map((g) => g.swatch));
    expect(strip.some((c) => c.layers.some((l) => swatches.has(l.colour)))).toBe(true);
  });

  it("reports bad input as an error with the core's message", () => {
    expect(() => engine.timeline({ ...kochi, lat: 95 }, "", nov15, 5.5, "sunset")).toThrow(/latitude/i);
    expect(() => engine.packInfo('{"schema": 99}')).toThrow();
  });

  it("bundles the example places and packs", () => {
    expect(examplePlaces.length).toBeGreaterThanOrEqual(14);
    expect(searchPlaces(examplePlaces, "koc")[0].key).toBe("kochi");
    const keys = bundledPacks.map((p) => p.key);
    expect(keys[0]).toBe("default");
    expect(keys).toContain("kerala");
    for (const p of bundledPacks) expect(engine.packInfo(p.text).name).toBe(p.key);
  });
});
