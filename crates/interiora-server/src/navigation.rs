//! Indoor routing and fingerprint positioning.

use std::collections::HashMap;

use axum::Json;
use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use interiora_core::Venue;
use interiora_core::graph::IndoorGraph;
use interiora_core::positioning::IndoorPosition;
use interiora_core::routing::{AccessibilityMode, find_route};

use crate::geo::{to_local, to_lonlat};
use crate::geojson::Geometry;
use crate::store::AppState;
use crate::{ApiError, missing_venue};

#[derive(Debug, Deserialize)]
pub struct RouteRequest {
    pub from: RoutePoint,
    pub to: RoutePoint,
    #[serde(default)]
    pub mode: RouteMode,
}

/// A route endpoint, given the way a map client has it: geographic position
/// plus the floor the client is showing.
#[derive(Debug, Deserialize)]
pub struct RoutePoint {
    pub lon: f64,
    pub lat: f64,
    pub floor: i32,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RouteMode {
    #[default]
    Default,
    Accessible,
}

impl From<RouteMode> for AccessibilityMode {
    fn from(mode: RouteMode) -> Self {
        match mode {
            RouteMode::Default => AccessibilityMode::Default,
            RouteMode::Accessible => AccessibilityMode::Wheelchair,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RouteResponse {
    pub geometry: Geometry,
    pub total_distance: f64,
    pub estimated_time_s: f64,
    pub instructions: Vec<String>,
    /// Floor ordinal per geometry vertex, so a client showing one floor can
    /// draw only the part of the route that belongs on it.
    pub floors: Vec<i32>,
}

pub async fn route(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<RouteRequest>,
) -> Result<Json<RouteResponse>, ApiError> {
    let venues = state.read();
    let stored = venues.get(&id).ok_or_else(|| missing_venue(id))?;
    let graph = stored
        .graph
        .as_ref()
        .ok_or_else(|| ApiError::unprocessable(format!("venue {id} has no navigation graph")))?;
    let venue = stored.venue();

    let from = snap(graph, venue, &request.from)?;
    let to = snap(graph, venue, &request.to)?;
    let route = find_route(graph, from, to, request.mode.into())
        .map_err(|e| ApiError::unprocessable(e.to_string()))?;

    let nodes: Vec<_> = route
        .node_ids
        .iter()
        .map(|id| graph.node(*id).expect("routed node is in the graph"))
        .collect();

    Ok(Json(RouteResponse {
        geometry: Geometry::LineString {
            coordinates: nodes
                .iter()
                .map(|n| to_lonlat(venue.anchor_lat, venue.anchor_lon, n.position))
                .collect(),
        },
        total_distance: route.total_distance,
        estimated_time_s: route.estimated_time_s,
        instructions: route
            .segments
            .iter()
            .map(|s| s.instruction.clone())
            .collect(),
        floors: nodes.iter().map(|n| n.floor_ordinal).collect(),
    }))
}

/// Nearest graph node to a geographic position, on that position's floor.
fn snap(graph: &IndoorGraph, venue: &Venue, point: &RoutePoint) -> Result<Uuid, ApiError> {
    let local = to_local(venue.anchor_lat, venue.anchor_lon, point.lon, point.lat);
    graph
        .nearest_node(&local, point.floor)
        .map(|node| node.id)
        .ok_or_else(|| ApiError::unprocessable(format!("no graph node on floor {}", point.floor)))
}

#[derive(Debug, Deserialize)]
pub struct PositionRequest {
    /// Beacon or access point identifier to RSSI in dBm.
    pub signals: HashMap<String, f64>,
}

#[derive(Debug, Serialize)]
pub struct PositionResponse {
    #[serde(flatten)]
    pub position: IndoorPosition,
    pub lon: f64,
    pub lat: f64,
}

pub async fn position(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<PositionRequest>,
) -> Result<Json<PositionResponse>, ApiError> {
    let venues = state.read();
    let stored = venues.get(&id).ok_or_else(|| missing_venue(id))?;
    let engine = stored
        .positioning
        .as_ref()
        .ok_or_else(|| ApiError::not_found(format!("venue {id} has no fingerprints")))?;

    let estimate = engine
        .estimate_position(&request.signals)
        .ok_or_else(|| ApiError::not_found("no position estimate for these signals"))?;

    let venue = stored.venue();
    let [lon, lat] = to_lonlat(venue.anchor_lat, venue.anchor_lon, estimate.position);
    Ok(Json(PositionResponse {
        position: estimate,
        lon,
        lat,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use interiora_core::graph::{IndoorNode, NodeKind};
    use interiora_core::venue::VenueCategory;

    const LAT: f64 = 45.5019;
    const LON: f64 = -73.5674;

    fn fixture() -> (IndoorGraph, Venue, Uuid, Uuid) {
        let mut graph = IndoorGraph::new();
        let near = graph.add_node(IndoorNode::new(
            interiora_core::floor_plan::Point2D::new(10.0, 0.0),
            0,
            NodeKind::Exit,
        ));
        graph.add_node(IndoorNode::new(
            interiora_core::floor_plan::Point2D::new(30.0, 14.0),
            0,
            NodeKind::Waypoint,
        ));
        let upstairs = graph.add_node(IndoorNode::new(
            interiora_core::floor_plan::Point2D::new(12.0, 16.0),
            1,
            NodeKind::Waypoint,
        ));
        let venue = Venue::new("Fixture", VenueCategory::ShoppingMall, LAT, LON);
        (graph, venue, near, upstairs)
    }

    fn point_at(x: f64, y: f64, floor: i32) -> RoutePoint {
        let [lon, lat] = to_lonlat(LAT, LON, interiora_core::floor_plan::Point2D::new(x, y));
        RoutePoint { lon, lat, floor }
    }

    #[test]
    fn snaps_to_the_closest_node_on_the_floor() {
        let (graph, venue, near, _) = fixture();
        let snapped = snap(&graph, &venue, &point_at(11.0, 1.0, 0)).unwrap();
        assert_eq!(snapped, near);
    }

    #[test]
    fn floor_wins_over_distance() {
        let (graph, venue, near, upstairs) = fixture();
        // (11, 1) is metres from the ground floor exit and 16 m from the only
        // node on floor 1, but asking for floor 1 must not snap downstairs
        let snapped = snap(&graph, &venue, &point_at(11.0, 1.0, 1)).unwrap();
        assert_eq!(snapped, upstairs);
        assert_ne!(snapped, near);
    }

    #[test]
    fn empty_floor_is_unprocessable() {
        let (graph, venue, _, _) = fixture();
        let err = snap(&graph, &venue, &point_at(11.0, 1.0, 7)).unwrap_err();
        assert_eq!(err.status(), axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        assert!(err.message().contains("floor 7"), "{}", err.message());
    }
}
