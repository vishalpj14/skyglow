//! The glow score: how much colour each cloud layer is likely to show.
//!
//! [`sky`](crate::sky) says which cloud heights *would* be lit, and with what
//! light. This module adds the weather: is there cloud at that height, does
//! other cloud block the sunbeam on its way, can you see the layer past
//! lower cloud, and how much haze and damp air dull the colour.
//!
//! The answer is a score from 0 to 10 per layer, built as a product of named
//! factors, each between 0 and 1:
//!
//! ```text
//! score = 10 x light x sun x colour x cover x path x view x haze x humidity
//! ```
//!
//! | Factor   | Question it answers                                              |
//! |----------|------------------------------------------------------------------|
//! | light    | Is the layer lit overhead (1), only toward the sun, or dark (0)? |
//! | sun      | With the sun still up, how sunset-coloured is the light?         |
//! | colour   | How rich is the colour for the beam's lowest height?             |
//! | cover    | Is there cloud at this height to catch the light?                |
//! | path     | Does cloud at other heights stand in the sunbeam's way?          |
//! | view     | Can you see this layer past lower cloud overhead?                |
//! | haze     | Does haze (aerosol) dull the light? Matters most for low beams.  |
//! | humidity | Does damp air add haze? Also matters most for low beams.         |
//!
//! The first three use only geometry; their product (times 10) is the
//! **potential**: the score if the cloud were there and the air clear. The
//! rest need [`Conditions`].
//!
//! **This is a rough guide, not a measurement.** The shape of every factor is
//! a curve in the pack (`glow` in `packs/default.json`), chosen from public
//! knowledge and rules of thumb (see `docs/GLOW.md`). None of it is
//! calibrated yet: that is what logged observations are for. Every score
//! carries all its factors, so it can always be explained.

use serde::{Deserialize, Serialize};

use crate::conditions::{Conditions, Weather};
use crate::params::Params;
use crate::place::Place;
use crate::sky::{self, LayerLight, SkyState};
use crate::sun::DEG;

/// The most instants one [`best_glow`] call will look at.
pub const MAX_BEST_POINTS: usize = 20_000;

/// A curve given as `[x, y]` points, joined by straight lines. Below the
/// first point and above the last, the end value holds.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Curve(pub Vec<[f64; 2]>);

impl Curve {
    /// The curve's value at `x`.
    pub fn at(&self, x: f64) -> f64 {
        let pts = &self.0;
        let first = pts[0];
        let last = pts[pts.len() - 1];
        if x <= first[0] {
            return first[1];
        }
        if x >= last[0] {
            return last[1];
        }
        for w in pts.windows(2) {
            let ([x0, y0], [x1, y1]) = (w[0], w[1]);
            if x <= x1 {
                return y0 + (y1 - y0) * (x - x0) / (x1 - x0);
            }
        }
        last[1]
    }

    fn validate(&self, name: &str) -> Result<(), String> {
        if self.0.is_empty() {
            return Err(format!("glow curve '{name}' needs at least one point"));
        }
        for p in &self.0 {
            if !(p[0].is_finite() && p[1].is_finite() && (0.0..=1.0).contains(&p[1])) {
                return Err(format!("glow curve '{name}': values must be numbers from 0 to 1"));
            }
        }
        if self.0.windows(2).any(|w| w[1][0] <= w[0][0]) {
            return Err(format!("glow curve '{name}': points must go left to right"));
        }
        Ok(())
    }
}

/// The three heights weather forecasts report cloud cover at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    Low,
    Mid,
    High,
}

impl Level {
    pub fn word(self) -> &'static str {
        match self {
            Level::Low => "low",
            Level::Mid => "middle",
            Level::High => "high",
        }
    }

    /// The forecast's cover for this level, percent.
    fn cover(self, c: &Conditions) -> Option<f64> {
        match self {
            Level::Low => c.cloud_low_pct,
            Level::Mid => c.cloud_mid_pct,
            Level::High => c.cloud_high_pct,
        }
    }
}

/// One number per forecast level.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PerLevel {
    pub low: f64,
    pub mid: f64,
    pub high: f64,
}

impl PerLevel {
    fn get(&self, level: Level) -> f64 {
        match level {
            Level::Low => self.low,
            Level::Mid => self.mid,
            Level::High => self.high,
        }
    }
}

/// A likely cloud type for a level's cover at or above `min_cover_pct`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CloudType {
    pub min_cover_pct: f64,
    pub label: String,
}

/// Likely cloud types per forecast level, lowest cover first. A rough guess
/// from the cover alone: forecasts do not say which type of cloud it is.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct CloudTypes {
    #[serde(default)]
    pub low: Vec<CloudType>,
    #[serde(default)]
    pub mid: Vec<CloudType>,
    #[serde(default)]
    pub high: Vec<CloudType>,
}

impl CloudTypes {
    fn list(&self, level: Level) -> &[CloudType] {
        match level {
            Level::Low => &self.low,
            Level::Mid => &self.mid,
            Level::High => &self.high,
        }
    }

    /// The likely type for `cover_pct` of `level`; `None` with no cloud.
    pub fn guess(&self, level: Level, cover_pct: f64) -> Option<&str> {
        if cover_pct <= 0.0 {
            return None;
        }
        self.list(level).iter().rev().find(|t| cover_pct >= t.min_cover_pct).map(|t| t.label.as_str())
    }
}

/// A word for scores at or above `min_score`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct GlowWord {
    pub min_score: f64,
    pub word: String,
}

