use satc::satc::*;

fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-10
}

// === Point operations ===

#[test]
fn test_point_create() {
    let p = satc_point_create(3.0, 4.0);
    assert_eq!(p, [3.0, 4.0]);
}

#[test]
fn test_point_copy() {
    let p = [1.0, 2.0];
    let q = [5.0, 6.0];
    let r = satc_point_copy(&p, &q);
    assert_eq!(r, [5.0, 6.0]);
}

#[test]
fn test_point_perp() {
    let p = [3.0, 4.0];
    let r = satc_point_perp(&p);
    assert_eq!(r, [4.0, -3.0]);
}

#[test]
fn test_point_reverse() {
    let p = [3.0, 4.0];
    let r = satc_point_reverse(&p);
    assert_eq!(r, [-3.0, -4.0]);
}

#[test]
fn test_point_add() {
    let p = [3.0, 4.0];
    let q = [1.0, 2.0];
    let r = satc_point_add(&p, &q);
    assert_eq!(r, [4.0, 6.0]);
}

#[test]
fn test_point_sub() {
    let p = [3.0, 4.0];
    let q = [1.0, 2.0];
    let r = satc_point_sub(&p, &q);
    assert_eq!(r, [2.0, 2.0]);
}

#[test]
fn test_point_scale_xy() {
    let p = [5.0, 5.0];
    let r = satc_point_scale_xy(&p, 10.0, 10.0);
    assert_eq!(r, [50.0, 50.0]);
}

#[test]
fn test_point_scale_xy_zero() {
    let p = [50.0, 50.0];
    let r = satc_point_scale_xy(&p, 0.0, 1.0);
    assert_eq!(r, [0.0, 50.0]);
}

#[test]
fn test_point_scale_x() {
    let p = [5.0, 5.0];
    let r = satc_point_scale_x(&p, 3.0);
    assert_eq!(r, [15.0, 15.0]);
}

#[test]
fn test_point_normalize() {
    let p = [3.0, 4.0];
    let r = satc_point_normalize(&p);
    assert!(approx(r[0], 0.6));
    assert!(approx(r[1], 0.8));
}

#[test]
fn test_point_normalize_zero() {
    let p = [0.0, 0.0];
    let r = satc_point_normalize(&p);
    assert_eq!(r, [0.0, 0.0]);
}

#[test]
fn test_point_rotate_zero() {
    // C: sin-cos pattern, rotating by 0 gives (x, -y)
    let p = [3.0, 4.0];
    let r = satc_point_rotate(&p, 0.0);
    assert_eq!(r[0], 3.0);
    assert_eq!(r[1], -4.0);
}

#[test]
fn test_point_rotate_pi4() {
    let p = [1.0, 0.0];
    let r = satc_point_rotate(&p, std::f64::consts::FRAC_PI_4);
    assert!(approx(r[0], 0.7071067811865476));
    assert!(approx(r[1], 0.7071067811865475));
}

#[test]
fn test_point_rotate_pi2() {
    let p = [1.0, 0.0];
    let r = satc_point_rotate(&p, std::f64::consts::FRAC_PI_2);
    assert!(approx(r[0], 0.0));
    assert!(approx(r[1], 1.0));
}

#[test]
fn test_point_rotate_pi() {
    let p = [1.0, 0.0];
    let r = satc_point_rotate(&p, std::f64::consts::PI);
    assert!(approx(r[0], -1.0));
    assert!(approx(r[1], 0.0));
}

#[test]
fn test_point_project() {
    // C: project (3,4) onto (1,0) => amt = 3/1 = 3, result = (3*3, 3*4) = (9, 12)
    let p = [3.0, 4.0];
    let q = [1.0, 0.0];
    let r = satc_point_project(&p, &q);
    assert_eq!(r, [9.0, 12.0]);
}

#[test]
fn test_point_reflect() {
    // C: reflect (2,3) along (1,0) => project gives amt=2, scaled (4,6), minus original => (2, 3)
    // Wait, C output was (6, 9)... let me re-check
    // project(p=(2,3), q=(1,0)): amt = dot((2,3),(1,0))/len2((1,0)) = 2/1 = 2, result = (2*2, 2*3) = (4, 6)
    // scale_x by 2: (8, 12)
    // subtract original: (8-2, 12-3) = (6, 9)
    let p = [2.0, 3.0];
    let axis = [1.0, 0.0];
    let r = satc_point_reflect(&p, &axis);
    assert_eq!(r, [6.0, 9.0]);
}

