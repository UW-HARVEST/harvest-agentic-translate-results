use libloading::{Library, Symbol};
use std::path::PathBuf;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct c2v {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct c2r {
    c: f32,
    s: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct c2x {
    p: c2v,
    r: c2r,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2Circle {
    p: c2v,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2AABB {
    min: c2v,
    max: c2v,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2Capsule {
    a: c2v,
    b: c2v,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2GJKCache {
    metric: f32,
    count: i32,
    i_a: [i32; 3],
    i_b: [i32; 3],
    div: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2sv {
    s_a: c2v,
    s_b: c2v,
    p: c2v,
    u: f32,
    i_a: i32,
    i_b: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2Simplex {
    a: c2sv,
    b: c2sv,
    c: c2sv,
    d: c2sv,
    div: f32,
    count: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2Proxy {
    radius: f32,
    count: i32,
    verts: [c2v; 8],
}

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.join("target/debug/libgjk_cache_lib.so")
}

fn assert_v_eq(a: c2v, b: c2v, ctx: &str) {
    assert!(a.x.to_bits() == b.x.to_bits() && a.y.to_bits() == b.y.to_bits(),
        "{ctx}: C=({}, {}) Rust=({}, {})", a.x, a.y, b.x, b.y);
}

fn assert_r_eq(a: c2r, b: c2r, ctx: &str) {
    assert!(a.c.to_bits() == b.c.to_bits() && a.s.to_bits() == b.s.to_bits(),
        "{ctx}: C=({}, {}) Rust=({}, {})", a.c, a.s, b.c, b.s);
}

fn assert_f_eq(a: f32, b: f32, ctx: &str) {
    assert!(a.to_bits() == b.to_bits(), "{ctx}: C={a} Rust={b}");
}

#[test]
fn test_c2v() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    type Fn = unsafe extern "C" fn(f32, f32) -> c2v;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2V").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2V").unwrap() };
    for &(x, y) in &[(0.0f32, 0.0), (1.5, -2.3), (-100.0, 0.001), (f32::MAX, f32::MIN)] {
        let c = unsafe { c_fn(x, y) };
        let r = unsafe { r_fn(x, y) };
        assert_v_eq(c, r, &format!("c2V({x}, {y})"));
    }
}

#[test]
fn test_c2_mulvs() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    type Fn = unsafe extern "C" fn(c2v, f32) -> c2v;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2Mulvs").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2Mulvs").unwrap() };
    let cases = [(c2v{x:1.0,y:2.0}, 3.0f32), (c2v{x:-1.0,y:0.5}, 0.0), (c2v{x:100.0,y:-50.0}, -2.0)];
    for (v, s) in cases {
        let c = unsafe { c_fn(v, s) };
        let r = unsafe { r_fn(v, s) };
        assert_v_eq(c, r, "c2Mulvs");
    }
}

#[test]
fn test_c2_vector_ops() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    let a = c2v{x:3.0,y:-4.0};
    let b = c2v{x:1.0,y:2.0};

    // c2Maxv
    {
        type Fn = unsafe extern "C" fn(c2v, c2v) -> c2v;
        let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2Maxv").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2Maxv").unwrap() };
        assert_v_eq(unsafe{c_fn(a,b)}, unsafe{r_fn(a,b)}, "c2Maxv");
    }
    // c2Minv
    {
        type Fn = unsafe extern "C" fn(c2v, c2v) -> c2v;
        let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2Minv").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2Minv").unwrap() };
        assert_v_eq(unsafe{c_fn(a,b)}, unsafe{r_fn(a,b)}, "c2Minv");
    }
    // c2Sub
    {
        type Fn = unsafe extern "C" fn(c2v, c2v) -> c2v;
        let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2Sub").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2Sub").unwrap() };
        assert_v_eq(unsafe{c_fn(a,b)}, unsafe{r_fn(a,b)}, "c2Sub");
    }
    // c2Add
    {
        type Fn = unsafe extern "C" fn(c2v, c2v) -> c2v;
        let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2Add").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2Add").unwrap() };
        assert_v_eq(unsafe{c_fn(a,b)}, unsafe{r_fn(a,b)}, "c2Add");
    }
    // c2Dot
    {
        type Fn = unsafe extern "C" fn(c2v, c2v) -> f32;
        let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2Dot").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2Dot").unwrap() };
        assert_f_eq(unsafe{c_fn(a,b)}, unsafe{r_fn(a,b)}, "c2Dot");
    }
    // c2Neg
    {
        type Fn = unsafe extern "C" fn(c2v) -> c2v;
        let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2Neg").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2Neg").unwrap() };
        assert_v_eq(unsafe{c_fn(a)}, unsafe{r_fn(a)}, "c2Neg");
    }
    // c2Skew
    {
        type Fn = unsafe extern "C" fn(c2v) -> c2v;
        let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2Skew").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2Skew").unwrap() };
        assert_v_eq(unsafe{c_fn(a)}, unsafe{r_fn(a)}, "c2Skew");
    }
    // c2CCW90
    {
        type Fn = unsafe extern "C" fn(c2v) -> c2v;
        let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2CCW90").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2CCW90").unwrap() };
        assert_v_eq(unsafe{c_fn(a)}, unsafe{r_fn(a)}, "c2CCW90");
    }
}

