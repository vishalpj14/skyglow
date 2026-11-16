<script setup lang="ts">
// The colour strip: minute by minute around sunset (or sunrise), what colour
// the sky stage is, and whether each cloud layer is lit, and with what light.
// Colours are the pack's UNCALIBRATED display guesses.
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import type { Glow, Pack, SkyState } from "../lib/types";
import { buildStrip, type StripColumn } from "../lib/strip";
import { hhmm, localHours } from "../lib/clock";

const props = defineProps<{ title: string; series: SkyState[]; pack: Pack; tz: number; cursorJd: number; glow?: Glow[] }>();
const emit = defineEmits<{ seek: [jd: number] }>();

const wrap = ref<HTMLDivElement>();
const width = ref(640);
let ro: ResizeObserver | null = null;
onMounted(() => {
  // Measure straight away (the first frame should already fit), then follow resizes.
  if (wrap.value?.clientWidth) width.value = Math.max(200, wrap.value.clientWidth);
  ro = new ResizeObserver(([e]) => (width.value = Math.max(200, e.contentRect.width)));
  if (wrap.value) ro.observe(wrap.value);
});
onBeforeUnmount(() => ro?.disconnect());

const labelW = 92;
const rowH = 22;
const gap = 2;
const axisH = 20;

const withWeather = computed(() => !!props.glow?.some((g) => g.has_conditions));
const cols = computed<StripColumn[]>(() => buildStrip(props.series, props.pack, props.glow));
const rows = computed(() => [
  "Sky",
  ...(props.series[0]?.layers ?? []).map((l) => `${l.name} ${l.height_km} km`),
]);
const H = computed(() => rows.value.length * (rowH + gap) + axisH);

const start = computed(() => props.series[0]?.jd ?? 0);
const end = computed(() => props.series[props.series.length - 1]?.jd ?? 1);
const x = (jd: number) => labelW + ((jd - start.value) / (end.value - start.value || 1)) * (width.value - labelW);
const cellW = computed(() => (width.value - labelW) / Math.max(1, cols.value.length - 1) + 0.6);

/** Axis ticks every 10 minutes, on round clock times. */
const ticks = computed(() => {
  const out: number[] = [];
  const step = 10 / 1440;
  const first = Math.ceil((start.value + props.tz / 24 + 0.5) / step) * step - props.tz / 24 - 0.5;
  for (let t = first; t <= end.value + 1e-9; t += step) out.push(t);
  return out;
});

const hover = ref<StripColumn | null>(null);
function at(ev: PointerEvent | MouseEvent): StripColumn | null {
  const r = (ev.currentTarget as SVGSVGElement).getBoundingClientRect();
  const jd = start.value + ((ev.clientX - r.left - labelW) / (width.value - labelW)) * (end.value - start.value);
  let best: StripColumn | null = null;
  for (const c of cols.value) if (!best || Math.abs(c.jd - jd) < Math.abs(best.jd - jd)) best = c;
  return best;
}
const pretty = (s: string) => s.replace(/_/g, " ");

// The tooltip sits BELOW the strip, so it never hides the colours it
// describes. It is centred on the hovered minute, kept inside the card, and
// a small pointer marks the exact column.
const tip = ref<HTMLDivElement>();
const tipW = ref(220);
const tipLeft = computed(() => {
  if (!hover.value) return 0;
  return Math.min(Math.max(0, x(hover.value.jd) - tipW.value / 2), Math.max(0, width.value - tipW.value));
});
const pointerX = computed(() => (hover.value ? x(hover.value.jd) - tipLeft.value : 0));
watch(hover, async () => {
  await nextTick();
  if (tip.value) tipW.value = tip.value.offsetWidth;
});
const inWindow = computed(() => props.cursorJd >= start.value && props.cursorJd <= end.value);
</script>

<template>
  <section class="card">
    <h2>{{ title }}</h2>
    <div ref="wrap" class="wrap">
      <svg
        v-if="series.length"
        :width="width"
        :height="H"
        role="img"
        :aria-label="`${title}: colour of the sky and each cloud layer over time`"
        @pointermove="hover = at($event)"
        @pointerleave="hover = null"
        @click="(e: MouseEvent) => { const c = at(e); if (c) emit('seek', c.jd); }"
      >
        <text v-for="(r, i) in rows" :key="r" x="0" :y="i * (rowH + gap) + rowH / 2 + 4" class="row-label">{{ r }}</text>
        <g v-for="c in cols" :key="c.jd">
          <rect :x="x(c.jd) - cellW / 2" y="0" :width="cellW" :height="rowH" :fill="c.sky.colour" />
          <rect v-for="(l, i) in c.layers" :key="i" :x="x(c.jd) - cellW / 2" :y="(i + 1) * (rowH + gap)" :width="cellW" :height="rowH" :fill="l.colour" />
        </g>
        <text v-for="t in ticks" :key="t" :x="x(t)" :y="H - 5" text-anchor="middle" class="axis">{{ hhmm(localHours(t, tz)) }}</text>
        <line v-if="inWindow" :x1="x(cursorJd)" :x2="x(cursorJd)" y1="0" :y2="H - axisH" class="cursor" />
        <line v-if="hover" :x1="x(hover.jd)" :x2="x(hover.jd)" y1="0" :y2="H - axisH" class="crosshair" />
      </svg>
      <p v-else class="muted">No {{ title.toLowerCase() }} today at this latitude (polar day or night).</p>
      <div v-if="hover" ref="tip" class="tip strip-tip" :style="{ left: tipLeft + 'px', top: H + 8 + 'px', '--pointer-x': pointerX + 'px' }">
        <strong>{{ hhmm(localHours(hover.jd, tz)) }}</strong><br />
        <span class="swatch" :style="{ background: hover.sky.colour }"></span>Sky: {{ pretty(hover.sky.label) }}<br />
        <template v-for="(l, i) in hover.layers" :key="i">
          <span class="swatch" :style="{ background: l.colour }"></span>{{ rows[i + 1] }}: {{ l.label }}<br />
        </template>
      </div>
    </div>
    <p v-if="withWeather" class="muted note">
      Each row is one cloud height from the pack. <strong>With the forecast:</strong> a row fades into the sky where little cloud is forecast at that height, or where other cloud or haze is likely to block it. Colours and fading are rough, uncalibrated guesses.
    </p>
    <p v-else class="muted note">Each row is one cloud height from the pack, if a cloud were there. Colours are rough, uncalibrated guesses from the pack.</p>
  </section>
</template>

<style scoped>
.wrap { position: relative; width: 100%; }
svg { display: block; cursor: crosshair; touch-action: pan-y; }
.row-label { fill: var(--text-2); font-size: 11px; }
.axis { fill: var(--muted); font-size: 10px; }
.cursor { stroke: var(--accent); stroke-width: 2; }
.crosshair { stroke: var(--text); stroke-width: 1; opacity: 0.6; }
.note { font-size: 0.8rem; margin-top: 6px; }
/* A fixed width, so its size does not change as it moves (and the clamping stays exact). */
.strip-tip { width: min(260px, 100%); white-space: normal; line-height: 1.5; }
/* Pointer up to the hovered column. */
.strip-tip::before {
  content: ""; position: absolute; top: -6px; left: clamp(10px, var(--pointer-x), calc(100% - 10px));
  width: 10px; height: 10px; transform: translateX(-50%) rotate(45deg);
  background: var(--surface); border-left: 1px solid var(--line); border-top: 1px solid var(--line);
}
</style>
