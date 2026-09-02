//! Differential tests for `randombytes/`.
//!
//! Loads BOTH shared objects through the harness and drives the `extern "C"`
//! exports as an external C consumer would. The C at `c_src/libsodium/` is the
//! ground truth.
//!
//! Facts confirmed against the source (randombytes/randombytes.c and the two
//! backend implementations):
//!   * `randombytes_buf_deterministic` is a seeded ChaCha20 keystream
//!     (fixed nonce "LibsodiumDRG"), so its output is fully deterministic and
//!     must match bit-for-bit.
//!   * `randombytes_buf`/`_random`/`_uniform`/`_stir` draw from the OS CSPRNG
//!     and are non-deterministic; only structural invariants can be pinned.
//!   * Both the `sysrandom` and `internal` backends leave `.uniform == NULL`,
//!     so `randombytes_uniform` uses the generic path which returns 0 for any
//!     `upper_bound < 2` WITHOUT consuming randomness. Hence uniform(0) and
//!     uniform(1) are a deterministic 0 in both libraries.
//!   * `randombytes_set_implementation` just stores the pointer; the two
//!     backends are exported as the globals `randombytes_sysrandom_implementation`
//!     (the default) and `randombytes_internal_implementation`. Each library's
//!     global must be installed into THAT SAME library.

mod harness;
use harness::*;

use std::ffi::{c_int, CStr};
use std::os::raw::c_char;
use std::sync::{Mutex, MutexGuard, OnceLock};

const SEED: u64 = 0x5EED_0012;

/// The live randombytes backend keeps *process-global mutable state* (the open
/// urandom fd, the installed implementation, init flags) that is SHARED by all
/// tests in this process because they dlopen the same two `.so`s. `cargo test`
/// runs `#[test]`s on concurrent threads, so any test that observes or mutates
/// that state (set_implementation / stir / close / buf / random / uniform)
/// must run mutually exclusive with the others, or an interleaving from
/// another thread perturbs the C and Rust backends into different states and
/// the differential comparison sees a spurious mismatch. `buf_deterministic`
/// is exempt: it is a pure seeded ChaCha20 keystream with no shared state.
fn state_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|p| p.into_inner())
}

// ---------------------------------------------------------------------------
// (a) randombytes_buf_deterministic — DETERMINISTIC, compare bytes exactly.
// ---------------------------------------------------------------------------

type BufDet = unsafe extern "C" fn(*mut u8, usize, *const u8);

#[test]
fn buf_deterministic_exact() {
    // Seed length is 32.
    let (csb, rsb) = sym::<unsafe extern "C" fn() -> usize>("randombytes_seedbytes");
    let (csv, rsv) = unsafe { (csb(), rsb()) };
    assert_eq!(csv, rsv, "randombytes_seedbytes");
    assert_eq!(csv, 32, "randombytes_seedbytes == 32");

    let (c, r) = sym::<BufDet>("randombytes_buf_deterministic");
    let mut rng = Rng::new(SEED);

    let lens = [0usize, 1, 32, 63, 64, 65, 128, 4096];
    // A spread of fixed and random 32-byte seeds.
    let mut seeds: Vec<Vec<u8>> = vec![vec![0u8; 32], vec![0xffu8; 32]];
    for _ in 0..12 {
        seeds.push(rng.bytes(32));
    }

    for len in lens {
        for (si, seed) in seeds.iter().enumerate() {
            let mut oc = out_buf(len);
            let mut or = out_buf(len);
            unsafe {
                c(oc.as_mut_ptr(), len, seed.as_ptr());
                r(or.as_mut_ptr(), len, seed.as_ptr());
            }
            eqb(&format!("buf_deterministic len={len} seed#{si}"), &oc, &or);
        }
    }

    // Determinism sanity: same seed twice yields identical output in C.
    for len in [1usize, 64, 100] {
        let seed = rng.bytes(32);
        let mut a = out_buf(len);
        let mut b = out_buf(len);
        unsafe {
            c(a.as_mut_ptr(), len, seed.as_ptr());
            c(b.as_mut_ptr(), len, seed.as_ptr());
        }
        eqb(&format!("buf_deterministic repeatable len={len}"), &a, &b);
    }
}

