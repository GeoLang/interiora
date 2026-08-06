//! Local floor metres to geographic coordinates.
//!
//! Floors are placed on a tangent plane at the venue anchor: `+x` is east,
//! `+y` is north, and there is no rotation term, so a floor plan drawn on a
//! rotated axis will come out rotated. Accurate to centimetres over a
//! building-sized extent, meaningless beyond a few kilometres.

use interiora_core::floor_plan::Point2D;

/// Metres per degree of latitude, spherical earth.
const METRES_PER_DEGREE: f64 = 111_320.0;

/// Project a local floor point to `[lon, lat]`.
pub fn to_lonlat(anchor_lat: f64, anchor_lon: f64, point: Point2D) -> [f64; 2] {
    let lat = anchor_lat + point.y / METRES_PER_DEGREE;
    let lon = anchor_lon + point.x / (METRES_PER_DEGREE * anchor_lat.to_radians().cos());
    [lon, lat]
}

/// Project `lon`/`lat` back to a local floor point.
pub fn to_local(anchor_lat: f64, anchor_lon: f64, lon: f64, lat: f64) -> Point2D {
    Point2D::new(
        (lon - anchor_lon) * METRES_PER_DEGREE * anchor_lat.to_radians().cos(),
        (lat - anchor_lat) * METRES_PER_DEGREE,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const LAT: f64 = 45.5019;
    const LON: f64 = -73.5674;

    #[test]
    fn anchor_maps_to_origin() {
        let [lon, lat] = to_lonlat(LAT, LON, Point2D::new(0.0, 0.0));
        assert!((lon - LON).abs() < 1e-12);
        assert!((lat - LAT).abs() < 1e-12);
    }

    #[test]
    fn plus_y_is_north_and_plus_x_is_east() {
        let [lon, lat] = to_lonlat(LAT, LON, Point2D::new(100.0, 100.0));
        assert!(lat > LAT, "+y must increase latitude");
        assert!(lon > LON, "+x must increase longitude");
        // 100 m north is 100/111320 degrees of latitude
        assert!((lat - LAT - 100.0 / 111_320.0).abs() < 1e-12);
        // a degree of longitude is shorter than a degree of latitude here, so
        // the same 100 m spans more degrees east than north
        assert!(lon - LON > lat - LAT);
    }

    #[test]
    fn round_trips_to_local() {
        let point = Point2D::new(37.0, -12.5);
        let [lon, lat] = to_lonlat(LAT, LON, point);
        let back = to_local(LAT, LON, lon, lat);
        assert!((back.x - point.x).abs() < 1e-6, "x drifted: {}", back.x);
        assert!((back.y - point.y).abs() < 1e-6, "y drifted: {}", back.y);
    }

    #[test]
    fn metre_scale_is_realistic() {
        // a 40 m wide building must not span more than a thousandth of a degree
        let [lon, _] = to_lonlat(LAT, LON, Point2D::new(40.0, 0.0));
        let span = lon - LON;
        assert!(span > 0.0004 && span < 0.001, "40 m spans {span} degrees");
    }
}
