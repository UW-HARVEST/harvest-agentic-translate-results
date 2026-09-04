//! Phase B — CONFIGS.md rows 120–128: the full crypto_secretstream
//! xchacha20poly1305 state machine.
//!
//! Differentially tests the C `.so` against the Rust `.so`, both loaded via
//! `libloading`. Every symbol is fetched from BOTH libraries and driven
//! through the complete init/push/pull/rekey lifecycle.
//!
//! KEY TESTING APPROACH
//! --------------------
//! `crypto_secretstream_xchacha20poly1305_init_push` writes a RANDOM header
//! (via `randombytes_buf`), so its output cannot be compared directly between
//! two independent implementations. Instead we exploit determinism:
//!
//!   * `init_pull(state, header, key)` is a PURE function of (header, key): it
//!     runs HChaCha20 over the header's first 16 bytes, resets the counter and
//!     copies the inonce. No randomness. Therefore, given the SAME header+key,
//!     the C and Rust states are byte-for-byte equivalent.
//!
//!   * From two such equivalent states, every subsequent `_push` MUST produce
//!     identical ciphertext, and every `_pull` MUST produce identical
//!     plaintext/tag — byte-for-byte. This gives us deterministic byte-exact
//!     comparison of `_push` (rows 120–124, 128) without ever depending on the
//!     random header of `init_push`.
//!
//!   * We ALSO verify the real `init_push` path: C-init_push produces a header
//!     and encrypting state; we then feed that header to `init_pull` in BOTH
//!     libs and confirm both decrypt the C ciphertext identically. And we
//!     cross round-trip: C-push -> Rust-pull and Rust-push -> C-pull.
//!
//! Because a "push state" and a "pull state" built from the SAME header+key
//! are identical (the struct only stores k+nonce+pad; direction is not
//! recorded), an init_pull-derived state can legitimately be used to drive
//! `_push`. That is the trick that makes byte-exact push comparison possible.

mod common;
use common::*;

// ---------------------------------------------------------------------------
// C signatures — see
// c_src/libsodium/include/sodium/crypto_secretstream_xchacha20poly1305.h
// Lengths are `unsigned long long` (u64); out-length params are `*mut u64`;
// tag is `unsigned char` (u8). The opaque state is passed as `*mut u8`.
// ---------------------------------------------------------------------------

type SizeFn = unsafe extern "C" fn() -> usize;
type UcharFn = unsafe extern "C" fn() -> u8;
type Keygen = unsafe extern "C" fn(*mut u8);

// int init_push(state*, unsigned char header[HEADERBYTES], const unsigned char k[KEYBYTES])
type InitPush = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
// int init_pull(state*, const unsigned char header[HEADERBYTES], const unsigned char k[KEYBYTES])
type InitPull = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> i32;
// int push(state*, c*, clen_p*, m*, mlen(ull), ad*, adlen(ull), tag(uchar))
type Push = unsafe extern "C" fn(*mut u8, *mut u8, *mut u64, *const u8, u64, *const u8, u64, u8) -> i32;
// int pull(state*, m*, mlen_p*, tag_p*, c*, clen(ull), ad*, adlen(ull))
type Pull = unsafe extern "C" fn(*mut u8, *mut u8, *mut u64, *mut u8, *const u8, u64, *const u8, u64) -> i32;
// void rekey(state*)
type Rekey = unsafe extern "C" fn(*mut u8);

const NS: &str = "crypto_secretstream_xchacha20poly1305";

const HEADERBYTES: usize = 24;
const KEYBYTES: usize = 32;
const ABYTES: usize = 17; // 1 + 16

const TAG_MESSAGE: u8 = 0x00;
const TAG_PUSH: u8 = 0x01;
const TAG_REKEY: u8 = 0x02;
const TAG_FINAL: u8 = 0x03;

// ---------------------------------------------------------------------------
// Thin per-library driver. Holds one library's function pointers + an owned
// state buffer, and exposes the state machine operations.
// ---------------------------------------------------------------------------

