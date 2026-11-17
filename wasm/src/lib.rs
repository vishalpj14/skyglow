//! # skyglow-wasm
//!
//! A thin bridge that lets a web page call `skyglow-core`. It adds no physics
//! of its own: every function reads its inputs, calls the core, and hands the
//! answer back.
//!
//! The bridge speaks **JSON text** in both directions. A place or a pack goes
//! in as the same JSON you would put in `examples/places/*.json` or
//! `packs/*.json`; results come back as JSON that the web app parses. That
//! keeps the bridge tiny and means the web app sees exactly the same field
//! names as the Rust code.
//!
//! An empty pack string means "the built-in default pack".
//!
//! The logic lives in plain Rust functions (tested with `cargo test`); the
//! `#[wasm_bindgen]` wrappers at the bottom only convert errors for
//! JavaScript.

use skyglow_core::conditions::{ConditionsSeries, Weather};
use skyglow_core::sunward::{self, WeatherPlan};
use skyglow_core::glow;
use skyglow_core::openmeteo;
use skyglow_core::params::{Params, DEFAULT_PACK_JSON};
use skyglow_core::place::Place;
use skyglow_core::sky::{self, SkyState};
use skyglow_core::sun;
use skyglow_core::time;
use skyglow_core::timeline::{build_timeline_event, Event};
use wasm_bindgen::prelude::*;

/// Most instants one `sky_series` call will compute (a day at 1-minute steps
/// is 1440), so a typo in the step cannot freeze the page.
pub const MAX_SERIES_POINTS: usize = 3000;

fn load_pack(pack_json: &str) -> Result<Params, String> {
    if pack_json.trim().is_empty() {
        Ok(Params::builtin())
    } else {
        Params::from_json(pack_json)
    }
}

fn load_place(place_json: &str) -> Result<Place, String> {
    let place: Place =
        serde_json::from_str(place_json).map_err(|e| format!("bad place JSON: {e}"))?;
    place.validate()?;
    Ok(place)
}

fn to_json<T: serde::Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string(value).map_err(|e| format!("could not encode result: {e}"))
}

fn check_date(year: i32, month: u32, day: u32) -> Result<(i32, u32, u32), String> {
    // Reuse the core's own date checks (month range, days in month).
    time::parse_date(&format!("{year:04}-{month:02}-{day:02}"))
}

/// The pack, validated and re-encoded, so the page can show stage names,
/// cloud heights and swatches.
pub fn pack_info_json(pack_json: &str) -> Result<String, String> {
    to_json(&load_pack(pack_json)?)
}

/// The timeline of one event (`"sunset"` or `"sunrise"`) for a place and a
/// local date. Same numbers as the command line's report.
pub fn timeline_json(
    place_json: &str,
    pack_json: &str,
    year: i32,
    month: u32,
    day: u32,
    tz_hours: f64,
    event: &str,
) -> Result<String, String> {
    let place = load_place(place_json)?;
    let params = load_pack(pack_json)?;
    let date = check_date(year, month, day)?;
    let event = Event::parse(event)?;
    to_json(&build_timeline_event(&params, &place, date, tz_hours, event))
}

/// The sky at one instant (Julian Day, UT). Same numbers as `--at`.
pub fn sky_at_json(place_json: &str, pack_json: &str, jd: f64) -> Result<String, String> {
    let place = load_place(place_json)?;
    let params = load_pack(pack_json)?;
    if !jd.is_finite() {
        return Err("time must be a finite Julian Day".to_string());
    }
    to_json(&sky::sky_state(&params, &place, jd))
}

