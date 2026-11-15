<script setup lang="ts">
// Draws the page-transition circle (see lib/transition.ts).
import { computed } from "vue";
import { FADE_MS, GROW_MS, iris } from "../lib/transition";

const style = computed(() => {
  const i = iris.value;
  if (!i) return {};
  const d = i.r * 2;
  return {
    left: `${i.x - i.r}px`,
    top: `${i.y - i.r}px`,
    width: `${d}px`,
    height: `${d}px`,
    // A sun: bright accent in the middle, deepening toward the edge.
    animationDuration: `${i.phase === "grow" ? GROW_MS : FADE_MS}ms`,
    background: `radial-gradient(circle, color-mix(in srgb, ${i.colour} 85%, white) 0%, ${i.colour} 35%, color-mix(in srgb, ${i.colour} 55%, var(--bg)) 100%)`,
  };
});
</script>

<template>
  <div v-if="iris" class="iris" :class="iris.phase" :style="style" aria-hidden="true"></div>
</template>

<style scoped>
.iris {
  position: fixed;
  z-index: 100;
  border-radius: 50%;
  pointer-events: none;
  transform: scale(0);
}
.iris.grow { animation: iris-grow 710ms cubic-bezier(0.7, 0, 0.25, 1) forwards; }
.iris.fade { transform: scale(1); animation: iris-fade 570ms ease-out forwards; }
@keyframes iris-grow {
  from { transform: scale(0); }
  to { transform: scale(1); }
}
@keyframes iris-fade {
  from { opacity: 1; transform: scale(1); }
  to { opacity: 0; transform: scale(1.08); }
}
</style>
