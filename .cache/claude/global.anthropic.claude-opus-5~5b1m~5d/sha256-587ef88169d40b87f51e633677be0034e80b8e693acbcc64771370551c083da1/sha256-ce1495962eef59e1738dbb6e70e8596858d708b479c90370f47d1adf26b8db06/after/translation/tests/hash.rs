//! Differential tests for the `hash` area:
//!
//!   * `crypto_hash/crypto_hash.c`
//!   * `crypto_hash/sha256/{hash_sha256.c,cp/hash_sha256_cp.c}`
//!   * `crypto_hash/sha512/{hash_sha512.c,cp/hash_sha512_cp.c}`
//!   * `crypto_hash/sha3/hash_sha3.c`
//!   * `crypto_core/keccak1600/{keccak1600.c,ref/keccak1600_ref.c}`
//!   * `crypto_xof/{shake128,shake256,turboshake128,turboshake256}/**`
//!
//! Every exported symbol of those object files (90 of them, all functions —
//! `nm` shows no exported data objects for this area, so `both_data!` is not
//! applicable here) is exercised at least once.
//!
//! Note on state comparison: `crypto_hash_sha256_state` / `_sha512_state` are
//! plain, hole-free structs, so the FULL state buffer is compared byte for
//! byte.  The sha3 / xof / keccak states are `unsigned char opaque[N]` blobs
//! onto which a smaller internal struct is cast; the bytes past the last
//! written field are never touched by either implementation, so those buffers
//! are canary-filled and compared in full as well (a divergence in the tail
//! would therefore also be caught as an out-of-bounds write).

#![allow(clippy::too_many_arguments)]

#[macro_use]
mod common;

use core::ffi::{c_char, c_int};

// ---------------------------------------------------------------- types ------

type SizeFn = unsafe extern "C" fn() -> usize;
type U8Fn = unsafe extern "C" fn() -> u8;
type StrFn = unsafe extern "C" fn() -> *const c_char;

/// `int f(unsigned char *out, const unsigned char *in, unsigned long long inlen)`
type HashFn = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
/// `int f(state *)`
type InitFn = unsafe extern "C" fn(*mut u8) -> c_int;
/// `int f(state *, const unsigned char *, unsigned long long)`
type UpdFn = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
/// `int f(state *, unsigned char *out)`
type FinFn = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;

/// `int f(unsigned char *out, size_t outlen, const unsigned char *in, unsigned long long inlen)`
type XofFn = unsafe extern "C" fn(*mut u8, usize, *const u8, u64) -> c_int;
/// `int f(state *, unsigned char domain)`
type InitDomFn = unsafe extern "C" fn(*mut u8, u8) -> c_int;
/// `int f(state *, unsigned char *out, size_t outlen)`
type SqueezeFn = unsafe extern "C" fn(*mut u8, *mut u8, usize) -> c_int;

/// internal `_ref` one-shot: `int f(out, size_t outlen, in, size_t inlen)`
type RefXofFn = unsafe extern "C" fn(*mut u8, usize, *const u8, usize) -> c_int;
/// internal `_ref_update`: `int f(state *, const unsigned char *, size_t)`
type RefUpdFn = unsafe extern "C" fn(*mut u8, *const u8, usize) -> c_int;

/// keccak1600 public wrappers
type KecInitFn = unsafe extern "C" fn(*mut u8);
type KecXorFn = unsafe extern "C" fn(*mut u8, *const u8, usize, usize);
type KecExtFn = unsafe extern "C" fn(*const u8, *mut u8, usize, usize);
type KecPermFn = unsafe extern "C" fn(*mut u8);
/// keccak1600 internal ref (`void *state`)
type RefKecInitFn = unsafe extern "C" fn(*mut u8);
type RefKecXorFn = unsafe extern "C" fn(*mut u8, *const u8, usize, usize);
type RefKecExtFn = unsafe extern "C" fn(*const u8, *mut u8, usize, usize);
type RefKecPermFn = unsafe extern "C" fn(*mut u8);

// ------------------------------------------------------------- buffers -------

/// 16-byte-aligned scratch for any opaque state in this area (largest is 256).
#[repr(C, align(16))]
#[derive(Clone, Copy)]
struct Sbuf([u8; 384]);

impl Sbuf {
    fn canary() -> Self {
        Sbuf([0xA5u8; 384])
    }
    fn p(&mut self) -> *mut u8 {
        self.0.as_mut_ptr()
    }
    fn cp(&self) -> *const u8 {
        self.0.as_ptr()
    }
}

/// Output buffer with a trailing canary guard so over-writes are detected.
fn guarded(len: usize) -> Vec<u8> {
    vec![0x5Au8; len + 32]
}

/// `both!` needs a literal symbol name; this is the run-time-name equivalent.
fn pair<T: Copy>(name: &str) -> (T, T) {
    let l = common::libs();
    let mut n = name.as_bytes().to_vec();
    n.push(0);
    unsafe {
        let c: libloading::Symbol<T> = l
            .c
            .get(&n)
            .unwrap_or_else(|e| panic!("C missing {name}: {e}"));
        let r: libloading::Symbol<T> = l
            .r
            .get(&n)
            .unwrap_or_else(|e| panic!("Rust missing {name}: {e}"));
        (*c, *r)
    }
}

fn cmp_states(ctx: &str, c: &Sbuf, r: &Sbuf) {
    common::eqb(&format!("{ctx} [state]"), &c.0, &r.0);
}

// ------------------------------------------------------------ size list ------

/// Sizes that straddle every internal boundary used in this area:
/// 64/56 (sha256), 128/112 (sha512), 136 (sha3-256 & shake256 rate),
/// 72 (sha3-512 rate), 168 (shake128 / turboshake128 rate).
const SIZES: &[usize] = &[
    0, 1, 2, 3, 7, 8, 9, 15, 16, 31, 32, 55, 56, 57, 63, 64, 65, 71, 72, 73, 111, 112, 113, 127,
    128, 129, 135, 136, 137, 143, 144, 167, 168, 169, 191, 192, 200, 255, 256, 271, 272, 335, 336,
    337, 1000, 4096,
];

// =============================================================== getters =====

