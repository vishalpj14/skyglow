// Plain words for a glow score's factors, so the app can always say why a
// layer scored what it did. The numbers come from the core (core/src/glow.rs).

import type { Conditions, LayerGlow, Level } from "./types";

export interface FactorLine {
  /** What the factor is, in words, with the numbers behind it. */
  text: string;
  /** The factor itself, 0 to 1 (null when not known). */
  value: number | null;
}

export const levelWord = (l: Level) => (l === "mid" ? "middle" : l);

function cover(c: Conditions | null, l: Level): number | undefined {
  if (!c) return undefined;
  return l === "low" ? c.cloud_low_pct : l === "mid" ? c.cloud_mid_pct : c.cloud_high_pct;
}

/** "middle cloud (30%) and high cloud (60%)" */
function clouds(levels: Level[], c: Conditions | null): string {
  return levels
    .map((l) => {
      const pct = cover(c, l);
      return `${levelWord(l)} cloud${pct != null ? ` (${Math.round(pct)}%)` : ""}`;
    })
    .join(" and ");
}

/** One line per factor, in the order they multiply. */
export function explainLayer(l: LayerGlow, c: Conditions | null): FactorLine[] {
  const f = l.factors;
  const z = l.beam_low_km;
  const lines: FactorLine[] = [];

  lines.push({
    text:
      f.light === 0
        ? "In Earth's shadow: no direct sunlight reaches the part you can see"
        : f.light === 1
          ? "Lit overhead by direct sunlight"
          : "Lit only toward the sun: Earth's shadow is overhead, so only a band near the horizon glows",
    value: f.light,
  });
  if (f.sun < 1) lines.push({ text: "Sun still up: the light is only partly sunset-coloured", value: f.sun });
  if (z != null) {
    lines.push({
      text: z === 0 ? "Sunbeam skims the ground at the shadow edge: deep colour, fading" : `Sunbeam skims ${z.toFixed(1)} km above the ground on its way: sets how rich the colour is`,
      value: f.colour,
    });
  }
  if (!l.unknown.length || f.cover != null || l.score != null || c) {
    lines.push({
      text: l.cover_pct != null ? `${Math.round(l.cover_pct)}% ${levelWord(l.level)} cloud forecast at this height` : `${levelWord(l.level)} cloud cover not known`,
      value: f.cover,
    });
  }
  if (f.path < 1) lines.push({ text: `Sunbeam passes through ${clouds(l.blocked_by, c)} on its way`, value: f.path });
  if (f.view < 1) lines.push({ text: `You see it through ${clouds(l.seen_through, c)} below`, value: f.view });
  if (c?.aerosol_optical_depth != null) {
    lines.push({
      text: `Haze: aerosol optical depth ${c.aerosol_optical_depth.toFixed(2)}${z != null && z < 3 ? ", and this beam runs low through it" : ""}`,
      value: f.haze,
    });
  }
  if (c?.humidity_pct != null) lines.push({ text: `Humidity ${Math.round(c.humidity_pct)}% near the ground (damp air adds haze)`, value: f.humidity });
  return lines;
}

/** "×0.82", with "×1" for no effect. */
export function times(v: number | null): string {
  if (v == null) return "?";
  return v === 1 ? "×1" : `×${v.toFixed(2)}`;
}