/// The glow numbers from a pack. See `docs/GLOW.md` for where each comes from.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GlowParams {
    /// Free text about where the numbers came from.
    #[serde(default)]
    pub note: String,
    /// Forecast "low" cloud is below this height above the ground, km
    /// (Open-Meteo: 3).
    pub low_top_km: f64,
    /// Forecast "high" cloud is above this height above the ground, km
    /// (Open-Meteo: 8).
    pub high_base_km: f64,
    /// Distances toward the sun (km) at which to read the forecast, so the
    /// sunbeam's path uses the cloud actually there. Empty: this place only.
    #[serde(default)]
    pub sunward_km: Vec<f64>,
    /// Share of sunlight a fully covered level stops (0 to 1).
    pub opacity: PerLevel,
    /// Cover (percent) to how good a canvas the layer is.
    pub cover: Curve,
    /// Lowest height of the sunbeam (km) to how rich its colour is.
    pub colour: Curve,
    /// Height of Earth's shadow edge toward the sun (degrees) to the share of
    /// glow left when a layer is lit only toward the sun.
    pub toward_sun: Curve,
    /// Sun elevation (degrees, sun up) to how sunset-coloured its light is.
    pub sun_up: Curve,
    /// Lowest height of the sunbeam (km) to how much haze and humidity count:
    /// a beam skimming low crosses far more hazy air.
    pub low_beam: Curve,
    /// Aerosol optical depth to the light left after haze, for a low beam.
    pub aerosol: Curve,
    /// Relative humidity (percent) to the light left after damp haze, for a low beam.
    pub humidity: Curve,
    /// Words for score ranges, lowest first.
    pub words: Vec<GlowWord>,
    /// Likely cloud types from the cover (optional; none: no type shown).
    #[serde(default)]
    pub cloud_types: CloudTypes,
}

impl GlowParams {
    pub fn validate(&self) -> Result<(), String> {
        if !(self.low_top_km > 0.0 && self.high_base_km > self.low_top_km) {
            return Err("glow: low_top_km must be above 0 and below high_base_km".to_string());
        }
        if self.sunward_km.len() > 10
            || self.sunward_km.iter().any(|d| !(d.is_finite() && *d > 0.0 && *d <= 2000.0))
            || self.sunward_km.windows(2).any(|w| w[1] <= w[0])
        {
            return Err("glow: sunward_km must be up to 10 distances, each above 0 and at most 2000 km, nearest first".to_string());
        }
        for (name, v) in [("low", self.opacity.low), ("mid", self.opacity.mid), ("high", self.opacity.high)] {
            if !(0.0..=1.0).contains(&v) {
                return Err(format!("glow: opacity.{name} must be from 0 to 1"));
            }
        }
        for (name, c) in [
            ("cover", &self.cover),
            ("colour", &self.colour),
            ("toward_sun", &self.toward_sun),
            ("sun_up", &self.sun_up),
            ("low_beam", &self.low_beam),
            ("aerosol", &self.aerosol),
            ("humidity", &self.humidity),
        ] {
            c.validate(name)?;
        }
        for (name, list) in [("low", &self.cloud_types.low), ("mid", &self.cloud_types.mid), ("high", &self.cloud_types.high)] {
            if list.iter().any(|t| !(t.min_cover_pct > 0.0 && t.min_cover_pct <= 100.0))
                || list.windows(2).any(|w| w[1].min_cover_pct <= w[0].min_cover_pct)
            {
                return Err(format!(
                    "glow: cloud_types.{name} must go from low cover to high, each above 0 and at most 100 percent"
                ));
            }
        }
        if self.words.is_empty() || self.words[0].min_score > 0.0 {
            return Err("glow: words must start at min_score 0".to_string());
        }
        if self.words.windows(2).any(|w| w[1].min_score <= w[0].min_score) {
            return Err("glow: words must go from low scores to high".to_string());
        }
        Ok(())
    }

    /// Which forecast level a cloud belongs to, from its height above the ground.
    pub fn level_of(&self, above_ground_km: f64) -> Level {
        let height_km = above_ground_km;
        if height_km < self.low_top_km {
            Level::Low
        } else if height_km < self.high_base_km {
            Level::Mid
        } else {
            Level::High
        }
    }

    /// The word for a score.
    pub fn word(&self, score: f64) -> &str {
        self.words
            .iter()
            .rev()
            .find(|w| score >= w.min_score)
            .map_or("", |w| w.word.as_str())
    }
}

/// Every factor behind one layer's score. Factors that need conditions are
/// 1 (no effect) when the value is not known; `unknown` lists those.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Factors {
    pub light: f64,
    pub sun: f64,
    pub colour: f64,
    /// `None` when this level's cover is not known.
    pub cover: Option<f64>,
    pub path: f64,
    pub view: f64,
    pub haze: f64,
    pub humidity: f64,
}

/// One place where the sunbeam crosses another cloud level on its way.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Crossing {
    pub level: Level,
    /// Distance from you toward the sun, along the ground, km.
    pub distance_km: f64,
    /// The forecast cover of that level there, percent, if known.
    pub cover_pct: Option<f64>,
}

/// The glow of one cloud layer at one instant.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LayerGlow {
    pub name: String,
    /// Height above sea level, km.
    pub height_km: f64,
    /// Height above the place's ground, km.
    pub above_ground_km: f64,
    /// The forecast level whose cover this layer uses.
    pub level: Level,
    /// The heights that level spans here, km above sea level, bottom and top
    /// (forecasts count levels from the ground; the top of high cloud is the
    /// pack's ceiling).
    pub range_km: [f64; 2],
    /// A likely cloud type for the forecast cover (a rough guess from the
    /// cover alone). `None` without a forecast or with no cloud.
    pub cloud_type: Option<String>,
    /// The lowest height (km) the sunbeam reaching the visible part of this
    /// layer dips to: the grazing height overhead, 0 at a shadow edge toward
    /// the sun. `None` while unlit.
    pub beam_low_km: Option<f64>,
    /// How far toward the sun the lit part you see is, km: 0 overhead, the
    /// shadow-edge distance when lit only toward the sun.
    pub cover_at_km: f64,
    /// The forecast cover of this layer's level there, percent.
    pub cover_pct: Option<f64>,
    pub factors: Factors,
    /// Geometry only: the score if cloud were there and the air clear, 0 to 10.
    pub potential: f64,
    /// 0 to 10, or `None` when there are no conditions or this level's cover
    /// is not known.
    pub score: Option<f64>,
    /// The pack's word for the score.
    pub word: Option<String>,
    /// Levels whose cloud stands in the sunbeam's way (counted in `path`).
    pub blocked_by: Vec<Level>,
    /// Every place the beam crosses another level, nearest first; each one
    /// lets through `1 - cover x opacity` of the light.
    pub crossings: Vec<Crossing>,
    /// Lower levels you look through to see this layer (counted in `view`).
    pub seen_through: Vec<Level>,
    /// Values that were not known and so had no effect, in words.
    pub unknown: Vec<String>,
}

