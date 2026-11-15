//! Earth's shadow and cloud layers.
//!
//! After the sun sets *for you*, it is still shining on the atmosphere
//! higher up and further toward the sun. A cloud stays lit until Earth's
//! shadow reaches it. Because Earth is a sphere, higher clouds keep the light
//! longer, which is why high cirrus and altocumulus turn pink *after* the
//! low clouds have gone grey.
//!
//! Geometry: a point at height `h` above sea level sees the sun until the sun
//! is `acos(R / (R + h))` degrees below the *horizontal at that point*. For a
//! cloud directly overhead of you, that is also your own sun depression.
//! Looking toward the sun, clouds sit at a different point on the globe, where
//! the sun is higher by the angle between you and them, so they stay lit
//! longer.
//!
//! ```text
//!   depression of sun below YOUR horizon:  e (degrees)
//!   cloud height h, central angle between you and the cloud: g (degrees)
//!   cloud is lit  <=>  e < acos(R/(R+h)) + g
//! ```
//!
//! All angles are geometric: atmospheric refraction (about half a degree at
//! the horizon) is ignored here and left to the calibration packs.

use crate::sun::DEG;

/// Sun depression (degrees below your horizon) at which Earth's shadow reaches
/// a cloud at `height_km` **directly overhead**.
///
/// Example values for R = 6371 km: 2 km -> 1.4 degrees, 6 km -> 2.5 degrees,
/// 10 km -> 3.2 degrees.
pub fn shadow_depression_deg(height_km: f64, earth_radius_km: f64) -> f64 {
    (earth_radius_km / (earth_radius_km + height_km)).acos() / DEG
}

/// The height (km) at which a cloud **directly overhead** is exactly on the
/// edge of Earth's shadow, for a given sun depression. Overhead clouds higher
/// than this are still lit; lower ones are in shadow. This is the inverse of
/// [`shadow_depression_deg`].
pub fn overhead_shadow_height_km(depression_deg: f64, earth_radius_km: f64) -> f64 {
    earth_radius_km * (1.0 / (depression_deg * DEG).cos() - 1.0)
}

/// Lowest altitude (km above the ground) a sunbeam reaches on its way to a
/// cloud. This is the key to **colour**: a beam that dives close to the ground
/// crosses hundreds of kilometres of dense air and arrives deep red or pink
/// (and dim); a beam that stays high crosses thin air and arrives gold or
/// white.
///
/// `sun_elevation_at_cloud_deg` is the sun's elevation measured at the
/// *cloud's own position* (negative = below the cloud's horizon). The beam's
/// closest approach to Earth's centre is `(R + h) * cos(elevation)`.
///
/// * `Some(z)` - the beam skims `z` km above the ground (never more than the
///   cloud's own height).
/// * `None` - the cloud is inside Earth's shadow, no direct light.
pub fn grazing_height_km(
    cloud_height_km: f64,
    sun_elevation_at_cloud_deg: f64,
    earth_radius_km: f64,
) -> Option<f64> {
    if sun_elevation_at_cloud_deg >= 0.0 {
        return Some(cloud_height_km);
    }
    let z = (earth_radius_km + cloud_height_km) * (sun_elevation_at_cloud_deg * DEG).cos()
        - earth_radius_km;
    if z < 0.0 {
        None
    } else {
        Some(z)
    }
}

/// [`grazing_height_km`] for a cloud directly overhead, where the sun's
/// elevation at the cloud equals minus your own sun depression.
pub fn overhead_grazing_height_km(
    cloud_height_km: f64,
    depression_deg: f64,
    earth_radius_km: f64,
) -> Option<f64> {
    grazing_height_km(cloud_height_km, -depression_deg, earth_radius_km)
}

/// Angle along the ground (as seen from Earth's centre) from you to the point
/// where a cloud layer at `height_km` stops being lit, for a given sun
/// depression. Positive: the shadow edge lies *toward the sun* and clouds
/// nearer than this are in shadow. Negative: the shadow edge is behind you
/// and the layer is lit overhead.
pub fn shadow_edge_arc_deg(height_km: f64, depression_deg: f64, earth_radius_km: f64) -> f64 {
    depression_deg - shadow_depression_deg(height_km, earth_radius_km)
}

/// Elevation angle in the sky (toward the sun's azimuth) of the shadow edge on
/// a cloud layer.
///
/// * `None`  - the layer is still lit overhead (the edge is behind you).
/// * `Some(e)` with `e > 0` - clouds **below** that angle toward the horizon
///   are lit; clouds above it are in shadow.
/// * `Some(e)` with `e <= 0` - the edge is beyond your horizon, so none of
///   this layer is visible and lit.
pub fn shadow_edge_elevation_deg(
    height_km: f64,
    depression_deg: f64,
    earth_radius_km: f64,
) -> Option<f64> {
    let gamma = shadow_edge_arc_deg(height_km, depression_deg, earth_radius_km);
    if gamma <= 0.0 {
        return None;
    }
    let g = gamma * DEG;
    let up = (earth_radius_km + height_km) * g.cos() - earth_radius_km;
    let along = (earth_radius_km + height_km) * g.sin();
    Some(up.atan2(along) / DEG)
}