struct Lib {
    init_push: libloading::Symbol<'static, InitPush>,
    init_pull: libloading::Symbol<'static, InitPull>,
    push: libloading::Symbol<'static, Push>,
    pull: libloading::Symbol<'static, Pull>,
    rekey: libloading::Symbol<'static, Rekey>,
    state: Vec<u8>,
}

impl Lib {
    fn do_init_push(&mut self, key: &[u8]) -> (i32, [u8; HEADERBYTES]) {
        let mut header = [0u8; HEADERBYTES];
        let sp = self.state.as_mut_ptr();
        let r = unsafe { (self.init_push)(sp, header.as_mut_ptr(), key.as_ptr()) };
        (r, header)
    }

    fn do_init_pull(&mut self, header: &[u8], key: &[u8]) -> i32 {
        let sp = self.state.as_mut_ptr();
        unsafe { (self.init_pull)(sp, header.as_ptr(), key.as_ptr()) }
    }

    /// Push one message; returns (ret, clen, ciphertext).
    fn do_push(&mut self, m: &[u8], ad: Option<&[u8]>, tag: u8) -> (i32, u64, Vec<u8>) {
        let mut out = vec![0u8; m.len() + ABYTES];
        let mut clen: u64 = 0;
        let (adp, adlen) = ad_ptr(ad);
        let sp = self.state.as_mut_ptr();
        let r = unsafe {
            (self.push)(sp, out.as_mut_ptr(), &mut clen, m.as_ptr(), m.len() as u64, adp, adlen, tag)
        };
        (r, clen, out)
    }

    /// Pull one message; returns (ret, mlen, tag, plaintext).
    fn do_pull(&mut self, c: &[u8], ad: Option<&[u8]>) -> (i32, u64, u8, Vec<u8>) {
        let mut m = vec![0u8; c.len().saturating_sub(ABYTES)];
        let mut mlen: u64 = 0;
        let mut tag: u8 = 0xee;
        let (adp, adlen) = ad_ptr(ad);
        let sp = self.state.as_mut_ptr();
        let r = unsafe {
            (self.pull)(sp, m.as_mut_ptr(), &mut mlen, &mut tag, c.as_ptr(), c.len() as u64, adp, adlen)
        };
        (r, mlen, tag, m)
    }

    fn do_rekey(&mut self) {
        let sp = self.state.as_mut_ptr();
        unsafe { (self.rekey)(sp) }
    }
}

fn ad_ptr(ad: Option<&[u8]>) -> (*const u8, u64) {
    match ad {
        None => (core::ptr::null(), 0),
        Some(a) => (a.as_ptr(), a.len() as u64),
    }
}

fn statebytes() -> usize {
    let d = duo();
    let (c, r): (libloading::Symbol<SizeFn>, libloading::Symbol<SizeFn>) =
        d.pair(&format!("{NS}_statebytes"));
    let cv = unsafe { c() };
    let rv = unsafe { r() };
    assert_eq!(cv, rv, "statebytes mismatch");
    cv
}

/// Build a `Lib` driver for the given library handle.
fn make_lib(lib: &'static libloading::Library, sb: usize) -> Lib {
    let sym = |name: &str| {
        let mut z = name.as_bytes().to_vec();
        z.push(0);
        z
    };
    unsafe {
        Lib {
            init_push: lib.get(&sym(&format!("{NS}_init_push"))).unwrap(),
            init_pull: lib.get(&sym(&format!("{NS}_init_pull"))).unwrap(),
            push: lib.get(&sym(&format!("{NS}_push"))).unwrap(),
            pull: lib.get(&sym(&format!("{NS}_pull"))).unwrap(),
            rekey: lib.get(&sym(&format!("{NS}_rekey"))).unwrap(),
            // Extra 64 bytes of guard/slack as mandated (statebytes()+64).
            state: vec![0u8; sb + 64],
        }
    }
}

/// Returns a fresh (c_lib, rust_lib) driver pair, each with its own state buf.
fn libs() -> (Lib, Lib) {
    let d = duo();
    let sb = statebytes();
    (make_lib(&d.c, sb), make_lib(&d.r, sb))
}

// ===========================================================================
// Row 127 — keygen, statebytes and every *bytes / tag_* constant.
// ===========================================================================

