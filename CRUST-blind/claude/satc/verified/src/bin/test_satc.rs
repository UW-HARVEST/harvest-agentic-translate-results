#![allow(dead_code, unused_imports)]

use satc::satc::*;

const EPS: f64 = 1e-10;

fn approx_eq(a: f64, b: f64) {
    assert!((a - b).abs() < EPS, "expected {} got {}", b, a);
}

fn approx_eq_pt(a: &[f64; 2], b: &[f64; 2]) {
    approx_eq(a[0], b[0]);
    approx_eq(a[1], b[1]);
}

// ==================== Point operations ====================

#[test]
fn test_point_copy() {
    let p = [99.0, 99.0];
    let q = [5.0, 7.0];
    let r = satc_point_copy(&p, &q);
    assert_eq!(r, [5.0, 7.0]);
}

#[test]
fn test_point_perp() {
    let p = [3.0, 4.0];
    let r = satc_point_perp(&p);
    assert_eq!(r, [4.0, -3.0]);
}

#[test]
fn test_point_reverse() {
    let p = [3.0, -4.0];
    let r = satc_point_reverse(&p);
    assert_eq!(r, [-3.0, 4.0]);
}

#[test]
fn test_point_add() {
    let p = [1.5, 2.5];
    let q = [3.5, -1.0];
    let r = satc_point_add(&p, &q);
    assert_eq!(r, [5.0, 1.5]);
}

#[test]
fn test_point_sub() {
    let p = [5.0, 7.0];
    let q = [1.0, 3.0];
    let r = satc_point_sub(&p, &q);
    assert_eq!(r, [4.0, 4.0]);
}

#[test]
fn test_point_scale_xy() {
    let p = [5.0, 5.0];
    let r = satc_point_scale_xy(&p, 10.0, 10.0);
    assert_eq!(r, [50.0, 50.0]);

    let r2 = satc_point_scale_xy(&r, 0.0, 1.0);
    assert_eq!(r2, [0.0, 50.0]);

    let r3 = satc_point_scale_xy(&r2, 1.0, 0.0);
    assert_eq!(r3, [0.0, 0.0]);
}

#[test]
fn test_point_scale_x() {
    // C scale_x(p, x) calls scale_xy(p, x, x) — scales BOTH by same factor.
    let p = [3.0, 4.0];
    let r = satc_point_scale_x(&p, 2.5);
    assert_eq!(r, [7.5, 10.0]);
}

#[test]
fn test_point_create() {
    let p = satc_point_create(3.5, -1.5);
    assert_eq!(p, [3.5, -1.5]);
}

#[test]
fn test_point_rotate() {
    // rotate(2,3,0): formula gives (2*cos(0)-3*sin(0), 2*sin(0)-3*cos(0)) = (2, -3)
    let p = [2.0, 3.0];
    let r = satc_point_rotate(&p, 0.0);
    approx_eq_pt(&r, &[2.0, -3.0]);

    // rotate(1,0, pi/2): C output 6.12e-17, 1
    let p2 = [1.0, 0.0];
    let r2 = satc_point_rotate(&p2, std::f64::consts::PI / 2.0);
    approx_eq(r2[0], 0.0);
    approx_eq(r2[1], 1.0);

    // rotate(1,1, pi/4): C output ~ (0, 0)
    let p3 = [1.0, 1.0];
    let r3 = satc_point_rotate(&p3, std::f64::consts::PI / 4.0);
    approx_eq(r3[0], 0.0);
    approx_eq(r3[1], 0.0);
}

#[test]
fn test_point_normalize() {
    let p = [3.0, 4.0];
    let r = satc_point_normalize(&p);
    approx_eq(r[0], 0.6);
    approx_eq(r[1], 0.8);
}

#[test]
fn test_point_normalize_unit() {
    let p = [1.0, 0.0];
    let r = satc_point_normalize(&p);
    approx_eq_pt(&r, &[1.0, 0.0]);
}

#[test]
fn test_point_project() {
    // The C macro for satc_point_project has an operator precedence bug:
    // amt = satc_point_dot(p, q) / satc_point_len2(q)
    // expands to: p[0]*q[0] + p[1]*q[1] / q[0]*q[0] + q[1]*q[1]
    // For project((2,3) onto (1,0)): 2*1 + 3*0/1*1 + 0*0 = 2 -> (2*2, 2*3) = (4, 6)
    let p = [2.0, 3.0];
    let q = [1.0, 0.0];
    let r = satc_point_project(&p, &q);
    assert_eq!(r, [4.0, 6.0]);
}

