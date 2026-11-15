// Turns a SkyState into plain sentences, the same wording as the command
// line's `--at` report, so the app and the CLI read alike.

import type { LayerState, SkyState } from "./types";

const deg = "°";

export function describeSun(s: SkyState): string[] {
  if (s.elevation_deg >= 0) {
    const lines = [`Sun is above the horizon: ${s.elevation_deg.toFixed(2)}${deg} elevation, azimuth ${s.azimuth_deg.toFixed(0)}${deg}.`];
    if (s.skyline) {
      lines.push(
        `Skyline in that direction: ${s.skyline.horizon_elevation_deg.toFixed(1)}${deg}, so the sun is ` +
          (s.skyline.sun_hidden ? "hidden behind terrain." : "visible."),
      );
    }
    return lines;
  }
  const lines = [
    `Sun is ${s.depression_deg.toFixed(2)}${deg} below the horizon (${s.rising ? "rising" : "setting"}), azimuth ${s.azimuth_deg.toFixed(0)}${deg}.`,
  ];
  // Past 20 degrees the threshold is hundreds of km: not worth saying (same rule as the CLI).
  if (s.overhead_lit_above_km != null && s.depression_deg < 20) {
    lines.push(`A cloud directly overhead is lit only if it is higher than ${s.overhead_lit_above_km.toFixed(1)} km.`);
  }
  return lines;
}

export function describeLayer(l: LayerState): string {
  const L = l.light;
  switch (L.kind) {
    case "lit_overhead":
      return `Lit overhead; sunlight skims ${L.grazing_km.toFixed(1)} km above the ground${L.colour ? `, likely ${L.colour}` : ""}.`;
    case "lit_toward_sun":
      return `Lit only below ${L.edge_elevation_deg.toFixed(1)}${deg} elevation toward the sun (shadow edge ${L.edge_distance_km.toFixed(0)} km away; edge light is deep red-pink).`;
    case "edge_beyond_horizon":
      return "Shadow edge is past your horizon: none of this layer is lit.";
    case "shadow_overhead":
      return "In Earth's shadow overhead.";
  }
}

/** Is any part of the layer you can see in direct sunlight? */
export function layerLit(l: LayerState): boolean {
  return l.light.kind === "lit_overhead" || l.light.kind === "lit_toward_sun";
}
