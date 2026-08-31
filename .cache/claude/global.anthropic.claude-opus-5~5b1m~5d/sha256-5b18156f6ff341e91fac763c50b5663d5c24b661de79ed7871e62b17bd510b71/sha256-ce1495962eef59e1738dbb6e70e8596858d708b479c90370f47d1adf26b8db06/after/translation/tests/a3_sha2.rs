//! Area 3 — SHA-2: `crypto_hash/crypto_hash.c`,
//! `crypto_hash/sha256/{hash_sha256.c, cp/hash_sha256_cp.c}` and
//! `crypto_hash/sha512/{hash_sha512.c, cp/hash_sha512_cp.c}`.
//!
//! Every check is differential: the C reference and the translated Rust are
//! both reached through `dlsym`, never by calling the Rust crate directly.
//!
//! The `crypto_hash_sha{256,512}_state` structs are *public* (declared in
//! `crypto_hash_sha256.h` / `crypto_hash_sha512.h`), so this file allocates
//! the state as `statebytes()` raw bytes and compares the **full opaque state
//! image** between C and Rust after `init` and after every single `update` —
//! a much stronger property than merely matching final digests.
mod common;
use common::*;
use std::ffi::{c_char, c_int, CStr};

// ------------------------------------------------------------------- types

type Hash = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
type Init = unsafe extern "C" fn(*mut u8) -> c_int;
type Update = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
type Fin = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;
type SizeFn = unsafe extern "C" fn() -> usize;
type StrFn = unsafe extern "C" fn() -> *const c_char;

/// One SHA-2 family, with the C and Rust entry points side by side.
#[derive(Clone, Copy)]
struct Fam {
    name: &'static str,
    /// digest size
    outlen: usize,
    /// compression block size (64 / 128)
    block: usize,
    /// `statebytes()`
    sb: usize,
    one: (Hash, Hash),
    init: (Init, Init),
    upd: (Update, Update),
    fin: (Fin, Fin),
}

fn sizes(name: &str) -> (usize, usize) {
    let (c, r) = both::<SizeFn>(name);
    unsafe { (c(), r()) }
}

fn fam(name: &'static str, outlen: usize, block: usize) -> Fam {
    let (bc, br) = sizes(&format!("{name}_bytes"));
    assert_eq!(bc, br, "{name}_bytes: C {bc} vs Rust {br}");
    assert_eq!(bc, outlen, "{name}_bytes must be {outlen}");
    let (sc, sr) = sizes(&format!("{name}_statebytes"));
    assert_eq!(sc, sr, "{name}_statebytes: C {sc} vs Rust {sr}");

    let one = both::<Hash>(name);
    let init = both::<Init>(&format!("{name}_init"));
    let upd = both::<Update>(&format!("{name}_update"));
    let fin = both::<Fin>(&format!("{name}_final"));
    Fam {
        name,
        outlen,
        block,
        sb: sc,
        one: (*one.0, *one.1),
        init: (*init.0, *init.1),
        upd: (*upd.0, *upd.1),
        fin: (*fin.0, *fin.1),
    }
}

fn sha256() -> Fam {
    fam("crypto_hash_sha256", 32, 64)
}
fn sha512() -> Fam {
    fam("crypto_hash_sha512", 64, 128)
}

// --------------------------------------------------------------- state slab
//
// `statebytes()` bytes of 8-byte-aligned storage (the structs contain
// `uint64_t` members) followed by a guard pattern, so an over-long write by
// either implementation is caught.

struct St {
    w: Vec<u64>,
    n: usize,
}

impl St {
    fn new(n: usize) -> St {
        let words = (n + PAD + 7) / 8;
        let mut s = St { w: vec![0u64; words], n };
        let total = words * 8;
        let b = unsafe { std::slice::from_raw_parts_mut(s.w.as_mut_ptr() as *mut u8, total) };
        for i in n..total {
            b[i] = 0xA5u8.wrapping_add((i - n) as u8);
        }
        s
    }
    fn ptr(&mut self) -> *mut u8 {
        self.w.as_mut_ptr() as *mut u8
    }
    fn raw(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.w.as_ptr() as *const u8, self.w.len() * 8) }
    }
    /// The `statebytes()` prefix — the part the library owns.
    fn body(&self) -> &[u8] {
        &self.raw()[..self.n]
    }
    /// Overwrite state bytes `off..off+v.len()` (used to forge bit counters).
    fn poke(&mut self, off: usize, v: &[u8]) {
        assert!(off + v.len() <= self.n);
        let p = self.ptr();
        unsafe { std::ptr::copy_nonoverlapping(v.as_ptr(), p.add(off), v.len()) };
    }
    #[track_caller]
    fn check(&self, what: &str) {
        check_pad(what, self.raw(), self.n);
    }
}

// ------------------------------------------------------------------ drivers

/// One-shot on both libraries; returns the (agreed) digest.
#[track_caller]
fn one(f: &Fam, data: &[u8], tag: &str) -> Vec<u8> {
    let mut oc = padded(f.outlen);
    let mut or = padded(f.outlen);
    let (rc, rr) = unsafe {
        (
            (f.one.0)(oc.as_mut_ptr(), data.as_ptr(), data.len() as u64),
            (f.one.1)(or.as_mut_ptr(), data.as_ptr(), data.len() as u64),
        )
    };
    eqi(&format!("{}({tag}) rc", f.name), rc, rr);
    assert_eq!(rc, 0, "{}({tag}) must return 0 (infallible)", f.name);
    eqb(&format!("{}({tag}) digest", f.name), &oc[..f.outlen], &or[..f.outlen]);
    check_pad(&format!("{}({tag}) C out", f.name), &oc, f.outlen);
    check_pad(&format!("{}({tag}) Rust out", f.name), &or, f.outlen);
    oc.truncate(f.outlen);
    oc
}

