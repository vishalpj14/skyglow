# Decision log

Short records of why things are the way they are. Add new entries at the bottom; never rewrite history, supersede instead.

## 0001 - Rust for the core

**Decision:** The engine is written in Rust.
**Why:** One implementation can compile to WASM (browser and phone), a Python module (photo lab) and a command-line tool, which removes the need to keep two codebases in agreement. It is also fast enough for per-pixel and radiative-transfer work later, and the compiler's error messages are good for learning.
**Cost:** Heavier toolchain, and fewer contributors know Rust than JavaScript. Mitigated by keeping most contribution surface in packs and the UI.
**Escape hatch:** The core is small. If WASM friction becomes a problem it can be ported to TypeScript, checked against golden test vectors.

## 0002 - Static-first app, no backend

**Decision:** The public app is fully static; live data is fetched directly from the browser.
**Why:** Free hosting, no operations, easy to fork, works offline in the field.

## 0003 - Parameter packs instead of hardcoded estimates

**Decision:** Estimated numbers (stage boundaries, cloud heights) live in versioned JSON packs.
**Why:** Corrections ship as small data updates instead of code releases, regions can differ, and photo-derived calibration has somewhere to land.
**Rules:** Numbers only, never code. A `schema` field is mandatory and unknown versions are rejected.

## 0004 - Vue 3 + TypeScript for the front end

**Decision:** Vue 3 (Composition API) with TypeScript and Vite, built as a PWA.
**Why:** Familiar from coursework, small and fast on phones, good dev tooling for the planned debug panels.

## 0005 - Python lab with SQLite and notebooks

**Decision:** Photo analysis and calibration live in a Python package using SQLite and Jupyter notebooks.
**Why:** The dataset (hundreds of photos) is small and tabular, notebooks make the analysis visible and teachable, and SQLite is a single file you can query with plain SQL.

## 0006 - Licence: MIT for code

**Decision:** MIT for the repository for now.
**Why:** Simple and permissive for an open-source tool. Pack data may later get its own licence (for example CC0 or CC-BY) if community contributions begin; Apache-2.0 can be added as a dual licence then.

## 0007 - Tests use independent expected values

**Decision:** Tests check against known astronomical facts and hand calculations, not outputs copied from the implementation.
**Why:** A test that only repeats the code's answer cannot catch a wrong formula.

## 0008 - Sunrise is sunset played backwards

**Decision:** One engine handles both events. `--event sunset|sunrise|both` chooses; with no `--event`, sunset is the default (what most people watch) unless every `--at` time is before solar noon (sunrise) or the times are mixed (both). Packs gained optional `morning_name` and `morning_description` for each stage; old packs without them still load (schema stays 1).
**Why:** The sun-depression angles and Earth-shadow geometry are identical in the morning, only reversed in time. Sharing the stage boundaries avoids two sets of numbers drifting apart, while morning names keep the output natural ("first light", not "afterglow").

## 0009 - "Rising" or "setting" comes from the hour angle

**Decision:** At a given instant the sun is called rising if its hour angle is negative (before solar noon) and setting if positive (after), computed in `sky_state`. Previously `--at` compared the clock time with solar noon on the same calendar date.
**Why:** The calendar-date rule is wrong between local midnight and solar midnight, when the sun is still sinking. At high latitude in summer that shows up in twilight, not only deep at night: Reykjavik on 21 June at 00:05 has the sun 0.7 degrees down and still sinking, which the old rule called rising. The hour angle flips exactly at solar midnight, at any clock offset.

## 0010 - The web bridge speaks JSON text

**Decision:** `wasm/` exposes a handful of functions that take places and packs as JSON text and return JSON text, built with `wasm-bindgen` only. TypeScript types for the answers are written by hand in `web/src/lib/types.ts`, matching the Rust field names.
**Why:** Places and packs already have a JSON format, so the web app can pass them through untouched. One small dependency, no generated binding layer to learn, and the browser sees exactly what `cargo test` sees. The cost (a JSON round trip per call) is tiny next to drawing.

## 0011 - The web app is offline-first; online extras are opt-in

