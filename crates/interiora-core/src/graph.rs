//! Indoor navigation graph — nodes, edges, and connectivity for routing.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::floor_plan::Point2D;

/// Indoor navigation graph for a venue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndoorGraph {
    pub nodes: Vec<IndoorNode>,
    pub edges: Vec<IndoorEdge>,
    pub(crate) adjacency: HashMap<Uuid, Vec<usize>>,
}

impl IndoorGraph {
    /// Create a new empty graph.
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            adjacency: HashMap::new(),
        }
    }

    /// Add a node and return its ID.
    pub fn add_node(&mut self, node: IndoorNode) -> Uuid {
        let id = node.id;
        self.nodes.push(node);
        self.adjacency.entry(id).or_default();
        id
    }

    /// Add an edge between two nodes.
    pub fn add_edge(&mut self, from: Uuid, to: Uuid, traversal: TraversalType) {
        let from_pos = self.node_position(from);
        let to_pos = self.node_position(to);
        let weight = match (from_pos, to_pos) {
            (Some(a), Some(b)) => a.distance_to(&b),
            _ => 1.0,
        };

        let edge_idx = self.edges.len();
        self.edges.push(IndoorEdge {
            from,
            to,
            weight,
            traversal,
            accessible: true,
        });

        self.adjacency.entry(from).or_default().push(edge_idx);
        // Add reverse edge for bidirectional traversal
        let rev_idx = self.edges.len();
        self.edges.push(IndoorEdge {
            from: to,
            to: from,
            weight,
            traversal,
            accessible: true,
        });
        self.adjacency.entry(to).or_default().push(rev_idx);
    }

    /// Get a node by ID.
    pub fn node(&self, id: Uuid) -> Option<&IndoorNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Get edges from a node.
    pub fn edges_from(&self, node_id: Uuid) -> Vec<&IndoorEdge> {
        self.adjacency
            .get(&node_id)
            .map(|indices| indices.iter().map(|&i| &self.edges[i]).collect())
            .unwrap_or_default()
    }

    /// Find the nearest node to a given position on a given floor.
    pub fn nearest_node(&self, position: &Point2D, floor_ordinal: i32) -> Option<&IndoorNode> {
        self.nodes
            .iter()
            .filter(|n| n.floor_ordinal == floor_ordinal)
            .min_by(|a, b| {
                let da = a.position.distance_to(position);
                let db = b.position.distance_to(position);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// Total node count.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Total edge count (including reverse edges).
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    fn node_position(&self, id: Uuid) -> Option<Point2D> {
        self.node(id).map(|n| n.position)
    }
}

impl Default for IndoorGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// A node in the indoor navigation graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndoorNode {
    pub id: Uuid,
    /// Position in local floor coordinates.
    pub position: Point2D,
    /// Which floor this node is on.
    pub floor_ordinal: i32,
    /// Node type.
    pub kind: NodeKind,
    /// Optional label (e.g., room name, POI name).
    pub label: Option<String>,
}

impl IndoorNode {
    pub fn new(position: Point2D, floor_ordinal: i32, kind: NodeKind) -> Self {
        Self {
            id: Uuid::new_v4(),
            position,
            floor_ordinal,
            kind,
            label: None,
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

/// Type of navigation node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeKind {
    /// A waypoint along a corridor.
    Waypoint,
    /// A room/unit entrance.
    Entrance,
    /// Elevator connection between floors.
    Elevator,
    /// Staircase connection between floors.
    Stairs,
    /// Escalator connection.
    Escalator,
    /// Building entrance/exit.
    Exit,
    /// Decision point (intersection).
    Junction,
}

/// An edge in the indoor navigation graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndoorEdge {
    pub from: Uuid,
    pub to: Uuid,
    /// Edge weight (typically distance in meters).
    pub weight: f64,
    /// How this edge is traversed.
    pub traversal: TraversalType,
    /// Whether this edge is wheelchair-accessible.
    pub accessible: bool,
}

/// How an edge is traversed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraversalType {
    /// Walking on same floor.
    Walk,
    /// Taking an elevator (floor change).
    Elevator,
    /// Taking stairs (floor change).
    Stairs,
    /// Taking an escalator (floor change).
    Escalator,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_graph() {
        let mut graph = IndoorGraph::new();
        let n1 = graph.add_node(IndoorNode::new(
            Point2D::new(0.0, 0.0),
            0,
            NodeKind::Entrance,
        ));
        let n2 = graph.add_node(IndoorNode::new(
            Point2D::new(10.0, 0.0),
            0,
            NodeKind::Waypoint,
        ));
        graph.add_edge(n1, n2, TraversalType::Walk);

        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 2); // bidirectional
    }

    #[test]
    fn test_edges_from() {
        let mut graph = IndoorGraph::new();
        let n1 = graph.add_node(IndoorNode::new(
            Point2D::new(0.0, 0.0),
            0,
            NodeKind::Junction,
        ));
        let n2 = graph.add_node(IndoorNode::new(
            Point2D::new(5.0, 0.0),
            0,
            NodeKind::Waypoint,
        ));
        let n3 = graph.add_node(IndoorNode::new(
            Point2D::new(0.0, 5.0),
            0,
            NodeKind::Waypoint,
        ));
        graph.add_edge(n1, n2, TraversalType::Walk);
        graph.add_edge(n1, n3, TraversalType::Walk);

        let edges = graph.edges_from(n1);
        assert_eq!(edges.len(), 2);
    }

    #[test]
    fn test_nearest_node() {
        let mut graph = IndoorGraph::new();
        graph.add_node(IndoorNode::new(
            Point2D::new(0.0, 0.0),
            0,
            NodeKind::Waypoint,
        ));
        graph.add_node(IndoorNode::new(
            Point2D::new(10.0, 10.0),
            0,
            NodeKind::Waypoint,
        ));
        graph.add_node(IndoorNode::new(
            Point2D::new(5.0, 5.0),
            1,
            NodeKind::Waypoint,
        )); // different floor

        let nearest = graph.nearest_node(&Point2D::new(9.0, 9.0), 0).unwrap();
        assert!((nearest.position.x - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_edge_weight_is_distance() {
        let mut graph = IndoorGraph::new();
        let n1 = graph.add_node(IndoorNode::new(
            Point2D::new(0.0, 0.0),
            0,
            NodeKind::Waypoint,
        ));
        let n2 = graph.add_node(IndoorNode::new(
            Point2D::new(3.0, 4.0),
            0,
            NodeKind::Waypoint,
        ));
        graph.add_edge(n1, n2, TraversalType::Walk);

        let edges = graph.edges_from(n1);
        assert!((edges[0].weight - 5.0).abs() < 1e-10);
    }
}
