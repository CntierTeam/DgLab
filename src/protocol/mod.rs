//! Coyote V2 / V3 protocol encode/decode.
//!
//! Behavior aligned with public protocol docs and DGLAB-BT packing rules.
//! No GPL source is copied.

pub mod v2;
pub mod v3;

pub use v2::{Channel, V2};
pub use v3::{IntensityMode, Pulse, V3};