#[test]
fn getters() {
    let cases_size: &[(&str, usize)] = &[
        ("crypto_hash_bytes", 64),
        ("crypto_hash_sha256_bytes", 32),
        ("crypto_hash_sha256_statebytes", 104),
        ("crypto_hash_sha512_bytes", 64),
        ("crypto_hash_sha512_statebytes", 208),
        ("crypto_hash_sha3256_bytes", 32),
        ("crypto_hash_sha3256_statebytes", 256),
        ("crypto_hash_sha3512_bytes", 64),
        ("crypto_hash_sha3512_statebytes", 256),
        ("crypto_core_keccak1600_statebytes", 224),
        ("crypto_xof_shake128_blockbytes", 168),
        ("crypto_xof_shake128_statebytes", 256),
        ("crypto_xof_shake256_blockbytes", 136),
        ("crypto_xof_shake256_statebytes", 256),
        ("crypto_xof_turboshake128_blockbytes", 168),
        ("crypto_xof_turboshake128_statebytes", 256),
        ("crypto_xof_turboshake256_blockbytes", 136),
        ("crypto_xof_turboshake256_statebytes", 256),
    ];
    for (name, expect) in cases_size {
        let (c, r) = pair::<SizeFn>(name);
        let (cv, rv) = unsafe { (c(), r()) };
        assert_eq!(cv, rv, "{name}: C={cv} Rust={rv}");
        assert_eq!(cv, *expect, "{name}: C returned {cv}, header says {expect}");
    }

    for name in [
        "crypto_xof_shake128_domain_standard",
        "crypto_xof_shake256_domain_standard",
        "crypto_xof_turboshake128_domain_standard",
        "crypto_xof_turboshake256_domain_standard",
    ] {
        let (c, r) = pair::<U8Fn>(name);
        let (cv, rv) = unsafe { (c(), r()) };
        assert_eq!(cv, rv, "{name}: C={cv} Rust={rv}");
        assert_eq!(cv, 0x1F, "{name}: expected 0x1F, got {cv:#x}");
    }

    // crypto_hash_primitive() -> "sha512"
    let (c, r) = both!("crypto_hash_primitive", StrFn);
    unsafe {
        let (cp, rp) = (c(), r());
        assert!(!cp.is_null() && !rp.is_null(), "primitive returned NULL");
        let cs = std::ffi::CStr::from_ptr(cp).to_bytes().to_vec();
        let rs = std::ffi::CStr::from_ptr(rp).to_bytes().to_vec();
        common::eqb("crypto_hash_primitive", &cs, &rs);
        assert_eq!(&cs[..], b"sha512");
    }
}

// ============================================================ crypto_hash ====

#[test]
fn crypto_hash_wrapper() {
    let (ch_c, ch_r) = both!("crypto_hash", HashFn);
    let (s5_c, s5_r) = both!("crypto_hash_sha512", HashFn);
    let mut rng = common::Rng::new(0x0001);

    // NULL input with length 0 is tolerated by the C (update() early-returns).
    unsafe {
        let mut co = guarded(64);
        let mut ro = guarded(64);
        let rc = ch_c(co.as_mut_ptr(), core::ptr::null(), 0);
        let rr = ch_r(ro.as_mut_ptr(), core::ptr::null(), 0);
        common::eqi("crypto_hash(in=NULL,0)", rc, rr);
        assert_eq!(rc, 0);
        common::eqb("crypto_hash(in=NULL,0)", &co, &ro);
    }

    for &n in SIZES {
        for trial in 0..3 {
            let msg = rng.bytes(n);
            let mut co = guarded(64);
            let mut ro = guarded(64);
            let mut so = guarded(64);
            unsafe {
                let rc = ch_c(co.as_mut_ptr(), msg.as_ptr(), n as u64);
                let rr = ch_r(ro.as_mut_ptr(), msg.as_ptr(), n as u64);
                common::eqi(&format!("crypto_hash n={n}"), rc, rr);
                assert_eq!(rc, 0);
                common::eqb(&format!("crypto_hash n={n} t={trial}"), &co, &ro);
                // crypto_hash == crypto_hash_sha512 (C), and Rust agrees.
                s5_c(so.as_mut_ptr(), msg.as_ptr(), n as u64);
                common::eqb(&format!("crypto_hash==sha512 n={n}"), &co, &so);
                s5_r(so.as_mut_ptr(), msg.as_ptr(), n as u64);
                common::eqb(&format!("crypto_hash_r==sha512_r n={n}"), &ro, &so);
            }
        }
    }
}

// ======================================================= sha2 (256 / 512) ====

struct Sha2 {
    name: &'static str,
    outlen: usize,
    block: usize,
    statebytes: usize,
    one: (HashFn, HashFn),
    init: (InitFn, InitFn),
    upd: (UpdFn, UpdFn),
    fin: (FinFn, FinFn),
}

fn sha2_specs() -> Vec<Sha2> {
    vec![
        Sha2 {
            name: "sha256",
            outlen: 32,
            block: 64,
            statebytes: 104,
            one: both!("crypto_hash_sha256", HashFn),
            init: both!("crypto_hash_sha256_init", InitFn),
            upd: both!("crypto_hash_sha256_update", UpdFn),
            fin: both!("crypto_hash_sha256_final", FinFn),
        },
        Sha2 {
            name: "sha512",
            outlen: 64,
            block: 128,
            statebytes: 208,
            one: both!("crypto_hash_sha512", HashFn),
            init: both!("crypto_hash_sha512_init", InitFn),
            upd: both!("crypto_hash_sha512_update", UpdFn),
            fin: both!("crypto_hash_sha512_final", FinFn),
        },
    ]
}

#[test]
fn sha2_oneshot() {
    let mut rng = common::Rng::new(0xC0FFEE);
    for s in sha2_specs() {
        // statebytes must agree with the header-derived value (checked in getters)
        for &n in SIZES {
            for trial in 0..3 {
                let msg = rng.bytes(n);
                let mut co = guarded(s.outlen);
                let mut ro = guarded(s.outlen);
                unsafe {
                    let rc = (s.one.0)(co.as_mut_ptr(), msg.as_ptr(), n as u64);
                    let rr = (s.one.1)(ro.as_mut_ptr(), msg.as_ptr(), n as u64);
                    common::eqi(&format!("{} one n={n}", s.name), rc, rr);
                    assert_eq!(rc, 0);
                    common::eqb(&format!("{} one n={n} t={trial}", s.name), &co, &ro);
                }
            }
        }
        // in = NULL, inlen = 0
        unsafe {
            let mut co = guarded(s.outlen);
            let mut ro = guarded(s.outlen);
            let rc = (s.one.0)(co.as_mut_ptr(), core::ptr::null(), 0);
            let rr = (s.one.1)(ro.as_mut_ptr(), core::ptr::null(), 0);
            common::eqi(&format!("{} one NULL", s.name), rc, rr);
            common::eqb(&format!("{} one NULL", s.name), &co, &ro);
        }
    }
}

/// Build a set of interesting chunk splittings of `n` bytes.
fn splits(rng: &mut common::Rng, n: usize, block: usize) -> Vec<Vec<usize>> {
    let mut out: Vec<Vec<usize>> = Vec::new();
    out.push(vec![n]); // 1 chunk
    // splits landing exactly on a block boundary (exercises offset == block/rate)
    if n >= block {
        out.push(vec![block, n - block]);
        if n > block {
            out.push(vec![block, 0, n - block]);
        }
    }
    if n >= 2 * block {
        out.push(vec![2 * block, n - 2 * block]);
    }
    if n > 0 {
        out.push(vec![1, n - 1]);
        out.push(vec![n - 1, 1]);
        if n <= 64 {
            out.push(vec![1usize; n]);
        }
    }
    // random splits with k chunks
    for k in 2..=5usize {
        let mut v = Vec::with_capacity(k);
        let mut left = n;
        for i in 0..k {
            let take = if i + 1 == k {
                left
            } else {
                rng.below(left + 1)
            };
            v.push(take);
            left -= take;
        }
        out.push(v);
    }
    // zero-length updates interleaved
    let mut z = vec![0usize];
    z.extend(splits_flat(n));
    z.push(0);
    out.push(z);
    out
}

fn splits_flat(n: usize) -> Vec<usize> {
    if n == 0 {
        vec![0]
    } else if n == 1 {
        vec![1]
    } else {
        vec![n / 2, n - n / 2]
    }
}

