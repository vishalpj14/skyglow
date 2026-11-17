//! The state of the sky at one instant.
//!
//! [`timeline`](crate::timeline) answers "*when* does each thing happen this
//! evening?". This module answers the other question: "*what* is the sky doing
//! right now?". Give [`sky_state`] a place and a moment (a Julian Day) and it
//! tells you:
//!
//! * where the sun is (elevation and compass direction),
//! * whether it is climbing (morning) or sinking (evening),
//! * whether a mountain skyline hides it, if the place has one,
//! * the lowest height a cloud directly overhead must have to still catch
//!   sunlight (the "lit-height threshold"),
//! * and, for every cloud layer in the pack, whether it is lit overhead, lit
//!   only toward the sun (and where the edge of Earth's shadow sits), or dark.
//!
//! The command line's `--at 18:08` and the web app's time slider both call
//! this one function, so they can never disagree.
//!
//! Nothing here reads files or remembers anything between calls: the same
//! inputs always give the same answer.

use serde::Serialize;

use crate::cloud;
use crate::params::Params;
use crate::place::Place;
use crate::sun::{self, SunState};

/// What the mountain skyline does to the sun at this instant.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct SkylineView {
    /// Height of the skyline in the sun's direction, degrees above level.
    pub horizon_elevation_deg: f64,
    /// True if the terrain is in front of the sun.
    pub sun_hidden: bool,
}

/// How sunlight reaches one cloud layer.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LayerLight {
    /// The layer is in sunlight right overhead. `grazing_km` is the lowest
    /// height the sunbeam dips to on its way there (see
    /// [`cloud::grazing_height_km`]); `colour` is the pack's rough colour
    /// label for that grazing height, if it has one. When the sun is above the
    /// horizon the beam never dips, so `grazing_km` is the layer's own height
    /// and there is no colour label: the grazing-height colour model only
    /// applies once the sun is below the horizon.
    LitOverhead {
        grazing_km: f64,
        colour: Option<String>,
        /// The pack's display colour for that band, `#rrggbb`, if it has one.
        swatch: Option<String>,
    },
    /// Earth's shadow has reached the layer overhead, but the part of the
    /// layer toward the sun is still lit. Looking toward the sun, clouds
    /// below `edge_elevation_deg` are lit and clouds above it are dark. The
    /// edge is `edge_distance_km` away along the ground. Light right at the
    /// edge is deep red-pink.
    LitTowardSun {
        edge_elevation_deg: f64,
        edge_distance_km: f64,
    },
    /// The shadow edge has moved past your horizon: no part of this layer
    /// that you can see is lit.
    EdgeBeyondHorizon,
    /// Overhead the layer is in Earth's shadow. (Rare: it shows up only when
    /// the shadow edge sits exactly overhead.)
    ShadowOverhead,
}

/// One cloud layer at this instant.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LayerState {
    pub name: String,
    /// Height above sea level, km: the pack's height above the ground, added
    /// to the place's ground height, and never above the pack's ceiling.
    pub height_km: f64,
    /// Height above the place's ground, km.
    pub above_ground_km: f64,
    pub light: LayerLight,
}

/// Everything about the sky at one moment, for one place.
#[derive(Debug, Clone, Serialize)]
pub struct SkyState {
    /// The moment, as a Julian Day (UT).
    pub jd: f64,
    /// The full sun calculation, with every intermediate value.
    pub sun: SunState,
    /// Sun elevation above the level horizon, degrees (negative: below).
    pub elevation_deg: f64,
    /// Sun direction, degrees clockwise from north.
    pub azimuth_deg: f64,
    /// How far the sun is below the horizon, degrees; 0 while it is up.
    pub depression_deg: f64,
    /// True before solar noon (sun climbing), false after (sun sinking).
    /// Decided by the hour angle, so it is right at any clock offset.
    pub rising: bool,
    /// Which of the pack's stages (golden hour, afterglow, ...) the sun's
    /// elevation is in, as an index into `params.stages`. `None` when the sun
    /// is above or below every stage (midday, deep night).
    pub stage: Option<usize>,
    /// Present only if the place has a skyline.
    pub skyline: Option<SkylineView>,
    /// While the sun is below the horizon: a cloud directly overhead is lit
    /// only if it is higher than this many km. `None` while the sun is up
    /// (every cloud overhead is lit).
    pub overhead_lit_above_km: Option<f64>,
    /// One entry per cloud layer in the pack, in pack order.
    pub layers: Vec<LayerState>,
}

