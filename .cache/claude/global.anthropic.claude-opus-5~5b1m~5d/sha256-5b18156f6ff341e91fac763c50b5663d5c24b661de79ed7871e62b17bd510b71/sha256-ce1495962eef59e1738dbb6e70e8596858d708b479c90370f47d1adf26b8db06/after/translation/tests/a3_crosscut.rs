//! Area 3 — the four AREA-WIDE configuration rows (configs 3.128 – 3.131),
//! exercised in one place across EVERY area-3 primitive at once:
//!
//! * 3.128 input-content axis (all-zero / all-0xFF / `i & 0xff` / random)
//! * 3.129 state-reuse axis (`init`→`update`→`final`→`init`→`update`→`final`)
//! * 3.130 aliased `out`/`in` buffers for every one-shot entry point
//! * 3.131 primitive-vs-generic consistency
mod common;
use common::*;
use std::ffi::{c_char, c_int};

type Hash = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
type Xof = unsafe extern "C" fn(*mut u8, usize, *const u8, u64) -> c_int;
type GHash = unsafe extern "C" fn(*mut u8, usize, *const u8, u64, *const u8, usize) -> c_int;
type SHash = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> c_int;
type StInit = unsafe extern "C" fn(*mut u8) -> c_int;
type StUpd = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
type StFin = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;
type XofSqueeze = unsafe extern "C" fn(*mut u8, *mut u8, usize) -> c_int;
type SizeFn = unsafe extern "C" fn() -> usize;
type NameFn = unsafe extern "C" fn() -> *const c_char;

/// The four content patterns of row 3.128.
fn patterns(len: usize, rng: &mut Rng) -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("zeros", vec![0u8; len]),
        ("ones", vec![0xffu8; len]),
        ("ramp", (0..len).map(|i| (i & 0xff) as u8).collect()),
        ("random", rng.bytes(len)),
    ]
}

const LENS: &[usize] = &[
    0, 1, 2, 7, 8, 63, 64, 65, 71, 72, 73, 103, 104, 105, 127, 128, 129, 135, 136, 137, 143, 144,
    145, 167, 168, 169, 255, 256, 257, 1000,
];

// ------------------------------------------- 3.128 / 3.130 for fixed-size hashes

/// `(one-shot symbol, digest size)` for every fixed-output area-3 hash.
fn fixed_hashes() -> Vec<(&'static str, usize)> {
    vec![
        ("crypto_hash", 64),
        ("crypto_hash_sha256", 32),
        ("crypto_hash_sha512", 64),
        ("crypto_hash_sha3256", 32),
        ("crypto_hash_sha3512", 64),
    ]
}

#[test]
fn r3_128_content_patterns_fixed_hashes() {
    let mut rng = Rng::new(0x3128);
    for (sym, outlen) in fixed_hashes() {
        let (c, r) = both::<Hash>(sym);
        for &len in LENS {
            for (tag, input) in patterns(len, &mut rng) {
                let mut a = padded(outlen);
                let mut b = padded(outlen);
                unsafe {
                    eqi(
                        &format!("{sym}({tag},{len}) ret"),
                        c(a.as_mut_ptr(), input.as_ptr(), len as u64),
                        r(b.as_mut_ptr(), input.as_ptr(), len as u64),
                    );
                }
                eqb(&format!("{sym}({tag},{len})"), &a[..outlen], &b[..outlen]);
                check_pad(sym, &a, outlen);
                check_pad(sym, &b, outlen);
            }
        }
    }
}

#[test]
fn r3_130_aliased_out_and_in_fixed_hashes() {
    // The C consumes all of `in` before writing `out`, so aliasing is defined.
    let mut rng = Rng::new(0x3130);
    for (sym, outlen) in fixed_hashes() {
        let (c, r) = both::<Hash>(sym);
        for &len in LENS {
            for (tag, input) in patterns(len, &mut rng) {
                let cap = len.max(outlen);
                // out == in
                let mut a = padded(cap);
                a[..len].copy_from_slice(&input);
                let mut b = a.clone();
                unsafe {
                    eqi(
                        &format!("{sym} alias0 ({tag},{len})"),
                        c(a.as_mut_ptr(), a.as_ptr(), len as u64),
                        r(b.as_mut_ptr(), b.as_ptr(), len as u64),
                    );
                }
                eqb(&format!("{sym} alias0 ({tag},{len})"), &a, &b);
                check_pad(sym, &a, cap);
                // out == in + k, for a few overlaps
                for k in [1usize, 3, 8] {
                    if len < k {
                        continue;
                    }
                    let mut a = padded(cap + k);
                    a[k..k + len].copy_from_slice(&input);
                    let mut b = a.clone();
                    unsafe {
                        eqi(
                            &format!("{sym} alias{k} ({tag},{len})"),
                            c(a.as_mut_ptr(), a.as_ptr().add(k), len as u64),
                            r(b.as_mut_ptr(), b.as_ptr().add(k), len as u64),
                        );
                    }
                    eqb(&format!("{sym} alias{k} ({tag},{len})"), &a, &b);
                    check_pad(sym, &a, cap + k);
                }
            }
        }
    }
}

