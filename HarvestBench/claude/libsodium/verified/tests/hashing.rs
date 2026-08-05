//! Differential tests for the HASHING family: crypto_hash (sha512), sha256,
//! sha512, sha3_256, sha3_512, generichash/blake2b, shorthash/siphash24 &
//! siphashx24, and the XOF family (shake128/256, turboshake128/256).
//!
//! Every call goes through the exported symbol loaded from BOTH the C and the
//! Rust cdylib; outputs are compared byte-for-byte. The C library is ground
//! truth.

#[macro_use]
mod common;
use common::{libs, Rng};

// ---- FFI type aliases ---------------------------------------------------

type OneShot = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
type HashInit = unsafe extern "C" fn(*mut u8) -> i32;
type HashUpdate = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
type HashFinal = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;
type SizeFn = unsafe extern "C" fn() -> usize;

// generichash one-shot: (out, outlen, in, inlen, key, keylen)
type GhOneShot =
    unsafe extern "C" fn(*mut u8, usize, *const u8, u64, *const u8, usize) -> i32;
// generichash salt_personal: (out, outlen, in, inlen, key, keylen, salt, personal)
type GhSaltPersonal = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const u8,
    u64,
    *const u8,
    usize,
    *const u8,
    *const u8,
) -> i32;
// generichash init: (state, key, keylen, outlen)
type GhInit = unsafe extern "C" fn(*mut u8, *const u8, usize, usize) -> i32;
// generichash init salt/personal: (state, key, keylen, outlen, salt, personal)
type GhInitSP =
    unsafe extern "C" fn(*mut u8, *const u8, usize, usize, *const u8, *const u8) -> i32;
type GhUpdate = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
// generichash final: (state, out, outlen)
type GhFinal = unsafe extern "C" fn(*mut u8, *mut u8, usize) -> i32;

// shorthash: (out, in, inlen, k)
type ShortHash = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> i32;

// xof one-shot: (out, outlen, in, inlen)
type XofOneShot = unsafe extern "C" fn(*mut u8, usize, *const u8, u64) -> i32;
type XofInit = unsafe extern "C" fn(*mut u8) -> i32;
type XofInitDomain = unsafe extern "C" fn(*mut u8, u8) -> i32;
type XofUpdate = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
// xof squeeze: (state, out, outlen)
type XofSqueeze = unsafe extern "C" fn(*mut u8, *mut u8, usize) -> i32;

/// Interesting input lengths: 0, 1, around block boundaries, and large.
fn lengths() -> Vec<usize> {
    vec![
        0, 1, 2, 3, 7, 8, 15, 16, 31, 32, 55, 56, 63, 64, 65, 71, 72, 111, 112, 127, 128, 129,
        135, 136, 137, 167, 168, 169, 191, 200, 255, 256, 257, 511, 512, 1000, 1024, 4096, 10000,
    ]
}

// =========================================================================
// PHASE B — valid paths
// =========================================================================

/// Generic driver for a fixed-output hash with a one-shot symbol.
fn check_oneshot_hash(name: &[u8], outbytes: usize) {
    let l = libs();
    let mut rng = Rng::new(0xA11CE ^ name.len() as u64);
    unsafe {
        let (c, r) = sympair!(l, name, OneShot);
        for &len in lengths().iter() {
            let input = rng.vec(len);
            let mut co = vec![0u8; outbytes];
            let mut ro = vec![0u8; outbytes];
            let rc = c(co.as_mut_ptr(), input.as_ptr(), len as u64);
            let rr = r(ro.as_mut_ptr(), input.as_ptr(), len as u64);
            assert_eq!(rc, rr, "{} rc mismatch len={}", String::from_utf8_lossy(name), len);
            assert_eq!(co, ro, "{} out mismatch len={}", String::from_utf8_lossy(name), len);
        }
    }
}

#[test]
fn sha512_oneshot() {
    check_oneshot_hash(b"crypto_hash_sha512", 64);
}

#[test]
fn crypto_hash_oneshot_is_sha512() {
    // crypto_hash == sha512; also compare crypto_hash vs crypto_hash_sha512.
    check_oneshot_hash(b"crypto_hash", 64);
    let l = libs();
    let mut rng = Rng::new(0xBEEF);
    unsafe {
        let (ch, _rh) = sympair!(l, b"crypto_hash", OneShot);
        let (cs, _rs) = sympair!(l, b"crypto_hash_sha512", OneShot);
        for &len in lengths().iter() {
            let input = rng.vec(len);
            let mut a = [0u8; 64];
            let mut b = [0u8; 64];
            ch(a.as_mut_ptr(), input.as_ptr(), len as u64);
            cs(b.as_mut_ptr(), input.as_ptr(), len as u64);
            assert_eq!(a, b, "crypto_hash != sha512 at len={}", len);
        }
    }
}

