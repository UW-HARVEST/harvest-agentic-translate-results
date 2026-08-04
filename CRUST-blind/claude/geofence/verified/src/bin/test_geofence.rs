#![allow(unused_imports)]

use geofence::geofence::{is_position_in_geofence, Point};

// === Square polygon (poly1 from C tests) ===

#[test]
fn test_geofence_origin_in_square() {
    let poly = [
        Point { x: 1.0, y: 1.0 },
        Point { x: 1.0, y: -1.0 },
        Point { x: -1.0, y: -1.0 },
        Point { x: -1.0, y: 1.0 },
    ];
    let p = Point { x: 0.0, y: 0.0 };
    let result = is_position_in_geofence(&p, &poly);
    assert_eq!(result, true);
}

#[test]
fn test_geofence_p2_in_square() {
    let poly = [
        Point { x: 1.0, y: 1.0 },
        Point { x: 1.0, y: -1.0 },
        Point { x: -1.0, y: -1.0 },
        Point { x: -1.0, y: 1.0 },
    ];
    let p = Point { x: -1.0, y: 0.0 };
    let result = is_position_in_geofence(&p, &poly);
    assert_eq!(result, true);
}

// === Diamond polygon (poly2 from C tests) ===

#[test]
fn test_geofence_p3_in_diamond() {
    let poly = [
        Point { x: 0.0, y: 1.0 },
        Point { x: 1.0, y: 0.0 },
        Point { x: 0.0, y: -1.0 },
        Point { x: -1.0, y: 0.0 },
    ];
    let p = Point { x: 0.0, y: 0.5 };
    let result = is_position_in_geofence(&p, &poly);
    assert_eq!(result, true);
}

#[test]
fn test_geofence_p4_outside_diamond() {
    let poly = [
        Point { x: 0.0, y: 1.0 },
        Point { x: 1.0, y: 0.0 },
        Point { x: 0.0, y: -1.0 },
        Point { x: -1.0, y: 0.0 },
    ];
    let p = Point { x: 5.0, y: 0.5 };
    let result = is_position_in_geofence(&p, &poly);
    assert_eq!(result, false);
}

#[test]
fn test_geofence_origin_in_diamond() {
    let poly = [
        Point { x: 0.0, y: 1.0 },
        Point { x: 1.0, y: 0.0 },
        Point { x: 0.0, y: -1.0 },
        Point { x: -1.0, y: 0.0 },
    ];
    let p = Point { x: 0.0, y: 0.0 };
    let result = is_position_in_geofence(&p, &poly);
    assert_eq!(result, true);
}

#[test]
fn test_geofence_p2_in_diamond() {
    let poly = [
        Point { x: 0.0, y: 1.0 },
        Point { x: 1.0, y: 0.0 },
        Point { x: 0.0, y: -1.0 },
        Point { x: -1.0, y: 0.0 },
    ];
    let p = Point { x: -1.0, y: 0.0 };
    let result = is_position_in_geofence(&p, &poly);
    assert_eq!(result, true);
}

// === Diamond with center vertex (poly3 from C tests) ===

#[test]
fn test_geofence_p3_in_diamond_with_center() {
    let poly = [
        Point { x: 0.0, y: 1.0 },
        Point { x: 1.0, y: 0.0 },
        Point { x: 0.0, y: -1.0 },
        Point { x: -1.0, y: 0.0 },
        Point { x: 0.0, y: 0.0 },
    ];
    let p = Point { x: 0.0, y: 0.5 };
    let result = is_position_in_geofence(&p, &poly);
    assert_eq!(result, true);
}

#[test]
fn test_geofence_origin_in_diamond_with_center() {
    let poly = [
        Point { x: 0.0, y: 1.0 },
        Point { x: 1.0, y: 0.0 },
        Point { x: 0.0, y: -1.0 },
        Point { x: -1.0, y: 0.0 },
        Point { x: 0.0, y: 0.0 },
    ];
    let p = Point { x: 0.0, y: 0.0 };
    let result = is_position_in_geofence(&p, &poly);
    assert_eq!(result, true);
}

#[test]
fn test_geofence_p5_in_diamond_with_center() {
    let poly = [
        Point { x: 0.0, y: 1.0 },
        Point { x: 1.0, y: 0.0 },
        Point { x: 0.0, y: -1.0 },
        Point { x: -1.0, y: 0.0 },
        Point { x: 0.0, y: 0.0 },
    ];
    let p = Point { x: 0.5, y: 0.0 };
    let result = is_position_in_geofence(&p, &poly);
    assert_eq!(result, true);
}

