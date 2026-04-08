use satc::satc::*;

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-10
}

fn nearest_hundredth(n: f64) -> f64 {
    (n * 100.0 + 0.5).floor() / 100.0
}

// --- Point operations ---

#[test]
fn test_point_copy() {
    let p = [1.0, 2.0];
    let q = [3.0, 4.0];
    let r = satc_point_copy(&p, &q);
    assert_eq!(r, [3.0, 4.0]);
}

#[test]
fn test_point_perp() {
    assert_eq!(satc_point_perp(&[3.0, 4.0]), [4.0, -3.0]);
    assert_eq!(satc_point_perp(&[0.0, 0.0]), [0.0, 0.0]);
    assert_eq!(satc_point_perp(&[1.0, 0.0]), [0.0, -1.0]);
}

#[test]
fn test_point_reverse() {
    assert_eq!(satc_point_reverse(&[3.0, 4.0]), [-3.0, -4.0]);
    assert_eq!(satc_point_reverse(&[0.0, 0.0]), [0.0, 0.0]);
}

#[test]
fn test_point_add() {
    assert_eq!(satc_point_add(&[1.0, 2.0], &[3.0, 4.0]), [4.0, 6.0]);
    assert_eq!(satc_point_add(&[0.0, 0.0], &[0.0, 0.0]), [0.0, 0.0]);
}

#[test]
fn test_point_sub() {
    assert_eq!(satc_point_sub(&[5.0, 7.0], &[3.0, 4.0]), [2.0, 3.0]);
}

#[test]
fn test_point_scale_xy() {
    let v = satc_point_scale_xy(&[5.0, 5.0], 10.0, 10.0);
    assert_eq!(v, [50.0, 50.0]);
    let v2 = satc_point_scale_xy(&v, 0.0, 1.0);
    assert_eq!(v2, [0.0, 50.0]);
    let v3 = satc_point_scale_xy(&v2, 1.0, 0.0);
    assert_eq!(v3, [0.0, 0.0]);
}

#[test]
fn test_point_scale_x() {
    assert_eq!(satc_point_scale_x(&[3.0, 4.0], 2.0), [6.0, 8.0]);
    assert_eq!(satc_point_scale_x(&[3.0, 4.0], 0.0), [0.0, 0.0]);
}

#[test]
fn test_point_create() {
    assert_eq!(satc_point_create(1.5, 2.5), [1.5, 2.5]);
}

#[test]
fn test_point_rotate() {
    let r = satc_point_rotate(&[1.0, 0.0], std::f64::consts::FRAC_PI_4);
    // C: x=0.70710678118654757 y=0.70710678118654746
    assert!(near(r[0], 0.7071067811865476));
    assert!(near(r[1], 0.7071067811865475));
}

#[test]
fn test_point_normalize() {
    let n = satc_point_normalize(&[3.0, 4.0]);
    assert!(near(n[0], 0.6));
    assert!(near(n[1], 0.8));
    // Zero vector stays zero
    assert_eq!(satc_point_normalize(&[0.0, 0.0]), [0.0, 0.0]);
}

#[test]
fn test_point_project() {
    // project(3,4) onto (1,0): amt = 3/1 = 3, result = (3*3, 3*4) = (9, 12)
    let r = satc_point_project(&[3.0, 4.0], &[1.0, 0.0]);
    assert_eq!(r, [9.0, 12.0]);
}

#[test]
fn test_point_reflect() {
    // reflect(1,2) on (1,0) -> (1, 2) per C ground truth
    let r = satc_point_reflect(&[1.0, 2.0], &[1.0, 0.0]);
    assert!(near(r[0], 1.0));
    assert!(near(r[1], 2.0));
}

// --- Voronoi region ---