#[test]
fn sha256_oneshot() {
    check_oneshot_hash(b"crypto_hash_sha256", 32);
}

#[test]
fn sha3_256_oneshot() {
    check_oneshot_hash(b"crypto_hash_sha3256", 32);
}

#[test]
fn sha3_512_oneshot() {
    check_oneshot_hash(b"crypto_hash_sha3512", 64);
}

/// Streaming driver: init/update(*)/final, comparing C vs Rust and also each
/// against its own one-shot result.
fn check_streaming_hash(
    prefix: &str,
    outbytes: usize,
    statebytes_sym: &[u8],
) {
    let l = libs();
    let init_n = format!("{}_init", prefix);
    let upd_n = format!("{}_update", prefix);
    let fin_n = format!("{}_final", prefix);
    let mut rng = Rng::new(0x5EED ^ prefix.len() as u64);
    unsafe {
        let (c_sb, r_sb) = sympair!(l, statebytes_sym, SizeFn);
        let csb = c_sb();
        let rsb = r_sb();
        assert_eq!(csb, rsb, "{} statebytes differ", prefix);
        let sb = csb;

        let (c_init, r_init) = sympair!(l, init_n.as_bytes(), HashInit);
        let (c_upd, r_upd) = sympair!(l, upd_n.as_bytes(), HashUpdate);
        let (c_fin, r_fin) = sympair!(l, fin_n.as_bytes(), HashFinal);
        let (c_os, _r_os) = sympair!(l, prefix.as_bytes(), OneShot);

        for &len in lengths().iter() {
            let input = rng.vec(len);
            // random chunk split
            let mut chunks: Vec<usize> = Vec::new();
            let mut remaining = len;
            while remaining > 0 {
                let take = 1 + rng.range(remaining);
                chunks.push(take);
                remaining -= take;
            }

            let mut cstate = vec![0u8; sb];
            let mut rstate = vec![0u8; sb];
            let ic = c_init(cstate.as_mut_ptr());
            let ir = r_init(rstate.as_mut_ptr());
            assert_eq!(ic, ir, "{} init rc", prefix);

            let mut off = 0usize;
            for &ck in &chunks {
                let uc = c_upd(cstate.as_mut_ptr(), input[off..].as_ptr(), ck as u64);
                let ur = r_upd(rstate.as_mut_ptr(), input[off..].as_ptr(), ck as u64);
                assert_eq!(uc, ur, "{} update rc len={}", prefix, len);
                off += ck;
            }
            let mut cout = vec![0u8; outbytes];
            let mut rout = vec![0u8; outbytes];
            let fc = c_fin(cstate.as_mut_ptr(), cout.as_mut_ptr());
            let fr = r_fin(rstate.as_mut_ptr(), rout.as_mut_ptr());
            assert_eq!(fc, fr, "{} final rc", prefix);
            assert_eq!(cout, rout, "{} streaming out mismatch len={}", prefix, len);

            // Equivalence with one-shot
            let mut osout = vec![0u8; outbytes];
            c_os(osout.as_mut_ptr(), input.as_ptr(), len as u64);
            assert_eq!(cout, osout, "{} streaming != oneshot len={}", prefix, len);
        }
    }
}

#[test]
fn sha256_streaming() {
    check_streaming_hash("crypto_hash_sha256", 32, b"crypto_hash_sha256_statebytes");
}

#[test]
fn sha512_streaming() {
    check_streaming_hash("crypto_hash_sha512", 64, b"crypto_hash_sha512_statebytes");
}

#[test]
fn sha3_256_streaming() {
    check_streaming_hash("crypto_hash_sha3256", 32, b"crypto_hash_sha3256_statebytes");
}

#[test]
fn sha3_512_streaming() {
    check_streaming_hash("crypto_hash_sha3512", 64, b"crypto_hash_sha3512_statebytes");
}

// ---- generichash / blake2b ---------------------------------------------

