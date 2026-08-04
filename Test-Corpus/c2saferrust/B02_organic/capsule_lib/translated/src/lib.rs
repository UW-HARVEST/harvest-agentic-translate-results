







extern "C" {
    fn sqrtf(__x: ::core::ffi::c_float) -> ::core::ffi::c_float;
}
pub type C2_TYPE = ::core::ffi::c_uint;
pub const C2_TYPE_CAPSULE: C2_TYPE = 2;
pub const C2_TYPE_AABB: C2_TYPE = 1;
pub const C2_TYPE_CIRCLE: C2_TYPE = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: ::core::ffi::c_float,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct c2v {
    pub x: ::core::ffi::c_float,
    pub y: ::core::ffi::c_float,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct c2GJKCache {
    pub metric: ::core::ffi::c_float,
    pub count: ::core::ffi::c_int,
    pub iA: [::core::ffi::c_int; 3],
    pub iB: [::core::ffi::c_int; 3],
    pub div: ::core::ffi::c_float,
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
    pub c: ::core::ffi::c_float,
    pub s: ::core::ffi::c_float,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct c2Simplex {
    pub a: c2sv,
    pub b: c2sv,
    pub c: c2sv,
    pub d: c2sv,
    pub div: ::core::ffi::c_float,
    pub count: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: ::core::ffi::c_float,
    pub iA: ::core::ffi::c_int,
    pub iB: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct c2Proxy {
    pub radius: ::core::ffi::c_float,
    pub count: ::core::ffi::c_int,
    pub verts: [c2v; 8],
}
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
    pub r: ::core::ffi::c_float,
}
#[no_mangle]
pub fn c2V(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

#[no_mangle]
pub fn c2Mulvs(a: c2v, b: f32) -> c2v {
    c2v {
        x: a.x * b,
        y: a.y * b,
    }
}

#[no_mangle]
pub fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2v {
        x: a.x.max(b.x),
        y: a.y.max(b.y),
    }
}

#[no_mangle]
pub fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2V(a.x.min(b.x), a.y.min(b.y))
}

#[no_mangle]
pub fn c2Clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2Maxv(lo, c2Minv(a, hi))
}

#[no_mangle]
pub fn c2Sub(a: c2v, b: c2v) -> c2v {
    c2v {
        x: a.x - b.x,
        y: a.y - b.y,
    }
}

#[no_mangle]
pub fn c2Dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}

