//! `skyglow` command-line tool: print the evening light timeline for a place.
//!
//! ```text
//! skyglow 9.93 76.27 2026-11-15 --tz 5.5
//! ```

use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

use skyglow_core::conditions::{Basis, ConditionsSeries};
use skyglow_core::glow;
use skyglow_core::openmeteo;
use skyglow_core::params::Params;
use skyglow_core::place::Place;
use skyglow_core::sky::{self, LayerLight};
use skyglow_core::sun;
use skyglow_core::time;
use skyglow_core::timeline::{build_timeline_event, Event, Timeline};

const USAGE: &str = "\
skyglow - when does the evening sky do its thing?

USAGE:
    skyglow <lat> <lon> [YYYY-MM-DD] [options]
    skyglow --place <name> [YYYY-MM-DD] [options]
    skyglow --observe <file.json> [options]

ARGUMENTS:
    <lat>   Latitude in degrees, north positive      (Kochi: 9.93)
    <lon>   Longitude in degrees, east positive      (Kochi: 76.27)
    [date]  Local calendar date, default: today

OPTIONS:
    --tz <hours>     UTC offset of your clock, e.g. 5.5 for India (default 0 = UTC)
    --pack <file>    Use a parameter pack JSON file instead of the built-in default
    --event <name>   sunset, sunrise or both. Default: sunset, unless every --at
                     time is before solar noon (then sunrise), or the --at times
                     are mixed (then both)
    --at <HH:MM>     Report the sun and cloud-shadow state at a local clock time,
                     morning or evening (can be repeated, e.g. --at 18:08 --at 18:11)
    --place <name>   Use a named place (coordinates, height, skyline, time zone)
    --places <path>  Where places live: a folder of <name>.json files, or one JSON
                     file holding many places. Default search order:
                     test/places/<name>.json, test/places.json, examples/places/<name>.json
    --observe <file> Replay an observation file against the model, with the
                     conditions stored in it, if any (see docs/OBSERVATIONS.md)
    --conditions <file>
                     Cloud cover, moisture and haze readings to show next to each
                     --at time. Reported only: they do not change the prediction
                     yet (see docs/CONDITIONS.md)
    --open-meteo <file>
                     The same, from an answer you saved from the Open-Meteo weather
                     service (can be repeated: forecast and air quality are merged).
                     skyglow itself never goes online; docs/CONDITIONS.md has the
                     addresses to fetch
    --explain       Also print every intermediate value of the sun calculation
    -h, --help       Show this help

EXAMPLES:
    skyglow 9.93 76.27 2026-11-15 --tz 5.5
    skyglow 9.93 76.27 2026-11-15 --tz 5.5 --pack packs/kerala.json --at 18:08 --at 18:12
    skyglow --place singapore 2026-11-15
    skyglow --place quito 2026-11-15 --event sunrise
    skyglow --place kochi 2026-11-15 --tz 5.5 --at 06:10 --at 06:20
    skyglow --place kochi 2026-11-15 --at 18:08 --conditions examples/conditions/sample-evening.json
    skyglow --observe test/observations/my-sunset.json";

/// One timed note inside an observation file.
#[derive(Debug, Deserialize)]
struct ObservationEvent {
    time: String,
    #[serde(default)]
    note: String,
    /// Where the time came from, e.g. "photo" (EXIF) or "memory".
    #[serde(default)]
    source: String,
}

/// An observation file: what you saw, when, and where.
#[derive(Debug, Deserialize)]
struct Observation {
    place: Option<String>,
    lat: Option<f64>,
    lon: Option<f64>,
    date: String,
    tz: Option<f64>,
    pack: Option<String>,
    /// "sunset", "sunrise" or "both". Optional: guessed from the event times.
    event: Option<String>,
    #[serde(default)]
    notes: String,
    /// The air that evening: inline readings, a conditions file, or saved
    /// Open-Meteo answers. See `docs/OBSERVATIONS.md` and [`observation_conditions`].
    conditions: Option<serde_json::Value>,
    events: Vec<ObservationEvent>,
}

/// Parsed command-line options.
struct Options {
    positional: Vec<String>,
    tz: Option<f64>,
    pack: Option<String>,
    explain: bool,
    at: Vec<f64>,
    place: Option<String>,
    /// Explicit places folder or file; `None` means use the default search order.
    places_file: Option<String>,
    observe: Option<String>,
    event: Option<String>,
    conditions: Option<String>,
    /// Saved Open-Meteo answers (forecast and/or air quality), to be merged.
    open_meteo: Vec<String>,
}

