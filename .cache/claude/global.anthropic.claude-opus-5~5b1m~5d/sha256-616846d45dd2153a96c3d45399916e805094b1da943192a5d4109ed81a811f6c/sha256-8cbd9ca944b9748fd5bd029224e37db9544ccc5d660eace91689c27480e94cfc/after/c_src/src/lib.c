#include "lib.h"

typedef enum {
    C2_TYPE_CIRCLE,
    C2_TYPE_AABB,
    C2_TYPE_CAPSULE,
} C2_TYPE;

typedef struct c2v {
	float x;
	float y;
} c2v;

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

c2v c2V(float x, float y) {
	c2v a;
	a.x = x;
	a.y = y;
	return a;
}

c2v c2Mulvs(c2v a, float b) {
	a.x *= b;
	a.y *= b;
	return a;
}

c2v c2Maxv(c2v a, c2v b) {
	return c2V(((a.x) > (b.x) ? (a.x) : (b.x)),
			((a.y) > (b.y) ? (a.y) : (b.y)));
}

c2v c2Minv(c2v a, c2v b) {
	return c2V(((a.x) < (b.x) ? (a.x) : (b.x)),
			((a.y) < (b.y) ? (a.y) : (b.y)));
}

c2v c2Clampv(c2v a, c2v lo, c2v hi) {
	return c2Maxv(lo, c2Minv(a, hi));
}

c2v c2Sub(c2v a, c2v b) {
	a.x -= b.x;
	a.y -= b.y;
	return a;
}

float c2Dot(c2v a, c2v b) {
	return a.x * b.x + a.y * b.y;
}

int c2CircletoCircle(c2Circle A, c2Circle B) {
	c2v c = c2Sub(B.p, A.p);
	float d2 = c2Dot(c, c);
	float r2 = A.r + B.r;
	r2 = r2 * r2;
	return d2 < r2;
}

int c2CircletoAABB(c2Circle A, c2AABB B) {
	c2v L = c2Clampv(A.p, B.min, B.max);
	c2v ab = c2Sub(A.p, L);
	float d2 = c2Dot(ab, ab);
	float r2 = A.r * A.r;
	return d2 < r2;
}

int c2CircletoCapsule(c2Circle A, c2Capsule B) {
	c2v n = c2Sub(B.b, B.a);
	c2v ap = c2Sub(A.p, B.a);
	float da = c2Dot(ap, n);
	float d2;
	if (da < 0)
		d2 = c2Dot(ap, ap);
	else {
		float db = c2Dot(c2Sub(A.p, B.b), n);
		if (db < 0) {
			c2v e = c2Sub(ap, c2Mulvs(n, (da / c2Dot(n, n))));
			d2 = c2Dot(e, e);
		} else {
			c2v bp = c2Sub(A.p, B.b);
			d2 = c2Dot(bp, bp);
		}
	}
	float r = A.r + B.r;
	return d2 < r * r;
}

int c2Collided(const void *A, const void *B, C2_TYPE typeB) {
	switch (typeB) {
		case C2_TYPE_CIRCLE:
			return c2CircletoCircle(*(c2Circle *)A, *(c2Circle *)B);
		case C2_TYPE_AABB:
			return c2CircletoAABB(*(c2Circle *)A, *(c2AABB *)B);
		case C2_TYPE_CAPSULE:
			return c2CircletoCapsule(*(c2Circle *)A, *(c2Capsule *)B);
		default:
			return 0;
	}
}

int circle_collide(float x, float y, float r) {
	int result = 0;

	c2Circle circle_in;
	circle_in.p = c2V(x, y);
	circle_in.r = r;

	c2Circle circle;
	circle.p = c2V(-70.0f, 0);
	circle.r = 20.0f;

	c2AABB aabb;
	aabb.min = c2V(-40.0f, -40.0f);
	aabb.max = c2V(-15.0f, -15.0f);

	c2Capsule capsule;
	capsule.a = c2V(-40.0f, 40.0f);
	capsule.b = c2V(-20.0f, 100.0f);
	capsule.r = 10.0f;

	result += c2Collided(&circle_in, &circle, C2_TYPE_CIRCLE);

	result += (c2Collided(&circle_in, &aabb, C2_TYPE_AABB) << 1);

	result += (c2Collided(&circle_in, &capsule, C2_TYPE_CAPSULE) << 2);

	return result;
}
