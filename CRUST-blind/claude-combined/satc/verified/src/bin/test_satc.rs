use satc::satc::*;

fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() <= eps
}

#[test]
fn test_satc_point_create() {
    let p = satc_point_create(3.5, -2.0);
    assert_eq!(p[0], 3.5);
    assert_eq!(p[1], -2.0);
}

#[test]
fn test_satc_point_copy() {
    let p = [0.0, 0.0];
    let q = [7.0, 9.0];
    let result = satc_point_copy(&p, &q);
    assert_eq!(result[0], 7.0);
    assert_eq!(result[1], 9.0);
}

#[test]
fn test_satc_point_perp() {
    // perp of (3, 4) -> (4, -3)
    let p = [3.0, 4.0];
    let result = satc_point_perp(&p);
    assert_eq!(result[0], 4.0);
    assert_eq!(result[1], -3.0);
}

#[test]
fn test_satc_point_reverse() {
    // reverse of (3, 4) -> (-3, -4)
    let p = [3.0, 4.0];
    let result = satc_point_reverse(&p);
    assert_eq!(result[0], -3.0);
    assert_eq!(result[1], -4.0);
}

#[test]
fn test_satc_point_add() {
    // (1,2) + (3,4) -> (4, 6)
    let p = [1.0, 2.0];
    let q = [3.0, 4.0];
    let result = satc_point_add(&p, &q);
    assert_eq!(result[0], 4.0);
    assert_eq!(result[1], 6.0);
}

#[test]
fn test_satc_point_sub() {
    // (5,6) - (2,3) -> (3, 3)
    let p = [5.0, 6.0];
    let q = [2.0, 3.0];
    let result = satc_point_sub(&p, &q);
    assert_eq!(result[0], 3.0);
    assert_eq!(result[1], 3.0);
}

#[test]
fn test_satc_point_scale_xy() {
    // (5,5) -> (50, 50) -> (0, 50) -> (0, 0)
    let v = [5.0, 5.0];
    let v = satc_point_scale_xy(&v, 10.0, 10.0);
    assert_eq!(v[0], 50.0);
    assert_eq!(v[1], 50.0);
    let v = satc_point_scale_xy(&v, 0.0, 1.0);
    assert_eq!(v[0], 0.0);
    assert_eq!(v[1], 50.0);
    let v = satc_point_scale_xy(&v, 1.0, 0.0);
    assert_eq!(v[0], 0.0);
    assert_eq!(v[1], 0.0);
}

#[test]
fn test_satc_point_scale_x() {
    // scale (2,3) by 5 -> (10, 15)
    let p = [2.0, 3.0];
    let result = satc_point_scale_x(&p, 5.0);
    assert_eq!(result[0], 10.0);
    assert_eq!(result[1], 15.0);
}

#[test]
fn test_satc_point_rotate() {
    // rotate (1, 0) by PI/2: (0, 1) — note C has bug: y = x*sin - y*cos
    let p = [1.0, 0.0];
    let result = satc_point_rotate(&p, std::f64::consts::PI / 2.0);
    assert!(approx_eq(result[0], 0.0, 1e-9));
    assert!(approx_eq(result[1], 1.0, 1e-9));

    // rotate (1, 0) by 0 -> (1, 0)
    let p = [1.0, 0.0];
    let result = satc_point_rotate(&p, 0.0);
    assert_eq!(result[0], 1.0);
    assert_eq!(result[1], 0.0);
}

#[test]
fn test_satc_point_normalize() {
    // (3, 4) -> (0.6, 0.8)
    let p = [3.0, 4.0];
    let result = satc_point_normalize(&p);
    assert!(approx_eq(result[0], 0.6, 1e-12));
    assert!(approx_eq(result[1], 0.8, 1e-12));
}

#[test]
fn test_satc_point_normalize_zero() {
    // (0,0) -> stays (0,0) (no division)
    let p = [0.0, 0.0];
    let result = satc_point_normalize(&p);
    assert_eq!(result[0], 0.0);
    assert_eq!(result[1], 0.0);
}

#[test]
fn test_satc_point_project() {
    // project p=(2,3) onto q=(1,0) -> (4, 6)
    let p = [2.0, 3.0];
    let q = [1.0, 0.0];
    let result = satc_point_project(&p, &q);
    assert_eq!(result[0], 4.0);
    assert_eq!(result[1], 6.0);
}

#[test]
fn test_satc_point_reflect() {
    // reflect p=(2,3) along axis=(1,0) -> (6, 9)
    let p = [2.0, 3.0];
    let axis = [1.0, 0.0];
    let result = satc_point_reflect(&p, &axis);
    assert_eq!(result[0], 6.0);
    assert_eq!(result[1], 9.0);
}

