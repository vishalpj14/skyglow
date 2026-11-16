# wasm: the bridge from the web to the engine

A tiny Rust crate that lets a web page call `skyglow-core`. It adds **no physics**: each function reads its inputs, calls the core, and hands back the answer.

It speaks **JSON text** both ways:

- a place goes in as the same JSON as `examples/places/*.json`;
- a pack goes in as the same JSON as `packs/*.json` (an empty string means the built-in default);
- answers come back as JSON with the same field names as the Rust structs.

| JavaScript name | What it answers |
|---|---|
| `timeline(place, pack, year, month, day, tz, event)` | One sunrise or sunset, like the command line's report |
| `skyAt(place, pack, jd)` | The sky at one instant, like `--at` |
| `skySeries(place, pack, jdStart, jdEnd, stepMinutes)` | The sky at many instants (sun path, colour strip); at most 3000 points per call |
| `packInfo(pack)` | The pack, checked and decoded |
| `conditionsFromOpenMeteo(answers, source)` | Merge the text of Open-Meteo answers (a JSON list of texts) into a conditions series. The page fetches; the engine only reads |
| `conditionsAt(series, jd)` | The conditions at one instant from a series, or `null` if nothing is close enough |
| `openMeteoForecastHourly()`, `openMeteoAirQualityHourly()` | The `hourly=` lists to ask Open-Meteo for |
| `glowAt(place, pack, series, jd)` | The glow score of every layer at one instant, with all factors. An empty series means "no forecast": only the potential |
| `glowSeries(place, pack, series, jdStart, jdEnd, stepMinutes)` | The glow at many instants, on the same grid as `skySeries` |
| `bestGlow(place, pack, series, jdStart, jdEnd, stepMinutes)` | The best glow moment in a window, or `null` |
| `defaultPack()` | Text of the built-in pack |
| `jdFromLocal`, `formatClock`, `solarNoon` | Clock helpers |

Bad input (unreadable JSON, latitude out of range, a date like 30 February) comes back as a JavaScript error carrying the core's message, never a crash.

Tests: `cargo test -p skyglow-wasm` checks the bridge in plain Rust; `web/tests/engine.test.ts` checks the compiled WebAssembly from JavaScript. How to build it is in [web/README.md](../web/README.md).