// ---------------------------------------------------------------------------
// (b) non-deterministic APIs — structural invariants only.
// ---------------------------------------------------------------------------

#[test]
fn buf_non_deterministic_canary_intact() {
    let _g = state_lock();
    let (c, r) = sym::<unsafe extern "C" fn(*mut u8, usize)>("randombytes_buf");
    for len in [0usize, 1, 16, 32, 64, 256, 1000] {
        let mut bc = out_buf(len);
        let mut br = out_buf(len);
        unsafe {
            c(bc.as_mut_ptr(), len);
            r(br.as_mut_ptr(), len);
        }
        // Canary (bytes past `len`) must be untouched by both.
        let fresh = out_buf(len);
        eqb(&format!("randombytes_buf C canary len={len}"), &bc[len..], &fresh[len..]);
        eqb(&format!("randombytes_buf Rust canary len={len}"), &br[len..], &fresh[len..]);
    }
    // With a large buffer the payload should not remain all-zero (probabilistic
    // but astronomically safe for 256 bytes).
    let big = 256usize;
    let mut bc = out_buf(big);
    let mut br = out_buf(big);
    unsafe {
        c(bc.as_mut_ptr(), big);
        r(br.as_mut_ptr(), big);
    }
    assert_ne!(&bc[..big], &vec![0u8; big][..], "C randombytes_buf all zeros");
    assert_ne!(&br[..big], &vec![0u8; big][..], "Rust randombytes_buf all zeros");
}

#[test]
fn random_returns_u32() {
    let _g = state_lock();
    // randombytes_random just returns a u32; call both a few times and make
    // sure they don't trivially return a constant.
    let (c, r) = sym::<unsafe extern "C" fn() -> u32>("randombytes_random");
    let mut cs = std::collections::HashSet::new();
    let mut rs = std::collections::HashSet::new();
    unsafe {
        for _ in 0..64 {
            cs.insert(c());
            rs.insert(r());
        }
    }
    assert!(cs.len() > 1, "C randombytes_random looks constant");
    assert!(rs.len() > 1, "Rust randombytes_random looks constant");
}

#[test]
fn uniform_bounds_and_small_bound_zero() {
    let _g = state_lock();
    let (c, r) = sym::<unsafe extern "C" fn(u32) -> u32>("randombytes_uniform");

    // Verified against the source: for upper_bound < 2 the generic path (used
    // by both backends, whose `.uniform` is NULL) returns 0 without consuming
    // randomness. Both libraries must therefore return 0 for 0 and 1.
    unsafe {
        assert_eq!(c(0), 0, "C uniform(0)");
        assert_eq!(r(0), 0, "Rust uniform(0)");
        assert_eq!(c(1), 0, "C uniform(1)");
        assert_eq!(r(1), 0, "Rust uniform(1)");
    }

    // For bound >= 1, the result must be < bound (many draws each).
    for &bound in &[1u32, 2, 3, 7, 16, 17, 100, 255, 256, 1000, 0x7fff_ffff, 0x8000_0000, u32::MAX] {
        for _ in 0..200 {
            unsafe {
                let cv = c(bound);
                let rv = r(bound);
                assert!(cv < bound, "C uniform({bound}) = {cv} not < bound");
                assert!(rv < bound, "Rust uniform({bound}) = {rv} not < bound");
            }
        }
    }
}

#[test]
fn stir_does_not_crash() {
    let _g = state_lock();
    let (c, r) = sym::<unsafe extern "C" fn()>("randombytes_stir");
    unsafe {
        c();
        r();
    }
    // After stir, buf must still work and leave the canary intact.
    let (cb, rb) = sym::<unsafe extern "C" fn(*mut u8, usize)>("randombytes_buf");
    let mut bc = out_buf(64);
    let mut br = out_buf(64);
    unsafe {
        cb(bc.as_mut_ptr(), 64);
        rb(br.as_mut_ptr(), 64);
    }
    let fresh = out_buf(64);
    eqb("post-stir C canary", &bc[64..], &fresh[64..]);
    eqb("post-stir Rust canary", &br[64..], &fresh[64..]);
}