/// The sky at evenly spaced instants from `jd_start` to `jd_end` (both
/// included), every `step_minutes`. Used for the sun path and colour strip.
pub fn sky_series(
    place: &Place,
    params: &Params,
    jd_start: f64,
    jd_end: f64,
    step_minutes: f64,
) -> Result<Vec<SkyState>, String> {
    if !(jd_start.is_finite() && jd_end.is_finite() && jd_end >= jd_start) {
        return Err("series needs a start before its end".to_string());
    }
    if !(step_minutes.is_finite() && step_minutes > 0.0) {
        return Err("series step must be more than 0 minutes".to_string());
    }
    let step = step_minutes / 1440.0;
    // Julian Days are about 2.46 million, so a difference of two carries
    // rounding error near 1e-9 days. Allow a millionth of a step of slack so
    // an end time that lands exactly on a step is kept.
    let count = ((jd_end - jd_start) / step + 1e-6).floor() as usize + 1;
    if count > MAX_SERIES_POINTS {
        return Err(format!(
            "series would have {count} points; the limit is {MAX_SERIES_POINTS}, use a larger step"
        ));
    }
    Ok((0..count)
        .map(|i| sky::sky_state(params, place, jd_start + i as f64 * step))
        .collect())
}

pub fn sky_series_json(
    place_json: &str,
    pack_json: &str,
    jd_start: f64,
    jd_end: f64,
    step_minutes: f64,
) -> Result<String, String> {
    let place = load_place(place_json)?;
    let params = load_pack(pack_json)?;
    to_json(&sky_series(&place, &params, jd_start, jd_end, step_minutes)?)
}

/// Merge the text of one or more Open-Meteo answers into a conditions series.
/// `responses_json` is a JSON list of those texts, e.g. `["{...}", "{...}"]`.
/// The page does the fetching; the core does the reading.
pub fn conditions_from_open_meteo_json(responses_json: &str, source: &str) -> Result<String, String> {
    let texts: Vec<String> = serde_json::from_str(responses_json)
        .map_err(|e| format!("expected a JSON list of answer texts: {e}"))?;
    let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
    to_json(&openmeteo::series_from_responses(&refs, source)?)
}

/// The conditions at one instant from a series (as produced above, or a
/// hand-written conditions file). `"null"` when the series has nothing close
/// enough to that time.
pub fn conditions_at_json(series_json: &str, jd: f64) -> Result<String, String> {
    let series = ConditionsSeries::from_json(series_json)?;
    to_json(&series.at(jd))
}

/// Weather from its JSON text: a weather bundle (here, sunset, sunrise) or a
/// plain conditions series (here only). Empty text means "no conditions".
fn load_weather(weather_json: &str) -> Result<Option<Weather>, String> {
    if weather_json.trim().is_empty() {
        Ok(None)
    } else {
        Weather::from_json(weather_json).map(Some)
    }
}

/// The places to ask the forecast about for a place and local date: the
/// place, then points toward sunset and sunrise (see `core/src/sunward.rs`).
pub fn weather_plan_json(
    place_json: &str,
    pack_json: &str,
    year: i32,
    month: u32,
    day: u32,
    tz_hours: f64,
) -> Result<String, String> {
    let place = load_place(place_json)?;
    let params = load_pack(pack_json)?;
    let date = check_date(year, month, day)?;
    to_json(&sunward::plan(&params, &place, date, tz_hours))
}

/// Read Open-Meteo answers for the places in a plan (each answer a list, one
/// entry per place, or a single answer for the place alone) into weather.
pub fn weather_from_open_meteo_json(plan_json: &str, responses_json: &str, source: &str) -> Result<String, String> {
    let plan: WeatherPlan = serde_json::from_str(plan_json).map_err(|e| format!("bad plan JSON: {e}"))?;
    let texts: Vec<String> = serde_json::from_str(responses_json)
        .map_err(|e| format!("expected a JSON list of answer texts: {e}"))?;
    let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
    to_json(&openmeteo::weather_from_responses(&plan, &refs, source)?)
}

/// The glow of every layer at one instant (see `core/src/glow.rs`).
pub fn glow_at_json(place_json: &str, pack_json: &str, weather_json: &str, jd: f64) -> Result<String, String> {
    let place = load_place(place_json)?;
    let params = load_pack(pack_json)?;
    let weather = load_weather(weather_json)?;
    if !jd.is_finite() {
        return Err("time must be a finite Julian Day".to_string());
    }
    to_json(&glow::glow_at(&params, &place, weather.as_ref(), jd))
}

