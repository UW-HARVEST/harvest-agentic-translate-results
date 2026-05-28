//#include "lib.h"
#include <math.h>

typedef enum {
	C2_TYPE_CIRCLE,
	C2_TYPE_AABB,
	C2_TYPE_CAPSULE,
} C2_TYPE;

typedef struct c2v {
	float x;
	float y;
} c2v;

typedef struct c2r {
	float c;
	float s;
} c2r;

typedef struct c2x {
	c2v p;
	c2r r;
} c2x;

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

typedef struct c2GJKCache {
	float metric;
	int count;
	int iA[3];
	int iB[3];
	float div;
} c2GJKCache;

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

c2r c2RotIdentity(void) {
	c2r r;
	r.c = 1.0f;
	r.s = 0;
	return r;
}

c2x c2xIdentity(void) {
	c2x x;
	x.p = c2V(0, 0);
	x.r = c2RotIdentity();
	return x;
}

typedef struct {
	float radius;
	int count;
	c2v verts[8];
} c2Proxy;

void c2BBVerts(c2v *out, c2AABB *bb) {
	out[0] = bb->min;
	out[1] = c2V(bb->max.x, bb->min.y);
	out[2] = bb->max;
	out[3] = c2V(bb->min.x, bb->max.y);
}

void c2MakeProxy(const void *shape, C2_TYPE type, c2Proxy *p) {
	switch (type) {
		case C2_TYPE_CIRCLE: {
					     c2Circle *c = (c2Circle *)shape;
					     p->radius = c->r;
					     p->count = 1;
					     p->verts[0] = c->p;
				     } break;
		case C2_TYPE_AABB: {
					   c2AABB *bb = (c2AABB *)shape;
					   p->radius = 0;
					   p->count = 4;
					   c2BBVerts(p->verts, bb);
				   } break;
		case C2_TYPE_CAPSULE: {
					      c2Capsule *c = (c2Capsule *)shape;
					      p->radius = c->r;
					      p->count = 2;
					      p->verts[0] = c->a;
					      p->verts[1] = c->b;
				      } break;
	}
}

typedef struct {
	c2v sA;
	c2v sB;
	c2v p;
	float u;
	int iA;
	int iB;
} c2sv;

typedef struct {
	c2sv a, b, c, d;
	float div;
	int count;
} c2Simplex;

float c2Len(c2v a) {
	return sqrtf(c2Dot(a, a));
}

float c2Det2(c2v a, c2v b) {
	return a.x * b.y - a.y * b.x;
}

float c2GJKSimplexMetric(c2Simplex *s) {
	switch (s->count) {
		default:
		case 1:
			return 0;
		case 2:
			return c2Len(c2Sub(s->b.p, s->a.p));
		case 3:
			return c2Det2(c2Sub(s->b.p, s->a.p), c2Sub(s->c.p, s->a.p));
	}
}

c2v c2Mulrv(c2r a, c2v b) {
	return c2V(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y);
}

c2v c2Add(c2v a, c2v b) {
	a.x += b.x;
	a.y += b.y;
	return a;
}

c2v c2Mulxv(c2x a, c2v b) {
	return c2Add(c2Mulrv(a.r, b), a.p);
}

void c22(c2Simplex *s) {
	c2v a = s->a.p;
	c2v b = s->b.p;
	float u = c2Dot(b, c2Sub(b, a));
	float v = c2Dot(a, c2Sub(a, b));
	if (v <= 0) {
		s->a.u = 1.0f;
		s->div = 1.0f;
		s->count = 1;
	} else if (u <= 0) {
		s->a = s->b;
		s->a.u = 1.0f;
		s->div = 1.0f;
		s->count = 1;
	} else {
		s->a.u = u;
		s->b.u = v;
		s->div = u + v;
		s->count = 2;
	}
}