/// `init` / `update`×n / `final`, comparing the *whole* opaque state after
/// `init` and after every `update`, then the digest.  Returns the digest.
#[track_caller]
fn stream(f: &Fam, data: &[u8], chunks: &[usize], tag: &str) -> Vec<u8> {
    assert_eq!(
        chunks.iter().sum::<usize>(),
        data.len(),
        "{tag}: chunk sizes must add up to the input length"
    );
    let mut sc = St::new(f.sb);
    let mut sr = St::new(f.sb);

    let (ic, ir) = unsafe { ((f.init.0)(sc.ptr()), (f.init.1)(sr.ptr())) };
    eqi(&format!("{}_init({tag}) rc", f.name), ic, ir);
    assert_eq!(ic, 0);
    eqb(&format!("{}_init({tag}) state", f.name), sc.body(), sr.body());
    // self-check: `init` copies the IV, so the inspected window really is the
    // live state and not some unrelated (still zeroed) memory.
    assert!(
        sc.body().iter().any(|&b| b != 0),
        "{}_init({tag}): state image is all zero — wrong memory inspected?",
        f.name
    );

    let mut off = 0usize;
    for (k, &n) in chunks.iter().enumerate() {
        let p = data[off..off + n].as_ptr();
        let (uc, ur) = unsafe {
            (
                (f.upd.0)(sc.ptr(), p, n as u64),
                (f.upd.1)(sr.ptr(), p, n as u64),
            )
        };
        eqi(&format!("{}_update({tag}) #{k} len {n} rc", f.name), uc, ur);
        assert_eq!(uc, 0);
        eqb(
            &format!("{}_update({tag}) #{k} len {n} state", f.name),
            sc.body(),
            sr.body(),
        );
        off += n;
    }

    let mut oc = padded(f.outlen);
    let mut or = padded(f.outlen);
    let (fc, fr) = unsafe {
        (
            (f.fin.0)(sc.ptr(), oc.as_mut_ptr()),
            (f.fin.1)(sr.ptr(), or.as_mut_ptr()),
        )
    };
    eqi(&format!("{}_final({tag}) rc", f.name), fc, fr);
    assert_eq!(fc, 0);
    eqb(&format!("{}_final({tag}) digest", f.name), &oc[..f.outlen], &or[..f.outlen]);
    check_pad(&format!("{}_final({tag}) C out", f.name), &oc, f.outlen);
    check_pad(&format!("{}_final({tag}) Rust out", f.name), &or, f.outlen);
    // `final` ends with sodium_memzero(state): the whole struct must be zero.
    eqb(&format!("{}_final({tag}) state after", f.name), sc.body(), sr.body());
    assert!(
        sc.body().iter().all(|&b| b == 0),
        "{}_final({tag}): C did not zero the state",
        f.name
    );
    assert!(
        sr.body().iter().all(|&b| b == 0),
        "{}_final({tag}): Rust did not zero the state",
        f.name
    );
    sc.check(&format!("{}({tag}) C state guard", f.name));
    sr.check(&format!("{}({tag}) Rust state guard", f.name));

    oc.truncate(f.outlen);
    oc
}

/// `stream` + the one-shot must agree.
#[track_caller]
fn both_ways(f: &Fam, data: &[u8], chunks: &[usize], tag: &str) {
    let a = one(f, data, tag);
    let b = stream(f, data, chunks, tag);
    eqb(&format!("{}({tag}) one-shot vs streaming", f.name), &a, &b);
}

// ------------------------------------------------------------- length sets

/// 0..=300 plus the block-multiple neighbourhoods and several multi-KiB sizes.
fn lengths_full() -> Vec<usize> {
    let mut v: Vec<usize> = (0..=300).collect();
    v.extend_from_slice(&[
        511, 512, 513, 1023, 1024, 1025, 2047, 2048, 2049, 4095, 4096, 4097, 8192, 10000, 12345,
    ]);
    v
}

/// A cheaper set that still straddles every SHA-2 boundary.
fn lengths_boundary() -> Vec<usize> {
    vec![
        0, 1, 2, 55, 56, 57, 63, 64, 65, 111, 112, 113, 119, 120, 127, 128, 129, 135, 136, 137,
        143, 144, 183, 184, 191, 192, 239, 240, 247, 248, 255, 256, 257, 511, 512, 513, 1024, 1025,
    ]
}

// ===================================================================== 3.3 / 3.13
// one-shot over the full length ladder, randomized content.

#[test]
fn sha256_one_shot_lengths() {
    let f = sha256();
    let mut rng = Rng::new(0x5a256);
    for len in lengths_full() {
        let data = rng.bytes(len);
        one(&f, &data, &format!("len {len}"));
    }
}

#[test]
fn sha512_one_shot_lengths() {
    let f = sha512();
    let mut rng = Rng::new(0x5a512);
    for len in lengths_full() {
        let data = rng.bytes(len);
        one(&f, &data, &format!("len {len}"));
    }
}

// ===================================================================== 3.128
// content axis: all-zero, all-0xFF and the incrementing `i & 0xff` pattern
// (the LOAD32_BE / LOAD64_BE / STORE*_BE helpers are endian-sensitive).