#[test]
fn test_satc_voronoi_region() {
    let line = [10.0, 0.0];

    let p_left = [-5.0, 3.0];
    assert_eq!(satc_voronoi_region(&line, &p_left), SATC_LEFT_VORONOI_REGION);
    assert_eq!(satc_voronoi_region(&line, &p_left), -1);

    let p_mid = [5.0, 3.0];
    assert_eq!(satc_voronoi_region(&line, &p_mid), SATC_MIDDLE_VORONOI_REGION);
    assert_eq!(satc_voronoi_region(&line, &p_mid), 0);

    let p_right = [15.0, 3.0];
    assert_eq!(satc_voronoi_region(&line, &p_right), SATC_RIGHT_VORONOI_REGION);
    assert_eq!(satc_voronoi_region(&line, &p_right), 1);
}

#[test]
fn test_satc_flatten_points_on() {
    let pts = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
    let normal = [1.0, 0.0];
    let mut result = [0.0, 0.0];
    satc_flatten_points_on(4, &pts, &normal, &mut result);
    assert_eq!(result[0], 0.0);
    assert_eq!(result[1], 10.0);
}

#[test]
fn test_satc_circle_create() {
    let c = satc_circle_create([10.0, 20.0], 5.5);
    assert_eq!(c.pos[0], 10.0);
    assert_eq!(c.pos[1], 20.0);
    assert_eq!(c.r, 5.5);
}

#[test]
fn test_satc_circle_get_aabb() {
    let c = satc_circle_create([5.0, 6.0], 3.0);
    let aabb = satc_circle_get_aabb(&c);
    assert_eq!(aabb.pos[0], 2.0);
    assert_eq!(aabb.pos[1], 3.0);
    assert_eq!(aabb.num_points, 4);
    assert_eq!(aabb.points[0][0], 0.0);
    assert_eq!(aabb.points[0][1], 0.0);
    assert_eq!(aabb.points[1][0], 6.0);
    assert_eq!(aabb.points[1][1], 0.0);
    assert_eq!(aabb.points[2][0], 6.0);
    assert_eq!(aabb.points[2][1], 6.0);
    assert_eq!(aabb.points[3][0], 0.0);
    assert_eq!(aabb.points[3][1], 6.0);
}

#[test]
fn test_satc_polygon_create() {
    let pos = [0.0, 0.0];
    let pts = vec![[0.0, 0.0], [40.0, 0.0], [40.0, 40.0], [0.0, 40.0]];
    let polygon = satc_polygon_create(pos, pts);
    assert_eq!(polygon.pos[0], 0.0);
    assert_eq!(polygon.pos[1], 0.0);
    assert_eq!(polygon.num_points, 4);
    assert_eq!(polygon.points.len(), 4);
    assert_eq!(polygon.points[0], [0.0, 0.0]);
    assert_eq!(polygon.points[1], [40.0, 0.0]);
    assert_eq!(polygon.angle, 0.0);
    assert_eq!(polygon.offset[0], 0.0);
    assert_eq!(polygon.offset[1], 0.0);
    // calc_points should match points (offset=0, angle=0)
    assert_eq!(polygon.calc_points.len(), 4);
    for i in 0..4 {
        assert_eq!(polygon.calc_points[i], polygon.points[i]);
    }
    // Edges
    assert_eq!(polygon.edges.len(), 4);
    assert_eq!(polygon.edges[0], [40.0, 0.0]);
    assert_eq!(polygon.edges[1], [0.0, 40.0]);
    assert_eq!(polygon.edges[2], [-40.0, 0.0]);
    assert_eq!(polygon.edges[3], [0.0, -40.0]);
    // Normals (perp + normalize): edge (40,0) -> perp (0,-40) -> normalized (0,-1)
    assert_eq!(polygon.normals.len(), 4);
    assert!(approx_eq(polygon.normals[0][0], 0.0, 1e-12));
    assert!(approx_eq(polygon.normals[0][1], -1.0, 1e-12));
    assert!(approx_eq(polygon.normals[1][0], 1.0, 1e-12));
    assert!(approx_eq(polygon.normals[1][1], 0.0, 1e-12));
    assert!(approx_eq(polygon.normals[2][0], 0.0, 1e-12));
    assert!(approx_eq(polygon.normals[2][1], 1.0, 1e-12));
    assert!(approx_eq(polygon.normals[3][0], -1.0, 1e-12));
    assert!(approx_eq(polygon.normals[3][1], 0.0, 1e-12));
}