/// The glow of every layer at one instant.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Glow {
    pub jd: f64,
    /// The highest cloud overhead in the forecast, `None` without conditions
    /// or when every level is (nearly) clear.
    pub highest_cloud: Option<HighestCloud>,
    /// False when no conditions were given: only `potential` is meaningful.
    pub has_conditions: bool,
    /// True when readings along the sun's direction were used for the
    /// sunbeam's path; false when the local forecast stood in for them.
    pub along_sun: bool,
    pub layers: Vec<LayerGlow>,
}

/// Cover below this (percent) counts as a trace, not cloud, for
/// [`HighestCloud`]: blending forecast hours leaves values such as 0.3%.
pub const TRACE_COVER_PCT: f64 = 5.0;

/// The highest forecast level with cloud overhead. Forecasts give cover per
/// level, not cloud tops, so where in the range the cloud sits is not known.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HighestCloud {
    pub level: Level,
    pub cover_pct: f64,
    /// The heights that level spans here, km above sea level; the top of the
    /// high level is the pack's ceiling, the highest a cloud can be.
    pub range_km: [f64; 2],
}

impl Glow {
    /// The layer with the highest score, if any score is known.
    pub fn best(&self) -> Option<&LayerGlow> {
        self.layers
            .iter()
            .filter(|l| l.score.is_some())
            .fold(None, |best: Option<&LayerGlow>, l| match best {
                Some(b) if b.score >= l.score => Some(b),
                _ => Some(l),
            })
    }
}

/// Where a straight sunbeam heading to a cloud at `h_km` crosses the height
/// `level_km`, as distances from the cloud toward the sun, km, nearest first.
///
/// Over the few hundred km that matter, Earth's curve is close to a
/// parabola: x km toward the sun, the beam is `h + x tan(e) + x^2 / 2R` above
/// the ground, where `e` is the sun's elevation at the cloud (negative after
/// sunset). Its lowest point is `R tan(-e)` away at `h - R tan^2(e) / 2`,
/// which matches the exact grazing height to well under 1%.
pub fn beam_crossings_km(h_km: f64, sun_elevation_deg: f64, level_km: f64, r: f64) -> Vec<f64> {
    let t = (sun_elevation_deg * DEG).tan();
    let disc = t * t - 2.0 * (h_km - level_km) / r;
    if disc < 0.0 {
        return Vec::new();
    }
    let s = disc.sqrt();
    let mut xs: Vec<f64> = [-t - s, -t + s].into_iter().map(|k| k * r).filter(|x| *x > 1e-6).collect();
    xs.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
    xs
}

/// Cloud cover of `level` at `distance_km` toward the sun: a straight line
/// between the reading here (0 km) and those along the way. Past the last
/// reading, the last value holds. `None` if no reading knows this level.
fn cover_along(level: Level, distance_km: f64, here: &Conditions, along: &[(f64, Conditions)]) -> Option<f64> {
    let pts: Vec<[f64; 2]> = std::iter::once((0.0, here))
        .chain(along.iter().map(|(d, c)| (*d, c)))
        .filter_map(|(d, c)| level.cover(c).map(|v| [d, v]))
        .collect();
    (!pts.is_empty()).then(|| Curve(pts).at(distance_km))
}