#[test]
fn test_geofence_p6_outside_diamond_with_center() {
    let poly = [
        Point { x: 0.0, y: 1.0 },
        Point { x: 1.0, y: 0.0 },
        Point { x: 0.0, y: -1.0 },
        Point { x: -1.0, y: 0.0 },
        Point { x: 0.0, y: 0.0 },
    ];
    let p = Point { x: -1.0, y: 1.0 };
    let result = is_position_in_geofence(&p, &poly);
    assert_eq!(result, false);
}

// === Real geofence test cases (Bangalore coordinates) ===

#[test]
fn test_geofence_real_point1_inside_poly1() {
    let p = Point { x: 77.64664, y: 12.90383 };
    let poly = [
        Point { x: 77.64354, y: 12.90448 },
        Point { x: 77.64671, y: 12.90573 },
        Point { x: 77.64815, y: 12.90367 },
        Point { x: 77.64703, y: 12.90215 },
    ];
    let result = is_position_in_geofence(&p, &poly);
    assert_eq!(result, true);
}

#[test]
fn test_geofence_real_point2_outside_poly2() {
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
    let result = is_position_in_geofence(&p, &poly);
    assert_eq!(result, false);
}

#[test]
fn test_geofence_real_point3_inside_poly2() {
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
    let result = is_position_in_geofence(&p, &poly);
    assert_eq!(result, true);
}

// === Edge cases ===

#[test]
fn test_geofence_two_vertices_returns_false() {
    let poly = [
        Point { x: 0.0, y: 0.0 },
        Point { x: 1.0, y: 1.0 },
    ];
    let p = Point { x: 0.0, y: 0.0 };
    let result = is_position_in_geofence(&p, &poly);
    assert_eq!(result, false);
}

#[test]
fn test_geofence_one_vertex_returns_false() {
    let poly = [Point { x: 0.0, y: 0.0 }];
    let p = Point { x: 0.0, y: 0.0 };
    let result = is_position_in_geofence(&p, &poly);
    assert_eq!(result, false);
}

#[test]
fn test_geofence_zero_vertices_returns_false() {
    let poly: [Point; 0] = [];
    let p = Point { x: 0.0, y: 0.0 };
    let result = is_position_in_geofence(&p, &poly);
    assert_eq!(result, false);
}

// === Additional triangle test ===

#[test]
fn test_geofence_triangle_inside() {
    let poly = [
        Point { x: 0.0, y: 1.0 },
        Point { x: 1.0, y: 0.0 },
        Point { x: -1.0, y: 0.0 },
    ];
    let p = Point { x: 0.0, y: 0.3 };
    let result = is_position_in_geofence(&p, &poly);
    assert_eq!(result, true);
}

#[test]
fn test_geofence_triangle_outside() {
    let poly = [
        Point { x: 0.0, y: 1.0 },
        Point { x: 1.0, y: 0.0 },
        Point { x: -1.0, y: 0.0 },
    ];
    let p = Point { x: 0.0, y: -0.5 };
    let result = is_position_in_geofence(&p, &poly);
    assert_eq!(result, false);
}

// === Negative coordinates: point inside square at negative quadrant ===

#[test]
fn test_geofence_negative_point_inside_square() {
    let poly = [
        Point { x: 1.0, y: 1.0 },
        Point { x: 1.0, y: -1.0 },
        Point { x: -1.0, y: -1.0 },
        Point { x: -1.0, y: 1.0 },
    ];
    let p = Point { x: -0.5, y: -0.5 };
    let result = is_position_in_geofence(&p, &poly);
    assert_eq!(result, true);
}

#[test]
fn test_geofence_outside_square() {
    let poly = [
        Point { x: 1.0, y: 1.0 },
        Point { x: 1.0, y: -1.0 },
        Point { x: -1.0, y: -1.0 },
        Point { x: -1.0, y: 1.0 },
    ];
    let p = Point { x: 2.0, y: 2.0 };
    let result = is_position_in_geofence(&p, &poly);
    assert_eq!(result, false);
}

// === Verify Point struct fields are accessible ===

#[test]
fn test_point_construction_and_access() {
    let p = Point { x: 1.5, y: -2.5 };
    assert_eq!(p.x, 1.5);
    assert_eq!(p.y, -2.5);
}

#[test]
fn test_point_copy_semantics() {
    let p1 = Point { x: 3.0, y: 4.0 };
    let p2 = p1; // Copy
    assert_eq!(p1.x, 3.0);
    assert_eq!(p1.y, 4.0);
    assert_eq!(p2.x, 3.0);
    assert_eq!(p2.y, 4.0);
}

fn main() {}
