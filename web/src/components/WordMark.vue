<script setup lang="ts">
// The word "skyglow", painted in the logo's colours while the logo plays.
// A coloured copy of the word sits exactly over the normal one and fades in
// with the logo (glow 0..1), so the change is smooth both ways.
import { computed, ref, watch } from "vue";
import { cssColourToHex, LOGO_REST, wordmarkColours, type LogoLook } from "../lib/mood";

const props = defineProps<{ text: string; look?: LogoLook; glow?: number }>();

const base = ref<HTMLElement>();
const baseHex = ref("#f0efec");
// Read the normal text colour each time the glow starts (the theme may have changed).
watch(
  () => (props.glow ?? 0) > 0,
  (on) => {
    if (on && base.value) baseHex.value = cssColourToHex(getComputedStyle(base.value).color);
  },
);

const paint = computed(() => {
  const c = wordmarkColours(props.look ?? LOGO_REST, baseHex.value);
  return `linear-gradient(90deg, ${c[0]} 0%, ${c[1]} 30%, ${c[2]} 65%, ${c[3]} 100%)`;
});
</script>

<template>
  <span class="wm">
    <span ref="base" class="base" :style="{ opacity: 1 - (glow ?? 0) }">{{ text }}</span>
    <span v-if="(glow ?? 0) > 0" class="paint" aria-hidden="true" :style="{ backgroundImage: paint, opacity: glow }">{{ text }}</span>
  </span>
</template>

<style scoped>
.wm { position: relative; display: inline-block; }
.paint {
  position: absolute;
  inset: 0;
  color: transparent;
  -webkit-background-clip: text;
  background-clip: text;
  pointer-events: none;
}
</style>