void c23(c2Simplex *s) {
	c2v a = s->a.p;
	c2v b = s->b.p;
	c2v c = s->c.p;
	float uAB = c2Dot(b, c2Sub(b, a));
	float vAB = c2Dot(a, c2Sub(a, b));
	float uBC = c2Dot(c, c2Sub(c, b));
	float vBC = c2Dot(b, c2Sub(b, c));
	float uCA = c2Dot(a, c2Sub(a, c));
	float vCA = c2Dot(c, c2Sub(c, a));
	float area = c2Det2(c2Sub(b, a), c2Sub(c, a));
	float uABC = c2Det2(b, c) * area;
	float vABC = c2Det2(c, a) * area;
	float wABC = c2Det2(a, b) * area;
	if (vAB <= 0 && uCA <= 0) {
		s->a.u = 1.0f;
		s->div = 1.0f;
		s->count = 1;
	} else if (uAB <= 0 && vBC <= 0) {
		s->a = s->b;
		s->a.u = 1.0f;
		s->div = 1.0f;
		s->count = 1;
	} else if (uBC <= 0 && vCA <= 0) {
		s->a = s->c;
		s->a.u = 1.0f;
		s->div = 1.0f;
		s->count = 1;
	} else if (uAB > 0 && vAB > 0 && wABC <= 0) {
		s->a.u = uAB;
		s->b.u = vAB;
		s->div = uAB + vAB;
		s->count = 2;
	} else if (uBC > 0 && vBC > 0 && uABC <= 0) {
		s->a = s->b;
		s->b = s->c;
		s->a.u = uBC;
		s->b.u = vBC;
		s->div = uBC + vBC;
		s->count = 2;
	} else if (uCA > 0 && vCA > 0 && vABC <= 0) {
		s->b = s->a;
		s->a = s->c;
		s->a.u = uCA;
		s->b.u = vCA;
		s->div = uCA + vCA;
		s->count = 2;
	} else {
		s->a.u = uABC;
		s->b.u = vABC;
		s->c.u = wABC;
		s->div = uABC + vABC + wABC;
		s->count = 3;
	}
}

c2v c2Neg(c2v a) {
	return c2V(-a.x, -a.y);
}

c2v c2Skew(c2v a) {
	c2v b;
	b.x = -a.y;
	b.y = a.x;
	return b;
}

c2v c2CCW90(c2v a) {
	c2v b;
	b.x = a.y;
	b.y = -a.x;
	return b;
}

c2v c2D(c2Simplex *s) {
	switch (s->count) {
		case 1:
			return c2Neg(s->a.p);
		case 2: {
				c2v ab = c2Sub(s->b.p, s->a.p);
				if (c2Det2(ab, c2Neg(s->a.p)) > 0)
					return c2Skew(ab);
				return c2CCW90(ab);
			}
		case 3:
		default:
			return c2V(0, 0);
	}
}

int c2Support(const c2v *verts, int count, c2v d) {
	int imax = 0;
	float dmax = c2Dot(verts[0], d);
	for (int i = 1; i < count; ++i) {
		float dot = c2Dot(verts[i], d);
		if (dot > dmax) {
			imax = i;
			dmax = dot;
		}
	}
	return imax;
}

void c2Witness(c2Simplex *s, c2v *a, c2v *b) {
	float den = 1.0f / s->div;
	switch (s->count) {
		case 1:
			*a = s->a.sA;
			*b = s->a.sB;
			break;
		case 2:
			*a = c2Add(c2Mulvs(s->a.sA, (den * s->a.u)),
					c2Mulvs(s->b.sA, (den * s->b.u)));
			*b = c2Add(c2Mulvs(s->a.sB, (den * s->a.u)),
					c2Mulvs(s->b.sB, (den * s->b.u)));
			break;
		case 3:
			*a = c2Add(c2Add(c2Mulvs(s->a.sA, (den * s->a.u)),
						c2Mulvs(s->b.sA, (den * s->b.u))),
					c2Mulvs(s->c.sA, (den * s->c.u)));
			*b = c2Add(c2Add(c2Mulvs(s->a.sB, (den * s->a.u)),
						c2Mulvs(s->b.sB, (den * s->b.u))),
					c2Mulvs(s->c.sB, (den * s->c.u)));
			break;
		default:
			*a = c2V(0, 0);
			*b = c2V(0, 0);
	}
}