/// The glow at evenly spaced instants, like `sky_series` (same limits).
pub fn glow_series_json(
    place_json: &str,
    pack_json: &str,
    weather_json: &str,
    jd_start: f64,
    jd_end: f64,
    step_minutes: f64,
) -> Result<String, String> {
    let place = load_place(place_json)?;
    let params = load_pack(pack_json)?;
    let weather = load_weather(weather_json)?;
    // Reuse sky_series for its checks and spacing.
    let skies = sky_series(&place, &params, jd_start, jd_end, step_minutes)?;
    let out: Vec<glow::Glow> = skies
        .iter()
        .map(|st| {
            let here = weather.as_ref().and_then(|w| w.here_at(st.jd));
            let along = weather.as_ref().map_or_else(Vec::new, |w| w.along_at(st.rising, st.jd));
            glow::glow(&params, st, here.as_ref(), &along)
        })
        .collect();
    to_json(&out)
}

/// The best glow moment in a window; JSON `null` when no score is known.
pub fn best_glow_json(
    place_json: &str,
    pack_json: &str,
    weather_json: &str,
    jd_start: f64,
    jd_end: f64,
    step_minutes: f64,
) -> Result<String, String> {
    let place = load_place(place_json)?;
    let params = load_pack(pack_json)?;
    match load_weather(weather_json)? {
        Some(weather) => to_json(&glow::best_glow(&params, &place, &weather, jd_start, jd_end, step_minutes)?),
        None => Ok("null".to_string()),
    }
}

// ---------------------------------------------------------------------------
// JavaScript-facing wrappers. Errors become JavaScript exceptions.
// ---------------------------------------------------------------------------

fn js<T>(r: Result<T, String>) -> Result<T, JsError> {
    r.map_err(|e| JsError::new(&e))
}

/// Text of the built-in default pack.
#[wasm_bindgen(js_name = defaultPack)]
pub fn default_pack() -> String {
    DEFAULT_PACK_JSON.to_string()
}

#[wasm_bindgen(js_name = packInfo)]
pub fn pack_info(pack_json: &str) -> Result<String, JsError> {
    js(pack_info_json(pack_json))
}

#[wasm_bindgen]
pub fn timeline(
    place_json: &str,
    pack_json: &str,
    year: i32,
    month: u32,
    day: u32,
    tz_hours: f64,
    event: &str,
) -> Result<String, JsError> {
    js(timeline_json(place_json, pack_json, year, month, day, tz_hours, event))
}

#[wasm_bindgen(js_name = skyAt)]
pub fn sky_at(place_json: &str, pack_json: &str, jd: f64) -> Result<String, JsError> {
    js(sky_at_json(place_json, pack_json, jd))
}

#[wasm_bindgen(js_name = skySeries)]
pub fn sky_series_js(
    place_json: &str,
    pack_json: &str,
    jd_start: f64,
    jd_end: f64,
    step_minutes: f64,
) -> Result<String, JsError> {
    js(sky_series_json(place_json, pack_json, jd_start, jd_end, step_minutes))
}

#[wasm_bindgen(js_name = conditionsFromOpenMeteo)]
pub fn conditions_from_open_meteo(responses_json: &str, source: &str) -> Result<String, JsError> {
    js(conditions_from_open_meteo_json(responses_json, source))
}

#[wasm_bindgen(js_name = conditionsAt)]
pub fn conditions_at(series_json: &str, jd: f64) -> Result<String, JsError> {
    js(conditions_at_json(series_json, jd))
}

#[wasm_bindgen(js_name = weatherPlan)]
pub fn weather_plan(place_json: &str, pack_json: &str, year: i32, month: u32, day: u32, tz_hours: f64) -> Result<String, JsError> {
    js(weather_plan_json(place_json, pack_json, year, month, day, tz_hours))
}

#[wasm_bindgen(js_name = weatherFromOpenMeteo)]
pub fn weather_from_open_meteo(plan_json: &str, responses_json: &str, source: &str) -> Result<String, JsError> {
    js(weather_from_open_meteo_json(plan_json, responses_json, source))
}

