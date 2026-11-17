//! What the air is like: cloud cover, moisture and haze.
//!
//! Everything else in this crate is geometry, and geometry is the same every
//! evening. What changes from one evening to the next is the *air*: which
//! clouds are there, how damp it is, how much dust and smoke is floating
//! about. This module holds those measurements as plain numbers.
//!
//! * [`Conditions`] is one reading: the air at one moment.
//! * [`ConditionsSeries`] is a list of readings over time (a forecast is
//!   usually one per hour) that can be asked "what was it like at 18:08?".
//!
//! The core never fetches anything. An *adapter* outside the core (the web
//! app, a script, or you with a text editor) gets the numbers from a weather
//! service and hands them over in this shape. See `docs/CONDITIONS.md`.
//!
//! **These numbers do not change any prediction yet.** For now they are
//! carried and reported next to the geometry. Using them (a thick low cloud
//! blocking the light, haze greying the colour) is the next step, and it will
//! be explained the same way everything else is.
//!
//! Every value is optional. A service may know the cloud cover but not the
//! aerosols; a missing value stays missing instead of being guessed.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::time;

/// Two readings further apart than this are not joined by a straight line:
/// too much can change in between. (A data-handling limit, not physics.
/// Hourly forecasts and 3-hourly aerosol data both fit well inside it.)
pub const MAX_GAP_HOURS: f64 = 6.0;

/// A single reading is trusted for this long before and after its own time,
/// when there is no close-enough pair of readings to work between.
pub const HOLD_MINUTES: f64 = 60.0;

/// The air at one moment. Every field is optional: `None` means "not known".
#[derive(Debug, Clone, Copy, PartialEq, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Conditions {
    /// Share of the sky covered by low cloud (below about 3 km), 0 to 100.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cloud_low_pct: Option<f64>,
    /// Share of the sky covered by middle cloud (about 3 to 8 km), 0 to 100.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cloud_mid_pct: Option<f64>,
    /// Share of the sky covered by high cloud (above about 8 km), 0 to 100.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cloud_high_pct: Option<f64>,
    /// Relative humidity of the air near the ground, 0 to 100.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub humidity_pct: Option<f64>,
    /// All the water vapour in the column of air above you, as the depth of
    /// water it would make if it all fell as rain, millimetres (the same
    /// number as kg per square metre). Dry air: under 10. Tropical wet
    /// season: 50 and more.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub water_vapour_mm: Option<f64>,
    /// Aerosol optical depth at 550 nm (green light): how much dust, smoke,
    /// salt and pollution dims a beam going straight up. No unit. Very clean
    /// air: about 0.05. Hazy: 0.5. Thick smoke: above 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aerosol_optical_depth: Option<f64>,
}

impl Conditions {
    /// Every field as (name, value, largest sensible value).
    fn fields(&self) -> [(&'static str, Option<f64>, f64); 6] {
        [
            ("cloud_low_pct", self.cloud_low_pct, 100.0),
            ("cloud_mid_pct", self.cloud_mid_pct, 100.0),
            ("cloud_high_pct", self.cloud_high_pct, 100.0),
            ("humidity_pct", self.humidity_pct, 100.0),
            ("water_vapour_mm", self.water_vapour_mm, 100.0),
            ("aerosol_optical_depth", self.aerosol_optical_depth, 10.0),
        ]
    }

    /// True if nothing at all is known.
    pub fn is_empty(&self) -> bool {
        self.fields().iter().all(|(_, v, _)| v.is_none())
    }

    /// Check that every known value is a sensible number.
    pub fn validate(&self) -> Result<(), String> {
        for (name, value, max) in self.fields() {
            if let Some(v) = value {
                if !(v.is_finite() && (0.0..=max).contains(&v)) {
                    return Err(format!("{name} must be between 0 and {max}, got {v}"));
                }
            }
        }
        Ok(())
    }

    /// The reading a fraction `t` (0 to 1) of the way from `self` to `other`,
    /// on a straight line. A value known on only one side stays unknown.
    fn blend(&self, other: &Conditions, t: f64) -> Conditions {
        let mix = |a: Option<f64>, b: Option<f64>| match (a, b) {
            (Some(a), Some(b)) => Some(a + (b - a) * t),
            _ => None,
        };
        Conditions {
            cloud_low_pct: mix(self.cloud_low_pct, other.cloud_low_pct),
            cloud_mid_pct: mix(self.cloud_mid_pct, other.cloud_mid_pct),
            cloud_high_pct: mix(self.cloud_high_pct, other.cloud_high_pct),
            humidity_pct: mix(self.humidity_pct, other.humidity_pct),
            water_vapour_mm: mix(self.water_vapour_mm, other.water_vapour_mm),
            aerosol_optical_depth: mix(self.aerosol_optical_depth, other.aerosol_optical_depth),
        }
    }
}

/// One reading with its time.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Sample {
    /// When the reading applies, as a Julian Day (UT).
    pub jd: f64,
    #[serde(flatten)]
    pub conditions: Conditions,
}

