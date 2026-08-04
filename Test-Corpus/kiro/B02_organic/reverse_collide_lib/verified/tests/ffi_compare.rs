use libloading::{Library, Symbol};
use std::os::raw::c_int;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2v { x: f32, y: f32 }

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2r { c: f32, s: f32 }

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2x { p: C2v, r: C2r }

#[repr(C)]
#[derive(Clone, Copy)]
struct C2Circle { p: C2v, r: f32 }

#[repr(C)]
#[derive(Clone, Copy)]
struct C2AABB { min: C2v, max: C2v }

#[repr(C)]
#[derive(Clone, Copy)]
struct C2Capsule { a: C2v, b: C2v, r: f32 }

#[repr(C)]
#[derive(Clone, Copy)]
struct C2GJKCache { metric: f32, count: c_int, i_a: [c_int; 3], i_b: [c_int; 3], div: f32 }

#[repr(C)]
#[derive(Clone, Copy)]
struct C2sv { s_a: C2v, s_b: C2v, p: C2v, u: f32, i_a: c_int, i_b: c_int }

#[repr(C)]
#[derive(Clone, Copy)]
struct C2Simplex { a: C2sv, b: C2sv, c: C2sv, d: C2sv, div: f32, count: c_int }

#[repr(C)]
#[derive(Clone, Copy)]
struct C2Proxy { radius: f32, count: c_int, verts: [C2v; 8] }

const C2_TYPE_CIRCLE: c_int = 0;
const C2_TYPE_AABB: c_int = 1;
const C2_TYPE_CAPSULE: c_int = 2;

fn c_lib() -> Library { unsafe { Library::new("c_src/build/libtranslated_rust.so").unwrap() } }
fn rs_lib() -> Library { unsafe { Library::new("target/debug/libreverse_collide_lib.so").unwrap() } }

fn assert_v_eq(a: C2v, b: C2v, ctx: &str) {
    assert!(a.x.to_bits() == b.x.to_bits() && a.y.to_bits() == b.y.to_bits(),
        "{ctx}: C=({},{}) Rust=({},{})", a.x, a.y, b.x, b.y);
}

#[test]
fn test_c2v() {
    let c = c_lib(); let r = rs_lib();
    unsafe {
        let cf: Symbol<unsafe extern "C" fn(f32, f32) -> C2v> = c.get(b"c2V").unwrap();
        let rf: Symbol<unsafe extern "C" fn(f32, f32) -> C2v> = r.get(b"c2V").unwrap();
        for (x, y) in [(0.0f32, 0.0), (1.5, -2.3), (f32::MAX, f32::MIN)] {
            assert_v_eq(cf(x, y), rf(x, y), &format!("c2V({x},{y})"));
        }
    }
}

