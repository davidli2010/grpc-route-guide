// Copyright 2020 David Li
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Common utilities for the RouteGuide.

use crate::route_guide::{Feature, FeatureDatabase, Point, Rectangle};
use std::path::PathBuf;

const COORD_FACTOR: f64 = 1e7;

/// Gets the latitude for the given point.
#[inline]
fn get_latitude(location: &Point) -> f64 {
    location.get_latitude() as f64 / COORD_FACTOR
}

/// Gets the longitude for the given point.
#[inline]
fn get_longitude(location: &Point) -> f64 {
    location.get_longitude() as f64 / COORD_FACTOR
}

/// Gets the default features file.
#[inline]
fn get_default_features_file() -> PathBuf {
    let dir = env!("CARGO_MANIFEST_DIR");
    let path = PathBuf::from(dir).join("data/route_guide_db.json");
    assert!(path.exists());
    path
}

/// Parses the JSON input file containing the list of features.
#[inline]
pub fn load_database() -> FeatureDatabase {
    let file = get_default_features_file();
    let file = std::fs::File::open(file).unwrap();
    serde_json::from_reader(file).unwrap()
}

/// Indicates whether the given feature exists (i.e. has a valid name).
#[inline]
pub fn exists(feature: &Feature) -> bool {
    !feature.get_name().is_empty()
}

/// Indicates whether the given two points are equal.
#[inline]
pub fn point_eq(p1: &Point, p2: &Point) -> bool {
    if p1.get_latitude() == p2.get_latitude() && p1.get_longitude() == p2.get_longitude() {
        true
    } else {
        false
    }
}

/// Checks if the given point is in features.
#[inline]
pub fn check_feature(features: &[Feature], location: &Point) -> Option<Feature> {
    features.iter().find_map(|f| {
        if point_eq(f.get_location(), location) {
            Some(f.clone())
        } else {
            None
        }
    })
}

/// Indicates whether the given point is in the range of the given rectangle.
#[inline]
pub fn in_range(point: &Point, rect: &Rectangle) -> bool {
    use std::cmp::{max, min};

    let lo = rect.get_lo();
    let hi = rect.get_hi();

    let left = min(lo.get_longitude(), hi.get_longitude());
    let right = max(lo.get_longitude(), hi.get_longitude());
    let top = max(lo.get_latitude(), hi.get_latitude());
    let bottom = min(lo.get_latitude(), hi.get_latitude());

    let lat = point.get_latitude();
    let lon = point.get_longitude();

    if lon >= left && lon <= right && lat >= bottom && lat <= top {
        true
    } else {
        false
    }
}

/// Calculates distance between two points.
#[inline]
pub fn calc_distance(start: &Point, end: &Point) -> i32 {
    const R: i32 = 6371000; // earth radius in meters

    let lat1 = get_latitude(start).to_radians();
    let lat2 = get_latitude(end).to_radians();
    let lon1 = get_longitude(start).to_radians();
    let lon2 = get_longitude(end).to_radians();

    let delta_lat = lat2 - lat1;
    let delta_lon = lon2 - lon1;

    let a = (delta_lat / 2f64).sin() * (delta_lat / 2f64).sin()
        + lat1.cos() * lat2.cos() * (delta_lon / 2f64).sin() * (delta_lon / 2f64).sin();
    let c = 2f64 * a.sqrt().atan2((1f64 - a).sqrt());
    let distance = R as f64 * c;
    distance as i32
}

/// Format point to `String`.
#[inline]
pub fn format_point(point: &Point) -> String {
    format!("({}, {})", get_latitude(point), get_longitude(point))
}