// ------------------------------------------------- 3.128 / 3.130 for the XOFs

fn xofs() -> Vec<&'static str> {
    vec![
        "crypto_xof_shake128",
        "crypto_xof_shake256",
        "crypto_xof_turboshake128",
        "crypto_xof_turboshake256",
    ]
}

#[test]
fn r3_128_and_3_130_content_patterns_and_aliasing_xofs() {
    let mut rng = Rng::new(0x3129);
    for sym in xofs() {
        let (c, r) = both::<Xof>(sym);
        for &len in LENS {
            for (tag, input) in patterns(len, &mut rng) {
                for outlen in [0usize, 1, 31, 32, 168, 169, 400] {
                    let mut a = padded(outlen);
                    let mut b = padded(outlen);
                    unsafe {
                        eqi(
                            &format!("{sym}({tag},{len}->{outlen}) ret"),
                            c(a.as_mut_ptr(), outlen, input.as_ptr(), len as u64),
                            r(b.as_mut_ptr(), outlen, input.as_ptr(), len as u64),
                        );
                    }
                    eqb(&format!("{sym}({tag},{len}->{outlen})"), &a[..outlen], &b[..outlen]);
                    check_pad(sym, &a, outlen);
                    check_pad(sym, &b, outlen);
                }
                // aliased out/in
                for outlen in [1usize, 32, 200] {
                    let cap = len.max(outlen);
                    let mut a = padded(cap);
                    a[..len].copy_from_slice(&input);
                    let mut b = a.clone();
                    unsafe {
                        eqi(
                            &format!("{sym} alias ({tag},{len}->{outlen}) ret"),
                            c(a.as_mut_ptr(), outlen, a.as_ptr(), len as u64),
                            r(b.as_mut_ptr(), outlen, b.as_ptr(), len as u64),
                        );
                    }
                    eqb(&format!("{sym} alias ({tag},{len}->{outlen})"), &a, &b);
                    check_pad(sym, &a, cap);
                }
            }
        }
    }
}

// ---------------------------------- 3.128 / 3.130 / 3.131 generichash + shorthash

#[test]
fn r3_128_and_3_130_and_3_131_generichash() {
    let mut rng = Rng::new(0x312A);
    let (cg, rg) = both::<GHash>("crypto_generichash");
    let (cb, rb) = both::<GHash>("crypto_generichash_blake2b");
    for &len in LENS {
        for (tag, input) in patterns(len, &mut rng) {
            for outlen in [16usize, 32, 64] {
                for keylen in [0usize, 16, 32, 64] {
                    let key = rng.bytes(keylen);
                    let kp = if keylen == 0 {
                        std::ptr::null()
                    } else {
                        key.as_ptr()
                    };
                    let mut a = padded(outlen);
                    let mut b = padded(outlen);
                    let mut a2 = padded(outlen);
                    let mut b2 = padded(outlen);
                    unsafe {
                        eqi(
                            "crypto_generichash ret",
                            cg(a.as_mut_ptr(), outlen, input.as_ptr(), len as u64, kp, keylen),
                            rg(b.as_mut_ptr(), outlen, input.as_ptr(), len as u64, kp, keylen),
                        );
                        eqi(
                            "crypto_generichash_blake2b ret",
                            cb(a2.as_mut_ptr(), outlen, input.as_ptr(), len as u64, kp, keylen),
                            rb(b2.as_mut_ptr(), outlen, input.as_ptr(), len as u64, kp, keylen),
                        );
                    }
                    eqb(&format!("generichash({tag},{len},{outlen},{keylen})"), &a, &b);
                    // 3.131: the generic wrapper is the blake2b primitive
                    eqb("crypto_generichash == _blake2b (C)", &a, &a2);
                    eqb("crypto_generichash == _blake2b (Rust)", &b, &b2);
                    check_pad("generichash", &a, outlen);
                    check_pad("generichash", &b, outlen);
                }
            }
            // 3.130: aliased out/in
            let outlen = 32usize;
            let cap = len.max(outlen);
            let mut a = padded(cap);
            a[..len].copy_from_slice(&input);
            let mut b = a.clone();
            unsafe {
                eqi(
                    "generichash alias ret",
                    cg(a.as_mut_ptr(), outlen, a.as_ptr(), len as u64, std::ptr::null(), 0),
                    rg(b.as_mut_ptr(), outlen, b.as_ptr(), len as u64, std::ptr::null(), 0),
                );
            }
            eqb(&format!("generichash alias ({tag},{len})"), &a, &b);
            check_pad("generichash alias", &a, cap);
        }
    }
}

