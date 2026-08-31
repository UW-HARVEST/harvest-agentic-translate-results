//! Area 4b — `crypto_onetimeauth` (generic) and `crypto_onetimeauth_poly1305`
//! (donna / `poly1305_donna32.h` backend).
//!
//! Covers `configs_4.md` rows 4.142–4.182 and `errors_4.md` rows 4.11, 4.12,
//! 4.22, 4.23 (poly1305 half), 4.24 (poly1305 half).
//!
//! Every check compares the **full opaque state** (256 bytes, the size reported
//! by `crypto_onetimeauth_poly1305_statebytes()`) between C and Rust after
//! `_init` and after *every* `_update`, which is a far stronger constraint than
//! only matching the final 16-byte tag: it pins the 26-bit-limb `h`/`r`
//! representation, the `pad[4]` copy, the `leftover` counter, the 16-byte
//! `buffer` contents and the `final` flag, i.e. the whole donna32 struct
//! layout.
mod common;
use common::*;
use std::ffi::{c_char, c_int};

type Auth = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> c_int;
type Verify = unsafe extern "C" fn(*const u8, *const u8, u64, *const u8) -> c_int;
type Init = unsafe extern "C" fn(*mut u8, *const u8) -> c_int;
type Update = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
type Fin = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;
type Keygen = unsafe extern "C" fn(*mut u8);
type SizeFn = unsafe extern "C" fn() -> usize;
type StrFn = unsafe extern "C" fn() -> *const c_char;
type PickFn = unsafe extern "C" fn() -> c_int;

const TAG: usize = 16; // crypto_onetimeauth_poly1305_BYTES
const KEY: usize = 32; // crypto_onetimeauth_poly1305_KEYBYTES
const BLOCK: usize = 16; // poly1305_block_size

/// Message lengths named by rows 4.142–4.157 / 4.172.
const LENS: [usize; 8] = [0, 1, 15, 16, 17, 31, 32, 33];

/// The deterministic RNG streams installed by `common` are process-global, so
/// any test that reseeds and drains them must hold this lock (`cargo test` runs
/// tests on separate threads).  Only `keygen()` consumes randomness here, but
/// the lock keeps that true by construction.
static RNG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

// ------------------------------------------------------------------ plumbing

fn size_of_both(name: &str) -> usize {
    let (c, r) = both::<SizeFn>(name);
    let (cv, rv) = unsafe { (c(), r()) };
    assert_eq!(cv, rv, "{name}: size mismatch (C {cv}, Rust {rv})");
    cv
}

/// `crypto_onetimeauth_poly1305_statebytes()`, agreed by both libraries.
fn statebytes() -> usize {
    size_of_both("crypto_onetimeauth_poly1305_statebytes")
}

/// One-shot `*(out, in, inlen, k)` on both libraries.
#[track_caller]
fn oneshot_sym(name: &str, msg: &[u8], key: &[u8], label: &str) -> Vec<u8> {
    assert_eq!(key.len(), KEY);
    let (c, r) = both::<Auth>(name);
    let mut co = padded(TAG);
    let mut ro = padded(TAG);
    let (rc, rr) = unsafe {
        (
            c(co.as_mut_ptr(), msg.as_ptr(), msg.len() as u64, key.as_ptr()),
            r(ro.as_mut_ptr(), msg.as_ptr(), msg.len() as u64, key.as_ptr()),
        )
    };
    eqi(&format!("{label}: {name} rc"), rc, rr);
    // errors_4 4.23 — no reachable rejection, always 0.
    assert_eq!(rc, 0, "{label}: {name} must return 0");
    eqb(&format!("{label}: {name} tag"), &co[..TAG], &ro[..TAG]);
    check_pad(&format!("{label}: {name} C out"), &co, TAG);
    check_pad(&format!("{label}: {name} R out"), &ro, TAG);
    co.truncate(TAG);
    co
}

#[track_caller]
fn oneshot(msg: &[u8], key: &[u8], label: &str) -> Vec<u8> {
    oneshot_sym("crypto_onetimeauth_poly1305", msg, key, label)
}

/// `*_verify` on both libraries; asserts equal return code and returns it.
#[track_caller]
fn verify_sym(name: &str, tag: &[u8], msg: &[u8], key: &[u8], label: &str) -> c_int {
    assert_eq!(tag.len(), TAG);
    let (c, r) = both::<Verify>(name);
    let (rc, rr) = unsafe {
        (
            c(tag.as_ptr(), msg.as_ptr(), msg.len() as u64, key.as_ptr()),
            r(tag.as_ptr(), msg.as_ptr(), msg.len() as u64, key.as_ptr()),
        )
    };
    eqi(&format!("{label}: {name} rc"), rc, rr);
    rc
}

#[track_caller]
fn verify(tag: &[u8], msg: &[u8], key: &[u8], label: &str) -> c_int {
    verify_sym("crypto_onetimeauth_poly1305_verify", tag, msg, key, label)
}

/// Result of a streaming run: the tag plus the full state snapshot taken after
/// `init` and after each `update`.
struct Run {
    tag: Vec<u8>,
    states: Vec<Vec<u8>>,
}

