//! Where is the sun, and when does it reach a given height in the sky?
//!
//! This follows the NOAA / Meeus low-precision solar position method. It is
//! accurate to roughly 0.01 degrees (a few seconds of time), far better than
//! anything the cloud and aerosol physics later on can justify.
//!
//! The chain of ideas, in order:
//!
//! 1. Time `T` in Julian centuries since J2000.
//! 2. The sun's **mean longitude** `L0` and **mean anomaly** `M` (where it
//!    would be if Earth's orbit were a perfect circle).
//! 3. The **equation of centre** `C` corrects for the orbit being slightly
//!    elliptical -> **true longitude**, then **apparent longitude**.
//! 4. The **obliquity** (Earth's 23.4 degree axial tilt) turns ecliptic
//!    longitude into **declination**: how far north/south of the equator the
//!    sun sits. Declination is what makes seasons.
//! 5. The **equation of time** is the gap between clock time and sundial time.
//! 6. Latitude + declination + **hour angle** (how far past solar noon) give
//!    the sun's **elevation** and **azimuth** for the observer.

use serde::Serialize;

use crate::time::julian_day;

/// Degrees-to-radians factor. Divide radians by this to get degrees.
pub const DEG: f64 = std::f64::consts::PI / 180.0;

/// Every intermediate value of the calculation (used by `--explain`).
#[derive(Debug, Clone, Copy, Serialize)]
pub struct SunState {
    /// Julian Day (UT) the state was computed for.
    pub jd: f64,
    /// Julian centuries since J2000.
    pub t_centuries: f64,
    /// Mean longitude of the sun, degrees.
    pub mean_longitude_deg: f64,
    /// Mean anomaly, degrees.
    pub mean_anomaly_deg: f64,
    /// Eccentricity of Earth's orbit (dimensionless, about 0.0167).
    pub eccentricity: f64,
    /// Equation of the centre, degrees.
    pub equation_of_center_deg: f64,
    /// True ecliptic longitude, degrees.
    pub true_longitude_deg: f64,
    /// Apparent ecliptic longitude (nutation and aberration corrected), degrees.
    pub apparent_longitude_deg: f64,
    /// Corrected obliquity of the ecliptic (axial tilt), degrees.
    pub obliquity_deg: f64,
    /// Declination: sun's angle north (+) or south (-) of the celestial equator.
    pub declination_deg: f64,
    /// Equation of time in minutes (positive: sundial ahead of clock).
    pub equation_of_time_min: f64,
    /// Hour angle in degrees: 0 at solar noon, +90 about six hours later.
    pub hour_angle_deg: f64,
    /// Geometric elevation above the horizon in degrees (no refraction).
    pub elevation_deg: f64,
    /// Azimuth in degrees clockwise from north (90 = east, 270 = west).
    pub azimuth_deg: f64,
}

fn norm360(x: f64) -> f64 {
    x.rem_euclid(360.0)
}