fn main() {
    if let Err(message) = run() {
        eprintln!("error: {message}\n");
        eprintln!("{USAGE}");
        std::process::exit(2);
    }
}

fn take_value(args: &[String], i: &mut usize, message: &str) -> Result<String, String> {
    *i += 1;
    args.get(*i).cloned().ok_or_else(|| message.to_string())
}

fn parse_args() -> Result<Option<Options>, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut o = Options {
        positional: Vec::new(),
        tz: None,
        pack: None,
        explain: false,
        at: Vec::new(),
        place: None,
        places_file: None,
        observe: None,
        event: None,
        conditions: None,
        open_meteo: Vec::new(),
    };
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                println!("{USAGE}");
                return Ok(None);
            }
            "--explain" => o.explain = true,
            "--tz" => {
                let v = take_value(&args, &mut i, "--tz needs a value, for example --tz 5.5")?;
                o.tz = Some(
                    v.parse()
                        .map_err(|_| format!("cannot read timezone offset '{v}'"))?,
                );
            }
            "--pack" => o.pack = Some(take_value(&args, &mut i, "--pack needs a file path")?),
            "--at" => {
                let v = take_value(&args, &mut i, "--at needs a clock time, for example --at 18:08")?;
                o.at.push(time::parse_clock(&v)?);
            }
            "--event" => {
                let v = take_value(&args, &mut i, "--event needs sunset, sunrise or both")?;
                o.event = Some(v);
            }
            "--place" => o.place = Some(take_value(&args, &mut i, "--place needs a name")?),
            "--places" => {
                o.places_file = Some(take_value(&args, &mut i, "--places needs a file path")?)
            }
            "--observe" => {
                o.observe = Some(take_value(&args, &mut i, "--observe needs a file path")?)
            }
            "--conditions" => {
                o.conditions = Some(take_value(&args, &mut i, "--conditions needs a file path")?)
            }
            "--open-meteo" => {
                o.open_meteo.push(take_value(&args, &mut i, "--open-meteo needs a file path")?)
            }
            other if other.starts_with("--") => return Err(format!("unknown option {other}")),
            _ => o.positional.push(args[i].clone()),
        }
        i += 1;
    }
    Ok(Some(o))
}

fn parse_lat(s: &str) -> Result<f64, String> {
    let v: f64 = s.parse().map_err(|_| format!("cannot read latitude '{s}'"))?;
    if !(-90.0..=90.0).contains(&v) {
        return Err("latitude must be between -90 and 90".to_string());
    }
    Ok(v)
}

fn parse_lon(s: &str) -> Result<f64, String> {
    let v: f64 = s.parse().map_err(|_| format!("cannot read longitude '{s}'"))?;
    if !(-180.0..=180.0).contains(&v) {
        return Err("longitude must be between -180 and 180".to_string());
    }
    Ok(v)
}

fn load_params(path: Option<&String>) -> Result<Params, String> {
    match path {
        Some(p) => {
            let text =
                std::fs::read_to_string(p).map_err(|e| format!("cannot read pack '{p}': {e}"))?;
            Params::from_json(&text)
        }
        None => Ok(Params::builtin()),
    }
}

/// Read the conditions the user pointed at, if any (see `docs/CONDITIONS.md`):
/// a skyglow conditions file (`--conditions`), or one or more saved
/// Open-Meteo answers (`--open-meteo`), merged.
fn load_conditions(opts: &Options) -> Result<Option<ConditionsSeries>, String> {
    let read = |p: &String| {
        std::fs::read_to_string(p).map_err(|e| format!("cannot read conditions '{p}': {e}"))
    };
    match (&opts.conditions, opts.open_meteo.is_empty()) {
        (Some(_), false) => Err("use --conditions or --open-meteo, not both".to_string()),
        (Some(p), true) => ConditionsSeries::from_json(&read(p)?)
            .map(Some)
            .map_err(|e| format!("conditions file '{p}': {e}")),
        (None, false) => {
            let texts: Vec<String> = opts.open_meteo.iter().map(read).collect::<Result<_, _>>()?;
            let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
            openmeteo::series_from_responses(&refs, "Open-Meteo, from saved answers")
                .map(Some)
                .map_err(|e| format!("{e} (files in the order given: {})", opts.open_meteo.join(", ")))
        }
        (None, true) => Ok(None),
    }
}

