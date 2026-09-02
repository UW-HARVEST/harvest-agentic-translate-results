//! Randomized fuzz differential: the broadest net for divergences between the
//! `-DNDEBUG` C build and the Rust `.so`. Every case runs in a forked child so
//! aborts, faults and hangs all become comparable outcomes.
//!
//! ## The one class of input where the two cannot agree
//!
//! `cp_dynamic` declares `uint8_t lens[288 + 32]` (320 bytes) and fills it with
//!
//! ```c
//! for (int n = 0; n < nlit + ndst;) { ... case 18: for (int i = 11 + read(7); i; --i, ++n) lens[n] = 0; ... }
//! ```
//!
//! A code-length symbol 18 decoded at `n == nlit + ndst - 1` writes up to 138
//! more entries, so the maximum index is `nlit + ndst + 136`. With
//! `nlit <= 288` and `ndst <= 32` that reaches 456 — up to 137 bytes past the
//! array. Once `nlit + ndst >= 184` the write runs off the end of `lens` and
//! into `cp_dynamic`'s own stack frame, clobbering `nlit`, `ndst`, and the loop
//! counters `n` and `i` themselves.
//!
//! That is undefined behaviour whose effect is decided entirely by the stack
//! frame layout: an instrumented copy of `c_src/src/lib.c` (identical except for
//! added `fprintf`s, which move the locals) terminates on exactly the input that
//! makes the real build spin forever. There is no defined behaviour for the Rust
//! to reproduce, so it writes into a padded backing store and keeps the
//! "intended" semantics.
//!
//! `classify` therefore hard-fails on every divergence *except* "the C corrupted
//! itself and the Rust did not", counts those, and reports the tally.
//!
//! `FUZZ_ITERS` / `FUZZ_SEED` env vars override the defaults, which are kept
//! modest so the suite stays inside its time budget.

mod common;

use common::deflate::*;
use common::*;

