//! t05 — differential verification of the MAC surface:
//! poly1305 / `crypto_onetimeauth`, HMAC-SHA2 / `crypto_auth`, and
//! BLAKE2b / `crypto_generichash`.
//!
//! Covers CONFIGS.md rows **98–121** and ERRORS.md rows **154–161** and
//! **222–236**. Every call goes through `dlsym` on BOTH the C `libsodium.so`
//! and the Rust `liblibsodium.so`; the Rust crate is never linked or called
//! directly, so the `#[no_mangle]` export wrappers are under test too.
//!
//! Row → test-function map
//! ```text
//! CONFIGS 98      -> r98_poly1305_oneshot
//! CONFIGS 99      -> r99_poly1305_leftover_axis
//! CONFIGS 100     -> r100_poly1305_update_chunkings
//! CONFIGS 101     -> r101_poly1305_final_leftover
//! CONFIGS 102     -> r102_onetimeauth_dispatch  (+ pick_best_implementation)
//! CONFIGS 103     -> r103_hmacsha256_init_keylen_axis
//! CONFIGS 104     -> r104_hmacsha256_streaming_vs_oneshot
//! CONFIGS 105     -> r105_hmacsha512_init_keylen_axis
//! CONFIGS 106     -> r106_hmacsha512_streaming_vs_oneshot
//! CONFIGS 107     -> r107_hmacsha512256_streaming_and_truncation
//! CONFIGS 108     -> r108_crypto_auth_dispatch
//! CONFIGS 109     -> r109_e157_e160_all_verify_byte_positions
//! CONFIGS 110     -> r110_r121_keygen_all      [RNG, mutex-guarded]
//! CONFIGS 111     -> r111_blake2b_oneshot_unkeyed
//! CONFIGS 112     -> r112_blake2b_oneshot_keyed
//! CONFIGS 113     -> r113_e230_null_key_positive_keylen
//! CONFIGS 114     -> r114_blake2b_salt_personal_oneshot
//! CONFIGS 115     -> r115_blake2b_streaming_unkeyed
//! CONFIGS 116     -> r116_blake2b_streaming_keyed
//! CONFIGS 117     -> r117_blake2b_update_multichunk
//! CONFIGS 118     -> r118_blake2b_init_salt_personal_streaming
//! CONFIGS 119     -> r119_e234_blake2b_final_outlen_mismatch
//! CONFIGS 120     -> r120_e235_e236_generichash_dispatch
//! CONFIGS 121     -> r110_r121_keygen_all      [RNG, mutex-guarded]
//! ERRORS  154-156 -> e154_e156_hmac_init_null_key_misuse         [forked]
//! ERRORS  157-160 -> r109_e157_e160_all_verify_byte_positions
//! ERRORS  161     -> e161_poly1305_verify_mismatch
//! ERRORS  222-224 -> e222_e224_e225_generichash_oneshot_bounds
//! ERRORS  225     -> e222_e224_e225_generichash_oneshot_bounds
//! ERRORS  226     -> e226_salt_personal_bounds
//! ERRORS  227-229 -> e227_e229_generichash_init_bounds
//! ERRORS  230     -> r113_e230_null_key_positive_keylen
//! ERRORS  231     -> e231_init_salt_personal_bounds
//! ERRORS  232     -> e232_generichash_final_bad_outlen_misuse    [forked]
//! ERRORS  233     -> e233_generichash_final_twice
//! ERRORS  234     -> r119_e234_blake2b_final_outlen_mismatch
//! ERRORS  235     -> r120_e235_e236_generichash_dispatch,
//!                    e222_e224_e225_generichash_oneshot_bounds,
//!                    e227_e229_generichash_init_bounds,
//!                    e232_generichash_final_bad_outlen_misuse,
//!                    e233_generichash_final_twice
//! ERRORS  236     -> r120_e235_e236_generichash_dispatch
//! ```

mod common;
use common::*;
use libc::c_int;
use libloading::Library;
use std::ffi::{c_char, CStr};
use std::ptr;
use std::sync::Mutex;

// ---------------------------------------------------------------- FFI aliases

type SizeFn = unsafe extern "C" fn() -> usize;
type StrFn = unsafe extern "C" fn() -> *const c_char;
type IntFn = unsafe extern "C" fn() -> c_int;
type KeygenFn = unsafe extern "C" fn(*mut u8);

/// `crypto_onetimeauth`, `crypto_auth*` one-shot: (out, in, inlen, k)
type MacFn = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> c_int;
/// `*_verify`: (h, in, inlen, k)
type MacVerifyFn = unsafe extern "C" fn(*const u8, *const u8, u64, *const u8) -> c_int;

/// `crypto_onetimeauth*_init`: (state, key)
type OtaInit = unsafe extern "C" fn(*mut u8, *const u8) -> c_int;
/// `*_update`: (state, in, inlen) — shared by poly1305 / hmac / generichash
type StUpdate = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
/// `crypto_onetimeauth*_final` / `crypto_auth*_final`: (state, out)
type StFinal = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;
/// `crypto_auth_hmacsha*_init`: (state, key, keylen)
type HmacInit = unsafe extern "C" fn(*mut u8, *const u8, usize) -> c_int;

/// `crypto_generichash*_init`: (state, key, keylen, outlen)
type GhInit = unsafe extern "C" fn(*mut u8, *const u8, usize, usize) -> c_int;
/// `crypto_generichash_blake2b_init_salt_personal`
type GhInitSp =
    unsafe extern "C" fn(*mut u8, *const u8, usize, usize, *const u8, *const u8) -> c_int;
/// `crypto_generichash*_final`: (state, out, outlen)
type GhFinal = unsafe extern "C" fn(*mut u8, *mut u8, usize) -> c_int;
/// `crypto_generichash*`: (out, outlen, in, inlen, key, keylen)
type GhOne = unsafe extern "C" fn(*mut u8, usize, *const u8, u64, *const u8, usize) -> c_int;
/// `crypto_generichash_blake2b_salt_personal`
type GhOneSp = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const u8,
    u64,
    *const u8,
    usize,
    *const u8,
    *const u8,
) -> c_int;

type SetImplFn = unsafe extern "C" fn(*const RandombytesImpl) -> c_int;

// ------------------------------------------------------------ opaque state buf
//
// Every opaque state is allocated as an over-sized, 64-byte-aligned buffer
// pre-filled with 0xAA. Both libraries therefore see an IDENTICAL starting
// layout; comparing the WHOLE buffer after `_init` and after every `_update`
// catches state-layout divergence (and overruns) that a final-digest-only
// comparison would miss.
//
// Largest state here: crypto_auth_hmacsha512_state == 416; blake2b == 384
// (needs 64-byte alignment); poly1305 == 256.

const SB: usize = 512;
/// Guard bytes appended to every output buffer; must survive untouched.
const GUARD: usize = 32;
const FILL: u8 = 0xAA;

#[repr(C, align(64))]
struct StBuf([u8; SB]);

fn new_state() -> Box<StBuf> {
    Box::new(StBuf([FILL; SB]))
}

/// Neither library may write past the `n` bytes it was asked to produce.
fn assert_guard(what: &str, buf: &[u8], n: usize) {
    for (i, &b) in buf[n..].iter().enumerate() {
        assert_eq!(
            b, FILL,
            "{what}: byte {} past the requested {n}-byte output was overwritten \
             ({FILL:#04x} -> {b:#04x}); full buffer = {}",
            n + i,
            hexs(buf)
        );
    }
}

/// Result of running a scripted sequence of calls against one library.
struct Run {
    rets: Vec<c_int>,
    /// (requested output length, full buffer incl. guard)
    outs: Vec<(usize, Vec<u8>)>,
    /// Whole opaque-state snapshot after `_init` and after each subsequent op.
    states: Vec<Vec<u8>>,
}

impl Run {
    fn new() -> Self {
        Run { rets: Vec::new(), outs: Vec::new(), states: Vec::new() }
    }
    /// The last digest produced (without the guard region).
    fn digest(&self) -> Vec<u8> {
        let (n, ref b) = *self.outs.last().expect("no digest produced");
        b[..n].to_vec()
    }
}

fn cmp_runs(tag: &str, a: &Run, b: &Run) {
    assert_eq!(
        a.rets, b.rets,
        "{tag}: RETURN-CODE SEQUENCE MISMATCH\n  C   ={:?}\n  rust={:?}",
        a.rets, b.rets
    );
    assert_eq!(
        a.outs.len(),
        b.outs.len(),
        "{tag}: number of produced outputs differs (C={} rust={})",
        a.outs.len(),
        b.outs.len()
    );
    for i in 0..a.outs.len() {
        let (n, ref bc) = a.outs[i];
        let (n2, ref br) = b.outs[i];
        assert_eq!(n, n2, "{tag}: out#{i} requested length bookkeeping differs");
        assert_guard(&format!("{tag} out#{i} (C)"), bc, n);
        assert_guard(&format!("{tag} out#{i} (rust)"), br, n);
        assert_eq_bytes(&format!("{tag} out#{i}"), bc, br);
    }
    assert_eq!(
        a.states.len(),
        b.states.len(),
        "{tag}: number of state snapshots differs"
    );
    for i in 0..a.states.len() {
        assert_eq_bytes(
            &format!("{tag} OPAQUE STATE snapshot #{i} (0=after init)"),
            &a.states[i],
            &b.states[i],
        );
    }
}

// ------------------------------------------------------------------- utilities

fn assert_size(name: &str, expect: usize) -> usize {
    let (c, r) = unsafe { pair::<SizeFn>(name) };
    let (cv, rv) = unsafe { (c(), r()) };
    assert_eq!(cv, rv, "{name}(): C={cv} rust={rv}");
    assert_eq!(cv, expect, "{name}(): C returned {cv}, spec says {expect}");
    cv
}

/// `*_statebytes()` must agree across the two libraries, must match the value
/// derived from the header struct, AND must be comfortably smaller than the
/// fixed `SB` scratch buffer so that the 0xAA tail is a real overrun guard.
fn assert_statebytes(name: &str, expect: usize) {
    let n = assert_size(name, expect);
    assert!(
        SB >= n + 64,
        "{name}() = {n}; the test scratch buffer SB={SB} is not generously oversized"
    );
}

fn assert_cstr(name: &str, expect: &str) {
    let (c, r) = unsafe { pair::<StrFn>(name) };
    let (cs, rs) = unsafe { (CStr::from_ptr(c()), CStr::from_ptr(r())) };
    assert_eq!(cs, rs, "{name}(): C={cs:?} rust={rs:?}");
    assert_eq!(cs.to_str().unwrap(), expect, "{name}(): C returned {cs:?}");
}

/// Both libraries must return the same value from a no-argument `int` function.
fn assert_int_fn(name: &str, expect: c_int) {
    let (c, r) = unsafe { pair::<IntFn>(name) };
    let (cv, rv) = unsafe { (c(), r()) };
    assert_eq!(cv, rv, "{name}(): C={cv} rust={rv}");
    assert_eq!(cv, expect, "{name}(): C returned {cv}, spec says {expect}");
}

/// Disable core dumps for this process *and* every child it forks: the
/// intentional `sodium_misuse()` aborts would otherwise each dump the whole
/// address space (≈100x slowdown).
fn no_core() {
    unsafe {
        let rl = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
        libc::setrlimit(libc::RLIMIT_CORE, &rl);
    }
}

const MISUSE: Outcome = Outcome::Signaled(SIGABRT);

// --------------------------------------------------------- scripted op scripts

/// Op script for the fixed-output-length families (poly1305, HMAC).
#[derive(Clone)]
enum MOp {
    Upd(Vec<u8>),
    Fin,
}

/// Op script for BLAKE2b (`_final` carries its own `outlen`).
#[derive(Clone)]
enum GOp {
    Upd(Vec<u8>),
    Fin(usize),
}

fn describe_m(ops: &[MOp]) -> String {
    let mut s = String::new();
    for op in ops {
        if !s.is_empty() {
            s.push(',');
        }
        match op {
            MOp::Upd(v) => s.push_str(&format!("upd({})", v.len())),
            MOp::Fin => s.push_str("fin"),
        }
    }
    if s.is_empty() {
        s.push_str("<init only>");
    }
    s
}