fn patterns(len: usize) -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("zeros", vec![0u8; len]),
        ("ff", vec![0xFFu8; len]),
        ("incr", (0..len).map(|i| (i & 0xff) as u8).collect()),
        ("descr", (0..len).map(|i| !((i & 0xff) as u8)).collect()),
    ]
}

#[test]
fn sha2_content_patterns() {
    for f in [sha256(), sha512()] {
        for len in lengths_boundary() {
            for (pn, data) in patterns(len) {
                let tag = format!("{pn} len {len}");
                let a = one(&f, &data, &tag);
                let b = stream(&f, &data, &[len], &tag);
                eqb(&format!("{}({tag}) one-shot vs streaming", f.name), &a, &b);
            }
        }
    }
}

// ===================================================================== 3.1 / 3.2 / 3.131
// the generic `crypto_hash` wrapper is a pure tail call to crypto_hash_sha512.

#[test]
fn crypto_hash_generic_equals_sha512() {
    let (cc, cr) = both::<Hash>("crypto_hash");
    let (ch_c, ch_r): (Hash, Hash) = (*cc, *cr);
    let f = sha512();
    let mut rng = Rng::new(0xc0ffee);
    for len in lengths_full() {
        let data = rng.bytes(len);
        let mut a = padded(64);
        let mut b = padded(64);
        let (rc, rr) = unsafe {
            (
                ch_c(a.as_mut_ptr(), data.as_ptr(), len as u64),
                ch_r(b.as_mut_ptr(), data.as_ptr(), len as u64),
            )
        };
        eqi(&format!("crypto_hash(len {len}) rc"), rc, rr);
        assert_eq!(rc, 0, "crypto_hash must always return 0");
        eqb(&format!("crypto_hash(len {len}) digest"), &a[..64], &b[..64]);
        check_pad(&format!("crypto_hash(len {len}) C out"), &a, 64);
        check_pad(&format!("crypto_hash(len {len}) Rust out"), &b, 64);
        let d = one(&f, &data, &format!("len {len}"));
        eqb(
            &format!("crypto_hash(len {len}) == crypto_hash_sha512"),
            &a[..64],
            &d,
        );
    }
    // content axis for the wrapper too
    for len in [0usize, 1, 111, 112, 127, 128, 129, 256] {
        for (pn, data) in patterns(len) {
            let mut a = padded(64);
            let mut b = padded(64);
            unsafe {
                eqi(
                    &format!("crypto_hash({pn} len {len}) rc"),
                    ch_c(a.as_mut_ptr(), data.as_ptr(), len as u64),
                    ch_r(b.as_mut_ptr(), data.as_ptr(), len as u64),
                );
            }
            eqb(&format!("crypto_hash({pn} len {len})"), &a[..64], &b[..64]);
            let d = one(&f, &data, &format!("{pn} len {len}"));
            eqb(&format!("crypto_hash({pn} len {len}) == sha512"), &a[..64], &d);
        }
    }
    // `in == NULL` with `inlen == 0`: update() returns before any deref.
    let mut a = padded(64);
    let mut b = padded(64);
    unsafe {
        eqi(
            "crypto_hash(NULL, 0) rc",
            ch_c(a.as_mut_ptr(), std::ptr::null(), 0),
            ch_r(b.as_mut_ptr(), std::ptr::null(), 0),
        );
    }
    eqb("crypto_hash(NULL, 0)", &a[..64], &b[..64]);
    check_pad("crypto_hash(NULL,0) C out", &a, 64);
    check_pad("crypto_hash(NULL,0) Rust out", &b, 64);
}

#[test]
fn sha2_oneshot_null_input_zero_length() {
    for f in [sha256(), sha512()] {
        let mut oc = padded(f.outlen);
        let mut or = padded(f.outlen);
        let (rc, rr) = unsafe {
            (
                (f.one.0)(oc.as_mut_ptr(), std::ptr::null(), 0),
                (f.one.1)(or.as_mut_ptr(), std::ptr::null(), 0),
            )
        };
        eqi(&format!("{}(NULL,0) rc", f.name), rc, rr);
        eqb(&format!("{}(NULL,0)", f.name), &oc[..f.outlen], &or[..f.outlen]);
        check_pad(&format!("{}(NULL,0) C out", f.name), &oc, f.outlen);
        check_pad(&format!("{}(NULL,0) Rust out", f.name), &or, f.outlen);
        // must equal the empty-message digest computed with a non-NULL pointer
        let d = one(&f, &[], "empty non-null");
        eqb(&format!("{}(NULL,0) == {}(non-null,0)", f.name, f.name), &oc[..f.outlen], &d);
    }
}

// ===================================================================== 3.4 / 3.14 / 3.12 / 3.22
// streaming with a single update of each length; full-state comparison.

#[test]
fn sha256_stream_single_update() {
    let f = sha256();
    let mut rng = Rng::new(0x1256);
    for len in lengths_full() {
        let data = rng.bytes(len);
        both_ways(&f, &data, &[len], &format!("1x{len}"));
    }
}

#[test]
fn sha512_stream_single_update() {
    let f = sha512();
    let mut rng = Rng::new(0x1512);
    for len in lengths_full() {
        let data = rng.bytes(len);
        both_ways(&f, &data, &[len], &format!("1x{len}"));
    }
}

// ===================================================================== 3.5 / 3.15
// 1-byte updates: `r` walks the whole block, and the lazy
// `inlen < block - r` branch is taken at every offset.

#[test]
fn sha2_one_byte_updates() {
    for f in [sha256(), sha512()] {
        let mut rng = Rng::new(0x7b17e ^ f.block as u64);
        for total in [0usize, 1, 63, 64, 65, 127, 128, 129, 300] {
            let data = rng.bytes(total);
            let chunks = vec![1usize; total];
            both_ways(&f, &data, &chunks, &format!("{total}x1"));
        }
    }
}