// === Voronoi region ===

#[test]
fn test_voronoi_region_left() {
    let line = [10.0, 0.0];
    let point = [-1.0, 0.0];
    assert_eq!(satc_voronoi_region(&line, &point), SATC_LEFT_VORONOI_REGION);
}

#[test]
fn test_voronoi_region_middle() {
    let line = [10.0, 0.0];
    let point = [5.0, 0.0];
    assert_eq!(satc_voronoi_region(&line, &point), SATC_MIDDLE_VORONOI_REGION);
}

#[test]
fn test_voronoi_region_right() {
    let line = [10.0, 0.0];
    let point = [200.0, 0.0];
    assert_eq!(satc_voronoi_region(&line, &point), SATC_RIGHT_VORONOI_REGION);
}

// === Box and polygon creation ===

#[test]
fn test_box_to_polygon() {
    let b = satc_box_create([5.0, 10.0], 30.0, 40.0);
    let poly = satc_box_to_polygon(&b);
    assert_eq!(poly.pos, [5.0, 10.0]);
    assert_eq!(poly.num_points, 4);
    assert_eq!(poly.points[0], [0.0, 0.0]);
    assert_eq!(poly.points[1], [30.0, 0.0]);
    assert_eq!(poly.points[2], [30.0, 40.0]);
    assert_eq!(poly.points[3], [0.0, 40.0]);
    assert_eq!(poly.calc_points[0], [0.0, 0.0]);
    assert_eq!(poly.calc_points[1], [30.0, 0.0]);
    assert_eq!(poly.edges[0], [30.0, 0.0]);
    assert_eq!(poly.edges[1], [0.0, 40.0]);
    assert!(approx(poly.normals[0][0], 0.0));
    assert!(approx(poly.normals[0][1], -1.0));
    assert!(approx(poly.normals[1][0], 1.0));
    assert!(approx(poly.normals[1][1], 0.0));
}

#[test]
fn test_polygon_create() {
    let points = vec![[0.0, 0.0], [10.0, 0.0], [5.0, 10.0]];
    let poly = satc_polygon_create([0.0, 0.0], points);
    assert_eq!(poly.num_points, 3);
    assert_eq!(poly.points[0], [0.0, 0.0]);
    assert_eq!(poly.points[1], [10.0, 0.0]);
    assert_eq!(poly.points[2], [5.0, 10.0]);
    // calc_points should equal points (no offset, no angle)
    assert_eq!(poly.calc_points[0], [0.0, 0.0]);
    assert_eq!(poly.calc_points[1], [10.0, 0.0]);
    assert_eq!(poly.calc_points[2], [5.0, 10.0]);
}

#[test]
fn test_polygon_with_offset() {
    let points = vec![[0.0, 0.0], [10.0, 0.0], [5.0, 10.0]];
    let mut poly = satc_polygon_create([0.0, 0.0], points);
    poly.offset = [5.0, 5.0];
    let pts = poly.points.clone();
    satc_polygon_set_points(&mut poly, pts);
    assert_eq!(poly.calc_points[0], [5.0, 5.0]);
    assert_eq!(poly.calc_points[1], [15.0, 5.0]);
    assert_eq!(poly.calc_points[2], [10.0, 15.0]);
}

// === Polygon centroid ===

#[test]
fn test_polygon_centroid_square() {
    let points = vec![[0.0, 0.0], [40.0, 0.0], [40.0, 40.0], [0.0, 40.0]];
    let poly = satc_polygon_create([0.0, 0.0], points);
    let c = satc_polygon_get_centroid(&poly);
    assert_eq!(c, [20.0, 20.0]);
}

#[test]
fn test_polygon_centroid_triangle() {
    let points = vec![[0.0, 0.0], [100.0, 0.0], [50.0, 99.0]];
    let poly = satc_polygon_create([0.0, 0.0], points);
    let c = satc_polygon_get_centroid(&poly);
    assert_eq!(c, [50.0, 33.0]);
}

// === Circle AABB ===

#[test]
fn test_circle_get_aabb() {
    let c = satc_circle_create([10.0, 20.0], 5.0);
    let aabb = satc_circle_get_aabb(&c);
    assert_eq!(aabb.pos, [5.0, 15.0]);
    assert_eq!(aabb.points[0], [0.0, 0.0]);
    assert_eq!(aabb.points[1], [10.0, 0.0]);
    assert_eq!(aabb.points[2], [10.0, 10.0]);
    assert_eq!(aabb.points[3], [0.0, 10.0]);
}

