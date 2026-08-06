//! # interiora-server
//!
//! HTTP API over interiora-core. Upload an [`doc::IndoorMapDoc`], then read
//! floors back as GeoJSON, route between two points inside the venue, and
//! estimate a position from radio signals.
//!
//! Venues live in memory. Set `INTERIORA_DATA_DIR` to also load and store them
//! as JSON documents in that directory.

pub mod doc;
pub mod geo;
pub mod geojson;
pub mod navigation;
pub mod store;
pub mod venues;

use std::path::PathBuf;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Serialize;
use tower_http::cors::CorsLayer;
use uuid::Uuid;

pub use store::AppState;

/// Build the router, restoring any documents in `INTERIORA_DATA_DIR`.
pub fn create_router() -> std::io::Result<Router> {
    let data_dir = std::env::var_os("INTERIORA_DATA_DIR").map(PathBuf::from);
    Ok(router(AppState::new(data_dir)?))
}

/// Build the router over an explicit state, which is how tests get an isolated
/// store.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/venues", get(venues::list).post(venues::upload))
        .route("/venues/{id}", axum::routing::delete(venues::delete))
        .route(
            "/venues/{id}/floors/{ordinal}/geojson",
            get(venues::floor_geojson),
        )
        .route("/venues/{id}/route", post(navigation::route))
        .route("/venues/{id}/position", post(navigation::position))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

/// Start the server on the given address.
pub async fn serve(bind: &str) -> std::io::Result<()> {
    let router = create_router()?;
    let listener = tokio::net::TcpListener::bind(bind).await?;
    axum::serve(listener, router).await
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok", "version": env!("CARGO_PKG_VERSION")}))
}

/// A failure the client can read: a status and a message, as JSON.
#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    message: String,
}

#[derive(Serialize)]
struct ApiErrorBody<'a> {
    error: &'a str,
}

impl ApiError {
    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }

    /// The request is well formed but the data cannot satisfy it.
    pub fn unprocessable(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ApiErrorBody {
                error: &self.message,
            }),
        )
            .into_response()
    }
}

fn missing_venue(id: Uuid) -> ApiError {
    ApiError::not_found(format!("venue {id} not found"))
}