#[test]
fn generichash_oneshot_unkeyed_and_keyed() {
    let l = libs();
    let mut rng = Rng::new(0x6142B);
    unsafe {
        let (c_gh, r_gh) = sympair!(l, b"crypto_generichash", GhOneShot);
        let (c_b2, r_b2) = sympair!(l, b"crypto_generichash_blake2b", GhOneShot);

        // outlens: min(16), default(32), max(64), and some in between
        let outlens = [16usize, 17, 20, 24, 31, 32, 48, 63, 64];
        // keylens: 0 (unkeyed), min(16), some, max(64)
        let keylens = [0usize, 1, 16, 24, 32, 63, 64];

        for &outlen in &outlens {
            for &keylen in &keylens {
                for &inlen in &[0usize, 1, 63, 64, 128, 129, 1000] {
                    let input = rng.vec(inlen);
                    let key = rng.vec(keylen);
                    let kptr = if keylen == 0 {
                        std::ptr::null()
                    } else {
                        key.as_ptr()
                    };
                    let mut co = vec![0u8; outlen];
                    let mut ro = vec![0u8; outlen];
                    let rc = c_gh(
                        co.as_mut_ptr(),
                        outlen,
                        input.as_ptr(),
                        inlen as u64,
                        kptr,
                        keylen,
                    );
                    let rr = r_gh(
                        ro.as_mut_ptr(),
                        outlen,
                        input.as_ptr(),
                        inlen as u64,
                        kptr,
                        keylen,
                    );
                    assert_eq!(rc, rr, "generichash rc outlen={} keylen={} inlen={}", outlen, keylen, inlen);
                    assert_eq!(co, ro, "generichash out outlen={} keylen={} inlen={}", outlen, keylen, inlen);

                    // blake2b == generichash
                    let mut cb = vec![0u8; outlen];
                    let mut rb = vec![0u8; outlen];
                    let rcb = c_b2(cb.as_mut_ptr(), outlen, input.as_ptr(), inlen as u64, kptr, keylen);
                    let rrb = r_b2(rb.as_mut_ptr(), outlen, input.as_ptr(), inlen as u64, kptr, keylen);
                    assert_eq!(rcb, rrb, "blake2b rc");
                    assert_eq!(cb, rb, "blake2b out");
                    assert_eq!(co, cb, "generichash != blake2b");
                }
            }
        }
    }
}

#[test]
fn generichash_streaming_equiv() {
    let l = libs();
    let mut rng = Rng::new(0x57EA3);
    unsafe {
        let (c_sb, r_sb) = sympair!(l, b"crypto_generichash_statebytes", SizeFn);
        assert_eq!(c_sb(), r_sb(), "generichash statebytes differ");
        let sb = c_sb();

        let (c_init, r_init) = sympair!(l, b"crypto_generichash_init", GhInit);
        let (c_upd, r_upd) = sympair!(l, b"crypto_generichash_update", GhUpdate);
        let (c_fin, r_fin) = sympair!(l, b"crypto_generichash_final", GhFinal);
        let (c_os, _r_os) = sympair!(l, b"crypto_generichash", GhOneShot);

        let outlens = [16usize, 32, 48, 64];
        let keylens = [0usize, 16, 32, 64];

        for &outlen in &outlens {
            for &keylen in &keylens {
                for &inlen in &[0usize, 1, 64, 200, 1000] {
                    let input = rng.vec(inlen);
                    let key = rng.vec(keylen);
                    let kptr = if keylen == 0 { std::ptr::null() } else { key.as_ptr() };

                    // 64-byte aligned state buffers (contract requires 64-byte alignment)
                    let mut cstate = AlignedState::new(sb);
                    let mut rstate = AlignedState::new(sb);
                    let ic = c_init(cstate.ptr(), kptr, keylen, outlen);
                    let ir = r_init(rstate.ptr(), kptr, keylen, outlen);
                    assert_eq!(ic, ir, "gh init rc");

                    let mut off = 0usize;
                    let mut remaining = inlen;
                    while remaining > 0 {
                        let take = 1 + rng.range(remaining);
                        let uc = c_upd(cstate.ptr(), input[off..].as_ptr(), take as u64);
                        let ur = r_upd(rstate.ptr(), input[off..].as_ptr(), take as u64);
                        assert_eq!(uc, ur, "gh update rc");
                        off += take;
                        remaining -= take;
                    }
                    let mut cout = vec![0u8; outlen];
                    let mut rout = vec![0u8; outlen];
                    let fc = c_fin(cstate.ptr(), cout.as_mut_ptr(), outlen);
                    let fr = r_fin(rstate.ptr(), rout.as_mut_ptr(), outlen);
                    assert_eq!(fc, fr, "gh final rc");
                    assert_eq!(cout, rout, "gh streaming out outlen={} keylen={} inlen={}", outlen, keylen, inlen);

                    let mut os = vec![0u8; outlen];
                    c_os(os.as_mut_ptr(), outlen, input.as_ptr(), inlen as u64, kptr, keylen);
                    assert_eq!(cout, os, "gh streaming != oneshot");
                }
            }
        }
    }
}

