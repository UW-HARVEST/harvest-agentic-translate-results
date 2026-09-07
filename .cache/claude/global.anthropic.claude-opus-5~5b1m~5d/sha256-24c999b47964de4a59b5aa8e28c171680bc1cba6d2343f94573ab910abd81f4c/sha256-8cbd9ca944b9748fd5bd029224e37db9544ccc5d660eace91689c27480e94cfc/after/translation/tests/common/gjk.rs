//! Shared `c2GJK` driver used by the Phase B / Phase C GJK test files.

#![allow(non_snake_case, dead_code)]

use super::*;
use std::ffi::{c_int, c_uint, c_void};

/// One of the three shapes `c2MakeProxy` understands, plus a raw escape hatch
/// for out-of-range enum values.
#[derive(Copy, Clone, Debug)]
pub enum Shape {
    Circle(c2Circle),
    Aabb(c2AABB),
    Capsule(c2Capsule),
    /// Arbitrary 32-byte blob paired with an arbitrary `C2_TYPE` value.
    Raw([u32; 8], c_uint),
}

impl Shape {
    pub fn ty(&self) -> c_uint {
        match self {
            Shape::Circle(_) => C2_TYPE_CIRCLE,
            Shape::Aabb(_) => C2_TYPE_AABB,
            Shape::Capsule(_) => C2_TYPE_CAPSULE,
            Shape::Raw(_, t) => *t,
        }
    }
    pub fn ptr(&self) -> *const c_void {
        match self {
            Shape::Circle(c) => c as *const c2Circle as *const c_void,
            Shape::Aabb(c) => c as *const c2AABB as *const c_void,
            Shape::Capsule(c) => c as *const c2Capsule as *const c_void,
            Shape::Raw(b, _) => b.as_ptr() as *const c_void,
        }
    }
}

/// Everything `c2GJK` can be configured with.
#[derive(Copy, Clone, Debug)]
pub struct Cfg {
    pub a: Shape,
    pub b: Shape,
    pub ax: Option<c2x>,
    pub bx: Option<c2x>,
    pub use_radius: c_int,
    pub out_a: bool,
    pub out_b: bool,
    pub iters: bool,
    /// `None` -> pass a NULL cache pointer.
    pub cache: Option<c2GJKCache>,
}

impl Cfg {
    pub fn new(a: Shape, b: Shape) -> Cfg {
        Cfg {
            a,
            b,
            ax: None,
            bx: None,
            use_radius: 1,
            out_a: true,
            out_b: true,
            iters: true,
            cache: None,
        }
    }
    pub fn radius(mut self, u: c_int) -> Cfg {
        self.use_radius = u;
        self
    }
    pub fn xf(mut self, ax: Option<c2x>, bx: Option<c2x>) -> Cfg {
        self.ax = ax;
        self.bx = bx;
        self
    }
    pub fn outs(mut self, a: bool, b: bool, it: bool) -> Cfg {
        self.out_a = a;
        self.out_b = b;
        self.iters = it;
        self
    }
    pub fn cache(mut self, c: Option<c2GJKCache>) -> Cfg {
        self.cache = c;
        self
    }
}

/// Observable result of one `c2GJK` call, in comparable bit form.
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Obs {
    pub dist: u32,
    pub a: Option<(u32, u32)>,
    pub b: Option<(u32, u32)>,
    pub iters: Option<c_int>,
    pub cache: Option<Vec<u32>>,
    /// Poison sentinels for the out-params the call was told to skip: they must
    /// come back untouched.
    pub untouched: Vec<u32>,
}

const POISON_A: c2v = c2v { x: -12345.678, y: 98765.43 };
const POISON_B: c2v = c2v { x: 24680.135, y: -13579.246 };
const POISON_IT: c_int = -424242;

