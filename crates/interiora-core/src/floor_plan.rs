//! Floor plan — geometry and features for a single building level.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A single floor/level in a venue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloorPlan {
    /// Unique floor ID.
    pub id: Uuid,
    /// Level information.
    pub level: FloorLevel,
    /// Units (rooms, shops, areas) on this floor.
    pub units: Vec<Unit>,
    /// Openings (doors, gates) on this floor.
    pub openings: Vec<Opening>,
    /// Points of interest on this floor.
    pub amenities: Vec<Amenity>,
}

impl FloorPlan {
    /// Create a new empty floor plan.
    pub fn new(level: FloorLevel) -> Self {
        Self {
            id: Uuid::new_v4(),
            level,
            units: Vec::new(),
            openings: Vec::new(),
            amenities: Vec::new(),
        }
    }

    /// Add a unit (room/shop/area).
    pub fn add_unit(&mut self, unit: Unit) {
        self.units.push(unit);
    }

    /// Add an opening (door).
    pub fn add_opening(&mut self, opening: Opening) {
        self.openings.push(opening);
    }

    /// Add an amenity.
    pub fn add_amenity(&mut self, amenity: Amenity) {
        self.amenities.push(amenity);
    }

    /// Find a unit by name.
    pub fn find_unit(&self, name: &str) -> Option<&Unit> {
        self.units.iter().find(|u| u.name == name)
    }
}

/// Floor level metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloorLevel {
    /// Integer ordinal (... -1=basement, 0=ground, 1=first, 2=second ...).
    pub ordinal: i32,
    /// Human-readable name.
    pub name: String,
    /// Short label (e.g., "B1", "G", "1F").
    pub short_name: Option<String>,
}

impl FloorLevel {
    pub fn new(ordinal: i32, name: impl Into<String>) -> Self {
        Self {
            ordinal,
            name: name.into(),
            short_name: None,
        }
    }
}

/// A spatial unit on a floor (room, shop, corridor, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Unit {
    pub id: Uuid,
    pub name: String,
    pub category: UnitCategory,
    /// Polygon boundary as (x, y) pairs in local floor coordinates.
    pub geometry: Vec<Point2D>,
    /// Centroid for label placement.
    pub centroid: Point2D,
}

impl Unit {
    pub fn new(name: impl Into<String>, category: UnitCategory, geometry: Vec<Point2D>) -> Self {
        let centroid = compute_centroid(&geometry);
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            category,
            geometry,
            centroid,
        }
    }
}

/// Unit category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnitCategory {
    Room,
    Shop,
    Corridor,
    Lobby,
    Restroom,
    Elevator,
    Stairs,
    Escalator,
    Parking,
    Storage,
    Office,
    Other,
}

/// An opening (door, gate, archway).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opening {
    pub id: Uuid,
    /// Position of the opening.
    pub position: Point2D,
    /// Type of opening.
    pub kind: OpeningKind,
    /// Whether this opening is accessible (wheelchair).
    pub accessible: bool,
}

/// Opening type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpeningKind {
    Door,
    AutomaticDoor,
    Gate,
    Archway,
    EmergencyExit,
}

/// An amenity / point of interest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Amenity {
    pub id: Uuid,
    pub name: String,
    pub category: AmenityCategory,
    pub position: Point2D,
}

/// Amenity category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AmenityCategory {
    Restroom,
    Elevator,
    Stairs,
    Escalator,
    ATM,
    InfoDesk,
    FoodCourt,
    Parking,
    Entrance,
    Exit,
    EmergencyExit,
    WaterFountain,
    ChargingStation,
    Other,
}

/// 2D point in local floor coordinate system (meters from floor origin).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Point2D {
    pub x: f64,
    pub y: f64,
}

impl Point2D {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Euclidean distance to another point.
    pub fn distance_to(&self, other: &Self) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}

fn compute_centroid(points: &[Point2D]) -> Point2D {
    if points.is_empty() {
        return Point2D::new(0.0, 0.0);
    }
    let n = points.len() as f64;
    let sum_x: f64 = points.iter().map(|p| p.x).sum();
    let sum_y: f64 = points.iter().map(|p| p.y).sum();
    Point2D::new(sum_x / n, sum_y / n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_floor() {
        let mut floor = FloorPlan::new(FloorLevel::new(0, "Ground"));
        floor.add_unit(Unit::new(
            "Lobby",
            UnitCategory::Lobby,
            vec![
                Point2D::new(0.0, 0.0),
                Point2D::new(10.0, 0.0),
                Point2D::new(10.0, 10.0),
                Point2D::new(0.0, 10.0),
            ],
        ));
        assert_eq!(floor.units.len(), 1);
        assert_eq!(floor.find_unit("Lobby").unwrap().centroid.x, 5.0);
    }

    #[test]
    fn test_point_distance() {
        let a = Point2D::new(0.0, 0.0);
        let b = Point2D::new(3.0, 4.0);
        assert!((a.distance_to(&b) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_floor_level_ordering() {
        let basement = FloorLevel::new(-1, "Basement");
        let ground = FloorLevel::new(0, "Ground");
        assert!(basement.ordinal < ground.ordinal);
    }
}
