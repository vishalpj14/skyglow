// The glow score through the real engine, and how the app draws and explains
// it. Expected values are worked by hand in the comments.
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { beforeAll, describe, expect, it } from "vitest";
import * as engine from "../src/lib/engine";
import { buildStrip, layersToShow, layerSummary, mixHex, weatherStrength } from "../src/lib/strip";
import { explainLayer, highestCloudText, rangeText, times } from "../src/lib/glowText";
import { heightText } from "../src/lib/describe";
import { examplePlaces } from "../src/lib/places";
import type { LayerGlow } from "../src/lib/types";

const kochi = examplePlaces.find((p) => p.key === "kochi")!.place;
const nov15 = { year: 2026, month: 11, day: 15 };

beforeAll(async () => {
  const bytes = readFileSync(fileURLToPath(new URL("../src/wasm-pkg/skyglow_wasm_bg.wasm", import.meta.url)));
  await engine.loadEngine(bytes);
});

/** A conditions file with the same weather all evening (16:00 to 20:00 IST, 10:30 to 14:30 UTC). */
function steady(values: Record<string, number>): string {
  const samples = ["10:30", "12:30", "14:30"].map((t) => ({ utc: `2026-11-15T${t}`, ...values }));
  return JSON.stringify({ source: "test", samples });
}

describe("glow score in the app", () => {
  it("a clear forecast gives no glow, and the strip fades every cloud row into the sky", () => {
    const clear = steady({ cloud_low_pct: 0, cloud_mid_pct: 0, cloud_high_pct: 0 });
    const set = engine.timeline(kochi, "", nov15, 5.5, "sunset");
    const start = set.event_jd! - 40 / 1440;
    const end = set.event_jd! + 60 / 1440;
    const best = engine.bestGlow(kochi, "", clear, start, end, 1)!;
    expect(best.score).toBe(0);

    const sky = engine.skySeries(kochi, "", start, end, 1);
    const glow = engine.glowSeries(kochi, "", clear, start, end, 1);
    expect(glow).toHaveLength(sky.length);
    const pack = engine.packInfo("");
    const strip = buildStrip(sky, pack, glow);
    // Cover 0 everywhere: strength 0, so every layer cell is the sky's colour.
    for (const col of strip) for (const cell of col.layers) expect(cell.colour).toBe(col.sky.colour);
    expect(strip[30].layers[2].label).toMatch(/no high cloud forecast/);
  });

  it("the best sunset glow with high cloud only is on the high layer, after sunset", () => {
    const cirrus = steady({ cloud_low_pct: 0, cloud_mid_pct: 0, cloud_high_pct: 60, aerosol_optical_depth: 0.1, humidity_pct: 60 });
    const set = engine.timeline(kochi, "", nov15, 5.5, "sunset");
    const best = engine.bestGlow(kochi, "", cirrus, set.event_jd! - 40 / 1440, set.event_jd! + 60 / 1440, 1)!;
    expect(best.layer).toBe("high");
    expect(best.jd).toBeGreaterThan(set.event_jd!);
    expect(best.score).toBeGreaterThan(0);
    expect(best.score).toBeLessThanOrEqual(10);
  });

  it("a bank of low cloud toward the sunset cuts the late glow that the local forecast misses", () => {
    // Clear overhead, 60% high cloud. In one version the sunset points are as
    // clear as here; in the other a solid low deck lies 150 km and more
    // toward the sun. The plan comes from the engine, the answers are written
    // here by hand.
    const plan = engine.weatherPlan(kochi, "", nov15, 5.5);
    const times = [1794744000 - 7200, 1794744000, 1794744000 + 7200]; // 10:00, 12:00, 14:00 UTC
    const answer = (low: number) => ({ hourly: { time: times, cloud_cover_low: [low, low, low], cloud_cover_high: [60, 60, 60] } });
    const lowAt = (i: number, deck: boolean) => {
      if (!deck || i === 0) return 0;
      const sunset = plan.sunset!;
      const k = i - sunset.first;
      return k >= 0 && k < sunset.distances_km.length && sunset.distances_km[k] >= 150 ? 100 : 0;
    };
    const weatherFor = (deck: boolean) =>
      JSON.stringify(engine.weatherFromOpenMeteo(plan, [JSON.stringify(plan.points.map((_, i) => answer(lowAt(i, deck))))], "test"));
    const set = engine.timeline(kochi, "", nov15, 5.5, "sunset");
    // Sun about 3 deg down: the high cloud's beam now dives below 1.5 km, through the low level.
    let jd = set.event_jd!;
    while (engine.skyAt(kochi, "", jd).elevation_deg > -3) jd += 1 / 1440;
    const clear = engine.glowAt(kochi, "", weatherFor(false), jd).layers[2];
    const deck = engine.glowAt(kochi, "", weatherFor(true), jd).layers[2];
    expect(clear.factors.path).toBe(1);
    expect(deck.factors.path).toBeLessThan(0.5);
    expect(deck.crossings.some((x) => x.level === "low" && x.distance_km > 150 && x.cover_pct === 100)).toBe(true);
    expect(deck.score!).toBeLessThan(clear.score!);
    expect(engine.glowAt(kochi, "", weatherFor(true), jd).along_sun).toBe(true);
  });

  it("without a forecast there is only the potential", () => {
    const g = engine.glowAt(kochi, "", "", engine.jdFromLocal(nov15, 18 + 8 / 60, 5.5));
    expect(g.has_conditions).toBe(false);
    expect(g.layers.every((l) => l.score === null)).toBe(true);
    expect(g.layers[2].potential).toBeGreaterThan(0);
  });
});

