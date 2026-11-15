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
  cloud_layers: { name: string; height_km: number }[];
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
  height_km: number;
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
  height_km: number;
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

export interface LocalDate {
  year: number;
  month: number;
  day: number;
}