#[test]
fn r127_constants_and_keygen() {
    let d = duo();

    // size_t-returning constants.
    for name in ["abytes", "headerbytes", "keybytes", "messagebytes_max", "statebytes"] {
        let (c, r): (libloading::Symbol<SizeFn>, libloading::Symbol<SizeFn>) =
            d.pair(&format!("{NS}_{name}"));
        let cv = unsafe { c() };
        let rv = unsafe { r() };
        assert_eq!(cv, rv, "{NS}_{name}: C={cv} Rust={rv}");
    }

    // Sanity against the header-defined values.
    let (c, r): (libloading::Symbol<SizeFn>, libloading::Symbol<SizeFn>) =
        d.pair(&format!("{NS}_abytes"));
    assert_eq!(unsafe { c() }, ABYTES);
    assert_eq!(unsafe { r() }, ABYTES);
    let (c, r): (libloading::Symbol<SizeFn>, libloading::Symbol<SizeFn>) =
        d.pair(&format!("{NS}_headerbytes"));
    assert_eq!(unsafe { c() }, HEADERBYTES);
    assert_eq!(unsafe { r() }, HEADERBYTES);
    let (c, r): (libloading::Symbol<SizeFn>, libloading::Symbol<SizeFn>) =
        d.pair(&format!("{NS}_keybytes"));
    assert_eq!(unsafe { c() }, KEYBYTES);
    assert_eq!(unsafe { r() }, KEYBYTES);

    // uchar-returning tag accessors.
    for (name, want) in [
        ("tag_message", TAG_MESSAGE),
        ("tag_push", TAG_PUSH),
        ("tag_rekey", TAG_REKEY),
        ("tag_final", TAG_FINAL),
    ] {
        let (c, r): (libloading::Symbol<UcharFn>, libloading::Symbol<UcharFn>) =
            d.pair(&format!("{NS}_{name}"));
        let cv = unsafe { c() };
        let rv = unsafe { r() };
        assert_eq!(cv, rv, "{NS}_{name}: C={cv} Rust={rv}");
        assert_eq!(cv, want, "{NS}_{name} unexpected value");
    }

    // keygen: fills KEYBYTES. We cannot compare C vs Rust byte-for-byte
    // (randomness), so we check length + distinctness.
    let (ckg, rkg): (libloading::Symbol<Keygen>, libloading::Symbol<Keygen>) =
        d.pair(&format!("{NS}_keygen"));
    let mut ck = vec![0u8; KEYBYTES];
    let mut rk = vec![0u8; KEYBYTES];
    unsafe {
        ckg(ck.as_mut_ptr());
        rkg(rk.as_mut_ptr());
    }
    assert!(ck.iter().any(|&b| b != 0), "C keygen produced all-zero key");
    assert!(rk.iter().any(|&b| b != 0), "Rust keygen produced all-zero key");
    let mut ck2 = vec![0u8; KEYBYTES];
    unsafe { ckg(ck2.as_mut_ptr()) };
    assert_ne!(ck, ck2, "C keygen produced identical consecutive keys");
}

// ===========================================================================
// Shared engine: run a scripted stream on BOTH libs using init_pull-derived
// (hence identical) states, comparing every ciphertext / plaintext / tag /
// length / return code byte-for-byte.
// ===========================================================================

struct Step {
    m: Vec<u8>,
    ad: Option<Vec<u8>>,
    tag: u8,
}

