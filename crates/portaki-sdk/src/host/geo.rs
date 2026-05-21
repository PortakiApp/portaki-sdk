//! `host::geo` — deterministic geo math (no network).

use serde::{Deserialize, Serialize};

/// WGS84 point.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Point {
    /// Latitude.
    pub lat: f64,
    /// Longitude.
    pub lng: f64,
}

/// Axis-aligned bounding box.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct BBox {
    /// Minimum latitude.
    pub min_lat: f64,
    /// Minimum longitude.
    pub min_lng: f64,
    /// Maximum latitude.
    pub max_lat: f64,
    /// Maximum longitude.
    pub max_lng: f64,
}

/// Haversine distance in meters between two points.
pub fn distance(p1: Point, p2: Point) -> f64 {
    const EARTH_RADIUS_M: f64 = 6_371_000.0;
    let lat1 = p1.lat.to_radians();
    let lat2 = p2.lat.to_radians();
    let dlat = (p2.lat - p1.lat).to_radians();
    let dlng = (p2.lng - p1.lng).to_radians();
    let a = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlng / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    EARTH_RADIUS_M * c
}

/// Approximate bounding box for a circle (naive degree conversion).
pub fn bounding_box(center: Point, radius_km: f64) -> BBox {
    let delta_lat = radius_km / 111.0;
    let delta_lng = radius_km / (111.0 * center.lat.to_radians().cos().abs().max(0.01));
    BBox {
        min_lat: center.lat - delta_lat,
        min_lng: center.lng - delta_lng,
        max_lat: center.lat + delta_lat,
        max_lng: center.lng + delta_lng,
    }
}

/// Ray-casting point-in-polygon test.
pub fn point_in_polygon(point: Point, polygon: &[Point]) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = polygon.len() - 1;
    for i in 0..polygon.len() {
        let pi = polygon[i];
        let pj = polygon[j];
        let intersects = ((pi.lng > point.lng) != (pj.lng > point.lng))
            && (point.lat
                < (pj.lat - pi.lat) * (point.lng - pi.lng) / (pj.lng - pi.lng + f64::EPSILON)
                    + pi.lat);
        if intersects {
            inside = !inside;
        }
        j = i;
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::{distance, Point};

    #[test]
    fn distance_is_zero_for_same_point() {
        let p = Point {
            lat: 43.55,
            lng: 7.01,
        };
        assert!(distance(p, p) < 0.01);
    }
}
