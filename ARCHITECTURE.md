# Architecture

Skyglow answers one question: *given a place and a time, what will the evening sky do?* It is built so the exact part works everywhere offline, and the uncertain part can improve over time without shipping new code.

## Principles

1. **Static first.** No backend, database or accounts in the public app. It can be hosted on GitHub Pages and forked by anyone.
2. **Exact vs estimated.** Sun geometry is exact maths in code. Anything estimated (stage boundaries, cloud heights, haze effects) lives in versioned data packs.
3. **Explainable.** Every calculation can expose its intermediate values. If a prediction looks wrong, you can see why.
4. **Fallbacks, never failures.** Live data, then cached pack, then built-in defaults.

## Layers

```
        +---------------------------+
        |  web/  Vue 3 + TS PWA     |   time scrubber, map/GPS, debug panel
        +-------------+-------------+
                      | WASM
        +-------------v-------------+      +-------------------------+
        |  core/  Rust engine       |<-----|  packs/  JSON parameters |
        |  time, sun, cloud,        |      |  (global + regional)     |
        |  params, timeline         |      +------------^------------+
        +------+--------------+-----+                   |
               |              | PyO3 / CLI              | exported corrections
        +------v-----+  +-----v--------------------------+
        |  CLI       |  |  lab/  Python                   |
        +------------+  |  photos -> SQLite -> notebooks  |
                        +---------------------------------+
```

| Layer | Responsibility | Depends on |
|---|---|---|
| `core/` | All maths. Pure functions, no I/O in the library. | serde (packs only) |
| `packs/` | Tunable numbers as JSON with a schema version. | nothing |
| `wasm/` | Bridge: JSON in, JSON out, no physics. | core, wasm-bindgen |
| `web/` | UI, GPS, offline caching, optional live data. | core (via `wasm/`) |
| `lab/` | Photo analysis, correlation, pack generation. | core (Python binding) |

## Core modules

- `time`: Julian Day, date parsing, clock formatting.
- `sun`: NOAA/Meeus solar position; finds when the sun crosses a given elevation.
- `cloud`: Earth-shadow geometry for cloud layers.
- `params`: loads and validates packs.
- `timeline`: assembles the evening timeline for a place and date.
- `sky`: the sky at one instant, `sky_state(params, place, jd)`: sun position, rising or setting, skyline, which cloud layers are lit and where the shadow edge is. The CLI's `--at` and the web time scrubber both use it.
- `conditions`: what the air is like, as plain numbers handed in from outside: `Conditions` (one reading) and `ConditionsSeries` (timed readings, with honest rules for the gaps between them). Shown by the CLI's `--conditions` and the web app's Conditions card; not used in predictions yet. See [docs/CONDITIONS.md](docs/CONDITIONS.md).
- `openmeteo`: reads the text of Open-Meteo forecast and air-quality answers into a `ConditionsSeries`. Text in, struct out: it never goes online.
- `sunward`: points along the sun's direction at sunset and sunrise (`plan`), where the forecast is read so the glow score can use the cloud the sunbeam crosses.
- `glow`: the glow score. Takes a `SkyState` and `Conditions` and returns a 0-10 score per layer with every factor (light, sun, colour, cover, path, view, haze, humidity); `best_glow` finds the best moment in a window. Curves in the pack's `glow` section. See [docs/GLOW.md](docs/GLOW.md).

## Live inputs (fetched, shown, and used by the glow score)

Each is optional; the engine falls back to pack defaults without them. The core never fetches: the web app fetches from Open-Meteo (`web/src/lib/conditions.ts`) and the command line reads saved answers (`--open-meteo`); both hand the text to `openmeteo`.

| Input | Source idea | Used for |
|---|---|---|
| Cloud cover at low, mid and high levels | Open-Meteo / NWP | Which layers exist and how much of the sky they cover |
| Aerosol optical depth, dust, PM2.5 | CAMS via Open-Meteo, ground stations (OpenAQ, CPCB) | Saturation and dullness |
| Humidity and column water vapour | NWP | Haze, how far clean light travels |

## Private data

`test/` is git-ignored. Real coordinates (named places) and your own observation files live there and are read by the CLI (`--place`, `--observe`). The public repository only documents the formats, see [docs/OBSERVATIONS.md](docs/OBSERVATIONS.md).

## Calibration chain

Each layer overrides the one before it:

1. Defaults built into the binary (`packs/default.json`).
2. Regional pack (for example `packs/kerala.json`).
3. Live forecast and aerosol data, when online.
4. Personal calibration from your own photos (lab output).

## Pack safety

Packs hold numbers only, never code. They carry a `schema` version, and a build refuses schema versions it does not understand. A bad pack can make a prediction worse, but cannot break or compromise the app.

## Testing strategy

- Rust unit tests next to each module, plus `core/tests/physics.rs` for facts checkable from outside (equinox, solstice, equation-of-time extremes, a Kochi sunset sanity check, equatorial twilight shorter than London).
- `wasm/` has its own Rust tests, and `web/tests/` runs the compiled WebAssembly engine from TypeScript (Kochi's almanac times, the CLI's wording, bad input becoming errors).
- CI (`.github/workflows/pages.yml`) runs both test suites before publishing the web app.
- Expected values come from known facts or hand calculation, not from the code under test.
- Later: golden test vectors generated from a trusted reference (Astropy or Skyfield) so the Rust core and any future port can be compared independently.

See [docs/DECISIONS.md](docs/DECISIONS.md) for why each choice was made.
