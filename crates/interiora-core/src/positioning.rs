//! Indoor positioning: k-nearest-neighbor location estimation over signal
//! strength readings the caller supplies. There is no BLE or WiFi acquisition here.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::floor_plan::Point2D;

/// An estimated indoor position.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndoorPosition {
    /// Position in local floor coordinates.
    pub position: Point2D,
    /// Floor ordinal.
    pub floor_ordinal: i32,
    /// Estimated accuracy in meters.
    pub accuracy: f64,
    /// Confidence (0.0–1.0).
    pub confidence: f64,
}

/// Signal strength readings recorded at a known position. The identifiers and
/// their dBm values come from the caller.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fingerprint {
    /// Known position where this fingerprint was recorded.
    pub position: Point2D,
    /// Floor ordinal.
    pub floor_ordinal: i32,
    /// Signal readings: beacon/AP identifier → RSSI (dBm).
    pub signals: HashMap<String, f64>,
}

/// Indoor positioning engine using fingerprint matching.
pub struct PositioningEngine {
    /// Fingerprint database (survey data).
    fingerprints: Vec<Fingerprint>,
    /// Number of nearest fingerprints to consider (k in k-NN).
    k: usize,
}

impl PositioningEngine {
    /// Create a new positioning engine.
    pub fn new() -> Self {
        Self {
            fingerprints: Vec::new(),
            k: 3,
        }
    }

    /// Set the k parameter for k-NN matching.
    pub fn set_k(&mut self, k: usize) {
        self.k = k.max(1);
    }

    /// Add a fingerprint to the database (from a site survey).
    pub fn add_fingerprint(&mut self, fingerprint: Fingerprint) {
        self.fingerprints.push(fingerprint);
    }

    /// Load multiple fingerprints (bulk survey import).
    pub fn load_fingerprints(&mut self, fingerprints: Vec<Fingerprint>) {
        self.fingerprints.extend(fingerprints);
    }

    /// Estimate position from current signal readings.
    ///
    /// Uses k-nearest-neighbor in signal space (Euclidean distance of RSSI vectors).
    pub fn estimate_position(&self, signals: &HashMap<String, f64>) -> Option<IndoorPosition> {
        if self.fingerprints.is_empty() {
            return None;
        }

        // Compute distance to each fingerprint in signal space
        let mut scored: Vec<(usize, f64)> = self
            .fingerprints
            .iter()
            .enumerate()
            .map(|(i, fp)| (i, signal_distance(signals, &fp.signals)))
            .collect();

        // Sort by distance (closest first)
        scored.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take k nearest
        let k = self.k.min(scored.len());
        let nearest = &scored[..k];

        // Weighted average position (inverse distance weighting)
        let mut total_weight = 0.0;
        let mut wx = 0.0;
        let mut wy = 0.0;
        let mut floor_votes: HashMap<i32, f64> = HashMap::new();

        for &(idx, dist) in nearest {
            let weight = if dist < 1e-10 { 1000.0 } else { 1.0 / dist };
            let fp = &self.fingerprints[idx];
            wx += fp.position.x * weight;
            wy += fp.position.y * weight;
            total_weight += weight;
            *floor_votes.entry(fp.floor_ordinal).or_default() += weight;
        }

        if total_weight < 1e-10 {
            return None;
        }

        let position = Point2D::new(wx / total_weight, wy / total_weight);
        let floor_ordinal = *floor_votes
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(floor, _)| floor)?;

        // Estimate accuracy from spread of nearest points
        let accuracy = nearest
            .iter()
            .map(|&(idx, _)| self.fingerprints[idx].position.distance_to(&position))
            .sum::<f64>()
            / k as f64;

        // Confidence based on how close the signal match is
        let avg_dist = nearest.iter().map(|&(_, d)| d).sum::<f64>() / k as f64;
        let confidence = (1.0 / (1.0 + avg_dist / 10.0)).clamp(0.0, 1.0);

        Some(IndoorPosition {
            position,
            floor_ordinal,
            accuracy,
            confidence,
        })
    }

    /// Number of fingerprints in the database.
    pub fn fingerprint_count(&self) -> usize {
        self.fingerprints.len()
    }
}

impl Default for PositioningEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Euclidean distance between two signal vectors.
/// Missing beacons in either vector are treated as -100 dBm (out of range).
fn signal_distance(a: &HashMap<String, f64>, b: &HashMap<String, f64>) -> f64 {
    let all_keys: std::collections::HashSet<&String> = a.keys().chain(b.keys()).collect();
    let mut sum_sq = 0.0;

    for key in all_keys {
        let va = a.get(key).copied().unwrap_or(-100.0);
        let vb = b.get(key).copied().unwrap_or(-100.0);
        sum_sq += (va - vb).powi(2);
    }

    sum_sq.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_engine() -> PositioningEngine {
        let mut engine = PositioningEngine::new();
        engine.add_fingerprint(Fingerprint {
            position: Point2D::new(0.0, 0.0),
            floor_ordinal: 0,
            signals: HashMap::from([
                ("beacon_a".to_string(), -50.0),
                ("beacon_b".to_string(), -70.0),
            ]),
        });
        engine.add_fingerprint(Fingerprint {
            position: Point2D::new(10.0, 0.0),
            floor_ordinal: 0,
            signals: HashMap::from([
                ("beacon_a".to_string(), -80.0),
                ("beacon_b".to_string(), -40.0),
            ]),
        });
        engine.add_fingerprint(Fingerprint {
            position: Point2D::new(5.0, 5.0),
            floor_ordinal: 0,
            signals: HashMap::from([
                ("beacon_a".to_string(), -65.0),
                ("beacon_b".to_string(), -55.0),
            ]),
        });
        engine
    }

    #[test]
    fn test_estimate_near_beacon_a() {
        let engine = make_engine();
        let signals = HashMap::from([
            ("beacon_a".to_string(), -52.0),
            ("beacon_b".to_string(), -68.0),
        ]);
        let pos = engine.estimate_position(&signals).unwrap();
        // Should be near (0, 0) since signals are close to first fingerprint
        assert!(pos.position.x < 3.0);
        assert!(pos.floor_ordinal == 0);
        assert!(pos.confidence > 0.0);
    }

    #[test]
    fn test_estimate_near_beacon_b() {
        let engine = make_engine();
        let signals = HashMap::from([
            ("beacon_a".to_string(), -78.0),
            ("beacon_b".to_string(), -42.0),
        ]);
        let pos = engine.estimate_position(&signals).unwrap();
        // Should be near (10, 0)
        assert!(pos.position.x > 7.0);
    }

    #[test]
    fn test_empty_engine() {
        let engine = PositioningEngine::new();
        let signals = HashMap::from([("beacon_a".to_string(), -50.0)]);
        assert!(engine.estimate_position(&signals).is_none());
    }

    #[test]
    fn test_signal_distance_identical() {
        let a = HashMap::from([("x".to_string(), -50.0)]);
        let b = HashMap::from([("x".to_string(), -50.0)]);
        assert!((signal_distance(&a, &b)).abs() < 1e-10);
    }
}