c2v c2Div(c2v a, float b) {
	return c2Mulvs(a, 1.0f / b);
}

c2v c2Norm(c2v a) {
	return c2Div(a, c2Len(a));
}

c2v c2L(c2Simplex *s) {
    float den = 1.0f / s->div;
    switch (s->count) {
    case 1:
        return s->a.p;
    case 2:
        return c2Add(c2Mulvs(s->a.p, (den * s->a.u)),
                     c2Mulvs(s->b.p, (den * s->b.u)));
    default:
        return c2V(0, 0);
    }
}

c2v c2MulrvT(c2r a, c2v b) {
    return c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y);
}

float c2GJK(const void *A, C2_TYPE typeA, const c2x *ax_ptr, const void *B,
		C2_TYPE typeB, const c2x *bx_ptr, c2v *outA, c2v *outB,
		int use_radius, int *iterations, c2GJKCache *cache) {
	c2x ax;
	c2x bx;
	if (!ax_ptr)
		ax = c2xIdentity();
	else
		ax = *ax_ptr;
	if (!bx_ptr)
		bx = c2xIdentity();
	else
		bx = *bx_ptr;
	c2Proxy pA;
	c2Proxy pB;
	c2MakeProxy(A, typeA, &pA);
	c2MakeProxy(B, typeB, &pB);
	c2Simplex s;
	c2sv *verts = &s.a;
	int cache_was_read = 0;
	if (cache) {
		int cache_was_good = !!cache->count;
		if (cache_was_good) {
			for (int i = 0; i < cache->count; ++i) {
				int iA = cache->iA[i];
				int iB = cache->iB[i];
				c2v sA = c2Mulxv(ax, pA.verts[iA]);
				c2v sB = c2Mulxv(bx, pB.verts[iB]);
				c2sv *v = verts + i;
				v->iA = iA;
				v->sA = sA;
				v->iB = iB;
				v->sB = sB;
				v->p = c2Sub(v->sB, v->sA);
				v->u = 0;
			}
			s.count = cache->count;
			s.div = cache->div;
			float metric_old = cache->metric;
			float metric = c2GJKSimplexMetric(&s);
			float min_metric = metric < metric_old ? metric : metric_old;
			float max_metric = metric > metric_old ? metric : metric_old;
			if (!(min_metric < max_metric * 2.0f && metric < -1.0e8f))
				cache_was_read = 1;
		}
	}
	if (!cache_was_read) {
		s.a.iA = 0;
		s.a.iB = 0;
		s.a.sA = c2Mulxv(ax, pA.verts[0]);
		s.a.sB = c2Mulxv(bx, pB.verts[0]);
		s.a.p = c2Sub(s.a.sB, s.a.sA);
		s.a.u = 1.0f;
		s.div = 1.0f;
		s.count = 1;
	}
	int saveA[3], saveB[3];
	int save_count = 0;
	float d0 = 3.40282346638528859811704183484516925e+38F;
	float d1 = 3.40282346638528859811704183484516925e+38F;
	int iter = 0;
	int hit = 0;
	while (iter < 20) {
		save_count = s.count;
		for (int i = 0; i < save_count; ++i) {
			saveA[i] = verts[i].iA;
			saveB[i] = verts[i].iB;
		}
		switch (s.count) {
			case 1:
				break;
			case 2:
				c22(&s);
				break;
			case 3:
				c23(&s);
				break;
		}
		if (s.count == 3) {
			hit = 1;
			break;
		}
		c2v p = c2L(&s);
		d1 = c2Dot(p, p);
		if (d1 > d0)
			break;
		d0 = d1;
		c2v d = c2D(&s);
		if (c2Dot(d, d) < 1.19209289550781250000000000000000000e-7F *
				1.19209289550781250000000000000000000e-7F)
			break;
		int iA = c2Support(pA.verts, pA.count, c2MulrvT(ax.r, c2Neg(d)));
		c2v sA = c2Mulxv(ax, pA.verts[iA]);
		int iB = c2Support(pB.verts, pB.count, c2MulrvT(bx.r, d));
		c2v sB = c2Mulxv(bx, pB.verts[iB]);
		c2sv *v = verts + s.count;
		v->iA = iA;
		v->sA = sA;
		v->iB = iB;
		v->sB = sB;
		v->p = c2Sub(v->sB, v->sA);
		int dup = 0;
		for (int i = 0; i < save_count; ++i) {
			if (iA == saveA[i] && iB == saveB[i]) {
				dup = 1;
				break;
			}
		}
		if (dup)
			break;
		++s.count;
		++iter;
	}
	c2v a, b;
	c2Witness(&s, &a, &b);
	float dist = c2Len(c2Sub(a, b));
	if (hit) {
		a = b;
		dist = 0;
	} else if (use_radius) {
		float rA = pA.radius;
		float rB = pB.radius;
		if (dist > rA + rB &&
				dist > 1.19209289550781250000000000000000000e-7F) {
			dist -= rA + rB;
			c2v n = c2Norm(c2Sub(b, a));
			a = c2Add(a, c2Mulvs(n, rA));
			b = c2Sub(b, c2Mulvs(n, rB));
			if (a.x == b.x && a.y == b.y)
				dist = 0;
		} else {
			c2v p = c2Mulvs(c2Add(a, b), 0.5f);
			a = p;
			b = p;
			dist = 0;
		}
	}
	if (cache) {
		cache->metric = c2GJKSimplexMetric(&s);
		cache->count = s.count;
		for (int i = 0; i < s.count; ++i) {
			c2sv *v = verts + i;
			cache->iA[i] = v->iA;
			cache->iB[i] = v->iB;
		}
		cache->div = s.div;
	}
	if (outA)
		*outA = a;
	if (outB)
		*outB = b;
	if (iterations)
		*iterations = iter;
	return dist;
}

