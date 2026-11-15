<script setup lang="ts">
// The mountain skyline, in the same two forms as place files (docs/PLACES.md):
// horizon points ("toward 250 degrees the skyline is 12 degrees high") and
// peaks read off a map (bearing, distance, height).
import type { Place } from "../lib/types";

const place = defineModel<Place>({ required: true });

function update(fn: (p: Place) => void) {
  const copy: Place = { ...place.value, horizon: place.value.horizon.map((h) => ({ ...h })), peaks: place.value.peaks.map((k) => ({ ...k })) };
  fn(copy);
  place.value = copy;
}
function val(e: Event): number {
  return Number((e.target as HTMLInputElement).value);
}
</script>

<template>
  <details class="skyline">
    <summary>Skyline ({{ place.horizon.length }} horizon points, {{ place.peaks.length }} peaks)</summary>
    <p class="muted">Mountains toward the sun hide it before the true horizon, while high clouds keep their light. Bearings are degrees from north (90 east, 270 west).</p>

    <h3>Horizon points</h3>
    <table v-if="place.horizon.length">
      <thead><tr><th>Bearing (&deg;)</th><th>Skyline height (&deg;)</th><th></th></tr></thead>
      <tbody>
        <tr v-for="(h, i) in place.horizon" :key="'h' + i">
          <td><input type="number" step="1" min="0" max="360" :value="h.az" @change="update((p) => (p.horizon[i].az = val($event)))" /></td>
          <td><input type="number" step="0.1" min="-5" max="60" :value="h.elev" @change="update((p) => (p.horizon[i].elev = val($event)))" /></td>
          <td><button type="button" aria-label="Remove point" @click="update((p) => p.horizon.splice(i, 1))">&times;</button></td>
        </tr>
      </tbody>
    </table>
    <button type="button" @click="update((p) => p.horizon.push({ az: 270, elev: 5 }))">Add horizon point</button>

    <h3>Peaks</h3>
    <table v-if="place.peaks.length">
      <thead><tr><th>Name</th><th>Bearing (&deg;)</th><th>Distance (km)</th><th>Height (m)</th><th></th></tr></thead>
      <tbody>
        <tr v-for="(k, i) in place.peaks" :key="'k' + i">
          <td><input type="text" :value="k.name" @change="update((p) => (p.peaks[i].name = ($event.target as HTMLInputElement).value))" /></td>
          <td><input type="number" step="1" min="0" max="360" :value="k.az" @change="update((p) => (p.peaks[i].az = val($event)))" /></td>
          <td><input type="number" step="0.1" min="0.1" :value="k.distance_km" @change="update((p) => (p.peaks[i].distance_km = val($event)))" /></td>
          <td><input type="number" step="10" min="0" :value="k.height_m" @change="update((p) => (p.peaks[i].height_m = val($event)))" /></td>
          <td><button type="button" aria-label="Remove peak" @click="update((p) => p.peaks.splice(i, 1))">&times;</button></td>
        </tr>
      </tbody>
    </table>
    <button type="button" @click="update((p) => p.peaks.push({ name: 'Peak', az: 90, distance_km: 20, height_m: 1500 }))">Add peak</button>
  </details>
</template>

<style scoped>
.skyline { margin: 10px 0; }
summary { cursor: pointer; color: var(--text-2); }
h3 { margin-top: 10px; }
table { margin-bottom: 6px; }
td input { min-width: 60px; }
td:last-child { width: 1%; }
.skyline :deep(table) { display: block; overflow-x: auto; }
</style>