#[test]
fn test_c2_arithmetic() {
    let c = c_lib(); let r = rs_lib();
    unsafe {
        let c_add: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = c.get(b"c2Add").unwrap();
        let r_add: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = r.get(b"c2Add").unwrap();
        let c_sub: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = c.get(b"c2Sub").unwrap();
        let r_sub: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = r.get(b"c2Sub").unwrap();
        let c_neg: Symbol<unsafe extern "C" fn(C2v) -> C2v> = c.get(b"c2Neg").unwrap();
        let r_neg: Symbol<unsafe extern "C" fn(C2v) -> C2v> = r.get(b"c2Neg").unwrap();
        let c_dot: Symbol<unsafe extern "C" fn(C2v, C2v) -> f32> = c.get(b"c2Dot").unwrap();
        let r_dot: Symbol<unsafe extern "C" fn(C2v, C2v) -> f32> = r.get(b"c2Dot").unwrap();
        let c_mulvs: Symbol<unsafe extern "C" fn(C2v, f32) -> C2v> = c.get(b"c2Mulvs").unwrap();
        let r_mulvs: Symbol<unsafe extern "C" fn(C2v, f32) -> C2v> = r.get(b"c2Mulvs").unwrap();
        let c_len: Symbol<unsafe extern "C" fn(C2v) -> f32> = c.get(b"c2Len").unwrap();
        let r_len: Symbol<unsafe extern "C" fn(C2v) -> f32> = r.get(b"c2Len").unwrap();
        let c_det2: Symbol<unsafe extern "C" fn(C2v, C2v) -> f32> = c.get(b"c2Det2").unwrap();
        let r_det2: Symbol<unsafe extern "C" fn(C2v, C2v) -> f32> = r.get(b"c2Det2").unwrap();
        let c_skew: Symbol<unsafe extern "C" fn(C2v) -> C2v> = c.get(b"c2Skew").unwrap();
        let r_skew: Symbol<unsafe extern "C" fn(C2v) -> C2v> = r.get(b"c2Skew").unwrap();
        let c_ccw: Symbol<unsafe extern "C" fn(C2v) -> C2v> = c.get(b"c2CCW90").unwrap();
        let r_ccw: Symbol<unsafe extern "C" fn(C2v) -> C2v> = r.get(b"c2CCW90").unwrap();

        let vecs = [
            C2v{x:0.0,y:0.0}, C2v{x:1.0,y:2.0}, C2v{x:-3.5,y:7.1},
            C2v{x:100.0,y:-0.001}, C2v{x:f32::EPSILON,y:f32::EPSILON},
        ];
        for a in &vecs { for b in &vecs {
            assert_v_eq(c_add(*a,*b), r_add(*a,*b), "c2Add");
            assert_v_eq(c_sub(*a,*b), r_sub(*a,*b), "c2Sub");
            assert_eq!(c_dot(*a,*b).to_bits(), r_dot(*a,*b).to_bits(), "c2Dot");
            assert_eq!(c_det2(*a,*b).to_bits(), r_det2(*a,*b).to_bits(), "c2Det2");
        }}
        for a in &vecs {
            assert_v_eq(c_neg(*a), r_neg(*a), "c2Neg");
            assert_v_eq(c_skew(*a), r_skew(*a), "c2Skew");
            assert_v_eq(c_ccw(*a), r_ccw(*a), "c2CCW90");
            assert_eq!(c_len(*a).to_bits(), r_len(*a).to_bits(), "c2Len");
            for s in [0.0f32, 1.0, -2.5, 100.0] {
                assert_v_eq(c_mulvs(*a,s), r_mulvs(*a,s), "c2Mulvs");
            }
        }
    }
}

#[test]
fn test_c2_vector_ops() {
    let c = c_lib(); let r = rs_lib();
    unsafe {
        let c_maxv: Symbol<unsafe extern "C" fn(C2v,C2v)->C2v> = c.get(b"c2Maxv").unwrap();
        let r_maxv: Symbol<unsafe extern "C" fn(C2v,C2v)->C2v> = r.get(b"c2Maxv").unwrap();
        let c_minv: Symbol<unsafe extern "C" fn(C2v,C2v)->C2v> = c.get(b"c2Minv").unwrap();
        let r_minv: Symbol<unsafe extern "C" fn(C2v,C2v)->C2v> = r.get(b"c2Minv").unwrap();
        let c_clamp: Symbol<unsafe extern "C" fn(C2v,C2v,C2v)->C2v> = c.get(b"c2Clampv").unwrap();
        let r_clamp: Symbol<unsafe extern "C" fn(C2v,C2v,C2v)->C2v> = r.get(b"c2Clampv").unwrap();
        let c_div: Symbol<unsafe extern "C" fn(C2v,f32)->C2v> = c.get(b"c2Div").unwrap();
        let r_div: Symbol<unsafe extern "C" fn(C2v,f32)->C2v> = r.get(b"c2Div").unwrap();
        let c_norm: Symbol<unsafe extern "C" fn(C2v)->C2v> = c.get(b"c2Norm").unwrap();
        let r_norm: Symbol<unsafe extern "C" fn(C2v)->C2v> = r.get(b"c2Norm").unwrap();

        let a = C2v{x:3.0,y:-1.0}; let b = C2v{x:1.0,y:5.0};
        assert_v_eq(c_maxv(a,b), r_maxv(a,b), "c2Maxv");
        assert_v_eq(c_minv(a,b), r_minv(a,b), "c2Minv");
        let lo = C2v{x:0.0,y:0.0}; let hi = C2v{x:2.0,y:2.0};
        assert_v_eq(c_clamp(a,lo,hi), r_clamp(a,lo,hi), "c2Clampv");
        assert_v_eq(c_div(a,2.0), r_div(a,2.0), "c2Div");
        assert_v_eq(c_norm(a), r_norm(a), "c2Norm");
    }
}