fn describe_g(ops: &[GOp]) -> String {
    let mut s = String::new();
    let mut run: Option<(usize, usize)> = None;
    let flush = |s: &mut String, r: Option<(usize, usize)>| {
        if let Some((n, c)) = r {
            if !s.is_empty() {
                s.push(',');
            }
            if c == 1 {
                s.push_str(&format!("upd({n})"));
            } else {
                s.push_str(&format!("upd({n})x{c}"));
            }
        }
    };
    for op in ops {
        match op {
            GOp::Upd(v) => {
                run = match run {
                    Some((m, c)) if m == v.len() => Some((m, c + 1)),
                    other => {
                        flush(&mut s, other);
                        Some((v.len(), 1))
                    }
                };
            }
            GOp::Fin(n) => {
                flush(&mut s, run.take());
                if !s.is_empty() {
                    s.push(',');
                }
                s.push_str(&format!("fin({n})"));
            }
        }
    }
    flush(&mut s, run.take());
    if s.is_empty() {
        s.push_str("<init only>");
    }
    s
}

/// Split `msg` into the exact chunk sizes given, then finalize.
fn mops(msg: &[u8], sizes: &[usize]) -> Vec<MOp> {
    let mut v = Vec::new();
    let mut i = 0usize;
    for &n in sizes {
        assert!(i + n <= msg.len(), "chunk script overruns the message");
        v.push(MOp::Upd(msg[i..i + n].to_vec()));
        i += n;
    }
    assert_eq!(i, msg.len(), "chunk script does not consume the whole message");
    v.push(MOp::Fin);
    v
}

fn gops(msg: &[u8], sizes: &[usize], outlen: usize) -> Vec<GOp> {
    let mut v = Vec::new();
    let mut i = 0usize;
    for &n in sizes {
        assert!(i + n <= msg.len(), "chunk script overruns the message");
        v.push(GOp::Upd(msg[i..i + n].to_vec()));
        i += n;
    }
    assert_eq!(i, msg.len(), "chunk script does not consume the whole message");
    v.push(GOp::Fin(outlen));
    v
}

// ============================================================================
// poly1305 / crypto_onetimeauth — CONFIGS 98–102, ERRORS 161
// ============================================================================

const POLY_BLOCK: usize = 16;

unsafe fn poly_run(lib: &'static Library, prefix: &str, key: &[u8], ops: &[MOp]) -> Run {
    let init = sym::<OtaInit>(lib, &format!("{prefix}_init"));
    let upd = sym::<StUpdate>(lib, &format!("{prefix}_update"));
    let fin = sym::<StFinal>(lib, &format!("{prefix}_final"));
    let mut st = new_state();
    let sp = st.0.as_mut_ptr();
    let mut run = Run::new();
    run.rets.push(init(sp, key.as_ptr()));
    run.states.push(st.0.to_vec());
    for op in ops {
        match op {
            MOp::Upd(v) => {
                run.rets.push(upd(sp, v.as_ptr(), v.len() as u64));
            }
            MOp::Fin => {
                let mut o = vec![FILL; 16 + GUARD];
                run.rets.push(fin(sp, o.as_mut_ptr()));
                run.outs.push((16, o));
            }
        }
        run.states.push(st.0.to_vec());
    }
    run
}

/// Drive one scripted poly1305 streaming sequence through both libraries and
/// require full agreement on return codes, digests, guard regions and every
/// intermediate opaque-state snapshot.
fn poly_cmp(what: &str, prefix: &str, key: &[u8], ops: &[MOp]) -> Run {
    let l = libs();
    let a = unsafe { poly_run(&l.c, prefix, key, ops) };
    let b = unsafe { poly_run(&l.r, prefix, key, ops) };
    let tag = format!(
        "{what} {prefix} key={} [{}]",
        hexs(key),
        describe_m(ops)
    );
    cmp_runs(&tag, &a, &b);
    for (i, x) in a.rets.iter().enumerate() {
        assert_eq!(*x, 0, "{tag}: call #{i} returned {x}, poly1305 always returns 0");
    }
    a
}

/// One-shot through both libraries; returns the C digest.
fn mac_oneshot(name: &str, taglen: usize, msg: &[u8], key: &[u8]) -> Vec<u8> {
    let (c, r) = unsafe { pair::<MacFn>(name) };
    let mut oc = vec![FILL; taglen + GUARD];
    let mut or = vec![FILL; taglen + GUARD];
    let (rc, rr) = unsafe {
        (
            c(oc.as_mut_ptr(), msg.as_ptr(), msg.len() as u64, key.as_ptr()),
            r(or.as_mut_ptr(), msg.as_ptr(), msg.len() as u64, key.as_ptr()),
        )
    };
    let tag = format!("{name}(mlen={} key={})", msg.len(), hexs(key));
    assert_eq!(rc, rr, "{tag}: return C={rc} rust={rr}");
    assert_eq!(rc, 0, "{tag}: C returned {rc}, spec says 0");
    assert_guard(&format!("{tag} (C)"), &oc, taglen);
    assert_guard(&format!("{tag} (rust)"), &or, taglen);
    assert_eq_bytes(&tag, &oc, &or);
    oc[..taglen].to_vec()
}

/// CONFIGS 98 — `crypto_onetimeauth_poly1305` one-shot over the documented
/// `inlen` set × several key patterns. Also pins the dispatch wrapper
/// (`crypto_onetimeauth`) to the same bytes.
#[test]
fn r98_poly1305_oneshot() {
    init_both();
    let mut rng = Rng::new(SEED ^ 98);
    let lens = [0usize, 1, 15, 16, 17, 31, 32, 33, 63, 64, 1000];
    let mut keys = patterns(32, &mut rng);
    keys.push(rng.bytes(32));
    keys.push({
        let mut k = vec![0u8; 32];
        k[0] = 0xff;
        k
    });
    keys.push({
        // r == 0 after clamping: exercises the degenerate multiplier
        let mut k = rng.bytes(32);
        for b in k[..16].iter_mut() {
            *b = 0;
        }
        k
    });
    let mut iters = 0usize;
    for &len in &lens {
        for k in &keys {
            let msg = rng.bytes(len);
            let d = mac_oneshot("crypto_onetimeauth_poly1305", 16, &msg, k);
            let d2 = mac_oneshot("crypto_onetimeauth", 16, &msg, k);
            assert_eq_bytes(
                "row98: crypto_onetimeauth != crypto_onetimeauth_poly1305",
                &d,
                &d2,
            );
            // and the same bytes must come out of the streaming API
            let s = poly_cmp(
                "row98",
                "crypto_onetimeauth_poly1305",
                k,
                &mops(&msg, &[len]),
            );
            assert_eq_bytes("row98: streaming != one-shot (C)", &d, &s.digest());
            iters += 1;
        }
    }
    assert!(iters >= 64, "row98 only ran {iters} inputs");
}

/// CONFIGS 99 — the leftover-buffer axis of `poly1305_update`. Every one of the
/// five branches is reached by an explicitly-named chunk script:
///   (a) `leftover == 0 && bytes < 16`      — buffered, no compression
///   (b) `leftover != 0 && want > bytes`    — top-up only, early return
///   (c) top-up exactly fills the buffer    — one block compressed, leftover=0
///   (d) `bytes >= 16`                      — bulk path
///   (e) nonzero remainder stored back      — leftover != 0 after the bulk path
#[test]
fn r99_poly1305_leftover_axis() {
    init_both();
    let mut rng = Rng::new(SEED ^ 99);
    let keys = patterns(32, &mut rng);
    // (name, chunk sizes)
    let scripts: &[(&str, &[usize])] = &[
        ("a: leftover==0, bytes<16", &[5]),
        ("a: leftover==0, bytes==15", &[15]),
        ("b: top-up only, early return", &[5, 3]),
        ("b: top-up only, 1+1+1", &[1, 1, 1]),
        ("b: top-up want>bytes at 15", &[14, 1]),
        ("c: top-up exactly fills", &[5, 11]),
        ("c: top-up fills, 8+8", &[8, 8]),
        ("d: bulk only", &[48]),
        ("d: bulk, 16 exactly", &[16]),
        ("e: bulk + remainder stored back", &[20]),
        ("e: bulk + remainder, 47", &[47]),
        ("b+c+d+e: 5 then 30", &[5, 30]),
        ("c+d+e: 3 then 45", &[3, 45]),
        ("d+e then top-up then bulk", &[20, 5, 40]),
        ("leftover 15 then bulk 33", &[15, 33]),
    ];
    let mut iters = 0usize;
    for (name, sizes) in scripts {
        let total: usize = sizes.iter().sum();
        for k in &keys {
            let msg = rng.bytes(total);
            let s = poly_cmp(
                &format!("row99 {name}"),
                "crypto_onetimeauth_poly1305",
                k,
                &mops(&msg, sizes),
            );
            let one = mac_oneshot("crypto_onetimeauth_poly1305", 16, &msg, k);
            assert_eq_bytes(
                &format!("row99 {name}: streaming != one-shot"),
                &one,
                &s.digest(),
            );
            iters += 1;
        }
    }
    assert!(iters >= 64, "row99 only ran {iters} inputs");
}

/// CONFIGS 100 — the exact `_update` chunkings demanded by the spec:
/// 1+15, 15+1, 16 in one call, 16 in two calls, 17, 31+1, 32, 33, plus a
/// 0-length update in every position.
#[test]
fn r100_poly1305_update_chunkings() {
    init_both();
    let mut rng = Rng::new(SEED ^ 100);
    let keys = patterns(32, &mut rng);
    let scripts: &[&[usize]] = &[
        &[1, 15],
        &[15, 1],
        &[16],
        &[16, 0],
        &[0, 16],
        &[8, 8],
        &[1, 15, 0],
        &[17],
        &[0, 17],
        &[31, 1],
        &[32],
        &[33],
        &[0],
        &[0, 0, 0],
        &[0, 1, 0, 15, 0],
        &[16, 0, 16],
        &[33, 0, 33],
    ];
    let mut iters = 0usize;
    for sizes in scripts {
        let total: usize = sizes.iter().sum();
        for k in &keys {
            let msg = rng.bytes(total);
            let s = poly_cmp(
                "row100",
                "crypto_onetimeauth_poly1305",
                k,
                &mops(&msg, sizes),
            );
            let one = mac_oneshot("crypto_onetimeauth_poly1305", 16, &msg, k);
            assert_eq_bytes("row100: streaming != one-shot", &one, &s.digest());
            // the same script through the crypto_onetimeauth dispatch wrappers
            let s2 = poly_cmp("row100 dispatch", "crypto_onetimeauth", k, &mops(&msg, sizes));
            assert_eq_bytes(
                "row100: crypto_onetimeauth streaming != poly1305 streaming",
                &s.digest(),
                &s2.digest(),
            );
            iters += 1;
        }
    }
    assert!(iters >= 64, "row100 only ran {iters} inputs");
}

/// CONFIGS 101 — `_final` with `leftover == 0` versus `leftover ∈ 1..15`
/// (the `0x01` pad byte path). The message length mod 16 selects the branch.
#[test]
fn r101_poly1305_final_leftover() {
    init_both();
    let mut rng = Rng::new(SEED ^ 101);
    let keys = patterns(32, &mut rng);
    let mut iters = 0usize;
    for leftover in 0..POLY_BLOCK {
        // 2 full blocks + `leftover` bytes -> st->leftover == leftover at _final
        let total = 2 * POLY_BLOCK + leftover;
        for k in &keys {
            let msg = rng.bytes(total);
            // one call: bulk then remainder
            let s = poly_cmp(
                &format!("row101 leftover={leftover} single-update"),
                "crypto_onetimeauth_poly1305",
                k,
                &mops(&msg, &[total]),
            );
            // dribbled in one byte at a time: same leftover at _final
            let ones: Vec<usize> = vec![1; total];
            let s2 = poly_cmp(
                &format!("row101 leftover={leftover} 1-byte updates"),
                "crypto_onetimeauth_poly1305",
                k,
                &mops(&msg, &ones),
            );
            assert_eq_bytes(
                "row101: chunking changed the digest",
                &s.digest(),
                &s2.digest(),
            );
            let one = mac_oneshot("crypto_onetimeauth_poly1305", 16, &msg, k);
            assert_eq_bytes("row101: streaming != one-shot", &one, &s.digest());
            iters += 1;
        }
    }
    assert!(iters >= 64, "row101 only ran {iters} inputs");
}

