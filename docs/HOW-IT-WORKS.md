# How skyglow works, in plain language

No physics background needed. Each part builds on the one before. The matching code is in `core/src/`, and every file there has comments explaining the same ideas.

## 1. Where is the sun?

The Earth spins once a day and circles the sun once a year, and its axis is tilted about 23.4 degrees. From those three facts you can calculate exactly where the sun is, at any time, anywhere.

Two numbers matter:

- **Declination**: how far north or south of the equator the sun is overhead *today*. It swings between +23.4 degrees (June) and -23.4 degrees (December). This is what makes seasons.
- **Hour angle**: how far the sun has moved past its highest point (solar noon). It grows by 15 degrees every hour, because the Earth turns 360 degrees in 24 hours.

Add your latitude and you get the sun's **elevation** (how high it is above the horizon) and **azimuth** (which compass direction it is in, where 90 is east and 270 is west).

One small wrinkle: clocks and sundials disagree by up to about 16 minutes through the year. That difference is called the **equation of time**, and the code corrects for it. See `core/src/sun.rs`.

## 2. What is "sunset", and what comes after?

Sunset is when the sun's elevation crosses about **-0.83 degrees**. The extra bit is because the atmosphere bends light and the sun has a size, so you still see it a little after it is geometrically below the horizon.

After that the sky does not switch off. We split the evening into stages by how far below the horizon the sun is:

| Stage | Sun elevation | What the sky tends to do |
|---|---|---|
| golden hour | +6 to -0.8 degrees | Warm, low, directional light |
| sunset glow | -0.8 to -2 | Orange-red light on low and mid clouds |
| afterglow | -2 to -4 | High clouds still lit; the pink and magenta window |
| purple light | -4 to -6 | Fading violet-pink |
| blue hour | -6 to -8 | Deep blue, cloud colour mostly gone |

These boundaries are *estimates*. They live in small JSON files called packs (`packs/`) so they can be corrected without changing the code.

### Why timing depends on where you are

Near the equator the sun drops almost straight down, about 15 degrees an hour. Near the poles it slides along the horizon at a shallow angle. So the same 6-degree drop that takes about 20 minutes in Kerala can take well over 45 minutes in Iceland. Skyglow computes this for your exact latitude, instead of assuming "golden hour is an hour".

## 3. Earth's shadow climbs the sky

Picture a lamp shining on a ball in a dark room. Where the ball blocks the light, there is a shadow, and it is pointing away from the lamp.

At sunset *you* are on the shadow side of the Earth, but the air higher up is still in sunlight, because the Earth is round and its shadow only rises slowly. Clouds are inside that sunlit air. A cloud stays lit until the shadow climbs up to it.

Higher clouds see the sun for longer. For a cloud directly overhead, it stays lit until the sun is this far below *your* horizon:

| Cloud height | Goes dark overhead when the sun is below the horizon by |
|---|---|
| 2 km (low) | about 1.4 degrees |
| 6 km (middle) | about 2.5 degrees |
| 10 km (high) | about 3.2 degrees |
| 13 km (tropical high cloud) | about 3.6 degrees |

That is why clouds go grey from the bottom of the sky upwards and from the east side first, and why high wispy clouds can still glow when the ground below them is already dim. See `core/src/cloud.rs`.

One more consequence: a cloud toward the sunset horizon is *far away*, so it is lit longer than the same cloud overhead. The edge of the shadow slides toward the horizon as the sun sinks.

## 4. Why clouds turn orange, then pink

Clouds have no colour of their own. They reflect whatever light reaches them.

Sunlight is a mix of all colours. Air scatters blue light sideways far more than red, which is why the daytime sky is blue. Light that travels a long way through air has most of its blue stripped out and arrives yellow, orange, or red.

So the colour of a cloud depends on **how much air the light crossed to reach it**. Skyglow measures that with one number, the **grazing height**: the lowest altitude the sunbeam reaches on its way to the cloud.

- Beam stays high (thin air): light is gold or bright orange.
- Beam dips lower: orange, then orange-red.
- Beam skims close to the ground (hundreds of kilometres of dense air): deep red or pink, and dim.
- At the shadow edge the beam would touch the ground, and the cloud goes dark.

That sequence matches what happens in a real evening: gold, orange, red, then a last pink flush on the edge of a cloud, then nothing. The pink often comes late because it needs the beam to be skimming low, which only happens right before the shadow arrives.

## Sunrise is a sunset played backwards

Everything above works in the morning too. The sun climbs through the same angles in reverse order, so the stages run backwards: blue hour, a violet-pink glow, first light on the highest clouds, a short sunrise glow, then golden hour. Earth's shadow lifts instead of arriving, so the *highest* clouds catch the light first and low clouds last. Ask for it with `--event sunrise`, or just give a morning time with `--at`.