#[test]
fn r3_128_and_3_130_and_3_131_shorthash() {
    let mut rng = Rng::new(0x312B);
    let (cs, rs) = both::<SHash>("crypto_shorthash");
    let (c24, r24) = both::<SHash>("crypto_shorthash_siphash24");
    let (cx, rx) = both::<SHash>("crypto_shorthash_siphashx24");
    for &len in LENS {
        for (tag, input) in patterns(len, &mut rng) {
            for kpat in [vec![0u8; 16], vec![0xffu8; 16], rng.bytes(16)] {
                let mut a = padded(8);
                let mut b = padded(8);
                let mut a2 = padded(8);
                let mut b2 = padded(8);
                let mut ax = padded(16);
                let mut bx = padded(16);
                unsafe {
                    eqi(
                        "crypto_shorthash ret",
                        cs(a.as_mut_ptr(), input.as_ptr(), len as u64, kpat.as_ptr()),
                        rs(b.as_mut_ptr(), input.as_ptr(), len as u64, kpat.as_ptr()),
                    );
                    eqi(
                        "siphash24 ret",
                        c24(a2.as_mut_ptr(), input.as_ptr(), len as u64, kpat.as_ptr()),
                        r24(b2.as_mut_ptr(), input.as_ptr(), len as u64, kpat.as_ptr()),
                    );
                    eqi(
                        "siphashx24 ret",
                        cx(ax.as_mut_ptr(), input.as_ptr(), len as u64, kpat.as_ptr()),
                        rx(bx.as_mut_ptr(), input.as_ptr(), len as u64, kpat.as_ptr()),
                    );
                }
                eqb(&format!("shorthash({tag},{len})"), &a, &b);
                eqb(&format!("siphashx24({tag},{len})"), &ax, &bx);
                // 3.131: the generic wrapper is siphash24
                eqb("crypto_shorthash == _siphash24 (C)", &a, &a2);
                eqb("crypto_shorthash == _siphash24 (Rust)", &b, &b2);
                check_pad("shorthash", &a, 8);
                check_pad("siphashx24", &ax, 16);
            }
            // 3.130: aliased out/in
            let cap = len.max(16);
            let k = rng.bytes(16);
            let mut a = padded(cap);
            a[..len].copy_from_slice(&input);
            let mut b = a.clone();
            unsafe {
                eqi(
                    "shorthash alias ret",
                    cs(a.as_mut_ptr(), a.as_ptr(), len as u64, k.as_ptr()),
                    rs(b.as_mut_ptr(), b.as_ptr(), len as u64, k.as_ptr()),
                );
            }
            eqb(&format!("shorthash alias ({tag},{len})"), &a, &b);
            check_pad("shorthash alias", &a, cap);
        }
    }
}

// ------------------------------------------------------ 3.129 state-reuse axis