/// CONFIGS 102 — the `crypto_onetimeauth` dispatch layer: constants, the
/// primitive name, `_verify`, and `_crypto_onetimeauth_poly1305_pick_best_implementation`
/// (which must keep selecting the donna implementation in this build).
#[test]
fn r102_onetimeauth_dispatch() {
    init_both();
    assert_statebytes("crypto_onetimeauth_statebytes", 256);
    assert_size("crypto_onetimeauth_bytes", 16);
    assert_size("crypto_onetimeauth_keybytes", 32);
    assert_statebytes("crypto_onetimeauth_poly1305_statebytes", 256);
    assert_size("crypto_onetimeauth_poly1305_bytes", 16);
    assert_size("crypto_onetimeauth_poly1305_keybytes", 32);
    assert_cstr("crypto_onetimeauth_primitive", "poly1305");

    // Re-picking the implementation must be a no-op in this build and must not
    // change any subsequent result.
    assert_int_fn("_crypto_onetimeauth_poly1305_pick_best_implementation", 0);

    let mut rng = Rng::new(SEED ^ 102);
    let keys = patterns(32, &mut rng);
    let (cv, rv) = unsafe { pair::<MacVerifyFn>("crypto_onetimeauth_verify") };
    let (cv2, rv2) = unsafe { pair::<MacVerifyFn>("crypto_onetimeauth_poly1305_verify") };
    let mut iters = 0usize;
    for &len in LENS.iter() {
        for k in &keys {
            let msg = rng.bytes(len);
            let d = mac_oneshot("crypto_onetimeauth", 16, &msg, k);
            let d2 = mac_oneshot("crypto_onetimeauth_poly1305", 16, &msg, k);
            assert_eq_bytes("row102: dispatch != poly1305", &d, &d2);
            for (nm, f, g) in [
                ("crypto_onetimeauth_verify", &cv, &rv),
                ("crypto_onetimeauth_poly1305_verify", &cv2, &rv2),
            ] {
                let (a, b) = unsafe {
                    (
                        f(d.as_ptr(), msg.as_ptr(), len as u64, k.as_ptr()),
                        g(d.as_ptr(), msg.as_ptr(), len as u64, k.as_ptr()),
                    )
                };
                assert_eq!(a, b, "row102 {nm}(correct tag): C={a} rust={b}");
                assert_eq!(a, 0, "row102 {nm}(correct tag): C returned {a}, want 0");
            }
            iters += 1;
        }
    }
    assert!(iters >= 64, "row102 only ran {iters} inputs");
    // repeat the pick after use: still stable and still donna
    assert_int_fn("_crypto_onetimeauth_poly1305_pick_best_implementation", 0);
    let msg = rng.bytes(77);
    let a = mac_oneshot("crypto_onetimeauth_poly1305", 16, &msg, &keys[3]);
    assert_int_fn("_crypto_onetimeauth_poly1305_pick_best_implementation", 0);
    let b = mac_oneshot("crypto_onetimeauth_poly1305", 16, &msg, &keys[3]);
    assert_eq_bytes("row102: pick_best_implementation changed the MAC", &a, &b);
}

/// ERRORS 161 — `crypto_onetimeauth_poly1305_verify` / `crypto_onetimeauth_verify`
/// on a `crypto_verify_16` mismatch: -1, at EVERY byte position of the tag,
/// for every bit within the byte that can be flipped cheaply.
#[test]
fn e161_poly1305_verify_mismatch() {
    init_both();
    let mut rng = Rng::new(SEED ^ 161);
    let keys = patterns(32, &mut rng);
    let names = ["crypto_onetimeauth_poly1305_verify", "crypto_onetimeauth_verify"];
    let mut iters = 0usize;
    for &len in &[0usize, 1, 15, 16, 17, 33, 64, 129] {
        for k in &keys {
            let msg = rng.bytes(len);
            let good = mac_oneshot("crypto_onetimeauth_poly1305", 16, &msg, k);
            for name in names {
                let (c, r) = unsafe { pair::<MacVerifyFn>(name) };
                // correct tag -> 0
                let (a, b) = unsafe {
                    (
                        c(good.as_ptr(), msg.as_ptr(), len as u64, k.as_ptr()),
                        r(good.as_ptr(), msg.as_ptr(), len as u64, k.as_ptr()),
                    )
                };
                assert_eq!(a, b, "e161 {name}(good): C={a} rust={b}");
                assert_eq!(a, 0, "e161 {name}(good): C returned {a}, want 0");
                // every byte position corrupted -> -1
                for pos in 0..16usize {
                    for bit in [0x01u8, 0x80, 0xff] {
                        let mut bad = good.clone();
                        bad[pos] ^= bit;
                        let (a, b) = unsafe {
                            (
                                c(bad.as_ptr(), msg.as_ptr(), len as u64, k.as_ptr()),
                                r(bad.as_ptr(), msg.as_ptr(), len as u64, k.as_ptr()),
                            )
                        };
                        assert_eq!(
                            a, b,
                            "e161 {name}(tag[{pos}] ^= {bit:#04x}, mlen={len}): C={a} rust={b}"
                        );
                        assert_eq!(
                            a, -1,
                            "e161 {name}(tag[{pos}] ^= {bit:#04x}): C returned {a}, want -1"
                        );
                    }
                }
                iters += 1;
            }
        }
    }
    assert!(iters >= 64, "e161 only ran {iters} inputs");
}

// ============================================================================
// HMAC-SHA2 / crypto_auth — CONFIGS 103–110, ERRORS 154–160
// ============================================================================

struct HmacFam {
    /// symbol prefix, e.g. `crypto_auth_hmacsha256`
    p: &'static str,
    /// SHA-2 block size used by `_init` for the ipad/opad and the pre-hash cutoff
    block: usize,
    /// tag length written by `_final`
    tag: usize,
    /// `*_statebytes()`
    sb: usize,
}

const HFAMS: &[HmacFam] = &[
    HmacFam { p: "crypto_auth_hmacsha256", block: 64, tag: 32, sb: 208 },
    HmacFam { p: "crypto_auth_hmacsha512", block: 128, tag: 64, sb: 416 },
    HmacFam { p: "crypto_auth_hmacsha512256", block: 128, tag: 32, sb: 416 },
];

unsafe fn hmac_run(
    lib: &'static Library,
    prefix: &str,
    key: *const u8,
    keylen: usize,
    tag: usize,
    ops: &[MOp],
) -> Run {
    let init = sym::<HmacInit>(lib, &format!("{prefix}_init"));
    let upd = sym::<StUpdate>(lib, &format!("{prefix}_update"));
    let fin = sym::<StFinal>(lib, &format!("{prefix}_final"));
    let mut st = new_state();
    let sp = st.0.as_mut_ptr();
    let mut run = Run::new();
    run.rets.push(init(sp, key, keylen));
    run.states.push(st.0.to_vec());
    for op in ops {
        match op {
            MOp::Upd(v) => {
                run.rets.push(upd(sp, v.as_ptr(), v.len() as u64));
            }
            MOp::Fin => {
                let mut o = vec![FILL; tag + GUARD];
                run.rets.push(fin(sp, o.as_mut_ptr()));
                run.outs.push((tag, o));
            }
        }
        run.states.push(st.0.to_vec());
    }
    run
}

fn hmac_cmp(what: &str, fam: &HmacFam, key: Option<&[u8]>, keylen: usize, ops: &[MOp]) -> Run {
    let l = libs();
    let kp = match key {
        Some(k) => {
            assert!(k.len() >= keylen);
            k.as_ptr()
        }
        None => ptr::null(),
    };
    let a = unsafe { hmac_run(&l.c, fam.p, kp, keylen, fam.tag, ops) };
    let b = unsafe { hmac_run(&l.r, fam.p, kp, keylen, fam.tag, ops) };
    let tag = format!(
        "{what} {} keylen={keylen}{} [{}]",
        fam.p,
        if key.is_none() { " key=NULL" } else { "" },
        describe_m(ops)
    );
    cmp_runs(&tag, &a, &b);
    for (i, x) in a.rets.iter().enumerate() {
        assert_eq!(*x, 0, "{tag}: call #{i} returned {x}, HMAC always returns 0");
    }
    a
}

/// Shared body for CONFIGS 103 / 105 and the 512256 delegation: the `keylen`
/// axis of `*_init`, including keylen == 0 with `key == NULL` (legal) and
/// keylen > block (the key gets PRE-HASHED using `state->ictx`).
fn hmac_init_keylen_axis(row: &str, fam: &HmacFam) -> usize {
    let b = fam.block;
    let keylens = [0usize, 1, 2, 32, b - 1, b, b + 1, b + 2, 2 * b, 2 * b + 7];
    let mut rng = Rng::new(SEED ^ (b as u64) ^ 0xA11C);
    let msgs: Vec<Vec<u8>> = [0usize, 1, 55, 64, 111, 128, 200, 1000]
        .iter()
        .map(|&n| rng.bytes(n))
        .collect();
    assert_statebytes(&format!("{}_statebytes", fam.p), fam.sb);
    assert_size(&format!("{}_bytes", fam.p), fam.tag);
    assert_size(&format!("{}_keybytes", fam.p), 32);

    let mut iters = 0usize;
    for &kl in &keylens {
        // several key byte patterns per length, including the all-zero and
        // all-0xff extremes (which make ipad/opad degenerate)
        let mut keys = patterns(kl.max(1), &mut rng);
        keys.push(rng.bytes(kl.max(1)));
        for k in &keys {
            for m in &msgs {
                let s = hmac_cmp(row, fam, Some(k), kl, &mops(m, &[m.len()]));
                // splitting the message must not change the tag
                let half = m.len() / 2;
                let s2 = hmac_cmp(
                    row,
                    fam,
                    Some(k),
                    kl,
                    &mops(m, &[0, half, 0, m.len() - half, 0]),
                );
                assert_eq_bytes(
                    &format!("{row}: {} chunking changed the tag", fam.p),
                    &s.digest(),
                    &s2.digest(),
                );
                iters += 1;
            }
        }
        // keylen == 0 is the ONLY legal `key == NULL` case (ERRORS 154-156)
        if kl == 0 {
            let s = hmac_cmp(row, fam, None, 0, &mops(&msgs[3], &[msgs[3].len()]));
            let s2 = hmac_cmp(row, fam, Some(&keys[0]), 0, &mops(&msgs[3], &[msgs[3].len()]));
            assert_eq_bytes(
                &format!("{row}: {} key=NULL keylen=0 != key!=NULL keylen=0", fam.p),
                &s.digest(),
                &s2.digest(),
            );
            iters += 1;
        }
    }
    iters
}

/// CONFIGS 103 — `crypto_auth_hmacsha256_init` keylen axis (block = 64).
#[test]
fn r103_hmacsha256_init_keylen_axis() {
    init_both();
    let n = hmac_init_keylen_axis("row103", &HFAMS[0]);
    assert!(n >= 64, "row103 only ran {n} inputs");
}

/// CONFIGS 104 — `crypto_auth_hmacsha256` streaming (incl. `inlen == 0`
/// updates) versus the one-shot, which always uses keylen == 32.
#[test]
fn r104_hmacsha256_streaming_vs_oneshot() {
    init_both();
    let fam = &HFAMS[0];
    let mut rng = Rng::new(SEED ^ 104);
    let keys = patterns(32, &mut rng);
    let mut iters = 0usize;
    for &len in LENS.iter() {
        for k in &keys {
            let msg = rng.bytes(len);
            let one = mac_oneshot(fam.p, fam.tag, &msg, k);
            let s = hmac_cmp("row104", fam, Some(k), 32, &mops(&msg, &[len]));
            assert_eq_bytes("row104: streaming != one-shot", &one, &s.digest());
            // random chunking with interleaved zero-length updates
            let mut sizes: Vec<usize> = vec![0];
            let mut i = 0usize;
            while i < len {
                let n = 1 + rng.below(len - i);
                sizes.push(n);
                i += n;
                if rng.next_u32() & 3 == 0 {
                    sizes.push(0);
                }
            }
            sizes.push(0);
            let s2 = hmac_cmp("row104 chunked", fam, Some(k), 32, &mops(&msg, &sizes));
            assert_eq_bytes("row104: chunked != one-shot", &one, &s2.digest());
            iters += 1;
        }
    }
    assert!(iters >= 64, "row104 only ran {iters} inputs");
}

