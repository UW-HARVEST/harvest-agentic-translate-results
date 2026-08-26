//! Phase D — symbol parity, plus a targeted search for `ERRORS.md` row 27
//! (the `while (iter < 20)` cap).
#![allow(non_snake_case)]
#![allow(clippy::unnecessary_cast, clippy::needless_range_loop, clippy::let_and_return)]
#![allow(clippy::field_reassign_with_default)]

mod common;
use common::*;
use std::ffi::c_void;
use std::process::Command;

/// The 46 dynamic symbols the C `.so` exports, as read by `nm -D`.
/// Regenerated mechanically below; this list is only a tripwire for the count.
const EXPECTED_COUNT: usize = 46;

fn nm_defined(path: &str) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", path])
        .output()
        .expect("failed to run nm -- is binutils installed?");
    assert!(out.status.success(), "nm failed on {path}: {:?}", String::from_utf8_lossy(&out.stderr));
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(str::to_string))
        .collect();
    v.sort();
    v.dedup();
    v
}

fn nm_undefined(path: &str) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", path])
        .output()
        .expect("failed to run nm");
    assert!(out.status.success());
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(1).map(str::to_string))
        .collect();
    v.sort();
    v.dedup();
    v
}

fn c_so() -> String {
    format!("{}/c_src/build/libtranslated_rust.so", env!("CARGO_MANIFEST_DIR"))
}
fn rust_so() -> String {
    if let Ok(p) = std::env::var("OMNI_RUST_SO") {
        return p;
    }
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    format!("{}/target/{profile}/libomni_manifold_lib.so", env!("CARGO_MANIFEST_DIR"))
}

/// Every symbol the C `.so` exports must also be exported by the Rust `.so`, with the
/// exact same name. The diff must be empty.
#[test]
fn symbol_diff_is_empty() {
    let c = nm_defined(&c_so());
    let r = nm_defined(&rust_so());

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    println!("C exports {} symbols, Rust exports {}", c.len(), r.len());
    assert!(
        missing.is_empty(),
        "{} C symbol(s) missing from the Rust .so: {missing:?}",
        missing.len()
    );
    assert_eq!(c.len(), EXPECTED_COUNT, "the C .so's export count changed: {c:?}");

    // Every C symbol must also be resolvable through dlsym, not merely present in
    // the table -- that is what an external caller actually does.
    let l = libs();
    for name in c.iter() {
        let _ = l.get::<unsafe extern "C" fn()>(name);
    }
}

/// The five `static` helpers must NOT be exported by either library.
#[test]
fn static_helpers_are_not_exported() {
    let c = nm_defined(&c_so());
    let r = nm_defined(&rust_so());
    for name in ["c2Clip", "c2SidePlanes", "c2SidePlanesFromPoly", "c2KeepDeep", "c2Incident"] {
        assert!(!c.iter().any(|s| s == name), "C unexpectedly exports {name}");
        assert!(!r.iter().any(|s| s == name), "Rust unexpectedly exports {name}");
    }
}

/// The Rust `.so` must not have unresolved non-libc dependencies.
#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let u = nm_undefined(&rust_so());
    let allowed_prefixes = [
        "_ITM_", "_Unwind_", "__cxa_", "__gmon_start__", "__tls_get_addr", "__errno_location",
        "__libc_", "_dl_",
    ];
    let allowed_exact = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat64", "getcwd",
        "getenv", "gettid", "lseek64", "malloc", "memcmp", "memcpy", "memmove", "memset",
        "mmap64", "munmap", "open64", "posix_memalign", "pthread_key_create",
        "pthread_key_delete", "pthread_getspecific", "pthread_setspecific", "read", "readlink",
        "realloc", "realpath", "sqrtf", "stat64", "statx", "strlen", "syscall", "write",
        "writev", "sysconf", "getrandom", "poll", "pipe2", "sigaction", "sigaltstack",
        "mprotect", "pthread_self", "pthread_mutex_lock", "pthread_mutex_unlock",
        "pthread_mutex_trylock", "pthread_mutex_destroy", "pthread_condattr_init",
        "pthread_cond_wait", "pthread_cond_signal", "pthread_cond_destroy", "pthread_rwlock_rdlock",
        "pthread_rwlock_unlock", "pthread_rwlock_wrlock", "nanosleep", "clock_gettime",
        "sched_yield", "environ", "__environ",
    ];
    let mut bad = Vec::new();
    for s in u.iter() {
        let base = s.split('@').next().unwrap();
        if allowed_prefixes.iter().any(|p| base.starts_with(p)) {
            continue;
        }
        if allowed_exact.contains(&base) {
            continue;
        }
        bad.push(s.clone());
    }
    assert!(bad.is_empty(), "Rust .so has unexpected undefined symbols: {bad:?}");
}