/// Invoke one library's `c2GJK` and capture everything observable.
pub fn call_full(f: FnGJK, cfg: &Cfg) -> (Obs, Option<c2GJKCache>) {
    let mut oa = POISON_A;
    let mut ob = POISON_B;
    let mut it: c_int = POISON_IT;
    let mut ca = cfg.cache;

    let ax_ptr = cfg.ax.as_ref().map(|x| x as *const c2x).unwrap_or(std::ptr::null());
    let bx_ptr = cfg.bx.as_ref().map(|x| x as *const c2x).unwrap_or(std::ptr::null());
    let oa_ptr = if cfg.out_a { &mut oa as *mut c2v } else { std::ptr::null_mut() };
    let ob_ptr = if cfg.out_b { &mut ob as *mut c2v } else { std::ptr::null_mut() };
    let it_ptr = if cfg.iters { &mut it as *mut c_int } else { std::ptr::null_mut() };
    let ca_ptr = ca.as_mut().map(|c| c as *mut c2GJKCache).unwrap_or(std::ptr::null_mut());

    let dist = unsafe {
        f(
            cfg.a.ptr(),
            cfg.a.ty(),
            ax_ptr,
            cfg.b.ptr(),
            cfg.b.ty(),
            bx_ptr,
            oa_ptr,
            ob_ptr,
            cfg.use_radius,
            it_ptr,
            ca_ptr,
        )
    };

    let mut untouched = Vec::new();
    if !cfg.out_a {
        untouched.extend_from_slice(&[oa.x.to_bits(), oa.y.to_bits()]);
    }
    if !cfg.out_b {
        untouched.extend_from_slice(&[ob.x.to_bits(), ob.y.to_bits()]);
    }
    if !cfg.iters {
        untouched.push(it as u32);
    }

    (
        Obs {
            dist: dist.to_bits(),
            a: if cfg.out_a { Some(vb(oa)) } else { None },
            b: if cfg.out_b { Some(vb(ob)) } else { None },
            iters: if cfg.iters { Some(it) } else { None },
            cache: ca.as_ref().map(cache_bits),
            untouched,
        },
        ca,
    )
}

/// Convenience wrapper when only the observable result is wanted.
pub fn call(f: FnGJK, cfg: &Cfg) -> Obs {
    call_full(f, cfg).0
}

/// Run a configuration through BOTH libraries and assert bit-identical
/// results.  Returns the (identical) cache state so caller chains can proceed.
pub fn check(ctx: &str, cfg: &Cfg) -> Option<c2GJKCache> {
    let (c, r) = apis();
    let (oc, cache_c) = call_full(c.c2GJK, cfg);
    let (or, _) = call_full(r.c2GJK, cfg);
    if oc != or {
        panic!("DIVERGENCE [{ctx}]\n  cfg  = {cfg:?}\n  C    = {oc:?}\n  RUST = {or:?}");
    }
    cache_c
}

/// Run a configuration through both libraries WITHOUT asserting equality.
/// Used only for the ERRORS.md rows where the C reads uninitialised or
/// out-of-bounds memory, so it has no reproducible observable behaviour; the
/// only property being checked is that neither library crashes.
pub fn run_both_no_assert(cfg: &Cfg) -> (Obs, Obs) {
    let (c, r) = apis();
    (call(c.c2GJK, cfg), call(r.c2GJK, cfg))
}

/// Number of proxy vertices `c2MakeProxy` initialises for each shape type.
/// Cached indices at or beyond this are reads of uninitialised stack in the C
/// (ERRORS.md rows 14/15), so tests that need defined behaviour bring the
/// cached indices back into range with this.
pub fn vert_count(ty: c_uint) -> c_int {
    match ty {
        C2_TYPE_CIRCLE => 1,
        C2_TYPE_AABB => 4,
        _ => 2,
    }
}

/// Fold every cached vertex index into `[0, count)` for the given shape types.
pub fn clamp_cache(mut ca: c2GJKCache, ta: c_uint, tb: c_uint) -> c2GJKCache {
    let (na, nb) = (vert_count(ta), vert_count(tb));
    for i in 0..3 {
        ca.iA[i] = ca.iA[i].rem_euclid(na);
        ca.iB[i] = ca.iB[i].rem_euclid(nb);
    }
    ca
}