impl SkyState {
    /// True while the sun's centre is above the level horizon.
    pub fn sun_up(&self) -> bool {
        self.elevation_deg >= 0.0
    }
}

/// Work out the state of the sky at Julian Day `jd` for `place`, using the
/// numbers in `params` (Earth radius, sunset elevation, cloud layers,
/// colour labels).
pub fn sky_state(params: &Params, place: &Place, jd: f64) -> SkyState {
    let s = sun::sun_state(jd, place.lat, place.lon);
    let r = params.earth_radius_km;
    let elevation = s.elevation_deg;
    let depression = if elevation >= 0.0 { 0.0 } else { -elevation };

    let skyline = if place.has_skyline() {
        let threshold =
            place.sun_visible_threshold_deg(s.azimuth_deg, params.sunset_elevation_deg, r);
        Some(SkylineView {
            horizon_elevation_deg: place.horizon_elevation_deg(s.azimuth_deg, r),
            sun_hidden: elevation < threshold,
        })
    } else {
        None
    };

    let overhead_lit_above_km = if elevation < 0.0 {
        Some(cloud::overhead_shadow_height_km(depression, r))
    } else {
        None
    };

    let alt_km = place.elevation_m / 1000.0;
    let layers = params
        .cloud_layers
        .iter()
        .map(|l| {
            // Use the signed depression (negative while the sun is up) so the
            // cloud formulas see the true sun position.
            let h = params.layer_height_km(l, alt_km);
            let mut light = layer_light(params, h, -elevation, r, alt_km);
            if elevation >= 0.0 {
                if let LayerLight::LitOverhead { colour, swatch, .. } = &mut light {
                    *colour = None;
                    *swatch = None;
                }
            }
            LayerState {
                name: l.name.clone(),
                height_km: h,
                above_ground_km: h - alt_km,
                light,
            }
        })
        .collect();

    SkyState {
        jd,
        sun: s,
        elevation_deg: elevation,
        azimuth_deg: s.azimuth_deg,
        depression_deg: depression,
        rising: s.hour_angle_deg < 0.0,
        stage: params.stage_index(elevation),
        skyline,
        overhead_lit_above_km,
        layers,
    }
}

