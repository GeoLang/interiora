// Comprehensive integration tests for interiora-core.

use std::collections::HashMap;

use interiora_core::floor_plan::*;
use interiora_core::graph::*;
use interiora_core::positioning::*;
use interiora_core::routing::*;
use interiora_core::venue::*;

// ═══════════════════════════════════════════════════════════════════════════
// Point2D tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_point2d_distance_zero() {
    let p = Point2D::new(5.0, 3.0);
    assert_eq!(p.distance_to(&p), 0.0);
}

#[test]
fn test_point2d_distance_horizontal() {
    let a = Point2D::new(0.0, 0.0);
    let b = Point2D::new(3.0, 0.0);
    assert!((a.distance_to(&b) - 3.0).abs() < 1e-10);
}

#[test]
fn test_point2d_distance_diagonal() {
    let a = Point2D::new(0.0, 0.0);
    let b = Point2D::new(3.0, 4.0);
    assert!((a.distance_to(&b) - 5.0).abs() < 1e-10);
}

// ═══════════════════════════════════════════════════════════════════════════
// FloorPlan tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_floor_plan_add_units() {
    let mut floor = FloorPlan::new(FloorLevel::new(0, "Ground"));
    floor.add_unit(Unit::new(
        "Shop A",
        UnitCategory::Shop,
        vec![
            Point2D::new(0.0, 0.0),
            Point2D::new(5.0, 0.0),
            Point2D::new(5.0, 5.0),
            Point2D::new(0.0, 5.0),
        ],
    ));
    floor.add_unit(Unit::new(
        "Shop B",
        UnitCategory::Shop,
        vec![
            Point2D::new(6.0, 0.0),
            Point2D::new(11.0, 0.0),
            Point2D::new(11.0, 5.0),
            Point2D::new(6.0, 5.0),
        ],
    ));
    assert_eq!(floor.units.len(), 2);
}

#[test]
fn test_floor_plan_find_unit() {
    let mut floor = FloorPlan::new(FloorLevel::new(1, "First"));
    floor.add_unit(Unit::new(
        "Meeting Room",
        UnitCategory::Room,
        vec![
            Point2D::new(0.0, 0.0),
            Point2D::new(4.0, 0.0),
            Point2D::new(4.0, 3.0),
            Point2D::new(0.0, 3.0),
        ],
    ));
    assert!(floor.find_unit("Meeting Room").is_some());
    assert!(floor.find_unit("Kitchen").is_none());
}

#[test]
fn test_floor_plan_openings() {
    let mut floor = FloorPlan::new(FloorLevel::new(0, "Ground"));
    floor.add_opening(Opening {
        id: uuid::Uuid::new_v4(),
        position: Point2D::new(5.0, 2.5),
        kind: OpeningKind::AutomaticDoor,
        accessible: true,
    });
    assert_eq!(floor.openings.len(), 1);
    assert!(floor.openings[0].accessible);
}

#[test]
fn test_floor_plan_amenities() {
    let mut floor = FloorPlan::new(FloorLevel::new(0, "Ground"));
    floor.add_amenity(Amenity {
        id: uuid::Uuid::new_v4(),
        name: "ATM".into(),
        category: AmenityCategory::ATM,
        position: Point2D::new(12.0, 3.0),
    });
    assert_eq!(floor.amenities.len(), 1);
    assert_eq!(floor.amenities[0].name, "ATM");
}

#[test]
fn test_unit_centroid_computation() {
    let unit = Unit::new(
        "Square Room",
        UnitCategory::Room,
        vec![
            Point2D::new(0.0, 0.0),
            Point2D::new(10.0, 0.0),
            Point2D::new(10.0, 10.0),
            Point2D::new(0.0, 10.0),
        ],
    );
    assert!((unit.centroid.x - 5.0).abs() < 1e-10);
    assert!((unit.centroid.y - 5.0).abs() < 1e-10);
}

