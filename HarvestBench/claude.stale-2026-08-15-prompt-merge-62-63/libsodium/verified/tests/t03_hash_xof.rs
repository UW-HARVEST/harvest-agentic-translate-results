//! t03 — differential verification of the hash / XOF / keccak / salsa-core surface.
//!
//! Covers CONFIGS.md rows **54–82** and ERRORS.md rows **81–83** and **237–244**.
//! Every call goes through `dlsym` on BOTH the C `libsodium.so` and the Rust
//! `liblibsodium.so`; the Rust crate is never linked or called directly.
//!
//! Row → test-function map (see the report at the bottom of each section):
//!   54            -> r54_e81_e83_crypto_verify_16_32_64
//!   55,56         -> r55_r56_shorthash_siphash24_and_x24
//!   57            -> r57_shorthash_dispatch_and_constants
//!   58            -> r58_sha256_oneshot
//!   59            -> r59_sha256_single_update
//!   60            -> r60_sha256_multi_update_chunking
//!   61            -> r61_sha512_oneshot
//!   62            -> r62_sha512_streaming_chunking
//!   63            -> r63_crypto_hash_dispatch
//!   64            -> r64_sha3256_oneshot
//!   65            -> r65_sha3512_oneshot
//!   66            -> r66_sha3256_streaming
//!   67            -> r67_sha3512_streaming
//!   68..71        -> r68_r71_xof_oneshot
//!   72            -> r72_xof_multi_squeeze
//!   73            -> r73_xof_absorb_chunking
//!   74            -> r74_e243_xof_init_with_domain
//!   75            -> r75_xof_padding_branch_axis
//!   76            -> r76_xof_blockbytes_statebytes_domain
//!   77,78,79      -> r77_r79_core_salsa20_2012_208
//!   80            -> r80_core_hsalsa20
//!   81            -> r81_core_hchacha20
//!   82            -> r82_core_keccak1600
//!   ERRORS 81-83  -> r54_e81_e83_crypto_verify_16_32_64
//!   ERRORS 237,238-> e237_e238_sha2_update_zero_and_double_final
//!   ERRORS 239-241-> e239_e240_e241_sha3_phase_and_oneshot_returns
//!   ERRORS 242    -> e242_xof_update_after_squeeze
//!   ERRORS 243    -> r74_e243_xof_init_with_domain
//!   ERRORS 244    -> e244_xof_squeeze_zero_finalizes

mod common;
use common::*;
use libc::{c_char, c_int};
use libloading::Library;
use std::ffi::CStr;

// ---------------------------------------------------------------- FFI aliases

type SizeFn = unsafe extern "C" fn() -> usize;
type StrFn = unsafe extern "C" fn() -> *const c_char;
type ByteFn = unsafe extern "C" fn() -> u8;

type HashOneShot = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
type HashInit = unsafe extern "C" fn(*mut u8) -> c_int;
type HashUpdate = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
type HashFinal = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;

type XofOneShot = unsafe extern "C" fn(*mut u8, usize, *const u8, u64) -> c_int;
type XofInit = unsafe extern "C" fn(*mut u8) -> c_int;
type XofInitDom = unsafe extern "C" fn(*mut u8, u8) -> c_int;
type XofUpdate = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
type XofSqueeze = unsafe extern "C" fn(*mut u8, *mut u8, usize) -> c_int;

type ShortHashFn = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> c_int;
type VerifyFn = unsafe extern "C" fn(*const u8, *const u8) -> c_int;
type CoreFn = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8) -> c_int;

type KecVoid = unsafe extern "C" fn(*mut u8);
type KecXor = unsafe extern "C" fn(*mut u8, *const u8, usize, usize);
type KecExtract = unsafe extern "C" fn(*const u8, *mut u8, usize, usize);

// ------------------------------------------------------------ opaque state buf
//
// Every opaque state is allocated as an over-sized, 64-byte-aligned buffer
// pre-filled with 0xAA. Both libraries therefore see an IDENTICAL starting
// layout, and comparing the whole buffer afterwards catches any divergence in
// which bytes get written (including state overruns and padding-byte writes).
//
// Largest state in this file: crypto_hash_sha3*_state / crypto_xof_*_state = 256.

const SB: usize = 512;
/// Guard bytes appended to every output buffer; must survive untouched.
const GUARD: usize = 32;

#[repr(C, align(64))]
struct StBuf([u8; SB]);

fn new_state() -> Box<StBuf> {
    Box::new(StBuf([0xAA; SB]))
}

/// Result of running a scripted sequence of calls against one library.
struct Run {
    rets: Vec<c_int>,
    outs: Vec<Vec<u8>>,
    state: Vec<u8>,
}

// ------------------------------------------------------------------- utilities

fn assert_size(name: &str, expect: usize) -> usize {
    let (c, r) = unsafe { pair::<SizeFn>(name) };
    let (cv, rv) = unsafe { (c(), r()) };
    assert_eq!(cv, rv, "{name}(): C={cv} rust={rv}");
    assert_eq!(cv, expect, "{name}(): C returned {cv}, spec says {expect}");
    cv
}

fn assert_byte_const(name: &str, expect: u8) {
    let (c, r) = unsafe { pair::<ByteFn>(name) };
    let (cv, rv) = unsafe { (c(), r()) };
    assert_eq!(cv, rv, "{name}(): C={cv:#04x} rust={rv:#04x}");
    assert_eq!(cv, expect, "{name}(): C returned {cv:#04x}, spec says {expect:#04x}");
}

fn assert_cstr(name: &str, expect: &str) {
    let (c, r) = unsafe { pair::<StrFn>(name) };
    let (cs, rs) = unsafe { (CStr::from_ptr(c()), CStr::from_ptr(r())) };
    assert_eq!(cs, rs, "{name}(): C={cs:?} rust={rs:?}");
    assert_eq!(cs.to_str().unwrap(), expect, "{name}(): C returned {cs:?}");
}

fn lens_with(extra: &[usize]) -> Vec<usize> {
    let mut v: Vec<usize> = LENS.iter().copied().chain(extra.iter().copied()).collect();
    v.sort_unstable();
    v.dedup();
    v
}

/// Guard-region check on the C side: the reference implementation must not have
/// written past the requested length (and Rust == C is asserted separately).
fn assert_guard(what: &str, buf: &[u8]) {
    let n = buf.len() - GUARD;
    for (i, &b) in buf[n..].iter().enumerate() {
        assert_eq!(
            b, 0xAA,
            "{what}: byte {} past the requested {n}-byte output was overwritten \
             (0xAA -> {b:#04x}); full buffer = {}",
            n + i,
            hexs(buf)
        );
    }
}

/// Random chunk decomposition of `msg` (may contain 0-length chunks).
fn rand_chunks(rng: &mut Rng, msg: &[u8]) -> Vec<Vec<u8>> {
    let mut out: Vec<Vec<u8>> = Vec::new();
    if rng.next_u32() & 1 == 0 {
        out.push(Vec::new());
    }
    let mut i = 0usize;
    while i < msg.len() {
        let n = 1 + rng.below(msg.len() - i);
        out.push(msg[i..i + n].to_vec());
        i += n;
        if rng.next_u32() & 3 == 0 {
            out.push(Vec::new());
        }
    }
    if rng.next_u32() & 1 == 0 {
        out.push(Vec::new());
    }
    out
}

/// Chunk decomposition biased to land exactly on / around a block boundary so
/// the `offset == rate`, `offset == rate-1`, exact-fill and `while` branches are
/// all reached.
fn biased_chunks(rng: &mut Rng, msg: &[u8], blk: usize) -> Vec<Vec<u8>> {
    let cands = [
        0usize,
        1,
        2,
        blk / 2,
        blk - 2,
        blk - 1,
        blk,
        blk + 1,
        blk + 2,
        2 * blk,
        2 * blk + 1,
    ];
    let mut out: Vec<Vec<u8>> = Vec::new();
    let mut i = 0usize;
    let mut stall = 0usize;
    while i < msg.len() {
        let mut n = *rng.pick(&cands);
        if n == 0 {
            stall += 1;
            if stall > 2 {
                n = 1;
            }
        } else {
            stall = 0;
        }
        if n > msg.len() - i {
            n = msg.len() - i;
        }
        out.push(msg[i..i + n].to_vec());
        i += n;
    }
    if out.is_empty() {
        out.push(Vec::new());
    }
    out
}

// -------------------------------------------------- scripted hash-state driver

#[derive(Clone)]
enum HOp {
    Upd(Vec<u8>),
    Fin,
}

fn describe_h(ops: &[HOp]) -> String {
    let mut s = String::new();
    for op in ops {
        if !s.is_empty() {
            s.push(',');
        }
        match op {
            HOp::Upd(v) => s.push_str(&format!("upd({})", v.len())),
            HOp::Fin => s.push_str("fin"),
        }
    }
    if s.is_empty() {
        s.push_str("<init only>");
    }
    s
}

unsafe fn hash_run(lib: &'static Library, prefix: &str, outlen: usize, ops: &[HOp]) -> Run {
    let init = sym::<HashInit>(lib, &format!("{prefix}_init"));
    let upd = sym::<HashUpdate>(lib, &format!("{prefix}_update"));
    let fin = sym::<HashFinal>(lib, &format!("{prefix}_final"));
    let mut st = new_state();
    let sp = st.0.as_mut_ptr();
    let mut rets = Vec::new();
    let mut outs = Vec::new();
    rets.push(init(sp));
    for op in ops {
        match op {
            HOp::Upd(v) => rets.push(upd(sp, v.as_ptr(), v.len() as u64)),
            HOp::Fin => {
                let mut o = vec![0xAAu8; outlen + GUARD];
                rets.push(fin(sp, o.as_mut_ptr()));
                outs.push(o);
            }
        }
    }
    Run { rets, outs, state: st.0.to_vec() }
}

/// Run the same script through both libraries and assert full agreement on
/// return codes, every output buffer (including the 0xAA guard region) and the
/// entire opaque state buffer.
fn hash_cmp(what: &str, prefix: &str, outlen: usize, ops: &[HOp]) -> Run {
    let l = libs();
    let a = unsafe { hash_run(&l.c, prefix, outlen, ops) };
    let b = unsafe { hash_run(&l.r, prefix, outlen, ops) };
    let tag = format!("{what} {prefix} [{}]", describe_h(ops));
    assert_eq!(
        a.rets, b.rets,
        "{tag}: RETURN-CODE SEQUENCE MISMATCH\n  C   ={:?}\n  rust={:?}",
        a.rets, b.rets
    );
    assert_eq!(a.outs.len(), b.outs.len(), "{tag}: number of outputs differs");
    for i in 0..a.outs.len() {
        assert_guard(&format!("{tag} output #{i} (C)"), &a.outs[i]);
        assert_eq_bytes(&format!("{tag} output #{i}"), &a.outs[i], &b.outs[i]);
    }
    assert_eq_bytes(&format!("{tag} OPAQUE STATE"), &a.state, &b.state);
    a
}