/// CONFIGS 105 — `crypto_auth_hmacsha512_init` keylen axis (block = 128).
#[test]
fn r105_hmacsha512_init_keylen_axis() {
    init_both();
    let n = hmac_init_keylen_axis("row105", &HFAMS[1]);
    assert!(n >= 64, "row105 only ran {n} inputs");
}

/// CONFIGS 106 — `crypto_auth_hmacsha512` streaming versus one-shot, 64-byte tag.
#[test]
fn r106_hmacsha512_streaming_vs_oneshot() {
    init_both();
    let fam = &HFAMS[1];
    let mut rng = Rng::new(SEED ^ 106);
    let keys = patterns(32, &mut rng);
    let mut iters = 0usize;
    for &len in LENS.iter() {
        for k in &keys {
            let msg = rng.bytes(len);
            let one = mac_oneshot(fam.p, fam.tag, &msg, k);
            assert_eq!(one.len(), 64, "row106: hmacsha512 tag must be 64 bytes");
            let s = hmac_cmp("row106", fam, Some(k), 32, &mops(&msg, &[len]));
            assert_eq_bytes("row106: streaming != one-shot", &one, &s.digest());
            let mut sizes: Vec<usize> = vec![0];
            let mut i = 0usize;
            while i < len {
                let n = 1 + rng.below(len - i);
                sizes.push(n);
                i += n;
            }
            sizes.push(0);
            let s2 = hmac_cmp("row106 chunked", fam, Some(k), 32, &mops(&msg, &sizes));
            assert_eq_bytes("row106: chunked != one-shot", &one, &s2.digest());
            iters += 1;
        }
    }
    assert!(iters >= 64, "row106 only ran {iters} inputs");
}

/// CONFIGS 107 — `crypto_auth_hmacsha512256_*`: the state IS a
/// `crypto_auth_hmacsha512_state` (identical bytes after `_init`/`_update`) and
/// `_final` truncates the 64-byte SHA-512 HMAC to 32 bytes.
#[test]
fn r107_hmacsha512256_streaming_and_truncation() {
    init_both();
    let fam = &HFAMS[2];
    let f512 = &HFAMS[1];
    let n = hmac_init_keylen_axis("row107", fam);
    assert!(n >= 64, "row107 only ran {n} inputs");

    let mut rng = Rng::new(SEED ^ 107);
    let keys = patterns(32, &mut rng);
    let mut iters = 0usize;
    for &len in LENS.iter() {
        for k in &keys {
            let msg = rng.bytes(len);
            let one = mac_oneshot(fam.p, fam.tag, &msg, k);
            let s = hmac_cmp("row107", fam, Some(k), 32, &mops(&msg, &[len]));
            assert_eq_bytes("row107: streaming != one-shot", &one, &s.digest());
            // truncation property: 512256 == first 32 bytes of 512
            let full = hmac_cmp("row107 as-512", f512, Some(k), 32, &mops(&msg, &[len]));
            assert_eq_bytes(
                "row107: hmacsha512256 != hmacsha512[..32]",
                &full.digest()[..32],
                &one,
            );
            // and the opaque state must be byte-identical up to `_final`
            assert_eq_bytes(
                "row107: hmacsha512256 state != hmacsha512 state after init",
                &s.states[0],
                &full.states[0],
            );
            assert_eq_bytes(
                "row107: hmacsha512256 state != hmacsha512 state after update",
                &s.states[1],
                &full.states[1],
            );
            iters += 1;
        }
    }
    assert!(iters >= 64, "row107 only ran {iters} inputs");
}

/// CONFIGS 108 — the `crypto_auth` dispatch layer (delegates to hmacsha512256).
#[test]
fn r108_crypto_auth_dispatch() {
    init_both();
    assert_size("crypto_auth_bytes", 32);
    assert_size("crypto_auth_keybytes", 32);
    assert_cstr("crypto_auth_primitive", "hmacsha512256");

    let mut rng = Rng::new(SEED ^ 108);
    let keys = patterns(32, &mut rng);
    let (cv, rv) = unsafe { pair::<MacVerifyFn>("crypto_auth_verify") };
    let mut iters = 0usize;
    for &len in LENS.iter() {
        for k in &keys {
            let msg = rng.bytes(len);
            let d = mac_oneshot("crypto_auth", 32, &msg, k);
            let d2 = mac_oneshot("crypto_auth_hmacsha512256", 32, &msg, k);
            assert_eq_bytes("row108: crypto_auth != hmacsha512256", &d, &d2);
            let (a, b) = unsafe {
                (
                    cv(d.as_ptr(), msg.as_ptr(), len as u64, k.as_ptr()),
                    rv(d.as_ptr(), msg.as_ptr(), len as u64, k.as_ptr()),
                )
            };
            assert_eq!(a, b, "row108 crypto_auth_verify(good): C={a} rust={b}");
            assert_eq!(a, 0, "row108 crypto_auth_verify(good): C returned {a}");
            iters += 1;
        }
    }
    assert!(iters >= 64, "row108 only ran {iters} inputs");
}

/// CONFIGS 109 + ERRORS 157/158/159/160 — every `crypto_auth*_verify`: the
/// correct tag returns 0, and a tag differing at ANY byte position returns -1.
#[test]
fn r109_e157_e160_all_verify_byte_positions() {
    init_both();
    let mut rng = Rng::new(SEED ^ 109);
    let keys = patterns(32, &mut rng);
    // (verify symbol, one-shot symbol, taglen, ERRORS row)
    let cases: &[(&str, &str, usize, &str)] = &[
        ("crypto_auth_hmacsha256_verify", "crypto_auth_hmacsha256", 32, "e157"),
        ("crypto_auth_hmacsha512_verify", "crypto_auth_hmacsha512", 64, "e158"),
        (
            "crypto_auth_hmacsha512256_verify",
            "crypto_auth_hmacsha512256",
            32,
            "e159",
        ),
        ("crypto_auth_verify", "crypto_auth", 32, "e160"),
    ];
    let mut iters = 0usize;
    for (vname, oname, taglen, row) in cases {
        let (c, r) = unsafe { pair::<MacVerifyFn>(vname) };
        for &len in &[0usize, 1, 32, 63, 64, 65, 127, 128, 129, 1000] {
            for k in &keys {
                let msg = rng.bytes(len);
                let good = mac_oneshot(oname, *taglen, &msg, k);
                let (a, b) = unsafe {
                    (
                        c(good.as_ptr(), msg.as_ptr(), len as u64, k.as_ptr()),
                        r(good.as_ptr(), msg.as_ptr(), len as u64, k.as_ptr()),
                    )
                };
                assert_eq!(a, b, "row109 {vname}(good, mlen={len}): C={a} rust={b}");
                assert_eq!(a, 0, "row109 {vname}(good): C returned {a}, want 0");
                for pos in 0..*taglen {
                    for bit in [0x01u8, 0x80] {
                        let mut bad = good.clone();
                        bad[pos] ^= bit;
                        let (a, b) = unsafe {
                            (
                                c(bad.as_ptr(), msg.as_ptr(), len as u64, k.as_ptr()),
                                r(bad.as_ptr(), msg.as_ptr(), len as u64, k.as_ptr()),
                            )
                        };
                        assert_eq!(
                            a, b,
                            "{row} {vname}(tag[{pos}] ^= {bit:#04x}, mlen={len}): C={a} rust={b}"
                        );
                        assert_eq!(
                            a, -1,
                            "{row} {vname}(tag[{pos}] ^= {bit:#04x}): C returned {a}, want -1"
                        );
                    }
                }
                iters += 1;
            }
        }
    }
    assert!(iters >= 64, "row109 only ran {iters} inputs");
}

/// ERRORS 154/155/156 — `crypto_auth_hmacsha{256,512,512256}_init` with
/// `key == NULL && 0 < keylen <= block` calls `sodium_misuse()`, i.e. aborts.
/// Both libraries must die from SIGABRT. Symbols are resolved and the state
/// buffer allocated in the PARENT; the child only calls a function pointer.
#[test]
fn e154_e156_hmac_init_null_key_misuse() {
    init_both();
    no_core();
    let l = libs();
    let cases: &[(&str, usize, &str)] = &[
        ("crypto_auth_hmacsha256_init", 64, "e154"),
        ("crypto_auth_hmacsha512_init", 128, "e155"),
        ("crypto_auth_hmacsha512256_init", 128, "e156"),
    ];
    for (name, block, row) in cases {
        let cf: HmacInit = *unsafe { sym::<HmacInit>(&l.c, name) };
        let rf: HmacInit = *unsafe { sym::<HmacInit>(&l.r, name) };
        let mut sc = new_state();
        let mut sr = new_state();
        let pc = sc.0.as_mut_ptr();
        let pr = sr.0.as_mut_ptr();
        for &kl in &[1usize, 2, 31, 32, block - 1, *block] {
            let oc = forked(move || unsafe { cf(pc, ptr::null(), kl) as i64 });
            let or = forked(move || unsafe { rf(pr, ptr::null(), kl) as i64 });
            assert_same_fatal(&format!("{row} {name}(NULL, {kl})"), oc, or);
            assert_eq!(
                oc, MISUSE,
                "{row} {name}(key=NULL, keylen={kl}): C outcome was {oc:?}, \
                 expected sodium_misuse() -> {MISUSE:?}"
            );
        }
        // keylen == 0 with key == NULL is explicitly NOT a misuse
        let oc = forked(move || unsafe { cf(pc, ptr::null(), 0) as i64 });
        let or = forked(move || unsafe { rf(pr, ptr::null(), 0) as i64 });
        assert_same_fatal(&format!("{row} {name}(NULL, 0)"), oc, or);
        assert_eq!(
            oc,
            Outcome::Returned(0),
            "{row} {name}(key=NULL, keylen=0) must return 0, got {oc:?}"
        );
    }
}

// ============================================================================
// BLAKE2b / crypto_generichash — CONFIGS 111–121, ERRORS 222–236
// ============================================================================

const BLAKE2B_BLOCK: usize = 128;
/// `blake2b_state.buf` is 2 * BLAKE2B_BLOCKBYTES.
const BLAKE2B_BUF: usize = 256;

/// The two API layers with an identical branch structure (ERRORS 235).
const GH_LAYERS: &[&str] = &["crypto_generichash_blake2b", "crypto_generichash"];

/// One-shot `crypto_generichash*` through both libraries. Returns
/// `(return code, C digest bytes)`.
fn gh_one(
    name: &str,
    outlen: usize,
    msg: &[u8],
    key: Option<&[u8]>,
    keylen: usize,
) -> (c_int, Vec<u8>) {
    let (c, r) = unsafe { pair::<GhOne>(name) };
    // Always allocate room for a full 64-byte digest + guard even when outlen
    // is rejected, so an errant write is visible.
    let cap = outlen.max(64);
    let mut oc = vec![FILL; cap + GUARD];
    let mut or = vec![FILL; cap + GUARD];
    let kp = key.map_or(ptr::null(), |k| k.as_ptr());
    let (rc, rr) = unsafe {
        (
            c(oc.as_mut_ptr(), outlen, msg.as_ptr(), msg.len() as u64, kp, keylen),
            r(or.as_mut_ptr(), outlen, msg.as_ptr(), msg.len() as u64, kp, keylen),
        )
    };
    let tag = format!(
        "{name}(outlen={outlen}, mlen={}, keylen={keylen}{})",
        msg.len(),
        if key.is_none() { ", key=NULL" } else { "" }
    );
    assert_eq!(rc, rr, "{tag}: return C={rc} rust={rr}");
    let written = if rc == 0 { outlen.min(cap) } else { 0 };
    assert_guard(&format!("{tag} (C)"), &oc, written);
    assert_guard(&format!("{tag} (rust)"), &or, written);
    assert_eq_bytes(&tag, &oc, &or);
    (rc, oc[..written].to_vec())
}