// ===================================================================== 3.6 / 3.16
// two-update matrix: `r != 0` entry, the `block - r` fill-and-transform, the
// bulk `while (inlen >= block)` loop and the `inlen &= block-1` tail.

#[test]
fn sha256_two_update_matrix() {
    let f = sha256();
    let firsts = [0usize, 1, 31, 32, 33, 63, 64, 65];
    let totals = [
        0usize, 1, 63, 64, 65, 127, 128, 129, 135, 136, 137, 143, 144, 191, 192, 255, 256, 257,
    ];
    let mut rng = Rng::new(0x2_0256);
    for &a in &firsts {
        for &t in &totals {
            if a > t {
                continue;
            }
            let data = rng.bytes(t);
            both_ways(&f, &data, &[a, t - a], &format!("({a},{})", t - a));
        }
    }
}

#[test]
fn sha512_two_update_matrix() {
    let f = sha512();
    let firsts = [0usize, 1, 63, 64, 65, 127, 128, 129];
    let totals = [
        0usize, 1, 63, 64, 65, 127, 128, 129, 135, 136, 137, 143, 144, 255, 256, 257, 383, 384,
        385,
    ];
    let mut rng = Rng::new(0x2_0512);
    for &a in &firsts {
        for &t in &totals {
            if a > t {
                continue;
            }
            let data = rng.bytes(t);
            both_ways(&f, &data, &[a, t - a], &format!("({a},{})", t - a));
        }
    }
}

// ===================================================================== randomized splits
// multi-chunk splits that straddle the 64-byte (sha256) / 128-byte (sha512)
// block boundaries, with zero-length updates interleaved.

#[test]
fn sha256_random_splits() {
    random_splits(&sha256(), 0xd15c256);
}

#[test]
fn sha512_random_splits() {
    random_splits(&sha512(), 0xd15c512);
}

fn random_splits(f: &Fam, seed: u64) {
    let mut rng = Rng::new(seed);
    let totals = [
        0usize, 1, 2, 63, 64, 65, 127, 128, 129, 130, 200, 255, 256, 257, 384, 512, 700, 1024,
        1025, 2000, 4096,
    ];
    for &total in &totals {
        for trial in 0..6 {
            let data = rng.bytes(total);
            // random chunking biased towards the block boundary
            let mut chunks: Vec<usize> = Vec::new();
            let mut left = total;
            while left > 0 {
                // sometimes a zero-length update in the middle
                if rng.below(4) == 0 {
                    chunks.push(0);
                }
                let n = match rng.below(5) {
                    0 => 1,
                    1 => rng.range(1, std::cmp::min(left, f.block + 2)),
                    2 => {
                        // land exactly on / just off the next block boundary
                        let done: usize = chunks.iter().sum();
                        let r = done % f.block;
                        let to_edge = f.block - r;
                        let cand = to_edge + rng.below(3);
                        std::cmp::max(1, std::cmp::min(left, cand))
                    }
                    3 => rng.range(1, std::cmp::min(left, 3)),
                    _ => rng.range(1, left),
                };
                let n = std::cmp::min(n, left);
                chunks.push(n);
                left -= n;
            }
            // and a zero-length update first and last
            chunks.insert(0, 0);
            chunks.push(0);
            both_ways(f, &data, &chunks, &format!("split total {total} #{trial}"));
        }
    }
}

// ===================================================================== 3.7 / 3.17 / errors 3.4 / 3.11
// `inlen == 0` is a *true* no-op: it returns before touching `count` or `buf`.

#[test]
fn sha2_zero_length_update_is_a_true_noop() {
    for f in [sha256(), sha512()] {
        let mut rng = Rng::new(0x0e0e ^ f.block as u64);
        for &pre in &[0usize, 1, 5, 63, 64, 65, 127, 128, 129, 200] {
            let data = rng.bytes(pre);
            let mut sc = St::new(f.sb);
            let mut sr = St::new(f.sb);
            unsafe {
                assert_eq!((f.init.0)(sc.ptr()), 0);
                assert_eq!((f.init.1)(sr.ptr()), 0);
            }
            if pre > 0 {
                unsafe {
                    assert_eq!((f.upd.0)(sc.ptr(), data.as_ptr(), pre as u64), 0);
                    assert_eq!((f.upd.1)(sr.ptr(), data.as_ptr(), pre as u64), 0);
                }
            }
            eqb(&format!("{}: state after {pre} B", f.name), sc.body(), sr.body());
            let snap_c = sc.body().to_vec();
            let snap_r = sr.body().to_vec();

            // zero-length update, non-NULL pointer
            let (a, b) = unsafe {
                (
                    (f.upd.0)(sc.ptr(), data.as_ptr(), 0),
                    (f.upd.1)(sr.ptr(), data.as_ptr(), 0),
                )
            };
            eqi(&format!("{}_update(0) rc", f.name), a, b);
            assert_eq!(a, 0);
            eqb(&format!("{}_update(0) must not touch C state", f.name), &snap_c, sc.body());
            eqb(&format!("{}_update(0) must not touch Rust state", f.name), &snap_r, sr.body());

            // zero-length update, NULL pointer (legal: no deref happens)
            let (a, b) = unsafe {
                (
                    (f.upd.0)(sc.ptr(), std::ptr::null(), 0),
                    (f.upd.1)(sr.ptr(), std::ptr::null(), 0),
                )
            };
            eqi(&format!("{}_update(NULL,0) rc", f.name), a, b);
            assert_eq!(a, 0);
            eqb(&format!("{}_update(NULL,0) C state", f.name), &snap_c, sc.body());
            eqb(&format!("{}_update(NULL,0) Rust state", f.name), &snap_r, sr.body());

            let mut oc = padded(f.outlen);
            let mut or = padded(f.outlen);
            unsafe {
                eqi(
                    &format!("{}_final rc", f.name),
                    (f.fin.0)(sc.ptr(), oc.as_mut_ptr()),
                    (f.fin.1)(sr.ptr(), or.as_mut_ptr()),
                );
            }
            eqb(&format!("{} zero-noop digest", f.name), &oc[..f.outlen], &or[..f.outlen]);
            check_pad("zero-noop C out", &oc, f.outlen);
            check_pad("zero-noop Rust out", &or, f.outlen);
            let d = one(&f, &data, &format!("len {pre}"));
            eqb(&format!("{} zero-noop == one-shot", f.name), &oc[..f.outlen], &d);
        }

        // an all-zero-length stream: init, many update(0), final == hash("")
        let chunks = vec![0usize; 16];
        both_ways(&f, &[], &chunks, "16 x update(0)");
    }
}

