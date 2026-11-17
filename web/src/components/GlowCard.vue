<script setup lang="ts">
// The glow score: the forecast (Conditions) put together with the geometry.
// Shows the best moment around sunrise and sunset, and each layer's score at
// the scrubber's time, with every factor explained. All numbers come from the
// core (core/src/glow.rs). A rough guide, not calibrated yet.
import { computed } from "vue";
import type { Glow, GlowMoment, Place, Timeline } from "../lib/types";
import * as engine from "../lib/engine";
import type { ConditionsState } from "../lib/useConditions";
import { explainLayer, highestCloudText, levelWord, rangeText, times } from "../lib/glowText";
import { heightText } from "../lib/describe";

const props = defineProps<{
  place: Place;
  packText: string;
  state: ConditionsState;
  timelines: Timeline[];
  cursorJd: number;
  tz: number;
}>();
const emit = defineEmits<{ seek: [jd: number] }>();

const series = computed(() => props.state.weatherText.value);

function safely<T>(fn: () => T, fallback: T): T {
  try {
    return fn();
  } catch {
    return fallback;
  }
}

/** Same windows as the colour strips: an hour before sunrise to 40 min after, 40 min before sunset to an hour after. */
const best = computed(() =>
  props.timelines.map((t) => {
    const jd = t.event_jd ?? t.sea_event_jd;
    const [before, after] = t.event === "sunrise" ? [60, 40] : [40, 60];
    // Step on whole clock minutes, so "Go" lands on exactly the minute found.
    const onMinute = (x: number) => Math.round(x * 1440) / 1440;
    const moment: GlowMoment | null =
      jd == null || !series.value
        ? null
        : safely(() => engine.bestGlow(props.place, props.packText, series.value, onMinute(jd - before / 1440), onMinute(jd + after / 1440), 1), null);
    return { event: t.event, moment, hasEvent: jd != null };
  }),
);

const now = computed<Glow | null>(() => safely(() => engine.glowAt(props.place, props.packText, series.value, props.cursorJd), null));
const here = computed(() => props.state.seriesText.value);
const reading = computed(() => (here.value ? safely(() => engine.conditionsAt(here.value, props.cursorJd), null) : null));

const ceilingKm = computed(() => safely(() => engine.packInfo(props.packText).cloud_ceiling_km, 16));
const highest = computed(() => (now.value ? highestCloudText(now.value, ceilingKm.value) : null));
const hhmm = (jd: number) => engine.formatClock(jd, props.tz).slice(0, 5);
/** Is the sun climbing at the scrubber's time? Decides which direction's points apply. */
const skyRising = computed(() => safely(() => engine.skyAt(props.place, props.packText, props.cursorJd).rising, false));
const farthest = computed(() => {
  const s = props.state.stored.value?.weather;
  const dir = skyRising.value ? s?.sunrise : s?.sunset;
  return dir?.points.length ? Math.round(dir.points[dir.points.length - 1].distance_km) : 0;
});
const cap = (s: string) => s.charAt(0).toUpperCase() + s.slice(1);
</script>