fn hash_oneshot_cmp(what: &str, name: &str, outlen: usize, msg: &[u8]) -> Vec<u8> {
    let (c, r) = unsafe { pair::<HashOneShot>(name) };
    let mut oc = vec![0xAAu8; outlen + GUARD];
    let mut or = vec![0xAAu8; outlen + GUARD];
    let rc = unsafe { c(oc.as_mut_ptr(), msg.as_ptr(), msg.len() as u64) };
    let rr = unsafe { r(or.as_mut_ptr(), msg.as_ptr(), msg.len() as u64) };
    let tag = format!("{what} {name}(mlen={})", msg.len());
    assert_eq!(rc, rr, "{tag}: return C={rc} rust={rr}");
    assert_eq!(rc, 0, "{tag}: C returned {rc}, spec says 0");
    assert_guard(&format!("{tag} (C)"), &oc);
    assert_eq_bytes(&tag, &oc, &or);
    oc
}

// --------------------------------------------------- scripted xof-state driver

#[derive(Clone)]
enum XOp {
    Upd(Vec<u8>),
    Sq(usize),
}

fn describe_x(ops: &[XOp]) -> String {
    let mut s = String::new();
    let mut sq_run: Option<(usize, usize)> = None; // (len, count) for run-length folding
    let flush = |s: &mut String, r: Option<(usize, usize)>| {
        if let Some((n, c)) = r {
            if !s.is_empty() {
                s.push(',');
            }
            if c == 1 {
                s.push_str(&format!("sq({n})"));
            } else {
                s.push_str(&format!("sq({n})x{c}"));
            }
        }
    };
    for op in ops {
        match op {
            XOp::Sq(n) => {
                sq_run = match sq_run {
                    Some((m, c)) if m == *n => Some((m, c + 1)),
                    other => {
                        flush(&mut s, other);
                        Some((*n, 1))
                    }
                };
            }
            XOp::Upd(v) => {
                flush(&mut s, sq_run.take());
                if !s.is_empty() {
                    s.push(',');
                }
                s.push_str(&format!("upd({})", v.len()));
            }
        }
    }
    flush(&mut s, sq_run.take());
    if s.is_empty() {
        s.push_str("<init only>");
    }
    s
}

unsafe fn xof_run(lib: &'static Library, name: &str, domain: Option<u8>, ops: &[XOp]) -> Run {
    let mut st = new_state();
    let sp = st.0.as_mut_ptr();
    let mut rets = Vec::new();
    match domain {
        None => {
            let f = sym::<XofInit>(lib, &format!("crypto_xof_{name}_init"));
            rets.push(f(sp));
        }
        Some(d) => {
            let f = sym::<XofInitDom>(lib, &format!("crypto_xof_{name}_init_with_domain"));
            rets.push(f(sp, d));
        }
    }
    let upd = sym::<XofUpdate>(lib, &format!("crypto_xof_{name}_update"));
    let sq = sym::<XofSqueeze>(lib, &format!("crypto_xof_{name}_squeeze"));
    let mut outs = Vec::new();
    for op in ops {
        match op {
            XOp::Upd(v) => rets.push(upd(sp, v.as_ptr(), v.len() as u64)),
            XOp::Sq(n) => {
                let mut o = vec![0xAAu8; *n + GUARD];
                rets.push(sq(sp, o.as_mut_ptr(), *n));
                outs.push(o);
            }
        }
    }
    Run { rets, outs, state: st.0.to_vec() }
}

fn xof_cmp(what: &str, name: &str, domain: Option<u8>, ops: &[XOp]) -> Run {
    let l = libs();
    let a = unsafe { xof_run(&l.c, name, domain, ops) };
    let b = unsafe { xof_run(&l.r, name, domain, ops) };
    let tag = format!(
        "{what} crypto_xof_{name} dom={} [{}]",
        match domain {
            None => "std".to_string(),
            Some(d) => format!("{d:#04x}"),
        },
        describe_x(ops)
    );
    assert_eq!(
        a.rets, b.rets,
        "{tag}: RETURN-CODE SEQUENCE MISMATCH\n  C   ={:?}\n  rust={:?}",
        a.rets, b.rets
    );
    assert_eq!(a.outs.len(), b.outs.len(), "{tag}: number of outputs differs");
    for i in 0..a.outs.len() {
        assert_guard(&format!("{tag} output #{i} (C)"), &a.outs[i]);
        assert_eq_bytes(&format!("{tag} output #{i}"), &a.outs[i], &b.outs[i]);
    }
    assert_eq_bytes(&format!("{tag} OPAQUE STATE"), &a.state, &b.state);
    a
}

/// Concatenate every squeezed chunk of `run`, trimming the 0xAA guard.
fn squeezed(run: &Run, ops: &[XOp]) -> Vec<u8> {
    let mut v = Vec::new();
    let mut i = 0usize;
    for op in ops {
        if let XOp::Sq(n) = op {
            v.extend_from_slice(&run.outs[i][..*n]);
            i += 1;
        }
    }
    v
}

fn xof_oneshot_cmp(what: &str, name: &str, outlen: usize, msg: &[u8]) -> Vec<u8> {
    let (c, r) = unsafe { pair::<XofOneShot>(&format!("crypto_xof_{name}")) };
    let mut oc = vec![0xAAu8; outlen + GUARD];
    let mut or = vec![0xAAu8; outlen + GUARD];
    let rc = unsafe { c(oc.as_mut_ptr(), outlen, msg.as_ptr(), msg.len() as u64) };
    let rr = unsafe { r(or.as_mut_ptr(), outlen, msg.as_ptr(), msg.len() as u64) };
    let tag = format!("{what} crypto_xof_{name}(outlen={outlen},inlen={})", msg.len());
    assert_eq!(rc, rr, "{tag}: return C={rc} rust={rr}");
    assert_eq!(rc, 0, "{tag}: C returned {rc}, spec says 0");
    assert_guard(&format!("{tag} (C)"), &oc);
    assert_eq_bytes(&tag, &oc, &or);
    oc
}

/// The four XOF variants: (name, rate).
const XOFS: &[(&str, usize)] = &[
    ("shake128", 168),
    ("shake256", 136),
    ("turboshake128", 168),
    ("turboshake256", 136),
];

/// Domain bytes from CONFIGS row 74 / ERRORS row 243 — no validation anywhere.
const DOMAINS: &[u8] = &[0x00, 0x01, 0x06, 0x07, 0x1F, 0x7F, 0x80, 0xFF];

// ===========================================================================
// CONFIGS row 54 + ERRORS rows 81, 82, 83 — crypto_verify_16 / _32 / _64
// ===========================================================================

#[test]
fn r54_e81_e83_crypto_verify_16_32_64() {
    init_both();
    let mut rng = Rng::new(SEED);
    let mut iters = 0usize;

    for (name, n) in [
        ("crypto_verify_16", 16usize),
        ("crypto_verify_32", 32),
        ("crypto_verify_64", 64),
    ] {
        assert_size(&format!("{name}_bytes"), n);
        let (c, r) = unsafe { pair::<VerifyFn>(name) };

        for _ in 0..64 {
            let a = rng.bytes(n);

            // Equal buffers -> 0 (ERRORS 81-83: "0 if equal").
            let b = a.clone();
            let (rc, rr) = unsafe { (c(a.as_ptr(), b.as_ptr()), r(a.as_ptr(), b.as_ptr())) };
            assert_eq!(rc, rr, "{name} equal: C={rc} rust={rr} x={}", hexs(&a));
            assert_eq!(rc, 0, "{name} equal: C returned {rc}, spec says 0");
            iters += 1;

            // Differ at EVERY byte position, with a random non-zero delta -> -1.
            for i in 0..n {
                let mut y = a.clone();
                let mut d = rng.byte();
                if d == 0 {
                    d = 1;
                }
                y[i] ^= d;
                let (rc, rr) = unsafe { (c(a.as_ptr(), y.as_ptr()), r(a.as_ptr(), y.as_ptr())) };
                assert_eq!(
                    rc, rr,
                    "{name}: differ at byte {i} (delta {d:#04x}) C={rc} rust={rr}\n  x={}\n  y={}",
                    hexs(&a),
                    hexs(&y)
                );
                assert_eq!(rc, -1, "{name}: differ at byte {i}: C returned {rc}, spec says -1");
                // and the reversed argument order
                let (rc2, rr2) = unsafe { (c(y.as_ptr(), a.as_ptr()), r(y.as_ptr(), a.as_ptr())) };
                assert_eq!(rc2, rr2, "{name}: reversed args, byte {i}: C={rc2} rust={rr2}");
                iters += 1;
            }
        }

        // all-0x00 vs all-0xff, both orders, plus each all-same pair.
        let z = vec![0u8; n];
        let f = vec![0xffu8; n];
        for (x, y, exp) in [
            (&z, &f, -1),
            (&f, &z, -1),
            (&z, &z, 0),
            (&f, &f, 0),
        ] {
            let (rc, rr) = unsafe { (c(x.as_ptr(), y.as_ptr()), r(x.as_ptr(), y.as_ptr())) };
            assert_eq!(rc, rr, "{name} 0x00/0xff pattern: C={rc} rust={rr}");
            assert_eq!(rc, exp, "{name} 0x00/0xff pattern: C returned {rc}, expected {exp}");
            iters += 1;
        }
    }
    assert!(iters >= 64, "row 54 drove only {iters} inputs");
    eprintln!("row 54 / ERRORS 81-83: {iters} comparisons");
}

// ===========================================================================
// CONFIGS rows 55, 56 — crypto_shorthash_siphash24 / _siphashx24
// ===========================================================================

