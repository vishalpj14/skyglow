# skyglow

**Find out when the sky will do its best colours, for any place on Earth.**

Skyglow tells you, for any GPS coordinate and date, when the sun sets, how long each phase of the evening light lasts, and when Earth's shadow reaches clouds at different heights. That is the timing behind the orange, pink and magenta skies that photographers chase. It runs on your own computer or phone, needs no internet, and shows its working so you can learn the physics as you use it.

It is made for sunset and star photographers who want to know *when* to be ready, and for anyone curious about why skies look the way they do.

> Think "Stellarium for sunset light". The sun maths is exact and works anywhere. The colour part is estimated, improves with real observations, and is honest about what it doesn't know yet.

## See it in action

```text
> cargo run -p skyglow-core -- 9.93 76.27 2026-11-15 --tz 5.5

  Solar noon   12:09:26
  Sunset       17:59:34   (look toward azimuth 251°)

Evening light stages (by sun elevation):
  golden_hour   17:30:10 -> 17:59:34   29 min  [+6.0° to -0.8°]
  sunset_glow   17:59:34 -> 18:04:34    5 min  [-0.8° to -2.0°]
  afterglow     18:04:34 -> 18:13:08    9 min  [-2.0° to -4.0°]
  purple_light  18:13:08 -> 18:21:40    9 min  [-4.0° to -6.0°]
  blue_hour     18:21:40 -> 18:30:12    9 min  [-6.0° to -8.0°]

Cloud layers: when Earth's shadow reaches a layer directly overhead:
  low     1.5 km   dark overhead at 18:01:20   (sun 1.24° below horizon)
  mid     5.0 km   dark overhead at 18:05:43   (sun 2.27° below horizon)
  high    9.0 km   dark overhead at 18:09:02   (sun 3.04° below horizon)
```

How to read it, in plain words:

- **Sunset at 17:59** and the sun disappears toward the west-south-west (azimuth 251°, where 90° is east and 270° is west).
- **Stages** are the phases of the evening. The numbers in brackets are how far the sun is above (+) or below (-) the horizon.
- **Low clouds go dark first** (18:01), then middle ones (18:05), then high ones (18:09). Earth's shadow climbs the sky, and the high clouds are the last to leave the light. That is why the pink often shows up *after* the sun has set.

New to this? Read **[How it works, in plain language](docs/HOW-IT-WORKS.md)**.

## Try it

You need [Rust](https://rustup.rs/) (the stable version). Then, from the project folder:

```powershell
# 1. Check everything works
cargo test

# 2. Any place on Earth: latitude, longitude, date, and your clock's offset from UTC
cargo run -p skyglow-core -- 9.93 76.27 2026-11-15 --tz 5.5

# 3. Use a ready-made example place by name (one file per place in examples/places/)
cargo run -p skyglow-core -- --place quito 2026-11-15

# 4. What was the sky doing at specific minutes?
cargo run -p skyglow-core -- 9.93 76.27 2026-11-15 --tz 5.5 --at 18:08 --at 18:12

# Sunrise works too (--event sunrise, or just ask about a morning time)
cargo run -p skyglow-core -- 9.93 76.27 2026-11-15 --tz 5.5 --at 06:10

# 5. See every step of the sun calculation
cargo run -p skyglow-core -- 9.93 76.27 2026-11-15 --tz 5.5 --explain

# 6. Show cloud cover, moisture and haze readings next to a time (shown only, not used yet)
cargo run -p skyglow-core -- --place kochi 2026-11-15 --at 18:08 --conditions examples/conditions/sample-evening.json
```

Southern and western coordinates are negative (Sydney is `-33.87 151.21`). Put options after the coordinates. Run `cargo run -p skyglow-core -- --help` for everything.

Want ideas? **[examples/](examples/README.md)** has 14 ready-made places (New York, Tokyo, Chennai, Srinagar, Yosemite, Quito, Tromso, Prudhoe Bay and more), guided experiments like equator vs Arctic, and a showcase of real program output for every place that you can regenerate with one command.

## In your browser

The same engine also runs as a web app that works offline on a phone: pick a place (GPS, search, map or coordinates), drag a time scrubber through the day, and see the sun's path, colour strips around sunrise and sunset, and the full timelines. See **[web/README.md](web/README.md)** to run it locally; the repository's GitHub Pages site hosts it once Pages is enabled.

## Your own sunsets

Skyglow gets better when you tell it what you really saw.

- Write down a few timed notes from an evening ("18:05, low cloud dark, high cloud glowing orange").
- Replay them against the model with `--observe`, and it prints what the physics says at each moment.
- Keep your exact home spots **private** in the git-ignored `test/` folder, and share anonymous examples publicly in `examples/`.

Details in **[docs/OBSERVATIONS.md](docs/OBSERVATIONS.md)**.

## What's inside

| Folder | What it is | Status |
|---|---|---|
| `core/` | The Rust engine and command-line tool | working |
| `packs/` | Small JSON files of tunable numbers, per region | working |
| `examples/` | Example places and a sample observation file | working |
| `docs/` | Explanations in plain language, physics notes, decision log | growing |
| `wasm/` | Bridge that lets the web app call the engine | working |
| `web/` | Phone-friendly, offline web app (Vue + TypeScript) | working |
| `lab/` | Python tools to learn from your photos | planned |

More: [ARCHITECTURE.md](ARCHITECTURE.md) (how the pieces fit), [docs/PHYSICS.md](docs/PHYSICS.md) (why skies get their colours), [docs/TESTING.md](docs/TESTING.md) (how it is tested), [CONTRIBUTING.md](CONTRIBUTING.md).

## What it can and can't do today

**Can:** sun position and sunset time anywhere, evening phases, when Earth's shadow reaches clouds of a given height, how deep sunlight dives on its way to a cloud (which decides how red or pink it looks).

**New:** it reads a weather forecast's cloud cover, humidity, water vapour and haze (the web app looks them up from the free Open-Meteo service and keeps them for offline; the command line reads them from a file, [docs/CONDITIONS.md](docs/CONDITIONS.md)), and turns them into a **glow score** from 0 to 10 per cloud layer, with the best moment around sunrise and sunset and every reason shown ([docs/GLOW.md](docs/GLOW.md)).

**Can't yet:** promise what the sky will look like. The glow score is a rough guide built from public rules of thumb, not yet calibrated against real photos, and it only knows the cloud forecast for your area, not along the sunbeam's path toward the sun. Those are the next steps, and the colour labels are rough guesses calibrated on very little data.

## Roadmap

1. **v0 (done):** sun and twilight geometry, cloud-shadow windows, command line, parameter packs, observation replay.
2. **v1 (now):** Vue web app you can open on a phone, offline, with a time scrubber, sun path and colour strips. Next: manual cloud and haze sliders.
3. **v2:** Live cloud-layer, aerosol and moisture data (cloud cover by height, aerosol optical depth, humidity, column water vapour).
4. **v3:** Photo lab: read photo times and GPS, measure sky colours, compare with forecasts, publish better regional packs.
5. **v4:** Sun and sunspot planning, twilight and low-horizon astrophotography.

## Licence

MIT, see [LICENSE](LICENSE).