/// Streaming `init` / `update`* / `final` on both libraries, comparing the
/// **entire** `statebytes()`-sized opaque buffer after `init` and after every
/// single `update`, plus the state left behind by `final`.
///
/// `init_sym` / `update_sym` / `final_sym` are named separately so that the
/// generic-vs-primitive cross-mixing configurations (row 4.179) can be driven
/// through the very same comparison logic.
#[track_caller]
fn stream_with(
    init_sym: &str,
    update_sym: &str,
    final_sym: &str,
    key: &[u8],
    chunks: &[&[u8]],
    label: &str,
) -> Run {
    assert_eq!(key.len(), KEY);
    let sb = statebytes();
    let (ci, ri) = both::<Init>(init_sym);
    let (cu, ru) = both::<Update>(update_sym);
    let (cf, rf) = both::<Fin>(final_sym);

    // `padded()` zeroes the buffer, so the bytes the donna32 struct never
    // touches (the tail padding and everything past
    // `sizeof(poly1305_state_internal_t)`) start out identical and the
    // full-width comparison below is meaningful.
    let mut cs = padded(sb);
    let mut rs = padded(sb);

    let (rc, rr) = unsafe { (ci(cs.as_mut_ptr(), key.as_ptr()), ri(rs.as_mut_ptr(), key.as_ptr())) };
    eqi(&format!("{label}: {init_sym} rc"), rc, rr);
    assert_eq!(rc, 0, "{label}: {init_sym} must return 0");
    eqb(&format!("{label}: FULL STATE after init"), &cs[..sb], &rs[..sb]);
    check_pad(&format!("{label}: C state"), &cs, sb);
    check_pad(&format!("{label}: R state"), &rs, sb);

    let mut states = vec![cs[..sb].to_vec()];

    for (i, ch) in chunks.iter().enumerate() {
        let (rc, rr) = unsafe {
            (
                cu(cs.as_mut_ptr(), ch.as_ptr(), ch.len() as u64),
                ru(rs.as_mut_ptr(), ch.as_ptr(), ch.len() as u64),
            )
        };
        eqi(&format!("{label}: {update_sym}[{i}] rc"), rc, rr);
        assert_eq!(rc, 0, "{label}: {update_sym} must return 0");
        eqb(
            &format!("{label}: FULL STATE after update[{i}] (len {})", ch.len()),
            &cs[..sb],
            &rs[..sb],
        );
        check_pad(&format!("{label}: C state"), &cs, sb);
        check_pad(&format!("{label}: R state"), &rs, sb);
        states.push(cs[..sb].to_vec());
    }

    let mut co = padded(TAG);
    let mut ro = padded(TAG);
    let (rc, rr) =
        unsafe { (cf(cs.as_mut_ptr(), co.as_mut_ptr()), rf(rs.as_mut_ptr(), ro.as_mut_ptr())) };
    eqi(&format!("{label}: {final_sym} rc"), rc, rr);
    assert_eq!(rc, 0, "{label}: {final_sym} must return 0");
    eqb(&format!("{label}: tag"), &co[..TAG], &ro[..TAG]);
    eqb(&format!("{label}: FULL STATE after final"), &cs[..sb], &rs[..sb]);
    check_pad(&format!("{label}: C out"), &co, TAG);
    check_pad(&format!("{label}: R out"), &ro, TAG);

    // `poly1305_finish()` ends with `sodium_memzero(st, sizeof *st)`, so the
    // 144-byte donna32 struct must be wiped while the rest of the 256-byte
    // opaque area is untouched.  Both are all-zero here, which the equality
    // above already pinned; assert the wipe explicitly as well.
    assert!(
        cs[..sb].iter().all(|&b| b == 0),
        "{label}: final() must sodium_memzero the whole internal state"
    );

    co.truncate(TAG);
    Run { tag: co, states }
}

#[track_caller]
fn stream(key: &[u8], chunks: &[&[u8]], label: &str) -> Run {
    stream_with(
        "crypto_onetimeauth_poly1305_init",
        "crypto_onetimeauth_poly1305_update",
        "crypto_onetimeauth_poly1305_final",
        key,
        chunks,
        label,
    )
}

fn chunks_of<'a>(msg: &'a [u8], cuts: &[usize]) -> Vec<&'a [u8]> {
    let mut v = Vec::new();
    let mut prev = 0usize;
    for &c in cuts {
        assert!(c >= prev && c <= msg.len());
        v.push(&msg[prev..c]);
        prev = c;
    }
    v.push(&msg[prev..]);
    v
}

fn random_cuts(rng: &mut Rng, len: usize, n: usize) -> Vec<usize> {
    let mut c: Vec<usize> = (0..n).map(|_| rng.below(len + 1)).collect();
    c.sort_unstable();
    c
}

/// Cuts biased to land on / next to 16-byte block boundaries so that the
/// leftover buffer is straddled instead of only randomly grazed.
fn boundary_cuts(rng: &mut Rng, len: usize, n: usize) -> Vec<usize> {
    let mut c = Vec::with_capacity(n);
    for _ in 0..n {
        let blk = rng.below(len / BLOCK + 1) * BLOCK;
        let jitter = rng.below(3) as isize - 1; // -1, 0, +1
        let p = (blk as isize + jitter).clamp(0, len as isize) as usize;
        c.push(p);
    }
    c.sort_unstable();
    c
}

/// Little-endian `u64` field of the raw donna32 state at byte offset `off`.
fn field(state: &[u8], off: usize) -> u64 {
    u64::from_le_bytes(state[off..off + 8].try_into().unwrap())
}

// donna32 `poly1305_state_internal_t` byte offsets (all fields are 8-byte
// aligned because `unsigned long` and `unsigned long long` are both 64-bit on
// this target).
const OFF_R: usize = 0; // unsigned long r[5]
const OFF_H: usize = 40; // unsigned long h[5]
const OFF_PAD: usize = 80; // unsigned long pad[4]
const OFF_LEFTOVER: usize = 112; // unsigned long long leftover
const OFF_BUFFER: usize = 120; // unsigned char buffer[16]
const OFF_FINAL: usize = 136; // unsigned char final
const INTERNAL_SIZE: usize = 144; // sizeof(poly1305_state_internal_t), padded

fn load32_le(b: &[u8]) -> u64 {
    u32::from_le_bytes(b[..4].try_into().unwrap()) as u64
}

// ======================================================================
// 4.177, 4.180, 4.179 (statebytes), 4.22 — accessors and struct size
// ======================================================================

#[test]
fn accessors() {
    // 4.177 — poly1305: 16 / 32 / sizeof(state) == 256.
    assert_eq!(size_of_both("crypto_onetimeauth_poly1305_bytes"), TAG);
    assert_eq!(size_of_both("crypto_onetimeauth_poly1305_keybytes"), KEY);
    let sb = size_of_both("crypto_onetimeauth_poly1305_statebytes");
    assert_eq!(sb, 256, "crypto_onetimeauth_poly1305_state is `unsigned char opaque[256]`");

    // errors_4 4.22 — the COMPILER_ASSERT in _donna_init:
    // sizeof(state) >= sizeof(poly1305_state_internal_t).
    assert!(sb >= INTERNAL_SIZE, "opaque state must cover the donna32 internal struct");

    // 4.180 — generic accessors.
    assert_eq!(size_of_both("crypto_onetimeauth_bytes"), TAG);
    assert_eq!(size_of_both("crypto_onetimeauth_keybytes"), KEY);
    // 4.179 — crypto_onetimeauth_state is a typedef of the poly1305 state.
    assert_eq!(size_of_both("crypto_onetimeauth_statebytes"), sb);

    let (cp, rp) = both::<StrFn>("crypto_onetimeauth_primitive");
    unsafe {
        let cs = std::ffi::CStr::from_ptr(cp());
        let rs = std::ffi::CStr::from_ptr(rp());
        assert_eq!(cs.to_bytes(), b"poly1305", "crypto_onetimeauth_PRIMITIVE");
        assert_eq!(cs, rs, "crypto_onetimeauth_primitive string mismatch");
    }

    // The donna implementation table is an exported data symbol in C; the Rust
    // port must export it too (it is the only implementation ever installed).
    assert!(
        has("crypto_onetimeauth_poly1305_donna_implementation"),
        "crypto_onetimeauth_poly1305_donna_implementation must be exported by both"
    );
}

