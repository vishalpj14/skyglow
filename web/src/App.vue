<script setup lang="ts">
// The whole app: pick a place, date and pack; see sunrise and sunset
// timelines, the sun's path, colour strips, and drag the time scrubber to
// read the sky at any minute. All numbers come from the Rust core.
import { computed, onBeforeUnmount, ref, watch } from "vue";
import type { LocalDate, Pack, Place, SkyState, Timeline } from "./lib/types";
import type { NamedPlace } from "./lib/places";
import * as engine from "./lib/engine";
import { browserTzHours, compareZones, deviceZoneName, formatDate, hhmm, offsetForZone, parseDate, resolveClock, todayAt, utcLabel, type ClockChoice } from "./lib/clock";
import { findSaved, rememberClock } from "./lib/saved";
import { examplePlaces, normalisePlace } from "./lib/places";
import { bundledPacks } from "./lib/packs";
import PlacePanel from "./components/PlacePanel.vue";
import SunPath from "./components/SunPath.vue";
import ColourStrip from "./components/ColourStrip.vue";
import TimelineTable from "./components/TimelineTable.vue";
import SkyReadout from "./components/SkyReadout.vue";
import UpdatePrompt from "./components/UpdatePrompt.vue";
import AppFooter from "./components/AppFooter.vue";
import AppHeader from "./components/AppHeader.vue";
import DocPage from "./components/DocPage.vue";
import IrisOverlay from "./components/IrisOverlay.vue";
import { route } from "./lib/route";
import { skyMood } from "./lib/mood";
import OnlineNotice from "./components/OnlineNotice.vue";

// ---- remembered settings (this browser only; may be unavailable) ----------
const LAST = "skyglow.last.v1";
interface Last { place: Place; packKey: string; useDst: boolean }
function readLast(): Partial<Last> {
  try {
    return JSON.parse(localStorage.getItem(LAST) ?? "{}");
  } catch {
    return {};
  }
}
const last = readLast();
const fallback = examplePlaces.find((p) => p.key === "kochi") ?? examplePlaces[0];

// ---- inputs ----------------------------------------------------------------
const place = ref<Place>(last.place ? normalisePlace(last.place) : fallback.place);
const useDst = ref(!!last.useDst);
const packKey = ref(last.packKey && bundledPacks.some((p) => p.key === last.packKey) ? last.packKey : "default");
const customPack = ref<{ name: string; text: string } | null>(null);
const packError = ref("");

// Times are shown on this device's clock unless the user chose otherwise
// for a saved place (or for this session).
const clockChoice = ref<ClockChoice>(findSaved(place.value)?.clock ?? { mode: "device" });
const clockNote = ref("");
/** Today on this device's own clock. */
const deviceToday = () => {
  const n = new Date();
  return { year: n.getFullYear(), month: n.getMonth() + 1, day: n.getDate() };
};
const dateText = ref(formatDate(deviceToday()));
const date = computed<LocalDate>(() => parseDate(dateText.value) ?? deviceToday());
const clock = computed(() => resolveClock(clockChoice.value, place.value, useDst.value, date.value));
const tz = computed(() => clock.value.tz);

function onChosen(p: NamedPlace) {
  clockChoice.value = (p.saved ? findSaved(p.place)?.clock : undefined) ?? { mode: "device" };
  clockNote.value = "";
}
function setClockMode(mode: ClockChoice["mode"]) {
  setClock(mode === "custom" ? { mode, tz: tz.value } : { mode });
}
function setClock(choice: ClockChoice) {
  clockChoice.value = choice;
  if (rememberClock(place.value, choice)) clockNote.value = `Remembered for ${place.value.name}.`;
  else if (choice.mode === "device") clockNote.value = "";
  else clockNote.value = "Save this place to keep this clock choice; other places use this device's clock.";
}
/** What "the place's local time" means here, so the user sees it before choosing. */
const placeClockLabel = computed(() => {
  const p = place.value;
  if (p.zone) return `${p.zone} (${utcLabel(offsetForZone(p.zone, date.value) ?? 0)})`;
  if (p.tz != null) return p.tz_name || utcLabel(p.tz);
  return "unknown, guessed from longitude";
});
const deviceClockLabel = computed(() => `${deviceZoneName() || "this device"} (${utcLabel(browserTzHours(date.value))})`);