/// The conditions stored with an observation, in one of three forms:
///
/// * `{ "source": ..., "samples": [...] }`: readings written right in the file;
/// * `{ "file": "name.json" }`: a skyglow conditions file;
/// * `{ "open_meteo": ["weather.json", "air.json"] }`: saved Open-Meteo answers.
///
/// File names are relative to the observation file's own folder, so an
/// evening's files can sit together and move together.
fn observation_conditions(block: &serde_json::Value, obs_path: &Path) -> Result<ConditionsSeries, String> {
    const FORMS: &str = "expected {\"samples\": [...]}, {\"file\": \"name.json\"} or {\"open_meteo\": [\"weather.json\", \"air.json\"]}";
    let map = block.as_object().ok_or_else(|| FORMS.to_string())?;
    let folder = obs_path.parent().unwrap_or(Path::new("."));
    let read = |name: &str| {
        let p = folder.join(name);
        std::fs::read_to_string(&p).map_err(|e| format!("cannot read '{}': {e}", p.display()))
    };
    let only = |key: &str| map.len() == 1 && map.contains_key(key);

    if map.contains_key("samples") {
        ConditionsSeries::from_json(&block.to_string())
    } else if only("file") {
        let name = map["file"].as_str().ok_or("\"file\" must be a file name")?;
        ConditionsSeries::from_json(&read(name)?).map_err(|e| format!("'{name}': {e}"))
    } else if only("open_meteo") {
        let names: Vec<&str> = map["open_meteo"]
            .as_array()
            .and_then(|list| list.iter().map(|v| v.as_str()).collect())
            .ok_or("\"open_meteo\" must be a list of file names")?;
        let texts: Vec<String> = names.iter().map(|n| read(n)).collect::<Result<_, _>>()?;
        let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
        openmeteo::series_from_responses(&refs, "Open-Meteo, saved with this observation")
    } else {
        Err(FORMS.to_string())
    }
}

/// Read one place file (`{ "name": ..., "lat": ..., ... }`).
fn read_place_file(path: &Path) -> Result<Place, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read '{}': {e}", path.display()))?;
    let place: Place = serde_json::from_str(&text)
        .map_err(|e| format!("bad place file '{}': {e}", path.display()))?;
    place.validate()?;
    Ok(place)
}

/// Read a file holding many places as `{ "key": { ... }, ... }` and pick one.
fn read_place_from_map(path: &Path, name: &str) -> Result<Option<Place>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read '{}': {e}", path.display()))?;
    let mut map: HashMap<String, Place> = serde_json::from_str(&text)
        .map_err(|e| format!("bad places file '{}': {e}", path.display()))?;
    match map.remove(name) {
        Some(mut p) => {
            if p.name.is_empty() {
                p.name = name.to_string();
            }
            p.validate()?;
            Ok(Some(p))
        }
        None => Ok(None),
    }
}

/// Look up a named place.
///
/// With `--places <folder>` we read `<folder>/<name>.json`; with
/// `--places <file>` we read that one file as a map of many places. With no
/// option we search, in order: `test/places/<name>.json` (private, git-ignored),
/// `test/places.json` (private map), `examples/places/<name>.json` (shipped).
fn get_place(name: &str, places: Option<&str>) -> Result<Place, String> {
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Err(format!(
            "place name '{name}' may only use letters, digits, '_' and '-'"
        ));
    }
    let mut tried: Vec<String> = Vec::new();
    let try_dir = |dir: &Path, tried: &mut Vec<String>| -> Result<Option<Place>, String> {
        let f = dir.join(format!("{name}.json"));
        if f.is_file() {
            let mut p = read_place_file(&f)?;
            if p.name.is_empty() {
                p.name = name.to_string();
            }
            return Ok(Some(p));
        }
        tried.push(f.display().to_string());
        Ok(None)
    };
    match places {
        Some(path) => {
            let p = Path::new(path);
            if p.is_dir() {
                if let Some(pl) = try_dir(p, &mut tried)? {
                    return Ok(pl);
                }
            } else if p.is_file() {
                if let Some(pl) = read_place_from_map(p, name)? {
                    return Ok(pl);
                }
                tried.push(format!("{path} (no place called '{name}')"));
            } else {
                tried.push(format!("{path} (not found)"));
            }
        }
        None => {
            if let Some(pl) = try_dir(Path::new("test/places"), &mut tried)? {
                return Ok(pl);
            }
            let map = Path::new("test/places.json");
            if map.is_file() {
                if let Some(pl) = read_place_from_map(map, name)? {
                    return Ok(pl);
                }
                tried.push(format!("test/places.json (no place called '{name}')"));
            }
            if let Some(pl) = try_dir(Path::new("examples/places"), &mut tried)? {
                return Ok(pl);
            }
        }
    }
    Err(format!("no place named '{name}'. Looked in: {}", tried.join(", ")))
}

