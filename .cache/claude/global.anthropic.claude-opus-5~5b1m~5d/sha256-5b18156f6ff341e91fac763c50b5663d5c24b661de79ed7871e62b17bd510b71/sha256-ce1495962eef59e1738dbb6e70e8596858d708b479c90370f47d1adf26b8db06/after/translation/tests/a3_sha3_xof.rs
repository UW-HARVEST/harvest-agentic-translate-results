//! Area 3, SHA-3 / XOF slice — `crypto_hash/sha3/hash_sha3.c` and
//! `crypto_xof/{shake128,shake256,turboshake128,turboshake256}/**`
//! (public wrappers + the `ref/` implementations) on top of
//! `crypto_core_keccak1600`.
//!
//! Covers `configs_3.md` rows 3.23 - 3.83 and `errors_3.md` rows 3.17 - 3.33.
//!
//! Everything is driven through `dlsym` on both `.so` files.  Because every
//! state in this area is a 256-byte `CRYPTO_ALIGN(16)` opaque blob whose
//! internal layout is a byte-for-byte port (`sha3_state_internal` /
//! `shake*_state_internal`), the tests compare the **full 256 state bytes**
//! after `init` and after every `update` / `final` / `squeeze`, not just the
//! digests.  The state buffers are zeroed before `init` on both sides so that
//! the struct tail padding (bytes 200..224 inside the keccak sub-state, and
//! the trailing padding of the internal struct) is deterministic.

mod common;
use common::*;
use libloading::Symbol;
use std::collections::HashSet;
use std::ffi::c_int;

// ------------------------------------------------------------------ ABI types

type OneShotHash = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
type OneShotXof = unsafe extern "C" fn(*mut u8, usize, *const u8, u64) -> c_int;
type RefXof = unsafe extern "C" fn(*mut u8, usize, *const u8, usize) -> c_int;
type Init0 = unsafe extern "C" fn(*mut u8) -> c_int;
type InitD = unsafe extern "C" fn(*mut u8, u8) -> c_int;
type Upd = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
/// The internal `*_ref_update` takes a `size_t`, not `unsigned long long`.
type RefUpd = unsafe extern "C" fn(*mut u8, *const u8, usize) -> c_int;
type Fin = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;
type Sq = unsafe extern "C" fn(*mut u8, *mut u8, usize) -> c_int;
type SzGetter = unsafe extern "C" fn() -> usize;
type U8Getter = unsafe extern "C" fn() -> u8;

// --------------------------------------------------------------- state buffer

/// Every SHA-3 / XOF state in this area is `unsigned char opaque[256]` with
/// `CRYPTO_ALIGN(16)`.
const SB: usize = 256;

/// A 16-byte-aligned state buffer with `PAD` guard bytes past the end, so that
/// an over-long write into the state itself is detected too.
#[repr(C, align(16))]
struct St([u8; SB + PAD]);

impl St {
    fn new() -> Box<St> {
        let mut s = Box::new(St([0u8; SB + PAD]));
        for (i, b) in s.0[SB..].iter_mut().enumerate() {
            *b = 0xA5u8.wrapping_add(i as u8);
        }
        s
    }
    fn p(&mut self) -> *mut u8 {
        self.0.as_mut_ptr()
    }
    fn snapshot(&self) -> Vec<u8> {
        self.0[..SB].to_vec()
    }
}

#[track_caller]
fn cmp_state(label: &str, c: &St, r: &St) {
    eqb(&format!("{label}: full state bytes"), &c.0[..SB], &r.0[..SB]);
    check_pad(&format!("{label}: state(C)"), &c.0, SB);
    check_pad(&format!("{label}: state(Rust)"), &r.0, SB);
}

// ----------------------------------------------------------------- input sets

/// `0..=300` (which straddles 72/104/136/144/168 and every +-1 neighbour)
/// plus every 2x / 3x / 4x rate multiple and several multi-KiB sizes.
fn full_lengths() -> Vec<usize> {
    let mut v: Vec<usize> = (0..=300).collect();
    v.extend([
        335, 336, 337, 407, 408, 409, 431, 432, 433, 503, 504, 505, 543, 544, 545, 671, 672, 673,
        1000, 1023, 1024, 1025, 2047, 2048, 2049, 3000, 4095, 4096, 4097, 5000, 8192,
    ]);
    v
}

/// Every rate boundary and its +-1 neighbours, plus the 2x/3x multiples.
const BOUNDARY: [usize; 30] = [
    0, 1, 2, 71, 72, 73, 103, 104, 105, 135, 136, 137, 143, 144, 145, 167, 168, 169, 215, 216, 217,
    271, 272, 273, 335, 336, 337, 407, 408, 409,
];

fn pattern(kind: usize, len: usize) -> Vec<u8> {
    match kind {
        0 => vec![0u8; len],
        1 => vec![0xffu8; len],
        2 => (0..len).map(|i| (i & 0xff) as u8).collect(),
        _ => Rng::new(0xC0FFEE ^ len as u64).bytes(len),
    }
}

/// Split points for a two-call absorb of `total` bytes.
fn splits_for(total: usize, rate: usize) -> Vec<usize> {
    let mut s: Vec<usize> = Vec::new();
    if total <= 180 {
        s.extend(0..=total);
    } else {
        for a in [
            0,
            1,
            2,
            3,
            rate - 2,
            rate - 1,
            rate,
            rate + 1,
            rate + 2,
            2 * rate - 1,
            2 * rate,
            2 * rate + 1,
            total / 2,
            total - 2,
            total - 1,
            total,
        ] {
            if a <= total {
                s.push(a);
            }
        }
        s.sort_unstable();
        s.dedup();
    }
    s
}

/// Random chunking of `total` bytes; `style` biases the chunk-size
/// distribution so that 1-byte chunks and 0-byte chunks are both common.
fn random_chunks(rng: &mut Rng, total: usize, rate: usize, style: usize) -> Vec<usize> {
    let mut out = Vec::new();
    let mut left = total;
    while left > 0 {
        // Interleave zero-length calls.
        if rng.below(4) == 0 {
            out.push(0);
        }
        let n = match style % 4 {
            0 => 1,
            1 => rng.range(1, std::cmp::min(left, 3)),
            2 => rng.range(1, std::cmp::min(left, rate + 2)),
            _ => rng.range(1, std::cmp::min(left, 2 * rate + 5)),
        };
        let n = std::cmp::min(n, left);
        out.push(n);
        left -= n;
    }
    if rng.below(2) == 0 {
        out.push(0);
    }
    out
}

// ================================================================ SHA-3 family

struct Sha3 {
    name: &'static str,
    rate: usize,
    outlen: usize,
    one: (Symbol<'static, OneShotHash>, Symbol<'static, OneShotHash>),
    init: (Symbol<'static, Init0>, Symbol<'static, Init0>),
    upd: (Symbol<'static, Upd>, Symbol<'static, Upd>),
    fin: (Symbol<'static, Fin>, Symbol<'static, Fin>),
}

fn sha3_families() -> Vec<Sha3> {
    let mut v = Vec::new();
    for (name, rate, outlen) in [
        ("crypto_hash_sha3256", 136usize, 32usize),
        ("crypto_hash_sha3512", 72usize, 64usize),
    ] {
        let needed = [
            name.to_string(),
            format!("{name}_init"),
            format!("{name}_update"),
            format!("{name}_final"),
            format!("{name}_statebytes"),
        ];
        if let Some(m) = needed.iter().find(|s| !has(s)) {
            eprintln!("SKIP {name}: `{m}` is not exported by both libraries");
            continue;
        }
        // Row 3.31 / 3.39: the state really is 256 bytes on both sides.
        let (sc, sr) = both::<SzGetter>(&format!("{name}_statebytes"));
        let (vc, vr) = unsafe { (sc(), sr()) };
        assert_eq!(vc, vr, "{name}_statebytes: C {vc} vs Rust {vr}");
        assert_eq!(vc, SB, "{name}_statebytes must be {SB}");
        v.push(Sha3 {
            name,
            rate,
            outlen,
            one: both(name),
            init: both(&format!("{name}_init")),
            upd: both(&format!("{name}_update")),
            fin: both(&format!("{name}_final")),
        });
    }
    assert!(!v.is_empty(), "no SHA-3 family is exported by both libraries");
    v
}

impl Sha3 {
    /// Row 3.23 / 3.32: one-shot, with guard bytes past `out`.
    fn one_shot(&self, msg: &[u8], label: &str) -> Vec<u8> {
        let n = self.outlen;
        let mut oc = padded(n);
        let mut or = padded(n);
        let rc = unsafe { (self.one.0)(oc.as_mut_ptr(), msg.as_ptr(), msg.len() as u64) };
        let rr = unsafe { (self.one.1)(or.as_mut_ptr(), msg.as_ptr(), msg.len() as u64) };
        eqi(&format!("{label} one-shot ret"), rc, rr);
        // errors_3 row 3.23: the one-shot is infallible.
        assert_eq!(rc, 0, "{label}: C one-shot must return 0");
        eqb(&format!("{label} one-shot out"), &oc[..n], &or[..n]);
        check_pad(&format!("{label} one-shot out(C)"), &oc, n);
        check_pad(&format!("{label} one-shot out(Rust)"), &or, n);
        oc[..n].to_vec()
    }
}

struct Sha3Run<'a> {
    f: &'a Sha3,
    c: Box<St>,
    r: Box<St>,
}