// ======================================================================
// 4.182 — donna32 backend: observable struct layout of the opaque state
// ======================================================================

#[test]
fn donna32_state_layout_after_init() {
    let mut rng = Rng::new(0x4_d032);
    let sb = statebytes();
    let (ci, ri) = both::<Init>("crypto_onetimeauth_poly1305_init");

    for rep in 0..12 {
        let key = if rep == 0 {
            vec![0u8; KEY]
        } else if rep == 1 {
            vec![0xffu8; KEY]
        } else {
            rng.bytes(KEY)
        };
        let mut cs = padded(sb);
        let mut rs = padded(sb);
        unsafe {
            eqi("layout init rc", ci(cs.as_mut_ptr(), key.as_ptr()), ri(rs.as_mut_ptr(), key.as_ptr()));
        }
        eqb("layout: FULL STATE after init", &cs[..sb], &rs[..sb]);

        // `r` — the five clamped 26-bit limbs of donna32 (a donna64 port would
        // have three 44/42-bit limbs here and this would fail).
        let exp_r = [
            load32_le(&key[0..]) & 0x3ffffff,
            (load32_le(&key[3..]) >> 2) & 0x3ffff03,
            (load32_le(&key[6..]) >> 4) & 0x3ffc0ff,
            (load32_le(&key[9..]) >> 6) & 0x3f03fff,
            (load32_le(&key[12..]) >> 8) & 0x00fffff,
        ];
        for (i, &e) in exp_r.iter().enumerate() {
            assert_eq!(field(&cs, OFF_R + 8 * i), e, "C r[{i}] (rep {rep})");
            assert_eq!(field(&rs, OFF_R + 8 * i), e, "Rust r[{i}] (rep {rep})");
        }
        // `h` == 0
        for i in 0..5 {
            assert_eq!(field(&cs, OFF_H + 8 * i), 0, "C h[{i}] must start at 0");
        }
        // `pad` == key[16..32] as four LE u32s
        for i in 0..4 {
            let e = load32_le(&key[16 + 4 * i..]);
            assert_eq!(field(&cs, OFF_PAD + 8 * i), e, "C pad[{i}]");
            assert_eq!(field(&rs, OFF_PAD + 8 * i), e, "Rust pad[{i}]");
        }
        // `leftover` == 0, `final` == 0
        assert_eq!(field(&cs, OFF_LEFTOVER), 0, "C leftover must start at 0");
        assert_eq!(cs[OFF_FINAL], 0, "C final must start at 0");
        // the 112 bytes of the opaque buffer past the internal struct are never
        // written by init
        assert!(cs[INTERNAL_SIZE..sb].iter().all(|&b| b == 0));
        assert!(rs[INTERNAL_SIZE..sb].iter().all(|&b| b == 0));
        check_pad("layout C state", &cs, sb);
        check_pad("layout R state", &rs, sb);
    }
}

/// The `leftover` counter and `buffer` contents after each update, read straight
/// out of the opaque state — this makes the donna leftover branches directly
/// observable instead of only indirectly via the tag.
#[test]
fn donna32_leftover_field_tracking() {
    let mut rng = Rng::new(0x4_d033);
    let sb = statebytes();
    let key = rng.bytes(KEY);
    let (ci, ri) = both::<Init>("crypto_onetimeauth_poly1305_init");
    let (cu, ru) = both::<Update>("crypto_onetimeauth_poly1305_update");

    // (chunk sizes, expected leftover after each chunk)
    let cases: &[(&[usize], &[u64])] = &[
        (&[0], &[0]),                     // 4.165 zero-length, leftover == 0
        (&[5, 0], &[5, 5]),               // 4.166 zero-length, leftover > 0
        (&[5, 0, 0, 0], &[5, 5, 5, 5]),   // repeated no-ops
        (&[1, 15], &[1, 0]),              // 4.158
        (&[15, 1], &[15, 0]),             // 4.159
        (&[8, 8], &[8, 0]),               // 4.160
        (&[15, 2], &[15, 1]),             // 4.161
        (&[16, 1], &[0, 1]),              // 4.162
        (&[17, 16], &[1, 1]),             // 4.163
        (&[5, 11], &[5, 0]),              // 4.167
        (&[5, 40], &[5, 13]),             // 4.168
        (&[5, 27], &[5, 0]),              // 4.169
        (&[16], &[0]),
        (&[32], &[0]),
        (&[33], &[1]),
        (&[1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1], &[
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0, 1,
        ]),
    ];

    for (sizes, expect) in cases {
        let total: usize = sizes.iter().sum();
        let msg = rng.bytes(total);
        let mut cs = padded(sb);
        let mut rs = padded(sb);
        unsafe {
            eqi("lo init rc", ci(cs.as_mut_ptr(), key.as_ptr()), ri(rs.as_mut_ptr(), key.as_ptr()));
        }
        eqb("lo: FULL STATE after init", &cs[..sb], &rs[..sb]);

        let mut off = 0usize;
        for (i, (&n, &exp)) in sizes.iter().zip(expect.iter()).enumerate() {
            let ch = &msg[off..off + n];
            let before = cs[..sb].to_vec();
            unsafe {
                eqi(
                    "lo update rc",
                    cu(cs.as_mut_ptr(), ch.as_ptr(), n as u64),
                    ru(rs.as_mut_ptr(), ch.as_ptr(), n as u64),
                );
            }
            eqb(&format!("lo {sizes:?}: FULL STATE after update[{i}]"), &cs[..sb], &rs[..sb]);
            assert_eq!(
                field(&cs, OFF_LEFTOVER),
                exp,
                "C leftover after {sizes:?} update[{i}] (len {n})"
            );
            assert_eq!(
                field(&rs, OFF_LEFTOVER),
                exp,
                "Rust leftover after {sizes:?} update[{i}] (len {n})"
            );
            // 4.165 / 4.166 — a zero-length update is a complete no-op.
            if n == 0 {
                assert_eq!(
                    before,
                    cs[..sb].to_vec(),
                    "zero-length update must not change the state (leftover was {})",
                    field(&before, OFF_LEFTOVER)
                );
            }
            // The tail of the leftover buffer past `leftover` is never rewound,
            // so C and Rust must keep the same stale bytes there — already
            // covered by the FULL STATE comparison; assert the live prefix
            // matches the message so the buffer offset arithmetic is pinned.
            let lo = field(&cs, OFF_LEFTOVER) as usize;
            if lo > 0 {
                let end = off + n;
                assert_eq!(
                    &cs[OFF_BUFFER..OFF_BUFFER + lo],
                    &msg[end - lo..end],
                    "C leftover buffer prefix after {sizes:?} update[{i}]"
                );
            }
            off += n;
        }
    }
}

