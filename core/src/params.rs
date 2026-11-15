//! The tunable numbers, loaded from JSON "packs".
//!
//! Physics that is exact (sun position) lives in code. Numbers that are
//! *estimates* (where the pink window starts, how high cirrus sits in the
//! tropics) live in a pack so they can be corrected without a new release.
//! See `packs/README.md` for the file format.
//!
//! The built-in default pack is compiled into the binary from
//! `packs/default.json`, so the engine always works offline.

use serde::Deserialize;

/// Pack schema version this build understands. Packs with another number are
/// rejected instead of being half-understood.
pub const SUPPORTED_SCHEMA: u32 = 1;

/// The default pack, embedded at compile time.
pub const DEFAULT_PACK_JSON: &str = include_str!("../../packs/default.json");

/// A cloud layer used for shadow calculations.
#[derive(Debug, Clone, Deserialize)]
pub struct CloudLayer {
    /// Short label such as `low`, `mid`, `high`.
    pub name: String,
    /// Typical height above the ground in kilometres.
    pub height_km: f64,
}

/// One named phase of the evening, defined by sun elevation.
#[derive(Debug, Clone, Deserialize)]
pub struct Stage {
    pub name: String,
    /// Sun elevation (degrees) where the stage begins (higher number).
    pub from_deg: f64,
    /// Sun elevation (degrees) where the stage ends (lower number).
    pub to_deg: f64,
    /// Plain-language description of what the sky tends to do.
    #[serde(default)]
    pub description: String,
}

/// A rough colour label for light that dives down to a given altitude.
///
/// The label applies when the beam's lowest altitude is at least
/// `min_height_km`; the entry with the highest matching threshold wins.
#[derive(Debug, Clone, Deserialize)]
pub struct GrazingColour {
    pub min_height_km: f64,
    pub label: String,
}

/// A full parameter set.
#[derive(Debug, Clone, Deserialize)]
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
    pub stages: Vec<Stage>,
    /// Optional colour hints keyed by grazing height (see `cloud::grazing_height_km`).
    #[serde(default)]
    pub grazing_colours: Vec<GrazingColour>,
}

impl Params {
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
        self.grazing_colours
            .iter()
            .filter(|c| grazing_km >= c.min_height_km)
            .max_by(|a, b| {
                a.min_height_km
                    .partial_cmp(&b.min_height_km)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|c| c.label.as_str())
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
        for layer in &self.cloud_layers {
            if !(layer.height_km > 0.0 && layer.height_km < 50.0) {
                return Err(format!(
                    "cloud layer '{}' height must be between 0 and 50 km",
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
    fn rejects_backwards_stage() {
        let mut p = Params::builtin();
        p.stages[0].from_deg = -50.0;
        assert!(p.validate().is_err());
    }
}
