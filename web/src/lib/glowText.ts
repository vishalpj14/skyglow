// Plain words for a glow score's factors, so the app can always say why a
// layer scored what it did. The numbers come from the core (core/src/glow.rs).

import type { Conditions, Glow, LayerGlow, Level } from "./types";

export interface FactorLine {
  /** What the factor is, in words, with the numbers behind it. */
  text: string;
  /** The factor itself, 0 to 1 (null when not known). */
  value: number | null;
}

export const levelWord = (l: Level) => (l === "mid" ? "middle" : l);

/** The highest cloud overhead, in words (the engine picks it; see core/src/glow.rs). */
export function highestCloudText(g: Glow, ceilingKm: number): string | null {
  if (!g.has_conditions) return null;
  const h = g.highest_cloud;
  if (!h) return "No cloud overhead in the forecast (under 5% at every level).";
  const [lo, hi] = h.range_km;
  return `Highest cloud overhead: ${levelWord(h.level)} cloud (${Math.round(h.cover_pct)}%), somewhere from ${lo.toFixed(1)} to ${hi.toFixed(1)} km above sea level. Forecasts give cover per level, not cloud tops; ${ceilingKm.toFixed(0)} km is the highest a cloud can be.`;
}

/**
 * "Middle cloud here: 6.5 to 11.5 km above sea level (3.0 to 8.0 km above
 * the ground)". Forecasts count their levels from the ground.
 */
export function rangeText(l: LayerGlow): string {
  const ground = l.height_km - l.above_ground_km;
  const [lo, hi] = l.range_km;
  const name = levelWord(l.level);
  const head = `${name[0].toUpperCase()}${name.slice(1)} cloud here: ${lo.toFixed(1)} to ${hi.toFixed(1)} km above sea level`;
  return ground >= 0.5 ? `${head} (${(lo - ground).toFixed(1)} to ${(hi - ground).toFixed(1)} km above the ground).` : `${head}.`;
}

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

/** "low cloud 223 km toward the sun (19%) and middle cloud 615 km (22%)": only crossings with some cloud. */
export function crossingText(l: LayerGlow): string {
  const parts = l.crossings
    .filter((x) => (x.cover_pct ?? 0) > 0)
    .map((x, i) => `${levelWord(x.level)} cloud ${Math.round(x.distance_km)} km${i === 0 ? " toward the sun" : ""} (${Math.round(x.cover_pct ?? 0)}%)`);
  if (!parts.length) return "cloud of unknown cover";
  return parts.length === 1 ? parts[0] : `${parts.slice(0, -1).join(", ")} and ${parts[parts.length - 1]}`;
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
      text: z === 0 ? "Sunbeam skims the ground at the shadow edge: deep colour, fading" : `Sunbeam skims ${z.toFixed(1)} km above sea level on its way: sets how rich the colour is`,
      value: f.colour,
    });
  }
  if (!l.unknown.length || f.cover != null || l.score != null || c) {
    const where = l.cover_at_km > 0 ? ` where it is lit, ${Math.round(l.cover_at_km)} km toward the sun` : " at this height";
    lines.push({
      text: l.cover_pct != null ? `${Math.round(l.cover_pct)}% ${levelWord(l.level)} cloud forecast${where}` : `${levelWord(l.level)} cloud cover not known`,
      value: f.cover,
    });
  }
  if (f.path < 1) lines.push({ text: `Sunbeam crosses ${crossingText(l)} on its way`, value: f.path });
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