// ===================================================================== 3.8
// updates that end exactly on a block boundary (`inlen == block - r`), so
// `inlen &= block-1` yields 0 and `buf` is left untouched.

#[test]
fn sha2_exact_block_boundary_updates() {
    for f in [sha256(), sha512()] {
        let mut rng = Rng::new(0xb10c ^ f.block as u64);
        for a in 0..f.block {
            let b = f.block - a; // second update lands exactly on the edge
            for &c in &[0usize, 1, 7, f.block - 1, f.block, f.block + 1] {
                let total = a + b + c;
                let data = rng.bytes(total);
                let mut chunks = vec![];
                if a > 0 {
                    chunks.push(a);
                }
                chunks.push(b);
                if c > 0 {
                    chunks.push(c);
                }
                both_ways(&f, &data, &chunks, &format!("edge({a},{b},{c})"));
            }
        }
    }
}

// ===================================================================== 3.9 / 3.10 / 3.18 / 3.19
// SHA*_Pad short branch (`r < 56` / `r < 112`) and long, two-block branch.

#[test]
fn sha256_pad_branches() {
    let f = sha256();
    let mut rng = Rng::new(0x9ad256);
    // r < 56 (short) and r >= 56 (long) residues, at several block counts
    let mut lens: Vec<usize> = Vec::new();
    for base in [0usize, 64, 128, 192, 640] {
        for r in [0usize, 1, 2, 54, 55, 56, 57, 62, 63] {
            lens.push(base + r);
        }
    }
    for len in lens {
        let data = rng.bytes(len);
        let r = len % 64;
        let tag = format!("len {len} (r={r}, {})", if r < 56 { "short" } else { "long" });
        both_ways(&f, &data, &[len], &tag);
        // also reach the same residue through a split so `buf` is filled by
        // two different code paths before SHA256_Pad runs
        if len > 0 {
            let a = len / 2;
            both_ways(&f, &data, &[a, len - a], &format!("{tag} split"));
        }
    }
}

#[test]
fn sha512_pad_branches() {
    let f = sha512();
    let mut rng = Rng::new(0x9ad512);
    let mut lens: Vec<usize> = Vec::new();
    for base in [0usize, 128, 256, 384, 1280] {
        for r in [0usize, 1, 2, 110, 111, 112, 113, 126, 127] {
            lens.push(base + r);
        }
    }
    for len in lens {
        let data = rng.bytes(len);
        let r = len % 128;
        let tag = format!("len {len} (r={r}, {})", if r < 112 { "short" } else { "long" });
        both_ways(&f, &data, &[len], &tag);
        if len > 0 {
            let a = len / 2;
            both_ways(&f, &data, &[a, len - a], &format!("{tag} split"));
        }
    }
}

// ===================================================================== errors 3.7 / 3.8 / 3.14
// documented quirks: `final` zeroes the whole state, so a second `final` and
// an `update` after `final` silently hash from an all-zero state and still
// return 0.  C and Rust must agree on that exact behaviour.

