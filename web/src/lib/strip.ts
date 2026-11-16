// Builds the colour strip: for each instant, one cell for the sky (the pack's
// stage swatch) and one cell per cloud layer (the grazing-light swatch while
// lit overhead, a dimmer edge colour while lit only toward the sun, dark
// otherwise). Pure data in, pure data out, so it is easy to test.
//
// These colours are the pack's UNCALIBRATED display guesses, not measured.
// With a forecast, each layer also fades by how much the weather says it
// will show (see weatherStrength): also a rough guide.

import type { Glow, LayerGlow, Pack, SkyState } from "./types";

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

/** Mix two #rrggbb colours: t = 0 gives `a`, t = 1 gives `b`. */
export function mixHex(a: string, b: string, t: number): string {
  const ch = (hex: string, i: number) => parseInt(hex.slice(1 + 2 * i, 3 + 2 * i), 16);
  const k = Math.min(1, Math.max(0, t));
  return "#" + [0, 1, 2].map((i) => Math.round(ch(a, i) + (ch(b, i) - ch(a, i)) * k).toString(16).padStart(2, "0")).join("");
}

/**
 * How strongly the forecast says this layer shows at all, 0 to 1: the glow
 * factors that are about the weather (cloud there, sunbeam not blocked, seen
 * past lower cloud, haze, damp). Geometry is already in the cell's colour.
 * For a dark layer only the cover counts (a dark cloud still hides the sky).
 * Null when its cover is not known.
 */
export function weatherStrength(l: LayerGlow): number | null {
  const f = l.factors;
  if (f.cover == null) return null;
  return f.light > 0 ? f.cover * f.path * f.view * f.haze * f.humidity : f.cover * f.view;
}

/**
 * Columns for the strip. With `glow` (one entry per instant, from the same
 * grid), each layer's cell fades toward the sky's colour as the forecast
 * says less of it will show, and its label carries the score.
 */
export function buildStrip(series: SkyState[], pack: Pack, glow?: Glow[]): StripColumn[] {
  return series.map((s, k) => {
    const sky = skyCell(s, pack);
    const g = glow?.[k];
    return {
      jd: s.jd,
      sky,
      layers: s.layers.map((_, i) => {
        const cell = layerCell(s, i);
        const lg = g?.has_conditions ? g.layers[i] : undefined;
        const strength = lg ? weatherStrength(lg) : null;
        if (!lg || strength == null) return cell;
        const label =
          lg.cover_pct === 0
            ? `${cell.label}; no ${lg.level === "mid" ? "middle" : lg.level} cloud forecast`
            : lg.score != null && lg.factors.light > 0
              ? `${cell.label}; glow ${lg.score.toFixed(1)}/10, ${lg.word}`
              : cell.label;
        return { colour: mixHex(sky.colour, cell.colour, strength), label };
      }),
    };
  });
}