void gjk_cache(char reverse, c2v *a9, c2v *b9, float a1, float a2, float a3, float a4,
		float b1, float b2, float b3, float b4, float b5) {
	c2GJKCache cache;
	cache.count = 0;

        c2Circle A = {
		{ 0, 0 },
		15.0f
	};

        c2Capsule B = {
		{ 100, -25 },
		{ 75, 100 },
		10
	};

	c2v a0, b0;
        c2v a, b;

	int iterations = -1;
        int cached_iterations = -1;
        float d0 = c2GJK(&A, C2_TYPE_CIRCLE, 0, &B, C2_TYPE_CAPSULE, 0, &a0, &b0, 1, &iterations, &cache);
        float d1 = c2GJK(&A, C2_TYPE_CIRCLE, 0, &B, C2_TYPE_CAPSULE, 0, &a, &b, 1, &cached_iterations, &cache);

	c2AABB bb;
        bb.min = c2V(a1, a2);
        bb.max = c2V(a3, a4);

	c2Capsule cap;
	cap.a = c2V(b1, b2);
	cap.b = c2V(b3, b4);
	cap.r = b5;

	if (reverse) {
		c2GJK(&cap, C2_TYPE_CAPSULE, 0, &bb, C2_TYPE_AABB, 0, &a, &b, 1, 0, 0);
	} else {
		c2GJK(&bb, C2_TYPE_AABB, 0, &cap, C2_TYPE_CAPSULE, 0, &a, &b, 1, 0, 0);
	}
}
