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

/** A layer whose forecast cover stays below this (percent) the whole strip gets no row. */
export const MIN_SHOWN_COVER_PCT = 5;

/**
 * Which layers to draw. Without a forecast, all of them (each row means "if a
 * cloud were there"). With one, only layers the forecast gives at least
 * MIN_SHOWN_COVER_PCT cover, or does not know about, at some moment in the
 * strip: a row that is (nearly) clear the whole time would only repeat the
 * sky. Blending between forecast hours gives tiny covers such as 0.3%, so the
 * cut is not at exactly 0.
 */
export function layersToShow(layerCount: number, glow?: Glow[]): number[] {
  const all = [...Array(layerCount).keys()];
  const known = glow?.filter((g) => g.has_conditions) ?? [];
  if (!known.length) return all;
  return all.filter((i) =>
    known.some((g) => {
      const c = g.layers[i]?.cover_pct;
      return c == null || c >= MIN_SHOWN_COVER_PCT;
    }),
  );
}

/** What one layer does over the whole strip, for its row's hover card. */
export interface LayerSummary {
  name: string;
  /** Above sea level and above the ground, km. */
  height_km: number;
  above_ground_km: number;
  /** From the forecast, when there is one. */
  level: LayerGlow["level"] | null;
  range_km: [number, number] | null;
  /** Lowest and highest forecast cover in the strip, percent. */
  cover_pct: [number, number] | null;
  /** Likely cloud type at the most cover. */
  cloud_type: string | null;
  /** The best score in the strip and when. */
  best: { jd: number; score: number; word: string } | null;
  /** When the layer is lit at all, overhead or toward the sun. */
  lit: [number, number] | null;
}

export function layerSummary(series: SkyState[], i: number, glow?: Glow[]): LayerSummary | null {
  const first = series[0]?.layers[i];
  if (!first) return null;
  const out: LayerSummary = {
    name: first.name,
    height_km: first.height_km,
    above_ground_km: first.above_ground_km,
    level: null,
    range_km: null,
    cover_pct: null,
    cloud_type: null,
    best: null,
    lit: null,
  };
  let mostCover = -1;
  series.forEach((s, k) => {
    const L = s.layers[i].light;
    if (L.kind === "lit_overhead" || L.kind === "lit_toward_sun") {
      out.lit = out.lit ? [out.lit[0], s.jd] : [s.jd, s.jd];
    }
    const g = glow?.[k];
    const lg = g?.has_conditions ? g.layers[i] : undefined;
    if (!lg) return;
    out.level = lg.level;
    out.range_km = lg.range_km;
    if (lg.cover_pct != null) {
      const c = lg.cover_pct;
      out.cover_pct = out.cover_pct ? [Math.min(out.cover_pct[0], c), Math.max(out.cover_pct[1], c)] : [c, c];
      if (c > mostCover) {
        mostCover = c;
        out.cloud_type = lg.cloud_type;
      }
    }
    if (lg.score != null && (!out.best || lg.score > out.best.score)) {
      out.best = { jd: s.jd, score: lg.score, word: lg.word ?? "" };
    }
  });
  return out;
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
        const cover =
          lg.cover_pct != null && lg.cover_pct > 0
            ? `; ${Math.round(lg.cover_pct)}% cover${lg.cloud_type ? `, likely ${lg.cloud_type}` : ""}`
            : "";
        const label =
          lg.cover_pct === 0
            ? `${cell.label}; no ${lg.level === "mid" ? "middle" : lg.level} cloud forecast`
            : lg.score != null && lg.factors.light > 0
              ? `${cell.label}; glow ${lg.score.toFixed(1)}/10, ${lg.word}${cover}`
              : `${cell.label}${cover}`;
        return { colour: mixHex(sky.colour, cell.colour, strength), label };
      }),
    };
  });
}