// ═══════════════════════════════════════════════════════════════════════════
// Venue tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_venue_creation() {
    let venue = Venue::new("Westfield", VenueCategory::ShoppingMall, 51.508, -0.221);
    assert_eq!(venue.name, "Westfield");
    assert_eq!(venue.category, VenueCategory::ShoppingMall);
    assert_eq!(venue.floor_count(), 0);
}

#[test]
fn test_venue_floor_insertion_order() {
    let mut venue = Venue::new("Airport", VenueCategory::Airport, 51.47, -0.45);
    venue.add_floor(FloorPlan::new(FloorLevel::new(2, "Departures")));
    venue.add_floor(FloorPlan::new(FloorLevel::new(0, "Ground")));
    venue.add_floor(FloorPlan::new(FloorLevel::new(1, "Check-in")));

    assert_eq!(venue.floor_count(), 3);
    // Sorted by ordinal
    assert_eq!(venue.floors[0].level.ordinal, 0);
    assert_eq!(venue.floors[1].level.ordinal, 1);
    assert_eq!(venue.floors[2].level.ordinal, 2);
}

#[test]
fn test_venue_floor_by_level() {
    let mut venue = Venue::new("Mall", VenueCategory::ShoppingMall, 40.0, -74.0);
    venue.add_floor(FloorPlan::new(FloorLevel::new(-1, "Basement")));
    venue.add_floor(FloorPlan::new(FloorLevel::new(0, "Ground")));
    venue.add_floor(FloorPlan::new(FloorLevel::new(1, "First")));

    assert!(venue.floor_by_level(0).is_some());
    assert!(venue.floor_by_level(-1).is_some());
    assert!(venue.floor_by_level(5).is_none());
}

#[test]
fn test_venue_ground_floor() {
    let mut venue = Venue::new("Office", VenueCategory::Office, 52.0, 13.0);
    venue.add_floor(FloorPlan::new(FloorLevel::new(0, "Ground")));
    assert!(venue.ground_floor().is_some());
    assert_eq!(venue.ground_floor().unwrap().level.name, "Ground");
}

#[test]
fn test_venue_serialization() {
    let mut venue = Venue::new("Museum", VenueCategory::Museum, 48.86, 2.33);
    venue.add_floor(FloorPlan::new(FloorLevel::new(0, "Entrance Hall")));

    let json = serde_json::to_string(&venue).unwrap();
    let back: Venue = serde_json::from_str(&json).unwrap();
    assert_eq!(back.name, "Museum");
    assert_eq!(back.floor_count(), 1);
}

// ═══════════════════════════════════════════════════════════════════════════
// IndoorGraph tests
// ═══════════════════════════════════════════════════════════════════════════

fn build_simple_graph() -> (IndoorGraph, uuid::Uuid, uuid::Uuid, uuid::Uuid) {
    let mut graph = IndoorGraph::new();
    let a = graph.add_node(IndoorNode::new(
        Point2D::new(0.0, 0.0),
        0,
        NodeKind::Entrance,
    ));
    let b = graph.add_node(IndoorNode::new(
        Point2D::new(10.0, 0.0),
        0,
        NodeKind::Junction,
    ));
    let c = graph.add_node(IndoorNode::new(Point2D::new(10.0, 10.0), 0, NodeKind::Exit));
    graph.add_edge(a, b, TraversalType::Walk);
    graph.add_edge(b, c, TraversalType::Walk);
    (graph, a, b, c)
}

#[test]
fn test_graph_add_nodes() {
    let mut graph = IndoorGraph::new();
    let id = graph.add_node(IndoorNode::new(
        Point2D::new(0.0, 0.0),
        0,
        NodeKind::Waypoint,
    ));
    assert_eq!(graph.node_count(), 1);
    assert!(graph.node(id).is_some());
}