/// Like [`shadow_edge_elevation_deg`], but for an observer standing
/// `observer_altitude_km` above sea level (cloud heights are above sea level
/// too). From a mountain the shadow edge looks slightly lower in the sky,
/// because you are closer to the cloud layer.
pub fn shadow_edge_elevation_from_altitude_deg(
    height_km: f64,
    depression_deg: f64,
    earth_radius_km: f64,
    observer_altitude_km: f64,
) -> Option<f64> {
    let gamma = shadow_edge_arc_deg(height_km, depression_deg, earth_radius_km);
    if gamma <= 0.0 {
        return None;
    }
    let g = gamma * DEG;
    let up = (earth_radius_km + height_km) * g.cos() - (earth_radius_km + observer_altitude_km);
    let along = (earth_radius_km + height_km) * g.sin();
    Some(up.atan2(along) / DEG)
}

/// Sun depression until which a cloud at `height_km`, `ground_distance_km`
/// away from you toward the sun, remains lit.
pub fn lit_until_depression_deg(
    height_km: f64,
    ground_distance_km: f64,
    earth_radius_km: f64,
) -> f64 {
    shadow_depression_deg(height_km, earth_radius_km) + (ground_distance_km / earth_radius_km) / DEG
}

#[cfg(test)]
mod tests {
    use super::*;

    const R: f64 = 6371.0;

    #[test]
    fn overhead_shadow_depressions() {
        assert!((shadow_depression_deg(2.0, R) - 1.435).abs() < 0.05);
        assert!((shadow_depression_deg(6.0, R) - 2.48).abs() < 0.05);
        assert!((shadow_depression_deg(10.0, R) - 3.21).abs() < 0.05);
    }

    #[test]
    fn overhead_height_is_inverse_of_depression() {
        let d = shadow_depression_deg(10.0, R);
        assert!((overhead_shadow_height_km(d, R) - 10.0).abs() < 0.01);
        assert!((overhead_shadow_height_km(3.0, R) - 8.75).abs() < 0.2);
    }

    #[test]
    fn grazing_height_is_zero_at_the_shadow_edge() {
        let edge = shadow_depression_deg(13.0, R);
        let z = overhead_grazing_height_km(13.0, edge - 1e-6, R).unwrap();
        assert!(z >= 0.0 && z < 0.05, "z = {z}");
        // A hair past the edge the cloud is in shadow.
        assert!(overhead_grazing_height_km(13.0, edge + 0.01, R).is_none());
    }

    #[test]
    fn grazing_height_for_a_tropical_high_cloud() {
        // 13 km cloud overhead with the sun 2.71 degrees down: about 5.9 km, worked by hand.
        let z = overhead_grazing_height_km(13.0, 2.71, R).unwrap();
        assert!(z > 5.5 && z < 6.3, "z = {z}");
        // Deeper sun, lower grazing height.
        let z2 = overhead_grazing_height_km(13.0, 3.42, R).unwrap();
        assert!(z2 < z && z2 > 1.2 && z2 < 2.0, "z2 = {z2}");
    }

    #[test]
    fn sun_above_cloud_horizon_means_no_grazing() {
        assert_eq!(grazing_height_km(10.0, 5.0, R), Some(10.0));
    }

    #[test]
    fn higher_clouds_stay_lit_longer() {
        assert!(shadow_depression_deg(12.0, R) > shadow_depression_deg(6.0, R));
        assert!(shadow_depression_deg(6.0, R) > shadow_depression_deg(1.5, R));
    }

    #[test]
    fn overhead_cirrus_still_lit_at_three_degrees() {
        // 10 km cirrus goes dark overhead at about 3.2 degrees of depression.
        assert!(shadow_edge_elevation_deg(10.0, 3.0, R).is_none());
    }

    #[test]
    fn shadow_edge_drops_toward_horizon_as_sun_sinks() {
        let e5 = shadow_edge_elevation_deg(10.0, 5.0, R).unwrap();
        let e7 = shadow_edge_elevation_deg(10.0, 7.0, R).unwrap();
        assert!(e5 > e7, "edge should sink: {e5} then {e7}");
        // Worked by hand: about 2 degrees above the horizon at 5 degrees of depression.
        assert!(e5 > 1.8 && e5 < 2.1, "got {e5}");
    }

    #[test]
    fn distance_extends_lit_time() {
        let overhead = lit_until_depression_deg(10.0, 0.0, R);
        let far = lit_until_depression_deg(10.0, 200.0, R);
        assert!(far > overhead);
        assert!((far - overhead - 1.8).abs() < 0.1); // 200 km is about 1.8 degrees of arc
    }

    #[test]
    fn altitude_zero_matches_sea_level_edge() {
        let a = shadow_edge_elevation_deg(10.0, 8.0, 6371.0).unwrap();
        let b = shadow_edge_elevation_from_altitude_deg(10.0, 8.0, 6371.0, 0.0).unwrap();
        assert!((a - b).abs() < 1e-9);
    }

    #[test]
    fn standing_higher_lowers_the_shadow_edge() {
        let sea = shadow_edge_elevation_from_altitude_deg(10.0, 8.0, 6371.0, 0.0).unwrap();
        let high = shadow_edge_elevation_from_altitude_deg(10.0, 8.0, 6371.0, 3.0).unwrap();
        assert!(high < sea);
    }
}