/// Compute the sun's state for a Julian Day and observer position.
///
/// `lat_deg` is positive north, `lon_deg` is positive east.
pub fn sun_state(jd: f64, lat_deg: f64, lon_deg: f64) -> SunState {
    let t = (jd - 2_451_545.0) / 36_525.0;

    // Orbit geometry.
    let l0 = norm360(280.46646 + t * (36_000.76983 + t * 0.000_303_2));
    let m = norm360(357.52911 + t * (35_999.05029 - 0.000_153_7 * t));
    let e = 0.016_708_634 - t * (0.000_042_037 + 0.000_000_126_7 * t);
    let mr = m * DEG;
    let c = mr.sin() * (1.914_602 - t * (0.004_817 + 0.000_014 * t))
        + (2.0 * mr).sin() * (0.019_993 - 0.000_101 * t)
        + (3.0 * mr).sin() * 0.000_289;
    let true_long = l0 + c;
    let omega = 125.04 - 1_934.136 * t;
    let app_long = true_long - 0.00569 - 0.00478 * (omega * DEG).sin();

    // Axial tilt.
    let eps0 = 23.0
        + (26.0 + (21.448 - t * (46.815 + t * (0.00059 - t * 0.001_813))) / 60.0) / 60.0;
    let eps = eps0 + 0.00256 * (omega * DEG).cos();

    // Declination.
    let decl = ((eps * DEG).sin() * (app_long * DEG).sin()).asin() / DEG;

    // Equation of time (minutes).
    let y = ((eps * DEG) / 2.0).tan().powi(2);
    let l0r = l0 * DEG;
    let eot_rad = y * (2.0 * l0r).sin() - 2.0 * e * mr.sin()
        + 4.0 * e * y * mr.sin() * (2.0 * l0r).cos()
        - 0.5 * y * y * (4.0 * l0r).sin()
        - 1.25 * e * e * (2.0 * mr).sin();
    let eot_min = 4.0 * eot_rad / DEG; // 4 minutes of time per degree

    // Hour angle from true solar time.
    let day_frac = (jd + 0.5) - (jd + 0.5).floor();
    let ut_min = day_frac * 1440.0;
    let true_solar_min = (ut_min + eot_min + 4.0 * lon_deg).rem_euclid(1440.0);
    let hour_angle = true_solar_min / 4.0 - 180.0;

    // Horizontal coordinates.
    let lat = lat_deg * DEG;
    let d = decl * DEG;
    let h = hour_angle * DEG;
    let sin_el = lat.sin() * d.sin() + lat.cos() * d.cos() * h.cos();
    let elevation = sin_el.clamp(-1.0, 1.0).asin() / DEG;
    let azimuth = norm360(h.sin().atan2(h.cos() * lat.sin() - d.tan() * lat.cos()) / DEG + 180.0);

    SunState {
        jd,
        t_centuries: t,
        mean_longitude_deg: l0,
        mean_anomaly_deg: m,
        eccentricity: e,
        equation_of_center_deg: c,
        true_longitude_deg: true_long,
        apparent_longitude_deg: app_long,
        obliquity_deg: eps,
        declination_deg: decl,
        equation_of_time_min: eot_min,
        hour_angle_deg: hour_angle,
        elevation_deg: elevation,
        azimuth_deg: azimuth,
    }
}

/// Julian Day of solar noon (sun due north/south, highest) for a local civil date.
///
/// `tz_hours` is the UTC offset of the clock the date belongs to; it only
/// decides *which* day's noon we mean when the longitude and clock disagree.
pub fn solar_noon(year: i32, month: u32, day: u32, lon_deg: f64, tz_hours: f64) -> f64 {
    let jd0 = julian_day(year, month, day, 0.0); // 0h UT on that date
    let local_noon_guess = jd0 + (12.0 - tz_hours) / 24.0;
    let eot = sun_state(local_noon_guess, 0.0, lon_deg).equation_of_time_min;
    let mut noon = jd0 + (720.0 - 4.0 * lon_deg - eot) / 1440.0;
    // Shift by whole days so we land on the noon closest to local noon.
    noon += (local_noon_guess - noon).round();
    noon
}

/// Julian Day when the sun *descends* through `target_deg` elevation after
/// solar noon, or `None` if it never does that day (polar day / polar night).
///
/// Between solar noon and the following solar midnight the elevation falls
/// steadily, so a simple bisection search is guaranteed to find the crossing.
pub fn evening_crossing(noon_jd: f64, lat_deg: f64, lon_deg: f64, target_deg: f64) -> Option<f64> {
    let el = |jd: f64| sun_state(jd, lat_deg, lon_deg).elevation_deg;
    let (mut a, mut b) = (noon_jd, noon_jd + 0.5);
    if el(a) < target_deg || el(b) > target_deg {
        return None;
    }
    for _ in 0..60 {
        let mid = 0.5 * (a + b);
        if el(mid) > target_deg {
            a = mid;
        } else {
            b = mid;
        }
    }
    Some(0.5 * (a + b))
}

/// The morning counterpart of [`evening_crossing`]: the Julian Day, in the
/// half day *before* `noon_jd`, when the sun climbs through `target_deg`.
/// Returns `None` if it never does (polar night, midnight sun).
pub fn morning_crossing(noon_jd: f64, lat_deg: f64, lon_deg: f64, target_deg: f64) -> Option<f64> {
    let el = |jd: f64| sun_state(jd, lat_deg, lon_deg).elevation_deg;
    let (mut a, mut b) = (noon_jd - 0.5, noon_jd);
    if el(a) > target_deg || el(b) < target_deg {
        return None;
    }
    for _ in 0..60 {
        let mid = 0.5 * (a + b);
        if el(mid) < target_deg {
            a = mid;
        } else {
            b = mid;
        }
    }
    Some(0.5 * (a + b))
}
