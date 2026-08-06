# Interiora

[![CI](https://github.com/GeoLang/interiora/actions/workflows/ci.yml/badge.svg)](https://github.com/GeoLang/interiora/actions)
[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL--3.0-blue.svg)](LICENSE)

**Indoor mapping and navigation SDK for the GeoLang ecosystem.**

Interiora provides data structures and algorithms for indoor venue modelling, wayfinding, and BLE/WiFi fingerprint-based positioning.

## Features

- **Venue modelling** — Venues, floors, units (rooms/shops/areas), openings, and amenities
- **Indoor graph** — Connectivity graph with typed traversals (walk, elevator, stairs, escalator)
- **Shortest-path routing** — Dijkstra with accessibility-aware mode (wheelchair routing avoids stairs)
- **Fingerprint positioning** — k-NN signal-space matching for BLE/WiFi indoor location estimation
- **Multi-floor support** — Floor changes via elevators, stairs, or escalators with navigation instructions

## Crates

| Crate | Description |
|-------|-------------|
| `interiora-core` | Core data types, graph, routing, and positioning engine |
| `interiora-server` | HTTP API: venue catalogue, floor GeoJSON, indoor routing, positioning |

## Quick Start

```rust
use interiora_core::{
    IndoorGraph, IndoorNode, NodeKind, TraversalType,
    PositioningEngine, Fingerprint,
    routing::{find_route, AccessibilityMode},
    floor_plan::Point2D,
};

// Build a graph
let mut graph = IndoorGraph::new();
let entrance = graph.add_node(IndoorNode::new(Point2D::new(0.0, 0.0), 0, NodeKind::Entrance));
let shop = graph.add_node(IndoorNode::new(Point2D::new(20.0, 5.0), 0, NodeKind::Waypoint));
graph.add_edge(entrance, shop, TraversalType::Walk);

// Find a route
let route = find_route(&graph, entrance, shop, AccessibilityMode::Default).unwrap();
println!("Distance: {:.1}m, ETA: {:.0}s", route.total_distance, route.estimated_time_s);
```

## License

AGPL-3.0-or-later, see [LICENSE](LICENSE).

Copyright (C) 2026 Grok Image Compression Inc.
