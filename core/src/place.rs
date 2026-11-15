//! Where you are standing: coordinates, height above sea level and the skyline.
//!
//! Three things about a place change *when you see* the sun set, even though
//! the sun itself does not care:
//!
//! 1. **Height above sea level.** From higher up you can see farther, so the
//!    sea horizon sits *below* eye level (the "dip"). The sun has to sink
//!    further before it touches that horizon, so you see it set a little later.
//!    Rule of thumb: dip in degrees = 0.0293 x square root of height in metres.
//!
//! 2. **The skyline.** Mountains and ridges in the direction of the sun hide
//!    it long before it reaches the true horizon. You can describe the skyline
//!    in two ways, and use both together:
//!      * `horizon`: a list of `{az, elev}` points ("toward 250 degrees the
//!        skyline is 12 degrees high"), joined by straight lines;
//!      * `peaks`: individual mountains given by bearing, distance and height,
//!        which the code turns into an angle for you.
//!
//! 3. **The clock.** Which time zone the printed times are in.
//!
//! None of this changes the *sky*: Earth's shadow still reaches the clouds at
//! the same moment. High clouds keep glowing after terrain has hidden the
//! sun from you. That is exactly why valley dwellers see colour on clouds
//! while the ground is already in shade.

use serde::Deserialize;

use crate::sun::{self, DEG};

/// Standard refraction coefficient used when a distant mountain is seen
/// through the atmosphere (light bends slightly around Earth's curve).
pub const REFRACTION_COEFFICIENT: f64 = 0.13;

/// One point of a hand-drawn skyline: the skyline is `elev` degrees high
/// toward compass bearing `az`.
#[derive(Debug, Clone, Deserialize)]
pub struct HorizonPoint {
    /// Compass bearing in degrees, 0 = north, 90 = east, 270 = west.
    pub az: f64,
    /// Skyline elevation in degrees above the level horizon.
    pub elev: f64,
}

/// A single mountain, described the way you would read it off a map.
#[derive(Debug, Clone, Deserialize)]
pub struct Peak {
    #[serde(default)]
    pub name: String,
    /// Compass bearing from you to the peak, degrees.
    pub az: f64,
    /// Distance from you to the peak, kilometres.
    pub distance_km: f64,
    /// Height of the peak above sea level, metres.
    pub height_m: f64,
    /// How wide the mountain looks: its skyline bump fades to nothing this
    /// many degrees to either side of `az`. Default 8.
    #[serde(default = "default_half_width")]
    pub half_width_deg: f64,
}

fn default_half_width() -> f64 {
    8.0
}

/// A place on Earth, as the engine needs to know it.
#[derive(Debug, Clone, Deserialize)]
pub struct Place {
    /// Display name, for example "Srinagar".
    #[serde(default)]
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    /// UTC offset (hours) of the local **standard** time, for example 5.5.
    pub tz: Option<f64>,
    /// Name of that local time, for example "India Standard Time (IST)".
    #[serde(default)]
    pub tz_name: String,
    /// UTC offset (hours) while daylight saving time is in effect, if the
    /// place has it.
    pub dst_tz: Option<f64>,
    #[serde(default)]
    pub dst_tz_name: String,
    /// When daylight saving applies, in words.
    #[serde(default)]
    pub dst_note: String,
    /// Height of the viewpoint above sea level, metres.
    #[serde(default)]
    pub elevation_m: f64,
    /// Hand-drawn skyline points, see [`HorizonPoint`].
    #[serde(default)]
    pub horizon: Vec<HorizonPoint>,
    /// Individual mountains, see [`Peak`].
    #[serde(default)]
    pub peaks: Vec<Peak>,
    #[serde(default)]
    pub note: String,
}

/// Atmospheric refraction in degrees for a target at the given apparent
/// elevation (Bennett's formula). About 0.57 degrees at the horizon, falling
/// quickly as you look higher.
pub fn refraction_deg(apparent_elevation_deg: f64) -> f64 {
    let h = apparent_elevation_deg.max(-1.0);
    let arg = (h + 7.31 / (h + 4.4)) * DEG;
    (1.0 / arg.tan()) / 60.0
}

/// Difference between two compass bearings, in -180..180 degrees.
fn angle_diff(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(360.0);
    if d > 180.0 {
        d - 360.0
    } else {
        d
    }
}

