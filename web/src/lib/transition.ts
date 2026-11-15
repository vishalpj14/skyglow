// Page-to-page "sunrise" transition: a circle in the next page's colour grows
// from the point you tapped until it covers the screen, the page changes
// underneath, then the circle fades away as the new page settles in.
//
// Any link to "#/..." anywhere in the app goes through here. With "reduce
// motion" switched on in the device settings, pages simply change.

import { ref } from "vue";
import { parseRoute, route } from "./route";
import { pageBySlug } from "./docs";

/** Accent of the sky page (a low golden sun). */
export const SKY_ACCENT = "#f2b45a";

export interface Iris {
  x: number;
  y: number;
  /** Radius that reaches the farthest screen corner from (x, y). */
  r: number;
  colour: string;
  phase: "grow" | "fade";
}

export const iris = ref<Iris | null>(null);
let busy = false;

// Whole transition: 1.28 s. The circle animation (IrisOverlay.vue) uses
// these same numbers, so the page always swaps while the screen is covered.
export const GROW_MS = 710;
export const FADE_MS = 570;

export function accentFor(hash: string): string {
  const r = parseRoute(hash);
  return r.name === "doc" ? pageBySlug(r.slug)?.accent ?? SKY_ACCENT : SKY_ACCENT;
}

/** Distance from (x, y) to the farthest corner of a w by h screen. */
export function coverRadius(x: number, y: number, w: number, h: number): number {
  return Math.hypot(Math.max(x, w - x), Math.max(y, h - y));
}

function reducedMotion(): boolean {
  return typeof matchMedia !== "undefined" && matchMedia("(prefers-reduced-motion: reduce)").matches;
}

const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));

/** Go to an app page ("#/docs/colours", "#/"), growing the circle from (x, y). */
export async function go(hash: string, x = innerWidth / 2, y = innerHeight / 2): Promise<void> {
  if (hash === "#" || hash === "") hash = "#/";
  if (busy || hash === (location.hash || "#/")) return;
  if (reducedMotion()) {
    location.hash = hash;
    return;
  }
  busy = true;
  try {
    iris.value = { x, y, r: coverRadius(x, y, innerWidth, innerHeight), colour: accentFor(hash), phase: "grow" };
    await wait(GROW_MS);
    // Swap the page while it is covered.
    history.pushState(null, "", hash);
    route.value = parseRoute(hash);
    window.scrollTo({ top: 0 });
    iris.value = { ...iris.value, phase: "fade" };
    await wait(FADE_MS);
  } finally {
    iris.value = null;
    busy = false;
  }
}

/** Send every in-app link click through the transition. Call once at start-up. */
export function interceptLinks(): void {
  document.addEventListener("click", (ev) => {
    if (ev.defaultPrevented || ev.button !== 0 || ev.metaKey || ev.ctrlKey || ev.shiftKey || ev.altKey) return;
    const a = (ev.target as Element).closest?.("a[href^='#/'], a[href='#']");
    if (!a || a.getAttribute("target")) return;
    ev.preventDefault();
    const r = a.getBoundingClientRect();
    // Keyboard "clicks" have no pointer position: start from the link's centre.
    const fromKeyboard = ev.detail === 0;
    go(a.getAttribute("href")!, fromKeyboard ? r.left + r.width / 2 : ev.clientX, fromKeyboard ? r.top + r.height / 2 : ev.clientY);
  });
}
