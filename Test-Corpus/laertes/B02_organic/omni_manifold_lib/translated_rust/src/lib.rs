extern "C" {
    fn sqrtf(__x: libc::c_float) -> libc::c_float;
    fn malloc(__size: size_t) -> *mut libc::c_void;
}
pub type C2_TYPE = libc::c_uint;
pub const C2_TYPE_POLY: C2_TYPE = 3;
pub const C2_TYPE_AABB: C2_TYPE = 2;
pub const C2_TYPE_CIRCLE: C2_TYPE = 1;
pub const C2_TYPE_CAPSULE: C2_TYPE = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct c2v {
    pub x: libc::c_float,
    pub y: libc::c_float,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct c2Manifold {
    pub count: libc::c_int,
    pub depths: [libc::c_float; 2],
    pub contact_points: [c2v; 2],
    pub n: c2v,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: libc::c_float,
}
pub type size_t = usize;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct c2Circle {
    pub p: c2v,
    pub r: libc::c_float,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct c2GJKCache {
    pub metric: libc::c_float,
    pub count: libc::c_int,
    pub iA: [libc::c_int; 3],
    pub iB: [libc::c_int; 3],
    pub div: libc::c_float,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct c2r {
    pub c: libc::c_float,
    pub s: libc::c_float,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct c2Simplex {
    pub a: c2sv,
    pub b: c2sv,
    pub c: c2sv,
    pub d: c2sv,
    pub div: libc::c_float,
    pub count: libc::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: libc::c_float,
    pub iA: libc::c_int,
    pub iB: libc::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct c2Proxy {
    pub radius: libc::c_float,
    pub count: libc::c_int,
    pub verts: [c2v; 8],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct c2Poly {
    pub count: libc::c_int,
    pub verts: [c2v; 8],
    pub norms: [c2v; 8],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct c2h {
    pub n: c2v,
    pub d: libc::c_float,
}
#[no_mangle]
pub extern "C" fn c2V(mut x: libc::c_float, mut y: libc::c_float) -> c2v {
    let mut a: c2v = c2v { x: 0., y: 0. };
    a.x = x;
    a.y = y;
    return a;
}
#[no_mangle]
pub extern "C" fn c2Mulvs(mut a: c2v, mut b: libc::c_float) -> c2v {
    a.x *= b;
    a.y *= b;
    return a;
}
#[no_mangle]
pub extern "C" fn c2Maxv(mut a: c2v, mut b: c2v) -> c2v {
    return c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    );
}
#[no_mangle]
pub extern "C" fn c2Minv(mut a: c2v, mut b: c2v) -> c2v {
    return c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    );
}
#[no_mangle]
pub extern "C" fn c2Clampv(mut a: c2v, mut lo: c2v, mut hi: c2v) -> c2v {
    return c2Maxv(lo, c2Minv(a, hi));
}
#[no_mangle]
pub extern "C" fn c2Sub(mut a: c2v, mut b: c2v) -> c2v {
    a.x -= b.x;
    a.y -= b.y;
    return a;
}
#[no_mangle]
pub extern "C" fn c2Dot(mut a: c2v, mut b: c2v) -> libc::c_float {
    return a.x * b.x + a.y * b.y;
}
#[no_mangle]
pub extern "C" fn c2Dist(mut h: c2h, mut p: c2v) -> libc::c_float {
    return c2Dot(h.n, p) - h.d;
}
#[no_mangle]
pub unsafe extern "C" fn c2PlaneAt(mut p: *const c2Poly, i: libc::c_int) -> c2h {
    let mut h: c2h = c2h {
        n: c2v { x: 0., y: 0. },
        d: 0.,
    };
    h.n = (*p).norms[i as usize];
    h.d = c2Dot((*p).norms[i as usize], (*p).verts[i as usize]);
    return h;
}
#[no_mangle]
pub extern "C" fn c2RotIdentity() -> c2r {
    let mut r: c2r = c2r { c: 0., s: 0. };
    r.c = 1.0f32;
    r.s = 0 as libc::c_int as libc::c_float;
    return r;
}
#[no_mangle]
pub extern "C" fn c2xIdentity() -> c2x {
    let mut x: c2x = c2x {
        p: c2v { x: 0., y: 0. },
        r: c2r { c: 0., s: 0. },
    };
    x.p = c2V(
        0 as libc::c_int as libc::c_float,
        0 as libc::c_int as libc::c_float,
    );
    x.r = c2RotIdentity();
    return x;
}
#[no_mangle]
pub unsafe extern "C" fn c2BBVerts(mut out: *mut c2v, mut bb: *mut c2AABB) {
    *out.offset(0 as libc::c_int as isize) = (*bb).min;
    *out.offset(1 as libc::c_int as isize) = c2V((*bb).max.x, (*bb).min.y);
    *out.offset(2 as libc::c_int as isize) = (*bb).max;
    *out.offset(3 as libc::c_int as isize) = c2V((*bb).min.x, (*bb).max.y);
}
#[no_mangle]
pub unsafe extern "C" fn c2MakeProxy(
    mut shape: *const libc::c_void,
    mut type_0: C2_TYPE,
    mut p: *mut c2Proxy,
) {
    match type_0 as libc::c_uint {
        1 => {
            let mut c: *mut c2Circle = shape as *mut c2Circle;
            (*p).radius = (*c).r;
            (*p).count = 1 as libc::c_int;
            (*p).verts[0 as libc::c_int as usize] = (*c).p;
        }
        2 => {
            let mut bb: *mut c2AABB = shape as *mut c2AABB;
            (*p).radius = 0 as libc::c_int as libc::c_float;
            (*p).count = 4 as libc::c_int;
            c2BBVerts(&raw mut (*p).verts as *mut c2v, bb);
        }
        0 => {
            let mut c_0: *mut c2Capsule = shape as *mut c2Capsule;
            (*p).radius = (*c_0).r;
            (*p).count = 2 as libc::c_int;
            (*p).verts[0 as libc::c_int as usize] = (*c_0).a;
            (*p).verts[1 as libc::c_int as usize] = (*c_0).b;
        }
        _ => {}
    };
}
#[no_mangle]
pub unsafe extern "C" fn c2Len(mut a: c2v) -> libc::c_float {
    return sqrtf(c2Dot(a, a));
}
#[no_mangle]
pub extern "C" fn c2Det2(mut a: c2v, mut b: c2v) -> libc::c_float {
    return a.x * b.y - a.y * b.x;
}
#[no_mangle]
pub unsafe extern "C" fn c2GJKSimplexMetric(mut s: *mut c2Simplex) -> libc::c_float {
    match (*s).count {
        2 => return c2Len(c2Sub((*s).b.p, (*s).a.p)),
        3 => return c2Det2(c2Sub((*s).b.p, (*s).a.p), c2Sub((*s).c.p, (*s).a.p)),
        1 | _ => return 0 as libc::c_int as libc::c_float,
    };
}
#[no_mangle]
pub extern "C" fn c2Mulrv(mut a: c2r, mut b: c2v) -> c2v {
    return c2V(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y);
}
#[no_mangle]
pub extern "C" fn c2MulrvT(mut a: c2r, mut b: c2v) -> c2v {
    return c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y);
}
#[no_mangle]
pub extern "C" fn c2Add(mut a: c2v, mut b: c2v) -> c2v {
    a.x += b.x;
    a.y += b.y;
    return a;
}
#[no_mangle]
pub extern "C" fn c2Mulxv(mut a: c2x, mut b: c2v) -> c2v {
    return c2Add(c2Mulrv(a.r, b), a.p);
}
#[no_mangle]
pub extern "C" fn c2MulxvT(mut a: c2x, mut b: c2v) -> c2v {
    return c2MulrvT(a.r, c2Sub(b, a.p));
}
#[no_mangle]
pub extern "C" fn c2Intersect(
    mut a: c2v,
    mut b: c2v,
    mut da: libc::c_float,
    mut db: libc::c_float,
) -> c2v {
    return c2Add(a, c2Mulvs(c2Sub(b, a), da / (da - db)));
}
unsafe extern "C" fn c2Clip(mut seg: *mut c2v, mut h: c2h) -> libc::c_int {
    let mut out: [c2v; 2] = [c2v { x: 0., y: 0. }; 2];
    let mut sp: libc::c_int = 0 as libc::c_int;
    let mut d0: libc::c_float = 0.;
    let mut d1: libc::c_float = 0.;
    d0 = c2Dist(h, *seg.offset(0 as libc::c_int as isize));
    if d0 < 0 as libc::c_int as libc::c_float {
        let fresh0 = sp;
        sp = sp + 1;
        out[fresh0 as usize] = *seg.offset(0 as libc::c_int as isize);
    }
    d1 = c2Dist(h, *seg.offset(1 as libc::c_int as isize));
    if d1 < 0 as libc::c_int as libc::c_float {
        let fresh1 = sp;
        sp = sp + 1;
        out[fresh1 as usize] = *seg.offset(1 as libc::c_int as isize);
    }
    if d0 == 0 as libc::c_int as libc::c_float
        && d1 == 0 as libc::c_int as libc::c_float
    {
        let fresh2 = sp;
        sp = sp + 1;
        out[fresh2 as usize] = *seg.offset(0 as libc::c_int as isize);
        let fresh3 = sp;
        sp = sp + 1;
        out[fresh3 as usize] = *seg.offset(1 as libc::c_int as isize);
    } else if d0 * d1 <= 0 as libc::c_int as libc::c_float {
        let fresh4 = sp;
        sp = sp + 1;
        out[fresh4 as usize] = c2Intersect(
            *seg.offset(0 as libc::c_int as isize),
            *seg.offset(1 as libc::c_int as isize),
            d0,
            d1,
        );
    }
    *seg.offset(0 as libc::c_int as isize) = out[0 as libc::c_int as usize];
    *seg.offset(1 as libc::c_int as isize) = out[1 as libc::c_int as usize];
    return sp;
}
#[no_mangle]
pub extern "C" fn c2Div(mut a: c2v, mut b: libc::c_float) -> c2v {
    return c2Mulvs(a, 1.0f32 / b);
}
#[no_mangle]
pub unsafe extern "C" fn c2Norm(mut a: c2v) -> c2v {
    return c2Div(a, c2Len(a));
}
#[no_mangle]
pub extern "C" fn c2Neg(mut a: c2v) -> c2v {
    return c2V(-a.x, -a.y);
}
#[no_mangle]
pub extern "C" fn c2CCW90(mut a: c2v) -> c2v {
    let mut b: c2v = c2v { x: 0., y: 0. };
    b.x = a.y;
    b.y = -a.x;
    return b;
}
unsafe extern "C" fn c2SidePlanes(
    mut seg: *mut c2v,
    mut ra: c2v,
    mut rb: c2v,
    mut h: *mut c2h,
) -> libc::c_int {
    let mut in_0: c2v = c2Norm(c2Sub(rb, ra));
    let mut left: c2h = c2h {
        n: c2Neg(in_0),
        d: c2Dot(c2Neg(in_0), ra),
    };
    let mut right: c2h = c2h {
        n: in_0,
        d: c2Dot(in_0, rb),
    };
    if c2Clip(seg, left) < 2 as libc::c_int {
        return 0 as libc::c_int;
    }
    if c2Clip(seg, right) < 2 as libc::c_int {
        return 0 as libc::c_int;
    }
    if !h.is_null() {
        (*h).n = c2CCW90(in_0);
        (*h).d = c2Dot(c2CCW90(in_0), ra);
    }
    return 1 as libc::c_int;
}
unsafe extern "C" fn c2SidePlanesFromPoly(
    mut seg: *mut c2v,
    mut x: c2x,
    mut p: *const c2Poly,
    mut e: libc::c_int,
    mut h: *mut c2h,
) -> libc::c_int {
    let mut ra: c2v = c2Mulxv(x, (*p).verts[e as usize]);
    let mut rb: c2v = c2Mulxv(
        x,
        (*p).verts[(if e + 1 as libc::c_int == (*p).count {
            0 as libc::c_int
        } else {
            e + 1 as libc::c_int
        }) as usize],
    );
    return c2SidePlanes(seg, ra, rb, h);
}
#[no_mangle]
pub unsafe extern "C" fn c22(mut s: *mut c2Simplex) {
    let mut a: c2v = (*s).a.p;
    let mut b: c2v = (*s).b.p;
    let mut u: libc::c_float = c2Dot(b, c2Sub(b, a));
    let mut v: libc::c_float = c2Dot(a, c2Sub(a, b));
    if v <= 0 as libc::c_int as libc::c_float {
        (*s).a.u = 1.0f32;
        (*s).div = 1.0f32;
        (*s).count = 1 as libc::c_int;
    } else if u <= 0 as libc::c_int as libc::c_float {
        (*s).a = (*s).b;
        (*s).a.u = 1.0f32;
        (*s).div = 1.0f32;
        (*s).count = 1 as libc::c_int;
    } else {
        (*s).a.u = u;
        (*s).b.u = v;
        (*s).div = u + v;
        (*s).count = 2 as libc::c_int;
    };
}
#[no_mangle]
pub unsafe extern "C" fn c23(mut s: *mut c2Simplex) {
    let mut a: c2v = (*s).a.p;
    let mut b: c2v = (*s).b.p;
    let mut c: c2v = (*s).c.p;
    let mut uAB: libc::c_float = c2Dot(b, c2Sub(b, a));
    let mut vAB: libc::c_float = c2Dot(a, c2Sub(a, b));
    let mut uBC: libc::c_float = c2Dot(c, c2Sub(c, b));
    let mut vBC: libc::c_float = c2Dot(b, c2Sub(b, c));
    let mut uCA: libc::c_float = c2Dot(a, c2Sub(a, c));
    let mut vCA: libc::c_float = c2Dot(c, c2Sub(c, a));
    let mut area: libc::c_float = c2Det2(c2Sub(b, a), c2Sub(c, a));
    let mut uABC: libc::c_float = c2Det2(b, c) * area;
    let mut vABC: libc::c_float = c2Det2(c, a) * area;
    let mut wABC: libc::c_float = c2Det2(a, b) * area;
    if vAB <= 0 as libc::c_int as libc::c_float
        && uCA <= 0 as libc::c_int as libc::c_float
    {
        (*s).a.u = 1.0f32;
        (*s).div = 1.0f32;
        (*s).count = 1 as libc::c_int;
    } else if uAB <= 0 as libc::c_int as libc::c_float
        && vBC <= 0 as libc::c_int as libc::c_float
    {
        (*s).a = (*s).b;
        (*s).a.u = 1.0f32;
        (*s).div = 1.0f32;
        (*s).count = 1 as libc::c_int;
    } else if uBC <= 0 as libc::c_int as libc::c_float
        && vCA <= 0 as libc::c_int as libc::c_float
    {
        (*s).a = (*s).c;
        (*s).a.u = 1.0f32;
        (*s).div = 1.0f32;
        (*s).count = 1 as libc::c_int;
    } else if uAB > 0 as libc::c_int as libc::c_float
        && vAB > 0 as libc::c_int as libc::c_float
        && wABC <= 0 as libc::c_int as libc::c_float
    {
        (*s).a.u = uAB;
        (*s).b.u = vAB;
        (*s).div = uAB + vAB;
        (*s).count = 2 as libc::c_int;
    } else if uBC > 0 as libc::c_int as libc::c_float
        && vBC > 0 as libc::c_int as libc::c_float
        && uABC <= 0 as libc::c_int as libc::c_float
    {
        (*s).a = (*s).b;
        (*s).b = (*s).c;
        (*s).a.u = uBC;
        (*s).b.u = vBC;
        (*s).div = uBC + vBC;
        (*s).count = 2 as libc::c_int;
    } else if uCA > 0 as libc::c_int as libc::c_float
        && vCA > 0 as libc::c_int as libc::c_float
        && vABC <= 0 as libc::c_int as libc::c_float
    {
        (*s).b = (*s).a;
        (*s).a = (*s).c;
        (*s).a.u = uCA;
        (*s).b.u = vCA;
        (*s).div = uCA + vCA;
        (*s).count = 2 as libc::c_int;
    } else {
        (*s).a.u = uABC;
        (*s).b.u = vABC;
        (*s).c.u = wABC;
        (*s).div = uABC + vABC + wABC;
        (*s).count = 3 as libc::c_int;
    };
}
#[no_mangle]
pub extern "C" fn c2Skew(mut a: c2v) -> c2v {
    let mut b: c2v = c2v { x: 0., y: 0. };
    b.x = -a.y;
    b.y = a.x;
    return b;
}
#[no_mangle]
pub unsafe extern "C" fn c2D(mut s: *mut c2Simplex) -> c2v {
    match (*s).count {
        1 => return c2Neg((*s).a.p),
        2 => {
            let mut ab: c2v = c2Sub((*s).b.p, (*s).a.p);
            if c2Det2(ab, c2Neg((*s).a.p)) > 0 as libc::c_int as libc::c_float {
                return c2Skew(ab);
            }
            return c2CCW90(ab);
        }
        3 | _ => {
            return c2V(
                0 as libc::c_int as libc::c_float,
                0 as libc::c_int as libc::c_float,
            );
        }
    };
}
#[no_mangle]
pub unsafe extern "C" fn c2Support(
    mut verts: *const c2v,
    mut count: libc::c_int,
    mut d: c2v,
) -> libc::c_int {
    let mut imax: libc::c_int = 0 as libc::c_int;
    let mut dmax: libc::c_float = c2Dot(*verts.offset(0 as libc::c_int as isize), d);
    let mut i: libc::c_int = 1 as libc::c_int;
    while i < count {
        let mut dot: libc::c_float = c2Dot(*verts.offset(i as isize), d);
        if dot > dmax {
            imax = i;
            dmax = dot;
        }
        i += 1;
    }
    return imax;
}
#[no_mangle]
pub unsafe extern "C" fn c2Witness(mut s: *mut c2Simplex, mut a: *mut c2v, mut b: *mut c2v) {
    let mut den: libc::c_float = 1.0f32 / (*s).div;
    match (*s).count {
        1 => {
            *a = (*s).a.sA;
            *b = (*s).a.sB;
        }
        2 => {
            *a = c2Add(
                c2Mulvs((*s).a.sA, den * (*s).a.u),
                c2Mulvs((*s).b.sA, den * (*s).b.u),
            );
            *b = c2Add(
                c2Mulvs((*s).a.sB, den * (*s).a.u),
                c2Mulvs((*s).b.sB, den * (*s).b.u),
            );
        }
        3 => {
            *a = c2Add(
                c2Add(
                    c2Mulvs((*s).a.sA, den * (*s).a.u),
                    c2Mulvs((*s).b.sA, den * (*s).b.u),
                ),
                c2Mulvs((*s).c.sA, den * (*s).c.u),
            );
            *b = c2Add(
                c2Add(
                    c2Mulvs((*s).a.sB, den * (*s).a.u),
                    c2Mulvs((*s).b.sB, den * (*s).b.u),
                ),
                c2Mulvs((*s).c.sB, den * (*s).c.u),
            );
        }
        _ => {
            *a = c2V(
                0 as libc::c_int as libc::c_float,
                0 as libc::c_int as libc::c_float,
            );
            *b = c2V(
                0 as libc::c_int as libc::c_float,
                0 as libc::c_int as libc::c_float,
            );
        }
    };
}
#[no_mangle]
pub unsafe extern "C" fn c2L(mut s: *mut c2Simplex) -> c2v {
    let mut den: libc::c_float = 1.0f32 / (*s).div;
    match (*s).count {
        1 => return (*s).a.p,
        2 => {
            return c2Add(
                c2Mulvs((*s).a.p, den * (*s).a.u),
                c2Mulvs((*s).b.p, den * (*s).b.u),
            );
        }
        _ => {
            return c2V(
                0 as libc::c_int as libc::c_float,
                0 as libc::c_int as libc::c_float,
            );
        }
    };
}
#[no_mangle]
pub unsafe extern "C" fn c2GJK(
    mut A: *const libc::c_void,
    mut typeA: C2_TYPE,
    mut ax_ptr: *const c2x,
    mut B: *const libc::c_void,
    mut typeB: C2_TYPE,
    mut bx_ptr: *const c2x,
    mut outA: *mut c2v,
    mut outB: *mut c2v,
    mut use_radius: libc::c_int,
    mut iterations: *mut libc::c_int,
    mut cache: *mut c2GJKCache,
) -> libc::c_float {
    let mut ax: c2x = c2x {
        p: c2v { x: 0., y: 0. },
        r: c2r { c: 0., s: 0. },
    };
    let mut bx: c2x = c2x {
        p: c2v { x: 0., y: 0. },
        r: c2r { c: 0., s: 0. },
    };
    if ax_ptr.is_null() {
        ax = c2xIdentity();
    } else {
        ax = *ax_ptr;
    }
    if bx_ptr.is_null() {
        bx = c2xIdentity();
    } else {
        bx = *bx_ptr;
    }
    let mut pA: c2Proxy = c2Proxy {
        radius: 0.,
        count: 0,
        verts: [c2v { x: 0., y: 0. }; 8],
    };
    let mut pB: c2Proxy = c2Proxy {
        radius: 0.,
        count: 0,
        verts: [c2v { x: 0., y: 0. }; 8],
    };
    c2MakeProxy(A, typeA, &raw mut pA);
    c2MakeProxy(B, typeB, &raw mut pB);
    let mut s: c2Simplex = c2Simplex {
        a: c2sv {
            sA: c2v { x: 0., y: 0. },
            sB: c2v { x: 0., y: 0. },
            p: c2v { x: 0., y: 0. },
            u: 0.,
            iA: 0,
            iB: 0,
        },
        b: c2sv {
            sA: c2v { x: 0., y: 0. },
            sB: c2v { x: 0., y: 0. },
            p: c2v { x: 0., y: 0. },
            u: 0.,
            iA: 0,
            iB: 0,
        },
        c: c2sv {
            sA: c2v { x: 0., y: 0. },
            sB: c2v { x: 0., y: 0. },
            p: c2v { x: 0., y: 0. },
            u: 0.,
            iA: 0,
            iB: 0,
        },
        d: c2sv {
            sA: c2v { x: 0., y: 0. },
            sB: c2v { x: 0., y: 0. },
            p: c2v { x: 0., y: 0. },
            u: 0.,
            iA: 0,
            iB: 0,
        },
        div: 0.,
        count: 0,
    };
    let mut verts: *mut c2sv = &raw mut s.a;
    let mut cache_was_read: libc::c_int = 0 as libc::c_int;
    if !cache.is_null() {
        let mut cache_was_good: libc::c_int = ((*cache).count != 0) as libc::c_int;
        if cache_was_good != 0 {
            let mut i: libc::c_int = 0 as libc::c_int;
            while i < (*cache).count {
                let mut iA: libc::c_int = (*cache).iA[i as usize];
                let mut iB: libc::c_int = (*cache).iB[i as usize];
                let mut sA: c2v = c2Mulxv(ax, pA.verts[iA as usize]);
                let mut sB: c2v = c2Mulxv(bx, pB.verts[iB as usize]);
                let mut v: *mut c2sv = verts.offset(i as isize);
                (*v).iA = iA;
                (*v).sA = sA;
                (*v).iB = iB;
                (*v).sB = sB;
                (*v).p = c2Sub((*v).sB, (*v).sA);
                (*v).u = 0 as libc::c_int as libc::c_float;
                i += 1;
            }
            s.count = (*cache).count;
            s.div = (*cache).div;
            let mut metric_old: libc::c_float = (*cache).metric;
            let mut metric: libc::c_float = c2GJKSimplexMetric(&raw mut s);
            let mut min_metric: libc::c_float = if metric < metric_old {
                metric
            } else {
                metric_old
            };
            let mut max_metric: libc::c_float = if metric > metric_old {
                metric
            } else {
                metric_old
            };
            if !(min_metric < max_metric * 2.0f32 && metric < -1.0e8f32) {
                cache_was_read = 1 as libc::c_int;
            }
        }
    }
    if cache_was_read == 0 {
        s.a.iA = 0 as libc::c_int;
        s.a.iB = 0 as libc::c_int;
        s.a.sA = c2Mulxv(ax, pA.verts[0 as libc::c_int as usize]);
        s.a.sB = c2Mulxv(bx, pB.verts[0 as libc::c_int as usize]);
        s.a.p = c2Sub(s.a.sB, s.a.sA);
        s.a.u = 1.0f32;
        s.div = 1.0f32;
        s.count = 1 as libc::c_int;
    }
    let mut saveA: [libc::c_int; 3] = [0; 3];
    let mut saveB: [libc::c_int; 3] = [0; 3];
    let mut save_count: libc::c_int = 0 as libc::c_int;
    let mut d0: libc::c_float = 3.40282346638528859811704183484516925e+38f32;
    let mut d1: libc::c_float = 3.40282346638528859811704183484516925e+38f32;
    let mut iter: libc::c_int = 0 as libc::c_int;
    let mut hit: libc::c_int = 0 as libc::c_int;
    while iter < 20 as libc::c_int {
        save_count = s.count;
        let mut i_0: libc::c_int = 0 as libc::c_int;
        while i_0 < save_count {
            saveA[i_0 as usize] = (*verts.offset(i_0 as isize)).iA;
            saveB[i_0 as usize] = (*verts.offset(i_0 as isize)).iB;
            i_0 += 1;
        }
        match s.count {
            2 => {
                c22(&raw mut s);
            }
            3 => {
                c23(&raw mut s);
            }
            1 | _ => {}
        }
        if s.count == 3 as libc::c_int {
            hit = 1 as libc::c_int;
            break;
        } else {
            let mut p: c2v = c2L(&raw mut s);
            d1 = c2Dot(p, p);
            if d1 > d0 {
                break;
            }
            d0 = d1;
            let mut d: c2v = c2D(&raw mut s);
            if c2Dot(d, d)
                < 1.19209289550781250000000000000000000e-7f32
                    * 1.19209289550781250000000000000000000e-7f32
            {
                break;
            }
            let mut iA_0: libc::c_int = c2Support(
                &raw mut pA.verts as *mut c2v,
                pA.count,
                c2MulrvT(ax.r, c2Neg(d)),
            );
            let mut sA_0: c2v = c2Mulxv(ax, pA.verts[iA_0 as usize]);
            let mut iB_0: libc::c_int =
                c2Support(&raw mut pB.verts as *mut c2v, pB.count, c2MulrvT(bx.r, d));
            let mut sB_0: c2v = c2Mulxv(bx, pB.verts[iB_0 as usize]);
            let mut v_0: *mut c2sv = verts.offset(s.count as isize);
            (*v_0).iA = iA_0;
            (*v_0).sA = sA_0;
            (*v_0).iB = iB_0;
            (*v_0).sB = sB_0;
            (*v_0).p = c2Sub((*v_0).sB, (*v_0).sA);
            let mut dup: libc::c_int = 0 as libc::c_int;
            let mut i_1: libc::c_int = 0 as libc::c_int;
            while i_1 < save_count {
                if iA_0 == saveA[i_1 as usize] && iB_0 == saveB[i_1 as usize] {
                    dup = 1 as libc::c_int;
                    break;
                } else {
                    i_1 += 1;
                }
            }
            if dup != 0 {
                break;
            }
            s.count += 1;
            iter += 1;
        }
    }
    let mut a: c2v = c2v { x: 0., y: 0. };
    let mut b: c2v = c2v { x: 0., y: 0. };
    c2Witness(&raw mut s, &raw mut a, &raw mut b);
    let mut dist: libc::c_float = c2Len(c2Sub(a, b));
    if hit != 0 {
        a = b;
        dist = 0 as libc::c_int as libc::c_float;
    } else if use_radius != 0 {
        let mut rA: libc::c_float = pA.radius;
        let mut rB: libc::c_float = pB.radius;
        if dist > rA + rB && dist > 1.19209289550781250000000000000000000e-7f32 {
            dist -= rA + rB;
            let mut n: c2v = c2Norm(c2Sub(b, a));
            a = c2Add(a, c2Mulvs(n, rA));
            b = c2Sub(b, c2Mulvs(n, rB));
            if a.x == b.x && a.y == b.y {
                dist = 0 as libc::c_int as libc::c_float;
            }
        } else {
            let mut p_0: c2v = c2Mulvs(c2Add(a, b), 0.5f32);
            a = p_0;
            b = p_0;
            dist = 0 as libc::c_int as libc::c_float;
        }
    }
    if !cache.is_null() {
        (*cache).metric = c2GJKSimplexMetric(&raw mut s);
        (*cache).count = s.count;
        let mut i_2: libc::c_int = 0 as libc::c_int;
        while i_2 < s.count {
            let mut v_1: *mut c2sv = verts.offset(i_2 as isize);
            (*cache).iA[i_2 as usize] = (*v_1).iA;
            (*cache).iB[i_2 as usize] = (*v_1).iB;
            i_2 += 1;
        }
        (*cache).div = s.div;
    }
    if !outA.is_null() {
        *outA = a;
    }
    if !outB.is_null() {
        *outB = b;
    }
    if !iterations.is_null() {
        *iterations = iter;
    }
    return dist;
}
#[no_mangle]
pub extern "C" fn c2Absv(mut a: c2v) -> c2v {
    return c2V(
        if a.x < 0 as libc::c_int as libc::c_float {
            -a.x
        } else {
            a.x
        },
        if a.y < 0 as libc::c_int as libc::c_float {
            -a.y
        } else {
            a.y
        },
    );
}
#[no_mangle]
pub unsafe extern "C" fn c2CircletoCircleManifold(
    mut A: c2Circle,
    mut B: c2Circle,
    mut m: *mut c2Manifold,
) {
    (*m).count = 0 as libc::c_int;
    let mut d: c2v = c2Sub(B.p, A.p);
    let mut d2: libc::c_float = c2Dot(d, d);
    let mut r: libc::c_float = A.r + B.r;
    if d2 < r * r {
        let mut l: libc::c_float = sqrtf(d2);
        let mut n: c2v = if l != 0 as libc::c_int as libc::c_float {
            c2Mulvs(d, 1.0f32 / l)
        } else {
            c2V(0 as libc::c_int as libc::c_float, 1.0f32)
        };
        (*m).count = 1 as libc::c_int;
        (*m).depths[0 as libc::c_int as usize] = r - l;
        (*m).contact_points[0 as libc::c_int as usize] = c2Sub(B.p, c2Mulvs(n, B.r));
        (*m).n = n;
    }
}
#[no_mangle]
pub unsafe extern "C" fn c2CircletoAABBManifold(
    mut A: c2Circle,
    mut B: c2AABB,
    mut m: *mut c2Manifold,
) {
    (*m).count = 0 as libc::c_int;
    let mut L: c2v = c2Clampv(A.p, B.min, B.max);
    let mut ab: c2v = c2Sub(L, A.p);
    let mut d2: libc::c_float = c2Dot(ab, ab);
    let mut r2: libc::c_float = A.r * A.r;
    if d2 < r2 {
        if d2 != 0 as libc::c_int as libc::c_float {
            let mut d: libc::c_float = sqrtf(d2);
            let mut n: c2v = c2Norm(ab);
            (*m).count = 1 as libc::c_int;
            (*m).depths[0 as libc::c_int as usize] = A.r - d;
            (*m).contact_points[0 as libc::c_int as usize] = c2Add(A.p, c2Mulvs(n, d));
            (*m).n = n;
        } else {
            let mut mid: c2v = c2Mulvs(c2Add(B.min, B.max), 0.5f32);
            let mut e: c2v = c2Mulvs(c2Sub(B.max, B.min), 0.5f32);
            let mut d_0: c2v = c2Sub(A.p, mid);
            let mut abs_d: c2v = c2Absv(d_0);
            let mut x_overlap: libc::c_float = e.x - abs_d.x;
            let mut y_overlap: libc::c_float = e.y - abs_d.y;
            let mut depth: libc::c_float = 0.;
            let mut n_0: c2v = c2v { x: 0., y: 0. };
            if x_overlap < y_overlap {
                depth = x_overlap;
                n_0 = c2V(1.0f32, 0 as libc::c_int as libc::c_float);
                n_0 = c2Mulvs(
                    n_0,
                    if d_0.x < 0 as libc::c_int as libc::c_float {
                        1.0f32
                    } else {
                        -1.0f32
                    },
                );
            } else {
                depth = y_overlap;
                n_0 = c2V(0 as libc::c_int as libc::c_float, 1.0f32);
                n_0 = c2Mulvs(
                    n_0,
                    if d_0.y < 0 as libc::c_int as libc::c_float {
                        1.0f32
                    } else {
                        -1.0f32
                    },
                );
            }
            (*m).count = 1 as libc::c_int;
            (*m).depths[0 as libc::c_int as usize] = A.r + depth;
            (*m).contact_points[0 as libc::c_int as usize] = c2Sub(A.p, c2Mulvs(n_0, depth));
            (*m).n = n_0;
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn c2CircletoCapsuleManifold(
    mut A: c2Circle,
    mut B: c2Capsule,
    mut m: *mut c2Manifold,
) {
    (*m).count = 0 as libc::c_int;
    let mut a: c2v = c2v { x: 0., y: 0. };
    let mut b: c2v = c2v { x: 0., y: 0. };
    let mut r: libc::c_float = A.r + B.r;
    let mut d: libc::c_float = c2GJK(
        &raw mut A as *const libc::c_void,
        C2_TYPE_CIRCLE,
        std::ptr::null::<c2x>(),
        &raw mut B as *const libc::c_void,
        C2_TYPE_CAPSULE,
        std::ptr::null::<c2x>(),
        &raw mut a,
        &raw mut b,
        0 as libc::c_int,
        std::ptr::null_mut::<libc::c_int>(),
        std::ptr::null_mut::<c2GJKCache>(),
    );
    if d < r {
        let mut n: c2v = c2v { x: 0., y: 0. };
        if d == 0 as libc::c_int as libc::c_float {
            n = c2Norm(c2Skew(c2Sub(B.b, B.a)));
        } else {
            n = c2Norm(c2Sub(b, a));
        }
        (*m).count = 1 as libc::c_int;
        (*m).depths[0 as libc::c_int as usize] = r - d;
        (*m).contact_points[0 as libc::c_int as usize] = c2Sub(b, c2Mulvs(n, B.r));
        (*m).n = n;
    }
}
#[no_mangle]
pub unsafe extern "C" fn c2AABBtoAABBManifold(
    mut A: c2AABB,
    mut B: c2AABB,
    mut m: *mut c2Manifold,
) {
    (*m).count = 0 as libc::c_int;
    let mut mid_a: c2v = c2Mulvs(c2Add(A.min, A.max), 0.5f32);
    let mut mid_b: c2v = c2Mulvs(c2Add(B.min, B.max), 0.5f32);
    let mut eA: c2v = c2Absv(c2Mulvs(c2Sub(A.max, A.min), 0.5f32));
    let mut eB: c2v = c2Absv(c2Mulvs(c2Sub(B.max, B.min), 0.5f32));
    let mut d: c2v = c2Sub(mid_b, mid_a);
    let mut dx: libc::c_float = eA.x + eB.x
        - (if d.x < 0 as libc::c_int as libc::c_float {
            -d.x
        } else {
            d.x
        });
    if dx < 0 as libc::c_int as libc::c_float {
        return;
    }
    let mut dy: libc::c_float = eA.y + eB.y
        - (if d.y < 0 as libc::c_int as libc::c_float {
            -d.y
        } else {
            d.y
        });
    if dy < 0 as libc::c_int as libc::c_float {
        return;
    }
    let mut n: c2v = c2v { x: 0., y: 0. };
    let mut depth: libc::c_float = 0.;
    let mut p: c2v = c2v { x: 0., y: 0. };
    if dx < dy {
        depth = dx;
        if d.x < 0 as libc::c_int as libc::c_float {
            n = c2V(-1.0f32, 0 as libc::c_int as libc::c_float);
            p = c2Sub(
                mid_a,
                c2V(eA.x, 0 as libc::c_int as libc::c_float),
            );
        } else {
            n = c2V(1.0f32, 0 as libc::c_int as libc::c_float);
            p = c2Add(
                mid_a,
                c2V(eA.x, 0 as libc::c_int as libc::c_float),
            );
        }
    } else {
        depth = dy;
        if d.y < 0 as libc::c_int as libc::c_float {
            n = c2V(0 as libc::c_int as libc::c_float, -1.0f32);
            p = c2Sub(
                mid_a,
                c2V(0 as libc::c_int as libc::c_float, eA.y),
            );
        } else {
            n = c2V(0 as libc::c_int as libc::c_float, 1.0f32);
            p = c2Add(
                mid_a,
                c2V(0 as libc::c_int as libc::c_float, eA.y),
            );
        }
    }
    (*m).count = 1 as libc::c_int;
    (*m).contact_points[0 as libc::c_int as usize] = p;
    (*m).depths[0 as libc::c_int as usize] = depth;
    (*m).n = n;
}
unsafe extern "C" fn c2KeepDeep(mut seg: *mut c2v, mut h: c2h, mut m: *mut c2Manifold) {
    let mut cp: libc::c_int = 0 as libc::c_int;
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < 2 as libc::c_int {
        let mut p: c2v = *seg.offset(i as isize);
        let mut d: libc::c_float = c2Dist(h, p);
        if d <= 0 as libc::c_int as libc::c_float {
            (*m).contact_points[cp as usize] = p;
            (*m).depths[cp as usize] = -d;
            cp += 1;
        }
        i += 1;
    }
    (*m).count = cp;
    (*m).n = h.n;
}
unsafe extern "C" fn c2Incident(
    mut incident: *mut c2v,
    mut ip: *const c2Poly,
    mut ix: c2x,
    mut rn_in_incident_space: c2v,
) {
    let mut index: libc::c_int = !(0 as libc::c_int);
    let mut min_dot: libc::c_float = 3.40282346638528859811704183484516925e+38f32;
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < (*ip).count {
        let mut dot: libc::c_float = c2Dot(rn_in_incident_space, (*ip).norms[i as usize]);
        if dot < min_dot {
            min_dot = dot;
            index = i;
        }
        i += 1;
    }
    *incident.offset(0 as libc::c_int as isize) = c2Mulxv(ix, (*ip).verts[index as usize]);
    *incident.offset(1 as libc::c_int as isize) = c2Mulxv(
        ix,
        (*ip).verts[(if index + 1 as libc::c_int == (*ip).count {
            0 as libc::c_int
        } else {
            index + 1 as libc::c_int
        }) as usize],
    );
}
#[no_mangle]
pub unsafe extern "C" fn c2CapsuletoPolyManifold(
    mut A: c2Capsule,
    mut B: *const c2Poly,
    mut bx_ptr: *const c2x,
    mut m: *mut c2Manifold,
) {
    (*m).count = 0 as libc::c_int;
    let mut a: c2v = c2v { x: 0., y: 0. };
    let mut b: c2v = c2v { x: 0., y: 0. };
    let mut d: libc::c_float = c2GJK(
        &raw mut A as *const libc::c_void,
        C2_TYPE_CAPSULE,
        std::ptr::null::<c2x>(),
        B as *const libc::c_void,
        C2_TYPE_POLY,
        bx_ptr,
        &raw mut a,
        &raw mut b,
        0 as libc::c_int,
        std::ptr::null_mut::<libc::c_int>(),
        std::ptr::null_mut::<c2GJKCache>(),
    );
    if d < 1.0e-6f32 {
        let mut bx: c2x = if !bx_ptr.is_null() {
            *bx_ptr
        } else {
            c2xIdentity()
        };
        let mut A_in_B: c2Capsule = c2Capsule {
            a: c2v { x: 0., y: 0. },
            b: c2v { x: 0., y: 0. },
            r: 0.,
        };
        A_in_B.a = c2MulxvT(bx, A.a);
        A_in_B.b = c2MulxvT(bx, A.b);
        let mut ab: c2v = c2Norm(c2Sub(A_in_B.a, A_in_B.b));
        let mut ab_h0: c2h = c2h {
            n: c2v { x: 0., y: 0. },
            d: 0.,
        };
        ab_h0.n = c2CCW90(ab);
        ab_h0.d = c2Dot(A_in_B.a, ab_h0.n);
        let mut v0: libc::c_int = c2Support(
            &raw const (*B).verts as *const c2v,
            (*B).count,
            c2Neg(ab_h0.n),
        );
        let mut s0: libc::c_float = c2Dist(ab_h0, (*B).verts[v0 as usize]);
        let mut ab_h1: c2h = c2h {
            n: c2v { x: 0., y: 0. },
            d: 0.,
        };
        ab_h1.n = c2Skew(ab);
        ab_h1.d = c2Dot(A_in_B.a, ab_h1.n);
        let mut v1: libc::c_int = c2Support(
            &raw const (*B).verts as *const c2v,
            (*B).count,
            c2Neg(ab_h1.n),
        );
        let mut s1: libc::c_float = c2Dist(ab_h1, (*B).verts[v1 as usize]);
        let mut index: libc::c_int = !(0 as libc::c_int);
        let mut sep: libc::c_float = -3.40282346638528859811704183484516925e+38f32;
        let mut code: libc::c_int = 0 as libc::c_int;
        let mut i: libc::c_int = 0 as libc::c_int;
        while i < (*B).count {
            let mut h: c2h = c2PlaneAt(B, i);
            let mut da: libc::c_float = c2Dot(A_in_B.a, c2Neg(h.n));
            let mut db: libc::c_float = c2Dot(A_in_B.b, c2Neg(h.n));
            let mut d_0: libc::c_float = 0.;
            if da > db {
                d_0 = c2Dist(h, A_in_B.a);
            } else {
                d_0 = c2Dist(h, A_in_B.b);
            }
            if d_0 > sep {
                sep = d_0;
                index = i;
            }
            i += 1;
        }
        if s0 > sep {
            sep = s0;
            index = v0;
            code = 1 as libc::c_int;
        }
        if s1 > sep {
            sep = s1;
            index = v1;
            code = 2 as libc::c_int;
        }
        match code {
            0 => {
                let mut seg: [c2v; 2] = [A.a, A.b];
                let mut h_0: c2h = c2h {
                    n: c2v { x: 0., y: 0. },
                    d: 0.,
                };
                if c2SidePlanesFromPoly(&raw mut seg as *mut c2v, bx, B, index, &raw mut h_0) == 0 {
                    return;
                }
                c2KeepDeep(&raw mut seg as *mut c2v, h_0, m);
                (*m).n = c2Neg((*m).n);
            }
            1 => {
                let mut incident: [c2v; 2] = [c2v { x: 0., y: 0. }; 2];
                c2Incident(&raw mut incident as *mut c2v, B, bx, ab_h0.n);
                let mut h_1: c2h = c2h {
                    n: c2v { x: 0., y: 0. },
                    d: 0.,
                };
                if c2SidePlanes(
                    &raw mut incident as *mut c2v,
                    A_in_B.b,
                    A_in_B.a,
                    &raw mut h_1,
                ) == 0
                {
                    return;
                }
                c2KeepDeep(&raw mut incident as *mut c2v, h_1, m);
            }
            2 => {
                let mut incident_0: [c2v; 2] = [c2v { x: 0., y: 0. }; 2];
                c2Incident(&raw mut incident_0 as *mut c2v, B, bx, ab_h1.n);
                let mut h_2: c2h = c2h {
                    n: c2v { x: 0., y: 0. },
                    d: 0.,
                };
                if c2SidePlanes(
                    &raw mut incident_0 as *mut c2v,
                    A_in_B.a,
                    A_in_B.b,
                    &raw mut h_2,
                ) == 0
                {
                    return;
                }
                c2KeepDeep(&raw mut incident_0 as *mut c2v, h_2, m);
            }
            _ => return,
        }
        let mut i_0: libc::c_int = 0 as libc::c_int;
        while i_0 < (*m).count {
            (*m).depths[i_0 as usize] += A.r;
            i_0 += 1;
        }
    } else if d < A.r {
        (*m).count = 1 as libc::c_int;
        (*m).n = c2Norm(c2Sub(b, a));
        (*m).contact_points[0 as libc::c_int as usize] = c2Add(a, c2Mulvs((*m).n, A.r));
        (*m).depths[0 as libc::c_int as usize] = A.r - d;
    }
}
#[no_mangle]
pub unsafe extern "C" fn c2Norms(
    mut verts: *mut c2v,
    mut norms: *mut c2v,
    mut count: libc::c_int,
) {
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < count {
        let mut a: libc::c_int = i;
        let mut b: libc::c_int = if (i + 1 as libc::c_int) < count {
            i + 1 as libc::c_int
        } else {
            0 as libc::c_int
        };
        let mut e: c2v = c2Sub(*verts.offset(b as isize), *verts.offset(a as isize));
        *norms.offset(i as isize) = c2Norm(c2CCW90(e));
        i += 1;
    }
}
#[no_mangle]
pub unsafe extern "C" fn c2AABBtoCapsuleManifold(
    mut A: c2AABB,
    mut B: c2Capsule,
    mut m: *mut c2Manifold,
) {
    (*m).count = 0 as libc::c_int;
    let mut p: c2Poly = c2Poly {
        count: 0,
        verts: [c2v { x: 0., y: 0. }; 8],
        norms: [c2v { x: 0., y: 0. }; 8],
    };
    c2BBVerts(&raw mut p.verts as *mut c2v, &raw mut A);
    p.count = 4 as libc::c_int;
    c2Norms(
        &raw mut p.verts as *mut c2v,
        &raw mut p.norms as *mut c2v,
        4 as libc::c_int,
    );
    c2CapsuletoPolyManifold(B, &raw mut p, std::ptr::null::<c2x>(), m);
    (*m).n = c2Neg((*m).n);
}
#[no_mangle]
pub unsafe extern "C" fn c2CapsuletoCapsuleManifold(
    mut A: c2Capsule,
    mut B: c2Capsule,
    mut m: *mut c2Manifold,
) {
    (*m).count = 0 as libc::c_int;
    let mut a: c2v = c2v { x: 0., y: 0. };
    let mut b: c2v = c2v { x: 0., y: 0. };
    let mut r: libc::c_float = A.r + B.r;
    let mut d: libc::c_float = c2GJK(
        &raw mut A as *const libc::c_void,
        C2_TYPE_CAPSULE,
        std::ptr::null::<c2x>(),
        &raw mut B as *const libc::c_void,
        C2_TYPE_CAPSULE,
        std::ptr::null::<c2x>(),
        &raw mut a,
        &raw mut b,
        0 as libc::c_int,
        std::ptr::null_mut::<libc::c_int>(),
        std::ptr::null_mut::<c2GJKCache>(),
    );
    if d < r {
        let mut n: c2v = c2v { x: 0., y: 0. };
        if d == 0 as libc::c_int as libc::c_float {
            n = c2Norm(c2Skew(c2Sub(A.b, A.a)));
        } else {
            n = c2Norm(c2Sub(b, a));
        }
        (*m).count = 1 as libc::c_int;
        (*m).depths[0 as libc::c_int as usize] = r - d;
        (*m).contact_points[0 as libc::c_int as usize] = c2Sub(b, c2Mulvs(n, B.r));
        (*m).n = n;
    }
}
#[no_mangle]
pub unsafe extern "C" fn c2Collide(
    mut A: *const libc::c_void,
    mut typeA: C2_TYPE,
    mut B: *const libc::c_void,
    mut typeB: C2_TYPE,
    mut m: *mut c2Manifold,
) {
    (*m).count = 0 as libc::c_int;
    match typeA as libc::c_uint {
        1 => match typeB as libc::c_uint {
            1 => {
                c2CircletoCircleManifold(*(A as *mut c2Circle), *(B as *mut c2Circle), m);
            }
            2 => {
                c2CircletoAABBManifold(*(A as *mut c2Circle), *(B as *mut c2AABB), m);
            }
            0 => {
                c2CircletoCapsuleManifold(*(A as *mut c2Circle), *(B as *mut c2Capsule), m);
            }
            _ => {}
        },
        2 => match typeB as libc::c_uint {
            1 => {
                c2CircletoAABBManifold(*(B as *mut c2Circle), *(A as *mut c2AABB), m);
                (*m).n = c2Neg((*m).n);
            }
            2 => {
                c2AABBtoAABBManifold(*(A as *mut c2AABB), *(B as *mut c2AABB), m);
            }
            0 => {
                c2AABBtoCapsuleManifold(*(A as *mut c2AABB), *(B as *mut c2Capsule), m);
            }
            _ => {}
        },
        0 => match typeB as libc::c_uint {
            1 => {
                c2CircletoCapsuleManifold(*(B as *mut c2Circle), *(A as *mut c2Capsule), m);
                (*m).n = c2Neg((*m).n);
            }
            2 => {
                c2AABBtoCapsuleManifold(*(B as *mut c2AABB), *(A as *mut c2Capsule), m);
                (*m).n = c2Neg((*m).n);
            }
            0 => {
                c2CapsuletoCapsuleManifold(*(A as *mut c2Capsule), *(B as *mut c2Capsule), m);
            }
            _ => {}
        },
        _ => {}
    };
}
#[no_mangle]
pub unsafe extern "C" fn ptr_from_parts(
    mut typ: C2_TYPE,
    mut a: libc::c_float,
    mut b: libc::c_float,
    mut c: libc::c_float,
    mut d: libc::c_float,
    mut e: libc::c_float,
) -> *mut libc::c_void {
    let mut circle: *mut c2Circle = std::ptr::null_mut::<c2Circle>();
    let mut aabb: *mut c2AABB = std::ptr::null_mut::<c2AABB>();
    let mut capsule: *mut c2Capsule = std::ptr::null_mut::<c2Capsule>();
    match typ as libc::c_uint {
        1 => {
            circle = malloc(std::mem::size_of::<c2Circle>() as size_t) as *mut c2Circle;
            (*circle).p = c2V(a, b);
            (*circle).r = c;
            return circle as *mut libc::c_void;
        }
        2 => {
            aabb = malloc(std::mem::size_of::<c2AABB>() as size_t) as *mut c2AABB;
            (*aabb).min = c2V(a, b);
            (*aabb).max = c2V(c, d);
            return aabb as *mut libc::c_void;
        }
        0 => {
            capsule = malloc(std::mem::size_of::<c2Capsule>() as size_t) as *mut c2Capsule;
            (*capsule).a = c2V(a, b);
            (*capsule).b = c2V(c, d);
            (*capsule).r = e;
            return capsule as *mut libc::c_void;
        }
        _ => {}
    }
    panic!("Reached end of non-void function without returning");
}
#[no_mangle]
pub unsafe extern "C" fn omni_manifold(
    mut m: *mut c2Manifold,
    mut type_a: C2_TYPE,
    mut a1: libc::c_float,
    mut a2: libc::c_float,
    mut a3: libc::c_float,
    mut a4: libc::c_float,
    mut a5: libc::c_float,
    mut type_b: C2_TYPE,
    mut b1: libc::c_float,
    mut b2: libc::c_float,
    mut b3: libc::c_float,
    mut b4: libc::c_float,
    mut b5: libc::c_float,
) {
    let mut A: *mut libc::c_void = ptr_from_parts(type_a, a1, a2, a3, a4, a5);
    let mut B: *mut libc::c_void = ptr_from_parts(type_b, b1, b2, b3, b4, b5);
    c2Collide(A, type_a, B, type_b, m);
}
