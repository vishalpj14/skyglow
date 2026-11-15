//! Keeps the shipped examples and parameter packs honest.
//!
//! If someone adds a place with a typo or a pack with a bad number, these
//! tests fail before it reaches anyone else. They also check a few "does the Earth behave like the Earth"
//! facts using the example places: the equator has fast twilight, polar
//! places can have no sunset at all.

use std::collections::BTreeMap;
use std::path::PathBuf;

use skyglow_core::params::Params;
use skyglow_core::place::Place;
use skyglow_core::sun;
use skyglow_core::timeline::build_timeline_for_place;

/// Path of a file or folder relative to the repository root.
fn repo_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join(relative)
}

/// Every `examples/places/<name>.json`, keyed by file name (without `.json`).
/// Each one is parsed and validated, so a typo fails here.
fn example_places() -> BTreeMap<String, Place> {
    let dir = repo_path("examples/places");
    let mut map = BTreeMap::new();
    for entry in std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display())) {
        let path = entry.expect("readable entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("readable place file");
        let place: Place = serde_json::from_str(&text)
            .unwrap_or_else(|e| panic!("bad JSON in {}: {e}", path.display()));
        place
            .validate()
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let key = path.file_stem().unwrap().to_string_lossy().to_string();
        map.insert(key, place);
    }
    assert!(map.len() >= 14, "expected the 14 example places, found {}", map.len());
    map
}

/// Minutes from sunset to the end of civil twilight (sun 6 degrees down), if both happen.
fn sunset_to_civil_dusk_minutes(p: &Place, date: (i32, u32, u32)) -> Option<f64> {
    let noon = sun::solar_noon(date.0, date.1, date.2, p.lon, p.tz.unwrap_or(0.0));
    let sunset = sun::evening_crossing(noon, p.lat, p.lon, -0.833)?;
    let dusk = sun::evening_crossing(noon, p.lat, p.lon, -6.0)?;
    Some((dusk - sunset) * 1440.0)
}

#[test]
fn every_example_place_is_valid_and_has_a_sunset_in_mid_november() {
    let params = Params::builtin();
    for (name, p) in example_places() {
        assert!(!p.tz_name.is_empty(), "{name}: missing tz_name");
        let tz = p.tz.unwrap_or_else(|| panic!("{name}: missing tz"));
        let tl = build_timeline_for_place(&params, &p, (2026, 11, 15), tz);
        let jd = tl
            .sunset_jd
            .unwrap_or_else(|| panic!("{name}: expected a sunset on 15 November"));
        let local = jd + tz / 24.0 + 0.5;
        let hour = (local - local.floor()) * 24.0;
        assert!(
            hour > 11.0 && hour < 21.5,
            "{name}: sunset at {hour:.2} local hours looks wrong (is tz right?)"
        );
    }
}

#[test]
fn twilight_is_fastest_at_the_equator_and_slowest_far_north() {
    let places = example_places();
    let date = (2026, 11, 15);
    let quito = sunset_to_civil_dusk_minutes(&places["quito"], date).expect("quito twilight");
    let singapore =
        sunset_to_civil_dusk_minutes(&places["singapore"], date).expect("singapore twilight");
    let reykjavik =
        sunset_to_civil_dusk_minutes(&places["reykjavik"], date).expect("reykjavik twilight");
    assert!(quito < 25.0, "quito took {quito} min");
    assert!(singapore < 25.0, "singapore took {singapore} min");
    assert!(reykjavik > 40.0, "reykjavik took {reykjavik} min");
}

#[test]
fn polar_night_and_midnight_sun_have_no_sunset() {
    let places = example_places();
    for name in ["tromso", "prudhoe_bay"] {
        let t = &places[name];
        let tz = t.tz.unwrap_or(0.0);

        // 21 December: the sun never climbs up to the horizon.
        let noon = sun::solar_noon(2026, 12, 21, t.lon, tz);
        assert!(
            sun::evening_crossing(noon, t.lat, t.lon, -0.833).is_none(),
            "{name}: expected polar night on 21 December"
        );

        // 21 June: the sun never goes down.
        let noon = sun::solar_noon(2026, 6, 21, t.lon, tz);
        assert!(
            sun::evening_crossing(noon, t.lat, t.lon, -0.833).is_none(),
            "{name}: expected midnight sun on 21 June"
        );
    }
}

#[test]
fn reykjavik_midsummer_sets_but_never_gets_dark() {
    let places = example_places();
    let r = &places["reykjavik"];
    let noon = sun::solar_noon(2026, 6, 21, r.lon, r.tz.unwrap_or(0.0));
    assert!(sun::evening_crossing(noon, r.lat, r.lon, -0.833).is_some(), "should set");
    assert!(sun::evening_crossing(noon, r.lat, r.lon, -6.0).is_none(), "never reaches civil dusk");
}

#[test]
fn every_pack_in_the_packs_folder_loads() {
    let dir = repo_path("packs");
    let mut count = 0;
    for entry in std::fs::read_dir(&dir).expect("packs folder should exist") {
        let path = entry.expect("readable entry").path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            let text = std::fs::read_to_string(&path).expect("readable pack");
            Params::from_json(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            count += 1;
        }
    }
    assert!(count >= 2, "expected at least default.json and kerala.json, found {count}");
}

#[test]
fn quito_sits_high_so_its_sunset_comes_later_than_at_sea_level() {
    let params = Params::builtin();
    let q = &example_places()["quito"];
    let tl = build_timeline_for_place(&params, q, (2026, 11, 15), -5.0);
    let delay = (tl.sea_sunset_jd.unwrap() - tl.sea_level_sunset_jd.unwrap()) * 1440.0;
    assert!(delay > 4.0 && delay < 9.0, "altitude delayed sunset by {delay} min");
}

#[test]
fn mountains_hide_the_sun_early_in_srinagar() {
    let params = Params::builtin();
    let s = &example_places()["srinagar"];
    let tl = build_timeline_for_place(&params, s, (2026, 11, 15), 5.5);
    assert!(tl.has_skyline);
    let early = (tl.sea_sunset_jd.unwrap() - tl.sunset_jd.unwrap()) * 1440.0;
    assert!(early > 3.0, "terrain only hid the sun {early} min early");
}

#[test]
fn the_yosemite_valley_walls_bring_the_sun_down_by_an_hour_or_more() {
    let params = Params::builtin();
    let y = &example_places()["yosemite"];
    let tl = build_timeline_for_place(&params, y, (2026, 11, 15), -8.0);
    let early = (tl.sea_sunset_jd.unwrap() - tl.sunset_jd.unwrap()) * 1440.0;
    assert!(early > 60.0 && early < 150.0, "valley hid the sun {early} min early");
}

#[test]
fn a_place_without_a_skyline_never_sees_the_sun_hidden_early() {
    let params = Params::builtin();
    let k = &example_places()["kochi"];
    let tl = build_timeline_for_place(&params, k, (2026, 11, 15), 5.5);
    assert!(!tl.has_skyline);
    assert_eq!(tl.sunset_jd, tl.sea_sunset_jd);
}

#[test]
fn daylight_saving_offsets_are_one_hour_apart() {
    for (name, p) in example_places() {
        if let (Some(std_tz), Some(dst)) = (p.tz, p.dst_tz) {
            assert!((dst - std_tz - 1.0).abs() < 1e-9, "{name}: dst should be standard + 1");
            assert!(!p.dst_tz_name.is_empty(), "{name}: missing dst_tz_name");
        }
    }
}