/// Raw, over-aligned, guard-padded state slab.
struct State {
    buf: Vec<u64>,
    len: usize,
}
impl State {
    fn new(len: usize) -> Self {
        let words = (len + PAD + 7) / 8;
        let mut buf = vec![0u64; words];
        let bytes = unsafe {
            std::slice::from_raw_parts_mut(buf.as_mut_ptr() as *mut u8, words * 8)
        };
        for (i, b) in bytes[len..].iter_mut().enumerate() {
            *b = 0xA5u8.wrapping_add(i as u8);
        }
        State { buf, len }
    }
    fn ptr(&mut self) -> *mut u8 {
        self.buf.as_mut_ptr() as *mut u8
    }
    fn image(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.buf.as_ptr() as *const u8, self.len) }
    }
    fn guard_ok(&self, what: &str) {
        let all = unsafe {
            std::slice::from_raw_parts(self.buf.as_ptr() as *const u8, self.buf.len() * 8)
        };
        for (i, b) in all[self.len..self.len + PAD].iter().enumerate() {
            assert_eq!(*b, 0xA5u8.wrapping_add(i as u8), "{what}: wrote past statebytes()+{i}");
        }
    }
}

/// `init → update* → final → init → update* → final` on the SAME state object
/// must reproduce the first digest exactly, for both libraries.
#[test]
fn r3_129_state_reuse_fixed_hashes() {
    let mut rng = Rng::new(0x3131);
    for (fam, outlen) in [
        ("crypto_hash_sha256", 32usize),
        ("crypto_hash_sha512", 64),
        ("crypto_hash_sha3256", 32),
        ("crypto_hash_sha3512", 64),
    ] {
        let (csb, rsb) = both::<SizeFn>(&format!("{fam}_statebytes"));
        let (ci, ri) = both::<StInit>(&format!("{fam}_init"));
        let (cu, ru) = both::<StUpd>(&format!("{fam}_update"));
        let (cf, rf) = both::<StFin>(&format!("{fam}_final"));
        let sb = unsafe { csb() };
        assert_eq!(sb, unsafe { rsb() }, "{fam}_statebytes");

        for &len in LENS {
            for (tag, input) in patterns(len, &mut rng) {
                // three randomized chunkings, run twice on the same state
                let mut digests_c: Vec<Vec<u8>> = Vec::new();
                let mut digests_r: Vec<Vec<u8>> = Vec::new();
                let mut sc = State::new(sb);
                let mut sr = State::new(sb);
                for round in 0..3 {
                    unsafe {
                        eqi(&format!("{fam}_init r{round}"), ci(sc.ptr()), ri(sr.ptr()));
                    }
                    eqb(
                        &format!("{fam} state after init r{round}"),
                        sc.image(),
                        sr.image(),
                    );
                    let mut off = 0usize;
                    while off < len {
                        let k = if round == 0 {
                            len - off
                        } else {
                            rng.range(1, len - off).min(len - off)
                        };
                        unsafe {
                            eqi(
                                &format!("{fam}_update r{round}"),
                                cu(sc.ptr(), input.as_ptr().add(off), k as u64),
                                ru(sr.ptr(), input.as_ptr().add(off), k as u64),
                            );
                        }
                        eqb(
                            &format!("{fam} state after update r{round}"),
                            sc.image(),
                            sr.image(),
                        );
                        off += k;
                    }
                    let mut dc = padded(outlen);
                    let mut dr = padded(outlen);
                    unsafe {
                        eqi(
                            &format!("{fam}_final r{round}"),
                            cf(sc.ptr(), dc.as_mut_ptr()),
                            rf(sr.ptr(), dr.as_mut_ptr()),
                        );
                    }
                    eqb(&format!("{fam} digest r{round} ({tag},{len})"), &dc, &dr);
                    eqb(
                        &format!("{fam} state after final r{round}"),
                        sc.image(),
                        sr.image(),
                    );
                    sc.guard_ok(fam);
                    sr.guard_ok(fam);
                    digests_c.push(dc[..outlen].to_vec());
                    digests_r.push(dr[..outlen].to_vec());
                }
                // re-init must fully reset: all three rounds agree
                assert_eq!(
                    digests_c[0], digests_c[1],
                    "{fam}: C re-init did not reset ({tag},{len})"
                );
                assert_eq!(
                    digests_c[1], digests_c[2],
                    "{fam}: C re-init did not reset ({tag},{len})"
                );
                assert_eq!(digests_r[0], digests_r[1], "{fam}: Rust re-init did not reset");
                assert_eq!(digests_r[1], digests_r[2], "{fam}: Rust re-init did not reset");
                // and the streamed digest equals the one-shot
                let (c1, r1) = both::<Hash>(fam);
                let mut oc = padded(outlen);
                let mut or = padded(outlen);
                unsafe {
                    c1(oc.as_mut_ptr(), input.as_ptr(), len as u64);
                    r1(or.as_mut_ptr(), input.as_ptr(), len as u64);
                }
                assert_eq!(&oc[..outlen], &digests_c[0][..], "{fam}: C stream != one-shot");
                assert_eq!(&or[..outlen], &digests_r[0][..], "{fam}: Rust stream != one-shot");
            }
        }
    }
}