#[test]
fn test_voronoi_region() {
    let line = [10.0, 0.0];
    assert_eq!(satc_voronoi_region(&line, &[-1.0, 0.0]), SATC_LEFT_VORONOI_REGION);
    assert_eq!(satc_voronoi_region(&line, &[5.0, 0.0]), SATC_MIDDLE_VORONOI_REGION);
    assert_eq!(satc_voronoi_region(&line, &[15.0, 0.0]), SATC_RIGHT_VORONOI_REGION);
    // Boundary: dot == 0 -> middle
    assert_eq!(satc_voronoi_region(&line, &[0.0, 5.0]), SATC_MIDDLE_VORONOI_REGION);
}

// --- Struct constructors ---

#[test]
fn test_circle_create() {
    let c = satc_circle_create([1.0, 2.0], 5.0);
    assert_eq!(c.pos, [1.0, 2.0]);
    assert_eq!(c.r, 5.0);
}

#[test]
fn test_circle_new() {
    let c = SatcCircle::new([1.0, 2.0], 5.0);
    assert_eq!(c.pos, [1.0, 2.0]);
    assert_eq!(c.r, 5.0);
}

#[test]
fn test_box_create() {
    let b = satc_box_create([5.0, 10.0], 30.0, 40.0);
    assert_eq!(b.pos, [5.0, 10.0]);
    assert_eq!(b.w, 30.0);
    assert_eq!(b.h, 40.0);
}

#[test]
fn test_box_new() {
    let b = SatcBox::new([5.0, 10.0], 30.0, 40.0);
    assert_eq!(b.pos, [5.0, 10.0]);
    assert_eq!(b.w, 30.0);
    assert_eq!(b.h, 40.0);
}

#[test]
fn test_response_create() {
    let r = satc_response_create();
    assert_eq!(r.overlap, f64::MAX);
    assert_eq!(r.overlap_n, [0.0, 0.0]);
    assert_eq!(r.overlap_v, [0.0, 0.0]);
    assert!(r.a_in_b);
    assert!(r.b_in_a);
}

#[test]
fn test_response_new() {
    let r = SatcResponse::new();
    assert_eq!(r.overlap, f64::MAX);
    assert!(r.a_in_b);
    assert!(r.b_in_a);
}

// --- Box to polygon ---

#[test]
fn test_box_to_polygon() {
    let b = satc_box_create([5.0, 10.0], 30.0, 40.0);
    let p = satc_box_to_polygon(&b);
    assert_eq!(p.pos, [5.0, 10.0]);
    assert_eq!(p.num_points, 4);
    assert_eq!(p.points[0], [0.0, 0.0]);
    assert_eq!(p.points[1], [30.0, 0.0]);
    assert_eq!(p.points[2], [30.0, 40.0]);
    assert_eq!(p.points[3], [0.0, 40.0]);
}

// --- Polygon create and set_points ---

#[test]
fn test_polygon_create() {
    let pts = vec![[0.0, 0.0], [10.0, 0.0], [5.0, 10.0]];
    let p = satc_polygon_create([0.0, 0.0], pts);
    assert_eq!(p.num_points, 3);
    assert_eq!(p.points[0], [0.0, 0.0]);
    assert_eq!(p.points[1], [10.0, 0.0]);
    assert_eq!(p.points[2], [5.0, 10.0]);
    assert_eq!(p.calc_points.len(), 3);
    assert_eq!(p.edges.len(), 3);
    assert_eq!(p.normals.len(), 3);
    assert_eq!(p.angle, 0.0);
    assert_eq!(p.offset, [0.0, 0.0]);
}

#[test]
fn test_polygon_new() {
    let p = SatcPolygon::new([1.0, 2.0], vec![[0.0, 0.0], [5.0, 0.0], [5.0, 5.0]]);
    assert_eq!(p.pos, [1.0, 2.0]);
    assert_eq!(p.num_points, 3);
}