/// How an answer from [`ConditionsSeries::at`] was worked out, so it can be
/// shown to the user instead of hidden.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Basis {
    /// On a straight line between the readings just before and just after.
    Between { before_jd: f64, after_jd: f64 },
    /// Copied from the nearest single reading (no close pair to work between).
    Nearest { sample_jd: f64 },
}

/// The conditions at one moment, and how they were worked out.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Reading {
    pub conditions: Conditions,
    pub basis: Basis,
}

/// Readings over time, oldest first.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ConditionsSeries {
    /// Where the numbers came from, in words ("Open-Meteo forecast", "my own
    /// guess from the window"). Shown next to the numbers.
    pub source: String,
    pub samples: Vec<Sample>,
}

impl ConditionsSeries {
    /// Build a series from readings in any order. Checks every number and
    /// sorts by time. Two readings for the same moment are an error.
    pub fn new(source: impl Into<String>, mut samples: Vec<Sample>) -> Result<Self, String> {
        if samples.is_empty() {
            return Err("conditions need at least one sample".to_string());
        }
        for (i, s) in samples.iter().enumerate() {
            let which = format!("sample {}", i + 1);
            if !s.jd.is_finite() {
                return Err(format!("{which}: time must be a finite Julian Day"));
            }
            if s.conditions.is_empty() {
                return Err(format!("{which}: has a time but no values"));
            }
            s.conditions.validate().map_err(|e| format!("{which}: {e}"))?;
        }
        samples.sort_by(|a, b| a.jd.total_cmp(&b.jd));
        if samples.windows(2).any(|w| w[0].jd == w[1].jd) {
            return Err("two samples have the same time".to_string());
        }
        Ok(ConditionsSeries { source: source.into(), samples })
    }

