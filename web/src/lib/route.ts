// A tiny hash router: "#/docs/colours" shows a page; anything else shows the
// sky tool. Hash routes work on GitHub Pages with no server setup.
import { ref } from "vue";

export type Route = { name: "sky" } | { name: "doc"; slug: string };

export function parseRoute(hash: string): Route {
  const m = /^#\/docs\/([a-z0-9-]+)\/?$/.exec(hash);
  return m ? { name: "doc", slug: m[1] } : { name: "sky" };
}

export const route = ref<Route>(typeof location === "undefined" ? { name: "sky" } : parseRoute(location.hash));

if (typeof window !== "undefined") {
  // Back/forward buttons and typed addresses (no animation for these).
  const sync = () => {
    const next = parseRoute(location.hash);
    if (JSON.stringify(next) !== JSON.stringify(route.value)) {
      route.value = next;
      window.scrollTo({ top: 0 });
    }
  };
  window.addEventListener("hashchange", sync);
  window.addEventListener("popstate", sync);
}
