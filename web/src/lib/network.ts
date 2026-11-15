// Is the device online? Follows the browser's own online/offline events, so
// every part of the app can react (pause online extras, show the footer status).
import { ref } from "vue";

export const isOnline = ref(typeof navigator === "undefined" ? true : navigator.onLine);

if (typeof window !== "undefined") {
  window.addEventListener("online", () => (isOnline.value = true));
  window.addEventListener("offline", () => (isOnline.value = false));
}