    /// Read a conditions file. The format (see `docs/CONDITIONS.md`):
    ///
    /// ```json
    /// {
    ///   "source": "where the numbers came from",
    ///   "samples": [
    ///     { "utc": "2026-11-15T12:00", "cloud_high_pct": 40, "humidity_pct": 78 }
    ///   ]
    /// }
    /// ```
    ///
    /// Each sample gives its time as `utc` (date and clock time in UTC) or as
    /// `jd` (Julian Day), then any of the [`Conditions`] fields. A misspelt
    /// field is an error, not silently ignored.
    pub fn from_json(json: &str) -> Result<Self, String> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Raw {
            #[serde(default)]
            source: String,
            samples: Vec<Map<String, Value>>,
        }
        let raw: Raw =
            serde_json::from_str(json).map_err(|e| format!("invalid conditions JSON: {e}"))?;
        let mut samples = Vec::with_capacity(raw.samples.len());
        for (i, mut fields) in raw.samples.into_iter().enumerate() {
            let which = format!("sample {}", i + 1);
            let jd = match (fields.remove("utc"), fields.remove("jd")) {
                (Some(Value::String(text)), None) => {
                    parse_utc(&text).map_err(|e| format!("{which}: {e}"))?
                }
                (None, Some(Value::Number(n))) => n.as_f64().unwrap_or(f64::NAN),
                (None, None) => return Err(format!("{which}: needs a time (\"utc\" or \"jd\")")),
                (Some(_), Some(_)) => {
                    return Err(format!("{which}: give \"utc\" or \"jd\", not both"))
                }
                _ => {
                    return Err(format!(
                        "{which}: \"utc\" must be text like \"2026-11-15T12:00\" and \"jd\" a number"
                    ))
                }
            };
            let conditions: Conditions = serde_json::from_value(Value::Object(fields))
                .map_err(|e| format!("{which}: {e}"))?;
            samples.push(Sample { jd, conditions });
        }
        ConditionsSeries::new(raw.source, samples)
    }

    /// Time of the first and last reading (Julian Days).
    pub fn span(&self) -> (f64, f64) {
        (self.samples[0].jd, self.samples[self.samples.len() - 1].jd)
    }

    /// The conditions at Julian Day `jd`, or `None` if the series has nothing
    /// close enough to say.
    ///
    /// * Between two readings no more than [`MAX_GAP_HOURS`] apart: a
    ///   straight line between them.
    /// * Otherwise: the nearest reading, if it is within [`HOLD_MINUTES`].
    /// * Otherwise nothing. The series never guesses far from its data.
    pub fn at(&self, jd: f64) -> Option<Reading> {
        if !jd.is_finite() {
            return None;
        }
        let next = self.samples.partition_point(|s| s.jd <= jd);
        let before = next.checked_sub(1).map(|i| &self.samples[i]);
        let after = self.samples.get(next);

        if let (Some(a), Some(b)) = (before, after) {
            if b.jd - a.jd <= MAX_GAP_HOURS / 24.0 {
                let t = (jd - a.jd) / (b.jd - a.jd);
                return Some(Reading {
                    conditions: a.conditions.blend(&b.conditions, t),
                    basis: Basis::Between { before_jd: a.jd, after_jd: b.jd },
                });
            }
        }
        let nearest = [before, after]
            .into_iter()
            .flatten()
            .min_by(|a, b| (a.jd - jd).abs().total_cmp(&(b.jd - jd).abs()))?;
        if (nearest.jd - jd).abs() <= HOLD_MINUTES / 1440.0 {
            Some(Reading {
                conditions: nearest.conditions,
                basis: Basis::Nearest { sample_jd: nearest.jd },
            })
        } else {
            None
        }
    }
}

/// One point along the sun's direction, with its readings.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SunwardPoint {
    /// Distance from the place toward the sun, along the ground, km.
    pub distance_km: f64,
    pub series: ConditionsSeries,
}

/// Readings at points along the sun's direction (see `crate::sunward`), so
/// the glow score can use the cloud the sunbeam actually passes through.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Sunward {
    /// Compass direction of the points, degrees from north.
    pub azimuth_deg: f64,
    /// Nearest first.
    pub points: Vec<SunwardPoint>,
}

/// All the readings for one place and date: here, and (if known) along the
/// sun's direction at sunset and at sunrise. Every part is optional.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct Weather {
    pub here: Option<ConditionsSeries>,
    pub sunset: Option<Sunward>,
    pub sunrise: Option<Sunward>,
}

impl Weather {
    /// Readings for this place only.
    pub fn here_only(series: ConditionsSeries) -> Self {
        Weather { here: Some(series), sunset: None, sunrise: None }
    }

    /// Read a weather bundle (as written by serde: `here`, `sunset`,
    /// `sunrise`), or a plain conditions file, which becomes `here` only.
    pub fn from_json(text: &str) -> Result<Self, String> {
        let v: Value = serde_json::from_str(text).map_err(|e| format!("invalid weather JSON: {e}"))?;
        if v.get("samples").is_some() {
            return ConditionsSeries::from_json(text).map(Weather::here_only);
        }
        let map = v.as_object().ok_or("weather must be an object")?;
        let series = |v: &Value| ConditionsSeries::from_json(&v.to_string());
        let sunward = |v: &Value, which: &str| -> Result<Option<Sunward>, String> {
            if v.is_null() {
                return Ok(None);
            }
            let azimuth_deg = v
                .get("azimuth_deg")
                .and_then(Value::as_f64)
                .ok_or_else(|| format!("{which}: needs azimuth_deg"))?;
            let mut points = Vec::new();
            for p in v.get("points").and_then(Value::as_array).ok_or_else(|| format!("{which}: needs points"))? {
                let distance_km = p
                    .get("distance_km")
                    .and_then(Value::as_f64)
                    .filter(|d| d.is_finite() && *d > 0.0)
                    .ok_or_else(|| format!("{which}: every point needs a distance_km above 0"))?;
                let s = series(p.get("series").ok_or_else(|| format!("{which}: every point needs a series"))?)
                    .map_err(|e| format!("{which}: {e}"))?;
                points.push(SunwardPoint { distance_km, series: s });
            }
            points.sort_by(|a, b| a.distance_km.total_cmp(&b.distance_km));
            Ok(Some(Sunward { azimuth_deg, points }))
        };
        for key in map.keys() {
            if !["here", "sunset", "sunrise"].contains(&key.as_str()) {
                return Err(format!("weather has an unknown field '{key}'"));
            }
        }
        Ok(Weather {
            here: match map.get("here") {
                Some(h) if !h.is_null() => Some(series(h).map_err(|e| format!("here: {e}"))?),
                _ => None,
            },
            sunset: map.get("sunset").map_or(Ok(None), |v| sunward(v, "sunset"))?,
            sunrise: map.get("sunrise").map_or(Ok(None), |v| sunward(v, "sunrise"))?,
        })
    }

