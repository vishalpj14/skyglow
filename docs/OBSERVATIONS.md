# Observations and private places

Your own sunsets are the calibration data for Skyglow. This page explains the file formats and how to keep real locations out of the public repository.

## Private vs public observations

There are two homes for observations. Which one you use is entirely up to you.

| | Private | Public |
|---|---|---|
| Folder | `test/` (git-ignored) | `examples/` (committed) |
| Coordinates | Whatever you like, exact spots included | Your choice; see the recommendation below |
| Contains | Your places, personal notes, anything you like | Observations you are happy to share |
| Who sees it | Only you | Everyone, and it helps calibrate the model for everyone |
| Used by | `--place`, `--observe` (private places take priority) | The same commands, so anyone can replay them |

**Recommendation (not a rule):** if you share an observation publicly, consider using a nearby city or coordinates rounded to one decimal place (about 11 km) instead of the exact spot where you stood, since that is often near where you live. Exact positions are not needed for the model to learn from your sky. The tool treats both folders identically.

## The private `test/` folder

The folder `test/` is listed in `.gitignore`, so nothing in it is ever committed. Put these there:

```
test/
  places/
    home.json                 your named spots, one file each (or one places.json map)
  observations/
    my-sunset.json            one file per evening you want to check
```

Everything below that mentions `test/` is local to your machine. The public repo contains only the code and these format docs.

## `places.json`

Named places with coordinates and the UTC offset of the clock you note times in.

```json
{
  "kochi": { "lat": 9.93, "lon": 76.27, "tz": 5.5 }
}
```

Use a place by name:

```powershell
cargo run -p skyglow-core -- --place kochi 2026-11-15 --pack packs/kerala.json
```

Names are looked up in `test/places/<name>.json` first (your private spots win), then in the map file `test/places.json`, then in `examples/places/<name>.json` (the public examples). Use `--places <folder-or-file>` to search one specific place. The richer one-file-per-place format (height, skyline, time zone name) is described in [PLACES.md](PLACES.md).

## Observation files

One JSON file per evening.

```json
{
  "place": "kochi",
  "date": "2026-03-20",
  "pack": "packs/kerala.json",
  "notes": "Free text about the whole evening.",
  "events": [
    { "time": "18:05:00", "source": "photo",  "note": "Low cloud dark, high cloud glowing orange." },
    { "time": "18:12:52", "source": "photo",  "note": "Last pink on a cloud edge." }
  ]
}
```

| Field | Meaning |
|---|---|
| `place` | Name of a place (a file in `places/`, or an entry in `places.json`). Alternatively give `lat` and `lon` directly. |
| `date` | Local date, `YYYY-MM-DD`. |
| `tz` | Optional UTC offset of your clock (otherwise the place's, then `--tz`, then 0). |
| `pack` | Optional parameter pack to use. |
| `notes` | Optional free text. |
| `events[].time` | Local clock time `HH:MM` or `HH:MM:SS`. |
| `events[].source` | Optional: `photo` (EXIF time, most reliable) or `memory` (approximate). |
| `events[].note` | What you saw at that moment. |

Replay one against the model:

```powershell
cargo run -p skyglow-core -- --observe test/observations/my-sunset.json
```

For every event the tool prints how far below the horizon the sun was, how high an overhead cloud must be to still be lit, and for each cloud layer whether it was lit, how deep the sunlight dived (the grazing height) and the rough colour the pack expects. Compare that with your note.

## Tips for useful observations

- **Prefer photo EXIF times** over memory. A few minutes of memory error moves the answer a lot.
- **Note where in the sky** the cloud edge or colour was, as an angle above the horizon (a fist at arm's length is about 10 degrees). Height and distance trade off, so this resolves them.
- **Note the cloud type** (cirrus, cirrocumulus, altocumulus, cumulonimbus anvil) and how much of the sky was covered at each level.
- **Note the air:** hazy, dusty, smoky, just rained, humid.
