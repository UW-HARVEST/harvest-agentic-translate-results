//! Area 6 — `crypto_secretstream/xchacha20poly1305`.
//!
//! Every operation compares the FULL opaque state struct
//! (`crypto_secretstream_xchacha20poly1305_statebytes()` bytes) between the C
//! and the Rust library, so nonce/counter/inonce/`_pad` chaining is checked
//! after each push and pull.
//!
//! Covers `configs_6.md` rows 6.103–6.129 and `errors_6.md` rows 6.68–6.81.
#![allow(clippy::too_many_arguments)]

mod common;
use common::*;
use libloading::Symbol;
use std::ffi::c_int;
use std::ptr::{null, null_mut};

type Getter = unsafe extern "C" fn() -> usize;
type TagGetter = unsafe extern "C" fn() -> u8;
type Keygen = unsafe extern "C" fn(*mut u8);
type Init = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;
type InitPull = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int;
type Push = unsafe extern "C" fn(
    *mut u8,   // state
    *mut u8,   // out
    *mut u64,  // outlen_p
    *const u8, // m
    u64,       // mlen
    *const u8, // ad
    u64,       // adlen
    u8,        // tag
) -> c_int;
type Pull = unsafe extern "C" fn(
    *mut u8,   // state
    *mut u8,   // m
    *mut u64,  // mlen_p
    *mut u8,   // tag_p
    *const u8, // in
    u64,       // inlen
    *const u8, // ad
    u64,       // adlen
) -> c_int;
type Rekey = unsafe extern "C" fn(*mut u8);

const P: &str = "crypto_secretstream_xchacha20poly1305";
const HEADER: usize = 24;
const ABYTES: usize = 17;
const KEYBYTES: usize = 32;
const TAG_MESSAGE: u8 = 0x00;
const TAG_PUSH: u8 = 0x01;
const TAG_REKEY: u8 = 0x02;
const TAG_FINAL: u8 = 0x03;
const POISON: u64 = 0xDEAD_BEEF_CAFE_1234;

const MLEN: [usize; 14] = [0, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129];
const ADLEN: [Option<usize>; 9] = [
    None,
    Some(0),
    Some(1),
    Some(15),
    Some(16),
    Some(17),
    Some(31),
    Some(32),
    Some(33),
];

fn poisoned(len: usize) -> Vec<u8> {
    let mut v = padded(len);
    for b in v[..len].iter_mut() {
        *b = 0xDD;
    }
    v
}

struct Ss {
    sb: usize,
    init_push: (Symbol<'static, Init>, Symbol<'static, Init>),
    init_pull: (Symbol<'static, InitPull>, Symbol<'static, InitPull>),
    push: (Symbol<'static, Push>, Symbol<'static, Push>),
    pull: (Symbol<'static, Pull>, Symbol<'static, Pull>),
    rekey: (Symbol<'static, Rekey>, Symbol<'static, Rekey>),
}

fn ss() -> Ss {
    let (sbc, sbr) = both::<Getter>(&format!("{P}_statebytes"));
    let sb = unsafe { sbc() };
    assert_eq!(sb, unsafe { sbr() }, "statebytes mismatch");
    Ss {
        sb,
        init_push: both::<Init>(&format!("{P}_init_push")),
        init_pull: both::<InitPull>(&format!("{P}_init_pull")),
        push: both::<Push>(&format!("{P}_push")),
        pull: both::<Pull>(&format!("{P}_pull")),
        rekey: both::<Rekey>(&format!("{P}_rekey")),
    }
}

/// A (C-state, Rust-state) pair kept in lockstep.
struct St {
    sb: usize,
    c: Vec<u8>,
    r: Vec<u8>,
}

impl St {
    fn new(sb: usize) -> St {
        St {
            sb,
            c: poisoned(sb),
            r: poisoned(sb),
        }
    }
    #[track_caller]
    fn agree(&self, label: &str) {
        eqb(&format!("{label}: state bytes"), &self.c, &self.r);
        check_pad(&format!("{label}: state (C)"), &self.c, self.sb);
        check_pad(&format!("{label}: state (Rust)"), &self.r, self.sb);
    }
    fn snapshot(&self) -> (Vec<u8>, Vec<u8>) {
        (self.c.clone(), self.r.clone())
    }
    fn counter(&self) -> [u8; 4] {
        [self.c[32], self.c[33], self.c[34], self.c[35]]
    }
    fn key(&self) -> Vec<u8> {
        self.c[..32].to_vec()
    }
    fn inonce(&self) -> Vec<u8> {
        self.c[36..44].to_vec()
    }
}

fn ad_ptr(ad: Option<&[u8]>) -> (*const u8, u64) {
    match ad {
        None => (null(), 0),
        Some(s) => (s.as_ptr(), s.len() as u64),
    }
}

// ------------------------------------------------------------------ wrappers

/// The deterministic RNG installed into both libraries is process-global, so
/// every `rng_reseed()` + pair-of-calls sequence must be atomic with respect to
/// the other test threads in this binary.
static RNG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn init_push(s: &Ss, st: &mut St, k: &[u8], seed: u64, label: &str) -> Vec<u8> {
    let _g = RNG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    rng_reseed(seed);
    let mut hc = padded(HEADER);
    let mut hr = padded(HEADER);
    let rc = unsafe { (s.init_push.0)(st.c.as_mut_ptr(), hc.as_mut_ptr(), k.as_ptr()) };
    let rr = unsafe { (s.init_push.1)(st.r.as_mut_ptr(), hr.as_mut_ptr(), k.as_ptr()) };
    eqi(&format!("{label}: init_push ret"), rc, rr);
    assert_eq!(rc, 0, "{label}: init_push must return 0");
    eqb(&format!("{label}: header"), &hc, &hr);
    check_pad(&format!("{label}: header (C)"), &hc, HEADER);
    check_pad(&format!("{label}: header (Rust)"), &hr, HEADER);
    st.agree(&format!("{label} after init_push"));
    hc.truncate(HEADER);
    hc
}

fn init_pull(s: &Ss, st: &mut St, header: &[u8], k: &[u8], label: &str) {
    let rc = unsafe { (s.init_pull.0)(st.c.as_mut_ptr(), header.as_ptr(), k.as_ptr()) };
    let rr = unsafe { (s.init_pull.1)(st.r.as_mut_ptr(), header.as_ptr(), k.as_ptr()) };
    eqi(&format!("{label}: init_pull ret"), rc, rr);
    // errors_6.md 6.69: never validates anything, always 0.
    assert_eq!(rc, 0, "{label}: init_pull must return 0");
    st.agree(&format!("{label} after init_pull"));
}

fn rekey(s: &Ss, st: &mut St, label: &str) {
    unsafe {
        (s.rekey.0)(st.c.as_mut_ptr());
        (s.rekey.1)(st.r.as_mut_ptr());
    }
    st.agree(&format!("{label} after rekey"));
}

