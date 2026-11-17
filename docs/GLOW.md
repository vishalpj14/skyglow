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

Forecasts give cloud cover at three levels. Open-Meteo defines them as low (up to 3 km), middle (3 to 8 km) and high (above 8 km). Weather models set these levels by air pressure compared with the ground's, so they count from the ground up: at Leh (3.5 km up), "low" cloud is roughly 3.5 to 6.5 km above sea level. Pack layer heights are above the ground for the same reason, and each layer takes the cover of the level its height above the ground falls in. The top of the high level is the pack's `cloud_ceiling_km` (16 km).

### Likely cloud type

From the cover alone, skyglow names a likely cloud type for each level (`cloud_types` in the pack), using the usual families of the WMO cloud atlas: low cloud is cumulus, stratocumulus or stratus; middle cloud altocumulus or altostratus; high cloud cirrus, cirrocumulus or cirrostratus. Little cover suggests broken cloud (cumulus, altocumulus patches, cirrus streaks); near-total cover suggests a sheet (stratus, altostratus, a cirrostratus veil). This is a **guess**: forecasts do not say which type of cloud it is, and it does not change the score.

### Highest cloud overhead

The engine also names the highest level with more than a trace of cloud overhead (5% or more) and the heights that level spans, for example "high cloud (99%), somewhere from 8 to 16 km above sea level". Forecasts give cover per level, not cloud tops, so it cannot say where in that range the cloud is; the top of the range is the pack's `cloud_ceiling_km`, the highest a cloud can be. Open-Meteo also offers cloud cover at fixed pressure heights, but it is worked out from humidity and misses thin cirrus (it showed 0% at every height where the level forecast said 99% high cloud), so it is not used.

### Path: what the sunbeam passes through

After sunset, the beam that lights a cloud overhead comes in from the sun's side and dives to its lowest point, the grazing height, before rising to the cloud. So:

- cloud at another level **higher than the beam's lowest point** is in its way;
- cloud **below** the beam's lowest point cannot block it.

That matches the classic disappointment: when the beam dives low late in the evening, a deck of low cloud toward the sun cuts the light off the high cloud above. Early on, when the beam stays high, the same low cloud does not matter.

**Where it crosses.** Over a few hundred kilometres, Earth's curve is close to a parabola, so the beam's height `x` km toward the sun is `h + x tan(e) + x² / 2R` (cloud height `h`, sun elevation `e`, negative after sunset; Earth radius `R`). Setting that equal to another level's height gives the exact places the beam passes through it: twice for a level below the cloud (once going down, once coming back up), once for a level above it. With the sun 2° down, a beam to a 5 km cloud crosses 1.5 km at about 150 and 290 km toward the sun, and 9 km at about 540 km.

**Whose cloud.** Those crossings are far away, so the cover above *you* says little about them. skyglow therefore asks the forecast about points along the ground toward the sun at sunset (and at sunrise): by default 50, 150, 300, 450 and 600 km (`sunward_km` in the pack). At each crossing it reads that level's cover there, on a straight line between the nearest points; past the last point, the last value holds. Without those points (the command line with only local answers, or an older saved copy), the local cover stands in everywhere, and the app says so.

Each crossing lets through `1 - cover x opacity` of the light, and the crossings multiply. The opacity is how much a fully covered level stops: thick low cloud stops most, thin cirrus little.

**Lit only toward the sun.** When Earth's shadow is already overhead, the lit part of a layer is out toward the sun, at the shadow edge. Its cover is read there, not overhead, and its beam's crossings are counted from there.

### View: what you look through

To see a high cloud, you look up through any lower cloud overhead. Each lower level lets you see `1 - cover x opacity` of it.

### Haze and humidity: they matter most for low beams

A beam that skims a few hundred metres above the ground crosses far more hazy air than one that stays at 8 km. So haze and humidity count fully for a beam that skims the ground, and less as its lowest point rises (the `low_beam` weight).

## Where the numbers come from

All numbers are in the `glow` section of `packs/default.json`. A regional pack can override them; a pack without a `glow` section uses the default's.

| Numbers | Source or reasoning |
|---|---|
| Points at 50, 150, 300, 450, 600 km | Geometry: for clouds up to about 13 km and the sun up to about 5° down, the beam crosses the lower levels between roughly 100 and 600 km toward the sun. Five points keep the request small. |
| Levels at 3 and 8 km above the ground | Open-Meteo's own definitions of its low, middle and high cloud cover, so the levels match the data. |
| Ceiling at 16 km above sea level | About the tropopause (the top of the weather layer) in the tropics, its highest; cloud tops rarely go above it. |
| Cloud types by cover (1, 60, 90%) | The WMO cloud atlas's cloud families per level; the cover thresholds are a rough guess (broken cloud, mostly covered, a sheet). |
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

- **Where the cloud is, exactly.** Along the sun's direction it knows the forecast at five points, each a grid square several kilometres across, joined by straight lines. A narrow clear slot on the horizon can still slip between them. Forecast models also place cloud only roughly.
- **Cloud type and thickness** beyond the three levels.
- **Your eyes and camera**: how bright and saturated something looks depends on exposure and on the sky around it.
- **Calibration**: the numbers above have not yet been fitted to observations.

The sun's direction changes by a few degrees over the hour around sunset; the points follow the direction at the moment of sunset (or sunrise) itself.

Haze and humidity still come from your own place, not from along the way.

## Where to see it

- **Web app:** the **Glow forecast** card gives the best moment around sunrise and sunset, and each layer's score at the time scrubber's minute, with a **Why** list of every factor. With a forecast, the colour strips fade each cloud row into the sky by how much the weather says it will show.
- **Command line:** with `--conditions` or `--open-meteo`, each `--at` time prints every layer's score and factors, and each report says when the best glow is.

In code: `core/src/glow.rs`.