// ======================================================================
// 4.142–4.149 — one-shot, every named length (plus 0..300 and multi-KiB)
// ======================================================================

#[test]
fn oneshot_named_lengths() {
    let mut rng = Rng::new(0x4_1305);
    for &len in LENS.iter() {
        for rep in 0..8 {
            let key = rng.bytes(KEY);
            let msg = rng.bytes(len);
            let tag = oneshot(&msg, &key, &format!("poly1305 len={len} rep={rep}"));
            // 4.172 — a freshly produced tag must verify.
            assert_eq!(verify(&tag, &msg, &key, &format!("good len={len}")), 0);
        }
    }
}

#[test]
fn oneshot_all_lengths_0_to_300() {
    let mut rng = Rng::new(0x4_1306);
    let key = rng.bytes(KEY);
    for len in 0..=300usize {
        let msg = rng.bytes(len);
        let tag = oneshot(&msg, &key, &format!("poly1305 sweep len={len}"));
        assert_eq!(verify(&tag, &msg, &key, &format!("sweep good len={len}")), 0);
    }
}

// ======================================================================
// 4.150–4.157 — streaming with a single update == one-shot
// ======================================================================

#[test]
fn streaming_single_update_named_lengths() {
    let mut rng = Rng::new(0x4_1307);
    for &len in LENS.iter() {
        for rep in 0..6 {
            let key = rng.bytes(KEY);
            let msg = rng.bytes(len);
            let a = oneshot(&msg, &key, &format!("os len={len}"));
            let b = stream(&key, &[&msg[..]], &format!("stream1 len={len} rep={rep}"));
            eqb(&format!("stream(single update) == one-shot len={len}"), &a, &b.tag);
        }
    }
}

#[test]
fn streaming_single_update_0_to_300() {
    let mut rng = Rng::new(0x4_1308);
    let key = rng.bytes(KEY);
    for len in 0..=300usize {
        let msg = rng.bytes(len);
        let a = oneshot(&msg, &key, &format!("os sweep len={len}"));
        let b = stream(&key, &[&msg[..]], &format!("stream1 sweep len={len}"));
        eqb(&format!("stream1 == one-shot len={len}"), &a, &b.tag);
    }
}

// ======================================================================
// 4.158–4.163, 4.167–4.169 — the named leftover-buffer split configurations
// ======================================================================

#[test]
fn leftover_branch_fixed_splits() {
    let mut rng = Rng::new(0x4_1309);
    let key = rng.bytes(KEY);

    // (total length, cut offsets, which row / donna branch it drives)
    let cases: &[(usize, &[usize], &str)] = &[
        (16, &[1], "4.158 (1,15): 2nd update fills leftover to exactly 16, flushes, resets"),
        (16, &[15], "4.159 (15,1): want = 16-15 = 1, block completed by one byte"),
        (16, &[8], "4.160 (8,8): pure leftover accumulation until the block completes"),
        (17, &[15], "4.161 (15,2): fill + flush + re-store 1 byte at buffer[0]"),
        (17, &[16], "4.162 (16,1): first update is the full-block path, leftover == 0"),
        (33, &[17], "4.163 (17,16): leftover 1, fill 15, flush, store 1"),
        (16, &[5], "4.167 (5,11): leftover completes the block, no remainder"),
        (45, &[5], "4.168 (5,40): fill 11 + flush + 16 full-block + store 13"),
        (32, &[5], "4.169 (5,27): fill 11 + flush + 16 full-block + store nothing"),
        // more of the same shapes at other leftover offsets
        (16, &[3], "(3,13)"),
        (16, &[13], "(13,3)"),
        (48, &[7], "(7,41)"),
        (64, &[9], "(9,55)"),
        (64, &[1, 2, 3, 20, 21], "many mixed cuts"),
        (100, &[16, 32, 48], "aligned cuts only"),
        (100, &[15, 31, 47, 63], "one-short-of-aligned cuts"),
        (100, &[17, 33, 49, 65], "one-past-aligned cuts"),
    ];

    for (len, cuts, why) in cases {
        let msg = rng.bytes(*len);
        let a = oneshot(&msg, &key, &format!("split ref {why}"));
        let b = stream(&key, &chunks_of(&msg, cuts), &format!("split len={len} cuts={cuts:?} :: {why}"));
        eqb(&format!("split len={len} cuts={cuts:?} :: {why}"), &a, &b.tag);
    }
}

// 4.164 — 33 successive 1-byte updates, and 1-byte updates for every length.
#[test]
fn many_one_byte_updates() {
    let mut rng = Rng::new(0x4_130a);
    let key = rng.bytes(KEY);
    for len in 0..=140usize {
        let msg = rng.bytes(len);
        let single: Vec<&[u8]> = msg.iter().map(std::slice::from_ref).collect();
        let a = oneshot(&msg, &key, &format!("1byte ref len={len}"));
        let b = stream(&key, &single, &format!("{len} one-byte updates"));
        eqb(&format!("{len} one-byte updates == one-shot"), &a, &b.tag);
    }
    // explicitly the row-4.164 case
    let msg = rng.bytes(33);
    let single: Vec<&[u8]> = msg.iter().map(std::slice::from_ref).collect();
    let a = oneshot(&msg, &key, "4.164 ref");
    let b = stream(&key, &single, "4.164: 33 one-byte updates");
    eqb("4.164: 33 one-byte updates == one-shot(33)", &a, &b.tag);
}

// 4.165, 4.166 — zero-length updates interleaved everywhere.
#[test]
fn zero_length_updates() {
    let mut rng = Rng::new(0x4_130b);
    let key = rng.bytes(KEY);

    // 4.165 — repeated zero-length updates immediately after init.
    for len in [0usize, 1, 15, 16, 17, 33] {
        let msg = rng.bytes(len);
        let empty: &[u8] = &msg[..0];
        let mut chunks: Vec<&[u8]> = vec![empty; 5];
        chunks.push(&msg[..]);
        chunks.extend_from_slice(&[empty; 3]);
        let a = oneshot(&msg, &key, &format!("zero ref len={len}"));
        let b = stream(&key, &chunks, &format!("4.165/4.166 zero-length updates len={len}"));
        eqb(&format!("zero-length updates are no-ops (len={len})"), &a, &b.tag);
    }

    // 4.166 — a zero-length update with leftover strictly between 1 and 15,
    // sandwiched at every possible leftover value.
    for lo in 1..BLOCK {
        let msg = rng.bytes(lo + 20);
        let empty: &[u8] = &msg[..0];
        let chunks: Vec<&[u8]> = vec![&msg[..lo], empty, empty, &msg[lo..]];
        let a = oneshot(&msg, &key, &format!("4.166 ref leftover={lo}"));
        let b = stream(&key, &chunks, &format!("4.166 zero-length update with leftover={lo}"));
        eqb(&format!("4.166 leftover={lo}"), &a, &b.tag);
    }
}