/// Describe the clock the times are shown in, e.g.
/// "India Standard Time (IST), UTC+5.5", plus a hint about daylight saving.
fn clock_label(place: &Place, tz: f64) -> String {
    let utc = format!("UTC{:+}", tz);
    let name = if place.tz == Some(tz) && !place.tz_name.is_empty() {
        Some(place.tz_name.as_str())
    } else if place.dst_tz == Some(tz) && !place.dst_tz_name.is_empty() {
        Some(place.dst_tz_name.as_str())
    } else {
        None
    };
    let mut label = match name {
        Some(n) => format!("{n}, {utc}"),
        None => utc,
    };
    if let (Some(dst), Some(std_tz)) = (place.dst_tz, place.tz) {
        if std_tz == tz {
            let when = if place.dst_note.is_empty() {
                String::new()
            } else {
                format!(": {}", place.dst_note)
            };
            label.push_str(&format!(
                "  [this place also uses daylight saving, UTC{dst:+}{when}; use --tz {dst} to see that clock]"
            ));
        }
    }
    label
}

/// Local clock hours of solar noon on a date.
fn solar_noon_hours(place: &Place, date: (i32, u32, u32), tz: f64) -> f64 {
    let jd = sun::solar_noon(date.0, date.1, date.2, place.lon, tz);
    let local = jd + tz / 24.0 + 0.5;
    (local - local.floor()) * 24.0
}

/// Decide which events to describe.
///
/// An explicit `--event` wins. Otherwise: sunset (what most people watch),
/// unless the clock times you asked about say otherwise. Times before solar
/// noon mean sunrise; a mix of morning and evening times means both.
fn choose_events(
    explicit: Option<&str>,
    hours: &[f64],
    noon_hours: f64,
) -> Result<Vec<Event>, String> {
    if let Some(text) = explicit {
        if text.eq_ignore_ascii_case("both") {
            return Ok(vec![Event::Sunrise, Event::Sunset]);
        }
        return Ok(vec![Event::parse(text)?]);
    }
    let morning = hours.iter().filter(|h| **h < noon_hours).count();
    let evening = hours.len() - morning;
    Ok(match (morning, evening) {
        (0, _) => vec![Event::Sunset],
        (_, 0) => vec![Event::Sunrise],
        _ => vec![Event::Sunrise, Event::Sunset],
    })
}

fn run() -> Result<(), String> {
    let opts = match parse_args()? {
        Some(o) => o,
        None => return Ok(()),
    };

    if let Some(path) = opts.observe.clone() {
        return run_observation(&path, &opts);
    }

    let (place, rest): (Place, &[String]) = match &opts.place {
        Some(name) => (get_place(name, opts.places_file.as_deref())?, &opts.positional[..]),
        None => {
            if opts.positional.len() < 2 {
                return Err("expected <lat> <lon> (or --place <name>)".to_string());
            }
            let lat = parse_lat(&opts.positional[0])?;
            let lon = parse_lon(&opts.positional[1])?;
            (Place::flat(lat, lon), &opts.positional[2..])
        }
    };
    if rest.len() > 1 {
        return Err("too many arguments".to_string());
    }
    let tz = opts.tz.or(place.tz).unwrap_or(0.0);
    let date = match rest.first() {
        Some(s) => time::parse_date(s)?,
        None => time::today_local(tz),
    };
    let params = load_params(opts.pack.as_ref())?;
    let conditions = load_conditions(&opts)?;

    let events = choose_events(
        opts.event.as_deref(),
        &opts.at,
        solar_noon_hours(&place, date, tz),
    )?;
    let mut timelines = Vec::new();
    for (n, event) in events.iter().enumerate() {
        if n > 0 {
            println!();
        }
        let timeline = build_timeline_event(&params, &place, date, tz, *event);
        print_report(&timeline, &params, &place, tz);
        if let Some(series) = &conditions {
            print_best_glow(&params, &place, series, &timeline);
        }
        timelines.push(timeline);
    }
    for hours in &opts.at {
        print_at(&params, &place, date, tz, *hours, conditions.as_ref());
    }
    if conditions.is_some() && opts.at.is_empty() {
        println!("\n(conditions are shown next to --at times; add one, for example --at 18:08)");
    }
    if opts.explain {
        match timelines.first().and_then(|t| t.event_jd.map(|jd| (t.event, jd))) {
            Some((event, jd)) => print_explain(event, jd, place.lat, place.lon),
            None => println!("\n(no sunrise or sunset today at this latitude, nothing to explain)"),
        }
    }
    Ok(())
}