fn push(
    s: &Ss,
    st: &mut St,
    m: &[u8],
    mlen: usize,
    ad: Option<&[u8]>,
    tag: u8,
    outlen_ptr: bool,
    label: &str,
) -> Vec<u8> {
    let (adp, adl) = ad_ptr(ad);
    let mut oc = padded(mlen + ABYTES);
    let mut or = padded(mlen + ABYTES);
    let mut lc = POISON;
    let mut lr = POISON;
    let (pc, pr) = if outlen_ptr {
        (&mut lc as *mut u64, &mut lr as *mut u64)
    } else {
        (null_mut(), null_mut())
    };
    let rc = unsafe {
        (s.push.0)(
            st.c.as_mut_ptr(),
            oc.as_mut_ptr(),
            pc,
            m.as_ptr(),
            mlen as u64,
            adp,
            adl,
            tag,
        )
    };
    let rr = unsafe {
        (s.push.1)(
            st.r.as_mut_ptr(),
            or.as_mut_ptr(),
            pr,
            m.as_ptr(),
            mlen as u64,
            adp,
            adl,
            tag,
        )
    };
    eqi(&format!("{label}: push ret"), rc, rr);
    assert_eq!(rc, 0, "{label}: push must return 0");
    eqb(&format!("{label}: push out"), &oc, &or);
    check_pad(&format!("{label}: push out (C)"), &oc, mlen + ABYTES);
    check_pad(&format!("{label}: push out (Rust)"), &or, mlen + ABYTES);
    if outlen_ptr {
        assert_eq!(lc, (mlen + ABYTES) as u64, "{label}: C *outlen_p");
        assert_eq!(lr, lc, "{label}: *outlen_p mismatch");
    } else {
        assert_eq!(lc, POISON);
        assert_eq!(lr, POISON);
    }
    st.agree(&format!("{label} after push"));
    oc.truncate(mlen + ABYTES);
    oc
}

/// Returns `(plaintext, tag)` for a successful pull.
fn pull(
    s: &Ss,
    st: &mut St,
    inbuf: &[u8],
    ad: Option<&[u8]>,
    mlen_ptr: bool,
    tag_ptr: bool,
    expect: c_int,
    label: &str,
) -> (Vec<u8>, u8) {
    let (adp, adl) = ad_ptr(ad);
    let inlen = inbuf.len();
    let mcap = if inlen >= ABYTES { inlen - ABYTES } else { 64 };
    let mut mc = poisoned(mcap);
    let mut mr = poisoned(mcap);
    let mut lc = POISON;
    let mut lr = POISON;
    let mut tc = 0x7Au8;
    let mut tr = 0x7Au8;
    let (plc, plr) = if mlen_ptr {
        (&mut lc as *mut u64, &mut lr as *mut u64)
    } else {
        (null_mut(), null_mut())
    };
    let (ptc, ptr_) = if tag_ptr {
        (&mut tc as *mut u8, &mut tr as *mut u8)
    } else {
        (null_mut(), null_mut())
    };
    let before = st.snapshot();
    let rc = unsafe {
        (s.pull.0)(
            st.c.as_mut_ptr(),
            mc.as_mut_ptr(),
            plc,
            ptc,
            inbuf.as_ptr(),
            inlen as u64,
            adp,
            adl,
        )
    };
    let rr = unsafe {
        (s.pull.1)(
            st.r.as_mut_ptr(),
            mr.as_mut_ptr(),
            plr,
            ptr_,
            inbuf.as_ptr(),
            inlen as u64,
            adp,
            adl,
        )
    };
    eqi(&format!("{label}: pull ret"), rc, rr);
    assert_eq!(rc, expect, "{label}: C pull return");
    eqb(&format!("{label}: pull m"), &mc, &mr);
    check_pad(&format!("{label}: pull m (C)"), &mc, mcap);
    check_pad(&format!("{label}: pull m (Rust)"), &mr, mcap);
    st.agree(&format!("{label} after pull"));
    if mlen_ptr {
        let want = if rc == 0 { (inlen - ABYTES) as u64 } else { 0 };
        assert_eq!(lc, want, "{label}: C *mlen_p");
        assert_eq!(lr, lc, "{label}: *mlen_p mismatch");
    } else {
        assert_eq!(lc, POISON);
        assert_eq!(lr, POISON);
    }
    if tag_ptr {
        assert_eq!(tc, tr, "{label}: *tag_p mismatch (C {tc}, Rust {tr})");
        if rc != 0 {
            // errors_6.md 6.74/6.75: `*tag_p` stays at the 0xff pre-store.
            assert_eq!(tc, 0xff, "{label}: *tag_p on failure");
        }
    } else {
        assert_eq!(tc, 0x7A);
        assert_eq!(tr, 0x7A);
    }
    if rc != 0 {
        // `_pull` does NOT zero `m` (unlike the AEADs) and does NOT advance
        // the state.
        assert!(
            mc[..mcap].iter().all(|b| *b == 0xDD),
            "{label}: m touched on failure (C)"
        );
        assert!(
            mr[..mcap].iter().all(|b| *b == 0xDD),
            "{label}: m touched on failure (Rust)"
        );
        assert_eq!(st.c, before.0, "{label}: C state advanced on failure");
        assert_eq!(st.r, before.1, "{label}: Rust state advanced on failure");
    }
    mc.truncate(mcap);
    (mc, tc)
}

// ============================================================ 6.103 / 6.104 / 6.105

#[test]
fn constant_getters() {
    let want_mbm: usize = {
        let a = usize::MAX - 17;
        let b = (64u64 * ((1u64 << 32) - 2)) as usize;
        if a < b {
            a
        } else {
            b
        }
    };
    for (suffix, want) in [
        ("_keybytes", KEYBYTES),
        ("_headerbytes", HEADER),
        ("_abytes", ABYTES),
        ("_messagebytes_max", want_mbm),
    ] {
        let (c, r) = both::<Getter>(&format!("{P}{suffix}"));
        unsafe {
            let cv = c();
            let rv = r();
            assert_eq!(cv, want, "C {P}{suffix}");
            assert_eq!(rv, cv, "Rust {P}{suffix}");
        }
    }
    let (c, r) = both::<Getter>(&format!("{P}_statebytes"));
    unsafe {
        let cv = c();
        let rv = r();
        assert_eq!(cv, rv, "statebytes mismatch (C {cv}, Rust {rv})");
        // sizeof(state) == k[32] + nonce[12] + _pad[8]
        assert_eq!(cv, 52, "statebytes must be sizeof(state) == 52");
    }
    // tag getters
    let mut tags = Vec::new();
    for (suffix, want) in [
        ("_tag_message", TAG_MESSAGE),
        ("_tag_push", TAG_PUSH),
        ("_tag_rekey", TAG_REKEY),
        ("_tag_final", TAG_FINAL),
    ] {
        let (c, r) = both::<TagGetter>(&format!("{P}{suffix}"));
        unsafe {
            let cv = c();
            let rv = r();
            assert_eq!(cv, want, "C {P}{suffix}");
            assert_eq!(rv, cv, "Rust {P}{suffix}");
        }
        tags.push(want);
    }
    assert_eq!(tags[3], tags[1] | tags[2], "TAG_FINAL != TAG_PUSH | TAG_REKEY");
}

#[test]
fn keygen() {
    let (c, r) = both::<Keygen>(&format!("{P}_keygen"));
    let _g = RNG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    for seed in [5u64, 55, 0xC0DE] {
        rng_reseed(seed);
        let mut a = padded(KEYBYTES);
        let mut b = padded(KEYBYTES);
        unsafe {
            c(a.as_mut_ptr());
            r(b.as_mut_ptr());
        }
        eqb("secretstream keygen", &a, &b);
        check_pad("secretstream keygen (C)", &a, KEYBYTES);
        check_pad("secretstream keygen (Rust)", &b, KEYBYTES);
        assert!(a[..KEYBYTES].iter().any(|x| *x != 0));
        let mut a2 = padded(KEYBYTES);
        let mut b2 = padded(KEYBYTES);
        unsafe {
            c(a2.as_mut_ptr());
            r(b2.as_mut_ptr());
        }
        eqb("secretstream keygen 2nd", &a2, &b2);
        assert_ne!(&a[..KEYBYTES], &a2[..KEYBYTES]);
    }
}

// ============================================================ 6.106 / 6.107 / errors 6.68

