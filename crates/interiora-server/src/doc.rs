//! The upload document: a venue plus the navigation graph and radio
//! fingerprints that go with it.
//!
//! ```json
//! {
//!   "venue": { "id": "...", "name": "Meridian Centre", "...": "serde Venue" },
//!   "graph": {
//!     "nodes": [{ "x": 10.0, "y": 0.0, "floor": 0, "kind": "Exit", "label": "Main Entrance" }],
//!     "edges": [[0, 1, "Walk"]]
//!   },
//!   "fingerprints": [
//!     { "position": { "x": 10.0, "y": 6.0 }, "floor_ordinal": 0, "signals": { "beacon-lobby": -45.0 } }
//!   ]
//! }
//! ```
//!
//! Edge endpoints are indices into `nodes`. `graph` and `fingerprints` may be
//! omitted; a venue without a graph cannot be routed on.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use interiora_core::Venue;
use interiora_core::floor_plan::Point2D;
use interiora_core::graph::{IndoorGraph, IndoorNode, NodeKind, TraversalType};
use interiora_core::positioning::Fingerprint;

/// A complete indoor map as uploaded and as persisted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndoorMapDoc {
    pub venue: Venue,
    #[serde(default)]
    pub graph: Option<GraphDoc>,
    #[serde(default)]
    pub fingerprints: Vec<Fingerprint>,
}

/// Navigation graph in index form, which is what a JSON document can express.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphDoc {
    pub nodes: Vec<NodeDoc>,
    pub edges: Vec<EdgeDoc>,
}

/// A graph node in local floor metres.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDoc {
    pub x: f64,
    pub y: f64,
    pub floor: i32,
    pub kind: NodeKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// `[from_index, to_index, traversal]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeDoc(pub usize, pub usize, pub TraversalType);

/// Build a routable graph from the document, keeping node order so that
/// `edges` indices line up.
pub fn build_graph(doc: &GraphDoc) -> Result<IndoorGraph, String> {
    let mut graph = IndoorGraph::new();
    let ids: Vec<Uuid> = doc
        .nodes
        .iter()
        .map(|n| {
            let mut node = IndoorNode::new(Point2D::new(n.x, n.y), n.floor, n.kind);
            node.label = n.label.clone();
            graph.add_node(node)
        })
        .collect();

    for EdgeDoc(from, to, traversal) in &doc.edges {
        let from_id = node_id(&ids, *from)?;
        let to_id = node_id(&ids, *to)?;
        graph.add_edge(from_id, to_id, *traversal);
    }

    // add_edge marks every edge accessible, so the wheelchair mode would be a
    // no-op without this: stairs and escalators are not wheelchair traversable.
    for edge in &mut graph.edges {
        if matches!(
            edge.traversal,
            TraversalType::Stairs | TraversalType::Escalator
        ) {
            edge.accessible = false;
        }
    }

    Ok(graph)
}

fn node_id(ids: &[Uuid], index: usize) -> Result<Uuid, String> {
    ids.get(index).copied().ok_or_else(|| {
        format!(
            "edge references node {index} but the graph has {} nodes",
            ids.len()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_floor_doc() -> GraphDoc {
        GraphDoc {
            nodes: vec![
                NodeDoc {
                    x: 0.0,
                    y: 0.0,
                    floor: 0,
                    kind: NodeKind::Waypoint,
                    label: Some("Ground".into()),
                },
                NodeDoc {
                    x: 3.0,
                    y: 4.0,
                    floor: 0,
                    kind: NodeKind::Stairs,
                    label: None,
                },
                NodeDoc {
                    x: 3.0,
                    y: 5.0,
                    floor: 1,
                    kind: NodeKind::Stairs,
                    label: None,
                },
            ],
            edges: vec![
                EdgeDoc(0, 1, TraversalType::Walk),
                EdgeDoc(1, 2, TraversalType::Stairs),
            ],
        }
    }

    #[test]
    fn indices_become_nodes_in_order() {
        let graph = build_graph(&two_floor_doc()).unwrap();
        assert_eq!(graph.node_count(), 3);
        assert_eq!(graph.nodes[0].label.as_deref(), Some("Ground"));
        assert_eq!(graph.nodes[2].floor_ordinal, 1);
        // weights come from the node positions, so 3-4-5
        let walk = graph
            .edges
            .iter()
            .find(|e| e.traversal == TraversalType::Walk)
            .unwrap();
        assert!((walk.weight - 5.0).abs() < 1e-9);
    }

    #[test]
    fn stairs_are_not_wheelchair_accessible() {
        let graph = build_graph(&two_floor_doc()).unwrap();
        for edge in &graph.edges {
            match edge.traversal {
                TraversalType::Stairs => assert!(!edge.accessible),
                _ => assert!(edge.accessible),
            }
        }
    }

    #[test]
    fn out_of_range_edge_is_rejected() {
        let mut doc = two_floor_doc();
        doc.edges.push(EdgeDoc(0, 9, TraversalType::Walk));
        let err = build_graph(&doc).unwrap_err();
        assert!(err.contains("node 9"), "unhelpful message: {err}");
    }
}
