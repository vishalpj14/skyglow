<script setup lang="ts">
// The sun's height over the whole local day, with the pack's light stages
// drawn as bands. Hover to read a time; click to move the time scrubber.
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import type { Pack, SkyState } from "../lib/types";
import { hhmm, localHours } from "../lib/clock";

const props = defineProps<{ series: SkyState[]; pack: Pack; tz: number; cursorJd: number; dayStartJd: number }>();
const emit = defineEmits<{ seek: [jd: number] }>();

const twilightZoom = ref(true);
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

const H = 240;
const pad = { l: 40, r: 10, t: 10, b: 26 };

const yRange = computed<[number, number]>(() => {
  if (twilightZoom.value) {
    // Stretch up to show a tall skyline (valley walls) when there is one.
    const sky = Math.max(0, ...props.series.map((s) => s.skyline?.horizon_elevation_deg ?? 0));
    return [-12, Math.max(12, Math.ceil((sky + 4) / 6) * 6)];
  }
  const max = Math.max(...props.series.map((s) => s.elevation_deg), 10);
  return [-20, Math.ceil((max + 5) / 10) * 10];
});

const x = (jd: number) => pad.l + ((jd - props.dayStartJd) / 1) * (width.value - pad.l - pad.r);
// Not clamped: values outside the range are drawn past the plot edge and
// clipped there, so a zoomed curve leaves the chart instead of flattening.
const y = (deg: number) => {
  const [lo, hi] = yRange.value;
  return pad.t + ((hi - deg) / (hi - lo)) * (H - pad.t - pad.b);
};
const clipId = `sunpath-clip-${Math.random().toString(36).slice(2)}`;

const path = computed(() =>
  props.series.map((s, i) => `${i ? "L" : "M"}${x(s.jd).toFixed(1)},${y(s.elevation_deg).toFixed(1)}`).join(""),
);

/** Terrain height in the sun's direction, only drawn if the place has a skyline. */
const skylinePath = computed(() => {
  if (!props.series.length || !props.series[0].skyline) return "";
  return props.series
    .map((s, i) => `${i ? "L" : "M"}${x(s.jd).toFixed(1)},${y(s.skyline!.horizon_elevation_deg).toFixed(1)}`)
    .join("");
});

const bands = computed(() =>
  props.pack.stages.map((st) => {
    const top = y(st.from_deg);
    const bottom = y(st.to_deg);
    return { st, top, h: bottom - top };
  }),
);

const yTicks = computed(() => {
  const [lo, hi] = yRange.value;
  const step = hi - lo > 30 ? 20 : 6;
  const out: number[] = [];
  for (let v = Math.ceil(lo / step) * step; v <= hi; v += step) out.push(v);
  return out;
});
const xTicks = [0, 3, 6, 9, 12, 15, 18, 21, 24];

const hover = ref<SkyState | null>(null);
function nearest(ev: PointerEvent): SkyState | null {
  const svg = ev.currentTarget as SVGSVGElement;
  const r = svg.getBoundingClientRect();
  const px = ev.clientX - r.left;
  const jd = props.dayStartJd + (px - pad.l) / (width.value - pad.l - pad.r);
  let best: SkyState | null = null;
  for (const s of props.series) if (!best || Math.abs(s.jd - jd) < Math.abs(best.jd - jd)) best = s;
  return best;
}
const stageName = (s: SkyState) =>
  s.stage == null ? (s.elevation_deg > 0 ? "day" : "night") : (s.rising && props.pack.stages[s.stage].morning_name) || props.pack.stages[s.stage].name;
</script>

