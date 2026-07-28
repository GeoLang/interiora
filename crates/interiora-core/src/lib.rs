//! # interiora-core
//!
//! Indoor mapping engine — floor plans, indoor graph routing,
//! and BLE/WiFi positioning.

pub mod error;
pub mod floor_plan;
pub mod graph;
pub mod positioning;
pub mod routing;
pub mod venue;

pub use error::Error;
pub use floor_plan::{FloorLevel, FloorPlan};
pub use graph::{IndoorEdge, IndoorGraph, IndoorNode};
pub use positioning::{Fingerprint, IndoorPosition, PositioningEngine};
pub use routing::{IndoorRoute, RouteSegment};
pub use venue::Venue;
