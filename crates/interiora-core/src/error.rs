//! Error types for the indoor mapping engine.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Floor/level not found.
    FloorNotFound(String),
    /// Node not found in graph.
    NodeNotFound(String),
    /// No route exists between two points.
    NoRoute { from: String, to: String },
    /// Invalid floor plan geometry.
    InvalidGeometry(String),
    /// Positioning system unavailable.
    PositioningUnavailable,
    /// Venue data parsing failed.
    ParseError(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FloorNotFound(id) => write!(f, "floor not found: {id}"),
            Self::NodeNotFound(id) => write!(f, "node not found: {id}"),
            Self::NoRoute { from, to } => write!(f, "no route from {from} to {to}"),
            Self::InvalidGeometry(msg) => write!(f, "invalid geometry: {msg}"),
            Self::PositioningUnavailable => write!(f, "positioning unavailable"),
            Self::ParseError(msg) => write!(f, "parse error: {msg}"),
        }
    }
}

impl std::error::Error for Error {}
