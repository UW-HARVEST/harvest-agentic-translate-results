#include "lib.h"
#include <math.h>

typedef enum {
	C2_TYPE_CIRCLE,
	C2_TYPE_AABB,
	C2_TYPE_CAPSULE,
} C2_TYPE;

typedef struct c2Circle {
	c2v p;
	float r;
} c2Circle;

typedef struct c2AABB {
	c2v min;
	c2v max;
} c2AABB;

typedef struct c2Capsule {
	c2v a;
	c2v b;
	float r;
} c2Capsule;

typedef struct c2Ray {
	c2v p;
	c2v d;
	float t;
} c2Ray;

c2v c2V(float x, float y) {
	c2v a;
	a.x = x;
	a.y = y;
	return a;
}

float c2Dot(c2v a, c2v b) {
	return a.x * b.x + a.y * b.y;
}

float c2Len(c2v a) {
	return sqrtf(c2Dot(a, a));
}

c2v c2Add(c2v a, c2v b) {
	a.x += b.x;
	a.y += b.y;
	return a;
}

c2v c2Sub(c2v a, c2v b) {
	a.x -= b.x;
	a.y -= b.y;
	return a;
}

c2v c2Mulvs(c2v a, float b) {
	a.x *= b;
	a.y *= b;
	return a;
}

c2v c2Div(c2v a, float b) {
	return c2Mulvs(a, 1.0f / b);
}

c2v c2Norm(c2v a) {
	return c2Div(a, c2Len(a));
}

c2v c2Minv(c2v a, c2v b) {
	return c2V(((a.x) < (b.x) ? (a.x) : (b.x)),
			((a.y) < (b.y) ? (a.y) : (b.y)));
}

c2v c2Maxv(c2v a, c2v b) {
	return c2V(((a.x) > (b.x) ? (a.x) : (b.x)),
			((a.y) > (b.y) ? (a.y) : (b.y)));
}

c2v c2Skew(c2v a) {
	c2v b;
	b.x = -a.y;
	b.y = a.x;
	return b;
}

c2v c2Absv(c2v a) {
	return c2V(((a.x) < 0 ? -(a.x) : (a.x)), ((a.y) < 0 ? -(a.y) : (a.y)));
}

int c2RaytoCircle(c2Ray A, c2Circle B, c2Raycast *out) {
	c2v p = B.p;
	c2v m = c2Sub(A.p, p);
	float c = c2Dot(m, m) - B.r * B.r;
	float b = c2Dot(m, A.d);
	float disc = b * b - c;
	if (disc < 0)
		return 0;
	float t = -b - sqrtf(disc);
	if (t >= 0 && t <= A.t) {
		out->t = t;
		c2v impact = c2Add(A.p, c2Mulvs(A.d, t));
		out->n = c2Norm(c2Sub(impact, p));
		return 1;
	}
	return 0;
}

int c2AABBtoAABB(c2AABB A, c2AABB B) {
	int d0 = B.max.x < A.min.x;
	int d1 = A.max.x < B.min.x;
	int d2 = B.max.y < A.min.y;
	int d3 = A.max.y < B.min.y;
	return !(d0 | d1 | d2 | d3);
}

static inline float c2SignedDistPointToPlane_OneDimensional(float p, float n,
		float d) {
	return p * n - d * n;
}

static inline float c2RayToPlane_OneDimensional(float da, float db) {
	if (da < 0)
		return 0;
	else if (da * db > 0)
		return 1.0f;
	else {
		float d = da - db;
		if (d != 0)
			return da / d;
		else
			return 0;
	}
}

