//! The tunable numbers, loaded from JSON "packs".
//!
//! Physics that is exact (sun position) lives in code. Numbers that are
//! *estimates* (where the pink window starts, how high cirrus sits in the
//! tropics) live in a pack so they can be corrected without a new release.
//! See `packs/README.md` for the file format.
//!
//! The built-in default pack is compiled into the binary from
//! `packs/default.json`, so the engine always works offline.

use serde::{Deserialize, Serialize};

use crate::glow::GlowParams;

/// Pack schema version this build understands. Packs with another number are
/// rejected instead of being half-understood.
pub const SUPPORTED_SCHEMA: u32 = 1;

/// The default pack, embedded at compile time.
pub const DEFAULT_PACK_JSON: &str = include_str!("../../packs/default.json");

/// A cloud layer used for shadow calculations.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CloudLayer {
    /// Short label such as `low`, `mid`, `high`.
    pub name: String,
    /// Typical height above the ground in kilometres. Weather forecasts
    /// count low, middle and high cloud from the ground up, so at a high
    /// place every layer sits higher above sea level (see
    /// [`Params::layer_height_km`]).
    pub height_km: f64,
}

/// One named phase of the evening, defined by sun elevation.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Stage {
    pub name: String,
    /// Sun elevation (degrees) where the stage begins (higher number).
    pub from_deg: f64,
    /// Sun elevation (degrees) where the stage ends (lower number).
    pub to_deg: f64,
    /// Plain-language description of what the sky tends to do.
    #[serde(default)]
    pub description: String,
    /// Name to show at sunrise, when the stages run in reverse. Falls back to `name`.
    #[serde(default)]
    pub morning_name: String,
    /// Description to show at sunrise. Falls back to `description`.
    #[serde(default)]
    pub morning_description: String,
    /// Optional display colour for this stage's sky, as `#rrggbb`. Only for
    /// drawing (the web colour strip); an uncalibrated guess, not physics.
    #[serde(default)]
    pub swatch: String,
}

/// A rough colour label for light that dives down to a given altitude.
///
/// The label applies when the beam's lowest altitude is at least
/// `min_height_km`; the entry with the highest matching threshold wins.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GrazingColour {
    pub min_height_km: f64,
    pub label: String,
    /// Optional display colour for this band, as `#rrggbb`. Uncalibrated.
    #[serde(default)]
    pub swatch: String,
}

/// A full parameter set.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Params {
    pub schema: u32,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub earth_radius_km: f64,
    /// Sun elevation that counts as "sunset" (about -0.833 accounts for
    /// atmospheric refraction and the sun's apparent radius).
    pub sunset_elevation_deg: f64,
    pub cloud_layers: Vec<CloudLayer>,
    /// The highest a cloud can sit, km above sea level (about the top of the
    /// weather layer, the tropopause, in the tropics). A layer never goes
    /// above it, however high the place. Optional in a pack file: a pack
    /// without it uses the built-in default pack's.
    #[serde(default = "default_ceiling")]
    pub cloud_ceiling_km: f64,
    pub stages: Vec<Stage>,
    /// Optional colour hints keyed by grazing height (see `cloud::grazing_height_km`).
    #[serde(default)]
    pub grazing_colours: Vec<GrazingColour>,
    /// The glow-score numbers (see `glow` and `docs/GLOW.md`). Optional in a
    /// pack file: a pack without them uses the built-in default pack's.
    #[serde(default = "default_glow")]
    pub glow: GlowParams,
}

/// The `glow` section of the built-in default pack, for packs that leave it out.
fn default_glow() -> GlowParams {
    #[derive(Deserialize)]
    struct OnlyGlow {
        glow: GlowParams,
    }
    serde_json::from_str::<OnlyGlow>(DEFAULT_PACK_JSON)
        .expect("the built-in default pack must have a valid glow section")
        .glow
}

/// `cloud_ceiling_km` of the built-in default pack, for packs that leave it out.
fn default_ceiling() -> f64 {
    #[derive(Deserialize)]
    struct OnlyCeiling {
        cloud_ceiling_km: f64,
    }
    serde_json::from_str::<OnlyCeiling>(DEFAULT_PACK_JSON)
        .expect("the built-in default pack must have cloud_ceiling_km")
        .cloud_ceiling_km
}

impl Params {
    /// Where a layer sits above sea level, km, for ground `ground_km` above
    /// sea level: the pack's height above the ground, but never above
    /// `cloud_ceiling_km`.
    pub fn layer_height_km(&self, layer: &CloudLayer, ground_km: f64) -> f64 {
        (ground_km + layer.height_km).min(self.cloud_ceiling_km)
    }

    /// Parse and validate a pack from JSON text.
    pub fn from_json(json: &str) -> Result<Params, String> {
        let params: Params =
            serde_json::from_str(json).map_err(|e| format!("invalid pack JSON: {e}"))?;
        params.validate()?;
        Ok(params)
    }

    /// The built-in default pack.
    pub fn builtin() -> Params {
        Params::from_json(DEFAULT_PACK_JSON).expect("built-in default pack must be valid")
    }

    /// Colour label for light whose lowest altitude is `grazing_km`, if the
    /// pack defines any.
    pub fn colour_hint(&self, grazing_km: f64) -> Option<&str> {
        self.colour_band(grazing_km).map(|c| c.label.as_str())
    }