#[test]
fn test_point_project_xy() {
    // For project((4,2) onto (1,1)): 4*1 + 2*1/1*1 + 1*1 = 4+2+1 = 7
    // result = (7*4, 7*2) = (28, 14)
    let p = [4.0, 2.0];
    let q = [1.0, 1.0];
    let r = satc_point_project(&p, &q);
    assert_eq!(r, [28.0, 14.0]);
}

#[test]
fn test_point_project_arbitrary() {
    // For project((6,8),(2,3)): 6*2 + 8*3/2*2 + 3*3 = 12 + 24 + 9 = 45
    // (8*3/2*2 = 24/2*2 = 12*2 = 24 due to L->R associativity)
    // result = (45*6, 45*8) = (270, 360)
    let p = [6.0, 8.0];
    let q = [2.0, 3.0];
    let r = satc_point_project(&p, &q);
    assert_eq!(r, [270.0, 360.0]);
}

#[test]
fn test_point_reflect() {
    // C reflect: project p onto axis, scale by 2, subtract original.
    // reflect((2,3) along (1,0)):
    //   project((2,3),(1,0)): dot=2, len2=1, amt=2 -> p=(4, 6)
    //   scale_x by 2: (8, 12)
    //   subtract original (2,3): (6, 9)
    let p = [2.0, 3.0];
    let axis = [1.0, 0.0];
    let r = satc_point_reflect(&p, &axis);
    assert_eq!(r, [6.0, 9.0]);
}

// ==================== Voronoi region ====================

#[test]
fn test_voronoi_region() {
    let line = [10.0, 0.0];
    let p1 = [-5.0, 1.0];
    let p2 = [5.0, 1.0];
    let p3 = [15.0, 1.0];
    assert_eq!(satc_voronoi_region(&line, &p1), SATC_LEFT_VORONOI_REGION);
    assert_eq!(satc_voronoi_region(&line, &p2), SATC_MIDDLE_VORONOI_REGION);
    assert_eq!(satc_voronoi_region(&line, &p3), SATC_RIGHT_VORONOI_REGION);
}

#[test]
fn test_voronoi_constants() {
    assert_eq!(SATC_LEFT_VORONOI_REGION, -1);
    assert_eq!(SATC_MIDDLE_VORONOI_REGION, 0);
    assert_eq!(SATC_RIGHT_VORONOI_REGION, 1);
}

#[test]
fn test_type_constants() {
    assert_eq!(SATC_TYPE_NONE, 0);
    assert_eq!(SATC_TYPE_CIRCLE, 1);
    assert_eq!(SATC_TYPE_POLYGON, 2);
    assert_eq!(SATC_TYPE_BOX, 3);
}

// ==================== Flatten points on ====================

#[test]
fn test_flatten_points_on() {
    let points: Vec<[f64; 2]> = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 5.0], [0.0, 5.0]];
    let mut result = [0.0, 0.0];
    let axis_x = [1.0, 0.0];
    satc_flatten_points_on(4, &points, &axis_x, &mut result);
    assert_eq!(result, [0.0, 10.0]);

    let axis_y = [0.0, 1.0];
    satc_flatten_points_on(4, &points, &axis_y, &mut result);
    assert_eq!(result, [0.0, 5.0]);
}

// ==================== Circle ====================

#[test]
fn test_circle_create() {
    let c = satc_circle_create([5.0, 7.0], 3.0);
    assert_eq!(c.pos, [5.0, 7.0]);
    assert_eq!(c.r, 3.0);
}

#[test]
fn test_circle_get_aabb() {
    let circle = satc_circle_create([5.0, 7.0], 3.0);
    let aabb = satc_circle_get_aabb(&circle);
    assert_eq!(aabb.pos, [2.0, 4.0]);
    assert_eq!(aabb.num_points, 4);
    assert_eq!(aabb.points[0], [0.0, 0.0]);
    assert_eq!(aabb.points[1], [6.0, 0.0]);
    assert_eq!(aabb.points[2], [6.0, 6.0]);
    assert_eq!(aabb.points[3], [0.0, 6.0]);
}