/// Work out the glow of every layer in `sky`, with the weather here and,
/// if known, along the sun's direction (`along`: distance km and readings,
/// nearest first). With no readings along the way, the local forecast stands
/// in for the whole path.
pub fn glow(params: &Params, sky: &SkyState, here: Option<&Conditions>, along: &[(f64, Conditions)]) -> Glow {
    let g = &params.glow;
    let r = params.earth_radius_km;
    // Each pack layer's level, and for each level the lowest and highest
    // pack layer in it (a level is counted once, however many layers share it).
    let levels: Vec<Level> = sky.layers.iter().map(|l| g.level_of(l.above_ground_km)).collect();
    let span = |level: Level| {
        let hs = sky.layers.iter().zip(&levels).filter(|(_, lv)| **lv == level).map(|(l, _)| l.height_km);
        hs.fold(None, |acc: Option<(f64, f64)>, h| Some(acc.map_or((h, h), |(lo, hi)| (lo.min(h), hi.max(h)))))
    };
    let mut present: Vec<Level> = levels.clone();
    present.sort();
    present.dedup();

    let sun = if sky.elevation_deg >= 0.0 { g.sun_up.at(sky.elevation_deg) } else { 1.0 };

    let layers: Vec<LayerGlow> = sky
        .layers
        .iter()
        .zip(&levels)
        .map(|(layer, &level)| {
            // Where the lit part you see is (km toward the sun), and the
            // sun's elevation there: it stands higher the closer you get to it.
            let (light, beam_low_km, at_km, beam_elev) = match &layer.light {
                LayerLight::LitOverhead { grazing_km, .. } => (1.0, Some(*grazing_km), 0.0, Some(sky.elevation_deg)),
                LayerLight::LitTowardSun { edge_elevation_deg, edge_distance_km } => (
                    g.toward_sun.at(*edge_elevation_deg),
                    Some(0.0),
                    *edge_distance_km,
                    Some(sky.elevation_deg + edge_distance_km / r / DEG),
                ),
                LayerLight::EdgeBeyondHorizon | LayerLight::ShadowOverhead => (0.0, None, 0.0, None),
            };
            let colour = beam_low_km.map_or(0.0, |z| g.colour.at(z));
            let potential = 10.0 * light * sun * colour;

            let ground = layer.height_km - layer.above_ground_km;
            let top = params.cloud_ceiling_km;
            let range_km = match level {
                Level::Low => [ground, ground + g.low_top_km],
                Level::Mid => [ground + g.low_top_km, ground + g.high_base_km],
                Level::High => [ground + g.high_base_km, top],
            }
            .map(|h| h.min(top));
            let mut out = LayerGlow {
                name: layer.name.clone(),
                height_km: layer.height_km,
                above_ground_km: layer.above_ground_km,
                level,
                range_km,
                cloud_type: None,
                beam_low_km,
                cover_at_km: at_km,
                cover_pct: None,
                factors: Factors { light, sun, colour, cover: None, path: 1.0, view: 1.0, haze: 1.0, humidity: 1.0 },
                potential,
                score: None,
                word: None,
                blocked_by: Vec::new(),
                crossings: Vec::new(),
                seen_through: Vec::new(),
                unknown: Vec::new(),
            };
            let Some(c) = here else { return out };
            let stops = |level: Level, pct: f64| 1.0 - pct / 100.0 * g.opacity.get(level);

            for &other in present.iter().filter(|&&o| o != level) {
                let (lowest, highest) = span(other).expect("present levels have layers");
                // The sunbeam: every place it crosses this other level's height.
                if let Some(e) = beam_elev {
                    let xs = beam_crossings_km(layer.height_km, e, highest, r);
                    if !xs.is_empty() {
                        out.blocked_by.push(other);
                    }
                    for x in xs {
                        let distance_km = at_km + x;
                        let cover_pct = cover_along(other, distance_km, c, along);
                        match cover_pct {
                            Some(pct) => out.factors.path *= stops(other, pct),
                            None => out.unknown.push(format!("{} cloud cover", other.word())),
                        }
                        out.crossings.push(Crossing { level: other, distance_km, cover_pct });
                    }
                }
                // Looking up, you see through cloud below this layer, above you.
                if lowest < layer.height_km {
                    out.seen_through.push(other);
                    match other.cover(c) {
                        Some(pct) => out.factors.view *= stops(other, pct),
                        None => out.unknown.push(format!("{} cloud cover", other.word())),
                    }
                }
            }
            out.crossings.sort_by(|a, b| a.distance_km.total_cmp(&b.distance_km));

            let weight = beam_low_km.map_or(0.0, |z| g.low_beam.at(z));
            match c.aerosol_optical_depth {
                Some(aod) => out.factors.haze = 1.0 - weight * (1.0 - g.aerosol.at(aod)),
                None => out.unknown.push("haze (aerosol optical depth)".to_string()),
            }
            match c.humidity_pct {
                Some(rh) => out.factors.humidity = 1.0 - weight * (1.0 - g.humidity.at(rh)),
                None => out.unknown.push("humidity".to_string()),
            }
            out.unknown.sort();
            out.unknown.dedup();

            match cover_along(level, at_km, c, along) {
                Some(pct) => {
                    let f = &mut out.factors;
                    out.cover_pct = Some(pct);
                    out.cloud_type = g.cloud_types.guess(level, pct).map(str::to_string);
                    f.cover = Some(g.cover.at(pct));
                    let score = potential * g.cover.at(pct) * f.path * f.view * f.haze * f.humidity;
                    out.word = Some(g.word(score).to_string());
                    out.score = Some(score);
                }
                None => out.unknown.insert(0, format!("{} cloud cover (needed for this layer's score)", level.word())),
            }
            out
        })
        .collect();

    let highest_cloud = here.and_then(|c| {
        [Level::High, Level::Mid, Level::Low].into_iter().find_map(|level| {
            let pct = level.cover(c)?;
            let layer = layers.iter().find(|l: &&LayerGlow| l.level == level)?;
            (pct >= TRACE_COVER_PCT).then(|| HighestCloud { level, cover_pct: pct, range_km: layer.range_km })
        })
    });
    Glow { jd: sky.jd, highest_cloud, has_conditions: here.is_some(), along_sun: here.is_some() && !along.is_empty(), layers }
}

/// The sky and its glow at one instant, with the weather read at that time:
/// here, and along the sun's direction (the sunrise points while the sun is
/// climbing, the sunset points while it is sinking).
pub fn glow_at(params: &Params, place: &Place, weather: Option<&Weather>, jd: f64) -> Glow {
    let st = sky::sky_state(params, place, jd);
    let here = weather.and_then(|w| w.here_at(jd));
    let along = weather.map_or_else(Vec::new, |w| w.along_at(st.rising, jd));
    glow(params, &st, here.as_ref(), &along)
}

/// The moment with the highest glow score in a window.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GlowMoment {
    pub jd: f64,
    /// The layer that glows most then.
    pub layer: String,
    pub height_km: f64,
    pub score: f64,
    pub word: String,
}