/// `init → update* → squeeze* → init → …` on the same XOF state object.
#[test]
fn r3_129_state_reuse_xofs() {
    let mut rng = Rng::new(0x3132);
    for fam in xofs() {
        let (csb, rsb) = both::<SizeFn>(&format!("{fam}_statebytes"));
        let (ci, ri) = both::<StInit>(&format!("{fam}_init"));
        let (cu, ru) = both::<StUpd>(&format!("{fam}_update"));
        let (cq, rq) = both::<XofSqueeze>(&format!("{fam}_squeeze"));
        let sb = unsafe { csb() };
        assert_eq!(sb, unsafe { rsb() });

        for &len in LENS {
            for (tag, input) in patterns(len, &mut rng) {
                let mut outs_c: Vec<Vec<u8>> = Vec::new();
                let mut outs_r: Vec<Vec<u8>> = Vec::new();
                let mut sc = State::new(sb);
                let mut sr = State::new(sb);
                for round in 0..3 {
                    unsafe {
                        eqi(&format!("{fam}_init r{round}"), ci(sc.ptr()), ri(sr.ptr()));
                    }
                    eqb(&format!("{fam} state after init r{round}"), sc.image(), sr.image());
                    let mut off = 0usize;
                    while off < len {
                        let k = if round == 0 { len - off } else { rng.range(1, len - off) };
                        unsafe {
                            eqi(
                                &format!("{fam}_update r{round}"),
                                cu(sc.ptr(), input.as_ptr().add(off), k as u64),
                                ru(sr.ptr(), input.as_ptr().add(off), k as u64),
                            );
                        }
                        eqb(&format!("{fam} state after update r{round}"), sc.image(), sr.image());
                        off += k;
                    }
                    // squeeze 400 bytes, in one call on round 0 and chunked after
                    let total = 400usize;
                    let mut got_c = Vec::new();
                    let mut got_r = Vec::new();
                    let mut left = total;
                    while left > 0 {
                        let k = if round == 0 { left } else { rng.range(1, left.min(200)) };
                        let mut bc = padded(k);
                        let mut br = padded(k);
                        unsafe {
                            eqi(
                                &format!("{fam}_squeeze r{round}"),
                                cq(sc.ptr(), bc.as_mut_ptr(), k),
                                rq(sr.ptr(), br.as_mut_ptr(), k),
                            );
                        }
                        eqb(&format!("{fam} squeeze r{round}"), &bc[..k], &br[..k]);
                        check_pad(fam, &bc, k);
                        eqb(&format!("{fam} state after squeeze r{round}"), sc.image(), sr.image());
                        got_c.extend_from_slice(&bc[..k]);
                        got_r.extend_from_slice(&br[..k]);
                        left -= k;
                    }
                    sc.guard_ok(fam);
                    sr.guard_ok(fam);
                    outs_c.push(got_c);
                    outs_r.push(got_r);
                }
                assert_eq!(outs_c[0], outs_c[1], "{fam}: C re-init after squeeze did not reset ({tag},{len})");
                assert_eq!(outs_c[1], outs_c[2], "{fam}: C re-init after squeeze did not reset");
                assert_eq!(outs_r[0], outs_r[1], "{fam}: Rust re-init after squeeze did not reset");
                assert_eq!(outs_r[1], outs_r[2], "{fam}: Rust re-init after squeeze did not reset");
                // one-shot equals the first `outlen` squeezed bytes
                let (c1, r1) = both::<Xof>(fam);
                for outlen in [1usize, 32, 400] {
                    let mut oc = padded(outlen);
                    let mut or = padded(outlen);
                    unsafe {
                        c1(oc.as_mut_ptr(), outlen, input.as_ptr(), len as u64);
                        r1(or.as_mut_ptr(), outlen, input.as_ptr(), len as u64);
                    }
                    assert_eq!(&oc[..outlen], &outs_c[0][..outlen], "{fam}: C one-shot != squeeze");
                    assert_eq!(&or[..outlen], &outs_r[0][..outlen], "{fam}: Rust one-shot != squeeze");
                }
            }
        }
    }
}