#[test]
fn test_circle_get_aabb_origin() {
    let circle = satc_circle_create([0.0, 0.0], 5.0);
    let aabb = satc_circle_get_aabb(&circle);
    assert_eq!(aabb.pos, [-5.0, -5.0]);
    assert_eq!(aabb.points[0], [0.0, 0.0]);
    assert_eq!(aabb.points[1], [10.0, 0.0]);
    assert_eq!(aabb.points[2], [10.0, 10.0]);
    assert_eq!(aabb.points[3], [0.0, 10.0]);
}

// ==================== Box ====================

#[test]
fn test_box_create() {
    let b = satc_box_create([1.0, 2.0], 10.0, 5.0);
    assert_eq!(b.pos, [1.0, 2.0]);
    assert_eq!(b.w, 10.0);
    assert_eq!(b.h, 5.0);
}

#[test]
fn test_box_to_polygon() {
    let b = satc_box_create([1.0, 2.0], 10.0, 5.0);
    let poly = satc_box_to_polygon(&b);
    assert_eq!(poly.pos, [1.0, 2.0]);
    assert_eq!(poly.num_points, 4);
    assert_eq!(poly.points[0], [0.0, 0.0]);
    assert_eq!(poly.points[1], [10.0, 0.0]);
    assert_eq!(poly.points[2], [10.0, 5.0]);
    assert_eq!(poly.points[3], [0.0, 5.0]);
    // edges
    assert_eq!(poly.edges[0], [10.0, 0.0]);
    assert_eq!(poly.edges[1], [0.0, 5.0]);
    assert_eq!(poly.edges[2], [-10.0, 0.0]);
    assert_eq!(poly.edges[3], [0.0, -5.0]);
    // normals (perp + normalize)
    approx_eq_pt(&poly.normals[0], &[0.0, -1.0]);
    approx_eq_pt(&poly.normals[1], &[1.0, 0.0]);
    approx_eq_pt(&poly.normals[2], &[0.0, 1.0]);
    approx_eq_pt(&poly.normals[3], &[-1.0, 0.0]);
}

// ==================== Polygon ====================

#[test]
fn test_polygon_create() {
    let pos = [10.0, 20.0];
    let points: Vec<[f64; 2]> = vec![[0.0, 0.0], [10.0, 0.0], [5.0, 10.0]];
    let poly = satc_polygon_create(pos, points);
    assert_eq!(poly.pos, [10.0, 20.0]);
    assert_eq!(poly.num_points, 3);
    assert_eq!(poly.points[0], [0.0, 0.0]);
    assert_eq!(poly.points[1], [10.0, 0.0]);
    assert_eq!(poly.points[2], [5.0, 10.0]);
    assert_eq!(poly.calc_points[0], [0.0, 0.0]);
    assert_eq!(poly.calc_points[1], [10.0, 0.0]);
    assert_eq!(poly.calc_points[2], [5.0, 10.0]);
    assert_eq!(poly.edges[0], [10.0, 0.0]);
    assert_eq!(poly.edges[1], [-5.0, 10.0]);
    assert_eq!(poly.edges[2], [-5.0, -10.0]);
    approx_eq_pt(&poly.normals[0], &[0.0, -1.0]);
    approx_eq_pt(&poly.normals[1], &[0.89442719099991585541, 0.4472135954999579277]);
    approx_eq_pt(&poly.normals[2], &[-0.89442719099991585541, 0.4472135954999579277]);
}

#[test]
fn test_polygon_set_points() {
    let mut poly = satc_polygon_create([0.0, 0.0], vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]);
    let new_points: Vec<[f64; 2]> = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
    satc_polygon_set_points(&mut poly, new_points);
    assert_eq!(poly.num_points, 4);
    assert_eq!(poly.points[0], [0.0, 0.0]);
    assert_eq!(poly.points[1], [10.0, 0.0]);
    assert_eq!(poly.points[2], [10.0, 10.0]);
    assert_eq!(poly.points[3], [0.0, 10.0]);
    assert_eq!(poly.calc_points.len(), 4);
    assert_eq!(poly.edges.len(), 4);
    assert_eq!(poly.normals.len(), 4);
}

