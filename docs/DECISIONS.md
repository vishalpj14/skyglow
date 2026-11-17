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

## 0019 - Lowercase name, a living logo, and other clocks at a glance

**Decision:** The app calls itself "skyglow" in lowercase. Its logo and wordmark animate on hover: the sun sets through the current pack's stage colours (or rises, when the time scrubber is on a morning at the chosen place), and the high cloud in the icon keeps its light a little longer at dusk and catches it first at dawn. The wordmark's colours lean 40% toward the normal text colour so it stays readable even at "night". When a place keeps a different time from the device, the scrubber also shows that moment on the place's clock, in UTC (unless UTC is one of the two) and on the device's clock, with day changes marked. The scrubber opens at the device's current time, and the first-visit online-extras notice is gone (the switch stays in the status pill).
**Why:** The logo becomes a tiny demonstration of what the app predicts, using the same stage colours as the charts. Showing a faraway place's time next to your own avoids the most common planning mistake, reading a time on the wrong clock. Opening at "now" makes the first screen answer "what is the sky doing right now?".

## 0020 - Conditions are plain numbers handed to the core, and optional

**Decision:** Cloud cover (low, middle, high), humidity, column water vapour and aerosol optical depth enter the engine as a plain `Conditions` struct, or a `ConditionsSeries` of timed readings (`core/src/conditions.rs`). The core never fetches them: adapters outside it (the web app, a script, a hand-written file read by the CLI's `--conditions`) do that. Every value is optional and a missing one is never guessed. Between readings the series draws a straight line only when they are at most 6 hours apart, holds a single reading for at most 60 minutes, and otherwise says nothing; each answer says which rule produced it. For now conditions are shown next to the geometry and change no prediction.
**Why:** Keeping fetching outside keeps the core pure, testable and usable offline, and lets any data source plug in. Optional values mean partial data (clouds but no aerosols) still helps. Refusing to stretch data across long gaps, and saying how each value was worked out, follows the "no black boxes" rule. Shipping the data shape before the model that uses it keeps each step small and lets real numbers be collected while the model is designed. The two limits are about handling data, not physics, so they are constants in code rather than pack values.

## 0021 - Live conditions come from Open-Meteo; the page fetches, the core reads

**Decision:** Live conditions come from Open-Meteo's forecast service (cloud cover by level, humidity, column water vapour) and its air-quality service (CAMS aerosol optical depth). Reading an answer is a pure function in the core (`core/src/openmeteo.rs`: text in, `ConditionsSeries` out); fetching is done outside it. The web app fetches when online extras are on, sending coordinates rounded to two decimals (about 1 km) and three dates; it keeps the twelve most recent answers in the browser, reuses one for an hour, and offline shows the saved copy with its age. The command line never goes online: `--open-meteo <file>` reads answers the user saved. The list of values to ask for lives in the core, next to the code that reads them. If one service fails the other is still used; if both fail the reason is shown.
**Why:** Open-Meteo needs no key or account, so the static, backend-free app (0002) can use it directly, and one provider covers all six numbers. Reading in the core means the web app and the command line can never translate differently, and it is tested once with hand-written answers. Keeping the network out of the command line keeps the core crate free of network code and its output repeatable. Rounding to about 1 km loses nothing (the forecast grid is coarser) and sends less about where the user is. Fetching automatically follows 0016: someone online expects the weather to be there, and the one switch still stops it. Showing source and age on every reading keeps a stale offline copy honest.

## 0022 - Observations carry their own conditions

**Decision:** An observation file may hold a `conditions` entry: readings written inline, a skyglow conditions file, or saved Open-Meteo answers. File names are relative to the observation's folder. `--observe` prints the conditions under every noted time; `--conditions` or `--open-meteo` on the command line replace them for that run, and the output says so. The web app's Conditions card can save the forecast as a conditions file (UTC times, no coordinates) to keep beside the observation.
**Why:** The colour model will be fitted on observations, and colour depends on the air, so every observation needs the air stored with it. Forecast services keep only a few months, so the numbers must be saved close to the evening, by the person who was there. Paths relative to the observation keep an evening's files together. Leaving coordinates out of the saved file means it reveals no more than the observation it sits beside.

## 0024 - The sunbeam's path uses the forecast along the sun's direction

**Decision:** The forecast is read at the place and at points 50, 150, 300, 450 and 600 km toward the sun at sunset and at sunrise (`glow.sunward_km`, worked out in `core/src/sunward.rs` from the sun's azimuth at the event). The glow score's path factor finds where the sunbeam crosses each other cloud level (a parabola for Earth's curve: two crossings below the cloud, one above) and reads that level's cover there, joining the points with straight lines. A layer lit only toward the sun takes its own cover at the shadow edge. The web app asks for all eleven places in one request per service; the command line prints the same addresses (`--open-meteo-urls`) and reads the saved answers. Without the points, the local cover stands in, and the output says which was used.
**Why:** The light that colours the late show crosses other cloud 100 to 600 km away, so the cover overhead was the wrong number for the most important factor; this was the model's largest known blind spot. Computing the crossing distances ties the readings to the geometry the engine already has, instead of an average along the way. Five points per direction keep requests small while covering the crossing range for clouds up to about 13 km. Points 50 km and more away say little more about where the user is than the place itself, which is already sent rounded; the privacy notes say so plainly.

## 0023 - The glow score: a product of named factors, numbers in packs

**Decision:** Conditions now change the prediction through a glow score from 0 to 10 per cloud layer (`core/src/glow.rs`). It is 10 times eight factors between 0 and 1: light, sun, colour (geometry), and cover, path, view, haze, humidity (weather). Each factor's shape is a curve of points in the pack's new optional `glow` section; packs without one use the default pack's, so the schema stays 1. Whether other cloud blocks the sunbeam is decided by the beam's lowest point (grazing height): only cloud above it can be in its way. Haze and humidity count most for beams that skim low. Missing values count as no effect and are listed; without the layer's own cover there is only the geometric potential. Every score is returned with all its factors, and the app and CLI show them. The first numbers come from public sources (Open-Meteo's level definitions, Corfidi's NOAA notes on twilight colour, cloud and aerosol textbooks, photographers' rules of thumb), listed in docs/GLOW.md, and are labelled uncalibrated.
**Why:** A product of named factors is easy to explain line by line and to fit later: each curve can be tuned from observations without touching code, and a factor whose curve stays flat simply drops out. Using the grazing height for blocking ties the weather to the geometry the engine already computes, and reproduces the known failure (low cloud toward the sun cuts off late light) without a separate rule. Following Corfidi (clean air is best) instead of "moderate haze boosts colour" keeps the default on published ground; factors cannot exceed 1, so a boost would need new evidence and a new factor. A Beer-Lambert extinction model was considered and rejected for now: with ground-level aerosol depth it predicts near-darkness exactly when vivid pink is commonly seen, because it ignores how the eye judges colour against a darkening sky.

## 0025 - Cloud layers count from the ground, up to a ceiling

**Decision:** The heights in a pack's `cloud_layers` are above the place's ground, not above sea level. The engine adds the ground height and caps the result at the pack's new optional `cloud_ceiling_km` (16 km in the default pack; packs without it use the default's, so the schema stays 1). Every layer in the engine's answers carries both its height above sea level (`height_km`, used for all the shadow geometry) and `above_ground_km`. The glow score picks a layer's forecast level from its height above the ground and reports the level's range here (`range_km`) and a likely cloud type from the cover (`cloud_types`, a labelled guess). The command line and the app show both heights for ground 500 m or more up. With a forecast, the colour strips leave out a layer whose forecast cover stays under 5% for the whole window (blending between forecast hours leaves traces such as 0.3%, so the cut is not at exactly 0), and hovering a row's name shows that layer's heights, range, cover, likely type and best glow. The glow answer also names the highest level with cloud overhead (`highest_cloud`) and its range, since forecasts give no cloud tops.
**Why:** At Leh (3.5 km up) the default low cloud at 1.5 km above sea level was underground, yet the engine still said it was lit. Weather models count their low, middle and high levels from the ground (by pressure compared with the surface), so ground-relative layers also match the forecast they are scored with. At sea level nothing changes. The ceiling keeps a very high place (8 km up) from putting high cloud above where weather happens. Hiding cloudless rows makes a clear forecast read as clear, instead of three rows that silently copy the sky.