// === Polygon AABB (has y_max=x_max bug from C) ===

#[test]
fn test_polygon_get_aabb() {
    let points = vec![[1.0, 2.0], [5.0, 1.0], [3.0, 7.0]];
    let poly = satc_polygon_create([0.0, 0.0], points);
    let aabb = satc_polygon_get_aabb(&poly);
    assert_eq!(aabb.pos, [1.0, 1.0]);
    assert_eq!(aabb.points[0], [0.0, 0.0]);
    assert_eq!(aabb.points[1], [4.0, 0.0]);
    assert_eq!(aabb.points[2], [4.0, 6.0]);
    assert_eq!(aabb.points[3], [0.0, 6.0]);
}

// === flatten_points_on ===

#[test]
fn test_flatten_points_on_x() {
    let pts = vec![[1.0, 2.0], [3.0, 4.0], [5.0, 0.0]];
    let normal = [1.0, 0.0];
    let mut result = [0.0, 0.0];
    satc_flatten_points_on(3, &pts, &normal, &mut result);
    assert_eq!(result, [1.0, 5.0]);
}

#[test]
fn test_flatten_points_on_y() {
    let pts = vec![[1.0, 2.0], [3.0, 4.0], [5.0, 0.0]];
    let normal = [0.0, 1.0];
    let mut result = [0.0, 0.0];
    satc_flatten_points_on(3, &pts, &normal, &mut result);
    assert_eq!(result, [0.0, 4.0]);
}

// === is_separating_axis ===

#[test]
fn test_is_separating_axis_overlap() {
    let a_pos = [0.0, 0.0];
    let b_pos = [5.0, 0.0];
    let a_pts = vec![[0.0, 0.0], [20.0, 0.0], [20.0, 20.0], [0.0, 20.0]];
    let b_pts = vec![[0.0, 0.0], [20.0, 0.0], [20.0, 20.0], [0.0, 20.0]];
    let axis = [1.0, 0.0];
    let mut r = satc_response_create();
    let sep = satc_is_separating_axis(&a_pos, &b_pos, 4, &a_pts, 4, &b_pts, &axis, &mut r);
    assert!(!sep);
    assert_eq!(r.overlap, 5.0);
    assert_eq!(r.overlap_n[0], -1.0);
    assert_eq!(r.overlap_n[1], 0.0);
    assert!(!r.a_in_b);
    assert!(!r.b_in_a);
}

#[test]
fn test_is_separating_axis_separated() {
    let a_pos = [0.0, 0.0];
    let b_pos = [50.0, 0.0];
    let a_pts = vec![[0.0, 0.0], [20.0, 0.0], [20.0, 20.0], [0.0, 20.0]];
    let b_pts = vec![[0.0, 0.0], [20.0, 0.0], [20.0, 20.0], [0.0, 20.0]];
    let axis = [1.0, 0.0];
    let mut r = satc_response_create();
    let sep = satc_is_separating_axis(&a_pos, &b_pos, 4, &a_pts, 4, &b_pts, &axis, &mut r);
    assert!(sep);
}

// === Point in circle ===

#[test]
fn test_point_in_circle_outside() {
    let c = satc_circle_create([100.0, 100.0], 20.0);
    assert!(!satc_point_in_circle(&[0.0, 0.0], &c));
}

#[test]
fn test_point_in_circle_inside() {
    let c = satc_circle_create([100.0, 100.0], 20.0);
    assert!(satc_point_in_circle(&[110.0, 110.0], &c));
}

#[test]
fn test_point_in_circle_boundary() {
    let c = satc_circle_create([0.0, 0.0], 10.0);
    assert!(satc_point_in_circle(&[10.0, 0.0], &c));
}

// === Point in polygon ===

#[test]
fn test_point_in_polygon_outside() {
    let points = vec![[0.0, 0.0], [30.0, 0.0], [0.0, 30.0]];
    let tri = satc_polygon_create([30.0, 0.0], points);
    assert!(!satc_point_in_polygon(&[0.0, 0.0], &tri));
}

#[test]
fn test_point_in_polygon_inside() {
    let points = vec![[0.0, 0.0], [30.0, 0.0], [0.0, 30.0]];
    let tri = satc_polygon_create([30.0, 0.0], points);
    assert!(satc_point_in_polygon(&[35.0, 5.0], &tri));
}

