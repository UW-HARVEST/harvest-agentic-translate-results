#include "../include/lib.h"
#include <math.h>
#include <stdlib.h>

typedef struct c2h {
    c2v n;
    float d;
} c2h;

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

typedef struct c2Poly {
    int count;
    c2v verts[8];
    c2v norms[8];
} c2Poly;

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

float c2Dist(c2h h, c2v p) {
    return c2Dot(h.n, p) - h.d;
}

c2h c2PlaneAt(const c2Poly *p, const int i) {
    c2h h;
    h.n = p->norms[i];
    h.d = c2Dot(p->norms[i], p->verts[i]);
    return h;
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

c2v c2MulrvT(c2r a, c2v b) {
    return c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y);
}

c2v c2Add(c2v a, c2v b) {
	a.x += b.x;
	a.y += b.y;
	return a;
}

c2v c2Mulxv(c2x a, c2v b) {
	return c2Add(c2Mulrv(a.r, b), a.p);
}

c2v c2MulxvT(c2x a, c2v b) {
    return c2MulrvT(a.r, c2Sub(b, a.p));
}

c2v c2Intersect(c2v a, c2v b, float da, float db) {
    return c2Add(a, c2Mulvs(c2Sub(b, a), (da / (da - db))));
}

static int c2Clip(c2v *seg, c2h h) {
    c2v out[2];
    int sp = 0;
    float d0, d1;
    if ((d0 = c2Dist(h, seg[0])) < 0)
        out[sp++] = seg[0];
    if ((d1 = c2Dist(h, seg[1])) < 0)
        out[sp++] = seg[1];
    if (d0 == 0 && d1 == 0) {
        out[sp++] = seg[0];
        out[sp++] = seg[1];
    } else if (d0 * d1 <= 0)
        out[sp++] = c2Intersect(seg[0], seg[1], d0, d1);
    seg[0] = out[0];
    seg[1] = out[1];
    return sp;
}

c2v c2Div(c2v a, float b) {
	return c2Mulvs(a, 1.0f / b);
}

c2v c2Norm(c2v a) {
    return c2Div(a, c2Len(a));
}

c2v c2Neg(c2v a) {
	return c2V(-a.x, -a.y);
}

c2v c2CCW90(c2v a) {
	c2v b;
	b.x = a.y;
	b.y = -a.x;
	return b;
}

static int c2SidePlanes(c2v *seg, c2v ra, c2v rb, c2h *h) {
    c2v in = c2Norm(c2Sub(rb, ra));
    c2h left = {c2Neg(in), c2Dot(c2Neg(in), ra)};
    c2h right = {in, c2Dot(in, rb)};
    if (c2Clip(seg, left) < 2)
        return 0;
    if (c2Clip(seg, right) < 2)
        return 0;
    if (h) {
        h->n = c2CCW90(in);
        h->d = c2Dot(c2CCW90(in), ra);
    }
    return 1;
}