/// The best glow between `jd_start` and `jd_end` (both included), checked
/// every `step_minutes`. `Ok(None)` when no score is known anywhere in the
/// window (no conditions close enough). Ties go to the earlier moment.
pub fn best_glow(
    params: &Params,
    place: &Place,
    weather: &Weather,
    jd_start: f64,
    jd_end: f64,
    step_minutes: f64,
) -> Result<Option<GlowMoment>, String> {
    if !(jd_start.is_finite() && jd_end.is_finite() && jd_end >= jd_start) {
        return Err("the window needs a start before its end".to_string());
    }
    if !(step_minutes.is_finite() && step_minutes > 0.0) {
        return Err("the step must be more than 0 minutes".to_string());
    }
    let step = step_minutes / 1440.0;
    // Slack of a millionth of a step: see wasm sky_series.
    let count = ((jd_end - jd_start) / step + 1e-6).floor() as usize + 1;
    if count > MAX_BEST_POINTS {
        return Err(format!("{count} moments to check; the limit is {MAX_BEST_POINTS}, use a larger step"));
    }
    let mut best: Option<GlowMoment> = None;
    for i in 0..count {
        let jd = jd_start + i as f64 * step;
        let g = glow_at(params, place, Some(weather), jd);
        if let Some(l) = g.best() {
            let score = l.score.expect("best has a score");
            if best.as_ref().map_or(true, |b| score > b.score) {
                best = Some(GlowMoment {
                    jd,
                    layer: l.name.clone(),
                    height_km: l.height_km,
                    score,
                    word: l.word.clone().unwrap_or_default(),
                });
            }
        }
    }
    Ok(best)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conditions::Sample;
    use crate::params::CloudLayer;
    use crate::sun;

    const R: f64 = 6371.0;

    /// Round, easy numbers so every expected value below can be worked by
    /// hand, independent of the shipped pack.
    fn simple_glow() -> GlowParams {
        let c = |pts: &[[f64; 2]]| Curve(pts.to_vec());
        GlowParams {
            note: String::new(),
            low_top_km: 3.0,
            high_base_km: 8.0,
            opacity: PerLevel { low: 0.5, mid: 0.5, high: 0.5 },
            cover: c(&[[0.0, 0.0], [100.0, 1.0]]),
            colour: c(&[[0.0, 1.0]]),
            toward_sun: c(&[[0.0, 0.5]]),
            sun_up: c(&[[0.0, 1.0], [10.0, 0.0]]),
            low_beam: c(&[[0.0, 1.0], [10.0, 0.0]]),
            aerosol: c(&[[0.0, 1.0], [1.0, 0.0]]),
            humidity: c(&[[0.0, 1.0], [100.0, 0.5]]),
            sunward_km: Vec::new(),
            cloud_types: CloudTypes::default(),
            words: vec![
                GlowWord { min_score: 0.0, word: "none".into() },
                GlowWord { min_score: 4.5, word: "good".into() },
            ],
        }
    }

    fn pack(heights: &[(&str, f64)], glow: GlowParams) -> Params {
        let mut p = Params::builtin();
        p.earth_radius_km = R;
        p.cloud_layers = heights.iter().map(|(n, h)| CloudLayer { name: n.to_string(), height_km: *h }).collect();
        p.glow = glow;
        p
    }

    /// The instant on 15 Nov 2026 in Kochi when the evening sun is at `deg`.
    fn kochi_evening_at(deg: f64) -> f64 {
        let noon = sun::solar_noon(2026, 11, 15, 76.27, 5.5);
        sun::evening_crossing(noon, 9.93, 76.27, deg).unwrap()
    }

    fn kochi() -> Place {
        Place::flat(9.93, 76.27)
    }

    fn weather() -> Conditions {
        Conditions {
            cloud_low_pct: Some(20.0),
            cloud_mid_pct: Some(40.0),
            cloud_high_pct: Some(60.0),
            humidity_pct: Some(50.0),
            water_vapour_mm: None,
            aerosol_optical_depth: Some(0.2),
        }
    }

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn curves_join_points_with_straight_lines() {
        let c = Curve(vec![[0.0, 0.0], [10.0, 1.0], [20.0, 0.5]]);
        assert_eq!(c.at(5.0), 0.5);
        assert_eq!(c.at(15.0), 0.75);
        assert_eq!(c.at(-3.0), 0.0, "before the first point, the first value holds");
        assert_eq!(c.at(99.0), 0.5, "after the last point, the last value holds");
        assert_eq!(Curve(vec![[4.0, 0.3]]).at(-50.0), 0.3);
    }

    #[test]
    fn heights_map_to_forecast_levels() {
        let g = simple_glow();
        assert_eq!(g.level_of(1.5), Level::Low);
        assert_eq!(g.level_of(3.0), Level::Mid, "3 km is the bottom of the middle level");
        assert_eq!(g.level_of(7.9), Level::Mid);
        assert_eq!(g.level_of(13.0), Level::High);
    }

    #[test]
    fn at_a_high_place_levels_count_from_the_ground() {
        // Ground 3 km up. The pack's 1.5 / 5 / 9 km are above the ground, so
        // the layers sit at 4.5 / 8 / 12 km above sea level. Forecast levels
        // also count from the ground: low 3 to 6 km, middle 6 to 11 km,
        // high 11 km up to the 16 km ceiling.
        let mut p = pack(&[("low", 1.5), ("mid", 5.0), ("high", 9.0)], simple_glow());
        p.cloud_ceiling_km = 16.0;
        p.glow.cloud_types.high = vec![
            CloudType { min_cover_pct: 1.0, label: "cirrus streaks".into() },
            CloudType { min_cover_pct: 50.0, label: "cirrostratus".into() },
        ];
        let mut place = kochi();
        place.elevation_m = 3000.0;
        let st = sky::sky_state(&p, &place, kochi_evening_at(-2.0));
        let g = glow(&p, &st, Some(&weather()), &[]);
        let (low, mid, high) = (&g.layers[0], &g.layers[1], &g.layers[2]);

        assert!(close(low.height_km, 4.5, 1e-9) && close(low.above_ground_km, 1.5, 1e-9));
        // At sea level 4.5 km would be middle cloud; here it is low.
        assert_eq!((low.level, mid.level, high.level), (Level::Low, Level::Mid, Level::High));
        assert_eq!(low.range_km, [3.0, 6.0]);
        assert_eq!(mid.range_km, [6.0, 11.0]);
        assert_eq!(high.range_km, [11.0, 16.0]);
        // High cover 60%: at or above 50, so the second type. The test
        // params have no low or middle types.
        assert_eq!(high.cloud_type.as_deref(), Some("cirrostratus"));
        assert_eq!(low.cloud_type, None);
        // No cloud, no type; no forecast, no type.
        assert_eq!(p.glow.cloud_types.guess(Level::High, 0.0), None);
        assert_eq!(p.glow.cloud_types.guess(Level::High, 10.0), Some("cirrus streaks"));
        assert_eq!(glow(&p, &st, None, &[]).layers[2].cloud_type, None);
    }

    #[test]
    fn the_highest_cloud_is_the_top_level_with_more_than_a_trace() {
        // Sea level, default levels: high cloud spans 8 km to the 16 km ceiling.
        let mut p = pack(&[("low", 1.5), ("mid", 5.0), ("high", 9.0)], simple_glow());
        p.cloud_ceiling_km = 16.0;
        let st = sky::sky_state(&p, &kochi(), kochi_evening_at(-2.0));
        // weather(): high 60% is the top level with cloud.
        let h = glow(&p, &st, Some(&weather()), &[]).highest_cloud.unwrap();
        assert_eq!((h.level, h.cover_pct, h.range_km), (Level::High, 60.0, [8.0, 16.0]));
        // A 4% trace of high cloud does not count: middle (40%, 3 to 8 km) is the highest.
        let trace = Conditions { cloud_high_pct: Some(4.0), ..weather() };
        let h = glow(&p, &st, Some(&trace), &[]).highest_cloud.unwrap();
        assert_eq!((h.level, h.range_km), (Level::Mid, [3.0, 8.0]));
        // Clear, or no forecast: none.
        let clear = Conditions { cloud_low_pct: Some(0.0), cloud_mid_pct: Some(0.0), cloud_high_pct: Some(0.0), ..weather() };
        assert_eq!(glow(&p, &st, Some(&clear), &[]).highest_cloud, None);
        assert_eq!(glow(&p, &st, None, &[]).highest_cloud, None);
    }

    #[test]
    fn the_ceiling_caps_a_layer_at_a_very_high_place() {
        // Ground 8 km: high cloud 9 km above it would be 17 km, past the
        // 16 km ceiling, so it sits at 16 km, 8 km above the ground.
        let mut p = pack(&[("high", 9.0)], simple_glow());
        p.cloud_ceiling_km = 16.0;
        let mut place = kochi();
        place.elevation_m = 8000.0;
        let st = sky::sky_state(&p, &place, kochi_evening_at(-2.0));
        assert!(close(st.layers[0].height_km, 16.0, 1e-9));
        assert!(close(st.layers[0].above_ground_km, 8.0, 1e-9));
    }

    #[test]
    fn scores_match_a_hand_calculation() {
        // Sun 2 deg down, R = 6371 km. Lowest point of the beam to a cloud
        // overhead at h: (R + h) cos 2deg - R, with cos 2deg = 0.9993908:
        //   high 9 km: 6380 x 0.9993908 - 6371 = 5.114 km
        //   mid  5 km: 6376 x 0.9993908 - 6371 = 1.116 km
        // Weather: low 20%, mid 40%, high 60%, aerosol 0.2, humidity 50%.
        // A level stops cover x 0.5: low 0.1, mid 0.2, high 0.3.
        let p = pack(&[("low", 1.5), ("mid", 5.0), ("high", 9.0)], simple_glow());
        let st = sky::sky_state(&p, &kochi(), kochi_evening_at(-2.0));
        let g = glow(&p, &st, Some(&weather()), &[]);
        let (mid, high) = (&g.layers[1], &g.layers[2]);

        // HIGH: the beam's lowest point (5.11 km) is above the mid layer (5.0)
        // and the low one, so nothing is in its way: path 1. You look up
        // through low and mid: view 0.9 x 0.8 = 0.72. Haze and humidity count
        // with weight 1 - 5.114 / 10 = 0.4886: haze 1 - 0.4886 x 0.2 = 0.9023,
        // humidity 1 - 0.4886 x 0.25 = 0.8778. Cover 60% gives 0.6.
        // Score 10 x 0.6 x 0.72 x 0.9023 x 0.8778 = 3.42.
        assert!(close(high.beam_low_km.unwrap(), 5.114, 0.01));
        assert_eq!(high.factors.path, 1.0);
        assert!(high.blocked_by.is_empty());
        assert_eq!(high.seen_through, vec![Level::Low, Level::Mid]);
        assert!(close(high.factors.view, 0.72, 1e-9));
        assert!(close(high.factors.haze, 0.9023, 0.001));
        assert!(close(high.factors.humidity, 0.8778, 0.001));
        assert!(close(high.score.unwrap(), 3.42, 0.01), "high = {:?}", high.score);
        assert_eq!(high.word.as_deref(), Some("none"));

        // MID: the beam dips to 1.12 km, below the low layer's 1.5 km, so it
        // crosses the low level twice (going down, then coming back up), and
        // it comes in from above through the high level once. With only the
        // local forecast, every crossing sees the local cover:
        // path 0.9 x 0.9 x 0.7 = 0.567. View through low only: 0.9.
        // Weight 1 - 0.1116 = 0.8884: haze 1 - 0.8884 x 0.2 = 0.8223,
        // humidity 1 - 0.8884 x 0.25 = 0.7779.
        // Score 10 x 0.4 x 0.567 x 0.9 x 0.8223 x 0.7779 = 1.31.
        assert_eq!(mid.blocked_by, vec![Level::Low, Level::High]);
        assert!(close(mid.factors.path, 0.567, 1e-9));
        assert!(close(mid.factors.view, 0.9, 1e-9));
        assert!(close(mid.score.unwrap(), 1.306, 0.01), "mid = {:?}", mid.score);
        // Where: tan 2deg = 0.034921, so with t = -0.034921 the beam's height
        // x km toward the sun is 5 + t x + x^2 / 12742. Setting it to 1.5:
        // x = 6371 (0.034921 -+ 0.010991) = 152.5 and 292.5 km. Setting it
        // to 9: x = 6371 (0.034921 + 0.049752) = 539.5 km.
        let at: Vec<(Level, f64)> = mid.crossings.iter().map(|c| (c.level, c.distance_km)).collect();
        assert_eq!(at.len(), 3);
        for ((level, d), (want_level, want_d)) in at.iter().zip([(Level::Low, 152.5), (Level::Low, 292.5), (Level::High, 539.5)]) {
            assert_eq!(*level, want_level);
            assert!(close(*d, want_d, 1.0), "crossing at {d} km, expected {want_d}");
        }
        assert!(!g.along_sun);

        assert_eq!(g.best().unwrap().name, "high");
    }

    #[test]
    fn without_weather_only_the_potential_is_known() {
        let p = pack(&[("high", 9.0)], simple_glow());
        let st = sky::sky_state(&p, &kochi(), kochi_evening_at(-2.0));
        let g = glow(&p, &st, None, &[]);
        assert!(!g.has_conditions);
        // Lit overhead, sun down, flat colour curve: 10 x 1 x 1 x 1.
        assert_eq!(g.layers[0].potential, 10.0);
        assert_eq!(g.layers[0].score, None);
        assert!(g.best().is_none());
    }

    #[test]
    fn a_clear_sky_has_nothing_to_glow() {
        let p = pack(&[("low", 1.5), ("mid", 5.0), ("high", 9.0)], simple_glow());
        let st = sky::sky_state(&p, &kochi(), kochi_evening_at(-2.0));
        let clear = Conditions { cloud_low_pct: Some(0.0), cloud_mid_pct: Some(0.0), cloud_high_pct: Some(0.0), ..Conditions::default() };
        let g = glow(&p, &st, Some(&clear), &[]);
        assert!(g.layers.iter().all(|l| l.score == Some(0.0) && l.word.as_deref() == Some("none")));
        // Unknown haze and humidity count as no effect, and are listed.
        assert!(g.layers[2].unknown.iter().any(|u| u.contains("haze")));
        assert_eq!(g.layers[2].factors.haze, 1.0);
    }

    #[test]
    fn missing_cover_for_a_layer_leaves_its_score_unknown() {
        let p = pack(&[("low", 1.5), ("high", 9.0)], simple_glow());
        let st = sky::sky_state(&p, &kochi(), kochi_evening_at(-2.0));
        let partial = Conditions { cloud_low_pct: Some(20.0), ..Conditions::default() };
        let g = glow(&p, &st, Some(&partial), &[]);
        assert_eq!(g.layers[1].score, None);
        assert!(g.layers[1].unknown[0].starts_with("high cloud cover"));
        assert!(g.layers[0].score.is_some(), "the low layer's own cover is known");
    }

    #[test]
    fn a_shadow_edge_toward_the_sun_gives_part_of_the_glow_and_a_ground_level_beam() {
        // Sun 5 deg down, 10 km cloud: lit only toward the sun (see sky.rs).
        // The beam at the edge skims the ground: lowest point 0, haze weight 1.
        // light 0.5 (toward_sun), cover 60% -> 0.6, haze 1 - 1 x 0.2 = 0.8,
        // humidity 1 - 1 x 0.25 = 0.75, nothing else in the way.
        // Score 10 x 0.5 x 0.6 x 0.8 x 0.75 = 1.8.
        let p = pack(&[("high", 10.0)], simple_glow());
        let st = sky::sky_state(&p, &kochi(), kochi_evening_at(-5.0));
        assert!(matches!(st.layers[0].light, LayerLight::LitTowardSun { .. }));
        let l = &glow(&p, &st, Some(&weather()), &[]).layers[0];
        assert_eq!(l.beam_low_km, Some(0.0));
        assert_eq!(l.factors.light, 0.5);
        assert!(close(l.score.unwrap(), 1.8, 1e-9));
    }

    #[test]
    fn a_high_sun_gives_no_sunset_glow_and_a_dark_layer_gives_none() {
        let p = pack(&[("low", 1.5), ("high", 9.0)], simple_glow());
        // Midday: sun about 60 deg up, beyond the sun_up curve's 10 deg.
        let noon = sun::solar_noon(2026, 11, 15, 76.27, 5.5);
        let g = glow(&p, &sky::sky_state(&p, &kochi(), noon), Some(&weather()), &[]);
        assert!(g.layers.iter().all(|l| l.factors.sun == 0.0 && l.score == Some(0.0)));
        // Sun 5 deg up: sun factor 1 - 5/10 = 0.5. A low cloud sees the beam
        // at its own height, so the high cloud above (9 km >= 1.5) is in its way.
        let g = glow(&p, &sky::sky_state(&p, &kochi(), kochi_evening_at(5.0)), Some(&weather()), &[]);
        assert!(close(g.layers[0].factors.sun, 0.5, 1e-3));
        assert_eq!(g.layers[0].blocked_by, vec![Level::High]);
        // Deep night: every layer dark.
        let g = glow(&p, &sky::sky_state(&p, &kochi(), kochi_evening_at(-30.0)), Some(&weather()), &[]);
        assert!(g.layers.iter().all(|l| l.factors.light == 0.0 && l.beam_low_km.is_none()));
    }

    #[test]
    fn the_best_moment_is_when_the_beam_dips_to_the_richest_colour() {
        // Colour peaks for a beam dipping to 3 km. For a 9 km cloud overhead
        // that happens when (6371 + 9) cos d = 6374, cos d = 0.9990596,
        // d = 2.485 deg below the horizon. Nothing else changes over the
        // window, so that must be the best moment.
        let mut g = simple_glow();
        g.colour = Curve(vec![[0.0, 0.0], [3.0, 1.0], [12.0, 0.0]]);
        g.sun_up = Curve(vec![[0.0, 0.0]]);
        g.toward_sun = Curve(vec![[0.0, 0.0]]);
        let p = pack(&[("high", 9.0)], g);
        let start = kochi_evening_at(1.0);
        let end = kochi_evening_at(-6.0);
        let still = Conditions { cloud_high_pct: Some(50.0), ..Conditions::default() };
        let series = crate::conditions::ConditionsSeries::new(
            "test",
            vec![Sample { jd: start - 0.05, conditions: still }, Sample { jd: end + 0.05, conditions: still }],
        )
        .unwrap();
        let series = Weather::here_only(series);
        let best = best_glow(&p, &kochi(), &series, start, end, 0.1).unwrap().unwrap();
        let depression = -sun::sun_state(best.jd, 9.93, 76.27).elevation_deg;
        assert!(close(depression, 2.485, 0.05), "best at {depression} deg down");
        assert_eq!(best.layer, "high");
        // Peak score: 10 x cover 0.5 = 5, "good".
        assert!(close(best.score, 5.0, 0.05));
        assert_eq!(best.word, "good");

        assert!(best_glow(&p, &kochi(), &series, end, start, 1.0).is_err());
        assert!(best_glow(&p, &kochi(), &series, start, end, 0.0).is_err());
        assert!(best_glow(&p, &kochi(), &series, start, start + 30.0, 1.0).is_err());
        // Far from the readings: no score anywhere.
        assert_eq!(best_glow(&p, &kochi(), &series, start + 2.0, start + 2.01, 1.0).unwrap(), None);
    }

    #[test]
    fn the_beam_crosses_a_level_only_if_it_dips_below_it() {
        // Sun 2 deg down, cloud at 9 km: the beam's lowest point is
        // 9 - 6371 x 0.034921^2 / 2 = 5.115 km. A level at 5 km is never
        // crossed; one at 6 km is crossed twice; one above the cloud once.
        assert!(beam_crossings_km(9.0, -2.0, 5.0, R).is_empty());
        assert_eq!(beam_crossings_km(9.0, -2.0, 6.0, R).len(), 2);
        assert_eq!(beam_crossings_km(9.0, -2.0, 12.0, R).len(), 1);
        // Sun up: the beam only rises toward the sun, so lower levels are
        // never crossed and a higher one is, close by.
        assert!(beam_crossings_km(1.5, 5.0, 1.0, R).is_empty());
        let up = beam_crossings_km(1.5, 5.0, 9.0, R);
        // 1.5 + x tan 5deg + x^2 / 12742 = 9: x = 6371 (-0.087489 + sqrt(0.0076543 + 0.0023544)) = 80.4 km.
        assert!(close(up[0], 80.4, 0.5), "{up:?}");
    }

    #[test]
    fn readings_toward_the_sun_decide_whether_the_beam_gets_through() {
        // The mid layer from the hand calculation above: its beam crosses the
        // low level at 152.5 and 292.5 km and the high level at 539.5 km.
        // Here there is little low cloud, but a solid bank of it lies 200 km
        // and more toward the sun, and no high cloud along the way:
        //   at 152.5 km, low cover 0 + (100 - 0) x 52.5 / 100 = 52.5%:
        //     lets through 1 - 0.525 x 0.5 = 0.7375
        //   at 292.5 km, low cover 100%: lets through 0.5
        //   at 539.5 km, high cover 0%: lets through 1
        // path = 0.7375 x 0.5 = 0.369 (it was 0.567 with the local forecast).
        let p = pack(&[("low", 1.5), ("mid", 5.0), ("high", 9.0)], simple_glow());
        let st = sky::sky_state(&p, &kochi(), kochi_evening_at(-2.0));
        let at = |low: f64| Conditions { cloud_low_pct: Some(low), cloud_mid_pct: Some(40.0), cloud_high_pct: Some(0.0), ..Conditions::default() };
        let along = [(100.0, at(0.0)), (200.0, at(100.0)), (300.0, at(100.0)), (600.0, at(100.0))];
        let g = glow(&p, &st, Some(&weather()), &along);
        let mid = &g.layers[1];
        assert!(g.along_sun);
        assert!(close(mid.factors.path, 0.369, 0.005), "path = {}", mid.factors.path);
        assert!(close(mid.crossings[0].cover_pct.unwrap(), 52.5, 1.0));
        assert_eq!(mid.crossings[2].cover_pct, Some(0.0));
        // The view up is still through the local low cloud (20%): 0.9.
        assert!(close(mid.factors.view, 0.9, 1e-9));
        // Score 10 x 0.4 x 0.369 x 0.9 x 0.8223 x 0.7779 = 0.85.
        assert!(close(mid.score.unwrap(), 0.85, 0.02), "mid = {:?}", mid.score);
    }

    #[test]
    fn a_layer_lit_only_toward_the_sun_uses_the_cover_where_it_is_lit() {
        // Sun 5 deg down, 10 km cloud: lit only toward the sun, with the
        // shadow edge about 199 km away (see sky.rs). No cloud overhead, but
        // high cover 50% at 100 km and 100% at 300 km: at the edge distance D
        // the cover is 50 + 50 (D - 100) / 200 percent.
        let p = pack(&[("high", 10.0)], simple_glow());
        let st = sky::sky_state(&p, &kochi(), kochi_evening_at(-5.0));
        let LayerLight::LitTowardSun { edge_distance_km, .. } = st.layers[0].light else { panic!("expected an edge toward the sun") };
        let high = |pct: f64| Conditions { cloud_high_pct: Some(pct), ..Conditions::default() };
        let along = [(100.0, high(50.0)), (300.0, high(100.0))];
        let l = &glow(&p, &st, Some(&high(0.0)), &along).layers[0];
        let expected = 50.0 + 50.0 * (edge_distance_km - 100.0) / 200.0;
        assert_eq!(l.cover_at_km, edge_distance_km);
        assert!(close(l.cover_pct.unwrap(), expected, 1e-9));
        assert!(l.score.unwrap() > 0.0, "lit cloud out there, though none overhead");
    }

    #[test]
    fn evening_uses_the_sunset_points_and_morning_the_sunrise_points() {
        use crate::conditions::{ConditionsSeries, Sample, Sunward, SunwardPoint};
        let p = pack(&[("low", 1.5), ("high", 9.0)], simple_glow());
        let evening = kochi_evening_at(-2.0);
        let noon = sun::solar_noon(2026, 11, 15, 76.27, 5.5);
        let morning = noon - (evening - noon);
        let steady = |c: Conditions| {
            // A reading every 2.4 hours from before the morning to after the evening.
            let samples = (0..12).map(|i| Sample { jd: morning - 0.1 + i as f64 * 0.1, conditions: c }).collect();
            ConditionsSeries::new("t", samples).unwrap()
        };
        let low = |pct: f64| Conditions { cloud_low_pct: Some(pct), cloud_high_pct: Some(50.0), ..Conditions::default() };
        let w = Weather {
            here: Some(steady(low(0.0))),
            sunset: Some(Sunward { azimuth_deg: 250.0, points: vec![SunwardPoint { distance_km: 100.0, series: steady(low(100.0)) }] }),
            sunrise: None,
        };
        assert!(glow_at(&p, &kochi(), Some(&w), evening).along_sun);
        assert!(!glow_at(&p, &kochi(), Some(&w), morning).along_sun);
    }

    #[test]
    fn bad_glow_numbers_are_rejected() {
        let ok = simple_glow();
        assert!(ok.validate().is_ok());
        let mut g = ok.clone();
        g.cover = Curve(vec![]);
        assert!(g.validate().is_err());
        let mut g = ok.clone();
        g.aerosol = Curve(vec![[1.0, 0.5], [0.5, 0.4]]);
        assert!(g.validate().unwrap_err().contains("left to right"));
        let mut g = ok.clone();
        g.humidity = Curve(vec![[0.0, 1.5]]);
        assert!(g.validate().is_err());
        let mut g = ok.clone();
        g.opacity.mid = 1.2;
        assert!(g.validate().is_err());
        let mut g = ok.clone();
        g.high_base_km = 2.0;
        assert!(g.validate().is_err());
        let mut g = ok.clone();
        g.sunward_km = vec![300.0, 100.0];
        assert!(g.validate().unwrap_err().contains("sunward_km"));
        let mut g = ok;
        g.words.reverse();
        assert!(g.validate().is_err());
    }
}
