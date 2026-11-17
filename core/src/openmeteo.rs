//! Turns answers from the Open-Meteo weather service into [`Conditions`].
//!
//! [Open-Meteo](https://open-meteo.com) is a free weather service that needs
//! no account or key. Two of its services carry what skyglow wants:
//!
//! * the **forecast** service: cloud cover at low, middle and high levels,
//!   humidity near the ground, and the water vapour in the whole column of
//!   air;
//! * the **air-quality** service, which passes on the European CAMS forecast:
//!   aerosol optical depth (haze, dust, smoke).
//!
//! This module does **not** go online. Something else fetches the answers
//! (the web app, or you with `curl`) and hands their text to
//! [`series_from_responses`], which reads them and merges them into one
//! [`ConditionsSeries`]. That keeps the core free of any network code, and
//! means the web app and the command line translate in exactly the same way.
//!
//! What it understands: the `hourly` block of an Open-Meteo answer, with
//! times either as text (`2026-11-15T12:00`, in whatever time zone was asked
//! for; the answer's own `utc_offset_seconds` is used to get back to UTC) or
//! as Unix seconds (`timeformat=unixtime`). Any other values in the answer
//! are ignored. A value that is missing (`null`) or outside the sensible
//! range stays unknown.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::conditions::{parse_utc, Conditions, ConditionsSeries, Sample, Sunward, SunwardPoint, Weather};
use crate::sunward::{Direction, WeatherPlan};

/// The `hourly=` list to ask the forecast service for.
pub const FORECAST_HOURLY: &str =
    "cloud_cover_low,cloud_cover_mid,cloud_cover_high,relative_humidity_2m,total_column_integrated_water_vapour";

/// The `hourly=` list to ask the air-quality service for.
pub const AIR_QUALITY_HOURLY: &str = "aerosol_optical_depth";

/// Julian Day of 1970-01-01 00:00 UTC, where Unix time starts.
const UNIX_EPOCH_JD: f64 = 2_440_587.5;

/// Where one of Open-Meteo's hourly variables goes in [`Conditions`], or
/// `None` for a variable skyglow does not read.
///
/// Units match without conversion: cover and humidity are percentages,
/// column water vapour is kg per square metre (the same number as mm), and
/// aerosol optical depth has no unit (Open-Meteo gives it at 550 nm).
fn slot<'a>(c: &'a mut Conditions, variable: &str) -> Option<&'a mut Option<f64>> {
    match variable {
        "cloud_cover_low" => Some(&mut c.cloud_low_pct),
        "cloud_cover_mid" => Some(&mut c.cloud_mid_pct),
        "cloud_cover_high" => Some(&mut c.cloud_high_pct),
        "relative_humidity_2m" => Some(&mut c.humidity_pct),
        "total_column_integrated_water_vapour" => Some(&mut c.water_vapour_mm),
        "aerosol_optical_depth" => Some(&mut c.aerosol_optical_depth),
        _ => None,
    }
}

/// One hourly time from an answer, as Unix seconds (UTC).
fn unix_seconds(time: &Value, utc_offset_seconds: f64) -> Result<i64, String> {
    match time {
        // Unix times from Open-Meteo are always UTC.
        Value::Number(n) => n
            .as_f64()
            .filter(|s| s.is_finite())
            .map(|s| s.round() as i64)
            .ok_or_else(|| "a time is not a usable number".to_string()),
        // Text times are on the clock that was asked for; step back to UTC.
        Value::String(text) => {
            let jd = parse_utc(text)?;
            Ok(((jd - UNIX_EPOCH_JD) * 86_400.0 - utc_offset_seconds).round() as i64)
        }
        _ => Err("a time is neither text nor a number".to_string()),
    }
}

/// Read one answer into `into`, keyed by Unix seconds. Values already there
/// are kept unless this answer knows the same value too.
fn read_response(v: &Value, into: &mut BTreeMap<i64, Conditions>) -> Result<(), String> {
    if v.get("error").and_then(Value::as_bool) == Some(true) {
        let reason = v.get("reason").and_then(Value::as_str).unwrap_or("no reason given");
        return Err(format!("Open-Meteo said: {reason}"));
    }
    let hourly = v
        .get("hourly")
        .and_then(Value::as_object)
        .ok_or("no \"hourly\" block (ask Open-Meteo for hourly values)")?;
    let times = hourly
        .get("time")
        .and_then(Value::as_array)
        .ok_or("the \"hourly\" block has no \"time\" list")?;
    let offset = v.get("utc_offset_seconds").and_then(Value::as_f64).unwrap_or(0.0);
    let seconds: Vec<i64> = times
        .iter()
        .map(|t| unix_seconds(t, offset))
        .collect::<Result<_, _>>()?;

    for (variable, values) in hourly {
        if slot(&mut Conditions::default(), variable).is_none() {
            continue;
        }
        let values = values
            .as_array()
            .ok_or_else(|| format!("\"{variable}\" is not a list"))?;
        if values.len() != seconds.len() {
            return Err(format!(
                "\"{variable}\" has {} values for {} times",
                values.len(),
                seconds.len()
            ));
        }
        for (when, value) in seconds.iter().zip(values) {
            let Some(number) = value.as_f64() else { continue };
            // Let Conditions' own range check decide if the number is usable.
            let mut probe = Conditions::default();
            *slot(&mut probe, variable).expect("checked above") = Some(number);
            if probe.validate().is_err() {
                continue;
            }
            let entry = into.entry(*when).or_default();
            *slot(entry, variable).expect("checked above") = Some(number);
        }
    }
    Ok(())
}

