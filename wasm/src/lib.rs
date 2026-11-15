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
    fn regional_pack_is_used_when_given() {
        let kerala = include_str!("../../packs/kerala.json");
        let v: Value = serde_json::from_str(&pack_info_json(kerala).unwrap()).unwrap();
        assert_eq!(v["name"], "kerala");
        let d: Value = serde_json::from_str(&pack_info_json("").unwrap()).unwrap();
        assert_eq!(d["name"], "default");
    }
}