#[test]
fn r55_r56_shorthash_siphash24_and_x24() {
    init_both();
    let mut rng = Rng::new(SEED ^ 0x55);
    let mut iters = 0usize;

    assert_size("crypto_shorthash_siphash24_bytes", 8);
    assert_size("crypto_shorthash_siphash24_keybytes", 16);
    assert_size("crypto_shorthash_siphashx24_bytes", 16);
    assert_size("crypto_shorthash_siphashx24_keybytes", 16);

    let row_lens: &[usize] = &[0, 1, 7, 8, 9, 15, 16, 17, 63, 64, 65, 1000];
    let lens = lens_with(row_lens);

    for (name, outlen) in [
        ("crypto_shorthash_siphash24", 8usize),
        ("crypto_shorthash_siphashx24", 16),
    ] {
        let (c, r) = unsafe { pair::<ShortHashFn>(name) };
        let keys = patterns(16, &mut rng);
        for k in &keys {
            for &n in &lens {
                let msg = rng.bytes(n);
                let mut oc = vec![0xAAu8; outlen + GUARD];
                let mut or = vec![0xAAu8; outlen + GUARD];
                let rc = unsafe { c(oc.as_mut_ptr(), msg.as_ptr(), n as u64, k.as_ptr()) };
                let rr = unsafe { r(or.as_mut_ptr(), msg.as_ptr(), n as u64, k.as_ptr()) };
                let tag = format!("{name}(inlen={n}, key={})", hexs(k));
                assert_eq!(rc, rr, "{tag}: return C={rc} rust={rr}");
                assert_eq!(rc, 0, "{tag}: C returned {rc}, expected 0");
                assert_guard(&format!("{tag} (C)"), &oc);
                assert_eq_bytes(&tag, &oc, &or);
                iters += 1;
            }
        }
        // All-0x00 / all-0xff messages at every block-straddling length.
        for pat in [0x00u8, 0xff] {
            for &n in row_lens {
                let msg = vec![pat; n];
                let k = rng.bytes(16);
                let mut oc = vec![0xAAu8; outlen + GUARD];
                let mut or = vec![0xAAu8; outlen + GUARD];
                unsafe { c(oc.as_mut_ptr(), msg.as_ptr(), n as u64, k.as_ptr()) };
                unsafe { r(or.as_mut_ptr(), msg.as_ptr(), n as u64, k.as_ptr()) };
                assert_guard(&format!("{name} pat={pat:#04x} n={n} (C)"), &oc);
                assert_eq_bytes(&format!("{name}(msg=all {pat:#04x}, inlen={n})"), &oc, &or);
                iters += 1;
            }
        }
    }
    assert!(iters >= 64, "rows 55/56 drove only {iters} inputs");
    eprintln!("rows 55/56: {iters} comparisons");
}

// ===========================================================================
// CONFIGS row 57 — crypto_shorthash dispatch + _bytes / _keybytes / _primitive
// ===========================================================================

#[test]
fn r57_shorthash_dispatch_and_constants() {
    init_both();
    let mut rng = Rng::new(SEED ^ 0x57);
    let mut iters = 0usize;

    assert_size("crypto_shorthash_bytes", 8);
    assert_size("crypto_shorthash_keybytes", 16);
    assert_cstr("crypto_shorthash_primitive", "siphash24");

    let (dc, dr) = unsafe { pair::<ShortHashFn>("crypto_shorthash") };
    let (sc, sr) = unsafe { pair::<ShortHashFn>("crypto_shorthash_siphash24") };

    for &n in &lens_with(&[0, 1, 7, 8, 9, 15, 16, 17, 63, 64, 65, 1000]) {
        for k in patterns(16, &mut rng) {
            let msg = rng.bytes(n);
            let mut a = vec![0xAAu8; 8 + GUARD];
            let mut b = vec![0xAAu8; 8 + GUARD];
            let mut a2 = vec![0xAAu8; 8 + GUARD];
            let mut b2 = vec![0xAAu8; 8 + GUARD];
            let rc = unsafe { dc(a.as_mut_ptr(), msg.as_ptr(), n as u64, k.as_ptr()) };
            let rr = unsafe { dr(b.as_mut_ptr(), msg.as_ptr(), n as u64, k.as_ptr()) };
            unsafe { sc(a2.as_mut_ptr(), msg.as_ptr(), n as u64, k.as_ptr()) };
            unsafe { sr(b2.as_mut_ptr(), msg.as_ptr(), n as u64, k.as_ptr()) };
            let tag = format!("crypto_shorthash(inlen={n})");
            assert_eq!(rc, rr, "{tag}: return C={rc} rust={rr}");
            assert_guard(&format!("{tag} (C)"), &a);
            assert_eq_bytes(&tag, &a, &b);
            // dispatch identity, inside each library
            assert_eq_bytes(&format!("C: {tag} != siphash24"), &a, &a2);
            assert_eq_bytes(&format!("rust: {tag} != siphash24"), &b, &b2);
            iters += 1;
        }
    }
    assert!(iters >= 64, "row 57 drove only {iters} inputs");
    eprintln!("row 57: {iters} comparisons");
}

// ===========================================================================
// CONFIGS row 58 — crypto_hash_sha256 one-shot
// ===========================================================================

#[test]
fn r58_sha256_oneshot() {
    init_both();
    let mut rng = Rng::new(SEED ^ 0x58);
    let mut iters = 0usize;

    assert_size("crypto_hash_sha256_bytes", 32);
    // sizeof(crypto_hash_sha256_state) = 8*4 + 8 + 64
    assert_size("crypto_hash_sha256_statebytes", 104);

    // Row 58 pad-boundary set (r == 56) plus the shared sweep.
    let lens = lens_with(&[0, 1, 55, 56, 63, 64, 65, 111, 112, 119, 120, 127, 128, 191, 192, 1000]);
    for &n in &lens {
        for rep in 0..2 {
            let msg = if rep == 0 { rng.bytes(n) } else { (0..n).map(|i| i as u8).collect() };
            hash_oneshot_cmp("row58", "crypto_hash_sha256", 32, &msg);
            iters += 1;
        }
        // all-0x00 / all-0xff shapes
        for pat in [0x00u8, 0xff] {
            hash_oneshot_cmp("row58", "crypto_hash_sha256", 32, &vec![pat; n]);
            iters += 1;
        }
    }
    assert!(iters >= 64, "row 58 drove only {iters} inputs");
    eprintln!("row 58: {iters} one-shot comparisons");
}

// ===========================================================================
// CONFIGS row 59 — crypto_hash_sha256_init/_update/_final, single update
// ===========================================================================

#[test]
fn r59_sha256_single_update() {
    init_both();
    let mut rng = Rng::new(SEED ^ 0x59);
    let mut iters = 0usize;

    let lens = lens_with(&[0, 1, 55, 56, 63, 64, 65, 111, 112, 119, 120, 127, 128, 191, 192, 1000]);
    for &n in &lens {
        for _ in 0..2 {
            let msg = rng.bytes(n);
            let ops = vec![HOp::Upd(msg.clone()), HOp::Fin];
            let run = hash_cmp("row59", "crypto_hash_sha256", 32, &ops);
            // must equal the one-shot in BOTH libraries
            let one = hash_oneshot_cmp("row59", "crypto_hash_sha256", 32, &msg);
            assert_eq_bytes(
                &format!("row59 sha256 streaming(single upd, n={n}) != one-shot"),
                &one,
                &run.outs[0],
            );
            iters += 1;
        }
    }
    assert!(iters >= 64, "row 59 drove only {iters} inputs");
    eprintln!("row 59: {iters} single-update comparisons");
}

// ===========================================================================
// CONFIGS row 60 — sha256 multi-update chunking (all four branches)
// ===========================================================================

#[test]
fn r60_sha256_multi_update_chunking() {
    init_both();
    let mut rng = Rng::new(SEED ^ 0x60);
    let mut iters = 0usize;

    let lens = lens_with(&[55, 56, 63, 64, 65, 119, 120, 127, 128, 129, 191, 192, 1000]);
    for &n in &lens {
        let msg = rng.bytes(n);
        let one = hash_oneshot_cmp("row60", "crypto_hash_sha256", 32, &msg);
        for variant in 0..3 {
            let chunks = match variant {
                0 => rand_chunks(&mut rng, &msg),
                1 => biased_chunks(&mut rng, &msg, 64),
                _ => {
                    // deliberately: one byte at a time, with 0-length chunks
                    let mut v: Vec<Vec<u8>> = Vec::new();
                    for (i, b) in msg.iter().enumerate() {
                        if i % 5 == 0 {
                            v.push(Vec::new());
                        }
                        v.push(vec![*b]);
                    }
                    v.push(Vec::new());
                    v
                }
            };
            let mut ops: Vec<HOp> = chunks.into_iter().map(HOp::Upd).collect();
            ops.push(HOp::Fin);
            let run = hash_cmp("row60", "crypto_hash_sha256", 32, &ops);
            assert_eq_bytes(
                &format!("row60 sha256 chunked({}) != one-shot(n={n})", describe_h(&ops)),
                &one,
                &run.outs[0],
            );
            // every ret must be 0
            assert!(run.rets.iter().all(|&x| x == 0), "row60: C rets {:?}", run.rets);
            iters += 1;
        }
    }
    // exact-fill sequences that leave r == 0 at every step
    for k in 1..=6usize {
        let msg = rng.bytes(64 * k);
        let one = hash_oneshot_cmp("row60", "crypto_hash_sha256", 32, &msg);
        let mut ops: Vec<HOp> = (0..k).map(|i| HOp::Upd(msg[i * 64..(i + 1) * 64].to_vec())).collect();
        ops.push(HOp::Fin);
        let run = hash_cmp("row60-exactfill", "crypto_hash_sha256", 32, &ops);
        assert_eq_bytes("row60 sha256 exact-fill chunks != one-shot", &one, &run.outs[0]);
        iters += 1;
    }
    assert!(iters >= 64, "row 60 drove only {iters} inputs");
    eprintln!("row 60: {iters} chunked comparisons");
}

// ===========================================================================
// CONFIGS row 61 — crypto_hash_sha512 one-shot
// ===========================================================================

#[test]
fn r61_sha512_oneshot() {
    init_both();
    let mut rng = Rng::new(SEED ^ 0x61);
    let mut iters = 0usize;

    assert_size("crypto_hash_sha512_bytes", 64);
    // sizeof(crypto_hash_sha512_state) = 8*8 + 2*8 + 128
    assert_size("crypto_hash_sha512_statebytes", 208);

    let lens = lens_with(&[0, 1, 111, 112, 119, 120, 127, 128, 129, 239, 240, 255, 256, 1000]);
    for &n in &lens {
        for rep in 0..2 {
            let msg = if rep == 0 { rng.bytes(n) } else { (0..n).map(|i| i as u8).collect() };
            hash_oneshot_cmp("row61", "crypto_hash_sha512", 64, &msg);
            iters += 1;
        }
        for pat in [0x00u8, 0xff] {
            hash_oneshot_cmp("row61", "crypto_hash_sha512", 64, &vec![pat; n]);
            iters += 1;
        }
    }
    assert!(iters >= 64, "row 61 drove only {iters} inputs");
    eprintln!("row 61: {iters} one-shot comparisons");
}

// ===========================================================================
// CONFIGS row 62 — sha512 _init/_update/_final, single + multi update
// ===========================================================================