/// Merge one or more Open-Meteo answers (forecast, air quality, or both, in
/// any order) into one series. `source` is the label shown next to the
/// numbers, for example "Open-Meteo forecast".
///
/// Hours where nothing usable is known are left out. If no hour is left at
/// all, that is an error.
pub fn series_from_responses(responses: &[&str], source: &str) -> Result<ConditionsSeries, String> {
    if responses.is_empty() {
        return Err("no Open-Meteo answers to read".to_string());
    }
    let mut by_time = BTreeMap::new();
    for (i, text) in responses.iter().enumerate() {
        let v: Value = serde_json::from_str(text).map_err(|e| format!("Open-Meteo answer {}: not JSON: {e}", i + 1))?;
        read_response(&v, &mut by_time).map_err(|e| format!("Open-Meteo answer {}: {e}", i + 1))?;
    }
    to_series(by_time, source)?.ok_or_else(|| "Open-Meteo answered, but with none of the values skyglow reads".to_string())
}

/// Readings keyed by Unix seconds into a series; `None` if nothing usable.
fn to_series(by_time: BTreeMap<i64, Conditions>, source: &str) -> Result<Option<ConditionsSeries>, String> {
    let samples: Vec<Sample> = by_time
        .into_iter()
        .filter(|(_, c)| !c.is_empty())
        .map(|(seconds, conditions)| Sample {
            jd: seconds as f64 / 86_400.0 + UNIX_EPOCH_JD,
            conditions,
        })
        .collect();
    if samples.is_empty() {
        return Ok(None);
    }
    ConditionsSeries::new(source, samples).map(Some)
}

