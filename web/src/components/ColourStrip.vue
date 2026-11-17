<script setup lang="ts">
// The colour strip: minute by minute around sunset (or sunrise), what colour
// the sky stage is, and whether each cloud layer is lit, and with what light.
// Colours are the pack's UNCALIBRATED display guesses.
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import type { Glow, Pack, SkyState } from "../lib/types";
import { buildStrip, layersToShow, layerSummary, type LayerSummary, type StripColumn } from "../lib/strip";
import { hhmm, localHours } from "../lib/clock";
import { isHighGround } from "../lib/describe";
import { levelWord } from "../lib/glowText";

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

const rowH = 22;
const gap = 2;
const axisH = 20;

const withWeather = computed(() => !!props.glow?.some((g) => g.has_conditions));
const cols = computed<StripColumn[]>(() => buildStrip(props.series, props.pack, props.glow));
// With a forecast, only the layers that have some cloud get a row.
const shown = computed(() => layersToShow(props.series[0]?.layers.length ?? 0, props.glow));
const hidden = computed(() => (props.series[0]?.layers ?? []).filter((_, i) => !shown.value.includes(i)).map((l) => l.name));
const hiddenText = computed(() => {
  const names = hidden.value.map((n) => (n === "mid" ? "middle" : n));
  return names.length > 1 ? `${names.slice(0, -1).join(", ")} and ${names[names.length - 1]}` : names.join("");
});
// At a high place each row also says how far the layer is above the ground.
const high = computed(() => !!props.series[0]?.layers.some(isHighGround));
const labelW = computed(() => (high.value ? 112 : 92));
const rows = computed(() => [
  { name: "Sky", sub: "" },
  ...shown.value.map((i) => {
    const l = props.series[0].layers[i];
    return { name: `${l.name} ${l.height_km.toFixed(1)} km`, sub: high.value ? `${l.above_ground_km.toFixed(1)} km above ground` : "" };
  }),
]);
const H = computed(() => rows.value.length * (rowH + gap) + axisH);

const start = computed(() => props.series[0]?.jd ?? 0);
const end = computed(() => props.series[props.series.length - 1]?.jd ?? 1);
const x = (jd: number) => labelW.value + ((jd - start.value) / (end.value - start.value || 1)) * (width.value - labelW.value);
const cellW = computed(() => (width.value - labelW.value) / Math.max(1, cols.value.length - 1) + 0.6);

/** Axis ticks every 10 minutes, on round clock times. */
const ticks = computed(() => {
  const out: number[] = [];
  const step = 10 / 1440;
  const first = Math.ceil((start.value + props.tz / 24 + 0.5) / step) * step - props.tz / 24 - 0.5;
  for (let t = first; t <= end.value + 1e-9; t += step) out.push(t);
  return out;
});

