# Examples

Ready-to-run examples with places from around the world. Run them from the repository root.

Every place is its own file in `examples/places/` and is picked up automatically, so you can use a place by name:

```powershell
cargo run -p skyglow-core -- --place quito 2026-11-15
```

## The places

| Name | Why it is interesting |
|---|---|
| `new_york`, `los_angeles`, `tokyo` | Big mid-latitude cities, a typical sunset |
| `chennai` | Tropical India, but the sea is to the *east*, so sunsets happen over land |
| `kochi` | Tropical India, open sea to the *west* |
| `srinagar`, `yosemite` | Valleys: mountains hide the low sun, so the real sunset is earlier than the flat-horizon calculation |
| `singapore`, `quito` | On the equator: fast, steep sunsets |
| `sydney`, `cape_town` | Southern hemisphere: seasons and sunset directions are mirrored |
| `reykjavik` | High latitude: slow twilight, never fully dark in midsummer |
| `tromso`, `prudhoe_bay` | Above the Arctic Circle: polar night and midnight sun |

Each place file holds the coordinates, **height above sea level**, the local time zone (with its name, and daylight saving where it exists), and for some places a rough **mountain skyline**. The report prints the local time zone and offers a hint when a place uses daylight saving. To see the daylight-saving clock add `--tz`, for example `--place sydney 2026-12-21 --tz 11`. See [docs/PLACES.md](../docs/PLACES.md) for the file format.

`examples/conditions/sample-evening.json` is a sample of cloud cover, moisture and haze readings for one Kochi evening. The numbers are **made up** to show the file format; try it with `--place kochi 2026-11-15 --at 18:08 --conditions examples/conditions/sample-evening.json`. See [docs/CONDITIONS.md](../docs/CONDITIONS.md).

## The showcase: every city, straight from the program

[`make-showcase.ps1`](make-showcase.ps1) runs the real command-line tool for every place on the December and June solstices and saves exactly what it printed, with nothing edited by hand:

```powershell
powershell -File examples\make-showcase.ps1
```

This creates one file per place, such as `examples/showcase/quito.txt`, each holding sunrise and sunset on both solstices (with the clock a local wall clock would show, daylight saving included). Open a few to compare the equator against the Arctic, summer against winter, and the days when Tromso and Prudhoe Bay have no sunset at all.

## Things worth trying

**1. The equator is fast, the far north is slow.**
Compare the "Evening light stages" minutes.

```powershell
cargo run -p skyglow-core -- --place quito 2026-11-15
cargo run -p skyglow-core -- --place reykjavik 2026-11-15
```

Quito's stages are each a few minutes. Reykjavik's are several times longer, because the sun slides along the horizon instead of dropping straight down.

**2. No sunset at all.**
Tromso and Prudhoe Bay in December are in polar night, so the sun never even reaches the horizon.

```powershell
cargo run -p skyglow-core -- --place tromso 2026-12-21
cargo run -p skyglow-core -- --place prudhoe_bay 2026-12-21
```

The report says "none today (polar day or night)" and the stage times show `--:--:--`.

**3. A sunset with no real darkness.**
Reykjavik in midsummer: the sun sets, but never gets 6 degrees below the horizon, so the later stages never happen.

```powershell
cargo run -p skyglow-core -- --place reykjavik 2026-06-21
```

**4. Southern hemisphere.**
Everything is mirrored. Notice the sunset direction (azimuth) and the clock offset.

```powershell
cargo run -p skyglow-core -- --place sydney 2026-12-21 --tz 11
cargo run -p skyglow-core -- --place cape_town 2026-12-21
```

**5. A regional pack.**
The Kerala pack uses higher tropical cloud heights.

```powershell
cargo run -p skyglow-core -- --place kochi 2026-11-15 --pack packs/kerala.json
```

**6. What was the sky doing at a given minute?**

```powershell
cargo run -p skyglow-core -- --place singapore 2026-11-15 --at 18:56 --at 19:00 --at 19:04
```

For each time it prints how far below the horizon the sun is, which cloud heights are still lit, and how deep the sunlight dived on its way to each cloud (which decides the colour).

**7. Sunrise.**
Every report can be flipped to the morning: the stages run in reverse and the highest clouds are lit first.

```powershell
cargo run -p skyglow-core -- --place kochi 2026-11-15 --event sunrise
cargo run -p skyglow-core -- --place kochi 2026-11-15 --at 06:10
```

**8. Mountains and height.**
Quito sits 2850 m up (the horizon dips about 1.6 degrees, so sunset comes later). Yosemite sits in a valley whose walls hide the sun early, while the clouds above keep glowing.

```powershell
cargo run -p skyglow-core -- --place quito 2026-11-15
cargo run -p skyglow-core -- --place yosemite 2026-11-15
```

`examples/observations/kochi-sample.json` is a made-up observation that carries the sample conditions, to show the format: `--observe examples/observations/kochi-sample.json`. To replay your own notes against the model, see [docs/OBSERVATIONS.md](../docs/OBSERVATIONS.md).

## Files here

| File | What it is |
|---|---|
| `places/` | One JSON file per example place: coordinates, height, time zone, skyline |
| `make-showcase.ps1` | Script that regenerates `showcase/` from the real program |
| `showcase/` | One program-output file per place, both solstices (created by the script) |

The automated tests in `core/tests/examples.rs` check that the places and packs stay valid, so examples never silently break.
