//! # skyglow-core
//!
//! The deterministic heart of Skyglow: given a place and a date, work out
//! *when* the sky does what, anywhere on Earth, with no data connection.
//!
//! Module map (read them in this order if you are learning the physics):
//!
//! 1. [`time`]     - calendar maths: Julian Day, parsing dates, clock formatting.
//! 2. [`sun`]      - where the sun is (declination, hour angle, elevation, azimuth)
//!                   and when it crosses a given elevation.
//! 3. [`cloud`]    - how Earth's shadow climbs the sky and when a cloud layer
//!                   at a given height goes dark.
//! 4. [`params`]   - the tunable numbers (stage boundaries, cloud heights) that
//!                   live in JSON "packs" instead of the code.
//! 5. [`place`]    - your viewpoint: height above sea level, skyline, time zone.
//! 6. [`timeline`] - ties everything together into one evening timeline.
//! 7. [`sky`]      - the sky at a single instant: sun position, which cloud
//!                   layers are lit, and where Earth's shadow edge sits.
//! 8. [`conditions`] - what the air is like (cloud cover, moisture, haze), as
//!                   plain numbers handed in from outside, here and along the
//!                   sun's direction. Used by the glow score.
//! 9. [`openmeteo`] - turns the text of an Open-Meteo weather answer into
//!                   `conditions`. It reads text it is given; it never goes
//!                   online itself.
//! 10. [`glow`]    - the glow score: sky geometry plus conditions (is the cloud
//!                   there, is the sunbeam blocked, haze, damp air) as a rough
//!                   0-10 score per layer, with every factor shown.
//! 11. [`sunward`] - points along the sun's direction where the forecast is
//!                   read, so the score uses the cloud the sunbeam crosses.
//!
//! Design rules: no global state, no I/O in the library (only the CLI reads
//! files), and every calculation can show its intermediate values
//! ("explain mode") so nothing is a black box.

pub mod cloud;
pub mod conditions;
pub mod glow;
pub mod openmeteo;
pub mod params;
pub mod place;
pub mod sky;
pub mod sun;
pub mod sunward;
pub mod time;
pub mod timeline;