/// `crypto_generichash_blake2b_salt_personal` through both libraries.
#[allow(clippy::too_many_arguments)]
fn gh_one_sp(
    outlen: usize,
    msg: &[u8],
    key: Option<&[u8]>,
    keylen: usize,
    salt: Option<&[u8]>,
    personal: Option<&[u8]>,
) -> (c_int, Vec<u8>) {
    let name = "crypto_generichash_blake2b_salt_personal";
    let (c, r) = unsafe { pair::<GhOneSp>(name) };
    let cap = outlen.max(64);
    let mut oc = vec![FILL; cap + GUARD];
    let mut or = vec![FILL; cap + GUARD];
    let kp = key.map_or(ptr::null(), |k| k.as_ptr());
    let sp = salt.map_or(ptr::null(), |k| k.as_ptr());
    let pp = personal.map_or(ptr::null(), |k| k.as_ptr());
    let (rc, rr) = unsafe {
        (
            c(oc.as_mut_ptr(), outlen, msg.as_ptr(), msg.len() as u64, kp, keylen, sp, pp),
            r(or.as_mut_ptr(), outlen, msg.as_ptr(), msg.len() as u64, kp, keylen, sp, pp),
        )
    };
    let tag = format!(
        "{name}(outlen={outlen}, mlen={}, keylen={keylen}, salt={}, personal={})",
        msg.len(),
        salt.map_or("NULL".into(), hexs),
        personal.map_or("NULL".into(), hexs)
    );
    assert_eq!(rc, rr, "{tag}: return C={rc} rust={rr}");
    let written = if rc == 0 { outlen.min(cap) } else { 0 };
    assert_guard(&format!("{tag} (C)"), &oc, written);
    assert_guard(&format!("{tag} (rust)"), &or, written);
    assert_eq_bytes(&tag, &oc, &or);
    (rc, oc[..written].to_vec())
}

/// How to initialize a BLAKE2b streaming state.
#[derive(Clone, Copy)]
enum GhInitKind<'a> {
    /// `*_init(state, key, keylen, outlen)`
    Plain { key: Option<&'a [u8]>, keylen: usize, outlen: usize },
    /// `*_init_salt_personal(state, key, keylen, outlen, salt, personal)`
    SaltPersonal {
        key: Option<&'a [u8]>,
        keylen: usize,
        outlen: usize,
        salt: Option<&'a [u8]>,
        personal: Option<&'a [u8]>,
    },
}

impl GhInitKind<'_> {
    fn describe(&self) -> String {
        match self {
            GhInitKind::Plain { key, keylen, outlen } => format!(
                "init(keylen={keylen}{}, outlen={outlen})",
                if key.is_none() { ", key=NULL" } else { "" }
            ),
            GhInitKind::SaltPersonal { key, keylen, outlen, salt, personal } => format!(
                "init_sp(keylen={keylen}{}, outlen={outlen}, salt={}, personal={})",
                if key.is_none() { ", key=NULL" } else { "" },
                salt.map_or("NULL".into(), hexs),
                personal.map_or("NULL".into(), hexs)
            ),
        }
    }
}

unsafe fn gh_run(lib: &'static Library, prefix: &str, kind: GhInitKind, ops: &[GOp]) -> Run {
    let upd = sym::<StUpdate>(lib, &format!("{prefix}_update"));
    let fin = sym::<GhFinal>(lib, &format!("{prefix}_final"));
    let mut st = new_state();
    let sp = st.0.as_mut_ptr();
    let mut run = Run::new();
    match kind {
        GhInitKind::Plain { key, keylen, outlen } => {
            let init = sym::<GhInit>(lib, &format!("{prefix}_init"));
            let kp = key.map_or(ptr::null(), |k| k.as_ptr());
            run.rets.push(init(sp, kp, keylen, outlen));
        }
        GhInitKind::SaltPersonal { key, keylen, outlen, salt, personal } => {
            let init = sym::<GhInitSp>(lib, &format!("{prefix}_init_salt_personal"));
            let kp = key.map_or(ptr::null(), |k| k.as_ptr());
            let s = salt.map_or(ptr::null(), |k| k.as_ptr());
            let p = personal.map_or(ptr::null(), |k| k.as_ptr());
            run.rets.push(init(sp, kp, keylen, outlen, s, p));
        }
    }
    run.states.push(st.0.to_vec());
    for op in ops {
        match op {
            GOp::Upd(v) => {
                run.rets.push(upd(sp, v.as_ptr(), v.len() as u64));
            }
            GOp::Fin(n) => {
                let mut o = vec![FILL; n.max(&64) + GUARD];
                let rc = fin(sp, o.as_mut_ptr(), *n);
                run.rets.push(rc);
                run.outs.push((if rc == 0 { *n } else { 0 }, o));
            }
        }
        run.states.push(st.0.to_vec());
    }
    run
}

fn gh_cmp(what: &str, prefix: &str, kind: GhInitKind, ops: &[GOp]) -> Run {
    let l = libs();
    let a = unsafe { gh_run(&l.c, prefix, kind, ops) };
    let b = unsafe { gh_run(&l.r, prefix, kind, ops) };
    let tag = format!("{what} {prefix} {} [{}]", kind.describe(), describe_g(ops));
    cmp_runs(&tag, &a, &b);
    a
}

/// Convenience: expect every call in the script to succeed.
fn gh_cmp_ok(what: &str, prefix: &str, kind: GhInitKind, ops: &[GOp]) -> Run {
    let a = gh_cmp(what, prefix, kind, ops);
    for (i, x) in a.rets.iter().enumerate() {
        assert_eq!(
            *x, 0,
            "{what} {prefix} {} [{}]: call #{i} returned {x}, expected 0",
            kind.describe(),
            describe_g(ops)
        );
    }
    a
}

/// CONFIGS 111 — `crypto_generichash_blake2b` one-shot, unkeyed, `outlen`
/// ∈ {1,15,16,32,63,64} (1 and 15 are BELOW `BYTES_MIN` and still accepted).
#[test]
fn r111_blake2b_oneshot_unkeyed() {
    init_both();
    let mut rng = Rng::new(SEED ^ 111);
    let outlens = [1usize, 2, 15, 16, 31, 32, 63, 64];
    let mut iters = 0usize;
    for &ol in &outlens {
        for &len in &[0usize, 1, 15, 63, 64, 127, 128, 129, 255, 256, 257, 383, 384, 1000] {
            let msg = rng.bytes(len);
            let (rc, d) = gh_one("crypto_generichash_blake2b", ol, &msg, None, 0);
            assert_eq!(rc, 0, "row111: outlen={ol} rejected (C returned {rc})");
            assert_eq!(d.len(), ol);
            // key=Some but keylen=0 must take the same unkeyed path
            let k = rng.bytes(32);
            let (rc2, d2) = gh_one("crypto_generichash_blake2b", ol, &msg, Some(&k), 0);
            assert_eq!(rc2, 0);
            assert_eq_bytes("row111: keylen=0 with non-NULL key differs", &d, &d2);
            // dispatch layer must produce the same bytes
            let (rc3, d3) = gh_one("crypto_generichash", ol, &msg, None, 0);
            assert_eq!(rc3, 0);
            assert_eq_bytes("row111: crypto_generichash != _blake2b", &d, &d3);
            // and streaming must match
            let s = gh_cmp_ok(
                "row111",
                "crypto_generichash_blake2b",
                GhInitKind::Plain { key: None, keylen: 0, outlen: ol },
                &gops(&msg, &[len], ol),
            );
            assert_eq_bytes("row111: streaming != one-shot", &d, &s.digest());
            iters += 1;
        }
    }
    assert!(iters >= 64, "row111 only ran {iters} inputs");
}

/// CONFIGS 112 — keyed one-shot: `outlen` ∈ {1,16,32,64} × `keylen`
/// ∈ {1,15,16,32,64} (1 and 15 are BELOW `KEYBYTES_MIN` and still accepted).
/// The keyed path pre-absorbs one zero-padded 128-byte block.
#[test]
fn r112_blake2b_oneshot_keyed() {
    init_both();
    let mut rng = Rng::new(SEED ^ 112);
    let mut iters = 0usize;
    for &ol in &[1usize, 15, 16, 32, 64] {
        for &kl in &[1usize, 2, 15, 16, 32, 63, 64] {
            for &len in &[0usize, 1, 127, 128, 129, 256, 1000] {
                let msg = rng.bytes(len);
                let key = rng.bytes(kl);
                let (rc, d) = gh_one("crypto_generichash_blake2b", ol, &msg, Some(&key), kl);
                assert_eq!(rc, 0, "row112: outlen={ol} keylen={kl} rejected ({rc})");
                assert_eq!(d.len(), ol);
                // keyed != unkeyed
                let (_, u) = gh_one("crypto_generichash_blake2b", ol, &msg, None, 0);
                assert_ne!(d, u, "row112: keyed digest equals the unkeyed one (keylen={kl})");
                // streaming keyed must match; the keyed init leaves buflen == 128
                let s = gh_cmp_ok(
                    "row112",
                    "crypto_generichash_blake2b",
                    GhInitKind::Plain { key: Some(&key), keylen: kl, outlen: ol },
                    &gops(&msg, &[len], ol),
                );
                assert_eq_bytes("row112: streaming != one-shot", &d, &s.digest());
                // dispatch layer
                let (_, d2) = gh_one("crypto_generichash", ol, &msg, Some(&key), kl);
                assert_eq_bytes("row112: crypto_generichash != _blake2b", &d, &d2);
                iters += 1;
            }
        }
    }
    assert!(iters >= 64, "row112 only ran {iters} inputs");
}

/// CONFIGS 113 + ERRORS 230 — `key == NULL` with `keylen > 0`:
///   * at the `*_init` wrapper this takes the UNKEYED path and returns 0;
///   * the ONE-SHOT `blake2b()` instead hits its own `NULL == key && keylen > 0`
///     guard and calls `sodium_misuse()` — asserted via `forked`.
#[test]
fn r113_e230_null_key_positive_keylen() {
    init_both();
    no_core();
    let mut rng = Rng::new(SEED ^ 113);
    let mut iters = 0usize;
    for &kl in &[1usize, 15, 16, 32, 64] {
        for &ol in &[1usize, 16, 32, 64] {
            for &len in &[0usize, 1, 129, 300] {
                let msg = rng.bytes(len);
                for prefix in GH_LAYERS {
                    // key == NULL, keylen > 0 -> unkeyed
                    let a = gh_cmp_ok(
                        "row113/e230",
                        prefix,
                        GhInitKind::Plain { key: None, keylen: kl, outlen: ol },
                        &gops(&msg, &[len], ol),
                    );
                    // keylen == 0 (unkeyed by the other half of the condition)
                    let b = gh_cmp_ok(
                        "row113/e230",
                        prefix,
                        GhInitKind::Plain { key: None, keylen: 0, outlen: ol },
                        &gops(&msg, &[len], ol),
                    );
                    assert_eq_bytes(
                        &format!("row113 {prefix}: key=NULL keylen={kl} is not the unkeyed path"),
                        &b.digest(),
                        &a.digest(),
                    );
                    assert_eq_bytes(
                        &format!("row113 {prefix}: key=NULL keylen={kl} state != unkeyed state"),
                        &b.states[0],
                        &a.states[0],
                    );
                    iters += 1;
                }
            }
        }
    }
    assert!(iters >= 64, "row113 only ran {iters} inputs");

    // The one-shot layers abort instead (blake2b()'s own NULL-key guard).
    let l = libs();
    let msg = rng.bytes(64);
    let mp = msg.as_ptr();
    for name in GH_LAYERS {
        let cf: GhOne = *unsafe { sym::<GhOne>(&l.c, name) };
        let rf: GhOne = *unsafe { sym::<GhOne>(&l.r, name) };
        let mut oc = vec![FILL; 64 + GUARD];
        let mut or = vec![FILL; 64 + GUARD];
        let pc = oc.as_mut_ptr();
        let pr = or.as_mut_ptr();
        for &kl in &[1usize, 32, 64] {
            let a = forked(move || unsafe { cf(pc, 32, mp, 64, ptr::null(), kl) as i64 });
            let b = forked(move || unsafe { rf(pr, 32, mp, 64, ptr::null(), kl) as i64 });
            assert_same_fatal(&format!("row113 {name}(key=NULL, keylen={kl})"), a, b);
            assert_eq!(
                a, MISUSE,
                "row113 {name}(key=NULL, keylen={kl}): C outcome was {a:?}, expected {MISUSE:?}"
            );
        }
    }
    // ...and the salt_personal one-shot too.
    let cf: GhOneSp =
        *unsafe { sym::<GhOneSp>(&l.c, "crypto_generichash_blake2b_salt_personal") };
    let rf: GhOneSp =
        *unsafe { sym::<GhOneSp>(&l.r, "crypto_generichash_blake2b_salt_personal") };
    let mut oc = vec![FILL; 64 + GUARD];
    let mut or = vec![FILL; 64 + GUARD];
    let pc = oc.as_mut_ptr();
    let pr = or.as_mut_ptr();
    let a = forked(move || unsafe {
        cf(pc, 32, mp, 64, ptr::null(), 32, ptr::null(), ptr::null()) as i64
    });
    let b = forked(move || unsafe {
        rf(pr, 32, mp, 64, ptr::null(), 32, ptr::null(), ptr::null()) as i64
    });
    assert_same_fatal("row113 salt_personal(key=NULL, keylen=32)", a, b);
    assert_eq!(a, MISUSE, "row113 salt_personal(key=NULL, keylen>0): C was {a:?}");
}