#[test]
fn init_push_and_init_pull_state_layout() {
    let s = ss();
    let mut rng = Rng::new(0x6106);
    for i in 0..8u64 {
        let k = rng.bytes(KEYBYTES);
        let mut pst = St::new(s.sb);
        let h1 = init_push(&s, &mut pst, &k, 0x1000 + i, &format!("init_push #{i}"));
        // counter reset to {1,0,0,0}
        assert_eq!(pst.counter(), [1, 0, 0, 0], "counter not reset");
        // inonce == header[16..24]
        eqb("inonce == header tail", &h1[16..24], &pst.inonce());
        // _pad zeroed
        assert!(
            pst.c[44..52].iter().all(|b| *b == 0),
            "state->_pad not zeroed (C)"
        );
        assert!(
            pst.r[44..52].iter().all(|b| *b == 0),
            "state->_pad not zeroed (Rust)"
        );
        // derived key must not be the raw key
        assert_ne!(pst.key(), k, "state->k == k (hchacha20 not applied?)");

        // two inits with the same key produce different headers
        let mut pst2 = St::new(s.sb);
        let h2 = init_push(&s, &mut pst2, &k, 0x2000 + i, &format!("init_push2 #{i}"));
        assert_ne!(h1, h2, "two init_push calls produced the same header");

        // init_pull from the pushed header yields an identical state
        let mut qst = St::new(s.sb);
        init_pull(&s, &mut qst, &h1, &k, &format!("init_pull #{i}"));
        eqb("init_pull state == init_push state", &pst.c, &qst.c);
    }
}

// ============================================================ 6.108 / 6.109

#[test]
fn single_frame_sessions() {
    let s = ss();
    let mut rng = Rng::new(0x6108);
    for &mlen in MLEN.iter() {
        let k = rng.bytes(KEYBYTES);
        let m = rng.bytes(mlen + 1);
        let label = format!("single mlen={mlen}");
        // reference run with all pointers present
        let mut pst = St::new(s.sb);
        let h = init_push(&s, &mut pst, &k, 0x3000 + mlen as u64, &label);
        let frame = push(&s, &mut pst, &m, mlen, None, TAG_MESSAGE, true, &label);
        assert_eq!(frame.len(), mlen + ABYTES);
        let mut qst = St::new(s.sb);
        init_pull(&s, &mut qst, &h, &k, &label);
        let (out, tag) = pull(&s, &mut qst, &frame, None, true, true, 0, &label);
        eqb(&format!("{label}: plaintext"), &m[..mlen], &out);
        assert_eq!(tag, TAG_MESSAGE);
        // push and pull states must have advanced identically
        eqb(&format!("{label}: push/pull state sync"), &pst.c, &qst.c);

        // row 6.109: outlen_p NULL on push, and all four combinations of the
        // two pull out-pointers; output bytes must be unchanged.
        for outlen_ptr in [true, false] {
            for mlen_ptr in [true, false] {
                for tag_ptr in [true, false] {
                    let mut p2 = St::new(s.sb);
                    let h2 = init_push(&s, &mut p2, &k, 0x3000 + mlen as u64, &label);
                    eqb(&format!("{label}: header determinism"), &h, &h2);
                    let f2 = push(&s, &mut p2, &m, mlen, None, TAG_MESSAGE, outlen_ptr, &label);
                    eqb(&format!("{label}: frame determinism"), &frame, &f2);
                    let mut q2 = St::new(s.sb);
                    init_pull(&s, &mut q2, &h2, &k, &label);
                    let (o2, _) = pull(&s, &mut q2, &f2, None, mlen_ptr, tag_ptr, 0, &label);
                    eqb(&format!("{label}: plaintext (ptr variants)"), &m[..mlen], &o2);
                    eqb(&format!("{label}: state (ptr variants)"), &qst.c, &q2.c);
                }
            }
        }
    }
}

// ============================================================ 6.110

#[test]
fn multi_frame_all_message_tags() {
    let s = ss();
    let mut rng = Rng::new(0x6110);
    for &nframes in [1usize, 2, 3, 8, 64].iter() {
        let k = rng.bytes(KEYBYTES);
        let label = format!("multi n={nframes}");
        let mut pst = St::new(s.sb);
        let h = init_push(&s, &mut pst, &k, 0x4000 + nframes as u64, &label);
        let mut qst = St::new(s.sb);
        init_pull(&s, &mut qst, &h, &k, &label);
        for i in 0..nframes {
            let mlen = MLEN[i % MLEN.len()];
            let m = rng.bytes(mlen + 1);
            let l = format!("{label} frame={i}");
            let f = push(&s, &mut pst, &m, mlen, None, TAG_MESSAGE, true, &l);
            let (out, tag) = pull(&s, &mut qst, &f, None, true, true, 0, &l);
            eqb(&format!("{l}: plaintext"), &m[..mlen], &out);
            assert_eq!(tag, TAG_MESSAGE);
            eqb(&format!("{l}: state sync"), &pst.c, &qst.c);
        }
    }
}

// ============================================================ 6.111 / 6.112 / 6.113 / 6.114

#[test]
fn tag_sequences() {
    let s = ss();
    let mut rng = Rng::new(0x6114);
    let sequences: Vec<Vec<u8>> = vec![
        vec![TAG_MESSAGE, TAG_PUSH, TAG_MESSAGE],
        vec![TAG_MESSAGE, TAG_REKEY, TAG_MESSAGE],
        vec![TAG_MESSAGE, TAG_MESSAGE, TAG_MESSAGE, TAG_FINAL],
        vec![
            TAG_MESSAGE,
            TAG_PUSH,
            TAG_MESSAGE,
            TAG_REKEY,
            TAG_MESSAGE,
            TAG_PUSH,
            TAG_FINAL,
        ],
    ];
    for (si, seq) in sequences.iter().enumerate() {
        let k = rng.bytes(KEYBYTES);
        let label = format!("seq#{si}");
        let mut pst = St::new(s.sb);
        let h = init_push(&s, &mut pst, &k, 0x5000 + si as u64, &label);
        let mut qst = St::new(s.sb);
        init_pull(&s, &mut qst, &h, &k, &label);
        for (i, &tag) in seq.iter().enumerate() {
            let mlen = MLEN[(i * 3 + 1) % MLEN.len()];
            let m = rng.bytes(mlen + 1);
            let l = format!("{label} frame={i} tag={tag:#02x}");
            let key_before = pst.key();
            let f = push(&s, &mut pst, &m, mlen, None, tag, true, &l);
            let key_after = pst.key();
            if tag & TAG_REKEY != 0 {
                assert_ne!(key_before, key_after, "{l}: implicit rekey did not happen");
                assert_eq!(pst.counter(), [1, 0, 0, 0], "{l}: counter not reset");
            } else {
                assert_eq!(key_before, key_after, "{l}: unexpected rekey");
            }
            let (out, got) = pull(&s, &mut qst, &f, None, true, true, 0, &l);
            eqb(&format!("{l}: plaintext"), &m[..mlen], &out);
            assert_eq!(got, tag, "{l}: tag round trip");
            eqb(&format!("{l}: state sync"), &pst.c, &qst.c);
        }
        // errors_6.md 6.77: `_pull` after TAG_FINAL is not latched, but the
        // implicit rekey means the next frame from a *stale* stream fails.
        if *seq.last().unwrap() == TAG_FINAL {
            let junk = rng.bytes(ABYTES + 5);
            pull(&s, &mut qst, &junk, None, true, true, -1, &label);
        }
    }
}

// ============================================================ 6.115 / 6.116 / 6.117 / errors 6.80

