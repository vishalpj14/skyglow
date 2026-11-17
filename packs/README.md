# Parameter packs

A pack is a small JSON file of **numbers** that tune Skyglow's estimates. Packs never contain code.

## Files

- `default.json`: global defaults, compiled into the program so it always works offline.
- `kerala.json`: a regional example. **Uncalibrated starting guesses.**

## Format (schema 1)

| Field | Type | Meaning |
|---|---|---|
| `schema` | integer | Must be `1`. Builds reject other versions. |
| `name` | string | Short identifier. |
| `description` | string, optional | What the pack is for and how trustworthy it is. |
| `earth_radius_km` | number | Used by the cloud-shadow geometry (6371). |
| `sunset_elevation_deg` | number | Sun elevation counted as "sunset" (-0.833 includes refraction and the sun's radius). |
| `cloud_layers` | list of `{name, height_km}` | Typical heights **above the ground** for low, mid, high cloud. At a place 3.5 km up, a 1.5 km layer sits at 5 km above sea level. Each must be below `cloud_ceiling_km`. |
| `cloud_ceiling_km` | optional number (default 16) | The highest a cloud can sit, km above sea level (about the top of the weather layer in the tropics). No layer goes above it, however high the place. A pack without it uses `default.json`'s. |
| `stages` | list of `{name, from_deg, to_deg, description, morning_name?, morning_description?}` | Named phases of the evening by sun elevation. `from_deg` must be higher than `to_deg`; stages must not overlap. The optional `morning_` fields give the names used at sunrise, when the same stages run in reverse; without them the evening name is reused. |
| `grazing_colours` | optional list of `{min_height_km, label}` | Rough colour label for light whose lowest altitude on the way to a cloud is at least `min_height_km`; the matching entry with the highest threshold wins. Uncalibrated. |
| `glow` | optional object | The glow-score numbers: forecast level heights, the distances toward the sun to read the forecast at (`sunward_km`), how much each level's cloud stops, curves for cover, colour, haze and humidity, and likely cloud types by cover (`cloud_types`). A pack without it uses `default.json`'s. Every field and where its numbers come from: [docs/GLOW.md](../docs/GLOW.md). |

Stages and `grazing_colours` entries may also carry an optional `swatch`: a display colour as `#rrggbb`, used only to draw the web app's colour strip and stage bands. Swatches are uncalibrated guesses, like the labels. A swatch that is not `#rrggbb` is rejected.

Inside `glow`, a **curve** is a list of `[x, y]` points from left to right, joined by straight lines; outside the first and last point the end value holds; every `y` is from 0 to 1. Fields inside `glow` must be spelt exactly (a typo is an error, not ignored).

Unknown extra fields at the top level are ignored, so packs can carry notes for future versions.

## Using a pack

```powershell
cargo run -p skyglow-core -- 9.93 76.27 2026-11-15 --tz 5.5 --pack packs/kerala.json
```

## Contributing a regional pack

1. Copy `default.json`, change `name` and `description`.
2. Adjust numbers only where you have a reason (photos, soundings, literature) and say what in `description`.
3. Run `cargo test` and the command above to check it loads.