const packText = computed(() =>
  packKey.value === "custom" && customPack.value ? customPack.value.text : bundledPacks.find((p) => p.key === packKey.value)?.text ?? "",
);
const pack = computed<Pack>(() => engine.packInfo(packText.value));

watch([place, packKey, useDst], () => {
  try {
    localStorage.setItem(LAST, JSON.stringify({ place: place.value, packKey: packKey.value === "custom" ? "default" : packKey.value, useDst: useDst.value }));
  } catch {
    /* storage blocked: nothing to remember, which is fine */
  }
});

async function loadPackFile(ev: Event) {
  const file = (ev.target as HTMLInputElement).files?.[0];
  if (!file) return;
  const text = await file.text();
  try {
    engine.packInfo(text); // validates; throws with the core's message
    customPack.value = { name: file.name, text };
    packKey.value = "custom";
    packError.value = "";
  } catch (e) {
    packError.value = `That pack was not loaded: ${e instanceof Error ? e.message : String(e)}`;
  }
}

// ---- the engine's answers ---------------------------------------------------
const error = ref("");
function safe<T>(fn: () => T, fallbackValue: T): T {
  try {
    return fn();
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
    return fallbackValue;
  }
}

const dayStart = computed(() => engine.jdFromLocal(date.value, 0, tz.value));
const timelines = computed<Timeline[]>(() => {
  error.value = "";
  return safe(
    () => (["sunrise", "sunset"] as const).map((ev) => engine.timeline(place.value, packText.value, date.value, tz.value, ev)),
    [],
  );
});
const daySeries = computed<SkyState[]>(() => safe(() => engine.skySeries(place.value, packText.value, dayStart.value, dayStart.value + 1, 4), []));

/** Minute-by-minute window around an event, for its colour strip. */
function windowAround(t: Timeline | undefined, before: number, after: number): SkyState[] {
  const jd = t?.event_jd ?? t?.sea_event_jd;
  if (jd == null) return [];
  return safe(() => engine.skySeries(place.value, packText.value, jd - before / 1440, jd + after / 1440, 1), []);
}
const sunriseStrip = computed(() => windowAround(timelines.value[0], 60, 40));
const sunsetStrip = computed(() => windowAround(timelines.value[1], 40, 60));

// ---- the time scrubber -----------------------------------------------------
const minute = ref(0);
const cursorJd = computed(() => dayStart.value + minute.value / 1440);
/** The same instant on the place's clock, UTC and yours, when the place keeps a different time. */
const zoneRows = computed(() => compareZones(place.value, useDst.value, date.value, cursorJd.value, tz.value));
const skyNow = computed<SkyState | null>(() => safe(() => engine.skyAt(place.value, packText.value, cursorJd.value), null));

function seek(jd: number) {
  minute.value = Math.min(1439, Math.max(0, Math.round((jd - dayStart.value) * 1440)));
}
function seekEvent(i: number, offsetMin = 0) {
  const jd = timelines.value[i]?.event_jd;
  if (jd != null) seek(jd + offsetMin / 1440);
}
// Switching only the clock keeps the same instant on screen.
watch(tz, (now, before) => {
  minute.value = Math.min(1439, Math.max(0, Math.round(minute.value + (now - before) * 60)));
});