#[test]
fn explicit_rekey() {
    let s = ss();
    let mut rng = Rng::new(0x6115);
    for &count in [0usize, 1, 2, 5].iter() {
        let k = rng.bytes(KEYBYTES);
        let label = format!("rekey count={count}");
        let mut pst = St::new(s.sb);
        let h = init_push(&s, &mut pst, &k, 0x6000 + count as u64, &label);
        let mut qst = St::new(s.sb);
        init_pull(&s, &mut qst, &h, &k, &label);

        let m0 = rng.bytes(33);
        let f0 = push(&s, &mut pst, &m0, 32, None, TAG_MESSAGE, true, &label);
        let (o0, _) = pull(&s, &mut qst, &f0, None, true, true, 0, &label);
        eqb(&format!("{label}: frame0"), &m0[..32], &o0);

        for i in 0..count {
            rekey(&s, &mut pst, &format!("{label} push #{i}"));
            rekey(&s, &mut qst, &format!("{label} pull #{i}"));
            assert_eq!(pst.counter(), [1, 0, 0, 0], "{label}: counter not reset by rekey");
            eqb(&format!("{label}: state sync after rekey"), &pst.c, &qst.c);
        }

        let m1 = rng.bytes(65);
        let f1 = push(&s, &mut pst, &m1, 64, None, TAG_MESSAGE, true, &label);
        let (o1, _) = pull(&s, &mut qst, &f1, None, true, true, 0, &label);
        eqb(&format!("{label}: frame1"), &m1[..64], &o1);
        eqb(&format!("{label}: final state sync"), &pst.c, &qst.c);
    }

    // row 6.117: implicit TAG_REKEY followed by an explicit _rekey on both
    // sides, in the same order.
    {
        let k = rng.bytes(KEYBYTES);
        let label = "implicit+explicit rekey";
        let mut pst = St::new(s.sb);
        let h = init_push(&s, &mut pst, &k, 0x7000, label);
        let mut qst = St::new(s.sb);
        init_pull(&s, &mut qst, &h, &k, label);
        let m = rng.bytes(20);
        let f = push(&s, &mut pst, &m, 19, None, TAG_REKEY, true, label);
        let (o, t) = pull(&s, &mut qst, &f, None, true, true, 0, label);
        eqb("implicit rekey plaintext", &m[..19], &o);
        assert_eq!(t, TAG_REKEY);
        eqb("state sync after implicit rekey", &pst.c, &qst.c);
        rekey(&s, &mut pst, label);
        rekey(&s, &mut qst, label);
        eqb("state sync after explicit rekey", &pst.c, &qst.c);
        let m2 = rng.bytes(8);
        let f2 = push(&s, &mut pst, &m2, 7, None, TAG_MESSAGE, true, label);
        let (o2, _) = pull(&s, &mut qst, &f2, None, true, true, 0, label);
        eqb("post-double-rekey plaintext", &m2[..7], &o2);
    }

    // errors_6.md 6.80: an explicit _rekey on only one side desynchronises the
    // session and every subsequent _pull fails with -1.
    {
        let k = rng.bytes(KEYBYTES);
        let label = "one-sided rekey";
        let mut pst = St::new(s.sb);
        let h = init_push(&s, &mut pst, &k, 0x7100, label);
        let mut qst = St::new(s.sb);
        init_pull(&s, &mut qst, &h, &k, label);
        rekey(&s, &mut pst, label); // only the push side
        let m = rng.bytes(40);
        let f = push(&s, &mut pst, &m, 39, None, TAG_MESSAGE, true, label);
        pull(&s, &mut qst, &f, None, true, true, -1, label);
        // and the pull state is unchanged, so catching up fixes the session
        rekey(&s, &mut qst, label);
        let (o, _) = pull(&s, &mut qst, &f, None, true, true, 0, label);
        eqb("resynced plaintext", &m[..39], &o);
    }
}

// ============================================================ 6.118 / 6.119 / errors 6.81

#[test]
fn ad_handling() {
    let s = ss();
    let mut rng = Rng::new(0x6118);
    for &mlen in [0usize, 1, 15, 16, 17, 63, 64, 65].iter() {
        for &adc in ADLEN.iter() {
            let adlen = adc.unwrap_or(0);
            let k = rng.bytes(KEYBYTES);
            let m = rng.bytes(mlen + 1);
            let adbuf = rng.bytes(adlen + 1);
            let ad: Option<&[u8]> = adc.map(|n| &adbuf[..n]);
            let label = format!("ad mlen={mlen} adc={adc:?}");
            let mut pst = St::new(s.sb);
            let h = init_push(&s, &mut pst, &k, 0x8000 + mlen as u64, &label);
            let f = push(&s, &mut pst, &m, mlen, ad, TAG_MESSAGE, true, &label);
            let mut qst = St::new(s.sb);
            init_pull(&s, &mut qst, &h, &k, &label);
            // an ad mismatch must fail (errors_6.md 6.81) and leave the state
            // untouched, so the correct ad still works afterwards
            if adlen > 0 {
                let mut bad = adbuf.clone();
                bad[0] ^= 0x40;
                pull(
                    &s,
                    &mut qst,
                    &f,
                    Some(&bad[..adlen]),
                    true,
                    true,
                    -1,
                    &format!("{label} bad-ad"),
                );
                // NULL/0 instead of the real ad
                pull(&s, &mut qst, &f, None, true, true, -1, &format!("{label} no-ad"));
            }
            let (out, _) = pull(&s, &mut qst, &f, ad, true, true, 0, &label);
            eqb(&format!("{label}: plaintext"), &m[..mlen], &out);
            // `ad == NULL, adlen == 0` and `ad != NULL, adlen == 0` must be
            // indistinguishable
            if adc == Some(0) {
                let mut p2 = St::new(s.sb);
                let h2 = init_push(&s, &mut p2, &k, 0x8000 + mlen as u64, &label);
                eqb("header determinism", &h, &h2);
                let f2 = push(&s, &mut p2, &m, mlen, None, TAG_MESSAGE, true, &label);
                eqb("NULL-ad vs empty-ad frame", &f, &f2);
            }
        }
    }

    // row 6.119: `ad` varying per frame
    {
        let k = rng.bytes(KEYBYTES);
        let label = "per-frame ad";
        let ad1 = rng.bytes(17);
        let ad2 = rng.bytes(32);
        let ad3 = rng.bytes(1);
        let ads: [Option<&[u8]>; 4] = [None, Some(&ad1[..17]), Some(&ad2[..32]), Some(&ad3[..0])];
        let mut pst = St::new(s.sb);
        let h = init_push(&s, &mut pst, &k, 0x8800, label);
        let mut qst = St::new(s.sb);
        init_pull(&s, &mut qst, &h, &k, label);
        for (i, ad) in ads.iter().enumerate() {
            let mlen = 7 * (i + 1);
            let m = rng.bytes(mlen + 1);
            let l = format!("{label} frame={i}");
            let f = push(&s, &mut pst, &m, mlen, *ad, TAG_MESSAGE, true, &l);
            let (out, _) = pull(&s, &mut qst, &f, *ad, true, true, 0, &l);
            eqb(&format!("{l}: plaintext"), &m[..mlen], &out);
            eqb(&format!("{l}: state sync"), &pst.c, &qst.c);
        }
    }
}

// ============================================================ 6.120 / 6.121

#[test]
fn message_length_sweeps() {
    let s = ss();
    let mut rng = Rng::new(0x6120);
    let lens: Vec<usize> = [0usize, 15, 16, 17, 47, 48, 49, 63, 64, 65]
        .into_iter()
        .chain([4096usize, 65536, 131072, 131073, 262145])
        .chain(0..=300)
        .collect();
    for mlen in lens {
        let k = rng.bytes(KEYBYTES);
        let m = rng.bytes(mlen + 1);
        let label = format!("len mlen={mlen}");
        let mut pst = St::new(s.sb);
        let h = init_push(&s, &mut pst, &k, 0x9000 + mlen as u64, &label);
        let f = push(&s, &mut pst, &m, mlen, None, TAG_MESSAGE, true, &label);
        assert_eq!(f.len(), mlen + ABYTES);
        let mut qst = St::new(s.sb);
        init_pull(&s, &mut qst, &h, &k, &label);
        let (out, tag) = pull(&s, &mut qst, &f, None, true, true, 0, &label);
        eqb(&format!("{label}: plaintext"), &m[..mlen], &out);
        assert_eq!(tag, TAG_MESSAGE);
        eqb(&format!("{label}: state sync"), &pst.c, &qst.c);
    }
}