#[test]
fn test_point_in_polygon_hexagon() {
    let points = vec![
        [2.0, 1.0], [2.0, 2.0], [1.0, 3.0],
        [0.0, 2.0], [0.0, 1.0], [1.0, 0.0],
    ];
    let poly = satc_polygon_create([0.0, 0.0], points);
    assert!(satc_point_in_polygon(&[1.0, 1.1], &poly));
}

// === Circle-circle collision ===

#[test]
fn test_circle_circle_collision() {
    let c1 = satc_circle_create([0.0, 0.0], 20.0);
    let c2 = satc_circle_create([30.0, 0.0], 20.0);
    let mut r = satc_response_create();
    let col = satc_test_circle_circle(&c1, &c2, &mut r);
    assert!(col);
    assert_eq!(r.overlap, 10.0);
    assert_eq!(r.overlap_n, [1.0, 0.0]);
    assert_eq!(r.overlap_v, [10.0, 0.0]);
    assert!(!r.a_in_b);
    assert!(!r.b_in_a);
}

#[test]
fn test_circle_circle_no_collision() {
    let c1 = satc_circle_create([0.0, 0.0], 5.0);
    let c2 = satc_circle_create([100.0, 0.0], 5.0);
    let mut r = satc_response_create();
    let col = satc_test_circle_circle(&c1, &c2, &mut r);
    assert!(!col);
}

#[test]
fn test_circle_circle_contained() {
    let c1 = satc_circle_create([0.0, 0.0], 5.0);
    let c2 = satc_circle_create([1.0, 0.0], 50.0);
    let mut r = satc_response_create();
    let col = satc_test_circle_circle(&c1, &c2, &mut r);
    assert!(col);
    assert_eq!(r.overlap, 54.0);
    assert!(r.a_in_b);
    assert!(!r.b_in_a);
}

// === Polygon-circle collision ===

#[test]
fn test_polygon_circle_collision() {
    let points = vec![[0.0, 0.0], [40.0, 0.0], [40.0, 40.0], [0.0, 40.0]];
    let poly = satc_polygon_create([0.0, 0.0], points);
    let circle = satc_circle_create([50.0, 50.0], 20.0);
    let mut r = satc_response_create();
    let col = satc_test_polygon_circle(&poly, &circle, &mut r);
    assert!(col);
    let nh = |n: f64| (n * 100.0 + 0.5).floor() / 100.0;
    assert_eq!(nh(r.overlap), 5.86);
    assert!(approx(r.overlap_n[0], 0.7071067811865475));
    assert!(approx(r.overlap_n[1], 0.7071067811865475));
    assert_eq!(nh(r.overlap_v[0]), 4.14);
    assert_eq!(nh(r.overlap_v[1]), 4.14);
    assert!(!r.a_in_b);
    assert!(!r.b_in_a);
}

#[test]
fn test_polygon_circle_no_collision() {
    let points = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
    let poly = satc_polygon_create([0.0, 0.0], points);
    let circle = satc_circle_create([200.0, 200.0], 5.0);
    let mut r = satc_response_create();
    let col = satc_test_polygon_circle(&poly, &circle, &mut r);
    assert!(!col);
}

#[test]
fn test_polygon_circle_inside() {
    let points = vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]];
    let poly = satc_polygon_create([0.0, 0.0], points);
    let circle = satc_circle_create([5.0, 5.0], 2.0);
    let mut r = satc_response_create();
    let col = satc_test_polygon_circle(&poly, &circle, &mut r);
    assert!(col);
    assert_eq!(r.overlap, 7.0);
    assert!(approx(r.overlap_n[0], 0.0));
    assert!(approx(r.overlap_n[1], -1.0));
    assert!(!r.a_in_b);
    assert!(r.b_in_a);
}

// === Circle-polygon collision (reversed) ===

#[test]
fn test_circle_polygon_collision() {
    let points = vec![[0.0, 0.0], [40.0, 0.0], [40.0, 40.0], [0.0, 40.0]];
    let poly = satc_polygon_create([0.0, 0.0], points);
    let circle = satc_circle_create([50.0, 50.0], 20.0);
    let mut r = satc_response_create();
    let col = satc_test_circle_polygon(&circle, &poly, &mut r);
    assert!(col);
    let nh = |n: f64| (n * 100.0 + 0.5).floor() / 100.0;
    assert_eq!(nh(r.overlap), 5.86);
    assert!(approx(r.overlap_n[0], -0.7071067811865475));
    assert!(approx(r.overlap_n[1], -0.7071067811865475));
    assert_eq!(nh(r.overlap_v[0]), -4.14);
    assert_eq!(nh(r.overlap_v[1]), -4.14);
    assert!(!r.a_in_b);
    assert!(!r.b_in_a);
}