#[test]
fn sha2_streaming() {
    let mut rng = common::Rng::new(0xBEEF01);
    for s in sha2_specs() {
        for &n in SIZES {
            let msg = rng.bytes(n);
            // reference digest from the C one-shot
            let mut want = vec![0u8; s.outlen];
            unsafe { (s.one.0)(want.as_mut_ptr(), msg.as_ptr(), n as u64) };

            for sp in splits(&mut rng, n, s.block) {
                let mut cs = Sbuf::canary();
                let mut rs = Sbuf::canary();
                unsafe {
                    let a = (s.init.0)(cs.p());
                    let b = (s.init.1)(rs.p());
                    common::eqi(&format!("{} init", s.name), a, b);
                    assert_eq!(a, 0);
                    cmp_states(&format!("{} after init n={n}", s.name), &cs, &rs);

                    let mut off = 0usize;
                    for (i, &len) in sp.iter().enumerate() {
                        let p = if len == 0 {
                            core::ptr::null()
                        } else {
                            msg.as_ptr().add(off)
                        };
                        let a = (s.upd.0)(cs.p(), p, len as u64);
                        let b = (s.upd.1)(rs.p(), p, len as u64);
                        common::eqi(&format!("{} upd n={n} i={i}", s.name), a, b);
                        assert_eq!(a, 0);
                        cmp_states(
                            &format!("{} after upd#{i} len={len} n={n} split={sp:?}", s.name),
                            &cs,
                            &rs,
                        );
                        off += len;
                    }
                    assert_eq!(off, n);

                    let mut co = guarded(s.outlen);
                    let mut ro = guarded(s.outlen);
                    let a = (s.fin.0)(cs.p(), co.as_mut_ptr());
                    let b = (s.fin.1)(rs.p(), ro.as_mut_ptr());
                    common::eqi(&format!("{} final", s.name), a, b);
                    assert_eq!(a, 0);
                    common::eqb(&format!("{} final n={n} split={sp:?}", s.name), &co, &ro);
                    common::eqb(
                        &format!("{} stream==oneshot n={n} split={sp:?}", s.name),
                        &co[..s.outlen],
                        &want,
                    );
                    // final() zeroizes the whole state in both
                    cmp_states(&format!("{} after final n={n}", s.name), &cs, &rs);
                    assert!(
                        cs.0[..s.statebytes].iter().all(|&b| b == 0),
                        "{}: state not zeroized by final()",
                        s.name
                    );
                }
            }
        }
    }
}

#[test]
fn sha2_huge() {
    let mut rng = common::Rng::new(0x5EED42);
    let msg = rng.bytes(300_003);
    for s in sha2_specs() {
        let mut co = guarded(s.outlen);
        let mut ro = guarded(s.outlen);
        unsafe {
            (s.one.0)(co.as_mut_ptr(), msg.as_ptr(), msg.len() as u64);
            (s.one.1)(ro.as_mut_ptr(), msg.as_ptr(), msg.len() as u64);
        }
        common::eqb(&format!("{} huge", s.name), &co, &ro);

        // and streaming with pseudo-random chunk sizes
        let mut cs = Sbuf::canary();
        let mut rs = Sbuf::canary();
        unsafe {
            (s.init.0)(cs.p());
            (s.init.1)(rs.p());
            let mut off = 0usize;
            while off < msg.len() {
                let len = (1 + rng.below(9000)).min(msg.len() - off);
                let a = (s.upd.0)(cs.p(), msg.as_ptr().add(off), len as u64);
                let b = (s.upd.1)(rs.p(), msg.as_ptr().add(off), len as u64);
                common::eqi(&format!("{} huge upd", s.name), a, b);
                off += len;
            }
            cmp_states(&format!("{} huge state", s.name), &cs, &rs);
            let mut c2 = guarded(s.outlen);
            let mut r2 = guarded(s.outlen);
            (s.fin.0)(cs.p(), c2.as_mut_ptr());
            (s.fin.1)(rs.p(), r2.as_mut_ptr());
            common::eqb(&format!("{} huge stream", s.name), &c2, &r2);
            common::eqb(
                &format!("{} huge stream==oneshot", s.name),
                &c2[..s.outlen],
                &co[..s.outlen],
            );
        }
    }
}

// ==================================================== sha3-256 / sha3-512 ====

struct Sha3 {
    name: &'static str,
    outlen: usize,
    rate: usize,
    one: (HashFn, HashFn),
    init: (InitFn, InitFn),
    upd: (UpdFn, UpdFn),
    fin: (FinFn, FinFn),
}

fn sha3_specs() -> Vec<Sha3> {
    vec![
        Sha3 {
            name: "sha3256",
            outlen: 32,
            rate: 136,
            one: both!("crypto_hash_sha3256", HashFn),
            init: both!("crypto_hash_sha3256_init", InitFn),
            upd: both!("crypto_hash_sha3256_update", UpdFn),
            fin: both!("crypto_hash_sha3256_final", FinFn),
        },
        Sha3 {
            name: "sha3512",
            outlen: 64,
            rate: 72,
            one: both!("crypto_hash_sha3512", HashFn),
            init: both!("crypto_hash_sha3512_init", InitFn),
            upd: both!("crypto_hash_sha3512_update", UpdFn),
            fin: both!("crypto_hash_sha3512_final", FinFn),
        },
    ]
}

#[test]
fn sha3_oneshot() {
    let mut rng = common::Rng::new(0x3A3A3A);
    for s in sha3_specs() {
        for &n in SIZES {
            for trial in 0..3 {
                let msg = rng.bytes(n);
                let mut co = guarded(s.outlen);
                let mut ro = guarded(s.outlen);
                unsafe {
                    let a = (s.one.0)(co.as_mut_ptr(), msg.as_ptr(), n as u64);
                    let b = (s.one.1)(ro.as_mut_ptr(), msg.as_ptr(), n as u64);
                    common::eqi(&format!("{} one n={n}", s.name), a, b);
                    assert_eq!(a, 0);
                    common::eqb(&format!("{} one n={n} t={trial}", s.name), &co, &ro);
                }
            }
        }
        unsafe {
            let mut co = guarded(s.outlen);
            let mut ro = guarded(s.outlen);
            let a = (s.one.0)(co.as_mut_ptr(), core::ptr::null(), 0);
            let b = (s.one.1)(ro.as_mut_ptr(), core::ptr::null(), 0);
            common::eqi(&format!("{} one NULL", s.name), a, b);
            common::eqb(&format!("{} one NULL", s.name), &co, &ro);
        }
    }
    // Known-answer sanity: SHA3-256("") and SHA3-512("") from FIPS 202.
    let (c, _) = both!("crypto_hash_sha3256", HashFn);
    let mut o = [0u8; 32];
    unsafe { c(o.as_mut_ptr(), core::ptr::null(), 0) };
    assert_eq!(
        common::hex(&o),
        "a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a",
        "C SHA3-256(\"\") KAT"
    );
    let (c, _) = both!("crypto_hash_sha3512", HashFn);
    let mut o = [0u8; 64];
    unsafe { c(o.as_mut_ptr(), core::ptr::null(), 0) };
    assert_eq!(
        common::hex(&o),
        "a69f73cca23a9ac5c8b567dc185a756e97c98216\
         4fe25859e0d1dcc1475c80a615b2123af1f5f94c\
         11e3e9402c3ac558f500199d95b6d3e301758586\
         281dcd26"
            .replace(' ', ""),
        "C SHA3-512(\"\") KAT"
    );
}