**Decision:** The app, the engine, the example places and the packs are all cached by a service worker, so everything works with no network. GPS asks the browser's permission and stays on the device. Online name search (OpenStreetMap Nominatim) and the map (OpenStreetMap tiles) only run when the user presses their button, and the app says what each sends.
**Why:** Sunsets are often watched from places with poor signal, and a location is personal. Nothing should leave the device unless the user asks for it.

## 0012 - Display colours live in packs

**Decision:** Stages and colour bands gained an optional `swatch` (`#rrggbb`) used only for drawing. Packs stay at schema 1, since the field is optional.
**Why:** "Numbers in packs, not code": the colour a stage is drawn in is as much a guess as its name, and should be correctable without a release.

## 0013 - No sunset colour labels while the sun is up

**Decision:** `sky_state` gives cloud layers no colour label or swatch while the sun is above the horizon.
**Why:** The grazing-height colour model describes sunlight that skims the ground to reach a cloud after the sun has set for it. With the sun up the beam never dips, and the model's "grazing height" is just the cloud's height, which would wrongly label a low noon cloud "orange-red to pink".

## 0014 - Times default to the device's own clock

**Decision:** The web app shows times on the clock the device itself shows (its time zone, daylight saving included). The user can switch to the place's local time or a fixed UTC offset, and that choice is remembered only for saved places.
**Why:** People plan with the clock in their hand. Looking up a faraway place should not silently change what "18:00" means; choosing another clock should be deliberate.

## 0015 - Online extras fill in place details, behind one switch

**Decision:** A single "Online extras" switch, off until the user turns it on, lets the app look up a picked place's IANA time zone and ground height (Open-Meteo) and a name (OpenStreetMap Nominatim). Only coordinates rounded to about 100 m are sent; answers are cached in the browser. Supersedes nothing in 0011: the app still works fully offline, and nothing leaves the device unless the user asks.
**Why:** Typing a time zone and height for every spot is tedious and error-prone, and an IANA zone handles daylight saving automatically. But a location is personal, so sending it is the user's decision, stated plainly next to the switch.

## 0016 - Online features are on by default (supersedes the default in 0015)

**Decision:** Online extras start switched on. The first visit shows a small notice in the bottom-left corner that says what is sent (rounded coordinates of picked places, to Open-Meteo and OpenStreetMap) with a one-tap "Turn off"; the choice is remembered. The place search looks things up online as you type (Photon, an OpenStreetMap search built for type-ahead) instead of behind a separate button. The app still works fully offline.
**Why:** Someone visiting the site is online and expects a search box to search. Asking them to find a switch first made the common case slow. Being clear about what is sent, and making it easy to stop, keeps the user in charge. Photon is used for type-ahead because Nominatim's usage policy does not allow search-as-you-type; Nominatim is kept for naming a single picked spot.

## 0017 - The docs are pages inside the app, read from GitHub

**Decision:** The web app shows the plain-language docs (how it works, physics, places, packs, observations, testing, decisions, privacy) as its own pages at `#/docs/<page>`. Online, each page is read from the repository's `main` branch on GitHub; offline, or if GitHub does not answer within 8 seconds, the copy bundled at build time is shown, with a label saying which. The markdown is sanitised (DOMPurify) before display, and links between docs stay in the app.
**Why:** One source of truth: the markdown in the repo is both the GitHub docs and the app's help pages, so they cannot drift apart, and a fix to a doc reaches the app without a new release. The bundled copy keeps "works offline" true. Sanitising matters because the text comes over the network.

## 0018 - In-app guides are short, plain rewrites (supersedes 0017)

**Decision:** The app's pages are no longer the repository docs fetched from GitHub. They are short, plain-language guides written for people using the app (`web/src/content/`), bundled with it, and each ends with "Go deeper" links to the full technical docs on GitHub. Page changes use a "sunrise" transition: a circle in the next page's colour grows from the tap point, the page swaps underneath, and the headline rises in; it is skipped when the device asks for reduced motion.
**Why:** Most visitors want the idea, not the formulas or file formats, and should not need to know where text comes from. Technical readers are one tap away. Bundling the guides keeps them offline and instant, and removes the network fetch and the sanitising step that came with it. The design draws on the GDG Cloud Kochi Community Day 2026 site (big numbered menu, hero headlines with one coloured word, circular page reveal).