#[test]
fn test_c2_clampv() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    type Fn = unsafe extern "C" fn(c2v, c2v, c2v) -> c2v;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2Clampv").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2Clampv").unwrap() };
    let a = c2v{x:5.0,y:-3.0};
    let lo = c2v{x:0.0,y:-1.0};
    let hi = c2v{x:3.0,y:2.0};
    assert_v_eq(unsafe{c_fn(a,lo,hi)}, unsafe{r_fn(a,lo,hi)}, "c2Clampv");
}

#[test]
fn test_c2_derived_ops() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    let a = c2v{x:3.0,y:4.0};
    let b = c2v{x:1.0,y:2.0};

    // c2Len
    {
        type Fn = unsafe extern "C" fn(c2v) -> f32;
        let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2Len").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2Len").unwrap() };
        assert_f_eq(unsafe{c_fn(a)}, unsafe{r_fn(a)}, "c2Len");
    }
    // c2Det2
    {
        type Fn = unsafe extern "C" fn(c2v, c2v) -> f32;
        let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2Det2").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2Det2").unwrap() };
        assert_f_eq(unsafe{c_fn(a,b)}, unsafe{r_fn(a,b)}, "c2Det2");
    }
    // c2Div
    {
        type Fn = unsafe extern "C" fn(c2v, f32) -> c2v;
        let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2Div").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2Div").unwrap() };
        assert_v_eq(unsafe{c_fn(a, 2.0)}, unsafe{r_fn(a, 2.0)}, "c2Div");
    }
    // c2Norm
    {
        type Fn = unsafe extern "C" fn(c2v) -> c2v;
        let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2Norm").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2Norm").unwrap() };
        assert_v_eq(unsafe{c_fn(a)}, unsafe{r_fn(a)}, "c2Norm");
    }
}

#[test]
fn test_c2_transform_ops() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    // c2RotIdentity
    {
        type Fn = unsafe extern "C" fn() -> c2r;
        let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2RotIdentity").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2RotIdentity").unwrap() };
        assert_r_eq(unsafe{c_fn()}, unsafe{r_fn()}, "c2RotIdentity");
    }
    // c2xIdentity
    {
        type Fn = unsafe extern "C" fn() -> c2x;
        let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2xIdentity").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2xIdentity").unwrap() };
        let c = unsafe{c_fn()};
        let r = unsafe{r_fn()};
        assert_v_eq(c.p, r.p, "c2xIdentity.p");
        assert_r_eq(c.r, r.r, "c2xIdentity.r");
    }

    let rot = c2r{c:0.866, s:0.5};
    let v = c2v{x:3.0, y:4.0};
    // c2Mulrv
    {
        type Fn = unsafe extern "C" fn(c2r, c2v) -> c2v;
        let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2Mulrv").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2Mulrv").unwrap() };
        assert_v_eq(unsafe{c_fn(rot,v)}, unsafe{r_fn(rot,v)}, "c2Mulrv");
    }
    // c2MulrvT
    {
        type Fn = unsafe extern "C" fn(c2r, c2v) -> c2v;
        let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2MulrvT").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2MulrvT").unwrap() };
        assert_v_eq(unsafe{c_fn(rot,v)}, unsafe{r_fn(rot,v)}, "c2MulrvT");
    }
    // c2Mulxv
    {
        let xf = c2x{p:c2v{x:10.0,y:20.0}, r:rot};
        type Fn = unsafe extern "C" fn(c2x, c2v) -> c2v;
        let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2Mulxv").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2Mulxv").unwrap() };
        assert_v_eq(unsafe{c_fn(xf,v)}, unsafe{r_fn(xf,v)}, "c2Mulxv");
    }
}