// ---------------------------------------------------------------------------
// (c) randombytes_set_implementation — install the two backends and compare
//     the reported name / seedbytes; NEVER cross the C global into Rust.
// ---------------------------------------------------------------------------

/// Read the ADDRESS of an exported data global (a `randombytes_implementation`
/// struct) from EACH library separately. `libloading` resolves the symbol's
/// address; dereferencing the `Symbol<*const ()>` reinterprets that stored
/// address as the value, so it yields the address of the global struct itself.
/// That pointer is fed straight back into the SAME library's
/// `randombytes_set_implementation`, so the two libraries never share a
/// backend struct.
fn impl_globals(name: &str) -> (*const u8, *const u8) {
    let l = libs();
    let mut n = name.as_bytes().to_vec();
    n.push(0);
    unsafe {
        let cs: libloading::Symbol<*const ()> = l
            .c
            .get(&n)
            .unwrap_or_else(|e| panic!("C .so missing `{name}`: {e}"));
        let rs: libloading::Symbol<*const ()> = l
            .r
            .get(&n)
            .unwrap_or_else(|e| panic!("Rust .so missing `{name}`: {e}"));
        // libloading resolves the symbol's address and, on deref, reinterprets
        // that stored address AS the requested type. With `T = *const ()` the
        // dereferenced value therefore IS the address of the global struct.
        (*cs as *const u8, *rs as *const u8)
    }
}

fn c_impl_name(func: unsafe extern "C" fn() -> *const c_char) -> String {
    unsafe {
        let p = func();
        assert!(!p.is_null(), "implementation_name returned NULL");
        CStr::from_ptr(p).to_string_lossy().into_owned()
    }
}

#[test]
fn set_implementation_internal_then_sysrandom() {
    let _g = state_lock();
    type SetImpl = unsafe extern "C" fn(*const u8) -> c_int;
    type ImplName = unsafe extern "C" fn() -> *const c_char;
    type Buf = unsafe extern "C" fn(*mut u8, usize);
    type Seedbytes = unsafe extern "C" fn() -> usize;

    let (c_set, r_set) = sym::<SetImpl>("randombytes_set_implementation");
    let (c_name, r_name) = sym::<ImplName>("randombytes_implementation_name");
    let (c_buf, r_buf) = sym::<Buf>("randombytes_buf");
    let (c_seed, r_seed) = sym::<Seedbytes>("randombytes_seedbytes");

    // The two backend globals, each read from its OWN library.
    let (c_internal, r_internal) = impl_globals("randombytes_internal_implementation");
    let (c_sys, r_sys) = impl_globals("randombytes_sysrandom_implementation");

    // Helper: install `cg`/`rg` into C/Rust respectively, then compare.
    let check = |label: &str, cg: *const u8, rg: *const u8, want: &str| {
        unsafe {
            assert_eq!(c_set(cg), 0, "C set_implementation({label})");
            assert_eq!(r_set(rg), 0, "Rust set_implementation({label})");
        }
        let cn = c_impl_name(c_name);
        let rn = c_impl_name(r_name);
        assert_eq!(cn, rn, "implementation_name after {label}: C={cn} Rust={rn}");
        assert_eq!(cn, want, "implementation_name after {label} should be {want}");

        // seedbytes is a compile-time constant, unaffected by the backend, but
        // must agree.
        unsafe {
            assert_eq!(c_seed(), r_seed(), "seedbytes after {label}");
            assert_eq!(c_seed(), 32, "seedbytes after {label} == 32");
        }

        // buf still works and leaves the canary intact.
        let mut bc = out_buf(48);
        let mut br = out_buf(48);
        unsafe {
            c_buf(bc.as_mut_ptr(), 48);
            r_buf(br.as_mut_ptr(), 48);
        }
        let fresh = out_buf(48);
        eqb(&format!("{label} C buf canary"), &bc[48..], &fresh[48..]);
        eqb(&format!("{label} Rust buf canary"), &br[48..], &fresh[48..]);
    };

    // internal backend.
    check("internal", c_internal, r_internal, "internal");
    // sysrandom backend.
    check("sysrandom", c_sys, r_sys, "sysrandom");

    // Restore the default (sysrandom) at the end.
    unsafe {
        assert_eq!(c_set(c_sys), 0, "C restore default");
        assert_eq!(r_set(r_sys), 0, "Rust restore default");
    }
    let cn = c_impl_name(c_name);
    let rn = c_impl_name(r_name);
    assert_eq!(cn, rn, "restored implementation_name");
    assert_eq!(cn, "sysrandom", "restored default is sysrandom");
}