/// Replay an observation file: print the model's view at each noted time.
fn run_observation(path: &str, opts: &Options) -> Result<(), String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read observation file '{path}': {e}"))?;
    let obs: Observation =
        serde_json::from_str(&text).map_err(|e| format!("bad observation file '{path}': {e}"))?;

    let place = match (&obs.place, obs.lat, obs.lon) {
        (Some(name), _, _) => get_place(name, opts.places_file.as_deref())?,
        (None, Some(lat), Some(lon)) => {
            if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
                return Err("observation coordinates are out of range".to_string());
            }
            Place::flat(lat, lon)
        }
        _ => {
            return Err(
                "observation needs either \"place\" or both \"lat\" and \"lon\"".to_string(),
            )
        }
    };
    let tz = obs.tz.or(opts.tz).or(place.tz).unwrap_or(0.0);
    let date = time::parse_date(&obs.date)?;
    let params = load_params(obs.pack.as_ref().or(opts.pack.as_ref()))?;
    // Conditions given on the command line replace the observation's own.
    let from_command_line = load_conditions(opts)?;
    let replaced = from_command_line.is_some() && obs.conditions.is_some();
    let conditions = match (from_command_line, &obs.conditions) {
        (Some(c), _) => Some(c),
        (None, Some(block)) => Some(
            observation_conditions(block, Path::new(path))
                .map_err(|e| format!("observation file '{path}', conditions: {e}"))?,
        ),
        (None, None) => None,
    };

    let hours_list: Vec<f64> = obs
        .events
        .iter()
        .map(|e| time::parse_clock(&e.time))
        .collect::<Result<_, _>>()?;
    let events = choose_events(
        obs.event.as_deref().or(opts.event.as_deref()),
        &hours_list,
        solar_noon_hours(&place, date, tz),
    )?;
    for (n, event) in events.iter().enumerate() {
        if n > 0 {
            println!();
        }
        let timeline = build_timeline_event(&params, &place, date, tz, *event);
        print_report(&timeline, &params, &place, tz);
        if let Some(series) = &conditions {
            print_best_glow(&params, &place, series, &timeline);
        }
    }
    if !obs.notes.is_empty() {
        println!();
        println!("Notes: {}", obs.notes);
    }
    if replaced {
        println!();
        println!("(Conditions from the command line are used instead of the ones in the observation file.)");
    }
    for ev in &obs.events {
        let hours = time::parse_clock(&ev.time)?;
        println!();
        let source = if ev.source.is_empty() {
            String::new()
        } else {
            format!(" [{}]", ev.source)
        };
        println!(">>> Observed {}{}: {}", ev.time, source, ev.note);
        print_at(&params, &place, date, tz, hours, conditions.as_ref());
    }
    Ok(())
}

fn clock(jd: Option<f64>, tz: f64) -> String {
    match jd {
        Some(j) => time::format_clock(j, tz),
        None => "--:--:--".to_string(),
    }
}

fn minutes(a: Option<f64>, b: Option<f64>) -> String {
    match (a, b) {
        (Some(a), Some(b)) => format!("{:>3.0} min", (b - a) * 1440.0),
        _ => "      -".to_string(),
    }
}

