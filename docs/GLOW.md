# The glow score

The geometry tells skyglow *which* cloud heights are lit, and how deep the sunbeam dives to reach them. The glow score adds the weather: is there cloud there, does other cloud get in the way, and how much haze and damp air dull the colour. It is a number from 0 to 10 per cloud layer, at any moment.

**It is a rough guide, not a measurement.** Its numbers come from public knowledge and photographers' rules of thumb, listed below. None of it has been checked against real photos yet. That is the next step: logged observations with their saved conditions ([OBSERVATIONS.md](OBSERVATIONS.md)) will be used to tune these numbers, region by region.

## How the score is built

The score is 10 times eight factors multiplied together. Each factor is between 0 (kills the glow) and 1 (no effect):

| Factor | Question | Needs the forecast? |
|---|---|---|
| **light** | Is the layer lit overhead (1), only toward the sun (part), or dark (0)? | no |
| **sun** | With the sun still up, how sunset-coloured is its light? (1 once it has set) | no |
| **colour** | How rich is the colour, given how low the sunbeam dives on its way? | no |
| **cover** | Is there cloud at this height to catch the light? | yes |
| **path** | Does cloud at *other* heights stand in the sunbeam's way? | yes |
| **view** | Can you see the layer past lower cloud overhead? | yes |
| **haze** | Does haze (aerosol) dull the light? | yes |
| **humidity** | Does damp air add haze? | yes |

The first three, times 10, are the **potential**: the score if cloud were there and the air were clear. Without a forecast, that is all skyglow shows.

If a value is missing from the forecast, its factor counts as 1 (no effect) and the app lists it as "not known". If the cover at the layer's own height is missing, there is no score, only the potential.

### Which forecast level a layer uses

Forecasts give cloud cover at three levels. Open-Meteo defines them as low (up to 3 km), middle (3 to 8 km) and high (above 8 km), so each pack layer takes the cover of the level its height falls in.

### Path: what the sunbeam passes through

After sunset, the beam that lights a cloud overhead comes in from the sun's side and dives to its lowest point, the grazing height, before rising to the cloud. It passes through every height above that lowest point. So:

- cloud at another level **higher than the beam's lowest point** is in its way;
- cloud **below** the beam's lowest point cannot block it.

That matches the classic disappointment: when the beam dives low late in the evening, a deck of low cloud toward the sun cuts the light off the high cloud above. Early on, when the beam stays high, the same low cloud does not matter.

Each level in the way lets through `1 - cover x opacity` of the light. The opacity is how much a fully covered level stops: thick low cloud stops most, thin cirrus little.

### View: what you look through

To see a high cloud, you look up through any lower cloud overhead. Each lower level lets you see `1 - cover x opacity` of it.

### Haze and humidity: they matter most for low beams

A beam that skims a few hundred metres above the ground crosses far more hazy air than one that stays at 8 km. So haze and humidity count fully for a beam that skims the ground, and less as its lowest point rises (the `low_beam` weight).

## Where the numbers come from

All numbers are in the `glow` section of `packs/default.json`. A regional pack can override them; a pack without a `glow` section uses the default's.

| Numbers | Source or reasoning |
|---|---|
| Levels at 3 and 8 km | Open-Meteo's own definitions of its low, middle and high cloud cover, so the levels match the data. |
| Opacity: low 0.8, mid 0.6, high 0.3 | Cloud physics textbooks: thick low cloud (stratocumulus, stratus) has a large optical depth and stops most direct sunlight; thin cirrus often has an optical depth below about 1 to 3 and lets much of it through. The exact values are guesses. |
| Cover: best around 50 to 80%, poor at 0, a little lower at 100% | The common photographers' rule of thumb: broken cloud makes the best canvas. With no cloud there is nothing to light; a total overcast at one level often hides the gaps the light needs. |
| Colour by grazing height: richest for beams diving to about 1 to 6 km | The grazing-height idea in [PHYSICS.md](PHYSICS.md): high beams are golden, beams diving low are red and pink, and beams skimming the ground fade fast. The peak position is a guess to be tested. |
| Toward the sun only: 0.15 to 0.7 | Once Earth's shadow is overhead, only a band of the layer toward the sun is lit. A higher shadow edge means a bigger lit band. |
| Sun still up: 0.75 at sunset, nothing above about 15 degrees | Golden-hour light is warm but not yet the deep sunset colours. |
| Aerosol: no effect up to 0.1, about 0.7 at 0.5, 0.4 at 1 | S. F. Corfidi, "The Colors of Sunset and Twilight" (NOAA Storm Prediction Center): the most vivid twilight colours come with clean air; haze and pollution near the ground mute and wash them out. Typical aerosol optical depths: about 0.05 very clean, 0.5 hazy, above 1 smoke or dust. |
| Humidity: no effect up to 70%, then falling | Haze particles take up water and swell sharply above roughly 80% relative humidity (hygroscopic growth, standard aerosol science), so damp air is hazier. |
| Words: none, faint, modest, good, vivid | Labels for score bands, for display only. |

### A deliberate choice about haze

An earlier note in this project said "moderate haze boosts saturation". The default pack does **not** do that: it follows Corfidi, where clean air gives the best colours. The famous exceptions, the vivid purple twilights after big volcanic eruptions, come from a thin aerosol layer high in the stratosphere. A single ground-level aerosol number cannot tell that apart from haze near the ground. Factors stop at 1, so a pack can only make haze hurt less, not help. If observations show moderate haze really helping, that would need a new factor, added on evidence.

### A physics model that was not used

A tempting alternative is to compute how much light survives the trip, using the forecast aerosol depth and the long grazing path through the air near the ground. Done that way, even moderate haze leaves almost no light for any beam diving below a few kilometres. Yet vivid pink is commonly seen at exactly those times. One reason is that your eye judges colour against a sky that is itself getting darker fast, so a cloud lit by a tiny fraction of sunlight can still look brilliant. Until there is data to model that, a simple, explainable curve is more honest than a precise-looking formula that is wrong.

## What it does not know

- **Where the cloud is.** The forecast gives cover for the area around you, not along the sunbeam's path toward the sun, which runs for hundreds of kilometres. A clear gap on the horizon toward the sun, which can make a spectacular evening, is invisible to it.
- **Cloud type and thickness** beyond the three levels.
- **Your eyes and camera**: how bright and saturated something looks depends on exposure and on the sky around it.
- **Calibration**: the numbers above have not yet been fitted to observations.

A natural next step is to sample the forecast at points along the sun's direction, so the path factor uses the cloud actually in the beam's way.

## Where to see it

- **Web app:** the **Glow forecast** card gives the best moment around sunrise and sunset, and each layer's score at the time scrubber's minute, with a **Why** list of every factor. With a forecast, the colour strips fade each cloud row into the sky by how much the weather says it will show.
- **Command line:** with `--conditions` or `--open-meteo`, each `--at` time prints every layer's score and factors, and each report says when the best glow is.

In code: `core/src/glow.rs`.