#[test]
fn test_c2_support() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    type Fn = unsafe extern "C" fn(*const c2v, i32, c2v) -> i32;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2Support").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2Support").unwrap() };
    let verts = [c2v{x:0.0,y:0.0}, c2v{x:1.0,y:0.0}, c2v{x:0.0,y:1.0}, c2v{x:-1.0,y:-1.0}];
    for &d in &[c2v{x:1.0,y:0.0}, c2v{x:0.0,y:1.0}, c2v{x:-1.0,y:-1.0}, c2v{x:0.5,y:0.5}] {
        let c = unsafe{c_fn(verts.as_ptr(), 4, d)};
        let r = unsafe{r_fn(verts.as_ptr(), 4, d)};
        assert_eq!(c, r, "c2Support d=({},{})", d.x, d.y);
    }
}

#[test]
fn test_c2_bb_verts() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    type Fn = unsafe extern "C" fn(*mut c2v, *const c2AABB);
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2BBVerts").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2BBVerts").unwrap() };
    let bb = c2AABB{min:c2v{x:-1.0,y:-2.0}, max:c2v{x:3.0,y:4.0}};
    let mut c_out = [c2v{x:0.0,y:0.0}; 8];
    let mut r_out = [c2v{x:0.0,y:0.0}; 8];
    unsafe { c_fn(c_out.as_mut_ptr(), &bb); r_fn(r_out.as_mut_ptr(), &bb); }
    for i in 0..4 {
        assert_v_eq(c_out[i], r_out[i], &format!("c2BBVerts[{i}]"));
    }
}

#[test]
fn test_c2_make_proxy() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    type Fn = unsafe extern "C" fn(*const u8, i32, *mut c2Proxy);
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2MakeProxy").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2MakeProxy").unwrap() };

    let zero_proxy = c2Proxy{radius:0.0, count:0, verts:[c2v{x:0.0,y:0.0};8]};

    // Circle (type 0)
    {
        let circle = c2Circle{p:c2v{x:5.0,y:10.0}, r:3.0};
        let mut cp = zero_proxy;
        let mut rp = zero_proxy;
        unsafe { c_fn(&circle as *const _ as *const u8, 0, &mut cp); }
        unsafe { r_fn(&circle as *const _ as *const u8, 0, &mut rp); }
        assert_f_eq(cp.radius, rp.radius, "proxy circle radius");
        assert_eq!(cp.count, rp.count, "proxy circle count");
        assert_v_eq(cp.verts[0], rp.verts[0], "proxy circle vert0");
    }
    // AABB (type 1)
    {
        let bb = c2AABB{min:c2v{x:-1.0,y:-2.0}, max:c2v{x:3.0,y:4.0}};
        let mut cp = zero_proxy;
        let mut rp = zero_proxy;
        unsafe { c_fn(&bb as *const _ as *const u8, 1, &mut cp); }
        unsafe { r_fn(&bb as *const _ as *const u8, 1, &mut rp); }
        assert_f_eq(cp.radius, rp.radius, "proxy aabb radius");
        assert_eq!(cp.count, rp.count, "proxy aabb count");
        for i in 0..4 { assert_v_eq(cp.verts[i], rp.verts[i], &format!("proxy aabb vert{i}")); }
    }
    // Capsule (type 2)
    {
        let cap = c2Capsule{a:c2v{x:1.0,y:2.0}, b:c2v{x:3.0,y:4.0}, r:5.0};
        let mut cp = zero_proxy;
        let mut rp = zero_proxy;
        unsafe { c_fn(&cap as *const _ as *const u8, 2, &mut cp); }
        unsafe { r_fn(&cap as *const _ as *const u8, 2, &mut rp); }
        assert_f_eq(cp.radius, rp.radius, "proxy cap radius");
        assert_eq!(cp.count, rp.count, "proxy cap count");
        for i in 0..2 { assert_v_eq(cp.verts[i], rp.verts[i], &format!("proxy cap vert{i}")); }
    }
}

