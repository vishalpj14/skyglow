import { describe, expect, it } from "vitest";
import { coloursIn, hueGradient, paintColourWords, sweepFor } from "../src/lib/hues";

const GOLD = "#f2b45a";
const ORANGE = "#f08a3c";
const ORANGE_RED = "#e0566b";
const RED = "#d9483b";
const PINK = "#e87ba4";

describe("colour words", () => {
  it("finds colours in the order a phrase names them", () => {
    expect(coloursIn("gold or bright orange")).toEqual([GOLD, ORANGE]);
    expect(coloursIn("deep red and pink")).toEqual([RED, PINK]);
    expect(coloursIn("orange, then orange-red")).toEqual([ORANGE, ORANGE_RED]);
  });

  it("does not split longer names or match inside other words", () => {
    expect(coloursIn("orange-red")).toEqual([ORANGE_RED]); // not orange + red
    expect(coloursIn("the redder it gets")).toEqual(["#a51d2d"]); // "redder" is its own, deeper red, not "red"
    expect(coloursIn("Golden hour")).toEqual([GOLD]); // the stage, once
    expect(coloursIn("Sunset glow")).toHaveLength(1);
    expect(coloursIn("Save on this device")).toEqual([]);
  });

  it("drops a colour repeated straight after itself", () => {
    expect(coloursIn("pink, pink and more pink")).toEqual([PINK]);
  });

  it("builds a gradient even from one colour", () => {
    expect(hueGradient([PINK])).toBe(`linear-gradient(90deg, ${PINK}, ${PINK})`);
    expect(hueGradient([RED, PINK])).toBe(`linear-gradient(90deg, ${RED}, ${PINK})`);
  });

  it("sweeps a headline accent into its twilight neighbour", () => {
    expect(sweepFor("#c8607e")).toBe("linear-gradient(90deg, #c8607e, #7a5a9e, #c8607e)"); // pink into purple
    expect(sweepFor("#2e4a7d")).toBe("linear-gradient(90deg, #2e4a7d, #7a5a9e, #2e4a7d)"); // last one looks back
  });

  it("marks only bold phrases that name a colour", () => {
    const html = "<p><strong>deep red and pink</strong> and <strong>Save</strong>, plus pink in plain text</p>";
    const out = paintColourWords(html);
    expect(out).toContain(
      '<strong class="hue" style="--hue-paint: linear-gradient(90deg, color-mix(in srgb, #d9483b 80%, var(--text)), color-mix(in srgb, #e87ba4 80%, var(--text)))">deep red and pink</strong>',
    );
    expect(out).toContain("<strong>Save</strong>");
    expect(out).toContain("plus pink in plain text");
  });
});