#[test]
fn sha2_double_final_and_update_after_final() {
    for f in [sha256(), sha512()] {
        let mut rng = Rng::new(0xdead ^ f.block as u64);
        for &len in &[0usize, 1, 55, 64, 130, 200] {
            let data = rng.bytes(len);

            // ---- double final
            let mut sc = St::new(f.sb);
            let mut sr = St::new(f.sb);
            unsafe {
                assert_eq!((f.init.0)(sc.ptr()), 0);
                assert_eq!((f.init.1)(sr.ptr()), 0);
                if len > 0 {
                    assert_eq!((f.upd.0)(sc.ptr(), data.as_ptr(), len as u64), 0);
                    assert_eq!((f.upd.1)(sr.ptr(), data.as_ptr(), len as u64), 0);
                }
            }
            let mut o1c = padded(f.outlen);
            let mut o1r = padded(f.outlen);
            unsafe {
                eqi(
                    &format!("{}_final#1 rc", f.name),
                    (f.fin.0)(sc.ptr(), o1c.as_mut_ptr()),
                    (f.fin.1)(sr.ptr(), o1r.as_mut_ptr()),
                );
            }
            eqb(&format!("{} final#1 digest", f.name), &o1c[..f.outlen], &o1r[..f.outlen]);
            // state is zeroed by final
            assert!(sc.body().iter().all(|&b| b == 0));
            assert!(sr.body().iter().all(|&b| b == 0));

            let mut o2c = padded(f.outlen);
            let mut o2r = padded(f.outlen);
            let (rc, rr) = unsafe {
                (
                    (f.fin.0)(sc.ptr(), o2c.as_mut_ptr()),
                    (f.fin.1)(sr.ptr(), o2r.as_mut_ptr()),
                )
            };
            eqi(&format!("{}_final#2 rc", f.name), rc, rr);
            assert_eq!(rc, 0, "{}: a second final still returns 0", f.name);
            eqb(&format!("{} final#2 digest", f.name), &o2c[..f.outlen], &o2r[..f.outlen]);
            check_pad("final#2 C out", &o2c, f.outlen);
            check_pad("final#2 Rust out", &o2r, f.outlen);
            assert_ne!(
                &o2c[..f.outlen],
                &o1c[..f.outlen],
                "{}: double-final digest is the zero-IV value, not a repeat",
                f.name
            );
            // and it is the same for every input length (state was zeroed)
            eqb(&format!("{} final#2 state after", f.name), sc.body(), sr.body());
            sc.check("double-final C guard");
            sr.check("double-final Rust guard");

            // ---- update after final
            let mut tc = St::new(f.sb);
            let mut tr = St::new(f.sb);
            unsafe {
                assert_eq!((f.init.0)(tc.ptr()), 0);
                assert_eq!((f.init.1)(tr.ptr()), 0);
                if len > 0 {
                    assert_eq!((f.upd.0)(tc.ptr(), data.as_ptr(), len as u64), 0);
                    assert_eq!((f.upd.1)(tr.ptr(), data.as_ptr(), len as u64), 0);
                }
            }
            let mut junk_c = padded(f.outlen);
            let mut junk_r = padded(f.outlen);
            unsafe {
                (f.fin.0)(tc.ptr(), junk_c.as_mut_ptr());
                (f.fin.1)(tr.ptr(), junk_r.as_mut_ptr());
            }
            let extra = rng.bytes(70);
            let (rc, rr) = unsafe {
                (
                    (f.upd.0)(tc.ptr(), extra.as_ptr(), extra.len() as u64),
                    (f.upd.1)(tr.ptr(), extra.as_ptr(), extra.len() as u64),
                )
            };
            eqi(&format!("{}_update-after-final rc", f.name), rc, rr);
            assert_eq!(rc, 0, "{}: update after final still returns 0", f.name);
            eqb(
                &format!("{} update-after-final state", f.name),
                tc.body(),
                tr.body(),
            );
            let mut o3c = padded(f.outlen);
            let mut o3r = padded(f.outlen);
            unsafe {
                eqi(
                    &format!("{}_final after resume rc", f.name),
                    (f.fin.0)(tc.ptr(), o3c.as_mut_ptr()),
                    (f.fin.1)(tr.ptr(), o3r.as_mut_ptr()),
                );
            }
            eqb(
                &format!("{} resumed-from-zero digest", f.name),
                &o3c[..f.outlen],
                &o3r[..f.outlen],
            );
            check_pad("resume C out", &o3c, f.outlen);
            check_pad("resume Rust out", &o3r, f.outlen);
            tc.check("resume C guard");
            tr.check("resume Rust guard");
        }
    }
}

// ===================================================================== 3.129
// state reuse: init -> update -> final -> init -> update -> final must give
// exactly the fresh digest (sha2 relies on the memzero in final + a fresh init).

#[test]
fn sha2_state_reuse_after_reinit() {
    for f in [sha256(), sha512()] {
        let mut rng = Rng::new(0x9e12 ^ f.block as u64);
        let mut sc = St::new(f.sb);
        let mut sr = St::new(f.sb);
        for &len in &[0usize, 1, 64, 100, 128, 255, 300] {
            let data = rng.bytes(len);
            let mut oc = padded(f.outlen);
            let mut or = padded(f.outlen);
            unsafe {
                eqi(
                    &format!("{}_init(reuse) rc", f.name),
                    (f.init.0)(sc.ptr()),
                    (f.init.1)(sr.ptr()),
                );
            }
            eqb(&format!("{} reuse init state", f.name), sc.body(), sr.body());
            if len > 0 {
                unsafe {
                    eqi(
                        &format!("{}_update(reuse) rc", f.name),
                        (f.upd.0)(sc.ptr(), data.as_ptr(), len as u64),
                        (f.upd.1)(sr.ptr(), data.as_ptr(), len as u64),
                    );
                }
                eqb(&format!("{} reuse update state", f.name), sc.body(), sr.body());
            }
            unsafe {
                eqi(
                    &format!("{}_final(reuse) rc", f.name),
                    (f.fin.0)(sc.ptr(), oc.as_mut_ptr()),
                    (f.fin.1)(sr.ptr(), or.as_mut_ptr()),
                );
            }
            eqb(&format!("{} reuse digest", f.name), &oc[..f.outlen], &or[..f.outlen]);
            let d = one(&f, &data, &format!("reuse len {len}"));
            eqb(&format!("{} reuse == fresh", f.name), &oc[..f.outlen], &d);
            sc.check("reuse C guard");
            sr.check("reuse Rust guard");
        }
    }
}

// ===================================================================== errors 3.5 / 3.12
// The bit counter has no overflow check: `count += inlen << 3` wraps silently
// and sha512 carries into `count[0]`.  Reaching 2^61 bytes of real input is
// impossible, but the state structs are *public*, so a forged counter is a
// legitimate way to drive the wrap / carry arithmetic through both libraries.

