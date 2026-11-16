# web: Skyglow in your browser

A phone-friendly web app that runs the real Skyglow engine (the Rust code in `core/`, compiled to WebAssembly) on your own device. Once it has loaded once, it works with no internet connection, and you can install it to your home screen like an app.

## What you can do

- **Pick a place:** start typing any town, city, peak or viewpoint: bundled and saved places match instantly, and when you are online, places from all over the world appear as you type. Or use your phone's GPS, type coordinates, or click a map. Set your height above sea level and, if mountains are in the way, the skyline (horizon points or peaks, as in [docs/PLACES.md](../docs/PLACES.md)).
- **Pick a date and clock:** times are shown on **this device's own clock** (the same time your phone or computer shows, daylight saving included). You can switch to the place's local time or a fixed UTC offset; for a saved place, that choice is remembered.
- **Other clocks at a glance:** when the place keeps a different time from your device, the time scrubber shows the same moment on the place's clock, in UTC and on yours, and says when it is already tomorrow (or still yesterday) there. If the place's time zone is not known, it is guessed from longitude and marked “about”. The scrubber is **live** by default: it follows the current time minute by minute (and rolls the date over at midnight). Moving it yourself pauses that; tap **Live** to jump back to now and keep following.
- **Online extras (on by default; switch them off from the status pill at the top):** for places picked by GPS, map, search or typed coordinates, Skyglow fills in the place's **time zone** (so its local time follows daylight saving by itself), its **ground height** from a terrain model, and a proper **name**. Offline, GPS still gives a rough height from the phone, and a note appears when the connection drops or comes back.
- **Pick a pack:** the bundled `default` and `kerala` packs, or load your own pack file.
- **See:**
  - the **time scrubber**: drag through the whole day and read the sky at any minute, in the same words as the command line's `--at`;
  - the **sun's height through the day**, with the light stages as coloured bands (and your skyline, if you set one);
  - **colour strips** around sunrise and sunset: minute by minute, the sky stage and whether each cloud height is lit, and with what light;
  - **sunrise and sunset tables**: the same numbers as the command line's report.

Click any chart or time to move the scrubber there.

- **Short guides inside the app:** How it works, Why skies turn pink, Places and mountains, Packs, Help it learn and Privacy. They are plain-language versions written for the app (in `src/content/`), bundled so they work offline, and each ends with “Go deeper” links to the full technical docs on GitHub.
- **Menu and page transitions:** the four-dot button opens a full-screen menu of every page. Moving between pages, a circle in the next page's colour grows from where you tapped, like a sun rising, and the new page's headline rises in. With “reduce motion” switched on in your device settings, pages simply change.
- **Online status and settings:** the pill at the top right says whether you are online. Tap it to switch online extras on or off.
- **Colours you can touch:** on the guides, hovering the headline's coloured word makes it flow into its twilight neighbour (“pink” into purple), and bold colour words such as **deep red and pink** or the stage names show their real colours. Nothing animates unless you hover.
- **The logo:** hover over it (or the word skyglow) and the little sun sets through the pack's stage colours, from day into night, with the word taking the same colours. When the time scrubber is on a morning, it plays as a sunrise instead. It follows the place and time you are looking at.

## Privacy

Everything runs in your browser. Only these send anything, and only while you are online:

| Feature | What leaves your device | Where to |
|---|---|---|
| Use my location | nothing (the browser asks your permission; the position stays in the page) | - |
| Typing in the place search (3+ letters) | only the text you typed | Photon (OpenStreetMap search) |
| Pick on a map | the map area you look at, to fetch map pictures | OpenStreetMap tile servers |
| Online extras (on unless switched off) | the coordinates of places you pick, rounded to about 100 m | Open-Meteo (time zone, ground height) and OpenStreetMap Nominatim (name) |

Online-extras answers are cached in the browser, so the same spot is not looked up twice. Each request gives up after 8 seconds, so a slow service never blocks the app. The footer shows whether you are online or offline.

Saved places and your last settings are kept in this browser's storage only. "Forget this saved place" removes one.

## Honesty

The sun's position and Earth's shadow are exact geometry. The **colours** in the strips and the stage names come from the pack and are rough, **uncalibrated** guesses. The app does not yet know whether clouds are actually there tonight; each row of a colour strip means "if a cloud were at this height".

## Build it yourself

You need Node 24, Rust (stable) with the WebAssembly target, and the `wasm-bindgen` tool at the same version as the crate:

```powershell
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.130 --locked
```

Then, in this folder:

```powershell
npm install
npm run dev        # builds the engine, then serves the app at http://localhost:5173
npm test           # builds the engine, then runs the web tests (the real engine, in Node)
npm run build      # type-checks and builds the offline-ready site into dist/
```

`npm run wasm` rebuilds only the engine (into `src/wasm-pkg/`, which is generated and not committed).

## How it is put together

| File | Job |
|---|---|
| `src/lib/engine.ts` | The only file that talks to the engine. Typed helpers around the WebAssembly bridge (`wasm/`). |
| `src/lib/types.ts` | TypeScript shapes of the engine's answers. Field names match the Rust structs. |
| `src/lib/clock.ts`, `describe.ts`, `strip.ts`, `places.ts`, `packs.ts`, `geo.ts` | Small pure helpers: clocks, sentences, strip colours, bundled places and packs, GPS and online search. |
| `src/components/` | The screens: header, place panel, skyline editor, map, sun path, colour strip, timeline table, readout, doc pages, footer, notices. |
| `src/content/*.md`, `src/lib/docs.ts` | The in-app guides: their plain-language text, headline, colour and “Go deeper” links. |
| `src/lib/route.ts`, `transition.ts` | The tiny `#/docs/<page>` router and the sunrise page transition. |
| `vite.config.ts` | Build settings and the offline service worker. |

All physics stays in Rust. The web code only asks questions and draws the answers.

## Publishing

`.github/workflows/pages.yml` tests everything and publishes the site to GitHub Pages on each push to `main`. One-time setup in the repository: **Settings, Pages, Source: GitHub Actions**.