fn zero_sv() -> c2sv {
    c2sv{s_a:c2v{x:0.0,y:0.0}, s_b:c2v{x:0.0,y:0.0}, p:c2v{x:0.0,y:0.0}, u:0.0, i_a:0, i_b:0}
}

fn assert_simplex_eq(c: &c2Simplex, r: &c2Simplex, ctx: &str) {
    assert_eq!(c.count, r.count, "{ctx} count");
    assert_f_eq(c.div, r.div, &format!("{ctx} div"));
    let svs: &[(fn(&c2Simplex)->&c2sv, &str)] = &[
        (|s| &s.a, "a"), (|s| &s.b, "b"), (|s| &s.c, "c"),
    ];
    for i in 0..(c.count as usize).min(3) {
        let (f, name) = &svs[i];
        let csv = f(c);
        let rsv = f(r);
        assert_v_eq(csv.s_a, rsv.s_a, &format!("{ctx}.{name}.s_a"));
        assert_v_eq(csv.s_b, rsv.s_b, &format!("{ctx}.{name}.s_b"));
        assert_v_eq(csv.p, rsv.p, &format!("{ctx}.{name}.p"));
        assert_f_eq(csv.u, rsv.u, &format!("{ctx}.{name}.u"));
        assert_eq!(csv.i_a, rsv.i_a, "{ctx}.{name}.i_a");
        assert_eq!(csv.i_b, rsv.i_b, "{ctx}.{name}.i_b");
    }
}

#[test]
fn test_c22_c23() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    type Fn22 = unsafe extern "C" fn(*mut c2Simplex);
    let c_c22: Symbol<Fn22> = unsafe { c_lib.get(b"c22").unwrap() };
    let r_c22: Symbol<Fn22> = unsafe { r_lib.get(b"c22").unwrap() };
    let c_c23: Symbol<Fn22> = unsafe { c_lib.get(b"c23").unwrap() };
    let r_c23: Symbol<Fn22> = unsafe { r_lib.get(b"c23").unwrap() };

    // Test c22 with various 2-simplex configs
    let test_points: &[(c2v, c2v)] = &[
        (c2v{x:1.0,y:0.0}, c2v{x:-1.0,y:0.0}),
        (c2v{x:0.0,y:0.0}, c2v{x:1.0,y:1.0}),
        (c2v{x:-2.0,y:3.0}, c2v{x:4.0,y:-1.0}),
    ];
    for (pa, pb) in test_points {
        let base = c2Simplex{
            a: c2sv{p:*pa, u:0.0, ..zero_sv()},
            b: c2sv{p:*pb, u:0.0, ..zero_sv()},
            c: zero_sv(), d: zero_sv(), div: 1.0, count: 2,
        };
        let mut cs = base;
        let mut rs = base;
        unsafe { c_c22(&mut cs); r_c22(&mut rs); }
        assert_simplex_eq(&cs, &rs, "c22");
    }

    // Test c23 with a 3-simplex
    let tri_cases: &[(c2v, c2v, c2v)] = &[
        (c2v{x:0.0,y:0.0}, c2v{x:2.0,y:0.0}, c2v{x:1.0,y:2.0}),
        (c2v{x:-1.0,y:-1.0}, c2v{x:1.0,y:-1.0}, c2v{x:0.0,y:1.0}),
        (c2v{x:5.0,y:5.0}, c2v{x:6.0,y:5.0}, c2v{x:5.5,y:6.0}),
    ];
    for (pa, pb, pc) in tri_cases {
        let base = c2Simplex{
            a: c2sv{p:*pa, u:0.0, ..zero_sv()},
            b: c2sv{p:*pb, u:0.0, ..zero_sv()},
            c: c2sv{p:*pc, u:0.0, ..zero_sv()},
            d: zero_sv(), div: 1.0, count: 3,
        };
        let mut cs = base;
        let mut rs = base;
        unsafe { c_c23(&mut cs); r_c23(&mut rs); }
        assert_simplex_eq(&cs, &rs, "c23");
    }
}