#[test]
fn test_polygon_set_points() {
    let mut p = satc_polygon_create([0.0, 0.0], vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]);
    assert_eq!(p.num_points, 3);
    satc_polygon_set_points(&mut p, vec![[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0]]);
    assert_eq!(p.num_points, 4);
    assert_eq!(p.points.len(), 4);
    assert_eq!(p.calc_points.len(), 4);
    assert_eq!(p.edges.len(), 4);
    assert_eq!(p.normals.len(), 4);
}

// --- Circle AABB ---

#[test]
fn test_circle_get_aabb() {
    let c = satc_circle_create([10.0, 10.0], 5.0);
    let aabb = satc_circle_get_aabb(&c);
    assert_eq!(aabb.pos, [5.0, 5.0]);
    assert_eq!(aabb.num_points, 4);
    assert_eq!(aabb.points[0], [0.0, 0.0]);
    assert_eq!(aabb.points[1], [10.0, 0.0]);
    assert_eq!(aabb.points[2], [10.0, 10.0]);
    assert_eq!(aabb.points[3], [0.0, 10.0]);
}

// --- Polygon AABB ---

#[test]
fn test_polygon_get_aabb() {
    let p = satc_polygon_create([0.0, 0.0], vec![[0.0, 0.0], [30.0, 0.0], [15.0, 20.0]]);
    let aabb = satc_polygon_get_aabb(&p);
    // C: y_max = x_max (bug preserved), but for this case x_min=0, so y_max starts at 0 = x_max starts at 0
    // x_min=0, x_max=30, y_min=0, y_max=20
    assert_eq!(aabb.pos, [0.0, 0.0]);
    assert_eq!(aabb.points[0], [0.0, 0.0]);
    assert_eq!(aabb.points[1], [30.0, 0.0]);
    assert_eq!(aabb.points[2], [30.0, 20.0]);
    assert_eq!(aabb.points[3], [0.0, 20.0]);
}

// --- Polygon centroid ---

#[test]
fn test_polygon_get_centroid_square() {
    let p = satc_polygon_create(
        [0.0, 0.0],
        vec![[0.0, 0.0], [40.0, 0.0], [40.0, 40.0], [0.0, 40.0]],
    );
    let c = satc_polygon_get_centroid(&p);
    assert_eq!(c, [20.0, 20.0]);
}

#[test]
fn test_polygon_get_centroid_triangle() {
    let p = satc_polygon_create(
        [0.0, 0.0],
        vec![[0.0, 0.0], [100.0, 0.0], [50.0, 99.0]],
    );
    let c = satc_polygon_get_centroid(&p);
    assert_eq!(c, [50.0, 33.0]);
}

// --- flatten_points_on ---

#[test]
fn test_flatten_points_on() {
    let pts = [[0.0, 0.0], [10.0, 0.0], [5.0, 5.0]];
    let normal = [1.0, 0.0];
    let mut result = [0.0; 2];
    satc_flatten_points_on(3, &pts, &normal, &mut result);
    assert_eq!(result[0], 0.0);
    assert_eq!(result[1], 10.0);
}

#[test]
fn test_flatten_points_on_negative() {
    // All negative dot products — tests that max is initialized to -MAX not MIN
    let pts = [[-5.0, 0.0], [-10.0, 0.0], [-3.0, 0.0]];
    let normal = [1.0, 0.0];
    let mut result = [0.0; 2];
    satc_flatten_points_on(3, &pts, &normal, &mut result);
    assert_eq!(result[0], -10.0);
    assert_eq!(result[1], -3.0);
}

// --- Point in circle ---

#[test]
fn test_point_in_circle() {
    let c = satc_circle_create([100.0, 100.0], 20.0);
    assert!(!satc_point_in_circle(&[0.0, 0.0], &c));
    assert!(satc_point_in_circle(&[110.0, 110.0], &c));
    // On boundary
    assert!(satc_point_in_circle(&[120.0, 100.0], &c));
    // Center
    assert!(satc_point_in_circle(&[100.0, 100.0], &c));
}

// --- Point in polygon ---