fn print_report(t: &Timeline, params: &Params, place: &Place, tz: f64) {
    let rising = t.event == Event::Sunrise;
    let word = t.event.word();
    let cap = if rising { "Sunrise" } else { "Sunset" };
    let ns = if t.lat_deg >= 0.0 { 'N' } else { 'S' };
    let ew = if t.lon_deg >= 0.0 { 'E' } else { 'W' };
    let title = if place.name.is_empty() {
        String::new()
    } else {
        format!("{}  ", place.name)
    };
    println!(
        "Skyglow  {}{:.4}\u{b0}{}  {:.4}\u{b0}{}   {:04}-{:02}-{:02}   {}   pack: {}",
        title,
        t.lat_deg.abs(),
        ns,
        t.lon_deg.abs(),
        ew,
        t.date.0,
        t.date.1,
        t.date.2,
        word,
        params.name
    );
    println!("  Local time: {}", clock_label(place, tz));
    if t.elevation_m > 0.0 || t.has_skyline {
        println!("  Viewpoint: {:.0} m above sea level{}", t.elevation_m, if t.has_skyline { ", with a mountain skyline" } else { "" });
    }
    println!();
    println!("  Solar noon   {}", clock(Some(t.solar_noon_jd), t.tz_hours));

    // Minutes by which terrain moves the event away from the flat-horizon time
    // (earlier at sunset, later at sunrise).
    let terrain_min = match (t.event_jd, t.sea_event_jd) {
        (Some(a), Some(b)) => {
            if rising {
                (a - b) * 1440.0
            } else {
                (b - a) * 1440.0
            }
        }
        _ => 0.0,
    };
    match (t.event_jd, t.event_azimuth_deg) {
        (Some(jd), Some(az)) => {
            let label = if terrain_min >= 1.0 {
                if rising { "Sun appears " } else { "Sun hidden  " }
            } else {
                if rising { "Sunrise     " } else { "Sunset      " }
            };
            println!(
                "  {} {}   (look toward azimuth {:.0}\u{b0})",
                label,
                time::format_clock(jd, t.tz_hours),
                az
            );
        }
        _ => println!("  {:<12} none today (polar day or night)", cap),
    }
    if t.elevation_m > 0.0 || t.has_skyline {
        println!(
            "  {} at sea level, flat horizon: {}",
            cap,
            clock(t.sea_level_event_jd, t.tz_hours)
        );
        if t.elevation_m > 0.0 {
            let shift = match (t.sea_level_event_jd, t.sea_event_jd) {
                (Some(a), Some(b)) => (b - a).abs() * 1440.0,
                _ => 0.0,
            };
            println!(
                "  Flat horizon from your height:     {}   (horizon dips {:.2}\u{b0}, {:.0} min {})",
                clock(t.sea_event_jd, t.tz_hours),
                t.dip_deg,
                shift,
                if rising { "earlier" } else { "later" }
            );
        }
        if t.has_skyline {
            if terrain_min >= 1.0 {
                if rising {
                    println!(
                        "  Terrain hides the sun for {:.0} min after the flat-horizon sunrise;",
                        terrain_min
                    );
                    println!("  high clouds already catch light before the sun clears the skyline.");
                } else {
                    println!(
                        "  Terrain hides the sun {:.0} min before the flat-horizon sunset;",
                        terrain_min
                    );
                    println!("  high clouds keep glowing after that.");
                }
            } else {
                println!("  The skyline does not block the sun in its {} direction today.", if rising { "rising" } else { "setting" });
            }
        }
    }

    println!();
    if rising {
        println!("Morning light stages (by sun elevation, in the order they happen):");
    } else {
        println!("Evening light stages (by sun elevation):");
    }
    let ordered: Vec<_> = if rising {
        t.stages.iter().rev().collect()
    } else {
        t.stages.iter().collect()
    };
    for s in ordered {
        let (first, second) = if rising { (s.to_deg, s.from_deg) } else { (s.from_deg, s.to_deg) };
        println!(
            "  {:<13} {} -> {}  {}  [{:+.1}\u{b0} to {:+.1}\u{b0}]",
            s.name,
            clock(s.start_jd, t.tz_hours),
            clock(s.end_jd, t.tz_hours),
            minutes(s.start_jd, s.end_jd),
            first,
            second
        );
        if !s.description.is_empty() {
            println!("                {}", s.description);
        }
    }

    println!();
    if rising {
        println!("Cloud layers: when sunlight first reaches a layer directly overhead:");
    } else {
        println!("Cloud layers: when Earth's shadow reaches a layer directly overhead:");
    }
    for l in &t.layers {
        println!(
            "  {:<5} {:>5.1} km   {} overhead at {}   (sun {:.2}\u{b0} below horizon)",
            l.name,
            l.height_km,
            if rising { "lit" } else { "dark" },
            clock(l.overhead_shadow_jd, t.tz_hours),
            l.shadow_depression_deg
        );
    }
    println!();
    if rising {
        println!("Clouds toward the sun are lit earlier than these overhead times.");
    } else {
        println!("Clouds toward the sun stay lit longer than these overhead times.");
    }
}

