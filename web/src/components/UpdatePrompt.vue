<script setup lang="ts">
// Offline support. The service worker keeps a copy of the whole app (engine
// included) so it opens with no network. When a newer version is published,
// it asks before switching, instead of changing under the user.
import { useRegisterSW } from "virtual:pwa-register/vue";

const { offlineReady, needRefresh, updateServiceWorker } = useRegisterSW();
function close() {
  offlineReady.value = false;
  needRefresh.value = false;
}
</script>

<template>
  <div v-if="offlineReady || needRefresh" class="toast card" role="status">
    <span v-if="needRefresh">A newer version of skyglow is ready.</span>
    <span v-else>skyglow is saved on this device and now works offline.</span>
    <button v-if="needRefresh" type="button" class="primary" @click="updateServiceWorker(true)">Reload</button>
    <button type="button" @click="close">Close</button>
  </div>
</template>

<style scoped>
.toast { position: fixed; right: 16px; bottom: 16px; left: 16px; max-width: 420px; margin-left: auto; display: flex; gap: 8px; align-items: center; flex-wrap: wrap; z-index: 10; }
.toast span { flex: 1 1 200px; }
</style>
