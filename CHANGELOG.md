# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- 2026-08-06: `interiora-server` crate, the first HTTP layer over the core.
  Upload an indoor map document (venue, navigation graph, fingerprints), list
  venues, read a floor as GeoJSON in lon/lat, route between two geographic
  points with a wheelchair-accessible mode, and estimate a position from BLE
  or WiFi signals. Venues are held in memory and mirrored to
  `INTERIORA_DATA_DIR` when it is set.
- 2026-08-06: two-stage Dockerfile for `interiora-server`, listening on `PORT`
  (3000) and persisting venues to the `/data` volume.
- 2026-08-06: `interiora-server` gates every `/venues` route behind a platform
  JWT (HS256, `sub`/`exp`/`role`) signed with `PLATFORM_JWT_SECRET`. Reads take
  any known role, uploads and deletes take `editor` or `admin`, an unknown role
  is refused. `/health` stays open, and the server refuses to start without a
  32-byte-or-longer secret.
- Initial release