#[test]
fn sha3_streaming() {
    let mut rng = common::Rng::new(0x3B3B3B);
    for s in sha3_specs() {
        for &n in SIZES {
            let msg = rng.bytes(n);
            let mut want = vec![0u8; s.outlen];
            unsafe { (s.one.0)(want.as_mut_ptr(), msg.as_ptr(), n as u64) };

            for sp in splits(&mut rng, n, s.rate) {
                let mut cs = Sbuf::canary();
                let mut rs = Sbuf::canary();
                unsafe {
                    let a = (s.init.0)(cs.p());
                    let b = (s.init.1)(rs.p());
                    common::eqi(&format!("{} init", s.name), a, b);
                    assert_eq!(a, 0);
                    cmp_states(&format!("{} after init", s.name), &cs, &rs);

                    let mut off = 0usize;
                    for (i, &len) in sp.iter().enumerate() {
                        let p = if len == 0 {
                            core::ptr::null()
                        } else {
                            msg.as_ptr().add(off)
                        };
                        let a = (s.upd.0)(cs.p(), p, len as u64);
                        let b = (s.upd.1)(rs.p(), p, len as u64);
                        common::eqi(&format!("{} upd n={n} i={i}", s.name), a, b);
                        assert_eq!(a, 0);
                        cmp_states(
                            &format!("{} after upd#{i} len={len} n={n} split={sp:?}", s.name),
                            &cs,
                            &rs,
                        );
                        off += len;
                    }

                    let mut co = guarded(s.outlen);
                    let mut ro = guarded(s.outlen);
                    let a = (s.fin.0)(cs.p(), co.as_mut_ptr());
                    let b = (s.fin.1)(rs.p(), ro.as_mut_ptr());
                    common::eqi(&format!("{} final", s.name), a, b);
                    assert_eq!(a, 0);
                    common::eqb(&format!("{} final n={n} split={sp:?}", s.name), &co, &ro);
                    common::eqb(
                        &format!("{} stream==oneshot n={n} split={sp:?}", s.name),
                        &co[..s.outlen],
                        &want,
                    );
                    cmp_states(&format!("{} after final", s.name), &cs, &rs);
                }
            }
        }
    }
}

/// `sha3_update` / `sha3_final` return -1 when the state is already FINALIZED,
/// and recover by permuting + resetting the offset.
#[test]
fn sha3_reuse_after_final() {
    let mut rng = common::Rng::new(0x3C3C3C);
    for s in sha3_specs() {
        for &n in &[0usize, 1, 71, 72, 135, 136, 137, 300] {
            let msg = rng.bytes(n);
            let extra = rng.bytes(n.max(1));
            let mut cs = Sbuf::canary();
            let mut rs = Sbuf::canary();
            unsafe {
                (s.init.0)(cs.p());
                (s.init.1)(rs.p());
                (s.upd.0)(cs.p(), msg.as_ptr(), n as u64);
                (s.upd.1)(rs.p(), msg.as_ptr(), n as u64);
                let mut c1 = guarded(s.outlen);
                let mut r1 = guarded(s.outlen);
                (s.fin.0)(cs.p(), c1.as_mut_ptr());
                (s.fin.1)(rs.p(), r1.as_mut_ptr());
                common::eqb(&format!("{} d1 n={n}", s.name), &c1, &r1);
                cmp_states(&format!("{} state after final n={n}", s.name), &cs, &rs);

                // final() again on a FINALIZED state -> -1
                let mut c2 = guarded(s.outlen);
                let mut r2 = guarded(s.outlen);
                let a = (s.fin.0)(cs.p(), c2.as_mut_ptr());
                let b = (s.fin.1)(rs.p(), r2.as_mut_ptr());
                common::eqi(&format!("{} final#2 n={n}", s.name), a, b);
                assert_eq!(a, -1, "{}: final on finalized state must return -1", s.name);
                common::eqb(&format!("{} d2 n={n}", s.name), &c2, &r2);
                cmp_states(&format!("{} state after final#2 n={n}", s.name), &cs, &rs);

                // update() on a FINALIZED state -> -1 and resets to ABSORBING
                let a = (s.upd.0)(cs.p(), extra.as_ptr(), extra.len() as u64);
                let b = (s.upd.1)(rs.p(), extra.as_ptr(), extra.len() as u64);
                common::eqi(&format!("{} upd-after-final n={n}", s.name), a, b);
                assert_eq!(a, -1, "{}: update after final must return -1", s.name);
                cmp_states(&format!("{} state after upd-after-final n={n}", s.name), &cs, &rs);

                let mut c3 = guarded(s.outlen);
                let mut r3 = guarded(s.outlen);
                let a = (s.fin.0)(cs.p(), c3.as_mut_ptr());
                let b = (s.fin.1)(rs.p(), r3.as_mut_ptr());
                common::eqi(&format!("{} final#3 n={n}", s.name), a, b);
                assert_eq!(a, 0);
                common::eqb(&format!("{} d3 n={n}", s.name), &c3, &r3);
            }
        }
    }
}

#[test]
fn sha3_huge() {
    let mut rng = common::Rng::new(0x3D3D3D);
    let msg = rng.bytes(280_007);
    for s in sha3_specs() {
        let mut co = guarded(s.outlen);
        let mut ro = guarded(s.outlen);
        unsafe {
            (s.one.0)(co.as_mut_ptr(), msg.as_ptr(), msg.len() as u64);
            (s.one.1)(ro.as_mut_ptr(), msg.as_ptr(), msg.len() as u64);
        }
        common::eqb(&format!("{} huge", s.name), &co, &ro);

        let mut cs = Sbuf::canary();
        let mut rs = Sbuf::canary();
        unsafe {
            (s.init.0)(cs.p());
            (s.init.1)(rs.p());
            let mut off = 0usize;
            while off < msg.len() {
                let len = (1 + rng.below(5000)).min(msg.len() - off);
                (s.upd.0)(cs.p(), msg.as_ptr().add(off), len as u64);
                (s.upd.1)(rs.p(), msg.as_ptr().add(off), len as u64);
                off += len;
            }
            cmp_states(&format!("{} huge state", s.name), &cs, &rs);
            let mut c2 = guarded(s.outlen);
            let mut r2 = guarded(s.outlen);
            (s.fin.0)(cs.p(), c2.as_mut_ptr());
            (s.fin.1)(rs.p(), r2.as_mut_ptr());
            common::eqb(&format!("{} huge stream", s.name), &c2, &r2);
            common::eqb(
                &format!("{} huge stream==oneshot", s.name),
                &c2[..s.outlen],
                &co[..s.outlen],
            );
        }
    }
}

// ================================================== crypto_core_keccak1600 ===

const KEC_STATE: usize = 224; // sizeof(crypto_core_keccak1600_state)
const KEC_USED: usize = 200; // KECCAK1600_STATEBYTES touched by the ref code