<template>
  <section class="card glow">
    <div class="head">
      <h2>Glow forecast</h2>
      <span class="badge" title="Built from public rules of thumb, not yet checked against real photos">rough guide</span>
    </div>

    <ul class="best">
      <li v-for="b in best" :key="b.event">
        <span class="ev">{{ cap(b.event) }}</span>
        <template v-if="!b.hasEvent"><span class="muted">no {{ b.event }} today here</span></template>
        <template v-else-if="!series"><span class="muted">needs the forecast (see Conditions)</span></template>
        <template v-else-if="!b.moment"><span class="muted">the forecast does not reach this {{ b.event }}</span></template>
        <template v-else-if="b.moment.score === 0"><span>No cloud forecast to light: expect a clear-sky gradient.</span></template>
        <template v-else>
          <span>
            <strong>{{ b.moment.layer }} cloud</strong> around <strong>{{ hhmm(b.moment.jd) }}</strong>:
            <span class="score">{{ b.moment.score.toFixed(1) }}</span>/10, {{ b.moment.word }}
          </span>
          <button type="button" @click="emit('seek', b.moment.jd)">Go</button>
        </template>
      </li>
    </ul>

    <template v-if="now">
      <h3>At {{ hhmm(cursorJd) }}</h3>
      <p v-if="highest" class="small highest">{{ highest }}</p>
      <p v-if="now.has_conditions" class="muted small">
        Sunbeam's path:
        <template v-if="now.along_sun">forecast at points up to {{ farthest }} km toward the {{ skyRising ? "sunrise" : "sunset" }}.</template>
        <template v-else>local forecast only, standing in for the whole way to the sun.</template>
      </p>
      <p v-if="!now.has_conditions" class="muted small">
        No forecast for this time, so these are only the <em>potential</em>: how well each height would glow if cloud were there and the air were clear.
      </p>
      <div v-for="l in now.layers" :key="l.name" class="layer">
        <div class="row">
          <span class="name">{{ l.name }} <small>{{ heightText(l) }}, {{ levelWord(l.level) }}</small></span>
          <span class="bar" role="img" :aria-label="`${l.name}: ${l.score != null ? l.score.toFixed(1) : 'potential ' + l.potential.toFixed(1)} out of 10`">
            <i class="potential" :style="{ width: l.potential * 10 + '%' }"></i>
            <i v-if="l.score != null" class="score-bar" :style="{ width: l.score * 10 + '%' }"></i>
          </span>
          <span class="val">
            <template v-if="l.score != null">{{ l.score.toFixed(1) }} <small>{{ l.word }}</small></template>
            <template v-else><small>could be</small> {{ l.potential.toFixed(1) }}</template>
          </span>
        </div>
        <p v-if="l.cloud_type" class="muted small kind">{{ l.cover_pct?.toFixed(0) }}% cover: likely {{ l.cloud_type }} (a guess from the cover)</p>
        <details>
          <summary>Why</summary>
          <p class="muted small">{{ rangeText(l) }}</p>
          <ul class="why">
            <li v-for="(f, i) in explainLayer(l, reading?.conditions ?? null)" :key="i">
              <span>{{ f.text }}</span><span class="x">{{ times(f.value) }}</span>
            </li>
          </ul>
          <p v-if="l.unknown.length" class="muted small">Not known, so not counted: {{ l.unknown.join(", ") }}.</p>
          <p class="muted small">Score = 10 × all of these<template v-if="l.score != null"> = {{ l.score.toFixed(1) }}</template>.</p>
        </details>
      </div>
      <p class="muted small key"><i class="potential sw"></i>could be, with cloud and clear air <i class="score-bar sw"></i>with the forecast</p>
    </template>

    <p class="muted small">
      Built from public rules of thumb (clean air gives the most vivid colour, thick low cloud toward the sun blocks it, some cloud beats none or total overcast). Not calibrated yet: your logged evenings will tune it.
      <a href="#/docs/colours">How the glow score works</a>
    </p>
  </section>
</template>

<style scoped>
.head { display: flex; align-items: center; gap: 10px; }
.head h2 { margin: 0; }
.badge { font-size: 0.72rem; text-transform: uppercase; letter-spacing: 0.08em; font-weight: 600; color: var(--text-2); border: 1px dashed var(--line); border-radius: 999px; padding: 1px 8px; }
.best { list-style: none; padding: 0; margin: 10px 0 4px; display: grid; gap: 6px; }
.best li { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
.best .ev { min-width: 5.5em; font-size: 0.78rem; text-transform: uppercase; letter-spacing: 0.08em; color: var(--muted); font-weight: 600; }
.best button { padding: 2px 10px; }
.score { font-weight: 700; font-variant-numeric: tabular-nums; }
h3 { font-size: 0.78rem; text-transform: uppercase; letter-spacing: 0.08em; color: var(--muted); font-weight: 600; margin: 14px 0 6px; }
.layer { margin: 6px 0; }
.row { display: grid; grid-template-columns: minmax(0, 10em) minmax(40px, 1fr) 6.5em; gap: 10px; align-items: center; }
.name small { color: var(--muted); display: block; font-size: 0.75rem; }
.bar { position: relative; height: 10px; border-radius: 999px; background: var(--surface-2); border: 1px solid var(--line); overflow: hidden; }
.bar i { position: absolute; left: 0; top: 0; bottom: 0; border-radius: 999px; }
.potential { background: repeating-linear-gradient(135deg, var(--line) 0 4px, transparent 4px 8px); }
.score-bar { background: var(--sun); }
.val { text-align: right; font-weight: 700; font-variant-numeric: tabular-nums; }
.val small { font-weight: 400; color: var(--muted); }
details { margin: 2px 0 0; }
summary { cursor: pointer; color: var(--text-2); font-size: 0.85rem; }
.why { list-style: none; padding: 0; margin: 6px 0; display: grid; gap: 3px; font-size: 0.85rem; }
.why li { display: flex; justify-content: space-between; gap: 12px; border-bottom: 1px dotted var(--line); padding-bottom: 2px; }
.why .x { font-variant-numeric: tabular-nums; font-weight: 600; white-space: nowrap; }
.small { font-size: 0.85rem; margin: 8px 0 0; }
.key { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
.sw { display: inline-block; width: 18px; height: 8px; border-radius: 999px; border: 1px solid var(--line); }
@media (max-width: 480px) {
  .row { grid-template-columns: minmax(0, 6em) minmax(40px, 1fr) 5.5em; }
}
</style>