#[test]
fn r62_sha512_streaming_chunking() {
    init_both();
    let mut rng = Rng::new(SEED ^ 0x62);
    let mut iters = 0usize;

    let lens = lens_with(&[0, 1, 111, 112, 119, 120, 127, 128, 129, 239, 240, 255, 256, 1000]);
    for &n in &lens {
        let msg = rng.bytes(n);
        let one = hash_oneshot_cmp("row62", "crypto_hash_sha512", 64, &msg);

        // single update
        let ops = vec![HOp::Upd(msg.clone()), HOp::Fin];
        let run = hash_cmp("row62-single", "crypto_hash_sha512", 64, &ops);
        assert_eq_bytes(
            &format!("row62 sha512 single-update(n={n}) != one-shot"),
            &one,
            &run.outs[0],
        );
        iters += 1;

        // multi update: random + block-biased + byte-at-a-time
        for variant in 0..3 {
            let chunks = match variant {
                0 => rand_chunks(&mut rng, &msg),
                1 => biased_chunks(&mut rng, &msg, 128),
                _ => {
                    let mut v: Vec<Vec<u8>> = vec![Vec::new()];
                    for (i, b) in msg.iter().enumerate() {
                        v.push(vec![*b]);
                        if i % 7 == 0 {
                            v.push(Vec::new());
                        }
                    }
                    v
                }
            };
            let mut ops: Vec<HOp> = chunks.into_iter().map(HOp::Upd).collect();
            ops.push(HOp::Fin);
            let run = hash_cmp("row62-multi", "crypto_hash_sha512", 64, &ops);
            assert_eq_bytes(
                &format!("row62 sha512 chunked({}) != one-shot(n={n})", describe_h(&ops)),
                &one,
                &run.outs[0],
            );
            iters += 1;
        }
    }
    // exact 128-byte fills
    for k in 1..=5usize {
        let msg = rng.bytes(128 * k);
        let one = hash_oneshot_cmp("row62", "crypto_hash_sha512", 64, &msg);
        let mut ops: Vec<HOp> =
            (0..k).map(|i| HOp::Upd(msg[i * 128..(i + 1) * 128].to_vec())).collect();
        ops.push(HOp::Fin);
        let run = hash_cmp("row62-exactfill", "crypto_hash_sha512", 64, &ops);
        assert_eq_bytes("row62 sha512 exact-fill chunks != one-shot", &one, &run.outs[0]);
        iters += 1;
    }
    assert!(iters >= 64, "row 62 drove only {iters} inputs");
    eprintln!("row 62: {iters} streaming comparisons");
}

// ===========================================================================
// CONFIGS row 63 — crypto_hash / _bytes / _primitive dispatch to sha512
// ===========================================================================

#[test]
fn r63_crypto_hash_dispatch() {
    init_both();
    let mut rng = Rng::new(SEED ^ 0x63);
    let mut iters = 0usize;

    assert_size("crypto_hash_bytes", 64);
    assert_cstr("crypto_hash_primitive", "sha512");

    let (hc, hr) = unsafe { pair::<HashOneShot>("crypto_hash") };
    let (sc, sr) = unsafe { pair::<HashOneShot>("crypto_hash_sha512") };

    for &n in &lens_with(&[0, 1, 111, 112, 127, 128, 129, 255, 256, 1000]) {
        for rep in 0..2 {
            let msg = if rep == 0 { rng.bytes(n) } else { vec![0xffu8; n] };
            let mut a = vec![0xAAu8; 64 + GUARD];
            let mut b = vec![0xAAu8; 64 + GUARD];
            let mut a2 = vec![0xAAu8; 64 + GUARD];
            let mut b2 = vec![0xAAu8; 64 + GUARD];
            let rc = unsafe { hc(a.as_mut_ptr(), msg.as_ptr(), n as u64) };
            let rr = unsafe { hr(b.as_mut_ptr(), msg.as_ptr(), n as u64) };
            unsafe { sc(a2.as_mut_ptr(), msg.as_ptr(), n as u64) };
            unsafe { sr(b2.as_mut_ptr(), msg.as_ptr(), n as u64) };
            let tag = format!("crypto_hash(mlen={n})");
            assert_eq!(rc, rr, "{tag}: return C={rc} rust={rr}");
            assert_eq!(rc, 0, "{tag}: C returned {rc}");
            assert_guard(&format!("{tag} (C)"), &a);
            assert_eq_bytes(&tag, &a, &b);
            assert_eq_bytes(&format!("C: {tag} != crypto_hash_sha512"), &a, &a2);
            assert_eq_bytes(&format!("rust: {tag} != crypto_hash_sha512"), &b, &b2);
            iters += 1;
        }
    }
    assert!(iters >= 64, "row 63 drove only {iters} inputs");
    eprintln!("row 63: {iters} comparisons");
}

// ===========================================================================
// CONFIGS row 64 — crypto_hash_sha3256 one-shot (rate 136)
// ===========================================================================

#[test]
fn r64_sha3256_oneshot() {
    init_both();
    let mut rng = Rng::new(SEED ^ 0x64);
    let mut iters = 0usize;

    assert_size("crypto_hash_sha3256_bytes", 32);
    assert_size("crypto_hash_sha3256_statebytes", 256);

    let lens = lens_with(&[0, 1, 135, 136, 137, 143, 144, 271, 272, 1000]);
    for &n in &lens {
        for rep in 0..2 {
            let msg = if rep == 0 { rng.bytes(n) } else { (0..n).map(|i| i as u8).collect() };
            hash_oneshot_cmp("row64", "crypto_hash_sha3256", 32, &msg);
            iters += 1;
        }
        for pat in [0x00u8, 0xff] {
            hash_oneshot_cmp("row64", "crypto_hash_sha3256", 32, &vec![pat; n]);
            iters += 1;
        }
    }
    assert!(iters >= 64, "row 64 drove only {iters} inputs");
    eprintln!("row 64: {iters} one-shot comparisons");
}

// ===========================================================================
// CONFIGS row 65 — crypto_hash_sha3512 one-shot (rate 72)
// ===========================================================================

#[test]
fn r65_sha3512_oneshot() {
    init_both();
    let mut rng = Rng::new(SEED ^ 0x65);
    let mut iters = 0usize;

    assert_size("crypto_hash_sha3512_bytes", 64);
    assert_size("crypto_hash_sha3512_statebytes", 256);

    let lens = lens_with(&[0, 1, 71, 72, 73, 143, 144, 1000]);
    for &n in &lens {
        for rep in 0..2 {
            let msg = if rep == 0 { rng.bytes(n) } else { (0..n).map(|i| i as u8).collect() };
            hash_oneshot_cmp("row65", "crypto_hash_sha3512", 64, &msg);
            iters += 1;
        }
        for pat in [0x00u8, 0xff] {
            hash_oneshot_cmp("row65", "crypto_hash_sha3512", 64, &vec![pat; n]);
            iters += 1;
        }
    }
    assert!(iters >= 64, "row 65 drove only {iters} inputs");
    eprintln!("row 65: {iters} one-shot comparisons");
}

/// Shared body for CONFIGS rows 66 / 67: exercise the `offset != 0` partial fill,
/// the `while (inlen - consumed >= rate)` loop that leaves `offset == rate`, and
/// the trailing remainder — plus mandatory multi-update chunking.
fn sha3_streaming_row(row: &str, prefix: &str, outlen: usize, rate: usize, seed: u64) -> usize {
    let mut rng = Rng::new(seed);
    let mut iters = 0usize;

    let lens = lens_with(&[
        0,
        1,
        rate - 2,
        rate - 1,
        rate,
        rate + 1,
        rate + 2,
        2 * rate - 1,
        2 * rate,
        2 * rate + 1,
        3 * rate,
        1000,
    ]);

    for &n in &lens {
        let msg = rng.bytes(n);
        let one = hash_oneshot_cmp(row, prefix, outlen, &msg);

        // single update
        let ops = vec![HOp::Upd(msg.clone()), HOp::Fin];
        let run = hash_cmp(&format!("{row}-single"), prefix, outlen, &ops);
        assert_eq_bytes(
            &format!("{row} {prefix} single-update(n={n}) != one-shot"),
            &one,
            &run.outs[0],
        );
        iters += 1;

        for variant in 0..3 {
            let chunks = match variant {
                0 => rand_chunks(&mut rng, &msg),
                1 => biased_chunks(&mut rng, &msg, rate),
                _ => {
                    // deterministic straddle: rate-1 then the rest in 1-byte pieces
                    let mut v: Vec<Vec<u8>> = Vec::new();
                    let cut = (rate - 1).min(msg.len());
                    v.push(msg[..cut].to_vec());
                    v.push(Vec::new());
                    for b in &msg[cut..] {
                        v.push(vec![*b]);
                    }
                    v
                }
            };
            let mut ops: Vec<HOp> = chunks.into_iter().map(HOp::Upd).collect();
            ops.push(HOp::Fin);
            let run = hash_cmp(&format!("{row}-multi"), prefix, outlen, &ops);
            assert_eq_bytes(
                &format!("{row} {prefix} chunked({}) != one-shot(n={n})", describe_h(&ops)),
                &one,
                &run.outs[0],
            );
            iters += 1;
        }
    }

    // exact-rate fills: every update ends with offset == rate
    for k in 1..=5usize {
        let msg = rng.bytes(rate * k);
        let one = hash_oneshot_cmp(row, prefix, outlen, &msg);
        let mut ops: Vec<HOp> =
            (0..k).map(|i| HOp::Upd(msg[i * rate..(i + 1) * rate].to_vec())).collect();
        ops.push(HOp::Fin);
        let run = hash_cmp(&format!("{row}-exactrate"), prefix, outlen, &ops);
        assert_eq_bytes(
            &format!("{row} {prefix} exact-rate chunks != one-shot"),
            &one,
            &run.outs[0],
        );
        // an exact-rate update followed by a 0-length update must not change anything
        let mut ops2 = ops.clone();
        ops2.insert(ops2.len() - 1, HOp::Upd(Vec::new()));
        let run2 = hash_cmp(&format!("{row}-exactrate-zeroupd"), prefix, outlen, &ops2);
        assert_eq_bytes(
            &format!("{row} {prefix} exact-rate + 0-length update changed the digest"),
            &run.outs[0],
            &run2.outs[0],
        );
        iters += 2;
    }
    iters
}

// ===========================================================================
// CONFIGS row 66 — crypto_hash_sha3256 _init/_update/_final (rate 136)
// ===========================================================================

#[test]
fn r66_sha3256_streaming() {
    init_both();
    let n = sha3_streaming_row("row66", "crypto_hash_sha3256", 32, 136, SEED ^ 0x66);
    assert!(n >= 64, "row 66 drove only {n} inputs");
    eprintln!("row 66: {n} streaming comparisons");
}

// ===========================================================================
// CONFIGS row 67 — crypto_hash_sha3512 _init/_update/_final (rate 72)
// ===========================================================================

#[test]
fn r67_sha3512_streaming() {
    init_both();
    let n = sha3_streaming_row("row67", "crypto_hash_sha3512", 64, 72, SEED ^ 0x67);
    assert!(n >= 64, "row 67 drove only {n} inputs");
    eprintln!("row 67: {n} streaming comparisons");
}

// ===========================================================================
// ERRORS rows 237, 238 — sha256/sha512 update(inlen==0) and double _final
// ===========================================================================