#[test]
fn sha256_forged_bit_counter_wraps() {
    let f = sha256();
    assert_eq!(f.sb, 104, "sizeof(crypto_hash_sha256_state)");
    const COUNT_OFF: usize = 32; // uint32_t state[8]
    let mut rng = Rng::new(0xfc256);
    let counts: [u64; 8] = [
        0xFFFF_FFFF_FFFF_FF00,
        0xFFFF_FFFF_FFFF_FE00,
        0xFFFF_FFFF_FFFF_F800,
        0xFFFF_FFFF_FFFF_FFF8,
        0x8000_0000_0000_0000,
        0x0000_0000_0000_0100,
        0x1234_5678_9ABC_D000,
        u64::MAX & !0x7,
    ];
    for &cnt in &counts {
        for &len in &[1usize, 7, 32, 64, 65, 200] {
            let data = rng.bytes(len);
            let mut sc = St::new(f.sb);
            let mut sr = St::new(f.sb);
            unsafe {
                assert_eq!((f.init.0)(sc.ptr()), 0);
                assert_eq!((f.init.1)(sr.ptr()), 0);
            }
            sc.poke(COUNT_OFF, &cnt.to_le_bytes());
            sr.poke(COUNT_OFF, &cnt.to_le_bytes());
            eqb("forged sha256 count", sc.body(), sr.body());
            let (rc, rr) = unsafe {
                (
                    (f.upd.0)(sc.ptr(), data.as_ptr(), len as u64),
                    (f.upd.1)(sr.ptr(), data.as_ptr(), len as u64),
                )
            };
            eqi("sha256_update(forged count) rc", rc, rr);
            assert_eq!(rc, 0, "no overflow error is ever reported");
            eqb(
                &format!("sha256 state after forged count {cnt:#x} + {len} B"),
                sc.body(),
                sr.body(),
            );
            let mut oc = padded(f.outlen);
            let mut or = padded(f.outlen);
            unsafe {
                eqi(
                    "sha256_final(forged) rc",
                    (f.fin.0)(sc.ptr(), oc.as_mut_ptr()),
                    (f.fin.1)(sr.ptr(), or.as_mut_ptr()),
                );
            }
            eqb(
                &format!("sha256 digest after forged count {cnt:#x} + {len} B"),
                &oc[..f.outlen],
                &or[..f.outlen],
            );
            check_pad("forged sha256 C out", &oc, f.outlen);
            check_pad("forged sha256 Rust out", &or, f.outlen);
            // self-check: COUNT_OFF really is the live bit counter — the digest
            // must differ from the same input hashed with count starting at 0.
            let plain = one(&f, &data, &format!("unforged len {len}"));
            assert_ne!(
                &oc[..f.outlen],
                &plain[..],
                "forging count {cnt:#x} had no effect — wrong state offset?"
            );
            sc.check("forged sha256 C guard");
            sr.check("forged sha256 Rust guard");
        }
    }
}

#[test]
fn sha512_forged_bit_counter_carries() {
    let f = sha512();
    assert_eq!(f.sb, 208, "sizeof(crypto_hash_sha512_state)");
    const C0_OFF: usize = 64; // uint64_t state[8]
    const C1_OFF: usize = 72;
    let mut rng = Rng::new(0xfc512);
    // (count[0], count[1]) pairs; count[1] is the low half of the 128-bit
    // bit counter, so these drive `(count[1] += bitlen[1]) < bitlen[1]`.
    let pairs: [(u64, u64); 8] = [
        (0, 0xFFFF_FFFF_FFFF_FC00),
        (0, 0xFFFF_FFFF_FFFF_F800),
        (0, 0xFFFF_FFFF_FFFF_FFF8),
        (0xFFFF_FFFF_FFFF_FFFF, 0xFFFF_FFFF_FFFF_FC00),
        (7, 0x8000_0000_0000_0000),
        (0, 0x0000_0000_0000_0400),
        (0x1234, 0xFEDC_BA98_7654_3000),
        (u64::MAX, u64::MAX & !0x7),
    ];
    for &(c0, c1) in &pairs {
        for &len in &[1usize, 7, 128, 129, 300] {
            let data = rng.bytes(len);
            let mut sc = St::new(f.sb);
            let mut sr = St::new(f.sb);
            unsafe {
                assert_eq!((f.init.0)(sc.ptr()), 0);
                assert_eq!((f.init.1)(sr.ptr()), 0);
            }
            for s in [&mut sc, &mut sr] {
                s.poke(C0_OFF, &c0.to_le_bytes());
                s.poke(C1_OFF, &c1.to_le_bytes());
            }
            eqb("forged sha512 count", sc.body(), sr.body());
            let (rc, rr) = unsafe {
                (
                    (f.upd.0)(sc.ptr(), data.as_ptr(), len as u64),
                    (f.upd.1)(sr.ptr(), data.as_ptr(), len as u64),
                )
            };
            eqi("sha512_update(forged count) rc", rc, rr);
            assert_eq!(rc, 0);
            eqb(
                &format!("sha512 state after forged count ({c0:#x},{c1:#x}) + {len} B"),
                sc.body(),
                sr.body(),
            );
            let mut oc = padded(f.outlen);
            let mut or = padded(f.outlen);
            unsafe {
                eqi(
                    "sha512_final(forged) rc",
                    (f.fin.0)(sc.ptr(), oc.as_mut_ptr()),
                    (f.fin.1)(sr.ptr(), or.as_mut_ptr()),
                );
            }
            eqb(
                &format!("sha512 digest after forged count ({c0:#x},{c1:#x}) + {len} B"),
                &oc[..f.outlen],
                &or[..f.outlen],
            );
            check_pad("forged sha512 C out", &oc, f.outlen);
            check_pad("forged sha512 Rust out", &or, f.outlen);
            let plain = one(&f, &data, &format!("unforged len {len}"));
            assert_ne!(
                &oc[..f.outlen],
                &plain[..],
                "forging count ({c0:#x},{c1:#x}) had no effect — wrong state offset?"
            );
            sc.check("forged sha512 C guard");
            sr.check("forged sha512 Rust guard");
        }
    }
}

