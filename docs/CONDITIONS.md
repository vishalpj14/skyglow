# Conditions: what the air is like

The sun's position and Earth's shadow are the same every year. What makes one evening dull and the next one glow is the *air*: which clouds are there, how damp it is, how much dust and smoke is about. Skyglow calls these the **conditions**.

**Status, honestly:** skyglow fetches (in the web app), carries and shows conditions, and turns them into a rough **glow score** per cloud layer ([GLOW.md](GLOW.md)). The timings and the geometric colour labels do not change; the score says how likely each layer is to actually show that colour. Its numbers are not calibrated yet.

## The six numbers

Every one is optional. A missing number stays missing; skyglow never fills it in with a guess.

| Field | What it means | Typical values |
|---|---|---|
| `cloud_low_pct` | Share of the sky covered by low cloud (below about 3 km) | 0 to 100 |
| `cloud_mid_pct` | Share covered by middle cloud (about 3 to 8 km) | 0 to 100 |
| `cloud_high_pct` | Share covered by high cloud (above about 8 km) | 0 to 100 |
| `humidity_pct` | Relative humidity of the air near the ground | 0 to 100 |
| `water_vapour_mm` | All the water vapour in the air above you, as the depth of water it would make if it fell as rain (the same number as kg per square metre) | dry air under 10, tropical wet season 50 and more |
| `aerosol_optical_depth` | How much dust, smoke, salt and pollution dims a beam of green light (550 nm) going straight up. No unit. | very clean 0.05, hazy 0.5, thick smoke above 1 |

The low, middle and high split is the one weather forecasts use. It is close to, but not the same as, the `low`, `mid` and `high` cloud heights in a parameter pack.

Why these? See "What spoils or boosts the show" in [PHYSICS.md](PHYSICS.md): low cloud on the sun's side blocks the light, a little haze deepens the colour, a lot of haze greys it out, and damp air scatters more.

## A conditions file

A list of readings over time, each with a time in **UTC** and any of the six numbers:

```json
{
  "source": "where the numbers came from",
  "samples": [
    { "utc": "2026-11-15T11:30", "cloud_low_pct": 20, "cloud_high_pct": 40, "humidity_pct": 72 },
    { "utc": "2026-11-15T12:30", "cloud_low_pct": 10, "cloud_high_pct": 60, "humidity_pct": 78 }
  ]
}
```

- `source` is shown next to the numbers, so say plainly what they are ("my guess from the window" is a fine source).
- A time is `utc` (`YYYY-MM-DDTHH:MM`, seconds and a trailing `Z` allowed) or `jd` (a Julian Day number), not both.
- Readings can be in any order. Two readings for the same moment are an error.
- A misspelt field name is an error, not silently ignored.
- Numbers outside the sensible ranges (a percentage above 100, a negative depth) are errors.

A sample lives in `examples/conditions/sample-evening.json`. Its numbers are made up, and its `source` says so.

## Using it

```powershell
cargo run -p skyglow-core -- --place kochi 2026-11-15 --at 18:08 --conditions examples/conditions/sample-evening.json
```

After the usual `--at` report you get:

```text
  Conditions, between the 18:00 and 19:00 readings (made-up sample numbers, not a real forecast):
    cloud cover    low 10%, mid 4%, high 61%
    humidity       79% near the ground
    water vapour   46 mm in the air above
  Glow score, 0-10 (a rough guide, not calibrated yet):
    ...
```

The glow lines are explained in [GLOW.md](GLOW.md).

It also works with `--observe`. Better still, store the conditions inside the observation file itself (`"conditions": ...`), so they travel with the evening: see [OBSERVATIONS.md](OBSERVATIONS.md).

## What happens between readings

Forecasts come once an hour; you may ask about 18:08. Skyglow follows three rules and always tells you which one it used:

1. **Between two readings no more than 6 hours apart:** a straight line between them. Cloud 40% at 17:30 and 60% at 18:30 gives 50% at 18:00. If a number is missing from either reading it stays missing.
2. **Otherwise, within 60 minutes of a single reading:** that reading, unchanged.
3. **Otherwise nothing.** Skyglow prints "no reading close enough to this time" instead of guessing.

