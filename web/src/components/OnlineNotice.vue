<script setup lang="ts">
// Small popup in the bottom-left corner:
// - the first time, says that online extras are on and what they send, with
//   a one-tap way to turn them off;
// - when the connection drops or comes back, a short note that fades away.
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { onlineExtras, onlineNoticeSeen } from "../lib/saved";
import { isOnline } from "../lib/network";

const firstRun = computed(() => onlineExtras.value && !onlineNoticeSeen.value && isOnline.value);

const flash = ref<{ title: string; text: string; tone: "on" | "off" } | null>(null);
let timer: number | undefined;
watch(isOnline, (up) => {
  window.clearTimeout(timer);
  flash.value = up
    ? { title: "Back online", text: "Search, the map and online extras are working again.", tone: "on" }
    : { title: "You're offline", text: "Everything keeps working on this device. Search and lookups wait for a connection.", tone: "off" };
  timer = window.setTimeout(() => (flash.value = null), up ? 3500 : 6000);
});
onBeforeUnmount(() => window.clearTimeout(timer));

function gotIt() {
  onlineNoticeSeen.value = true;
}
function turnOff() {
  onlineExtras.value = false;
  onlineNoticeSeen.value = true;
}
</script>

<template>
  <Transition name="pop">
    <aside v-if="firstRun" class="notice" role="dialog" aria-labelledby="notice-title" aria-live="polite">
      <div class="title"><span class="dot on" aria-hidden="true"></span><strong id="notice-title">Online extras are on</strong></div>
      <p>
        Places you pick get their time zone, ground height and name filled in. To do that, Skyglow sends only their
        rough coordinates to Open-Meteo and OpenStreetMap. Everything else stays on your device.
      </p>
      <div class="actions">
        <button type="button" class="primary" @click="gotIt">Got it</button>
        <button type="button" @click="turnOff">Turn off</button>
      </div>
    </aside>
    <aside v-else-if="flash" class="notice" role="status" aria-live="polite">
      <div class="title">
        <span class="dot" :class="flash.tone" aria-hidden="true"></span><strong>{{ flash.title }}</strong>
        <button type="button" class="close" aria-label="Close" @click="flash = null">&times;</button>
      </div>
      <p>{{ flash.text }}</p>
    </aside>
  </Transition>
</template>

<style scoped>
.notice {
  position: fixed;
  left: 16px;
  bottom: 16px;
  z-index: 20;
  width: min(340px, calc(100vw - 32px));
  background: var(--surface);
  border: 1px solid var(--line);
  border-radius: 12px;
  padding: 12px 14px;
  box-shadow: 0 6px 24px rgb(0 0 0 / 0.18);
  font-size: 0.88rem;
}
.title { display: flex; align-items: center; gap: 8px; }
.title strong { flex: 1; }
.notice p { margin: 6px 0 0; color: var(--text-2); }
.actions { display: flex; gap: 8px; margin-top: 10px; }
.dot { width: 9px; height: 9px; border-radius: 50%; background: var(--muted); flex: 0 0 auto; }
.dot.on { background: #2f9e44; }
.close { border: none; background: none; padding: 0 4px; font-size: 1.2rem; line-height: 1; color: var(--muted); }
.pop-enter-active, .pop-leave-active { transition: opacity 0.2s ease, transform 0.2s ease; }
.pop-enter-from, .pop-leave-to { opacity: 0; transform: translateY(8px); }
@media (prefers-reduced-motion: reduce) {
  .pop-enter-active, .pop-leave-active { transition: none; }
}
</style>