static int c2SidePlanesFromPoly(c2v *seg, c2x x, const c2Poly *p, int e, c2h *h) {
    c2v ra = c2Mulxv(x, p->verts[e]);
    c2v rb = c2Mulxv(x, p->verts[e + 1 == p->count ? 0 : e + 1]);
    return c2SidePlanes(seg, ra, rb, h);
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

c2v c2Skew(c2v a) {
	c2v b;
	b.x = -a.y;
	b.y = a.x;
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

c2v c2Absv(c2v a) {
    return c2V(((a.x) < 0 ? -(a.x) : (a.x)), ((a.y) < 0 ? -(a.y) : (a.y)));
}

void c2CircletoCircleManifold(c2Circle A, c2Circle B, c2Manifold *m) {
    m->count = 0;
    c2v d = c2Sub(B.p, A.p);
    float d2 = c2Dot(d, d);
    float r = A.r + B.r;
    if (d2 < r * r) {
        float l = sqrtf(d2);
        c2v n = l != 0 ? c2Mulvs(d, 1.0f / l) : c2V(0, 1.0f);
        m->count = 1;
        m->depths[0] = r - l;
        m->contact_points[0] = c2Sub(B.p, c2Mulvs(n, B.r));
        m->n = n;
    }
}

void c2CircletoAABBManifold(c2Circle A, c2AABB B, c2Manifold *m) {
    m->count = 0;
    c2v L = c2Clampv(A.p, B.min, B.max);
    c2v ab = c2Sub(L, A.p);
    float d2 = c2Dot(ab, ab);
    float r2 = A.r * A.r;
    if (d2 < r2) {
        if (d2 != 0) {
            float d = sqrtf(d2);
            c2v n = c2Norm(ab);
            m->count = 1;
            m->depths[0] = A.r - d;
            m->contact_points[0] = c2Add(A.p, c2Mulvs(n, d));
            m->n = n;
        } else {
            c2v mid = c2Mulvs(c2Add(B.min, B.max), 0.5f);
            c2v e = c2Mulvs(c2Sub(B.max, B.min), 0.5f);
            c2v d = c2Sub(A.p, mid);
            c2v abs_d = c2Absv(d);
            float x_overlap = e.x - abs_d.x;
            float y_overlap = e.y - abs_d.y;
            float depth;
            c2v n;
            if (x_overlap < y_overlap) {
                depth = x_overlap;
                n = c2V(1.0f, 0);
                n = c2Mulvs(n, d.x < 0 ? 1.0f : -1.0f);
            } else {
                depth = y_overlap;
                n = c2V(0, 1.0f);
                n = c2Mulvs(n, d.y < 0 ? 1.0f : -1.0f);
            }
            m->count = 1;
            m->depths[0] = A.r + depth;
            m->contact_points[0] = c2Sub(A.p, c2Mulvs(n, depth));
            m->n = n;
        }
    }
}

void c2CircletoCapsuleManifold(c2Circle A, c2Capsule B, c2Manifold *m) {
    m->count = 0;
    c2v a, b;
    float r = A.r + B.r;
    float d =
        c2GJK(&A, C2_TYPE_CIRCLE, 0, &B, C2_TYPE_CAPSULE, 0, &a, &b, 0, 0, 0);
    if (d < r) {
        c2v n;
        if (d == 0)
            n = c2Norm(c2Skew(c2Sub(B.b, B.a)));
        else
            n = c2Norm(c2Sub(b, a));
        m->count = 1;
        m->depths[0] = r - d;
        m->contact_points[0] = c2Sub(b, c2Mulvs(n, B.r));
        m->n = n;
    }
}

void c2AABBtoAABBManifold(c2AABB A, c2AABB B, c2Manifold *m) {
    m->count = 0;
    c2v mid_a = c2Mulvs(c2Add(A.min, A.max), 0.5f);
    c2v mid_b = c2Mulvs(c2Add(B.min, B.max), 0.5f);
    c2v eA = c2Absv(c2Mulvs(c2Sub(A.max, A.min), 0.5f));
    c2v eB = c2Absv(c2Mulvs(c2Sub(B.max, B.min), 0.5f));
    c2v d = c2Sub(mid_b, mid_a);
    float dx = eA.x + eB.x - ((d.x) < 0 ? -(d.x) : (d.x));
    if (dx < 0)
        return;
    float dy = eA.y + eB.y - ((d.y) < 0 ? -(d.y) : (d.y));
    if (dy < 0)
        return;
    c2v n;
    float depth;
    c2v p;
    if (dx < dy) {
        depth = dx;
        if (d.x < 0) {
            n = c2V(-1.0f, 0);
            p = c2Sub(mid_a, c2V(eA.x, 0));
        } else {
            n = c2V(1.0f, 0);
            p = c2Add(mid_a, c2V(eA.x, 0));
        }
    } else {
        depth = dy;
        if (d.y < 0) {
            n = c2V(0, -1.0f);
            p = c2Sub(mid_a, c2V(0, eA.y));
        } else {
            n = c2V(0, 1.0f);
            p = c2Add(mid_a, c2V(0, eA.y));
        }
    }
    m->count = 1;
    m->contact_points[0] = p;
    m->depths[0] = depth;
    m->n = n;
}

static void c2KeepDeep(c2v *seg, c2h h, c2Manifold *m) {
    int cp = 0;
    for (int i = 0; i < 2; ++i) {
        c2v p = seg[i];
        float d = c2Dist(h, p);
        if (d <= 0) {
            m->contact_points[cp] = p;
            m->depths[cp] = -d;
            ++cp;
        }
    }
    m->count = cp;
    m->n = h.n;
}

static void c2Incident(c2v *incident, const c2Poly *ip, c2x ix,
                       c2v rn_in_incident_space) {
    int index = ~0;
    float min_dot = 3.40282346638528859811704183484516925e+38F;
    for (int i = 0; i < ip->count; ++i) {
        float dot = c2Dot(rn_in_incident_space, ip->norms[i]);
        if (dot < min_dot) {
            min_dot = dot;
            index = i;
        }
    }
    incident[0] = c2Mulxv(ix, ip->verts[index]);
    incident[1] =
        c2Mulxv(ix, ip->verts[index + 1 == ip->count ? 0 : index + 1]);
}

void c2CapsuletoPolyManifold(c2Capsule A, const c2Poly *B, const c2x *bx_ptr,
                             c2Manifold *m) {
    m->count = 0;
    c2v a, b;
    float d =
        c2GJK(&A, C2_TYPE_CAPSULE, 0, B, C2_TYPE_POLY, bx_ptr, &a, &b, 0, 0, 0);
    if (d < 1.0e-6f) {
        c2x bx = bx_ptr ? *bx_ptr : c2xIdentity();
        c2Capsule A_in_B;
        A_in_B.a = c2MulxvT(bx, A.a);
        A_in_B.b = c2MulxvT(bx, A.b);
        c2v ab = c2Norm(c2Sub(A_in_B.a, A_in_B.b));
        c2h ab_h0;
        ab_h0.n = c2CCW90(ab);
        ab_h0.d = c2Dot(A_in_B.a, ab_h0.n);
        int v0 = c2Support(B->verts, B->count, c2Neg(ab_h0.n));
        float s0 = c2Dist(ab_h0, B->verts[v0]);
        c2h ab_h1;
        ab_h1.n = c2Skew(ab);
        ab_h1.d = c2Dot(A_in_B.a, ab_h1.n);
        int v1 = c2Support(B->verts, B->count, c2Neg(ab_h1.n));
        float s1 = c2Dist(ab_h1, B->verts[v1]);
        int index = ~0;
        float sep = -3.40282346638528859811704183484516925e+38F;
        int code = 0;
        for (int i = 0; i < B->count; ++i) {
            c2h h = c2PlaneAt(B, i);
            float da = c2Dot(A_in_B.a, c2Neg(h.n));
            float db = c2Dot(A_in_B.b, c2Neg(h.n));
            float d;
            if (da > db)
                d = c2Dist(h, A_in_B.a);
            else
                d = c2Dist(h, A_in_B.b);
            if (d > sep) {
                sep = d;
                index = i;
            }
        }
	if (s0 > sep) {
            sep = s0;
            index = v0;
            code = 1;
        }
        if (s1 > sep) {
            sep = s1;
            index = v1;
            code = 2;
        }
        switch (code) {
        case 0: {
            c2v seg[2] = {A.a, A.b};
            c2h h;
            if (!c2SidePlanesFromPoly(seg, bx, B, index, &h))
                return;
            c2KeepDeep(seg, h, m);
            m->n = c2Neg(m->n);
        } break;
	case 1: {
            c2v incident[2];
            c2Incident(incident, B, bx, ab_h0.n);
            c2h h;
            if (!c2SidePlanes(incident, A_in_B.b, A_in_B.a, &h))
                return;
            c2KeepDeep(incident, h, m);
        } break;
        case 2: {
            c2v incident[2];
            c2Incident(incident, B, bx, ab_h1.n);
            c2h h;
            if (!c2SidePlanes(incident, A_in_B.a, A_in_B.b, &h))
                return;
            c2KeepDeep(incident, h, m);
        } break;
        default:
            return;
        }
        for (int i = 0; i < m->count; ++i)
            m->depths[i] += A.r;
    } else if (d < A.r) {
        m->count = 1;
        m->n = c2Norm(c2Sub(b, a));
        m->contact_points[0] = c2Add(a, c2Mulvs(m->n, A.r));
        m->depths[0] = A.r - d;
    }
}

void c2Norms(c2v *verts, c2v *norms, int count) {
    for (int i = 0; i < count; ++i) {
        int a = i;
        int b = i + 1 < count ? i + 1 : 0;
        c2v e = c2Sub(verts[b], verts[a]);
        norms[i] = c2Norm(c2CCW90(e));
    }
}

void c2AABBtoCapsuleManifold(c2AABB A, c2Capsule B, c2Manifold *m) {
    m->count = 0;
    c2Poly p;
    c2BBVerts(p.verts, &A);
    p.count = 4;
    c2Norms(p.verts, p.norms, 4);
    c2CapsuletoPolyManifold(B, &p, 0, m);
    m->n = c2Neg(m->n);
}

void c2CapsuletoCapsuleManifold(c2Capsule A, c2Capsule B, c2Manifold *m) {
    m->count = 0;
    c2v a, b;
    float r = A.r + B.r;
    float d =
        c2GJK(&A, C2_TYPE_CAPSULE, 0, &B, C2_TYPE_CAPSULE, 0, &a, &b, 0, 0, 0);
    if (d < r) {
        c2v n;
        if (d == 0)
            n = c2Norm(c2Skew(c2Sub(A.b, A.a)));
        else
            n = c2Norm(c2Sub(b, a));
        m->count = 1;
        m->depths[0] = r - d;
        m->contact_points[0] = c2Sub(b, c2Mulvs(n, B.r));
        m->n = n;
    }
}

void c2Collide(const void *A, C2_TYPE typeA, const void *B, C2_TYPE typeB, c2Manifold *m) {
    m->count = 0;
    switch (typeA) {
    case C2_TYPE_CIRCLE:
        switch (typeB) {
        case C2_TYPE_CIRCLE:
            c2CircletoCircleManifold(*(c2Circle *)A, *(c2Circle *)B, m);
            break;
        case C2_TYPE_AABB:
            c2CircletoAABBManifold(*(c2Circle *)A, *(c2AABB *)B, m);
            break;
        case C2_TYPE_CAPSULE:
            c2CircletoCapsuleManifold(*(c2Circle *)A, *(c2Capsule *)B, m);
            break;
        }
        break;
    case C2_TYPE_AABB:
        switch (typeB) {
        case C2_TYPE_CIRCLE:
            c2CircletoAABBManifold(*(c2Circle *)B, *(c2AABB *)A, m);
            m->n = c2Neg(m->n);
            break;
        case C2_TYPE_AABB:
            c2AABBtoAABBManifold(*(c2AABB *)A, *(c2AABB *)B, m);
            break;
        case C2_TYPE_CAPSULE:
            c2AABBtoCapsuleManifold(*(c2AABB *)A, *(c2Capsule *)B, m);
            break;
        }
        break;
    case C2_TYPE_CAPSULE:
        switch (typeB) {
        case C2_TYPE_CIRCLE:
            c2CircletoCapsuleManifold(*(c2Circle *)B, *(c2Capsule *)A, m);
            m->n = c2Neg(m->n);
            break;
        case C2_TYPE_AABB:
            c2AABBtoCapsuleManifold(*(c2AABB *)B, *(c2Capsule *)A, m);
            m->n = c2Neg(m->n);
            break;
        case C2_TYPE_CAPSULE:
            c2CapsuletoCapsuleManifold(*(c2Capsule *)A, *(c2Capsule *)B, m);
            break;
        }
        break;
    }
}

void *ptr_from_parts(C2_TYPE typ, float a, float b, float c, float d, float e) {
	c2Circle *circle;
	c2AABB *aabb;
	c2Capsule *capsule;

	switch (typ) {
	case C2_TYPE_CIRCLE:
		circle = malloc(sizeof(c2Circle));
		circle->p = c2V(a, b);
		circle->r = c;
		return (void*)circle;
	case C2_TYPE_AABB:
		aabb = malloc(sizeof(c2AABB));
		aabb->min = c2V(a, b);
		aabb->max = c2V(c, d);
		return (void*)aabb;
	case C2_TYPE_CAPSULE:
		capsule = malloc(sizeof(c2Capsule));
		capsule->a = c2V(a, b);
		capsule->b = c2V(c, d);
		capsule->r = e;
		return (void*)capsule;
	}
}

void omni_manifold(c2Manifold *m,
		C2_TYPE type_a, float a1, float a2, float a3, float a4, float a5,
		C2_TYPE type_b, float b1, float b2, float b3, float b4, float b5) {
	void *A = ptr_from_parts(type_a, a1, a2, a3, a4, a5);
	void *B = ptr_from_parts(type_b, b1, b2, b3, b4, b5);

	c2Collide(A, type_a, B, type_b, m);
}
