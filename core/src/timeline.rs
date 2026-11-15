//! One sunset or sunrise, start to finish.
//!
//! [`build_timeline_event`] glues the pieces together: it finds solar noon,
//! then the time the sun crosses each stage boundary and each cloud layer's
//! shadow-depression angle, either in the evening (sunset) or in the morning
//! (sunrise). [`build_timeline`] and [`build_timeline_for_place`] are short
//! forms for the evening, which is what most people watch.
//!
//! A sunrise is a sunset played backwards: the same sun depression angles, the
//! same Earth-shadow geometry, just in the opposite order. So the stage
//! boundaries are shared, and only the names (see `morning_name` in packs) and
//! the direction of travel differ.
//!
//! Stages and cloud layers are about the *sky*, so they do not depend on your
//! height or skyline. Only the sunset/sunrise times do: those are about what
//! *you* can see.

use crate::cloud;
use crate::params::Params;
use crate::place::Place;
use crate::sun;

/// Which half of the day to describe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// The evening: sun going down.
    Sunset,
    /// The morning: sun coming up.
    Sunrise,
}

impl Event {
    /// "sunset" or "sunrise".
    pub fn word(self) -> &'static str {
        match self {
            Event::Sunset => "sunset",
            Event::Sunrise => "sunrise",
        }
    }

    /// Read `sunset` or `sunrise` (any letter case).
    pub fn parse(text: &str) -> Result<Event, String> {
        match text.trim().to_ascii_lowercase().as_str() {
            "sunset" | "evening" => Ok(Event::Sunset),
            "sunrise" | "morning" => Ok(Event::Sunrise),
            other => Err(format!("unknown event '{other}', expected sunset or sunrise")),
        }
    }
}

/// When one named stage of the light begins and ends (in time order, so at
/// sunrise `start_jd` is the *lower* sun elevation).
#[derive(Debug, Clone)]
pub struct StageTime {
    pub name: String,
    pub description: String,
    pub from_deg: f64,
    pub to_deg: f64,
    /// Julian Day the stage starts, or `None` if the sun never reaches that level.
    pub start_jd: Option<f64>,
    /// Julian Day the stage ends, or `None`.
    pub end_jd: Option<f64>,
}

/// When Earth's shadow reaches a cloud layer directly overhead.
#[derive(Debug, Clone)]
pub struct LayerReport {
    pub name: String,
    pub height_km: f64,
    /// Sun depression at which the layer is on the edge of Earth's shadow overhead.
    pub shadow_depression_deg: f64,
    /// Evening: when the layer goes dark overhead. Morning: when it first
    /// catches sunlight overhead.
    pub overhead_shadow_jd: Option<f64>,
}

/// Everything computed for one place, day and event.
#[derive(Debug, Clone)]
pub struct Timeline {
    pub event: Event,
    pub lat_deg: f64,
    pub lon_deg: f64,
    pub tz_hours: f64,
    pub date: (i32, u32, u32),
    pub solar_noon_jd: f64,
    /// Height of the viewpoint above sea level, metres.
    pub elevation_m: f64,
    /// How far below level the sea horizon sits from this height, degrees.
    pub dip_deg: f64,
    /// True if the place has a skyline (peaks or horizon points).
    pub has_skyline: bool,
    /// Sunset (or sunrise) for a level horizon at sea level: the classic almanac time.
    pub sea_level_event_jd: Option<f64>,
    /// Same, over a flat horizon seen from your height (later at sunset, earlier at sunrise).
    pub sea_event_jd: Option<f64>,
    /// When the sun actually disappears (or appears) for you: behind the
    /// skyline if there is one, otherwise the same as `sea_event_jd`.
    pub event_jd: Option<f64>,
    pub event_azimuth_deg: Option<f64>,
    pub stages: Vec<StageTime>,
    pub layers: Vec<LayerReport>,
}

/// Compute the evening timeline for a level place at sea level.
pub fn build_timeline(
    params: &Params,
    lat_deg: f64,
    lon_deg: f64,
    date: (i32, u32, u32),
    tz_hours: f64,
) -> Timeline {
    build_timeline_for_place(params, &Place::flat(lat_deg, lon_deg), date, tz_hours)
}

/// Compute the evening timeline for a place (with height and skyline).
pub fn build_timeline_for_place(
    params: &Params,
    place: &Place,
    date: (i32, u32, u32),
    tz_hours: f64,
) -> Timeline {
    build_timeline_event(params, place, date, tz_hours, Event::Sunset)
}

/// Compute the timeline of one event (sunset or sunrise) for a place, a
/// local civil date and a UTC offset.
pub fn build_timeline_event(
    params: &Params,
    place: &Place,
    date: (i32, u32, u32),
    tz_hours: f64,
    event: Event,
) -> Timeline {
    let (lat_deg, lon_deg) = (place.lat, place.lon);
    let noon = sun::solar_noon(date.0, date.1, date.2, lon_deg, tz_hours);
    let cross = |deg: f64| match event {
        Event::Sunset => sun::evening_crossing(noon, lat_deg, lon_deg, deg),
        Event::Sunrise => sun::morning_crossing(noon, lat_deg, lon_deg, deg),
    };

    let dip = place.dip_deg();
    let sea_level_event_jd = cross(params.sunset_elevation_deg);
    let sea_event_jd = cross(params.sunset_elevation_deg - dip);
    let event_jd = if place.has_skyline() {
        match event {
            Event::Sunset => place.evening_crossing_with_skyline(
                noon,
                params.sunset_elevation_deg,
                params.earth_radius_km,
            ),
            Event::Sunrise => place.morning_crossing_with_skyline(
                noon,
                params.sunset_elevation_deg,
                params.earth_radius_km,
            ),
        }
    } else {
        sea_event_jd
    };
    let event_azimuth_deg = event_jd.map(|jd| sun::sun_state(jd, lat_deg, lon_deg).azimuth_deg);

    let stages = params
        .stages
        .iter()
        .map(|s| {
            let (name, description) = match event {
                Event::Sunset => (s.name.clone(), s.description.clone()),
                Event::Sunrise => (
                    if s.morning_name.is_empty() { s.name.clone() } else { s.morning_name.clone() },
                    if s.morning_description.is_empty() {
                        s.description.clone()
                    } else {
                        s.morning_description.clone()
                    },
                ),
            };
            // Stages are written in evening order (high sun to low). At sunrise
            // the same boundaries are crossed the other way round.
            let (start_jd, end_jd) = match event {
                Event::Sunset => (cross(s.from_deg), cross(s.to_deg)),
                Event::Sunrise => (cross(s.to_deg), cross(s.from_deg)),
            };
            StageTime {
                name,
                description,
                from_deg: s.from_deg,
                to_deg: s.to_deg,
                start_jd,
                end_jd,
            }
        })
        .collect();

    let layers = params
        .cloud_layers
        .iter()
        .map(|l| {
            let depression = cloud::shadow_depression_deg(l.height_km, params.earth_radius_km);
            LayerReport {
                name: l.name.clone(),
                height_km: l.height_km,
                shadow_depression_deg: depression,
                overhead_shadow_jd: cross(-depression),
            }
        })
        .collect();

    Timeline {
        event,
        lat_deg,
        lon_deg,
        tz_hours,
        date,
        solar_noon_jd: noon,
        elevation_m: place.elevation_m,
        dip_deg: dip,
        has_skyline: place.has_skyline(),
        sea_level_event_jd,
        sea_event_jd,
        event_jd,
        event_azimuth_deg,
        stages,
        layers,
    }
}