#[no_mangle]
pub unsafe extern "C" fn c2RotIdentity() -> c2r {
    let mut r: c2r = c2r { c: 0., s: 0. };
    r.c = 1.0f32;
    r.s = 0 as ::core::ffi::c_int as ::core::ffi::c_float;
    return r;
}
#[no_mangle]
pub unsafe extern "C" fn c2xIdentity() -> c2x {
    let mut x: c2x = c2x {
        p: c2v { x: 0., y: 0. },
        r: c2r { c: 0., s: 0. },
    };
    x.p = c2V(0.0f32, 0.0f32);
    x.r = c2RotIdentity();
    return x;
}
#[no_mangle]
pub unsafe extern "C" fn c2BBVerts(mut out: *mut c2v, mut bb: *mut c2AABB) {
    *out.offset(0 as ::core::ffi::c_int as isize) = (*bb).min;
    *out.offset(1 as ::core::ffi::c_int as isize) = c2V((*bb).max.x, (*bb).min.y);
    *out.offset(2 as ::core::ffi::c_int as isize) = (*bb).max;
    *out.offset(3 as ::core::ffi::c_int as isize) = c2V((*bb).min.x, (*bb).max.y);
}
#[no_mangle]
pub unsafe extern "C" fn c2MakeProxy(
    mut shape: *const ::core::ffi::c_void,
    mut type_0: C2_TYPE,
    mut p: *mut c2Proxy,
) {
    match type_0 as ::core::ffi::c_uint {
        0 => {
            let mut c: *mut c2Circle = shape as *mut c2Circle;
            (*p).radius = (*c).r;
            (*p).count = 1 as ::core::ffi::c_int;
            (*p).verts[0 as ::core::ffi::c_int as usize] = (*c).p;
        }
        1 => {
            let mut bb: *mut c2AABB = shape as *mut c2AABB;
            (*p).radius = 0 as ::core::ffi::c_int as ::core::ffi::c_float;
            (*p).count = 4 as ::core::ffi::c_int;
            c2BBVerts(&raw mut (*p).verts as *mut c2v, bb);
        }
        2 => {
            let mut c_0: *mut c2Capsule = shape as *mut c2Capsule;
            (*p).radius = (*c_0).r;
            (*p).count = 2 as ::core::ffi::c_int;
            (*p).verts[0 as ::core::ffi::c_int as usize] = (*c_0).a;
            (*p).verts[1 as ::core::ffi::c_int as usize] = (*c_0).b;
        }
        _ => {}
    };
}
#[no_mangle]
pub unsafe extern "C" fn c2Len(mut a: c2v) -> ::core::ffi::c_float {
    return sqrtf(c2Dot(a, a));
}
#[no_mangle]
pub unsafe extern "C" fn c2Det2(mut a: c2v, mut b: c2v) -> ::core::ffi::c_float {
    return a.x * b.y - a.y * b.x;
}
#[no_mangle]
pub unsafe extern "C" fn c2GJKSimplexMetric(mut s: *mut c2Simplex) -> ::core::ffi::c_float {
    match (*s).count {
        2 => return c2Len(c2Sub((*s).b.p, (*s).a.p)),
        3 => return c2Det2(c2Sub((*s).b.p, (*s).a.p), c2Sub((*s).c.p, (*s).a.p)),
        1 | _ => return 0 as ::core::ffi::c_int as ::core::ffi::c_float,
    };
    match (*s).count {
        2 => return c2Len(c2Sub((*s).b.p, (*s).a.p)),
        3 => return c2Det2(c2Sub((*s).b.p, (*s).a.p), c2Sub((*s).c.p, (*s).a.p)),
        1 | _ => return 0 as ::core::ffi::c_int as ::core::ffi::c_float,
    };
    match (*s).count {
        2 => return c2Len(c2Sub((*s).b.p, (*s).a.p)),
        3 => return c2Det2(c2Sub((*s).b.p, (*s).a.p), c2Sub((*s).c.p, (*s).a.p)),
        1 | _ => return 0 as ::core::ffi::c_int as ::core::ffi::c_float,
    };
}
#[no_mangle]
pub unsafe extern "C" fn c2Mulrv(mut a: c2r, mut b: c2v) -> c2v {
    return c2V(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y);
}
#[no_mangle]
pub unsafe extern "C" fn c2Add(mut a: c2v, mut b: c2v) -> c2v {
    a.x += b.x;
    a.y += b.y;
    return a;
}
#[no_mangle]
pub unsafe extern "C" fn c2Mulxv(mut a: c2x, mut b: c2v) -> c2v {
    return c2Add(c2Mulrv(a.r, b), a.p);
}
#[no_mangle]
pub unsafe extern "C" fn c22(mut s: *mut c2Simplex) {
    let mut a: c2v = (*s).a.p;
    let mut b: c2v = (*s).b.p;
    let mut u: f32 = c2Dot(b, c2Sub(b, a));
    let mut v: f32 = c2Dot(a, c2Sub(a, b));
    if v <= 0 as ::core::ffi::c_int as ::core::ffi::c_float {
        (*s).a.u = 1.0f32;
        (*s).div = 1.0f32;
        (*s).count = 1 as ::core::ffi::c_int;
    } else if u <= 0 as ::core::ffi::c_int as ::core::ffi::c_float {
        (*s).a = (*s).b;
        (*s).a.u = 1.0f32;
        (*s).div = 1.0f32;
        (*s).count = 1 as ::core::ffi::c_int;
    } else {
        (*s).a.u = u;
        (*s).b.u = v;
        (*s).div = u + v;
        (*s).count = 2 as ::core::ffi::c_int;
    };
}
#[no_mangle]
pub unsafe extern "C" fn c23(mut s: *mut c2Simplex) {
    let mut a: c2v = (*s).a.p;
    let mut b: c2v = (*s).b.p;
    let mut c: c2v = (*s).c.p;
    let mut uAB: f32 = c2Dot(b, c2Sub(b, a));
    let mut vAB: f32 = c2Dot(a, c2Sub(a, b));
    let mut uBC: f32 = c2Dot(c, c2Sub(c, b));
    let mut vBC: f32 = c2Dot(b, c2Sub(b, c));
    let mut uCA: f32 = c2Dot(a, c2Sub(a, c));
    let mut vCA: f32 = c2Dot(c, c2Sub(c, a));
    let mut area: ::core::ffi::c_float = c2Det2(c2Sub(b, a), c2Sub(c, a));
    let mut area: ::core::ffi::c_float = c2Det2(c2Sub(b, a), c2Sub(c, a));
    let mut uABC: ::core::ffi::c_float = c2Det2(b, c) * area;
    let mut vABC: ::core::ffi::c_float = c2Det2(c, a) * area;
    let mut wABC: ::core::ffi::c_float = c2Det2(a, b) * area;
    if vAB <= 0 as ::core::ffi::c_int as ::core::ffi::c_float
        && uCA <= 0 as ::core::ffi::c_int as ::core::ffi::c_float
    {
        (*s).a.u = 1.0f32;
        (*s).div = 1.0f32;
        (*s).count = 1 as ::core::ffi::c_int;
    } else if uAB <= 0 as ::core::ffi::c_int as ::core::ffi::c_float
        && vBC <= 0 as ::core::ffi::c_int as ::core::ffi::c_float
    {
        (*s).a = (*s).b;
        (*s).a.u = 1.0f32;
        (*s).div = 1.0f32;
        (*s).count = 1 as ::core::ffi::c_int;
    } else if uBC <= 0 as ::core::ffi::c_int as ::core::ffi::c_float
        && vCA <= 0 as ::core::ffi::c_int as ::core::ffi::c_float
    {
        (*s).a = (*s).c;
        (*s).a.u = 1.0f32;
        (*s).div = 1.0f32;
        (*s).count = 1 as ::core::ffi::c_int;
    } else if uAB > 0 as ::core::ffi::c_int as ::core::ffi::c_float
        && vAB > 0 as ::core::ffi::c_int as ::core::ffi::c_float
        && wABC <= 0 as ::core::ffi::c_int as ::core::ffi::c_float
    {
        (*s).a.u = uAB;
        (*s).b.u = vAB;
        (*s).div = uAB + vAB;
        (*s).count = 2 as ::core::ffi::c_int;
    } else if uBC > 0 as ::core::ffi::c_int as ::core::ffi::c_float
        && vBC > 0 as ::core::ffi::c_int as ::core::ffi::c_float
        && uABC <= 0 as ::core::ffi::c_int as ::core::ffi::c_float
    {
        (*s).a = (*s).b;
        (*s).b = (*s).c;
        (*s).a.u = uBC;
        (*s).b.u = vBC;
        (*s).div = uBC + vBC;
        (*s).count = 2 as ::core::ffi::c_int;
    } else if uCA > 0 as ::core::ffi::c_int as ::core::ffi::c_float
        && vCA > 0 as ::core::ffi::c_int as ::core::ffi::c_float
        && vABC <= 0 as ::core::ffi::c_int as ::core::ffi::c_float
    {
        (*s).b = (*s).a;
        (*s).a = (*s).c;
        (*s).a.u = uCA;
        (*s).b.u = vCA;
        (*s).div = uCA + vCA;
        (*s).count = 2 as ::core::ffi::c_int;
    } else {
        (*s).a.u = uABC;
        (*s).b.u = vABC;
        (*s).c.u = wABC;
        (*s).div = uABC + vABC + wABC;
        (*s).count = 3 as ::core::ffi::c_int;
    };
}
#[no_mangle]
pub unsafe extern "C" fn c2Neg(mut a: c2v) -> c2v {
    return c2V(-a.x, -a.y);
}
#[no_mangle]
pub unsafe extern "C" fn c2Skew(mut a: c2v) -> c2v {
    let mut b: c2v = c2v { x: 0., y: 0. };
    b.x = -a.y;
    b.y = a.x;
    return b;
}
#[no_mangle]
pub unsafe extern "C" fn c2CCW90(mut a: c2v) -> c2v {
    let mut b: c2v = c2v { x: 0., y: 0. };
    b.x = a.y;
    b.y = -a.x;
    return b;
}
#[no_mangle]
pub unsafe extern "C" fn c2D(mut s: *mut c2Simplex) -> c2v {
    match (*s).count {
        1 => return c2Neg((*s).a.p),
        2 => {
            let mut ab: c2v = c2Sub((*s).b.p, (*s).a.p);
            if c2Det2(ab, c2Neg((*s).a.p)) > 0 as ::core::ffi::c_int as ::core::ffi::c_float {
                return c2Skew(ab);
            }
            return c2CCW90(ab);
        }
        3 | _ => {
            return c2V(0.0f32, 0.0f32);
        }
    };
}
#[no_mangle]
pub unsafe extern "C" fn c2Support(
    mut verts: *const c2v,
    mut count: ::core::ffi::c_int,
    mut d: c2v,
) -> ::core::ffi::c_int {
    let mut imax: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut dmax: f32 = c2Dot(*verts.offset(0 as ::core::ffi::c_int as isize), d);
    let mut i: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    while i < count {
        let mut dot: ::core::ffi::c_float = c2Dot(*verts.offset(i as isize), d);
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
    let mut den: ::core::ffi::c_float = 1.0f32 / (*s).div;
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
            *a = c2Add(
    c2Mulvs((*s).a.sA, den * (*s).a.u),
    c2Mulvs((*s).b.sA, den * (*s).b.u),
);
            *b = c2Add(
    c2Mulvs((*s).a.sB, den * (*s).a.u),
    c2Mulvs((*s).b.sB, den * (*s).b.u),
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
            *a = c2Add(
    c2Add(
        c2Mulvs((*s).a.sA, den * (*s).a.u),
        c2Mulvs((*s).b.sA, den * (*s).b.u),
    ),
    c2Mulvs((*s).c.sA, den * (*s).c.u),
);
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
            *b = c2Add(
    c2Add(
        c2Mulvs((*s).a.sB, den * (*s).a.u),
        c2Mulvs((*s).b.sB, den * (*s).b.u),
    ),
    c2Mulvs((*s).c.sB, den * (*s).c.u),
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
            *a = c2V(0.0f32, 0.0f32);
            *b = c2V(0.0f32, 0.0f32);
        }
    };
}
#[no_mangle]
pub unsafe extern "C" fn c2Div(mut a: c2v, mut b: ::core::ffi::c_float) -> c2v {
    return c2Mulvs(a, 1.0f32 / b);
}
#[no_mangle]
pub unsafe extern "C" fn c2Norm(mut a: c2v) -> c2v {
    return c2Div(a, c2Len(a));
}
#[no_mangle]
pub unsafe extern "C" fn c2L(mut s: *mut c2Simplex) -> c2v {
    let mut den: ::core::ffi::c_float = 1.0f32 / (*s).div;
    match (*s).count {
        1 => return (*s).a.p,
        2 => {
            return c2Add(
    c2Mulvs((*s).a.p, den * (*s).a.u),
    c2Mulvs((*s).b.p, den * (*s).b.u),
);
            return c2Add(
    c2Mulvs((*s).a.p, den * (*s).a.u),
    c2Mulvs((*s).b.p, den * (*s).b.u),
);
        }
        _ => {
            return c2V(0.0f32, 0.0f32);
        }
    };
}
#[no_mangle]
pub unsafe extern "C" fn c2MulrvT(mut a: c2r, mut b: c2v) -> c2v {
    return c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y);
}
#[no_mangle]
pub unsafe extern "C" fn c2GJK(
    mut A: *const ::core::ffi::c_void,
    mut typeA: C2_TYPE,
    mut ax_ptr: *const c2x,
    mut B: *const ::core::ffi::c_void,
    mut typeB: C2_TYPE,
    mut bx_ptr: *const c2x,
    mut outA: *mut c2v,
    mut outB: *mut c2v,
    mut use_radius: ::core::ffi::c_int,
    mut iterations: *mut ::core::ffi::c_int,
    mut cache: *mut c2GJKCache,
) -> ::core::ffi::c_float {
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
    let mut cache_was_read: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if !cache.is_null() {
        let mut cache_was_good: ::core::ffi::c_int = ((*cache).count != 0) as ::core::ffi::c_int;
        if cache_was_good != 0 {
            let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            while i < (*cache).count {
                let mut iA: ::core::ffi::c_int = (*cache).iA[i as usize];
                let mut iB: ::core::ffi::c_int = (*cache).iB[i as usize];
                let mut sA: c2v = c2Mulxv(ax, pA.verts[iA as usize]);
                let mut sB: c2v = c2Mulxv(bx, pB.verts[iB as usize]);
                let mut v: *mut c2sv = verts.offset(i as isize);
                (*v).iA = iA;
                (*v).sA = sA;
                (*v).iB = iB;
                (*v).sB = sB;
                (*v).p = c2Sub((*v).sB, (*v).sA);
                (*v).u = 0 as ::core::ffi::c_int as ::core::ffi::c_float;
                i += 1;
            }
            s.count = (*cache).count;
            s.div = (*cache).div;
            let mut metric_old: ::core::ffi::c_float = (*cache).metric;
            let mut metric: ::core::ffi::c_float = c2GJKSimplexMetric(&raw mut s);
            let mut min_metric: ::core::ffi::c_float = if metric < metric_old {
                metric
            } else {
                metric_old
            };
            let mut max_metric: ::core::ffi::c_float = if metric > metric_old {
                metric
            } else {
                metric_old
            };
            if !(min_metric < max_metric * 2.0f32 && metric < -1.0e8f32) {
                cache_was_read = 1 as ::core::ffi::c_int;
            }
        }
    }
    if cache_was_read == 0 {
        s.a.iA = 0 as ::core::ffi::c_int;
        s.a.iB = 0 as ::core::ffi::c_int;
        s.a.sA = c2Mulxv(ax, pA.verts[0 as ::core::ffi::c_int as usize]);
        s.a.sB = c2Mulxv(bx, pB.verts[0 as ::core::ffi::c_int as usize]);
        s.a.p = c2Sub(s.a.sB, s.a.sA);
        s.a.u = 1.0f32;
        s.div = 1.0f32;
        s.count = 1 as ::core::ffi::c_int;
    }
    let mut saveA: [::core::ffi::c_int; 3] = [0; 3];
    let mut saveB: [::core::ffi::c_int; 3] = [0; 3];
    let mut save_count: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut d0: ::core::ffi::c_float = 3.40282346638528859811704183484516925e+38f32;
    let mut d1: ::core::ffi::c_float = 3.40282346638528859811704183484516925e+38f32;
    let mut iter: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut hit: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while iter < 20 as ::core::ffi::c_int {
        save_count = s.count;
        let mut i_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
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
        if s.count == 3 as ::core::ffi::c_int {
            hit = 1 as ::core::ffi::c_int;
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
            let mut iA_0: ::core::ffi::c_int = c2Support(
                &raw mut pA.verts as *mut c2v,
                pA.count,
                c2MulrvT(ax.r, c2Neg(d)),
            );
            let mut sA_0: c2v = c2Mulxv(ax, pA.verts[iA_0 as usize]);
            let mut iB_0: ::core::ffi::c_int =
                c2Support(&raw mut pB.verts as *mut c2v, pB.count, c2MulrvT(bx.r, d));
            let mut sB_0: c2v = c2Mulxv(bx, pB.verts[iB_0 as usize]);
            let mut v_0: *mut c2sv = verts.offset(s.count as isize);
            (*v_0).iA = iA_0;
            (*v_0).sA = sA_0;
            (*v_0).iB = iB_0;
            (*v_0).sB = sB_0;
            (*v_0).p = c2Sub((*v_0).sB, (*v_0).sA);
            let mut dup: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            let mut i_1: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            while i_1 < save_count {
                if iA_0 == saveA[i_1 as usize] && iB_0 == saveB[i_1 as usize] {
                    dup = 1 as ::core::ffi::c_int;
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
    let mut dist: ::core::ffi::c_float = c2Len(c2Sub(a, b));
    if hit != 0 {
        a = b;
        dist = 0 as ::core::ffi::c_int as ::core::ffi::c_float;
    } else if use_radius != 0 {
        let mut rA: ::core::ffi::c_float = pA.radius;
        let mut rB: ::core::ffi::c_float = pB.radius;
        if dist > rA + rB && dist > 1.19209289550781250000000000000000000e-7f32 {
            dist -= rA + rB;
            let mut n: c2v = c2Norm(c2Sub(b, a));
            a = c2Add(a, c2Mulvs(n, rA));
            b = c2Sub(b, c2Mulvs(n, rB));
            if a.x == b.x && a.y == b.y {
                dist = 0 as ::core::ffi::c_int as ::core::ffi::c_float;
            }
        } else {
            let mut p_0: c2v = c2Mulvs(c2Add(a, b), 0.5f32);
            a = p_0;
            b = p_0;
            dist = 0 as ::core::ffi::c_int as ::core::ffi::c_float;
        }
    }
    if !cache.is_null() {
        (*cache).metric = c2GJKSimplexMetric(&raw mut s);
        (*cache).count = s.count;
        let mut i_2: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
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
pub unsafe extern "C" fn c2AABBtoAABB(mut A: c2AABB, mut B: c2AABB) -> ::core::ffi::c_int {
    let mut d0: ::core::ffi::c_int = (B.max.x < A.min.x) as ::core::ffi::c_int;
    let mut d1: ::core::ffi::c_int = (A.max.x < B.min.x) as ::core::ffi::c_int;
    let mut d2: ::core::ffi::c_int = (B.max.y < A.min.y) as ::core::ffi::c_int;
    let mut d3: ::core::ffi::c_int = (A.max.y < B.min.y) as ::core::ffi::c_int;
    return (d0 | d1 | d2 | d3 == 0) as ::core::ffi::c_int;
}
#[no_mangle]
pub fn c2CircletoAABB(a: c2Circle, b: c2AABB) -> bool {
    let l = c2Clampv(a.p, b.min, b.max);
    let ab = c2Sub(a.p, l);
    let d2 = c2Dot(ab, ab);
    let r2 = a.r * a.r;
    d2 < r2
}
       C2_TYPE_CAPSULE,
        ::core::ptr::null::<c2x>(),
        ::core::ptr::null_mut::<c2v>(),
        ::core::ptr::null_mut::<c2v>(),
        1 as ::core::ffi::c_int,
        ::core::ptr::null_mut::<::core::ffi::c_int>(),
        ::core::ptr::null_mut::<c2GJKCache>(),
    ) != 0.
    {
        return 0 as ::core::ffi::c_int;
    }
    return 1 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn c2CapsuletoCapsule(
    mut A: c2Capsule,
    mut B: c2Capsule,
) -> ::core::ffi::c_int {
    if c2GJK(
        &raw mut A as *const ::core::ffi::c_void,
        C2_TYPE_CAPSULE,
        ::core::ptr::null::<c2x>(),
        &raw mut B as *const ::core::ffi::c_void,
        C2_TYPE_CAPSULE,
        ::core::ptr::null::<c2x>(),
        ::core::ptr::null_mut::<c2v>(),
        ::core::ptr::null_mut::<c2v>(),
        1 as ::core::ffi::c_int,
        ::core::ptr::null_mut::<::core::ffi::c_int>(),
        ::core::ptr::null_mut::<c2GJKCache>(),
    ) != 0.
    {
        return 0 as ::core::ffi::c_int;
    }
    return 1 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn c2CircletoCircle(mut A: c2Circle, mut B: c2Circle) -> ::core::ffi::c_int {
    let mut c: c2v = c2Sub(B.p, A.p);
    let mut d2: f32 = c2Dot(c, c);
    let mut r2: ::core::ffi::c_float = A.r + B.r;
    r2 = r2 * r2;
    return (d2 < r2) as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn c2CircletoAABB(mut A: c2Circle, mut B: c2AABB) -> ::core::ffi::c_int {
    let mut L: c2v = c2Clampv(A.p, B.min, B.max);
    let mut ab: c2v = c2Sub(A.p, L);
    let mut d2: f32 = c2Dot(ab, ab);
    let mut r2: ::core::ffi::c_float = A.r * A.r;
    return (d2 < r2) as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn c2CircletoCapsule(
    mut A: c2Circle,
    mut B: c2Capsule,
) -> ::core::ffi::c_int {
    let mut n: c2v = c2Sub(B.b, B.a);
    let mut ap: c2v = c2Sub(A.p, B.a);
    let mut da: f32 = c2Dot(ap, n);
    let mut d2: ::core::ffi::c_float = 0.;
    if da < 0 as ::core::ffi::c_int as ::core::ffi::c_float {
        d2 = c2Dot(ap, ap);
    } else {
        let mut db: f32 = c2Dot(c2Sub(A.p, B.b), n);
        if db < 0 as ::core::ffi::c_int as ::core::ffi::c_float {
            let mut e: c2v = c2Sub(ap, c2Mulvs(n, da / c2Dot(n, n)));
            d2 = c2Dot(e, e);
        } else {
            let mut bp: c2v = c2Sub(A.p, B.b);
            d2 = c2Dot(bp, bp);
        }
    }
    let mut r: ::core::ffi::c_float = A.r + B.r;
    return (d2 < r * r) as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn c2Collided(
    mut A: *const ::core::ffi::c_void,
    mut typeA: C2_TYPE,
    mut B: *const ::core::ffi::c_void,
    mut typeB: C2_TYPE,
) -> ::core::ffi::c_int {
    match typeA as ::core::ffi::c_uint {
    0 => match typeB as ::core::ffi::c_uint {
        0 => {
            return c2CircletoCircle(*(A as *mut c2Circle), *(B as *mut c2Circle));
        }
        1 => {
            let a = *(A as *mut c2Circle);
            let b = *(B as *mut c2AABB);
            return if c2CircletoAABB(a, b) { 1 } else { 0 };
        }
        2 => {
            return c2CircletoCapsule(*(A as *mut c2Circle), *(B as *mut c2Capsule));
        }
        _ => return 0 as ::core::ffi::c_int,
    },
    1 => match typeB as ::core::ffi::c_uint {
        0 => {
            let a = *(B as *mut c2Circle);
            let b = *(A as *mut c2AABB);
            return if c2CircletoAABB(a, b) { 1 } else { 0 };
        }
        1 => return c2AABBtoAABB(*(A as *mut c2AABB), *(B as *mut c2AABB)),
        2 => return c2AABBtoCapsule(*(A as *mut c2AABB), *(B as *mut c2Capsule)),
        _ => return 0 as ::core::ffi::c_int,
    },
    2 => match typeB as ::core::ffi::c_uint {
        0 => {
            return c2CircletoCapsule(*(B as *mut c2Circle), *(A as *mut c2Capsule));
        }
        1 => return c2AABBtoCapsule(*(B as *mut c2AABB), *(A as *mut c2Capsule)),
        2 => {
            return c2CapsuletoCapsule(*(A as *mut c2Capsule), *(B as *mut c2Capsule));
        }
        _ => return 0 as ::core::ffi::c_int,
    },
    _ => return 0 as ::core::ffi::c_int,
};
    match typeA as ::core::ffi::c_uint {
    0 => match typeB as ::core::ffi::c_uint {
        0 => {
            return c2CircletoCircle(*(A as *mut c2Circle), *(B as *mut c2Circle));
        }
        1 => {
            let a = *(A as *mut c2Circle);
            let b = *(B as *mut c2AABB);
            return if c2CircletoAABB(a, b) { 1 } else { 0 };
        }
        2 => {
            return c2CircletoCapsule(*(A as *mut c2Circle), *(B as *mut c2Capsule));
        }
        _ => return 0 as ::core::ffi::c_int,
    },
    1 => match typeB as ::core::ffi::c_uint {
        0 => {
            let a = *(B as *mut c2Circle);
            let b = *(A as *mut c2AABB);
            return if c2CircletoAABB(a, b) { 1 } else { 0 };
        }
        1 => return c2AABBtoAABB(*(A as *mut c2AABB), *(B as *mut c2AABB)),
        2 => return c2AABBtoCapsule(*(A as *mut c2AABB), *(B as *mut c2Capsule)),
        _ => return 0 as ::core::ffi::c_int,
    },
    2 => match typeB as ::core::ffi::c_uint {
        0 => {
            return c2CircletoCapsule(*(B as *mut c2Circle), *(A as *mut c2Capsule));
        }
        1 => return c2AABBtoCapsule(*(B as *mut c2AABB), *(A as *mut c2Capsule)),
        2 => {
            return c2CapsuletoCapsule(*(A as *mut c2Capsule), *(B as *mut c2Capsule));
        }
        _ => return 0 as ::core::ffi::c_int,
    },
    _ => return 0 as ::core::ffi::c_int,
};
}
#[no_mangle]
pub unsafe extern "C" fn capsule(
    mut min_x: ::core::ffi::c_float,
    mut min_y: ::core::ffi::c_float,
    mut max_x: ::core::ffi::c_float,
    mut max_y: ::core::ffi::c_float,
    mut r: ::core::ffi::c_float,
) -> ::core::ffi::c_int {
    let mut result: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut capsule_in: c2Capsule = c2Capsule {
        a: c2v { x: 0., y: 0. },
        b: c2v { x: 0., y: 0. },
        r: 0.,
    };
    capsule_in.a = c2V(min_x, min_y);
    capsule_in.b = c2V(max_x, max_y);
    capsule_in.r = r;
    let mut circle: c2Circle = c2Circle {
        p: c2v { x: 0., y: 0. },
        r: 0.,
    };
    circle.p = c2V(-70.0f32, 0.0f32);
    circle.r = 20.0f32;
    let mut aabb: c2AABB = c2AABB {
        min: c2v { x: 0., y: 0. },
        max: c2v { x: 0., y: 0. },
    };
    aabb.min = c2V(-40.0f32, -40.0f32);
    aabb.max = c2V(-15.0f32, -15.0f32);
    let mut capsule_0: c2Capsule = c2Capsule {
        a: c2v { x: 0., y: 0. },
        b: c2v { x: 0., y: 0. },
        r: 0.,
    };
    capsule_0.a = c2V(-40.0f32, 40.0f32);
    capsule_0.b = c2V(-20.0f32, 100.0f32);
    capsule_0.r = 10.0f32;
    result += c2Collided(
        &raw mut circle as *const ::core::ffi::c_void,
        C2_TYPE_CIRCLE,
        &raw mut capsule_in as *const ::core::ffi::c_void,
        C2_TYPE_CAPSULE,
    );
    result += c2Collided(
        &raw mut aabb as *const ::core::ffi::c_void,
        C2_TYPE_AABB,
        &raw mut capsule_in as *const ::core::ffi::c_void,
        C2_TYPE_CAPSULE,
    ) << 1 as ::core::ffi::c_int;
    result += c2Collided(
        &raw mut capsule_0 as *const ::core::ffi::c_void,
        C2_TYPE_CAPSULE,
        &raw mut capsule_in as *const ::core::ffi::c_void,
        C2_TYPE_CAPSULE,
    ) << 2 as ::core::ffi::c_int;
    return result;
}
