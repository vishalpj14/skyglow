// Saved places and settings, shared by the whole app. Kept in this
// browser's storage only; never sent anywhere.
import { ref, watch } from "vue";
import type { ClockChoice } from "./clock";
import type { Place } from "./types";
import { keyFor, loadSaved, saveAll, type NamedPlace } from "./places";

export const savedPlaces = ref<NamedPlace[]>(loadSaved());

export function findSaved(place: Place): NamedPlace | undefined {
  return savedPlaces.value.find((s) => s.key === keyFor(place.name));
}

/** Save (or update) a place, keeping its clock choice. Returns false if storage is blocked. */
export function savePlace(place: Place, clock?: ClockChoice): boolean {
  const key = keyFor(place.name);
  const keep = clock ?? savedPlaces.value.find((s) => s.key === key)?.clock;
  savedPlaces.value = [...savedPlaces.value.filter((s) => s.key !== key), { key, place: { ...place }, saved: true, clock: keep }];
  return saveAll(savedPlaces.value);
}

export function forgetPlace(place: Place): void {
  const key = keyFor(place.name);
  savedPlaces.value = savedPlaces.value.filter((s) => s.key !== key);
  saveAll(savedPlaces.value);
}

/** Remember a clock choice for a saved place (no-op for unsaved places). */
export function rememberClock(place: Place, clock: ClockChoice): boolean {
  const s = findSaved(place);
  if (!s) return false;
  return savePlace(s.place, clock);
}

// ---- online extras switch --------------------------------------------------

const ONLINE = "skyglow.onlineExtras.v1";
function readOnline(): boolean {
  try {
    return localStorage.getItem(ONLINE) !== "off";
  } catch {
    return true;
  }
}
/**
 * On unless the user switched it off. It sends picked places' rounded
 * coordinates to outside services, so the app says so in a notice the first
 * time (see OnlineNotice.vue), with a one-tap way to turn it off.
 */
export const onlineExtras = ref(readOnline());

const NOTICE = "skyglow.onlineNoticeSeen.v1";
function readSeen(): boolean {
  try {
    return localStorage.getItem(NOTICE) === "yes";
  } catch {
    return false;
  }
}
/** Has the user dismissed the first-run online-extras notice? */
export const onlineNoticeSeen = ref(readSeen());
watch(onlineNoticeSeen, (seen) => {
  try {
    localStorage.setItem(NOTICE, seen ? "yes" : "no");
  } catch {
    /* storage blocked: the notice may show again next time */
  }
});
watch(onlineExtras, (on) => {
  try {
    localStorage.setItem(ONLINE, on ? "on" : "off");
  } catch {
    /* storage blocked: the switch just will not be remembered */
  }
});