/// CONFIGS 114 — `crypto_generichash_blake2b_salt_personal`: 16-byte salt and
/// personal, `NULL` for either (zero-filled), and the requirement that a
/// distinct salt OR a distinct personal changes the digest.
#[test]
fn r114_blake2b_salt_personal_oneshot() {
    init_both();
    let mut rng = Rng::new(SEED ^ 114);
    assert_size("crypto_generichash_blake2b_saltbytes", 16);
    assert_size("crypto_generichash_blake2b_personalbytes", 16);
    let zero = vec![0u8; 16];
    let salts = patterns(16, &mut rng);
    let pers = patterns(16, &mut rng);
    let mut iters = 0usize;
    for &ol in &[1usize, 16, 32, 64] {
        for &kl in &[0usize, 1, 16, 32, 64] {
            for &len in &[0usize, 1, 128, 129, 257] {
                let msg = rng.bytes(len);
                let key = rng.bytes(kl.max(1));
                let kopt = if kl == 0 { None } else { Some(&key[..]) };

                // NULL salt/personal == all-zero salt/personal
                let (rc, dn) = gh_one_sp(ol, &msg, kopt, kl, None, None);
                assert_eq!(rc, 0, "row114: rejected (C returned {rc})");
                let (_, dz) = gh_one_sp(ol, &msg, kopt, kl, Some(&zero), Some(&zero));
                assert_eq_bytes("row114: NULL salt/personal != zero salt/personal", &dn, &dz);
                let (_, dsn) = gh_one_sp(ol, &msg, kopt, kl, Some(&zero), None);
                assert_eq_bytes("row114: NULL personal != zero personal", &dn, &dsn);
                let (_, dnp) = gh_one_sp(ol, &msg, kopt, kl, None, Some(&zero));
                assert_eq_bytes("row114: NULL salt != zero salt", &dn, &dnp);

                // ...and equals the plain (non-salt/personal) API
                let (_, dplain) = gh_one("crypto_generichash_blake2b", ol, &msg, kopt, kl);
                assert_eq_bytes(
                    "row114: zero salt/personal != crypto_generichash_blake2b",
                    &dplain,
                    &dn,
                );

                // Distinct salt / personal must change the digest. Only assert
                // this at outlen >= 16: a 1..15-byte digest legitimately
                // collides by chance, which would be a false positive.
                // (The C-vs-Rust byte comparison above still runs at EVERY
                // outlen, including 1..15.)
                if ol >= 16 {
                    // distinct salt (personal fixed)
                    let mut seen: Vec<Vec<u8>> = vec![];
                    for s in &salts {
                        let (_, d) = gh_one_sp(ol, &msg, kopt, kl, Some(s), Some(&pers[0]));
                        assert!(
                            !seen.contains(&d),
                            "row114: salt {} produced a duplicate digest (outlen={ol})",
                            hexs(s)
                        );
                        seen.push(d);
                    }
                    // distinct personal (salt fixed)
                    let mut seen: Vec<Vec<u8>> = vec![];
                    for p in &pers {
                        let (_, d) = gh_one_sp(ol, &msg, kopt, kl, Some(&salts[0]), Some(p));
                        assert!(
                            !seen.contains(&d),
                            "row114: personal {} produced a duplicate digest (outlen={ol})",
                            hexs(p)
                        );
                        seen.push(d);
                    }
                } else {
                    // still exercise every salt/personal pair for the
                    // differential comparison at the sub-BYTES_MIN outlens
                    for s in &salts {
                        for p in &pers {
                            let (rc, _) = gh_one_sp(ol, &msg, kopt, kl, Some(s), Some(p));
                            assert_eq!(rc, 0, "row114: outlen={ol} rejected");
                        }
                    }
                }
                iters += 1;
            }
        }
    }
    assert!(iters >= 64, "row114 only ran {iters} inputs");
}

/// CONFIGS 115 — unkeyed streaming with the `inlen` set that straddles the
/// `inlen > 256 - buflen` branch of `blake2b_update`.
#[test]
fn r115_blake2b_streaming_unkeyed() {
    init_both();
    let mut rng = Rng::new(SEED ^ 115);
    let lens = [0usize, 1, 127, 128, 129, 255, 256, 257, 383, 384];
    let mut iters = 0usize;
    for prefix in GH_LAYERS {
        for &ol in &[1usize, 16, 32, 64] {
            for &len in &lens {
                let msg = rng.bytes(len);
                let s = gh_cmp_ok(
                    "row115",
                    prefix,
                    GhInitKind::Plain { key: None, keylen: 0, outlen: ol },
                    &gops(&msg, &[len], ol),
                );
                let (_, one) = gh_one("crypto_generichash_blake2b", ol, &msg, None, 0);
                assert_eq_bytes("row115: streaming != one-shot", &one, &s.digest());
                iters += 1;
            }
        }
    }
    assert!(iters >= 64, "row115 only ran {iters} inputs");
}

/// CONFIGS 116 — keyed streaming: the keyed init pre-absorbs one zero-padded
/// 128-byte block so the initial `buflen` is 128, which shifts the
/// `inlen > 256 - buflen` boundary to 128.
#[test]
fn r116_blake2b_streaming_keyed() {
    init_both();
    let mut rng = Rng::new(SEED ^ 116);
    let lens = [0usize, 1, 127, 128, 129, 255, 256, 257, 383, 384];
    let mut iters = 0usize;
    for prefix in GH_LAYERS {
        for &kl in &[1usize, 15, 32, 64] {
            for &len in &lens {
                let ol = 32usize;
                let msg = rng.bytes(len);
                let key = rng.bytes(kl);
                let s = gh_cmp_ok(
                    "row116",
                    prefix,
                    GhInitKind::Plain { key: Some(&key), keylen: kl, outlen: ol },
                    &gops(&msg, &[len], ol),
                );
                let (_, one) = gh_one("crypto_generichash_blake2b", ol, &msg, Some(&key), kl);
                assert_eq_bytes("row116: keyed streaming != keyed one-shot", &one, &s.digest());
                // the state right after a keyed init must differ from unkeyed
                let u = gh_cmp_ok(
                    "row116 unkeyed ref",
                    prefix,
                    GhInitKind::Plain { key: None, keylen: 0, outlen: ol },
                    &gops(&msg, &[len], ol),
                );
                assert_ne!(
                    s.states[0], u.states[0],
                    "row116: keyed init state equals the unkeyed one (keylen={kl})"
                );
                iters += 1;
            }
        }
    }
    assert!(iters >= 64, "row116 only ran {iters} inputs");
}

/// CONFIGS 117 — the multi-chunk `_update` scripts: 1+1, 63+65, 127+1,
/// 128+128, 1×256, 256×1, plus chunkings that land exactly on `buflen == 128`
/// (where `_final`'s extra-compress must NOT fire) and just past it.
#[test]
fn r117_blake2b_update_multichunk() {
    init_both();
    let mut rng = Rng::new(SEED ^ 117);
    let scripts: &[(&str, Vec<usize>)] = &[
        ("1+1", vec![1, 1]),
        ("63+65", vec![63, 65]),
        ("127+1", vec![127, 1]),
        ("128 (buflen==128 exactly)", vec![128]),
        ("128+128", vec![128, 128]),
        ("1x256", vec![256]),
        ("256x1", vec![1; 256]),
        ("0+128+0", vec![0, 128, 0]),
        ("127+1+128", vec![127, 1, 128]),
        ("129", vec![129]),
        ("255+1", vec![255, 1]),
        ("256+1", vec![256, 1]),
        ("64x6 (=384)", vec![64; 6]),
        ("1+127+128 (buflen==128)", vec![1, 127, 128]),
        ("128+127+1 (buflen==256)", vec![128, 127, 1]),
        ("3x128", vec![128; 3]),
    ];
    let mut iters = 0usize;
    for (name, sizes) in scripts {
        let total: usize = sizes.iter().sum();
        for &keyed in &[false, true] {
            for &ol in &[1usize, 32, 64] {
                let msg = rng.bytes(total);
                let key = rng.bytes(32);
                let kind = if keyed {
                    GhInitKind::Plain { key: Some(&key), keylen: 32, outlen: ol }
                } else {
                    GhInitKind::Plain { key: None, keylen: 0, outlen: ol }
                };
                let s = gh_cmp_ok(
                    &format!("row117 {name}"),
                    "crypto_generichash_blake2b",
                    kind,
                    &gops(&msg, sizes, ol),
                );
                let (_, one) = gh_one(
                    "crypto_generichash_blake2b",
                    ol,
                    &msg,
                    if keyed { Some(&key) } else { None },
                    if keyed { 32 } else { 0 },
                );
                assert_eq_bytes(
                    &format!("row117 {name}: chunked streaming != one-shot"),
                    &one,
                    &s.digest(),
                );
                // and the single-call streaming form
                let s1 = gh_cmp_ok(
                    &format!("row117 {name} single"),
                    "crypto_generichash_blake2b",
                    kind,
                    &gops(&msg, &[total], ol),
                );
                assert_eq_bytes(
                    &format!("row117 {name}: chunking changed the digest"),
                    &s1.digest(),
                    &s.digest(),
                );
                iters += 1;
            }
        }
    }
    assert!(iters >= 64, "row117 only ran {iters} inputs");
    assert_eq!(BLAKE2B_BUF, 2 * BLAKE2B_BLOCK);
}

/// CONFIGS 118 — `crypto_generichash_blake2b_init_salt_personal` streaming:
/// salt/personal (incl. NULL) × keyed/unkeyed × chunking.
#[test]
fn r118_blake2b_init_salt_personal_streaming() {
    init_both();
    let mut rng = Rng::new(SEED ^ 118);
    let salt = rng.bytes(16);
    let pers = rng.bytes(16);
    let zero = vec![0u8; 16];
    let scripts: &[Vec<usize>] = &[
        vec![0],
        vec![1],
        vec![127, 1],
        vec![128],
        vec![128, 128],
        vec![1; 130],
        vec![257],
        vec![63, 65, 129],
    ];
    let mut iters = 0usize;
    for sizes in scripts {
        let total: usize = sizes.iter().sum();
        let msg = rng.bytes(total);
        for &kl in &[0usize, 1, 32, 64] {
            for &ol in &[1usize, 32, 64] {
                let key = rng.bytes(kl.max(1));
                let kopt = if kl == 0 { None } else { Some(&key[..]) };
                for (sn, s, p) in [
                    ("NULL/NULL", None, None),
                    ("salt/NULL", Some(&salt[..]), None),
                    ("NULL/personal", None, Some(&pers[..])),
                    ("salt/personal", Some(&salt[..]), Some(&pers[..])),
                ] {
                    let run = gh_cmp_ok(
                        &format!("row118 {sn}"),
                        "crypto_generichash_blake2b",
                        GhInitKind::SaltPersonal {
                            key: kopt,
                            keylen: kl,
                            outlen: ol,
                            salt: s,
                            personal: p,
                        },
                        &gops(&msg, sizes, ol),
                    );
                    let (_, one) = gh_one_sp(ol, &msg, kopt, kl, s, p);
                    assert_eq_bytes(
                        &format!("row118 {sn}: streaming != one-shot"),
                        &one,
                        &run.digest(),
                    );
                    iters += 1;
                }
                // NULL salt/personal must be exactly the plain init
                let a = gh_cmp_ok(
                    "row118 null-sp",
                    "crypto_generichash_blake2b",
                    GhInitKind::SaltPersonal {
                        key: kopt,
                        keylen: kl,
                        outlen: ol,
                        salt: None,
                        personal: None,
                    },
                    &gops(&msg, sizes, ol),
                );
                let b = gh_cmp_ok(
                    "row118 plain",
                    "crypto_generichash_blake2b",
                    GhInitKind::Plain { key: kopt, keylen: kl, outlen: ol },
                    &gops(&msg, sizes, ol),
                );
                assert_eq_bytes(
                    "row118: init_salt_personal(NULL,NULL) state != init state",
                    &b.states[0],
                    &a.states[0],
                );
                assert_eq_bytes(
                    "row118: init_salt_personal(NULL,NULL) digest != init digest",
                    &b.digest(),
                    &a.digest(),
                );
                // zero salt/personal must equal NULL salt/personal
                let c = gh_cmp_ok(
                    "row118 zero-sp",
                    "crypto_generichash_blake2b",
                    GhInitKind::SaltPersonal {
                        key: kopt,
                        keylen: kl,
                        outlen: ol,
                        salt: Some(&zero),
                        personal: Some(&zero),
                    },
                    &gops(&msg, sizes, ol),
                );
                assert_eq_bytes(
                    "row118: zero salt/personal != NULL salt/personal",
                    &a.states[0],
                    &c.states[0],
                );
            }
        }
    }
    assert!(iters >= 64, "row118 only ran {iters} inputs");
}

