<script setup lang="ts">
// Where are you? Pick a bundled city or a saved place, use GPS, type
// coordinates, search online, or click a map. With online extras on, the
// time zone, ground height and place name are filled in automatically.
import { computed, defineAsyncComponent, onBeforeUnmount, ref, watch } from "vue";
import type { Place } from "../lib/types";
import { examplePlaces, normalisePlace, searchPlaces, type NamedPlace } from "../lib/places";
import { gpsPosition, searchOnline, type SearchHit } from "../lib/geo";
import { lookupDetails } from "../lib/online";
import { findSaved, forgetPlace, onlineExtras, savedPlaces, savePlace } from "../lib/saved";
import { isOnline } from "../lib/network";
import SkylineEditor from "./SkylineEditor.vue";

const MapPicker = defineAsyncComponent(() => import("./MapPicker.vue"));

const place = defineModel<Place>({ required: true });
const emit = defineEmits<{ chosen: [p: NamedPlace] }>();

const query = ref("");
const status = ref("");
const busy = ref(false);
const online = ref<SearchHit[]>([]);
const showMap = ref(false);
/** Where the current height came from, shown under the field. */
const heightSource = ref("");

const extrasActive = computed(() => onlineExtras.value && isOnline.value);
const matches = computed(() => searchPlaces([...savedPlaces.value, ...examplePlaces], query.value).slice(0, 8));

function choose(p: NamedPlace) {
  place.value = normalisePlace(p.place, p.key);
  heightSource.value = "";
  query.value = "";
  status.value = "";
  emit("chosen", p);
}

/**
 * A new place from bare coordinates. Fill in what we can: the phone's GPS
 * altitude offline, or (with online extras) the time zone, terrain height
 * and a proper name.
 */
async function setCoords(lat: number, lon: number, name: string, gpsAltitude: number | null = null, longName = "") {
  place.value = normalisePlace({ name, lat, lon, elevation_m: gpsAltitude ?? 0 });
  heightSource.value = gpsAltitude != null ? "from your phone's GPS (rough)" : "unknown: set it if you are high up";
  emit("chosen", { key: "", place: place.value, saved: false });
  if (!extrasActive.value) {
    status.value = isOnline.value
      ? "Tip: switch on online extras to fill in the time zone, ground height and place name automatically."
      : "";
    return;
  }
  // Pass the coordinates explicitly: right after assigning a v-model, reading
  // `place.value` still gives the old place until the parent re-renders.
  await fillDetails(lat, lon, longName);
}

async function fillDetails(lat: number, lon: number, longName = "") {
  busy.value = true;
  status.value = "Looking up time zone, ground height and name...";
  try {
    const d = await lookupDetails(lat, lon);
    // The user may have picked another place while we waited.
    if (place.value.lat !== lat || place.value.lon !== lon) {
      status.value = "";
      return;
    }
    const next = { ...place.value };
    const got: string[] = [];
    if (d.zone) {
      next.zone = d.zone;
      got.push(`time zone ${d.zone}`);
    }
    if (d.elevation_m != null) {
      next.elevation_m = d.elevation_m;
      heightSource.value = "ground height from terrain data (Open-Meteo, about 90 m detail)";
      got.push(`ground height ${d.elevation_m} m`);
    }
    if (d.name && ["My location", "Map point", TYPED].includes(next.name)) {
      next.name = d.name;
      got.push(`name ${d.name}`);
    }
    if (!next.note && (d.long_name || longName)) next.note = d.long_name || longName;
    place.value = next;
    status.value = got.length
      ? `Filled in: ${got.join(", ")}.`
      : "Online lookup did not answer in time; set the height by hand, or press “Fill in details online” to try again.";
  } catch (e) {
    status.value = `Online lookup failed: ${e instanceof Error ? e.message : String(e)}. Everything else still works.`;
  } finally {
    busy.value = false;
  }
}