#[test]
fn e237_e238_sha2_update_zero_and_double_final() {
    init_both();
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0x237);
    let mut iters = 0usize;

    for (prefix, outlen, blk) in
        [("crypto_hash_sha256", 32usize, 64usize), ("crypto_hash_sha512", 64, 128)]
    {
        // ---- ERRORS 237: update(inlen == 0) returns 0 and leaves the state alone.
        for &pre in &[0usize, 1, blk - 1, blk, blk + 1, 2 * blk + 3] {
            let msg = rng.bytes(pre);
            let base: Vec<HOp> = if pre == 0 { vec![] } else { vec![HOp::Upd(msg.clone())] };
            let mut with_zero = base.clone();
            with_zero.push(HOp::Upd(Vec::new()));

            let bc = unsafe { hash_run(&l.c, prefix, outlen, &base) };
            let zc = unsafe { hash_run(&l.c, prefix, outlen, &with_zero) };
            let br = unsafe { hash_run(&l.r, prefix, outlen, &base) };
            let zr = unsafe { hash_run(&l.r, prefix, outlen, &with_zero) };

            assert_eq!(
                *zc.rets.last().unwrap(),
                0,
                "ERRORS 237 {prefix}: C update(inlen=0) returned {:?}, spec says 0",
                zc.rets
            );
            assert_eq!(zc.rets, zr.rets, "ERRORS 237 {prefix}: rets C={:?} rust={:?}", zc.rets, zr.rets);
            assert_eq_bytes(
                &format!("ERRORS 237 C: {prefix} update(inlen=0) CHANGED the state (pre={pre})"),
                &bc.state,
                &zc.state,
            );
            assert_eq_bytes(
                &format!("ERRORS 237 rust: {prefix} update(inlen=0) CHANGED the state (pre={pre})"),
                &br.state,
                &zr.state,
            );
            assert_eq_bytes(
                &format!("ERRORS 237 {prefix} state after update(inlen=0) (pre={pre})"),
                &zc.state,
                &zr.state,
            );
            // ... and the digest is unaffected
            let mut a = base.clone();
            a.push(HOp::Fin);
            let mut b = with_zero.clone();
            b.push(HOp::Fin);
            let ra = hash_cmp("e237", prefix, outlen, &a);
            let rb = hash_cmp("e237", prefix, outlen, &b);
            assert_eq_bytes(
                &format!("ERRORS 237 {prefix}: 0-length update changed the digest (pre={pre})"),
                &ra.outs[0],
                &rb.outs[0],
            );
            iters += 1;
        }

        // ---- ERRORS 238: _final zeroizes the state, so a 2nd/3rd _final
        //      silently restarts from an all-zero state. Both libraries must
        //      agree on the return value AND on the (garbage) digest bytes.
        for &pre in &[0usize, 1, blk - 9, blk, blk + 7, 3 * blk] {
            let msg = rng.bytes(pre);
            let ops = vec![HOp::Upd(msg), HOp::Fin, HOp::Fin, HOp::Fin];
            let run = hash_cmp("e238", prefix, outlen, &ops);
            assert!(
                run.rets.iter().all(|&x| x == 0),
                "ERRORS 238 {prefix}: C rets {:?}, spec says every call returns 0",
                run.rets
            );
            // the 2nd and 3rd digests are the SAME "all-zero state" digest
            assert_eq_bytes(
                &format!("ERRORS 238 {prefix}: 2nd and 3rd _final differ"),
                &run.outs[1],
                &run.outs[2],
            );
            // ... and are NOT the real digest (proves the zeroization happened)
            assert_ne!(
                run.outs[0], run.outs[1],
                "ERRORS 238 {prefix}: 2nd _final produced the real digest, so the state \
                 was not zeroized (pre={pre})"
            );
            // the zeroized-state digest must match a fresh state that was
            // manually zeroed: init+final on a state whose fields are all zero
            // is not reachable through the API, so we only cross-check C vs Rust
            // (done by hash_cmp above).
            iters += 1;
        }
    }
    assert!(iters >= 24, "ERRORS 237/238 drove only {iters} inputs");
    eprintln!("ERRORS 237/238: {iters} scripted comparisons");
}

// ===========================================================================
// ERRORS rows 239, 240, 241 — sha3 phase != ABSORBING, and one-shot returns
// ===========================================================================

#[test]
fn e239_e240_e241_sha3_phase_and_oneshot_returns() {
    init_both();
    let mut rng = Rng::new(SEED ^ 0x239);
    let mut iters = 0usize;

    for (prefix, outlen, rate) in
        [("crypto_hash_sha3256", 32usize, 136usize), ("crypto_hash_sha3512", 64, 72)]
    {
        // ---- ERRORS 241: the one-shot discards _update/_final return values -> 0.
        for &n in &[0usize, 1, rate - 1, rate, rate + 1, 2 * rate + 5, 1000] {
            let msg = rng.bytes(n);
            hash_oneshot_cmp("e241", prefix, outlen, &msg); // asserts ret == 0 in both
            iters += 1;
        }

        // ---- ERRORS 239: _update after _final returns -1 but STILL absorbs.
        for &(a, b) in &[
            (0usize, 0usize),
            (1, 1),
            (rate - 1, 1),
            (rate, rate),
            (rate + 1, rate - 1),
            (2 * rate, 3),
            (7, 2 * rate + 1),
        ] {
            let m1 = rng.bytes(a);
            let m2 = rng.bytes(b);
            let ops = vec![HOp::Upd(m1), HOp::Fin, HOp::Upd(m2), HOp::Fin];
            let run = hash_cmp("e239", prefix, outlen, &ops);
            // rets: [init, upd, fin, upd-after-final, fin]
            // The update-after-final returns -1 but resets phase to ABSORBING,
            // so the *following* _final sees a valid phase and returns 0.
            assert_eq!(
                run.rets,
                vec![0, 0, 0, -1, 0],
                "ERRORS 239 {prefix}: C rets {:?}, spec says update-after-final == -1 \
                 (a={a} b={b})",
                run.rets
            );
            // The output of the second _final was still written (not left as 0xAA).
            assert!(
                run.outs[1][..outlen].iter().any(|&x| x != 0xAA),
                "ERRORS 239/240 {prefix}: the 2nd _final wrote nothing"
            );
            iters += 1;
        }

        // ---- ERRORS 240: _final twice returns -1 but output IS still written.
        for &n in &[0usize, 1, rate - 1, rate, rate + 1, 2 * rate] {
            let msg = rng.bytes(n);
            let ops = vec![HOp::Upd(msg), HOp::Fin, HOp::Fin, HOp::Fin];
            let run = hash_cmp("e240", prefix, outlen, &ops);
            assert_eq!(
                run.rets,
                vec![0, 0, 0, -1, -1],
                "ERRORS 240 {prefix}: C rets {:?} (n={n})",
                run.rets
            );
            assert!(
                run.outs[1][..outlen].iter().any(|&x| x != 0xAA),
                "ERRORS 240 {prefix}: 2nd _final wrote nothing (n={n})"
            );
            assert!(
                run.outs[2][..outlen].iter().any(|&x| x != 0xAA),
                "ERRORS 240 {prefix}: 3rd _final wrote nothing (n={n})"
            );
            iters += 1;
        }

        // A 0-length update after _final also flips the phase back to ABSORBING
        // (the `phase != ABSORBING` branch runs before the `inlen > 0` guards).
        for &n in &[0usize, 1, rate - 1, rate, rate + 1] {
            let msg = rng.bytes(n);
            let ops = vec![HOp::Upd(msg), HOp::Fin, HOp::Upd(Vec::new()), HOp::Fin];
            let run = hash_cmp("e239-zerolen", prefix, outlen, &ops);
            assert_eq!(
                run.rets,
                vec![0, 0, 0, -1, 0],
                "ERRORS 239 {prefix}: 0-length update after _final: C rets {:?} — the phase \
                 reset must happen before the inlen>0 guard, so the following _final returns 0",
                run.rets
            );
            iters += 1;
        }
    }
    assert!(iters >= 50, "ERRORS 239-241 drove only {iters} inputs");
    eprintln!("ERRORS 239/240/241: {iters} scripted comparisons");
}

// ===========================================================================
// CONFIGS rows 68-71 — one-shot shake128 / shake256 / turboshake128 / _256
// ===========================================================================

#[test]
fn r68_r71_xof_oneshot() {
    init_both();
    let mut rng = Rng::new(SEED ^ 0x68);
    let mut total = 0usize;

    for &(name, rate) in XOFS {
        let mut iters = 0usize;
        let outs: Vec<usize> =
            vec![0, 1, 7, rate - 1, rate, rate + 1, 2 * rate, 2 * rate + 1, 3 * rate, 1000];
        let ins: Vec<usize> = vec![0, 1, rate - 1, rate, rate + 1, 2 * rate + 1, 1000];
        for &ol in &outs {
            for &il in &ins {
                let msg = rng.bytes(il);
                let one = xof_oneshot_cmp("rows68-71", name, ol, &msg);
                // the streaming API must reproduce the one-shot in both libs
                let ops = vec![XOp::Upd(msg.clone()), XOp::Sq(ol)];
                let run = xof_cmp("rows68-71", name, None, &ops);
                assert_eq_bytes(
                    &format!("crypto_xof_{name} one-shot != init/update/squeeze (out={ol},in={il})"),
                    &one,
                    &run.outs[0],
                );
                iters += 1;
            }
        }
        assert!(iters >= 64, "crypto_xof_{name} one-shot drove only {iters} inputs");
        total += iters;
    }
    eprintln!("rows 68-71: {total} one-shot comparisons");
}

// ===========================================================================
// CONFIGS row 72 — multi-squeeze equivalences
// ===========================================================================