/// Run one full stream through both libs from init_pull-equivalent states.
/// `explicit_rekey_after` (if Some(n)) calls `_rekey` on both push-side and
/// both pull-side states after processing message index n.
fn run_stream(seed: u64, steps: &[Step], explicit_rekey_after: Option<usize>) {
    let (mut c_enc, mut r_enc) = libs();
    let (mut c_dec, mut r_dec) = libs();

    let mut rng = Rng::new(seed);
    let key = rng.bytes(KEYBYTES);
    let header = rng.bytes(HEADERBYTES);

    // init_pull is deterministic in (header,key): all four states become
    // byte-for-byte equivalent, so push/pull must agree across libs.
    let rc = c_enc.do_init_pull(&header, &key);
    let rr = r_enc.do_init_pull(&header, &key);
    eq_i32("init_pull(enc)", rc, rr);
    let rc = c_dec.do_init_pull(&header, &key);
    let rr = r_dec.do_init_pull(&header, &key);
    eq_i32("init_pull(dec)", rc, rr);

    for (i, s) in steps.iter().enumerate() {
        let adc = s.ad.as_deref();

        // --- push in both libs, compare ciphertext byte-for-byte ---
        let (rc, clc, cct) = c_enc.do_push(&s.m, adc, s.tag);
        let (rr, clr, rct) = r_enc.do_push(&s.m, adc, s.tag);
        eq_i32(&format!("push[{i}] ret"), rc, rr);
        assert_eq!(clc, clr, "push[{i}] clen: C={clc} Rust={clr}");
        assert_eq!(clc, (s.m.len() + ABYTES) as u64, "push[{i}] clen value");
        eq_bytes(&format!("push[{i}] ciphertext"), &cct, &rct);

        // --- pull in both libs from the SAME ciphertext (C's), compare ---
        let (rc, mlc, tc, mc) = c_dec.do_pull(&cct, adc);
        let (rr, mlr, tr, mr) = r_dec.do_pull(&cct, adc);
        eq_i32(&format!("pull[{i}] ret"), rc, rr);
        assert_eq!(rc, 0, "pull[{i}] should succeed");
        assert_eq!(mlc, mlr, "pull[{i}] mlen: C={mlc} Rust={mlr}");
        assert_eq!(tc, tr, "pull[{i}] tag: C={tc:#x} Rust={tr:#x}");
        assert_eq!(tc, s.tag, "pull[{i}] tag must equal pushed tag");
        eq_bytes(&format!("pull[{i}] plaintext"), &mc, &mr);
        eq_bytes(&format!("pull[{i}] roundtrip vs input C"), &mc, &s.m);
        eq_bytes(&format!("pull[{i}] roundtrip vs input R"), &mr, &s.m);

        // Optional explicit rekey on both sides after message i.
        if explicit_rekey_after == Some(i) {
            c_enc.do_rekey();
            r_enc.do_rekey();
            c_dec.do_rekey();
            r_dec.do_rekey();
        }
    }
}

// ===========================================================================
// Row 120 — MESSAGE(0) only, 1..=8 messages, mlen in {0,1,16,17,64,1000}.
// ===========================================================================

#[test]
fn r120_message_tag_only() {
    let mlens = [0usize, 1, 16, 17, 64, 1000];
    let mut rng = Rng::new(0x1200_0001);
    for nmsg in 1..=8usize {
        for &base_len in &mlens {
            let mut steps = Vec::new();
            for _ in 0..nmsg {
                steps.push(Step { m: rng.bytes(base_len), ad: None, tag: TAG_MESSAGE });
            }
            run_stream(rng.next_u64(), &steps, None);
        }
    }
}

// ===========================================================================
// Row 121 — tag PUSH(1) interleaved among MESSAGE messages.
// ===========================================================================

#[test]
fn r121_push_tag_interleaved() {
    let mut rng = Rng::new(0x1210_0001);
    for trial in 0..40 {
        let nmsg = 2 + rng.below(7); // 2..=8
        let mut steps = Vec::new();
        for j in 0..nmsg {
            let tag = if j % 3 == 1 { TAG_PUSH } else { TAG_MESSAGE };
            let len = [0usize, 1, 16, 17, 64, 1000][rng.below(6)];
            steps.push(Step { m: rng.bytes(len), ad: None, tag });
        }
        run_stream(0x5100_0000 ^ trial as u64, &steps, None);
    }
}

// ===========================================================================
// Row 122 — tag REKEY(2): exercises the implicit-rekey branch inside push/pull
// (the `tag & TAG_REKEY` condition triggers crypto_..._rekey internally).
// ===========================================================================

