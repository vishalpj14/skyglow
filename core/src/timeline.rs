//! One evening, start to finish.
//!
//! [`build_timeline_for_place`] glues the pieces together: it finds solar noon,
//! then the time the sun crosses each stage boundary and each cloud layer's
//! shadow-depression angle. [`build_timeline`] is the short form for a level
//! place at sea level.
//!
//! Stages and cloud layers are about the *sky*, so they do not depend on your
//! height or skyline. Only the sunset times do: those are about what *you* can
//! see.

use crate::cloud;
use crate::params::Params;
use crate::place::Place;
use crate::sun;

/// When one named stage of the evening begins and ends.
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
    /// Sun depression at which the layer goes dark overhead.
    pub shadow_depression_deg: f64,
    pub overhead_dark_jd: Option<f64>,
}

/// Everything computed for one place and day.
#[derive(Debug, Clone)]
pub struct Timeline {
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
    /// Sunset for a level horizon at sea level (the classic almanac time).
    pub sea_level_sunset_jd: Option<f64>,
    /// Sunset over a flat horizon seen from your height (later than sea level).
    pub sea_sunset_jd: Option<f64>,
    /// When the sun actually disappears for you: behind the skyline if there
    /// is one, otherwise the same as `sea_sunset_jd`.
    pub sunset_jd: Option<f64>,
    pub sunset_azimuth_deg: Option<f64>,
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

/// Compute the evening timeline for a place (with height and skyline), a
/// local civil date and a UTC offset.
pub fn build_timeline_for_place(
    params: &Params,
    place: &Place,
    date: (i32, u32, u32),
    tz_hours: f64,
) -> Timeline {
    let (lat_deg, lon_deg) = (place.lat, place.lon);
    let noon = sun::solar_noon(date.0, date.1, date.2, lon_deg, tz_hours);
    let cross = |deg: f64| sun::evening_crossing(noon, lat_deg, lon_deg, deg);

    let dip = place.dip_deg();
    let sea_level_sunset_jd = cross(params.sunset_elevation_deg);
    let sea_sunset_jd = cross(params.sunset_elevation_deg - dip);
    let sunset_jd = if place.has_skyline() {
        place.evening_crossing_with_skyline(noon, params.sunset_elevation_deg, params.earth_radius_km)
    } else {
        sea_sunset_jd
    };
    let sunset_azimuth_deg = sunset_jd.map(|jd| sun::sun_state(jd, lat_deg, lon_deg).azimuth_deg);

    let stages = params
        .stages
        .iter()
        .map(|s| StageTime {
            name: s.name.clone(),
            description: s.description.clone(),
            from_deg: s.from_deg,
            to_deg: s.to_deg,
            start_jd: cross(s.from_deg),
            end_jd: cross(s.to_deg),
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
                overhead_dark_jd: cross(-depression),
            }
        })
        .collect();

    Timeline {
        lat_deg,
        lon_deg,
        tz_hours,
        date,
        solar_noon_jd: noon,
        elevation_m: place.elevation_m,
        dip_deg: dip,
        has_skyline: place.has_skyline(),
        sea_level_sunset_jd,
        sea_sunset_jd,
        sunset_jd,
        sunset_azimuth_deg,
        stages,
        layers,
    }
}