#[test]
fn keccak1600_public() {
    let init = both!("crypto_core_keccak1600_init", KecInitFn);
    let xor = both!("crypto_core_keccak1600_xor_bytes", KecXorFn);
    let ext = both!("crypto_core_keccak1600_extract_bytes", KecExtFn);
    let p24 = both!("crypto_core_keccak1600_permute_24", KecPermFn);
    let p12 = both!("crypto_core_keccak1600_permute_12", KecPermFn);
    let mut rng = common::Rng::new(0xCECCA_0001);

    // ---- init on a canary-filled buffer: only the first 200 bytes are zeroed
    let mut cs = Sbuf::canary();
    let mut rs = Sbuf::canary();
    unsafe {
        init.0(cs.p());
        init.1(rs.p());
    }
    cmp_states("keccak init", &cs, &rs);
    assert!(cs.0[..KEC_USED].iter().all(|&b| b == 0));
    assert!(
        cs.0[KEC_USED..KEC_STATE].iter().all(|&b| b == 0xA5),
        "init must not touch bytes 200..224"
    );

    // ---- permute on the all-zero state, repeatedly (checks round constants)
    for round in 0..4 {
        unsafe {
            p24.0(cs.p());
            p24.1(rs.p());
        }
        cmp_states(&format!("keccak permute_24 x{round}"), &cs, &rs);
    }
    let mut cs = Sbuf::canary();
    let mut rs = Sbuf::canary();
    unsafe {
        init.0(cs.p());
        init.1(rs.p());
    }
    for round in 0..4 {
        unsafe {
            p12.0(cs.p());
            p12.1(rs.p());
        }
        cmp_states(&format!("keccak permute_12 x{round}"), &cs, &rs);
    }

    // ---- permute on an all-0xFF state
    let mut cs = Sbuf([0xFFu8; 384]);
    let mut rs = Sbuf([0xFFu8; 384]);
    unsafe {
        p24.0(cs.p());
        p24.1(rs.p());
    }
    cmp_states("keccak permute_24 ff", &cs, &rs);
    unsafe {
        p12.0(cs.p());
        p12.1(rs.p());
    }
    cmp_states("keccak permute_12 ff", &cs, &rs);

    // ---- randomized xor_bytes / extract_bytes / permute mix
    for iter in 0..200 {
        let seedbytes = rng.bytes(KEC_STATE);
        let mut cs = Sbuf::canary();
        let mut rs = Sbuf::canary();
        cs.0[..KEC_STATE].copy_from_slice(&seedbytes);
        rs.0[..KEC_STATE].copy_from_slice(&seedbytes);

        for step in 0..6 {
            // offsets deliberately include unaligned values so all three loops
            // of keccak1600_ref_xor_bytes (head / 8-byte body / tail) run.
            let offset = rng.below(KEC_USED);
            let length = rng.below(KEC_USED - offset + 1);
            let data = rng.bytes(length.max(1));
            unsafe {
                xor.0(cs.p(), data.as_ptr(), offset, length);
                xor.1(rs.p(), data.as_ptr(), offset, length);
            }
            cmp_states(
                &format!("keccak xor iter={iter} step={step} off={offset} len={length}"),
                &cs,
                &rs,
            );

            let eoff = rng.below(KEC_USED);
            let elen = rng.below(KEC_USED - eoff + 1);
            let mut cout = guarded(elen);
            let mut rout = guarded(elen);
            unsafe {
                ext.0(cs.cp(), cout.as_mut_ptr(), eoff, elen);
                ext.1(rs.cp(), rout.as_mut_ptr(), eoff, elen);
            }
            common::eqb(
                &format!("keccak extract iter={iter} off={eoff} len={elen}"),
                &cout,
                &rout,
            );
            // extract_bytes is a plain memcpy from the state
            common::eqb(
                &format!("keccak extract==state iter={iter}"),
                &cout[..elen],
                &cs.0[eoff..eoff + elen],
            );

            if rng.below(2) == 0 {
                unsafe {
                    p24.0(cs.p());
                    p24.1(rs.p());
                }
            } else {
                unsafe {
                    p12.0(cs.p());
                    p12.1(rs.p());
                }
            }
            cmp_states(&format!("keccak permute iter={iter} step={step}"), &cs, &rs);
        }
    }

    // ---- xor_bytes with length 0 must be a no-op at any offset
    for offset in [0usize, 1, 7, 8, 9, 199] {
        let mut cs = Sbuf::canary();
        let mut rs = Sbuf::canary();
        let d = [0xFFu8; 1];
        unsafe {
            xor.0(cs.p(), d.as_ptr(), offset, 0);
            xor.1(rs.p(), d.as_ptr(), offset, 0);
        }
        cmp_states(&format!("keccak xor len=0 off={offset}"), &cs, &rs);
        assert!(cs.0.iter().all(|&b| b == 0xA5));
    }
    // ---- extract_bytes with length 0
    let cs = Sbuf::canary();
    let rs = Sbuf::canary();
    let mut cout = guarded(0);
    let mut rout = guarded(0);
    unsafe {
        ext.0(cs.cp(), cout.as_mut_ptr(), 13, 0);
        ext.1(rs.cp(), rout.as_mut_ptr(), 13, 0);
    }
    common::eqb("keccak extract len=0", &cout, &rout);
}

#[test]
fn keccak1600_ref_internal() {
    let init = both!("_sodium_keccak1600_ref_init", RefKecInitFn);
    let xor = both!("_sodium_keccak1600_ref_xor_bytes", RefKecXorFn);
    let ext = both!("_sodium_keccak1600_ref_extract_bytes", RefKecExtFn);
    let p24 = both!("_sodium_keccak1600_ref_permute_24", RefKecPermFn);
    let p12 = both!("_sodium_keccak1600_ref_permute_12", RefKecPermFn);
    let mut rng = common::Rng::new(0x1600_1600);

    // init zeroes exactly KECCAK1600_STATEBYTES (200) bytes
    let mut cs = Sbuf::canary();
    let mut rs = Sbuf::canary();
    unsafe {
        init.0(cs.p());
        init.1(rs.p());
    }
    cmp_states("ref init", &cs, &rs);
    assert!(cs.0[..200].iter().all(|&b| b == 0));
    assert!(cs.0[200..].iter().all(|&b| b == 0xA5));

    for iter in 0..150 {
        let seed = rng.bytes(200);
        let mut cs = Sbuf::canary();
        let mut rs = Sbuf::canary();
        cs.0[..200].copy_from_slice(&seed);
        rs.0[..200].copy_from_slice(&seed);

        // exercise every offset alignment class
        let offset = (iter % 9) + rng.below(24) * 8;
        let length = rng.below(200 - offset + 1);
        let data = rng.bytes(length.max(1));
        unsafe {
            xor.0(cs.p(), data.as_ptr(), offset, length);
            xor.1(rs.p(), data.as_ptr(), offset, length);
        }
        cmp_states(
            &format!("ref xor iter={iter} off={offset} len={length}"),
            &cs,
            &rs,
        );

        unsafe {
            p24.0(cs.p());
            p24.1(rs.p());
        }
        cmp_states(&format!("ref p24 iter={iter}"), &cs, &rs);
        unsafe {
            p12.0(cs.p());
            p12.1(rs.p());
        }
        cmp_states(&format!("ref p12 iter={iter}"), &cs, &rs);

        let eoff = rng.below(200);
        let elen = rng.below(200 - eoff + 1);
        let mut cout = guarded(elen);
        let mut rout = guarded(elen);
        unsafe {
            ext.0(cs.cp(), cout.as_mut_ptr(), eoff, elen);
            ext.1(rs.cp(), rout.as_mut_ptr(), eoff, elen);
        }
        common::eqb(&format!("ref extract iter={iter}"), &cout, &rout);
    }
}

// ============================================================== XOF ==========