A straight line is a simplification: real cloud arrives in lumps, not smoothly. Treat in-between values as rough.

## Live numbers from Open-Meteo

[Open-Meteo](https://open-meteo.com) is a free weather service that needs no account or key. Two of its services carry all six numbers:

| Service | Gives | Asked for as |
|---|---|---|
| Forecast | cloud cover (low, middle, high), humidity, column water vapour | `cloud_cover_low`, `cloud_cover_mid`, `cloud_cover_high`, `relative_humidity_2m`, `total_column_integrated_water_vapour` |
| Air quality (passes on the European CAMS forecast) | aerosol optical depth | `aerosol_optical_depth` |

These are a **weather model's forecast** for a grid square some kilometres across, not a measurement at your spot. They cover roughly the last three months and the next few days (the aerosol forecast reaches less far ahead). Outside that range Open-Meteo refuses, and skyglow tells you so.

The engine itself never goes online (see [DECISIONS.md](DECISIONS.md), 0020 and 0021). Something else fetches the answers; the engine only reads their text (`core/src/openmeteo.rs`), the same way for the web app and the command line. A value Open-Meteo leaves empty, or one outside the sensible range, stays unknown.

### In the web app

With online extras on (the default), the **Conditions** card under the time scrubber looks up the forecast for the chosen place and date and shows the reading at the scrubber's time. It says which forecast hours the reading was worked out from, where it came from and how old it is.

- **What is sent:** the place's coordinates rounded to two decimals (about 1 km), the same for ten points along the lines toward sunset and sunrise (50 to 600 km away, so the glow score can use the cloud the sunbeam crosses, see [GLOW.md](GLOW.md)), and three dates (the day before, the day, the day after). All eleven go in one request to each service. Nothing else. The points are worked out from the place, so together they still say roughly where it is.
- **Saved for offline:** each answer is kept in the browser (the twelve most recent place-and-date pairs). A copy less than an hour old is used without asking again. Offline, or if Open-Meteo cannot be reached, the saved copy is shown with its age, however old.
- **Refresh** asks again straight away.
- **Save as file** downloads the forecast as a conditions file (times in UTC, no coordinates), to keep with an observation of that evening. See [OBSERVATIONS.md](OBSERVATIONS.md).
- **If only one service answers,** the other numbers are simply missing, and the card says so.
- **With online extras off,** nothing is fetched; a copy saved earlier is still shown.

### On the command line

The command line never goes online either. Ask it for the two addresses, save the answers yourself, then point skyglow at the files:

```powershell
cargo run -p skyglow-core -- --place kochi 2026-11-15 --open-meteo-urls
# copy each printed address into: curl.exe -o weather.json "<address>"   (and air.json)
cargo run -p skyglow-core -- --place kochi 2026-11-15 --at 18:08 --open-meteo weather.json --open-meteo air.json
```

`--open-meteo-urls` prints addresses covering the place and the points toward sunset and sunrise, worked out exactly as the web app does, with the same rounding. Answers saved from them let the glow score use the cloud along the sunbeam. An answer for the place alone (one pair of coordinates) also works: then the local forecast stands in for the whole path.

`--open-meteo` can be given once or twice, in either order; the answers are merged by time. Times in the answers may be in UTC (the default), in a local time zone (`&timezone=...`), or Unix seconds (`&timeformat=unixtime`); skyglow reads all three. Use `--conditions` or `--open-meteo`, not both. Change the coordinates and dates to yours, and keep saved answers for a private spot in `test/`.

Open-Meteo's data is free to use with credit (CC BY 4.0); the aerosol forecast comes from the Copernicus Atmosphere Monitoring Service.

In code: `core/src/conditions.rs` (`Conditions`, `ConditionsSeries`), `core/src/openmeteo.rs` (reading Open-Meteo answers), `web/src/lib/conditions.ts` (fetching and saving in the web app).