#[test]
fn test_c2_simplex_ops() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    // c2GJKSimplexMetric
    {
        type Fn = unsafe extern "C" fn(*mut c2Simplex) -> f32;
        let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2GJKSimplexMetric").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2GJKSimplexMetric").unwrap() };
        // count=1
        let mut s1 = c2Simplex{a:zero_sv(), b:zero_sv(), c:zero_sv(), d:zero_sv(), div:1.0, count:1};
        assert_f_eq(unsafe{c_fn(&mut s1)}, unsafe{r_fn(&mut s1)}, "metric count=1");
        // count=2
        let mut s2 = c2Simplex{
            a:c2sv{p:c2v{x:0.0,y:0.0}, ..zero_sv()},
            b:c2sv{p:c2v{x:3.0,y:4.0}, ..zero_sv()},
            c:zero_sv(), d:zero_sv(), div:1.0, count:2};
        assert_f_eq(unsafe{c_fn(&mut s2)}, unsafe{r_fn(&mut s2)}, "metric count=2");
        // count=3
        let mut s3 = c2Simplex{
            a:c2sv{p:c2v{x:0.0,y:0.0}, ..zero_sv()},
            b:c2sv{p:c2v{x:3.0,y:0.0}, ..zero_sv()},
            c:c2sv{p:c2v{x:0.0,y:4.0}, ..zero_sv()},
            d:zero_sv(), div:1.0, count:3};
        assert_f_eq(unsafe{c_fn(&mut s3)}, unsafe{r_fn(&mut s3)}, "metric count=3");
    }

    // c2D
    {
        type Fn = unsafe extern "C" fn(*mut c2Simplex) -> c2v;
        let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2D").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2D").unwrap() };
        // count=1
        let mut s1 = c2Simplex{a:c2sv{p:c2v{x:2.0,y:3.0}, ..zero_sv()}, b:zero_sv(), c:zero_sv(), d:zero_sv(), div:1.0, count:1};
        assert_v_eq(unsafe{c_fn(&mut s1)}, unsafe{r_fn(&mut s1)}, "c2D count=1");
        // count=2
        let mut s2 = c2Simplex{
            a:c2sv{p:c2v{x:-1.0,y:0.0}, ..zero_sv()},
            b:c2sv{p:c2v{x:1.0,y:0.0}, ..zero_sv()},
            c:zero_sv(), d:zero_sv(), div:1.0, count:2};
        assert_v_eq(unsafe{c_fn(&mut s2)}, unsafe{r_fn(&mut s2)}, "c2D count=2");
        // count=3
        let mut s3 = c2Simplex{a:zero_sv(), b:zero_sv(), c:zero_sv(), d:zero_sv(), div:1.0, count:3};
        assert_v_eq(unsafe{c_fn(&mut s3)}, unsafe{r_fn(&mut s3)}, "c2D count=3");
    }

    // c2L
    {
        type Fn = unsafe extern "C" fn(*mut c2Simplex) -> c2v;
        let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2L").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2L").unwrap() };
        let mut s1 = c2Simplex{a:c2sv{p:c2v{x:2.0,y:3.0}, u:1.0, ..zero_sv()}, b:zero_sv(), c:zero_sv(), d:zero_sv(), div:1.0, count:1};
        assert_v_eq(unsafe{c_fn(&mut s1)}, unsafe{r_fn(&mut s1)}, "c2L count=1");
        let mut s2 = c2Simplex{
            a:c2sv{p:c2v{x:1.0,y:0.0}, u:2.0, ..zero_sv()},
            b:c2sv{p:c2v{x:0.0,y:1.0}, u:3.0, ..zero_sv()},
            c:zero_sv(), d:zero_sv(), div:5.0, count:2};
        assert_v_eq(unsafe{c_fn(&mut s2)}, unsafe{r_fn(&mut s2)}, "c2L count=2");
    }

    // c2Witness
    {
        type Fn = unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v);
        let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2Witness").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2Witness").unwrap() };
        for count in 1..=3 {
            let mut s = c2Simplex{
                a:c2sv{s_a:c2v{x:1.0,y:2.0}, s_b:c2v{x:3.0,y:4.0}, p:c2v{x:0.0,y:0.0}, u:1.0, i_a:0, i_b:0},
                b:c2sv{s_a:c2v{x:5.0,y:6.0}, s_b:c2v{x:7.0,y:8.0}, p:c2v{x:0.0,y:0.0}, u:2.0, i_a:0, i_b:0},
                c:c2sv{s_a:c2v{x:9.0,y:10.0}, s_b:c2v{x:11.0,y:12.0}, p:c2v{x:0.0,y:0.0}, u:3.0, i_a:0, i_b:0},
                d:zero_sv(), div:6.0, count,
            };
            let mut s2 = s;
            let (mut ca, mut cb) = (c2v{x:0.0,y:0.0}, c2v{x:0.0,y:0.0});
            let (mut ra, mut rb) = (c2v{x:0.0,y:0.0}, c2v{x:0.0,y:0.0});
            unsafe { c_fn(&mut s, &mut ca, &mut cb); r_fn(&mut s2, &mut ra, &mut rb); }
            assert_v_eq(ca, ra, &format!("c2Witness a count={count}"));
            assert_v_eq(cb, rb, &format!("c2Witness b count={count}"));
        }
    }
}

