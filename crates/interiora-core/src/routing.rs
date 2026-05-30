//! Indoor routing — Dijkstra shortest path on the indoor graph.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::Error;
use crate::floor_plan::Point2D;
use crate::graph::{IndoorGraph, TraversalType};

/// A computed indoor route.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndoorRoute {
    /// Ordered list of node IDs along the route.
    pub node_ids: Vec<Uuid>,
    /// Route segments with instructions.
    pub segments: Vec<RouteSegment>,
    /// Total distance in meters.
    pub total_distance: f64,
    /// Estimated walk time in seconds (assuming 1.2 m/s).
    pub estimated_time_s: f64,
    /// Whether the route involves floor changes.
    pub multi_floor: bool,
}

/// A segment of an indoor route.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteSegment {
    /// Start position.
    pub from: Point2D,
    /// End position.
    pub to: Point2D,
    /// Floor ordinal for this segment.
    pub floor_ordinal: i32,
    /// Distance of this segment in meters.
    pub distance: f64,
    /// How to traverse this segment.
    pub traversal: TraversalType,
    /// Navigation instruction.
    pub instruction: String,
}

/// Walking speed in meters per second.
const WALK_SPEED_MPS: f64 = 1.2;

/// Accessibility preference for routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessibilityMode {
    /// Use any path (stairs, escalators, elevators).
    Default,
    /// Wheelchair accessible only (elevators, no stairs/escalators).
    Wheelchair,
}

/// Find shortest route between two nodes.
pub fn find_route(
    graph: &IndoorGraph,
    from: Uuid,
    to: Uuid,
    accessibility: AccessibilityMode,
) -> Result<IndoorRoute, Error> {
    if graph.node(from).is_none() {
        return Err(Error::NodeNotFound(from.to_string()));
    }
    if graph.node(to).is_none() {
        return Err(Error::NodeNotFound(to.to_string()));
    }

    // Dijkstra
    let mut dist: HashMap<Uuid, f64> = HashMap::new();
    let mut prev: HashMap<Uuid, Uuid> = HashMap::new();
    let mut heap = BinaryHeap::new();

    dist.insert(from, 0.0);
    heap.push(DijkstraState {
        cost: 0.0,
        node: from,
    });

    while let Some(DijkstraState { cost, node }) = heap.pop() {
        if node == to {
            break;
        }

        if cost > *dist.get(&node).unwrap_or(&f64::MAX) {
            continue;
        }

        for edge in graph.edges_from(node) {
            // Filter by accessibility
            if accessibility == AccessibilityMode::Wheelchair && !edge.accessible {
                continue;
            }

            let next_cost = cost + edge.weight;
            if next_cost < *dist.get(&edge.to).unwrap_or(&f64::MAX) {
                dist.insert(edge.to, next_cost);
                prev.insert(edge.to, node);
                heap.push(DijkstraState {
                    cost: next_cost,
                    node: edge.to,
                });
            }
        }
    }

    // Reconstruct path
    if !prev.contains_key(&to) && from != to {
        return Err(Error::NoRoute {
            from: from.to_string(),
            to: to.to_string(),
        });
    }

    let mut path = vec![to];
    let mut current = to;
    while current != from {
        current = *prev.get(&current).unwrap();
        path.push(current);
    }
    path.reverse();

    // Build segments
    let mut segments = Vec::new();
    let mut total_distance = 0.0;
    let mut has_floor_change = false;

    for window in path.windows(2) {
        let from_node = graph.node(window[0]).unwrap();
        let to_node = graph.node(window[1]).unwrap();

        let edge = graph
            .edges_from(window[0])
            .into_iter()
            .find(|e| e.to == window[1])
            .unwrap();

        if from_node.floor_ordinal != to_node.floor_ordinal {
            has_floor_change = true;
        }

        let instruction = make_instruction(edge.traversal, to_node.floor_ordinal, &to_node.label);
        total_distance += edge.weight;

        segments.push(RouteSegment {
            from: from_node.position,
            to: to_node.position,
            floor_ordinal: from_node.floor_ordinal,
            distance: edge.weight,
            traversal: edge.traversal,
            instruction,
        });
    }

    Ok(IndoorRoute {
        node_ids: path,
        segments,
        total_distance,
        estimated_time_s: total_distance / WALK_SPEED_MPS,
        multi_floor: has_floor_change,
    })
}

fn make_instruction(traversal: TraversalType, target_floor: i32, label: &Option<String>) -> String {
    let destination = label.as_deref().unwrap_or("waypoint");
    match traversal {
        TraversalType::Walk => format!("Walk to {destination}"),
        TraversalType::Elevator => format!("Take elevator to floor {target_floor}"),
        TraversalType::Stairs => format!("Take stairs to floor {target_floor}"),
        TraversalType::Escalator => format!("Take escalator to floor {target_floor}"),
    }
}

#[derive(Clone, PartialEq)]
struct DijkstraState {
    cost: f64,
    node: Uuid,
}