#[test]
fn test_c2_rotation() {
    let c = c_lib(); let r = rs_lib();
    unsafe {
        let c_ri: Symbol<unsafe extern "C" fn()->C2r> = c.get(b"c2RotIdentity").unwrap();
        let r_ri: Symbol<unsafe extern "C" fn()->C2r> = r.get(b"c2RotIdentity").unwrap();
        let cr = c_ri(); let rr = r_ri();
        assert_eq!(cr.c.to_bits(), rr.c.to_bits()); assert_eq!(cr.s.to_bits(), rr.s.to_bits());

        let c_xi: Symbol<unsafe extern "C" fn()->C2x> = c.get(b"c2xIdentity").unwrap();
        let r_xi: Symbol<unsafe extern "C" fn()->C2x> = r.get(b"c2xIdentity").unwrap();
        let cx = c_xi(); let rx = r_xi();
        assert_v_eq(cx.p, rx.p, "c2xIdentity.p");
        assert_eq!(cx.r.c.to_bits(), rx.r.c.to_bits()); assert_eq!(cx.r.s.to_bits(), rx.r.s.to_bits());

        let c_mulrv: Symbol<unsafe extern "C" fn(C2r,C2v)->C2v> = c.get(b"c2Mulrv").unwrap();
        let r_mulrv: Symbol<unsafe extern "C" fn(C2r,C2v)->C2v> = r.get(b"c2Mulrv").unwrap();
        let c_mulrvt: Symbol<unsafe extern "C" fn(C2r,C2v)->C2v> = c.get(b"c2MulrvT").unwrap();
        let r_mulrvt: Symbol<unsafe extern "C" fn(C2r,C2v)->C2v> = r.get(b"c2MulrvT").unwrap();
        let c_mulxv: Symbol<unsafe extern "C" fn(C2x,C2v)->C2v> = c.get(b"c2Mulxv").unwrap();
        let r_mulxv: Symbol<unsafe extern "C" fn(C2x,C2v)->C2v> = r.get(b"c2Mulxv").unwrap();

        let rot = C2r{c:0.6,s:0.8}; let v = C2v{x:3.0,y:4.0};
        assert_v_eq(c_mulrv(rot,v), r_mulrv(rot,v), "c2Mulrv");
        assert_v_eq(c_mulrvt(rot,v), r_mulrvt(rot,v), "c2MulrvT");
        let xf = C2x{p:C2v{x:10.0,y:20.0},r:rot};
        assert_v_eq(c_mulxv(xf,v), r_mulxv(xf,v), "c2Mulxv");
    }
}

#[test]
fn test_c2_bb_verts_and_make_proxy() {
    let c = c_lib(); let r = rs_lib();
    unsafe {
        // c2BBVerts
        let c_bb: Symbol<unsafe extern "C" fn(*mut C2v, *const C2AABB)> = c.get(b"c2BBVerts").unwrap();
        let r_bb: Symbol<unsafe extern "C" fn(*mut C2v, *const C2AABB)> = r.get(b"c2BBVerts").unwrap();
        let aabb = C2AABB{min:C2v{x:-10.0,y:-20.0},max:C2v{x:30.0,y:40.0}};
        let mut co = [C2v{x:0.0,y:0.0};8]; let mut ro = [C2v{x:0.0,y:0.0};8];
        c_bb(co.as_mut_ptr(), &aabb); r_bb(ro.as_mut_ptr(), &aabb);
        for i in 0..4 { assert_v_eq(co[i], ro[i], &format!("c2BBVerts[{i}]")); }

        // c2MakeProxy - circle
        let c_mp: Symbol<unsafe extern "C" fn(*const u8, c_int, *mut C2Proxy)> = c.get(b"c2MakeProxy").unwrap();
        let r_mp: Symbol<unsafe extern "C" fn(*const u8, c_int, *mut C2Proxy)> = r.get(b"c2MakeProxy").unwrap();
        let circle = C2Circle{p:C2v{x:5.0,y:6.0},r:3.0};
        let mut cp = std::mem::zeroed::<C2Proxy>(); let mut rp = std::mem::zeroed::<C2Proxy>();
        c_mp(&circle as *const _ as *const u8, C2_TYPE_CIRCLE, &mut cp);
        r_mp(&circle as *const _ as *const u8, C2_TYPE_CIRCLE, &mut rp);
        assert_eq!(cp.radius.to_bits(), rp.radius.to_bits());
        assert_eq!(cp.count, rp.count);
        for i in 0..cp.count as usize { assert_v_eq(cp.verts[i], rp.verts[i], "proxy circle vert"); }

        // c2MakeProxy - aabb
        c_mp(&aabb as *const _ as *const u8, C2_TYPE_AABB, &mut cp);
        r_mp(&aabb as *const _ as *const u8, C2_TYPE_AABB, &mut rp);
        assert_eq!(cp.radius.to_bits(), rp.radius.to_bits());
        assert_eq!(cp.count, rp.count);
        for i in 0..cp.count as usize { assert_v_eq(cp.verts[i], rp.verts[i], "proxy aabb vert"); }

        // c2MakeProxy - capsule
        let cap = C2Capsule{a:C2v{x:1.0,y:2.0},b:C2v{x:3.0,y:4.0},r:5.0};
        c_mp(&cap as *const _ as *const u8, C2_TYPE_CAPSULE, &mut cp);
        r_mp(&cap as *const _ as *const u8, C2_TYPE_CAPSULE, &mut rp);
        assert_eq!(cp.radius.to_bits(), rp.radius.to_bits());
        assert_eq!(cp.count, rp.count);
        for i in 0..cp.count as usize { assert_v_eq(cp.verts[i], rp.verts[i], "proxy capsule vert"); }
    }
}