// ======================================================================
// randomized multi-chunk splits straddling the 16-byte block boundary
// ======================================================================

#[test]
fn randomized_splits_0_to_300() {
    let mut rng = Rng::new(0x4_130c);
    let key = rng.bytes(KEY);
    for len in 0..=300usize {
        let msg = rng.bytes(len);
        let a = oneshot(&msg, &key, &format!("rnd ref len={len}"));
        for n in [1usize, 2, 3, 5, 8] {
            let cuts = random_cuts(&mut rng, len, n);
            let b = stream(&key, &chunks_of(&msg, &cuts), &format!("rnd len={len} cuts={cuts:?}"));
            eqb(&format!("rnd split len={len} cuts={cuts:?}"), &a, &b.tag);

            let bc = boundary_cuts(&mut rng, len, n);
            let c = stream(&key, &chunks_of(&msg, &bc), &format!("bnd len={len} cuts={bc:?}"));
            eqb(&format!("boundary split len={len} cuts={bc:?}"), &a, &c.tag);
        }
    }
}

// ======================================================================
// 4.170 — multi-KiB messages, one-shot and split at odd offsets
// ======================================================================

#[test]
fn long_messages() {
    let mut rng = Rng::new(0x4_130d);
    let key = rng.bytes(KEY);
    for &len in &[500usize, 1000, 1023, 1024, 1025, 2048, 4095, 4096, 4097, 8192, 10000] {
        let msg = rng.bytes(len);
        let a = oneshot(&msg, &key, &format!("long os len={len}"));

        // one big update
        let b = stream(&key, &[&msg[..]], &format!("long stream1 len={len}"));
        eqb(&format!("long len={len} stream1"), &a, &b.tag);

        // odd, block-straddling offsets
        for cuts in [
            vec![1usize, 17, 100, 513, 1000.min(len)],
            vec![15, 16, 17, len / 2, len / 2 + 1],
            vec![len / 3, len / 3 * 2],
            vec![len - 1],
            vec![len - len % BLOCK],
        ] {
            let mut cuts: Vec<usize> = cuts.into_iter().filter(|&c| c <= len).collect();
            cuts.sort_unstable();
            let c = stream(&key, &chunks_of(&msg, &cuts), &format!("long len={len} cuts={cuts:?}"));
            eqb(&format!("long len={len} cuts={cuts:?}"), &a, &c.tag);
        }

        // random splits
        for n in [3usize, 7, 16] {
            let cuts = random_cuts(&mut rng, len, n);
            let c = stream(&key, &chunks_of(&msg, &cuts), &format!("long rnd len={len} n={n}"));
            eqb(&format!("long rnd len={len} n={n}"), &a, &c.tag);
        }

        // fixed-size chunking at every size that is not a multiple of 16
        for step in [1usize, 7, 13, 16, 17, 31, 33, 127] {
            let mut cuts = Vec::new();
            let mut p = step;
            while p < len {
                cuts.push(p);
                p += step;
            }
            let c = stream(&key, &chunks_of(&msg, &cuts), &format!("long len={len} step={step}"));
            eqb(&format!("long len={len} step={step}"), &a, &c.tag);
        }
    }
}

// ======================================================================
// 4.171 — key-shape configurations + RFC 8439 known-answer test
// ======================================================================

#[test]
fn key_shapes() {
    let mut rng = Rng::new(0x4_130e);

    let mut pad_ff = vec![0u8; KEY];
    rng.fill(&mut pad_ff[..16]);
    for b in pad_ff[16..].iter_mut() {
        *b = 0xff; // `pad` all-ones: the final (h + pad) addition carries out
    }
    let mut r_ff = vec![0xffu8; KEY];
    rng.fill(&mut r_ff[16..]);

    let rfc8439_key: [u8; KEY] = [
        0x85, 0xd6, 0xbe, 0x78, 0x57, 0x55, 0x6d, 0x33, 0x7f, 0x44, 0x52, 0xfe, 0x42, 0xd5, 0x06,
        0xa8, 0x01, 0x03, 0x80, 0x8a, 0xfb, 0x0d, 0xb2, 0xfd, 0x4a, 0xbf, 0xf6, 0xaf, 0x41, 0x49,
        0xf5, 0x1b,
    ];

    let keys: Vec<(String, Vec<u8>)> = vec![
        ("all-zero".into(), vec![0u8; KEY]),
        ("all-0xff".into(), vec![0xffu8; KEY]),
        ("r=0xff... (max r before clamping)".into(), r_ff),
        ("pad=0xff... (final add carries)".into(), pad_ff),
        ("r=0 (clamped to zero)".into(), {
            let mut k = vec![0u8; KEY];
            rng.fill(&mut k[16..]);
            k
        }),
        ("pad=0".into(), {
            let mut k = vec![0u8; KEY];
            rng.fill(&mut k[..16]);
            k
        }),
        ("RFC 8439 2.5.2".into(), rfc8439_key.to_vec()),
        ("0x01 repeated".into(), vec![1u8; KEY]),
        ("0x80 repeated".into(), vec![0x80u8; KEY]),
        ("counting bytes".into(), (0..KEY as u8).collect()),
    ];

    for (name, key) in keys.iter() {
        for len in [0usize, 1, 15, 16, 17, 31, 32, 33, 34, 63, 64, 65, 128, 257] {
            // Messages chosen to also stress the carry chain: all-zero,
            // all-0xff and random.
            for msg in [vec![0u8; len], vec![0xffu8; len], rng.bytes(len)] {
                let a = oneshot(&msg, key, &format!("key `{name}` len={len}"));
                let cuts = boundary_cuts(&mut rng, len, 3);
                let b = stream(key, &chunks_of(&msg, &cuts), &format!("key `{name}` len={len} split"));
                eqb(&format!("key `{name}` len={len}: stream == one-shot"), &a, &b.tag);
                assert_eq!(verify(&a, &msg, key, &format!("key `{name}` len={len}")), 0);
            }
        }
    }

    // Absolute known-answer check (RFC 8439 §2.5.2) — pins that *both*
    // libraries compute real Poly1305 and not merely the same wrong thing.
    let msg = b"Cryptographic Forum Research Group";
    let expect: [u8; TAG] = [
        0xa8, 0x06, 0x1d, 0xc1, 0x30, 0x51, 0x36, 0xc6, 0xc2, 0x2b, 0x8b, 0xaf, 0x0c, 0x01, 0x27,
        0xa9,
    ];
    let tag = oneshot(msg, &rfc8439_key, "RFC 8439 KAT");
    eqb("RFC 8439 2.5.2 known-answer", &expect, &tag);
    // and the same through streaming, split inside the first block
    for cuts in [vec![0usize], vec![1], vec![15], vec![16], vec![17], vec![33], vec![8, 24]] {
        let b = stream(&rfc8439_key, &chunks_of(msg, &cuts), &format!("RFC 8439 KAT cuts={cuts:?}"));
        eqb("RFC 8439 2.5.2 known-answer (streaming)", &expect, &b.tag);
    }
    assert_eq!(verify(&expect, msg, &rfc8439_key, "RFC 8439 KAT verify"), 0);
}

