use geofence::geofence::{is_position_in_geofence, Point};

fn p(x: f64, y: f64) -> Point {
    Point { x, y }
}

// ---- is_position_in_geofence with polygon test cases from C ----

#[test]
fn geofence_point1_in_poly1() {
    // geofence_point1 inside geofence_poly1
    let pt = p(77.64664, 12.90383);
    let poly = vec![
        p(77.64354, 12.90448),
        p(77.64671, 12.90573),
        p(77.64815, 12.90367),
        p(77.64703, 12.90215),
    ];
    assert!(is_position_in_geofence(&pt, &poly));
}

#[test]
fn geofence_point2_outside_poly2() {
    // geofence_point2 outside geofence_poly2
    let pt = p(77.6183653, 12.9086164);
    let poly = vec![
        p(77.6084948, 12.9115446),
        p(77.6163054, 12.9114191),
        p(77.6179361, 12.907236),
        p(77.6226139, 12.9099132),
        p(77.6197386, 12.9190739),
        p(77.6300812, 12.9186138),
        p(77.6333857, 12.9106243),
        p(77.6314974, 12.9017561),
        p(77.6213264, 12.9001665),
        p(77.6100397, 12.9019652),
    ];
    assert!(!is_position_in_geofence(&pt, &poly));
}

#[test]
fn geofence_point3_in_poly2() {
    // geofence_point3 inside geofence_poly2
    let pt = p(77.6168203, 12.9081563);
    let poly = vec![
        p(77.6084948, 12.9115446),
        p(77.6163054, 12.9114191),
        p(77.6179361, 12.907236),
        p(77.6226139, 12.9099132),
        p(77.6197386, 12.9190739),
        p(77.6300812, 12.9186138),
        p(77.6333857, 12.9106243),
        p(77.6314974, 12.9017561),
        p(77.6213264, 12.9001665),
        p(77.6100397, 12.9019652),
    ];
    assert!(is_position_in_geofence(&pt, &poly));
}

// ---- is_point_in_polygon tests via is_position_in_geofence ----
// Since is_point_in_polygon is private, we test it indirectly.
// For small coordinate values the mercator projection is ~linear,
// so we use is_position_in_geofence with the same test logic.
// The C polygon tests use raw coordinates; here we replicate the
// expected boolean results through the geofence wrapper.

// poly1: square (1,1),(1,-1),(-1,-1),(-1,1)
fn poly1() -> Vec<Point> {
    vec![p(1.0, 1.0), p(1.0, -1.0), p(-1.0, -1.0), p(-1.0, 1.0)]
}

// poly2: diamond (0,1),(1,0),(0,-1),(-1,0)
fn poly2() -> Vec<Point> {
    vec![p(0.0, 1.0), p(1.0, 0.0), p(0.0, -1.0), p(-1.0, 0.0)]
}

// poly3: (0,1),(1,0),(0,-1),(-1,0),(0,0)
fn poly3() -> Vec<Point> {
    vec![p(0.0, 1.0), p(1.0, 0.0), p(0.0, -1.0), p(-1.0, 0.0), p(0.0, 0.0)]
}

#[test]
fn origin_in_poly1() {
    assert!(is_position_in_geofence(&p(0.0, 0.0), &poly1()));
}

#[test]
fn p2_in_poly1() {
    assert!(is_position_in_geofence(&p(-1.0, 0.0), &poly1()));
}

#[test]
fn p3_in_poly2() {
    assert!(is_position_in_geofence(&p(0.0, 0.5), &poly2()));
}

#[test]
fn p4_outside_poly2() {
    assert!(!is_position_in_geofence(&p(5.0, 0.5), &poly2()));
}

#[test]
fn origin_in_poly2() {
    assert!(is_position_in_geofence(&p(0.0, 0.0), &poly2()));
}

#[test]
fn p2_in_poly2() {
    assert!(is_position_in_geofence(&p(-1.0, 0.0), &poly2()));
}

#[test]
fn p3_in_poly3() {
    assert!(is_position_in_geofence(&p(0.0, 0.5), &poly3()));
}

#[test]
fn origin_in_poly3() {
    assert!(is_position_in_geofence(&p(0.0, 0.0), &poly3()));
}

#[test]
fn p5_in_poly3() {
    assert!(is_position_in_geofence(&p(0.5, 0.0), &poly3()));
}

#[test]
fn p6_outside_poly3() {
    assert!(!is_position_in_geofence(&p(-1.0, 1.0), &poly3()));
}

// ---- Edge cases ----

#[test]
fn fewer_than_3_vertices_returns_false() {
    assert!(!is_position_in_geofence(&p(0.0, 0.0), &[p(1.0, 1.0), p(2.0, 2.0)]));
}

#[test]
fn zero_vertices_returns_false() {
    assert!(!is_position_in_geofence(&p(0.0, 0.0), &[]));
}

fn main() {}
