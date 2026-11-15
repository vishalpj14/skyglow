# The physics of sunset colour

A plain-language tour of what Skyglow models, what it simplifies, and what it does not model yet.

## 1. Why the sun turns orange and red

Sunlight is white, a mix of all colours. Air molecules scatter short (blue) wavelengths far more than long (red) ones: **Rayleigh scattering**, which scales roughly with 1/wavelength^4. At midday the light travels through little air, so we see blue sky scattered all around and nearly white sun. Near the horizon the path through the atmosphere is many times longer, so most blue is scattered out of the direct beam. What is left reaching you is yellow, then orange, then red.

## 2. Clouds are screens, not light sources

A cloud has no colour of its own. It reflects whatever light reaches it. At sunset that light has already travelled a long way through air, so it is reddened. A cloud lit by that light glows orange or red. Because droplets and ice crystals are large compared to the wavelength (**Mie scattering**), clouds scatter all colours about equally, so they faithfully show the light's colour.

## 3. Why pink and magenta come *after* sunset

When the sun is below your horizon it still shines on the air above and beyond you. Earth's shadow climbs the sky from the east. Clouds higher up stay lit longer than low ones (see `core/src/cloud.rs`):

| Cloud height | Goes dark overhead when sun is below horizon by |
|---|---|
| 2 km | about 1.4 degrees |
| 6 km | about 2.5 degrees |
| 10 km | about 3.2 degrees |
| 13 km | about 3.6 degrees |

The light reaching high cloud has crossed a huge slice of atmosphere, losing almost all blue, and the cloud base is also tinted by blue-ish sky light from above, which mixes with the red to give pink and magenta. That is the **afterglow**, often the best colour of the evening.

## Why pink comes last: grazing height

The colour of light reaching a cloud depends on how much air it crossed. For a sunbeam on its way to a cloud, the lowest altitude it reaches (its **grazing height**) sets that:

```text
grazing height = (R + cloud height) * cos(sun elevation at the cloud) - R
```

Far from the shadow edge the beam passes through thin high air, so the cloud looks gold or orange. As Earth's shadow approaches, the beam dives lower and crosses hundreds of kilometres of dense air, so it arrives deep red or pink and dim. At the shadow edge the grazing height reaches zero and the cloud goes dark. That is why the vivid pink often appears on the last lit part of a cloud, just before it fades.

Example: for a 13 km tropical cloud directly overhead, the beam grazes about 6 km above the ground when the sun is 2.7 degrees below the horizon (gold to orange), about 1.6 km at 3.4 degrees (orange-red to pink), and reaches the ground at 3.66 degrees, where the cloud goes dark. The colour labels in the packs (`grazing_colours`) are rough, uncalibrated guesses that real observations will refine.

## 4. Why the equator is different

The sun descends at roughly `15 degrees/hour x cos(latitude)` near the horizon. Near the equator that is about 15 degrees per hour, so the window from sunset to civil twilight (sun 6 degrees below the horizon) lasts about 20 minutes. At 50 degrees north in November it takes well over 30. Intense, but short. Skyglow computes this for any latitude and date.

## 5. What spoils or boosts the show

- **Low cloud on the western horizon** blocks light from reaching the high cloud overhead. The classic "great clouds, no sunset" failure.
- **Aerosols** (dust, smoke, sea salt, pollution): a moderate amount increases saturation and redness; heavy haze dulls and greys everything. Aerosol optical depth (AOD) is the standard measure, available from forecast models.
- **Humidity** affects haze and how far light travels cleanly.
- **Sun-side clear gap:** if there is a clear slot at the horizon the sun can illuminate cloud undersides, giving the dramatic underlit look.

## 6. Modelled now vs later

| Effect | Status |
|---|---|
| Sun position, sunset and twilight times, any latitude | modelled (exact) |
| Earth-shadow timing for cloud layers | modelled (geometric) |
| Stage boundaries (golden, afterglow, ...) | estimated, in packs |
| Atmospheric refraction beyond the sunset constant | not yet |
| Real cloud heights from forecasts | planned (v2) |
| Aerosol effects on saturation | planned (v2) |
| Moisture: humidity and column water vapour (haze, tropical wet-season air) | planned (v2) |
| Grazing height as the colour variable | modelled (geometry), colour labels uncalibrated |
| True sky colour (radiative transfer) | planned, via lookup tables from the lab |

## Further reading

- Meeus, *Astronomical Algorithms* (solar position)
- NOAA solar calculator documentation
- Bohren and Huffman, *Absorption and Scattering of Light by Small Particles*
- Minnaert, *Light and Colour in the Outdoors*
