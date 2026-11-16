// Live conditions: asking Open-Meteo, reading its answers with the real
// engine, and keeping a copy for offline. No network here: the "service" is a
// function that returns hand-written answers.
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { beforeAll, describe, expect, it } from "vitest";
import * as engine from "../src/lib/engine";
import {
  ageText,
  cacheKey,
  conditionsFileName,
  conditionsFileText,
  conditionsUrls,
  FRESH_MS,
  loadConditions,
  readSaved,
  shiftDate,
  utcText,
  type Store,
} from "../src/lib/conditions";

beforeAll(async () => {
  const bytes = readFileSync(fileURLToPath(new URL("../src/wasm-pkg/skyglow_wasm_bg.wasm", import.meta.url)));
  await engine.loadEngine(bytes);
});

// 2000-01-01 12:00 UTC is Unix 946728000 and Julian Day 2451545.0 (J2000).
const J2000 = 2451545.0;
const T0 = 946728000;
const WEATHER = JSON.stringify({
  hourly: { time: [T0, T0 + 3600], cloud_cover_low: [10, 30], cloud_cover_high: [40, 60], relative_humidity_2m: [70, 80] },
});
const AEROSOLS = JSON.stringify({ hourly: { time: [T0, T0 + 3600], aerosol_optical_depth: [0.2, 0.4] } });
const REFUSED = JSON.stringify({ error: true, reason: "Parameter 'start_date' is out of allowed range" });

const jan1 = { year: 2000, month: 1, day: 1 };

function memoryStore(): Store & { data: Map<string, string> } {
  const data = new Map<string, string>();
  return { data, getItem: (k) => data.get(k) ?? null, setItem: (k, v) => void data.set(k, v) };
}

/** A pretend Open-Meteo that counts how often it is asked. */
function service(weather: string | Error, aerosols: string | Error) {
  const asked: URL[] = [];
  const fetchText = async (url: URL) => {
    asked.push(url);
    const answer = url.host.startsWith("air-quality") ? aerosols : weather;
    if (answer instanceof Error) throw answer;
    return answer;
  };
  return { asked, fetchText };
}

describe("asking Open-Meteo", () => {
  it("sends only rounded coordinates and three dates", () => {
    const { weather, aerosols } = conditionsUrls(9.931234, 76.267891, { year: 2026, month: 11, day: 15 });
    expect(weather.origin + weather.pathname).toBe("https://api.open-meteo.com/v1/forecast");
    expect(aerosols.origin + aerosols.pathname).toBe("https://air-quality-api.open-meteo.com/v1/air-quality");
    for (const u of [weather, aerosols]) {
      expect(u.searchParams.get("latitude")).toBe("9.93");
      expect(u.searchParams.get("longitude")).toBe("76.27");
      expect(u.searchParams.get("start_date")).toBe("2026-11-14");
      expect(u.searchParams.get("end_date")).toBe("2026-11-16");
      expect(u.searchParams.get("timezone")).toBe("GMT");
      expect([...u.searchParams.keys()].sort()).toEqual(["end_date", "hourly", "latitude", "longitude", "start_date", "timeformat", "timezone"]);
    }
    expect(weather.searchParams.get("hourly")).toContain("cloud_cover_high");
    expect(weather.searchParams.get("hourly")).toContain("total_column_integrated_water_vapour");
    expect(aerosols.searchParams.get("hourly")).toBe("aerosol_optical_depth");
  });

  it("steps dates across month and year ends", () => {
    expect(shiftDate({ year: 2026, month: 3, day: 1 }, -1)).toBe("2026-02-28");
    expect(shiftDate({ year: 2024, month: 3, day: 1 }, -1)).toBe("2024-02-29"); // leap year
    expect(shiftDate({ year: 2026, month: 12, day: 31 }, 1)).toBe("2027-01-01");
  });

  it("shares one saved answer between spots within about a kilometre", () => {
    expect(cacheKey(9.931, 76.268, jan1)).toBe(cacheKey(9.934, 76.271, jan1));
    expect(cacheKey(9.931, 76.268, jan1)).not.toBe(cacheKey(9.95, 76.268, jan1));
    expect(cacheKey(9.931, 76.268, jan1)).not.toBe(cacheKey(9.931, 76.268, { ...jan1, day: 2 }));
  });
});