#[test]
fn r72_xof_multi_squeeze() {
    init_both();
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0x72);
    let mut iters = 0usize;

    for &(name, rate) in XOFS {
        for &mlen in &[0usize, 1, rate - 1, rate, rate + 1, 2 * rate + 3] {
            let msg = rng.bytes(mlen);

            // (a) squeeze(rate) x 2  ==  squeeze(2*rate)
            let a = vec![XOp::Upd(msg.clone()), XOp::Sq(rate), XOp::Sq(rate)];
            let b = vec![XOp::Upd(msg.clone()), XOp::Sq(2 * rate)];
            let ra = xof_cmp("row72a", name, None, &a);
            let rb = xof_cmp("row72a", name, None, &b);
            assert_eq_bytes(
                &format!("row72 {name}: squeeze({rate})x2 != squeeze({}) [C]", 2 * rate),
                &squeezed(&ra, &a),
                &squeezed(&rb, &b),
            );
            // ... and independently inside each library
            let ac = unsafe { xof_run(&l.c, name, None, &a) };
            let bc = unsafe { xof_run(&l.c, name, None, &b) };
            let ar = unsafe { xof_run(&l.r, name, None, &a) };
            let br = unsafe { xof_run(&l.r, name, None, &b) };
            assert_eq_bytes(
                &format!("row72 C {name}: squeeze({rate})x2 != squeeze({})", 2 * rate),
                &squeezed(&ac, &a),
                &squeezed(&bc, &b),
            );
            assert_eq_bytes(
                &format!("row72 rust {name}: squeeze({rate})x2 != squeeze({})", 2 * rate),
                &squeezed(&ar, &a),
                &squeezed(&br, &b),
            );
            iters += 1;

            // (b) squeeze(1) x N  ==  squeeze(N)   (resumes mid-block)
            for &n in &[1usize, 2, 3, 7, 8, rate - 1, rate, rate + 1, rate + 2, 2 * rate + 1] {
                let mut ones = vec![XOp::Upd(msg.clone())];
                ones.extend((0..n).map(|_| XOp::Sq(1)));
                let big = vec![XOp::Upd(msg.clone()), XOp::Sq(n)];
                let r1 = xof_cmp("row72b", name, None, &ones);
                let r2 = xof_cmp("row72b", name, None, &big);
                assert_eq_bytes(
                    &format!("row72 {name}: squeeze(1)x{n} != squeeze({n}) [C]"),
                    &squeezed(&r1, &ones),
                    &squeezed(&r2, &big),
                );
                let c1 = unsafe { xof_run(&l.c, name, None, &ones) };
                let c2 = unsafe { xof_run(&l.c, name, None, &big) };
                let s1 = unsafe { xof_run(&l.r, name, None, &ones) };
                let s2 = unsafe { xof_run(&l.r, name, None, &big) };
                assert_eq_bytes(
                    &format!("row72 C {name}: squeeze(1)x{n} != squeeze({n})"),
                    &squeezed(&c1, &ones),
                    &squeezed(&c2, &big),
                );
                assert_eq_bytes(
                    &format!("row72 rust {name}: squeeze(1)x{n} != squeeze({n})"),
                    &squeezed(&s1, &ones),
                    &squeezed(&s2, &big),
                );
                iters += 1;
            }

            // (c) a random sequence of squeezes == one big squeeze
            for _ in 0..2 {
                let mut sizes = Vec::new();
                let mut tot = 0usize;
                let target = 2 * rate + 37;
                while tot < target {
                    let mut k = rng.below(rate + 5);
                    if k == 0 && rng.next_u32() & 1 == 0 {
                        k = 1;
                    }
                    if tot + k > target {
                        k = target - tot;
                    }
                    sizes.push(k);
                    tot += k;
                }
                let mut many = vec![XOp::Upd(msg.clone())];
                many.extend(sizes.iter().map(|&k| XOp::Sq(k)));
                let one = vec![XOp::Upd(msg.clone()), XOp::Sq(target)];
                let rm = xof_cmp("row72c", name, None, &many);
                let ro = xof_cmp("row72c", name, None, &one);
                assert_eq_bytes(
                    &format!("row72 {name}: split squeeze {sizes:?} != squeeze({target})"),
                    &squeezed(&rm, &many),
                    &squeezed(&ro, &one),
                );
                iters += 1;
            }
        }
    }
    assert!(iters >= 64, "row 72 drove only {iters} inputs");
    eprintln!("row 72: {iters} multi-squeeze comparisons");
}

// ===========================================================================
// CONFIGS row 73 — absorb chunking straddling a rate boundary
// ===========================================================================

#[test]
fn r73_xof_absorb_chunking() {
    init_both();
    let mut rng = Rng::new(SEED ^ 0x73);
    let mut iters = 0usize;

    for &(name, rate) in XOFS {
        let lens = lens_with(&[
            0,
            1,
            rate - 2,
            rate - 1,
            rate,
            rate + 1,
            rate + 2,
            2 * rate - 1,
            2 * rate,
            2 * rate + 1,
            3 * rate + 5,
            1000,
        ]);
        let ol = 2 * rate + 7;
        for &n in &lens {
            let msg = rng.bytes(n);
            let one = xof_oneshot_cmp("row73", name, ol, &msg);
            for variant in 0..3 {
                let chunks = match variant {
                    0 => rand_chunks(&mut rng, &msg),
                    1 => biased_chunks(&mut rng, &msg, rate),
                    _ => {
                        let mut v: Vec<Vec<u8>> = Vec::new();
                        let cut = (rate - 1).min(msg.len());
                        v.push(msg[..cut].to_vec());
                        v.push(Vec::new());
                        for b in &msg[cut..] {
                            v.push(vec![*b]);
                        }
                        v.push(Vec::new());
                        v
                    }
                };
                let mut ops: Vec<XOp> = chunks.into_iter().map(XOp::Upd).collect();
                ops.push(XOp::Sq(ol));
                let run = xof_cmp("row73", name, None, &ops);
                assert_eq_bytes(
                    &format!(
                        "row73 crypto_xof_{name} chunked({}) != one-shot(n={n})",
                        describe_x(&ops)
                    ),
                    &one,
                    &run.outs[0],
                );
                assert!(
                    run.rets.iter().all(|&x| x == 0),
                    "row73 {name}: C rets {:?}, all absorbs before the first squeeze must be 0",
                    run.rets
                );
                iters += 1;
            }
        }
        // exact-rate absorb chunks (each update leaves offset == rate)
        for k in 1..=4usize {
            let msg = rng.bytes(rate * k);
            let one = xof_oneshot_cmp("row73", name, ol, &msg);
            let mut ops: Vec<XOp> =
                (0..k).map(|i| XOp::Upd(msg[i * rate..(i + 1) * rate].to_vec())).collect();
            ops.push(XOp::Sq(ol));
            let run = xof_cmp("row73-exactrate", name, None, &ops);
            assert_eq_bytes(
                &format!("row73 crypto_xof_{name} exact-rate absorb != one-shot"),
                &one,
                &run.outs[0],
            );
            iters += 1;
        }
    }
    assert!(iters >= 64, "row 73 drove only {iters} inputs");
    eprintln!("row 73: {iters} absorb-chunking comparisons");
}

// ===========================================================================
// CONFIGS row 74 + ERRORS row 243 — _init_with_domain, any domain byte
// ===========================================================================

#[test]
fn r74_e243_xof_init_with_domain() {
    init_both();
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0x74);
    let mut iters = 0usize;

    for &(name, rate) in XOFS {
        for &d in DOMAINS {
            for &mlen in &[0usize, 1, rate - 1, rate, rate + 1, 2 * rate + 3] {
                let msg = rng.bytes(mlen);
                let ops = vec![XOp::Upd(msg.clone()), XOp::Sq(2 * rate + 11)];
                let run = xof_cmp("row74", name, Some(d), &ops);
                // ERRORS 243: every domain byte 0x00..0xFF is accepted, ret 0.
                assert_eq!(
                    run.rets[0], 0,
                    "ERRORS 243 crypto_xof_{name}_init_with_domain({d:#04x}): C returned {}, \
                     spec says 0 (no validation)",
                    run.rets[0]
                );
                assert!(
                    run.rets.iter().all(|&x| x == 0),
                    "row74 {name} dom={d:#04x}: C rets {:?}",
                    run.rets
                );
                iters += 1;
            }
        }

        // domain 0x1F == the standard domain, so _init_with_domain(0x1F) must be
        // byte-identical to _init in both libraries.
        for &mlen in &[0usize, 1, rate - 1, rate, rate + 1] {
            let msg = rng.bytes(mlen);
            let ops = vec![XOp::Upd(msg.clone()), XOp::Sq(rate + 5)];
            let std_c = unsafe { xof_run(&l.c, name, None, &ops) };
            let dom_c = unsafe { xof_run(&l.c, name, Some(0x1F), &ops) };
            let std_r = unsafe { xof_run(&l.r, name, None, &ops) };
            let dom_r = unsafe { xof_run(&l.r, name, Some(0x1F), &ops) };
            assert_eq_bytes(
                &format!("row74 C {name}: _init != _init_with_domain(0x1F)"),
                &std_c.outs[0],
                &dom_c.outs[0],
            );
            assert_eq_bytes(
                &format!("row74 rust {name}: _init != _init_with_domain(0x1F)"),
                &std_r.outs[0],
                &dom_r.outs[0],
            );
            iters += 1;
        }

        // Distinct domains must give distinct output (sanity: the domain byte is
        // really absorbed). 0x80 collides with the pad bit, so it is included.
        let msg = rng.bytes(rate + 3);
        let ops = vec![XOp::Upd(msg), XOp::Sq(32)];
        let mut seen: Vec<(u8, Vec<u8>)> = Vec::new();
        for &d in DOMAINS {
            let run = xof_cmp("row74-distinct", name, Some(d), &ops);
            for (pd, prev) in &seen {
                assert_ne!(
                    *prev, run.outs[0],
                    "row74 crypto_xof_{name}: domain {pd:#04x} and {d:#04x} produced the same \
                     output — the domain byte is not being absorbed"
                );
            }
            seen.push((d, run.outs[0].clone()));
            iters += 1;
        }
    }
    assert!(iters >= 64, "row 74 drove only {iters} inputs");
    eprintln!("row 74 / ERRORS 243: {iters} comparisons");
}

// ===========================================================================
// CONFIGS row 75 — padding-branch axis: offset == rate-1 / < rate-1 / == rate
// ===========================================================================

#[test]
fn r75_xof_padding_branch_axis() {
    init_both();
    let mut rng = Rng::new(SEED ^ 0x75);
    let mut iters = 0usize;

    for &(name, rate) in XOFS {
        // Scripts engineered to leave a specific `offset` at finalize time.
        // (a) offset == rate-1      -> single combined `domain ^ 0x80` byte
        // (b) offset <  rate-1      -> two separate xor_bytes calls
        // (c) offset == rate        -> extra permute first, then offset 0
        let scripts: Vec<(&str, Vec<usize>)> = vec![
            ("offset==rate-1 (one update)", vec![rate - 1]),
            ("offset==rate-1 (split)", vec![rate - 3, 2]),
            ("offset==rate-1 (after wrap)", vec![rate, rate - 1]),
            ("offset<rate-1 (empty)", vec![]),
            ("offset<rate-1 (1)", vec![1]),
            ("offset<rate-1 (rate-2)", vec![rate - 2]),
            ("offset<rate-1 (rate+1 -> 1)", vec![rate + 1]),
            ("offset<rate-1 (2*rate+3 -> 3)", vec![2 * rate + 3]),
            ("offset==rate (one update)", vec![rate]),
            ("offset==rate (split rate-1 + 1)", vec![rate - 1, 1]),
            ("offset==rate (2*rate)", vec![2 * rate]),
            ("offset==rate (rate then rate)", vec![rate, rate]),
        ];
        for (label, sizes) in &scripts {
            for &d in DOMAINS {
                let mut ops: Vec<XOp> = sizes.iter().map(|&k| XOp::Upd(rng.bytes(k))).collect();
                ops.push(XOp::Sq(rate + 9));
                let run = xof_cmp(&format!("row75 [{label}]"), name, Some(d), &ops);
                assert!(
                    run.rets.iter().all(|&x| x == 0),
                    "row75 {name} {label} dom={d:#04x}: C rets {:?}",
                    run.rets
                );
                iters += 1;
            }
        }
    }
    assert!(iters >= 64, "row 75 drove only {iters} inputs");
    eprintln!("row 75: {iters} padding-branch comparisons");
}

