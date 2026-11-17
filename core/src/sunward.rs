//! Points along the sun's direction, where the forecast should be read.
//!
//! The sunbeam that lights a cloud above you after sunset comes in low from
//! the sun's side, often crossing other cloud levels 100 to 600 km away. The
//! cloud cover *there* decides whether the light gets through, not the cover
//! above you. So skyglow asks the forecast about a few points along the
//! ground toward the sun at sunset (and at sunrise), as well as about the
//! place itself.
//!
//! This module only does the geometry: it never goes online. [`plan`] lists
//! the points (the place first, then the sunset points, then the sunrise
//! points, nearest first); the web app or the user fetches them, and
//! `openmeteo::weather_from_responses` reads the answers back in that order.

use serde::{Deserialize, Serialize};

use crate::params::Params;
use crate::place::Place;
use crate::sun::DEG;
use crate::timeline::{build_timeline_event, Event};

/// A point on the ground.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
pub struct GeoPoint {
    pub lat: f64,
    pub lon: f64,
}

/// Where one direction's points sit in [`WeatherPlan::points`].
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Direction {
    /// Compass direction of the sun at the event, degrees from north.
    pub azimuth_deg: f64,
    /// Index of this direction's first point in `points`.
    pub first: usize,
    /// Distance of each point from the place, km, nearest first.
    pub distances_km: Vec<f64>,
}

/// The places to ask the forecast about, in order.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct WeatherPlan {
    /// The place itself first, then the sunset points, then the sunrise points.
    pub points: Vec<GeoPoint>,
    /// `None` when there is no sunset that day (polar day or night) or the
    /// pack asks for no points along the way.
    pub sunset: Option<Direction>,
    pub sunrise: Option<Direction>,
}

/// The point `distance_km` from (`lat`, `lon`) along the ground, starting
/// toward compass direction `azimuth_deg`, on a sphere of radius `r` (the
/// standard great-circle "destination point" formula).
pub fn destination(lat: f64, lon: f64, azimuth_deg: f64, distance_km: f64, r: f64) -> GeoPoint {
    let (phi1, lambda1, theta) = (lat * DEG, lon * DEG, azimuth_deg * DEG);
    let delta = distance_km / r;
    let phi2 = (phi1.sin() * delta.cos() + phi1.cos() * delta.sin() * theta.cos()).asin();
    let lambda2 = lambda1
        + (theta.sin() * delta.sin() * phi1.cos()).atan2(delta.cos() - phi1.sin() * phi2.sin());
    // Keep longitude in -180..180.
    let lon2 = (lambda2 / DEG + 540.0).rem_euclid(360.0) - 180.0;
    GeoPoint { lat: phi2 / DEG, lon: lon2 }
}

/// The places to ask the forecast about for `place` on a local date: the
/// place, then the pack's `glow.sunward_km` distances toward the sun at
/// sunset and at sunrise.
pub fn plan(params: &Params, place: &Place, date: (i32, u32, u32), tz_hours: f64) -> WeatherPlan {
    let mut points = vec![GeoPoint { lat: place.lat, lon: place.lon }];
    let distances = &params.glow.sunward_km;
    let mut direction = |event: Event| -> Option<Direction> {
        if distances.is_empty() {
            return None;
        }
        let t = build_timeline_event(params, place, date, tz_hours, event);
        // The sea-horizon azimuth: the direction the sunbeam comes from,
        // whatever the skyline does.
        let az = t.event_azimuth_deg?;
        let first = points.len();
        points.extend(distances.iter().map(|d| destination(place.lat, place.lon, az, *d, params.earth_radius_km)));
        Some(Direction { azimuth_deg: az, first, distances_km: distances.clone() })
    };
    let sunset = direction(Event::Sunset);
    let sunrise = direction(Event::Sunrise);
    WeatherPlan { points, sunset, sunrise }
}

#[cfg(test)]
mod tests {
    use super::*;

    const R: f64 = 6371.0;

