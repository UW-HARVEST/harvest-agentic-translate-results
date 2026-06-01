#include <stdio.h>
#include "satc.h"

int main(void) {
    // 1. point operations
    {
        // perp of (3, 4) -> (4, -3)
        satc_point_alloca(p);
        satc_point_set_xy(p, 3.0, 4.0);
        satc_point_perp(p);
        printf("perp: %.10f %.10f\n", p[0], p[1]);
    }
    {
        // reverse of (3, 4) -> (-3, -4)
        satc_point_alloca(p);
        satc_point_set_xy(p, 3.0, 4.0);
        satc_point_reverse(p);
        printf("reverse: %.10f %.10f\n", p[0], p[1]);
    }
    {
        // add (1,2) + (3,4) -> (4,6)
        satc_point_alloca(p);
        satc_point_alloca(q);
        satc_point_set_xy(p, 1.0, 2.0);
        satc_point_set_xy(q, 3.0, 4.0);
        satc_point_add(p, q);
        printf("add: %.10f %.10f\n", p[0], p[1]);
    }
    {
        // sub (5,6) - (2,3) -> (3,3)
        satc_point_alloca(p);
        satc_point_alloca(q);
        satc_point_set_xy(p, 5.0, 6.0);
        satc_point_set_xy(q, 2.0, 3.0);
        satc_point_sub(p, q);
        printf("sub: %.10f %.10f\n", p[0], p[1]);
    }
    {
        // scale_xy (2,3) by (4, -1) -> (8, -3)
        satc_point_alloca(p);
        satc_point_set_xy(p, 2.0, 3.0);
        satc_point_scale_xy(p, 4.0, -1.0);
        printf("scale_xy: %.10f %.10f\n", p[0], p[1]);
    }
    {
        // scale_x (2,3) by 5 -> (10, 15)
        satc_point_alloca(p);
        satc_point_set_xy(p, 2.0, 3.0);
        satc_point_scale_x(p, 5.0);
        printf("scale_x: %.10f %.10f\n", p[0], p[1]);
    }
    {
        // rotate (1,0) by PI/2
        satc_point_alloca(p);
        satc_point_set_xy(p, 1.0, 0.0);
        satc_point_rotate(p, M_PI_2);
        printf("rotate (1,0) by PI/2: %.10f %.10f\n", p[0], p[1]);
    }
    {
        // rotate (1,0) by 0
        satc_point_alloca(p);
        satc_point_set_xy(p, 1.0, 0.0);
        satc_point_rotate(p, 0.0);
        printf("rotate (1,0) by 0: %.10f %.10f\n", p[0], p[1]);
    }
    {
        // normalize (3,4) -> (3/5, 4/5)
        satc_point_alloca(p);
        satc_point_set_xy(p, 3.0, 4.0);
        satc_point_normalize(p);
        printf("normalize: %.10f %.10f\n", p[0], p[1]);
    }
    {
        // normalize (0, 0)
        satc_point_alloca(p);
        satc_point_set_xy(p, 0.0, 0.0);
        satc_point_normalize(p);
        printf("normalize zero: %.10f %.10f\n", p[0], p[1]);
    }
    {
        // project p=(2,3) onto q=(1,0)
        satc_point_alloca(p);
        satc_point_alloca(q);
        satc_point_set_xy(p, 2.0, 3.0);
        satc_point_set_xy(q, 1.0, 0.0);
        satc_point_project(p, q);
        printf("project: %.10f %.10f\n", p[0], p[1]);
    }
    {
        // reflect p=(2,3) along axis=(1,0)
        satc_point_alloca(p);
        satc_point_alloca(axis);
        satc_point_set_xy(p, 2.0, 3.0);
        satc_point_set_xy(axis, 1.0, 0.0);
        satc_point_reflect(p, axis);
        printf("reflect: %.10f %.10f\n", p[0], p[1]);
    }

    // 2. voronoi
    {
        // line=(10,0). point=(-5,3) -> LEFT (-1)
        satc_point_alloca(line);
        satc_point_set_xy(line, 10.0, 0.0);
        satc_point_alloca(point);
        satc_point_set_xy(point, -5.0, 3.0);
        printf("voronoi left: %d\n", satc_voronoi_region(line, point));
        satc_point_set_xy(point, 5.0, 3.0);
        printf("voronoi middle: %d\n", satc_voronoi_region(line, point));
        satc_point_set_xy(point, 15.0, 3.0);
        printf("voronoi right: %d\n", satc_voronoi_region(line, point));
    }

    // 3. flatten_points_on
    {
        // points (0,0), (10,0), (10,10), (0,10), normal (1,0) -> [0, 10]
        satc_point_alloca(a); satc_point_set_xy(a, 0.0, 0.0);
        satc_point_alloca(b); satc_point_set_xy(b, 10.0, 0.0);
        satc_point_alloca(c); satc_point_set_xy(c, 10.0, 10.0);
        satc_point_alloca(d); satc_point_set_xy(d, 0.0, 10.0);
        satc_point_array_alloca(pts, 4);
        pts[0] = a; pts[1] = b; pts[2] = c; pts[3] = d;
        satc_point_alloca(normal); satc_point_set_xy(normal, 1.0, 0.0);
        satc_double_array_alloca(result, 2);
        satc_flatten_points_on(4, pts, normal, result);
        printf("flatten: %.10f %.10f\n", result[0], result[1]);
    }

    // 4. circle aabb
    {
        satc_point_alloca(c_pos); satc_point_set_xy(c_pos, 5.0, 6.0);
        satc_circle_t *c = satc_circle_create(c_pos, 3.0);
        satc_polygon_t *aabb = satc_circle_get_aabb(c);
        printf("circle_aabb pos: %.10f %.10f\n", aabb->pos[0], aabb->pos[1]);
        printf("circle_aabb num_points: %zu\n", aabb->num_points);
        for (size_t i = 0; i < aabb->num_points; i++) {
            printf("  pt%zu: %.10f %.10f\n", i, aabb->points[i][0], aabb->points[i][1]);
        }
        satc_polygon_destroy(aabb);
        satc_circle_destroy(c);
    }

    // 5. polygon get_aabb
    {
        satc_point_alloca(pos); satc_point_set_xy(pos, 0.0, 0.0);
        satc_point_alloca(a); satc_point_set_xy(a, 1.0, 2.0);
        satc_point_alloca(b); satc_point_set_xy(b, 5.0, 4.0);
        satc_point_alloca(c); satc_point_set_xy(c, 3.0, 8.0);
        satc_point_array_alloca(pts, 3);
        pts[0] = a; pts[1] = b; pts[2] = c;
        satc_polygon_t *poly = satc_polygon_create(pos, 3, pts);
        satc_polygon_t *aabb = satc_polygon_get_aabb(poly);
        printf("poly_aabb pos: %.10f %.10f\n", aabb->pos[0], aabb->pos[1]);
        printf("poly_aabb num_points: %zu\n", aabb->num_points);
        for (size_t i = 0; i < aabb->num_points; i++) {
            printf("  pt%zu: %.10f %.10f\n", i, aabb->points[i][0], aabb->points[i][1]);
        }
        satc_polygon_destroy(aabb);
        satc_polygon_destroy(poly);
    }

    // 6. point in circle
    {
        satc_point_alloca(c_pos); satc_point_set_xy(c_pos, 100.0, 100.0);
        satc_circle_t *c = satc_circle_create(c_pos, 20.0);
        satc_point_alloca(p1); satc_point_set_xy(p1, 0.0, 0.0);
        satc_point_alloca(p2); satc_point_set_xy(p2, 110.0, 110.0);
        satc_point_alloca(p3); satc_point_set_xy(p3, 100.0, 80.0); // on edge
        printf("point_in_circle p1: %d\n", satc_point_in_circle(p1, c));
        printf("point_in_circle p2: %d\n", satc_point_in_circle(p2, c));
        printf("point_in_circle p3: %d\n", satc_point_in_circle(p3, c));
        satc_circle_destroy(c);
    }

    // 7. circle-circle no collision
    {
        satc_point_alloca(p1); satc_point_set_xy(p1, 0.0, 0.0);
        satc_point_alloca(p2); satc_point_set_xy(p2, 100.0, 0.0);
        satc_circle_t *c1 = satc_circle_create(p1, 10.0);
        satc_circle_t *c2 = satc_circle_create(p2, 10.0);
        satc_response_t *r = satc_response_create();
        bool collided = satc_test_circle_circle(c1, c2, r);
        printf("circle_no_collide: %d\n", collided);
        satc_response_destroy(r);
        satc_circle_destroy(c2);
        satc_circle_destroy(c1);
    }

    // 8. circle-circle a_in_b
    {
        satc_point_alloca(p1); satc_point_set_xy(p1, 0.0, 0.0);
        satc_point_alloca(p2); satc_point_set_xy(p2, 0.0, 0.0);
        satc_circle_t *c1 = satc_circle_create(p1, 5.0);
        satc_circle_t *c2 = satc_circle_create(p2, 100.0);
        satc_response_t *r = satc_response_create();
        bool collided = satc_test_circle_circle(c1, c2, r);
        printf("circle_a_in_b: collided=%d a_in_b=%d b_in_a=%d overlap=%.10f overlap_v=%.10f,%.10f\n",
               collided, r->a_in_b, r->b_in_a, r->overlap, r->overlap_v[0], r->overlap_v[1]);
        satc_response_destroy(r);
        satc_circle_destroy(c2);
        satc_circle_destroy(c1);
    }

    // 9. polygon-polygon no collision
    {
        satc_point_alloca(p1); satc_point_set_xy(p1, 0.0, 0.0);
        satc_point_alloca(p2); satc_point_set_xy(p2, 100.0, 100.0);
        satc_box_t *b1 = satc_box_create(p1, 20.0, 20.0);
        satc_box_t *b2 = satc_box_create(p2, 20.0, 20.0);
        satc_polygon_t *pg1 = satc_box_to_polygon(b1);
        satc_polygon_t *pg2 = satc_box_to_polygon(b2);
        satc_response_t *r = satc_response_create();
        bool collided = satc_test_polygon_polygon(pg1, pg2, r);
        printf("poly_no_collide: %d\n", collided);
        satc_response_destroy(r);
        satc_polygon_destroy(pg2);
        satc_polygon_destroy(pg1);
        satc_box_destroy(b2);
        satc_box_destroy(b1);
    }

    // 10. polygon-polygon collision
    {
        satc_point_alloca(p1); satc_point_set_xy(p1, 0.0, 0.0);
        satc_point_alloca(p2); satc_point_set_xy(p2, 5.0, 0.0);
        satc_box_t *b1 = satc_box_create(p1, 20.0, 20.0);
        satc_box_t *b2 = satc_box_create(p2, 20.0, 20.0);
        satc_polygon_t *pg1 = satc_box_to_polygon(b1);
        satc_polygon_t *pg2 = satc_box_to_polygon(b2);
        satc_response_t *r = satc_response_create();
        bool collided = satc_test_polygon_polygon(pg1, pg2, r);
        printf("poly_collide: collided=%d overlap=%.10f overlap_n=%.10f,%.10f overlap_v=%.10f,%.10f a_in_b=%d b_in_a=%d\n",
               collided, r->overlap, r->overlap_n[0], r->overlap_n[1],
               r->overlap_v[0], r->overlap_v[1], r->a_in_b, r->b_in_a);
        satc_response_destroy(r);
        satc_polygon_destroy(pg2);
        satc_polygon_destroy(pg1);
        satc_box_destroy(b2);
        satc_box_destroy(b1);
    }

    // 11. circle-polygon
    {
        satc_point_alloca(c_pos); satc_point_set_xy(c_pos, 50.0, 50.0);
        satc_point_alloca(p_pos); satc_point_set_xy(p_pos, 0.0, 0.0);
        satc_point_alloca(a); satc_point_set_xy(a, 0.0, 0.0);
        satc_point_alloca(b); satc_point_set_xy(b, 40.0, 0.0);
        satc_point_alloca(c); satc_point_set_xy(c, 40.0, 40.0);
        satc_point_alloca(d); satc_point_set_xy(d, 0.0, 40.0);
        satc_point_array_alloca(pts, 4);
        pts[0]=a;pts[1]=b;pts[2]=c;pts[3]=d;
        satc_circle_t *circle = satc_circle_create(c_pos, 20.0);
        satc_polygon_t *polygon = satc_polygon_create(p_pos, 4, pts);
        satc_response_t *r = satc_response_create();
        bool collided = satc_test_circle_polygon(circle, polygon, r);
        printf("circle_polygon: collided=%d overlap=%.10f overlap_n=%.10f,%.10f overlap_v=%.10f,%.10f a_in_b=%d b_in_a=%d\n",
               collided, r->overlap, r->overlap_n[0], r->overlap_n[1],
               r->overlap_v[0], r->overlap_v[1], r->a_in_b, r->b_in_a);
        satc_response_destroy(r);
        satc_polygon_destroy(polygon);
        satc_circle_destroy(circle);
    }

    // 12. polygon-circle a_in_b case (small circle inside polygon)
    {
        satc_point_alloca(p_pos); satc_point_set_xy(p_pos, 0.0, 0.0);
        satc_point_alloca(c_pos); satc_point_set_xy(c_pos, 20.0, 20.0);
        satc_point_alloca(a); satc_point_set_xy(a, 0.0, 0.0);
        satc_point_alloca(b); satc_point_set_xy(b, 40.0, 0.0);
        satc_point_alloca(c); satc_point_set_xy(c, 40.0, 40.0);
        satc_point_alloca(d); satc_point_set_xy(d, 0.0, 40.0);
        satc_point_array_alloca(pts, 4);
        pts[0]=a;pts[1]=b;pts[2]=c;pts[3]=d;
        satc_polygon_t *polygon = satc_polygon_create(p_pos, 4, pts);
        satc_circle_t *circle = satc_circle_create(c_pos, 5.0);
        satc_response_t *r = satc_response_create();
        bool collided = satc_test_polygon_circle(polygon, circle, r);
        printf("polygon_circle_inside: collided=%d overlap=%.10f overlap_n=%.10f,%.10f overlap_v=%.10f,%.10f a_in_b=%d b_in_a=%d\n",
               collided, r->overlap, r->overlap_n[0], r->overlap_n[1],
               r->overlap_v[0], r->overlap_v[1], r->a_in_b, r->b_in_a);
        satc_response_destroy(r);
        satc_polygon_destroy(polygon);
        satc_circle_destroy(circle);
    }

    // 13. box -> polygon
    {
        satc_point_alloca(p); satc_point_set_xy(p, 1.5, 2.5);
        satc_box_t *b = satc_box_create(p, 3.0, 4.0);
        satc_polygon_t *pg = satc_box_to_polygon(b);
        printf("box_to_poly num_points=%zu pos=%.10f,%.10f\n", pg->num_points, pg->pos[0], pg->pos[1]);
        for (size_t i = 0; i < pg->num_points; i++) {
            printf("  pt%zu: %.10f %.10f\n", i, pg->points[i][0], pg->points[i][1]);
        }
        for (size_t i = 0; i < pg->num_calc_points; i++) {
            printf("  cp%zu: %.10f %.10f\n", i, pg->calc_points[i][0], pg->calc_points[i][1]);
        }
        for (size_t i = 0; i < pg->num_edges; i++) {
            printf("  ed%zu: %.10f %.10f\n", i, pg->edges[i][0], pg->edges[i][1]);
        }
        for (size_t i = 0; i < pg->num_normals; i++) {
            printf("  nm%zu: %.10f %.10f\n", i, pg->normals[i][0], pg->normals[i][1]);
        }
        satc_polygon_destroy(pg);
        satc_box_destroy(b);
    }

    return 0;
}