#[test]
fn r122_rekey_tag_implicit_branch() {
    let mut rng = Rng::new(0x1220_0001);
    for trial in 0..40 {
        let nmsg = 3 + rng.below(6); // 3..=8
        let mut steps = Vec::new();
        for j in 0..nmsg {
            let tag = if j == 1 || j % 4 == 2 { TAG_REKEY } else { TAG_MESSAGE };
            let len = [0usize, 1, 16, 17, 64, 1000][rng.below(6)];
            steps.push(Step { m: rng.bytes(len), ad: None, tag });
        }
        run_stream(0x5200_0000 ^ trial as u64, &steps, None);
    }
}

// ===========================================================================
// Row 123 — tag FINAL(3) as the last message.
// ===========================================================================

#[test]
fn r123_final_tag_last() {
    let mut rng = Rng::new(0x1230_0001);
    for trial in 0..40 {
        let nmsg = 1 + rng.below(8); // 1..=8
        let mut steps = Vec::new();
        for j in 0..nmsg {
            let is_last = j + 1 == nmsg;
            let tag = if is_last { TAG_FINAL } else { TAG_MESSAGE };
            let len = [0usize, 1, 16, 17, 64, 1000][rng.below(6)];
            steps.push(Step { m: rng.bytes(len), ad: None, tag });
        }
        run_stream(0x5300_0000 ^ trial as u64, &steps, None);
    }
}

// ===========================================================================
// Row 124 — per-message additional data varying within one stream:
// ad in {NULL/0, 1, 16, 17, 64} bytes, changing per message.
// ===========================================================================

#[test]
fn r124_varying_additional_data() {
    let ad_lens = [None, Some(0usize), Some(1), Some(16), Some(17), Some(64)];
    let mut rng = Rng::new(0x1240_0001);
    for trial in 0..40 {
        let nmsg = 3 + rng.below(6); // 3..=8
        let mut steps = Vec::new();
        for j in 0..nmsg {
            let ad = match ad_lens[rng.below(ad_lens.len())] {
                None => None,
                Some(n) => Some(rng.bytes(n)),
            };
            let len = [0usize, 1, 16, 17, 64, 1000][rng.below(6)];
            let tag = if j + 1 == nmsg { TAG_FINAL } else { TAG_MESSAGE };
            steps.push(Step { m: rng.bytes(len), ad, tag });
        }
        run_stream(0x5400_0000 ^ trial as u64, &steps, None);
    }
}

// ===========================================================================
// Row 125 — explicit crypto_..._rekey called mid-stream on BOTH sides after
// N messages, then continue the stream.
// ===========================================================================

#[test]
fn r125_explicit_rekey_midstream() {
    let mut rng = Rng::new(0x1250_0001);
    for trial in 0..40 {
        let nmsg = 4 + rng.below(5); // 4..=8
        let rekey_after = rng.below(nmsg.saturating_sub(1)); // ensure messages follow
        let mut steps = Vec::new();
        for j in 0..nmsg {
            let ad = if j % 2 == 0 {
                let n = rng.below(20);
                Some(rng.bytes(n))
            } else {
                None
            };
            let len = [0usize, 1, 16, 17, 64, 1000][rng.below(6)];
            let tag = if j + 1 == nmsg { TAG_FINAL } else { TAG_MESSAGE };
            steps.push(Step { m: rng.bytes(len), ad, tag });
        }
        run_stream(0x5500_0000 ^ trial as u64, &steps, Some(rekey_after));
    }
}

// ===========================================================================
// Row 126 — _pull with mlen_p == NULL and/or tag_p == NULL on a valid stream.
// ===========================================================================