struct Xof {
    name: &'static str,
    rate: usize,
    one: (XofFn, XofFn),
    init: (InitFn, InitFn),
    initd: (InitDomFn, InitDomFn),
    upd: (UpdFn, UpdFn),
    sq: (SqueezeFn, SqueezeFn),
    // internal `_sodium_*_ref*` entry points
    rone: (RefXofFn, RefXofFn),
    rinit: (InitFn, InitFn),
    rinitd: (InitDomFn, InitDomFn),
    rupd: (RefUpdFn, RefUpdFn),
    rsq: (SqueezeFn, SqueezeFn),
}

macro_rules! xof_spec {
    ($pub_:literal, $ref_:literal, $rate:expr) => {
        Xof {
            name: $pub_,
            rate: $rate,
            one: both!($pub_, XofFn),
            init: both!(concat!($pub_, "_init"), InitFn),
            initd: both!(concat!($pub_, "_init_with_domain"), InitDomFn),
            upd: both!(concat!($pub_, "_update"), UpdFn),
            sq: both!(concat!($pub_, "_squeeze"), SqueezeFn),
            rone: both!($ref_, RefXofFn),
            rinit: both!(concat!($ref_, "_init"), InitFn),
            rinitd: both!(concat!($ref_, "_init_with_domain"), InitDomFn),
            rupd: both!(concat!($ref_, "_update"), RefUpdFn),
            rsq: both!(concat!($ref_, "_squeeze"), SqueezeFn),
        }
    };
}

fn xof_specs() -> Vec<Xof> {
    vec![
        xof_spec!("crypto_xof_shake128", "_sodium_shake128_ref", 168),
        xof_spec!("crypto_xof_shake256", "_sodium_shake256_ref", 136),
        xof_spec!("crypto_xof_turboshake128", "_sodium_turboshake128_ref", 168),
        xof_spec!("crypto_xof_turboshake256", "_sodium_turboshake256_ref", 136),
    ]
}

const OUTLENS: &[usize] = &[
    0, 1, 2, 7, 8, 31, 32, 63, 64, 71, 72, 73, 135, 136, 137, 167, 168, 169, 200, 271, 272, 273,
    335, 336, 337, 504, 1000,
];

#[test]
fn xof_oneshot() {
    let mut rng = common::Rng::new(0x0F0F0F);
    for s in xof_specs() {
        for &inlen in SIZES {
            let msg = rng.bytes(inlen);
            for &outlen in OUTLENS {
                let mut co = guarded(outlen);
                let mut ro = guarded(outlen);
                unsafe {
                    let a = (s.one.0)(co.as_mut_ptr(), outlen, msg.as_ptr(), inlen as u64);
                    let b = (s.one.1)(ro.as_mut_ptr(), outlen, msg.as_ptr(), inlen as u64);
                    common::eqi(&format!("{} one in={inlen} out={outlen}", s.name), a, b);
                    assert_eq!(a, 0);
                    common::eqb(
                        &format!("{} one in={inlen} out={outlen}", s.name),
                        &co,
                        &ro,
                    );
                }
                // the internal `_ref` one-shot must give the same bytes
                let mut c2 = guarded(outlen);
                let mut r2 = guarded(outlen);
                unsafe {
                    let a = (s.rone.0)(c2.as_mut_ptr(), outlen, msg.as_ptr(), inlen);
                    let b = (s.rone.1)(r2.as_mut_ptr(), outlen, msg.as_ptr(), inlen);
                    common::eqi(&format!("{} ref-one in={inlen} out={outlen}", s.name), a, b);
                    assert_eq!(a, 0);
                    common::eqb(
                        &format!("{} ref-one in={inlen} out={outlen}", s.name),
                        &c2,
                        &r2,
                    );
                    common::eqb(
                        &format!("{} pub==ref in={inlen} out={outlen}", s.name),
                        &co,
                        &c2,
                    );
                }
            }
        }
        // in = NULL, inlen = 0 (never dereferenced by the C)
        unsafe {
            let mut co = guarded(64);
            let mut ro = guarded(64);
            let a = (s.one.0)(co.as_mut_ptr(), 64, core::ptr::null(), 0);
            let b = (s.one.1)(ro.as_mut_ptr(), 64, core::ptr::null(), 0);
            common::eqi(&format!("{} one NULL", s.name), a, b);
            common::eqb(&format!("{} one NULL", s.name), &co, &ro);
        }
    }

    // Known-answer sanity for the C side (FIPS 202 / RFC-style test vectors).
    let (c, _) = both!("crypto_xof_shake128", XofFn);
    let mut o = [0u8; 32];
    unsafe { c(o.as_mut_ptr(), 32, core::ptr::null(), 0) };
    assert_eq!(
        common::hex(&o),
        "7f9c2ba4e88f827d616045507605853ed73b8093f6efbc88eb1a6eacfa66ef26",
        "C SHAKE128(\"\", 32) KAT"
    );
    let (c, _) = both!("crypto_xof_shake256", XofFn);
    let mut o = [0u8; 32];
    unsafe { c(o.as_mut_ptr(), 32, core::ptr::null(), 0) };
    assert_eq!(
        common::hex(&o),
        "46b9dd2b0ba88d13233b3feb743eeb243fcd52ea62b81b82b50c27646ed5762f",
        "C SHAKE256(\"\", 32) KAT"
    );
}

#[test]
fn xof_streaming_and_squeeze_chunks() {
    let mut rng = common::Rng::new(0x0E0E0E);
    for s in xof_specs() {
        for &inlen in &[
            0usize, 1, 2, 71, 72, 135, 136, 137, 167, 168, 169, 200, 335, 336, 337, 1000,
        ] {
            let msg = rng.bytes(inlen);
            for &outlen in &[
                0usize, 1, 32, 135, 136, 137, 167, 168, 169, 336, 400, 1000,
            ] {
                let mut want = guarded(outlen);
                unsafe { (s.one.0)(want.as_mut_ptr(), outlen, msg.as_ptr(), inlen as u64) };

                for sp in splits(&mut rng, inlen, s.rate).into_iter().take(7) {
                    // squeeze in a few different chunkings
                    for osp in [
                        vec![outlen],
                        {
                            let mut v = splits_flat(outlen);
                            v.insert(0, 0);
                            v.push(0);
                            v
                        },
                        if outlen >= s.rate {
                            vec![s.rate, outlen - s.rate]
                        } else {
                            vec![outlen]
                        },
                        if outlen > 1 {
                            vec![1, outlen - 1]
                        } else {
                            vec![outlen]
                        },
                        if outlen > 0 {
                            let mut v = Vec::new();
                            let mut left = outlen;
                            let mut k = 1usize;
                            while left > 0 {
                                let t = k.min(left);
                                v.push(t);
                                left -= t;
                                k = k * 2 + 1;
                            }
                            v
                        } else {
                            vec![0]
                        },
                    ] {
                        let mut cs = Sbuf::canary();
                        let mut rs = Sbuf::canary();
                        unsafe {
                            let a = (s.init.0)(cs.p());
                            let b = (s.init.1)(rs.p());
                            common::eqi(&format!("{} init", s.name), a, b);
                            assert_eq!(a, 0);
                            cmp_states(&format!("{} after init", s.name), &cs, &rs);

                            let mut off = 0usize;
                            for (i, &len) in sp.iter().enumerate() {
                                let p = if len == 0 {
                                    core::ptr::null()
                                } else {
                                    msg.as_ptr().add(off)
                                };
                                let a = (s.upd.0)(cs.p(), p, len as u64);
                                let b = (s.upd.1)(rs.p(), p, len as u64);
                                common::eqi(&format!("{} upd i={i}", s.name), a, b);
                                assert_eq!(a, 0);
                                cmp_states(
                                    &format!(
                                        "{} after upd#{i} in={inlen} split={sp:?}",
                                        s.name
                                    ),
                                    &cs,
                                    &rs,
                                );
                                off += len;
                            }

                            let mut cout = guarded(outlen);
                            let mut rout = guarded(outlen);
                            let mut ooff = 0usize;
                            for (i, &len) in osp.iter().enumerate() {
                                let a = (s.sq.0)(cs.p(), cout.as_mut_ptr().add(ooff), len);
                                let b = (s.sq.1)(rs.p(), rout.as_mut_ptr().add(ooff), len);
                                common::eqi(&format!("{} sq i={i}", s.name), a, b);
                                assert_eq!(a, 0);
                                cmp_states(
                                    &format!(
                                        "{} after sq#{i} len={len} out={outlen} osp={osp:?}",
                                        s.name
                                    ),
                                    &cs,
                                    &rs,
                                );
                                ooff += len;
                            }
                            assert_eq!(ooff, outlen);
                            common::eqb(
                                &format!(
                                    "{} sq in={inlen} out={outlen} sp={sp:?} osp={osp:?}",
                                    s.name
                                ),
                                &cout,
                                &rout,
                            );
                            common::eqb(
                                &format!(
                                    "{} chunked==oneshot in={inlen} out={outlen} osp={osp:?}",
                                    s.name
                                ),
                                &cout,
                                &want,
                            );
                        }
                    }
                }
            }
        }
    }
}