#[test]
fn test_polygon_get_aabb() {
    let pos = [50.0, 60.0];
    let points: Vec<[f64; 2]> = vec![[1.0, -2.0], [4.0, 5.0], [-3.0, 7.0]];
    let poly = satc_polygon_create(pos, points);
    let aabb = satc_polygon_get_aabb(&poly);
    assert_eq!(aabb.pos, [-3.0, -2.0]);
    assert_eq!(aabb.num_points, 4);
    assert_eq!(aabb.points[0], [0.0, 0.0]);
    assert_eq!(aabb.points[1], [7.0, 0.0]);
    assert_eq!(aabb.points[2], [7.0, 9.0]);
    assert_eq!(aabb.points[3], [0.0, 9.0]);
}

#[test]
fn test_polygon_get_centroid_square() {
    let points: Vec<[f64; 2]> = vec![[0.0, 0.0], [40.0, 0.0], [40.0, 40.0], [0.0, 40.0]];
    let poly = satc_polygon_create([0.0, 0.0], points);
    let c = satc_polygon_get_centroid(&poly);
    assert_eq!(c, [20.0, 20.0]);
}

#[test]
fn test_polygon_get_centroid_triangle() {
    let points: Vec<[f64; 2]> = vec![[0.0, 0.0], [100.0, 0.0], [50.0, 99.0]];
    let poly = satc_polygon_create([0.0, 0.0], points);
    let c = satc_polygon_get_centroid(&poly);
    approx_eq(c[0], 50.0);
    approx_eq(c[1], 33.0);
}

// ==================== Response ====================

#[test]
fn test_response_create() {
    let r = satc_response_create();
    assert_eq!(r.overlap, f64::MAX);
    assert_eq!(r.overlap_n, [0.0, 0.0]);
    assert_eq!(r.overlap_v, [0.0, 0.0]);
    assert!(r.a_in_b);
    assert!(r.b_in_a);
}

// ==================== Circle-Circle ====================

#[test]
fn test_circle_circle_collision() {
    let a = satc_circle_create([0.0, 0.0], 20.0);
    let b = satc_circle_create([30.0, 0.0], 20.0);
    let mut r = satc_response_create();
    let collided = satc_test_circle_circle(&a, &b, &mut r);
    assert!(collided);
    approx_eq(r.overlap, 10.0);
    approx_eq_pt(&r.overlap_n, &[1.0, 0.0]);
    approx_eq_pt(&r.overlap_v, &[10.0, 0.0]);
    assert!(!r.a_in_b);
    assert!(!r.b_in_a);
}

#[test]
fn test_circle_circle_no_collision() {
    let a = satc_circle_create([0.0, 0.0], 20.0);
    let b = satc_circle_create([100.0, 0.0], 20.0);
    let mut r = satc_response_create();
    let collided = satc_test_circle_circle(&a, &b, &mut r);
    assert!(!collided);
}

#[test]
fn test_circle_circle_a_in_b() {
    let a = satc_circle_create([0.0, 0.0], 5.0);
    let b = satc_circle_create([0.0, 0.0], 20.0);
    let mut r = satc_response_create();
    let collided = satc_test_circle_circle(&a, &b, &mut r);
    assert!(collided);
    approx_eq(r.overlap, 25.0);
    assert!(r.a_in_b);
    assert!(!r.b_in_a);
}

#[test]
fn test_circle_circle_b_in_a() {
    let a = satc_circle_create([0.0, 0.0], 20.0);
    let b = satc_circle_create([0.0, 0.0], 5.0);
    let mut r = satc_response_create();
    let collided = satc_test_circle_circle(&a, &b, &mut r);
    assert!(collided);
    approx_eq(r.overlap, 25.0);
    assert!(!r.a_in_b);
    assert!(r.b_in_a);
}

// ==================== Polygon-Circle ====================

