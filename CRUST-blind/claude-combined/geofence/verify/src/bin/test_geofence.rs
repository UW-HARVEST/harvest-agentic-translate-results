#![allow(unused_imports)]
use geofence::geofence::{is_position_in_geofence, Point};

// =========================================================================
// is_position_in_geofence tests
// =========================================================================

#[test]
fn test_geofence1_inside() {
    let p = Point { x: 77.64664, y: 12.90383 };
    let poly = [
        Point { x: 77.64354, y: 12.90448 },
        Point { x: 77.64671, y: 12.90573 },
        Point { x: 77.64815, y: 12.90367 },
        Point { x: 77.64703, y: 12.90215 },
    ];
    assert_eq!(is_position_in_geofence(&p, &poly), true);
}

#[test]
fn test_geofence2_outside() {
    let p = Point { x: 77.6183653, y: 12.9086164 };
    let poly = [
        Point { x: 77.6084948, y: 12.9115446 },
        Point { x: 77.6163054, y: 12.9114191 },
        Point { x: 77.6179361, y: 12.907236 },
        Point { x: 77.6226139, y: 12.9099132 },
        Point { x: 77.6197386, y: 12.9190739 },
        Point { x: 77.6300812, y: 12.9186138 },
        Point { x: 77.6333857, y: 12.9106243 },
        Point { x: 77.6314974, y: 12.9017561 },
        Point { x: 77.6213264, y: 12.9001665 },
        Point { x: 77.6100397, y: 12.9019652 },
    ];
    assert_eq!(is_position_in_geofence(&p, &poly), false);
}

#[test]
fn test_geofence3_inside() {
    let p = Point { x: 77.6168203, y: 12.9081563 };
    let poly = [
        Point { x: 77.6084948, y: 12.9115446 },
        Point { x: 77.6163054, y: 12.9114191 },
        Point { x: 77.6179361, y: 12.907236 },
        Point { x: 77.6226139, y: 12.9099132 },
        Point { x: 77.6197386, y: 12.9190739 },
        Point { x: 77.6300812, y: 12.9186138 },
        Point { x: 77.6333857, y: 12.9106243 },
        Point { x: 77.6314974, y: 12.9017561 },
        Point { x: 77.6213264, y: 12.9001665 },
        Point { x: 77.6100397, y: 12.9019652 },
    ];
    assert_eq!(is_position_in_geofence(&p, &poly), true);
}

#[test]
fn test_geofence_square_origin() {
    let origin = Point { x: 0.0, y: 0.0 };
    let poly = [
        Point { x: 1.0, y: 1.0 },
        Point { x: 1.0, y: -1.0 },
        Point { x: -1.0, y: -1.0 },
        Point { x: -1.0, y: 1.0 },
    ];
    assert_eq!(is_position_in_geofence(&origin, &poly), true);
}

#[test]
fn test_geofence_square_far() {
    let far = Point { x: 2.0, y: 0.0 };
    let poly = [
        Point { x: 1.0, y: 1.0 },
        Point { x: 1.0, y: -1.0 },
        Point { x: -1.0, y: -1.0 },
        Point { x: -1.0, y: 1.0 },
    ];
    assert_eq!(is_position_in_geofence(&far, &poly), false);
}

#[test]
fn test_geofence_square_ne_inside() {
    let p = Point { x: 0.5, y: 0.5 };
    let poly = [
        Point { x: 1.0, y: 1.0 },
        Point { x: 1.0, y: -1.0 },
        Point { x: -1.0, y: -1.0 },
        Point { x: -1.0, y: 1.0 },
    ];
    assert_eq!(is_position_in_geofence(&p, &poly), true);
}

#[test]
fn test_geofence_two_vertices_returns_false() {
    let origin = Point { x: 0.0, y: 0.0 };
    let poly = [
        Point { x: 1.0, y: 1.0 },
        Point { x: -1.0, y: -1.0 },
    ];
    // C: returns false because number_of_vertices < 3
    assert_eq!(is_position_in_geofence(&origin, &poly), false);
}

#[test]
fn test_geofence_one_vertex_returns_false() {
    let origin = Point { x: 0.0, y: 0.0 };
    let poly = [Point { x: 0.0, y: 0.0 }];
    assert_eq!(is_position_in_geofence(&origin, &poly), false);
}