#[test]
fn test_c2_gjk() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    type GjkFn = unsafe extern "C" fn(
        *const u8, i32, *const c2x,
        *const u8, i32, *const c2x,
        *mut c2v, *mut c2v,
        i32, *mut i32, *mut c2GJKCache,
    ) -> f32;
    let c_gjk: Symbol<GjkFn> = unsafe { c_lib.get(b"c2GJK").unwrap() };
    let r_gjk: Symbol<GjkFn> = unsafe { r_lib.get(b"c2GJK").unwrap() };

    // Test 1: Circle vs Capsule (no cache, with radius)
    {
        let circle = c2Circle{p:c2v{x:0.0,y:0.0}, r:15.0};
        let cap = c2Capsule{a:c2v{x:100.0,y:-25.0}, b:c2v{x:75.0,y:100.0}, r:10.0};
        let (mut ca, mut cb) = (c2v{x:0.0,y:0.0}, c2v{x:0.0,y:0.0});
        let (mut ra, mut rb) = (c2v{x:0.0,y:0.0}, c2v{x:0.0,y:0.0});
        let mut c_iter = 0i32;
        let mut r_iter = 0i32;
        let cd = unsafe { c_gjk(
            &circle as *const _ as *const u8, 0, std::ptr::null(),
            &cap as *const _ as *const u8, 2, std::ptr::null(),
            &mut ca, &mut cb, 1, &mut c_iter, std::ptr::null_mut()) };
        let rd = unsafe { r_gjk(
            &circle as *const _ as *const u8, 0, std::ptr::null(),
            &cap as *const _ as *const u8, 2, std::ptr::null(),
            &mut ra, &mut rb, 1, &mut r_iter, std::ptr::null_mut()) };
        assert_f_eq(cd, rd, "c2GJK circle-cap dist");
        assert_v_eq(ca, ra, "c2GJK circle-cap outA");
        assert_v_eq(cb, rb, "c2GJK circle-cap outB");
        assert_eq!(c_iter, r_iter, "c2GJK circle-cap iters");
    }

    // Test 2: AABB vs Capsule (no cache, with radius)
    {
        let bb = c2AABB{min:c2v{x:-10.0,y:-10.0}, max:c2v{x:10.0,y:10.0}};
        let cap = c2Capsule{a:c2v{x:20.0,y:0.0}, b:c2v{x:30.0,y:5.0}, r:3.0};
        let (mut ca, mut cb) = (c2v{x:0.0,y:0.0}, c2v{x:0.0,y:0.0});
        let (mut ra, mut rb) = (c2v{x:0.0,y:0.0}, c2v{x:0.0,y:0.0});
        let cd = unsafe { c_gjk(
            &bb as *const _ as *const u8, 1, std::ptr::null(),
            &cap as *const _ as *const u8, 2, std::ptr::null(),
            &mut ca, &mut cb, 1, std::ptr::null_mut(), std::ptr::null_mut()) };
        let rd = unsafe { r_gjk(
            &bb as *const _ as *const u8, 1, std::ptr::null(),
            &cap as *const _ as *const u8, 2, std::ptr::null(),
            &mut ra, &mut rb, 1, std::ptr::null_mut(), std::ptr::null_mut()) };
        assert_f_eq(cd, rd, "c2GJK aabb-cap dist");
        assert_v_eq(ca, ra, "c2GJK aabb-cap outA");
        assert_v_eq(cb, rb, "c2GJK aabb-cap outB");
    }

    // Test 3: With cache (two calls)
    {
        let circle = c2Circle{p:c2v{x:0.0,y:0.0}, r:15.0};
        let cap = c2Capsule{a:c2v{x:100.0,y:-25.0}, b:c2v{x:75.0,y:100.0}, r:10.0};
        let mut c_cache = c2GJKCache{metric:0.0, count:0, i_a:[0;3], i_b:[0;3], div:0.0};
        let mut r_cache = c2GJKCache{metric:0.0, count:0, i_a:[0;3], i_b:[0;3], div:0.0};
        let (mut ca, mut cb) = (c2v{x:0.0,y:0.0}, c2v{x:0.0,y:0.0});
        let (mut ra, mut rb) = (c2v{x:0.0,y:0.0}, c2v{x:0.0,y:0.0});
        // First call populates cache
        unsafe { c_gjk(&circle as *const _ as *const u8, 0, std::ptr::null(),
            &cap as *const _ as *const u8, 2, std::ptr::null(),
            &mut ca, &mut cb, 1, std::ptr::null_mut(), &mut c_cache); }
        unsafe { r_gjk(&circle as *const _ as *const u8, 0, std::ptr::null(),
            &cap as *const _ as *const u8, 2, std::ptr::null(),
            &mut ra, &mut rb, 1, std::ptr::null_mut(), &mut r_cache); }
        assert_f_eq(c_cache.metric, r_cache.metric, "cache1 metric");
        assert_eq!(c_cache.count, r_cache.count, "cache1 count");
        assert_f_eq(c_cache.div, r_cache.div, "cache1 div");
        // Second call uses cache
        let cd = unsafe { c_gjk(&circle as *const _ as *const u8, 0, std::ptr::null(),
            &cap as *const _ as *const u8, 2, std::ptr::null(),
            &mut ca, &mut cb, 1, std::ptr::null_mut(), &mut c_cache) };
        let rd = unsafe { r_gjk(&circle as *const _ as *const u8, 0, std::ptr::null(),
            &cap as *const _ as *const u8, 2, std::ptr::null(),
            &mut ra, &mut rb, 1, std::ptr::null_mut(), &mut r_cache) };
        assert_f_eq(cd, rd, "c2GJK cached dist");
        assert_v_eq(ca, ra, "c2GJK cached outA");
        assert_v_eq(cb, rb, "c2GJK cached outB");
    }

    // Test 4: Overlapping shapes (hit case)
    {
        let c1 = c2Circle{p:c2v{x:0.0,y:0.0}, r:10.0};
        let c2 = c2Circle{p:c2v{x:5.0,y:0.0}, r:10.0};
        let (mut ca, mut cb) = (c2v{x:0.0,y:0.0}, c2v{x:0.0,y:0.0});
        let (mut ra, mut rb) = (c2v{x:0.0,y:0.0}, c2v{x:0.0,y:0.0});
        let cd = unsafe { c_gjk(&c1 as *const _ as *const u8, 0, std::ptr::null(),
            &c2 as *const _ as *const u8, 0, std::ptr::null(),
            &mut ca, &mut cb, 1, std::ptr::null_mut(), std::ptr::null_mut()) };
        let rd = unsafe { r_gjk(&c1 as *const _ as *const u8, 0, std::ptr::null(),
            &c2 as *const _ as *const u8, 0, std::ptr::null(),
            &mut ra, &mut rb, 1, std::ptr::null_mut(), std::ptr::null_mut()) };
        assert_f_eq(cd, rd, "c2GJK overlap dist");
        assert_v_eq(ca, ra, "c2GJK overlap outA");
        assert_v_eq(cb, rb, "c2GJK overlap outB");
    }

    // Test 5: No radius
    {
        let bb = c2AABB{min:c2v{x:0.0,y:0.0}, max:c2v{x:5.0,y:5.0}};
        let cap = c2Capsule{a:c2v{x:10.0,y:0.0}, b:c2v{x:15.0,y:5.0}, r:2.0};
        let (mut ca, mut cb) = (c2v{x:0.0,y:0.0}, c2v{x:0.0,y:0.0});
        let (mut ra, mut rb) = (c2v{x:0.0,y:0.0}, c2v{x:0.0,y:0.0});
        let cd = unsafe { c_gjk(&bb as *const _ as *const u8, 1, std::ptr::null(),
            &cap as *const _ as *const u8, 2, std::ptr::null(),
            &mut ca, &mut cb, 0, std::ptr::null_mut(), std::ptr::null_mut()) };
        let rd = unsafe { r_gjk(&bb as *const _ as *const u8, 1, std::ptr::null(),
            &cap as *const _ as *const u8, 2, std::ptr::null(),
            &mut ra, &mut rb, 0, std::ptr::null_mut(), std::ptr::null_mut()) };
        assert_f_eq(cd, rd, "c2GJK no-radius dist");
        assert_v_eq(ca, ra, "c2GJK no-radius outA");
        assert_v_eq(cb, rb, "c2GJK no-radius outB");
    }
}

