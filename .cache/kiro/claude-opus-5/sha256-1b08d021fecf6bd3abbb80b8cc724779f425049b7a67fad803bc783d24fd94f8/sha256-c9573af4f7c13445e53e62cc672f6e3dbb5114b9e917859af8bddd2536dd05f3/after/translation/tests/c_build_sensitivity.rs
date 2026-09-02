//! One-off sensitivity probe: compares the Rust `.so` against a C `.so` given
//! by the `ALT_C_SO` environment variable. Used to confirm the translation is
//! pinned to the *reference* C build (the one produced by the documented cmake
//! command) and to measure how much the NaN-payload and UB behaviour move when
//! the C is compiled differently.

mod common;
use common::*;
use std::ffi::c_void;

#[test]
fn alt_c_comparison() {
    let Ok(alt) = std::env::var("ALT_C_SO") else {
        println!("ALT_C_SO not set; skipping");
        return;
    };
    let lib = unsafe { libloading::Library::new(&alt) }.expect("dlopen ALT_C_SO");
    type FnVVf = unsafe extern "C" fn(c2v, c2v) -> f32;
    type FnCastRay =
        unsafe extern "C" fn(c2Ray, *const c_void, u32, *mut c2Raycast) -> std::ffi::c_int;
    let a_dot: FnVVf = unsafe { *lib.get(b"c2Dot").unwrap() };
    let a_cast: FnCastRay = unsafe { *lib.get(b"c2CastRay").unwrap() };

    let (c, r) = pair();
    let mut g = Rng::new(7);

    // --- c2Dot NaN-payload sensitivity ------------------------------------
    let mut n = 0usize;
    let mut ref_match = 0usize;
    let mut alt_match = 0usize;
    for _ in 0..400_000 {
        // force NaN pairs, which is the only place operand order is visible
        let mk = |g: &mut Rng| f32::from_bits(0x7FC0_0000 | (g.next_u32() & 0x3F_FFFF));
        let av = c2v { x: mk(&mut g), y: mk(&mut g) };
        let bv = c2v { x: mk(&mut g), y: mk(&mut g) };
        let cv = unsafe { (c.c2Dot)(av, bv) };
        let rv = unsafe { (r.c2Dot)(av, bv) };
        let av2 = unsafe { a_dot(av, bv) };
        n += 1;
        if cv.to_bits() == rv.to_bits() {
            ref_match += 1;
        }
        if av2.to_bits() == rv.to_bits() {
            alt_match += 1;
        }
    }
    println!("c2Dot NaN pairs: {n}");
    println!("  Rust matches REFERENCE C : {ref_match}/{n}");
    println!("  Rust matches ALT_C_SO    : {alt_match}/{n}");

    // --- c2CastRay UB edge sensitivity ------------------------------------
    let s = c2Circle { p: c2v { x: 5.0, y: 0.0 }, r: 1.0 };
    let mut ub_ref = 0usize;
    let mut ub_alt = 0usize;
    let mut ub_n = 0usize;
    for _ in 0..2000 {
        let eax = g.next_u32();
        let ray = c2Ray { p: g.v(20.0), d: g.dir(), t: g.mixed_f32(1e4) };
        let p = &s as *const c2Circle as *const c_void;
        let mut b1 = OutBuf::filled();
        let mut b2 = OutBuf::filled();
        let mut b3 = OutBuf::filled();
        let cv = unsafe { cast_ray_with_eax(c.c2CastRay, eax, ray, p, 3, b1.as_ptr()) };
        let rv = unsafe { cast_ray_with_eax(r.c2CastRay, eax, ray, p, 3, b2.as_ptr()) };
        let av = unsafe { cast_ray_with_eax(a_cast, eax, ray, p, 3, b3.as_ptr()) };
        ub_n += 1;
        if cv == rv {
            ub_ref += 1;
        }
        if av == rv {
            ub_alt += 1;
        }
    }
    println!("c2CastRay UB edge (controlled eax): {ub_n}");
    println!("  Rust matches REFERENCE C : {ub_ref}/{ub_n}");
    println!("  Rust matches ALT_C_SO    : {ub_alt}/{ub_n}");
}