#[test]
fn test_c2_support() {
    let c = c_lib(); let r = rs_lib();
    unsafe {
        let c_sup: Symbol<unsafe extern "C" fn(*const C2v, c_int, C2v) -> c_int> = c.get(b"c2Support").unwrap();
        let r_sup: Symbol<unsafe extern "C" fn(*const C2v, c_int, C2v) -> c_int> = r.get(b"c2Support").unwrap();
        let verts = [C2v{x:0.0,y:0.0}, C2v{x:10.0,y:0.0}, C2v{x:5.0,y:10.0}, C2v{x:-5.0,y:5.0},
                     C2v{x:0.0,y:0.0}, C2v{x:0.0,y:0.0}, C2v{x:0.0,y:0.0}, C2v{x:0.0,y:0.0}];
        let dirs = [C2v{x:1.0,y:0.0}, C2v{x:0.0,y:1.0}, C2v{x:-1.0,y:0.0}, C2v{x:0.0,y:-1.0}, C2v{x:1.0,y:1.0}];
        for d in &dirs {
            assert_eq!(c_sup(verts.as_ptr(), 4, *d), r_sup(verts.as_ptr(), 4, *d), "c2Support");
        }
    }
}

#[test]
fn test_collision_functions() {
    let c = c_lib(); let r = rs_lib();
    unsafe {
        // c2CircletoCircle
        let c_cc: Symbol<unsafe extern "C" fn(C2Circle,C2Circle)->c_int> = c.get(b"c2CircletoCircle").unwrap();
        let r_cc: Symbol<unsafe extern "C" fn(C2Circle,C2Circle)->c_int> = r.get(b"c2CircletoCircle").unwrap();
        let ca = C2Circle{p:C2v{x:0.0,y:0.0},r:5.0};
        let cb_hit = C2Circle{p:C2v{x:8.0,y:0.0},r:5.0};
        let cb_miss = C2Circle{p:C2v{x:20.0,y:0.0},r:5.0};
        assert_eq!(c_cc(ca,cb_hit), r_cc(ca,cb_hit), "CircletoCircle hit");
        assert_eq!(c_cc(ca,cb_miss), r_cc(ca,cb_miss), "CircletoCircle miss");

        // c2CircletoAABB
        let c_ca: Symbol<unsafe extern "C" fn(C2Circle,C2AABB)->c_int> = c.get(b"c2CircletoAABB").unwrap();
        let r_ca: Symbol<unsafe extern "C" fn(C2Circle,C2AABB)->c_int> = r.get(b"c2CircletoAABB").unwrap();
        let aabb = C2AABB{min:C2v{x:-5.0,y:-5.0},max:C2v{x:5.0,y:5.0}};
        let c_hit = C2Circle{p:C2v{x:8.0,y:0.0},r:5.0};
        let c_miss = C2Circle{p:C2v{x:20.0,y:0.0},r:2.0};
        assert_eq!(c_ca(c_hit,aabb), r_ca(c_hit,aabb), "CircletoAABB hit");
        assert_eq!(c_ca(c_miss,aabb), r_ca(c_miss,aabb), "CircletoAABB miss");

        // c2AABBtoAABB
        let c_aa: Symbol<unsafe extern "C" fn(C2AABB,C2AABB)->c_int> = c.get(b"c2AABBtoAABB").unwrap();
        let r_aa: Symbol<unsafe extern "C" fn(C2AABB,C2AABB)->c_int> = r.get(b"c2AABBtoAABB").unwrap();
        let b2 = C2AABB{min:C2v{x:3.0,y:3.0},max:C2v{x:10.0,y:10.0}};
        let b3 = C2AABB{min:C2v{x:50.0,y:50.0},max:C2v{x:60.0,y:60.0}};
        assert_eq!(c_aa(aabb,b2), r_aa(aabb,b2), "AABBtoAABB hit");
        assert_eq!(c_aa(aabb,b3), r_aa(aabb,b3), "AABBtoAABB miss");

        // c2CircletoCapsule
        let c_ccap: Symbol<unsafe extern "C" fn(C2Circle,C2Capsule)->c_int> = c.get(b"c2CircletoCapsule").unwrap();
        let r_ccap: Symbol<unsafe extern "C" fn(C2Circle,C2Capsule)->c_int> = r.get(b"c2CircletoCapsule").unwrap();
        let cap = C2Capsule{a:C2v{x:0.0,y:0.0},b:C2v{x:10.0,y:0.0},r:2.0};
        let ch = C2Circle{p:C2v{x:5.0,y:2.0},r:2.0};
        let cm = C2Circle{p:C2v{x:5.0,y:20.0},r:2.0};
        assert_eq!(c_ccap(ch,cap), r_ccap(ch,cap), "CircletoCapsule hit");
        assert_eq!(c_ccap(cm,cap), r_ccap(cm,cap), "CircletoCapsule miss");

        // c2AABBtoCapsule
        let c_ac: Symbol<unsafe extern "C" fn(C2AABB,C2Capsule)->c_int> = c.get(b"c2AABBtoCapsule").unwrap();
        let r_ac: Symbol<unsafe extern "C" fn(C2AABB,C2Capsule)->c_int> = r.get(b"c2AABBtoCapsule").unwrap();
        assert_eq!(c_ac(aabb,cap), r_ac(aabb,cap), "AABBtoCapsule hit");
        let cap_far = C2Capsule{a:C2v{x:50.0,y:50.0},b:C2v{x:60.0,y:50.0},r:1.0};
        assert_eq!(c_ac(aabb,cap_far), r_ac(aabb,cap_far), "AABBtoCapsule miss");

        // c2CapsuletoCapsule
        let c_capcap: Symbol<unsafe extern "C" fn(C2Capsule,C2Capsule)->c_int> = c.get(b"c2CapsuletoCapsule").unwrap();
        let r_capcap: Symbol<unsafe extern "C" fn(C2Capsule,C2Capsule)->c_int> = r.get(b"c2CapsuletoCapsule").unwrap();
        let cap2 = C2Capsule{a:C2v{x:5.0,y:-5.0},b:C2v{x:5.0,y:5.0},r:2.0};
        assert_eq!(c_capcap(cap,cap2), r_capcap(cap,cap2), "CapsuletoCapsule hit");
        assert_eq!(c_capcap(cap,cap_far), r_capcap(cap,cap_far), "CapsuletoCapsule miss");
    }
}