// ---------------------------------------------------------------------------
// (d) randombytes_close / randombytes_stir ordering, comparing return codes.
// ---------------------------------------------------------------------------

#[test]
fn close_and_stir_ordering() {
    let _g = state_lock();
    type Close = unsafe extern "C" fn() -> c_int;
    type Stir = unsafe extern "C" fn();
    type Buf = unsafe extern "C" fn(*mut u8, usize);
    type SetImpl = unsafe extern "C" fn(*const u8) -> c_int;

    let (c_close, r_close) = sym::<Close>("randombytes_close");
    let (c_stir, r_stir) = sym::<Stir>("randombytes_stir");
    let (c_buf, r_buf) = sym::<Buf>("randombytes_buf");
    let (c_set, r_set) = sym::<SetImpl>("randombytes_set_implementation");
    let (c_sys, r_sys) = impl_globals("randombytes_sysrandom_implementation");

    // Drive both libraries into an identical, known state first: install the
    // default sysrandom backend and force one buf so the backend initialises
    // (opens its fd / probes getrandom). `randombytes_close`'s return value is
    // a function of that mutable state, so from a matched starting point the
    // faithful C and Rust translations must return the SAME code at every step
    // of the sequence below — which is exactly what we assert (C vs Rust, not
    // against a hard-coded constant).
    unsafe {
        assert_eq!(c_set(c_sys), 0, "C install sysrandom");
        assert_eq!(r_set(r_sys), 0, "Rust install sysrandom");
        let mut warm = [0u8; 8];
        c_buf(warm.as_mut_ptr(), warm.len());
        r_buf(warm.as_mut_ptr(), warm.len());
    }

    // close, then stir, then buf, then close again — compare each close rc.
    unsafe {
        let c1 = c_close();
        let r1 = r_close();
        assert_eq!(c1, r1, "close #1 rc (C={c1} Rust={r1})");

        c_stir();
        r_stir();

        // buf must still function after a close+stir cycle.
        let mut bc = out_buf(32);
        let mut br = out_buf(32);
        c_buf(bc.as_mut_ptr(), 32);
        r_buf(br.as_mut_ptr(), 32);
        let fresh = out_buf(32);
        eqb("post close+stir C canary", &bc[32..], &fresh[32..]);
        eqb("post close+stir Rust canary", &br[32..], &fresh[32..]);

        let c2 = c_close();
        let r2 = r_close();
        assert_eq!(c2, r2, "close #2 rc (C={c2} Rust={r2})");

        // stir after close, then a final close.
        c_stir();
        r_stir();
        let c3 = c_close();
        let r3 = r_close();
        assert_eq!(c3, r3, "close #3 rc (C={c3} Rust={r3})");
    }

    // Leave both backends warmed and on the default so sibling tests see a
    // consistent starting point.
    unsafe {
        let mut warm = [0u8; 8];
        c_buf(warm.as_mut_ptr(), warm.len());
        r_buf(warm.as_mut_ptr(), warm.len());
    }
}