const isToday = computed(() => formatDate(todayAt(tz.value)) === formatDate(date.value));
// The logo's hover animation follows the scrubber: a sunset in the evening,
// a sunrise in the morning, in this pack's colours.
watch(
  [skyNow, pack],
  ([s, p]) => {
    skyMood.value = { rising: s?.rising ?? false, swatches: p.stages.map((st) => st.swatch) };
  },
  { immediate: true },
);

/** Julian Day of this instant: Unix time in days plus the Julian Day of 1970-01-01. */
function seekNow() {
  seek(Date.now() / 86_400_000 + 2_440_587.5);
}
// Open on the time the device's clock shows right now. Picking another place
// or date keeps the same clock time; the buttons jump to sunrise or sunset.
seekNow();

watch(route, (r) => {
  if (r.name === "sky") document.title = "skyglow";
});

const playing = ref(false);
let timer: number | undefined;
function togglePlay() {
  playing.value = !playing.value;
  window.clearInterval(timer);
  if (playing.value) {
    timer = window.setInterval(() => {
      minute.value = (minute.value + 1) % 1440;
    }, 80);
  }
}
onBeforeUnmount(() => window.clearInterval(timer));

</script>

<template>
  <AppHeader />

  <main v-if="route.name === 'doc'">
    <DocPage :slug="route.slug" />
  </main>

  <!-- v-show, not v-if: the sky keeps its place, date and scrubber while you read a page. -->
  <main v-show="route.name === 'sky'" class="layout">
    <h1 class="visually-hidden">skyglow: sunset and twilight light for any place</h1>
    <div class="side">
      <PlacePanel v-model="place" @chosen="onChosen" />

      <section class="card">
        <h2>Date, clock and pack</h2>
        <div class="row">
          <label>Date<input v-model="dateText" type="date" /></label>
        </div>
        <label>
          Show times on
          <select :value="clockChoice.mode" @change="setClockMode(($event.target as HTMLSelectElement).value as ClockChoice['mode'])">
            <option value="device">This device's clock: {{ deviceClockLabel }}</option>
            <option value="place">The place's local time: {{ placeClockLabel }}</option>
            <option value="custom">A fixed UTC offset</option>
          </select>
        </label>
        <label v-if="clockChoice.mode === 'custom'">
          UTC offset (hours)
          <input type="number" step="0.25" min="-14" max="14" :value="clockChoice.tz" @change="setClock({ mode: 'custom', tz: Number(($event.target as HTMLInputElement).value) || 0 })" />
        </label>
        <p class="muted small">
          Times shown in <strong>{{ clock.label }}</strong>.
          <template v-if="clock.guessed">This place's time zone is not known: switch on online extras, or type its offset in the place panel.</template>
          {{ clockNote }}
        </p>
        <label v-if="clockChoice.mode === 'place' && !place.zone && place.dst_tz != null" class="check">
          <input v-model="useDst" type="checkbox" /> Daylight saving time ({{ utcLabel(place.dst_tz) }}<template v-if="place.dst_note">: {{ place.dst_note }}</template>)
        </label>
        <div class="row">
          <label>
            Parameter pack
            <select v-model="packKey">
              <option v-for="p in bundledPacks" :key="p.key" :value="p.key">{{ p.key }}</option>
              <option v-if="customPack" value="custom">{{ customPack.name }} (your file)</option>
            </select>
          </label>
          <label>Load a pack file<input type="file" accept=".json,application/json" @change="loadPackFile" /></label>
        </div>
        <p v-if="packError" class="error">{{ packError }}</p>
        <p class="muted small">{{ pack.description }}</p>
      </section>
    </div>

    <div class="mainview">
      <p v-if="error" class="error card" role="alert">{{ error }}</p>

      <section class="card scrubber">
        <h2>Time scrubber</h2>
        <div class="scrub-row">
          <output class="time">{{ hhmm(minute / 60) }}</output>
          <input v-model.number="minute" type="range" min="0" max="1439" step="1" aria-label="Time of day" />
        </div>
        <ul v-if="zoneRows" class="zones" aria-label="This moment on other clocks">
          <li v-for="r in zoneRows" :key="r.who" :class="{ shown: r.shown }">
            <span class="who">{{ r.label }}</span>
            <span class="t">{{ r.time }}<small v-if="r.dayShift" class="shift">{{ r.dayShift > 0 ? "+" : "−" }}{{ Math.abs(r.dayShift) }} day</small></span>
            <small class="zone">{{ r.zone }}</small>
          </li>
        </ul>
        <div class="row buttons">
          <button type="button" @click="seekEvent(0)">Sunrise</button>
          <button type="button" @click="seekEvent(1)">Sunset</button>
          <button type="button" @click="minute = Math.max(0, minute - 1)" aria-label="One minute earlier">&minus;1 min</button>
          <button type="button" @click="minute = Math.min(1439, minute + 1)" aria-label="One minute later">+1 min</button>
          <button v-if="isToday" type="button" @click="seekNow">Now</button>
          <button type="button" class="primary" @click="togglePlay">{{ playing ? "Pause" : "Play" }}</button>
        </div>
        <SkyReadout v-if="skyNow" :s="skyNow" :pack="pack" />
      </section>

      <SunPath :series="daySeries" :pack="pack" :tz="tz" :cursor-jd="cursorJd" :day-start-jd="dayStart" @seek="seek" />

      <div class="pair">
        <ColourStrip title="Around sunrise" :series="sunriseStrip" :pack="pack" :tz="tz" :cursor-jd="cursorJd" @seek="seek" />
        <ColourStrip title="Around sunset" :series="sunsetStrip" :pack="pack" :tz="tz" :cursor-jd="cursorJd" @seek="seek" />
      </div>

      <div class="pair">
        <TimelineTable v-for="t in timelines" :key="t.event" :t="t" :pack="pack" @seek="seek" />
      </div>
    </div>
  </main>

  <AppFooter />
  <IrisOverlay />
  <OnlineNotice />
  <UpdatePrompt />
