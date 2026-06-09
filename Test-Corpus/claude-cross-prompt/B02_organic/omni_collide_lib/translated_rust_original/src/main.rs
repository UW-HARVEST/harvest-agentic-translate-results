// Translated from c_src/src/lib.c - byte-identical behavior preserved.
// The original C source defines a shared library (no `main`), so the
// executable produces no output.

#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(clippy::too_many_arguments)]

mod lib_c2 {
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    #[repr(C)]
    pub enum C2_TYPE {
        CAPSULE = 0,
        CIRCLE = 1,
        AABB = 2,
    }

    #[derive(Copy, Clone, Debug, Default)]
    pub struct C2v {
        pub x: f32,
        pub y: f32,
    }

    #[derive(Copy, Clone, Debug, Default)]
    pub struct C2r {
        pub c: f32,
        pub s: f32,
    }

    #[derive(Copy, Clone, Debug, Default)]
    pub struct C2x {
        pub p: C2v,
        pub r: C2r,
    }

    #[derive(Copy, Clone, Debug, Default)]
    pub struct C2Circle {
        pub p: C2v,
        pub r: f32,
    }

    #[derive(Copy, Clone, Debug, Default)]
    pub struct C2AABB {
        pub min: C2v,
        pub max: C2v,
    }

    #[derive(Copy, Clone, Debug, Default)]
    pub struct C2Capsule {
        pub a: C2v,
        pub b: C2v,
        pub r: f32,
    }

    #[derive(Copy, Clone, Debug, Default)]
    pub struct C2GJKCache {
        pub metric: f32,
        pub count: i32,
        pub iA: [i32; 3],
        pub iB: [i32; 3],
        pub div: f32,
    }

    #[derive(Copy, Clone, Debug)]
    pub enum Shape {
        Circle(C2Circle),
        AABB(C2AABB),
        Capsule(C2Capsule),
    }

    pub fn c2v(x: f32, y: f32) -> C2v {
        C2v { x, y }
    }

    pub fn c2_mulvs(mut a: C2v, b: f32) -> C2v {
        a.x *= b;
        a.y *= b;
        a
    }

    pub fn c2_maxv(a: C2v, b: C2v) -> C2v {
        c2v(
            if a.x > b.x { a.x } else { b.x },
            if a.y > b.y { a.y } else { b.y },
        )
    }

    pub fn c2_minv(a: C2v, b: C2v) -> C2v {
        c2v(
            if a.x < b.x { a.x } else { b.x },
            if a.y < b.y { a.y } else { b.y },
        )
    }

    pub fn c2_clampv(a: C2v, lo: C2v, hi: C2v) -> C2v {
        c2_maxv(lo, c2_minv(a, hi))
    }

    pub fn c2_sub(mut a: C2v, b: C2v) -> C2v {
        a.x -= b.x;
        a.y -= b.y;
        a
    }

    pub fn c2_dot(a: C2v, b: C2v) -> f32 {
        a.x * b.x + a.y * b.y
    }

    pub fn c2_rot_identity() -> C2r {
        C2r { c: 1.0, s: 0.0 }
    }

    pub fn c2x_identity() -> C2x {
        C2x {
            p: c2v(0.0, 0.0),
            r: c2_rot_identity(),
        }
    }

    #[derive(Copy, Clone, Debug, Default)]
    pub struct C2Proxy {
        pub radius: f32,
        pub count: i32,
        pub verts: [C2v; 8],
    }

    pub fn c2_bb_verts(out: &mut [C2v], bb: &C2AABB) {
        out[0] = bb.min;
        out[1] = c2v(bb.max.x, bb.min.y);
        out[2] = bb.max;
        out[3] = c2v(bb.min.x, bb.max.y);
    }

    pub fn c2_make_proxy(shape: &Shape, p: &mut C2Proxy) {
        match shape {
            Shape::Circle(c) => {
                p.radius = c.r;
                p.count = 1;
                p.verts[0] = c.p;
            }
            Shape::AABB(bb) => {
                p.radius = 0.0;
                p.count = 4;
                c2_bb_verts(&mut p.verts, bb);
            }
            Shape::Capsule(c) => {
                p.radius = c.r;
                p.count = 2;
                p.verts[0] = c.a;
                p.verts[1] = c.b;
            }
        }
    }

    #[derive(Copy, Clone, Debug, Default)]
    pub struct C2sv {
        pub sA: C2v,
        pub sB: C2v,
        pub p: C2v,
        pub u: f32,
        pub iA: i32,
        pub iB: i32,
    }