fn layer_light(
    params: &Params,
    height_km: f64,
    depression_deg: f64,
    r: f64,
    observer_alt_km: f64,
) -> LayerLight {
    match cloud::shadow_edge_elevation_from_altitude_deg(height_km, depression_deg, r, observer_alt_km)
    {
        None => match cloud::overhead_grazing_height_km(height_km, depression_deg, r) {
            Some(z) => {
                let band = params.colour_band(z);
                LayerLight::LitOverhead {
                    grazing_km: z,
                    colour: band.map(|b| b.label.clone()),
                    swatch: band.map(|b| b.swatch.clone()).filter(|s| !s.is_empty()),
                }
            }
            None => LayerLight::ShadowOverhead,
        },
        Some(e) if e <= 0.0 => LayerLight::EdgeBeyondHorizon,
        Some(e) => LayerLight::LitTowardSun {
            edge_elevation_deg: e,
            edge_distance_km: cloud::shadow_edge_arc_deg(height_km, depression_deg, r)
                * sun::DEG
                * r,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::{CloudLayer, Params};
    use crate::place::{HorizonPoint, Place};
    use crate::time::julian_day;

    const R: f64 = 6371.0;

    /// The built-in pack with known, round cloud heights so expected values
    /// can be worked out by hand.
    fn pack_with_layers(heights: &[(&str, f64)]) -> Params {
        let mut p = Params::builtin();
        p.earth_radius_km = R;
        p.cloud_layers = heights
            .iter()
            .map(|(n, h)| CloudLayer { name: n.to_string(), height_km: *h })
            .collect();
        p
    }

    /// Search for the instant (on 2026-11-15, Kochi) when the sun is at
    /// `target_deg` in the evening, without using this module.
    fn kochi_evening_at(target_deg: f64) -> f64 {
        let noon = sun::solar_noon(2026, 11, 15, 76.27, 5.5);
        sun::evening_crossing(noon, 9.93, 76.27, target_deg).unwrap()
    }

    #[test]
    fn kochi_midday_sun_is_high_in_the_south() {
        // 15 Nov: declination is about -18.5 deg, so at noon in Kochi (9.93 N)
        // the sun stands about 90 - (9.93 + 18.5) = 61.6 deg up, due south.
        let noon = sun::solar_noon(2026, 11, 15, 76.27, 5.5);
        let st = sky_state(&Params::builtin(), &Place::flat(9.93, 76.27), noon);
        assert!((st.elevation_deg - 61.6).abs() < 0.5, "el = {}", st.elevation_deg);
        assert!((st.azimuth_deg - 180.0).abs() < 1.0, "az = {}", st.azimuth_deg);
        assert!(st.sun_up());
        assert_eq!(st.depression_deg, 0.0);
        assert!(st.overhead_lit_above_km.is_none());
    }

    #[test]
    fn sun_up_means_every_layer_is_lit_at_its_own_height() {
        // With the sun above the horizon the beam never dips, so the grazing
        // height is simply the cloud's height.
        let p = pack_with_layers(&[("low", 2.0), ("high", 10.0)]);
        let noon = sun::solar_noon(2026, 11, 15, 76.27, 5.5);
        let st = sky_state(&p, &Place::flat(9.93, 76.27), noon);
        for (layer, h) in st.layers.iter().zip([2.0, 10.0]) {
            match &layer.light {
                LayerLight::LitOverhead { grazing_km, colour, swatch } => {
                    assert_eq!(*grazing_km, h);
                    assert!(colour.is_none() && swatch.is_none(), "no sunset colour at noon");
                }
                other => panic!("{} should be lit, got {other:?}", layer.name),
            }
        }
    }

    #[test]
    fn evening_is_setting_and_morning_is_rising() {
        let place = Place::flat(9.93, 76.27);
        let p = Params::builtin();
        // 18:30 and 06:00 IST on 15 Nov 2026 (UT = IST - 5.5 h).
        let evening = sky_state(&p, &place, julian_day(2026, 11, 15, 13.0));
        let morning = sky_state(&p, &place, julian_day(2026, 11, 15, 0.5));
        assert!(!evening.rising);
        assert!(morning.rising);
        // Evening sun is in the west-south-west, morning sun in the east-south-east.
        assert!(evening.azimuth_deg > 230.0 && evening.azimuth_deg < 270.0);
        assert!(morning.azimuth_deg > 90.0 && morning.azimuth_deg < 130.0);
    }

    #[test]
    fn rising_flips_at_solar_midnight_not_at_clock_midnight() {
        // Reykjavik, midsummer: the sun barely dips below the northern horizon.
        // Solar midnight is about half a day after solar noon. Ten minutes
        // before it the sun must still be sinking, ten minutes after it must
        // be climbing, whatever the clock says. Checked directly by comparing
        // the elevation one minute later.
        let (lat, lon) = (64.15, -21.94);
        let place = Place::flat(lat, lon);
        let p = Params::builtin();
        let midnight = sun::solar_noon(2026, 6, 20, lon, 0.0) + 0.5;
        let minute = 1.0 / 1440.0;
        let el = |jd: f64| sun::sun_state(jd, lat, lon).elevation_deg;
        for (jd, expect_rising) in [(midnight - 10.0 * minute, false), (midnight + 10.0 * minute, true)] {
            let st = sky_state(&p, &place, jd);
            assert!(!st.sun_up(), "sun should be just below the horizon");
            assert_eq!(el(jd + minute) > el(jd), expect_rising, "physics check");
            assert_eq!(st.rising, expect_rising);
        }
    }

    #[test]
    fn lit_height_threshold_matches_hand_calculation() {
        // Sun 3 deg down: R * (1/cos 3deg - 1) = 6371 * 0.001372 = 8.74 km.
        let jd = kochi_evening_at(-3.0);
        let st = sky_state(&pack_with_layers(&[]), &Place::flat(9.93, 76.27), jd);
        assert!((st.depression_deg - 3.0).abs() < 1e-6);
        let h = st.overhead_lit_above_km.unwrap();
        assert!((h - 8.74).abs() < 0.02, "h = {h}");
    }

    #[test]
    fn layers_split_at_the_shadow_line() {
        // Sun 3 deg down: the overhead shadow line is at 8.74 km (see above).
        // A 13 km cloud is lit overhead; the beam's lowest point is
        // (6371 + 13) cos 3deg - 6371 = 4.25 km, worked by hand.
        // A 2 km cloud goes dark overhead at 1.44 deg, so at 3 deg its shadow
        // edge is (3 - 1.44) deg of arc = 174 km away toward the sun. Earth's
        // curve drops 174^2 / (2 * 6371) = 2.4 km over that distance, more
        // than the cloud's 2 km, so the edge is hidden below your horizon.
        let p = pack_with_layers(&[("low", 2.0), ("high", 13.0)]);
        let st = sky_state(&p, &Place::flat(9.93, 76.27), kochi_evening_at(-3.0));
        match &st.layers[1].light {
            LayerLight::LitOverhead { grazing_km, .. } => {
                assert!((grazing_km - 4.25).abs() < 0.05, "z = {grazing_km}")
            }
            other => panic!("high cloud should be lit overhead, got {other:?}"),
        }
        assert_eq!(st.layers[0].light, LayerLight::EdgeBeyondHorizon);
    }

    #[test]
    fn shadow_edge_on_cirrus_sits_low_toward_the_sun() {
        // Sun 5 deg down, 10 km cirrus: worked by hand (see cloud.rs tests) the
        // edge is about 2 deg up toward the sun and (5 - 3.21) deg = 199 km away.
        let p = pack_with_layers(&[("high", 10.0)]);
        let st = sky_state(&p, &Place::flat(9.93, 76.27), kochi_evening_at(-5.0));
        match &st.layers[0].light {
            LayerLight::LitTowardSun { edge_elevation_deg, edge_distance_km } => {
                assert!(*edge_elevation_deg > 1.8 && *edge_elevation_deg < 2.1);
                assert!((edge_distance_km - 199.0).abs() < 3.0, "d = {edge_distance_km}");
            }
            other => panic!("expected a shadow edge toward the sun, got {other:?}"),
        }
    }

    #[test]
    fn skyline_hides_a_low_sun() {
        // A 10 deg wall all round; with the sun 5 deg up it must be hidden,
        // with it 30 deg up it must be visible.
        let mut place = Place::flat(9.93, 76.27);
        place.horizon = (0..8)
            .map(|i| HorizonPoint { az: i as f64 * 45.0, elev: 10.0 })
            .collect();
        let p = Params::builtin();
        let low = sky_state(&p, &place, kochi_evening_at(5.0));
        let high = sky_state(&p, &place, kochi_evening_at(30.0));
        let low_sky = low.skyline.unwrap();
        assert!((low_sky.horizon_elevation_deg - 10.0).abs() < 0.01);
        assert!(low_sky.sun_hidden);
        assert!(!high.skyline.unwrap().sun_hidden);
        // A place with no skyline reports none.
        assert!(sky_state(&p, &Place::flat(9.93, 76.27), low.jd).skyline.is_none());
    }

    #[test]
    fn stage_follows_sun_elevation() {
        // Default pack: afterglow spans -2 to -4 deg, blue hour -6 to -8.
        let p = Params::builtin();
        let place = Place::flat(9.93, 76.27);
        let name = |deg: f64| {
            sky_state(&p, &place, kochi_evening_at(deg))
                .stage
                .map(|i| p.stages[i].name.clone())
        };
        assert_eq!(name(-3.0).as_deref(), Some("afterglow"));
        assert_eq!(name(-7.0).as_deref(), Some("blue_hour"));
        assert_eq!(name(-30.0), None, "deep night is past every stage");
        assert_eq!(name(40.0), None, "high sun is before every stage");
    }

    #[test]
    fn colour_label_comes_from_the_pack() {
        let p = pack_with_layers(&[("high", 13.0)]);
        let st = sky_state(&p, &Place::flat(9.93, 76.27), kochi_evening_at(-3.0));
        // Grazing height is 4.25 km (worked above); the default pack's bands
        // are 6+ gold, 3+ orange, 1+ orange-red, 0+ deep red. So: orange.
        if let LayerLight::LitOverhead { colour, swatch, .. } = &st.layers[0].light {
            assert_eq!(colour.as_deref(), Some("orange"));
            let orange = p.grazing_colours.iter().find(|c| c.label == "orange").unwrap();
            assert_eq!(swatch.as_deref(), Some(orange.swatch.as_str()));
        } else {
            panic!("high cloud should be lit");
        }
    }
}
