# Place files

A place is a small JSON file: where you are, how high up, what is on the skyline, and which clock to print. The shipped examples live in `examples/places/`, one file each. Your own go in `test/places/` (private, git-ignored).

```json
{
  "name": "Srinagar",
  "lat": 34.0837,
  "lon": 74.7973,
  "elevation_m": 1585,
  "tz": 5.5,
  "tz_name": "India Standard Time (IST)",
  "peaks": [
    { "name": "Pir Panjal range (approximate)", "az": 235, "distance_km": 60, "height_m": 4200, "half_width_deg": 40 }
  ],
  "note": "Anything you like."
}
```

## Fields

| Field | Required | Meaning |
|---|---|---|
| `name` | no | Display name. |
| `lat`, `lon` | yes | Degrees, north and east positive. |
| `elevation_m` | no (0) | Height of your viewpoint above sea level, metres. |
| `tz` | no | UTC offset of your local **standard** time, e.g. `5.5`. |
| `tz_name` | no | Name of that clock, e.g. `India Standard Time (IST)`. Printed in the report. |
| `dst_tz`, `dst_tz_name`, `dst_note` | no | Daylight saving offset (usually `tz + 1`), its name, and when it applies in words. |
| `peaks` | no | Mountains: `az` (bearing from you, degrees), `distance_km`, `height_m` above sea level, optional `half_width_deg` (default 8). |
| `horizon` | no | Hand-drawn skyline: points `{az, elev}` joined by straight lines, wrapping round north. |
| `note` | no | Free text. |

## How the numbers are used

- **Time zone.** The report prints the clock name and offset. If a place uses daylight saving and you are viewing standard time, it tells you which `--tz` to use for the daylight-saving clock. Skyglow does not know today's date rules, so you choose.
- **Height.** Lowers the sea horizon by about `0.0293 x sqrt(elevation_m)` degrees, so the flat-horizon sunset comes later. The shadow edge is also computed from your height.
- **Skyline.** For the sun's bearing, the higher of the `horizon` line and any `peak` bump is the skyline. A peak's angle is `atan(height difference / distance)` minus a little for Earth's curve. The sun counts as hidden once it is below that angle (refraction corrected).
- **Sunrise too.** The same skyline and height are used in the morning: a ridge in the east delays the sunrise you see, and standing higher advances it.
- **Heights are above sea level.** Cloud heights in packs are above sea level too.

## Honest limits

The skyline is a handful of numbers, not a terrain model. Treat mountain results as a guide, and measure your own skyline for anything that matters. The example skylines (Srinagar, Yosemite, Quito) are approximate and illustrative. Skyglow assumes a flat horizon unless you describe one.

Try it:

```powershell
cargo run -p skyglow-core -- --place yosemite 2026-11-15
```