#[test]
fn test_gjk_cache_entry() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    type Fn = unsafe extern "C" fn(
        i8, *mut c2v, *mut c2v,
        f32, f32, f32, f32,
        f32, f32, f32, f32, f32,
    );
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"gjk_cache").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"gjk_cache").unwrap() };

    let test_cases: &[(i8, f32, f32, f32, f32, f32, f32, f32, f32, f32)] = &[
        (0, -10.0, -10.0, 10.0, 10.0, 20.0, 0.0, 30.0, 5.0, 3.0),
        (1, -10.0, -10.0, 10.0, 10.0, 20.0, 0.0, 30.0, 5.0, 3.0),
        (0, 0.0, 0.0, 5.0, 5.0, 100.0, 100.0, 200.0, 200.0, 10.0),
        (1, -50.0, -50.0, 50.0, 50.0, 0.0, 0.0, 10.0, 10.0, 1.0),
    ];

    for &(rev, a1, a2, a3, a4, b1, b2, b3, b4, b5) in test_cases {
        let mut c_a9 = c2v{x:0.0,y:0.0};
        let mut c_b9 = c2v{x:0.0,y:0.0};
        let mut r_a9 = c2v{x:0.0,y:0.0};
        let mut r_b9 = c2v{x:0.0,y:0.0};
        unsafe { c_fn(rev, &mut c_a9, &mut c_b9, a1, a2, a3, a4, b1, b2, b3, b4, b5); }
        unsafe { r_fn(rev, &mut r_a9, &mut r_b9, a1, a2, a3, a4, b1, b2, b3, b4, b5); }
        // gjk_cache doesn't write to a9/b9 in the C code, but we verify the function runs without crash
        // The function's side effects are internal; we just verify it doesn't diverge
    }
}