#[test]
fn test_c2_collided() {
    let c = c_lib(); let r = rs_lib();
    unsafe {
        let c_col: Symbol<unsafe extern "C" fn(*const u8, c_int, *const u8, c_int)->c_int> = c.get(b"c2Collided").unwrap();
        let r_col: Symbol<unsafe extern "C" fn(*const u8, c_int, *const u8, c_int)->c_int> = r.get(b"c2Collided").unwrap();

        let circle = C2Circle{p:C2v{x:0.0,y:0.0},r:5.0};
        let aabb = C2AABB{min:C2v{x:-3.0,y:-3.0},max:C2v{x:3.0,y:3.0}};
        let cap = C2Capsule{a:C2v{x:-10.0,y:0.0},b:C2v{x:10.0,y:0.0},r:2.0};

        // All type combinations
        let shapes: [(*const u8, c_int); 3] = [
            (&circle as *const _ as *const u8, C2_TYPE_CIRCLE),
            (&aabb as *const _ as *const u8, C2_TYPE_AABB),
            (&cap as *const _ as *const u8, C2_TYPE_CAPSULE),
        ];
        for (pa, ta) in &shapes { for (pb, tb) in &shapes {
            let cv = c_col(*pa, *ta, *pb, *tb);
            let rv = r_col(*pa, *ta, *pb, *tb);
            assert_eq!(cv, rv, "c2Collided({ta},{tb})");
        }}
    }
}