// ============================================================ 6.122 / errors 6.71

#[test]
fn out_of_range_tag_bytes() {
    let s = ss();
    let mut rng = Rng::new(0x6122);
    for tag in 0u16..=0xff {
        let tag = tag as u8;
        let k = rng.bytes(KEYBYTES);
        let mlen = (tag as usize) % 40;
        let m = rng.bytes(mlen + 1);
        let label = format!("tag={tag:#02x}");
        let mut pst = St::new(s.sb);
        let h = init_push(&s, &mut pst, &k, 0xA000 + tag as u64, &label);
        let mut qst = St::new(s.sb);
        init_pull(&s, &mut qst, &h, &k, &label);
        let key_before = pst.key();
        let f = push(&s, &mut pst, &m, mlen, None, tag, true, &label);
        let key_after = pst.key();
        if tag & TAG_REKEY != 0 {
            assert_ne!(key_before, key_after, "{label}: no implicit rekey on push");
        } else {
            assert_eq!(key_before, key_after, "{label}: unexpected rekey on push");
        }
        let (out, got) = pull(&s, &mut qst, &f, None, true, true, 0, &label);
        eqb(&format!("{label}: plaintext"), &m[..mlen], &out);
        assert_eq!(got, tag, "{label}: tag not round-tripped verbatim");
        eqb(&format!("{label}: state sync (rekey symmetry)"), &pst.c, &qst.c);
        // the stream continues correctly regardless of the bogus tag
        let m2 = rng.bytes(9);
        let f2 = push(&s, &mut pst, &m2, 8, None, TAG_MESSAGE, true, &label);
        let (o2, _) = pull(&s, &mut qst, &f2, None, true, true, 0, &label);
        eqb(&format!("{label}: follow-up frame"), &m2[..8], &o2);
    }
}

// ============================================================ 6.123 / errors 6.69

#[test]
fn init_pull_accepts_any_header() {
    let s = ss();
    let mut rng = Rng::new(0x6123);
    let k = rng.bytes(KEYBYTES);
    let mut pst = St::new(s.sb);
    let good = init_push(&s, &mut pst, &k, 0xB000, "hdr");
    let frame = push(&s, &mut pst, &[0u8; 8], 8, None, TAG_MESSAGE, true, "hdr");

    let mut other = St::new(s.sb);
    let other_hdr = init_push(&s, &mut other, &k, 0xB001, "hdr-other");

    for (what, hdr) in [
        ("all-zero", vec![0u8; HEADER]),
        ("all-0xff", vec![0xffu8; HEADER]),
        ("other-session", other_hdr.clone()),
        ("good", good.clone()),
    ] {
        let mut qst = St::new(s.sb);
        init_pull(&s, &mut qst, &hdr, &k, &format!("init_pull {what}"));
        let expect = if what == "good" { 0 } else { -1 };
        pull(
            &s,
            &mut qst,
            &frame,
            None,
            true,
            true,
            expect,
            &format!("pull with {what} header"),
        );
    }
    // wrong key, right header
    let mut k2 = k.clone();
    k2[0] ^= 0xff;
    let mut qst = St::new(s.sb);
    init_pull(&s, &mut qst, &good, &k2, "wrong key");
    pull(&s, &mut qst, &frame, None, true, true, -1, "pull wrong key");
}

// ============================================================ 6.124

#[test]
fn in_place_push_and_pull() {
    let s = ss();
    let mut rng = Rng::new(0x6124);
    for &mlen in [0usize, 1, 64, 65].iter() {
        let k = rng.bytes(KEYBYTES);
        let m = rng.bytes(mlen + 1);
        let label = format!("in-place mlen={mlen}");

        // reference frame
        let mut pst = St::new(s.sb);
        let h = init_push(&s, &mut pst, &k, 0xC000 + mlen as u64, &label);
        let reference = push(&s, &mut pst, &m, mlen, None, TAG_MESSAGE, true, &label);

        // (a) push with m == out + 1 (the layout the API permits: out[0] is
        //     written before the keystream reads m)
        let mut p2 = St::new(s.sb);
        let h2 = init_push(&s, &mut p2, &k, 0xC000 + mlen as u64, &label);
        eqb(&format!("{label}: header determinism"), &h, &h2);
        let mut oc = padded(mlen + ABYTES);
        let mut or = padded(mlen + ABYTES);
        oc[1..1 + mlen].copy_from_slice(&m[..mlen]);
        or[1..1 + mlen].copy_from_slice(&m[..mlen]);
        let mut lc = 0u64;
        let mut lr = 0u64;
        let rc = unsafe {
            (s.push.0)(
                p2.c.as_mut_ptr(),
                oc.as_mut_ptr(),
                &mut lc,
                oc.as_ptr().add(1),
                mlen as u64,
                null(),
                0,
                TAG_MESSAGE,
            )
        };
        let rr = unsafe {
            (s.push.1)(
                p2.r.as_mut_ptr(),
                or.as_mut_ptr(),
                &mut lr,
                or.as_ptr().add(1),
                mlen as u64,
                null(),
                0,
                TAG_MESSAGE,
            )
        };
        eqi(&format!("{label}: in-place push ret"), rc, rr);
        assert_eq!(rc, 0);
        assert_eq!(lc, lr);
        eqb(&format!("{label}: in-place push out"), &oc, &or);
        p2.agree(&format!("{label}: in-place push state"));
        eqb(
            &format!("{label}: in-place push == out-of-place"),
            &reference,
            &oc[..mlen + ABYTES],
        );
        check_pad(&format!("{label}: in-place push pad"), &oc, mlen + ABYTES);

        // (b) pull with m == in, and with m == in + 1
        for shift in [0usize, 1] {
            let mut q = St::new(s.sb);
            init_pull(&s, &mut q, &h, &k, &label);
            let mut bc = padded(mlen + ABYTES);
            let mut br = padded(mlen + ABYTES);
            bc[..mlen + ABYTES].copy_from_slice(&reference);
            br[..mlen + ABYTES].copy_from_slice(&reference);
            let mut mc = 0u64;
            let mut mr = 0u64;
            let mut tc = 0u8;
            let mut tr = 0u8;
            let rc = unsafe {
                (s.pull.0)(
                    q.c.as_mut_ptr(),
                    bc.as_mut_ptr().add(shift),
                    &mut mc,
                    &mut tc,
                    bc.as_ptr(),
                    (mlen + ABYTES) as u64,
                    null(),
                    0,
                )
            };
            let rr = unsafe {
                (s.pull.1)(
                    q.r.as_mut_ptr(),
                    br.as_mut_ptr().add(shift),
                    &mut mr,
                    &mut tr,
                    br.as_ptr(),
                    (mlen + ABYTES) as u64,
                    null(),
                    0,
                )
            };
            eqi(&format!("{label}: in-place pull ret shift={shift}"), rc, rr);
            assert_eq!(rc, 0, "{label}: in-place pull shift={shift}");
            assert_eq!(mc, mlen as u64);
            assert_eq!(mr, mc);
            assert_eq!(tc, TAG_MESSAGE);
            assert_eq!(tr, tc);
            eqb(&format!("{label}: in-place pull buf shift={shift}"), &bc, &br);
            q.agree(&format!("{label}: in-place pull state shift={shift}"));
            eqb(
                &format!("{label}: in-place pull plaintext shift={shift}"),
                &m[..mlen],
                &bc[shift..shift + mlen],
            );
            check_pad(
                &format!("{label}: in-place pull pad shift={shift}"),
                &bc,
                mlen + ABYTES,
            );
        }
    }
}