#[wasm_bindgen(js_name = glowAt)]
pub fn glow_at(place_json: &str, pack_json: &str, weather_json: &str, jd: f64) -> Result<String, JsError> {
    js(glow_at_json(place_json, pack_json, weather_json, jd))
}

#[wasm_bindgen(js_name = glowSeries)]
pub fn glow_series(
    place_json: &str,
    pack_json: &str,
    series_json: &str,
    jd_start: f64,
    jd_end: f64,
    step_minutes: f64,
) -> Result<String, JsError> {
    js(glow_series_json(place_json, pack_json, series_json, jd_start, jd_end, step_minutes))
}

#[wasm_bindgen(js_name = bestGlow)]
pub fn best_glow(
    place_json: &str,
    pack_json: &str,
    series_json: &str,
    jd_start: f64,
    jd_end: f64,
    step_minutes: f64,
) -> Result<String, JsError> {
    js(best_glow_json(place_json, pack_json, series_json, jd_start, jd_end, step_minutes))
}

/// The `hourly=` list to ask Open-Meteo's forecast service for.
#[wasm_bindgen(js_name = openMeteoForecastHourly)]
pub fn open_meteo_forecast_hourly() -> String {
    openmeteo::FORECAST_HOURLY.to_string()
}

/// The `hourly=` list to ask Open-Meteo's air-quality service for.
#[wasm_bindgen(js_name = openMeteoAirQualityHourly)]
pub fn open_meteo_air_quality_hourly() -> String {
    openmeteo::AIR_QUALITY_HOURLY.to_string()
}

/// Julian Day (UT) for a local date and clock time (`hours`, e.g. 18.5).
#[wasm_bindgen(js_name = jdFromLocal)]
pub fn jd_from_local(year: i32, month: u32, day: u32, hours: f64, tz_hours: f64) -> f64 {
    time::jd_from_local((year, month, day), hours, tz_hours)
}

/// `HH:MM:SS` on a clock `tz_hours` from UTC.
#[wasm_bindgen(js_name = formatClock)]
pub fn format_clock(jd: f64, tz_hours: f64) -> String {
    time::format_clock(jd, tz_hours)
}