    /// The readings here at `jd`, if any are close enough.
    pub fn here_at(&self, jd: f64) -> Option<Conditions> {
        self.here.as_ref().and_then(|s| s.at(jd)).map(|r| r.conditions)
    }

    /// Readings along the sun's direction at `jd`, nearest first, as
    /// (distance km, conditions). Uses the sunrise points while the sun is
    /// rising and the sunset points while it is sinking. Points with nothing
    /// close enough in time are left out.
    pub fn along_at(&self, rising: bool, jd: f64) -> Vec<(f64, Conditions)> {
        let which = if rising { &self.sunrise } else { &self.sunset };
        which.as_ref().map_or_else(Vec::new, |w| {
            w.points
                .iter()
                .filter_map(|p| p.series.at(jd).map(|r| (p.distance_km, r.conditions)))
                .collect()
        })
    }
}

/// Julian Day for a UTC date and time written like `2026-11-15T12:00`,
/// `2026-11-15T12:00:30Z` or `2026-11-15 12:00`.
pub fn parse_utc(text: &str) -> Result<f64, String> {
    let trimmed = text.trim();
    let body = trimmed.strip_suffix('Z').unwrap_or(trimmed);
    let (date, clock) = body
        .split_once(|c| c == 'T' || c == ' ')
        .ok_or_else(|| format!("expected a UTC time like 2026-11-15T12:00, got '{text}'"))?;
    let (y, m, d) = time::parse_date(date)?;
    let hours = time::parse_clock(clock)?;
    Ok(time::julian_day(y, m, d, hours))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2000-01-01 12:00 UTC is Julian Day 2451545.0 exactly (the "J2000"
    /// epoch every astronomy book quotes).
    const J2000: f64 = 2_451_545.0;
    const HOUR: f64 = 1.0 / 24.0;
    const MINUTE: f64 = 1.0 / 1440.0;

    fn high(pct: f64) -> Conditions {
        Conditions { cloud_high_pct: Some(pct), ..Conditions::default() }
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn utc_text_becomes_the_right_julian_day() {
        assert_eq!(parse_utc("2000-01-01T12:00").unwrap(), J2000);
        assert_eq!(parse_utc("2000-01-01T12:00:00Z").unwrap(), J2000);
        assert_eq!(parse_utc(" 2000-01-01 12:00 ").unwrap(), J2000);
        // Six hours later is a quarter of a day later.
        assert!(close(parse_utc("2000-01-01T18:00").unwrap(), J2000 + 0.25));
        assert!(parse_utc("2000-01-01").is_err(), "a date alone is not a time");
        assert!(parse_utc("2000-13-01T12:00").is_err());
        assert!(parse_utc("2000-01-01T25:00").is_err());
    }

    #[test]
    fn a_quarter_of_the_way_between_two_readings() {
        // 20% at 12:00 and 60% at 13:00. At 12:15, a quarter of the way:
        // 20 + (60 - 20) / 4 = 30%.
        let s = ConditionsSeries::new(
            "test",
            vec![
                Sample { jd: J2000, conditions: high(20.0) },
                Sample { jd: J2000 + HOUR, conditions: high(60.0) },
            ],
        )
        .unwrap();
        let r = s.at(J2000 + 15.0 * MINUTE).unwrap();
        assert!(close(r.conditions.cloud_high_pct.unwrap(), 30.0));
        assert_eq!(r.basis, Basis::Between { before_jd: J2000, after_jd: J2000 + HOUR });
        // Exactly on a reading gives that reading.
        assert!(close(s.at(J2000).unwrap().conditions.cloud_high_pct.unwrap(), 20.0));
        assert!(close(s.at(J2000 + HOUR).unwrap().conditions.cloud_high_pct.unwrap(), 60.0));
    }

    #[test]
    fn a_value_known_on_one_side_only_stays_unknown() {
        let both = Conditions { humidity_pct: Some(80.0), ..high(10.0) };
        let s = ConditionsSeries::new(
            "test",
            vec![
                Sample { jd: J2000, conditions: both },
                Sample { jd: J2000 + HOUR, conditions: high(30.0) },
            ],
        )
        .unwrap();
        let c = s.at(J2000 + 30.0 * MINUTE).unwrap().conditions;
        assert!(close(c.cloud_high_pct.unwrap(), 20.0), "halfway between 10 and 30");
        assert_eq!(c.humidity_pct, None, "humidity is missing at 13:00, so it is not guessed");
    }

    #[test]
    fn never_guesses_far_from_the_data() {
        let s = ConditionsSeries::new(
            "test",
            vec![
                Sample { jd: J2000, conditions: high(20.0) },
                Sample { jd: J2000 + HOUR, conditions: high(60.0) },
            ],
        )
        .unwrap();
        // Half an hour outside either end: the nearest reading is used as is.
        let early = s.at(J2000 - 30.0 * MINUTE).unwrap();
        assert_eq!(early.conditions.cloud_high_pct, Some(20.0));
        assert_eq!(early.basis, Basis::Nearest { sample_jd: J2000 });
        assert_eq!(s.at(J2000 + 90.0 * MINUTE).unwrap().conditions.cloud_high_pct, Some(60.0));
        // Two hours outside: nothing.
        assert!(s.at(J2000 - 2.0 * HOUR).is_none());
        assert!(s.at(J2000 + 3.0 * HOUR).is_none());
        assert!(s.at(f64::NAN).is_none());
    }

    #[test]
    fn readings_a_day_apart_are_not_joined() {
        // Noon one day and noon the next: the evening in between is unknown.
        let s = ConditionsSeries::new(
            "test",
            vec![
                Sample { jd: J2000, conditions: high(0.0) },
                Sample { jd: J2000 + 1.0, conditions: high(100.0) },
            ],
        )
        .unwrap();
        assert!(s.at(J2000 + 0.5).is_none(), "midnight is 12 h from both readings");
        // Twenty minutes after the first reading it still holds, unblended.
        let r = s.at(J2000 + 20.0 * MINUTE).unwrap();
        assert_eq!(r.conditions.cloud_high_pct, Some(0.0));
        assert_eq!(r.basis, Basis::Nearest { sample_jd: J2000 });
    }

    #[test]
    fn samples_are_sorted_and_checked() {
        let s = ConditionsSeries::new(
            "test",
            vec![
                Sample { jd: J2000 + HOUR, conditions: high(60.0) },
                Sample { jd: J2000, conditions: high(20.0) },
            ],
        )
        .unwrap();
        assert_eq!(s.span(), (J2000, J2000 + HOUR));

        let err = |samples| ConditionsSeries::new("test", samples).unwrap_err();
        assert!(err(vec![]).contains("at least one"));
        assert!(err(vec![Sample { jd: J2000, conditions: Conditions::default() }]).contains("no values"));
        assert!(err(vec![Sample { jd: J2000, conditions: high(101.0) }]).contains("cloud_high_pct"));
        assert!(err(vec![Sample { jd: J2000, conditions: high(-1.0) }]).contains("cloud_high_pct"));
        assert!(err(vec![Sample { jd: f64::NAN, conditions: high(5.0) }]).contains("time"));
        assert!(err(vec![
            Sample { jd: J2000, conditions: high(5.0) },
            Sample { jd: J2000, conditions: high(6.0) },
        ])
        .contains("same time"));
        let smoky = Conditions { aerosol_optical_depth: Some(11.0), ..Conditions::default() };
        assert!(err(vec![Sample { jd: J2000, conditions: smoky }]).contains("aerosol_optical_depth"));
    }

    #[test]
    fn reads_a_file_and_rejects_typos() {
        let s = ConditionsSeries::from_json(
            r#"{
                "source": "by hand",
                "samples": [
                    { "utc": "2000-01-01T13:00", "cloud_low_pct": 10, "aerosol_optical_depth": 0.3 },
                    { "jd": 2451545.0, "cloud_low_pct": 30, "water_vapour_mm": 42.5 }
                ]
            }"#,
        )
        .unwrap();
        assert_eq!(s.source, "by hand");
        assert_eq!(s.samples[0].jd, J2000, "sorted: the jd sample is the earlier one");
        assert_eq!(s.samples[0].conditions.water_vapour_mm, Some(42.5));
        assert_eq!(s.samples[1].conditions.aerosol_optical_depth, Some(0.3));
        // Halfway: low cloud (30 + 10) / 2 = 20; the other two are one-sided.
        let c = s.at(J2000 + 30.0 * MINUTE).unwrap().conditions;
        assert!(close(c.cloud_low_pct.unwrap(), 20.0));
        assert_eq!(c.water_vapour_mm, None);

        let bad = |sample: &str| {
            ConditionsSeries::from_json(&format!(r#"{{"samples":[{sample}]}}"#)).unwrap_err()
        };
        assert!(bad(r#"{"utc":"2000-01-01T12:00","cloud_hi_pct":5}"#).contains("cloud_hi_pct"));
        assert!(bad(r#"{"cloud_high_pct":5}"#).contains("needs a time"));
        assert!(bad(r#"{"utc":"2000-01-01T12:00","jd":2451545.0,"cloud_high_pct":5}"#).contains("not both"));
        assert!(bad(r#"{"utc":"noon","cloud_high_pct":5}"#).contains("sample 1"));
        assert!(bad(r#"{"utc":"2000-01-01T12:00","cloud_high_pct":"lots"}"#).contains("sample 1"));
        assert!(ConditionsSeries::from_json("{").is_err());
    }

    #[test]
    fn weather_reads_a_bundle_or_a_plain_file_and_picks_the_right_direction() {
        let one = |pct: f64| ConditionsSeries::new("t", vec![Sample { jd: J2000, conditions: high(pct) }]).unwrap();
        let w = Weather {
            here: Some(one(10.0)),
            sunset: Some(Sunward {
                azimuth_deg: 250.0,
                points: vec![
                    SunwardPoint { distance_km: 100.0, series: one(20.0) },
                    SunwardPoint { distance_km: 300.0, series: one(30.0) },
                ],
            }),
            sunrise: Some(Sunward { azimuth_deg: 110.0, points: vec![SunwardPoint { distance_km: 100.0, series: one(90.0) }] }),
        };
        let back = Weather::from_json(&serde_json::to_string(&w).unwrap()).unwrap();
        assert_eq!(back, w);
        assert_eq!(w.here_at(J2000).unwrap().cloud_high_pct, Some(10.0));
        let evening: Vec<(f64, Option<f64>)> = w.along_at(false, J2000).iter().map(|(d, c)| (*d, c.cloud_high_pct)).collect();
        assert_eq!(evening, vec![(100.0, Some(20.0)), (300.0, Some(30.0))]);
        assert_eq!(w.along_at(true, J2000)[0].1.cloud_high_pct, Some(90.0));
        // Far from the readings in time: nothing along the way either.
        assert!(w.along_at(false, J2000 + 1.0).is_empty());

        // A plain conditions file is "here" only.
        let plain = Weather::from_json(&serde_json::to_string(&one(5.0)).unwrap()).unwrap();
        assert_eq!(plain.here_at(J2000).unwrap().cloud_high_pct, Some(5.0));
        assert!(plain.sunset.is_none() && plain.along_at(false, J2000).is_empty());

        assert!(Weather::from_json(r#"{"hre": null}"#).unwrap_err().contains("hre"));
        assert!(Weather::from_json(r#"{"sunset": {"points": []}}"#).unwrap_err().contains("azimuth"));
    }

    #[test]
    fn a_series_survives_a_round_trip_through_json() {
        let s = ConditionsSeries::new(
            "round trip",
            vec![Sample { jd: J2000, conditions: Conditions { humidity_pct: Some(55.0), ..high(12.0) } }],
        )
        .unwrap();
        let text = serde_json::to_string(&s).unwrap();
        assert!(!text.contains("null"), "unknown values are left out, not written as null");
        assert_eq!(ConditionsSeries::from_json(&text).unwrap(), s);
    }
}
