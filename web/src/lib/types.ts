// TypeScript shapes of the JSON the Rust core sends back. Field names match
// the Rust structs exactly (see core/src/*.rs); keep the two in step.

export type EventName = "sunset" | "sunrise";

export interface HorizonPoint {
  az: number;
  elev: number;
}

export interface Peak {
  name: string;
  az: number;
  distance_km: number;
  height_m: number;
  half_width_deg?: number;
}

/** Same format as examples/places/*.json. */
export interface Place {
  name: string;
  lat: number;
  lon: number;
  tz?: number | null;
  tz_name?: string;
  dst_tz?: number | null;
  dst_tz_name?: string;
  dst_note?: string;
  elevation_m: number;
  horizon: HorizonPoint[];
  peaks: Peak[];
  note?: string;
  /** IANA time zone (e.g. "Asia/Kolkata"), found online. Web only: the core ignores it. */
  zone?: string;
}

export interface Stage {
  name: string;
  from_deg: number;
  to_deg: number;
  description: string;
  morning_name: string;
  morning_description: string;
  swatch: string;
}

export interface GrazingColour {
  min_height_km: number;
  label: string;
  swatch: string;
}

export interface Pack {
  schema: number;
  name: string;
  description: string;
  earth_radius_km: number;
  sunset_elevation_deg: number;
  /** Layer heights are above the ground; the engine adds the place's height. */
  cloud_layers: { name: string; height_km: number }[];
  /** The highest a cloud can sit, km above sea level. */
  cloud_ceiling_km: number;
  stages: Stage[];
  grazing_colours: GrazingColour[];
}

export interface StageTime {
  name: string;
  description: string;
  from_deg: number;
  to_deg: number;
  start_jd: number | null;
  end_jd: number | null;
}

export interface LayerReport {
  name: string;
  /** Above sea level, km. */
  height_km: number;
  above_ground_km: number;
  shadow_depression_deg: number;
  overhead_shadow_jd: number | null;
}

export interface Timeline {
  event: EventName;
  lat_deg: number;
  lon_deg: number;
  tz_hours: number;
  date: [number, number, number];
  solar_noon_jd: number;
  elevation_m: number;
  dip_deg: number;
  has_skyline: boolean;
  sea_level_event_jd: number | null;
  sea_event_jd: number | null;
  event_jd: number | null;
  event_azimuth_deg: number | null;
  stages: StageTime[];
  layers: LayerReport[];
}

export type LayerLight =
  | { kind: "lit_overhead"; grazing_km: number; colour: string | null; swatch: string | null }
  | { kind: "lit_toward_sun"; edge_elevation_deg: number; edge_distance_km: number }
  | { kind: "edge_beyond_horizon" }
  | { kind: "shadow_overhead" };

export interface LayerState {
  name: string;
  /** Above sea level, km. */
  height_km: number;
  above_ground_km: number;
  light: LayerLight;
}

export interface SkyState {
  jd: number;
  elevation_deg: number;
  azimuth_deg: number;
  depression_deg: number;
  rising: boolean;
  stage: number | null;
  skyline: { horizon_elevation_deg: number; sun_hidden: boolean } | null;
  overhead_lit_above_km: number | null;
  layers: LayerState[];
}

/** What the air is like at one moment (core/src/conditions.rs). A missing field means "not known". */
export interface Conditions {
  /** Share of the sky covered by low cloud (below about 3 km), 0 to 100. */
  cloud_low_pct?: number;
  /** Middle cloud (about 3 to 8 km), 0 to 100. */
  cloud_mid_pct?: number;
  /** High cloud (above about 8 km), 0 to 100. */
  cloud_high_pct?: number;
  /** Relative humidity near the ground, 0 to 100. */
  humidity_pct?: number;
  /** Water vapour in the whole column of air, mm. */
  water_vapour_mm?: number;
  /** Aerosol optical depth at 550 nm: haze, dust and smoke. */
  aerosol_optical_depth?: number;
}

export type ConditionsSample = Conditions & { jd: number };

export interface ConditionsSeries {
  /** Where the numbers came from, in words. */
  source: string;
  samples: ConditionsSample[];
}

/** How a reading was worked out from the series. */
export type ConditionsBasis =
  | { kind: "between"; before_jd: number; after_jd: number }
  | { kind: "nearest"; sample_jd: number };

export interface ConditionsReading {
  conditions: Conditions;
  basis: ConditionsBasis;
}

/** A forecast cloud level (core/src/glow.rs). */
export type Level = "low" | "mid" | "high";

/** Every factor behind a glow score, each 0 to 1 (core/src/glow.rs). */
export interface GlowFactors {
  light: number;
  sun: number;
  colour: number;
  cover: number | null;
  path: number;
  view: number;
  haze: number;
  humidity: number;
}

/** Where the sunbeam crosses another cloud level on its way (core/src/glow.rs). */
export interface Crossing {
  level: Level;
  /** Distance toward the sun, km. */
  distance_km: number;
  cover_pct: number | null;
}

export interface LayerGlow {
  name: string;
  /** Above sea level, km. */
  height_km: number;
  above_ground_km: number;
  level: Level;
  /** The heights this forecast level spans here, km above sea level: [bottom, top]. */
  range_km: [number, number];
  /** A likely cloud type from the forecast cover (a rough guess), or null. */
  cloud_type: string | null;
  beam_low_km: number | null;
  /** How far toward the sun the lit part you see is, km (0 overhead). */
  cover_at_km: number;
  cover_pct: number | null;
  factors: GlowFactors;
  /** Geometry only, 0 to 10: the score if cloud were there and the air clear. */
  potential: number;
  /** 0 to 10, or null without conditions. */
  score: number | null;
  word: string | null;
  blocked_by: Level[];
  crossings: Crossing[];
  seen_through: Level[];
  unknown: string[];
}

/** The highest forecast level with cloud overhead (core/src/glow.rs). */
export interface HighestCloud {
  level: Level;
  cover_pct: number;
  /** km above sea level; forecasts give no cloud tops, so the cloud is somewhere in here. */
  range_km: [number, number];
}

export interface Glow {
  jd: number;
  highest_cloud: HighestCloud | null;
  has_conditions: boolean;
  /** True when the forecast along the sun's direction was used for the sunbeam's path. */
  along_sun: boolean;
  layers: LayerGlow[];
}

export interface GlowMoment {
  jd: number;
  layer: string;
  height_km: number;
  score: number;
  word: string;
}

export interface GeoPoint {
  lat: number;
  lon: number;
}

/** Where one direction's points sit in a WeatherPlan (core/src/sunward.rs). */
export interface Direction {
  azimuth_deg: number;
  first: number;
  distances_km: number[];
}

/** The places to ask the forecast about: the place, then toward sunset, then toward sunrise. */
export interface WeatherPlan {
  points: GeoPoint[];
  sunset: Direction | null;
  sunrise: Direction | null;
}

export interface Sunward {
  azimuth_deg: number;
  points: { distance_km: number; series: ConditionsSeries }[];
}

/** All the readings for one place and date (core/src/conditions.rs). */
export interface Weather {
  here: ConditionsSeries | null;
  sunset: Sunward | null;
  sunrise: Sunward | null;
}

export interface LocalDate {
  year: number;
  month: number;
  day: number;
}