// ============================================================ 6.125

#[test]
fn corner_keys() {
    let s = ss();
    let mut rng = Rng::new(0x6125);
    for k in [vec![0u8; KEYBYTES], vec![0xffu8; KEYBYTES]] {
        for &mlen in [0usize, 1, 64].iter() {
            let m = rng.bytes(mlen + 1);
            let label = format!("corner k={:#02x} mlen={mlen}", k[0]);
            let mut pst = St::new(s.sb);
            let h = init_push(&s, &mut pst, &k, 0xD000 + mlen as u64, &label);
            let f = push(&s, &mut pst, &m, mlen, None, TAG_MESSAGE, true, &label);
            let mut qst = St::new(s.sb);
            init_pull(&s, &mut qst, &h, &k, &label);
            let (out, tag) = pull(&s, &mut qst, &f, None, true, true, 0, &label);
            eqb(&format!("{label}: plaintext"), &m[..mlen], &out);
            assert_eq!(tag, TAG_MESSAGE);
        }
    }
    // corner headers (all-zero / all-0xff) driven through _init_pull on both
    // the "push" and the "pull" side, so the wire format is fully pinned.
    for hdr in [vec![0u8; HEADER], vec![0xffu8; HEADER]] {
        for k in [vec![0u8; KEYBYTES], vec![0x5Au8; KEYBYTES]] {
            let label = format!("corner hdr={:#02x} k={:#02x}", hdr[0], k[0]);
            let mut pst = St::new(s.sb);
            init_pull(&s, &mut pst, &hdr, &k, &label);
            let mut qst = St::new(s.sb);
            init_pull(&s, &mut qst, &hdr, &k, &label);
            for i in 0..4usize {
                let mlen = i * 21;
                let m: Vec<u8> = (0..mlen + 1).map(|j| (j * 13 + i) as u8).collect();
                let f = push(&s, &mut pst, &m, mlen, None, TAG_MESSAGE, true, &label);
                let (out, _) = pull(&s, &mut qst, &f, None, true, true, 0, &label);
                eqb(&format!("{label}: frame {i}"), &m[..mlen], &out);
            }
        }
    }
}

// ============================================================ 6.126

/// Byte-exact wire-format pin: both states are seeded from a hard-coded header
/// and key via `_init_pull` (no randomness at all), then a fixed sequence of
/// `(tag, ad, m)` frames is pushed and pulled.  Any change to the framing,
/// the `_pad` zeroing, the `ic` values or the quirky Poly1305 padding shows up
/// as a byte difference.
#[test]
fn pinned_wire_format() {
    let s = ss();
    let hdr: Vec<u8> = (0..HEADER).map(|i| 0x11u8.wrapping_mul(i as u8 + 1)).collect();
    let k: Vec<u8> = (0..KEYBYTES).map(|i| 0x07u8.wrapping_add(i as u8 * 3)).collect();
    let frames: Vec<(u8, usize, usize)> = vec![
        (TAG_MESSAGE, 0, 0),
        (TAG_MESSAGE, 1, 1),
        (TAG_PUSH, 16, 15),
        (TAG_MESSAGE, 17, 16),
        (TAG_REKEY, 47, 17),
        (TAG_MESSAGE, 48, 0),
        (TAG_MESSAGE, 64, 32),
        (TAG_FINAL, 65, 33),
    ];
    let mut pst = St::new(s.sb);
    init_pull(&s, &mut pst, &hdr, &k, "pinned push-state");
    let mut qst = St::new(s.sb);
    init_pull(&s, &mut qst, &hdr, &k, "pinned pull-state");
    let mut stream: Vec<u8> = Vec::new();
    for (i, &(tag, mlen, adlen)) in frames.iter().enumerate() {
        let m: Vec<u8> = (0..mlen + 1).map(|j| (j * 29 + i * 5) as u8).collect();
        let ad: Vec<u8> = (0..adlen + 1).map(|j| (j * 19 + i) as u8).collect();
        let adr: Option<&[u8]> = if adlen == 0 && i % 2 == 0 {
            None
        } else {
            Some(&ad[..adlen])
        };
        let l = format!("pinned frame {i}");
        let f = push(&s, &mut pst, &m, mlen, adr, tag, true, &l);
        stream.extend_from_slice(&f);
        let (out, got) = pull(&s, &mut qst, &f, adr, true, true, 0, &l);
        eqb(&format!("{l}: plaintext"), &m[..mlen], &out);
        assert_eq!(got, tag);
        eqb(&format!("{l}: state sync"), &pst.c, &qst.c);
    }
    // the concatenated stream length is fully determined
    let total: usize = frames.iter().map(|f| f.1 + ABYTES).sum();
    assert_eq!(stream.len(), total);
}

// ============================================================ 6.127

#[test]
fn statebytes_exact_allocation() {
    let s = ss();
    let mut rng = Rng::new(0x6127);
    // exactly statebytes() bytes at offsets 0..=3 inside a guarded buffer
    for off in 0..4usize {
        let k = rng.bytes(KEYBYTES);
        let label = format!("exact-state off={off}");
        let mut bufc = padded(s.sb + off);
        let mut bufr = padded(s.sb + off);
        for i in 0..off {
            bufc[i] = 0x3C;
            bufr[i] = 0x3C;
        }
        let mut hc = padded(HEADER);
        let mut hr = padded(HEADER);
        {
            let _g = RNG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            rng_reseed(0xE000 + off as u64);
            unsafe {
                assert_eq!(
                    (s.init_push.0)(bufc.as_mut_ptr().add(off), hc.as_mut_ptr(), k.as_ptr()),
                    0
                );
                assert_eq!(
                    (s.init_push.1)(bufr.as_mut_ptr().add(off), hr.as_mut_ptr(), k.as_ptr()),
                    0
                );
            }
        }
        eqb(&format!("{label}: header"), &hc, &hr);
        // run a small session entirely on the offset states
        let mut qc = padded(s.sb + off);
        let mut qr = padded(s.sb + off);
        for i in 0..off {
            qc[i] = 0x3C;
            qr[i] = 0x3C;
        }
        unsafe {
            assert_eq!(
                (s.init_pull.0)(qc.as_mut_ptr().add(off), hc.as_ptr(), k.as_ptr()),
                0
            );
            assert_eq!(
                (s.init_pull.1)(qr.as_mut_ptr().add(off), hr.as_ptr(), k.as_ptr()),
                0
            );
        }
        for frame in 0..4usize {
            let mlen = frame * 37;
            let m = rng.bytes(mlen + 1);
            let mut fc = padded(mlen + ABYTES);
            let mut fr = padded(mlen + ABYTES);
            let mut lc = 0u64;
            let mut lr = 0u64;
            unsafe {
                assert_eq!(
                    (s.push.0)(
                        bufc.as_mut_ptr().add(off),
                        fc.as_mut_ptr(),
                        &mut lc,
                        m.as_ptr(),
                        mlen as u64,
                        null(),
                        0,
                        TAG_MESSAGE
                    ),
                    0
                );
                assert_eq!(
                    (s.push.1)(
                        bufr.as_mut_ptr().add(off),
                        fr.as_mut_ptr(),
                        &mut lr,
                        m.as_ptr(),
                        mlen as u64,
                        null(),
                        0,
                        TAG_MESSAGE
                    ),
                    0
                );
            }
            eqb(&format!("{label}: frame {frame}"), &fc, &fr);
            let mut oc = padded(mlen);
            let mut or = padded(mlen);
            let mut mc = 0u64;
            let mut mr = 0u64;
            let mut tc = 0u8;
            let mut tr = 0u8;
            unsafe {
                assert_eq!(
                    (s.pull.0)(
                        qc.as_mut_ptr().add(off),
                        oc.as_mut_ptr(),
                        &mut mc,
                        &mut tc,
                        fc.as_ptr(),
                        (mlen + ABYTES) as u64,
                        null(),
                        0
                    ),
                    0
                );
                assert_eq!(
                    (s.pull.1)(
                        qr.as_mut_ptr().add(off),
                        or.as_mut_ptr(),
                        &mut mr,
                        &mut tr,
                        fr.as_ptr(),
                        (mlen + ABYTES) as u64,
                        null(),
                        0
                    ),
                    0
                );
            }
            eqb(&format!("{label}: plaintext {frame}"), &oc, &or);
            eqb(&format!("{label}: plaintext bytes"), &m[..mlen], &oc[..mlen]);
            assert_eq!((mc, tc), (mlen as u64, TAG_MESSAGE));
            assert_eq!((mr, tr), (mc, tc));
        }
        // no over-read/over-write past exactly statebytes()
        check_pad(&format!("{label}: push state (C)"), &bufc, s.sb + off);
        check_pad(&format!("{label}: push state (Rust)"), &bufr, s.sb + off);
        check_pad(&format!("{label}: pull state (C)"), &qc, s.sb + off);
        check_pad(&format!("{label}: pull state (Rust)"), &qr, s.sb + off);
        for i in 0..off {
            assert_eq!(bufc[i], 0x3C, "{label}: underflow write (C)");
            assert_eq!(bufr[i], 0x3C, "{label}: underflow write (Rust)");
            assert_eq!(qc[i], 0x3C, "{label}: underflow write (C, pull)");
            assert_eq!(qr[i], 0x3C, "{label}: underflow write (Rust, pull)");
        }
        eqb(&format!("{label}: final push state"), &bufc, &bufr);
        eqb(&format!("{label}: final pull state"), &qc, &qr);
    }
}