#[test]
fn test_graph_add_edges_bidirectional() {
    let (graph, a, b, _) = build_simple_graph();
    // Each add_edge creates 2 directed edges (bidirectional)
    assert_eq!(graph.edge_count(), 4); // 2 edges × 2 directions
    assert!(!graph.edges_from(a).is_empty());
    assert!(!graph.edges_from(b).is_empty());
}

#[test]
fn test_graph_edge_weight_is_distance() {
    let (graph, a, _, _) = build_simple_graph();
    let edges = graph.edges_from(a);
    // A→B distance should be 10.0 (horizontal 10 units)
    assert!((edges[0].weight - 10.0).abs() < 1e-10);
}

#[test]
fn test_graph_nearest_node() {
    let (graph, a, b, _) = build_simple_graph();
    let query = Point2D::new(2.0, 0.0); // closer to A
    let nearest = graph.nearest_node(&query, 0).unwrap();
    assert_eq!(nearest.id, a);

    let query2 = Point2D::new(8.0, 0.0); // closer to B
    let nearest2 = graph.nearest_node(&query2, 0).unwrap();
    assert_eq!(nearest2.id, b);
}

#[test]
fn test_graph_nearest_node_floor_filter() {
    let mut graph = IndoorGraph::new();
    graph.add_node(IndoorNode::new(
        Point2D::new(0.0, 0.0),
        0,
        NodeKind::Waypoint,
    ));
    let floor1 = graph.add_node(IndoorNode::new(
        Point2D::new(0.0, 0.0),
        1,
        NodeKind::Waypoint,
    ));

    let query = Point2D::new(0.0, 0.0);
    let nearest = graph.nearest_node(&query, 1).unwrap();
    assert_eq!(nearest.id, floor1);
}

#[test]
fn test_graph_node_with_label() {
    let mut graph = IndoorGraph::new();
    let node =
        IndoorNode::new(Point2D::new(5.0, 5.0), 0, NodeKind::Entrance).with_label("Main Entrance");
    let id = graph.add_node(node);
    let n = graph.node(id).unwrap();
    assert_eq!(n.label.as_deref(), Some("Main Entrance"));
}

// ═══════════════════════════════════════════════════════════════════════════
// Routing tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_route_simple_path() {
    let (graph, a, _, c) = build_simple_graph();
    let route = find_route(&graph, a, c, AccessibilityMode::Default).unwrap();
    assert_eq!(route.node_ids.len(), 3); // A → B → C
    assert!(!route.multi_floor);
    // Total distance: 10 + 10 = 20
    assert!((route.total_distance - 20.0).abs() < 1e-6);
}

#[test]
fn test_route_estimated_time() {
    let (graph, a, _, c) = build_simple_graph();
    let route = find_route(&graph, a, c, AccessibilityMode::Default).unwrap();
    // 20m at 1.2 m/s ≈ 16.67s
    assert!((route.estimated_time_s - 20.0 / 1.2).abs() < 0.1);
}

#[test]
fn test_route_same_node() {
    let (graph, a, _, _) = build_simple_graph();
    let route = find_route(&graph, a, a, AccessibilityMode::Default).unwrap();
    assert_eq!(route.node_ids.len(), 1);
    assert_eq!(route.total_distance, 0.0);
}

#[test]
fn test_route_nonexistent_node() {
    let (graph, a, _, _) = build_simple_graph();
    let fake = uuid::Uuid::new_v4();
    let result = find_route(&graph, a, fake, AccessibilityMode::Default);
    assert!(result.is_err());
}

