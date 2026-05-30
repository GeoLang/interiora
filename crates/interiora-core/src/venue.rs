//! Venue — top-level indoor venue model (building, mall, airport, etc.).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::floor_plan::FloorPlan;

/// A venue containing multiple floors with indoor mapping data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Venue {
    /// Unique venue identifier.
    pub id: Uuid,
    /// Venue name.
    pub name: String,
    /// Venue category.
    pub category: VenueCategory,
    /// Geographic anchor point (latitude, longitude) for the venue entrance.
    pub anchor_lat: f64,
    pub anchor_lon: f64,
    /// Address.
    pub address: Option<String>,
    /// Floor plans ordered by level.
    pub floors: Vec<FloorPlan>,
}

impl Venue {
    /// Create a new venue.
    pub fn new(name: impl Into<String>, category: VenueCategory, lat: f64, lon: f64) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            category,
            anchor_lat: lat,
            anchor_lon: lon,
            address: None,
            floors: Vec::new(),
        }
    }

    /// Add a floor plan.
    pub fn add_floor(&mut self, floor: FloorPlan) {
        self.floors.push(floor);
        self.floors.sort_by_key(|f| f.level.ordinal);
    }

    /// Get a floor by ordinal level number.
    pub fn floor_by_level(&self, ordinal: i32) -> Option<&FloorPlan> {
        self.floors.iter().find(|f| f.level.ordinal == ordinal)
    }

    /// Get ground floor (ordinal 0).
    pub fn ground_floor(&self) -> Option<&FloorPlan> {
        self.floor_by_level(0)
    }

    /// Total number of floors.
    pub fn floor_count(&self) -> usize {
        self.floors.len()
    }
}

/// Venue category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VenueCategory {
    ShoppingMall,
    Airport,
    TrainStation,
    Hospital,
    University,
    Museum,
    Office,
    Hotel,
    ConventionCenter,
    Warehouse,
    Parking,
    Other,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::floor_plan::{FloorLevel, FloorPlan};

    #[test]
    fn test_create_venue() {
        let venue = Venue::new("Test Mall", VenueCategory::ShoppingMall, 51.5, -0.1);
        assert_eq!(venue.name, "Test Mall");
        assert_eq!(venue.category, VenueCategory::ShoppingMall);
        assert_eq!(venue.floor_count(), 0);
    }

    #[test]
    fn test_add_floors_sorted() {
        let mut venue = Venue::new("Airport", VenueCategory::Airport, 51.5, -0.1);
        venue.add_floor(FloorPlan::new(FloorLevel::new(1, "First Floor")));
        venue.add_floor(FloorPlan::new(FloorLevel::new(-1, "Basement")));
        venue.add_floor(FloorPlan::new(FloorLevel::new(0, "Ground")));

        assert_eq!(venue.floors[0].level.ordinal, -1);
        assert_eq!(venue.floors[1].level.ordinal, 0);
        assert_eq!(venue.floors[2].level.ordinal, 1);
    }

    #[test]
    fn test_floor_by_level() {
        let mut venue = Venue::new("Office", VenueCategory::Office, 40.7, -74.0);
        venue.add_floor(FloorPlan::new(FloorLevel::new(0, "Ground")));
        venue.add_floor(FloorPlan::new(FloorLevel::new(1, "First")));

        assert!(venue.floor_by_level(0).is_some());
        assert!(venue.floor_by_level(5).is_none());
        assert_eq!(venue.ground_floor().unwrap().level.name, "Ground");
    }
}