// ===========================================================================
// CONFIGS row 76 — _blockbytes / _statebytes / _domain_standard
// ===========================================================================

#[test]
fn r76_xof_blockbytes_statebytes_domain() {
    init_both();
    for &(name, rate) in XOFS {
        assert_size(&format!("crypto_xof_{name}_blockbytes"), rate);
        assert_size(&format!("crypto_xof_{name}_statebytes"), 256);
        assert_byte_const(&format!("crypto_xof_{name}_domain_standard"), 0x1F);
    }
    // The oversized state buffer used everywhere in this file must be >= the
    // library's own statebytes(), otherwise the comparisons above are unsound.
    for &(name, _) in XOFS {
        let (c, _) = unsafe { pair::<SizeFn>(&format!("crypto_xof_{name}_statebytes")) };
        assert!(unsafe { c() } <= SB, "state buffer too small for {name}");
    }
    eprintln!("row 76: 4 x (blockbytes, statebytes, domain_standard) verified");
}

// ===========================================================================
// ERRORS row 242 — _update after _squeeze returns -1 but still absorbs
// ===========================================================================

#[test]
fn e242_xof_update_after_squeeze() {
    init_both();
    let mut rng = Rng::new(SEED ^ 0x242);
    let mut iters = 0usize;

    for &(name, rate) in XOFS {
        for &(a, s, b) in &[
            (0usize, 1usize, 0usize),
            (1, 1, 1),
            (rate - 1, rate, 1),
            (rate, rate, rate),
            (rate + 1, 1, rate - 1),
            (2 * rate, 3, 2 * rate + 1),
            (7, 2 * rate + 5, 7),
            (0, 0, rate), // squeeze(0) also finalizes -> phase == SQUEEZING
        ] {
            let m1 = rng.bytes(a);
            let m2 = rng.bytes(b);
            let ops = vec![XOp::Upd(m1), XOp::Sq(s), XOp::Upd(m2), XOp::Sq(rate + 3)];
            let run = xof_cmp("e242", name, None, &ops);
            // rets: [init, upd, squeeze, upd-after-squeeze, squeeze]
            assert_eq!(
                run.rets,
                vec![0, 0, 0, -1, 0],
                "ERRORS 242 crypto_xof_{name}_update after _squeeze: C rets {:?}, spec says the \
                 4th call returns -1 (a={a} s={s} b={b})",
                run.rets
            );
            // The post-squeeze update still absorbed: the following squeeze must
            // differ from what a plain re-squeeze would have produced.
            let plain = vec![XOp::Upd(Vec::new()), XOp::Sq(rate + 3)];
            let pr = xof_cmp("e242-ref", name, None, &plain);
            assert_ne!(
                run.outs[1], pr.outs[0],
                "ERRORS 242 {name}: the post-squeeze update appears not to have been absorbed"
            );
            iters += 1;
        }
    }
    assert!(iters >= 32, "ERRORS 242 drove only {iters} inputs");
    eprintln!("ERRORS 242: {iters} scripted comparisons");
}

// ===========================================================================
// ERRORS row 244 — _squeeze(outlen == 0) still finalizes when ABSORBING
// ===========================================================================

#[test]
fn e244_xof_squeeze_zero_finalizes() {
    init_both();
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0x244);
    let mut iters = 0usize;

    for &(name, rate) in XOFS {
        for &mlen in &[0usize, 1, rate - 2, rate - 1, rate, rate + 1, 2 * rate + 3] {
            let msg = rng.bytes(mlen);

            // squeeze(0) returns 0, writes nothing, but DOES finalize.
            let zero_then = vec![XOp::Upd(msg.clone()), XOp::Sq(0), XOp::Sq(rate + 5)];
            let plain = vec![XOp::Upd(msg.clone()), XOp::Sq(rate + 5)];
            let rz = xof_cmp("e244", name, None, &zero_then);
            let rp = xof_cmp("e244", name, None, &plain);
            assert_eq!(
                rz.rets,
                vec![0, 0, 0, 0],
                "ERRORS 244 crypto_xof_{name}_squeeze(outlen=0): C rets {:?}, spec says 0",
                rz.rets
            );
            // nothing written into the outlen==0 buffer (guard region only)
            assert!(
                rz.outs[0].iter().all(|&x| x == 0xAA),
                "ERRORS 244 {name}: squeeze(outlen=0) wrote bytes: {}",
                hexs(&rz.outs[0])
            );
            // the following squeeze is unaffected by the empty one
            assert_eq_bytes(
                &format!("ERRORS 244 {name}: squeeze(0)+squeeze(n) != squeeze(n) (mlen={mlen})"),
                &rp.outs[0],
                &rz.outs[1],
            );

            // ... and squeeze(0) really did finalize: the state changed.
            let absorbed_only = vec![XOp::Upd(msg.clone())];
            let zero_only = vec![XOp::Upd(msg.clone()), XOp::Sq(0)];
            for (libname, lib) in [("C", &l.c), ("rust", &l.r)] {
                let s0 = unsafe { xof_run(lib, name, None, &absorbed_only) };
                let s1 = unsafe { xof_run(lib, name, None, &zero_only) };
                assert_ne!(
                    s0.state, s1.state,
                    "ERRORS 244 {libname} {name}: squeeze(outlen=0) did not finalize the state \
                     (mlen={mlen})"
                );
            }
            // and both libraries agree on that finalized state
            let sc = unsafe { xof_run(&l.c, name, None, &zero_only) };
            let sr = unsafe { xof_run(&l.r, name, None, &zero_only) };
            assert_eq_bytes(
                &format!("ERRORS 244 {name}: state after squeeze(0)"),
                &sc.state,
                &sr.state,
            );

            // squeeze(0) while already SQUEEZING is a plain no-op in both libs.
            let sq0_again = vec![XOp::Upd(msg.clone()), XOp::Sq(3), XOp::Sq(0), XOp::Sq(rate)];
            let sq0_absent = vec![XOp::Upd(msg.clone()), XOp::Sq(3), XOp::Sq(rate)];
            let ra = xof_cmp("e244-again", name, None, &sq0_again);
            let rb = xof_cmp("e244-again", name, None, &sq0_absent);
            assert_eq_bytes(
                &format!("ERRORS 244 {name}: squeeze(0) mid-squeeze changed the stream"),
                &squeezed(&rb, &sq0_absent),
                &squeezed(&ra, &sq0_again),
            );
            iters += 1;
        }
    }
    assert!(iters >= 28, "ERRORS 244 drove only {iters} inputs");
    eprintln!("ERRORS 244: {iters} scripted comparisons");
}

// ===========================================================================
// CONFIGS rows 77, 78, 79 — crypto_core_salsa20 / _salsa2012 / _salsa208
// ===========================================================================

/// Constant blocks for the `c` argument: NULL, the implicit sigma, and explicit
/// 16-byte patterns.
fn const_blocks(rng: &mut Rng) -> Vec<Option<Vec<u8>>> {
    vec![
        None,
        Some(b"expand 32-byte k".to_vec()), // == the c == NULL default
        Some(vec![0u8; 16]),
        Some(vec![0xffu8; 16]),
        Some((0..16u8).collect()),
        Some(rng.bytes(16)),
        Some(rng.bytes(16)),
    ]
}

fn core_row(row: &str, name: &str, outlen: usize, seed: u64) -> usize {
    let mut rng = Rng::new(seed);
    let mut iters = 0usize;
    let (c, r) = unsafe { pair::<CoreFn>(name) };

    let keys = patterns(32, &mut rng);
    let ins = patterns(16, &mut rng);
    let consts = const_blocks(&mut rng);

    // Reference outputs for the c == NULL case, per (key, in), so the "sigma is
    // the default" property can be checked separately in each library.
    for (ki, k) in keys.iter().enumerate() {
        for (ii, inp) in ins.iter().enumerate() {
            let mut null_c: Option<Vec<u8>> = None;
            let mut null_r: Option<Vec<u8>> = None;
            for (ci, cb) in consts.iter().enumerate() {
                let cp: *const u8 = match cb {
                    None => std::ptr::null(),
                    Some(v) => v.as_ptr(),
                };
                let mut oc = vec![0xAAu8; outlen + GUARD];
                let mut or = vec![0xAAu8; outlen + GUARD];
                let rc = unsafe { c(oc.as_mut_ptr(), inp.as_ptr(), k.as_ptr(), cp) };
                let rr = unsafe { r(or.as_mut_ptr(), inp.as_ptr(), k.as_ptr(), cp) };
                let tag = format!(
                    "{row} {name}(k=#{ki} {}, in=#{ii} {}, c=#{ci} {})",
                    hexs(k),
                    hexs(inp),
                    match cb {
                        None => "NULL".to_string(),
                        Some(v) => hexs(v),
                    }
                );
                assert_eq!(rc, rr, "{tag}: return C={rc} rust={rr}");
                assert_eq!(rc, 0, "{tag}: C returned {rc}, expected 0");
                assert_guard(&format!("{tag} (C)"), &oc);
                assert_eq_bytes(&tag, &oc, &or);
                if ci == 0 {
                    null_c = Some(oc.clone());
                    null_r = Some(or.clone());
                } else if ci == 1 {
                    // explicit sigma must equal the c == NULL default
                    assert_eq_bytes(
                        &format!("{row} C {name}: c=NULL != c=\"expand 32-byte k\""),
                        null_c.as_ref().unwrap(),
                        &oc,
                    );
                    assert_eq_bytes(
                        &format!("{row} rust {name}: c=NULL != c=\"expand 32-byte k\""),
                        null_r.as_ref().unwrap(),
                        &or,
                    );
                }
                iters += 1;
            }
        }
    }

    // Extra fully-random sweep.
    for _ in 0..24 {
        let k = rng.bytes(32);
        let inp = rng.bytes(16);
        let cb = rng.bytes(16);
        for cp in [std::ptr::null(), cb.as_ptr()] {
            let mut oc = vec![0xAAu8; outlen + GUARD];
            let mut or = vec![0xAAu8; outlen + GUARD];
            unsafe { c(oc.as_mut_ptr(), inp.as_ptr(), k.as_ptr(), cp) };
            unsafe { r(or.as_mut_ptr(), inp.as_ptr(), k.as_ptr(), cp) };
            assert_guard(&format!("{row} {name} random (C)"), &oc);
            assert_eq_bytes(
                &format!("{row} {name} random(k={}, in={})", hexs(&k), hexs(&inp)),
                &oc,
                &or,
            );
            iters += 1;
        }
    }
    iters
}