#[test]
fn generichash_salt_personal() {
    let l = libs();
    let mut rng = Rng::new(0x5A17);
    unsafe {
        let (c_sp, r_sp) = sympair!(l, b"crypto_generichash_blake2b_salt_personal", GhSaltPersonal);
        let (c_b2, _r) = sympair!(l, b"crypto_generichash_blake2b", GhOneShot);

        for &outlen in &[16usize, 32, 64] {
            for &keylen in &[0usize, 16, 32, 64] {
                for &inlen in &[0usize, 1, 128, 500] {
                    let input = rng.vec(inlen);
                    let key = rng.vec(keylen);
                    let salt = rng.vec(16);
                    let personal = rng.vec(16);
                    let kptr = if keylen == 0 { std::ptr::null() } else { key.as_ptr() };

                    let mut co = vec![0u8; outlen];
                    let mut ro = vec![0u8; outlen];
                    let rc = c_sp(co.as_mut_ptr(), outlen, input.as_ptr(), inlen as u64, kptr, keylen, salt.as_ptr(), personal.as_ptr());
                    let rr = r_sp(ro.as_mut_ptr(), outlen, input.as_ptr(), inlen as u64, kptr, keylen, salt.as_ptr(), personal.as_ptr());
                    assert_eq!(rc, rr, "salt_personal rc");
                    assert_eq!(co, ro, "salt_personal out outlen={} keylen={} inlen={}", outlen, keylen, inlen);

                    // With NULL salt & personal, must equal plain blake2b (C zero-fills).
                    let mut cn = vec![0u8; outlen];
                    let mut rn = vec![0u8; outlen];
                    c_sp(cn.as_mut_ptr(), outlen, input.as_ptr(), inlen as u64, kptr, keylen, std::ptr::null(), std::ptr::null());
                    r_sp(rn.as_mut_ptr(), outlen, input.as_ptr(), inlen as u64, kptr, keylen, std::ptr::null(), std::ptr::null());
                    assert_eq!(cn, rn, "salt_personal null out");
                    let mut plain = vec![0u8; outlen];
                    c_b2(plain.as_mut_ptr(), outlen, input.as_ptr(), inlen as u64, kptr, keylen);
                    assert_eq!(cn, plain, "salt_personal(null,null) != blake2b");
                }
            }
        }
    }
}

#[test]
fn generichash_init_salt_personal_streaming() {
    let l = libs();
    let mut rng = Rng::new(0x151A17);
    unsafe {
        let (c_sb, _r_sb) = sympair!(l, b"crypto_generichash_statebytes", SizeFn);
        let sb = c_sb();
        let (c_init, r_init) = sympair!(l, b"crypto_generichash_blake2b_init_salt_personal", GhInitSP);
        let (c_upd, r_upd) = sympair!(l, b"crypto_generichash_blake2b_update", GhUpdate);
        let (c_fin, r_fin) = sympair!(l, b"crypto_generichash_blake2b_final", GhFinal);
        let (c_sp, _r_sp) = sympair!(l, b"crypto_generichash_blake2b_salt_personal", GhSaltPersonal);

        for &outlen in &[16usize, 32, 64] {
            for &keylen in &[0usize, 32, 64] {
                for &inlen in &[0usize, 100, 1000] {
                    let input = rng.vec(inlen);
                    let key = rng.vec(keylen);
                    let salt = rng.vec(16);
                    let personal = rng.vec(16);
                    let kptr = if keylen == 0 { std::ptr::null() } else { key.as_ptr() };

                    let mut cstate = AlignedState::new(sb);
                    let mut rstate = AlignedState::new(sb);
                    let ic = c_init(cstate.ptr(), kptr, keylen, outlen, salt.as_ptr(), personal.as_ptr());
                    let ir = r_init(rstate.ptr(), kptr, keylen, outlen, salt.as_ptr(), personal.as_ptr());
                    assert_eq!(ic, ir, "init_sp rc");
                    let uc = c_upd(cstate.ptr(), input.as_ptr(), inlen as u64);
                    let ur = r_upd(rstate.ptr(), input.as_ptr(), inlen as u64);
                    assert_eq!(uc, ur);
                    let mut cout = vec![0u8; outlen];
                    let mut rout = vec![0u8; outlen];
                    c_fin(cstate.ptr(), cout.as_mut_ptr(), outlen);
                    r_fin(rstate.ptr(), rout.as_mut_ptr(), outlen);
                    assert_eq!(cout, rout, "init_sp streaming out");

                    // equals salt_personal one-shot
                    let mut os = vec![0u8; outlen];
                    c_sp(os.as_mut_ptr(), outlen, input.as_ptr(), inlen as u64, kptr, keylen, salt.as_ptr(), personal.as_ptr());
                    assert_eq!(cout, os, "init_sp streaming != salt_personal oneshot");
                }
            }
        }
    }
}

// ---- shorthash ----------------------------------------------------------

fn check_shorthash(name: &[u8], outbytes: usize) {
    let l = libs();
    let mut rng = Rng::new(0x517A11 ^ name.len() as u64);
    unsafe {
        let (c, r) = sympair!(l, name, ShortHash);
        for &len in lengths().iter() {
            for _ in 0..4 {
                let input = rng.vec(len);
                let key = rng.vec(16);
                let mut co = vec![0u8; outbytes];
                let mut ro = vec![0u8; outbytes];
                let rc = c(co.as_mut_ptr(), input.as_ptr(), len as u64, key.as_ptr());
                let rr = r(ro.as_mut_ptr(), input.as_ptr(), len as u64, key.as_ptr());
                assert_eq!(rc, rr, "{} rc", String::from_utf8_lossy(name));
                assert_eq!(co, ro, "{} out len={}", String::from_utf8_lossy(name), len);
            }
        }
    }
}