/// `_ref_init` / `_ref_init_with_domain` / `_ref_update` / `_ref_squeeze`
/// (the internal `_sodium_*` symbols), driven exactly like the public API.
#[test]
fn xof_ref_streaming() {
    let mut rng = common::Rng::new(0x0D0D0D);
    for s in xof_specs() {
        for &inlen in &[0usize, 1, 71, 136, 168, 169, 337, 700] {
            let msg = rng.bytes(inlen);
            for &outlen in &[0usize, 1, 136, 168, 337, 600] {
                for domain in [None, Some(0x1Fu8)] {
                    let mut cs = Sbuf::canary();
                    let mut rs = Sbuf::canary();
                    unsafe {
                        let (a, b) = match domain {
                            None => ((s.rinit.0)(cs.p()), (s.rinit.1)(rs.p())),
                            Some(d) => ((s.rinitd.0)(cs.p(), d), (s.rinitd.1)(rs.p(), d)),
                        };
                        common::eqi(&format!("{} ref init", s.name), a, b);
                        assert_eq!(a, 0);
                        cmp_states(&format!("{} ref after init", s.name), &cs, &rs);

                        // two updates
                        let half = inlen / 2;
                        for (o, l) in [(0usize, half), (half, inlen - half)] {
                            let p = if l == 0 {
                                core::ptr::null()
                            } else {
                                msg.as_ptr().add(o)
                            };
                            let a = (s.rupd.0)(cs.p(), p, l);
                            let b = (s.rupd.1)(rs.p(), p, l);
                            common::eqi(&format!("{} ref upd", s.name), a, b);
                            assert_eq!(a, 0);
                            cmp_states(&format!("{} ref after upd l={l}", s.name), &cs, &rs);
                        }

                        // squeeze in two pieces
                        let mut cout = guarded(outlen);
                        let mut rout = guarded(outlen);
                        let h = outlen / 2;
                        let mut ooff = 0usize;
                        for l in [h, outlen - h] {
                            let a = (s.rsq.0)(cs.p(), cout.as_mut_ptr().add(ooff), l);
                            let b = (s.rsq.1)(rs.p(), rout.as_mut_ptr().add(ooff), l);
                            common::eqi(&format!("{} ref sq", s.name), a, b);
                            assert_eq!(a, 0);
                            cmp_states(&format!("{} ref after sq l={l}", s.name), &cs, &rs);
                            ooff += l;
                        }
                        common::eqb(
                            &format!("{} ref stream in={inlen} out={outlen}", s.name),
                            &cout,
                            &rout,
                        );
                        // ... and it must equal the public one-shot for the
                        // standard domain
                        let mut want = guarded(outlen);
                        (s.one.0)(want.as_mut_ptr(), outlen, msg.as_ptr(), inlen as u64);
                        common::eqb(
                            &format!("{} ref==pub in={inlen} out={outlen}", s.name),
                            &cout,
                            &want,
                        );
                    }
                }
            }
        }
    }
}

/// `init_with_domain` over the whole `unsigned char` range of interesting
/// domain bytes, including the `offset == RATE - 1` special case where the
/// padding collapses to `domain ^ 0x80`.
#[test]
fn xof_domains() {
    let mut rng = common::Rng::new(0x0C0C0C);
    for s in xof_specs() {
        for domain in [
            0x00u8, 0x01, 0x02, 0x06, 0x07, 0x0B, 0x1F, 0x7F, 0x80, 0x81, 0xA5, 0xFE, 0xFF,
        ] {
            // inlen chosen so that offset lands on RATE-1, RATE, 0 and mid-block
            for &inlen in &[
                0usize,
                1,
                s.rate - 2,
                s.rate - 1,
                s.rate,
                s.rate + 1,
                2 * s.rate - 1,
                2 * s.rate,
                2 * s.rate + 1,
            ] {
                let msg = rng.bytes(inlen);
                for &outlen in &[0usize, 1, 32, s.rate, s.rate + 1, 2 * s.rate + 5] {
                    let mut cs = Sbuf::canary();
                    let mut rs = Sbuf::canary();
                    let mut cout = guarded(outlen);
                    let mut rout = guarded(outlen);
                    unsafe {
                        let a = (s.initd.0)(cs.p(), domain);
                        let b = (s.initd.1)(rs.p(), domain);
                        common::eqi(&format!("{} initd {domain:#x}", s.name), a, b);
                        assert_eq!(a, 0);
                        cmp_states(&format!("{} initd {domain:#x}", s.name), &cs, &rs);

                        let p = if inlen == 0 {
                            core::ptr::null()
                        } else {
                            msg.as_ptr()
                        };
                        let a = (s.upd.0)(cs.p(), p, inlen as u64);
                        let b = (s.upd.1)(rs.p(), p, inlen as u64);
                        common::eqi(&format!("{} initd upd", s.name), a, b);
                        cmp_states(
                            &format!("{} initd {domain:#x} upd in={inlen}", s.name),
                            &cs,
                            &rs,
                        );

                        let a = (s.sq.0)(cs.p(), cout.as_mut_ptr(), outlen);
                        let b = (s.sq.1)(rs.p(), rout.as_mut_ptr(), outlen);
                        common::eqi(&format!("{} initd sq", s.name), a, b);
                        assert_eq!(a, 0);
                        cmp_states(
                            &format!("{} initd {domain:#x} sq in={inlen} out={outlen}", s.name),
                            &cs,
                            &rs,
                        );
                        common::eqb(
                            &format!(
                                "{} domain={domain:#x} in={inlen} out={outlen}",
                                s.name
                            ),
                            &cout,
                            &rout,
                        );
                    }

                    // the standard domain must reproduce the one-shot exactly
                    if domain == 0x1F {
                        let mut want = guarded(outlen);
                        unsafe {
                            (s.one.0)(want.as_mut_ptr(), outlen, msg.as_ptr(), inlen as u64)
                        };
                        common::eqb(
                            &format!("{} domain-std==oneshot in={inlen} out={outlen}", s.name),
                            &cout,
                            &want,
                        );
                    }
                }
            }
        }
    }
}