#[test]
fn r126_pull_null_out_params() {
    let mut rng = Rng::new(0x1260_0001);

    // Which out-params to NULL: (mlen_p null?, tag_p null?)
    let variants = [(true, false), (false, true), (true, true)];

    for &(null_mlen, null_tag) in &variants {
        for trial in 0..20 {
            let (mut c_enc, mut r_enc) = libs();
            let (mut c_dec, mut r_dec) = libs();

            let key = rng.bytes(KEYBYTES);
            let header = rng.bytes(HEADERBYTES);
            c_enc.do_init_pull(&header, &key);
            r_enc.do_init_pull(&header, &key);
            c_dec.do_init_pull(&header, &key);
            r_dec.do_init_pull(&header, &key);

            let nmsg = 1 + rng.below(5);
            for j in 0..nmsg {
                let m = { let l = [0usize, 1, 16, 17, 64][rng.below(5)]; rng.bytes(l) };
                let tag = if j + 1 == nmsg { TAG_FINAL } else { TAG_MESSAGE };

                let (_, _, cct) = c_enc.do_push(&m, None, tag);
                let (_, _, rct) = r_enc.do_push(&m, None, tag);
                eq_bytes(&format!("r126 push[{j}] ct"), &cct, &rct);

                let (rc, mc) = pull_nullable(&mut c_dec, &cct, null_mlen, null_tag);
                let (rr, mr) = pull_nullable(&mut r_dec, &cct, null_mlen, null_tag);
                eq_i32("r126 pull ret", rc, rr);
                assert_eq!(rc, 0, "r126 pull should succeed");
                eq_bytes("r126 pull plaintext", &mc, &mr);
                eq_bytes("r126 pull roundtrip", &mc, &m);
            }
            let _ = trial;
        }
    }
}

/// Call `_pull` with mlen_p and/or tag_p as NULL. Returns (ret, plaintext).
fn pull_nullable(lib: &mut Lib, c: &[u8], null_mlen: bool, null_tag: bool) -> (i32, Vec<u8>) {
    let mut m = vec![0u8; c.len().saturating_sub(ABYTES)];
    let mut mlen: u64 = 0;
    let mut tag: u8 = 0;
    let mlen_p = if null_mlen { core::ptr::null_mut() } else { &mut mlen as *mut u64 };
    let tag_p = if null_tag { core::ptr::null_mut() } else { &mut tag as *mut u8 };
    let sp = lib.state.as_mut_ptr();
    let r = unsafe {
        (lib.pull)(sp, m.as_mut_ptr(), mlen_p, tag_p, c.as_ptr(), c.len() as u64, core::ptr::null(), 0)
    };
    (r, m)
}

// ===========================================================================
// Row 128 — implicit counter/nonce-increment branch: drive 300+ sequential
// messages through one stream and compare every ciphertext byte-for-byte.
// ===========================================================================

#[test]
fn r128_long_sequential_stream() {
    let mut rng = Rng::new(0x1280_0001);
    let key = rng.bytes(KEYBYTES);
    let header = rng.bytes(HEADERBYTES);

    let (mut c_enc, mut r_enc) = libs();
    let (mut c_dec, mut r_dec) = libs();
    c_enc.do_init_pull(&header, &key);
    r_enc.do_init_pull(&header, &key);
    c_dec.do_init_pull(&header, &key);
    r_dec.do_init_pull(&header, &key);

    let n = 350usize;
    for i in 0..n {
        let mlen = [0usize, 1, 16, 17, 33, 64, 200][rng.below(7)];
        let m = rng.bytes(mlen);
        let ad = if i % 5 == 0 {
            let n = rng.below(18);
            Some(rng.bytes(n))
        } else {
            None
        };
        // Occasionally use REKEY tag to drive the internal rekey path within
        // the long stream, but keep the final one FINAL.
        let tag = if i + 1 == n {
            TAG_FINAL
        } else if i % 37 == 36 {
            TAG_REKEY
        } else {
            TAG_MESSAGE
        };

        let (rc, clc, cct) = c_enc.do_push(&m, ad.as_deref(), tag);
        let (rr, clr, rct) = r_enc.do_push(&m, ad.as_deref(), tag);
        eq_i32(&format!("r128 push[{i}] ret"), rc, rr);
        assert_eq!(clc, clr, "r128 push[{i}] clen");
        eq_bytes(&format!("r128 push[{i}] ciphertext"), &cct, &rct);

        let (rc, mlc, tc, mc) = c_dec.do_pull(&cct, ad.as_deref());
        let (rr, mlr, tr, mr) = r_dec.do_pull(&cct, ad.as_deref());
        eq_i32(&format!("r128 pull[{i}] ret"), rc, rr);
        assert_eq!(rc, 0, "r128 pull[{i}] should succeed");
        assert_eq!(mlc, mlr, "r128 pull[{i}] mlen");
        assert_eq!(tc, tr, "r128 pull[{i}] tag");
        eq_bytes(&format!("r128 pull[{i}] plaintext"), &mc, &mr);
        eq_bytes(&format!("r128 pull[{i}] roundtrip"), &mc, &m);
    }
}