// ---------------------------------------------------------------------------
// Shape generators
// ---------------------------------------------------------------------------

pub const TYPES: [u32; 3] = [0, 1, 2];

/// Build a shape of the given type near `centre`, with extent/radius `scale`.
pub fn shape_at(rng: &mut Rng, ty: u32, centre: c2v, scale: f32) -> Shape {
    match ty {
        0 => Shape::Circle(c2Circle {
            p: centre,
            r: scale * (0.2 + rng.unit()),
        }),
        1 => {
            let hx = scale * (0.2 + rng.unit());
            let hy = scale * (0.2 + rng.unit());
            Shape::Aabb(c2AABB {
                min: c2v { x: centre.x - hx, y: centre.y - hy },
                max: c2v { x: centre.x + hx, y: centre.y + hy },
            })
        }
        _ => {
            let d = c2v { x: rng.sym(scale), y: rng.sym(scale) };
            Shape::Capsule(c2Capsule {
                a: c2v { x: centre.x - d.x, y: centre.y - d.y },
                b: c2v { x: centre.x + d.x, y: centre.y + d.y },
                r: scale * (0.2 + rng.unit()),
            })
        }
    }
}

/// `shape_at` with a randomly drawn centre (avoids double mutable borrows).
pub fn shape_rand(rng: &mut Rng, ty: u32, centre_mag: f32, scale: f32) -> Shape {
    let centre = rng.v(centre_mag);
    shape_at(rng, ty, centre, scale)
}

/// A shape built from wholly unconstrained floats.
pub fn shape_bits(rng: &mut Rng, ty: u32) -> Shape {
    match ty {
        0 => Shape::Circle(c2Circle { p: rng.v_bits(), r: rng.any_bits() }),
        1 => Shape::Aabb(c2AABB { min: rng.v_bits(), max: rng.v_bits() }),
        _ => Shape::Capsule(c2Capsule { a: rng.v_bits(), b: rng.v_bits(), r: rng.any_bits() }),
    }
}

/// A shape built from the "spicy" generator (mostly finite, some specials).
pub fn shape_spicy(rng: &mut Rng, ty: u32, mag: f32) -> Shape {
    match ty {
        0 => Shape::Circle(c2Circle { p: rng.v_spicy(mag), r: rng.spicy(mag) }),
        1 => Shape::Aabb(c2AABB { min: rng.v_spicy(mag), max: rng.v_spicy(mag) }),
        _ => Shape::Capsule(c2Capsule {
            a: rng.v_spicy(mag),
            b: rng.v_spicy(mag),
            r: rng.spicy(mag),
        }),
    }
}

pub fn ty_name(t: u32) -> &'static str {
    match t {
        0 => "circle",
        1 => "aabb",
        2 => "capsule",
        _ => "raw",
    }
}

/// Random `c2x` transforms covering every branch of `c2Mulrv`/`c2MulrvT`.
pub fn xforms(rng: &mut Rng) -> Vec<(&'static str, Option<c2x>)> {
    vec![
        ("null", None),
        (
            "identity",
            Some(c2x { p: c2v { x: 0.0, y: 0.0 }, r: c2r { c: 1.0, s: 0.0 } }),
        ),
        (
            "translate",
            Some(c2x { p: rng.v(50.0), r: c2r { c: 1.0, s: 0.0 } }),
        ),
        ("rotate", Some(c2x { p: c2v { x: 0.0, y: 0.0 }, r: rng.rot() })),
        ("rot+trans", Some(c2x { p: rng.v(50.0), r: rng.rot() })),
        (
            "nonunit",
            Some(c2x { p: rng.v(50.0), r: c2r { c: rng.sym(3.0), s: rng.sym(3.0) } }),
        ),
    ]
}
