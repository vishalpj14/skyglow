import { describe, expect, it } from "vitest";
import { along, DAY, hexToRgb, mix, moodFrames, NIGHT, rgbToHex } from "../src/lib/mood";

describe("logo animation colours", () => {
  it("blends colours by hand-checkable amounts", () => {
    expect(hexToRgb("#ff8000")).toEqual([255, 128, 0]);
    expect(rgbToHex([255, 128, 0])).toBe("#ff8000");
    // Halfway between black and white is mid grey (127.5 rounds to 128 = 0x80).
    expect(mix("#000000", "#ffffff", 0.5)).toBe("#808080");
    expect(mix("#123456", "#abcdef", 0)).toBe("#123456");
    expect(mix("#123456", "#abcdef", 1)).toBe("#abcdef");
  });

  it("walks along a list of colours", () => {
    const list = ["#000000", "#ff0000", "#ffffff"];
    expect(along(list, 0)).toBe("#000000");
    expect(along(list, 0.5)).toBe("#ff0000");
    expect(along(list, 1)).toBe("#ffffff");
    expect(along(list, 0.25)).toBe("#800000"); // halfway from black to red
    expect(along(list, -1)).toBe("#000000"); // clamped
    expect(along(list, 2)).toBe("#ffffff");
  });

  it("plays a sunset from day to night, and a sunrise the other way", () => {
    const swatches = ["#f2b45a", "#e9794a", "#c8607e"];
    expect(moodFrames({ rising: false, swatches })).toEqual([DAY, ...swatches, NIGHT]);
    expect(moodFrames({ rising: true, swatches })).toEqual([NIGHT, ...[...swatches].reverse(), DAY]);
    // Stages without a swatch are skipped.
    expect(moodFrames({ rising: false, swatches: ["", "#c8607e"] })).toEqual([DAY, "#c8607e", NIGHT]);
  });
});

import { logoFrame } from "../src/lib/mood";

describe("logo sunset and sunrise", () => {
  const swatches = ["#f2b45a", "#e9794a", "#c8607e", "#7a5a9e", "#2e4a7d"];
  const evening = { rising: false, swatches };
  const morning = { rising: true, swatches };

  it("sets the sun from a day sky into night", () => {
    const start = logoFrame(evening, 0);
    const end = logoFrame(evening, 1);
    expect(start.bottom).toBe(DAY);
    expect(start.sunY).toBe(16); // high
    expect(end.bottom).toBe(NIGHT);
    expect(end.sunY).toBe(62); // below the horizon at 46 (radius 10)
  });

  it("rises from night into day in the morning", () => {
    expect(logoFrame(morning, 0).bottom).toBe(NIGHT);
    expect(logoFrame(morning, 0).sunY).toBe(62);
    expect(logoFrame(morning, 1).bottom).toBe(DAY);
    expect(logoFrame(morning, 1).sunY).toBe(16);
    // Halfway through a sunrise is the same sky as halfway through a sunset.
    expect(logoFrame(morning, 0.5).bottom).toBe(logoFrame(evening, 0.5).bottom);
  });

  it("keeps the high cloud lit longer at dusk and lights it first at dawn", () => {
    // Late in a sunset the sky has gone to night, but the cloud still shows blue-hour colour.
    expect(logoFrame(evening, 1).cloud).not.toBe(NIGHT);
    // Early in a sunrise the sky is still night, but the cloud already has colour.
    expect(logoFrame(morning, 0).cloud).not.toBe(NIGHT);
  });
});
