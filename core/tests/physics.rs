//! Physics sanity tests that exercise the public API the same way the CLI and
//! the web app will. Expected values come from well-known facts (equinox,
//! solstice, equation-of-time extremes) and hand calculation, not from the
//! code under test.

use skyglow_core::params::Params;
use skyglow_core::sun;
use skyglow_core::time::julian_day;
use skyglow_core::timeline::build_timeline;

const KOCHI: (f64, f64) = (9.93, 76.27);
const IST: f64 = 5.5;

/// Local hour of day (0..24) for a Julian Day at a UTC offset.
fn local_hours(jd: f64, tz: f64) -> f64 {
    let local = jd + tz / 24.0 + 0.5;
    (local - local.floor()) * 24.0
}

/// Minutes between the sun setting (-0.833 deg) and reaching -6 deg (civil twilight end).
fn sunset_to_civil_dusk_minutes(lat: f64, lon: f64, date: (i32, u32, u32), tz: f64) -> f64 {
    let noon = sun::solar_noon(date.0, date.1, date.2, lon, tz);
    let sunset = sun::evening_crossing(noon, lat, lon, -0.833).expect("sunset exists");
    let dusk = sun::evening_crossing(noon, lat, lon, -6.0).expect("civil dusk exists");
    (dusk - sunset) * 1440.0
}

#[test]
fn kochi_sunset_is_about_six_pm_in_mid_november() {
    let params = Params::builtin();
    let tl = build_timeline(&params, KOCHI.0, KOCHI.1, (2026, 11, 15), IST);
    let h = local_hours(tl.event_jd.expect("sunset exists"), IST);
    // Published almanac values for Kochi in mid-November are within a few minutes of 17:59.
    assert!(h > 17.0 + 56.0 / 60.0 && h < 18.0 + 3.0 / 60.0, "sunset at {h} h");
}

#[test]
fn kochi_sunset_azimuth_is_west_south_west() {
    let params = Params::builtin();
    let tl = build_timeline(&params, KOCHI.0, KOCHI.1, (2026, 11, 15), IST);
    let az = tl.event_azimuth_deg.expect("sunset exists");
    // Declination is about -18.7 deg in mid-November: cos(az) = sin(dec)/cos(lat) gives ~251 deg.
    assert!(az > 247.0 && az < 255.0, "azimuth {az}");
}

#[test]
fn equatorial_twilight_is_much_shorter_than_london() {
    let kochi = sunset_to_civil_dusk_minutes(KOCHI.0, KOCHI.1, (2026, 11, 15), IST);
    let london = sunset_to_civil_dusk_minutes(51.5074, -0.1278, (2026, 11, 15), 0.0);
    assert!(kochi > 19.0 && kochi < 24.0, "Kochi sunset to civil dusk: {kochi} min");
    assert!(london > 30.0, "London sunset to civil dusk: {london} min");
    assert!(kochi < london);
}

#[test]
fn equinox_noon_sun_is_overhead_at_equator() {
    let noon = sun::solar_noon(2026, 3, 20, 0.0, 0.0);
    let s = sun::sun_state(noon, 0.0, 0.0);
    assert!(s.elevation_deg > 89.5, "elevation {}", s.elevation_deg);
}

#[test]
fn solstice_declination_matches_axial_tilt() {
    let s = sun::sun_state(julian_day(2026, 6, 21, 12.0), 0.0, 0.0);
    assert!((s.declination_deg - 23.44).abs() < 0.05, "dec {}", s.declination_deg);
    let s = sun::sun_state(julian_day(2026, 12, 21, 12.0), 0.0, 0.0);
    assert!((s.declination_deg + 23.44).abs() < 0.05, "dec {}", s.declination_deg);
}

#[test]
fn equation_of_time_extremes() {
    // Sundial is about 16.4 min ahead of the clock in early November and
    // about 14.2 min behind in mid-February.
    let nov = sun::sun_state(julian_day(2026, 11, 3, 12.0), 0.0, 0.0).equation_of_time_min;
    let feb = sun::sun_state(julian_day(2026, 2, 11, 12.0), 0.0, 0.0).equation_of_time_min;
    assert!(nov > 16.0 && nov < 16.9, "november {nov}");
    assert!(feb < -13.8 && feb > -14.6, "february {feb}");
}

#[test]
fn midnight_sun_means_no_sunset() {
    let noon = sun::solar_noon(2026, 6, 21, 0.0, 0.0);
    assert!(sun::evening_crossing(noon, 80.0, 0.0, -0.833).is_none());
}

#[test]
fn stages_follow_each_other_in_time() {
    let params = Params::builtin();
    let tl = build_timeline(&params, KOCHI.0, KOCHI.1, (2026, 11, 15), IST);
    for pair in tl.stages.windows(2) {
        let end = pair[0].end_jd.expect("end");
        let next_start = pair[1].start_jd.expect("start");
        assert!((end - next_start).abs() < 1e-6, "{} -> {}", pair[0].name, pair[1].name);
    }
    for stage in &tl.stages {
        assert!(stage.end_jd.unwrap() > stage.start_jd.unwrap(), "{}", stage.name);
    }
}