#[test]
fn test_satc_polygon_get_aabb() {
    let pos = [0.0, 0.0];
    let pts = vec![[1.0, 2.0], [5.0, 4.0], [3.0, 8.0]];
    let polygon = satc_polygon_create(pos, pts);
    let aabb = satc_polygon_get_aabb(&polygon);
    assert_eq!(aabb.pos[0], 1.0);
    assert_eq!(aabb.pos[1], 2.0);
    assert_eq!(aabb.num_points, 4);
    // Box from (0,0) to (4, 6)
    assert_eq!(aabb.points[0], [0.0, 0.0]);
    assert_eq!(aabb.points[1], [4.0, 0.0]);
    assert_eq!(aabb.points[2], [4.0, 6.0]);
    assert_eq!(aabb.points[3], [0.0, 6.0]);
}

#[test]
fn test_satc_polygon_get_centroid_square() {
    // Square 0,0 -> 40,40, centroid is (20, 20)
    let pos = [0.0, 0.0];
    let pts = vec![[0.0, 0.0], [40.0, 0.0], [40.0, 40.0], [0.0, 40.0]];
    let polygon = satc_polygon_create(pos, pts);
    let centroid = satc_polygon_get_centroid(&polygon);
    assert_eq!(centroid[0], 20.0);
    assert_eq!(centroid[1], 20.0);
}

#[test]
fn test_satc_polygon_get_centroid_triangle() {
    // Triangle (0,0),(100,0),(50,99). centroid=(50, 33).
    let pos = [0.0, 0.0];
    let pts = vec![[0.0, 0.0], [100.0, 0.0], [50.0, 99.0]];
    let polygon = satc_polygon_create(pos, pts);
    let centroid = satc_polygon_get_centroid(&polygon);
    assert_eq!(centroid[0], 50.0);
    assert_eq!(centroid[1], 33.0);
}

#[test]
fn test_satc_response_create() {
    let r = satc_response_create();
    assert_eq!(r.overlap, std::f64::MAX);
    assert_eq!(r.overlap_n[0], 0.0);
    assert_eq!(r.overlap_n[1], 0.0);
    assert_eq!(r.overlap_v[0], 0.0);
    assert_eq!(r.overlap_v[1], 0.0);
    assert!(r.a_in_b);
    assert!(r.b_in_a);
}

#[test]
fn test_satc_box_create() {
    let b = satc_box_create([1.0, 2.0], 3.0, 4.0);
    assert_eq!(b.pos[0], 1.0);
    assert_eq!(b.pos[1], 2.0);
    assert_eq!(b.w, 3.0);
    assert_eq!(b.h, 4.0);
}

#[test]
fn test_satc_box_to_polygon() {
    let b = satc_box_create([1.5, 2.5], 3.0, 4.0);
    let pg = satc_box_to_polygon(&b);
    assert_eq!(pg.num_points, 4);
    assert_eq!(pg.pos[0], 1.5);
    assert_eq!(pg.pos[1], 2.5);
    assert_eq!(pg.points[0], [0.0, 0.0]);
    assert_eq!(pg.points[1], [3.0, 0.0]);
    assert_eq!(pg.points[2], [3.0, 4.0]);
    assert_eq!(pg.points[3], [0.0, 4.0]);

    // calc_points equal points
    assert_eq!(pg.calc_points[0], [0.0, 0.0]);
    assert_eq!(pg.calc_points[1], [3.0, 0.0]);
    assert_eq!(pg.calc_points[2], [3.0, 4.0]);
    assert_eq!(pg.calc_points[3], [0.0, 4.0]);

    // Edges
    assert_eq!(pg.edges[0], [3.0, 0.0]);
    assert_eq!(pg.edges[1], [0.0, 4.0]);
    assert_eq!(pg.edges[2], [-3.0, 0.0]);
    assert_eq!(pg.edges[3], [0.0, -4.0]);

    // Normals
    assert!(approx_eq(pg.normals[0][0], 0.0, 1e-12));
    assert!(approx_eq(pg.normals[0][1], -1.0, 1e-12));
    assert!(approx_eq(pg.normals[1][0], 1.0, 1e-12));
    assert!(approx_eq(pg.normals[1][1], 0.0, 1e-12));
    assert!(approx_eq(pg.normals[2][0], 0.0, 1e-12));
    assert!(approx_eq(pg.normals[2][1], 1.0, 1e-12));
    assert!(approx_eq(pg.normals[3][0], -1.0, 1e-12));
    assert!(approx_eq(pg.normals[3][1], 0.0, 1e-12));
}

