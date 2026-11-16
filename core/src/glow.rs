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

use crate::conditions::{Conditions, ConditionsSeries};
use crate::params::Params;
use crate::place::Place;
use crate::sky::{self, LayerLight, SkyState};

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
    /// Forecast "low" cloud is below this height, km (Open-Meteo: 3).
    pub low_top_km: f64,
    /// Forecast "high" cloud is above this height, km (Open-Meteo: 8).
    pub high_base_km: f64,
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
}

impl GlowParams {
    pub fn validate(&self) -> Result<(), String> {
        if !(self.low_top_km > 0.0 && self.high_base_km > self.low_top_km) {
            return Err("glow: low_top_km must be above 0 and below high_base_km".to_string());
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
        if self.words.is_empty() || self.words[0].min_score > 0.0 {
            return Err("glow: words must start at min_score 0".to_string());
        }
        if self.words.windows(2).any(|w| w[1].min_score <= w[0].min_score) {
            return Err("glow: words must go from low scores to high".to_string());
        }
        Ok(())
    }

    /// Which forecast level a cloud height belongs to.
    pub fn level_of(&self, height_km: f64) -> Level {
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

/// The glow of one cloud layer at one instant.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LayerGlow {
    pub name: String,
    pub height_km: f64,
    /// The forecast level whose cover this layer uses.
    pub level: Level,
    /// The lowest height (km) the sunbeam reaching the visible part of this
    /// layer dips to: the grazing height overhead, 0 at a shadow edge toward
    /// the sun. `None` while unlit.
    pub beam_low_km: Option<f64>,
    /// The forecast numbers used, percent (cover of this level and of the
    /// others it was compared with), for the explanation.
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
    /// Lower levels you look through to see this layer (counted in `view`).
    pub seen_through: Vec<Level>,
    /// Values that were not known and so had no effect, in words.
    pub unknown: Vec<String>,
}

/// The glow of every layer at one instant.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Glow {
    pub jd: f64,
    /// False when no conditions were given: only `potential` is meaningful.
    pub has_conditions: bool,
    pub layers: Vec<LayerGlow>,
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

/// Work out the glow of every layer in `sky`, with the weather in
/// `conditions` if known.
pub fn glow(params: &Params, sky: &SkyState, conditions: Option<&Conditions>) -> Glow {
    let g = &params.glow;
    // Each pack layer's level, and for each level the lowest and highest
    // pack layer in it (a level is counted once, however many layers share it).
    let levels: Vec<Level> = sky.layers.iter().map(|l| g.level_of(l.height_km)).collect();
    let span = |level: Level| {
        let hs = sky.layers.iter().zip(&levels).filter(|(_, lv)| **lv == level).map(|(l, _)| l.height_km);
        hs.fold(None, |acc: Option<(f64, f64)>, h| Some(acc.map_or((h, h), |(lo, hi)| (lo.min(h), hi.max(h)))))
    };
    let mut present: Vec<Level> = levels.clone();
    present.sort();
    present.dedup();

    let sun = if sky.elevation_deg >= 0.0 { g.sun_up.at(sky.elevation_deg) } else { 1.0 };

    let layers = sky
        .layers
        .iter()
        .zip(&levels)
        .map(|(layer, &level)| {
            let (light, beam_low_km) = match &layer.light {
                LayerLight::LitOverhead { grazing_km, .. } => (1.0, Some(*grazing_km)),
                LayerLight::LitTowardSun { edge_elevation_deg, .. } => (g.toward_sun.at(*edge_elevation_deg), Some(0.0)),
                LayerLight::EdgeBeyondHorizon | LayerLight::ShadowOverhead => (0.0, None),
            };
            let colour = beam_low_km.map_or(0.0, |z| g.colour.at(z));
            let potential = 10.0 * light * sun * colour;

            let mut out = LayerGlow {
                name: layer.name.clone(),
                height_km: layer.height_km,
                level,
                beam_low_km,
                cover_pct: None,
                factors: Factors { light, sun, colour, cover: None, path: 1.0, view: 1.0, haze: 1.0, humidity: 1.0 },
                potential,
                score: None,
                word: None,
                blocked_by: Vec::new(),
                seen_through: Vec::new(),
                unknown: Vec::new(),
            };
            let Some(c) = conditions else { return out };

            // How much a level stops: its cover times its opacity.
            let stop = |other: Level, unknown: &mut Vec<String>| match other.cover(c) {
                Some(pct) => 1.0 - pct / 100.0 * g.opacity.get(other),
                None => {
                    unknown.push(format!("{} cloud cover", other.word()));
                    1.0
                }
            };
            for &other in present.iter().filter(|&&o| o != level) {
                let (lowest, highest) = span(other).expect("present levels have layers");
                // The beam crosses every height above its lowest point.
                if beam_low_km.is_some_and(|z| highest >= z) {
                    out.blocked_by.push(other);
                    out.factors.path *= stop(other, &mut out.unknown);
                }
                // Looking up, you see through cloud below this layer.
                if lowest < layer.height_km {
                    out.seen_through.push(other);
                    out.factors.view *= stop(other, &mut out.unknown);
                }
            }

            let weight = beam_low_km.map_or(0.0, |z| g.low_beam.at(z));
            match c.aerosol_optical_depth {
                Some(aod) => out.factors.haze = 1.0 - weight * (1.0 - g.aerosol.at(aod)),
                None => out.unknown.push("haze (aerosol optical depth)".to_string()),
            }
            match c.humidity_pct {
                Some(rh) => out.factors.humidity = 1.0 - weight * (1.0 - g.humidity.at(rh)),
                None => out.unknown.push("humidity".to_string()),
            }
            out.unknown.dedup();

            match level.cover(c) {
                Some(pct) => {
                    let f = &mut out.factors;
                    out.cover_pct = Some(pct);
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

    Glow { jd: sky.jd, has_conditions: conditions.is_some(), layers }
}

/// The sky and its glow at one instant, with the conditions read from
/// `series` at that time (if it has any close enough).
pub fn glow_at(params: &Params, place: &Place, series: Option<&ConditionsSeries>, jd: f64) -> Glow {
    let st = sky::sky_state(params, place, jd);
    let reading = series.and_then(|s| s.at(jd));
    glow(params, &st, reading.as_ref().map(|r| &r.conditions))
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
    series: &ConditionsSeries,
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
        let g = glow_at(params, place, Some(series), jd);
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
    fn scores_match_a_hand_calculation() {
        // Sun 2 deg down, R = 6371 km. Lowest point of the beam to a cloud
        // overhead at h: (R + h) cos 2deg - R, with cos 2deg = 0.9993908:
        //   high 9 km: 6380 x 0.9993908 - 6371 = 5.114 km
        //   mid  5 km: 6376 x 0.9993908 - 6371 = 1.116 km
        // Weather: low 20%, mid 40%, high 60%, aerosol 0.2, humidity 50%.
        // A level stops cover x 0.5: low 0.1, mid 0.2, high 0.3.
        let p = pack(&[("low", 1.5), ("mid", 5.0), ("high", 9.0)], simple_glow());
        let st = sky::sky_state(&p, &kochi(), kochi_evening_at(-2.0));
        let g = glow(&p, &st, Some(&weather()));
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

        // MID: the beam dips to 1.12 km, below the low layer's 1.5 km and
        // the high layer's 9 km, so both are in its way: path 0.9 x 0.7 = 0.63.
        // View through low only: 0.9. Weight 1 - 0.1116 = 0.8884: haze
        // 1 - 0.8884 x 0.2 = 0.8223, humidity 1 - 0.8884 x 0.25 = 0.7779.
        // Score 10 x 0.4 x 0.63 x 0.9 x 0.8223 x 0.7779 = 1.45.
        assert_eq!(mid.blocked_by, vec![Level::Low, Level::High]);
        assert!(close(mid.factors.path, 0.63, 1e-9));
        assert!(close(mid.factors.view, 0.9, 1e-9));
        assert!(close(mid.score.unwrap(), 1.45, 0.01), "mid = {:?}", mid.score);

        assert_eq!(g.best().unwrap().name, "high");
    }

    #[test]
    fn without_weather_only_the_potential_is_known() {
        let p = pack(&[("high", 9.0)], simple_glow());
        let st = sky::sky_state(&p, &kochi(), kochi_evening_at(-2.0));
        let g = glow(&p, &st, None);
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
        let g = glow(&p, &st, Some(&clear));
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
        let g = glow(&p, &st, Some(&partial));
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
        let l = &glow(&p, &st, Some(&weather())).layers[0];
        assert_eq!(l.beam_low_km, Some(0.0));
        assert_eq!(l.factors.light, 0.5);
        assert!(close(l.score.unwrap(), 1.8, 1e-9));
    }

    #[test]
    fn a_high_sun_gives_no_sunset_glow_and_a_dark_layer_gives_none() {
        let p = pack(&[("low", 1.5), ("high", 9.0)], simple_glow());
        // Midday: sun about 60 deg up, beyond the sun_up curve's 10 deg.
        let noon = sun::solar_noon(2026, 11, 15, 76.27, 5.5);
        let g = glow(&p, &sky::sky_state(&p, &kochi(), noon), Some(&weather()));
        assert!(g.layers.iter().all(|l| l.factors.sun == 0.0 && l.score == Some(0.0)));
        // Sun 5 deg up: sun factor 1 - 5/10 = 0.5. A low cloud sees the beam
        // at its own height, so the high cloud above (9 km >= 1.5) is in its way.
        let g = glow(&p, &sky::sky_state(&p, &kochi(), kochi_evening_at(5.0)), Some(&weather()));
        assert!(close(g.layers[0].factors.sun, 0.5, 1e-3));
        assert_eq!(g.layers[0].blocked_by, vec![Level::High]);
        // Deep night: every layer dark.
        let g = glow(&p, &sky::sky_state(&p, &kochi(), kochi_evening_at(-30.0)), Some(&weather()));
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
        let series = ConditionsSeries::new(
            "test",
            vec![Sample { jd: start - 0.05, conditions: still }, Sample { jd: end + 0.05, conditions: still }],
        )
        .unwrap();
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
        let mut g = ok;
        g.words.reverse();
        assert!(g.validate().is_err());
    }
}
