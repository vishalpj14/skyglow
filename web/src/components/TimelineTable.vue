<script setup lang="ts">
// One sunrise or sunset as a table: the same numbers as the CLI report.
// This is also the text alternative for the charts.
import { computed } from "vue";
import type { Pack, Timeline } from "../lib/types";
import { formatClock } from "../lib/engine";

const props = defineProps<{ t: Timeline; pack: Pack }>();
const emit = defineEmits<{ seek: [jd: number] }>();

const rising = computed(() => props.t.event === "sunrise");
const clock = (jd: number | null) => (jd == null ? "--:--" : formatClock(jd, props.t.tz_hours).slice(0, 5));
const mins = (a: number | null, b: number | null) => (a == null || b == null ? "" : `${Math.round((b - a) * 1440)} min`);
const stages = computed(() => (rising.value ? [...props.t.stages].reverse() : props.t.stages));
const swatch = (name: string) => props.pack.stages.find((s) => s.name === name || s.morning_name === name)?.swatch || "transparent";
const terrainMin = computed(() => {
  const { event_jd: a, sea_event_jd: b } = props.t;
  if (a == null || b == null) return 0;
  return Math.round((rising.value ? a - b : b - a) * 1440);
});
</script>

<template>
  <section class="card">
    <h2>{{ rising ? "Sunrise" : "Sunset" }}</h2>
    <p v-if="t.event_jd != null">
      <button type="button" class="linkish" @click="emit('seek', t.event_jd!)">
        <strong>{{ clock(t.event_jd) }}</strong>
      </button>
      toward azimuth {{ t.event_azimuth_deg?.toFixed(0) }}&deg;
      <template v-if="t.has_skyline && terrainMin >= 1">
        &middot; terrain {{ rising ? "hides the sun for" : "hides the sun" }} {{ terrainMin }} min {{ rising ? "after the flat-horizon sunrise" : "before the flat-horizon sunset" }}
      </template>
    </p>
    <p v-else class="muted">None today (polar day or night).</p>
    <p v-if="t.elevation_m > 0" class="muted small">
      From {{ t.elevation_m.toFixed(0) }} m the sea horizon dips {{ t.dip_deg.toFixed(2) }}&deg;. Sea-level, flat-horizon time: {{ clock(t.sea_level_event_jd) }}.
    </p>

    <h3>Light stages</h3>
    <table>
      <thead><tr><th>Stage</th><th>From</th><th>To</th><th>Length</th></tr></thead>
      <tbody>
        <tr v-for="s in stages" :key="s.name" :title="s.description">
          <td><span class="swatch" :style="{ background: swatch(s.name) }"></span>{{ s.name.replace(/_/g, " ") }}</td>
          <td class="num"><button v-if="s.start_jd != null" type="button" class="linkish" @click="emit('seek', s.start_jd!)">{{ clock(s.start_jd) }}</button><span v-else>--:--</span></td>
          <td class="num">{{ clock(s.end_jd) }}</td>
          <td class="num">{{ mins(s.start_jd, s.end_jd) }}</td>
        </tr>
      </tbody>
    </table>

    <h3>Cloud layers overhead</h3>
    <table>
      <thead><tr><th>Layer</th><th>{{ rising ? "First lit" : "Goes dark" }}</th><th>Sun below horizon</th></tr></thead>
      <tbody>
        <tr v-for="l in t.layers" :key="l.name">
          <td>{{ l.name }} {{ l.height_km }} km</td>
          <td class="num">{{ clock(l.overhead_shadow_jd) }}</td>
          <td class="num">{{ l.shadow_depression_deg.toFixed(2) }}&deg;</td>
        </tr>
      </tbody>
    </table>
    <p class="muted small">Clouds toward the sun are lit {{ rising ? "earlier" : "longer" }} than these overhead times.</p>
  </section>
</template>

<style scoped>
h3 { margin-top: 12px; }
.small { font-size: 0.85rem; }
.linkish { border: none; background: none; padding: 0; color: var(--accent); text-decoration: underline; font: inherit; }
</style>
