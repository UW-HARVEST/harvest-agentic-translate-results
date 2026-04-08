use geofence::geofence::{is_point_in_polygon, is_position_in_geofence, Point};

// ---- is_point_in_polygon tests (matching C test.c polygon tests) ----

#[test]
fn test_pip_origin_in_square() {
    let p = Point { x: 0.0, y: 0.0 };
    let poly = [Point{x:1.0,y:1.0}, Point{x:1.0,y:-1.0}, Point{x:-1.0,y:-1.0}, Point{x:-1.0,y:1.0}];
    assert_eq!(is_point_in_polygon(&p, &poly), true);
}

#[test]
fn test_pip_neg1_0_in_square() {
    let p = Point { x: -1.0, y: 0.0 };
    let poly = [Point{x:1.0,y:1.0}, Point{x:1.0,y:-1.0}, Point{x:-1.0,y:-1.0}, Point{x:-1.0,y:1.0}];
    assert_eq!(is_point_in_polygon(&p, &poly), true);
}

#[test]
fn test_pip_0_05_in_diamond() {
    let p = Point { x: 0.0, y: 0.5 };
    let poly = [Point{x:0.0,y:1.0}, Point{x:1.0,y:0.0}, Point{x:0.0,y:-1.0}, Point{x:-1.0,y:0.0}];
    assert_eq!(is_point_in_polygon(&p, &poly), true);
}

#[test]
fn test_pip_5_05_outside_diamond() {
    let p = Point { x: 5.0, y: 0.5 };
    let poly = [Point{x:0.0,y:1.0}, Point{x:1.0,y:0.0}, Point{x:0.0,y:-1.0}, Point{x:-1.0,y:0.0}];
    assert_eq!(is_point_in_polygon(&p, &poly), false);
}

#[test]
fn test_pip_origin_in_diamond() {
    let p = Point { x: 0.0, y: 0.0 };
    let poly = [Point{x:0.0,y:1.0}, Point{x:1.0,y:0.0}, Point{x:0.0,y:-1.0}, Point{x:-1.0,y:0.0}];
    assert_eq!(is_point_in_polygon(&p, &poly), true);
}

#[test]
fn test_pip_neg1_0_in_diamond() {
    let p = Point { x: -1.0, y: 0.0 };
    let poly = [Point{x:0.0,y:1.0}, Point{x:1.0,y:0.0}, Point{x:0.0,y:-1.0}, Point{x:-1.0,y:0.0}];
    assert_eq!(is_point_in_polygon(&p, &poly), true);
}

#[test]
fn test_pip_0_05_in_poly3() {
    let p = Point { x: 0.0, y: 0.5 };
    let poly = [Point{x:0.0,y:1.0}, Point{x:1.0,y:0.0}, Point{x:0.0,y:-1.0}, Point{x:-1.0,y:0.0}, Point{x:0.0,y:0.0}];
    assert_eq!(is_point_in_polygon(&p, &poly), true);
}

#[test]
fn test_pip_origin_in_poly3() {
    let p = Point { x: 0.0, y: 0.0 };
    let poly = [Point{x:0.0,y:1.0}, Point{x:1.0,y:0.0}, Point{x:0.0,y:-1.0}, Point{x:-1.0,y:0.0}, Point{x:0.0,y:0.0}];
    assert_eq!(is_point_in_polygon(&p, &poly), true);
}

#[test]
fn test_pip_05_0_in_poly3() {
    let p = Point { x: 0.5, y: 0.0 };
    let poly = [Point{x:0.0,y:1.0}, Point{x:1.0,y:0.0}, Point{x:0.0,y:-1.0}, Point{x:-1.0,y:0.0}, Point{x:0.0,y:0.0}];
    assert_eq!(is_point_in_polygon(&p, &poly), true);
}

#[test]
fn test_pip_neg1_1_outside_poly3() {
    let p = Point { x: -1.0, y: 1.0 };
    let poly = [Point{x:0.0,y:1.0}, Point{x:1.0,y:0.0}, Point{x:0.0,y:-1.0}, Point{x:-1.0,y:0.0}, Point{x:0.0,y:0.0}];
    assert_eq!(is_point_in_polygon(&p, &poly), false);
}

// ---- Edge cases ----

#[test]
fn test_pip_fewer_than_3_vertices() {
    let p = Point { x: 0.0, y: 0.0 };
    let line = [Point{x:0.0,y:0.0}, Point{x:1.0,y:1.0}];
    assert_eq!(is_point_in_polygon(&p, &line), false);
    assert_eq!(is_point_in_polygon(&p, &[]), false);
}

// ---- is_position_in_geofence tests (matching C test.c geofence tests) ----

#[test]
fn test_geofence_point1_inside_poly1() {
    let p = Point { x: 77.64664, y: 12.90383 };
    let poly = [
        Point{x:77.64354, y:12.90448}, Point{x:77.64671, y:12.90573},
        Point{x:77.64815, y:12.90367}, Point{x:77.64703, y:12.90215},
    ];
    assert_eq!(is_position_in_geofence(&p, &poly), true);
}

#[test]
fn test_geofence_point2_outside_poly2() {
    let p = Point { x: 77.6183653, y: 12.9086164 };
    let poly = [
        Point{x:77.6084948, y:12.9115446}, Point{x:77.6163054, y:12.9114191},
        Point{x:77.6179361, y:12.907236},  Point{x:77.6226139, y:12.9099132},
        Point{x:77.6197386, y:12.9190739}, Point{x:77.6300812, y:12.9186138},
        Point{x:77.6333857, y:12.9106243}, Point{x:77.6314974, y:12.9017561},
        Point{x:77.6213264, y:12.9001665}, Point{x:77.6100397, y:12.9019652},
    ];
    assert_eq!(is_position_in_geofence(&p, &poly), false);
}

#[test]
fn test_geofence_point3_inside_poly2() {
    let p = Point { x: 77.6168203, y: 12.9081563 };
    let poly = [
        Point{x:77.6084948, y:12.9115446}, Point{x:77.6163054, y:12.9114191},
        Point{x:77.6179361, y:12.907236},  Point{x:77.6226139, y:12.9099132},
        Point{x:77.6197386, y:12.9190739}, Point{x:77.6300812, y:12.9186138},
        Point{x:77.6333857, y:12.9106243}, Point{x:77.6314974, y:12.9017561},
        Point{x:77.6213264, y:12.9001665}, Point{x:77.6100397, y:12.9019652},
    ];
    assert_eq!(is_position_in_geofence(&p, &poly), true);
}

fn main() {}