    /// Great-circle distance by the haversine formula, worked independently
    /// of `destination`.
    fn haversine_km(a: GeoPoint, b: GeoPoint) -> f64 {
        let (p1, p2) = (a.lat * DEG, b.lat * DEG);
        let dp = p2 - p1;
        let dl = (b.lon - a.lon) * DEG;
        let h = (dp / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
        2.0 * R * h.sqrt().asin()
    }

    /// Initial compass bearing from a to b, worked independently.
    fn bearing_deg(a: GeoPoint, b: GeoPoint) -> f64 {
        let (p1, p2) = (a.lat * DEG, b.lat * DEG);
        let dl = (b.lon - a.lon) * DEG;
        let y = dl.sin() * p2.cos();
        let x = p1.cos() * p2.sin() - p1.sin() * p2.cos() * dl.cos();
        (y.atan2(x) / DEG).rem_euclid(360.0)
    }

    #[test]
    fn one_degree_of_arc_is_111_km() {
        // 2 x pi x 6371 / 360 = 111.195 km per degree along a great circle.
        let east = destination(0.0, 0.0, 90.0, 111.195, R);
        assert!(east.lat.abs() < 1e-9 && (east.lon - 1.0).abs() < 1e-4, "{east:?}");
        let north = destination(0.0, 0.0, 0.0, 111.195, R);
        assert!((north.lat - 1.0).abs() < 1e-4 && north.lon.abs() < 1e-9, "{north:?}");
        // Westward across the date line wraps to the eastern side.
        let west = destination(0.0, -179.5, 270.0, 111.195, R);
        assert!((west.lon - 179.5).abs() < 1e-4, "{west:?}");
    }

    #[test]
    fn destination_agrees_with_haversine_and_the_initial_bearing() {
        let start = GeoPoint { lat: 9.93, lon: 76.27 };
        for (az, d) in [(252.0, 300.0), (110.0, 600.0), (10.0, 50.0)] {
            let p = destination(start.lat, start.lon, az, d, R);
            assert!((haversine_km(start, p) - d).abs() < 1e-6, "distance for {az} deg");
            assert!((bearing_deg(start, p) - az).abs() < 1e-6, "bearing for {az} deg");
        }
    }

    #[test]
    fn the_plan_lists_the_place_then_sunset_then_sunrise_points() {
        // Kochi, 15 Nov: the sun sets in the west-south-west (about 252 deg)
        // and rises in the east-south-east (about 108 deg).
        let mut p = Params::builtin();
        p.glow.sunward_km = vec![100.0, 300.0];
        let place = Place::flat(9.93, 76.27);
        let plan = plan(&p, &place, (2026, 11, 15), 5.5);
        assert_eq!(plan.points.len(), 5);
        assert_eq!(plan.points[0], GeoPoint { lat: 9.93, lon: 76.27 });
        let set = plan.sunset.as_ref().unwrap();
        let rise = plan.sunrise.as_ref().unwrap();
        assert!(set.azimuth_deg > 245.0 && set.azimuth_deg < 260.0, "sunset az {}", set.azimuth_deg);
        assert!(rise.azimuth_deg > 100.0 && rise.azimuth_deg < 115.0, "sunrise az {}", rise.azimuth_deg);
        assert_eq!((set.first, rise.first), (1, 3));
        for (dir, i, d) in [(set, 1, 100.0), (set, 2, 300.0), (rise, 3, 100.0), (rise, 4, 300.0)] {
            assert!((haversine_km(plan.points[0], plan.points[i]) - d).abs() < 1e-6);
            assert!((bearing_deg(plan.points[0], plan.points[i]) - dir.azimuth_deg).abs() < 1e-6);
        }
        // Sunset points are west of Kochi, sunrise points east.
        assert!(plan.points[2].lon < 76.27 && plan.points[4].lon > 76.27);

        // No distances in the pack: the place only.
        p.glow.sunward_km.clear();
        let only = super::plan(&p, &place, (2026, 11, 15), 5.5);
        assert_eq!(only.points.len(), 1);
        assert!(only.sunset.is_none() && only.sunrise.is_none());
    }

    #[test]
    fn no_sunset_means_no_sunset_points() {
        // Tromso in December: polar night, no sunset or sunrise.
        let p = Params::builtin();
        let plan = plan(&p, &Place::flat(69.65, 18.96), (2026, 12, 21), 1.0);
        assert!(plan.sunset.is_none() && plan.sunrise.is_none());
        assert_eq!(plan.points.len(), 1);
    }
}