#[test]
fn shorthash_siphash24() {
    check_shorthash(b"crypto_shorthash", 8);
    check_shorthash(b"crypto_shorthash_siphash24", 8);
}

#[test]
fn shorthash_siphashx24() {
    check_shorthash(b"crypto_shorthash_siphashx24", 16);
}

// ---- XOF ----------------------------------------------------------------

struct XofDef {
    prefix: &'static str,
    blockbytes: usize,
}

fn xof_defs() -> Vec<XofDef> {
    vec![
        XofDef { prefix: "crypto_xof_shake128", blockbytes: 168 },
        XofDef { prefix: "crypto_xof_shake256", blockbytes: 136 },
        XofDef { prefix: "crypto_xof_turboshake128", blockbytes: 168 },
        XofDef { prefix: "crypto_xof_turboshake256", blockbytes: 136 },
    ]
}

#[test]
fn xof_oneshot() {
    let l = libs();
    for d in xof_defs() {
        let mut rng = Rng::new(0x0F ^ d.prefix.len() as u64);
        unsafe {
            let (c, r) = sympair!(l, d.prefix.as_bytes(), XofOneShot);
            // varied outlens including block boundaries and large
            let outlens = [0usize, 1, 16, 32, d.blockbytes - 1, d.blockbytes, d.blockbytes + 1, 2 * d.blockbytes, 500, 1000];
            for &outlen in &outlens {
                for &inlen in &[0usize, 1, 100, d.blockbytes, d.blockbytes + 1, 1000] {
                    let input = rng.vec(inlen);
                    let mut co = vec![0u8; outlen];
                    let mut ro = vec![0u8; outlen];
                    let rc = c(co.as_mut_ptr(), outlen, input.as_ptr(), inlen as u64);
                    let rr = r(ro.as_mut_ptr(), outlen, input.as_ptr(), inlen as u64);
                    assert_eq!(rc, rr, "{} rc outlen={} inlen={}", d.prefix, outlen, inlen);
                    assert_eq!(co, ro, "{} out outlen={} inlen={}", d.prefix, outlen, inlen);
                }
            }
        }
    }
}

#[test]
fn xof_streaming_equiv() {
    let l = libs();
    for d in xof_defs() {
        let mut rng = Rng::new(0xEE ^ d.prefix.len() as u64);
        let init_n = format!("{}_init", d.prefix);
        let upd_n = format!("{}_update", d.prefix);
        let sq_n = format!("{}_squeeze", d.prefix);
        let sb_n = format!("{}_statebytes", d.prefix);
        unsafe {
            let (c_sb, r_sb) = sympair!(l, sb_n.as_bytes(), SizeFn);
            assert_eq!(c_sb(), r_sb(), "{} statebytes differ", d.prefix);
            let sb = c_sb();
            let (c_init, r_init) = sympair!(l, init_n.as_bytes(), XofInit);
            let (c_upd, r_upd) = sympair!(l, upd_n.as_bytes(), XofUpdate);
            let (c_sq, r_sq) = sympair!(l, sq_n.as_bytes(), XofSqueeze);
            let (c_os, _r_os) = sympair!(l, d.prefix.as_bytes(), XofOneShot);

            for &inlen in &[0usize, 1, 50, d.blockbytes, d.blockbytes + 5, 1000] {
                for &total_out in &[0usize, 1, 32, d.blockbytes + 7, 500] {
                    let input = rng.vec(inlen);

                    let mut cstate = AlignedState::new(sb);
                    let mut rstate = AlignedState::new(sb);
                    assert_eq!(c_init(cstate.ptr()), r_init(rstate.ptr()), "{} init rc", d.prefix);

                    // update in random chunks
                    let mut off = 0usize;
                    let mut remaining = inlen;
                    while remaining > 0 {
                        let take = 1 + rng.range(remaining);
                        let uc = c_upd(cstate.ptr(), input[off..].as_ptr(), take as u64);
                        let ur = r_upd(rstate.ptr(), input[off..].as_ptr(), take as u64);
                        assert_eq!(uc, ur, "{} update rc", d.prefix);
                        off += take;
                        remaining -= take;
                    }

                    // squeeze incrementally in random chunks
                    let mut cout = vec![0u8; total_out];
                    let mut rout = vec![0u8; total_out];
                    let mut so = 0usize;
                    let mut srem = total_out;
                    while srem > 0 {
                        let take = 1 + rng.range(srem);
                        let sc = c_sq(cstate.ptr(), cout[so..].as_mut_ptr(), take);
                        let sr = r_sq(rstate.ptr(), rout[so..].as_mut_ptr(), take);
                        assert_eq!(sc, sr, "{} squeeze rc", d.prefix);
                        so += take;
                        srem -= take;
                    }
                    assert_eq!(cout, rout, "{} streaming out inlen={} out={}", d.prefix, inlen, total_out);

                    // Incremental squeeze must equal one-shot output.
                    let mut os = vec![0u8; total_out];
                    c_os(os.as_mut_ptr(), total_out, input.as_ptr(), inlen as u64);
                    assert_eq!(cout, os, "{} streaming != oneshot inlen={} out={}", d.prefix, inlen, total_out);
                }
            }
        }
    }
}

