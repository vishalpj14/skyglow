<script setup lang="ts">
// Cloud cover, moisture and haze at the scrubber's time, from the Open-Meteo
// forecast. Fetched when online extras are on, saved in this browser so it is
// still here offline, and always labelled with where it came from and how old
// it is. The Glow card turns these numbers into a rough score.
import { computed } from "vue";
import type { LocalDate } from "../lib/types";
import * as engine from "../lib/engine";
import { ageText, conditionsFileName, conditionsFileText, downloadText } from "../lib/conditions";
import type { ConditionsState } from "../lib/useConditions";
import { isOnline } from "../lib/network";
import { onlineExtras } from "../lib/saved";

const props = defineProps<{ state: ConditionsState; date: LocalDate; cursorJd: number; tz: number }>();
const st = props.state;

/** Keep this forecast as a file, to store next to an observation of this evening. */
function saveFile() {
  if (st.stored.value) downloadText(conditionsFileName(props.date), conditionsFileText(st.stored.value));
}

const reading = computed(() => {
  if (!st.seriesText.value) return null;
  try {
    return engine.conditionsAt(st.seriesText.value, props.cursorJd);
  } catch {
    return null;
  }
});

const hhmm = (jd: number) => engine.formatClock(jd, props.tz).slice(0, 5);
const basisText = computed(() => {
  const b = reading.value?.basis;
  if (!b) return "";
  return b.kind === "between"
    ? `Worked out between the ${hhmm(b.before_jd)} and ${hhmm(b.after_jd)} forecast hours.`
    : `Taken from the ${hhmm(b.sample_jd)} forecast hour.`;
});

const cover = computed(() => {
  const c = reading.value?.conditions;
  if (!c) return [];
  return [
    { name: "High", hint: "above about 8 km", pct: c.cloud_high_pct },
    { name: "Middle", hint: "about 3 to 8 km", pct: c.cloud_mid_pct },
    { name: "Low", hint: "below about 3 km", pct: c.cloud_low_pct },
  ].filter((r): r is { name: string; hint: string; pct: number } => r.pct != null);
});

/** Where the copy on screen came from and how old it is. Re-read as the scrubber ticks. */
const sourceText = computed(() => {
  const s = st.stored.value;
  if (!s) return "";
  void props.cursorJd;
  const age = ageText(Date.now() - s.fetchedAt);
  const lead = st.from.value === "network" ? `${s.series.source}, fetched ${age}.` : `${s.series.source}, saved on this device ${age}.`;
  const missing = !s.parts.aerosols ? " Haze was not available." : !s.parts.weather ? " Cloud and moisture were not available." : "";
  return lead + missing;
});

const emptyText = computed(() => {
  if (st.busy.value) return "Looking up the forecast…";
  if (st.problem.value) return `No forecast for this place and date: ${st.problem.value}.`;
  if (!onlineExtras.value) return "Online extras are off, so skyglow has not looked up the weather. Switch them on from the status pill at the top.";
  if (!isOnline.value) return "You are offline and no forecast is saved for this place and date. Everything else still works.";
  return "No forecast yet.";
});
</script>

<template>
  <section class="card conditions">
    <div class="head">
      <h2>Conditions</h2>
      <div class="actions">
        <button v-if="st.stored.value" type="button" title="Download this forecast to keep with your photos or notes of this evening" @click="saveFile">Save as file</button>
        <button type="button" :disabled="st.busy.value || !st.mayFetch.value" :title="st.mayFetch.value ? 'Ask Open-Meteo again now' : 'Needs a connection and online extras'" @click="st.load(true)">
          {{ st.busy.value ? "Looking up…" : "Refresh" }}
        </button>
      </div>
    </div>

    <template v-if="st.stored.value">
      <div v-if="reading" class="grid" aria-live="polite">
        <div v-if="cover.length" class="block">
          <h3>Cloud cover</h3>
          <div v-for="r in cover" :key="r.name" class="meter">
            <span class="name">{{ r.name }} <small>{{ r.hint }}</small></span>
            <span class="bar" role="img" :aria-label="`${r.name} cloud covers ${Math.round(r.pct)} percent of the sky`"><i :style="{ width: r.pct + '%' }"></i></span>
            <span class="val">{{ Math.round(r.pct) }}%</span>
          </div>
        </div>
        <dl class="block">
          <template v-if="reading.conditions.humidity_pct != null">
            <dt>Humidity</dt>
            <dd>{{ Math.round(reading.conditions.humidity_pct) }}% <small>near the ground</small></dd>
          </template>
          <template v-if="reading.conditions.water_vapour_mm != null">
            <dt>Water vapour</dt>
            <dd>{{ Math.round(reading.conditions.water_vapour_mm) }} mm <small>in the air above (dry under 10, tropical wet 50+)</small></dd>
          </template>
          <template v-if="reading.conditions.aerosol_optical_depth != null">
            <dt>Haze</dt>
            <dd>{{ reading.conditions.aerosol_optical_depth.toFixed(2) }} <small>aerosol optical depth (very clean 0.05, hazy 0.5)</small></dd>
          </template>
        </dl>
      </div>
      <p v-else class="muted">The forecast has no reading close to this time. Move the scrubber, or pick a date nearer today.</p>

      <p class="muted note">
        {{ basisText }} {{ sourceText }}
        <template v-if="st.problem.value">Could not refresh: {{ st.problem.value }}.</template>
      </p>
    </template>
    <p v-else class="muted">{{ emptyText }}</p>

    <p class="muted note">A weather model's forecast for the area, not a measurement. The Glow card above turns it into a rough score.</p>
    <p v-if="st.stored.value" class="muted note">
      Photographed this evening? <strong>Save as file</strong> and keep it with your notes: the forecast service only goes back about three months, and these numbers are what skyglow will learn colours from. <a href="#/docs/observations">How to log an evening</a>
    </p>
  </section>
</template>

<style scoped>
.head { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
.head h2 { margin: 0; }
.actions { display: flex; gap: 8px; flex-wrap: wrap; justify-content: end; }
.grid { display: grid; gap: 14px 28px; margin-top: 10px; grid-template-columns: minmax(0, 1fr); }
@media (min-width: 720px) {
  .grid { grid-template-columns: minmax(0, 1.2fr) minmax(0, 1fr); }
}
.block { margin: 0; min-width: 0; }
h3 { font-size: 0.78rem; text-transform: uppercase; letter-spacing: 0.08em; color: var(--muted); font-weight: 600; margin: 0 0 6px; }
.meter { display: grid; grid-template-columns: minmax(0, 11em) minmax(40px, 1fr) 3.2em; gap: 10px; align-items: center; margin: 5px 0; }
.name small { color: var(--muted); display: block; font-size: 0.75rem; }
.bar { height: 10px; border-radius: 999px; background: var(--surface-2); border: 1px solid var(--line); overflow: hidden; }
.bar i { display: block; height: 100%; background: var(--text-2); border-radius: 999px; }
.val { text-align: right; font-variant-numeric: tabular-nums; font-weight: 600; }
dl { display: grid; grid-template-columns: auto minmax(0, 1fr); gap: 6px 14px; align-content: start; }
dt { color: var(--muted); }
dd { margin: 0; font-weight: 600; font-variant-numeric: tabular-nums; }
dd small { font-weight: 400; color: var(--muted); }
.note { font-size: 0.85rem; margin: 10px 0 0; }
@media (max-width: 480px) {
  .meter { grid-template-columns: minmax(0, 6.5em) minmax(40px, 1fr) 3.2em; }
}
</style>