/// absorb / squeeze interleaving: `update()` after `squeeze()` returns -1,
/// permutes the state and restarts absorbing.  Then squeezing again works.
#[test]
fn xof_absorb_squeeze_interleave() {
    let mut rng = common::Rng::new(0x0B0B0B);
    for s in xof_specs() {
        for &inlen in &[0usize, 1, 71, s.rate - 1, s.rate, s.rate + 1, 300] {
            for &outlen in &[0usize, 1, 32, s.rate - 1, s.rate, s.rate + 1, 400] {
                let msg = rng.bytes(inlen.max(1));
                let extra = rng.bytes(inlen.max(1));
                let mut cs = Sbuf::canary();
                let mut rs = Sbuf::canary();
                unsafe {
                    (s.init.0)(cs.p());
                    (s.init.1)(rs.p());
                    (s.upd.0)(cs.p(), msg.as_ptr(), inlen as u64);
                    (s.upd.1)(rs.p(), msg.as_ptr(), inlen as u64);

                    let mut c1 = guarded(outlen);
                    let mut r1 = guarded(outlen);
                    (s.sq.0)(cs.p(), c1.as_mut_ptr(), outlen);
                    (s.sq.1)(rs.p(), r1.as_mut_ptr(), outlen);
                    common::eqb(&format!("{} il sq1", s.name), &c1, &r1);
                    cmp_states(&format!("{} il after sq1", s.name), &cs, &rs);

                    // update while SQUEEZING -> -1, phase reset to ABSORBING
                    let a = (s.upd.0)(cs.p(), extra.as_ptr(), inlen as u64);
                    let b = (s.upd.1)(rs.p(), extra.as_ptr(), inlen as u64);
                    common::eqi(&format!("{} il upd-after-sq", s.name), a, b);
                    assert_eq!(
                        a, -1,
                        "{}: update() while squeezing must return -1",
                        s.name
                    );
                    cmp_states(&format!("{} il after upd-after-sq", s.name), &cs, &rs);

                    // squeeze again (re-finalizes)
                    let mut c2 = guarded(outlen);
                    let mut r2 = guarded(outlen);
                    let a = (s.sq.0)(cs.p(), c2.as_mut_ptr(), outlen);
                    let b = (s.sq.1)(rs.p(), r2.as_mut_ptr(), outlen);
                    common::eqi(&format!("{} il sq2", s.name), a, b);
                    assert_eq!(a, 0);
                    common::eqb(&format!("{} il sq2", s.name), &c2, &r2);
                    cmp_states(&format!("{} il after sq2", s.name), &cs, &rs);

                    // several more squeezes keep tracking
                    for k in 0..4 {
                        let mut c3 = guarded(s.rate + 3);
                        let mut r3 = guarded(s.rate + 3);
                        let a = (s.sq.0)(cs.p(), c3.as_mut_ptr(), s.rate + 3);
                        let b = (s.sq.1)(rs.p(), r3.as_mut_ptr(), s.rate + 3);
                        common::eqi(&format!("{} il sq{k}", s.name), a, b);
                        common::eqb(&format!("{} il sq extra {k}", s.name), &c3, &r3);
                        cmp_states(&format!("{} il after sq extra {k}", s.name), &cs, &rs);
                    }
                }
            }
        }
    }

    // Same dance through the internal `_ref_*` symbols.
    for s in xof_specs() {
        let msg = rng.bytes(500);
        let mut cs = Sbuf::canary();
        let mut rs = Sbuf::canary();
        unsafe {
            (s.rinitd.0)(cs.p(), 0x0B);
            (s.rinitd.1)(rs.p(), 0x0B);
            (s.rupd.0)(cs.p(), msg.as_ptr(), 500);
            (s.rupd.1)(rs.p(), msg.as_ptr(), 500);
            let mut c1 = guarded(300);
            let mut r1 = guarded(300);
            (s.rsq.0)(cs.p(), c1.as_mut_ptr(), 300);
            (s.rsq.1)(rs.p(), r1.as_mut_ptr(), 300);
            common::eqb(&format!("{} ref il sq1", s.name), &c1, &r1);
            let a = (s.rupd.0)(cs.p(), msg.as_ptr(), 137);
            let b = (s.rupd.1)(rs.p(), msg.as_ptr(), 137);
            common::eqi(&format!("{} ref il upd-after-sq", s.name), a, b);
            assert_eq!(a, -1);
            cmp_states(&format!("{} ref il state", s.name), &cs, &rs);
            let mut c2 = guarded(200);
            let mut r2 = guarded(200);
            (s.rsq.0)(cs.p(), c2.as_mut_ptr(), 200);
            (s.rsq.1)(rs.p(), r2.as_mut_ptr(), 200);
            common::eqb(&format!("{} ref il sq2", s.name), &c2, &r2);
            cmp_states(&format!("{} ref il state2", s.name), &cs, &rs);
        }
    }
}

#[test]
fn xof_huge() {
    let mut rng = common::Rng::new(0x0A0A0A);
    let msg = rng.bytes(260_011);
    for s in xof_specs() {
        let outlen = 40_009usize;
        let mut co = guarded(outlen);
        let mut ro = guarded(outlen);
        unsafe {
            let a = (s.one.0)(co.as_mut_ptr(), outlen, msg.as_ptr(), msg.len() as u64);
            let b = (s.one.1)(ro.as_mut_ptr(), outlen, msg.as_ptr(), msg.len() as u64);
            common::eqi(&format!("{} huge", s.name), a, b);
        }
        common::eqb(&format!("{} huge", s.name), &co, &ro);

        // streaming with random absorb + squeeze chunk sizes
        let mut cs = Sbuf::canary();
        let mut rs = Sbuf::canary();
        let mut cout = guarded(outlen);
        let mut rout = guarded(outlen);
        unsafe {
            (s.init.0)(cs.p());
            (s.init.1)(rs.p());
            let mut off = 0usize;
            while off < msg.len() {
                let len = (1 + rng.below(7000)).min(msg.len() - off);
                (s.upd.0)(cs.p(), msg.as_ptr().add(off), len as u64);
                (s.upd.1)(rs.p(), msg.as_ptr().add(off), len as u64);
                off += len;
            }
            cmp_states(&format!("{} huge absorb state", s.name), &cs, &rs);
            let mut ooff = 0usize;
            while ooff < outlen {
                let len = (1 + rng.below(1000)).min(outlen - ooff);
                (s.sq.0)(cs.p(), cout.as_mut_ptr().add(ooff), len);
                (s.sq.1)(rs.p(), rout.as_mut_ptr().add(ooff), len);
                ooff += len;
            }
            cmp_states(&format!("{} huge squeeze state", s.name), &cs, &rs);
        }
        common::eqb(&format!("{} huge stream", s.name), &cout, &rout);
        common::eqb(
            &format!("{} huge stream==oneshot", s.name),
            &cout,
            &co,
        );
    }
}