const hover = ref<StripColumn | null>(null);
/** The layer row whose label is under the pointer (its index in the series), or null. */
const hoverRow = ref<number | null>(null);
function move(ev: PointerEvent) {
  const r = (ev.currentTarget as SVGSVGElement).getBoundingClientRect();
  const row = Math.floor((ev.clientY - r.top) / (rowH + gap)) - 1;
  if (ev.clientX - r.left < labelW.value && row >= 0 && row < shown.value.length) {
    hoverRow.value = shown.value[row];
    hover.value = null;
  } else {
    hoverRow.value = null;
    hover.value = ev.clientX - r.left < labelW.value ? null : at(ev);
  }
}
const summary = computed<LayerSummary | null>(() =>
  hoverRow.value == null ? null : layerSummary(props.series, hoverRow.value, props.glow),
);
const span = (a: number, b: number) => (Math.round(a) === Math.round(b) ? `${Math.round(a)}%` : `${Math.round(a)} to ${Math.round(b)}%`);
function at(ev: PointerEvent | MouseEvent): StripColumn | null {
  const r = (ev.currentTarget as SVGSVGElement).getBoundingClientRect();
  const jd = start.value + ((ev.clientX - r.left - labelW.value) / (width.value - labelW.value)) * (end.value - start.value);
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
const pointerX = computed(() => (hover.value ? x(hover.value.jd) - tipLeft.value : summary.value ? 20 : 0));
watch([hover, summary], async () => {
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
        @pointermove="move"
        @pointerleave="hover = null; hoverRow = null"
        @click="(e: MouseEvent) => { const c = at(e); if (c) emit('seek', c.jd); }"
      >
        <template v-for="(r, i) in rows" :key="r.name">
          <text v-if="!r.sub" x="0" :y="i * (rowH + gap) + rowH / 2 + 4" class="row-label" :class="{ hot: i > 0 && shown[i - 1] === hoverRow }">{{ r.name }}</text>
          <template v-else>
            <text x="0" :y="i * (rowH + gap) + 9" class="row-label" :class="{ hot: shown[i - 1] === hoverRow }">{{ r.name }}</text>
            <text x="0" :y="i * (rowH + gap) + 19" class="row-sub">{{ r.sub }}</text>
          </template>
        </template>
        <g v-for="c in cols" :key="c.jd">
          <rect :x="x(c.jd) - cellW / 2" y="0" :width="cellW" :height="rowH" :fill="c.sky.colour" />
          <rect v-for="(li, row) in shown" :key="li" :x="x(c.jd) - cellW / 2" :y="(row + 1) * (rowH + gap)" :width="cellW" :height="rowH" :fill="c.layers[li].colour" />
        </g>
        <text v-for="t in ticks" :key="t" :x="x(t)" :y="H - 5" text-anchor="middle" class="axis">{{ hhmm(localHours(t, tz)) }}</text>
        <line v-if="inWindow" :x1="x(cursorJd)" :x2="x(cursorJd)" y1="0" :y2="H - axisH" class="cursor" />
        <line v-if="hover" :x1="x(hover.jd)" :x2="x(hover.jd)" y1="0" :y2="H - axisH" class="crosshair" />
      </svg>
      <p v-else class="muted">No {{ title.toLowerCase() }} today at this latitude (polar day or night).</p>
      <div v-if="hover" ref="tip" class="tip strip-tip" :style="{ left: tipLeft + 'px', top: H + 8 + 'px', '--pointer-x': pointerX + 'px' }">
        <strong>{{ hhmm(localHours(hover.jd, tz)) }}</strong><br />
        <span class="swatch" :style="{ background: hover.sky.colour }"></span>Sky: {{ pretty(hover.sky.label) }}<br />
        <template v-for="(li, row) in shown" :key="li">
          <span class="swatch" :style="{ background: hover.layers[li].colour }"></span>{{ rows[row + 1].name }}: {{ hover.layers[li].label }}<br />
        </template>
      </div>
      <div v-else-if="summary" ref="tip" class="tip strip-tip" :style="{ left: '0px', top: H + 8 + 'px', '--pointer-x': pointerX + 'px' }">
        <strong>{{ summary.name }} cloud layer</strong><br />
        Height: {{ summary.height_km.toFixed(1) }} km above sea level<template v-if="high">, {{ summary.above_ground_km.toFixed(1) }} km above the ground</template><br />
        <template v-if="summary.level && summary.range_km">
          Forecast's {{ levelWord(summary.level) }} cloud here: {{ summary.range_km[0].toFixed(1) }} to {{ summary.range_km[1].toFixed(1) }} km above sea level<br />
        </template>
        <template v-if="pack.cloud_ceiling_km">Highest a cloud can be: {{ pack.cloud_ceiling_km.toFixed(0) }} km above sea level (the pack's ceiling)<br /></template>
        <template v-if="summary.cover_pct">
          Cover in this window: {{ span(summary.cover_pct[0], summary.cover_pct[1]) }}<template v-if="summary.cloud_type">, likely {{ summary.cloud_type }}</template><br />
        </template>
        <template v-if="summary.best && summary.best.score > 0">
          Best glow: {{ summary.best.score.toFixed(1) }}/10, {{ summary.best.word }}, at {{ hhmm(localHours(summary.best.jd, tz)) }}<br />
        </template>
        <template v-if="summary.lit">Lit by the sun in this window: {{ hhmm(localHours(summary.lit[0], tz)) }} to {{ hhmm(localHours(summary.lit[1], tz)) }}<br /></template>
        <span v-if="summary.cloud_type" class="muted">The cloud type is a guess from the cover alone.</span>
      </div>
    </div>
    <p v-if="withWeather && hidden.length === (series[0]?.layers.length ?? 0)" class="muted note">
      <strong>Clear sky:</strong> the forecast has (almost) no cloud at any height in this window, so there is nothing for the low sun to light. Only the sky's colour is shown.
    </p>
    <p v-else-if="withWeather && hidden.length" class="muted note">
      Little or no cloud forecast at {{ hiddenText }} height in this window, so {{ hidden.length === 1 ? "that row is" : "those rows are" }} left out.
    </p>
    <p v-if="series.length && shown.length" class="muted note">Hover a row's name for that layer's heights, cover and likely cloud type.</p>
    <p v-if="withWeather && shown.length" class="muted note">
      Each row is one cloud height from the pack. <strong>With the forecast:</strong> a row fades into the sky where little cloud is forecast at that height, or where other cloud or haze is likely to block it. Colours and fading are rough, uncalibrated guesses.
    </p>
    <p v-else-if="!withWeather" class="muted note">Each row is one cloud height from the pack, if a cloud were there. Colours are rough, uncalibrated guesses from the pack.</p>
  </section>
</template>

<style scoped>
.wrap { position: relative; width: 100%; }
svg { display: block; cursor: crosshair; touch-action: pan-y; }
.row-label { fill: var(--text-2); font-size: 11px; cursor: help; }
.row-label.hot { fill: var(--text); font-weight: 600; }
.row-sub { fill: var(--muted); font-size: 9px; }
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