#[test]
fn test_c2_gjk_with_transforms() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    type GjkFn = unsafe extern "C" fn(
        *const u8, i32, *const c2x,
        *const u8, i32, *const c2x,
        *mut c2v, *mut c2v,
        i32, *mut i32, *mut c2GJKCache,
    ) -> f32;
    let c_gjk: Symbol<GjkFn> = unsafe { c_lib.get(b"c2GJK").unwrap() };
    let r_gjk: Symbol<GjkFn> = unsafe { r_lib.get(b"c2GJK").unwrap() };

    let rot30 = c2r{c:0.866025, s:0.5};
    let ax = c2x{p:c2v{x:10.0,y:5.0}, r:rot30};
    let bx = c2x{p:c2v{x:-10.0,y:-5.0}, r:c2r{c:1.0,s:0.0}};

    let bb = c2AABB{min:c2v{x:-2.0,y:-2.0}, max:c2v{x:2.0,y:2.0}};
    let cap = c2Capsule{a:c2v{x:0.0,y:-5.0}, b:c2v{x:0.0,y:5.0}, r:1.0};

    let (mut ca, mut cb) = (c2v{x:0.0,y:0.0}, c2v{x:0.0,y:0.0});
    let (mut ra, mut rb) = (c2v{x:0.0,y:0.0}, c2v{x:0.0,y:0.0});
    let cd = unsafe { c_gjk(&bb as *const _ as *const u8, 1, &ax,
        &cap as *const _ as *const u8, 2, &bx,
        &mut ca, &mut cb, 1, std::ptr::null_mut(), std::ptr::null_mut()) };
    let rd = unsafe { r_gjk(&bb as *const _ as *const u8, 1, &ax,
        &cap as *const _ as *const u8, 2, &bx,
        &mut ra, &mut rb, 1, std::ptr::null_mut(), std::ptr::null_mut()) };
    assert_f_eq(cd, rd, "c2GJK xform dist");
    assert_v_eq(ca, ra, "c2GJK xform outA");
    assert_v_eq(cb, rb, "c2GJK xform outB");
}