#[test]
fn test_point_in_polygon() {
    let tri = satc_polygon_create(
        [30.0, 0.0],
        vec![[0.0, 0.0], [30.0, 0.0], [0.0, 30.0]],
    );
    assert!(!satc_point_in_polygon(&[0.0, 0.0], &tri));
    assert!(satc_point_in_polygon(&[35.0, 5.0], &tri));
}

#[test]
fn test_point_in_polygon_hexagon() {
    let poly = satc_polygon_create(
        [0.0, 0.0],
        vec![
            [2.0, 1.0], [2.0, 2.0], [1.0, 3.0],
            [0.0, 2.0], [0.0, 1.0], [1.0, 0.0],
        ],
    );
    assert!(satc_point_in_polygon(&[1.0, 1.1], &poly));
}

// --- Circle-circle collision ---

#[test]
fn test_circle_circle_collision() {
    let c1 = satc_circle_create([0.0, 0.0], 20.0);
    let c2 = satc_circle_create([30.0, 0.0], 20.0);
    let mut r = satc_response_create();
    assert!(satc_test_circle_circle(&c1, &c2, &mut r));
    assert_eq!(r.overlap, 10.0);
    assert_eq!(r.overlap_n, [1.0, 0.0]);
    assert_eq!(r.overlap_v, [10.0, 0.0]);
    assert!(!r.a_in_b);
    assert!(!r.b_in_a);
}

#[test]
fn test_circle_circle_no_collision() {
    let c1 = satc_circle_create([0.0, 0.0], 10.0);
    let c2 = satc_circle_create([100.0, 0.0], 10.0);
    let mut r = satc_response_create();
    assert!(!satc_test_circle_circle(&c1, &c2, &mut r));
}

#[test]
fn test_circle_circle_contained() {
    let c1 = satc_circle_create([0.0, 0.0], 5.0);
    let c2 = satc_circle_create([0.0, 0.0], 20.0);
    let mut r = satc_response_create();
    assert!(satc_test_circle_circle(&c1, &c2, &mut r));
    assert_eq!(r.overlap, 25.0);
    assert!(r.a_in_b);
    assert!(!r.b_in_a);
}

// --- Polygon-circle collision ---

#[test]
fn test_polygon_circle_collision() {
    let circle = satc_circle_create([50.0, 50.0], 20.0);
    let poly = satc_polygon_create(
        [0.0, 0.0],
        vec![[0.0, 0.0], [40.0, 0.0], [40.0, 40.0], [0.0, 40.0]],
    );
    let mut r = satc_response_create();
    assert!(satc_test_polygon_circle(&poly, &circle, &mut r));
    assert_eq!(nearest_hundredth(r.overlap), 5.86);
    assert_eq!(nearest_hundredth(r.overlap_v[0]), 4.14);
    assert_eq!(nearest_hundredth(r.overlap_v[1]), 4.14);
    assert!(!r.a_in_b);
    assert!(!r.b_in_a);
}

// --- Circle-polygon collision ---

#[test]
fn test_circle_polygon_collision() {
    let circle = satc_circle_create([50.0, 50.0], 20.0);
    let poly = satc_polygon_create(
        [0.0, 0.0],
        vec![[0.0, 0.0], [40.0, 0.0], [40.0, 40.0], [0.0, 40.0]],
    );
    let mut r = satc_response_create();
    assert!(satc_test_circle_polygon(&circle, &poly, &mut r));
    assert_eq!(nearest_hundredth(r.overlap), 5.86);
    // Reversed from polygon-circle
    assert_eq!(nearest_hundredth(r.overlap_v[0]), -4.14);
    assert_eq!(nearest_hundredth(r.overlap_v[1]), -4.14);
    assert!(!r.a_in_b);
    assert!(!r.b_in_a);
}

// --- Polygon-polygon collision ---