/// Report the sun and Earth-shadow state at one local clock time.
///
/// All the physics is in [`sky::sky_state`]; this function only prints it.
fn print_at(
    params: &Params,
    place: &Place,
    date: (i32, u32, u32),
    tz: f64,
    hours: f64,
    conditions: Option<&ConditionsSeries>,
) {
    let jd = time::jd_from_local(date, hours, tz);
    print_sky_at(params, place, jd, tz);
    if let Some(series) = conditions {
        print_conditions(series, jd, tz);
        print_glow(params, place, series, jd);
    }
}

/// The glow score per layer, with every factor, when conditions are known.
fn print_glow(params: &Params, place: &Place, series: &ConditionsSeries, jd: f64) {
    let g = glow::glow_at(params, place, Some(series), jd);
    if !g.has_conditions {
        return;
    }
    println!("  Glow score, 0-10 (a rough guide, not calibrated yet):");
    for l in &g.layers {
        let f = &l.factors;
        let score = match (l.score, &l.word) {
            (Some(s), Some(w)) => format!("{s:>4.1}  {w:<7}"),
            _ => "   ?  unknown".to_string(),
        };
        let cover = f.cover.map_or("?".to_string(), |c| format!("{c:.2}"));
        println!(
            "    {:<5} {:>5.1} km  {}  = 10 x light {:.2} x sun {:.2} x colour {:.2} x cover {} x path {:.2} x view {:.2} x haze {:.2} x humidity {:.2}",
            l.name, l.height_km, score, f.light, f.sun, f.colour, cover, f.path, f.view, f.haze, f.humidity
        );
        if f.path < 1.0 {
            let names: Vec<&str> = l.blocked_by.iter().map(|lv| lv.word()).collect();
            println!("                       sunbeam passes through {} cloud", names.join(" and "));
        }
        if !l.unknown.is_empty() {
            println!("                       not known, so not counted: {}", l.unknown.join(", "));
        }
    }
}

/// The best glow moment around an event (the same windows as the web app's
/// colour strips), when conditions are known.
fn print_best_glow(params: &Params, place: &Place, series: &ConditionsSeries, t: &Timeline) {
    let Some(jd) = t.event_jd.or(t.sea_event_jd) else { return };
    let (before, after) = if t.event == Event::Sunrise { (60.0, 40.0) } else { (40.0, 60.0) };
    // Step on whole clock minutes, so the time printed is the minute checked.
    let on_minute = |x: f64| (x * 1440.0).round() / 1440.0;
    match glow::best_glow(params, place, series, on_minute(jd - before / 1440.0), on_minute(jd + after / 1440.0), 1.0) {
        Ok(Some(best)) if best.score > 0.0 => println!(
            "Best glow (rough guide): {} cloud ({:.1} km) around {}, {:.1}/10, {}.",
            best.layer,
            best.height_km,
            &time::format_clock(best.jd, t.tz_hours)[..5],
            best.score,
            best.word
        ),
        Ok(Some(_)) => println!("Best glow (rough guide): none expected; the forecast has no cloud to light."),
        Ok(None) => println!("Best glow: the conditions do not reach this {}.", t.event.word()),
        Err(e) => println!("Best glow: could not work it out ({e})."),
    }
}

/// The readings from a `--conditions` file at one instant, with how they were
/// worked out. Numbers only: no judgement about what they do to the sky yet.
fn print_conditions(series: &ConditionsSeries, jd: f64, tz: f64) {
    let hhmm = |j: f64| time::format_clock(j, tz)[..5].to_string();
    let Some(reading) = series.at(jd) else {
        println!("  Conditions: no reading close enough to this time");
        return;
    };
    let how = match reading.basis {
        Basis::Between { before_jd, after_jd } => {
            format!("between the {} and {} readings", hhmm(before_jd), hhmm(after_jd))
        }
        Basis::Nearest { sample_jd } => format!("from the {} reading", hhmm(sample_jd)),
    };
    let source = if series.source.is_empty() { "source not given" } else { series.source.as_str() };
    println!("  Conditions, {how} ({source}):");
    let c = reading.conditions;
    let cover: Vec<String> = [("low", c.cloud_low_pct), ("mid", c.cloud_mid_pct), ("high", c.cloud_high_pct)]
        .iter()
        .filter_map(|(name, v)| v.map(|v| format!("{name} {v:.0}%")))
        .collect();
    if !cover.is_empty() {
        println!("    cloud cover    {}", cover.join(", "));
    }
    if let Some(v) = c.humidity_pct {
        println!("    humidity       {v:.0}% near the ground");
    }
    if let Some(v) = c.water_vapour_mm {
        println!("    water vapour   {v:.0} mm in the air above");
    }
    if let Some(v) = c.aerosol_optical_depth {
        println!("    haze           aerosol optical depth {v:.2}");
    }
}