#[test]
fn higher_cloud_layers_go_dark_later() {
    let params = Params::builtin();
    let tl = build_timeline(&params, KOCHI.0, KOCHI.1, (2026, 11, 15), IST);
    let times: Vec<f64> = tl.layers.iter().map(|l| l.overhead_shadow_jd.unwrap()).collect();
    assert!(times[0] < times[1] && times[1] < times[2], "{times:?}");
}

// ---------------------------------------------------------------------------
// Sunrise: the same geometry as sunset, played backwards.
// ---------------------------------------------------------------------------

use skyglow_core::place::{HorizonPoint, Place};
use skyglow_core::timeline::{build_timeline_event, Event};

fn kochi_event(event: Event) -> skyglow_core::timeline::Timeline {
    let params = Params::builtin();
    let place = Place::flat(KOCHI.0, KOCHI.1);
    build_timeline_event(&params, &place, (2026, 11, 15), IST, event)
}

#[test]
fn kochi_sunrise_is_about_a_quarter_past_six_in_mid_november() {
    let h = local_hours(kochi_event(Event::Sunrise).event_jd.expect("sunrise exists"), IST);
    assert!(h > 6.0 && h < 6.6, "sunrise at {h:.2} local hours");
}

#[test]
fn sunrise_and_sunset_mirror_each_other_around_solar_noon() {
    let rise = kochi_event(Event::Sunrise);
    let set = kochi_event(Event::Sunset);
    let before = rise.solar_noon_jd - rise.event_jd.unwrap();
    let after = set.event_jd.unwrap() - set.solar_noon_jd;
    // The sun's declination drifts by a fraction of a degree in a day, so the
    // two halves differ by well under two minutes.
    assert!((before - after).abs() * 86_400.0 < 120.0, "halves differ by {} s", (before - after).abs() * 86_400.0);
}

#[test]
fn kochi_sunrise_is_toward_the_east_south_east_in_november() {
    // cos(azimuth) = sin(declination) / cos(latitude) = -0.32 -> about 109 degrees.
    let az = kochi_event(Event::Sunrise).event_azimuth_deg.expect("sunrise exists");
    assert!(az > 100.0 && az < 118.0, "azimuth {az}");
}

#[test]
fn sunrise_stages_run_in_the_opposite_order_and_use_morning_names() {
    let tl = kochi_event(Event::Sunrise);
    let names: Vec<&str> = tl.stages.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"sunrise_glow"), "names: {names:?}");
    assert!(names.contains(&"first_light"), "names: {names:?}");
    for s in &tl.stages {
        assert!(s.start_jd.unwrap() < s.end_jd.unwrap(), "{} runs backwards", s.name);
    }
    // The pack lists stages in evening order, so each next entry happens *earlier* at sunrise.
    for pair in tl.stages.windows(2) {
        let later = pair[0].start_jd.unwrap();
        let earlier = pair[1].end_jd.unwrap();
        assert!((later - earlier).abs() < 1e-6, "stages do not meet at their boundary");
    }
}

#[test]
fn high_clouds_catch_the_morning_light_before_low_clouds() {
    let tl = kochi_event(Event::Sunrise);
    let times: Vec<f64> = tl.layers.iter().map(|l| l.overhead_shadow_jd.unwrap()).collect();
    // layers are low, mid, high: the highest is lit first (earliest time).
    assert!(times[2] < times[1] && times[1] < times[0], "times: {times:?}");
}

#[test]
fn an_eastern_ridge_delays_sunrise_and_height_advances_it() {
    let params = Params::builtin();
    let flat = Place::flat(KOCHI.0, KOCHI.1);
    let mut ridge = Place::flat(KOCHI.0, KOCHI.1);
    ridge.horizon = vec![HorizonPoint { az: 0.0, elev: 10.0 }];
    let mut high = Place::flat(KOCHI.0, KOCHI.1);
    high.elevation_m = 2000.0;
    let date = (2026, 11, 15);
    let f = build_timeline_event(&params, &flat, date, IST, Event::Sunrise).event_jd.unwrap();
    let r = build_timeline_event(&params, &ridge, date, IST, Event::Sunrise).event_jd.unwrap();
    let h = build_timeline_event(&params, &high, date, IST, Event::Sunrise).event_jd.unwrap();
    let ridge_delay = (r - f) * 1440.0;
    let height_gain = (f - h) * 1440.0;
    assert!(ridge_delay > 35.0 && ridge_delay < 55.0, "ridge delayed sunrise by {ridge_delay} min");
    assert!(height_gain > 4.0 && height_gain < 8.0, "height advanced sunrise by {height_gain} min");
}

#[test]
fn midnight_sun_and_polar_night_have_no_sunrise() {
    let params = Params::builtin();
    let tromso = Place::flat(69.6492, 18.9553);
    for date in [(2026, 6, 21), (2026, 12, 21)] {
        let tl = build_timeline_event(&params, &tromso, date, 1.0, Event::Sunrise);
        assert!(tl.event_jd.is_none(), "unexpected sunrise on {date:?}");
    }
}
