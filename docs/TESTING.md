# Testing

Skyglow's tests are written to be read. Each one states a fact about the real world and checks that the code agrees.

## Run them

```powershell
cargo test
```

You should see every test pass. To run just one file:

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
| `core/src/params.rs` (bottom) | Pack loading: good packs load, bad schema or backwards stages are rejected |
| `core/tests/physics.rs` | Whole-engine facts: sunset times, equinox, solstice, equation of time, equator vs London twilight |
| `core/tests/examples.rs` | The shipped `examples/` and `packs/` folders stay valid, and example places behave like the real Earth |

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