#[test]
fn xof_init_with_domain() {
    let l = libs();
    for d in xof_defs() {
        let mut rng = Rng::new(0xD0 ^ d.prefix.len() as u64);
        let init_n = format!("{}_init_with_domain", d.prefix);
        let upd_n = format!("{}_update", d.prefix);
        let sq_n = format!("{}_squeeze", d.prefix);
        let sb_n = format!("{}_statebytes", d.prefix);
        let dom_n = format!("{}_domain_standard", d.prefix);
        unsafe {
            let (c_sb, _r_sb) = sympair!(l, sb_n.as_bytes(), SizeFn);
            let sb = c_sb();
            let (c_dom, r_dom) = sympair!(l, dom_n.as_bytes(), unsafe extern "C" fn() -> u8);
            assert_eq!(c_dom(), r_dom(), "{} domain_standard differ", d.prefix);

            let (c_init, r_init) = sympair!(l, init_n.as_bytes(), XofInitDomain);
            let (c_upd, r_upd) = sympair!(l, upd_n.as_bytes(), XofUpdate);
            let (c_sq, r_sq) = sympair!(l, sq_n.as_bytes(), XofSqueeze);

            for &domain in &[0x01u8, 0x06, 0x07, 0x1f, 0x80, 0xff, c_dom()] {
                for &inlen in &[0usize, 1, 200] {
                    let input = rng.vec(inlen);
                    let mut cstate = AlignedState::new(sb);
                    let mut rstate = AlignedState::new(sb);
                    assert_eq!(c_init(cstate.ptr(), domain), r_init(rstate.ptr(), domain), "{} init_dom rc", d.prefix);
                    c_upd(cstate.ptr(), input.as_ptr(), inlen as u64);
                    r_upd(rstate.ptr(), input.as_ptr(), inlen as u64);
                    let mut cout = vec![0u8; 100];
                    let mut rout = vec![0u8; 100];
                    c_sq(cstate.ptr(), cout.as_mut_ptr(), 100);
                    r_sq(rstate.ptr(), rout.as_mut_ptr(), 100);
                    assert_eq!(cout, rout, "{} domain={} out mismatch", d.prefix, domain);
                }
            }
        }
    }
}

// =========================================================================
// PHASE C — error paths
// =========================================================================

#[test]
fn generichash_bad_outlen() {
    let l = libs();
    let mut rng = Rng::new(0xE44);
    unsafe {
        let (c, r) = sympair!(l, b"crypto_generichash", GhOneShot);
        let input = rng.vec(64);
        // outlen 0 and >64 must both return -1.
        for &outlen in &[0usize, 65, 100, 1000] {
            let cap = outlen.max(1);
            let mut co = vec![0u8; cap];
            let mut ro = vec![0u8; cap];
            let rc = c(co.as_mut_ptr(), outlen, input.as_ptr(), 64, std::ptr::null(), 0);
            let rr = r(ro.as_mut_ptr(), outlen, input.as_ptr(), 64, std::ptr::null(), 0);
            assert_eq!(rc, -1, "C should reject outlen={}", outlen);
            assert_eq!(rc, rr, "generichash outlen={} rc mismatch", outlen);
        }
    }
}

#[test]
fn generichash_bad_keylen() {
    let l = libs();
    let mut rng = Rng::new(0xE45);
    unsafe {
        let (c, r) = sympair!(l, b"crypto_generichash", GhOneShot);
        let input = rng.vec(32);
        for &keylen in &[65usize, 100, 200] {
            let key = rng.vec(keylen);
            let mut co = [0u8; 32];
            let mut ro = [0u8; 32];
            let rc = c(co.as_mut_ptr(), 32, input.as_ptr(), 32, key.as_ptr(), keylen);
            let rr = r(ro.as_mut_ptr(), 32, input.as_ptr(), 32, key.as_ptr(), keylen);
            assert_eq!(rc, -1, "C should reject keylen={}", keylen);
            assert_eq!(rc, rr, "generichash keylen={} rc mismatch", keylen);
        }
    }
}