<template>
  <section class="card">
    <div class="head">
      <h2>Sun height through the day</h2>
      <label class="zoom"><input v-model="twilightZoom" type="checkbox" /> Zoom to twilight</label>
    </div>
    <div ref="wrap" class="wrap">
      <svg
        :width="width"
        :height="H"
        role="img"
        :aria-label="`Sun elevation over the day. Click to set the time.`"
        @pointermove="hover = nearest($event)"
        @pointerleave="hover = null"
        @click="(e: MouseEvent) => { const s = nearest(e as PointerEvent); if (s) emit('seek', s.jd); }"
      >
        <defs>
          <clipPath :id="clipId"><rect :x="pad.l" :y="pad.t" :width="width - pad.l - pad.r" :height="H - pad.t - pad.b" /></clipPath>
        </defs>
        <g v-for="b in bands" :key="b.st.name" :clip-path="`url(#${clipId})`">
          <rect :x="pad.l" :y="b.top" :width="width - pad.l - pad.r" :height="Math.max(0, b.h)" :fill="b.st.swatch || 'transparent'" opacity="0.22" />
          <text v-if="b.h >= 13" :x="width - pad.r - 4" :y="b.top + b.h / 2 + 4" text-anchor="end" class="band-label">{{ b.st.name.replace(/_/g, " ") }}</text>
        </g>
        <line v-for="t in yTicks" :key="'y' + t" :x1="pad.l" :x2="width - pad.r" :y1="y(t)" :y2="y(t)" class="grid" />
        <text v-for="t in yTicks" :key="'yl' + t" :x="pad.l - 6" :y="y(t) + 4" text-anchor="end" class="axis">{{ t }}&deg;</text>
        <line :x1="pad.l" :x2="width - pad.r" :y1="y(0)" :y2="y(0)" class="horizon" />
        <text v-for="t in xTicks" :key="'x' + t" :x="x(dayStartJd + t / 24)" :y="H - 8" text-anchor="middle" class="axis">{{ String(t).padStart(2, "0") }}</text>
        <g :clip-path="`url(#${clipId})`">
          <path v-if="skylinePath" :d="skylinePath" class="skyline" />
          <path :d="path" class="sun" />
        </g>
        <line :x1="x(cursorJd)" :x2="x(cursorJd)" :y1="pad.t" :y2="H - pad.b" class="cursor" />
        <template v-if="hover">
          <line :x1="x(hover.jd)" :x2="x(hover.jd)" :y1="pad.t" :y2="H - pad.b" class="crosshair" />
          <circle v-if="hover.elevation_deg >= yRange[0] && hover.elevation_deg <= yRange[1]" :cx="x(hover.jd)" :cy="y(hover.elevation_deg)" r="4" class="dot" />
        </template>
      </svg>
      <div v-if="hover" class="tip" :style="{ left: Math.min(x(hover.jd) + 10, width - 170) + 'px', top: '8px' }">
        <strong>{{ hhmm(localHours(hover.jd, tz)) }}</strong> &middot; sun {{ hover.elevation_deg.toFixed(1) }}&deg;<br />
        {{ stageName(hover).replace(/_/g, " ") }}
      </div>
    </div>
    <p class="muted legend-note">
      Orange line: sun height. Grey line at 0&deg;: the level horizon.
      <template v-if="skylinePath">Thin line: the skyline in the sun's direction.</template>
      Bands: the pack's light stages. Click to set the time.
    </p>
  </section>
</template>

<style scoped>
.head { display: flex; justify-content: space-between; align-items: baseline; gap: 8px; flex-wrap: wrap; }
.zoom { display: flex; gap: 6px; align-items: center; }
.wrap { position: relative; width: 100%; }
svg { display: block; cursor: crosshair; touch-action: pan-y; }
.grid { stroke: var(--grid); stroke-width: 1; }
.horizon { stroke: var(--muted); stroke-width: 1; }
.axis { fill: var(--muted); font-size: 11px; }
.band-label { fill: var(--text-2); font-size: 11px; }
.sun { fill: none; stroke: var(--sun); stroke-width: 2; stroke-linejoin: round; stroke-linecap: round; }
.skyline { fill: none; stroke: var(--text-2); stroke-width: 1; }
.cursor { stroke: var(--accent); stroke-width: 2; }
.crosshair { stroke: var(--text-2); stroke-width: 1; }
.dot { fill: var(--sun); stroke: var(--surface); stroke-width: 2; }
.legend-note { font-size: 0.8rem; margin-top: 6px; }
</style>