#[test]
fn test_satc_polygon_set_points() {
    let pos = [0.0, 0.0];
    let pts = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
    let mut polygon = satc_polygon_create(pos, pts);
    let new_pts = vec![[0.0, 0.0], [5.0, 0.0], [5.0, 5.0]];
    satc_polygon_set_points(&mut polygon, new_pts);
    assert_eq!(polygon.num_points, 3);
    assert_eq!(polygon.points.len(), 3);
    assert_eq!(polygon.calc_points.len(), 3);
    assert_eq!(polygon.edges.len(), 3);
    assert_eq!(polygon.normals.len(), 3);
    assert_eq!(polygon.points[0], [0.0, 0.0]);
    assert_eq!(polygon.points[1], [5.0, 0.0]);
    assert_eq!(polygon.points[2], [5.0, 5.0]);
}

#[test]
fn test_satc_point_in_circle() {
    let c = satc_circle_create([100.0, 100.0], 20.0);
    let p1 = [0.0, 0.0];
    assert!(!satc_point_in_circle(&p1, &c));
    let p2 = [110.0, 110.0];
    assert!(satc_point_in_circle(&p2, &c));
    let p3 = [100.0, 80.0]; // exactly on edge
    assert!(satc_point_in_circle(&p3, &c));
}

#[test]
fn test_satc_point_in_polygon() {
    // Triangle test
    let triangle_pos = [30.0, 0.0];
    let pts = vec![[0.0, 0.0], [30.0, 0.0], [0.0, 30.0]];
    let triangle = satc_polygon_create(triangle_pos, pts);
    let p1 = [0.0, 0.0];
    assert!(!satc_point_in_polygon(&p1, &triangle));
    let p2 = [35.0, 5.0];
    assert!(satc_point_in_polygon(&p2, &triangle));
}

#[test]
fn test_satc_point_in_polygon_small() {
    let polygon_pos = [0.0, 0.0];
    let pts = vec![
        [2.0, 1.0],
        [2.0, 2.0],
        [1.0, 3.0],
        [0.0, 2.0],
        [0.0, 1.0],
        [1.0, 0.0],
    ];
    let polygon = satc_polygon_create(polygon_pos, pts);
    let p = [1.0, 1.1];
    assert!(satc_point_in_polygon(&p, &polygon));
}

#[test]
fn test_satc_test_circle_circle_collide() {
    let c1 = satc_circle_create([0.0, 0.0], 20.0);
    let c2 = satc_circle_create([30.0, 0.0], 20.0);
    let mut response = satc_response_create();
    let collided = satc_test_circle_circle(&c1, &c2, &mut response);
    assert!(collided);
    assert_eq!(response.overlap_v[0], 10.0);
    assert_eq!(response.overlap_v[1], 0.0);
    assert_eq!(response.overlap, 10.0);
    assert_eq!(response.overlap_n[0], 1.0);
    assert_eq!(response.overlap_n[1], 0.0);
    assert!(!response.a_in_b);
    assert!(!response.b_in_a);
}

#[test]
fn test_satc_test_circle_circle_no_collide() {
    let c1 = satc_circle_create([0.0, 0.0], 10.0);
    let c2 = satc_circle_create([100.0, 0.0], 10.0);
    let mut response = satc_response_create();
    let collided = satc_test_circle_circle(&c1, &c2, &mut response);
    assert!(!collided);
}

#[test]
fn test_satc_test_polygon_polygon_collide() {
    // Two boxes overlapping by 5 in x-direction
    let b1 = satc_box_create([0.0, 0.0], 20.0, 20.0);
    let b2 = satc_box_create([5.0, 0.0], 20.0, 20.0);
    let pg1 = satc_box_to_polygon(&b1);
    let pg2 = satc_box_to_polygon(&b2);
    let mut response = satc_response_create();
    let collided = satc_test_polygon_polygon(&pg1, &pg2, &mut response);
    assert!(collided);
    assert!(approx_eq(response.overlap, 5.0, 1e-9));
    assert!(approx_eq(response.overlap_n[0], -1.0, 1e-9));
    assert!(approx_eq(response.overlap_n[1], 0.0, 1e-9));
    assert!(approx_eq(response.overlap_v[0], -5.0, 1e-9));
    assert!(approx_eq(response.overlap_v[1], 0.0, 1e-9));
    assert!(!response.a_in_b);
    assert!(!response.b_in_a);
}

#[test]
fn test_satc_test_polygon_polygon_no_collide() {
    let b1 = satc_box_create([0.0, 0.0], 20.0, 20.0);
    let b2 = satc_box_create([100.0, 100.0], 20.0, 20.0);
    let pg1 = satc_box_to_polygon(&b1);
    let pg2 = satc_box_to_polygon(&b2);
    let mut response = satc_response_create();
    let collided = satc_test_polygon_polygon(&pg1, &pg2, &mut response);
    assert!(!collided);
}

