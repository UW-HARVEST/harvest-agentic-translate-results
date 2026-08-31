//! Area 1 — `randombytes/`: the dispatch layer, the deterministic DRBG, the
//! default `uniform` rejection sampler, and the two exported implementation
//! descriptor structs.
mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};

type BufFn = unsafe extern "C" fn(*mut c_void, usize);
type DetFn = unsafe extern "C" fn(*mut c_void, usize, *const u8);
type U32Fn = unsafe extern "C" fn() -> u32;
type UniformFn = unsafe extern "C" fn(u32) -> u32;
type SizeFn = unsafe extern "C" fn() -> usize;
type IntFn = unsafe extern "C" fn() -> c_int;
type NaClFn = unsafe extern "C" fn(*mut u8, u64);
type NameFn = unsafe extern "C" fn() -> *const c_char;

#[test]
fn seedbytes_matches() {
    let (c, r) = both::<SizeFn>("randombytes_seedbytes");
    unsafe {
        assert_eq!(c(), r());
        assert_eq!(c(), 32);
    }
}

#[test]
fn buf_all_lengths_use_the_installed_implementation() {
    let (c, r) = both::<BufFn>("randombytes_buf");
    for len in 0..=300usize {
        rng_reset();
        let mut a = padded(len);
        let mut b = padded(len);
        unsafe {
            c(a.as_mut_ptr() as *mut c_void, len);
            r(b.as_mut_ptr() as *mut c_void, len);
        }
        eqb(&format!("randombytes_buf({len})"), &a, &b);
        check_pad("randombytes_buf", &a, len);
        check_pad("randombytes_buf", &b, len);
    }
}

#[test]
fn buf_deterministic_all_lengths_and_seeds() {
    let (c, r) = both::<DetFn>("randombytes_buf_deterministic");
    let mut rng = Rng::new(0x201);
    let mut seeds: Vec<[u8; 32]> = vec![[0u8; 32], [0xffu8; 32]];
    for i in 0..32 {
        let mut s = [0u8; 32];
        s[i] = 1;
        seeds.push(s);
    }
    for _ in 0..8 {
        let mut s = [0u8; 32];
        rng.fill(&mut s);
        seeds.push(s);
    }
    for seed in &seeds {
        for len in [
            0usize, 1, 31, 32, 33, 63, 64, 65, 127, 128, 129, 191, 192, 255, 256, 257, 1000, 4096,
        ] {
            let mut a = padded(len);
            let mut b = padded(len);
            unsafe {
                c(a.as_mut_ptr() as *mut c_void, len, seed.as_ptr());
                r(b.as_mut_ptr() as *mut c_void, len, seed.as_ptr());
            }
            eqb(
                &format!("buf_deterministic(len={len}, seed={})", hex(seed)),
                &a,
                &b,
            );
            check_pad("buf_deterministic", &a, len);
            check_pad("buf_deterministic", &b, len);
        }
    }
}

#[test]
fn random_uses_the_installed_implementation() {
    let (c, r) = both::<U32Fn>("randombytes_random");
    rng_reset();
    for i in 0..500 {
        unsafe {
            assert_eq!(c(), r(), "randombytes_random call {i}");
        }
    }
}

#[test]
fn uniform_default_rejection_sampler() {
    // `uniform` is left NULL in the installed implementation, so the library's
    // own rejection-sampling loop is what runs.
    let (c, r) = both::<UniformFn>("randombytes_uniform");
    let mut bounds: Vec<u32> = vec![
        0,
        1,
        2,
        3,
        4,
        5,
        7,
        8,
        255,
        256,
        257,
        1000,
        0x7fff_ffff,
        0x8000_0000,
        0x8000_0001,
        0xffff_fffe,
        0xffff_ffff,
    ];
    let mut rng = Rng::new(0x202);
    for _ in 0..40 {
        bounds.push(rng.next_u32());
    }
    for ub in bounds {
        rng_reset();
        for i in 0..20 {
            unsafe {
                assert_eq!(c(ub), r(ub), "randombytes_uniform({ub}) call {i}");
            }
        }
    }
}

#[test]
fn stir_and_close_match() {
    let (cs, rs) = both::<unsafe extern "C" fn()>("randombytes_stir");
    let (cc, rc) = both::<IntFn>("randombytes_close");
    unsafe {
        cs();
        rs();
        eqi("randombytes_close", cc(), rc());
    }
}

#[test]
fn implementation_name_reports_the_installed_impl() {
    let (c, r) = both::<NameFn>("randombytes_implementation_name");
    unsafe {
        let a = std::ffi::CStr::from_ptr(c());
        let b = std::ffi::CStr::from_ptr(r());
        assert_eq!(a, b, "randombytes_implementation_name");
        assert_eq!(a.to_bytes(), b"difftest");
    }
}

#[test]
fn nacl_compat_entry_point() {
    let (c, r) = both::<NaClFn>("randombytes");
    for len in [0u64, 1, 7, 8, 64, 257] {
        rng_reset();
        let mut a = padded(len as usize);
        let mut b = padded(len as usize);
        unsafe {
            c(a.as_mut_ptr(), len);
            r(b.as_mut_ptr(), len);
        }
        eqb(&format!("randombytes({len})"), &a, &b);
        check_pad("randombytes", &a, len as usize);
    }
}

#[test]
fn exported_implementation_names_match() {
    // Reach the descriptors' implementation_name() callbacks without changing
    // the globally installed implementation.
    for sym in [
        "randombytes_sysrandom_implementation",
        "randombytes_internal_implementation",
    ] {
        let (c, r) = both::<*const c_void>(sym);
        unsafe {
            let cn = *(*c as *const NameFn);
            let rn = *(*r as *const NameFn);
            let a = std::ffi::CStr::from_ptr(cn());
            let b = std::ffi::CStr::from_ptr(rn());
            assert_eq!(a, b, "{sym}: implementation_name()");
        }
    }
}

#[test]
fn buf_deterministic_oversized_aborts() {
    // size > 0x4000000000ULL  =>  sodium_misuse()
    let (c, r) = both::<DetFn>("randombytes_buf_deterministic");
    for size in [0x4000_0000_001u64, u64::MAX / 2] {
        let cc = c.clone();
        let rr = r.clone();
        eq_abort(
            &format!("buf_deterministic(size={size})"),
            move || unsafe {
                let seed = [7u8; 32];
                // The pointer is never dereferenced before the size check.
                cc(std::ptr::null_mut(), size as usize, seed.as_ptr());
            },
            move || unsafe {
                let seed = [7u8; 32];
                rr(std::ptr::null_mut(), size as usize, seed.as_ptr());
            },
        );
    }
}
