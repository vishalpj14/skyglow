// Colour words in the guides ("gold", "deep red and pink", "Blue hour") reveal
// their colours when you hover them. This finds the colours a phrase names,
// in the order it names them, and marks up bold phrases that have any.

/** Longest names first, so "orange-red" wins over "orange" and "red". */
const HUES: [RegExp, string][] = [
  [/\bsunset glow\b/i, "#e9794a"],
  [/\bafterglow\b/i, "#c8607e"],
  [/\bgolden hour\b/i, "#f2b45a"],
  [/\bpurple light\b/i, "#8a6ac0"],
  [/\bblue hour\b/i, "#4f7fd0"],
  [/\borange-red\b/i, "#e0566b"],
  // "the redder it gets": a deeper red than plain red.
  [/\bredder\b/i, "#a51d2d"],
  [/\bgold(en)?\b/i, "#f2b45a"],
  [/\borange\b/i, "#f08a3c"],
  [/\bred\b/i, "#d9483b"],
  [/\bpink\b/i, "#e87ba4"],
  [/\bmagenta\b/i, "#c2408f"],
  [/\b(purple|violet)\b/i, "#8a6ac0"],
  [/\bblue\b/i, "#4f7fd0"],
  [/\bgr[ae]y\b/i, "#8f8e88"],
];

/** The colours a phrase names, in reading order, without immediate repeats. */
export function coloursIn(text: string): string[] {
  const found: { at: number; end: number; hex: string }[] = [];
  for (const [re, hex] of HUES) {
    const g = new RegExp(re.source, "gi");
    for (let m = g.exec(text); m; m = g.exec(text)) {
      const at = m.index;
      const end = at + m[0].length;
      // Skip words already covered by a longer name ("red" inside "orange-red").
      if (!found.some((f) => at < f.end && end > f.at)) found.push({ at, end, hex });
    }
  }
  return found
    .sort((a, b) => a.at - b.at)
    .map((f) => f.hex)
    .filter((hex, i, all) => i === 0 || all[i - 1] !== hex);
}

/** A left-to-right gradient through the colours (one colour still makes a gradient). */
export function hueGradient(colours: string[]): string {
  const c = colours.length === 1 ? [colours[0], colours[0]] : colours;
  return `linear-gradient(90deg, ${c.join(", ")})`;
}

/** The twilight colours in evening order, as the guides use them for accents. */
export const TWILIGHT = ["#f2b45a", "#e9794a", "#c8607e", "#7a5a9e", "#2e4a7d"];

/**
 * A headline accent's hover sweep: its own colour, the next twilight colour
 * along (the one before, for the last), and back. "Pink" flows into purple.
 */
export function sweepFor(accent: string): string {
  const i = TWILIGHT.indexOf(accent.toLowerCase());
  const next = i < 0 ? TWILIGHT[2] : TWILIGHT[i + 1] ?? TWILIGHT[i - 1];
  return `linear-gradient(90deg, ${accent}, ${next}, ${accent})`;
}

/** Mark up every <strong> phrase in rendered markdown that names a colour. */
export function paintColourWords(html: string): string {
  return html.replace(/<strong>([^<]*)<\/strong>/g, (whole, inner: string) => {
    const colours = coloursIn(inner);
    if (!colours.length) return whole;
    // Lean each colour a fifth of the way toward the page's text colour, so pale
    // gold stays readable on a light page and deep blue on a dark one.
    const readable = colours.map((c) => `color-mix(in srgb, ${c} 80%, var(--text))`);
    return `<strong class="hue" style="--hue-paint: ${hueGradient(readable)}">${inner}</strong>`;
  });
}