// ======================================================================
// 4.172–4.175 / errors_4 4.11 — verify
// ======================================================================

#[test]
fn verify_good_and_every_corruption() {
    let mut rng = Rng::new(0x4_130f);
    for &len in LENS.iter() {
        let key = rng.bytes(KEY);
        let msg = rng.bytes(len);
        let tag = oneshot(&msg, &key, &format!("verify base len={len}"));

        // 4.172 — good tag verifies.
        assert_eq!(verify(&tag, &msg, &key, &format!("good len={len}")), 0);

        // 4.173 / 4.174 / errors_4 4.11 — every byte position, every bit.
        for i in 0..TAG {
            for bit in 0..8u32 {
                let mut bad = tag.clone();
                bad[i] ^= 1u8 << bit;
                assert_eq!(
                    verify(&bad, &msg, &key, &format!("bad[{i}:{bit}] len={len}")),
                    -1,
                    "flipping bit {bit} of tag byte {i} must reject (len={len})"
                );
            }
            // whole-byte corruptions too
            for delta in [1u8, 0x7f, 0x80, 0xff] {
                let mut bad = tag.clone();
                bad[i] = bad[i].wrapping_add(delta);
                if bad[i] == tag[i] {
                    continue;
                }
                assert_eq!(verify(&bad, &msg, &key, &format!("bad byte[{i}]+{delta}")), -1);
            }
        }

        // 4.175 — all-zero, all-0xff, random, wrong message, wrong key.
        assert_eq!(verify(&[0u8; TAG], &msg, &key, "zero tag"), if tag == vec![0u8; TAG] { 0 } else { -1 });
        assert_eq!(
            verify(&[0xffu8; TAG], &msg, &key, "0xff tag"),
            if tag == vec![0xffu8; TAG] { 0 } else { -1 }
        );
        for _ in 0..10 {
            let rnd = rng.bytes(TAG);
            if rnd == tag {
                continue;
            }
            assert_eq!(verify(&rnd, &msg, &key, "random tag"), -1);
        }
        let mut other = msg.clone();
        other.push(0);
        assert_eq!(verify(&tag, &other, &key, "wrong msg (extra byte)"), -1);
        if len > 0 {
            let mut flip = msg.clone();
            flip[len - 1] ^= 1;
            assert_eq!(verify(&tag, &flip, &key, "wrong msg (flipped bit)"), -1);
            assert_eq!(verify(&tag, &msg[..len - 1], &key, "wrong msg (truncated)"), -1);
        }
        // 4.175 (wrong key) — done clamp-aware over *every* key bit, which
        // additionally pins the exact `r` clamping mask
        // (0x0ffffffc0ffffffc0ffffffc0fffffff) in both libraries:
        //   * key[16..32] is `pad`, added at the very end, so every bit matters;
        //   * key[3|7|11|15] contribute only their low 4 bits to `r`;
        //   * key[4|8|12] contribute only their high 6 bits to `r`;
        //   * for an empty message `r` is multiplied by nothing at all, so no
        //     bit of key[0..16] can change the tag.
        for kb in 0..KEY {
            for bit in 0..8u32 {
                let mut k2 = key.clone();
                k2[kb] ^= 1u8 << bit;
                let rc = verify(&tag, &msg, &k2, &format!("key[{kb}:{bit}] len={len}"));
                let matters = if kb >= 16 {
                    true
                } else if len == 0 {
                    false
                } else if kb % 4 == 3 {
                    bit < 4
                } else if kb % 4 == 0 && kb != 0 {
                    bit >= 2
                } else {
                    true
                };
                assert_eq!(
                    rc,
                    if matters { -1 } else { 0 },
                    "len={len}: flipping bit {bit} of key byte {kb} \
                     (clamp says it {}) gave {rc}",
                    if matters { "matters" } else { "is discarded" }
                );
            }
        }
    }
}

/// The tag from a streaming run must verify, for every length in 0..300, and a
/// single-bit corruption of it must be rejected.
#[test]
fn verify_streaming_tags_sweep() {
    let mut rng = Rng::new(0x4_1310);
    let key = rng.bytes(KEY);
    for len in 0..=300usize {
        let msg = rng.bytes(len);
        let cuts = boundary_cuts(&mut rng, len, 3);
        let run = stream(&key, &chunks_of(&msg, &cuts), &format!("verify stream len={len}"));
        assert_eq!(verify(&run.tag, &msg, &key, &format!("stream good len={len}")), 0);
        let mut bad = run.tag.clone();
        let i = rng.below(TAG);
        bad[i] ^= 1u8 << rng.below(8);
        assert_eq!(verify(&bad, &msg, &key, &format!("stream bad len={len}")), -1);
    }
}

// ======================================================================
// 4.176 / errors_4 4.24 — keygen
// ======================================================================

#[test]
fn keygen() {
    let _rng_guard = RNG_LOCK.lock().unwrap();
    for name in ["crypto_onetimeauth_poly1305_keygen", "crypto_onetimeauth_keygen"] {
        let (c, r) = both::<Keygen>(name);
        // A single `rng_reset()` per C/Rust pair, so the two independent RNG
        // streams hand out identical bytes to identical calls.
        rng_reset();
        let mut ck = padded(KEY);
        unsafe { c(ck.as_mut_ptr()) };
        rng_reset();
        let mut rk = padded(KEY);
        unsafe { r(rk.as_mut_ptr()) };
        eqb(name, &ck[..KEY], &rk[..KEY]);
        check_pad(&format!("{name} C"), &ck, KEY);
        check_pad(&format!("{name} R"), &rk, KEY);
        assert!(ck[..KEY].iter().any(|&b| b != 0), "{name} produced an all-zero key");

        // Successive calls (from the same rewound seed on both sides) differ.
        let mut prev: Option<Vec<u8>> = None;
        rng_reset();
        let mut cks = Vec::new();
        for _ in 0..8 {
            let mut k = padded(KEY);
            unsafe { c(k.as_mut_ptr()) };
            check_pad(&format!("{name} C seq"), &k, KEY);
            cks.push(k[..KEY].to_vec());
        }
        rng_reset();
        for (i, want) in cks.iter().enumerate() {
            let mut k = padded(KEY);
            unsafe { r(k.as_mut_ptr()) };
            check_pad(&format!("{name} R seq"), &k, KEY);
            eqb(&format!("{name} call {i}"), want, &k[..KEY]);
            if let Some(p) = &prev {
                assert_ne!(p, want, "{name}: successive outputs must differ");
            }
            prev = Some(want.clone());
        }

        // The generated key is usable.
        let msg = vec![0x5au8; 70];
        let t = oneshot(&msg, &cks[0], &format!("{name} key usable"));
        assert_eq!(verify(&t, &msg, &cks[0], "keygen key verify"), 0);
    }

    // Both keygens write exactly KEYBYTES; the two entry points are the same
    // code path (`randombytes_buf(k, 32)`), so from a rewound RNG they agree.
    let (cp, rp) = both::<Keygen>("crypto_onetimeauth_poly1305_keygen");
    let (cg, rg) = both::<Keygen>("crypto_onetimeauth_keygen");
    for (a, b, what) in [
        (&cp, &cg, "C poly1305_keygen == C generic keygen"),
        (&rp, &rg, "Rust poly1305_keygen == Rust generic keygen"),
    ] {
        rng_reset();
        let mut x = padded(KEY);
        unsafe { a(x.as_mut_ptr()) };
        rng_reset();
        let mut y = padded(KEY);
        unsafe { b(y.as_mut_ptr()) };
        eqb(what, &x[..KEY], &y[..KEY]);
    }
}

