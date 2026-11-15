// What the logo's hover animation should play: a sunset or a sunrise, in the
// current pack's stage colours. The app keeps it in step with the time
// scrubber, so it follows the place and time you are looking at.
import { ref } from "vue";

export const NIGHT = "#141a2e";
export const DAY = "#8fb8e8";

export interface SkyMood {
  /** True when the scrubber's moment is before solar noon (sun climbing). */
  rising: boolean;
  /** Stage colours in evening order (golden hour first), from the pack. */
  swatches: string[];
}

export const skyMood = ref<SkyMood>({
  rising: false,
  swatches: ["#f2b45a", "#e9794a", "#c8607e", "#7a5a9e", "#2e4a7d"],
});

/**
 * The colours the logo's sky passes through: day, each stage, then night for
 * a sunset; the same in reverse for a sunrise.
 */
export function moodFrames(m: SkyMood): string[] {
  const evening = [DAY, ...m.swatches.filter(Boolean), NIGHT];
  return m.rising ? evening.reverse() : evening;
}

// ---- colour arithmetic (small, so the logo needs no library) -----------------

export function hexToRgb(hex: string): [number, number, number] {
  const n = parseInt(hex.replace("#", ""), 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

export function rgbToHex([r, g, b]: [number, number, number]): string {
  return "#" + [r, g, b].map((v) => Math.round(Math.min(255, Math.max(0, v))).toString(16).padStart(2, "0")).join("");
}

/** Blend two colours: t = 0 gives `a`, t = 1 gives `b`. */
export function mix(a: string, b: string, t: number): string {
  const A = hexToRgb(a);
  const B = hexToRgb(b);
  return rgbToHex([A[0] + (B[0] - A[0]) * t, A[1] + (B[1] - A[1]) * t, A[2] + (B[2] - A[2]) * t]);
}

/** The colour at position t (0..1) along a list of colours, blending between neighbours. */
export function along(colours: string[], t: number): string {
  if (colours.length === 1) return colours[0];
  const x = Math.min(1, Math.max(0, t)) * (colours.length - 1);
  const i = Math.min(colours.length - 2, Math.floor(x));
  return mix(colours[i], colours[i + 1], x - i);
}

// ---- one frame of the logo animation -------------------------------------------

export interface LogoLook {
  top: string;
  mid: string;
  bottom: string;
  sun: string;
  /** Sun centre height in the 64-unit icon; the horizon is at 46, so 62 is hidden. */
  sunY: number;
  cloud: string;
}

/** The icon's resting look (the app icon). */
export const LOGO_REST: LogoLook = { top: "#2e4a7d", mid: "#c8607e", bottom: "#f2b45a", sun: "#f6c35b", sunY: 46, cloud: "#f08a3c" };

/**
 * The logo at progress p (0..1) of its animation. A sunset takes the sun from
 * high in a day sky (y 16) to hidden below the horizon (y 62) in a night sky;
 * a sunrise does the reverse. The little high cloud keeps its light a bit
 * longer at dusk and catches it a bit earlier at dawn, as real high clouds do.
 */
export function logoFrame(m: SkyMood, p: number): LogoLook {
  const frames = moodFrames(m);
  const c = along(frames, p);
  const height = m.rising ? p : 1 - p; // 1 = high in the sky
  return {
    top: mix(c, NIGHT, 0.6),
    mid: mix(c, NIGHT, 0.22),
    bottom: c,
    sun: mix("#e0566b", "#f6c35b", height),
    sunY: 62 - 46 * height,
    cloud: along(frames, m.rising ? p + 0.14 : p - 0.14),
  };
}

export function blendLook(a: LogoLook, b: LogoLook, t: number): LogoLook {
  return {
    top: mix(a.top, b.top, t),
    mid: mix(a.mid, b.mid, t),
    bottom: mix(a.bottom, b.bottom, t),
    sun: mix(a.sun, b.sun, t),
    sunY: a.sunY + (b.sunY - a.sunY) * t,
    cloud: mix(a.cloud, b.cloud, t),
  };
}

// ---- the wordmark ("skyglow" text) ------------------------------------------------

/** How far each wordmark colour leans toward the normal text colour, so it stays readable. */
export const WORDMARK_LEAN = 0.4;

/**
 * Colours for painting the word "skyglow" in step with the logo: the sun's
 * colour first, then the horizon, then up into the sky. Each is blended
 * toward the page's normal text colour (`base`) so the word never fades
 * into the background, even when the logo's sky has gone to night.
 */
export function wordmarkColours(look: LogoLook, base: string, lean = WORDMARK_LEAN): string[] {
  return [look.sun, look.bottom, look.mid, look.top].map((c) => mix(c, base, lean));
}

/** "rgb(240, 239, 236)" or "#f0efec" to "#f0efec"; anything else falls back. */
export function cssColourToHex(css: string, fallback = "#f0efec"): string {
  if (/^#[0-9a-f]{6}$/i.test(css)) return css.toLowerCase();
  const m = /rgba?\(\s*(\d+)[,\s]+(\d+)[,\s]+(\d+)/i.exec(css);
  return m ? rgbToHex([Number(m[1]), Number(m[2]), Number(m[3])]) : fallback;
}