#[test]
fn generichash_init_bad_params() {
    let l = libs();
    unsafe {
        let (c_sb, _r) = sympair!(l, b"crypto_generichash_statebytes", SizeFn);
        let sb = c_sb();
        let (c_init, r_init) = sympair!(l, b"crypto_generichash_init", GhInit);
        // outlen 0, outlen>64, keylen>64 must all return -1
        let cases: [(usize, usize); 4] = [(0, 0), (65, 0), (32, 65), (0, 65)];
        for &(outlen, keylen) in &cases {
            let key = vec![0u8; keylen.max(1)];
            let kptr = if keylen == 0 { std::ptr::null() } else { key.as_ptr() };
            let mut cstate = AlignedState::new(sb);
            let mut rstate = AlignedState::new(sb);
            let rc = c_init(cstate.ptr(), kptr, keylen, outlen);
            let rr = r_init(rstate.ptr(), kptr, keylen, outlen);
            assert_eq!(rc, -1, "C init should reject outlen={} keylen={}", outlen, keylen);
            assert_eq!(rc, rr, "init rc mismatch outlen={} keylen={}", outlen, keylen);
        }
    }
}

#[test]
fn generichash_salt_personal_bad_params() {
    let l = libs();
    unsafe {
        let (c, r) = sympair!(l, b"crypto_generichash_blake2b_salt_personal", GhSaltPersonal);
        let salt = [0u8; 16];
        let personal = [0u8; 16];
        let input = [0u8; 16];
        for &outlen in &[0usize, 65] {
            let cap = outlen.max(1);
            let mut co = vec![0u8; cap];
            let mut ro = vec![0u8; cap];
            let rc = c(co.as_mut_ptr(), outlen, input.as_ptr(), 16, std::ptr::null(), 0, salt.as_ptr(), personal.as_ptr());
            let rr = r(ro.as_mut_ptr(), outlen, input.as_ptr(), 16, std::ptr::null(), 0, salt.as_ptr(), personal.as_ptr());
            assert_eq!(rc, -1, "C salt_personal reject outlen={}", outlen);
            assert_eq!(rc, rr, "salt_personal outlen={} rc mismatch", outlen);
        }
        // keylen > 64
        let key = [0u8; 65];
        let mut co = [0u8; 32];
        let mut ro = [0u8; 32];
        let rc = c(co.as_mut_ptr(), 32, input.as_ptr(), 16, key.as_ptr(), 65, salt.as_ptr(), personal.as_ptr());
        let rr = r(ro.as_mut_ptr(), 32, input.as_ptr(), 16, key.as_ptr(), 65, salt.as_ptr(), personal.as_ptr());
        assert_eq!(rc, -1);
        assert_eq!(rc, rr, "salt_personal keylen reject mismatch");
    }
}

#[test]
fn sha3_double_final_returns_error() {
    // After finalize, a second final (without re-init) hits the FINALIZED phase
    // and returns -1 in C. Verify Rust matches.
    let l = libs();
    for (prefix, outbytes, sb_sym) in [
        ("crypto_hash_sha3256", 32usize, "crypto_hash_sha3256_statebytes"),
        ("crypto_hash_sha3512", 64usize, "crypto_hash_sha3512_statebytes"),
    ] {
        let init_n = format!("{}_init", prefix);
        let fin_n = format!("{}_final", prefix);
        unsafe {
            let (c_sb, _r) = sympair!(l, sb_sym.as_bytes(), SizeFn);
            let sb = c_sb();
            let (c_init, r_init) = sympair!(l, init_n.as_bytes(), HashInit);
            let (c_fin, r_fin) = sympair!(l, fin_n.as_bytes(), HashFinal);

            let mut cstate = vec![0u8; sb];
            let mut rstate = vec![0u8; sb];
            c_init(cstate.as_mut_ptr());
            r_init(rstate.as_mut_ptr());
            let mut cout = vec![0u8; outbytes];
            let mut rout = vec![0u8; outbytes];
            let c1 = c_fin(cstate.as_mut_ptr(), cout.as_mut_ptr());
            let r1 = r_fin(rstate.as_mut_ptr(), rout.as_mut_ptr());
            assert_eq!(c1, r1, "{} first final rc", prefix);
            // second final: phase != absorbing -> returns -1
            let c2 = c_fin(cstate.as_mut_ptr(), cout.as_mut_ptr());
            let r2 = r_fin(rstate.as_mut_ptr(), rout.as_mut_ptr());
            assert_eq!(c2, r2, "{} second final rc mismatch", prefix);
            assert_eq!(cout, rout, "{} second final out mismatch", prefix);
        }
    }
}

