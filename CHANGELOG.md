# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- 2026-11-05: `require_auth` calls `platform_auth::verify_token` instead of
  `decode_token`. A token with a `role` and a `token_use`, `agora_use` or
  `geolang_use` claim, null values included, used to pass and now gets 401. So
  does a token with an empty `sub` or a non-string `name`. `auth::Claims` is
  deleted.
- 2026-11-04: `interiora-server` validates platform tokens and parses the
  bearer header with the shared `platform-auth` crate. `PLATFORM_JWT_SECRET`,
  the route rules, the role policy and the 32-byte secret check are unchanged.
  A token with a non-string `aud` used to pass and now gets 401. `jsonwebtoken`
  moves from 9 to 11, and its crypto backend moves from `ring` to `aws-lc-sys`.
  Both need a C compiler to build.
- 2026-11-04: CI and the release workflow call the reusable GeoLang workflows.
  Release archive names and the image name are unchanged.
- 2026-09-23: the README and docs page document how to run the server, the
  image, the roles and the route output.
- 2026-09-17: non-major Cargo updates and major GitHub Actions updates.

## [0.2.0] - 2026-09-16

First tagged release, at commit `2995a71`. The workspace version in `Cargo.toml`
stayed at 0.1.0, so `/health` in the binaries and the image from this tag
reports `0.1.0`.

### Added
- 2026-09-16: a `v*` tag publishes `ghcr.io/geolang/interiora` and attaches
  `interiora-server` archives for Linux and macOS, x86_64 and aarch64, to the
  GitHub release.
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
- 2026-05-30: `interiora-core`: venue and floor plan models, navigation graph,
  routing and fingerprint positioning.