impl<'a> Sha3Run<'a> {
    fn new(f: &'a Sha3, label: &str) -> Sha3Run<'a> {
        let mut me = Sha3Run { f, c: St::new(), r: St::new() };
        me.init(label);
        me
    }
    fn init(&mut self, label: &str) {
        let rc = unsafe { (self.f.init.0)(self.c.p()) };
        let rr = unsafe { (self.f.init.1)(self.r.p()) };
        eqi(&format!("{label} init ret"), rc, rr);
        // errors_3 row 3.17: init is infallible.
        assert_eq!(rc, 0, "{label}: C init must return 0");
        cmp_state(&format!("{label} after init"), &self.c, &self.r);
    }
    fn update(&mut self, d: &[u8], label: &str) -> c_int {
        let rc = unsafe { (self.f.upd.0)(self.c.p(), d.as_ptr(), d.len() as u64) };
        let rr = unsafe { (self.f.upd.1)(self.r.p(), d.as_ptr(), d.len() as u64) };
        eqi(&format!("{label} update({}) ret", d.len()), rc, rr);
        cmp_state(&format!("{label} after update({})", d.len()), &self.c, &self.r);
        rc
    }
    fn finish(&mut self, label: &str) -> (c_int, Vec<u8>) {
        let n = self.f.outlen;
        let mut oc = padded(n);
        let mut or = padded(n);
        let rc = unsafe { (self.f.fin.0)(self.c.p(), oc.as_mut_ptr()) };
        let rr = unsafe { (self.f.fin.1)(self.r.p(), or.as_mut_ptr()) };
        eqi(&format!("{label} final ret"), rc, rr);
        eqb(&format!("{label} final out"), &oc[..n], &or[..n]);
        check_pad(&format!("{label} final out(C)"), &oc, n);
        check_pad(&format!("{label} final out(Rust)"), &or, n);
        cmp_state(&format!("{label} after final"), &self.c, &self.r);
        (rc, oc[..n].to_vec())
    }
    /// `final` with the output buffer pre-filled with `fill`, used to prove
    /// that all `outlen` bytes are actually written (errors_3 row 3.20).
    fn finish_filled(&mut self, fill: u8, label: &str) -> (c_int, Vec<u8>) {
        let n = self.f.outlen;
        let mut oc = padded(n);
        let mut or = padded(n);
        for b in oc[..n].iter_mut() {
            *b = fill;
        }
        for b in or[..n].iter_mut() {
            *b = fill;
        }
        let rc = unsafe { (self.f.fin.0)(self.c.p(), oc.as_mut_ptr()) };
        let rr = unsafe { (self.f.fin.1)(self.r.p(), or.as_mut_ptr()) };
        eqi(&format!("{label} final ret"), rc, rr);
        eqb(&format!("{label} final out"), &oc[..n], &or[..n]);
        check_pad(&format!("{label} final out(C)"), &oc, n);
        check_pad(&format!("{label} final out(Rust)"), &or, n);
        cmp_state(&format!("{label} after final"), &self.c, &self.r);
        (rc, oc[..n].to_vec())
    }
    /// Absorb `chunks` in order and finalize.
    fn stream(f: &'a Sha3, msg: &[u8], chunks: &[usize], label: &str) -> Vec<u8> {
        let mut run = Sha3Run::new(f, label);
        let mut off = 0usize;
        for (i, &n) in chunks.iter().enumerate() {
            let n = std::cmp::min(n, msg.len() - off);
            let rc = run.update(&msg[off..off + n], &format!("{label}[{i}]"));
            assert_eq!(rc, 0, "{label}[{i}]: absorb while ABSORBING must return 0");
            off += n;
        }
        assert_eq!(off, msg.len(), "{label}: chunk sizes must cover the message");
        let (rc, d) = run.finish(label);
        assert_eq!(rc, 0, "{label}: first final must return 0");
        d
    }
}

// ------------------------------------------------------------ SHA-3: accessors

/// configs_3 rows 3.31 / 3.39, errors_3 row 3.24.
#[test]
fn sha3_accessors() {
    for (name, want_bytes) in [("crypto_hash_sha3256", 32usize), ("crypto_hash_sha3512", 64)] {
        for (sym, want) in [
            (format!("{name}_bytes"), want_bytes),
            (format!("{name}_statebytes"), SB),
        ] {
            if !has(&sym) {
                eprintln!("SKIP {sym}: not exported by both libraries");
                continue;
            }
            let (c, r) = both::<SzGetter>(&sym);
            let (vc, vr) = unsafe { (c(), r()) };
            assert_eq!(vc, vr, "{sym}: C {vc} vs Rust {vr}");
            assert_eq!(vc, want, "{sym}: expected {want}, got {vc}");
        }
    }
}

// ------------------------------------------------------- SHA-3: state layout

fn le64(b: &[u8]) -> u64 {
    u64::from_le_bytes(b.try_into().unwrap())
}

/// The full-state comparison used everywhere else is only meaningful if the
/// scalar bookkeeping fields really live at the same offsets in C and Rust.
/// `sha3_state_internal` is `{ keccak1600_state state[224]; size_t offset,
/// rate, outlen; uint8_t phase; }`, so decode and pin those fields on both
/// sides.  This also proves `rate`/`outlen` are carried in the state
/// (configs_3 row 3.40).
#[test]
fn sha3_state_layout_is_byte_identical() {
    for f in &sha3_families() {
        let label = format!("{} layout", f.name);
        let mut run = Sha3Run::new(f, &label);
        for (which, st) in [("C", &run.c), ("Rust", &run.r)] {
            let b = &st.0;
            assert!(b[..200].iter().all(|&x| x == 0), "{label}/{which}: keccak lanes not zeroed");
            assert_eq!(le64(&b[224..232]), 0, "{label}/{which}: offset after init");
            assert_eq!(le64(&b[232..240]), f.rate as u64, "{label}/{which}: rate after init");
            assert_eq!(le64(&b[240..248]), f.outlen as u64, "{label}/{which}: outlen");
            assert_eq!(b[248], 0, "{label}/{which}: phase must be ABSORBING(0)");
        }
        // `offset` tracks the absorb position modulo the rate.
        let mut expect = 0usize;
        for n in [1usize, f.rate - 2, 1, f.rate, 3, 0, f.rate - 3] {
            let d = vec![0x11u8; n];
            assert_eq!(run.update(&d, &format!("{label} +{n}")), 0);
            // Mirror the C bookkeeping: after each update, `offset` is either
            // `total % rate` or exactly `rate` when the update ended on a
            // block boundary and nothing followed it.
            expect = if n == 0 {
                expect
            } else {
                let t = expect + n;
                if t % f.rate == 0 {
                    f.rate
                } else {
                    t % f.rate
                }
            };
            for (which, st) in [("C", &run.c), ("Rust", &run.r)] {
                assert_eq!(
                    le64(&st.0[224..232]),
                    expect as u64,
                    "{label}/{which}: offset after +{n}"
                );
                assert_eq!(st.0[248], 0, "{label}/{which}: still ABSORBING");
            }
        }
        let (rc, _) = run.finish(&label);
        assert_eq!(rc, 0);
        for (which, st) in [("C", &run.c), ("Rust", &run.r)] {
            assert_eq!(le64(&st.0[224..232]), 0, "{label}/{which}: offset reset by final");
            assert_eq!(st.0[248], 1, "{label}/{which}: phase must be FINALIZED(1)");
            assert_eq!(le64(&st.0[232..240]), f.rate as u64, "{label}/{which}: rate preserved");
            assert_eq!(le64(&st.0[240..248]), f.outlen as u64, "{label}/{which}: outlen preserved");
        }
    }
}

/// Same for `shake*_state_internal` = `{ keccak1600_state state[224];
/// size_t offset; uint8_t phase; uint8_t domain; }` — in particular the
/// `domain` byte must be stored verbatim for every value (errors_3 row 3.25).
#[test]
fn xof_state_layout_is_byte_identical() {
    for f in &xof_families() {
        let label = format!("{} layout", f.name);
        // plain init stores DOMAIN_STANDARD
        {
            let mut run = XofRun::new(f, &label);
            for (which, st) in [("C", &run.c), ("Rust", &run.r)] {
                let b = &st.0;
                assert!(b[..200].iter().all(|&x| x == 0), "{label}/{which}: lanes not zeroed");
                assert_eq!(le64(&b[224..232]), 0, "{label}/{which}: offset after init");
                assert_eq!(b[232], 0, "{label}/{which}: phase must be ABSORBING(0)");
                assert_eq!(b[233], 0x1f, "{label}/{which}: domain must be DOMAIN_STANDARD");
            }
            let _ = run.squeeze(1, &format!("{label} warm"));
            for (which, st) in [("C", &run.c), ("Rust", &run.r)] {
                assert_eq!(st.0[232], 1, "{label}/{which}: phase must be SQUEEZING(1)");
                assert_eq!(le64(&st.0[224..232]), 1, "{label}/{which}: offset after squeeze(1)");
                assert_eq!(st.0[233], 0x1f, "{label}/{which}: domain preserved");
            }
        }
        // every domain byte is stored verbatim, no validation, no clamping
        for d in 0u8..=0xff {
            let mut run = XofRun::with_domain(f, d, &format!("{label} d={d:#04x}"));
            for (which, st) in [("C", &run.c), ("Rust", &run.r)] {
                assert_eq!(
                    st.0[233], d,
                    "{label}/{which}: domain {d:#04x} must be stored verbatim"
                );
                assert_eq!(le64(&st.0[224..232]), 0, "{label}/{which}: offset after init");
                assert_eq!(st.0[232], 0, "{label}/{which}: phase after init");
            }
            // absorb/squeeze must not disturb `domain`
            assert_eq!(run.update(&[0x77u8; 5], &format!("{label} d={d:#04x} m")), 0);
            let _ = run.squeeze(f.rate + 1, &format!("{label} d={d:#04x} sq"));
            for (which, st) in [("C", &run.c), ("Rust", &run.r)] {
                assert_eq!(st.0[233], d, "{label}/{which}: domain {d:#04x} preserved");
            }
        }
        // `offset` tracks the absorb position modulo the rate.
        let mut run = XofRun::new(f, &format!("{label} offs"));
        let mut expect = 0usize;
        for n in [1usize, f.rate - 2, 1, f.rate, 3, 0, f.rate - 3] {
            assert_eq!(run.update(&vec![0x22u8; n], &format!("{label} +{n}")), 0);
            expect = if n == 0 {
                expect
            } else {
                let t = expect + n;
                if t % f.rate == 0 {
                    f.rate
                } else {
                    t % f.rate
                }
            };
            for (which, st) in [("C", &run.c), ("Rust", &run.r)] {
                assert_eq!(
                    le64(&st.0[224..232]),
                    expect as u64,
                    "{label}/{which}: offset after +{n}"
                );
            }
        }
        // and the squeeze position modulo the rate afterwards
        let mut sq = 0usize;
        for n in [1usize, f.rate - 1, f.rate, 2 * f.rate + 5, 0] {
            let _ = run.squeeze(n, &format!("{label} sq{n}"));
            sq = if n == 0 {
                sq
            } else {
                let t = sq + n;
                if t % f.rate == 0 {
                    f.rate
                } else {
                    t % f.rate
                }
            };
            for (which, st) in [("C", &run.c), ("Rust", &run.r)] {
                assert_eq!(
                    le64(&st.0[224..232]),
                    sq as u64,
                    "{label}/{which}: squeeze offset after {n}"
                );
                assert_eq!(st.0[232], 1, "{label}/{which}: phase stays SQUEEZING");
            }
        }
    }
}

// ------------------------------------------------------------ SHA-3: one-shot

/// configs_3 rows 3.23 / 3.32 / 3.128: one-shot over 0..300, every rate
/// boundary, multi-KiB inputs, and all four content patterns.
#[test]
fn sha3_one_shot_all_lengths() {
    let fams = sha3_families();
    let lens = full_lengths();
    for f in &fams {
        let mut rng = Rng::new(0x5A3_0001 ^ f.rate as u64);
        let big = rng.bytes(8192);
        for &len in &lens {
            f.one_shot(&big[..len], &format!("{} rnd len={len}", f.name));
        }
        for kind in 0..3 {
            for &len in BOUNDARY.iter() {
                let m = pattern(kind, len);
                f.one_shot(&m, &format!("{} pat{kind} len={len}", f.name));
            }
        }
    }
}

/// errors_3 row 3.19 / area note: `in == NULL` with `inlen == 0` is defined
/// (nothing is dereferenced).
#[test]
fn sha3_one_shot_null_input_zero_len() {
    for f in &sha3_families() {
        let n = f.outlen;
        let mut oc = padded(n);
        let mut or = padded(n);
        let rc = unsafe { (f.one.0)(oc.as_mut_ptr(), std::ptr::null(), 0) };
        let rr = unsafe { (f.one.1)(or.as_mut_ptr(), std::ptr::null(), 0) };
        eqi(&format!("{} one-shot(NULL,0) ret", f.name), rc, rr);
        eqb(&format!("{} one-shot(NULL,0)", f.name), &oc[..n], &or[..n]);
        check_pad(&format!("{} one-shot(NULL,0) C", f.name), &oc, n);
        check_pad(&format!("{} one-shot(NULL,0) Rust", f.name), &or, n);
        // Must equal the empty-message digest.
        let empty = f.one_shot(&[], &format!("{} empty", f.name));
        eqb(&format!("{} NULL,0 == empty", f.name), &oc[..n], &empty);
    }
}

// ----------------------------------------------------------- SHA-3: streaming

/// configs_3 rows 3.24 / 3.33 / 3.30 / 3.38: single `update`, full state
/// compare after init and after the update, and one-shot equivalence.
#[test]
fn sha3_streaming_single_update() {
    let fams = sha3_families();
    let lens = full_lengths();
    for f in &fams {
        let mut rng = Rng::new(0x5A3_0002 ^ f.rate as u64);
        let big = rng.bytes(8192);
        for &len in &lens {
            let msg = &big[..len];
            let label = format!("{} 1xupdate len={len}", f.name);
            let d = Sha3Run::stream(f, msg, &[len], &label);
            let o = f.one_shot(msg, &label);
            eqb(&format!("{label}: streaming == one-shot"), &o, &d);
        }
    }
}

/// configs_3 rows 3.25 / 3.34: `inlen` fed as 1-byte updates, with the full
/// state compared after every single one.
#[test]
fn sha3_one_byte_updates() {
    let fams = sha3_families();
    for f in &fams {
        let mut rng = Rng::new(0x5A3_0003 ^ f.rate as u64);
        let msg = rng.bytes(300);
        let label = format!("{} 1-byte updates", f.name);
        let mut run = Sha3Run::new(f, &label);
        for i in 0..msg.len() {
            let rc = run.update(&msg[i..i + 1], &format!("{label} @{i}"));
            assert_eq!(rc, 0, "{label} @{i}: must return 0");
        }
        let (rc, d) = run.finish(&label);
        assert_eq!(rc, 0);
        let o = f.one_shot(&msg, &label);
        eqb(&format!("{label}: == one-shot"), &o, &d);
    }
}

/// configs_3 rows 3.26 / 3.35 / 3.30 / 3.38: every two-call split of a set of
/// totals chosen to hit all four `sha3_update` arms.
#[test]
fn sha3_two_update_splits() {
    let fams = sha3_families();
    for f in &fams {
        let mut rng = Rng::new(0x5A3_0004 ^ f.rate as u64);
        let big = rng.bytes(512);
        let totals: Vec<usize> = vec![
            0,
            1,
            2,
            f.rate - 1,
            f.rate,
            f.rate + 1,
            2 * f.rate - 1,
            2 * f.rate,
            2 * f.rate + 1,
            3 * f.rate,
            3 * f.rate + 1,
            300,
        ];
        for &total in &totals {
            let msg = &big[..total];
            let o = f.one_shot(msg, &format!("{} split total={total}", f.name));
            for a in splits_for(total, f.rate) {
                let label = format!("{} split total={total} a={a}", f.name);
                let d = Sha3Run::stream(f, msg, &[a, total - a], &label);
                eqb(&format!("{label}: == one-shot"), &o, &d);
            }
        }
    }
}

/// configs_3 row 3.27, errors_3 row 3.19: `update(inlen == 0)` must be a true
/// no-op (state byte-identical before and after) at every position.
#[test]
fn sha3_zero_length_updates_are_noops() {
    let fams = sha3_families();
    for f in &fams {
        let mut rng = Rng::new(0x5A3_0005 ^ f.rate as u64);
        // A prefix length hitting every interesting `offset` value.
        for pre in [0usize, 1, f.rate - 1, f.rate, f.rate + 1, 2 * f.rate, 2 * f.rate + 7] {
            let msg = rng.bytes(pre + 5);
            let label = format!("{} zero-update pre={pre}", f.name);
            let mut run = Sha3Run::new(f, &label);
            // leading zero-length update
            let before = run.c.snapshot();
            assert_eq!(run.update(&[], &format!("{label} lead")), 0);
            assert_eq!(before, run.c.snapshot(), "{label}: leading update(0) mutated C state");
            assert_eq!(run.update(&msg[..pre], &format!("{label} body")), 0);
            let mid = run.c.snapshot();
            for k in 0..3 {
                assert_eq!(run.update(&[], &format!("{label} mid{k}")), 0);
                assert_eq!(mid, run.c.snapshot(), "{label}: mid update(0) mutated C state");
            }
            assert_eq!(run.update(&msg[pre..], &format!("{label} tail")), 0);
            let post = run.c.snapshot();
            assert_eq!(run.update(&[], &format!("{label} trail")), 0);
            assert_eq!(post, run.c.snapshot(), "{label}: trailing update(0) mutated C state");
            let (rc, d) = run.finish(&label);
            assert_eq!(rc, 0);
            let o = f.one_shot(&msg, &label);
            eqb(&format!("{label}: == one-shot"), &o, &d);
        }
    }
}

/// configs_3 rows 3.26 / 3.35: randomized multi-chunk absorb splits, including
/// runs of 1-byte updates and interleaved zero-length updates.
#[test]
fn sha3_random_multichunk_absorb() {
    let fams = sha3_families();
    for f in &fams {
        let mut rng = Rng::new(0x5A3_0006 ^ f.rate as u64);
        for it in 0..240 {
            let total = rng.range(0, 900);
            let msg = rng.bytes(total);
            let o = f.one_shot(&msg, &format!("{} rndchunk#{it}", f.name));
            let chunks = random_chunks(&mut rng, total, f.rate, it);
            let label = format!("{} rndchunk#{it} total={total} nchunks={}", f.name, chunks.len());
            let d = Sha3Run::stream(f, &msg, &chunks, &label);
            eqb(&format!("{label}: == one-shot"), &o, &d);
        }
    }
}

/// configs_3 rows 3.28 / 3.29 / 3.36 / 3.37, errors_3 rows 3.21 / 3.22:
/// the `offset == rate - 1` fused pad byte (`0x06 ^ 0x80`) and the
/// `offset == rate` extra `permute_24` inside `sha3_final`.
#[test]
fn sha3_final_pad_boundaries() {
    let fams = sha3_families();
    for f in &fams {
        let mut rng = Rng::new(0x5A3_0007 ^ f.rate as u64);
        let big = rng.bytes(4 * f.rate + 4);
        for k in 1..=4usize {
            // total == k*rate - 1  -> offset == rate-1 -> fused pad 0x86
            for total in [k * f.rate - 1, k * f.rate, k * f.rate + 1] {
                let msg = &big[..total];
                let o = f.one_shot(msg, &format!("{} pad total={total}", f.name));
                // reach that offset through several different chunkings, so
                // both the "one big update" and "ends exactly at rate" paths
                // are used.
                let chunkings: Vec<Vec<usize>> = vec![
                    vec![total],
                    vec![0, total],
                    (0..total).map(|_| 1).collect(),
                    vec![f.rate.min(total), total - f.rate.min(total)],
                    vec![total.saturating_sub(1), total.min(1)],
                ];
                for (i, ch) in chunkings.iter().enumerate() {
                    let label = format!("{} pad total={total} ch{i}", f.name);
                    let d = Sha3Run::stream(f, msg, ch, &label);
                    eqb(&format!("{label}: == one-shot"), &o, &d);
                }
            }
        }
    }
}

// -------------------------------------------------------- SHA-3: error quirks

/// errors_3 row 3.18: `update` after `final` returns exactly `-1`, **but** the
/// state is still mutated and the new data is still absorbed.
#[test]
fn sha3_update_after_final_returns_minus1_but_absorbs() {
    let fams = sha3_families();
    for f in &fams {
        let mut rng = Rng::new(0x5A3_0008 ^ f.rate as u64);
        for pre in [0usize, 1, f.rate - 1, f.rate, f.rate + 1, 2 * f.rate + 3] {
            let m1 = rng.bytes(pre);
            let label = format!("{} upd-after-final pre={pre}", f.name);
            let mut run = Sha3Run::new(f, &label);
            assert_eq!(run.update(&m1, &format!("{label} m1")), 0);
            let (rc, d1) = run.finish(&label);
            assert_eq!(rc, 0, "{label}: first final must return 0");
            let after_final = run.c.snapshot();

            // Non-empty second absorb: exact sentinel is -1.
            let m2 = vec![0xA7u8; f.rate + 5];
            let rc2 = run.update(&m2, &format!("{label} m2"));
            assert_eq!(rc2, -1, "{label}: update after final must return exactly -1");
            let after_upd = run.c.snapshot();
            assert_ne!(
                after_final, after_upd,
                "{label}: update after final must still mutate the state"
            );

            // Prove the *data* was absorbed, not just the permute applied: an
            // empty update from the same finalized state gives a different
            // state (permute only).
            let mut run0 = Sha3Run::new(f, &format!("{label} ref"));
            assert_eq!(run0.update(&m1, &format!("{label} ref m1")), 0);
            let (rc0, d0) = run0.finish(&format!("{label} ref"));
            assert_eq!(rc0, 0);
            eqb(&format!("{label}: reproducible first digest"), &d1, &d0);
            assert_eq!(
                run0.update(&[], &format!("{label} ref empty")),
                -1,
                "{label}: empty update after final must also return -1"
            );
            let after_empty = run0.c.snapshot();
            assert_ne!(after_final, after_empty, "{label}: empty update still permutes");
            assert_ne!(
                after_empty, after_upd,
                "{label}: the absorbed data must change the state"
            );

            // A following `final` succeeds again (phase was reset to ABSORBING).
            let (rc3, d3) = run.finish(&format!("{label} 2nd"));
            assert_eq!(rc3, 0, "{label}: final after a resumed absorb must return 0");
            assert_ne!(d1, d3, "{label}: resumed digest must differ from the first");
        }
    }
}

/// errors_3 row 3.20: a second `final` returns exactly `-1` but still writes
/// `outlen` bytes (verified with two different pre-fill patterns) and still
/// re-sets `phase`/`offset`.  Squeeze-after-squeeze chains are also compared.
#[test]
fn sha3_double_final_returns_minus1_but_writes() {
    let fams = sha3_families();
    for f in &fams {
        let mut rng = Rng::new(0x5A3_0009 ^ f.rate as u64);
        for pre in [0usize, 1, f.rate - 1, f.rate, 2 * f.rate] {
            let msg = rng.bytes(pre);
            let n = f.outlen;
            let mut outs: Vec<Vec<u8>> = Vec::new();
            for fill in [0x00u8, 0x5A, 0xFF] {
                let label = format!("{} 2xfinal pre={pre} fill={fill:02x}", f.name);
                let mut run = Sha3Run::new(f, &label);
                assert_eq!(run.update(&msg, &format!("{label} m")), 0);
                let (rc1, _d1) = run.finish(&label);
                assert_eq!(rc1, 0);
                let (rc2, d2) = run.finish_filled(fill, &format!("{label} 2nd"));
                assert_eq!(rc2, -1, "{label}: second final must return exactly -1");
                assert_eq!(d2.len(), n);
                // A third final keeps returning -1 and keeps advancing the stream.
                let (rc3, d3) = run.finish_filled(fill, &format!("{label} 3rd"));
                assert_eq!(rc3, -1, "{label}: third final must return exactly -1");
                assert_ne!(d2, d3, "{label}: repeated final must advance the state");
                outs.push(d2);
                outs.push(d3);
            }
            // The pre-fill pattern must not survive anywhere: all three runs
            // produced identical bytes, so all `outlen` bytes were written.
            for i in (2..outs.len()).step_by(2) {
                eqb(
                    &format!("{} 2xfinal pre={pre}: outlen bytes fully written", f.name),
                    &outs[0],
                    &outs[i],
                );
                eqb(
                    &format!("{} 3xfinal pre={pre}: outlen bytes fully written", f.name),
                    &outs[1],
                    &outs[i + 1],
                );
            }
        }
    }
}

/// configs_3 row 3.129: `init` after `final` must fully reset the state.
#[test]
fn sha3_state_reuse_after_final() {
    let fams = sha3_families();
    for f in &fams {
        let mut rng = Rng::new(0x5A3_000A ^ f.rate as u64);
        let m1 = rng.bytes(f.rate + 9);
        let m2 = rng.bytes(2 * f.rate - 1);
        let label = format!("{} reuse", f.name);
        let mut run = Sha3Run::new(f, &label);
        assert_eq!(run.update(&m1, &format!("{label} m1")), 0);
        let (rc, d1) = run.finish(&label);
        assert_eq!(rc, 0);
        run.init(&format!("{label} re-init"));
        assert_eq!(run.update(&m2, &format!("{label} m2")), 0);
        let (rc, d2) = run.finish(&format!("{label} 2nd"));
        assert_eq!(rc, 0);
        eqb(&format!("{label}: m1"), &f.one_shot(&m1, &label), &d1);
        eqb(&format!("{label}: m2 after re-init"), &f.one_shot(&m2, &label), &d2);
    }
}

/// configs_3 row 3.40: `rate`/`outlen` live in the state, not in `sha3_final`.
#[test]
fn sha3_digest_size_matrix() {
    let fams = sha3_families();
    if fams.len() < 2 {
        eprintln!("SKIP sha3_digest_size_matrix: both SHA-3 variants are needed");
        return;
    }
    let mut rng = Rng::new(0x5A3_000B);
    for len in [0usize, 1, 71, 72, 73, 135, 136, 137, 271, 272, 1024] {
        let msg = rng.bytes(len);
        let a = fams[0].one_shot(&msg, "matrix 256");
        let b = fams[1].one_shot(&msg, "matrix 512");
        assert_eq!(a.len(), 32);
        assert_eq!(b.len(), 64);
        assert_ne!(&a[..], &b[..32], "SHA3-256 must not be a prefix of SHA3-512 (len={len})");
    }
}

/// configs_3 row 3.130: `out` aliasing `in` for the one-shot.
#[test]
fn sha3_one_shot_aliased_out_in() {
    let fams = sha3_families();
    for f in &fams {
        let mut rng = Rng::new(0x5A3_000C ^ f.rate as u64);
        for len in [0usize, 1, 31, 32, 33, 64, 71, 72, 73, 135, 136, 137, 200, 300] {
            let base = rng.bytes(len.max(f.outlen));
            let mut bc = padded(base.len());
            let mut br = padded(base.len());
            bc[..base.len()].copy_from_slice(&base);
            br[..base.len()].copy_from_slice(&base);
            let rc = unsafe { (f.one.0)(bc.as_mut_ptr(), bc.as_ptr(), len as u64) };
            let rr = unsafe { (f.one.1)(br.as_mut_ptr(), br.as_ptr(), len as u64) };
            eqi(&format!("{} alias len={len} ret", f.name), rc, rr);
            eqb(&format!("{} alias len={len}", f.name), &bc, &br);
            check_pad(&format!("{} alias C len={len}", f.name), &bc, base.len());
            eqb(
                &format!("{} alias len={len} == non-aliased", f.name),
                &f.one_shot(&base[..len], &format!("{} alias ref", f.name)),
                &bc[..f.outlen],
            );
        }
    }
}

// ================================================================= XOF family

struct Xof {
    name: &'static str,
    rate: usize,
    refname: &'static str,
    one: (Symbol<'static, OneShotXof>, Symbol<'static, OneShotXof>),
    init: (Symbol<'static, Init0>, Symbol<'static, Init0>),
    initd: (Symbol<'static, InitD>, Symbol<'static, InitD>),
    upd: (Symbol<'static, Upd>, Symbol<'static, Upd>),
    sq: (Symbol<'static, Sq>, Symbol<'static, Sq>),
}

fn xof_families() -> Vec<Xof> {
    let mut v = Vec::new();
    for (name, rate, refname) in [
        ("crypto_xof_shake128", 168usize, "_sodium_shake128_ref"),
        ("crypto_xof_shake256", 136usize, "_sodium_shake256_ref"),
        ("crypto_xof_turboshake128", 168usize, "_sodium_turboshake128_ref"),
        ("crypto_xof_turboshake256", 136usize, "_sodium_turboshake256_ref"),
    ] {
        let needed = [
            name.to_string(),
            format!("{name}_init"),
            format!("{name}_init_with_domain"),
            format!("{name}_update"),
            format!("{name}_squeeze"),
            format!("{name}_statebytes"),
            format!("{name}_blockbytes"),
            format!("{name}_domain_standard"),
        ];
        if let Some(m) = needed.iter().find(|s| !has(s)) {
            eprintln!("SKIP {name}: `{m}` is not exported by both libraries");
            continue;
        }
        let (sc, sr) = both::<SzGetter>(&format!("{name}_statebytes"));
        let (vc, vr) = unsafe { (sc(), sr()) };
        assert_eq!(vc, vr, "{name}_statebytes: C {vc} vs Rust {vr}");
        assert_eq!(vc, SB, "{name}_statebytes must be {SB}");
        v.push(Xof {
            name,
            rate,
            refname,
            one: both(name),
            init: both(&format!("{name}_init")),
            initd: both(&format!("{name}_init_with_domain")),
            upd: both(&format!("{name}_update")),
            sq: both(&format!("{name}_squeeze")),
        });
    }
    assert!(!v.is_empty(), "no XOF family is exported by both libraries");
    v
}

impl Xof {
    /// configs_3 rows 3.41 / 3.53 / 3.63 / 3.74, errors_3 row 3.32.
    fn one_shot(&self, msg: &[u8], outlen: usize, label: &str) -> Vec<u8> {
        let mut oc = padded(outlen);
        let mut or = padded(outlen);
        let rc =
            unsafe { (self.one.0)(oc.as_mut_ptr(), outlen, msg.as_ptr(), msg.len() as u64) };
        let rr =
            unsafe { (self.one.1)(or.as_mut_ptr(), outlen, msg.as_ptr(), msg.len() as u64) };
        eqi(&format!("{label} one-shot ret"), rc, rr);
        assert_eq!(rc, 0, "{label}: C one-shot must return 0");
        eqb(&format!("{label} one-shot out"), &oc[..outlen], &or[..outlen]);
        check_pad(&format!("{label} one-shot out(C)"), &oc, outlen);
        check_pad(&format!("{label} one-shot out(Rust)"), &or, outlen);
        oc[..outlen].to_vec()
    }
}

struct XofRun<'a> {
    f: &'a Xof,
    c: Box<St>,
    r: Box<St>,
}