/// CONFIGS 119 + ERRORS 234 — `_final`'s `outlen` is NOT checked against
/// `_init`'s. A mismatch silently returns 0 and produces the first `outlen`
/// bytes of the same 64-byte root; C and Rust must agree exactly.
#[test]
fn r119_e234_blake2b_final_outlen_mismatch() {
    init_both();
    let mut rng = Rng::new(SEED ^ 119);
    let mut iters = 0usize;
    for prefix in GH_LAYERS {
        for &init_ol in &[1usize, 16, 32, 63, 64] {
            for &fin_ol in &[1usize, 16, 32, 63, 64] {
                for &len in &[0usize, 1, 129, 256, 300] {
                    let msg = rng.bytes(len);
                    let kind = GhInitKind::Plain { key: None, keylen: 0, outlen: init_ol };
                    let s = gh_cmp_ok(
                        &format!("row119 init={init_ol} fin={fin_ol}"),
                        prefix,
                        kind,
                        &gops(&msg, &[len], fin_ol),
                    );
                    assert_eq!(s.digest().len(), fin_ol);
                    // The root digest is determined by `init`'s outlen (it is
                    // XORed into the parameter block); `final`'s outlen only
                    // truncates. Verify against a matching-final run.
                    let full = gh_cmp_ok(
                        &format!("row119 init={init_ol} fin=64"),
                        prefix,
                        kind,
                        &gops(&msg, &[len], 64),
                    );
                    assert_eq_bytes(
                        &format!(
                            "row119: init={init_ol} fin={fin_ol} is not a prefix of the \
                             64-byte root"
                        ),
                        &full.digest()[..fin_ol],
                        &s.digest(),
                    );
                    if init_ol == fin_ol {
                        let (_, one) = gh_one("crypto_generichash_blake2b", init_ol, &msg, None, 0);
                        assert_eq_bytes("row119: matching outlen != one-shot", &one, &s.digest());
                    }
                    iters += 1;
                }
            }
        }
    }
    assert!(iters >= 64, "row119 only ran {iters} inputs");
}

/// CONFIGS 120 + ERRORS 235/236 — the `crypto_generichash` dispatch layer:
/// every constant accessor, `_statebytes() == 384`, the primitive name, and
/// `_crypto_generichash_blake2b_pick_best_implementation`.
#[test]
fn r120_e235_e236_generichash_dispatch() {
    init_both();
    // ERRORS 236 + the blake2b twin
    assert_statebytes("crypto_generichash_statebytes", 384);
    assert_statebytes("crypto_generichash_blake2b_statebytes", 384);
    for (a, b, v) in [
        ("crypto_generichash_bytes_min", "crypto_generichash_blake2b_bytes_min", 16usize),
        ("crypto_generichash_bytes_max", "crypto_generichash_blake2b_bytes_max", 64),
        ("crypto_generichash_bytes", "crypto_generichash_blake2b_bytes", 32),
        ("crypto_generichash_keybytes_min", "crypto_generichash_blake2b_keybytes_min", 16),
        ("crypto_generichash_keybytes_max", "crypto_generichash_blake2b_keybytes_max", 64),
        ("crypto_generichash_keybytes", "crypto_generichash_blake2b_keybytes", 32),
    ] {
        assert_size(a, v);
        assert_size(b, v);
    }
    assert_cstr("crypto_generichash_primitive", "blake2b");
    assert_int_fn("_crypto_generichash_blake2b_pick_best_implementation", 0);

    let mut rng = Rng::new(SEED ^ 120);
    let mut iters = 0usize;
    for &ol in &[1usize, 16, 32, 64] {
        for &kl in &[0usize, 1, 32, 64] {
            for &len in &[0usize, 1, 128, 129, 256, 257, 1000] {
                let msg = rng.bytes(len);
                let key = rng.bytes(kl.max(1));
                let kopt = if kl == 0 { None } else { Some(&key[..]) };
                let (rcb, db) = gh_one("crypto_generichash_blake2b", ol, &msg, kopt, kl);
                let (rcd, dd) = gh_one("crypto_generichash", ol, &msg, kopt, kl);
                assert_eq!(rcb, rcd, "row120: dispatch return {rcd} != blake2b {rcb}");
                assert_eq_bytes("row120: crypto_generichash != _blake2b", &db, &dd);
                // streaming through both layers must agree bit-for-bit,
                // including the whole opaque state after init and each update
                let kind = GhInitKind::Plain { key: kopt, keylen: kl, outlen: ol };
                let ops = gops(&msg, &[len.min(64), len - len.min(64)], ol);
                let a = gh_cmp_ok("row120 blake2b", "crypto_generichash_blake2b", kind, &ops);
                let b = gh_cmp_ok("row120 dispatch", "crypto_generichash", kind, &ops);
                assert_eq!(a.rets, b.rets, "row120: dispatch return codes differ");
                for i in 0..a.states.len() {
                    assert_eq_bytes(
                        &format!("row120: dispatch state snapshot #{i} != blake2b"),
                        &a.states[i],
                        &b.states[i],
                    );
                }
                assert_eq_bytes("row120: dispatch digest != blake2b", &a.digest(), &b.digest());
                assert_eq_bytes("row120: streaming != one-shot", &db, &a.digest());
                iters += 1;
            }
        }
    }
    assert!(iters >= 64, "row120 only ran {iters} inputs");
    assert_int_fn("_crypto_generichash_blake2b_pick_best_implementation", 0);
}

/// ERRORS 222/223/224/225 (+235) — the one-shot bound checks return -1 without
/// writing anything, while `outlen`/`keylen` in 1..15 (below `*_MIN`) are
/// ACCEPTED, proving the `_MIN` constants are not enforced.
#[test]
fn e222_e224_e225_generichash_oneshot_bounds() {
    init_both();
    let mut rng = Rng::new(SEED ^ 222);
    let msg = rng.bytes(137);
    let key = rng.bytes(200);
    for name in GH_LAYERS {
        // ERRORS 222: outlen == 0
        let (rc, _) = gh_one(name, 0, &msg, Some(&key), 32);
        assert_eq!(rc, -1, "e222 {name}(outlen=0): C returned {rc}, want -1");
        // ERRORS 223: outlen > 64
        for &ol in &[65usize, 100, 255, 256, 1000] {
            let (rc, _) = gh_one(name, ol, &msg, Some(&key), 32);
            assert_eq!(rc, -1, "e223 {name}(outlen={ol}): C returned {rc}, want -1");
        }
        // ERRORS 224: keylen > 64
        for &kl in &[65usize, 100, 200] {
            let (rc, _) = gh_one(name, 32, &msg, Some(&key), kl);
            assert_eq!(rc, -1, "e224 {name}(keylen={kl}): C returned {rc}, want -1");
        }
        // both bad at once
        let (rc, _) = gh_one(name, 0, &msg, Some(&key), 200);
        assert_eq!(rc, -1, "e222/e224 {name}(outlen=0, keylen=200): got {rc}");

        // ERRORS 225: outlen 1..15 and keylen 1..15 are ACCEPTED
        for ol in 1usize..16 {
            let (rc, d) = gh_one(name, ol, &msg, None, 0);
            assert_eq!(rc, 0, "e225 {name}(outlen={ol}) must be accepted, got {rc}");
            assert_eq!(d.len(), ol);
            for kl in 1usize..16 {
                let (rc, d) = gh_one(name, ol, &msg, Some(&key), kl);
                assert_eq!(
                    rc, 0,
                    "e225 {name}(outlen={ol}, keylen={kl}) must be accepted, got {rc}"
                );
                assert_eq!(d.len(), ol);
            }
        }
        // the boundary values themselves
        for &ol in &[16usize, 64] {
            let (rc, _) = gh_one(name, ol, &msg, Some(&key), 64);
            assert_eq!(rc, 0, "e225 {name}(outlen={ol}, keylen=64): got {rc}");
        }
    }
}

/// ERRORS 226 — `crypto_generichash_blake2b_salt_personal`:
/// `outlen == 0` | `outlen > 64` | `keylen > 64` all return -1.
#[test]
fn e226_salt_personal_bounds() {
    init_both();
    let mut rng = Rng::new(SEED ^ 226);
    let msg = rng.bytes(99);
    let key = rng.bytes(200);
    let salt = rng.bytes(16);
    let pers = rng.bytes(16);
    for (s, p) in [
        (None, None),
        (Some(&salt[..]), None),
        (None, Some(&pers[..])),
        (Some(&salt[..]), Some(&pers[..])),
    ] {
        let (rc, _) = gh_one_sp(0, &msg, Some(&key), 32, s, p);
        assert_eq!(rc, -1, "e226 salt_personal(outlen=0): got {rc}");
        for &ol in &[65usize, 100, 255, 256] {
            let (rc, _) = gh_one_sp(ol, &msg, Some(&key), 32, s, p);
            assert_eq!(rc, -1, "e226 salt_personal(outlen={ol}): got {rc}");
        }
        for &kl in &[65usize, 100, 200] {
            let (rc, _) = gh_one_sp(32, &msg, Some(&key), kl, s, p);
            assert_eq!(rc, -1, "e226 salt_personal(keylen={kl}): got {rc}");
        }
        // and the accepted boundary
        let (rc, d) = gh_one_sp(64, &msg, Some(&key), 64, s, p);
        assert_eq!(rc, 0, "e226 salt_personal(64,64) must be accepted, got {rc}");
        assert_eq!(d.len(), 64);
    }
}

/// Compare one `*_init` bound-rejection call across both libraries, asserting
/// the return value AND that the whole opaque state is untouched.
fn gh_init_reject(prefix: &str, key: Option<&[u8]>, keylen: usize, outlen: usize, row: &str) {
    let l = libs();
    let kp = key.map_or(ptr::null(), |k| k.as_ptr());
    let mut sc = new_state();
    let mut sr = new_state();
    let (ci, ri) = unsafe {
        (
            sym::<GhInit>(&l.c, &format!("{prefix}_init")),
            sym::<GhInit>(&l.r, &format!("{prefix}_init")),
        )
    };
    let (rc, rr) = unsafe {
        (
            ci(sc.0.as_mut_ptr(), kp, keylen, outlen),
            ri(sr.0.as_mut_ptr(), kp, keylen, outlen),
        )
    };
    let tag = format!("{row} {prefix}_init(keylen={keylen}, outlen={outlen})");
    assert_eq!(rc, rr, "{tag}: return C={rc} rust={rr}");
    assert_eq!(rc, -1, "{tag}: C returned {rc}, want -1");
    assert_eq_bytes(&format!("{tag} OPAQUE STATE"), &sc.0, &sr.0);
    for (i, &b) in sc.0.iter().enumerate() {
        assert_eq!(
            b, FILL,
            "{tag}: C wrote to state byte {i} ({FILL:#04x} -> {b:#04x}) on a rejected init"
        );
    }
}