// ============================================================ 6.128 / errors 6.72

/// White-box: force `STATE_COUNTER` to `0xffffffff` so that `sodium_increment`
/// wraps it to zero and `sodium_is_zero()` triggers an implicit rekey without
/// any `TAG_REKEY`.
#[test]
fn counter_wrap_triggers_implicit_rekey() {
    let s = ss();
    let mut rng = Rng::new(0x6128);
    for &mlen in [0usize, 1, 33, 64].iter() {
        let k = rng.bytes(KEYBYTES);
        let m = rng.bytes(mlen + 1);
        let label = format!("counter-wrap mlen={mlen}");
        let mut pst = St::new(s.sb);
        let h = init_push(&s, &mut pst, &k, 0xF000 + mlen as u64, &label);
        let mut qst = St::new(s.sb);
        init_pull(&s, &mut qst, &h, &k, &label);
        for st in [&mut pst, &mut qst] {
            for i in 32..36 {
                st.c[i] = 0xff;
                st.r[i] = 0xff;
            }
        }
        pst.agree(&format!("{label}: doctored push state"));
        qst.agree(&format!("{label}: doctored pull state"));
        let key_before = pst.key();
        let f = push(&s, &mut pst, &m, mlen, None, TAG_MESSAGE, true, &label);
        assert_ne!(
            key_before,
            pst.key(),
            "{label}: counter wrap did not rekey the push state"
        );
        assert_eq!(
            pst.counter(),
            [1, 0, 0, 0],
            "{label}: counter not reset after the wrap rekey"
        );
        let (out, tag) = pull(&s, &mut qst, &f, None, true, true, 0, &label);
        eqb(&format!("{label}: plaintext"), &m[..mlen], &out);
        assert_eq!(tag, TAG_MESSAGE);
        eqb(&format!("{label}: state sync after wrap"), &pst.c, &qst.c);
        // the session continues
        let m2 = rng.bytes(20);
        let f2 = push(&s, &mut pst, &m2, 19, None, TAG_MESSAGE, true, &label);
        let (o2, _) = pull(&s, &mut qst, &f2, None, true, true, 0, &label);
        eqb(&format!("{label}: follow-up"), &m2[..19], &o2);
    }
}

// ============================================================ 6.129

#[test]
fn not_interchangeable_with_aead_xchacha20poly1305() {
    type Dec = unsafe extern "C" fn(
        *mut u8,
        *mut u64,
        *mut u8,
        *const u8,
        u64,
        *const u8,
        u64,
        *const u8,
        *const u8,
    ) -> c_int;
    type Enc = unsafe extern "C" fn(
        *mut u8,
        *mut u64,
        *const u8,
        u64,
        *const u8,
        u64,
        *const u8,
        *const u8,
        *const u8,
    ) -> c_int;
    let (adec_c, adec_r) = both::<Dec>("crypto_aead_xchacha20poly1305_ietf_decrypt");
    let (aenc_c, aenc_r) = both::<Enc>("crypto_aead_xchacha20poly1305_ietf_encrypt");
    let s = ss();
    let mut rng = Rng::new(0x6129);
    for &mlen in [0usize, 1, 32, 64].iter() {
        let k = rng.bytes(KEYBYTES);
        let m = rng.bytes(mlen + 1);
        let label = format!("cross-api mlen={mlen}");
        let mut pst = St::new(s.sb);
        let h = init_push(&s, &mut pst, &k, 0x9900 + mlen as u64, &label);
        let f = push(&s, &mut pst, &m, mlen, None, TAG_MESSAGE, true, &label);
        // secretstream frame fed to the AEAD open with the header as the nonce
        let mut oc = poisoned(f.len());
        let mut or = poisoned(f.len());
        let mut lc = POISON;
        let mut lr = POISON;
        let rc = unsafe {
            adec_c(
                oc.as_mut_ptr(),
                &mut lc,
                null_mut(),
                f.as_ptr(),
                f.len() as u64,
                null(),
                0,
                h.as_ptr(),
                k.as_ptr(),
            )
        };
        let rr = unsafe {
            adec_r(
                or.as_mut_ptr(),
                &mut lr,
                null_mut(),
                f.as_ptr(),
                f.len() as u64,
                null(),
                0,
                h.as_ptr(),
                k.as_ptr(),
            )
        };
        eqi(&format!("{label}: aead open of a secretstream frame"), rc, rr);
        assert_eq!(rc, -1, "{label}: the AEAD must reject secretstream framing");

        // and the AEAD's own output must not be pullable
        let mut cc = padded(mlen + 16);
        let mut cr2 = padded(mlen + 16);
        unsafe {
            assert_eq!(
                aenc_c(
                    cc.as_mut_ptr(),
                    null_mut(),
                    m.as_ptr(),
                    mlen as u64,
                    null(),
                    0,
                    null(),
                    h.as_ptr(),
                    k.as_ptr()
                ),
                0
            );
            assert_eq!(
                aenc_r(
                    cr2.as_mut_ptr(),
                    null_mut(),
                    m.as_ptr(),
                    mlen as u64,
                    null(),
                    0,
                    null(),
                    h.as_ptr(),
                    k.as_ptr()
                ),
                0
            );
        }
        eqb(&format!("{label}: aead ct"), &cc, &cr2);
        let mut qst = St::new(s.sb);
        init_pull(&s, &mut qst, &h, &k, &label);
        // the AEAD frame is one byte shorter than a secretstream frame; feed
        // it padded out to ABYTES + mlen so the length check is not what fails
        let mut wire = cc[..mlen + 16].to_vec();
        wire.push(0);
        pull(&s, &mut qst, &wire, None, true, true, -1, &label);
    }
}