describe("loading conditions", () => {
  it("merges both services and reads halfway between two hours", async () => {
    const store = memoryStore();
    const s = service(WEATHER, AEROSOLS);
    const r = await loadConditions(9.93, 76.27, jan1, { online: true, now: 1000, fetchText: s.fetchText, store });
    expect(r.from).toBe("network");
    expect(r.stored!.parts).toEqual({ weather: true, aerosols: true });
    expect(r.stored!.series.samples).toHaveLength(2);
    expect(r.stored!.series.samples[0].jd).toBe(J2000);
    // 12:30 UTC: high cloud (40 + 60) / 2 = 50, haze (0.2 + 0.4) / 2 = 0.3.
    const at = engine.conditionsAt(JSON.stringify(r.stored!.series), J2000 + 0.5 / 24)!;
    expect(at.conditions.cloud_high_pct).toBeCloseTo(50, 6);
    expect(at.conditions.aerosol_optical_depth).toBeCloseTo(0.3, 6);
    expect(at.basis.kind).toBe("between");
    // Three hours past the last reading: nothing, rather than a guess.
    expect(engine.conditionsAt(JSON.stringify(r.stored!.series), J2000 + 4 / 24)).toBeNull();
  });

  it("uses a fresh saved answer without asking again, and asks again when it is old or forced", async () => {
    const store = memoryStore();
    const s = service(WEATHER, AEROSOLS);
    const opts = { online: true, fetchText: s.fetchText, store };
    await loadConditions(9.93, 76.27, jan1, { ...opts, now: 0 });
    expect(s.asked).toHaveLength(2);

    const soon = await loadConditions(9.93, 76.27, jan1, { ...opts, now: FRESH_MS - 1 });
    expect(soon.from).toBe("saved");
    expect(s.asked).toHaveLength(2);

    const later = await loadConditions(9.93, 76.27, jan1, { ...opts, now: FRESH_MS + 1 });
    expect(later.from).toBe("network");
    expect(s.asked).toHaveLength(4);

    await loadConditions(9.93, 76.27, jan1, { ...opts, now: FRESH_MS + 2, force: true });
    expect(s.asked).toHaveLength(6);
  });

  it("offline, shows the saved copy however old, and never goes online", async () => {
    const store = memoryStore();
    const s = service(WEATHER, AEROSOLS);
    await loadConditions(9.93, 76.27, jan1, { online: true, now: 0, fetchText: s.fetchText, store });
    const r = await loadConditions(9.93, 76.27, jan1, { online: false, now: 30 * FRESH_MS, fetchText: s.fetchText, store });
    expect(r.from).toBe("saved");
    expect(r.stored!.fetchedAt).toBe(0);
    expect(s.asked).toHaveLength(2);

    const elsewhere = await loadConditions(48.85, 2.35, jan1, { online: false, fetchText: s.fetchText, store });
    expect(elsewhere).toEqual({ stored: null, from: "none", problem: undefined });
    expect(s.asked).toHaveLength(2);
  });

  it("keeps going with one service when the other fails", async () => {
    const noHaze = await loadConditions(9.93, 76.27, jan1, { online: true, ...service(WEATHER, new Error("down")), store: memoryStore() });
    expect(noHaze.from).toBe("network");
    expect(noHaze.stored!.parts).toEqual({ weather: true, aerosols: false });
    expect(noHaze.stored!.series.samples[0].aerosol_optical_depth).toBeUndefined();

    const hazeOnly = await loadConditions(9.93, 76.27, jan1, { online: true, ...service(REFUSED, AEROSOLS), store: memoryStore() });
    expect(hazeOnly.stored!.parts).toEqual({ weather: false, aerosols: true });
    expect(hazeOnly.stored!.series.samples[0].cloud_high_pct).toBeUndefined();
  });

  it("says why when nothing could be fetched, and falls back to the saved copy", async () => {
    const store = memoryStore();
    const nothing = await loadConditions(9.93, 76.27, jan1, { online: true, ...service(REFUSED, REFUSED), store });
    expect(nothing.stored).toBeNull();
    expect(nothing.from).toBe("none");
    expect(nothing.problem).toBe("Open-Meteo said: Parameter 'start_date' is out of allowed range");

    await loadConditions(9.93, 76.27, jan1, { online: true, now: 0, ...service(WEATHER, AEROSOLS), store });
    const down = await loadConditions(9.93, 76.27, jan1, {
      online: true,
      now: 2 * FRESH_MS,
      ...service(new Error("down"), new Error("down")),
      store,
    });
    expect(down.from).toBe("saved");
    expect(down.stored!.fetchedAt).toBe(0);
    expect(down.problem).toBe("Open-Meteo could not be reached");
  });

  it("works with no storage at all, and ignores a damaged saved copy", async () => {
    const r = await loadConditions(9.93, 76.27, jan1, { online: true, ...service(WEATHER, AEROSOLS), store: null });
    expect(r.from).toBe("network");

    const store = memoryStore();
    store.data.set("skyglow.conditions.v1", "{not json");
    expect(readSaved(cacheKey(9.93, 76.27, jan1), store)).toBeNull();
    store.data.set("skyglow.conditions.v1", JSON.stringify({ [cacheKey(9.93, 76.27, jan1)]: { series: "oops" } }));
    expect(readSaved(cacheKey(9.93, 76.27, jan1), store)).toBeNull();
  });

  it("keeps only the twelve most recent answers", async () => {
    const store = memoryStore();
    const s = service(WEATHER, AEROSOLS);
    for (let day = 1; day <= 14; day++) {
      await loadConditions(9.93, 76.27, { ...jan1, day }, { online: true, now: day, fetchText: s.fetchText, store });
    }
    expect(readSaved(cacheKey(9.93, 76.27, { ...jan1, day: 1 }), store)).toBeNull();
    expect(readSaved(cacheKey(9.93, 76.27, { ...jan1, day: 2 }), store)).toBeNull();
    expect(readSaved(cacheKey(9.93, 76.27, { ...jan1, day: 3 }), store)).not.toBeNull();
    expect(readSaved(cacheKey(9.93, 76.27, { ...jan1, day: 14 }), store)).not.toBeNull();
  });
});

