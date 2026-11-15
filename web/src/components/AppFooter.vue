<script setup lang="ts">
// The site footer: who made it, where the code and docs are, whose data the
// online features use, and whether the app is online right now.
import { onlineExtras } from "../lib/saved";
import { isOnline } from "../lib/network";

import { DOC_PAGES, REPO as repo, blobUrl as doc } from "../lib/docs";
import { ref } from "vue";
import LogoMark from "./LogoMark.vue";
const version = __APP_VERSION__;
const year = new Date().getFullYear();
const logo = ref<InstanceType<typeof LogoMark>>();
</script>

<template>
  <footer class="site-footer">
    <div class="inner">
      <div class="top">
        <a class="brand" href="#/" @mouseenter="logo?.play()" @mouseleave="logo?.rest()" @focus="logo?.play()" @blur="logo?.rest()"><LogoMark ref="logo" :size="22" />skyglow</a>
        <nav aria-label="Pages">
          <a v-for="p in DOC_PAGES" :key="p.slug" :href="`#/docs/${p.slug}`" :title="p.blurb">{{ p.title }}</a>
          <a :href="repo" target="_blank" rel="noopener noreferrer">GitHub</a>
          <a :href="`${repo}/issues`" target="_blank" rel="noopener noreferrer">Report a problem</a>
        </nav>
        <span
          class="pill"
          :class="isOnline ? 'on' : 'off'"
          role="status"
          :title="isOnline ? (onlineExtras ? 'Online extras on' : 'Online extras off') : 'Everything still works on this device'"
        >
          <span class="dot" aria-hidden="true"></span>{{ isOnline ? "Online" : "Offline" }}
        </span>
      </div>
      <div class="bottom">
        <span>Made by <strong>Vishal PJ</strong> &middot; &copy; {{ year }} &middot; <a :href="doc('LICENSE')" target="_blank" rel="noopener noreferrer">MIT</a> &middot; v{{ version }}</span>
        <span>
          Data &copy; <a href="https://www.openstreetmap.org/copyright" rel="noopener">OpenStreetMap</a> &middot;
          <a href="https://photon.komoot.io/" rel="noopener">Photon</a> &middot;
          <a href="https://open-meteo.com/" rel="noopener">Open-Meteo</a>
        </span>
      </div>
    </div>
  </footer>
</template>

<style scoped>
.site-footer { margin-top: 32px; border-top: 1px solid var(--line); color: var(--muted); font-size: 0.85rem; }
.inner { max-width: 1280px; margin: 0 auto; padding: 18px 16px 22px; display: grid; gap: 12px; }
.top, .bottom { display: flex; flex-wrap: wrap; align-items: center; gap: 10px 20px; }
.top { justify-content: space-between; }
.bottom { justify-content: space-between; font-size: 0.8rem; }
.brand { display: inline-flex; align-items: center; gap: 8px; font-weight: 700; color: var(--text); text-decoration: none; }
/* Phones: logo and status share the first line, links get their own line below. */
nav { display: flex; flex-wrap: wrap; gap: 6px 18px; order: 3; flex-basis: 100%; }
@media (min-width: 700px) {
  nav { order: 0; flex: 1 1 auto; flex-basis: auto; }
}
a { color: var(--text-2); text-decoration: none; }
a:hover { color: var(--text); text-decoration: underline; text-underline-offset: 3px; }
strong { color: var(--text-2); font-weight: 600; }
.pill {
  display: inline-flex; align-items: center; gap: 6px; padding: 2px 10px;
  border: 1px solid var(--line); border-radius: 999px; color: var(--text-2); font-size: 0.8rem;
}
.dot { width: 7px; height: 7px; border-radius: 50%; background: var(--muted); }
.pill.on .dot { background: #2f9e44; }
</style>