#[test]
fn sha3_update_after_final_returns_error() {
    // Calling update after final (FINALIZED phase) returns -1 in C.
    let l = libs();
    for (prefix, outbytes, sb_sym) in [
        ("crypto_hash_sha3256", 32usize, "crypto_hash_sha3256_statebytes"),
        ("crypto_hash_sha3512", 64usize, "crypto_hash_sha3512_statebytes"),
    ] {
        let init_n = format!("{}_init", prefix);
        let upd_n = format!("{}_update", prefix);
        let fin_n = format!("{}_final", prefix);
        unsafe {
            let (c_sb, _r) = sympair!(l, sb_sym.as_bytes(), SizeFn);
            let sb = c_sb();
            let (c_init, r_init) = sympair!(l, init_n.as_bytes(), HashInit);
            let (c_upd, r_upd) = sympair!(l, upd_n.as_bytes(), HashUpdate);
            let (c_fin, r_fin) = sympair!(l, fin_n.as_bytes(), HashFinal);

            let mut cstate = vec![0u8; sb];
            let mut rstate = vec![0u8; sb];
            c_init(cstate.as_mut_ptr());
            r_init(rstate.as_mut_ptr());
            let data = [1u8, 2, 3, 4];
            c_upd(cstate.as_mut_ptr(), data.as_ptr(), 4);
            r_upd(rstate.as_mut_ptr(), data.as_ptr(), 4);
            let mut cout = vec![0u8; outbytes];
            let mut rout = vec![0u8; outbytes];
            c_fin(cstate.as_mut_ptr(), cout.as_mut_ptr());
            r_fin(rstate.as_mut_ptr(), rout.as_mut_ptr());
            // update after final
            let cu = c_upd(cstate.as_mut_ptr(), data.as_ptr(), 4);
            let ru = r_upd(rstate.as_mut_ptr(), data.as_ptr(), 4);
            assert_eq!(cu, ru, "{} update-after-final rc mismatch", prefix);
            // resulting state must still hash identically
            let mut cf = vec![0u8; outbytes];
            let mut rf = vec![0u8; outbytes];
            let c2 = c_fin(cstate.as_mut_ptr(), cf.as_mut_ptr());
            let r2 = r_fin(rstate.as_mut_ptr(), rf.as_mut_ptr());
            assert_eq!(c2, r2);
            assert_eq!(cf, rf, "{} post-recovery out mismatch", prefix);
        }
    }
}

#[test]
fn xof_update_after_squeeze_returns_error() {
    // Once squeezing has begun, update returns -1 (phase != absorbing) in C.
    let l = libs();
    for d in xof_defs() {
        let init_n = format!("{}_init", d.prefix);
        let upd_n = format!("{}_update", d.prefix);
        let sq_n = format!("{}_squeeze", d.prefix);
        let sb_n = format!("{}_statebytes", d.prefix);
        unsafe {
            let (c_sb, _r) = sympair!(l, sb_n.as_bytes(), SizeFn);
            let sb = c_sb();
            let (c_init, r_init) = sympair!(l, init_n.as_bytes(), XofInit);
            let (c_upd, r_upd) = sympair!(l, upd_n.as_bytes(), XofUpdate);
            let (c_sq, r_sq) = sympair!(l, sq_n.as_bytes(), XofSqueeze);

            let mut cstate = AlignedState::new(sb);
            let mut rstate = AlignedState::new(sb);
            c_init(cstate.ptr());
            r_init(rstate.ptr());
            let data = [9u8; 10];
            c_upd(cstate.ptr(), data.as_ptr(), 10);
            r_upd(rstate.ptr(), data.as_ptr(), 10);
            let mut co = [0u8; 16];
            let mut ro = [0u8; 16];
            c_sq(cstate.ptr(), co.as_mut_ptr(), 16);
            r_sq(rstate.ptr(), ro.as_mut_ptr(), 16);
            // update after squeeze
            let cu = c_upd(cstate.ptr(), data.as_ptr(), 10);
            let ru = r_upd(rstate.ptr(), data.as_ptr(), 10);
            assert_eq!(cu, ru, "{} update-after-squeeze rc mismatch", d.prefix);
            // continue squeeze -> outputs still match
            let mut co2 = [0u8; 32];
            let mut ro2 = [0u8; 32];
            c_sq(cstate.ptr(), co2.as_mut_ptr(), 32);
            r_sq(rstate.ptr(), ro2.as_mut_ptr(), 32);
            assert_eq!(co2, ro2, "{} post-error squeeze out mismatch", d.prefix);
        }
    }
}

// =========================================================================
// helpers
// =========================================================================

/// Heap buffer aligned to 64 bytes, for state structures the C API requires to
/// be aligned (blake2b state must be 64-byte aligned; keccak states 16).
struct AlignedState {
    ptr: *mut u8,
    layout: std::alloc::Layout,
}

impl AlignedState {
    fn new(size: usize) -> Self {
        let layout = std::alloc::Layout::from_size_align(size.max(1), 64).unwrap();
        let ptr = unsafe { std::alloc::alloc_zeroed(layout) };
        assert!(!ptr.is_null());
        AlignedState { ptr, layout }
    }
    fn ptr(&mut self) -> *mut u8 {
        self.ptr
    }
}

impl Drop for AlignedState {
    fn drop(&mut self) {
        unsafe { std::alloc::dealloc(self.ptr, self.layout) }
    }
}