impl<'a> XofRun<'a> {
    fn new(f: &'a Xof, label: &str) -> XofRun<'a> {
        let mut me = XofRun { f, c: St::new(), r: St::new() };
        me.init(label);
        me
    }
    fn with_domain(f: &'a Xof, domain: u8, label: &str) -> XofRun<'a> {
        let mut me = XofRun { f, c: St::new(), r: St::new() };
        me.init_domain(domain, label);
        me
    }
    fn init(&mut self, label: &str) {
        let rc = unsafe { (self.f.init.0)(self.c.p()) };
        let rr = unsafe { (self.f.init.1)(self.r.p()) };
        eqi(&format!("{label} init ret"), rc, rr);
        assert_eq!(rc, 0, "{label}: C init must return 0");
        cmp_state(&format!("{label} after init"), &self.c, &self.r);
    }
    fn init_domain(&mut self, domain: u8, label: &str) {
        let rc = unsafe { (self.f.initd.0)(self.c.p(), domain) };
        let rr = unsafe { (self.f.initd.1)(self.r.p(), domain) };
        eqi(&format!("{label} init_with_domain({domain:#04x}) ret"), rc, rr);
        // errors_3 rows 3.25 / 3.26: every byte value is accepted, always 0.
        assert_eq!(rc, 0, "{label}: init_with_domain({domain:#04x}) must return 0");
        cmp_state(&format!("{label} after init_with_domain({domain:#04x})"), &self.c, &self.r);
    }
    fn update(&mut self, d: &[u8], label: &str) -> c_int {
        let rc = unsafe { (self.f.upd.0)(self.c.p(), d.as_ptr(), d.len() as u64) };
        let rr = unsafe { (self.f.upd.1)(self.r.p(), d.as_ptr(), d.len() as u64) };
        eqi(&format!("{label} update({}) ret", d.len()), rc, rr);
        cmp_state(&format!("{label} after update({})", d.len()), &self.c, &self.r);
        rc
    }
    fn squeeze(&mut self, n: usize, label: &str) -> Vec<u8> {
        let mut oc = padded(n);
        let mut or = padded(n);
        let rc = unsafe { (self.f.sq.0)(self.c.p(), oc.as_mut_ptr(), n) };
        let rr = unsafe { (self.f.sq.1)(self.r.p(), or.as_mut_ptr(), n) };
        eqi(&format!("{label} squeeze({n}) ret"), rc, rr);
        // errors_3 rows 3.29 / 3.30: squeeze has no error path at all.
        assert_eq!(rc, 0, "{label}: squeeze({n}) must always return 0");
        eqb(&format!("{label} squeeze({n}) out"), &oc[..n], &or[..n]);
        check_pad(&format!("{label} squeeze({n}) out(C)"), &oc, n);
        check_pad(&format!("{label} squeeze({n}) out(Rust)"), &or, n);
        cmp_state(&format!("{label} after squeeze({n})"), &self.c, &self.r);
        oc[..n].to_vec()
    }
    /// Absorb `chunks`, then squeeze `sq_chunks`, returning the concatenation.
    fn stream(
        f: &'a Xof,
        domain: Option<u8>,
        msg: &[u8],
        chunks: &[usize],
        sq_chunks: &[usize],
        label: &str,
    ) -> Vec<u8> {
        let mut run = match domain {
            Some(d) => XofRun::with_domain(f, d, label),
            None => XofRun::new(f, label),
        };
        let mut off = 0usize;
        for (i, &n) in chunks.iter().enumerate() {
            let n = std::cmp::min(n, msg.len() - off);
            let rc = run.update(&msg[off..off + n], &format!("{label} abs[{i}]"));
            assert_eq!(rc, 0, "{label} abs[{i}]: absorb while ABSORBING must return 0");
            off += n;
        }
        assert_eq!(off, msg.len(), "{label}: absorb chunks must cover the message");
        let mut out = Vec::new();
        for (i, &n) in sq_chunks.iter().enumerate() {
            out.extend_from_slice(&run.squeeze(n, &format!("{label} sq[{i}]")));
        }
        out
    }
}

// -------------------------------------------------------------- XOF accessors

/// configs_3 rows 3.51 / 3.60 / 3.72 / 3.80, errors_3 row 3.33.
#[test]
fn xof_accessors() {
    for (name, blk) in [
        ("crypto_xof_shake128", 168usize),
        ("crypto_xof_shake256", 136),
        ("crypto_xof_turboshake128", 168),
        ("crypto_xof_turboshake256", 136),
    ] {
        for (sym, want) in [
            (format!("{name}_blockbytes"), blk),
            (format!("{name}_statebytes"), SB),
        ] {
            if !has(&sym) {
                eprintln!("SKIP {sym}: not exported by both libraries");
                continue;
            }
            let (c, r) = both::<SzGetter>(&sym);
            let (vc, vr) = unsafe { (c(), r()) };
            assert_eq!(vc, vr, "{sym}: C {vc} vs Rust {vr}");
            assert_eq!(vc, want, "{sym}: expected {want}, got {vc}");
        }
        let sym = format!("{name}_domain_standard");
        if !has(&sym) {
            eprintln!("SKIP {sym}: not exported by both libraries");
            continue;
        }
        let (c, r) = both::<U8Getter>(&sym);
        let (vc, vr) = unsafe { (c(), r()) };
        assert_eq!(vc, vr, "{sym}: C {vc:#04x} vs Rust {vr:#04x}");
        assert_eq!(vc, 0x1f, "{sym}: expected 0x1f, got {vc:#04x}");
    }
}

// ---------------------------------------------------------------- XOF one-shot

/// configs_3 rows 3.41 / 3.53 / 3.63 / 3.74 / 3.128: `(inlen, outlen)` grid
/// over 0..300, every rate boundary, and multi-KiB inputs.
#[test]
fn xof_one_shot_grid() {
    let fams = xof_families();
    for f in &fams {
        let mut rng = Rng::new(0x1F0_0001 ^ f.rate as u64 ^ f.name.len() as u64);
        let big = rng.bytes(8192);
        let outlens = [0usize, 1, 32, f.rate - 1, f.rate, f.rate + 1, 2 * f.rate + 1, 512];
        // Fast axis: every inlen 0..=300 with one moderate outlen.
        for len in 0..=300usize {
            f.one_shot(&big[..len], 64, &format!("{} inlen={len}", f.name));
        }
        // Full grid on the boundary lengths + the multi-KiB inputs.
        let mut lens: Vec<usize> = BOUNDARY.to_vec();
        lens.extend([1000, 1024, 2048, 4096, 5000, 8192]);
        for &len in &lens {
            for &ol in &outlens {
                f.one_shot(&big[..len], ol, &format!("{} inlen={len} outlen={ol}", f.name));
            }
        }
        // content patterns
        for kind in 0..3 {
            for &len in BOUNDARY.iter() {
                let m = pattern(kind, len);
                f.one_shot(&m, 137, &format!("{} pat{kind} len={len}", f.name));
            }
        }
    }
}

/// configs_3 rows 3.42 / 3.54 / 3.64 / 3.75: streaming with one `update` and
/// one `squeeze` must equal the one-shot over the whole grid.
#[test]
fn xof_streaming_single_update_single_squeeze() {
    let fams = xof_families();
    for f in &fams {
        let mut rng = Rng::new(0x1F0_0002 ^ f.rate as u64 ^ f.name.len() as u64);
        let big = rng.bytes(8192);
        let outlens = [0usize, 1, 32, f.rate - 1, f.rate, f.rate + 1, 2 * f.rate + 1, 512];
        let mut lens: Vec<usize> = (0..=300).collect();
        lens.extend([335, 336, 337, 407, 408, 409, 1024, 2048, 4096, 8192]);
        for &len in &lens {
            let msg = &big[..len];
            for &ol in &outlens {
                let label = format!("{} 1x1 inlen={len} outlen={ol}", f.name);
                let d = XofRun::stream(f, None, msg, &[len], &[ol], &label);
                let o = f.one_shot(msg, ol, &label);
                eqb(&format!("{label}: streaming == one-shot"), &o, &d);
            }
        }
    }
}

/// configs_3 rows 3.43 / 3.55 / 3.65 / 3.76: multi-call absorb, incl. long
/// runs of 1-byte updates.
#[test]
fn xof_multi_absorb() {
    let fams = xof_families();
    for f in &fams {
        let mut rng = Rng::new(0x1F0_0003 ^ f.rate as u64 ^ f.name.len() as u64);
        let rate = f.rate;
        let sets: Vec<Vec<usize>> = vec![
            vec![1; 300],
            vec![rate - 1, 1],
            vec![rate, 1],
            vec![1, rate - 1],
            vec![1, rate],
            vec![100, rate - 100, rate],
            vec![rate + 1, rate - 1, 1, 0, 7],
            vec![0, rate, 0, rate, 0],
            vec![2 * rate, 1],
            vec![2 * rate - 1, 2],
        ];
        for (i, chunks) in sets.iter().enumerate() {
            let total: usize = chunks.iter().sum();
            let msg = rng.bytes(total);
            for &ol in &[0usize, 1, 64, rate, 2 * rate + 3, 600] {
                let label = format!("{} absorb-set{i} total={total} outlen={ol}", f.name);
                let d = XofRun::stream(f, None, &msg, chunks, &[ol], &label);
                let o = f.one_shot(&msg, ol, &label);
                eqb(&format!("{label}: == one-shot"), &o, &d);
            }
        }
    }
}

/// configs_3 rows 3.44 / 3.56 / 3.66 / 3.77: chunked squeeze must equal a
/// single squeeze of the same total.
#[test]
fn xof_chunked_squeeze() {
    let fams = xof_families();
    for f in &fams {
        let mut rng = Rng::new(0x1F0_0004 ^ f.rate as u64 ^ f.name.len() as u64);
        let rate = f.rate;
        let msg = rng.bytes(3 * rate + 11);
        for &inlen in &[0usize, 1, rate - 1, rate, rate + 1, 3 * rate + 11] {
            let m = &msg[..inlen];
            let sets: Vec<Vec<usize>> = vec![
                vec![1; 512],
                vec![1, rate - 1, 512 - rate],
                vec![rate - 1, 1, 512 - rate],
                vec![rate, rate, 512 - 2 * rate],
                vec![rate + 1, 512 - rate - 1],
                vec![0, 1, 0, rate, 0, 512 - rate - 1, 0],
                vec![511, 1],
                vec![512],
            ];
            let base = f.one_shot(m, 512, &format!("{} sq inlen={inlen}", f.name));
            for (i, sq) in sets.iter().enumerate() {
                let total: usize = sq.iter().sum();
                assert_eq!(total, 512, "squeeze set {i} must total 512");
                let label = format!("{} sq-set{i} inlen={inlen}", f.name);
                let d = XofRun::stream(f, None, m, &[inlen], sq, &label);
                eqb(&format!("{label}: chunked squeeze == single 512-B squeeze"), &base, &d);
            }
        }
    }
}

/// configs_3 row 3.45: `squeeze(0)` before / between real squeezes is a true
/// no-op (state byte-identical, no permute).
#[test]
fn xof_zero_length_squeeze_is_noop() {
    let fams = xof_families();
    for f in &fams {
        let mut rng = Rng::new(0x1F0_0005 ^ f.rate as u64 ^ f.name.len() as u64);
        for inlen in [0usize, 1, f.rate - 1, f.rate, f.rate + 1] {
            let msg = rng.bytes(inlen);
            let label = format!("{} sq0 inlen={inlen}", f.name);
            let mut run = XofRun::new(f, &label);
            assert_eq!(run.update(&msg, &format!("{label} m")), 0);
            // NOTE: the *first* squeeze(0) still finalizes (padding + permute),
            // because `shake*_ref_squeeze` calls the finalizer before looking
            // at `outlen`.  What must not happen is a further permute.
            let before_first = run.c.snapshot();
            let e = run.squeeze(0, &format!("{label} first0"));
            assert!(e.is_empty());
            let after_first = run.c.snapshot();
            assert_ne!(
                before_first, after_first,
                "{label}: the first squeeze(0) must still apply the padding"
            );
            for k in 0..3 {
                assert!(run.squeeze(0, &format!("{label} again0-{k}")).is_empty());
                assert_eq!(
                    after_first,
                    run.c.snapshot(),
                    "{label}: a repeated squeeze(0) must not touch the state"
                );
            }
            // Real squeeze split by zero-length calls == one call.
            let mut out = Vec::new();
            out.extend_from_slice(&run.squeeze(0, &format!("{label} z")));
            out.extend_from_slice(&run.squeeze(f.rate + 5, &format!("{label} a")));
            out.extend_from_slice(&run.squeeze(0, &format!("{label} z2")));
            out.extend_from_slice(&run.squeeze(300, &format!("{label} b")));
            let base = f.one_shot(&msg, f.rate + 305, &label);
            eqb(&format!("{label}: zero-interleaved squeeze == one-shot"), &base, &out);
        }
    }
}

/// configs_3 rows 3.46 / 3.47 / 3.57 / 3.58 / 3.67 / 3.68 / 3.78: the
/// `offset == RATE - 1` fused pad and the `offset == RATE` extra permute in
/// `*_finalize`.
#[test]
fn xof_finalize_pad_boundaries() {
    let fams = xof_families();
    for f in &fams {
        let mut rng = Rng::new(0x1F0_0006 ^ f.rate as u64 ^ f.name.len() as u64);
        let big = rng.bytes(4 * f.rate + 4);
        for k in 1..=4usize {
            for total in [k * f.rate - 1, k * f.rate, k * f.rate + 1] {
                let msg = &big[..total];
                for &ol in &[1usize, 32, f.rate, 2 * f.rate + 1] {
                    let base = f.one_shot(msg, ol, &format!("{} fpad total={total}", f.name));
                    let chunkings: Vec<Vec<usize>> = vec![
                        vec![total],
                        vec![0, total, 0],
                        (0..total).map(|_| 1).collect(),
                        vec![f.rate.min(total), total - f.rate.min(total)],
                        vec![total.saturating_sub(1), total.min(1)],
                    ];
                    for (i, ch) in chunkings.iter().enumerate() {
                        let label = format!("{} fpad total={total} ol={ol} ch{i}", f.name);
                        let d = XofRun::stream(f, None, msg, ch, &[ol], &label);
                        eqb(&format!("{label}: == one-shot"), &base, &d);
                    }
                }
            }
        }
    }
}

// ------------------------------------------------------------ XOF domain bytes

/// configs_3 rows 3.48 / 3.69: `init_with_domain(DOMAIN_STANDARD)` must be
/// indistinguishable from plain `init`, down to the state bytes.
#[test]
fn xof_init_with_standard_domain_equals_init() {
    let fams = xof_families();
    for f in &fams {
        let mut rng = Rng::new(0x1F0_0007 ^ f.rate as u64 ^ f.name.len() as u64);
        let (dc, dr) = both::<U8Getter>(&format!("{}_domain_standard", f.name));
        let (vc, vr) = unsafe { (dc(), dr()) };
        assert_eq!(vc, vr);
        for inlen in [0usize, 1, f.rate - 1, f.rate, f.rate + 1, 2 * f.rate + 5] {
            let msg = rng.bytes(inlen);
            for &ol in &[1usize, 64, 2 * f.rate + 3] {
                let label = format!("{} std-domain inlen={inlen} ol={ol}", f.name);
                let plain = XofRun::stream(f, None, &msg, &[inlen], &[ol], &label);
                let with = XofRun::stream(f, Some(vc), &msg, &[inlen], &[ol], &label);
                eqb(&format!("{label}: init == init_with_domain(0x1f)"), &plain, &with);
            }
        }
        // Also compare the raw state right after each init.
        let mut a = XofRun::new(f, &format!("{} init", f.name));
        let mut b = XofRun::with_domain(f, vc, &format!("{} initd", f.name));
        assert_eq!(
            a.c.snapshot(),
            b.c.snapshot(),
            "{}: init and init_with_domain(0x1f) must leave identical state",
            f.name
        );
        let _ = a.squeeze(8, "warm");
        let _ = b.squeeze(8, "warm");
    }
}

/// configs_3 rows 3.49 / 3.50 / 3.59 / 3.70 / 3.71 / 3.79, errors_3 rows
/// 3.25 / 3.26: **every** domain byte 0x00..=0xFF, in the normal pad path, in
/// the `offset == RATE - 1` fused path, and with `offset == RATE` at the first
/// squeeze.  The C validates nothing, so 0x00 and 0x80 must be accepted
/// exactly like any other value.
#[test]
fn xof_domain_full_sweep() {
    let fams = xof_families();
    for f in &fams {
        let mut rng = Rng::new(0x1F0_0008 ^ f.rate as u64 ^ f.name.len() as u64);
        let big = rng.bytes(3 * f.rate + 1);
        // absorb lengths: offset==0 (empty), a mid offset, offset==RATE-1
        // (fused pad), offset==RATE (extra permute), and 2*RATE-1 / 2*RATE.
        let cases: [(usize, &str); 6] = [
            (0, "offset=0"),
            (37, "offset=37"),
            (f.rate - 1, "offset=RATE-1 (fused pad)"),
            (f.rate, "offset=RATE (extra permute)"),
            (2 * f.rate - 1, "offset=RATE-1 2nd block"),
            (2 * f.rate, "offset=RATE 2nd block"),
        ];
        for (inlen, what) in cases {
            let msg = &big[..inlen];
            let mut seen: HashSet<Vec<u8>> = HashSet::new();
            for d in 0u8..=0xff {
                let label = format!("{} domain={d:#04x} {what}", f.name);
                // one call, and a 1-byte-chunk absorb + 2-chunk squeeze,
                // so the domain byte is exercised on both paths.
                let a = XofRun::stream(f, Some(d), msg, &[inlen], &[64], &label);
                let ones: Vec<usize> = (0..inlen).map(|_| 1).collect();
                let b = XofRun::stream(f, Some(d), msg, &ones, &[1, 63], &label);
                eqb(&format!("{label}: chunked == single"), &a, &b);
                assert!(
                    seen.insert(a.clone()),
                    "{label}: domain byte must reach the pad (duplicate output)"
                );
            }
            assert_eq!(
                seen.len(),
                256,
                "{}: all 256 domain bytes must give distinct streams ({what})",
                f.name
            );
        }
    }
}

/// configs_3 row 3.50: `domain = 0x06` (the SHA-3 domain byte) under a SHAKE
/// rate is reachable with no validation whatsoever.  Because nothing but the
/// rate, the domain byte and the round count distinguish the two code paths,
/// SHAKE256 (rate 136, 24 rounds) with `domain = 0x06` is *exactly* SHA3-256,
/// while TurboSHAKE256 (rate 136, **12** rounds) and SHAKE128 (rate 168) are
/// not.  Both directions are pinned here.
#[test]
fn xof_sha3_domain_byte_is_reachable() {
    let xs = xof_families();
    let hs = sha3_families();
    let mut rng = Rng::new(0x1F0_0009);
    for f in &xs {
        // SHAKE* use permute_24 (like SHA-3); TurboSHAKE* use permute_12.
        let round24 = !f.name.contains("turbo");
        for inlen in [0usize, 1, f.rate - 1, f.rate, f.rate + 1, 2 * f.rate, 300] {
            let msg = rng.bytes(inlen);
            let label = format!("{} domain=0x06 inlen={inlen}", f.name);
            let out = XofRun::stream(f, Some(0x06), &msg, &[inlen], &[64], &label);
            assert_eq!(out.len(), 64);
            for h in &hs {
                let d = h.one_shot(&msg, &label);
                if round24 && f.rate == h.rate {
                    eqb(
                        &format!("{label}: must equal {} (same rate, same permute)", h.name),
                        &d,
                        &out[..d.len()],
                    );
                } else {
                    assert_ne!(
                        &out[..d.len()],
                        &d[..],
                        "{label}: rate {} XOF must not equal {} (rate {})",
                        f.rate,
                        h.name,
                        h.rate
                    );
                }
            }
        }
    }
}

// ------------------------------------------------------------- XOF long stream

/// configs_3 row 3.83: `squeeze(4096)` in one call vs many randomized chunk
/// sizes (crossing ~24-30 blocks).
#[test]
fn xof_long_stream_continuity() {
    let fams = xof_families();
    const N: usize = 4096;
    for f in &fams {
        let mut rng = Rng::new(0x1F0_000A ^ f.rate as u64 ^ f.name.len() as u64);
        for inlen in [0usize, 1, f.rate, 2 * f.rate + 3] {
            let msg = rng.bytes(inlen);
            let base = f.one_shot(&msg, N, &format!("{} long inlen={inlen}", f.name));
            // fixed chunkings
            let mut sets: Vec<Vec<usize>> = vec![
                vec![N],
                vec![1; N],
                vec![f.rate; N / f.rate]
                    .into_iter()
                    .chain(std::iter::once(N % f.rate))
                    .collect(),
                vec![f.rate - 1, N - (f.rate - 1)],
                vec![f.rate + 1, N - (f.rate + 1)],
                vec![N - 1, 1],
            ];
            // randomized chunkings
            for style in 1..4usize {
                sets.push(random_chunks(&mut rng, N, f.rate, style));
            }
            for (i, sq) in sets.iter().enumerate() {
                let label = format!("{} long inlen={inlen} set{i}", f.name);
                let d = XofRun::stream(f, None, &msg, &[inlen], sq, &label);
                assert_eq!(d.len(), N, "{label}: squeeze chunks must total {N}");
                eqb(&format!("{label}: == single {N}-B squeeze"), &base, &d);
            }
        }
    }
}

/// Randomized absorb x squeeze split matrix (configs_3 rows 3.52 / 3.61 /
/// 3.73 / 3.81).
#[test]
fn xof_random_absorb_squeeze_matrix() {
    let fams = xof_families();
    for f in &fams {
        let mut rng = Rng::new(0x1F0_000B ^ f.rate as u64 ^ f.name.len() as u64);
        for it in 0..160usize {
            let inlen = rng.range(0, 700);
            let outlen = rng.range(0, 900);
            let msg = rng.bytes(inlen);
            let base = f.one_shot(&msg, outlen, &format!("{} rnd#{it}", f.name));
            let abs = random_chunks(&mut rng, inlen, f.rate, it);
            let sq = random_chunks(&mut rng, outlen, f.rate, it / 4);
            let label = format!("{} rnd#{it} in={inlen} out={outlen}", f.name);
            let d = XofRun::stream(f, None, &msg, &abs, &sq, &label);
            eqb(&format!("{label}: == one-shot"), &base, &d);
        }
    }
}

// -------------------------------------------------------------- XOF error rows

/// errors_3 rows 3.27 / 3.28: `update` after `squeeze` returns exactly `-1`,
/// but the state is still mutated and the new data is still absorbed.
#[test]
fn xof_update_after_squeeze_returns_minus1_but_absorbs() {
    let fams = xof_families();
    for f in &fams {
        let mut rng = Rng::new(0x1F0_000C ^ f.rate as u64 ^ f.name.len() as u64);
        for inlen in [0usize, 1, f.rate - 1, f.rate, f.rate + 1, 2 * f.rate + 3] {
            let m1 = rng.bytes(inlen);
            let label = format!("{} upd-after-sq inlen={inlen}", f.name);
            let mut run = XofRun::new(f, &label);
            assert_eq!(run.update(&m1, &format!("{label} m1")), 0);
            let s1 = run.squeeze(37, &format!("{label} sq1"));
            assert_eq!(s1.len(), 37);
            let after_sq = run.c.snapshot();

            let m2 = vec![0x3Cu8; f.rate + 7];
            let rc = run.update(&m2, &format!("{label} m2"));
            assert_eq!(rc, -1, "{label}: update after squeeze must return exactly -1");
            let after_upd = run.c.snapshot();
            assert_ne!(after_sq, after_upd, "{label}: update after squeeze must mutate the state");

            // reference run: same prefix, then an *empty* update after squeeze
            let mut refrun = XofRun::new(f, &format!("{label} ref"));
            assert_eq!(refrun.update(&m1, &format!("{label} ref m1")), 0);
            let s1b = refrun.squeeze(37, &format!("{label} ref sq1"));
            eqb(&format!("{label}: reproducible squeeze"), &s1, &s1b);
            assert_eq!(
                refrun.update(&[], &format!("{label} ref empty")),
                -1,
                "{label}: an empty update after squeeze must also return -1"
            );
            assert_ne!(after_sq, refrun.c.snapshot(), "{label}: empty update still permutes");
            assert_ne!(
                refrun.c.snapshot(),
                after_upd,
                "{label}: the absorbed data must change the state"
            );

            // Absorbing resumes: a further squeeze is legal and returns 0.
            let s2 = run.squeeze(64, &format!("{label} sq2"));
            assert_eq!(s2.len(), 64);
            // ...and a second `-1` update chains the same way.
            assert_eq!(run.update(&m2, &format!("{label} m3")), -1);
            let _ = run.squeeze(11, &format!("{label} sq3"));
        }
    }
}

/// errors_3 rows 3.29 / 3.30: `squeeze` after `squeeze` is legal stream
/// continuation - always `0`, never an error, and equal to one long squeeze.
#[test]
fn xof_squeeze_after_squeeze_is_stream_continuation() {
    let fams = xof_families();
    for f in &fams {
        let mut rng = Rng::new(0x1F0_000D ^ f.rate as u64 ^ f.name.len() as u64);
        for inlen in [0usize, 1, f.rate - 1, f.rate, 2 * f.rate] {
            let msg = rng.bytes(inlen);
            let label = format!("{} sqsq inlen={inlen}", f.name);
            let mut run = XofRun::new(f, &label);
            assert_eq!(run.update(&msg, &format!("{label} m")), 0);
            let mut acc = Vec::new();
            // 40 consecutive squeezes with wildly varying sizes.
            for k in 0..40usize {
                let n = match k % 5 {
                    0 => 1,
                    1 => f.rate - 1,
                    2 => f.rate,
                    3 => f.rate + 1,
                    _ => rng.range(0, 3 * f.rate),
                };
                acc.extend_from_slice(&run.squeeze(n, &format!("{label} #{k}")));
            }
            let base = f.one_shot(&msg, acc.len(), &label);
            eqb(&format!("{label}: 40 squeezes == one squeeze of {}", acc.len()), &base, &acc);
        }
    }
}

/// configs_3 row 3.129: `init` (and `init_with_domain`) after `squeeze` must
/// fully reset `phase`/`offset`/`domain`.
#[test]
fn xof_state_reuse_after_squeeze() {
    let fams = xof_families();
    for f in &fams {
        let mut rng = Rng::new(0x1F0_000E ^ f.rate as u64 ^ f.name.len() as u64);
        let m1 = rng.bytes(f.rate + 9);
        let m2 = rng.bytes(2 * f.rate - 1);
        let label = format!("{} reuse", f.name);
        let mut run = XofRun::new(f, &label);
        assert_eq!(run.update(&m1, &format!("{label} m1")), 0);
        let a = run.squeeze(200, &format!("{label} sq1"));
        eqb(&format!("{label}: m1"), &f.one_shot(&m1, 200, &label), &a);

        run.init(&format!("{label} re-init"));
        assert_eq!(run.update(&m2, &format!("{label} m2")), 0);
        let b = run.squeeze(200, &format!("{label} sq2"));
        eqb(&format!("{label}: m2 after re-init"), &f.one_shot(&m2, 200, &label), &b);

        run.init_domain(0x80, &format!("{label} re-initd"));
        assert_eq!(run.update(&m2, &format!("{label} m3")), 0);
        let c = run.squeeze(200, &format!("{label} sq3"));
        let d = XofRun::stream(f, Some(0x80), &m2, &[m2.len()], &[200], &label);
        eqb(&format!("{label}: m2 after re-init_with_domain(0x80)"), &d, &c);
        assert_ne!(b, c, "{label}: a different domain must give a different stream");
    }
}

/// configs_3 row 3.130: `out` aliasing `in` for the XOF one-shots.
#[test]
fn xof_one_shot_aliased_out_in() {
    let fams = xof_families();
    for f in &fams {
        let mut rng = Rng::new(0x1F0_000F ^ f.rate as u64 ^ f.name.len() as u64);
        for inlen in [0usize, 1, 32, f.rate - 1, f.rate, f.rate + 1, 300] {
            for &ol in &[0usize, 1, 32, f.rate, 2 * f.rate + 1] {
                let n = inlen.max(ol);
                let base = rng.bytes(n);
                let mut bc = padded(n);
                let mut br = padded(n);
                bc[..n].copy_from_slice(&base);
                br[..n].copy_from_slice(&base);
                let rc =
                    unsafe { (f.one.0)(bc.as_mut_ptr(), ol, bc.as_ptr(), inlen as u64) };
                let rr =
                    unsafe { (f.one.1)(br.as_mut_ptr(), ol, br.as_ptr(), inlen as u64) };
                eqi(&format!("{} alias in={inlen} out={ol} ret", f.name), rc, rr);
                eqb(&format!("{} alias in={inlen} out={ol}", f.name), &bc, &br);
                check_pad(&format!("{} alias C in={inlen} out={ol}", f.name), &bc, n);
                check_pad(&format!("{} alias R in={inlen} out={ol}", f.name), &br, n);
                let ref_out =
                    f.one_shot(&base[..inlen], ol, &format!("{} alias ref", f.name));
                eqb(
                    &format!("{} alias in={inlen} out={ol} == non-aliased", f.name),
                    &ref_out,
                    &bc[..ol],
                );
            }
        }
    }
}

// -------------------------------------------------------- cross-family checks

/// configs_3 rows 3.62 / 3.82: SHAKE128 vs SHAKE256 (different rate),
/// TurboSHAKE vs SHAKE at equal rate (12 vs 24 rounds).
#[test]
fn xof_families_are_mutually_distinct() {
    let fams = xof_families();
    if fams.len() < 2 {
        eprintln!("SKIP xof_families_are_mutually_distinct: need >= 2 families");
        return;
    }
    let mut rng = Rng::new(0x1F0_0010);
    for inlen in [0usize, 1, 100, 135, 136, 137, 167, 168, 169, 272, 336, 1024] {
        let msg = rng.bytes(inlen);
        let outs: Vec<(&str, Vec<u8>)> = fams
            .iter()
            .map(|f| (f.name, f.one_shot(&msg, 256, &format!("{} distinct", f.name))))
            .collect();
        for i in 0..outs.len() {
            for j in i + 1..outs.len() {
                assert_ne!(
                    outs[i].1, outs[j].1,
                    "{} and {} must differ (inlen={inlen})",
                    outs[i].0, outs[j].0
                );
            }
        }
    }
}

/// errors_3 rows 3.25 / 3.27 / 3.29 at their own entry points: the internal
/// `*_ref_{init,init_with_domain,update,squeeze}` functions (exported by both
/// libraries under `_sodium_*_ref_*`) take the internal state directly, so the
/// domain-byte sweep, the `-1` on absorb-after-squeeze and the total absence of
/// a squeeze error path are pinned without going through the public wrapper.
#[test]
fn xof_ref_streaming_entry_points() {
    for f in &xof_families() {
        let names = [
            format!("{}_init", f.refname),
            format!("{}_init_with_domain", f.refname),
            format!("{}_update", f.refname),
            format!("{}_squeeze", f.refname),
        ];
        if let Some(m) = names.iter().find(|s| !has(s)) {
            eprintln!("SKIP {}: `{m}` is not exported by both libraries", f.refname);
            continue;
        }
        let init = both::<Init0>(&names[0]);
        let initd = both::<InitD>(&names[1]);
        // The internal `update`/`squeeze` take a `size_t` length, not `u64`.
        let upd = both::<RefUpd>(&names[2]);
        let sq = both::<Sq>(&names[3]);
        let mut rng = Rng::new(0x1F0_0012 ^ f.rate as u64 ^ f.name.len() as u64);

        // `_ref_init` must agree with `_ref_init_with_domain(0x1f)` and with
        // the public `_init`, state byte for state byte.
        {
            let (mut a, mut b) = (St::new(), St::new());
            let (mut pa, mut pb) = (St::new(), St::new());
            unsafe {
                assert_eq!((init.0)(a.p()), 0);
                assert_eq!((init.1)(b.p()), 0);
                assert_eq!((initd.0)(pa.p(), 0x1f), 0);
                assert_eq!((initd.1)(pb.p(), 0x1f), 0);
            }
            cmp_state(&format!("{}_init", f.refname), &a, &b);
            cmp_state(&format!("{}_init_with_domain(0x1f)", f.refname), &pa, &pb);
            assert_eq!(a.snapshot(), pa.snapshot(), "{}: _init == _init_with_domain(0x1f)", f.refname);
        }

        for d in 0u8..=0xff {
            for inlen in [0usize, 1, f.rate - 1, f.rate, f.rate + 1] {
                let msg = rng.bytes(inlen);
                let label = format!("{} d={d:#04x} in={inlen}", f.refname);
                let (mut sc, mut sr) = (St::new(), St::new());
                unsafe {
                    // errors_3 3.25: no range check on `domain`, always 0.
                    eqi(&format!("{label} initd ret"), (initd.0)(sc.p(), d), (initd.1)(sr.p(), d));
                    assert_eq!((initd.0)(sc.p(), d), 0);
                    assert_eq!((initd.1)(sr.p(), d), 0);
                }
                cmp_state(&format!("{label} after init"), &sc, &sr);
                let rc = unsafe { (upd.0)(sc.p(), msg.as_ptr(), inlen) };
                let rr = unsafe { (upd.1)(sr.p(), msg.as_ptr(), inlen) };
                eqi(&format!("{label} update ret"), rc, rr);
                assert_eq!(rc, 0, "{label}: absorb while ABSORBING must return 0");
                cmp_state(&format!("{label} after update"), &sc, &sr);

                // errors_3 3.29: squeeze never errors, in one call or many.
                let n = f.rate + 9;
                let mut oc = padded(n);
                let mut or = padded(n);
                let qc = unsafe { (sq.0)(sc.p(), oc.as_mut_ptr(), n) };
                let qr = unsafe { (sq.1)(sr.p(), or.as_mut_ptr(), n) };
                eqi(&format!("{label} squeeze ret"), qc, qr);
                assert_eq!(qc, 0, "{label}: squeeze must always return 0");
                eqb(&format!("{label} squeeze out"), &oc[..n], &or[..n]);
                check_pad(&format!("{label} squeeze C"), &oc, n);
                check_pad(&format!("{label} squeeze Rust"), &or, n);
                cmp_state(&format!("{label} after squeeze"), &sc, &sr);

                // Must match the public streaming path with the same domain.
                let want = XofRun::stream(f, Some(d), &msg, &[inlen], &[n], &label);
                eqb(&format!("{label}: _ref == public wrapper"), &want, &oc[..n]);

                // errors_3 3.27: absorb-after-squeeze returns exactly -1.
                let after_sq = sc.snapshot();
                let m2 = vec![0x5Eu8; 3];
                let uc = unsafe { (upd.0)(sc.p(), m2.as_ptr(), m2.len()) };
                let ur = unsafe { (upd.1)(sr.p(), m2.as_ptr(), m2.len()) };
                eqi(&format!("{label} upd-after-sq ret"), uc, ur);
                assert_eq!(uc, -1, "{label}: absorb after squeeze must return exactly -1");
                cmp_state(&format!("{label} after upd-after-sq"), &sc, &sr);
                assert_ne!(after_sq, sc.snapshot(), "{label}: the data is still absorbed");
            }
        }
    }
}

/// errors_3 rows 3.31 / 3.32: the internal `*_ref` one-shots (exported by both
/// libraries under `_sodium_*_ref`) must agree with each other and with the
/// public one-shot wrapper.
#[test]
fn xof_ref_one_shots_match_public() {
    let fams = xof_families();
    for f in &fams {
        if !has(f.refname) {
            eprintln!("SKIP {}: not exported by both libraries", f.refname);
            continue;
        }
        let (c, r) = both::<RefXof>(f.refname);
        let mut rng = Rng::new(0x1F0_0011 ^ f.rate as u64 ^ f.name.len() as u64);
        for inlen in [0usize, 1, f.rate - 1, f.rate, f.rate + 1, 2 * f.rate, 300, 1024] {
            for &ol in &[0usize, 1, 32, f.rate, 2 * f.rate + 1, 512] {
                let msg = rng.bytes(inlen);
                let mut oc = padded(ol);
                let mut or = padded(ol);
                let rc = unsafe { c(oc.as_mut_ptr(), ol, msg.as_ptr(), inlen) };
                let rr = unsafe { r(or.as_mut_ptr(), ol, msg.as_ptr(), inlen) };
                eqi(&format!("{} ret", f.refname), rc, rr);
                assert_eq!(rc, 0, "{}: must always return 0", f.refname);
                eqb(&format!("{} in={inlen} out={ol}", f.refname), &oc[..ol], &or[..ol]);
                check_pad(&format!("{} C", f.refname), &oc, ol);
                check_pad(&format!("{} Rust", f.refname), &or, ol);
                let pub_out =
                    f.one_shot(&msg, ol, &format!("{} pub in={inlen} out={ol}", f.name));
                eqb(
                    &format!("{} == {} (in={inlen} out={ol})", f.refname, f.name),
                    &pub_out,
                    &oc[..ol],
                );
            }
        }
    }
}
