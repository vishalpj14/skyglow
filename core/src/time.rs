//! Minimal calendar and clock helpers (no external dependencies).
//!
//! Everything in the engine is expressed as a **Julian Day** (JD): the number
//! of days since noon on 1 January 4713 BC. It is a single continuous number,
//! which makes "time between two events" a plain subtraction.
//! `JD 2451545.0` is 2000-01-01 12:00 UT (the astronomical epoch "J2000").

use std::time::{SystemTime, UNIX_EPOCH};

/// Julian Day (UT) for a Gregorian calendar date plus a fractional hour of day.
///
/// Algorithm: Meeus, *Astronomical Algorithms*, ch. 7. Valid for dates after
/// the Gregorian reform (1582), which is all we need.
pub fn julian_day(year: i32, month: u32, day: u32, hour: f64) -> f64 {
    let mut y = year as f64;
    let mut m = month as f64;
    if month <= 2 {
        y -= 1.0;
        m += 12.0;
    }
    let a = (y / 100.0).floor();
    let b = 2.0 - a + (a / 4.0).floor();
    (365.25 * (y + 4716.0)).floor() + (30.6001 * (m + 1.0)).floor() + day as f64 + hour / 24.0 + b
        - 1524.5
}

/// Number of days in a Gregorian month.
pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
            if leap {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

/// Parse `YYYY-MM-DD` into `(year, month, day)`.
pub fn parse_date(s: &str) -> Result<(i32, u32, u32), String> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return Err(format!("expected a date like 2026-11-15, got '{s}'"));
    }
    let year: i32 = parts[0]
        .parse()
        .map_err(|_| format!("bad year in '{s}'"))?;
    let month: u32 = parts[1]
        .parse()
        .map_err(|_| format!("bad month in '{s}'"))?;
    let day: u32 = parts[2]
        .parse()
        .map_err(|_| format!("bad day in '{s}'"))?;
    if !(1..=12).contains(&month) {
        return Err(format!("month must be 1-12 in '{s}'"));
    }
    if day < 1 || day > days_in_month(year, month) {
        return Err(format!("day out of range for that month in '{s}'"));
    }
    Ok((year, month, day))
}

/// Convert "days since 1970-01-01" to a civil date (Howard Hinnant's algorithm).
pub fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    (year as i32, m as u32, d as u32)
}

/// Today's date at a given UTC offset (in hours), from the system clock.
pub fn today_local(tz_hours: f64) -> (i32, u32, u32) {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let shifted = secs + (tz_hours * 3600.0).round() as i64;
    civil_from_days(shifted.div_euclid(86_400))
}

/// Format a Julian Day as local clock time `HH:MM:SS` at a UTC offset.
pub fn format_clock(jd: f64, tz_hours: f64) -> String {
    // JD starts at noon, so add 0.5 to make the fractional part "days since midnight".
    let local = jd + tz_hours / 24.0 + 0.5;
    let frac = local - local.floor();
    let total = (frac * 86_400.0).round() as i64 % 86_400;
    format!("{:02}:{:02}:{:02}", total / 3600, (total % 3600) / 60, total % 60)
}

/// Parse a local clock time such as `18:08` or `18:08:30` into hours (18.1333).
pub fn parse_clock(s: &str) -> Result<f64, String> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() < 2 || parts.len() > 3 {
        return Err(format!("expected a time like 18:08 or 18:08:30, got '{s}'"));
    }
    let h: f64 = parts[0].parse().map_err(|_| format!("bad hour in '{s}'"))?;
    let m: f64 = parts[1].parse().map_err(|_| format!("bad minute in '{s}'"))?;
    let sec: f64 = if parts.len() == 3 {
        parts[2].parse().map_err(|_| format!("bad second in '{s}'"))?
    } else {
        0.0
    };
    if !(0.0..24.0).contains(&h) || !(0.0..60.0).contains(&m) || !(0.0..60.0).contains(&sec) {
        return Err(format!("time out of range in '{s}'"));
    }
    Ok(h + m / 60.0 + sec / 3600.0)
}

/// Julian Day for a local civil date and clock time (in hours) at a UTC offset.
pub fn jd_from_local(date: (i32, u32, u32), hours: f64, tz_hours: f64) -> f64 {
    julian_day(date.0, date.1, date.2, 0.0) + (hours - tz_hours) / 24.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn j2000_epoch() {
        assert!((julian_day(2000, 1, 1, 12.0) - 2_451_545.0).abs() < 1e-9);
    }

    #[test]
    fn known_julian_day_2026() {
        // 2026-11-15 00:00 UT, counted by hand: 9815 days after 2000-01-01 00:00 (JD 2451544.5).
        assert!((julian_day(2026, 11, 15, 0.0) - 2_461_359.5).abs() < 1e-9);
    }

    #[test]
    fn parses_good_and_bad_dates() {
        assert_eq!(parse_date("2026-11-15"), Ok((2026, 11, 15)));
        assert!(parse_date("2026-13-01").is_err());
        assert!(parse_date("2026-02-30").is_err());
        assert!(parse_date("hello").is_err());
    }

    #[test]
    fn leap_years() {
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2100, 2), 28);
        assert_eq!(days_in_month(2000, 2), 29);
    }

    #[test]
    fn civil_from_days_known_values() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(20_000), (2024, 10, 4));
    }

    #[test]
    fn clock_formatting_applies_offset() {
        let jd = julian_day(2026, 11, 15, 12.0 + 30.0 / 60.0 + 15.0 / 3600.0);
        assert_eq!(format_clock(jd, 5.5), "18:00:15");
        assert_eq!(format_clock(jd, 0.0), "12:30:15");
    }

    #[test]
    fn parses_clock_times() {
        assert!((parse_clock("18:08").unwrap() - (18.0 + 8.0 / 60.0)).abs() < 1e-12);
        assert!((parse_clock("18:08:30").unwrap() - (18.0 + 8.0 / 60.0 + 30.0 / 3600.0)).abs() < 1e-12);
        assert!(parse_clock("25:00").is_err());
        assert!(parse_clock("noon").is_err());
    }

    #[test]
    fn local_time_to_julian_day() {
        // 18:00 IST (UTC+5.5) is 12:30 UT.
        let a = jd_from_local((2026, 11, 15), 18.0, 5.5);
        let b = julian_day(2026, 11, 15, 12.5);
        assert!((a - b).abs() < 1e-9);
    }
}
