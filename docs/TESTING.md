# Testing

Skyglow's tests are written to be read. Each one states a fact about the real world and checks that the code agrees.

## Run them

```powershell
cargo test
```

You should see every test pass. That covers the engine, the command line and the web bridge (`wasm/`). The web app has its own tests, which run the real engine compiled to WebAssembly:

```powershell
cd web
npm test
```

To run just one Rust test file:

```powershell
cargo test --test physics      # core/tests/physics.rs
cargo test --test examples     # core/tests/examples.rs
cargo test cloud               # every test with "cloud" in its name
```

## Where the tests are

| Where | What it checks |
|---|---|
| `core/src/time.rs` (bottom) | Date and clock helpers: Julian Day numbers, date parsing, leap years, clock formatting |
| `core/src/cloud.rs` (bottom) | Earth-shadow geometry: shadow heights, shadow edge, grazing height |
| `core/src/place.rs` (bottom) | Height above sea level, horizon dip, mountain skyline, terrain-aware sunset |
| `core/src/sky.rs` (bottom) | The sky at one instant: noon sun height in Kochi (90 - latitude - declination), rising vs setting, the overhead lit-height threshold and grazing height worked by hand, a shadow edge past the horizon, skyline hiding a low sun, colour label from the pack |
| `core/src/conditions.rs` (bottom) | Conditions readings: UTC text against the J2000 epoch (Julian Day 2451545.0), values between readings worked by hand, no guessing across long gaps or far from the data, typos and out-of-range numbers rejected |
| `core/src/openmeteo.rs` (bottom) | Reading Open-Meteo answers written by hand: Unix time 946728000 lands on J2000, local-clock times are stepped back to UTC, forecast and air quality merge by time, empty and nonsense values stay unknown, refusals are reported in words |
| `core/src/glow.rs` (bottom) | The glow score with round test numbers: every factor of a two-layer evening worked by hand (beam lowest points from `(R + h) cos d - R`), what blocks the sunbeam and what you look through, a clear sky scoring 0, missing values, a shadow edge toward the sun, a high sun, and the best moment landing where the beam dips to the richest colour |
| `core/src/params.rs` (bottom) | Pack loading: good packs load, bad schema or backwards stages are rejected |
| `core/tests/physics.rs` | Whole-engine facts: sunset times, equinox, solstice, equation of time, equator vs London twilight, sunrise mirroring sunset, ridge and height effects at sunrise |
| `wasm/src/lib.rs` (bottom) | The web bridge: Kochi's almanac sunset through JSON, series spacing and limits, bad input becomes an error |
| `web/tests/clock.test.ts` | Clock helpers: J2000 epoch hours, leap years, dates across time zones, UTC labels, standard vs daylight time |
| `web/tests/engine.test.ts` | The compiled WebAssembly engine from TypeScript: Kochi sunrise and sunset against the almanac, the overhead lit height worked by hand, a day of sky states and colour strip, bundled places and packs |
| `web/tests/glow.test.ts` | The glow score through the compiled engine: a clear forecast scores 0 and fades every strip row into the sky, high cloud glows best after sunset, colour mixing, and the "Why" lines multiplying back to the score |
| `web/tests/conditions.test.ts` | Live conditions with a pretend Open-Meteo (no network): only rounded coordinates and dates are sent, a fresh saved answer is reused, offline shows the saved copy, one service failing does not lose the other, failures are explained |
| `core/tests/observe.rs` | Runs the real `skyglow` program on observations that carry conditions (inline, a file, saved Open-Meteo answers): values halfway between hand-written readings, names found beside the observation, the command line replacing the file's conditions, broken blocks explained. Unix times are worked out by hand in the file |
| `core/tests/examples.rs` | The shipped `examples/` and `packs/` folders stay valid (places, packs, sample conditions), and example places behave like the real Earth |

## What "independent" means here

A test that only repeats what the code computes proves nothing: if the formula is wrong, the test is wrong the same way. So expected values come from outside the code:

- **Known facts:** at the equinox the sun is overhead at the equator at noon, at the June solstice the sun's declination equals the Earth's axial tilt (about 23.4 degrees), and the equation of time peaks near +16.4 minutes in early November and -14.2 minutes in mid February.
- **Hand calculation:** for example, a 10 km cloud overhead goes dark when the sun is `acos(6371 / 6381)` = 3.2 degrees below the horizon. The test comment shows the working.
- **Comparisons:** twilight must be shorter near the equator than far north. We don't pin exact minutes, we check the relationship.

Tolerances are deliberately loose (a few minutes, a fraction of a degree) because the aim is to catch wrong formulas, not to match one almanac to the second.

## Adding a test

1. Pick a fact you can state without running the code.
2. Write the expected value from that fact, with a comment saying where it comes from.
3. Put it next to the code it checks (inline `#[cfg(test)]` module) for small units, or in `core/tests/` for whole-engine behaviour.

## Private tests

Your real spots and observations live in the git-ignored `test/` folder and are not part of `cargo test`. Replay them with `--observe`, see [OBSERVATIONS.md](OBSERVATIONS.md). They are for *checking the model against your sky*, not for pass or fail.