#[test]
fn test_satc_test_polygon_circle() {
    let circle = satc_circle_create([50.0, 50.0], 20.0);
    let pos = [0.0, 0.0];
    let pts = vec![[0.0, 0.0], [40.0, 0.0], [40.0, 40.0], [0.0, 40.0]];
    let polygon = satc_polygon_create(pos, pts);
    let mut response = satc_response_create();
    let collided = satc_test_polygon_circle(&polygon, &circle, &mut response);
    assert!(collided);
    let nearest_hundredth = |n: f64| (n * 100.0 + 0.5).floor() / 100.0;
    assert_eq!(nearest_hundredth(response.overlap), 5.86);
    assert_eq!(nearest_hundredth(response.overlap_v[0]), 4.14);
    assert_eq!(nearest_hundredth(response.overlap_v[1]), 4.14);
    assert!(!response.a_in_b);
    assert!(!response.b_in_a);
}

#[test]
fn test_satc_test_circle_polygon() {
    let circle = satc_circle_create([50.0, 50.0], 20.0);
    let pos = [0.0, 0.0];
    let pts = vec![[0.0, 0.0], [40.0, 0.0], [40.0, 40.0], [0.0, 40.0]];
    let polygon = satc_polygon_create(pos, pts);
    let mut response = satc_response_create();
    let collided = satc_test_circle_polygon(&circle, &polygon, &mut response);
    assert!(collided);
    // C ground truth: overlap=5.8578643763, overlap_n=(-0.7071, -0.7071), overlap_v=(-4.1421, -4.1421)
    let nearest_hundredth = |n: f64| (n * 100.0 + 0.5).floor() / 100.0;
    assert_eq!(nearest_hundredth(response.overlap), 5.86);
    assert!(approx_eq(response.overlap_n[0], -0.7071067812, 1e-6));
    assert!(approx_eq(response.overlap_n[1], -0.7071067812, 1e-6));
    assert!(approx_eq(response.overlap_v[0], -4.1421356237, 1e-6));
    assert!(approx_eq(response.overlap_v[1], -4.1421356237, 1e-6));
    // a_in_b/b_in_a are swapped compared to test_polygon_circle.
    assert!(!response.a_in_b);
    assert!(!response.b_in_a);
}

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

#[test]
fn test_satc_is_separating_axis_separating() {
    // Two boxes that are not overlapping, axis is x-axis normal.
    let a_pos = [0.0, 0.0];
    let b_pos = [100.0, 0.0];
    let a_pts = vec![[0.0, 0.0], [20.0, 0.0], [20.0, 20.0], [0.0, 20.0]];
    let b_pts = vec![[0.0, 0.0], [20.0, 0.0], [20.0, 20.0], [0.0, 20.0]];
    let axis = [1.0, 0.0];
    let mut response = satc_response_create();
    let result = satc_is_separating_axis(&a_pos, &b_pos, 4, &a_pts, 4, &b_pts, &axis, &mut response);
    assert!(result);
}

#[test]
fn test_satc_is_separating_axis_not_separating() {
    // Two overlapping boxes
    let a_pos = [0.0, 0.0];
    let b_pos = [5.0, 0.0];
    let a_pts = vec![[0.0, 0.0], [20.0, 0.0], [20.0, 20.0], [0.0, 20.0]];
    let b_pts = vec![[0.0, 0.0], [20.0, 0.0], [20.0, 20.0], [0.0, 20.0]];
    let axis = [1.0, 0.0];
    let mut response = satc_response_create();
    let result = satc_is_separating_axis(&a_pos, &b_pos, 4, &a_pts, 4, &b_pts, &axis, &mut response);
    assert!(!result);
    // After this call, response.overlap should be the smallest overlap so far.
    // range_a = [0, 20], range_b shifted by 5 = [5, 25]. range_a[0] < range_b[0],
    // range_a[1] (20) < range_b[1] (25) -> overlap = 20-25 = -5.
    // abs_overlap = 5, less than f64::MAX, so set overlap = 5, a_in_b = false, b_in_a = false.
    assert_eq!(response.overlap, 5.0);
    assert!(!response.a_in_b);
    assert!(!response.b_in_a);
    // overlap was negative, so overlap_n is reversed axis = (-1, 0)
    assert_eq!(response.overlap_n[0], -1.0);
    assert_eq!(response.overlap_n[1], 0.0);
}

fn main() {}