#[test]
fn test_route_multi_floor() {
    let mut graph = IndoorGraph::new();
    let lobby = graph.add_node(IndoorNode::new(
        Point2D::new(0.0, 0.0),
        0,
        NodeKind::Entrance,
    ));
    let elevator_g = graph.add_node(IndoorNode::new(
        Point2D::new(10.0, 0.0),
        0,
        NodeKind::Elevator,
    ));
    let elevator_1 = graph.add_node(IndoorNode::new(
        Point2D::new(10.0, 0.0),
        1,
        NodeKind::Elevator,
    ));
    let office = graph.add_node(IndoorNode::new(
        Point2D::new(20.0, 0.0),
        1,
        NodeKind::Entrance,
    ));

    graph.add_edge(lobby, elevator_g, TraversalType::Walk);
    graph.add_edge(elevator_g, elevator_1, TraversalType::Elevator);
    graph.add_edge(elevator_1, office, TraversalType::Walk);

    let route = find_route(&graph, lobby, office, AccessibilityMode::Default).unwrap();
    assert!(route.multi_floor);
    assert_eq!(route.node_ids.len(), 4);
}

// ═══════════════════════════════════════════════════════════════════════════
// Positioning tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_positioning_empty_db() {
    let engine = PositioningEngine::new();
    let signals = HashMap::from([("beacon1".to_string(), -60.0)]);
    assert!(engine.estimate_position(&signals).is_none());
}

#[test]
fn test_positioning_single_fingerprint() {
    let mut engine = PositioningEngine::new();
    engine.add_fingerprint(Fingerprint {
        position: Point2D::new(5.0, 5.0),
        floor_ordinal: 0,
        signals: HashMap::from([("ap1".to_string(), -50.0), ("ap2".to_string(), -70.0)]),
    });

    let signals = HashMap::from([("ap1".to_string(), -50.0), ("ap2".to_string(), -70.0)]);
    let pos = engine.estimate_position(&signals).unwrap();
    assert!((pos.position.x - 5.0).abs() < 0.1);
    assert!((pos.position.y - 5.0).abs() < 0.1);
    assert_eq!(pos.floor_ordinal, 0);
}

#[test]
fn test_positioning_knn_averaging() {
    let mut engine = PositioningEngine::new();
    engine.set_k(2);

    // Two reference points
    engine.add_fingerprint(Fingerprint {
        position: Point2D::new(0.0, 0.0),
        floor_ordinal: 0,
        signals: HashMap::from([("ap1".to_string(), -40.0)]),
    });
    engine.add_fingerprint(Fingerprint {
        position: Point2D::new(10.0, 0.0),
        floor_ordinal: 0,
        signals: HashMap::from([("ap1".to_string(), -80.0)]),
    });

    // Signal midway between the two
    let signals = HashMap::from([("ap1".to_string(), -60.0)]);
    let pos = engine.estimate_position(&signals).unwrap();
    // Should be somewhere between 0 and 10 on x-axis
    assert!(pos.position.x > 0.0 && pos.position.x < 10.0);
}

#[test]
fn test_positioning_floor_detection() {
    let mut engine = PositioningEngine::new();
    engine.set_k(1);

    engine.add_fingerprint(Fingerprint {
        position: Point2D::new(5.0, 5.0),
        floor_ordinal: 0,
        signals: HashMap::from([("ground_ap".to_string(), -45.0)]),
    });
    engine.add_fingerprint(Fingerprint {
        position: Point2D::new(5.0, 5.0),
        floor_ordinal: 1,
        signals: HashMap::from([("first_floor_ap".to_string(), -45.0)]),
    });

    // Should match ground floor fingerprint
    let signals = HashMap::from([("ground_ap".to_string(), -45.0)]);
    let pos = engine.estimate_position(&signals).unwrap();
    assert_eq!(pos.floor_ordinal, 0);
}

#[test]
fn test_positioning_load_bulk_fingerprints() {
    let mut engine = PositioningEngine::new();
    let fps: Vec<Fingerprint> = (0..10)
        .map(|i| Fingerprint {
            position: Point2D::new(i as f64 * 2.0, 0.0),
            floor_ordinal: 0,
            signals: HashMap::from([("ap".to_string(), -40.0 - (i as f64) * 4.0)]),
        })
        .collect();
    engine.load_fingerprints(fps);

    // Should be able to estimate
    let signals = HashMap::from([("ap".to_string(), -56.0)]);
    let pos = engine.estimate_position(&signals).unwrap();
    assert_eq!(pos.floor_ordinal, 0);
}