// ======================================================================
// 4.178 — generic crypto_onetimeauth / _verify == poly1305 versions
// ======================================================================

#[test]
fn generic_oneshot_and_verify() {
    let mut rng = Rng::new(0x4_1311);
    for &len in LENS.iter() {
        for rep in 0..4 {
            let key = rng.bytes(KEY);
            let msg = rng.bytes(len);
            let g = oneshot_sym("crypto_onetimeauth", &msg, &key, &format!("generic len={len} rep={rep}"));
            let p = oneshot(&msg, &key, &format!("delegate len={len} rep={rep}"));
            eqb(&format!("crypto_onetimeauth == crypto_onetimeauth_poly1305 (len={len})"), &g, &p);

            // good tag
            let gv = verify_sym("crypto_onetimeauth_verify", &g, &msg, &key, "generic good");
            assert_eq!(gv, 0);
            assert_eq!(gv, verify(&p, &msg, &key, "delegate good"));

            // errors_4 4.12 — every corruption propagates from 4.11.
            for i in 0..TAG {
                for bit in 0..8u32 {
                    let mut bad = g.clone();
                    bad[i] ^= 1u8 << bit;
                    let gv = verify_sym("crypto_onetimeauth_verify", &bad, &msg, &key, "generic bad");
                    assert_eq!(gv, -1);
                    assert_eq!(
                        gv,
                        verify(&bad, &msg, &key, "delegate bad"),
                        "crypto_onetimeauth_verify must agree with crypto_onetimeauth_poly1305_verify"
                    );
                }
            }
        }
    }

    // Length sweep of the generic wrapper too.
    let key = rng.bytes(KEY);
    for len in 0..=200usize {
        let msg = rng.bytes(len);
        let g = oneshot_sym("crypto_onetimeauth", &msg, &key, &format!("generic sweep len={len}"));
        let p = oneshot(&msg, &key, &format!("delegate sweep len={len}"));
        eqb(&format!("generic sweep len={len}"), &g, &p);
        assert_eq!(verify_sym("crypto_onetimeauth_verify", &g, &msg, &key, "sweep"), 0);
    }
}

// ======================================================================
// 4.179 — generic streaming wrappers, including cross-mixing with the
//         primitive-specific ones (the state types are the same typedef)
// ======================================================================

#[test]
fn generic_streaming_and_crossmixing() {
    let mut rng = Rng::new(0x4_1312);
    const GI: &str = "crypto_onetimeauth_init";
    const GU: &str = "crypto_onetimeauth_update";
    const GF: &str = "crypto_onetimeauth_final";
    const PI: &str = "crypto_onetimeauth_poly1305_init";
    const PU: &str = "crypto_onetimeauth_poly1305_update";
    const PF: &str = "crypto_onetimeauth_poly1305_final";

    for &len in &[0usize, 1, 15, 16, 17, 31, 32, 33, 64, 100, 257] {
        let key = rng.bytes(KEY);
        let msg = rng.bytes(len);
        let reference = oneshot(&msg, &key, &format!("crossmix ref len={len}"));

        // all 8 combinations of generic / primitive for init, update, final
        for i in [GI, PI] {
            for u in [GU, PU] {
                for f in [GF, PF] {
                    // single update
                    let r1 = stream_with(i, u, f, &key, &[&msg[..]], &format!("mix {i}/{u}/{f} len={len}"));
                    eqb(&format!("crossmix {i}/{u}/{f} single len={len}"), &reference, &r1.tag);

                    // multi update, block-straddling
                    let cuts = boundary_cuts(&mut rng, len, 3);
                    let r2 = stream_with(
                        i,
                        u,
                        f,
                        &key,
                        &chunks_of(&msg, &cuts),
                        &format!("mix {i}/{u}/{f} len={len} cuts={cuts:?}"),
                    );
                    eqb(&format!("crossmix {i}/{u}/{f} split len={len}"), &reference, &r2.tag);

                    // the intermediate states must be identical to the pure
                    // poly1305 run as well (pure cast-and-delegate wrappers)
                    let pure = stream_with(
                        PI,
                        PU,
                        PF,
                        &key,
                        &chunks_of(&msg, &cuts),
                        &format!("pure len={len} cuts={cuts:?}"),
                    );
                    assert_eq!(r2.states.len(), pure.states.len());
                    for (n, (a, b)) in r2.states.iter().zip(pure.states.iter()).enumerate() {
                        eqb(&format!("crossmix {i}/{u}/{f} state[{n}] len={len}"), b, a);
                    }
                }
            }
        }

        // alternating generic and primitive update calls on one state
        let sb = statebytes();
        let (ci, ri) = both::<Init>(GI);
        let (cug, rug) = both::<Update>(GU);
        let (cup, rup) = both::<Update>(PU);
        let (cf, rf) = both::<Fin>(PF);
        let mut cs = padded(sb);
        let mut rs = padded(sb);
        unsafe {
            eqi("alt init", ci(cs.as_mut_ptr(), key.as_ptr()), ri(rs.as_mut_ptr(), key.as_ptr()));
        }
        eqb("alt: FULL STATE after init", &cs[..sb], &rs[..sb]);
        let mut off = 0usize;
        let mut which = 0;
        while off < len {
            let n = std::cmp::min(len - off, 1 + which % 20);
            let ch = &msg[off..off + n];
            unsafe {
                if which % 2 == 0 {
                    eqi(
                        "alt update generic",
                        cug(cs.as_mut_ptr(), ch.as_ptr(), n as u64),
                        rug(rs.as_mut_ptr(), ch.as_ptr(), n as u64),
                    );
                } else {
                    eqi(
                        "alt update poly1305",
                        cup(cs.as_mut_ptr(), ch.as_ptr(), n as u64),
                        rup(rs.as_mut_ptr(), ch.as_ptr(), n as u64),
                    );
                }
            }
            eqb(&format!("alt: FULL STATE after update[{which}] len={len}"), &cs[..sb], &rs[..sb]);
            off += n;
            which += 1;
        }
        let mut co = padded(TAG);
        let mut ro = padded(TAG);
        unsafe {
            eqi("alt final", cf(cs.as_mut_ptr(), co.as_mut_ptr()), rf(rs.as_mut_ptr(), ro.as_mut_ptr()));
        }
        eqb("alt tag C==R", &co[..TAG], &ro[..TAG]);
        eqb(&format!("alt tag == one-shot (len={len})"), &reference, &co[..TAG]);
        check_pad("alt C out", &co, TAG);
        check_pad("alt R out", &ro, TAG);
    }
}

