import { describe, expect, it } from "vitest";
import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { DOC_PAGES, blobUrl, guideNumber, pageBySlug } from "../src/lib/docs";
import { parseRoute } from "../src/lib/route";
import { accentFor, coverRadius, FADE_MS, GROW_MS, SKY_ACCENT } from "../src/lib/transition";

describe("app pages", () => {
  it("each have text, a three-part headline and a colour", () => {
    const slugs = new Set<string>();
    for (const p of DOC_PAGES) {
      expect(slugs.has(p.slug), `duplicate ${p.slug}`).toBe(false);
      slugs.add(p.slug);
      expect(p.body.length, p.slug).toBeGreaterThan(300);
      expect(p.headline).toHaveLength(3);
      expect(p.accent).toMatch(/^#[0-9a-f]{6}$/i);
    }
    expect(pageBySlug("packs")?.title).toBe("Packs");
    expect(pageBySlug("nope")).toBeUndefined();
  });

  it("link only to technical docs that exist in the repository", () => {
    for (const p of DOC_PAGES) {
      expect(p.deeper.length, p.slug).toBeGreaterThan(0);
      for (const d of p.deeper) {
        expect(existsSync(fileURLToPath(new URL(`../../${d.path}`, import.meta.url))), `${p.slug} -> ${d.path}`).toBe(true);
      }
    }
    expect(blobUrl("docs/PHYSICS.md", "https://github.com/example/skyglow")).toBe("https://github.com/example/skyglow/blob/main/docs/PHYSICS.md");
  });

  it("number the guides 01 to N, first to last", () => {
    expect(guideNumber(DOC_PAGES[0].slug)).toBe("01");
    expect(guideNumber(DOC_PAGES[DOC_PAGES.length - 1].slug)).toBe(String(DOC_PAGES.length).padStart(2, "0"));
    expect(guideNumber("how-it-works")).toBe("01");
    expect(guideNumber("not-a-guide")).toBe("");
  });

  it("route by the address hash", () => {
    expect(parseRoute("#/docs/colours")).toEqual({ name: "doc", slug: "colours" });
    expect(parseRoute("#/")).toEqual({ name: "sky" });
    expect(parseRoute("")).toEqual({ name: "sky" });
    expect(parseRoute("#section")).toEqual({ name: "sky" });
  });
});

describe("page transition", () => {
  it("grows a circle big enough to reach the farthest corner", () => {
    // From the top-left corner of a 300 x 400 screen, the far corner is a 3-4-5 triangle away.
    expect(coverRadius(0, 0, 300, 400)).toBe(500);
    // From the centre it is half the diagonal.
    expect(coverRadius(150, 200, 300, 400)).toBe(250);
    // From near the right edge, the far side is still on the left.
    expect(coverRadius(290, 10, 300, 400)).toBeCloseTo(Math.hypot(290, 390), 9);
  });

  it("takes 1.28 seconds in all (0.94 s plus the 0.34 s asked for)", () => {
    expect(GROW_MS + FADE_MS).toBe(1280);
  });

  it("uses the colour of the page it is going to", () => {
    expect(accentFor("#/")).toBe(SKY_ACCENT);
    expect(accentFor("#/docs/colours")).toBe(pageBySlug("colours")!.accent);
    expect(accentFor("#/docs/unknown")).toBe(SKY_ACCENT);
  });
});
