//! End-to-end tests over the router, using the shipped demo venue.

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use tower::ServiceExt;

use interiora_core::floor_plan::Point2D;
use interiora_server::geo::to_lonlat;
use interiora_server::{AppState, router};

const DEMO: &str = include_str!("../../../examples/venue-demo.json");
const ANCHOR_LAT: f64 = 45.5019;
const ANCHOR_LON: f64 = -73.5674;

fn demo_doc() -> Value {
    serde_json::from_str(DEMO).expect("the shipped demo document must parse")
}

fn fresh() -> Router {
    router(AppState::new(None).unwrap())
}

async fn send(app: &Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).expect("every response body is JSON")
    };
    (status, body)
}

fn get(uri: &str) -> Request<Body> {
    Request::builder().uri(uri).body(Body::empty()).unwrap()
}

fn post(uri: &str, body: &Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(body).unwrap()))
        .unwrap()
}

fn delete(uri: &str) -> Request<Body> {
    Request::builder()
        .method("DELETE")
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

async fn upload_demo(app: &Router) -> String {
    let (status, body) = send(app, post("/venues", &demo_doc())).await;
    assert_eq!(status, StatusCode::OK, "upload failed: {body}");
    body["id"].as_str().unwrap().to_string()
}

/// A point in local floor metres, as a client would send it.
fn at(x: f64, y: f64, floor: i32) -> Value {
    let [lon, lat] = to_lonlat(ANCHOR_LAT, ANCHOR_LON, Point2D::new(x, y));
    json!({ "lon": lon, "lat": lat, "floor": floor })
}

#[tokio::test]
async fn health_reports_ok() {
    let (status, body) = send(&fresh(), get("/health")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn upload_then_list() {
    let app = fresh();
    let (status, body) = send(&app, get("/venues")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 0);

    let id = upload_demo(&app).await;

    let (_, body) = send(&app, get("/venues")).await;
    let listed = body.as_array().unwrap();
    assert_eq!(listed.len(), 1);
    let venue = &listed[0];
    assert_eq!(venue["id"], id);
    assert_eq!(venue["name"], "Meridian Centre");
    assert_eq!(venue["category"], "ShoppingMall");
    assert_eq!(venue["floor_count"], 2);
    assert_eq!(venue["floors"], json!([0, 1]));
    assert!((venue["lat"].as_f64().unwrap() - ANCHOR_LAT).abs() < 1e-9);
    assert!((venue["lon"].as_f64().unwrap() - ANCHOR_LON).abs() < 1e-9);
}

#[tokio::test]
async fn floor_geojson_carries_units_openings_and_amenities() {
    let app = fresh();
    let id = upload_demo(&app).await;

    let (status, body) = send(&app, get(&format!("/venues/{id}/floors/0/geojson"))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["type"], "FeatureCollection");

    let features = body["features"].as_array().unwrap();
    // ground floor: 5 units, 3 openings, 4 amenities
    assert_eq!(features.len(), 12);
    let count = |feature: &str| {
        features
            .iter()
            .filter(|f| f["properties"]["feature"] == feature)
            .count()
    };
    assert_eq!(count("unit"), 5);
    assert_eq!(count("opening"), 3);
    assert_eq!(count("amenity"), 4);

    let lobby = features
        .iter()
        .find(|f| f["properties"]["name"] == "Lobby")
        .expect("the ground floor has a Lobby unit");
    assert_eq!(lobby["properties"]["category"], "Lobby");
    assert_eq!(lobby["properties"]["level"], 0);
    assert_eq!(lobby["properties"]["level_name"], "Ground");
    assert_eq!(lobby["geometry"]["type"], "Polygon");

    let ring = lobby["geometry"]["coordinates"][0].as_array().unwrap();
    // four corners, closed by repeating the first
    assert_eq!(ring.len(), 5);
    assert_eq!(ring[0], ring[4]);

    // the lobby's origin corner sits on the venue anchor, and the far corner
    // (20 m east, 12 m north) is a fraction of a degree away
    let corner = |i: usize| (ring[i][0].as_f64().unwrap(), ring[i][1].as_f64().unwrap());
    let (lon0, lat0) = corner(0);
    assert!((lon0 - ANCHOR_LON).abs() < 1e-9 && (lat0 - ANCHOR_LAT).abs() < 1e-9);
    let (lon2, lat2) = corner(2);
    assert!(lon2 > ANCHOR_LON && lat2 > ANCHOR_LAT);
    assert!((lat2 - ANCHOR_LAT - 12.0 / 111_320.0).abs() < 1e-9);

    // openings carry accessibility, amenities do not
    let opening = features
        .iter()
        .find(|f| f["properties"]["feature"] == "opening")
        .unwrap();
    assert_eq!(opening["geometry"]["type"], "Point");
    assert_eq!(opening["properties"]["accessible"], true);
    let amenity = features
        .iter()
        .find(|f| f["properties"]["name"] == "Information")
        .unwrap();
    assert_eq!(amenity["properties"]["category"], "InfoDesk");
    assert!(amenity["properties"]["accessible"].is_null());
}

#[tokio::test]
async fn geojson_404s_for_unknown_venue_and_floor() {
    let app = fresh();
    let id = upload_demo(&app).await;

    let (status, body) = send(&app, get(&format!("/venues/{id}/floors/7/geojson"))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body["error"].as_str().unwrap().contains("floor 7"));

    let other = uuid::Uuid::new_v4();
    let (status, _) = send(&app, get(&format!("/venues/{other}/floors/0/geojson"))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn default_route_takes_the_stairs() {
    let app = fresh();
    let id = upload_demo(&app).await;

    let request = json!({
        "from": at(10.0, 0.5, 0),
        "to": at(29.0, 10.0, 1),
    });
    let (status, body) = send(&app, post(&format!("/venues/{id}/route"), &request)).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    assert_eq!(body["geometry"]["type"], "LineString");
    let coords = body["geometry"]["coordinates"].as_array().unwrap();
    let floors = body["floors"].as_array().unwrap();
    assert_eq!(coords.len(), floors.len(), "one floor per vertex");
    assert!(coords.len() > 2);
    // starts on the ground floor, ends upstairs
    assert_eq!(floors[0], 0);
    assert_eq!(floors[floors.len() - 1], 1);

    let instructions: Vec<&str> = body["instructions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i.as_str().unwrap())
        .collect();
    assert_eq!(instructions.len(), coords.len() - 1);
    assert!(
        instructions.iter().any(|i| i.contains("Take stairs")),
        "the shorter path is the stairwell: {instructions:?}"
    );

    let distance = body["total_distance"].as_f64().unwrap();
    assert!(distance > 0.0);
    // 1.2 m/s walking speed
    let time = body["estimated_time_s"].as_f64().unwrap();
    assert!((time - distance / 1.2).abs() < 1e-6);
}

#[tokio::test]
async fn accessible_route_takes_the_elevator() {
    let app = fresh();
    let id = upload_demo(&app).await;

    let body_for = |mode: &str| {
        json!({
            "from": at(10.0, 0.5, 0),
            "to": at(29.0, 10.0, 1),
            "mode": mode,
        })
    };

    let (_, default) = send(
        &app,
        post(&format!("/venues/{id}/route"), &body_for("default")),
    )
    .await;
    let (status, accessible) = send(
        &app,
        post(&format!("/venues/{id}/route"), &body_for("accessible")),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{accessible}");

    let instructions: Vec<&str> = accessible["instructions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i.as_str().unwrap())
        .collect();
    assert!(
        instructions.iter().any(|i| i.contains("Take elevator")),
        "{instructions:?}"
    );
    assert!(
        !instructions.iter().any(|i| i.contains("Take stairs")),
        "a wheelchair route must not use the stairwell: {instructions:?}"
    );
    // the elevator is at the far west end, so avoiding stairs costs distance
    assert!(
        accessible["total_distance"].as_f64().unwrap()
            > default["total_distance"].as_f64().unwrap()
    );
}

#[tokio::test]
async fn route_422s_when_the_floor_has_no_nodes() {
    let app = fresh();
    let id = upload_demo(&app).await;

    let request = json!({
        "from": at(10.0, 0.5, 0),
        "to": at(29.0, 10.0, 4),
    });
    let (status, body) = send(&app, post(&format!("/venues/{id}/route"), &request)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        body["error"].as_str().unwrap().contains("floor 4"),
        "{body}"
    );
}

#[tokio::test]
async fn route_422s_when_the_venue_has_no_graph() {
    let app = fresh();
    let mut doc = demo_doc();
    doc["graph"] = Value::Null;
    let (status, body) = send(&app, post("/venues", &doc)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let id = body["id"].as_str().unwrap().to_string();

    let request = json!({ "from": at(10.0, 0.5, 0), "to": at(29.0, 10.0, 1) });
    let (status, body) = send(&app, post(&format!("/venues/{id}/route"), &request)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        body["error"]
            .as_str()
            .unwrap()
            .contains("no navigation graph"),
        "{body}"
    );
}

#[tokio::test]
async fn upload_422s_on_a_dangling_edge() {
    let app = fresh();
    let mut doc = demo_doc();
    doc["graph"]["edges"]
        .as_array_mut()
        .unwrap()
        .push(json!([0, 99, "Walk"]));
    let (status, body) = send(&app, post("/venues", &doc)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        body["error"].as_str().unwrap().contains("node 99"),
        "{body}"
    );
}

#[tokio::test]
async fn position_estimates_the_ground_floor() {
    let app = fresh();
    let id = upload_demo(&app).await;

    let request = json!({
        "signals": {
            "beacon-lobby": -47.0,
            "beacon-east": -70.0,
            "beacon-upper": -84.0,
        }
    });
    let (status, body) = send(&app, post(&format!("/venues/{id}/position"), &request)).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    // these readings are closest to the lobby fingerprint at (10, 6) on floor 0
    assert_eq!(body["floor_ordinal"], 0);
    let x = body["position"]["x"].as_f64().unwrap();
    let y = body["position"]["y"].as_f64().unwrap();
    assert!(x < 20.0 && y < 12.0, "estimate landed at ({x}, {y})");
    assert!(body["confidence"].as_f64().unwrap() > 0.0);

    // the same point, projected, so a map client can drop a marker
    let [lon, lat] = to_lonlat(ANCHOR_LAT, ANCHOR_LON, Point2D::new(x, y));
    assert!((body["lon"].as_f64().unwrap() - lon).abs() < 1e-12);
    assert!((body["lat"].as_f64().unwrap() - lat).abs() < 1e-12);
}

#[tokio::test]
async fn position_404s_without_fingerprints() {
    let app = fresh();
    let mut doc = demo_doc();
    doc["fingerprints"] = json!([]);
    let (_, body) = send(&app, post("/venues", &doc)).await;
    let id = body["id"].as_str().unwrap().to_string();

    let request = json!({ "signals": { "beacon-lobby": -47.0 } });
    let (status, body) = send(&app, post(&format!("/venues/{id}/position"), &request)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(
        body["error"].as_str().unwrap().contains("no fingerprints"),
        "{body}"
    );
}

#[tokio::test]
async fn delete_removes_the_venue() {
    let app = fresh();
    let id = upload_demo(&app).await;

    let (status, _) = send(&app, delete(&format!("/venues/{id}"))).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (_, body) = send(&app, get("/venues")).await;
    assert_eq!(body.as_array().unwrap().len(), 0);

    let (status, _) = send(&app, get(&format!("/venues/{id}/floors/0/geojson"))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _) = send(&app, delete(&format!("/venues/{id}"))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_data_dir_survives_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let app = router(AppState::new(Some(dir.path().to_path_buf())).unwrap());
    let id = upload_demo(&app).await;
    assert!(dir.path().join(format!("{id}.json")).exists());

    // a second server over the same directory sees the venue and can route on it
    let restarted = router(AppState::new(Some(dir.path().to_path_buf())).unwrap());
    let (_, body) = send(&restarted, get("/venues")).await;
    assert_eq!(body.as_array().unwrap().len(), 1);
    assert_eq!(body[0]["id"], id);

    let request = json!({ "from": at(10.0, 0.5, 0), "to": at(29.0, 10.0, 1) });
    let (status, _) = send(&restarted, post(&format!("/venues/{id}/route"), &request)).await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = send(&restarted, delete(&format!("/venues/{id}"))).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(!dir.path().join(format!("{id}.json")).exists());
}
