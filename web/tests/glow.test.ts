// The glow score through the real engine, and how the app draws and explains
// it. Expected values are worked by hand in the comments.
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { beforeAll, describe, expect, it } from "vitest";
import * as engine from "../src/lib/engine";
import { buildStrip, mixHex, weatherStrength } from "../src/lib/strip";
import { explainLayer, times } from "../src/lib/glowText";
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
    level: "high",
    beam_low_km: 1.4,
    cover_pct: 50,
    factors: { light: 1, sun: 1, colour: 0.9, cover: 1, path: 0.8, view: 0.5, haze: 0.9, humidity: 1 },
    potential: 9,
    score: 3.24,
    word: "modest",
    blocked_by: ["mid"],
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
    expect(text).toContain("passes through middle cloud (33%)");
    expect(text).toContain("through low cloud (50%)");
    expect(text).toContain("aerosol optical depth 0.31, and this beam runs low");
    expect(text).toContain("Humidity 70%");
    // The factors multiply back to the score: 10 x 1 x 0.9 x 1 x 0.8 x 0.5 x 0.9 x 1 = 3.24.
    const product = lines.reduce((p, l) => p * (l.value ?? 1), 10);
    expect(product).toBeCloseTo(3.24, 9);
    expect(times(1)).toBe("×1");
    expect(times(0.456)).toBe("×0.46");
  });
});
