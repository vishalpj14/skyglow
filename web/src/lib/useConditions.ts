// The forecast for the place and date on screen, shared by the Conditions
// card, the Glow card and the colour strips. Loads when the place or date
// changes (after a short pause, so typing coordinates sends one request), and
// again when the connection or the online-extras switch comes back.
import { computed, onBeforeUnmount, ref, watch, type Ref } from "vue";
import type { LocalDate, Place, WeatherPlan } from "./types";
import { cacheKey, loadConditions, type StoredConditions } from "./conditions";
import { isOnline } from "./network";
import { onlineExtras } from "./saved";

export function useConditions(place: Ref<Place>, date: Ref<LocalDate>, plan: Ref<WeatherPlan | null>) {
  const stored = ref<StoredConditions | null>(null);
  const from = ref<"network" | "saved" | "none">("none");
  const problem = ref("");
  const busy = ref(false);
  const mayFetch = computed(() => isOnline.value && onlineExtras.value);
  const key = computed(() => cacheKey(place.value.lat, place.value.lon, date.value));

  // Only the newest request may write its answer (places can change quickly).
  let latest = 0;
  async function load(force = false) {
    const mine = ++latest;
    busy.value = true;
    const r = await loadConditions(place.value.lat, place.value.lon, date.value, { online: mayFetch.value, force, plan: plan.value });
    if (mine !== latest) return;
    stored.value = r.stored;
    from.value = r.from;
    problem.value = r.problem ?? "";
    busy.value = false;
  }

  let timer: number | undefined;
  watch(
    [key, mayFetch],
    ([k], old) => {
      window.clearTimeout(timer);
      if (old && k !== old[0]) stored.value = null;
      timer = window.setTimeout(() => load(), old ? 500 : 0);
    },
    { immediate: true },
  );
  onBeforeUnmount(() => window.clearTimeout(timer));

  /** The series as JSON text for the engine; "" when there is none. */
  const seriesText = computed(() => (stored.value ? JSON.stringify(stored.value.series) : ""));

  /** Everything (here and along the sun's direction) as JSON text for the glow; "" when there is none. */
  const weatherText = computed(() => (stored.value ? JSON.stringify(stored.value.weather) : ""));

  return { stored, from, problem, busy, mayFetch, load, seriesText, weatherText };
}

export type ConditionsState = ReturnType<typeof useConditions>;