impl Place {
    /// A level, sea-level place with no skyline: just coordinates.
    pub fn flat(lat: f64, lon: f64) -> Place {
        Place {
            name: String::new(),
            lat,
            lon,
            tz: None,
            tz_name: String::new(),
            dst_tz: None,
            dst_tz_name: String::new(),
            dst_note: String::new(),
            elevation_m: 0.0,
            horizon: Vec::new(),
            peaks: Vec::new(),
            note: String::new(),
        }
    }

    /// Check that the numbers make sense.
    pub fn validate(&self) -> Result<(), String> {
        let who = if self.name.is_empty() {
            "place".to_string()
        } else {
            format!("place '{}'", self.name)
        };
        if !(-90.0..=90.0).contains(&self.lat) {
            return Err(format!("{who}: latitude must be between -90 and 90"));
        }
        if !(-180.0..=180.0).contains(&self.lon) {
            return Err(format!("{who}: longitude must be between -180 and 180"));
        }
        if !(-500.0..=9000.0).contains(&self.elevation_m) {
            return Err(format!("{who}: elevation_m must be between -500 and 9000"));
        }
        for tz in [self.tz, self.dst_tz].into_iter().flatten() {
            if !(-12.0..=14.0).contains(&tz) {
                return Err(format!("{who}: time zone offsets must be between -12 and 14"));
            }
        }
        for p in &self.horizon {
            if !(0.0..=360.0).contains(&p.az) || !(-10.0..=60.0).contains(&p.elev) {
                return Err(format!(
                    "{who}: horizon points need az 0-360 and elev between -10 and 60"
                ));
            }
        }
        for p in &self.peaks {
            let ok = (0.0..=360.0).contains(&p.az)
                && p.distance_km > 0.0
                && p.distance_km <= 500.0
                && (-500.0..=9000.0).contains(&p.height_m)
                && (0.0..=90.0).contains(&p.half_width_deg);
            if !ok {
                return Err(format!(
                    "{who}: peak '{}' has a value out of range (az 0-360, distance 0-500 km, height -500 to 9000 m, half_width 0-90)",
                    p.name
                ));
            }
        }
        Ok(())
    }

    /// True if the place describes any skyline at all.
    pub fn has_skyline(&self) -> bool {
        !self.horizon.is_empty() || !self.peaks.is_empty()
    }

    /// How far below level the sea horizon sits, because of your height
    /// above sea level (degrees).
    pub fn dip_deg(&self) -> f64 {
        0.0293 * self.elevation_m.max(0.0).sqrt()
    }

    /// Apparent elevation (degrees) of a peak as seen from this place.
    ///
    /// Geometry: the angle `atan(height difference / distance)`, minus how
    /// much the Earth's curve drops the peak (partly offset by refraction).
    pub fn peak_elevation_deg(&self, peak: &Peak, earth_radius_km: f64) -> f64 {
        let d_m = peak.distance_km * 1000.0;
        let dh = peak.height_m - self.elevation_m;
        let r_m = earth_radius_km * 1000.0;
        let geometric = dh.atan2(d_m) / DEG;
        let curvature = (1.0 - REFRACTION_COEFFICIENT) * (d_m / (2.0 * r_m)) / DEG;
        geometric - curvature
    }