#[test]
fn test_geofence_empty_returns_false() {
    let origin = Point { x: 0.0, y: 0.0 };
    let poly: [Point; 0] = [];
    assert_eq!(is_position_in_geofence(&origin, &poly), false);
}

#[test]
fn test_geofence_diamond_origin() {
    let origin = Point { x: 0.0, y: 0.0 };
    let poly = [
        Point { x: 0.0, y: 1.0 },
        Point { x: 1.0, y: 0.0 },
        Point { x: 0.0, y: -1.0 },
        Point { x: -1.0, y: 0.0 },
    ];
    assert_eq!(is_position_in_geofence(&origin, &poly), true);
}

#[test]
fn test_geofence_diamond_north() {
    let p = Point { x: 0.0, y: 0.5 };
    let poly = [
        Point { x: 0.0, y: 1.0 },
        Point { x: 1.0, y: 0.0 },
        Point { x: 0.0, y: -1.0 },
        Point { x: -1.0, y: 0.0 },
    ];
    assert_eq!(is_position_in_geofence(&p, &poly), true);
}

#[test]
fn test_geofence_diamond_outside() {
    let p = Point { x: 5.0, y: 0.5 };
    let poly = [
        Point { x: 0.0, y: 1.0 },
        Point { x: 1.0, y: 0.0 },
        Point { x: 0.0, y: -1.0 },
        Point { x: -1.0, y: 0.0 },
    ];
    assert_eq!(is_position_in_geofence(&p, &poly), false);
}

#[test]
fn test_geofence_on_vertex() {
    // Point at the (0,0) vertex of the polygon
    let p = Point { x: 0.0, y: 0.0 };
    let poly = [
        Point { x: 0.0, y: 0.0 },
        Point { x: 1.0, y: 0.0 },
        Point { x: 1.0, y: 1.0 },
        Point { x: 0.0, y: 1.0 },
    ];
    // C returns true for this case
    assert_eq!(is_position_in_geofence(&p, &poly), true);
}

#[test]
fn test_geofence_on_edge() {
    let p = Point { x: 0.5, y: 0.0 };
    let poly = [
        Point { x: 0.0, y: 0.0 },
        Point { x: 1.0, y: 0.0 },
        Point { x: 1.0, y: 1.0 },
        Point { x: 0.0, y: 1.0 },
    ];
    // C returns true for this case
    assert_eq!(is_position_in_geofence(&p, &poly), true);
}

#[test]
fn test_geofence_concave_star_origin() {
    let origin = Point { x: 0.0, y: 0.0 };
    let poly = [
        Point { x: 0.0, y: 1.0 },
        Point { x: 1.0, y: 0.0 },
        Point { x: 0.0, y: -1.0 },
        Point { x: -1.0, y: 0.0 },
        Point { x: 0.0, y: 0.0 },
    ];
    assert_eq!(is_position_in_geofence(&origin, &poly), true);
}

#[test]
fn test_geofence_concave_star_north() {
    let p = Point { x: 0.0, y: 0.5 };
    let poly = [
        Point { x: 0.0, y: 1.0 },
        Point { x: 1.0, y: 0.0 },
        Point { x: 0.0, y: -1.0 },
        Point { x: -1.0, y: 0.0 },
        Point { x: 0.0, y: 0.0 },
    ];
    assert_eq!(is_position_in_geofence(&p, &poly), true);
}

#[test]
fn test_geofence_concave_star_east() {
    let p = Point { x: 0.5, y: 0.0 };
    let poly = [
        Point { x: 0.0, y: 1.0 },
        Point { x: 1.0, y: 0.0 },
        Point { x: 0.0, y: -1.0 },
        Point { x: -1.0, y: 0.0 },
        Point { x: 0.0, y: 0.0 },
    ];
    assert_eq!(is_position_in_geofence(&p, &poly), true);
}

#[test]
fn test_geofence_concave_star_outside() {
    let p = Point { x: -1.0, y: 1.0 };
    let poly = [
        Point { x: 0.0, y: 1.0 },
        Point { x: 1.0, y: 0.0 },
        Point { x: 0.0, y: -1.0 },
        Point { x: -1.0, y: 0.0 },
        Point { x: 0.0, y: 0.0 },
    ];
    assert_eq!(is_position_in_geofence(&p, &poly), false);
}

fn main() {}