// ============================================================ errors 6.74 / 6.78

#[test]
fn pull_short_input() {
    let s = ss();
    let mut rng = Rng::new(0x6E74);
    let k = rng.bytes(KEYBYTES);
    let mut pst = St::new(s.sb);
    let h = init_push(&s, &mut pst, &k, 0xAB00, "short");
    let m = rng.bytes(9);
    let good = push(&s, &mut pst, &m, 8, None, TAG_MESSAGE, true, "short");

    for inlen in 0..ABYTES {
        for mlen_ptr in [true, false] {
            for tag_ptr in [true, false] {
                let mut qst = St::new(s.sb);
                init_pull(&s, &mut qst, &h, &k, "short");
                let before = qst.snapshot();
                let data = rng.bytes(inlen.max(1));
                pull(
                    &s,
                    &mut qst,
                    &data[..inlen],
                    None,
                    mlen_ptr,
                    tag_ptr,
                    -1,
                    &format!("short inlen={inlen}"),
                );
                assert_eq!(qst.c, before.0, "state advanced (C)");
                assert_eq!(qst.r, before.1, "state advanced (Rust)");
                // ...and the correct frame still pulls afterwards
                let (out, tag) = pull(&s, &mut qst, &good, None, true, true, 0, "short recover");
                eqb("short recover plaintext", &m[..8], &out);
                assert_eq!(tag, TAG_MESSAGE);
            }
        }
    }
    // inlen == ABYTES exactly: valid empty-message frame
    let empty = {
        let mut p = St::new(s.sb);
        let hh = init_push(&s, &mut p, &k, 0xAB01, "empty");
        let f = push(&s, &mut p, &[0u8; 1], 0, None, TAG_MESSAGE, true, "empty");
        (hh, f)
    };
    assert_eq!(empty.1.len(), ABYTES);
    let mut qst = St::new(s.sb);
    init_pull(&s, &mut qst, &empty.0, &k, "empty");
    pull(&s, &mut qst, &empty.1, None, true, true, 0, "empty");
}

// ============================================================ errors 6.75 / 6.77

#[test]
fn pull_mac_failures_leave_state_intact() {
    let s = ss();
    let mut rng = Rng::new(0x6E75);
    for &mlen in [0usize, 1, 16, 33].iter() {
        let k = rng.bytes(KEYBYTES);
        let m = rng.bytes(mlen + 1);
        let label = format!("mac-fail mlen={mlen}");
        let mut pst = St::new(s.sb);
        let h = init_push(&s, &mut pst, &k, 0xBB00 + mlen as u64, &label);
        let f = push(&s, &mut pst, &m, mlen, None, TAG_MESSAGE, true, &label);
        let mut qst = St::new(s.sb);
        init_pull(&s, &mut qst, &h, &k, &label);
        // every byte position (tag byte, ciphertext, trailing MAC)
        for pos in 0..f.len() {
            let mut bad = f.clone();
            bad[pos] ^= 0x20;
            pull(
                &s,
                &mut qst,
                &bad,
                None,
                true,
                true,
                -1,
                &format!("{label} pos={pos}"),
            );
        }
        // the state was never advanced, so the good frame still works
        let (out, tag) = pull(&s, &mut qst, &f, None, true, true, 0, &label);
        eqb(&format!("{label}: plaintext"), &m[..mlen], &out);
        assert_eq!(tag, TAG_MESSAGE);
        // replaying the same frame now fails (the state has advanced)
        pull(&s, &mut qst, &f, None, true, true, -1, &format!("{label} replay"));
    }
}

// ============================================================ errors 6.79 (mlen == 0 only)

#[test]
fn pull_with_null_m_is_only_safe_for_empty_messages() {
    let s = ss();
    let mut rng = Rng::new(0x6E79);
    let k = rng.bytes(KEYBYTES);
    let mut pst = St::new(s.sb);
    let h = init_push(&s, &mut pst, &k, 0xCC00, "null-m");
    let f = push(&s, &mut pst, &[0u8; 1], 0, None, TAG_MESSAGE, true, "null-m");
    let mut qst = St::new(s.sb);
    init_pull(&s, &mut qst, &h, &k, "null-m");
    let mut ml = POISON;
    let mut tg = 0u8;
    let mut ml2 = POISON;
    let mut tg2 = 0u8;
    let rc = unsafe {
        (s.pull.0)(
            qst.c.as_mut_ptr(),
            null_mut(),
            &mut ml,
            &mut tg,
            f.as_ptr(),
            f.len() as u64,
            null(),
            0,
        )
    };
    let rr = unsafe {
        (s.pull.1)(
            qst.r.as_mut_ptr(),
            null_mut(),
            &mut ml2,
            &mut tg2,
            f.as_ptr(),
            f.len() as u64,
            null(),
            0,
        )
    };
    eqi("pull m == NULL, mlen == 0", rc, rr);
    assert_eq!(rc, 0);
    assert_eq!((ml, tg), (0, TAG_MESSAGE));
    assert_eq!((ml2, tg2), (ml, tg));
    qst.agree("pull m == NULL state");
    // `m == NULL` with `mlen > 0` is undefined behaviour in C (unconditional
    // `crypto_stream_chacha20_ietf_xor_ic(m, ...)`), so it is deliberately not
    // exercised here.
}

// ============================================================ errors 6.70 / 6.76

#[test]
fn push_and_pull_messagebytes_max_abort() {
    let s = ss();
    let mbm: u64 = {
        let a = (usize::MAX - 17) as u64;
        let b = 64u64 * ((1u64 << 32) - 2);
        if a < b {
            a
        } else {
            b
        }
    };
    let sb = s.sb;
    // _push with mlen > MESSAGEBYTES_MAX
    let (pc, pr) = (s.push.0.clone(), s.push.1.clone());
    eq_abort(
        "secretstream _push mlen > MESSAGEBYTES_MAX",
        move || unsafe {
            let mut st = vec![0u8; sb];
            let mut out = [0u8; 64];
            let mut ol = 0u64;
            pc(
                st.as_mut_ptr(),
                out.as_mut_ptr(),
                &mut ol,
                out.as_ptr(),
                mbm + 1,
                null(),
                0,
                TAG_MESSAGE,
            );
        },
        move || unsafe {
            let mut st = vec![0u8; sb];
            let mut out = [0u8; 64];
            let mut ol = 0u64;
            pr(
                st.as_mut_ptr(),
                out.as_mut_ptr(),
                &mut ol,
                out.as_ptr(),
                mbm + 1,
                null(),
                0,
                TAG_MESSAGE,
            );
        },
    );
    // _pull with inlen - 17 > MESSAGEBYTES_MAX
    let (lc, lr) = (s.pull.0.clone(), s.pull.1.clone());
    eq_abort(
        "secretstream _pull mlen > MESSAGEBYTES_MAX",
        move || unsafe {
            let mut st = vec![0u8; sb];
            let mut out = [0u8; 64];
            let mut ml = 0u64;
            let mut tg = 0u8;
            lc(
                st.as_mut_ptr(),
                out.as_mut_ptr(),
                &mut ml,
                &mut tg,
                out.as_ptr(),
                mbm + 18,
                null(),
                0,
            );
        },
        move || unsafe {
            let mut st = vec![0u8; sb];
            let mut out = [0u8; 64];
            let mut ml = 0u64;
            let mut tg = 0u8;
            lr(
                st.as_mut_ptr(),
                out.as_mut_ptr(),
                &mut ml,
                &mut tg,
                out.as_ptr(),
                mbm + 18,
                null(),
                0,
            );
        },
    );
}
