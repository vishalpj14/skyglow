import { createApp } from "vue";
import App from "./App.vue";
import "./style.css";
import { loadEngine } from "./lib/engine";
import { interceptLinks } from "./lib/transition";

const root = document.getElementById("app")!;
interceptLinks();
loadEngine()
  .then(() => createApp(App).mount(root))
  .catch((e: unknown) => {
    root.textContent = `The sky engine could not start: ${e instanceof Error ? e.message : String(e)}`;
  });
