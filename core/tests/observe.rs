//! Replaying observations with the conditions stored in them, through the
//! real `skyglow` program (`--observe`).
//!
//! Expected numbers are worked out by hand from the readings written in each
//! test, never copied from the program's output.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Unix time of 2026-11-15 12:00 UTC, worked by hand: 2024-01-01 is
/// 1704067200; plus 366 days (2024, a leap year) and 365 days (2025) gives
/// 2026-01-01 = 1767225600; plus 318 days to 15 November (304 days in
/// January to October, then 14) gives 1794700800 at midnight; plus 12 h.
const NOV15_NOON_UTC: i64 = 1_794_744_000;
const HOUR: i64 = 3600;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// Run `skyglow` from the repository root; returns (success, stdout, stderr).
fn skyglow(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_skyglow"))
        .args(args)
        .current_dir(repo_root())
        .output()
        .expect("the skyglow program runs");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// A fresh folder for one test's files.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("skyglow-observe-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// An observation in Kochi (UTC+5:30) on 15 November with one event at 18:00,
/// which is 12:30 UTC: halfway between the 12:00 and 13:00 UTC readings.
fn write_observation(dir: &Path, conditions: &str) -> String {
    let text = format!(
        r#"{{ "place": "kochi", "date": "2026-11-15", "conditions": {conditions},
             "events": [ {{ "time": "18:00", "note": "test" }} ] }}"#
    );
    let path = dir.join("evening.json");
    std::fs::write(&path, text).unwrap();
    path.to_string_lossy().into_owned()
}

#[test]
fn the_shipped_example_observation_shows_its_conditions() {
    // The sample conditions have high cloud 60% at 12:30 UTC and 70% at
    // 13:30 UTC. 18:08 in Kochi is 12:38 UTC, 8 minutes into that hour:
    // 60 + 10 x 8 / 60 = 61.3, printed as 61%. The readings are labelled on
    // Kochi's clock: 18:00 and 19:00.
    let (ok, out, err) = skyglow(&["--observe", "examples/observations/kochi-sample.json"]);
    assert!(ok, "{err}");
    assert!(out.contains(">>> Observed 18:08"), "{out}");
    assert!(out.contains("Conditions, between the 18:00 and 19:00 readings"), "{out}");
    assert!(out.contains("high 61%"), "{out}");
    assert!(out.contains("made-up sample numbers"), "{out}");
}

#[test]
fn saved_open_meteo_answers_next_to_the_observation_are_read() {
    let dir = scratch("open-meteo");
    std::fs::write(
        dir.join("weather.json"),
        format!(r#"{{"hourly":{{"time":[{},{}],"cloud_cover_high":[20,80]}}}}"#, NOV15_NOON_UTC, NOV15_NOON_UTC + HOUR),
    )
    .unwrap();
    std::fs::write(
        dir.join("air.json"),
        format!(r#"{{"hourly":{{"time":[{},{}],"aerosol_optical_depth":[0.1,0.3]}}}}"#, NOV15_NOON_UTC, NOV15_NOON_UTC + HOUR),
    )
    .unwrap();
    // Relative names: found beside the observation, not in the current folder.
    let obs = write_observation(&dir, r#"{ "open_meteo": ["weather.json", "air.json"] }"#);
    let (ok, out, err) = skyglow(&["--observe", &obs]);
    assert!(ok, "{err}");
    // Halfway: (20 + 80) / 2 = 50% and (0.1 + 0.3) / 2 = 0.20. The UTC hours
    // 12:00 and 13:00 are 17:30 and 18:30 in Kochi.
    assert!(out.contains("between the 17:30 and 18:30 readings (Open-Meteo, saved with this observation)"), "{out}");
    assert!(out.contains("high 50%"), "{out}");
    assert!(out.contains("aerosol optical depth 0.20"), "{out}");
}

#[test]
fn readings_can_be_written_inside_the_observation() {
    let dir = scratch("inline");
    let obs = write_observation(
        &dir,
        r#"{ "source": "noted by hand", "samples": [
              { "utc": "2026-11-15T12:00", "humidity_pct": 60 },
              { "utc": "2026-11-15T13:00", "humidity_pct": 80 } ] }"#,
    );
    let (ok, out, err) = skyglow(&["--observe", &obs]);
    assert!(ok, "{err}");
    // Halfway between 60% and 80%.
    assert!(out.contains("(noted by hand)"), "{out}");
    assert!(out.contains("humidity       70% near the ground"), "{out}");
}

#[test]
fn the_command_line_replaces_the_observations_own_conditions_and_says_so() {
    let dir = scratch("override");
    let obs = write_observation(
        &dir,
        r#"{ "source": "noted by hand", "samples": [ { "utc": "2026-11-15T12:30", "humidity_pct": 60 } ] }"#,
    );
    let (ok, out, err) = skyglow(&["--observe", &obs, "--conditions", "examples/conditions/sample-evening.json"]);
    assert!(ok, "{err}");
    assert!(out.contains("used instead of the ones in the observation file"), "{out}");
    assert!(out.contains("made-up sample numbers"), "{out}");
    assert!(!out.contains("noted by hand"), "{out}");
}

#[test]
fn a_broken_conditions_block_is_an_error_that_names_the_problem() {
    let dir = scratch("broken");
    for (block, expect) in [
        (r#"{ "file": "missing.json" }"#, "cannot read"),
        (r#"{ "file": "a.json", "open_meteo": [] }"#, "expected"),
        (r#""cloudy""#, "expected"),
        (r#"{ "open_meteo": "weather.json" }"#, "list of file names"),
        (r#"{ "samples": [ { "utc": "2026-11-15T12:00", "cloud_high_pct": 150 } ] }"#, "cloud_high_pct"),
    ] {
        let obs = write_observation(&dir, block);
        let (ok, _, err) = skyglow(&["--observe", &obs]);
        assert!(!ok, "{block} should fail");
        assert!(err.contains("conditions") && err.contains(expect), "{block}: {err}");
    }
}
