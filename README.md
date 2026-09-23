# Interiora

[![CI](https://github.com/GeoLang/interiora/actions/workflows/ci.yml/badge.svg)](https://github.com/GeoLang/interiora/actions)
[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL--3.0-blue.svg)](LICENSE)

**Indoor mapping and navigation library and HTTP server for the GeoLang platform.**

Interiora models indoor venues, routes between points inside them, and estimates a position from signal readings the caller supplies. `interiora-core` is the Rust library and `interiora-server` serves it over HTTP.

## Features

- **Venue modelling**: venues, floors sorted by ordinal, units with a category such as room, shop or corridor, openings and amenities
- **Indoor graph**: nodes joined by two-way edges with a traversal type (walk, elevator, stairs, escalator), and nearest-node search per floor
- **Shortest-path routing**: Dijkstra, with one instruction per segment and a walk time at 1.2 m/s
- **Wheelchair routing**: `AccessibilityMode::Wheelchair` skips edges whose `accessible`
  flag is false. `IndoorGraph::add_edge` always sets it true, so a core caller clears it on
  `graph.edges` by hand. The server's document loader clears it on stair and escalator edges.
- **Fingerprint positioning**: k-NN matching (k = 3 by default) over a caller-supplied map
  of signal identifier to RSSI. Scanning and any radio model are the caller's. There is no
  BLE or WiFi code here.
- **Multi-floor routes**: floor changes over elevator, stair and escalator edges

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

Every `/venues` route needs a platform JWT signed HS256 with `PLATFORM_JWT_SECRET`
(32 bytes or more, shared with the other GeoLang services). The token's `role`
claim must be `viewer`, `editor` or `admin`, and uploads and deletes need
`editor` or `admin`. `/health` is open. The server will not start without the
secret.

## HTTP API

`interiora-server` takes one upload document per venue, an `IndoorMapDoc`
holding the venue, an optional navigation graph whose edges are index pairs
into its node list, and optional fingerprints. `examples/venue-demo.json` is a
complete document you can post as is.

| Route | Role | What it does |
|-------|------|--------------|
| `GET /health` | open | status and crate version |
| `GET /venues` | any known role | venue summaries sorted by name, with floor ordinals |
| `POST /venues` | editor or admin | store an upload document, returns its venue id |
| `DELETE /venues/{id}` | editor or admin | drop a venue |
| `GET /venues/{id}/floors/{ordinal}/geojson` | any known role | the floor as a GeoJSON FeatureCollection in lon/lat, with unit, opening, and amenity features |
| `POST /venues/{id}/route` | any known role | route between two `{lon, lat, floor}` points, `mode` of `default` or `accessible`. Returns a LineString, distance, walk time, instructions and the floor of each vertex |
| `POST /venues/{id}/position` | any known role | estimate a position from a `signals` map of identifier to RSSI. Returns local position, floor, accuracy, confidence and lon/lat |

Route endpoints snap to the nearest graph node on their floor. Local floor metres
are placed on a tangent plane at the venue anchor, `+x` east and `+y` north.

Environment:

- `PLATFORM_JWT_SECRET`: required, the shared HS256 secret.
- `PORT`: listen port, 3000 by default.
- `INTERIORA_DATA_DIR`: when set, venues are mirrored to that directory as JSON
  documents and reloaded at startup. Without it they are held in memory only.

Run it locally:

```bash
PLATFORM_JWT_SECRET="$(openssl rand -hex 32)" cargo run -p interiora-server
```

The Dockerfile builds `interiora-server`, exposes port 3000, and points
`INTERIORA_DATA_DIR` at the `/data` volume. A `v*` tag publishes the image as
`ghcr.io/geolang/interiora` and attaches `interiora-server` binaries for Linux
and macOS, x86_64 and aarch64, to the GitHub release.

## Quick Start

```rust
use interiora_core::{
    IndoorGraph, IndoorNode,
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
