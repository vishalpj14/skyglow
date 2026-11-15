<script setup lang="ts">
// Click a map to set the place. Online only: map tiles come from
// OpenStreetMap and are not cached for offline use. Leaflet is loaded only
// when this panel is opened, so the rest of the app stays small.
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import type { Map as LeafletMap, CircleMarker } from "leaflet";
import "leaflet/dist/leaflet.css";
import { round } from "../lib/geo";

const props = defineProps<{ lat: number; lon: number }>();
const emit = defineEmits<{ pick: [lat: number, lon: number] }>();

const el = ref<HTMLDivElement>();
const offline = ref(!navigator.onLine);
let map: LeafletMap | null = null;
let marker: CircleMarker | null = null;

onMounted(async () => {
  const L = await import("leaflet");
  if (!el.value) return;
  map = L.map(el.value, { worldCopyJump: true }).setView([props.lat, props.lon], 8);
  L.tileLayer("https://tile.openstreetmap.org/{z}/{x}/{y}.png", {
    maxZoom: 18,
    attribution: '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors',
  }).addTo(map);
  marker = L.circleMarker([props.lat, props.lon], { radius: 7, weight: 2 }).addTo(map);
  map.on("click", (e) => {
    const lon = ((((e.latlng.lng + 180) % 360) + 360) % 360) - 180;
    emit("pick", round(e.latlng.lat, 4), round(lon, 4));
  });
});

watch(
  () => [props.lat, props.lon] as const,
  ([lat, lon]) => marker?.setLatLng([lat, lon]),
);

onBeforeUnmount(() => map?.remove());
</script>

<template>
  <div>
    <p v-if="offline" class="muted">You are offline, so the map cannot load. Type coordinates or pick a bundled city instead.</p>
    <div ref="el" class="map" role="application" aria-label="Map: click to set the place"></div>
    <small>Click to set the place. Map tiles load from OpenStreetMap.</small>
  </div>
</template>

<style scoped>
.map { height: 260px; border-radius: 8px; margin: 6px 0; border: 1px solid var(--line); }
</style>