impl Eq for DijkstraState {}

impl Ord for DijkstraState {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .cost
            .partial_cmp(&self.cost)
            .unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for DijkstraState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{IndoorGraph, IndoorNode, NodeKind};

    fn test_graph() -> (IndoorGraph, Uuid, Uuid, Uuid) {
        let mut graph = IndoorGraph::new();
        let a = graph.add_node(
            IndoorNode::new(Point2D::new(0.0, 0.0), 0, NodeKind::Entrance).with_label("Entrance"),
        );
        let b = graph.add_node(
            IndoorNode::new(Point2D::new(10.0, 0.0), 0, NodeKind::Waypoint).with_label("Corridor"),
        );
        let c = graph.add_node(
            IndoorNode::new(Point2D::new(10.0, 10.0), 0, NodeKind::Entrance).with_label("Shop A"),
        );
        graph.add_edge(a, b, TraversalType::Walk);
        graph.add_edge(b, c, TraversalType::Walk);
        (graph, a, b, c)
    }

    #[test]
    fn test_find_route() {
        let (graph, a, _b, c) = test_graph();
        let route = find_route(&graph, a, c, AccessibilityMode::Default).unwrap();

        assert_eq!(route.node_ids.len(), 3);
        assert!((route.total_distance - 20.0).abs() < 1e-10);
        assert!(!route.multi_floor);
        assert!(route.estimated_time_s > 0.0);
    }

    #[test]
    fn test_no_route() {
        let mut graph = IndoorGraph::new();
        let a = graph.add_node(IndoorNode::new(
            Point2D::new(0.0, 0.0),
            0,
            NodeKind::Waypoint,
        ));
        let b = graph.add_node(IndoorNode::new(
            Point2D::new(10.0, 0.0),
            0,
            NodeKind::Waypoint,
        ));
        // No edge between them

        let result = find_route(&graph, a, b, AccessibilityMode::Default);
        assert!(result.is_err());
    }

    #[test]
    fn test_multi_floor_route() {
        let mut graph = IndoorGraph::new();
        let g = graph.add_node(
            IndoorNode::new(Point2D::new(0.0, 0.0), 0, NodeKind::Waypoint).with_label("Ground"),
        );
        let elev_g = graph.add_node(
            IndoorNode::new(Point2D::new(5.0, 0.0), 0, NodeKind::Elevator).with_label("Elevator"),
        );
        let elev_1 = graph.add_node(
            IndoorNode::new(Point2D::new(5.0, 0.0), 1, NodeKind::Elevator).with_label("Elevator"),
        );
        let dest = graph.add_node(
            IndoorNode::new(Point2D::new(10.0, 0.0), 1, NodeKind::Entrance).with_label("Office"),
        );

        graph.add_edge(g, elev_g, TraversalType::Walk);
        graph.add_edge(elev_g, elev_1, TraversalType::Elevator);
        graph.add_edge(elev_1, dest, TraversalType::Walk);

        let route = find_route(&graph, g, dest, AccessibilityMode::Default).unwrap();
        assert!(route.multi_floor);
        assert_eq!(route.node_ids.len(), 4);
    }

    #[test]
    fn test_accessibility_avoids_stairs() {
        let mut graph = IndoorGraph::new();
        let a = graph.add_node(IndoorNode::new(
            Point2D::new(0.0, 0.0),
            0,
            NodeKind::Waypoint,
        ));
        let stairs = graph.add_node(IndoorNode::new(Point2D::new(5.0, 0.0), 1, NodeKind::Stairs));
        let elev = graph.add_node(IndoorNode::new(
            Point2D::new(10.0, 0.0),
            1,
            NodeKind::Elevator,
        ));
        let dest = graph.add_node(IndoorNode::new(
            Point2D::new(15.0, 0.0),
            1,
            NodeKind::Entrance,
        ));

        // Stairs (not accessible)
        let edge_idx = graph.edges.len();
        graph.edges.push(crate::graph::IndoorEdge {
            from: a,
            to: stairs,
            weight: 5.0,
            traversal: TraversalType::Stairs,
            accessible: false,
        });
        graph.adjacency.entry(a).or_default().push(edge_idx);
        let rev_idx = graph.edges.len();
        graph.edges.push(crate::graph::IndoorEdge {
            from: stairs,
            to: a,
            weight: 5.0,
            traversal: TraversalType::Stairs,
            accessible: false,
        });
        graph.adjacency.entry(stairs).or_default().push(rev_idx);

        // Elevator (accessible)
        graph.add_edge(a, elev, TraversalType::Elevator);
        graph.add_edge(stairs, dest, TraversalType::Walk);
        graph.add_edge(elev, dest, TraversalType::Walk);

        // Wheelchair mode should find route via elevator
        let route = find_route(&graph, a, dest, AccessibilityMode::Wheelchair).unwrap();
        assert!(route.node_ids.contains(&elev));
        assert!(!route.node_ids.contains(&stairs));
    }
}
