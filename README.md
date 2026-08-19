# Interiora

[![CI](https://github.com/GeoLang/interiora/actions/workflows/ci.yml/badge.svg)](https://github.com/GeoLang/interiora/actions)
[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL--3.0-blue.svg)](LICENSE)

**Indoor mapping and navigation SDK for the GeoLang ecosystem.**

Interiora provides data structures and algorithms for indoor venue modelling, wayfinding, and fingerprint positioning over caller-supplied signal readings.

## Features

- **Venue modelling** — Venues, floors, units (rooms/shops/areas), openings, and amenities
- **Indoor graph** — Connectivity graph with typed traversals (walk, elevator, stairs, escalator)
- **Shortest-path routing** — Dijkstra with accessibility-aware mode. Routing filters on an
  edge's `accessible` flag, which `IndoorGraph::add_edge` always sets true, so avoiding
  stairs happens only over `interiora-server`, whose document loader clears the flag on
  stair and escalator edges as it builds the graph.
- **Fingerprint positioning** — k-NN matching over a caller-supplied map of signal
  identifier to RSSI. Scanning and any radio-specific model are the caller's; there is no
  BLE or WiFi code here.
- **Multi-floor support** — Floor changes via elevators, stairs, or escalators with navigation instructions

### Limits

- The reported `accuracy` in metres is the mean distance from the estimate to the k chosen
  fingerprints, so it measures how spread the survey points are, not a validated error
  bound. `confidence` is an unlabelled 0-1 curve over the same distances.
- Floor GeoJSON carries no rotation term, so a plan drawn on a rotated axis comes out
  rotated on the map.

## Crates

| Crate | Description |
|-------|-------------|
| `interiora-core` | Core data types, graph, routing, and positioning engine |
| `interiora-server` | HTTP API: venue catalogue, floor GeoJSON, indoor routing, positioning |

Every `/venues` route needs a platform JWT signed with `PLATFORM_JWT_SECRET`
(32+ bytes, shared with the other GeoLang services); uploads and deletes need
the `editor` or `admin` role. `/health` is open, and the server will not start
without the secret.

## Quick Start

```rust
use interiora_core::{
    IndoorGraph, IndoorNode,
    PositioningEngine, Fingerprint,
    graph::{NodeKind, TraversalType},
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