describe("drawing and explaining", () => {
  it("mixes colours channel by channel", () => {
    // Halfway between black and #ff8040: 0x80, 0x40, 0x20.
    expect(mixHex("#000000", "#ff8040", 0.5)).toBe("#804020");
    expect(mixHex("#123456", "#abcdef", 0)).toBe("#123456");
    expect(mixHex("#123456", "#abcdef", 1)).toBe("#abcdef");
    expect(mixHex("#123456", "#abcdef", 7)).toBe("#abcdef");
  });

  const layer = (over: Partial<LayerGlow>): LayerGlow => ({
    name: "high",
    height_km: 9,
    above_ground_km: 9,
    level: "high",
    range_km: [8, 16],
    cloud_type: "cirrus streaks",
    beam_low_km: 1.4,
    cover_pct: 50,
    factors: { light: 1, sun: 1, colour: 0.9, cover: 1, path: 0.8, view: 0.5, haze: 0.9, humidity: 1 },
    potential: 9,
    score: 3.24,
    word: "modest",
    cover_at_km: 0,
    blocked_by: ["mid"],
    crossings: [{ level: "mid", distance_km: 300.4, cover_pct: 33 }],
    seen_through: ["low"],
    unknown: [],
    ...over,
  });

  it("strength is the weather part of the score", () => {
    // Lit: 1 x 0.8 x 0.5 x 0.9 x 1 = 0.36. Dark: only cover and view, 1 x 0.5.
    expect(weatherStrength(layer({}))).toBeCloseTo(0.36, 9);
    expect(weatherStrength(layer({ factors: { ...layer({}).factors, light: 0 } }))).toBeCloseTo(0.5, 9);
    expect(weatherStrength(layer({ factors: { ...layer({}).factors, cover: null } }))).toBeNull();
  });

  it("explains every factor in words, with the forecast's numbers", () => {
    const lines = explainLayer(layer({}), { cloud_low_pct: 50, cloud_mid_pct: 33, aerosol_optical_depth: 0.31, humidity_pct: 70 });
    const text = lines.map((l) => l.text).join(" | ");
    expect(text).toContain("Lit overhead");
    expect(text).toContain("skims 1.4 km");
    expect(text).toContain("50% high cloud forecast");
    expect(text).toContain("crosses middle cloud 300 km toward the sun (33%)");
    expect(text).toContain("through low cloud (50%)");
    expect(text).toContain("aerosol optical depth 0.31, and this beam runs low");
    expect(text).toContain("Humidity 70%");
    // The factors multiply back to the score: 10 x 1 x 0.9 x 1 x 0.8 x 0.5 x 0.9 x 1 = 3.24.
    const product = lines.reduce((p, l) => p * (l.value ?? 1), 10);
    expect(product).toBeCloseTo(3.24, 9);
    expect(times(1)).toBe("×1");
    expect(times(0.456)).toBe("×0.46");
  });

  it("names both heights at a high place, and the level's range", () => {
    expect(heightText({ height_km: 1.5, above_ground_km: 1.5 })).toBe("1.5 km");
    expect(heightText({ height_km: 4.967, above_ground_km: 1.5 })).toBe("5.0 km (1.5 km above ground)");
    expect(rangeText(layer({}))).toBe("High cloud here: 8.0 to 16.0 km above sea level.");
    // Ground 3.5 km: middle cloud 3 to 8 km above it.
    expect(rangeText(layer({ level: "mid", height_km: 8.5, above_ground_km: 5, range_km: [6.5, 11.5] }))).toBe(
      "Middle cloud here: 6.5 to 11.5 km above sea level (3.0 to 8.0 km above the ground).",
    );
  });
});