## 5. Height and mountains

Two things about where *you* stand change when *you* see the sun go, without changing the sky itself:

- **Height above sea level.** From higher up you can see farther, so the sea horizon sits slightly *below* eye level. The sun has to sink that much further before it touches it. The dip is about 0.0293 x the square root of your height in metres (2850 m gives roughly 1.6 degrees, so sunset comes a few minutes later near the equator).
- **Mountains in the way.** A ridge toward the sun hides it long before it reaches the true horizon. Skyglow lets you describe the skyline as a few `peaks` (bearing, distance, height) or as `horizon` points, and works out the angle for you.

Earth's shadow still arrives at the clouds on its own schedule, so high clouds keep glowing after the ground is in shade. That gap is the glow you see from a valley. See [PLACES.md](PLACES.md) for the format. Cloud heights in packs are heights above sea level, and the skyline is an approximation, so treat mountain results as a guide, not a measurement.

### The sky at one moment

Everything above can also be asked about a single instant instead of a whole evening: "it is 18:08, what is the sky doing?" Skyglow answers with where the sun is, whether it is climbing or sinking, whether a ridge hides it, the lowest height an overhead cloud needs to still catch light, and, for each cloud layer, whether it is lit overhead (and roughly what colour), lit only toward the sun (and where the shadow edge sits), or dark. Ask for it with `--at 18:08`. The planned web app's time slider will call the same calculation, so the two always agree. See `core/src/sky.rs`.

## 6. The honest list of what is missing

The sun position and the shadow geometry are exact. Everything about what the sky will *actually* look like depends on things Skyglow cannot know yet:

- **Which clouds exist tonight**, at what heights, and how thick. A solid layer of low cloud on the sunset side blocks the light before it reaches the high clouds.
- **Dust, smoke and pollution**, which change how saturated the colours are. A moderate amount boosts colour; heavy haze greys it out.
- **Moisture** in the air, which affects haze and how far clean light travels.
- **The exact colour labels.** The guesses about which grazing height looks pink or orange come from a tiny number of observations and need more.

A first step is in place: skyglow reads a weather forecast's cloud cover, humidity, water vapour and haze (the web app looks them up from the free Open-Meteo service; the command line reads them from a file, see [CONDITIONS.md](CONDITIONS.md)). It turns them into a rough **glow score** from 0 to 10 per cloud layer: is there cloud at that height, does other cloud block the sunbeam on its way (only cloud higher than the beam's lowest point can), can you see the layer past lower cloud, and how much haze and damp air dull the colour. Every score comes with all its reasons. The numbers behind it come from public rules of thumb and are not calibrated yet. See [GLOW.md](GLOW.md).

Planned work is to fill these in from live weather data and from photos taken by real people. This is the main reason the project is open source: more observations make better predictions.

## 7. A worked example

Kochi, 15 November 2026, clock offset +5.5:

```text
Sunset       17:59:34   (look toward azimuth 251°)
  low     1.5 km   dark overhead at 18:01:20   (sun 1.24° below horizon)
  mid     5.0 km   dark overhead at 18:05:43   (sun 2.27° below horizon)
  high    9.0 km   dark overhead at 18:09:02   (sun 3.04° below horizon)
```

Reading it as a story:

1. At 17:59 the sun touches the western horizon. You are about to lose direct light.
2. By 18:01 the sun is about 1 degree below the horizon. Low clouds overhead have gone grey.
3. By 18:05 middle clouds are going. High cloud overhead is still catching light.
4. At 18:09 even high cloud overhead is gone. Only clouds toward the sunset horizon, much farther away, are still lit.
5. By 18:21 the sun is 6 degrees down and the colour is basically over.

The whole show took 20 minutes. Near the equator that is normal, and it is why being late by ten minutes can mean missing the best of it.

## Glossary

| Term | Meaning |
|---|---|
| Azimuth | Compass direction of the sun, measured clockwise from north (east 90, south 180, west 270) |
| Civil twilight | The period from sunset until the sun is 6 degrees below the horizon |
| Declination | How far north or south of the equator the sun is overhead today |
| Depression | How far the sun is *below* the horizon (the opposite of elevation) |
| Elevation | How high the sun is above the horizon |
| Equation of time | The difference between clock time and sundial time, up to about 16 minutes |
| Grazing height | The lowest altitude a sunbeam reaches on its way to a cloud. It decides how red the light is |
| Hour angle | How far the sun has moved past solar noon, 15 degrees per hour |
| Pack | A small JSON file of tunable numbers (cloud heights, stage boundaries, colour hints) |
| Shadow edge | The boundary between sunlit and shadowed cloud, where Earth's shadow reaches |
| Solar noon | The moment the sun is at its highest and due north or south of you |
