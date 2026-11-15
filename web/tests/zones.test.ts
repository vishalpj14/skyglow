import { describe, expect, it } from "vitest";
import { offsetForZone, resolveClock } from "../src/lib/clock";
import { normalisePlace } from "../src/lib/places";
import { shortName } from "../src/lib/online";

const jan = { year: 2026, month: 1, day: 15 };
const jul = { year: 2026, month: 7, day: 15 };

describe("time zones", () => {
  it("knows fixed and daylight-saving offsets", () => {
    // Known facts: India is UTC+5:30 all year, Nepal +5:45; New York is
    // UTC-5 in winter and UTC-4 in summer; Sydney is +11 in January, +10 in July.
    expect(offsetForZone("Asia/Kolkata", jan)).toBe(5.5);
    expect(offsetForZone("Asia/Kolkata", jul)).toBe(5.5);
    expect(offsetForZone("Asia/Kathmandu", jan)).toBe(5.75);
    expect(offsetForZone("America/New_York", jan)).toBe(-5);
    expect(offsetForZone("America/New_York", jul)).toBe(-4);
    expect(offsetForZone("Australia/Sydney", jan)).toBe(11);
    expect(offsetForZone("Australia/Sydney", jul)).toBe(10);
    expect(offsetForZone("Europe/London", jan)).toBe(0);
    expect(offsetForZone("Not/AZone", jan)).toBeNull();
  });

  it("uses this device's clock unless told otherwise", () => {
    const tokyo = normalisePlace({ name: "Tokyo", lat: 35.68, lon: 139.69, tz: 9 });
    const device = { tz: 5.5, zone: "Asia/Kolkata" };
    expect(resolveClock({ mode: "device" }, tokyo, false, jan, device)).toMatchObject({ tz: 5.5, guessed: false });
    expect(resolveClock({ mode: "place" }, tokyo, false, jan, device).tz).toBe(9);
    expect(resolveClock({ mode: "custom", tz: -3 }, tokyo, false, jan, device).tz).toBe(-3);
  });

  it("prefers a looked-up time zone, which handles daylight saving by itself", () => {
    const ny = normalisePlace({ name: "NY", lat: 40.7, lon: -74, zone: "America/New_York" });
    const device = { tz: 0, zone: "UTC" };
    expect(resolveClock({ mode: "place" }, ny, false, jan, device).tz).toBe(-5);
    expect(resolveClock({ mode: "place" }, ny, false, jul, device).tz).toBe(-4);
    // No zone and no offset: a longitude guess, flagged as one.
    const bare = normalisePlace({ name: "x", lat: 0, lon: -74 });
    expect(resolveClock({ mode: "place" }, bare, false, jan, device)).toMatchObject({ tz: -5, guessed: true });
  });

  it("picks a short name from a reverse-geocoding answer", () => {
    expect(shortName({ name: "Fort Kochi", address: { city: "Kochi" } })).toBe("Fort Kochi");
    expect(shortName({ name: "", address: { village: "Cumbaya", state: "Pichincha" } })).toBe("Cumbaya");
    expect(shortName({ address: { state: "Kerala" } })).toBe("Kerala");
    expect(shortName({})).toBeUndefined();
  });
});

import { photonHits } from "../src/lib/geo";

describe("online search results", () => {
  it("builds readable labels and drops nameless or repeated places", () => {
    const f = (name: string | undefined, state: string, lon: number, lat: number) => ({
      properties: { name, state, country: "Ecuador" },
      geometry: { coordinates: [lon, lat] as [number, number] },
    });
    const hits = photonHits([
      f("Quito", "Pichincha", -78.5124956, -0.2201998),
      f("Quito", "Pichincha", -78.51, -0.22), // same label: a duplicate
      f(undefined, "Pichincha", -78, 0), // no name: useless in a list
      f("Pichincha", "Pichincha", -78.6, -0.1), // name equals state: not repeated
    ]);
    expect(hits).toEqual([
      { name: "Quito", label: "Quito, Pichincha, Ecuador", lat: -0.2202, lon: -78.5125 },
      { name: "Pichincha", label: "Pichincha, Ecuador", lat: -0.1, lon: -78.6 },
    ]);
  });
});

import { compareZones } from "../src/lib/clock";

describe("comparing a faraway place's time with yours", () => {
  // 2026-01-15 12:00 UTC as a Julian Day: JD 2461056.0 (J2000 = 2451545.0 on
  // 2000-01-01 12:00, plus 9511 days).
  const jdNoonUtc = 2451545.0 + 9511;
  const jan = { year: 2026, month: 1, day: 15 };
  const india = { tz: 5.5, zone: "Asia/Kolkata" };

  it("says nothing when the place is on your own clock", () => {
    const kochi = normalisePlace({ name: "Kochi", lat: 9.93, lon: 76.27, zone: "Asia/Kolkata" });
    expect(compareZones(kochi, false, jan, jdNoonUtc, 5.5, india)).toBeNull();
  });

  it("lists the place, UTC and you, with day changes", () => {
    const tokyo = normalisePlace({ name: "Tokyo", lat: 35.68, lon: 139.69, zone: "Asia/Tokyo" });
    const rows = compareZones(tokyo, false, jan, jdNoonUtc, 5.5, india)!;
    expect(rows.map((r) => [r.who, r.time, r.dayShift, r.shown])).toEqual([
      ["place", "21:00", 0, false], // 12:00 UTC + 9 h
      ["utc", "12:00", 0, false],
      ["you", "17:30", 0, true], // 12:00 UTC + 5.5 h, the clock on screen
    ]);
    expect(rows[0].zone).toBe("Asia/Tokyo, UTC+9");
    // At 20:00 UTC it is already the next day in Tokyo (05:00) but not in India (01:30 next day too).
    const late = compareZones(tokyo, false, jan, jdNoonUtc + 8 / 24, 0, india)!;
    expect(late.map((r) => [r.who, r.time, r.dayShift])).toEqual([
      ["place", "05:00", 1],
      ["utc", "20:00", 0],
      ["you", "01:30", 1],
    ]);
  });

  it("guesses a zone from longitude when none is known, and says so", () => {
    const sea = normalisePlace({ name: "Mid-Pacific", lat: 0, lon: -150 });
    const rows = compareZones(sea, false, jan, jdNoonUtc, 5.5, india)!;
    expect(rows[0].time).toBe("02:00"); // -150 / 15 = UTC-10
    expect(rows[0].zone).toMatch(/^about UTC-10, guessed/);
    expect(rows[0].dayShift).toBe(0); // 02:00 on the 15th, same day as 17:30 in India
  });

  it("leaves UTC out when it is the place's own clock", () => {
    const london = normalisePlace({ name: "London", lat: 51.5, lon: -0.1, zone: "Europe/London" });
    const rows = compareZones(london, false, jan, jdNoonUtc, 5.5, india)!;
    expect(rows.map((r) => r.who)).toEqual(["place", "you"]);
  });
});