async function useGps() {
  busy.value = true;
  status.value = "Asking the browser for your location...";
  try {
    const g = await gpsPosition();
    busy.value = false;
    await setCoords(g.lat, g.lon, "My location", g.elevation_m);
    if (!extrasActive.value) status.value = "Location set. It stays on this device. " + status.value;
  } catch (e) {
    status.value = e instanceof Error ? e.message : String(e);
  } finally {
    busy.value = false;
  }
}

// ---- search as you type ------------------------------------------------------
// Bundled and saved places match instantly. When online, a short pause in
// typing also searches OpenStreetMap (via Photon); results appear below.
const searching = ref(false);
const searchError = ref("");
let timer: number | undefined;
let inFlight: AbortController | null = null;

watch([query, isOnline], ([q, up]) => {
  window.clearTimeout(timer);
  inFlight?.abort();
  online.value = [];
  searchError.value = "";
  searching.value = false;
  if (!up || q.trim().length < 3) return;
  searching.value = true;
  timer = window.setTimeout(async () => {
    const ctrl = new AbortController();
    inFlight = ctrl;
    try {
      const hits = await searchOnline(q, ctrl.signal);
      if (!ctrl.signal.aborted) online.value = hits;
    } catch (e) {
      if (!ctrl.signal.aborted) searchError.value = "Online search is not answering right now; bundled and saved places still work.";
    } finally {
      if (!ctrl.signal.aborted) searching.value = false;
    }
  }, 400);
});
onBeforeUnmount(() => {
  window.clearTimeout(timer);
  inFlight?.abort();
});

function onEnter() {
  if (matches.value[0]) choose(matches.value[0]);
  else if (online.value[0]) pickOnline(online.value[0]);
}

function pickOnline(h: SearchHit) {
  query.value = "";
  setCoords(h.lat, h.lon, h.name, null, h.label);
}

function num(e: Event): number {
  return Number((e.target as HTMLInputElement).value);
}
/** Name given to a place made by typing coordinates; online extras replace it with a real name. */
const TYPED = "Typed coordinates";

function setField<K extends "lat" | "lon" | "elevation_m">(k: K, v: number) {
  if (!Number.isFinite(v)) return;
  if (k === "elevation_m") {
    place.value = { ...place.value, elevation_m: v };
    heightSource.value = "typed by you";
    return;
  }
  // A small nudge (under ~5 km) keeps the place. A bigger move is a new
  // place: the old name, note, skyline and clock no longer apply there.
  if (Math.abs(v - place.value[k]) < 0.05) {
    place.value = { ...place.value, [k]: v };
    return;
  }
  const next = { ...place.value, [k]: v };
  place.value = normalisePlace({ name: TYPED, lat: next.lat, lon: next.lon });
  heightSource.value = "unknown: set it if you are high up";
  emit("chosen", { key: "", place: place.value, saved: false });
  status.value = extrasActive.value
    ? "New coordinates: press “Fill in details online” below to get the time zone, height and name."
    : "New coordinates: started a fresh place (the old skyline and clock do not apply here).";
}
function setTz(text: string) {
  const v = text.trim() === "" ? null : Number(text);
  if (v === null || Number.isFinite(v)) place.value = { ...place.value, tz: v, tz_name: "" };
}

const isSaved = computed(() => !!findSaved(place.value));

function save() {
  status.value = savePlace(place.value) ? `Saved "${place.value.name}" on this device only.` : "This browser blocks storage, so the place could not be saved.";
}
function forget() {
  forgetPlace(place.value);
  status.value = `Removed "${place.value.name}" from this device.`;
}
</script>