fn print_sky_at(params: &Params, place: &Place, jd: f64, tz: f64) {
    let st = sky::sky_state(params, place, jd);
    println!();
    println!("At {}:", time::format_clock(jd, tz));
    if st.sun_up() {
        println!(
            "  Sun is above the horizon: {:.2}\u{b0} elevation, azimuth {:.0}\u{b0}",
            st.elevation_deg, st.azimuth_deg
        );
        if let Some(view) = st.skyline {
            println!(
                "  Skyline in that direction: {:.1}\u{b0}  ->  sun is {}",
                view.horizon_elevation_deg,
                if view.sun_hidden { "hidden behind terrain" } else { "visible" }
            );
        }
        return;
    }
    println!(
        "  Sun is {:.2}\u{b0} below the horizon ({}), azimuth {:.0}\u{b0}",
        st.depression_deg,
        if st.rising { "rising" } else { "setting" },
        st.azimuth_deg
    );
    // Past 20 degrees the threshold is hundreds of km: not worth printing.
    if let Some(km) = st.overhead_lit_above_km.filter(|_| st.depression_deg < 20.0) {
        println!("  A cloud directly overhead is lit only if it is higher than {km:.1} km");
    }
    for l in &st.layers {
        match &l.light {
            LayerLight::LitOverhead { grazing_km, colour, .. } => {
                let hint = match colour {
                    Some(label) => format!(", likely {label}"),
                    None => String::new(),
                };
                println!(
                    "  {:<5} {:>5.1} km   lit overhead; sunlight skims {:.1} km above ground{}",
                    l.name, l.height_km, grazing_km, hint
                );
            }
            LayerLight::ShadowOverhead => println!(
                "  {:<5} {:>5.1} km   in Earth's shadow overhead",
                l.name, l.height_km
            ),
            LayerLight::EdgeBeyondHorizon => println!(
                "  {:<5} {:>5.1} km   shadow edge is past your horizon: none of this layer is lit",
                l.name, l.height_km
            ),
            LayerLight::LitTowardSun { edge_elevation_deg, edge_distance_km } => println!(
                "  {:<5} {:>5.1} km   lit only below {:.1}\u{b0} elevation toward the sun (shadow edge {:.0} km away; edge light is deep red-pink)",
                l.name, l.height_km, edge_elevation_deg, edge_distance_km
            ),
        }
    }
}

fn print_explain(event: Event, jd: f64, lat: f64, lon: f64) {
    let s = sun::sun_state(jd, lat, lon);
    println!();
    println!("Explain: the sun calculation at the moment of {}", event.word());
    println!("  Julian Day (UT)                  {:.6}", s.jd);
    println!("  Julian centuries since J2000     {:.9}", s.t_centuries);
    println!("  Mean longitude                   {:.4}\u{b0}", s.mean_longitude_deg);
    println!("  Mean anomaly                     {:.4}\u{b0}", s.mean_anomaly_deg);
    println!("  Orbit eccentricity               {:.6}", s.eccentricity);
    println!("  Equation of centre               {:.4}\u{b0}", s.equation_of_center_deg);
    println!("  True longitude                   {:.4}\u{b0}", s.true_longitude_deg);
    println!("  Apparent longitude               {:.4}\u{b0}", s.apparent_longitude_deg);
    println!("  Axial tilt (obliquity)           {:.4}\u{b0}", s.obliquity_deg);
    println!("  Declination (north/south of equator) {:+.4}\u{b0}", s.declination_deg);
    println!("  Equation of time                 {:+.3} min", s.equation_of_time_min);
    println!("  Hour angle (0 = solar noon)      {:+.3}\u{b0}", s.hour_angle_deg);
    println!("  Elevation (geometric)            {:+.3}\u{b0}", s.elevation_deg);
    println!("  Azimuth (from north, clockwise)  {:.3}\u{b0}", s.azimuth_deg);
}
