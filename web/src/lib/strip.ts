// Builds the colour strip: for each instant, one cell for the sky (the pack's
// stage swatch) and one cell per cloud layer (the grazing-light swatch while
// lit overhead, a dimmer edge colour while lit only toward the sun, dark
// otherwise). Pure data in, pure data out, so it is easy to test.
//
// These colours are the pack's UNCALIBRATED display guesses, not measured.

import type { Pack, SkyState } from "./types";

/** Colours for states the pack does not colour. */
export const NIGHT = "#141a2e";
export const DAY = "#8fb8e8";
export const SHADOW = "#2a2d3a";
/** Cloud in full daylight (sun above the horizon): white. */
export const CLOUD_DAY = "#e9edf2";
/** Cloud lit only toward the sun: the edge light is deep red-pink. */
export const EDGE = "#7d3150";

export interface StripCell {
  colour: string;
  label: string;
}

export interface StripColumn {
  jd: number;
  sky: StripCell;
  layers: StripCell[];
}

export function skyCell(s: SkyState, pack: Pack): StripCell {
  if (s.stage != null) {
    const st = pack.stages[s.stage];
    return { colour: st.swatch || DAY, label: s.rising && st.morning_name ? st.morning_name : st.name };
  }
  return s.elevation_deg > 0 ? { colour: DAY, label: "day" } : { colour: NIGHT, label: "night" };
}

export function layerCell(s: SkyState, index: number): StripCell {
  const L = s.layers[index].light;
  switch (L.kind) {
    case "lit_overhead":
      if (s.elevation_deg >= 0) return { colour: CLOUD_DAY, label: "daylight" };
      return { colour: L.swatch ?? CLOUD_DAY, label: L.colour ?? "lit" };
    case "lit_toward_sun":
      return { colour: EDGE, label: "lit only toward the sun" };
    default:
      return { colour: SHADOW, label: "in shadow" };
  }
}

export function buildStrip(series: SkyState[], pack: Pack): StripColumn[] {
  return series.map((s) => ({
    jd: s.jd,
    sky: skyCell(s, pack),
    layers: s.layers.map((_, i) => layerCell(s, i)),
  }));
}