// ═══════════════════════════════════════════════════════════════════════════
// Integration: venue + graph + routing end-to-end
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_integration_mall_navigation() {
    // Build a small mall venue
    let mut venue = Venue::new("Test Mall", VenueCategory::ShoppingMall, 51.5, -0.1);

    let mut ground = FloorPlan::new(FloorLevel::new(0, "Ground"));
    ground.add_unit(Unit::new(
        "Food Court",
        UnitCategory::Other,
        vec![
            Point2D::new(0.0, 0.0),
            Point2D::new(20.0, 0.0),
            Point2D::new(20.0, 15.0),
            Point2D::new(0.0, 15.0),
        ],
    ));
    venue.add_floor(ground);

    // Build navigation graph
    let mut graph = IndoorGraph::new();
    let entrance = graph.add_node(
        IndoorNode::new(Point2D::new(0.0, 7.5), 0, NodeKind::Exit).with_label("Main Entrance"),
    );
    let junction = graph.add_node(IndoorNode::new(
        Point2D::new(10.0, 7.5),
        0,
        NodeKind::Junction,
    ));
    let food_court = graph.add_node(
        IndoorNode::new(Point2D::new(20.0, 7.5), 0, NodeKind::Entrance)
            .with_label("Food Court Entrance"),
    );

    graph.add_edge(entrance, junction, TraversalType::Walk);
    graph.add_edge(junction, food_court, TraversalType::Walk);

    // Route from entrance to food court
    let route = find_route(&graph, entrance, food_court, AccessibilityMode::Default).unwrap();
    assert_eq!(route.node_ids.len(), 3);
    assert!(!route.multi_floor);
    assert!((route.total_distance - 20.0).abs() < 1e-6);
    assert_eq!(venue.floor_count(), 1);
}

#[test]
fn test_integration_multi_floor_wheelchair() {
    let mut graph = IndoorGraph::new();

    // Ground floor
    let entrance = graph.add_node(IndoorNode::new(Point2D::new(0.0, 0.0), 0, NodeKind::Exit));
    let stairs_g = graph.add_node(IndoorNode::new(Point2D::new(5.0, 0.0), 0, NodeKind::Stairs));
    let elevator_g = graph.add_node(IndoorNode::new(
        Point2D::new(10.0, 0.0),
        0,
        NodeKind::Elevator,
    ));

    // First floor
    let stairs_1 = graph.add_node(IndoorNode::new(Point2D::new(5.0, 0.0), 1, NodeKind::Stairs));
    let elevator_1 = graph.add_node(IndoorNode::new(
        Point2D::new(10.0, 0.0),
        1,
        NodeKind::Elevator,
    ));
    let destination = graph.add_node(IndoorNode::new(
        Point2D::new(20.0, 0.0),
        1,
        NodeKind::Entrance,
    ));

    // Ground floor connections
    graph.add_edge(entrance, stairs_g, TraversalType::Walk);
    graph.add_edge(entrance, elevator_g, TraversalType::Walk);

    // Floor connections
    graph.add_edge(stairs_g, stairs_1, TraversalType::Stairs);
    graph.add_edge(elevator_g, elevator_1, TraversalType::Elevator);

    // First floor connections
    graph.add_edge(stairs_1, destination, TraversalType::Walk);
    graph.add_edge(elevator_1, destination, TraversalType::Walk);

    // Default route should work
    let route = find_route(&graph, entrance, destination, AccessibilityMode::Default).unwrap();
    assert!(route.multi_floor);
    assert!(route.total_distance > 0.0);

    // Wheelchair route should also work (via elevator)
    let wheelchair_route =
        find_route(&graph, entrance, destination, AccessibilityMode::Wheelchair).unwrap();
    assert!(wheelchair_route.multi_floor);
}