// ===========================================================================
// Extra: the REAL init_push path. init_push produces a random header; we feed
// that header to init_pull in BOTH libs and confirm both decrypt the C
// ciphertext identically. Also cross round-trip: C-push -> Rust-pull and
// Rust-push -> C-pull.
// ===========================================================================

#[test]
fn init_push_real_path_and_cross_roundtrip() {
    let mut rng = Rng::new(0x1200_9999);

    for trial in 0..30 {
        // --- (a) C init_push -> both init_pull from C header -> decrypt C ct ---
        {
            let key = rng.bytes(KEYBYTES);
            let (mut c_enc, mut r_enc) = libs();
            let (mut c_dec, mut r_dec) = libs();

            // Drive C init_push (random header) AND Rust init_push (its own
            // random header) to exercise both code paths / return codes.
            let (rc, c_header) = c_enc.do_init_push(&key);
            let (rr, _r_header) = r_enc.do_init_push(&key);
            eq_i32("init_push ret", rc, rr);

            // Both decrypt sides init_pull from the C header+key => identical.
            let dc = c_dec.do_init_pull(&c_header, &key);
            let dr = r_dec.do_init_pull(&c_header, &key);
            eq_i32("init_pull from C header", dc, dr);

            let nmsg = 1 + rng.below(6);
            for j in 0..nmsg {
                let m = { let l = [0usize, 1, 16, 17, 64, 1000][rng.below(6)]; rng.bytes(l) };
                let ad = if j % 2 == 0 {
                    let n = rng.below(20);
                    Some(rng.bytes(n))
                } else {
                    None
                };
                let tag = if j + 1 == nmsg { TAG_FINAL } else { TAG_MESSAGE };

                // Encrypt with the C encrypting state (real init_push state).
                let (_, _, cct) = c_enc.do_push(&m, ad.as_deref(), tag);

                // Both libs pull from C ct; must agree and round-trip.
                let (prc, _, tc, mc) = c_dec.do_pull(&cct, ad.as_deref());
                let (prr, _, tr, mr) = r_dec.do_pull(&cct, ad.as_deref());
                eq_i32("cross pull ret", prc, prr);
                assert_eq!(prc, 0, "cross pull should succeed");
                assert_eq!(tc, tr, "cross pull tag");
                assert_eq!(tc, tag, "cross pull tag == pushed");
                eq_bytes("cross pull plaintext", &mc, &mr);
                eq_bytes("cross pull roundtrip", &mc, &m);
            }
        }

        // --- (b) Cross: Rust-push then C-pull, both init_pull from same header ---
        {
            let mut rng2 = Rng::new(0x7777_0000 ^ trial as u64);
            let key = rng2.bytes(KEYBYTES);
            let header = rng2.bytes(HEADERBYTES);

            let (_c_enc, mut r_enc) = libs();
            let (mut c_dec, _r_dec) = libs();
            r_enc.do_init_pull(&header, &key);
            c_dec.do_init_pull(&header, &key);

            let nmsg = 1 + rng2.below(5);
            for j in 0..nmsg {
                let m = { let l = [0usize, 1, 16, 17, 64][rng2.below(5)]; rng2.bytes(l) };
                let tag = if j + 1 == nmsg { TAG_FINAL } else { TAG_MESSAGE };
                let (_, _, rct) = r_enc.do_push(&m, None, tag);
                let (prc, _, tc, mc) = c_dec.do_pull(&rct, None);
                assert_eq!(prc, 0, "Rust-push -> C-pull should succeed");
                assert_eq!(tc, tag, "Rust-push -> C-pull tag");
                eq_bytes("Rust-push -> C-pull roundtrip", &mc, &m);
            }
        }
    }
}