/// ERRORS 227/228/229 (+235) — `*_init` bound checks: `outlen == 0`,
/// `outlen > 64`, `keylen > 64`.
#[test]
fn e227_e229_generichash_init_bounds() {
    init_both();
    let mut rng = Rng::new(SEED ^ 227);
    let key = rng.bytes(200);
    for prefix in GH_LAYERS {
        gh_init_reject(prefix, Some(&key), 32, 0, "e227");
        gh_init_reject(prefix, None, 0, 0, "e227");
        for &ol in &[65usize, 100, 255, 256, 1000] {
            gh_init_reject(prefix, Some(&key), 32, ol, "e228");
            gh_init_reject(prefix, None, 0, ol, "e228");
        }
        for &kl in &[65usize, 100, 200] {
            gh_init_reject(prefix, Some(&key), kl, 32, "e229");
            // key == NULL does not save it: keylen is checked first
            gh_init_reject(prefix, None, kl, 32, "e229");
        }
        gh_init_reject(prefix, Some(&key), 200, 0, "e227/e229");
    }
}

/// ERRORS 231 — `crypto_generichash_blake2b_init_salt_personal`:
/// `outlen == 0` | `outlen > 64` | `keylen > 64` return -1 and leave the state
/// untouched, for every salt/personal NULL combination.
#[test]
fn e231_init_salt_personal_bounds() {
    init_both();
    let l = libs();
    let mut rng = Rng::new(SEED ^ 231);
    let key = rng.bytes(200);
    let salt = rng.bytes(16);
    let pers = rng.bytes(16);
    let (ci, ri) = unsafe {
        (
            sym::<GhInitSp>(&l.c, "crypto_generichash_blake2b_init_salt_personal"),
            sym::<GhInitSp>(&l.r, "crypto_generichash_blake2b_init_salt_personal"),
        )
    };
    let bad: &[(usize, usize)] = &[
        (32, 0),
        (200, 0),
        (32, 65),
        (32, 100),
        (32, 255),
        (32, 256),
        (65, 32),
        (100, 32),
        (200, 32),
    ];
    for (s, p) in [
        (None, None),
        (Some(&salt[..]), None),
        (None, Some(&pers[..])),
        (Some(&salt[..]), Some(&pers[..])),
    ] {
        let sp = s.map_or(ptr::null(), |x| x.as_ptr());
        let pp = p.map_or(ptr::null(), |x| x.as_ptr());
        for &(kl, ol) in bad {
            let mut sc = new_state();
            let mut sr = new_state();
            let (rc, rr) = unsafe {
                (
                    ci(sc.0.as_mut_ptr(), key.as_ptr(), kl, ol, sp, pp),
                    ri(sr.0.as_mut_ptr(), key.as_ptr(), kl, ol, sp, pp),
                )
            };
            let tag = format!("e231 init_salt_personal(keylen={kl}, outlen={ol})");
            assert_eq!(rc, rr, "{tag}: return C={rc} rust={rr}");
            assert_eq!(rc, -1, "{tag}: C returned {rc}, want -1");
            assert_eq_bytes(&format!("{tag} OPAQUE STATE"), &sc.0, &sr.0);
            for (i, &b) in sc.0.iter().enumerate() {
                assert_eq!(b, FILL, "{tag}: C wrote to state byte {i} on a rejected init");
            }
        }
    }
}

/// ERRORS 232 (+235) — `crypto_generichash*_final` with a bad `(uint8_t)outlen`
/// reaches `blake2b_final`'s own guard and calls `sodium_misuse()` — it does
/// NOT return -1. Asserted via `forked`: both libraries must die on SIGABRT.
/// A GOOD outlen on the very same script must instead return 0, so the row
/// cannot pass vacuously.
#[test]
fn e232_generichash_final_bad_outlen_misuse() {
    init_both();
    no_core();
    let l = libs();
    let mut rng = Rng::new(SEED ^ 232);
    let msg = rng.bytes(200);
    let mp = msg.as_ptr();
    let mlen = msg.len() as u64;
    for prefix in GH_LAYERS {
        // Resolve everything and allocate every buffer in the PARENT.
        let ci: GhInit = *unsafe { sym::<GhInit>(&l.c, &format!("{prefix}_init")) };
        let ri: GhInit = *unsafe { sym::<GhInit>(&l.r, &format!("{prefix}_init")) };
        let cu: StUpdate = *unsafe { sym::<StUpdate>(&l.c, &format!("{prefix}_update")) };
        let ru: StUpdate = *unsafe { sym::<StUpdate>(&l.r, &format!("{prefix}_update")) };
        let cf: GhFinal = *unsafe { sym::<GhFinal>(&l.c, &format!("{prefix}_final")) };
        let rf: GhFinal = *unsafe { sym::<GhFinal>(&l.r, &format!("{prefix}_final")) };
        let mut sc = new_state();
        let mut sr = new_state();
        let spc = sc.0.as_mut_ptr();
        let spr = sr.0.as_mut_ptr();
        let mut oc = vec![FILL; 256 + GUARD];
        let mut or = vec![FILL; 256 + GUARD];
        let poc = oc.as_mut_ptr();
        let por = or.as_mut_ptr();

        // 0 and 65..=255 all survive `assert(outlen <= UINT8_MAX)` in the
        // wrapper and then trip blake2b_final's `!outlen || outlen > 64`.
        for &ol in &[0usize, 65, 66, 100, 128, 254, 255] {
            let a = forked(move || unsafe {
                ci(spc, ptr::null(), 0, 32);
                cu(spc, mp, mlen);
                cf(spc, poc, ol) as i64
            });
            let b = forked(move || unsafe {
                ri(spr, ptr::null(), 0, 32);
                ru(spr, mp, mlen);
                rf(spr, por, ol) as i64
            });
            assert_same_fatal(&format!("e232 {prefix}_final(outlen={ol})"), a, b);
            assert_eq!(
                a, MISUSE,
                "e232 {prefix}_final(outlen={ol}): C outcome was {a:?}, \
                 expected sodium_misuse() -> {MISUSE:?} (NOT -1)"
            );
        }
        // Control: the identical script with a legal outlen returns 0.
        for &ol in &[1usize, 32, 64] {
            let a = forked(move || unsafe {
                ci(spc, ptr::null(), 0, 32);
                cu(spc, mp, mlen);
                cf(spc, poc, ol) as i64
            });
            let b = forked(move || unsafe {
                ri(spr, ptr::null(), 0, 32);
                ru(spr, mp, mlen);
                rf(spr, por, ol) as i64
            });
            assert_same_fatal(&format!("e232 control {prefix}_final(outlen={ol})"), a, b);
            assert_eq!(
                a,
                Outcome::Returned(0),
                "e232 control {prefix}_final(outlen={ol}) must return 0, got {a:?}"
            );
        }
    }
}

/// ERRORS 233 (+235) — `_final` called twice on the same state hits
/// `blake2b_is_lastblock` and returns -1 WITHOUT writing the output.
#[test]
fn e233_generichash_final_twice() {
    init_both();
    let mut rng = Rng::new(SEED ^ 233);
    for prefix in GH_LAYERS {
        for &ol in &[1usize, 16, 32, 64] {
            for &len in &[0usize, 1, 129, 256, 300] {
                let msg = rng.bytes(len);
                let run = gh_cmp(
                    "e233",
                    prefix,
                    GhInitKind::Plain { key: None, keylen: 0, outlen: ol },
                    &gops(&msg, &[len], ol)
                        .into_iter()
                        .chain([GOp::Fin(ol), GOp::Fin(ol)])
                        .collect::<Vec<_>>(),
                );
                // rets: [init, update, fin, fin, fin]
                assert_eq!(run.rets[0], 0, "e233: init failed");
                assert_eq!(run.rets[2], 0, "e233: first _final returned {}", run.rets[2]);
                assert_eq!(
                    run.rets[3], -1,
                    "e233 {prefix}: second _final returned {}, want -1",
                    run.rets[3]
                );
                assert_eq!(
                    run.rets[4], -1,
                    "e233 {prefix}: third _final returned {}, want -1",
                    run.rets[4]
                );
                // the rejected finals must not touch the output buffer at all
                for i in 1..3 {
                    let (_, ref b) = run.outs[i];
                    for (j, &x) in b.iter().enumerate() {
                        assert_eq!(
                            x, FILL,
                            "e233 {prefix}: rejected _final #{i} wrote to output byte {j}"
                        );
                    }
                }
            }
        }
    }
}

// ------------------------------------------------------------------ RNG rows
//
// `install_det_rng` mutates a GLOBAL in both `.so`s, so every test that touches
// it must hold this mutex and restore the default implementation afterwards.
// `cargo test` runs the tests in this file as parallel threads over one shared
// pair of libraries.

static RNG_LOCK: Mutex<()> = Mutex::new(());

fn restore_default_rng() {
    let l = libs();
    let (cs, rs) = unsafe { pair::<SetImplFn>("randombytes_set_implementation") };
    // `libloading::Symbol<T>` reinterprets the SYMBOL ADDRESS as `T`, so asking
    // for a thin pointer type yields the address of the data object itself.
    let cd = unsafe { sym::<*const RandombytesImpl>(&l.c, "randombytes_sysrandom_implementation") };
    let rd = unsafe { sym::<*const RandombytesImpl>(&l.r, "randombytes_sysrandom_implementation") };
    unsafe {
        cs(*cd);
        rs(*rd);
    }
}

/// CONFIGS 110 + 121 — every `*_keygen` in this file, driven by the injected
/// deterministic RNG so the produced keys are byte-comparable, with a trailing
/// guard region that must stay untouched.
#[test]
fn r110_r121_keygen_all() {
    init_both();
    let _g = RNG_LOCK.lock().unwrap();
    install_det_rng(false);

    // (symbol, key length)
    let cases: &[(&str, usize)] = &[
        ("crypto_onetimeauth_keygen", 32),
        ("crypto_onetimeauth_poly1305_keygen", 32),
        ("crypto_auth_keygen", 32),
        ("crypto_auth_hmacsha256_keygen", 32),
        ("crypto_auth_hmacsha512_keygen", 32),
        ("crypto_auth_hmacsha512256_keygen", 32),
        ("crypto_generichash_keygen", 32),
        ("crypto_generichash_blake2b_keygen", 32),
    ];
    let mut iters = 0usize;
    let mut first: Option<Vec<u8>> = None;
    for (name, kl) in cases {
        let (c, r) = unsafe { pair::<KeygenFn>(name) };
        for it in 0..9 {
            reset_det_rng();
            let mut kc = vec![FILL; kl + GUARD];
            let mut kr = vec![FILL; kl + GUARD];
            unsafe {
                c(kc.as_mut_ptr());
                r(kr.as_mut_ptr());
            }
            assert_eq_bytes(&format!("{name} output (iter {it})"), &kc, &kr);
            assert_guard(&format!("{name} (C)"), &kc, *kl);
            assert_guard(&format!("{name} (rust)"), &kr, *kl);
            assert!(
                kc[..*kl] != vec![FILL; *kl][..],
                "{name}: wrote nothing into the key buffer"
            );
            // every one of these keygens is `randombytes_buf(k, 32)`, so with a
            // reset counter they must all produce the SAME stream
            match &first {
                None => first = Some(kc[..*kl].to_vec()),
                Some(f) => assert_eq_bytes(
                    &format!("{name}: randombytes_buf(32) stream differs from the first keygen"),
                    f,
                    &kc[..*kl],
                ),
            }
            iters += 1;
        }
    }
    assert!(iters >= 64, "rows 110/121 only ran {iters} inputs");

    // Advancing the counter must change the output (proves the RNG is used).
    let (c, r) = unsafe { pair::<KeygenFn>("crypto_generichash_blake2b_keygen") };
    reset_det_rng();
    let mut a = vec![FILL; 32];
    let mut b = vec![FILL; 32];
    unsafe {
        c(a.as_mut_ptr());
        r(b.as_mut_ptr());
    }
    assert_eq_bytes("keygen: first draw", &a, &b);
    let mut a2 = vec![FILL; 32];
    let mut b2 = vec![FILL; 32];
    unsafe {
        c(a2.as_mut_ptr());
        r(b2.as_mut_ptr());
    }
    assert_eq_bytes("keygen: second draw", &a2, &b2);
    assert_ne!(a, a2, "keygen: the injected RNG did not advance");

    restore_default_rng();
}