/// blake2b: `_init → _update* → _final → _init → …` on the same state.
#[test]
fn r3_129_state_reuse_generichash() {
    let mut rng = Rng::new(0x3133);
    for fam in ["crypto_generichash", "crypto_generichash_blake2b"] {
        let (csb, rsb) = both::<SizeFn>(&format!("{fam}_statebytes"));
        let (ci, ri) =
            both::<unsafe extern "C" fn(*mut u8, *const u8, usize, usize) -> c_int>(&format!(
                "{fam}_init"
            ));
        let (cu, ru) = both::<StUpd>(&format!("{fam}_update"));
        let (cf, rf) =
            both::<unsafe extern "C" fn(*mut u8, *mut u8, usize) -> c_int>(&format!("{fam}_final"));
        let sb = unsafe { csb() };
        assert_eq!(sb, unsafe { rsb() });

        for &len in LENS {
            for (tag, input) in patterns(len, &mut rng) {
                for (outlen, keylen) in [(16usize, 0usize), (32, 32), (64, 64)] {
                    let key = rng.bytes(keylen);
                    let kp = if keylen == 0 { std::ptr::null() } else { key.as_ptr() };
                    let mut digests_c: Vec<Vec<u8>> = Vec::new();
                    let mut digests_r: Vec<Vec<u8>> = Vec::new();
                    let mut sc = State::new(sb);
                    let mut sr = State::new(sb);
                    for round in 0..3 {
                        unsafe {
                            eqi(
                                &format!("{fam}_init r{round}"),
                                ci(sc.ptr(), kp, keylen, outlen),
                                ri(sr.ptr(), kp, keylen, outlen),
                            );
                        }
                        eqb(&format!("{fam} state after init r{round}"), sc.image(), sr.image());
                        let mut off = 0usize;
                        while off < len {
                            let k = if round == 0 { len - off } else { rng.range(1, len - off) };
                            unsafe {
                                eqi(
                                    &format!("{fam}_update r{round}"),
                                    cu(sc.ptr(), input.as_ptr().add(off), k as u64),
                                    ru(sr.ptr(), input.as_ptr().add(off), k as u64),
                                );
                            }
                            eqb(
                                &format!("{fam} state after update r{round}"),
                                sc.image(),
                                sr.image(),
                            );
                            off += k;
                        }
                        let mut dc = padded(outlen);
                        let mut dr = padded(outlen);
                        unsafe {
                            eqi(
                                &format!("{fam}_final r{round}"),
                                cf(sc.ptr(), dc.as_mut_ptr(), outlen),
                                rf(sr.ptr(), dr.as_mut_ptr(), outlen),
                            );
                        }
                        eqb(&format!("{fam} digest r{round} ({tag},{len})"), &dc, &dr);
                        eqb(&format!("{fam} state after final r{round}"), sc.image(), sr.image());
                        sc.guard_ok(fam);
                        sr.guard_ok(fam);
                        digests_c.push(dc[..outlen].to_vec());
                        digests_r.push(dr[..outlen].to_vec());
                    }
                    assert_eq!(
                        digests_c[0], digests_c[1],
                        "{fam}: C re-init did not reset ({tag},{len},{outlen},{keylen})"
                    );
                    assert_eq!(digests_c[1], digests_c[2], "{fam}: C re-init did not reset");
                    assert_eq!(digests_r[0], digests_r[1], "{fam}: Rust re-init did not reset");
                    assert_eq!(digests_r[1], digests_r[2], "{fam}: Rust re-init did not reset");
                    // stream == one-shot
                    let (c1, r1) = both::<GHash>(fam);
                    let mut oc = padded(outlen);
                    let mut or = padded(outlen);
                    unsafe {
                        c1(oc.as_mut_ptr(), outlen, input.as_ptr(), len as u64, kp, keylen);
                        r1(or.as_mut_ptr(), outlen, input.as_ptr(), len as u64, kp, keylen);
                    }
                    assert_eq!(&oc[..outlen], &digests_c[0][..], "{fam}: C stream != one-shot");
                    assert_eq!(&or[..outlen], &digests_r[0][..], "{fam}: Rust stream != one-shot");
                }
            }
        }
    }
}