#[test]
fn test_c2gjk() {
    let c = c_lib(); let r = rs_lib();
    unsafe {
        type GjkFn = unsafe extern "C" fn(*const u8,c_int,*const C2x,*const u8,c_int,*const C2x,*mut C2v,*mut C2v,c_int,*mut c_int,*mut C2GJKCache)->f32;
        let c_gjk: Symbol<GjkFn> = c.get(b"c2GJK").unwrap();
        let r_gjk: Symbol<GjkFn> = r.get(b"c2GJK").unwrap();

        let circle_a = C2Circle{p:C2v{x:0.0,y:0.0},r:5.0};
        let circle_b = C2Circle{p:C2v{x:15.0,y:0.0},r:3.0};
        let aabb = C2AABB{min:C2v{x:20.0,y:20.0},max:C2v{x:30.0,y:30.0}};
        let cap = C2Capsule{a:C2v{x:-5.0,y:10.0},b:C2v{x:5.0,y:10.0},r:2.0};

        let cases: Vec<(*const u8,c_int,*const u8,c_int,c_int)> = vec![
            (&circle_a as *const _ as _, C2_TYPE_CIRCLE, &circle_b as *const _ as _, C2_TYPE_CIRCLE, 1),
            (&circle_a as *const _ as _, C2_TYPE_CIRCLE, &aabb as *const _ as _, C2_TYPE_AABB, 0),
            (&aabb as *const _ as _, C2_TYPE_AABB, &cap as *const _ as _, C2_TYPE_CAPSULE, 1),
            (&circle_a as *const _ as _, C2_TYPE_CIRCLE, &cap as *const _ as _, C2_TYPE_CAPSULE, 1),
        ];

        for (i, (pa,ta,pb,tb,ur)) in cases.iter().enumerate() {
            let (mut ca, mut cb) = (C2v{x:0.0,y:0.0}, C2v{x:0.0,y:0.0});
            let (mut ra, mut rb) = (C2v{x:0.0,y:0.0}, C2v{x:0.0,y:0.0});
            let mut ci = 0i32; let mut ri = 0i32;
            let cd = c_gjk(*pa,*ta,std::ptr::null(),*pb,*tb,std::ptr::null(),&mut ca,&mut cb,*ur,&mut ci,std::ptr::null_mut());
            let rd = r_gjk(*pa,*ta,std::ptr::null(),*pb,*tb,std::ptr::null(),&mut ra,&mut rb,*ur,&mut ri,std::ptr::null_mut());
            assert_eq!(cd.to_bits(), rd.to_bits(), "c2GJK dist case {i}");
            assert_v_eq(ca, ra, &format!("c2GJK outA case {i}"));
            assert_v_eq(cb, rb, &format!("c2GJK outB case {i}"));
            assert_eq!(ci, ri, "c2GJK iterations case {i}");
        }
    }
}

#[test]
fn test_reverse_collide() {
    let c = c_lib(); let r = rs_lib();
    unsafe {
        let c_rc: Symbol<unsafe extern "C" fn(f32,f32,f32)->c_int> = c.get(b"reverse_collide").unwrap();
        let r_rc: Symbol<unsafe extern "C" fn(f32,f32,f32)->c_int> = r.get(b"reverse_collide").unwrap();

        let cases = [
            (0.0f32, 0.0f32, 1.0f32),    // near origin
            (-70.0, 0.0, 1.0),            // at circle center
            (-30.0, -30.0, 5.0),          // near AABB
            (-30.0, 70.0, 15.0),          // near capsule
            (100.0, 100.0, 1.0),          // far away
            (-50.0, 0.0, 25.0),           // overlapping circle
            (-27.5, -27.5, 20.0),         // overlapping AABB
            (-30.0, 50.0, 5.0),           // near capsule segment
            (0.0, 0.0, 100.0),            // huge radius
            (-40.0, 40.0, 1.0),           // at capsule endpoint a
            (-20.0, 100.0, 1.0),          // at capsule endpoint b
            (-15.0, -15.0, 0.1),          // corner of AABB
            (-70.0, 0.0, 20.0),           // exactly at circle boundary
            (0.0, 0.0, 0.0),             // zero radius
        ];
        for (x, y, rad) in cases {
            let cv = c_rc(x, y, rad);
            let rv = r_rc(x, y, rad);
            assert_eq!(cv, rv, "reverse_collide({x},{y},{rad}): C={cv} Rust={rv}");
        }
    }
}