// === Polygon-polygon collision ===

#[test]
fn test_polygon_polygon_no_collision() {
    let b1 = satc_box_create([0.0, 0.0], 20.0, 20.0);
    let b2 = satc_box_create([100.0, 100.0], 20.0, 20.0);
    let p1 = satc_box_to_polygon(&b1);
    let p2 = satc_box_to_polygon(&b2);
    let mut r = satc_response_create();
    let col = satc_test_polygon_polygon(&p1, &p2, &mut r);
    assert!(!col);
}

#[test]
fn test_polygon_polygon_collision() {
    let b1 = satc_box_create([0.0, 0.0], 20.0, 20.0);
    let b2 = satc_box_create([10.0, 10.0], 20.0, 20.0);
    let p1 = satc_box_to_polygon(&b1);
    let p2 = satc_box_to_polygon(&b2);
    let mut r = satc_response_create();
    let col = satc_test_polygon_polygon(&p1, &p2, &mut r);
    assert!(col);
    assert_eq!(r.overlap, 10.0);
    assert_eq!(r.overlap_n[1], 1.0);
    assert_eq!(r.overlap_v[1], 10.0);
    assert!(!r.a_in_b);
    assert!(!r.b_in_a);
}

#[test]
fn test_polygon_polygon_overlap_offset5() {
    let b1 = satc_box_create([0.0, 0.0], 20.0, 20.0);
    let b2 = satc_box_create([5.0, 0.0], 20.0, 20.0);
    let p1 = satc_box_to_polygon(&b1);
    let p2 = satc_box_to_polygon(&b2);
    let mut r = satc_response_create();
    let col = satc_test_polygon_polygon(&p1, &p2, &mut r);
    assert!(col);
    assert_eq!(r.overlap, 5.0);
    assert_eq!(r.overlap_n[0], -1.0);
    assert_eq!(r.overlap_v[0], -5.0);
    assert!(!r.a_in_b);
    assert!(!r.b_in_a);
}

#[test]
fn test_polygon_polygon_contained() {
    let b1 = satc_box_create([0.0, 0.0], 100.0, 100.0);
    let b2 = satc_box_create([5.0, 5.0], 10.0, 10.0);
    let p1 = satc_box_to_polygon(&b1);
    let p2 = satc_box_to_polygon(&b2);
    let mut r = satc_response_create();
    let col = satc_test_polygon_polygon(&p1, &p2, &mut r);
    assert!(col);
    assert_eq!(r.overlap, 15.0);
    assert!(approx(r.overlap_n[0], 0.0));
    assert!(approx(r.overlap_n[1], -1.0));
    assert_eq!(r.overlap_v[1], -15.0);
    assert!(!r.a_in_b);
    assert!(r.b_in_a);
}

// === Response creation ===

#[test]
fn test_response_create() {
    let r = satc_response_create();
    assert_eq!(r.overlap, f64::MAX);
    assert_eq!(r.overlap_n, [0.0, 0.0]);
    assert_eq!(r.overlap_v, [0.0, 0.0]);
    assert!(r.a_in_b);
    assert!(r.b_in_a);
}

// === Circle creation ===

#[test]
fn test_circle_create() {
    let c = satc_circle_create([10.0, 20.0], 5.0);
    assert_eq!(c.pos, [10.0, 20.0]);
    assert_eq!(c.r, 5.0);
}

// === SatcCircle::new ===

#[test]
fn test_circle_new() {
    let c = SatcCircle::new([1.0, 2.0], 3.0);
    assert_eq!(c.pos, [1.0, 2.0]);
    assert_eq!(c.r, 3.0);
}

// === SatcBox::new ===

#[test]
fn test_box_new() {
    let b = SatcBox::new([1.0, 2.0], 3.0, 4.0);
    assert_eq!(b.pos, [1.0, 2.0]);
    assert_eq!(b.w, 3.0);
    assert_eq!(b.h, 4.0);
}

// === Constants ===

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

fn main() {}