// ------------------------------------------------- 3.131 generic-vs-primitive

#[test]
fn r3_131_generic_wrappers_and_primitive_names() {
    // crypto_hash ≡ crypto_hash_sha512, byte for byte, on all four patterns.
    let mut rng = Rng::new(0x3134);
    let (cg, rg) = both::<Hash>("crypto_hash");
    let (cs, rs) = both::<Hash>("crypto_hash_sha512");
    for &len in LENS {
        for (tag, input) in patterns(len, &mut rng) {
            let mut a = padded(64);
            let mut b = padded(64);
            let mut a2 = padded(64);
            let mut b2 = padded(64);
            unsafe {
                cg(a.as_mut_ptr(), input.as_ptr(), len as u64);
                rg(b.as_mut_ptr(), input.as_ptr(), len as u64);
                cs(a2.as_mut_ptr(), input.as_ptr(), len as u64);
                rs(b2.as_mut_ptr(), input.as_ptr(), len as u64);
            }
            eqb(&format!("crypto_hash ({tag},{len})"), &a, &b);
            eqb("crypto_hash == _sha512 (C)", &a, &a2);
            eqb("crypto_hash == _sha512 (Rust)", &b, &b2);
        }
    }
    // primitive names and byte-size accessors
    for (name, want) in [
        ("crypto_hash_primitive", "sha512"),
        ("crypto_generichash_primitive", "blake2b"),
        ("crypto_shorthash_primitive", "siphash24"),
    ] {
        let (c, r) = both::<NameFn>(name);
        unsafe {
            let a = std::ffi::CStr::from_ptr(c());
            let b = std::ffi::CStr::from_ptr(r());
            assert_eq!(a, b, "{name}");
            assert_eq!(a.to_str().unwrap(), want, "{name}");
        }
    }
    for name in [
        "crypto_hash_bytes",
        "crypto_hash_sha256_bytes",
        "crypto_hash_sha512_bytes",
        "crypto_hash_sha3256_bytes",
        "crypto_hash_sha3512_bytes",
        "crypto_hash_sha256_statebytes",
        "crypto_hash_sha512_statebytes",
        "crypto_hash_sha3256_statebytes",
        "crypto_hash_sha3512_statebytes",
        "crypto_generichash_bytes",
        "crypto_generichash_bytes_min",
        "crypto_generichash_bytes_max",
        "crypto_generichash_keybytes",
        "crypto_generichash_keybytes_min",
        "crypto_generichash_keybytes_max",
        "crypto_generichash_statebytes",
        "crypto_generichash_blake2b_bytes",
        "crypto_generichash_blake2b_bytes_min",
        "crypto_generichash_blake2b_bytes_max",
        "crypto_generichash_blake2b_keybytes",
        "crypto_generichash_blake2b_keybytes_min",
        "crypto_generichash_blake2b_keybytes_max",
        "crypto_generichash_blake2b_saltbytes",
        "crypto_generichash_blake2b_personalbytes",
        "crypto_generichash_blake2b_statebytes",
        "crypto_shorthash_bytes",
        "crypto_shorthash_keybytes",
        "crypto_shorthash_siphash24_bytes",
        "crypto_shorthash_siphash24_keybytes",
        "crypto_shorthash_siphashx24_bytes",
        "crypto_shorthash_siphashx24_keybytes",
        "crypto_xof_shake128_blockbytes",
        "crypto_xof_shake256_blockbytes",
        "crypto_xof_turboshake128_blockbytes",
        "crypto_xof_turboshake256_blockbytes",
        "crypto_xof_shake128_statebytes",
        "crypto_xof_shake256_statebytes",
        "crypto_xof_turboshake128_statebytes",
        "crypto_xof_turboshake256_statebytes",
    ] {
        let (c, r) = both::<SizeFn>(name);
        unsafe {
            assert_eq!(c(), r(), "{name}");
        }
    }
    for name in [
        "crypto_xof_shake128_domain_standard",
        "crypto_xof_shake256_domain_standard",
        "crypto_xof_turboshake128_domain_standard",
        "crypto_xof_turboshake256_domain_standard",
    ] {
        let (c, r) = both::<unsafe extern "C" fn() -> u8>(name);
        unsafe {
            assert_eq!(c(), r(), "{name}");
        }
    }
}