describe("a high place (Leh, ground 3467 m)", () => {
  const leh = { ...kochi, name: "Leh", lat: 34.16, lon: 77.58, elevation_m: 3467, tz: 5.5, horizon: [], peaks: [] };
  const nov18 = { year: 2026, month: 11, day: 18 };
  /** Leh 18 Nov: steady weather all evening (15:30 to 19:30 IST is 10:00 to 14:00 UTC). */
  const lehSteady = (values: Record<string, number>) =>
    JSON.stringify({ source: "test", samples: ["10:00", "12:00", "14:00"].map((t) => ({ utc: `2026-11-18T${t}`, ...values })) });

  it("puts every layer its pack height above the ground", () => {
    const t = engine.timeline(leh, "", nov18, 5.5, "sunset");
    // Pack: 1.5 / 5 / 9 km above the ground.
    expect(t.layers.map((l) => l.above_ground_km)).toEqual([1.5, 5, 9].map((h) => expect.closeTo(h, 9)));
    expect(t.layers[0].height_km).toBeCloseTo(4.967, 9);
  });

  it("leaves out rows the forecast says are clear, and sums up a layer", () => {
    const set = engine.timeline(leh, "", nov18, 5.5, "sunset");
    const start = set.event_jd! - 40 / 1440;
    const end = set.event_jd! + 60 / 1440;
    const sky = engine.skySeries(leh, "", start, end, 1);
    // Only high cloud, 70%: low and middle rows go.
    const glow = engine.glowSeries(leh, "", lehSteady({ cloud_low_pct: 0, cloud_mid_pct: 0, cloud_high_pct: 70 }), start, end, 1);
    expect(layersToShow(3, glow)).toEqual([2]);
    // A clear sky: no rows. No forecast: every row.
    const clear = engine.glowSeries(leh, "", lehSteady({ cloud_low_pct: 0, cloud_mid_pct: 0, cloud_high_pct: 0 }), start, end, 1);
    expect(layersToShow(3, clear)).toEqual([]);
    // A trace of cloud (4%) is not enough for a row; 5% is.
    const trace = engine.glowSeries(leh, "", lehSteady({ cloud_low_pct: 4, cloud_mid_pct: 5, cloud_high_pct: 0 }), start, end, 1);
    expect(layersToShow(3, trace)).toEqual([1]);

    // Highest cloud: high 70% at Leh spans 11.467 km to the 16 km ceiling.
    const g = glow[30];
    expect(g.highest_cloud!.level).toBe("high");
    expect(highestCloudText(g, 16)).toBe(
      "Highest cloud overhead: high cloud (70%), somewhere from 11.5 to 16.0 km above sea level. Forecasts give cover per level, not cloud tops; 16 km is the highest a cloud can be.",
    );
    expect(clear[30].highest_cloud).toBeNull();
    expect(highestCloudText(clear[30], 16)).toBe("No cloud overhead in the forecast (under 5% at every level).");
    expect(layersToShow(3, undefined)).toEqual([0, 1, 2]);
    expect(layersToShow(3, engine.glowSeries(leh, "", "", start, end, 1))).toEqual([0, 1, 2]);

    const high = layerSummary(sky, 2, glow)!;
    expect(high.above_ground_km).toBeCloseTo(9, 9);
    // The default pack: high cloud from 8 km above the ground (11.467 km) to the 16 km ceiling.
    expect(high.range_km![0]).toBeCloseTo(11.467, 9);
    expect(high.range_km![1]).toBe(16);
    expect(high.cover_pct).toEqual([70, 70]);
    expect(high.cloud_type).toBe("cirrus and cirrocumulus");
    expect(high.best!.score).toBeGreaterThan(0);
    expect(high.lit![0]).toBe(sky[0].jd);
  });
});
