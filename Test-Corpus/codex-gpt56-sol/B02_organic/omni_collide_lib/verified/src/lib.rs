#![no_std]
#![allow(non_camel_case_types, non_snake_case, unsafe_op_in_unsafe_fn)]

use core::ffi::{c_float, c_int, c_void};
use core::mem::MaybeUninit;
use core::ptr;

#[cfg(not(test))]
core::arch::global_asm!(
    ".hidden rust_eh_personality",
    ".globl rust_eh_personality",
    "rust_eh_personality:",
    "ret",
);

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    unsafe { abort() }
}

type C2_TYPE = c_int;

const C2_TYPE_CAPSULE: C2_TYPE = 0;
const C2_TYPE_CIRCLE: C2_TYPE = 1;
const C2_TYPE_AABB: C2_TYPE = 2;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2v {
    pub x: c_float,
    pub y: c_float,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2r {
    pub c: c_float,
    pub s: c_float,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2Circle {
    pub p: c2v,
    pub r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2GJKCache {
    pub metric: c_float,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: c_float,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2Proxy {
    pub radius: c_float,
    pub count: c_int,
    pub verts: [c2v; 8],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: c_float,
    pub iA: c_int,
    pub iB: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2Simplex {
    pub a: c2sv,
    pub b: c2sv,
    pub c: c2sv,
    pub d: c2sv,
    pub div: c_float,
    pub count: c_int,
}

#[link(name = "m")]
unsafe extern "C" {
    fn sqrtf(value: c_float) -> c_float;
}

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
}

#[cfg(not(test))]
unsafe extern "C" {
    fn abort() -> !;
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2V(x: c_float, y: c_float) -> c2v {
    c2v { x, y }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2Mulvs(a: c2v, b: c_float) -> c2v {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        let mut x = a.x;
        let mut y = a.y;
        unsafe {
            core::arch::asm!(
                "mulss {x}, {b}",
                "mulss {y}, {b}",
                x = inout(xmm_reg) x,
                y = inout(xmm_reg) y,
                b = in(xmm_reg) b,
                options(pure, nomem, nostack)
            );
        }
        return c2v { x, y };
    }

    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    c2v {
        x: a.x * b,
        y: a.y * b,
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2Clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2Maxv(lo, c2Minv(a, hi))
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2Sub(mut a: c2v, b: c2v) -> c2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> c_float {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        let x = a.x;
        let mut y = b.y;
        unsafe {
            core::arch::asm!(
                "mulss {x}, {bx}",
                "mulss {y}, {ay}",
                "addss {y}, {x}",
                x = inout(xmm_reg) x => _,
                y = inout(xmm_reg) y,
                ay = in(xmm_reg) a.y,
                bx = in(xmm_reg) b.x,
                options(pure, nomem, nostack)
            );
        }
        return y;
    }

    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    {
        (a.x * b.x) + (a.y * b.y)
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2RotIdentity() -> c2r {
    c2r { c: 1.0, s: 0.0 }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2xIdentity() -> c2x {
    c2x {
        p: c2V(0.0, 0.0),
        r: c2RotIdentity(),
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn c2BBVerts(out: *mut c2v, bb: *mut c2AABB) {
    let bb = &*bb;
    ptr::write(out, bb.min);
    ptr::write(out.add(1), c2V(bb.max.x, bb.min.y));
    ptr::write(out.add(2), bb.max);
    ptr::write(out.add(3), c2V(bb.min.x, bb.max.y));
}

#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn c2MakeProxy(shape: *const c_void, typ: C2_TYPE, p: *mut c2Proxy) {
    match typ {
        C2_TYPE_CIRCLE => {
            let c = &*(shape.cast::<c2Circle>());
            (*p).radius = c.r;
            (*p).count = 1;
            (*p).verts[0] = c.p;
        }
        C2_TYPE_AABB => {
            let bb = shape.cast::<c2AABB>();
            (*p).radius = 0.0;
            (*p).count = 4;
            c2BBVerts((*p).verts.as_mut_ptr(), bb.cast_mut());
        }
        C2_TYPE_CAPSULE => {
            let c = &*(shape.cast::<c2Capsule>());
            (*p).radius = c.r;
            (*p).count = 2;
            (*p).verts[0] = c.a;
            (*p).verts[1] = c.b;
        }
        _ => {}
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2Len(a: c2v) -> c_float {
    unsafe { sqrtf(c2Dot(a, a)) }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2Det2(a: c2v, b: c2v) -> c_float {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        let mut x = b.y;
        let y = b.x;
        unsafe {
            core::arch::asm!(
                "mulss {x}, {ax}",
                "mulss {y}, {ay}",
                "subss {x}, {y}",
                x = inout(xmm_reg) x,
                y = inout(xmm_reg) y => _,
                ax = in(xmm_reg) a.x,
                ay = in(xmm_reg) a.y,
                options(pure, nomem, nostack)
            );
        }
        return x;
    }

    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    {
        (a.x * b.y) - (a.y * b.x)
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn c2GJKSimplexMetric(s: *mut c2Simplex) -> c_float {
    match (*s).count {
        1 => 0.0,
        2 => c2Len(c2Sub((*s).b.p, (*s).a.p)),
        3 => c2Det2(c2Sub((*s).b.p, (*s).a.p), c2Sub((*s).c.p, (*s).a.p)),
        _ => 0.0,
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2Mulrv(a: c2r, b: c2v) -> c2v {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        // Match GCC's scalar SSE operand order. This is observable when both
        // operands produce NaNs with different payloads.
        let mut x = b.x;
        let xy = b.y;
        let mut y = a.s;
        let yx = b.y;
        unsafe {
            core::arch::asm!(
                "mulss {x}, {ac}",
                "mulss {xy}, {as_}",
                "subss {x}, {xy}",
                "mulss {y}, {bx}",
                "mulss {yx}, {ac}",
                "addss {y}, {yx}",
                x = inout(xmm_reg) x,
                xy = inout(xmm_reg) xy => _,
                y = inout(xmm_reg) y,
                yx = inout(xmm_reg) yx => _,
                ac = in(xmm_reg) a.c,
                as_ = in(xmm_reg) a.s,
                bx = in(xmm_reg) b.x,
                options(pure, nomem, nostack)
            );
        }
        return c2V(x, y);
    }

    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    {
        c2V(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2Add(a: c2v, b: c2v) -> c2v {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        let mut x = b.x;
        let mut y = b.y;
        unsafe {
            core::arch::asm!(
                "addss {x}, {ax}",
                "addss {y}, {ay}",
                x = inout(xmm_reg) x,
                y = inout(xmm_reg) y,
                ax = in(xmm_reg) a.x,
                ay = in(xmm_reg) a.y,
                options(pure, nomem, nostack)
            );
        }
        return c2v { x, y };
    }

    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    {
        let mut a = a;
        a.x += b.x;
        a.y += b.y;
        a
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2Mulxv(a: c2x, b: c2v) -> c2v {
    c2Add(c2Mulrv(a.r, b), a.p)
}

#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn c22(s: *mut c2Simplex) {
    let a = (*s).a.p;
    let b = (*s).b.p;
    let u = c2Dot(b, c2Sub(b, a));
    let v = c2Dot(a, c2Sub(a, b));
    if v <= 0.0 {
        (*s).a.u = 1.0;
        (*s).div = 1.0;
        (*s).count = 1;
    } else if u <= 0.0 {
        (*s).a = (*s).b;
        (*s).a.u = 1.0;
        (*s).div = 1.0;
        (*s).count = 1;
    } else {
        (*s).a.u = u;
        (*s).b.u = v;
        (*s).div = u + v;
        (*s).count = 2;
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn c23(s: *mut c2Simplex) {
    let a = (*s).a.p;
    let b = (*s).b.p;
    let c = (*s).c.p;
    let uAB = c2Dot(b, c2Sub(b, a));
    let vAB = c2Dot(a, c2Sub(a, b));
    let uBC = c2Dot(c, c2Sub(c, b));
    let vBC = c2Dot(b, c2Sub(b, c));
    let uCA = c2Dot(a, c2Sub(a, c));
    let vCA = c2Dot(c, c2Sub(c, a));
    let area = c2Det2(c2Sub(b, a), c2Sub(c, a));
    let uABC = c2Det2(b, c) * area;
    let vABC = c2Det2(c, a) * area;
    let wABC = c2Det2(a, b) * area;

    if vAB <= 0.0 && uCA <= 0.0 {
        (*s).a.u = 1.0;
        (*s).div = 1.0;
        (*s).count = 1;
    } else if uAB <= 0.0 && vBC <= 0.0 {
        (*s).a = (*s).b;
        (*s).a.u = 1.0;
        (*s).div = 1.0;
        (*s).count = 1;
    } else if uBC <= 0.0 && vCA <= 0.0 {
        (*s).a = (*s).c;
        (*s).a.u = 1.0;
        (*s).div = 1.0;
        (*s).count = 1;
    } else if uAB > 0.0 && vAB > 0.0 && wABC <= 0.0 {
        (*s).a.u = uAB;
        (*s).b.u = vAB;
        (*s).div = uAB + vAB;
        (*s).count = 2;
    } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {
        (*s).a = (*s).b;
        (*s).b = (*s).c;
        (*s).a.u = uBC;
        (*s).b.u = vBC;
        (*s).div = uBC + vBC;
        (*s).count = 2;
    } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {
        (*s).b = (*s).a;
        (*s).a = (*s).c;
        (*s).a.u = uCA;
        (*s).b.u = vCA;
        (*s).div = uCA + vCA;
        (*s).count = 2;
    } else {
        (*s).a.u = uABC;
        (*s).b.u = vABC;
        (*s).c.u = wABC;
        (*s).div = uABC + vABC + wABC;
        (*s).count = 3;
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2Neg(a: c2v) -> c2v {
    c2V(-a.x, -a.y)
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2Skew(a: c2v) -> c2v {
    c2v { x: -a.y, y: a.x }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2CCW90(a: c2v) -> c2v {
    c2v { x: a.y, y: -a.x }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn c2D(s: *mut c2Simplex) -> c2v {
    match (*s).count {
        1 => c2Neg((*s).a.p),
        2 => {
            let ab = c2Sub((*s).b.p, (*s).a.p);
            if c2Det2(ab, c2Neg((*s).a.p)) > 0.0 {
                c2Skew(ab)
            } else {
                c2CCW90(ab)
            }
        }
        _ => c2V(0.0, 0.0),
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn c2Support(verts: *const c2v, count: c_int, d: c2v) -> c_int {
    let mut imax = 0;
    let mut dmax = c2Dot(ptr::read_volatile(verts), d);
    let mut i = 1;
    while i < count {
        let dot = c2Dot(*verts.add(i as usize), d);
        if dot > dmax {
            imax = i;
            dmax = dot;
        }
        i += 1;
    }
    imax
}

#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn c2Witness(s: *mut c2Simplex, a: *mut c2v, b: *mut c2v) {
    let den = 1.0 / (*s).div;
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
            *a = c2V(0.0, 0.0);
            *b = c2V(0.0, 0.0);
        }
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2Div(a: c2v, b: c_float) -> c2v {
    c2Mulvs(a, 1.0 / b)
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn c2L(s: *mut c2Simplex) -> c2v {
    let den = 1.0 / (*s).div;
    match (*s).count {
        1 => (*s).a.p,
        2 => c2Add(
            c2Mulvs((*s).a.p, den * (*s).a.u),
            c2Mulvs((*s).b.p, den * (*s).b.u),
        ),
        _ => c2V(0.0, 0.0),
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        let mut x = a.c;
        let xy = b.y;
        let mut y = a.s;
        let sign = f32::from_bits(0x8000_0000);
        let yy = b.y;
        unsafe {
            core::arch::asm!(
                "mulss {x}, {bx}",
                "mulss {xy}, {as_}",
                "addss {x}, {xy}",
                "xorps {y}, {sign}",
                "mulss {y}, {bx}",
                "mulss {yy}, {ac}",
                "addss {y}, {yy}",
                x = inout(xmm_reg) x,
                xy = inout(xmm_reg) xy => _,
                y = inout(xmm_reg) y,
                yy = inout(xmm_reg) yy => _,
                sign = in(xmm_reg) sign,
                ac = in(xmm_reg) a.c,
                as_ = in(xmm_reg) a.s,
                bx = in(xmm_reg) b.x,
                options(pure, nomem, nostack)
            );
        }
        return c2V(x, y);
    }

    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    {
        c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn c2GJK(
    A: *const c_void,
    typeA: C2_TYPE,
    ax_ptr: *const c2x,
    B: *const c_void,
    typeB: C2_TYPE,
    bx_ptr: *const c2x,
    outA: *mut c2v,
    outB: *mut c2v,
    use_radius: c_int,
    iterations: *mut c_int,
    cache: *mut c2GJKCache,
) -> c_float {
    let ax = if ax_ptr.is_null() {
        c2xIdentity()
    } else {
        *ax_ptr
    };
    let bx = if bx_ptr.is_null() {
        c2xIdentity()
    } else {
        *bx_ptr
    };

    let mut pA = MaybeUninit::<c2Proxy>::uninit();
    let mut pB = MaybeUninit::<c2Proxy>::uninit();
    c2MakeProxy(A, typeA, pA.as_mut_ptr());
    c2MakeProxy(B, typeB, pB.as_mut_ptr());
    let pA = pA.assume_init();
    let pB = pB.assume_init();

    let mut s = MaybeUninit::<c2Simplex>::zeroed().assume_init();
    let verts = (&raw mut s).cast::<c2sv>();
    let mut cache_was_read = 0;
    if !cache.is_null() {
        let cache_was_good = ((*cache).count != 0) as c_int;
        if cache_was_good != 0 {
            let mut i = 0;
            while i < (*cache).count {
                let iA = (*cache).iA[i as usize];
                let iB = (*cache).iB[i as usize];
                let sA = c2Mulxv(ax, pA.verts[iA as usize]);
                let sB = c2Mulxv(bx, pB.verts[iB as usize]);
                let v = verts.add(i as usize);
                (*v).iA = iA;
                (*v).sA = sA;
                (*v).iB = iB;
                (*v).sB = sB;
                (*v).p = c2Sub((*v).sB, (*v).sA);
                (*v).u = 0.0;
                i += 1;
            }
            s.count = (*cache).count;
            s.div = (*cache).div;
            let metric_old = (*cache).metric;
            let metric = c2GJKSimplexMetric(&mut s);
            let min_metric = if metric < metric_old {
                metric
            } else {
                metric_old
            };
            let max_metric = if metric > metric_old {
                metric
            } else {
                metric_old
            };
            if !(min_metric < max_metric * 2.0 && metric < -1.0e8) {
                cache_was_read = 1;
            }
        }
    }

    if cache_was_read == 0 {
        s.a.iA = 0;
        s.a.iB = 0;
        s.a.sA = c2Mulxv(ax, pA.verts[0]);
        s.a.sB = c2Mulxv(bx, pB.verts[0]);
        s.a.p = c2Sub(s.a.sB, s.a.sA);
        s.a.u = 1.0;
        s.div = 1.0;
        s.count = 1;
    }

    let mut saveA = [0; 3];
    let mut saveB = [0; 3];
    let mut save_count: c_int;
    let mut d0 = c_float::MAX;
    let mut d1: c_float;
    let mut iter = 0;
    let mut hit = 0;

    while iter < 20 {
        save_count = s.count;
        let mut i = 0;
        while i < save_count {
            saveA[i as usize] = (*verts.add(i as usize)).iA;
            saveB[i as usize] = (*verts.add(i as usize)).iB;
            i += 1;
        }
        match s.count {
            1 => {}
            2 => c22(&mut s),
            3 => c23(&mut s),
            _ => {}
        }
        if s.count == 3 {
            hit = 1;
            break;
        }
        let p = c2L(&mut s);
        d1 = c2Dot(p, p);
        if d1 > d0 {
            break;
        }
        d0 = d1;
        let d = c2D(&mut s);
        if c2Dot(d, d) < c_float::EPSILON * c_float::EPSILON {
            break;
        }
        let iA = c2Support(pA.verts.as_ptr(), pA.count, c2MulrvT(ax.r, c2Neg(d)));
        let sA = c2Mulxv(ax, pA.verts[iA as usize]);
        let iB = c2Support(pB.verts.as_ptr(), pB.count, c2MulrvT(bx.r, d));
        let sB = c2Mulxv(bx, pB.verts[iB as usize]);
        let v = verts.add(s.count as usize);
        (*v).iA = iA;
        (*v).sA = sA;
        (*v).iB = iB;
        (*v).sB = sB;
        (*v).p = c2Sub((*v).sB, (*v).sA);
        let mut dup = 0;
        let mut i = 0;
        while i < save_count {
            if iA == saveA[i as usize] && iB == saveB[i as usize] {
                dup = 1;
                break;
            }
            i += 1;
        }
        if dup != 0 {
            break;
        }
        s.count += 1;
        iter += 1;
    }

    let mut a = MaybeUninit::<c2v>::uninit();
    let mut b = MaybeUninit::<c2v>::uninit();
    c2Witness(&mut s, a.as_mut_ptr(), b.as_mut_ptr());
    let mut a = a.assume_init();
    let mut b = b.assume_init();
    let mut dist = c2Len(c2Sub(a, b));
    if hit != 0 {
        a = b;
        dist = 0.0;
    } else if use_radius != 0 {
        let rA = pA.radius;
        let rB = pB.radius;
        if dist > rA + rB && dist > c_float::EPSILON {
            dist -= rA + rB;
            let n = c2Norm(c2Sub(b, a));
            a = c2Add(a, c2Mulvs(n, rA));
            b = c2Sub(b, c2Mulvs(n, rB));
            if a.x == b.x && a.y == b.y {
                dist = 0.0;
            }
        } else {
            let p = c2Mulvs(c2Add(a, b), 0.5);
            a = p;
            b = p;
            dist = 0.0;
        }
    }

    if !cache.is_null() {
        (*cache).metric = c2GJKSimplexMetric(&mut s);
        (*cache).count = s.count;
        let mut i = 0;
        while i < s.count {
            let v = verts.add(i as usize);
            (*cache).iA[i as usize] = (*v).iA;
            (*cache).iB[i as usize] = (*v).iB;
            i += 1;
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
    dist
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2AABBtoAABB(A: c2AABB, B: c2AABB) -> c_int {
    let d0 = (B.max.x < A.min.x) as c_int;
    let d1 = (A.max.x < B.min.x) as c_int;
    let d2 = (B.max.y < A.min.y) as c_int;
    let d3 = (A.max.y < B.min.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn c2AABBtoCapsule(A: c2AABB, B: c2Capsule) -> c_int {
    if c2GJK(
        ptr::from_ref(&A).cast(),
        C2_TYPE_AABB,
        ptr::null(),
        ptr::from_ref(&B).cast(),
        C2_TYPE_CAPSULE,
        ptr::null(),
        ptr::null_mut(),
        ptr::null_mut(),
        1,
        ptr::null_mut(),
        ptr::null_mut(),
    ) != 0.0
    {
        0
    } else {
        1
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn c2CapsuletoCapsule(A: c2Capsule, B: c2Capsule) -> c_int {
    if c2GJK(
        ptr::from_ref(&A).cast(),
        C2_TYPE_CAPSULE,
        ptr::null(),
        ptr::from_ref(&B).cast(),
        C2_TYPE_CAPSULE,
        ptr::null(),
        ptr::null_mut(),
        ptr::null_mut(),
        1,
        ptr::null_mut(),
        ptr::null_mut(),
    ) != 0.0
    {
        0
    } else {
        1
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2CircletoCircle(A: c2Circle, B: c2Circle) -> c_int {
    let c = c2Sub(B.p, A.p);
    let d2 = c2Dot(c, c);
    let mut r2 = A.r + B.r;
    r2 *= r2;
    (d2 < r2) as c_int
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2CircletoAABB(A: c2Circle, B: c2AABB) -> c_int {
    let L = c2Clampv(A.p, B.min, B.max);
    let ab = c2Sub(A.p, L);
    let d2 = c2Dot(ab, ab);
    let r2 = A.r * A.r;
    (d2 < r2) as c_int
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn c2CircletoCapsule(A: c2Circle, B: c2Capsule) -> c_int {
    let n = c2Sub(B.b, B.a);
    let ap = c2Sub(A.p, B.a);
    let da = c2Dot(ap, n);
    let d2;
    if da < 0.0 {
        d2 = c2Dot(ap, ap);
    } else {
        let db = c2Dot(c2Sub(A.p, B.b), n);
        if db < 0.0 {
            let e = c2Sub(ap, c2Mulvs(n, da / c2Dot(n, n)));
            d2 = c2Dot(e, e);
        } else {
            let bp = c2Sub(A.p, B.b);
            d2 = c2Dot(bp, bp);
        }
    }
    let r = A.r + B.r;
    (d2 < r * r) as c_int
}

#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn c2Collided(
    A: *const c_void,
    typeA: C2_TYPE,
    B: *const c_void,
    typeB: C2_TYPE,
) -> c_int {
    match typeA {
        C2_TYPE_CIRCLE => match typeB {
            C2_TYPE_CIRCLE => c2CircletoCircle(*A.cast(), *B.cast()),
            C2_TYPE_AABB => c2CircletoAABB(*A.cast(), *B.cast()),
            C2_TYPE_CAPSULE => c2CircletoCapsule(*A.cast(), *B.cast()),
            _ => 0,
        },
        C2_TYPE_AABB => match typeB {
            C2_TYPE_CIRCLE => c2CircletoAABB(*B.cast(), *A.cast()),
            C2_TYPE_AABB => c2AABBtoAABB(*A.cast(), *B.cast()),
            C2_TYPE_CAPSULE => c2AABBtoCapsule(*A.cast(), *B.cast()),
            _ => 0,
        },
        C2_TYPE_CAPSULE => match typeB {
            C2_TYPE_CIRCLE => c2CircletoCapsule(*B.cast(), *A.cast()),
            C2_TYPE_AABB => c2AABBtoCapsule(*B.cast(), *A.cast()),
            C2_TYPE_CAPSULE => c2CapsuletoCapsule(*A.cast(), *B.cast()),
            _ => 0,
        },
        _ => 0,
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn ptr_from_parts(
    typ: C2_TYPE,
    a: c_float,
    b: c_float,
    c: c_float,
    d: c_float,
    e: c_float,
) -> *mut c_void {
    match typ {
        C2_TYPE_CIRCLE => {
            let circle = malloc(size_of::<c2Circle>()).cast::<c2Circle>();
            (*circle).p = c2V(a, b);
            (*circle).r = c;
            circle.cast()
        }
        C2_TYPE_AABB => {
            let aabb = malloc(size_of::<c2AABB>()).cast::<c2AABB>();
            (*aabb).min = c2V(a, b);
            (*aabb).max = c2V(c, d);
            aabb.cast()
        }
        C2_TYPE_CAPSULE => {
            let capsule = malloc(size_of::<c2Capsule>()).cast::<c2Capsule>();
            (*capsule).a = c2V(a, b);
            (*capsule).b = c2V(c, d);
            (*capsule).r = e;
            capsule.cast()
        }
        _ => core::hint::unreachable_unchecked(),
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn omni_collide(
    type_a: C2_TYPE,
    a1: c_float,
    a2: c_float,
    a3: c_float,
    a4: c_float,
    a5: c_float,
    type_b: C2_TYPE,
    b1: c_float,
    b2: c_float,
    b3: c_float,
    b4: c_float,
    b5: c_float,
) -> c_int {
    let A = ptr_from_parts(type_a, a1, a2, a3, a4, a5);
    let B = ptr_from_parts(type_b, b1, b2, b3, b4, b5);
    c2Collided(A, type_a, B, type_b)
}