// ===================================================================== 3.130
// `out` overlapping `in`: the C reference reads all of `in` before writing
// `out`, so the aliased case is well defined and must not regress.

#[test]
fn sha2_aliased_out_and_in() {
    for f in [sha256(), sha512()] {
        let mut rng = Rng::new(0xa11a5 ^ f.block as u64);
        for &len in &[0usize, 1, 16, 31, 32, 33, 63, 64, 65, 100, 128, 200] {
            let data = rng.bytes(len);
            let expect = one(&f, &data, &format!("alias ref len {len}"));
            let cap = std::cmp::max(len, f.outlen);
            for lib in 0..2 {
                let mut buf = padded(cap);
                buf[..len].copy_from_slice(&data);
                let p = buf.as_mut_ptr();
                let rc = unsafe {
                    if lib == 0 {
                        (f.one.0)(p, p as *const u8, len as u64)
                    } else {
                        (f.one.1)(p, p as *const u8, len as u64)
                    }
                };
                assert_eq!(rc, 0);
                eqb(
                    &format!("{} aliased out==in len {len} lib {lib}", f.name),
                    &expect,
                    &buf[..f.outlen],
                );
                check_pad(&format!("{} aliased guard len {len}", f.name), &buf, cap);
            }
        }
    }
}

// ===================================================================== 3.2 / 3.11 / 3.21 / accessors

#[test]
fn sha2_accessors() {
    // crypto_hash_bytes / crypto_hash_primitive
    let (cb, rb) = sizes("crypto_hash_bytes");
    assert_eq!(cb, rb, "crypto_hash_bytes: C {cb} vs Rust {rb}");
    assert_eq!(cb, 64, "crypto_hash_bytes must be crypto_hash_sha512_BYTES");

    let (cp, rp) = both::<StrFn>("crypto_hash_primitive");
    let (cs, rs) = unsafe { (CStr::from_ptr(cp()), CStr::from_ptr(rp())) };
    assert_eq!(cs, rs, "crypto_hash_primitive mismatch");
    assert_eq!(cs.to_bytes(), b"sha512");
    // stable across calls (static storage, not a scratch buffer)
    let (cs2, rs2) = unsafe { (CStr::from_ptr(cp()), CStr::from_ptr(rp())) };
    assert_eq!(cs2.to_bytes(), b"sha512");
    assert_eq!(rs2.to_bytes(), b"sha512");

    let (c256, r256) = sizes("crypto_hash_sha256_bytes");
    assert_eq!(c256, r256);
    assert_eq!(c256, 32);
    let (c512, r512) = sizes("crypto_hash_sha512_bytes");
    assert_eq!(c512, r512);
    assert_eq!(c512, 64);

    // statebytes == sizeof(struct) with the layout fixed by the public header:
    // sha256: uint32_t[8] + uint64_t + uint8_t[64]  = 32 + 8 + 64  = 104
    // sha512: uint64_t[8] + uint64_t[2] + uint8_t[128] = 64 + 16 + 128 = 208
    let (sc, sr) = sizes("crypto_hash_sha256_statebytes");
    assert_eq!(sc, sr, "crypto_hash_sha256_statebytes: C {sc} vs Rust {sr}");
    assert_eq!(sc, 104);
    let (sc, sr) = sizes("crypto_hash_sha512_statebytes");
    assert_eq!(sc, sr, "crypto_hash_sha512_statebytes: C {sc} vs Rust {sr}");
    assert_eq!(sc, 208);

    // `crypto_hash_statebytes` does not exist in libsodium 1.0.23 (the generic
    // crypto_hash API is one-shot only) — neither library may invent it.
    let name = b"crypto_hash_statebytes\0";
    let in_c = unsafe { c_lib().get::<*const std::ffi::c_void>(name).is_ok() };
    let in_r = unsafe { rust_lib().get::<*const std::ffi::c_void>(name).is_ok() };
    assert_eq!(
        in_c, in_r,
        "crypto_hash_statebytes presence differs (C {in_c}, Rust {in_r})"
    );
    assert!(!in_c, "crypto_hash_statebytes must not be exported");
    assert!(!has("crypto_hash_statebytes"));
}

// ===================================================================== interop
// a state produced by `crypto_hash_sha512_init` in one library is *not* mixed
// across libraries, but the two must be bit-identical, which the streaming
// tests above assert.  Here we additionally pin the digest-size matrix: the
// same message under sha256 and sha512 must differ, and each must be its own
// documented length.

#[test]
fn sha2_digest_size_matrix() {
    let f256 = sha256();
    let f512 = sha512();
    let mut rng = Rng::new(0xd16e57);
    for &len in &[0usize, 1, 64, 128, 256] {
        let data = rng.bytes(len);
        let a = one(&f256, &data, &format!("matrix len {len}"));
        let b = one(&f512, &data, &format!("matrix len {len}"));
        assert_eq!(a.len(), 32);
        assert_eq!(b.len(), 64);
        assert_ne!(&a[..], &b[..32], "sha256 and sha512 must not coincide");
    }
}