#[test]
fn r77_r79_core_salsa20_2012_208() {
    init_both();
    for (row, name, seed) in [
        ("row77", "crypto_core_salsa20", SEED ^ 0x77),
        ("row78", "crypto_core_salsa2012", SEED ^ 0x78),
        ("row79", "crypto_core_salsa208", SEED ^ 0x79),
    ] {
        for (sfx, v) in [("outputbytes", 64usize), ("inputbytes", 16), ("keybytes", 32), ("constbytes", 16)] {
            assert_size(&format!("{name}_{sfx}"), v);
        }
        let n = core_row(row, name, 64, seed);
        assert!(n >= 64, "{row} {name} drove only {n} inputs");
        eprintln!("{row}: {n} {name} comparisons");
    }
}

// ===========================================================================
// CONFIGS row 80 — crypto_core_hsalsa20
// ===========================================================================

#[test]
fn r80_core_hsalsa20() {
    init_both();
    for (sfx, v) in [("outputbytes", 32usize), ("inputbytes", 16), ("keybytes", 32), ("constbytes", 16)] {
        assert_size(&format!("crypto_core_hsalsa20_{sfx}"), v);
    }
    let n = core_row("row80", "crypto_core_hsalsa20", 32, SEED ^ 0x80);
    assert!(n >= 64, "row 80 drove only {n} inputs");
    eprintln!("row 80: {n} crypto_core_hsalsa20 comparisons");
}

// ===========================================================================
// CONFIGS row 81 — crypto_core_hchacha20
// ===========================================================================

#[test]
fn r81_core_hchacha20() {
    init_both();
    for (sfx, v) in [("outputbytes", 32usize), ("inputbytes", 16), ("keybytes", 32), ("constbytes", 16)] {
        assert_size(&format!("crypto_core_hchacha20_{sfx}"), v);
    }
    let n = core_row("row81", "crypto_core_hchacha20", 32, SEED ^ 0x81);
    assert!(n >= 64, "row 81 drove only {n} inputs");
    eprintln!("row 81: {n} crypto_core_hchacha20 comparisons");
}

// ===========================================================================
// CONFIGS row 82 — crypto_core_keccak1600_init / _xor_bytes / _extract_bytes /
//                  _permute_24 / _permute_12
// ===========================================================================

/// The Keccak-f[1600] state is 200 bytes; the public struct is opaque[224] and
/// `_init` only zeroes the first 200 (the rest keeps the 0xAA pre-fill in BOTH
/// libraries — comparing the whole buffer proves that).
const KECCAK_STATE: usize = 200;

#[derive(Clone)]
enum KOp {
    Xor(Vec<u8>, usize),
    Ext(usize, usize),
    P24,
    P12,
}

fn describe_k(ops: &[KOp]) -> String {
    let mut s = String::from("init");
    for op in ops {
        s.push(',');
        match op {
            KOp::Xor(d, o) => s.push_str(&format!("xor(off={o},len={})", d.len())),
            KOp::Ext(o, l) => s.push_str(&format!("ext(off={o},len={l})")),
            KOp::P24 => s.push_str("p24"),
            KOp::P12 => s.push_str("p12"),
        }
    }
    s
}

unsafe fn kec_run(lib: &'static Library, ops: &[KOp]) -> Run {
    let init = sym::<KecVoid>(lib, "crypto_core_keccak1600_init");
    let xorb = sym::<KecXor>(lib, "crypto_core_keccak1600_xor_bytes");
    let extb = sym::<KecExtract>(lib, "crypto_core_keccak1600_extract_bytes");
    let p24 = sym::<KecVoid>(lib, "crypto_core_keccak1600_permute_24");
    let p12 = sym::<KecVoid>(lib, "crypto_core_keccak1600_permute_12");
    let mut st = new_state();
    let sp = st.0.as_mut_ptr();
    init(sp);
    let mut outs = Vec::new();
    for op in ops {
        match op {
            KOp::Xor(d, off) => xorb(sp, d.as_ptr(), *off, d.len()),
            KOp::Ext(off, len) => {
                let mut o = vec![0xAAu8; *len + GUARD];
                extb(sp, o.as_mut_ptr(), *off, *len);
                outs.push(o);
            }
            KOp::P24 => p24(sp),
            KOp::P12 => p12(sp),
        }
    }
    Run { rets: Vec::new(), outs, state: st.0.to_vec() }
}

fn kec_cmp(what: &str, ops: &[KOp]) -> Run {
    let l = libs();
    let a = unsafe { kec_run(&l.c, ops) };
    let b = unsafe { kec_run(&l.r, ops) };
    let tag = format!("{what} keccak1600 [{}]", describe_k(ops));
    assert_eq!(a.outs.len(), b.outs.len(), "{tag}: number of extractions differs");
    for i in 0..a.outs.len() {
        assert_guard(&format!("{tag} extract #{i} (C)"), &a.outs[i]);
        assert_eq_bytes(&format!("{tag} extract #{i}"), &a.outs[i], &b.outs[i]);
    }
    assert_eq_bytes(&format!("{tag} OPAQUE STATE"), &a.state, &b.state);
    a
}

#[test]
fn r82_core_keccak1600() {
    init_both();
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0x82);
    let mut iters = 0usize;

    assert_size("crypto_core_keccak1600_statebytes", 224);

    // (a) _init alone: the first 200 bytes become zero, the rest of the 224-byte
    //     struct keeps the 0xAA pre-fill. Verified byte-for-byte against C.
    let base = kec_cmp("row82-init", &[]);
    assert!(
        base.state[..KECCAK_STATE].iter().all(|&x| x == 0),
        "row82: C _init left non-zero bytes in the first {KECCAK_STATE}"
    );
    assert!(
        base.state[KECCAK_STATE..].iter().all(|&x| x == 0xAA),
        "row82: C _init wrote past the {KECCAK_STATE}-byte Keccak state"
    );
    iters += 1;

    // (b) exhaustive offset/length combos around the 8-byte fast-path boundary.
    for off in 0..=9usize {
        for len in [0usize, 1, 2, 3, 7, 8, 9, 15, 16, 17, 63, 64, 65, 135, 136, 167, 168] {
            if off + len > KECCAK_STATE {
                continue;
            }
            let data = rng.bytes(len);
            let ops = vec![
                KOp::Xor(data.clone(), off),
                KOp::Ext(off, len),
                KOp::Ext(0, KECCAK_STATE),
                KOp::P24,
                KOp::Ext(0, KECCAK_STATE),
                KOp::P12,
                KOp::Ext(0, KECCAK_STATE),
            ];
            kec_cmp("row82-offlen", &ops);
            iters += 1;
        }
    }

    // (c) XOR semantics: xor_bytes twice with the same data must cancel out.
    for _ in 0..16 {
        let off = rng.below(64);
        let len = rng.below(KECCAK_STATE - off + 1);
        let data = rng.bytes(len);
        let twice = vec![KOp::Xor(data.clone(), off), KOp::Xor(data.clone(), off)];
        let r2 = kec_cmp("row82-xor-cancel", &twice);
        assert_eq_bytes(
            "row82: xor_bytes applied twice did not cancel (C)",
            &base.state,
            &r2.state,
        );
        iters += 1;
    }

    // (d) whole-state fill at every alignment, then both permutations, comparing
    //     the extracted state after each step.
    for pat in [0x00u8, 0xff] {
        for off in 0..=8usize {
            let len = KECCAK_STATE - off;
            let ops = vec![
                KOp::Xor(vec![pat; len], off),
                KOp::Ext(0, KECCAK_STATE),
                KOp::P24,
                KOp::Ext(0, KECCAK_STATE),
                KOp::P24,
                KOp::Ext(0, KECCAK_STATE),
                KOp::P12,
                KOp::Ext(0, KECCAK_STATE),
            ];
            kec_cmp("row82-fill", &ops);
            iters += 1;
        }
    }

    // (e) long random op sequences mixing all five entry points.
    for _ in 0..64 {
        let nops = 3 + rng.below(10);
        let mut ops = Vec::new();
        for _ in 0..nops {
            match rng.below(4) {
                0 => {
                    let off = rng.below(KECCAK_STATE);
                    let len = rng.below(KECCAK_STATE - off + 1);
                    ops.push(KOp::Xor(rng.bytes(len), off));
                }
                1 => {
                    let off = rng.below(KECCAK_STATE);
                    let len = rng.below(KECCAK_STATE - off + 1);
                    ops.push(KOp::Ext(off, len));
                }
                2 => ops.push(KOp::P24),
                _ => ops.push(KOp::P12),
            }
        }
        ops.push(KOp::Ext(0, KECCAK_STATE));
        kec_cmp("row82-random", &ops);
        iters += 1;
    }

    // (f) permute_24 must NOT equal permute_12 (round-count regression guard),
    //     checked inside each library.
    for _ in 0..8 {
        let data = rng.bytes(KECCAK_STATE);
        let a24 = unsafe { kec_run(&l.c, &[KOp::Xor(data.clone(), 0), KOp::P24, KOp::Ext(0, KECCAK_STATE)]) };
        let a12 = unsafe { kec_run(&l.c, &[KOp::Xor(data.clone(), 0), KOp::P12, KOp::Ext(0, KECCAK_STATE)]) };
        let b24 = unsafe { kec_run(&l.r, &[KOp::Xor(data.clone(), 0), KOp::P24, KOp::Ext(0, KECCAK_STATE)]) };
        let b12 = unsafe { kec_run(&l.r, &[KOp::Xor(data.clone(), 0), KOp::P12, KOp::Ext(0, KECCAK_STATE)]) };
        assert_ne!(a24.outs[0], a12.outs[0], "row82 C: permute_24 == permute_12");
        assert_ne!(b24.outs[0], b12.outs[0], "row82 rust: permute_24 == permute_12");
        assert_eq_bytes("row82 permute_24", &a24.outs[0], &b24.outs[0]);
        assert_eq_bytes("row82 permute_12", &a12.outs[0], &b12.outs[0]);
        // 12 rounds of permute_12 are the LAST 12 rounds of permute_24, so
        // permute_12 twice must not accidentally equal permute_24.
        iters += 1;
    }

    // (g) extract_bytes must not modify the state.
    for _ in 0..8 {
        let data = rng.bytes(KECCAK_STATE);
        let no_ext = vec![KOp::Xor(data.clone(), 0), KOp::P24];
        let with_ext = vec![
            KOp::Xor(data.clone(), 0),
            KOp::P24,
            KOp::Ext(0, KECCAK_STATE),
            KOp::Ext(3, 7),
            KOp::Ext(199, 1),
        ];
        let a = kec_cmp("row82-ext-const", &no_ext);
        let b = kec_cmp("row82-ext-const", &with_ext);
        assert_eq_bytes("row82: extract_bytes mutated the state (C)", &a.state, &b.state);
        iters += 1;
    }

    assert!(iters >= 64, "row 82 drove only {iters} inputs");
    eprintln!("row 82: {iters} keccak1600 scripted comparisons");
}