int c2RaytoAABB(c2Ray A, c2AABB B, c2Raycast *out) {
	c2v p0 = A.p;
	c2v p1 = c2Add(A.p, c2Mulvs(A.d, A.t));
	c2AABB a_box;
	a_box.min = c2Minv(p0, p1);
	a_box.max = c2Maxv(p0, p1);
	if (!c2AABBtoAABB(a_box, B))
		return 0;
	c2v ab = c2Sub(p1, p0);
	c2v n = c2Skew(ab);
	c2v abs_n = c2Absv(n);
	c2v half_extents = c2Mulvs(c2Sub(B.max, B.min), 0.5f);
	c2v center_of_b_box = c2Mulvs(c2Add(B.min, B.max), 0.5f);
	float d = ((c2Dot(n, c2Sub(p0, center_of_b_box))) < 0
			? -(c2Dot(n, c2Sub(p0, center_of_b_box)))
			: (c2Dot(n, c2Sub(p0, center_of_b_box)))) -
		c2Dot(abs_n, half_extents);
	if (d > 0)
		return 0;
	float da0 = c2SignedDistPointToPlane_OneDimensional(p0.x, -1.0f, B.min.x);
	float db0 = c2SignedDistPointToPlane_OneDimensional(p1.x, -1.0f, B.min.x);
	float da1 = c2SignedDistPointToPlane_OneDimensional(p0.x, 1.0f, B.max.x);
	float db1 = c2SignedDistPointToPlane_OneDimensional(p1.x, 1.0f, B.max.x);
	float da2 = c2SignedDistPointToPlane_OneDimensional(p0.y, -1.0f, B.min.y);
	float db2 = c2SignedDistPointToPlane_OneDimensional(p1.y, -1.0f, B.min.y);
	float da3 = c2SignedDistPointToPlane_OneDimensional(p0.y, 1.0f, B.max.y);
	float db3 = c2SignedDistPointToPlane_OneDimensional(p1.y, 1.0f, B.max.y);
	float t0 = c2RayToPlane_OneDimensional(da0, db0);
	float t1 = c2RayToPlane_OneDimensional(da1, db1);
	float t2 = c2RayToPlane_OneDimensional(da2, db2);
	float t3 = c2RayToPlane_OneDimensional(da3, db3);
	int hit0 = t0 <= 1.0f;
	int hit1 = t1 <= 1.0f;
	int hit2 = t2 <= 1.0f;
	int hit3 = t3 <= 1.0f;
	int hit = hit0 | hit1 | hit2 | hit3;
	if (hit) {
		t0 = (float)hit0 * t0;
		t1 = (float)hit1 * t1;
		t2 = (float)hit2 * t2;
		t3 = (float)hit3 * t3;
		if (t0 >= t1 && t0 >= t2 && t0 >= t3) {
			out->t = t0 * A.t;
			out->n = c2V(-1, 0);
		} else if (t1 >= t0 && t1 >= t2 && t1 >= t3) {
			out->t = t1 * A.t;
			out->n = c2V(1, 0);
		} else if (t2 >= t0 && t2 >= t1 && t2 >= t3) {
			out->t = t2 * A.t;
			out->n = c2V(0, -1);
		} else {
			out->t = t3 * A.t;
			out->n = c2V(0, 1);
		}
		return 1;
	} else
		return 0;
}

typedef struct c2m {
	c2v x;
	c2v y;
} c2m;

c2v c2CCW90(c2v a) {
	c2v b;
	b.x = a.y;
	b.y = -a.x;
	return b;
}

c2v c2MulmvT(c2m a, c2v b) {
	c2v c;
	c.x = a.x.x * b.x + a.x.y * b.y;
	c.y = a.y.x * b.x + a.y.y * b.y;
	return c;
}

int c2AABBtoPoint(c2AABB A, c2v B) {
	int d0 = B.x < A.min.x;
	int d1 = B.y < A.min.y;
	int d2 = B.x > A.max.x;
	int d3 = B.y > A.max.y;
	return !(d0 | d1 | d2 | d3);
}

int c2CircleToPoint(c2Circle A, c2v B) {
	c2v n = c2Sub(A.p, B);
	float d2 = c2Dot(n, n);
	return d2 < A.r * A.r;
}

