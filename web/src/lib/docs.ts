// The app's own pages: short, plain-language versions of the project docs,
// written for people using the app. They are bundled with the app (so they
// work offline) and each one ends with links to the full technical docs on
// GitHub for anyone who wants to go deeper.

import howItWorks from "../content/how-it-works.md?raw";
import colours from "../content/colours.md?raw";
import places from "../content/places.md?raw";
import packs from "../content/packs.md?raw";
import observations from "../content/observations.md?raw";
import privacy from "../content/privacy.md?raw";

export const REPO = import.meta.env.VITE_SOURCE_URL || "https://github.com/vishalpj14/skyglow";
const BRANCH = "main";

export interface DeeperLink {
  label: string;
  /** Path of the technical doc in the repository. */
  path: string;
}

export interface DocPage {
  slug: string;
  /** Short name for menus. */
  title: string;
  /** Small label above the headline. */
  eyebrow: string;
  /** Headline in three parts; the middle one is drawn in the accent colour. */
  headline: [string, string, string];
  blurb: string;
  /** Accent colour, taken from the sky's own twilight colours. */
  accent: string;
  body: string;
  deeper: DeeperLink[];
}

export const DOC_PAGES: DocPage[] = [
  {
    slug: "how-it-works",
    title: "How it works",
    eyebrow: "The idea",
    headline: ["When the sky ", "lights up", ", and why."],
    blurb: "The whole idea in five minutes, no physics needed.",
    accent: "#f2b45a",
    body: howItWorks,
    deeper: [
      { label: "How it works (full version)", path: "docs/HOW-IT-WORKS.md" },
      { label: "Architecture", path: "ARCHITECTURE.md" },
    ],
  },
  {
    slug: "colours",
    title: "Why skies turn pink",
    eyebrow: "The colours",
    headline: ["Why the sky turns ", "pink", " last."],
    blurb: "Air, light and Earth's shadow, in plain words.",
    accent: "#c8607e",
    body: colours,
    deeper: [
      { label: "The physics of sunset colour", path: "docs/PHYSICS.md" },
      { label: "Earth-shadow geometry (code)", path: "core/src/cloud.rs" },
    ],
  },
  {
    slug: "places",
    title: "Places and mountains",
    eyebrow: "Where you stand",
    headline: ["Your spot, your ", "horizon", "."],
    blurb: "Height, ridges and valleys change what you see.",
    accent: "#e9794a",
    body: places,
    deeper: [
      { label: "Place files and skyline format", path: "docs/PLACES.md" },
      { label: "Example places", path: "examples/README.md" },
    ],
  },
  {
    slug: "packs",
    title: "Packs",
    eyebrow: "The guesses",
    headline: ["Exact maths, ", "honest", " guesses."],
    blurb: "What is certain, what is estimated, and how it improves.",
    accent: "#7a5a9e",
    body: packs,
    deeper: [
      { label: "Pack file format", path: "packs/README.md" },
      { label: "The default pack", path: "packs/default.json" },
      { label: "The Kerala pack", path: "packs/kerala.json" },
    ],
  },
  {
    slug: "observations",
    title: "Help it learn",
    eyebrow: "Your sunsets",
    headline: ["Every sunset ", "teaches", " it."],
    blurb: "Note what you saw, and compare it with the model.",
    accent: "#e0566b",
    body: observations,
    deeper: [
      { label: "Observation files and replay", path: "docs/OBSERVATIONS.md" },
      { label: "How skyglow is tested", path: "docs/TESTING.md" },
    ],
  },
  {
    slug: "privacy",
    title: "Privacy",
    eyebrow: "Your data",
    headline: ["It runs on ", "your device", "."],
    blurb: "What stays with you, and the little that goes online.",
    accent: "#2e4a7d",
    body: privacy,
    deeper: [
      { label: "The web app, in detail", path: "web/README.md" },
      { label: "Design decisions", path: "docs/DECISIONS.md" },
    ],
  },
];

/** A guide's number as shown everywhere (menu, page label, next card): "01" to "06". */
export function guideNumber(slug: string): string {
  const i = DOC_PAGES.findIndex((p) => p.slug === slug);
  return i < 0 ? "" : String(i + 1).padStart(2, "0");
}

export function pageBySlug(slug: string): DocPage | undefined {
  return DOC_PAGES.find((p) => p.slug === slug);
}

/** A file's page on GitHub, e.g. https://github.com/owner/repo/blob/main/docs/PHYSICS.md */
export function blobUrl(path: string, repo = REPO): string {
  return `${repo}/blob/${BRANCH}/${path}`;
}
