<script setup lang="ts">
// The top bar: logo, connection status (tap for the online-extras switch),
// and a menu button that opens a full-screen list of every page.
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { onlineExtras } from "../lib/saved";
import { isOnline } from "../lib/network";
import { route } from "../lib/route";
import { DOC_PAGES, REPO, guideNumber } from "../lib/docs";
import { SKY_ACCENT } from "../lib/transition";
import LogoMark from "./LogoMark.vue";

const logo = ref<InstanceType<typeof LogoMark>>();
const statusOpen = ref(false);
const menuOpen = ref(false);
const statusBox = ref<HTMLElement>();
const menuFirst = ref<HTMLAnchorElement[]>([]);

const items = computed(() => [
  { href: "#/", num: "", title: "Sky", blurb: "Plan tonight's sunrise and sunset, minute by minute.", accent: SKY_ACCENT, here: route.value.name === "sky" },
  ...DOC_PAGES.map((p) => ({
    href: `#/docs/${p.slug}`,
    num: guideNumber(p.slug),
    title: p.title,
    blurb: p.blurb,
    accent: p.accent,
    here: route.value.name === "doc" && route.value.slug === p.slug,
  })),
]);

function onDocClick(e: MouseEvent) {
  if (statusOpen.value && statusBox.value && !statusBox.value.contains(e.target as Node)) statusOpen.value = false;
}
function onKey(e: KeyboardEvent) {
  if (e.key === "Escape") {
    statusOpen.value = false;
    menuOpen.value = false;
  }
}
onMounted(() => {
  document.addEventListener("click", onDocClick);
  document.addEventListener("keydown", onKey);
});
onBeforeUnmount(() => {
  document.removeEventListener("click", onDocClick);
  document.removeEventListener("keydown", onKey);
  document.documentElement.style.overflow = "";
});

// Close the menu once the next page is showing; lock page scroll while open.
watch(route, () => (menuOpen.value = false));
watch(menuOpen, async (open) => {
  document.documentElement.style.overflow = open ? "hidden" : "";
  if (open) {
    await nextTick();
    menuFirst.value[0]?.focus();
  }
});
</script>

<template>
  <header class="site-header" :class="{ 'menu-open': menuOpen }">
    <div class="bar">
      <a
        class="brand"
        href="#/"
        aria-label="skyglow, back to the sky"
        @mouseenter="logo?.play()"
        @mouseleave="logo?.rest()"
        @focus="logo?.play()"
        @blur="logo?.rest()"
      >
        <LogoMark ref="logo" :size="32" />
        <span class="words">
          <span class="name">skyglow</span>
          <span class="tag">sunset and twilight light, predicted</span>
        </span>
      </a>

      <div ref="statusBox" class="status">
        <button type="button" class="pill" :class="isOnline ? 'on' : 'off'" :aria-expanded="statusOpen" aria-haspopup="dialog" @click="statusOpen = !statusOpen">
          <span class="dot" aria-hidden="true"></span>{{ isOnline ? "Online" : "Offline" }}
        </button>
        <div v-if="statusOpen" class="pop card" role="dialog" aria-label="Connection and online extras">
          <p class="state">
            <strong>{{ isOnline ? "You're online." : "You're offline." }}</strong>
            {{ isOnline ? "Search and the map work worldwide." : "Everything else keeps working on this device." }}
          </p>
          <label class="switch">
            <input v-model="onlineExtras" type="checkbox" />
            <span>
              <strong>Online extras</strong><br />
              Fill in the time zone, ground height and name of places you pick, using only their rough coordinates.
            </span>
          </label>
          <a href="#/docs/privacy" @click="statusOpen = false">What goes online, and when</a>
        </div>
      </div>

      <button type="button" class="menu-btn" :aria-expanded="menuOpen" aria-controls="site-menu" :aria-label="menuOpen ? 'Close menu' : 'Open menu'" @click="menuOpen = !menuOpen">
        <span v-if="!menuOpen" class="dots" aria-hidden="true"><i style="background:#2e4a7d"></i><i style="background:#c8607e"></i><i style="background:#e9794a"></i><i style="background:#f2b45a"></i></span>
        <span v-else aria-hidden="true" class="x">&times;</span>
      </button>
    </div>
    <div class="glow" aria-hidden="true"></div>
  </header>

  <Transition name="menu">
    <nav v-if="menuOpen" id="site-menu" class="menu" aria-label="All pages">
      <div class="menu-inner">
        <p class="caps">skyglow</p>
        <ol>
          <li v-for="(it, i) in items" :key="it.href" :style="{ '--accent-item': it.accent, '--i': i }">
            <span v-if="it.num" class="num">{{ it.num }}</span>
            <span v-else class="num sun" aria-hidden="true"></span>
            <a ref="menuFirst" :href="it.href" :aria-current="it.here ? 'page' : undefined">
              <span class="title">{{ it.title }}</span>
              <span class="sub"><span v-if="it.here" class="here">You're here</span>{{ it.blurb }}</span>
            </a>
          </li>
        </ol>
        <p class="menu-foot">
          <a :href="REPO" target="_blank" rel="noopener noreferrer">GitHub &nearr;</a>
          <span>Made by Vishal PJ</span>
        </p>
      </div>
    </nav>
  </Transition>
</template>