#[test]
fn test_polygon_polygon_no_collision() {
    let b1 = satc_box_create([0.0, 0.0], 20.0, 20.0);
    let p1 = satc_box_to_polygon(&b1);
    let b2 = satc_box_create([100.0, 100.0], 20.0, 20.0);
    let p2 = satc_box_to_polygon(&b2);
    let mut r = satc_response_create();
    assert!(!satc_test_polygon_polygon(&p1, &p2, &mut r));
}

#[test]
fn test_polygon_polygon_collision() {
    let b1 = satc_box_create([0.0, 0.0], 20.0, 20.0);
    let p1 = satc_box_to_polygon(&b1);
    let b2 = satc_box_create([10.0, 10.0], 20.0, 20.0);
    let p2 = satc_box_to_polygon(&b2);
    let mut r = satc_response_create();
    assert!(satc_test_polygon_polygon(&p1, &p2, &mut r));
    assert_eq!(r.overlap, 10.0);
    assert!(!r.a_in_b);
    assert!(!r.b_in_a);
}

// --- is_separating_axis ---

#[test]
fn test_is_separating_axis_separated() {
    let a_pos = [0.0, 0.0];
    let b_pos = [100.0, 0.0];
    let a_pts = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
    let b_pts = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
    let axis = [1.0, 0.0];
    let mut r = satc_response_create();
    assert!(satc_is_separating_axis(&a_pos, &b_pos, 4, &a_pts, 4, &b_pts, &axis, &mut r));
}

#[test]
fn test_is_separating_axis_overlapping() {
    let a_pos = [0.0, 0.0];
    let b_pos = [5.0, 0.0];
    let a_pts = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
    let b_pts = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
    let axis = [1.0, 0.0];
    let mut r = satc_response_create();
    assert!(!satc_is_separating_axis(&a_pos, &b_pos, 4, &a_pts, 4, &b_pts, &axis, &mut r));
}

// --- Constants ---

#[test]
fn test_constants() {
    assert_eq!(SATC_TYPE_NONE, 0);
    assert_eq!(SATC_TYPE_CIRCLE, 1);
    assert_eq!(SATC_TYPE_POLYGON, 2);
    assert_eq!(SATC_TYPE_BOX, 3);
    assert_eq!(SATC_LEFT_VORONOI_REGION, -1);
    assert_eq!(SATC_MIDDLE_VORONOI_REGION, 0);
    assert_eq!(SATC_RIGHT_VORONOI_REGION, 1);
}

// --- Edge cases ---

#[test]
fn test_circle_circle_touching() {
    // Exactly touching: distance == sum of radii
    let c1 = satc_circle_create([0.0, 0.0], 10.0);
    let c2 = satc_circle_create([20.0, 0.0], 10.0);
    let mut r = satc_response_create();
    // distance_sq (400) == total_radius_sq (400), so not > , returns true
    assert!(satc_test_circle_circle(&c1, &c2, &mut r));
    assert_eq!(r.overlap, 0.0);
}

#[test]
fn test_circle_circle_same_position() {
    let c1 = satc_circle_create([5.0, 5.0], 10.0);
    let c2 = satc_circle_create([5.0, 5.0], 10.0);
    let mut r = satc_response_create();
    assert!(satc_test_circle_circle(&c1, &c2, &mut r));
    assert_eq!(r.overlap, 20.0);
}

#[test]
fn test_polygon_recalc_with_offset() {
    // C ground truth: offset (5,5) applied to triangle (0,0),(10,0),(5,10)
    let mut p = satc_polygon_create([0.0, 0.0], vec![[0.0, 0.0], [10.0, 0.0], [5.0, 10.0]]);
    p.offset = [5.0, 5.0];
    satc_polygon_set_points(&mut p, vec![[0.0, 0.0], [10.0, 0.0], [5.0, 10.0]]);
    assert_eq!(p.calc_points[0], [5.0, 5.0]);
    assert_eq!(p.calc_points[1], [15.0, 5.0]);
    assert_eq!(p.calc_points[2], [10.0, 15.0]);
}

fn main() {}
