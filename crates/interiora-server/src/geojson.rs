//! GeoJSON response types. Coordinates are always `[lon, lat]`.

use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct FeatureCollection {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub features: Vec<Feature>,
}

impl FeatureCollection {
    pub fn new(features: Vec<Feature>) -> Self {
        Self {
            kind: "FeatureCollection",
            features,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Feature {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub geometry: Geometry,
    pub properties: FeatureProperties,
}

impl Feature {
    pub fn new(geometry: Geometry, properties: FeatureProperties) -> Self {
        Self {
            kind: "Feature",
            geometry,
            properties,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum Geometry {
    Point { coordinates: [f64; 2] },
    LineString { coordinates: Vec<[f64; 2]> },
    Polygon { coordinates: Vec<Vec<[f64; 2]>> },
}

/// What a client needs to label and filter a feature.
#[derive(Debug, Serialize)]
pub struct FeatureProperties {
    /// `unit`, `opening`, or `amenity`.
    pub feature: &'static str,
    pub id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub category: String,
    pub level: i32,
    pub level_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessible: Option<bool>,
}