int c2RaytoCapsule(c2Ray A, c2Capsule B, c2Raycast *out) {
	c2m M;
	M.y = c2Norm(c2Sub(B.b, B.a));
	M.x = c2CCW90(M.y);
	c2v cap_n = c2Sub(B.b, B.a);
	c2v yBb = c2MulmvT(M, cap_n);
	c2v yAp = c2MulmvT(M, c2Sub(A.p, B.a));
	c2v yAd = c2MulmvT(M, A.d);
	c2v yAe = c2Add(yAp, c2Mulvs(yAd, A.t));
	c2AABB capsule_bb;
	capsule_bb.min = c2V(-B.r, 0);
	capsule_bb.max = c2V(B.r, yBb.y);
	out->n = c2Norm(cap_n);
	out->t = 0;
	if (c2AABBtoPoint(capsule_bb, yAp)) {
		return 1;
	} else {
		c2Circle capsule_a;
		c2Circle capsule_b;
		capsule_a.p = B.a;
		capsule_a.r = B.r;
		capsule_b.p = B.b;
		capsule_b.r = B.r;
		if (c2CircleToPoint(capsule_a, A.p)) {
			return 1;
		} else if (c2CircleToPoint(capsule_b, A.p)) {
			return 1;
		}
	}
	if (yAe.x * yAp.x < 0 ||
			((((yAe.x) < 0 ? -(yAe.x) : (yAe.x))) <
			 (((yAp.x) < 0 ? -(yAp.x) : (yAp.x)))
			 ? (((yAe.x) < 0 ? -(yAe.x) : (yAe.x)))
			 : (((yAp.x) < 0 ? -(yAp.x) : (yAp.x)))) < B.r) {
		c2Circle Ca, Cb;
		Ca.p = B.a;
		Ca.r = B.r;
		Cb.p = B.b;
		Cb.r = B.r;
		if (((yAp.x) < 0 ? -(yAp.x) : (yAp.x)) < B.r) {
			if (yAp.y < 0)
				return c2RaytoCircle(A, Ca, out);
			else
				return c2RaytoCircle(A, Cb, out);
		} else {
			float c = yAp.x > 0 ? B.r : -B.r;
			float d = (yAe.x - yAp.x);
			float t = (c - yAp.x) / d;
			float y = yAp.y + (yAe.y - yAp.y) * t;
			if (y <= 0)
				return c2RaytoCircle(A, Ca, out);
			if (y >= yBb.y)
				return c2RaytoCircle(A, Cb, out);
			else {
				out->n = c > 0 ? M.x : c2Skew(M.y);
				out->t = t * A.t;
				return 1;
			}
		}
	}
	return 0;
}

int c2CastRay(c2Ray A, const void *B, C2_TYPE typeB, c2Raycast *out) {
	switch (typeB) {
		case C2_TYPE_CIRCLE:
			return c2RaytoCircle(A, *(c2Circle *)B, out);
		case C2_TYPE_AABB:
			return c2RaytoAABB(A, *(c2AABB *)B, out);
		case C2_TYPE_CAPSULE:
			return c2RaytoCapsule(A, *(c2Capsule *)B, out);
			return 0;
	}
}

int gen_ray(c2Raycast *cast1, c2Raycast *cast2, c2Raycast *cast3,
		float mp_x, float mp_y, float r_p_x, float r_p_y,
		float c_p_x, float c_p_y, float c_r,
		float cap_a_x, float cap_a_y, float cap_b_x, float cap_b_y, float cap_r,
		float bb_min_x, float bb_min_y, float bb_max_x, float bb_max_y) {
	int hit = 0;

	c2v mp = c2V(mp_x, mp_y);

	c2Ray ray;
	ray.p = c2V(r_p_x, r_p_y);
	ray.d = c2Norm(c2Sub(mp, ray.p));
	ray.t = c2Dot(mp, ray.d) - c2Dot(ray.p, ray.d);

	c2Circle c;
	c.p = c2V(c_p_x, c_p_y);
	c.r = c_r;

	hit += c2CastRay(ray, &c, C2_TYPE_CIRCLE, cast1);

        c2Capsule cap;
	cap.a = c2V(cap_a_x, cap_a_y);
	cap.b = c2V(cap_b_x, cap_b_y);
	cap.r = cap_r;

	hit += (c2CastRay(ray, &cap, C2_TYPE_CAPSULE, cast2) << 1);

	c2AABB bb;
	bb.min = c2V(bb_min_x, bb_min_y);
	bb.max = c2V(bb_max_x, bb_max_y);

	hit += (c2CastRay(ray, &bb, C2_TYPE_AABB, cast3) << 2);

	return hit;
}