// ======================================================================
// 4.181 — _crypto_onetimeauth_poly1305_pick_best_implementation
// ======================================================================

#[test]
fn pick_best_implementation_is_a_no_op() {
    let (cp, rp) = both::<PickFn>("_crypto_onetimeauth_poly1305_pick_best_implementation");
    let mut rng = Rng::new(0x4_1313);
    let key = rng.bytes(KEY);
    let msg = rng.bytes(70);

    // baseline before touching anything
    let base = oneshot(&msg, &key, "pick baseline");
    let base_stream = stream(&key, &chunks_of(&msg, &[5, 21]), "pick baseline stream");
    eqb("pick baseline stream == one-shot", &base, &base_stream.tag);

    for round in 0..4 {
        let (rc, rr) = unsafe { (cp(), rp()) };
        eqi("pick_best_implementation rc", rc, rr);
        // With neither HAVE_TI_MODE nor HAVE_EMMINTRIN_H the sse2 block is not
        // compiled, so the function unconditionally re-installs donna and
        // returns 0.
        assert_eq!(rc, 0, "must return 0 (errors_4 4.23)");

        let after = oneshot(&msg, &key, &format!("pick round {round}"));
        eqb("pick_best_implementation must not change the one-shot tag", &base, &after);

        let s = stream(&key, &chunks_of(&msg, &[5, 21]), &format!("pick stream round {round}"));
        eqb("pick_best_implementation must not change the streaming tag", &base, &s.tag);
        assert_eq!(s.states.len(), base_stream.states.len());
        for (i, (a, b)) in s.states.iter().zip(base_stream.states.iter()).enumerate() {
            eqb(&format!("pick round {round}: state[{i}] unchanged"), b, a);
        }
        assert_eq!(verify(&base, &msg, &key, "pick verify"), 0);
    }

    // ...and interleaved *between* init and update of a live state.
    let sb = statebytes();
    let (ci, ri) = both::<Init>("crypto_onetimeauth_poly1305_init");
    let (cu, ru) = both::<Update>("crypto_onetimeauth_poly1305_update");
    let (cf, rf) = both::<Fin>("crypto_onetimeauth_poly1305_final");
    let mut cs = padded(sb);
    let mut rs = padded(sb);
    unsafe {
        eqi("interleave init", ci(cs.as_mut_ptr(), key.as_ptr()), ri(rs.as_mut_ptr(), key.as_ptr()));
        eqi("interleave pick", cp(), rp());
        eqi(
            "interleave update",
            cu(cs.as_mut_ptr(), msg.as_ptr(), 5),
            ru(rs.as_mut_ptr(), msg.as_ptr(), 5),
        );
        eqi("interleave pick 2", cp(), rp());
        eqi(
            "interleave update 2",
            cu(cs.as_mut_ptr(), msg[5..].as_ptr(), (msg.len() - 5) as u64),
            ru(rs.as_mut_ptr(), msg[5..].as_ptr(), (msg.len() - 5) as u64),
        );
    }
    eqb("interleave: FULL STATE", &cs[..sb], &rs[..sb]);
    let mut co = padded(TAG);
    let mut ro = padded(TAG);
    unsafe {
        eqi("interleave final", cf(cs.as_mut_ptr(), co.as_mut_ptr()), rf(rs.as_mut_ptr(), ro.as_mut_ptr()));
    }
    eqb("interleave tag C==R", &co[..TAG], &ro[..TAG]);
    eqb("interleave tag == baseline", &base, &co[..TAG]);
}

// ======================================================================
// broad randomized fuzz
// ======================================================================

#[test]
fn poly1305_randomized_fuzz() {
    let mut rng = Rng::new(0x4_0130_5eed);
    for iter in 0..600 {
        let len = match iter % 8 {
            0 => rng.range(0, 40),
            1 => rng.range(0, 300),
            2 => 16 * rng.range(0, 20),
            3 => 16 * rng.range(0, 20) + 1,
            4 => 16 * rng.range(1, 20) - 1,
            5 => rng.range(1000, 5000),
            _ => rng.range(0, 200),
        };
        let key = match iter % 5 {
            0 => vec![0u8; KEY],
            1 => vec![0xffu8; KEY],
            _ => rng.bytes(KEY),
        };
        let msg = match iter % 4 {
            0 => vec![0u8; len],
            1 => vec![0xffu8; len],
            _ => rng.bytes(len),
        };
        let label = format!("fuzz iter={iter} len={len}");

        let os = oneshot(&msg, &key, &label);
        let n = rng.range(1, 10);
        let cuts = if iter % 2 == 0 {
            random_cuts(&mut rng, len, n)
        } else {
            boundary_cuts(&mut rng, len, n)
        };
        let a = stream(&key, &chunks_of(&msg, &cuts), &format!("{label} cuts={cuts:?}"));
        eqb(&format!("{label}: stream == one-shot"), &os, &a.tag);

        // generic wrappers agree
        let g = oneshot_sym("crypto_onetimeauth", &msg, &key, &label);
        eqb(&format!("{label}: generic == poly1305"), &os, &g);

        assert_eq!(verify(&os, &msg, &key, &label), 0);
        assert_eq!(verify_sym("crypto_onetimeauth_verify", &os, &msg, &key, &label), 0);
        let mut bad = os.clone();
        let bi = rng.below(TAG);
        bad[bi] ^= 1u8 << rng.below(8);
        assert_eq!(verify(&bad, &msg, &key, &label), -1);
        assert_eq!(verify_sym("crypto_onetimeauth_verify", &bad, &msg, &key, &label), -1);
    }
}