/// Julian Day of solar noon for a local date.
#[wasm_bindgen(js_name = solarNoon)]
pub fn solar_noon(year: i32, month: u32, day: u32, lon_deg: f64, tz_hours: f64) -> f64 {
    sun::solar_noon(year, month, day, lon_deg, tz_hours)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const KOCHI: &str = r#"{"name":"Kochi","lat":9.93,"lon":76.27,"tz":5.5}"#;

    fn local_hours(jd: f64, tz: f64) -> f64 {
        let local = jd + tz / 24.0 + 0.5;
        (local - local.floor()) * 24.0
    }

    #[test]
    fn timeline_gives_kochi_sunset_near_six_pm() {
        // Almanac: Kochi sunset on 15 Nov is within a few minutes of 17:59 IST.
        let v: Value =
            serde_json::from_str(&timeline_json(KOCHI, "", 2026, 11, 15, 5.5, "sunset").unwrap())
                .unwrap();
        assert_eq!(v["event"], "sunset");
        let h = local_hours(v["event_jd"].as_f64().unwrap(), 5.5);
        assert!(h > 17.0 + 56.0 / 60.0 && h < 18.0 + 3.0 / 60.0, "sunset at {h}");
        assert_eq!(v["stages"].as_array().unwrap().len(), 5);
    }

    #[test]
    fn sky_at_reports_layers_with_a_kind_tag() {
        // 18:30 IST, sun several degrees down: every layer has a "kind".
        let jd = jd_from_local(2026, 11, 15, 18.5, 5.5);
        let v: Value = serde_json::from_str(&sky_at_json(KOCHI, "", jd).unwrap()).unwrap();
        assert_eq!(v["rising"], false);
        assert!(v["elevation_deg"].as_f64().unwrap() < 0.0);
        for layer in v["layers"].as_array().unwrap() {
            assert!(layer["light"]["kind"].is_string(), "{layer}");
        }
    }

    #[test]
    fn series_spacing_and_limits() {
        let place = load_place(KOCHI).unwrap();
        let params = Params::builtin();
        let start = jd_from_local(2026, 11, 15, 17.0, 5.5);
        // One hour at 10-minute steps is 7 points, both ends included.
        let s = sky_series(&place, &params, start, start + 1.0 / 24.0, 10.0).unwrap();
        assert_eq!(s.len(), 7);
        assert!((s[6].jd - s[0].jd - 1.0 / 24.0).abs() < 1e-9);
        // The sun only sinks between 17:00 and 18:00 in Kochi.
        assert!(s.windows(2).all(|w| w[1].elevation_deg < w[0].elevation_deg));
        assert!(sky_series(&place, &params, start, start + 10.0, 1.0).is_err());
        assert!(sky_series(&place, &params, start, start + 1.0, 0.0).is_err());
        assert!(sky_series(&place, &params, start + 1.0, start, 5.0).is_err());
    }

    #[test]
    fn bad_input_is_an_error_not_a_crash() {
        assert!(timeline_json("{", "", 2026, 11, 15, 5.5, "sunset").is_err());
        assert!(timeline_json(r#"{"lat":95,"lon":0}"#, "", 2026, 11, 15, 0.0, "sunset").is_err());
        assert!(timeline_json(KOCHI, "", 2026, 2, 30, 5.5, "sunset").is_err());
        assert!(timeline_json(KOCHI, "", 2026, 11, 15, 5.5, "noon").is_err());
        assert!(timeline_json(KOCHI, "{\"schema\":9}", 2026, 11, 15, 5.5, "sunset").is_err());
        assert!(sky_at_json(KOCHI, "", f64::NAN).is_err());
    }

    #[test]
    fn open_meteo_answers_go_in_and_readings_come_out() {
        // Unix 946728000 is 2000-01-01 12:00 UTC, Julian Day 2451545.0 (J2000).
        let forecast = r#"{"hourly":{"time":[946728000,946731600],"cloud_cover_high":[40,60]}}"#;
        let air = r#"{"hourly":{"time":[946728000,946731600],"aerosol_optical_depth":[0.2,0.4]}}"#;
        let list = serde_json::to_string(&[forecast, air]).unwrap();
        let series = conditions_from_open_meteo_json(&list, "test").unwrap();
        // Half an hour in: (40 + 60) / 2 = 50 and (0.2 + 0.4) / 2 = 0.3.
        let r: Value =
            serde_json::from_str(&conditions_at_json(&series, 2_451_545.0 + 0.5 / 24.0).unwrap()).unwrap();
        assert!((r["conditions"]["cloud_high_pct"].as_f64().unwrap() - 50.0).abs() < 1e-6);
        assert!((r["conditions"]["aerosol_optical_depth"].as_f64().unwrap() - 0.3).abs() < 1e-6);
        assert_eq!(r["basis"]["kind"], "between");
        assert!(r["conditions"].get("humidity_pct").is_none(), "unknown values are left out");
        // A day later the series has nothing to say: JSON null, not an error.
        assert_eq!(conditions_at_json(&series, 2_451_546.0).unwrap(), "null");
        assert!(conditions_from_open_meteo_json("not a list", "test").is_err());
        assert!(conditions_from_open_meteo_json(r#"["{\"error\":true,\"reason\":\"nope\"}"]"#, "test")
            .unwrap_err()
            .contains("nope"));
    }

    #[test]
    fn glow_goes_through_the_bridge() {
        // 18:08 IST on 15 Nov: sun about 2.8 deg down, the 9 km layer lit
        // overhead. A clear sky in the conditions means a score of 0 for every
        // layer; with no conditions there is only the potential.
        let jd = jd_from_local(2026, 11, 15, 18.0 + 8.0 / 60.0, 5.5);
        let clear = r#"{"samples":[{"jd":2461360.0,"cloud_low_pct":0,"cloud_mid_pct":0,"cloud_high_pct":0},
                                    {"jd":2461360.25,"cloud_low_pct":0,"cloud_mid_pct":0,"cloud_high_pct":0}]}"#;
        let v: Value = serde_json::from_str(&glow_at_json(KOCHI, "", clear, jd).unwrap()).unwrap();
        assert_eq!(v["has_conditions"], true);
        for l in v["layers"].as_array().unwrap() {
            assert_eq!(l["score"], 0.0, "{l}");
        }
        let v: Value = serde_json::from_str(&glow_at_json(KOCHI, "", "", jd).unwrap()).unwrap();
        assert_eq!(v["has_conditions"], false);
        assert!(v["layers"][2]["potential"].as_f64().unwrap() > 0.0, "the high layer could glow");
        assert!(v["layers"][2]["score"].is_null());

        let s: Value = serde_json::from_str(&glow_series_json(KOCHI, "", clear, jd, jd + 1.0 / 24.0, 10.0).unwrap()).unwrap();
        assert_eq!(s.as_array().unwrap().len(), 7);
        // No cloud anywhere: the best moment scores 0.
        let b: Value = serde_json::from_str(&best_glow_json(KOCHI, "", clear, jd, jd + 0.02, 1.0).unwrap()).unwrap();
        assert_eq!(b["score"], 0.0);
        assert_eq!(best_glow_json(KOCHI, "", "", jd, jd + 0.02, 1.0).unwrap(), "null");
        assert!(glow_at_json(KOCHI, "", "{", jd).is_err());
    }

    #[test]
    fn a_plan_and_its_answers_make_weather_for_the_glow() {
        // Kochi, 15 Nov, default pack: the place plus 5 points each toward
        // sunset and sunrise.
        let plan_json = weather_plan_json(KOCHI, "", 2026, 11, 15, 5.5).unwrap();
        let plan: Value = serde_json::from_str(&plan_json).unwrap();
        let n = plan["points"].as_array().unwrap().len();
        assert_eq!(n, 11);
        assert_eq!(plan["sunset"]["first"], 1);
        assert_eq!(plan["sunrise"]["first"], 6);
        // One answer listing all 11 places, the same high cloud everywhere
        // except a solid low deck along the way.
        let one = |low: i32| format!(r#"{{"hourly":{{"time":[1794744000,1794747600],"cloud_cover_low":[{low},{low}],"cloud_cover_high":[60,60]}}}}"#);
        let answer = format!("[{}]", (0..n).map(|i| one(if i == 0 { 0 } else { 100 })).collect::<Vec<_>>().join(","));
        let list = serde_json::to_string(&[answer]).unwrap();
        let weather = weather_from_open_meteo_json(&plan_json, &list, "test").unwrap();
        // 18:00 IST = 12:30 UTC, inside the readings.
        let jd = jd_from_local(2026, 11, 15, 18.0, 5.5);
        let g: Value = serde_json::from_str(&glow_at_json(KOCHI, "", &weather, jd).unwrap()).unwrap();
        assert_eq!(g["along_sun"], true);
        // The plain series still works as "here only".
        let here_only: Value = serde_json::from_str(&weather).unwrap();
        let plain = serde_json::to_string(&here_only["here"]).unwrap();
        let g: Value = serde_json::from_str(&glow_at_json(KOCHI, "", &plain, jd).unwrap()).unwrap();
        assert_eq!(g["along_sun"], false);
        assert!(weather_from_open_meteo_json("{", &list, "t").is_err());
    }

    #[test]
    fn regional_pack_is_used_when_given() {
        let kerala = include_str!("../../packs/kerala.json");
        let v: Value = serde_json::from_str(&pack_info_json(kerala).unwrap()).unwrap();
        assert_eq!(v["name"], "kerala");
        let d: Value = serde_json::from_str(&pack_info_json("").unwrap()).unwrap();
        assert_eq!(d["name"], "default");
    }
}