/// Read Open-Meteo answers that cover the places in `plan` (asked for in a
/// single request with comma-separated coordinates, so each answer is a list
/// with one entry per place, in the plan's order) into a [`Weather`].
///
/// An answer for a single place (not a list) is read as "here" only, so
/// answers saved for just the place itself still work. A place along the way
/// with nothing usable is simply left out; the place itself must have data.
pub fn weather_from_responses(plan: &WeatherPlan, responses: &[&str], source: &str) -> Result<Weather, String> {
    if responses.is_empty() {
        return Err("no Open-Meteo answers to read".to_string());
    }
    // Each answer as a list of per-place answers.
    let mut per_answer: Vec<Vec<Value>> = Vec::new();
    for (i, text) in responses.iter().enumerate() {
        let v: Value = serde_json::from_str(text).map_err(|e| format!("Open-Meteo answer {}: not JSON: {e}", i + 1))?;
        per_answer.push(match v {
            Value::Array(items) => items,
            single => vec![single],
        });
    }
    let places = per_answer[0].len();
    if per_answer.iter().any(|a| a.len() != places) {
        return Err("the Open-Meteo answers cover different numbers of places".to_string());
    }
    if places != 1 && places != plan.points.len() {
        return Err(format!(
            "the Open-Meteo answers cover {places} places; this place and date need {} (or 1 for the place alone)",
            plan.points.len()
        ));
    }
    let mut series: Vec<Option<ConditionsSeries>> = Vec::with_capacity(places);
    for place in 0..places {
        let mut by_time = BTreeMap::new();
        for (i, answer) in per_answer.iter().enumerate() {
            read_response(&answer[place], &mut by_time).map_err(|e| format!("Open-Meteo answer {}: {e}", i + 1))?;
        }
        series.push(to_series(by_time, source)?);
    }
    let here = series[0].take().ok_or("Open-Meteo answered, but with none of the values skyglow reads for this place")?;
    if places == 1 {
        return Ok(Weather::here_only(here));
    }
    let direction = |d: &Option<Direction>, series: &mut Vec<Option<ConditionsSeries>>| {
        d.as_ref().map(|d| Sunward {
            azimuth_deg: d.azimuth_deg,
            points: d
                .distances_km
                .iter()
                .enumerate()
                .filter_map(|(k, km)| series[d.first + k].take().map(|s| SunwardPoint { distance_km: *km, series: s }))
                .collect(),
        })
    };
    let sunset = direction(&plan.sunset, &mut series);
    let sunrise = direction(&plan.sunrise, &mut series);
    Ok(Weather { here: Some(here), sunset, sunrise })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sunward::GeoPoint;

    /// 2000-01-01 12:00 UTC: Julian Day 2451545.0 (the J2000 epoch) and Unix
    /// time 946728000 (946684800 is that day's midnight, plus 12 x 3600).
    const J2000: f64 = 2_451_545.0;
    const HOUR: f64 = 1.0 / 24.0;

    /// A forecast answer shaped like Open-Meteo's, times as UTC text.
    const FORECAST: &str = r#"{
        "latitude": 9.875, "longitude": 76.25, "utc_offset_seconds": 0, "timezone": "GMT",
        "hourly_units": { "time": "iso8601", "cloud_cover_low": "%" },
        "hourly": {
            "time": ["2000-01-01T12:00", "2000-01-01T13:00", "2000-01-01T14:00"],
            "cloud_cover_low": [10, 20, null],
            "cloud_cover_mid": [0, 0, 5],
            "cloud_cover_high": [40, 60, 80],
            "relative_humidity_2m": [70, 74, 78],
            "total_column_integrated_water_vapour": [44.5, 45.0, 45.5],
            "temperature_2m": [29.1, 28.4, 27.9]
        }
    }"#;

    /// An air-quality answer for the same hours, times as Unix seconds.
    const AIR: &str = r#"{
        "utc_offset_seconds": 0,
        "hourly_units": { "time": "unixtime" },
        "hourly": {
            "time": [946728000, 946731600, 946735200],
            "aerosol_optical_depth": [0.30, 0.34, null]
        }
    }"#;

    #[test]
    fn a_forecast_answer_becomes_hourly_readings() {
        let s = series_from_responses(&[FORECAST], "test").unwrap();
        assert_eq!(s.source, "test");
        assert_eq!(s.samples.len(), 3);
        assert_eq!(s.samples[0].jd, J2000, "12:00 UTC on 1 Jan 2000 is J2000");
        let first = s.samples[0].conditions;
        assert_eq!(first.cloud_low_pct, Some(10.0));
        assert_eq!(first.cloud_high_pct, Some(40.0));
        assert_eq!(first.humidity_pct, Some(70.0));
        assert_eq!(first.water_vapour_mm, Some(44.5));
        assert_eq!(first.aerosol_optical_depth, None, "the forecast carries no aerosols");
        assert_eq!(s.samples[2].conditions.cloud_low_pct, None, "null stays unknown");
        // Halfway through the first hour: high cloud (40 + 60) / 2 = 50.
        let mid = s.at(J2000 + HOUR / 2.0).unwrap().conditions;
        assert!((mid.cloud_high_pct.unwrap() - 50.0).abs() < 1e-6);
    }

    #[test]
    fn forecast_and_air_quality_merge_by_time_in_either_order() {
        for pair in [[FORECAST, AIR], [AIR, FORECAST]] {
            let s = series_from_responses(&pair, "test").unwrap();
            assert_eq!(s.samples.len(), 3, "same three hours, not six");
            assert_eq!(s.samples[0].jd, J2000);
            assert_eq!(s.samples[0].conditions.aerosol_optical_depth, Some(0.30));
            assert_eq!(s.samples[0].conditions.cloud_high_pct, Some(40.0));
            assert_eq!(s.samples[1].conditions.aerosol_optical_depth, Some(0.34));
            assert_eq!(s.samples[2].conditions.aerosol_optical_depth, None);
        }
    }

    #[test]
    fn local_clock_times_are_stepped_back_to_utc() {
        // Asked for on India's clock (UTC+5:30 = 19800 s): 17:30 there is
        // 12:00 UTC, so the reading must land on J2000 all the same.
        let india = r#"{
            "utc_offset_seconds": 19800,
            "hourly": { "time": ["2000-01-01T17:30"], "cloud_cover_high": [55] }
        }"#;
        let s = series_from_responses(&[india], "test").unwrap();
        assert!((s.samples[0].jd - J2000).abs() < 1e-9, "jd = {}", s.samples[0].jd);
        assert_eq!(s.samples[0].conditions.cloud_high_pct, Some(55.0));
    }

    #[test]
    fn nonsense_values_are_dropped_not_trusted() {
        let odd = r#"{
            "hourly": {
                "time": [946728000, 946731600],
                "cloud_cover_high": [140, 30],
                "aerosol_optical_depth": [-1, 0.2]
            }
        }"#;
        let s = series_from_responses(&[odd], "test").unwrap();
        // The first hour had nothing usable, so only the second remains.
        assert_eq!(s.samples.len(), 1);
        assert!((s.samples[0].jd - (J2000 + HOUR)).abs() < 1e-9);
        assert_eq!(s.samples[0].conditions.cloud_high_pct, Some(30.0));
    }

    #[test]
    fn answers_for_several_places_become_weather_along_the_sun() {
        // A plan of three places: here, one point toward sunset (200 km) and
        // one toward sunrise (200 km). Each answer is a list in that order.
        let plan = WeatherPlan {
            points: vec![GeoPoint { lat: 0.0, lon: 0.0 }, GeoPoint { lat: 0.0, lon: -1.8 }, GeoPoint { lat: 0.0, lon: 1.8 }],
            sunset: Some(Direction { azimuth_deg: 270.0, first: 1, distances_km: vec![200.0] }),
            sunrise: Some(Direction { azimuth_deg: 90.0, first: 2, distances_km: vec![200.0] }),
        };
        let one = |high: i32| format!(r#"{{"hourly":{{"time":[946728000],"cloud_cover_high":[{high}]}}}}"#);
        let weather = format!("[{},{},{}]", one(10), one(20), one(30));
        let air = r#"[{"hourly":{"time":[946728000],"aerosol_optical_depth":[0.1]}},
                      {"hourly":{"time":[946728000],"aerosol_optical_depth":[null]}},
                      {"hourly":{"time":[946728000],"aerosol_optical_depth":[0.3]}}]"#;
        let w = weather_from_responses(&plan, &[&weather, air], "test").unwrap();
        assert_eq!(w.here_at(J2000).unwrap().cloud_high_pct, Some(10.0));
        assert_eq!(w.here_at(J2000).unwrap().aerosol_optical_depth, Some(0.1));
        let set = w.sunset.as_ref().unwrap();
        assert_eq!(set.azimuth_deg, 270.0);
        assert_eq!(set.points[0].distance_km, 200.0);
        assert_eq!(w.along_at(false, J2000)[0].1.cloud_high_pct, Some(20.0));
        assert_eq!(w.along_at(true, J2000)[0].1.cloud_high_pct, Some(30.0));
        assert_eq!(w.along_at(true, J2000)[0].1.aerosol_optical_depth, Some(0.3));

        // A single-place answer is "here" only.
        let w = weather_from_responses(&plan, &[&one(40)], "test").unwrap();
        assert_eq!(w.here_at(J2000).unwrap().cloud_high_pct, Some(40.0));
        assert!(w.sunset.is_none());

        // Wrong counts are reported in words.
        let two = format!("[{},{}]", one(1), one(2));
        assert!(weather_from_responses(&plan, &[&two], "test").unwrap_err().contains("cover 2 places"));
        assert!(weather_from_responses(&plan, &[&weather, &one(5)], "test").unwrap_err().contains("different numbers"));
        let nothing_here = format!("[{},{},{}]", r#"{"hourly":{"time":[946728000],"cloud_cover_high":[null]}}"#, one(2), one(3));
        assert!(weather_from_responses(&plan, &[&nothing_here], "test").unwrap_err().contains("this place"));
    }

    #[test]
    fn problems_are_reported_in_words() {
        let err = |texts: &[&str]| series_from_responses(texts, "test").unwrap_err();
        assert!(err(&[]).contains("no Open-Meteo answers"));
        assert!(err(&["{"]).contains("answer 1"));
        let refused = r#"{"error": true, "reason": "Parameter 'start_date' is out of allowed range"}"#;
        assert!(err(&[FORECAST, refused]).contains("answer 2: Open-Meteo said: Parameter 'start_date'"));
        assert!(err(&[r#"{"latitude": 1}"#]).contains("hourly"));
        assert!(err(&[r#"{"hourly": {"cloud_cover_low": [1]}}"#]).contains("time"));
        let short = r#"{"hourly": {"time": [946728000, 946731600], "cloud_cover_low": [1]}}"#;
        assert!(err(&[short]).contains("1 values for 2 times"));
        let nothing = r#"{"hourly": {"time": [946728000], "temperature_2m": [20]}}"#;
        assert!(err(&[nothing]).contains("none of the values"));
        let bad_time = r#"{"hourly": {"time": ["teatime"], "cloud_cover_low": [1]}}"#;
        assert!(err(&[bad_time]).contains("answer 1"));
    }

    #[test]
    fn the_variable_lists_name_only_things_this_module_reads() {
        for name in FORECAST_HOURLY.split(',').chain(AIR_QUALITY_HOURLY.split(',')) {
            assert!(slot(&mut Conditions::default(), name).is_some(), "{name} is asked for but not read");
        }
    }
}
