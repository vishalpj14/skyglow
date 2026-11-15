<script setup lang="ts">
// One of the app's plain-language pages: a big hero, the text, then links
// to the technical docs for anyone who wants to go deeper.
import { computed, watch } from "vue";
import { marked } from "marked";
import { DOC_PAGES, blobUrl, guideNumber, pageBySlug } from "../lib/docs";

const props = defineProps<{ slug: string }>();

const index = computed(() => DOC_PAGES.findIndex((p) => p.slug === props.slug));
const page = computed(() => pageBySlug(props.slug));
const next = computed(() => DOC_PAGES[(index.value + 1) % DOC_PAGES.length]);
// The text is our own bundled markdown, so no sanitising step is needed.
const html = computed(() => (page.value ? (marked.parse(page.value.body, { async: false, gfm: true }) as string) : ""));

watch(
  page,
  (p) => {
    document.title = p ? `${p.title} · skyglow` : "skyglow";
  },
  { immediate: true },
);
</script>

<template>
  <div v-if="page" :key="page.slug" class="doc" :style="{ '--accent-page': page.accent }">
    <section class="hero">
      <p class="eyebrow">
        <span class="chip"><span class="dot"></span>{{ page.eyebrow }}</span>
        <span class="caps">Guide {{ guideNumber(page.slug) }} / {{ guideNumber(DOC_PAGES[DOC_PAGES.length - 1].slug) }}</span>
      </p>
      <h1>
        <span class="rise r1">{{ page.headline[0] }}</span><span class="rise r2 accent">{{ page.headline[1] }}</span><span class="rise r3">{{ page.headline[2] }}</span>
      </h1>
      <p class="blurb rise r4">{{ page.blurb }}</p>
    </section>

    <article class="prose rise r5" v-html="html"></article>

    <section class="deeper" aria-labelledby="deeper-title">
      <h2 id="deeper-title" class="caps">Go deeper</h2>
      <p class="muted">The full technical write-ups, formulas and code live on GitHub.</p>
      <div class="cards">
        <a v-for="d in page.deeper" :key="d.path" class="link-card" :href="blobUrl(d.path)" target="_blank" rel="noopener noreferrer">
          <span>{{ d.label }}</span>
          <small>{{ d.path }}</small>
          <span class="arrow" aria-hidden="true">&nearr;</span>
        </a>
      </div>
    </section>

    <nav class="next" aria-label="Next page">
      <a :href="`#/docs/${next.slug}`" :style="{ '--accent-next': next.accent }">
        <span class="caps">Next guide &middot; {{ guideNumber(next.slug) }}</span>
        <span class="next-title">{{ next.title }} <span aria-hidden="true">&rarr;</span></span>
        <span class="muted">{{ next.blurb }}</span>
      </a>
      <a href="#/" class="back">&larr; Back to the sky</a>
    </nav>
  </div>
  <div v-else class="doc">
    <h1>Page not found</h1>
    <p><a href="#/">Back to the sky</a></p>
  </div>
</template>

<style scoped>
.doc { max-width: 880px; margin: 0 auto; padding: 24px 16px 40px; }

/* ---- hero ---- */
.hero { padding: clamp(16px, 6vw, 56px) 0 clamp(20px, 4vw, 36px); }
.eyebrow { display: flex; flex-wrap: wrap; align-items: center; gap: 10px 14px; margin: 0 0 18px; }
.chip {
  display: inline-flex; align-items: center; gap: 8px; padding: 5px 14px; border-radius: 999px;
  background: var(--surface); border: 1px solid var(--line); font-size: 0.85rem; color: var(--text);
  box-shadow: 0 2px 10px rgb(0 0 0 / 0.08);
}
.dot { width: 8px; height: 8px; border-radius: 50%; background: var(--accent-page); }
.caps { font-size: 0.75rem; letter-spacing: 0.14em; text-transform: uppercase; color: var(--muted); font-weight: 600; }
h1 {
  font-size: clamp(2.4rem, 9vw, 5.2rem); line-height: 1.02; letter-spacing: -0.035em; font-weight: 800;
  margin: 0 0 18px; color: var(--text);
}
h1 .accent { color: var(--accent-page); }
.blurb { font-size: clamp(1.05rem, 2.6vw, 1.3rem); color: var(--text-2); max-width: 40ch; margin: 0; }

/* Words rise in one after another as the page arrives. */
.rise { display: inline-block; animation: rise 700ms cubic-bezier(0.2, 0.7, 0.2, 1) both; }
h1 .rise { display: inline; }
.r1 { animation-delay: 120ms; } .r2 { animation-delay: 200ms; } .r3 { animation-delay: 260ms; }
.r4 { animation-delay: 340ms; } .r5 { display: block; animation-delay: 420ms; }
@keyframes rise { from { opacity: 0; transform: translateY(18px); } to { opacity: 1; transform: none; } }
@media (prefers-reduced-motion: reduce) { .rise { animation: none; } }

/* ---- body text ---- */
.prose { font-size: 1.06rem; line-height: 1.7; color: var(--text); max-width: 68ch; }
.prose :deep(h2) {
  font-size: clamp(1.35rem, 3.5vw, 1.75rem); letter-spacing: -0.02em; margin: 2.2em 0 0.5em; line-height: 1.2;
}
.prose :deep(h2)::before {
  content: ""; display: block; width: 36px; height: 4px; border-radius: 4px; background: var(--accent-page); margin-bottom: 14px;
}
.prose :deep(p), .prose :deep(ul) { margin: 0 0 1em; color: var(--text-2); }
.prose :deep(strong) { color: var(--text); }
.prose :deep(li) { margin: 0.35em 0; }
.prose :deep(li::marker) { color: var(--accent-page); }
.prose :deep(table) { margin: 0 0 1.2em; }
.prose :deep(th), .prose :deep(td) { padding: 8px 10px; }

/* ---- go deeper ---- */
.deeper { margin-top: 48px; padding-top: 28px; border-top: 1px solid var(--line); }
.deeper h2 { margin: 0 0 6px; }
.cards { display: grid; gap: 10px; grid-template-columns: repeat(auto-fit, minmax(230px, 1fr)); margin-top: 14px; }
.link-card {
  position: relative; display: grid; gap: 2px; padding: 14px 40px 14px 16px; border-radius: 14px;
  background: var(--surface); border: 1px solid var(--line); color: var(--text); text-decoration: none;
  transition: transform 0.15s ease, border-color 0.15s ease;
}
.link-card:hover { transform: translateY(-2px); border-color: var(--accent-page); }
.link-card small { color: var(--muted); font-family: ui-monospace, Consolas, monospace; font-size: 0.75rem; }
.arrow { position: absolute; right: 14px; top: 14px; color: var(--accent-page); }

/* ---- next page ---- */
.next { margin-top: 48px; display: grid; gap: 14px; }
.next a:first-child {
  display: grid; gap: 4px; padding: 22px 22px 20px; border-radius: 18px; text-decoration: none; color: var(--text);
  background: linear-gradient(120deg, color-mix(in srgb, var(--accent-next) 22%, var(--surface)), var(--surface) 70%);
  border: 1px solid var(--line);
}
.next-title { font-size: clamp(1.6rem, 5vw, 2.4rem); font-weight: 800; letter-spacing: -0.03em; }
.back { color: var(--accent); justify-self: start; }
</style>