    /// The whole colour band (label and swatch) for a grazing height.
    pub fn colour_band(&self, grazing_km: f64) -> Option<&GrazingColour> {
        self.grazing_colours
            .iter()
            .filter(|c| grazing_km >= c.min_height_km)
            .max_by(|a, b| {
                a.min_height_km
                    .partial_cmp(&b.min_height_km)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// Index of the stage whose elevation range holds `elevation_deg`
    /// (`from_deg` >= elevation > `to_deg`), or `None` outside every stage.
    pub fn stage_index(&self, elevation_deg: f64) -> Option<usize> {
        self.stages
            .iter()
            .position(|s| elevation_deg <= s.from_deg && elevation_deg > s.to_deg)
    }

    /// Check that the numbers make physical and logical sense.
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != SUPPORTED_SCHEMA {
            return Err(format!(
                "pack schema {} is not supported (this build understands {})",
                self.schema, SUPPORTED_SCHEMA
            ));
        }
        if !(self.earth_radius_km > 1000.0 && self.earth_radius_km < 10_000.0) {
            return Err("earth_radius_km must be between 1000 and 10000".to_string());
        }
        if !(self.cloud_ceiling_km > 0.0 && self.cloud_ceiling_km < 50.0) {
            return Err("cloud_ceiling_km must be between 0 and 50 km".to_string());
        }
        for layer in &self.cloud_layers {
            if !(layer.height_km > 0.0 && layer.height_km < self.cloud_ceiling_km) {
                return Err(format!(
                    "cloud layer '{}' height must be above 0 and below cloud_ceiling_km",
                    layer.name
                ));
            }
        }
        for stage in &self.stages {
            if stage.from_deg <= stage.to_deg {
                return Err(format!(
                    "stage '{}' must start at a higher sun elevation than it ends",
                    stage.name
                ));
            }
        }
        let swatches = self
            .stages
            .iter()
            .map(|s| (&s.name, &s.swatch))
            .chain(self.grazing_colours.iter().map(|c| (&c.label, &c.swatch)));
        for (name, swatch) in swatches {
            if !swatch.is_empty() && !is_hex_colour(swatch) {
                return Err(format!(
                    "swatch '{swatch}' for '{name}' must look like #rrggbb"
                ));
            }
        }
        self.glow.validate()?;
        for pair in self.stages.windows(2) {
            if pair[1].from_deg > pair[0].to_deg + 1e-9 {
                return Err(format!(
                    "stage '{}' overlaps the one before it",
                    pair[1].name
                ));
            }
        }
        Ok(())
    }
}

/// True for `#rrggbb` (six hex digits after a hash).
fn is_hex_colour(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_pack_loads() {
        let p = Params::builtin();
        assert_eq!(p.name, "default");
        assert!(!p.stages.is_empty());
        assert_eq!(p.cloud_layers.len(), 3);
    }

    #[test]
    fn rejects_unknown_schema() {
        let bad = DEFAULT_PACK_JSON.replace("\"schema\": 1", "\"schema\": 99");
        assert!(Params::from_json(&bad).is_err());
    }

    #[test]
    fn colour_hints_follow_grazing_height() {
        let p = Params::builtin();
        let high = p.colour_hint(5.9).expect("hint for 5.9 km");
        let low = p.colour_hint(1.6).expect("hint for 1.6 km");
        let lowest = p.colour_hint(0.2).expect("hint for 0.2 km");
        assert_ne!(high, low);
        assert_ne!(low, lowest);
    }

    #[test]
    fn swatches_are_optional_but_must_be_hex() {
        let mut p = Params::builtin();
        assert!(p.stages.iter().all(|s| s.swatch.len() == 7));
        p.stages[0].swatch = String::new();
        assert!(p.validate().is_ok(), "an empty swatch means none");
        p.stages[0].swatch = "orange".to_string();
        assert!(p.validate().is_err());
        p.stages[0].swatch = "#12345g".to_string();
        assert!(p.validate().is_err());
    }

    #[test]
    fn a_pack_without_glow_numbers_borrows_the_defaults() {
        let kerala = Params::from_json(include_str!("../../packs/kerala.json")).unwrap();
        assert_eq!(kerala.glow, Params::builtin().glow);
        // A pack's own glow section wins, and is checked.
        let bad = DEFAULT_PACK_JSON.replace("\"low_top_km\": 3.0", "\"low_top_km\": 30.0");
        assert!(Params::from_json(&bad).unwrap_err().contains("low_top_km"));
    }

    #[test]
    fn layers_count_from_the_ground_up_to_the_ceiling() {
        let p = Params::builtin();
        let low = &p.cloud_layers[0];
        assert_eq!(low.height_km, 1.5);
        // Sea level: unchanged. Leh (3.47 km): 1.5 km above its ground.
        assert!((p.layer_height_km(low, 0.0) - 1.5).abs() < 1e-12);
        assert!((p.layer_height_km(low, 3.47) - 4.97).abs() < 1e-12);
        // Never above the ceiling, however high the place.
        let high = &p.cloud_layers[2];
        assert_eq!(p.layer_height_km(high, 8.0), p.cloud_ceiling_km);
        // A pack without a ceiling borrows the default's.
        let kerala = Params::from_json(include_str!("../../packs/kerala.json")).unwrap();
        assert_eq!(kerala.cloud_ceiling_km, p.cloud_ceiling_km);
        // A layer at or above the ceiling is rejected.
        let mut bad = Params::builtin();
        bad.cloud_layers[2].height_km = bad.cloud_ceiling_km;
        assert!(bad.validate().unwrap_err().contains("cloud_ceiling_km"));
    }

    #[test]
    fn rejects_backwards_stage() {
        let mut p = Params::builtin();
        p.stages[0].from_deg = -50.0;
        assert!(p.validate().is_err());
    }
}