<template>
  <section class="card place">
    <h2>Place</h2>

    <label>
      Find a place
      <input
        v-model="query"
        type="search"
        :placeholder="isOnline ? 'Any town, city, peak or viewpoint' : 'Bundled or saved place, e.g. Kochi'"
        autocomplete="off"
        @keydown.enter.prevent="onEnter"
        @keydown.esc="query = ''"
      />
    </label>
    <div v-if="query" class="results">
      <ul v-if="matches.length" class="hits">
        <li v-for="m in matches" :key="(m.saved ? 'saved:' : '') + m.key">
          <button type="button" @click="choose(m)">
            {{ m.place.name }} <small>{{ m.saved ? "saved" : "ready offline" }}</small>
          </button>
        </li>
      </ul>
      <ul v-if="online.length" class="hits">
        <li v-for="h in online" :key="h.lat + ',' + h.lon">
          <button type="button" @click="pickOnline(h)">{{ h.name }} <small>{{ h.label.slice(h.name.length + 2) }}</small></button>
        </li>
      </ul>
      <p class="muted hint">
        <template v-if="!isOnline">Offline: showing bundled and saved places only.</template>
        <template v-else-if="query.trim().length < 3">Keep typing to search everywhere.</template>
        <template v-else-if="searching">Searching...</template>
        <template v-else-if="searchError">{{ searchError }}</template>
        <template v-else-if="!online.length && !matches.length">No places found.</template>
        <template v-else-if="online.length">Search by Photon, &copy; OpenStreetMap contributors.</template>
      </p>
    </div>

    <div class="row actions">
      <button type="button" :disabled="busy" @click="useGps">Use my location (GPS)</button>
      <button v-if="isOnline" type="button" @click="showMap = !showMap">{{ showMap ? "Hide map" : "Pick on a map" }}</button>
    </div>
    <MapPicker v-if="showMap && isOnline" :lat="place.lat" :lon="place.lon" @pick="(lat: number, lon: number) => setCoords(lat, lon, 'Map point')" />
    <p v-if="status" class="status muted" role="status">{{ status }}</p>

    <h3>{{ place.name || "Unnamed place" }}</h3>
    <p v-if="place.note" class="muted note">{{ place.note }}</p>
    <div class="row">
      <label>Latitude (N +)<input type="number" step="0.0001" min="-90" max="90" :value="place.lat" @change="setField('lat', num($event))" /></label>
      <label>Longitude (E +)<input type="number" step="0.0001" min="-180" max="180" :value="place.lon" @change="setField('lon', num($event))" /></label>
    </div>
    <div class="row">
      <label>
        Height above sea level (m)
        <input type="number" step="1" min="0" :value="place.elevation_m" @change="setField('elevation_m', num($event))" />
        <small v-if="heightSource">{{ heightSource }}</small>
      </label>
      <label>
        Place's standard UTC offset (h)
        <input type="number" step="0.25" min="-14" max="14" :value="place.tz ?? ''" :placeholder="place.zone ? place.zone : 'unknown'" @change="setTz(($event.target as HTMLInputElement).value)" />
      </label>
    </div>
    <button v-if="extrasActive" type="button" class="fill" :disabled="busy" @click="fillDetails(place.lat, place.lon)">Fill in details online for these coordinates</button>

    <SkylineEditor v-model="place" />

    <div class="row actions">
      <button v-if="!isSaved" type="button" @click="save">Save on this device</button>
      <button v-else type="button" @click="forget">Forget this saved place</button>
    </div>
  </section>
</template>

<style scoped>
.head { display: flex; justify-content: space-between; align-items: baseline; gap: 8px; }
.switch { display: flex; gap: 6px; align-items: center; color: var(--text); }
.small { font-size: 0.8rem; }
.results { margin: 6px 0 10px; }
.hits { list-style: none; padding: 0; margin: 0 0 6px; display: grid; gap: 4px; }
.hits small { color: var(--muted); margin-left: 4px; }
.hint { font-size: 0.8rem; margin: 2px 0 0; }
.hits button { width: 100%; text-align: left; }
.actions { margin: 10px 0; }
.status { margin-top: 6px; }
h3 { margin-top: 12px; }
.note { font-size: 0.85rem; }
.row { margin-bottom: 8px; }
.fill { width: 100%; margin-bottom: 4px; }
label small { display: block; color: var(--muted); margin-top: 2px; }
</style>