describe("saving a forecast as a file", () => {
  it("writes a conditions file the engine reads back to the same numbers", async () => {
    const r = await loadConditions(9.93, 76.27, jan1, {
      online: true,
      now: Date.UTC(2000, 0, 1, 14, 5),
      ...service(WEATHER, AEROSOLS),
      store: memoryStore(),
    });
    const text = conditionsFileText(r.stored!);
    // UTC times a person can read, one reading per line, and no coordinates.
    expect(text).toContain('"utc":"2000-01-01T12:00"');
    expect(text).toContain('"utc":"2000-01-01T13:00"');
    expect(text).toContain('"source": "Open-Meteo forecast, fetched 2000-01-01 14:05 UTC"');
    expect(text).not.toMatch(/9\.93|76\.27|latitude/);
    expect(text.split("\n").filter((l) => l.includes('"utc"'))).toHaveLength(2);
    // Read back by the core: halfway, low cloud (10 + 30) / 2 = 20, haze 0.3.
    const at = engine.conditionsAt(text, J2000 + 0.5 / 24)!;
    expect(at.conditions.cloud_low_pct).toBeCloseTo(20, 6);
    expect(at.conditions.aerosol_optical_depth).toBeCloseTo(0.3, 6);
  });

  it("rounds times to the minute and names the file by date", () => {
    expect(utcText(J2000)).toBe("2000-01-01T12:00");
    expect(utcText(J2000 + 0.4 / 1440)).toBe("2000-01-01T12:00");
    expect(utcText(J2000 + 0.6 / 1440)).toBe("2000-01-01T12:01");
    expect(conditionsFileName({ year: 2026, month: 11, day: 4 })).toBe("skyglow-conditions-2026-11-04.json");
  });
});

describe("age in words", () => {
  it("reads like a person would say it", () => {
    expect(ageText(20_000)).toBe("just now");
    expect(ageText(12 * 60_000)).toBe("12 min ago");
    expect(ageText(3 * 3_600_000 + 5 * 60_000)).toBe("3 h ago");
    expect(ageText(50 * 3_600_000)).toBe("2 days ago");
  });
});