    /// Skyline from the hand-drawn `horizon` points, joined by straight lines
    /// and wrapping around north. Zero if none were given.
    fn profile_elevation_deg(&self, az_deg: f64) -> f64 {
        let n = self.horizon.len();
        if n == 0 {
            return 0.0;
        }
        if n == 1 {
            return self.horizon[0].elev;
        }
        let mut pts: Vec<&HorizonPoint> = self.horizon.iter().collect();
        pts.sort_by(|a, b| {
            a.az.rem_euclid(360.0)
                .partial_cmp(&b.az.rem_euclid(360.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let az = az_deg.rem_euclid(360.0);
        for i in 0..n {
            let a = pts[i];
            let b = pts[(i + 1) % n];
            let a_az = a.az.rem_euclid(360.0);
            let mut b_az = b.az.rem_euclid(360.0);
            if i == n - 1 {
                b_az += 360.0; // the last segment wraps around north to the first point
            }
            let mut x = az;
            if x < a_az {
                x += 360.0;
            }
            if x >= a_az && x <= b_az {
                let span = b_az - a_az;
                if span <= 0.0 {
                    return a.elev;
                }
                let t = (x - a_az) / span;
                return a.elev + t * (b.elev - a.elev);
            }
        }
        pts[0].elev
    }

    /// Height of the skyline (degrees above level) toward a compass bearing:
    /// the higher of the hand-drawn profile and any peaks.
    pub fn horizon_elevation_deg(&self, az_deg: f64, earth_radius_km: f64) -> f64 {
        let mut e = self.profile_elevation_deg(az_deg);
        for p in &self.peaks {
            if p.half_width_deg <= 0.0 {
                continue;
            }
            let off = angle_diff(az_deg, p.az).abs();
            if off < p.half_width_deg {
                let bump =
                    self.peak_elevation_deg(p, earth_radius_km) * (1.0 - off / p.half_width_deg);
                if bump > e {
                    e = bump;
                }
            }
        }
        e
    }

    /// The (geometric) sun elevation at which the sun disappears for you, in
    /// the direction `az_deg`.
    ///
    /// `flat_sunset_deg` is the pack's sunset elevation for a level horizon
    /// at sea level (about -0.833). Height above sea level lowers it by the
    /// horizon dip. A skyline above level raises it, after allowing for the
    /// fact that refraction lifts the sun less at higher angles.
    pub fn sun_visible_threshold_deg(
        &self,
        az_deg: f64,
        flat_sunset_deg: f64,
        earth_radius_km: f64,
    ) -> f64 {
        let sea = flat_sunset_deg - self.dip_deg();
        let skyline = self.horizon_elevation_deg(az_deg, earth_radius_km);
        if skyline > 0.0 {
            let terrain = flat_sunset_deg + skyline - (refraction_deg(skyline) - refraction_deg(0.0));
            if terrain > sea {
                return terrain;
            }
        }
        sea
    }

    /// Julian Day when the sun disappears for this place on the evening
    /// after the given solar noon: behind the skyline, or below the horizon
    /// if there is no skyline. `None` if it never does that day.
    ///
    /// Because the skyline can change with bearing, this scans forward in
    /// 30-second steps from solar noon to find the *first* time the sun drops
    /// out of sight, then refines by bisection.
    pub fn evening_crossing_with_skyline(
        &self,
        noon_jd: f64,
        flat_sunset_deg: f64,
        earth_radius_km: f64,
    ) -> Option<f64> {
        let g = |jd: f64| -> f64 {
            let s = sun::sun_state(jd, self.lat, self.lon);
            s.elevation_deg
                - self.sun_visible_threshold_deg(s.azimuth_deg, flat_sunset_deg, earth_radius_km)
        };
        if g(noon_jd) <= 0.0 {
            return None;
        }
        let step = 30.0 / 86_400.0;
        let end = noon_jd + 0.5;
        let mut a = noon_jd;
        let mut t = a + step;
        while a < end {
            if g(t) <= 0.0 {
                let (mut lo, mut hi) = (a, t);
                for _ in 0..40 {
                    let mid = 0.5 * (lo + hi);
                    if g(mid) > 0.0 {
                        lo = mid;
                    } else {
                        hi = mid;
                    }
                }
                return Some(0.5 * (lo + hi));
            }
            a = t;
            t += step;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const R: f64 = 6371.0;

    fn minutes_between(a: f64, b: f64) -> f64 {
        (b - a) * 1440.0
    }

    #[test]
    fn horizon_dip_grows_with_height() {
        let mut p = Place::flat(0.0, 0.0);
        assert_eq!(p.dip_deg(), 0.0);
        p.elevation_m = 2850.0;
        // 0.0293 * sqrt(2850) = 1.56 degrees.
        assert!((p.dip_deg() - 1.56).abs() < 0.01, "dip {}", p.dip_deg());
    }

    #[test]
    fn refraction_is_about_half_a_degree_at_the_horizon() {
        assert!((refraction_deg(0.0) - 0.575).abs() < 0.01);
        assert!(refraction_deg(10.0) < 0.12);
    }

    #[test]
    fn skyline_points_are_joined_by_straight_lines() {
        let mut p = Place::flat(0.0, 0.0);
        p.horizon = vec![
            HorizonPoint { az: 200.0, elev: 0.0 },
            HorizonPoint { az: 280.0, elev: 8.0 },
        ];
        // Halfway between the two points.
        assert!((p.horizon_elevation_deg(240.0, R) - 4.0).abs() < 0.01);
        // Wrapping around north from 280 (8 degrees) back to 200 (0 degrees).
        let north = p.horizon_elevation_deg(100.0, R);
        assert!(north > 2.7 && north < 3.0, "north {north}");
    }

    #[test]
    fn peak_height_and_distance_become_an_angle() {
        let mut p = Place::flat(0.0, 0.0);
        p.elevation_m = 1500.0;
        let peak = Peak {
            name: "test".to_string(),
            az: 270.0,
            distance_km: 50.0,
            height_m: 4000.0,
            half_width_deg: 8.0,
        };
        // atan(2500 / 50000) = 2.86 degrees, minus about 0.2 for Earth's curve.
        let e = p.peak_elevation_deg(&peak, R);
        assert!(e > 2.5 && e < 2.9, "peak elevation {e}");
    }

    #[test]
    fn peak_bump_fades_to_nothing_at_its_edges() {
        let mut p = Place::flat(0.0, 0.0);
        p.peaks = vec![Peak {
            name: "test".to_string(),
            az: 270.0,
            distance_km: 20.0,
            height_m: 3000.0,
            half_width_deg: 10.0,
        }];
        let centre = p.horizon_elevation_deg(270.0, R);
        let mid = p.horizon_elevation_deg(275.0, R);
        let edge = p.horizon_elevation_deg(281.0, R);
        assert!(centre > mid && mid > 0.0);
        assert_eq!(edge, 0.0);
    }

    #[test]
    fn flat_place_threshold_is_exactly_the_pack_value() {
        let p = Place::flat(10.0, 76.0);
        assert_eq!(p.sun_visible_threshold_deg(250.0, -0.833, R), -0.833);
    }

    #[test]
    fn level_place_matches_the_plain_sunset_search() {
        let p = Place::flat(9.93, 76.27);
        let noon = sun::solar_noon(2026, 11, 15, p.lon, 5.5);
        let a = p.evening_crossing_with_skyline(noon, -0.833, R).unwrap();
        let b = sun::evening_crossing(noon, p.lat, p.lon, -0.833).unwrap();
        assert!((a - b).abs() * 86_400.0 < 1.0, "differ by {} s", (a - b).abs() * 86_400.0);
    }

    #[test]
    fn a_ten_degree_skyline_brings_sunset_forward_by_about_45_minutes() {
        // Worked by hand at 10 degrees north in mid-November: the sun reaches
        // 9.65 degrees (hour angle 76.2) about 45 minutes before it reaches -0.83 (hour angle 87.5).
        let flat = Place::flat(9.93, 76.27);
        let mut walled = Place::flat(9.93, 76.27);
        walled.horizon = vec![HorizonPoint { az: 0.0, elev: 10.0 }];
        let noon = sun::solar_noon(2026, 11, 15, flat.lon, 5.5);
        let a = flat.evening_crossing_with_skyline(noon, -0.833, R).unwrap();
        let b = walled.evening_crossing_with_skyline(noon, -0.833, R).unwrap();
        let m = minutes_between(b, a);
        assert!(m > 35.0 && m < 55.0, "skyline moved sunset by {m} min");
    }

    #[test]
    fn height_above_sea_level_delays_the_visible_sunset() {
        let sea = Place::flat(9.93, 76.27);
        let mut high = Place::flat(9.93, 76.27);
        high.elevation_m = 2000.0; // dip 1.31 degrees, roughly 5-6 minutes near the tropics
        let noon = sun::solar_noon(2026, 11, 15, sea.lon, 5.5);
        let a = sea.evening_crossing_with_skyline(noon, -0.833, R).unwrap();
        let b = high.evening_crossing_with_skyline(noon, -0.833, R).unwrap();
        let m = minutes_between(a, b);
        assert!(m > 4.0 && m < 8.0, "altitude delayed sunset by {m} min");
    }

    #[test]
    fn validation_catches_nonsense() {
        let mut p = Place::flat(95.0, 0.0);
        assert!(p.validate().is_err());
        p = Place::flat(10.0, 76.0);
        p.elevation_m = 20_000.0;
        assert!(p.validate().is_err());
        p = Place::flat(10.0, 76.0);
        p.peaks = vec![Peak {
            name: "bad".to_string(),
            az: 270.0,
            distance_km: -5.0,
            height_m: 1000.0,
            half_width_deg: 8.0,
        }];
        assert!(p.validate().is_err());
    }
}
