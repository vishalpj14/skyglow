<script setup lang="ts">
// The skyglow mark. At rest it is the app icon. Call play() (on hover) and
// the little sun sets through the pack's stage colours, or rises through
// them in reverse when the time scrubber is on a morning. rest() eases back.
import { computed, onBeforeUnmount, ref } from "vue";
import { blendLook, LOGO_REST, logoFrame, skyMood, type LogoLook } from "../lib/mood";

defineProps<{ size: number }>();

const PLAY_MS = 1800;
const BLEND_MS = 220;
const REST_MS = 380;

const look = ref<LogoLook>({ ...LOGO_REST });
let raf = 0;
let from: LogoLook = { ...LOGO_REST };
const frameAt = (p: number) => logoFrame(skyMood.value, p);
const blend = blendLook;

const ease = (t: number) => (t < 0.5 ? 2 * t * t : 1 - Math.pow(-2 * t + 2, 2) / 2);
const reduced = () => typeof matchMedia !== "undefined" && matchMedia("(prefers-reduced-motion: reduce)").matches;

function play() {
  cancelAnimationFrame(raf);
  if (reduced()) {
    look.value = frameAt(1);
    return;
  }
  from = { ...look.value };
  const start = performance.now();
  const step = (now: number) => {
    const e = now - start;
    const p = ease(Math.min(1, e / PLAY_MS));
    look.value = blend(from, frameAt(p), Math.min(1, e / BLEND_MS));
    if (e < PLAY_MS) raf = requestAnimationFrame(step);
  };
  raf = requestAnimationFrame(step);
}

function rest() {
  cancelAnimationFrame(raf);
  if (reduced()) {
    look.value = { ...LOGO_REST };
    return;
  }
  from = { ...look.value };
  const start = performance.now();
  const step = (now: number) => {
    const t = Math.min(1, (now - start) / REST_MS);
    look.value = blend(from, LOGO_REST, ease(t));
    if (t < 1) raf = requestAnimationFrame(step);
  };
  raf = requestAnimationFrame(step);
}

defineExpose({ play, rest });
onBeforeUnmount(() => cancelAnimationFrame(raf));

const id = `sky-${Math.random().toString(36).slice(2, 8)}`;
const l = computed(() => look.value);
</script>

<template>
  <svg :width="size" :height="size" viewBox="0 0 64 64" aria-hidden="true" class="mark">
    <defs>
      <linearGradient :id="id" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" :stop-color="l.top" />
        <stop offset="0.55" :stop-color="l.mid" />
        <stop offset="1" :stop-color="l.bottom" />
      </linearGradient>
      <clipPath :id="`${id}-r`"><rect width="64" height="64" rx="14" /></clipPath>
    </defs>
    <g :clip-path="`url(#${id}-r)`">
      <rect width="64" height="64" :fill="`url(#${id})`" />
      <circle cx="32" :cy="l.sunY" r="10" :fill="l.sun" />
      <rect x="0" y="46" width="64" height="18" fill="#141a2e" />
      <path d="M12 22 h22 a6 6 0 0 1 0 6 h-22 z" :fill="l.cloud" opacity="0.9" />
    </g>
  </svg>
</template>

<style scoped>
.mark { display: block; flex: 0 0 auto; }
</style>