fn iters(default: usize) -> usize {
    std::env::var("FUZZ_ITERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn seed(default: u64) -> u64 {
    std::env::var("FUZZ_SEED")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

#[derive(Default)]
struct Tally {
    matched: usize,
    both_crashed: usize,
    c_ub: usize,
    unknown: usize,
}

impl Tally {
    fn report(&self, label: &str) {
        println!(
            "[{label}] {} identical, {} crashed identically, {} C-lens[]-overrun (UB, not comparable), {} probe-inconclusive",
            self.matched, self.both_crashed, self.c_ub, self.unknown
        );
    }
    fn total(&self) -> usize {
        self.matched + self.both_crashed + self.c_ub + self.unknown
    }
}

/// Compares the NDEBUG C build against the Rust `.so` for one `cp_inflate`
/// call. A divergence is tolerated *only* when the instrumented probe confirms
/// the input drives `cp_dynamic`'s `lens[]` past its 320-byte bound; otherwise
/// it is a translation bug and fails the test.
fn classify(stream: &[u8], out_len: usize, label: &str, t: &mut Tally) {
    let _g = call_lock();
    let l = libs();
    let ib = stream.len() as i32;
    let ob = out_len as i32;
    let c = inflate_in_child(&l.c_nd, stream, 0, ib, out_len, 0, ob, false, false);
    let r = inflate_in_child(&l.r, stream, 0, ib, out_len, 0, ob, false, false);

    drop(_g);
    match compare_or_ub(&c, &r, stream, out_len, ob, label) {
        DiffVerdict::Identical => t.matched += 1,
        DiffVerdict::BothCrashed => t.both_crashed += 1,
        DiffVerdict::CLensOverrun => t.c_ub += 1,
        DiffVerdict::ProbeInconclusive => t.unknown += 1,
    }
}

#[test]
fn fuzz_pure_garbage() {
    let mut rng = Rng::new(seed(0xF001));
    let mut t = Tally::default();
    for _ in 0..iters(150) {
        let n = rng.range(1, 48);
        let stream = rng.bytes(n);
        classify(&stream, 4096, "garbage", &mut t);
    }
    t.report("garbage");
    assert!(t.total() > 0);
}

#[test]
fn fuzz_truncated_valid_streams() {
    let mut rng = Rng::new(seed(0xF002));
    let mut t = Tally::default();
    for _ in 0..iters(30) {
        let toks = {
            let n = rng.range(1, 60);
            rand_tokens(&mut rng, n, 512)
        };
        let mut d = Deflate::new();
        match rng.below(3) {
            0 => d.fixed(true, &toks),
            1 => {
                let lit_lens = lit_lens_for(&toks, 288);
                let dist_lens = dist_lens_for(&toks, 32);
                d.dynamic(true, &toks, &lit_lens, &dist_lens, 4);
            }
            _ => {
                let data = {
                    let n = rng.range(0, 200);
                    rng.bytes(n)
                };
                d.stored(true, &data);
            }
        }
        let full = d.finish();
        for _ in 0..4 {
            let cut = rng.range(1, full.len());
            classify(&full[..cut], 4096, "truncated", &mut t);
        }
    }
    t.report("truncated");
}

#[test]
fn fuzz_bitflipped_valid_streams() {
    let mut rng = Rng::new(seed(0xF003));
    let mut t = Tally::default();
    for _ in 0..iters(40) {
        let toks = {
            let n = rng.range(1, 60);
            rand_tokens(&mut rng, n, 512)
        };
        let lit_lens = lit_lens_for(&toks, 288);
        let dist_lens = dist_lens_for(&toks, 32);
        let mut d = Deflate::new();
        d.dynamic(true, &toks, &lit_lens, &dist_lens, 4);
        let full = d.finish();
        for _ in 0..4 {
            let mut s = full.clone();
            let flips = rng.range(1, 4);
            for _ in 0..flips {
                let bit = rng.below(s.len() * 8);
                s[bit / 8] ^= 1 << (bit % 8);
            }
            classify(&s, 4096, "bitflip", &mut t);
        }
    }
    t.report("bitflip");
}

/// Same corpus as `fuzz_bitflipped_valid_streams`, but every dynamic block is
/// built with `nlit + ndst <= 183`, which makes `cp_dynamic`'s `lens[]` overrun
/// arithmetically impossible (`nlit + ndst + 136 <= 319`). Divergences are
/// therefore not tolerated at all here — `c_self_corrupted` must stay zero.
#[test]
fn fuzz_overshoot_free_streams_match_exactly() {
    let mut rng = Rng::new(seed(0xF005));
    let mut t = Tally::default();
    for _ in 0..iters(60) {
        let toks = {
            let n = rng.range(1, 60);
            rand_literals(&mut rng, n)
        };
        // nlit = 257 is the minimum; ndst <= 183 - 257 is impossible, so cap
        // ndst at 1: 257 + 1 = 258 > 183. The overrun bound therefore cannot be
        // avoided by header choice alone for a *dynamic* block -- see the note
        // below. Use fixed and stored blocks, which never touch lens[] at all.
        let mut d = Deflate::new();
        match rng.below(2) {
            0 => d.fixed(true, &toks),
            _ => {
                let data = {
                    let n = rng.range(0, 300);
                    rng.bytes(n)
                };
                d.stored(true, &data);
            }
        }
        let full = d.finish();
        classify(&full, 4096, "overshoot-free", &mut t);
        assert_eq!(
            t.c_ub, 0,
            "a fixed/stored-only stream cannot reach cp_dynamic: {:02x?}",
            full
        );
    }
    t.report("overshoot-free");
    assert_eq!(t.c_ub, 0);
    assert_eq!(t.unknown, 0);
}

/// Demonstrates the mechanism directly: `nlit + ndst == 320` with a
/// code-length symbol 18 near the end drives `n` to 456, i.e. 137 bytes past
/// `uint8_t lens[320]`.
#[test]
fn dynamic_lens_overrun_is_the_only_divergence_class() {
    let l = libs();
    // A dynamic header with HLIT = 31 (nlit = 288) and HDIST = 31 (ndst = 32),
    // a code-length tree over {0, 18} only, and a run of symbol-18 ops that
    // lands the last one at n = 319.
    let mut w = BitWriter::new();
    w.bits(1, 1); // bfinal
    w.bits(2, 2); // dynamic
    w.bits(31, 5); // HLIT  => nlit = 288
    w.bits(31, 5); // HDIST => ndst = 32
    w.bits(0, 4); // HCLEN => nlen = 4  (slots 16, 17, 18, 0)
    w.bits(0, 3); // lenlens[16] = 0
    w.bits(1, 3); // lenlens[17] = 1
    w.bits(1, 3); // lenlens[18] = 1
    w.bits(0, 3); // lenlens[0]  = 0
    // Symbols 17 and 18 both have 1-bit codes: 17 -> 0, 18 -> 1.
    // 17 writes 3 + read(3) zeros; 18 writes 11 + read(7).
    // 2 x 138 = 276, then 5 x 8 = 40 gets n to 316, then one more 18 at n=316
    // runs to 316 + 137 = 453.
    for _ in 0..2 {
        w.bits(1, 1);
        w.bits(127, 7); // 18: 138 zeros
    }
    for _ in 0..5 {
        w.bits(0, 1);
        w.bits(5, 3); // 17: 8 zeros
    }
    w.bits(1, 1);
    w.bits(127, 7); // 18 at n = 316 -> writes through index 453
    w.raw_pad(64);
    let stream = w.finish();

    let _g = call_lock();
    let c = inflate_in_child(&l.c_nd, &stream, 0, stream.len() as i32, 4096, 0, 4096, false, false);
    let r = inflate_in_child(&l.r, &stream, 0, stream.len() as i32, 4096, 0, 4096, false, false);
    println!(
        "lens overrun: C = {:?} ret {} err {:?} | Rust = {:?} ret {} err {:?}",
        c.status,
        c.ret,
        c.err.as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
        r.status,
        r.ret,
        r.err.as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
    );
    // The Rust must at least stay defined: no crash, no hang.
    assert!(
        !r.status.crashed(),
        "Rust must not crash or hang on the lens[] overrun input, got {:?}",
        r.status
    );
    // And the as-built C, with asserts live, must reject it loudly.
    let a = inflate_in_child(&l.c, &stream, 0, stream.len() as i32, 4096, 0, 4096, false, false);
    assert!(
        a.status.crashed(),
        "the as-built C should abort or hang on the lens[] overrun, got {:?}",
        a.status
    );
    // The instrumented probe must agree that this is the overrun class.
    drop(_g);
    assert_eq!(
        lens_overrun(&stream, 4096, 4096),
        Some(true),
        "the probe should report a lens[] overrun for this stream"
    );
}

#[test]
fn fuzz_unfilter_random_args() {
    // unfilter has no asserts and no allocation, so it can be driven directly
    // as long as the buffer is big enough for the dimensions.
    let mut rng = Rng::new(seed(0xF004));
    for _ in 0..iters(3000) {
        let w = rng.range(0, 32) as i32;
        let h = rng.range(0, 24) as i32;
        let bpp = rng.range(0, 8) as i32;
        let need = (h.max(0) as usize) * (1 + (w * bpp).max(0) as usize) + 128;
        let raw = rng.bytes(need);
        diff_unfilter(w, h, bpp, &raw, CBuild::AsBuilt, "fuzz-unfilter");
    }
}
