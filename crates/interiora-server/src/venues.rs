//! Venue catalogue: upload, list, delete, and per-floor GeoJSON.

use std::fmt::Debug;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::{Json, extract};
use serde::Serialize;
use uuid::Uuid;

use interiora_core::floor_plan::{FloorPlan, Point2D};
use interiora_core::venue::{Venue, VenueCategory};

use crate::doc::IndoorMapDoc;
use crate::geo::to_lonlat;
use crate::geojson::{Feature, FeatureCollection, FeatureProperties, Geometry};
use crate::store::{AppState, InsertError};
use crate::{ApiError, missing_venue};

/// What the catalogue shows for one venue.
#[derive(Debug, Serialize)]
pub struct VenueSummary {
    pub id: Uuid,
    pub name: String,
    pub category: VenueCategory,
    pub lat: f64,
    pub lon: f64,
    pub floor_count: usize,
    /// Floor ordinals, low to high, for the floor picker.
    pub floors: Vec<i32>,
}

#[derive(Debug, Serialize)]
pub struct VenueId {
    pub id: Uuid,
}

pub async fn upload(
    State(state): State<AppState>,
    Json(doc): Json<IndoorMapDoc>,
) -> Result<Json<VenueId>, ApiError> {
    let id = state.insert(doc).map_err(|e| match e {
        InsertError::Invalid(message) => ApiError::unprocessable(message),
        InsertError::Io(error) => ApiError::internal(error.to_string()),
    })?;
    Ok(Json(VenueId { id }))
}

pub async fn list(State(state): State<AppState>) -> Json<Vec<VenueSummary>> {
    let venues = state.read();
    let mut summaries: Vec<VenueSummary> = venues
        .values()
        .map(|stored| {
            let venue = stored.venue();
            VenueSummary {
                id: venue.id,
                name: venue.name.clone(),
                category: venue.category,
                lat: venue.anchor_lat,
                lon: venue.anchor_lon,
                floor_count: venue.floor_count(),
                floors: venue.floors.iter().map(|f| f.level.ordinal).collect(),
            }
        })
        .collect();
    // the store is a hash map, so sort for a stable catalogue
    summaries.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
    Json(summaries)
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let removed = state
        .remove(id)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    if removed {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(missing_venue(id))
    }
}

pub async fn floor_geojson(
    State(state): State<AppState>,
    extract::Path((id, ordinal)): extract::Path<(Uuid, i32)>,
) -> Result<Json<FeatureCollection>, ApiError> {
    let venues = state.read();
    let stored = venues.get(&id).ok_or_else(|| missing_venue(id))?;
    let venue = stored.venue();
    let floor = venue
        .floor_by_level(ordinal)
        .ok_or_else(|| ApiError::not_found(format!("venue {id} has no floor {ordinal}")))?;
    Ok(Json(FeatureCollection::new(floor_features(venue, floor))))
}

fn floor_features(venue: &Venue, floor: &FloorPlan) -> Vec<Feature> {
    let project = |p: Point2D| to_lonlat(venue.anchor_lat, venue.anchor_lon, p);
    let level = floor.level.ordinal;
    let level_name = &floor.level.name;

    let units = floor.units.iter().map(|unit| {
        // GeoJSON rings close on their first position
        let mut ring: Vec<[f64; 2]> = unit.geometry.iter().map(|p| project(*p)).collect();
        if let Some(first) = ring.first().copied()
            && ring.last() != Some(&first)
        {
            ring.push(first);
        }
        Feature::new(
            Geometry::Polygon {
                coordinates: vec![ring],
            },
            FeatureProperties {
                feature: "unit",
                id: unit.id,
                name: Some(unit.name.clone()),
                category: variant_name(unit.category),
                level,
                level_name: level_name.clone(),
                accessible: None,
            },
        )
    });

    let openings = floor.openings.iter().map(|opening| {
        Feature::new(
            Geometry::Point {
                coordinates: project(opening.position),
            },
            FeatureProperties {
                feature: "opening",
                id: opening.id,
                name: None,
                category: variant_name(opening.kind),
                level,
                level_name: level_name.clone(),
                accessible: Some(opening.accessible),
            },
        )
    });

    let amenities = floor.amenities.iter().map(|amenity| {
        Feature::new(
            Geometry::Point {
                coordinates: project(amenity.position),
            },
            FeatureProperties {
                feature: "amenity",
                id: amenity.id,
                name: Some(amenity.name.clone()),
                category: variant_name(amenity.category),
                level,
                level_name: level_name.clone(),
                accessible: None,
            },
        )
    });

    units.chain(openings).chain(amenities).collect()
}

/// The core categories are fieldless enums, where Debug prints exactly the
/// name serde would.
fn variant_name(value: impl Debug) -> String {
    format!("{value:?}")
}