</template>

<style scoped>
.visually-hidden { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); white-space: nowrap; }
.layout { display: grid; gap: 12px; padding: 12px 16px; max-width: 1280px; margin: 0 auto; grid-template-columns: minmax(0, 1fr); }
.side, .mainview { display: grid; gap: 12px; align-content: start; min-width: 0; }
.pair { display: grid; gap: 12px; grid-template-columns: minmax(0, 1fr); }
@media (min-width: 960px) {
  .layout { grid-template-columns: minmax(300px, 360px) minmax(0, 1fr); }
}
@media (min-width: 1200px) {
  .pair { grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); }
}
.small { font-size: 0.85rem; }
.check { display: flex; gap: 6px; align-items: center; margin: 6px 0; }
.scrub-row { display: flex; gap: 12px; align-items: center; }
.scrub-row input { flex: 1; min-width: 0; }
.time { font-size: 1.6rem; font-variant-numeric: tabular-nums; font-weight: 600; min-width: 4.5ch; }
.zones { list-style: none; padding: 0; margin: 10px 0 2px; display: flex; flex-wrap: wrap; gap: 8px; }
.zones li {
  display: grid; gap: 1px; padding: 6px 12px; border-radius: 12px; border: 1px solid var(--line);
  background: var(--surface-2); min-width: 0; flex: 1 1 140px;
}
.zones li.shown { border-color: var(--accent); }
.zones .who { font-size: 0.72rem; text-transform: uppercase; letter-spacing: 0.08em; color: var(--muted); font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.zones .t { font-size: 1.15rem; font-weight: 700; font-variant-numeric: tabular-nums; }
.zones .shift { font-size: 0.72rem; font-weight: 600; color: var(--text-2); margin-left: 6px; }
.zones .zone { color: var(--muted); font-size: 0.75rem; }
.buttons { margin: 8px 0 12px; }
.buttons button { flex: 0 0 auto; }
</style>