    #[derive(Copy, Clone, Debug, Default)]
    pub struct C2Simplex {
        pub a: C2sv,
        pub b: C2sv,
        pub c: C2sv,
        pub d: C2sv,
        pub div: f32,
        pub count: i32,
    }

    impl C2Simplex {
        pub fn vert(&self, i: usize) -> &C2sv {
            match i {
                0 => &self.a,
                1 => &self.b,
                2 => &self.c,
                _ => &self.d,
            }
        }
        pub fn vert_mut(&mut self, i: usize) -> &mut C2sv {
            match i {
                0 => &mut self.a,
                1 => &mut self.b,
                2 => &mut self.c,
                _ => &mut self.d,
            }
        }
    }

    pub fn c2_len(a: C2v) -> f32 {
        c2_dot(a, a).sqrt()
    }

    pub fn c2_det2(a: C2v, b: C2v) -> f32 {
        a.x * b.y - a.y * b.x
    }

    pub fn c2_gjk_simplex_metric(s: &C2Simplex) -> f32 {
        match s.count {
            2 => c2_len(c2_sub(s.b.p, s.a.p)),
            3 => c2_det2(c2_sub(s.b.p, s.a.p), c2_sub(s.c.p, s.a.p)),
            _ => 0.0,
        }
    }

    pub fn c2_mulrv(a: C2r, b: C2v) -> C2v {
        c2v(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
    }

    pub fn c2_add(mut a: C2v, b: C2v) -> C2v {
        a.x += b.x;
        a.y += b.y;
        a
    }

    pub fn c2_mulxv(a: C2x, b: C2v) -> C2v {
        c2_add(c2_mulrv(a.r, b), a.p)
    }

    pub fn c2_2(s: &mut C2Simplex) {
        let a = s.a.p;
        let b = s.b.p;
        let u = c2_dot(b, c2_sub(b, a));
        let v = c2_dot(a, c2_sub(a, b));
        if v <= 0.0 {
            s.a.u = 1.0;
            s.div = 1.0;
            s.count = 1;
        } else if u <= 0.0 {
            s.a = s.b;
            s.a.u = 1.0;
            s.div = 1.0;
            s.count = 1;
        } else {
            s.a.u = u;
            s.b.u = v;
            s.div = u + v;
            s.count = 2;
        }
    }

    pub fn c2_3(s: &mut C2Simplex) {
        let a = s.a.p;
        let b = s.b.p;
        let c = s.c.p;
        let uAB = c2_dot(b, c2_sub(b, a));
        let vAB = c2_dot(a, c2_sub(a, b));
        let uBC = c2_dot(c, c2_sub(c, b));
        let vBC = c2_dot(b, c2_sub(b, c));
        let uCA = c2_dot(a, c2_sub(a, c));
        let vCA = c2_dot(c, c2_sub(c, a));
        let area = c2_det2(c2_sub(b, a), c2_sub(c, a));
        let uABC = c2_det2(b, c) * area;
        let vABC = c2_det2(c, a) * area;
        let wABC = c2_det2(a, b) * area;
        if vAB <= 0.0 && uCA <= 0.0 {
            s.a.u = 1.0;
            s.div = 1.0;
            s.count = 1;
        } else if uAB <= 0.0 && vBC <= 0.0 {
            s.a = s.b;
            s.a.u = 1.0;
            s.div = 1.0;
            s.count = 1;
        } else if uBC <= 0.0 && vCA <= 0.0 {
            s.a = s.c;
            s.a.u = 1.0;
            s.div = 1.0;
            s.count = 1;
        } else if uAB > 0.0 && vAB > 0.0 && wABC <= 0.0 {
            s.a.u = uAB;
            s.b.u = vAB;
            s.div = uAB + vAB;
            s.count = 2;
        } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {
            s.a = s.b;
            s.b = s.c;
            s.a.u = uBC;
            s.b.u = vBC;
            s.div = uBC + vBC;
            s.count = 2;
        } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {
            s.b = s.a;
            s.a = s.c;
            s.a.u = uCA;
            s.b.u = vCA;
            s.div = uCA + vCA;
            s.count = 2;
        } else {
            s.a.u = uABC;
            s.b.u = vABC;
            s.c.u = wABC;
            s.div = uABC + vABC + wABC;
            s.count = 3;
        }
    }

    pub fn c2_neg(a: C2v) -> C2v {
        c2v(-a.x, -a.y)
    }

    pub fn c2_skew(a: C2v) -> C2v {
        C2v { x: -a.y, y: a.x }
    }

    pub fn c2_ccw90(a: C2v) -> C2v {
        C2v { x: a.y, y: -a.x }
    }

    pub fn c2_d(s: &C2Simplex) -> C2v {
        match s.count {
            1 => c2_neg(s.a.p),
            2 => {
                let ab = c2_sub(s.b.p, s.a.p);
                if c2_det2(ab, c2_neg(s.a.p)) > 0.0 {
                    c2_skew(ab)
                } else {
                    c2_ccw90(ab)
                }
            }
            _ => c2v(0.0, 0.0),
        }
    }

    pub fn c2_support(verts: &[C2v], count: i32, d: C2v) -> i32 {
        let mut imax: i32 = 0;
        let mut dmax = c2_dot(verts[0], d);
        for i in 1..count {
            let dot = c2_dot(verts[i as usize], d);
            if dot > dmax {
                imax = i;
                dmax = dot;
            }
        }
        imax
    }

    pub fn c2_witness(s: &C2Simplex, a: &mut C2v, b: &mut C2v) {
        let den = 1.0f32 / s.div;
        match s.count {
            1 => {
                *a = s.a.sA;
                *b = s.a.sB;
            }
            2 => {
                *a = c2_add(
                    c2_mulvs(s.a.sA, den * s.a.u),
                    c2_mulvs(s.b.sA, den * s.b.u),
                );
                *b = c2_add(
                    c2_mulvs(s.a.sB, den * s.a.u),
                    c2_mulvs(s.b.sB, den * s.b.u),
                );
            }
            3 => {
                *a = c2_add(
                    c2_add(
                        c2_mulvs(s.a.sA, den * s.a.u),
                        c2_mulvs(s.b.sA, den * s.b.u),
                    ),
                    c2_mulvs(s.c.sA, den * s.c.u),
                );
                *b = c2_add(
                    c2_add(
                        c2_mulvs(s.a.sB, den * s.a.u),
                        c2_mulvs(s.b.sB, den * s.b.u),
                    ),
                    c2_mulvs(s.c.sB, den * s.c.u),
                );
            }
            _ => {
                *a = c2v(0.0, 0.0);
                *b = c2v(0.0, 0.0);
            }
        }
    }

    pub fn c2_div(a: C2v, b: f32) -> C2v {
        c2_mulvs(a, 1.0 / b)
    }

    pub fn c2_norm(a: C2v) -> C2v {
        c2_div(a, c2_len(a))
    }

    pub fn c2_l(s: &C2Simplex) -> C2v {
        let den = 1.0f32 / s.div;
        match s.count {
            1 => s.a.p,
            2 => c2_add(
                c2_mulvs(s.a.p, den * s.a.u),
                c2_mulvs(s.b.p, den * s.b.u),
            ),
            _ => c2v(0.0, 0.0),
        }
    }

    pub fn c2_mulrv_t(a: C2r, b: C2v) -> C2v {
        c2v(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
    }

    pub fn c2_gjk(
        a_shape: &Shape,
        ax_ptr: Option<&C2x>,
        b_shape: &Shape,
        bx_ptr: Option<&C2x>,
        out_a: Option<&mut C2v>,
        out_b: Option<&mut C2v>,
        use_radius: i32,
        iterations: Option<&mut i32>,
        cache: Option<&mut C2GJKCache>,
    ) -> f32 {
        let ax = match ax_ptr {
            None => c2x_identity(),
            Some(p) => *p,
        };
        let bx = match bx_ptr {
            None => c2x_identity(),
            Some(p) => *p,
        };
        let mut p_a = C2Proxy::default();
        let mut p_b = C2Proxy::default();
        c2_make_proxy(a_shape, &mut p_a);
        c2_make_proxy(b_shape, &mut p_b);
        let mut s = C2Simplex::default();
        let mut cache_was_read = 0i32;
        if let Some(ref c) = cache {
            let cache_was_good = c.count != 0;
            if cache_was_good {
                for i in 0..c.count as usize {
                    let iA = c.iA[i];
                    let iB = c.iB[i];
                    let sA = c2_mulxv(ax, p_a.verts[iA as usize]);
                    let sB = c2_mulxv(bx, p_b.verts[iB as usize]);
                    let v = s.vert_mut(i);
                    v.iA = iA;
                    v.sA = sA;
                    v.iB = iB;
                    v.sB = sB;
                    v.p = c2_sub(v.sB, v.sA);
                    v.u = 0.0;
                }
                s.count = c.count;
                s.div = c.div;
                let metric_old = c.metric;
                let metric = c2_gjk_simplex_metric(&s);
                let min_metric = if metric < metric_old { metric } else { metric_old };
                let max_metric = if metric > metric_old { metric } else { metric_old };
                if !(min_metric < max_metric * 2.0 && metric < -1.0e8) {
                    cache_was_read = 1;
                }
            }
        }
        if cache_was_read == 0 {
            s.a.iA = 0;
            s.a.iB = 0;
            s.a.sA = c2_mulxv(ax, p_a.verts[0]);
            s.a.sB = c2_mulxv(bx, p_b.verts[0]);
            s.a.p = c2_sub(s.a.sB, s.a.sA);
            s.a.u = 1.0;
            s.div = 1.0;
            s.count = 1;
        }
        let mut save_a = [0i32; 3];
        let mut save_b = [0i32; 3];
        let mut save_count: i32;
        let mut d0: f32 = 3.402_823_466_385_288_6e+38f32;
        let mut d1: f32;
        let mut iter: i32 = 0;
        let mut hit: i32 = 0;
        while iter < 20 {
            save_count = s.count;
            for i in 0..save_count as usize {
                save_a[i] = s.vert(i).iA;
                save_b[i] = s.vert(i).iB;
            }
            match s.count {
                1 => {}
                2 => c2_2(&mut s),
                3 => c2_3(&mut s),
                _ => {}
            }
            if s.count == 3 {
                hit = 1;
                break;
            }
            let p = c2_l(&s);
            d1 = c2_dot(p, p);
            if d1 > d0 {
                break;
            }
            d0 = d1;
            let d = c2_d(&s);
            if c2_dot(d, d)
                < 1.192_092_9e-7f32 * 1.192_092_9e-7f32
            {
                break;
            }
            let iA = c2_support(&p_a.verts, p_a.count, c2_mulrv_t(ax.r, c2_neg(d)));
            let sA = c2_mulxv(ax, p_a.verts[iA as usize]);
            let iB = c2_support(&p_b.verts, p_b.count, c2_mulrv_t(bx.r, d));
            let sB = c2_mulxv(bx, p_b.verts[iB as usize]);
            {
                let v = s.vert_mut(s.count as usize);
                v.iA = iA;
                v.sA = sA;
                v.iB = iB;
                v.sB = sB;
                v.p = c2_sub(v.sB, v.sA);
            }
            let mut dup = 0i32;
            for i in 0..save_count as usize {
                if iA == save_a[i] && iB == save_b[i] {
                    dup = 1;
                    break;
                }
            }
            if dup != 0 {
                break;
            }
            s.count += 1;
            iter += 1;
        }
        let mut a = c2v(0.0, 0.0);
        let mut b = c2v(0.0, 0.0);
        c2_witness(&s, &mut a, &mut b);
        let mut dist = c2_len(c2_sub(a, b));
        if hit != 0 {
            a = b;
            dist = 0.0;
        } else if use_radius != 0 {
            let rA = p_a.radius;
            let rB = p_b.radius;
            if dist > rA + rB && dist > 1.192_092_9e-7f32 {
                dist -= rA + rB;
                let n = c2_norm(c2_sub(b, a));
                a = c2_add(a, c2_mulvs(n, rA));
                b = c2_sub(b, c2_mulvs(n, rB));
                if a.x == b.x && a.y == b.y {
                    dist = 0.0;
                }
            } else {
                let p = c2_mulvs(c2_add(a, b), 0.5);
                a = p;
                b = p;
                dist = 0.0;
            }
        }
        if let Some(c) = cache {
            c.metric = c2_gjk_simplex_metric(&s);
            c.count = s.count;
            for i in 0..s.count as usize {
                let v = s.vert(i);
                c.iA[i] = v.iA;
                c.iB[i] = v.iB;
            }
            c.div = s.div;
        }
        if let Some(o) = out_a {
            *o = a;
        }
        if let Some(o) = out_b {
            *o = b;
        }
        if let Some(it) = iterations {
            *it = iter;
        }
        dist
    }

    pub fn c2_aabb_to_aabb(a: C2AABB, b: C2AABB) -> i32 {
        let d0 = (b.max.x < a.min.x) as i32;
        let d1 = (a.max.x < b.min.x) as i32;
        let d2 = (b.max.y < a.min.y) as i32;
        let d3 = (a.max.y < b.min.y) as i32;
        if (d0 | d1 | d2 | d3) != 0 {
            0
        } else {
            1
        }
    }

    pub fn c2_aabb_to_capsule(a: C2AABB, b: C2Capsule) -> i32 {
        let sa = Shape::AABB(a);
        let sb = Shape::Capsule(b);
        if c2_gjk(&sa, None, &sb, None, None, None, 1, None, None) != 0.0 {
            return 0;
        }
        1
    }

    pub fn c2_capsule_to_capsule(a: C2Capsule, b: C2Capsule) -> i32 {
        let sa = Shape::Capsule(a);
        let sb = Shape::Capsule(b);
        if c2_gjk(&sa, None, &sb, None, None, None, 1, None, None) != 0.0 {
            return 0;
        }
        1
    }

    pub fn c2_circle_to_circle(a: C2Circle, b: C2Circle) -> i32 {
        let c = c2_sub(b.p, a.p);
        let d2 = c2_dot(c, c);
        let mut r2 = a.r + b.r;
        r2 = r2 * r2;
        (d2 < r2) as i32
    }

    pub fn c2_circle_to_aabb(a: C2Circle, b: C2AABB) -> i32 {
        let l = c2_clampv(a.p, b.min, b.max);
        let ab = c2_sub(a.p, l);
        let d2 = c2_dot(ab, ab);
        let r2 = a.r * a.r;
        (d2 < r2) as i32
    }

    pub fn c2_circle_to_capsule(a: C2Circle, b: C2Capsule) -> i32 {
        let n = c2_sub(b.b, b.a);
        let ap = c2_sub(a.p, b.a);
        let da = c2_dot(ap, n);
        let d2;
        if da < 0.0 {
            d2 = c2_dot(ap, ap);
        } else {
            let db = c2_dot(c2_sub(a.p, b.b), n);
            if db < 0.0 {
                let e = c2_sub(ap, c2_mulvs(n, da / c2_dot(n, n)));
                d2 = c2_dot(e, e);
            } else {
                let bp = c2_sub(a.p, b.b);
                d2 = c2_dot(bp, bp);
            }
        }
        let r = a.r + b.r;
        (d2 < r * r) as i32
    }

    pub fn c2_collided(a: &Shape, b: &Shape) -> i32 {
        match (a, b) {
            (Shape::Circle(ac), Shape::Circle(bc)) => c2_circle_to_circle(*ac, *bc),
            (Shape::Circle(ac), Shape::AABB(bb)) => c2_circle_to_aabb(*ac, *bb),
            (Shape::Circle(ac), Shape::Capsule(bc)) => c2_circle_to_capsule(*ac, *bc),
            (Shape::AABB(aa), Shape::Circle(bc)) => c2_circle_to_aabb(*bc, *aa),
            (Shape::AABB(aa), Shape::AABB(bb)) => c2_aabb_to_aabb(*aa, *bb),
            (Shape::AABB(aa), Shape::Capsule(bc)) => c2_aabb_to_capsule(*aa, *bc),
            (Shape::Capsule(ac), Shape::Circle(bc)) => c2_circle_to_capsule(*bc, *ac),
            (Shape::Capsule(ac), Shape::AABB(bb)) => c2_aabb_to_capsule(*bb, *ac),
            (Shape::Capsule(ac), Shape::Capsule(bc)) => c2_capsule_to_capsule(*ac, *bc),
        }
    }

    pub fn shape_from_parts(typ: C2_TYPE, a: f32, b: f32, c: f32, d: f32, e: f32) -> Shape {
        match typ {
            C2_TYPE::CIRCLE => Shape::Circle(C2Circle {
                p: c2v(a, b),
                r: c,
            }),
            C2_TYPE::AABB => Shape::AABB(C2AABB {
                min: c2v(a, b),
                max: c2v(c, d),
            }),
            C2_TYPE::CAPSULE => Shape::Capsule(C2Capsule {
                a: c2v(a, b),
                b: c2v(c, d),
                r: e,
            }),
        }
    }

    pub fn omni_collide(
        type_a: C2_TYPE,
        a1: f32,
        a2: f32,
        a3: f32,
        a4: f32,
        a5: f32,
        type_b: C2_TYPE,
        b1: f32,
        b2: f32,
        b3: f32,
        b4: f32,
        b5: f32,
    ) -> i32 {
        let a = shape_from_parts(type_a, a1, a2, a3, a4, a5);
        let b = shape_from_parts(type_b, b1, b2, b3, b4, b5);
        c2_collided(&a, &b)
    }
}

fn main() {
    // The original C source is a library and provides no executable entry point.
    // Reproducing this faithfully means producing no output.
    let _ = lib_c2::omni_collide;
}