<style scoped>
.site-header {
  position: sticky; top: 0; z-index: 30;
  background: color-mix(in srgb, var(--bg) 85%, transparent);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
}
.site-header.menu-open { background: transparent; backdrop-filter: none; }
.bar { max-width: 1280px; margin: 0 auto; padding: 10px 16px; display: flex; align-items: center; gap: 10px; }
.brand { display: flex; align-items: center; gap: 10px; text-decoration: none; color: var(--text); margin-right: auto; }
.menu-open .brand { color: #f0efec; }
.brand :deep(.mark) { border-radius: 9px; }
.words { display: grid; line-height: 1.15; }
.name { font-weight: 800; font-size: 1.2rem; letter-spacing: -0.02em; }
.tag { font-size: 0.74rem; color: var(--muted); }
@media (max-width: 420px) { .tag { display: none; } }

.status { position: relative; }
.pill {
  display: inline-flex; align-items: center; gap: 7px; padding: 6px 14px; border-radius: 999px;
  border: 1px solid var(--line); background: var(--surface); color: var(--text-2); font-size: 0.85rem;
}
.dot { width: 8px; height: 8px; border-radius: 50%; background: var(--muted); }
.pill.on .dot { background: #2f9e44; box-shadow: 0 0 0 3px color-mix(in srgb, #2f9e44 25%, transparent); }
.pop {
  position: absolute; right: 0; top: calc(100% + 8px); width: min(320px, calc(100vw - 32px));
  box-shadow: 0 10px 30px rgb(0 0 0 / 0.2); font-size: 0.86rem; display: grid; gap: 10px;
}
.pop p { margin: 0; color: var(--text-2); }
.pop .state strong { color: var(--text); }
.switch { display: flex; gap: 10px; align-items: flex-start; color: var(--text-2); font-size: 0.86rem; }
.switch input { margin-top: 3px; width: 18px; height: 18px; flex: 0 0 auto; }
.switch strong { color: var(--text); }

.menu-btn {
  width: 44px; height: 44px; border-radius: 50%; padding: 0; display: grid; place-items: center;
  background: var(--surface-2); border: 1px solid var(--line);
}
.menu-open .menu-btn { background: #23262e; border-color: #343844; color: #f0efec; }
.dots { display: grid; grid-template-columns: 7px 7px; gap: 4px; }
.dots i { width: 7px; height: 7px; border-radius: 50%; display: block; }
.x { font-size: 1.6rem; line-height: 1; }
.glow { height: 3px; background: linear-gradient(90deg, #2e4a7d, #7a5a9e, #c8607e, #e9794a, #f2b45a); opacity: 0.85; }
.menu-open .glow { opacity: 0; }

/* ---- full-screen menu (always dark, like the sky after sunset) ---- */
.menu {
  position: fixed; inset: 0; z-index: 25; overflow-y: auto;
  background:
    radial-gradient(60% 50% at 85% 0%, rgb(46 74 125 / 0.45), transparent 70%),
    radial-gradient(50% 40% at 0% 100%, rgb(200 96 126 / 0.25), transparent 70%),
    #0d1018;
  color: #f0efec;
}
.menu-inner { max-width: 1100px; margin: 0 auto; padding: 96px 20px 32px; }
.caps { font-size: 0.75rem; letter-spacing: 0.16em; text-transform: uppercase; color: #8f8e88; font-weight: 600; margin: 0 0 8px; }
ol { list-style: none; margin: 0; padding: 0; }
li {
  display: grid; grid-template-columns: 44px 1fr; align-items: start; border-top: 1px solid #262a35; padding: 10px 0 14px;
  animation: item-in 520ms cubic-bezier(0.2, 0.7, 0.2, 1) both; animation-delay: calc(var(--i) * 45ms + 60ms);
}
.num { color: var(--accent-item); font-weight: 700; font-size: 0.85rem; padding-top: 14px; font-variant-numeric: tabular-nums; }
/* The sky page is the app itself, not a numbered guide: it gets a small sun. */
.num.sun { width: 14px; height: 14px; margin-top: 18px; padding: 0; border-radius: 50%; background: var(--accent-item); box-shadow: 0 0 12px var(--accent-item); }
li a { display: grid; gap: 4px; text-decoration: none; color: inherit; }
.title { font-size: clamp(2rem, 7vw, 3.6rem); font-weight: 800; letter-spacing: -0.035em; line-height: 1.05; transition: color 0.2s, transform 0.2s; }
li a:hover .title, li a:focus-visible .title { color: var(--accent-item); transform: translateX(6px); }
li a[aria-current="page"] .title { text-decoration: underline; text-decoration-color: var(--accent-item); text-decoration-thickness: 4px; text-underline-offset: 6px; }
.sub { color: #a9a8a1; font-size: 0.95rem; display: flex; flex-wrap: wrap; gap: 8px; align-items: center; }
.here { background: var(--accent-item); color: #0d1018; font-size: 0.7rem; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; padding: 2px 9px; border-radius: 999px; }
.menu-foot { display: flex; justify-content: space-between; gap: 12px; margin-top: 28px; padding-top: 16px; border-top: 1px solid #262a35; color: #8f8e88; font-size: 0.85rem; }
.menu-foot a { color: #f0efec; }
@keyframes item-in { from { opacity: 0; transform: translateY(16px); } to { opacity: 1; transform: none; } }
.menu-enter-active, .menu-leave-active { transition: opacity 0.25s ease, clip-path 0.45s cubic-bezier(0.7, 0, 0.25, 1); }
.menu-enter-from, .menu-leave-to { opacity: 0; clip-path: circle(0% at calc(100% - 38px) 32px); }
.menu-enter-to, .menu-leave-from { clip-path: circle(150% at calc(100% - 38px) 32px); }
@media (prefers-reduced-motion: reduce) {
  li { animation: none; }
  .menu-enter-active, .menu-leave-active { transition: opacity 0.15s; }
  .menu-enter-from, .menu-leave-to { clip-path: none; }
}
</style>