// ---------------------------------------------------------------------------
// ERRORS.md row 27: how close can `*iterations` get to the `while (iter < 20)` cap?
// ---------------------------------------------------------------------------

/// Hunts for the largest `*iterations` the solver will report, over every type pair,
/// with and without transforms and with hand-built caches that seed arbitrary simplex
/// states. Whatever the maximum turns out to be, the C and Rust values are compared on
/// every single call, so the cap's behaviour is verified even if the cap itself is
/// never reached.
#[test]
fn row27_iteration_cap_search() {
    let l = libs();
    let (cf, rf) = l.get::<FnGJK>("c2GJK");
    let mut rng = Rng::new(27);
    let mut max_iter = 0i32;
    let mut hist = std::collections::BTreeMap::new();

    for &ta in VALID_TYPES.iter() {
        for &tb in VALID_TYPES.iter() {
            for i in 0..12_000 {
                let bb1 = c2AABB { min: rng.vec_mixed(8.0), max: rng.vec_mixed(8.0) };
                let bb2 = c2AABB { min: rng.vec_mixed(8.0), max: rng.vec_mixed(8.0) };
                let cap1 = c2Capsule { a: rng.vec_mixed(8.0), b: rng.vec_mixed(8.0), r: rng.f_mixed(2.0) };
                let cap2 = c2Capsule { a: rng.vec_mixed(8.0), b: rng.vec_mixed(8.0), r: rng.f_mixed(2.0) };
                let ci1 = c2Circle { p: rng.vec_mixed(8.0), r: rng.f_mixed(2.0) };
                let ci2 = c2Circle { p: rng.vec_mixed(8.0), r: rng.f_mixed(2.0) };
                let (pa, na) = match ta {
                    C2_TYPE_CIRCLE => (&ci1 as *const _ as *const c_void, 1u32),
                    C2_TYPE_AABB => (&bb1 as *const _ as *const c_void, 4),
                    _ => (&cap1 as *const _ as *const c_void, 2),
                };
                let (pb, nb) = match tb {
                    C2_TYPE_CIRCLE => (&ci2 as *const _ as *const c_void, 1u32),
                    C2_TYPE_AABB => (&bb2 as *const _ as *const c_void, 4),
                    _ => (&cap2 as *const _ as *const c_void, 2),
                };
                let xa = rng.xform(10.0);
                let xb = rng.xform(10.0);
                let (oax, obx) = match i % 3 {
                    0 => (std::ptr::null(), std::ptr::null()),
                    1 => (&xa as *const c2x, std::ptr::null()),
                    _ => (&xa as *const c2x, &xb as *const c2x),
                };
                // Seed the simplex from a hand-built cache half the time.
                let use_cache = i % 2 == 0;
                let cache = c2GJKCache {
                    metric: rng.f_mixed(50.0),
                    count: 1 + (rng.below(3) as i32),
                    iA: [rng.below(na) as i32, rng.below(na) as i32, rng.below(na) as i32],
                    iB: [rng.below(nb) as i32, rng.below(nb) as i32, rng.below(nb) as i32],
                    div: rng.f_mixed(10.0),
                };
                let mut cc = cache;
                let mut rc = cache;
                let (mut c_a, mut c_b, mut c_i) = (c2v::default(), c2v::default(), -1i32);
                let (mut r_a, mut r_b, mut r_i) = (c2v::default(), c2v::default(), -1i32);
                zero_stack();
                let cd = unsafe {
                    cf(pa, ta, oax, pb, tb, obx, &mut c_a, &mut c_b, (i % 2) as i32, &mut c_i,
                       if use_cache { &mut cc } else { std::ptr::null_mut() })
                };
                zero_stack();
                let rd = unsafe {
                    rf(pa, ta, oax, pb, tb, obx, &mut r_a, &mut r_b, (i % 2) as i32, &mut r_i,
                       if use_cache { &mut rc } else { std::ptr::null_mut() })
                };
                let ctx = format!("ta={ta} tb={tb} i={i} cache={use_cache}");
                eq_f32("c2GJK dist", &ctx, cd, rd);
                eq("c2GJK outA", &ctx, &c_a, &r_a);
                eq("c2GJK outB", &ctx, &c_b, &r_b);
                eq_i32("c2GJK iterations", &ctx, c_i, r_i);
                if use_cache {
                    eq("c2GJK cache", &ctx, &cc, &rc);
                }
                max_iter = max_iter.max(c_i);
                *hist.entry(c_i).or_insert(0u64) += 1;
            }
        }
    }
    println!("row27 iteration histogram: {hist:?}");
    println!("row27 maximum *iterations observed: {max_iter} (hard cap in C is 20)");
    assert!(max_iter >= 1, "the solver never iterated at all -- the search is not exercising it");
    assert!(max_iter <= 20, "iterations exceeded the C hard cap of 20");
}