#[test]
fn test_polygon_circle_collision() {
    let circle = satc_circle_create([50.0, 50.0], 20.0);
    let polygon = satc_polygon_create(
        [0.0, 0.0],
        vec![[0.0, 0.0], [40.0, 0.0], [40.0, 40.0], [0.0, 40.0]],
    );
    let mut r = satc_response_create();
    let collided = satc_test_polygon_circle(&polygon, &circle, &mut r);
    assert!(collided);
    approx_eq(r.overlap, 5.8578643762690489893);
    approx_eq(r.overlap_n[0], 0.7071067811865475);
    approx_eq(r.overlap_n[1], 0.7071067811865475);
    approx_eq(r.overlap_v[0], 4.142135623730950);
    approx_eq(r.overlap_v[1], 4.142135623730950);
    assert!(!r.a_in_b);
    assert!(!r.b_in_a);
}

#[test]
fn test_polygon_circle_no_collision() {
    let circle = satc_circle_create([200.0, 200.0], 5.0);
    let polygon = satc_polygon_create(
        [0.0, 0.0],
        vec![[0.0, 0.0], [40.0, 0.0], [40.0, 40.0], [0.0, 40.0]],
    );
    let mut r = satc_response_create();
    let collided = satc_test_polygon_circle(&polygon, &circle, &mut r);
    assert!(!collided);
}

#[test]
fn test_polygon_circle_b_in_a() {
    let circle = satc_circle_create([50.0, 50.0], 5.0);
    let polygon = satc_polygon_create(
        [0.0, 0.0],
        vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]],
    );
    let mut r = satc_response_create();
    let collided = satc_test_polygon_circle(&polygon, &circle, &mut r);
    assert!(collided);
    assert!(!r.a_in_b);
    assert!(r.b_in_a);
}

// ==================== Circle-Polygon ====================

#[test]
fn test_circle_polygon_collision() {
    let circle = satc_circle_create([50.0, 50.0], 20.0);
    let polygon = satc_polygon_create(
        [0.0, 0.0],
        vec![[0.0, 0.0], [40.0, 0.0], [40.0, 40.0], [0.0, 40.0]],
    );
    let mut r = satc_response_create();
    let collided = satc_test_circle_polygon(&circle, &polygon, &mut r);
    assert!(collided);
    approx_eq(r.overlap, 5.8578643762690489893);
    approx_eq(r.overlap_n[0], -0.7071067811865475);
    approx_eq(r.overlap_n[1], -0.7071067811865475);
    approx_eq(r.overlap_v[0], -4.142135623730950);
    approx_eq(r.overlap_v[1], -4.142135623730950);
    assert!(!r.a_in_b);
    assert!(!r.b_in_a);
}

// ==================== Polygon-Polygon ====================

#[test]
fn test_polygon_polygon_collision() {
    let b1 = satc_box_create([0.0, 0.0], 30.0, 30.0);
    let b2 = satc_box_create([25.0, 0.0], 30.0, 30.0);
    let p1 = satc_box_to_polygon(&b1);
    let p2 = satc_box_to_polygon(&b2);
    let mut r = satc_response_create();
    let collided = satc_test_polygon_polygon(&p1, &p2, &mut r);
    assert!(collided);
    approx_eq(r.overlap, 5.0);
    approx_eq_pt(&r.overlap_n, &[1.0, 0.0]);
    approx_eq_pt(&r.overlap_v, &[5.0, 0.0]);
    assert!(!r.a_in_b);
    assert!(!r.b_in_a);
}

#[test]
fn test_polygon_polygon_no_collision() {
    let b1 = satc_box_create([0.0, 0.0], 20.0, 20.0);
    let b2 = satc_box_create([100.0, 100.0], 20.0, 20.0);
    let p1 = satc_box_to_polygon(&b1);
    let p2 = satc_box_to_polygon(&b2);
    let mut r = satc_response_create();
    let collided = satc_test_polygon_polygon(&p1, &p2, &mut r);
    assert!(!collided);
}

#[test]
fn test_polygon_polygon_triangles() {
    let p1 = satc_polygon_create([0.0, 0.0], vec![[0.0, 0.0], [10.0, 0.0], [5.0, 10.0]]);
    let p2 = satc_polygon_create([0.0, 0.0], vec![[5.0, 5.0], [15.0, 5.0], [10.0, 15.0]]);
    let mut r = satc_response_create();
    let collided = satc_test_polygon_polygon(&p1, &p2, &mut r);
    assert!(collided);
    approx_eq(r.overlap, 5.0);
    approx_eq(r.overlap_n[0], 0.0);
    approx_eq(r.overlap_n[1], 1.0);
    approx_eq(r.overlap_v[0], 0.0);
    approx_eq(r.overlap_v[1], 5.0);
    assert!(!r.a_in_b);
    assert!(!r.b_in_a);
}

