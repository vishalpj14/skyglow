<script setup lang="ts">
// What the sky is doing at the scrubber's time, in words. Same wording as
// the CLI's `--at`, because both read the same SkyState from the core.
import { computed } from "vue";
import type { Pack, SkyState } from "../lib/types";
import { describeLayer, describeSun, heightText } from "../lib/describe";
import { layerCell, skyCell } from "../lib/strip";

const props = defineProps<{ s: SkyState; pack: Pack }>();
const sky = computed(() => skyCell(props.s, props.pack));
const stageText = computed(() => {
  if (props.s.stage == null) return "";
  const st = props.pack.stages[props.s.stage];
  return props.s.rising && st.morning_description ? st.morning_description : st.description;
});
</script>

<template>
  <div class="readout" aria-live="polite">
    <p><span class="swatch" :style="{ background: sky.colour }"></span><strong>{{ sky.label.replace(/_/g, " ") }}</strong>. {{ stageText }}</p>
    <p v-for="line in describeSun(s)" :key="line">{{ line }}</p>
    <table v-if="s.elevation_deg < 0">
      <tbody>
        <tr v-for="(l, i) in s.layers" :key="l.name">
          <td class="num"><span class="swatch" :style="{ background: layerCell(s, i).colour }"></span>{{ l.name }} {{ heightText(l) }}</td>
          <td>{{ describeLayer(l) }}</td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<style scoped>
.readout p { margin-bottom: 4px; }
table { margin-top: 6px; }
</style>