// ==================== Point in circle ====================

#[test]
fn test_point_in_circle_inside() {
    let c = satc_circle_create([100.0, 100.0], 20.0);
    let inside = [110.0, 110.0];
    assert!(satc_point_in_circle(&inside, &c));
}

#[test]
fn test_point_in_circle_outside() {
    let c = satc_circle_create([100.0, 100.0], 20.0);
    let outside = [0.0, 0.0];
    assert!(!satc_point_in_circle(&outside, &c));
}

#[test]
fn test_point_in_circle_on_edge() {
    // distance_sq <= radius_sq, exactly on edge counts as inside
    let c = satc_circle_create([100.0, 100.0], 20.0);
    let on_edge = [120.0, 100.0];
    assert!(satc_point_in_circle(&on_edge, &c));
}

// ==================== Point in polygon ====================

#[test]
fn test_point_in_polygon_inside_triangle() {
    let triangle = satc_polygon_create(
        [30.0, 0.0],
        vec![[0.0, 0.0], [30.0, 0.0], [0.0, 30.0]],
    );
    // From C test: point (35, 5) is inside (since polygon pos is (30, 0), so absolute coords 65, 5)
    // Actually the C test: pos=(30,0), points relative. Points abs: (30,0)(60,0)(30,30). p2=(35,5) in polygon.
    let p = [35.0, 5.0];
    assert!(satc_point_in_polygon(&p, &triangle));
}

#[test]
fn test_point_in_polygon_outside_triangle() {
    let triangle = satc_polygon_create(
        [30.0, 0.0],
        vec![[0.0, 0.0], [30.0, 0.0], [0.0, 30.0]],
    );
    let p = [0.0, 0.0];
    assert!(!satc_point_in_polygon(&p, &triangle));
}

#[test]
fn test_point_in_polygon_small() {
    // From C test: small hexagonal polygon
    let polygon = satc_polygon_create(
        [0.0, 0.0],
        vec![
            [2.0, 1.0],
            [2.0, 2.0],
            [1.0, 3.0],
            [0.0, 2.0],
            [0.0, 1.0],
            [1.0, 0.0],
        ],
    );
    let point = [1.0, 1.1];
    assert!(satc_point_in_polygon(&point, &polygon));
}

#[test]
fn test_point_in_polygon_square() {
    let square = satc_polygon_create(
        [0.0, 0.0],
        vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]],
    );
    assert!(satc_point_in_polygon(&[50.0, 50.0], &square));
    assert!(!satc_point_in_polygon(&[200.0, 200.0], &square));
}

// ==================== is_separating_axis ====================

#[test]
fn test_is_separating_axis_separating() {
    let a_pos = [0.0, 0.0];
    let b_pos = [50.0, 0.0];
    let a_points: Vec<[f64; 2]> = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
    let b_points: Vec<[f64; 2]> = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
    let axis = [1.0, 0.0];
    let mut r = satc_response_create();
    let sep = satc_is_separating_axis(
        &a_pos, &b_pos, 4, &a_points, 4, &b_points, &axis, &mut r,
    );
    assert!(sep);
}

#[test]
fn test_is_separating_axis_overlap() {
    // Wide A; B is fully inside on x-axis when offset is 5
    let a_pos = [0.0, 0.0];
    let b_pos = [5.0, 0.0];
    let a_points: Vec<[f64; 2]> = vec![[0.0, 0.0], [100.0, 0.0], [100.0, 10.0], [0.0, 10.0]];
    let b_points: Vec<[f64; 2]> = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
    let axis = [1.0, 0.0];
    let mut r = satc_response_create();
    let sep = satc_is_separating_axis(
        &a_pos, &b_pos, 4, &a_points, 4, &b_points, &axis, &mut r,
    );
    assert!(!sep);
    approx_eq(r.overlap, 15.0);
    approx_eq(r.overlap_n[0], -1.0);
    approx_eq(r.overlap_n[1], 0.0);
    assert!(!r.a_in_b);
    assert!(r.b_in_a);
}

fn main() {}
