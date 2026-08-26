//! Phase C — error / rejection-path differential tests, one test per row of
//! `ERRORS.md` (E1 … E20).
//!
//! The C library has no error codes at all (see `ERRORS.md` for the grep
//! evidence), so "same rejection" is asserted as: identical `tflac_u32` return
//! value **and** identical post-state of the whole shared arena — or, for the
//! null-pointer rows, death by the identical signal.

mod harness;
use harness::*;

use std::os::unix::process::ExitStatusExt;

// ===========================================================================
// E1 / E2 / E12 / E13 — null pointers (the only signal-valued inputs)
// ===========================================================================

/// Spawn this same test binary to run `helper_null_deref` against one side and
/// return `(signal, exit_code)`.
fn run_null_child(side: &str, which: &str) -> (Option<i32>, Option<i32>) {
    let exe = std::env::current_exe().expect("current_exe");
    let out = std::process::Command::new(exe)
        .args(["--exact", "helper_null_deref", "--ignored", "--test-threads=1"])
        .env("HV_NULL_SIDE", side)
        .env("HV_NULL_FN", which)
        .env("C_SO", c_so_path())
        .env("RUST_SO", rust_so_path())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .expect("spawn child");
    (out.status.signal(), out.status.code())
}

/// E1 (`tflac_pack_u64le(NULL, ..)`), E2 (`tflac_md5_addsample(NULL, ..)`),
/// E12 (`update_md5(NULL, samples)`), E13 (`update_md5(t, NULL)`):
/// both `.so`s must die with the *same* signal, i.e. the Rust must fault the
/// way the C faults rather than panicking/aborting differently.
#[test]
fn err_e1_e2_e12_null_pointers_crash_identically() {
    for which in ["pack", "addsample", "update_t", "update_samples"] {
        let (cs, cc) = run_null_child("C", which);
        let (rs, rc) = run_null_child("RUST", which);
        assert_eq!(
            (cs, cc),
            (rs, rc),
            "null-pointer behaviour differs for {which}: C=(signal={cs:?}, code={cc:?}) \
             RUST=(signal={rs:?}, code={rc:?})"
        );
        // and it must actually be a fault, not a clean exit
        assert_eq!(
            cs,
            Some(libc_sigsegv()),
            "expected SIGSEGV from the C side for {which}, got signal={cs:?} code={cc:?}"
        );
    }
}

fn libc_sigsegv() -> i32 {
    11 // SIGSEGV on Linux
}

/// Child-side worker for the null-pointer rows.  Never run directly.
#[test]
#[ignore = "spawned as a subprocess by err_e1_e2_e12_null_pointers_crash_identically"]
fn helper_null_deref() {
    let side = match std::env::var("HV_NULL_SIDE") {
        Ok(s) => s,
        Err(_) => return, // invoked by a plain `--ignored` run: do nothing
    };
    let which = std::env::var("HV_NULL_FN").unwrap();
    let l = libs();
    let s = if side == "C" { &l.c } else { &l.r };
    unsafe {
        match which.as_str() {
            "pack" => (s.pack)(std::ptr::null_mut(), 0x0123_4567_89AB_CDEF),
            "addsample" => (s.add)(std::ptr::null_mut(), 64, 0x0123_4567_89AB_CDEF),
            "update_t" => {
                let samples = vec![0i32; 512];
                let _ = (s.upd)(std::ptr::null_mut(), samples.as_ptr());
            }
            "update_samples" => {
                let mut rec = vec![0u8; ARENA];
                let _ = (s.upd)(rec.as_mut_ptr(), std::ptr::null());
            }
            other => panic!("unknown HV_NULL_FN={other}"),
        }
    }
    // Should be unreachable; a distinctive code so the parent notices.
    std::process::exit(77);
}

// ===========================================================================
// tflac_md5_addsample error/degenerate rows
// ===========================================================================

/// E3 — `bits == 0` (zero "length"): accepted, still writes 8 bytes, `pos`
/// unchanged, no copy loop.
#[test]
fn err_e3_bits_zero() {
    let mut rng = Rng::new(0xE003);
    for pos in 0..64u32 {
        for i in 0..16 {
            let mut tpl = rng.arena();
            let buf = rng.buf72();
            let total = rng.u64();
            put_md5(&mut tpl, 0, pos, total, &buf);
            diff_add(&tpl, 0, 0, rng.u64(), &format!("E3 pos={pos} i={i}"));
        }
    }
    // and out-of-range pos with bits == 0
    for pos in [64u32, 1000, u32::MAX] {
        let mut tpl = rng.arena();
        let buf = rng.buf72();
        put_md5(&mut tpl, 0, pos, rng.u64(), &buf);
        diff_add(&tpl, 0, 0, rng.u64(), &format!("E3 oor pos={pos}"));
    }
}

/// E4 — `bits` not a multiple of 8: remainder silently discarded by `bits/8`
/// while `total += bits` keeps it.
#[test]
fn err_e4_bits_not_multiple_of_8() {
    let mut rng = Rng::new(0xE004);
    let mut bitsets: Vec<u32> = (1..8).collect();
    bitsets.extend([9u32, 15, 17, 31, 33, 63, 65, 127, 511, 513, 575, 577]);
    for bits in bitsets {
        for pos in [0u32, 1, 7, 8, 55, 56, 57, 63] {
            for i in 0..8 {
                let mut tpl = rng.arena();
                let buf = rng.buf72();
                put_md5(&mut tpl, 0, pos, rng.u64(), &buf);
                diff_add(
                    &tpl,
                    0,
                    bits,
                    rng.u64(),
                    &format!("E4 bits={bits} pos={pos} i={i}"),
                );
            }
        }
    }
}

/// E5 — oversized `bits == u32::MAX` ⇒ `bytes == 0x1FFF_FFFF`, no bounds check.
#[test]
fn err_e5_bits_max_u32() {
    let mut rng = Rng::new(0xE005);
    for bits in [u32::MAX, u32::MAX - 1, 0xFFFF_FFF8, 0x8000_0000, 0x7FFF_FFFF] {
        for pos in 0..64u32 {
            let mut tpl = rng.arena();
            let buf = rng.buf72();
            put_md5(&mut tpl, 0, pos, rng.u64(), &buf);
            diff_add(&tpl, 0, bits, rng.u64(), &format!("E5 bits={bits:#x} pos={pos}"));
        }
    }
}

/// E6 — `m->pos` already out of its 0..63 range on entry, incl. exactly one
/// step past (`64`) and far past.
#[test]
fn err_e6_pos_out_of_range() {
    let mut rng = Rng::new(0xE006);
    let poss = [
        64u32, 65, 71, 72, 73, 127, 128, 129, 1000, 0x1_0000, 0x8000_0000, 0xFFFF_FF00,
    ];
    for pos in poss {
        for bits in [0u32, 8, 63, 64, 512, 576, u32::MAX] {
            for i in 0..8 {
                let mut tpl = rng.arena();
                let buf = rng.buf72();
                put_md5(&mut tpl, 0, pos, rng.u64(), &buf);
                diff_add(
                    &tpl,
                    0,
                    bits,
                    rng.u64(),
                    &format!("E6 pos={pos:#x} bits={bits} i={i}"),
                );
            }
        }
    }
    // The documented C behaviour for the canonical one-step-past case.
    let l = libs();
    let mut tpl = vec![0u8; ARENA];
    put_md5(&mut tpl, 0, 64, 0, &[0xAAu8; BUF_LEN]);
    let mut a = Arena::from_template(&tpl);
    unsafe { (l.c.add)(a.ptr(), 64, 0) };
    assert_eq!(get_pos(a.bytes(), 0), 8, "E6: pos=64,bits=64 ⇒ pos becomes 8");
}

/// E7 — `m->pos + bits/8` overflows `tflac_u32` ⇒ the `>= 64` test is FALSE and
/// the copy loop is skipped entirely.
#[test]
fn err_e7_pos_wraparound() {
    let mut rng = Rng::new(0xE007);
    for (pos, bits) in [
        (0xFFFF_FFFFu32, 64u32),
        (0xFFFF_FFFF, 8),
        (0xFFFF_FFF8, 64),
        (0xFFFF_FFC0, 512),
        (0xFFFF_FF00, u32::MAX),
        (0x8000_0000, u32::MAX),
    ] {
        for i in 0..16 {
            let mut tpl = rng.arena();
            let buf = rng.buf72();
            put_md5(&mut tpl, 0, pos, rng.u64(), &buf);
            diff_add(
                &tpl,
                0,
                bits,
                rng.u64(),
                &format!("E7 pos={pos:#x} bits={bits:#x} i={i}"),
            );
        }
    }
    // Canonical case: pos=0xFFFFFFFF, bits=64 ⇒ pos wraps to 7, 7 < 64 ⇒ no copy.
    let l = libs();
    let mut tpl = vec![0u8; ARENA];
    put_md5(&mut tpl, 0, 0xFFFF_FFFF, 0, &[0xAAu8; BUF_LEN]);
    let mut a = Arena::from_template(&tpl);
    unsafe { (l.c.add)(a.ptr(), 64, 0) };
    assert_eq!(get_pos(a.bytes(), 0), 7, "E7: 0xFFFFFFFF + 8 wraps to 7");
}

/// E8 — `m->total + bits` overflows `tflac_u64` (wraps, never saturates).
#[test]
fn err_e8_total_wraparound() {
    let mut rng = Rng::new(0xE008);
    for total in [
        u64::MAX,
        u64::MAX - 1,
        u64::MAX - 63,
        u64::MAX - 64,
        u64::MAX - 0xFFFF_FFFE,
        u64::MAX - 0xFFFF_FFFF,
    ] {
        for bits in [0u32, 1, 64, 0xFFFF_FFFF] {
            let mut tpl = rng.arena();
            let buf = rng.buf72();
            put_md5(&mut tpl, 0, rng.u32() % 64, total, &buf);
            diff_add(
                &tpl,
                0,
                bits,
                rng.u64(),
                &format!("E8 total={total:#x} bits={bits}"),
            );
        }
    }
    // Canonical case.
    let l = libs();
    let mut tpl = vec![0u8; ARENA];
    put_md5(&mut tpl, 0, 0, u64::MAX, &[0u8; BUF_LEN]);
    let mut a = Arena::from_template(&tpl);
    unsafe { (l.c.add)(a.ptr(), 64, 0) };
    assert_eq!(get_total(a.bytes(), 0), 63, "E8: u64::MAX + 64 wraps to 63");
}

/// E9 — `pos % 64` in 57..=63 makes `tflac_pack_u64le` write past the 64-byte
/// logical block into the `buffer[64..72]` tail.
#[test]
fn err_e9_write_spills_into_tail() {
    let mut rng = Rng::new(0xE009);
    for pos in 57..64u32 {
        for extra in [0u32, 64, 128, 1024] {
            for bits in [0u32, 8, 64, 512] {
                for i in 0..8 {
                    let mut tpl = rng.arena();
                    let buf = rng.buf72();
                    put_md5(&mut tpl, 0, pos + extra, rng.u64(), &buf);
                    diff_add(
                        &tpl,
                        0,
                        bits,
                        rng.u64(),
                        &format!("E9 pos={} bits={bits} i={i}", pos + extra),
                    );
                }
            }
        }
    }
    // Canonical: pos = 63 ⇒ bytes 63..71 of the buffer are overwritten.
    let l = libs();
    let mut tpl = vec![0u8; ARENA];
    put_md5(&mut tpl, 0, 63, 0, &[0u8; BUF_LEN]);
    let mut a = Arena::from_template(&tpl);
    unsafe { (l.c.add)(a.ptr(), 0, u64::MAX) };
    let b = a.bytes();
    assert_eq!(&b[BUF_OFF + 63..BUF_OFF + 71], &[0xFFu8; 8], "E9 spill");
    assert_eq!(b[BUF_OFF + 71], 0, "E9 must not write buffer[71]");
}

/// E10 — the copy loop reads past `buffer[72]`, up to `buffer[126]`
/// (38 bytes past `sizeof(tflac_md5)`).
#[test]
fn err_e10_copy_loop_reads_past_buffer() {
    let mut rng = Rng::new(0xE010);
    for r in 9..64u32 {
        let bits = bits_for_reduced_pos(r);
        for i in 0..16 {
            let mut tpl = rng.arena();
            let buf = rng.buf72();
            put_md5(&mut tpl, 0, 0, rng.u64(), &buf);
            diff_add(&tpl, 0, bits, rng.u64(), &format!("E10 r={r} bits={bits} i={i}"));
        }
    }
    // Explicit worst case: reduced pos = 63 ⇒ reads buffer[64..127].
    let bits = bits_for_reduced_pos(63);
    let mut tpl = index_pattern();
    put_md5(&mut tpl, 0, 0, 0, &{
        let mut b = [0u8; BUF_LEN];
        for (i, x) in b.iter_mut().enumerate() {
            *x = 0x80 | (i as u8 & 0x3f);
        }
        b
    });
    diff_add(&tpl, 0, bits, 0xDEAD_BEEF_CAFE_F00D, "E10 worst-case r=63");

    // Same, but with the record at several offsets so the OOB source bytes differ.
    for off in [8usize, 128, 1024, ARENA - ADD_MAX_TOUCH - 8] {
        let mut tpl = index_pattern();
        put_md5(&mut tpl, off, 0, 0, &[0x11u8; BUF_LEN]);
        diff_add(&tpl, off, bits, 0, &format!("E10 worst-case off={off}"));
    }
}

fn index_pattern() -> Vec<u8> {
    (0..ARENA).map(|i| i as u8).collect()
}

/// E11 — `while (bytes--)` with `bytes == 0`: zero iterations, and the
/// post-decrement underflow to `0xFFFF_FFFF` must be discarded.
#[test]
fn err_e11_copy_loop_zero_iterations() {
    let mut rng = Rng::new(0xE011);
    for (pos, bits) in [
        (56u32, 64u32),
        (0, 512),
        (32, 256),
        (63, 8),
        (60, 32),
        (1, 8 * 63),
        (0, 8 * 128),
    ] {
        for i in 0..32 {
            let mut tpl = rng.arena();
            let buf = rng.buf72();
            put_md5(&mut tpl, 0, pos, rng.u64(), &buf);
            diff_add(&tpl, 0, bits, rng.u64(), &format!("E11 pos={pos} bits={bits} i={i}"));
        }
    }
    // Canonical: pos=56,bits=64 ⇒ pos becomes 0, the copy loop must not run, so
    // buffer[0..56] keeps its original contents.
    let l = libs();
    let mut tpl = vec![0u8; ARENA];
    let mut buf = [0u8; BUF_LEN];
    for (i, x) in buf.iter_mut().enumerate() {
        *x = 0xA0 | (i as u8 & 0xF);
    }
    put_md5(&mut tpl, 0, 56, 0, &buf);
    let mut a = Arena::from_template(&tpl);
    unsafe { (l.c.add)(a.ptr(), 64, 0) };
    assert_eq!(get_pos(a.bytes(), 0), 0);
    assert_eq!(
        &a.bytes()[BUF_OFF..BUF_OFF + 56],
        &buf[..56],
        "E11: zero-iteration copy loop must leave buffer[0..56] untouched"
    );
}

// ===========================================================================
// update_md5 error/degenerate rows
// ===========================================================================

/// E14 — `samples` shorter than the 136-element span `update_md5` unconditionally
/// reads (the iteration count is hard-coded, not derived from `b`).
#[test]
fn err_e14_samples_shorter_than_read_span() {
    let mut rng = Rng::new(0xE014);
    // "logical" buffer lengths a caller would size from cur_blocksize*channels
    for (cb, ch) in [(1u32, 1u32), (1, 2), (2, 2), (8, 1), (16, 2), (32, 1), (33, 4)] {
        let logical = (cb as usize) * (ch as usize);
        for i in 0..32 {
            // the first `logical` elements are the caller's data, the rest is
            // "adjacent memory" — identical on both sides, hence comparable
            let mut stpl = rng.arena();
            for e in 0..logical.min(ARENA / 4) {
                let v = rng.i32();
                stpl[e * 4..e * 4 + 4].copy_from_slice(&v.to_le_bytes());
            }
            let mut tpl = rng.arena();
            let buf = rng.buf72();
            put_tflac(&mut tpl, 0, 0, 0, &buf, cb, ch);
            let r = diff_upd(
                &tpl,
                0,
                &stpl,
                0,
                &format!("E14 cb={cb} ch={ch} logical={logical} i={i}"),
            );
            assert_eq!(r, cb.wrapping_mul(ch).wrapping_sub(40));
        }
    }
}

/// E15 — `b = cur_blocksize * channels` underflows: every `b` in 0..=40.
#[test]
fn err_e15_b_underflow() {
    let mut rng = Rng::new(0xE015);
    for b in 0..=40u32 {
        let pairs: Vec<(u32, u32)> = if b == 0 {
            vec![(0, 0), (0, 1), (1, 0), (0, 0xFFFF_FFFF), (0xFFFF_FFFF, 0)]
        } else {
            vec![(b, 1), (1, b)]
        };
        for (cb, ch) in pairs {
            for i in 0..8 {
                let mut tpl = rng.arena();
                let buf = rng.buf72();
                put_tflac(&mut tpl, 0, 0, 0, &buf, cb, ch);
                let stpl = rng.arena();
                let r = diff_upd(&tpl, 0, &stpl, 0, &format!("E15 b={b} cb={cb} ch={ch} i={i}"));
                assert_eq!(
                    r,
                    b.wrapping_sub(40),
                    "E15 b={b}: expected unsigned wraparound"
                );
            }
        }
    }
    // the exact boundary: b == 40 returns 0, b == 39 returns 0xFFFFFFFF
    let mut tpl = vec![0u8; ARENA];
    put_tflac(&mut tpl, 0, 0, 0, &[0u8; BUF_LEN], 40, 1);
    assert_eq!(diff_upd(&tpl, 0, &vec![0u8; ARENA], 0, "E15 b=40"), 0);
    let mut tpl = vec![0u8; ARENA];
    put_tflac(&mut tpl, 0, 0, 0, &[0u8; BUF_LEN], 39, 1);
    assert_eq!(
        diff_upd(&tpl, 0, &vec![0u8; ARENA], 0, "E15 b=39"),
        0xFFFF_FFFF
    );
}

/// E16 — `cur_blocksize * channels` overflows `tflac_u32`.
#[test]
fn err_e16_b_multiply_overflow() {
    let mut rng = Rng::new(0xE016);
    for (cb, ch) in [
        (0x1_0000u32, 0x1_0000u32),
        (0xFFFF_FFFF, 3),
        (0x8000_0000, 2),
        (0xFFFF, 0x1_0001),
        (0xFFFF_FFFF, 0xFFFF_FFFF),
        (0x1_0000, 0x1_0001),
        (0xABCD_EF01, 0x1234_5678),
    ] {
        for i in 0..32 {
            let mut tpl = rng.arena();
            let buf = rng.buf72();
            put_tflac(&mut tpl, 0, 0, 0, &buf, cb, ch);
            let stpl = rng.arena();
            let r = diff_upd(&tpl, 0, &stpl, 0, &format!("E16 cb={cb:#x} ch={ch:#x} i={i}"));
            assert_eq!(r, cb.wrapping_mul(ch).wrapping_sub(40));
        }
    }
    // fully random overflow-prone pairs
    for i in 0..512 {
        let (cb, ch) = (rng.u32(), rng.u32());
        let mut tpl = rng.arena();
        let buf = rng.buf72();
        put_tflac(&mut tpl, 0, 0, 0, &buf, cb, ch);
        let stpl = rng.arena();
        let r = diff_upd(&tpl, 0, &stpl, 0, &format!("E16 rand i={i} cb={cb:#x} ch={ch:#x}"));
        assert_eq!(r, cb.wrapping_mul(ch).wrapping_sub(40));
    }
}

/// E17 — `t->md5_ctx.pos` out of range through the public entry point.
#[test]
fn err_e17_update_md5_pos_out_of_range() {
    let mut rng = Rng::new(0xE017);
    for pos in [
        64u32, 65, 71, 72, 127, 128, 1000, 0x1_0000, 0x8000_0000, 0xFFFF_FFF8, 0xFFFF_FFFF,
    ] {
        for i in 0..32 {
            let mut tpl = rng.arena();
            let buf = rng.buf72();
            put_tflac(&mut tpl, 0, pos, rng.u64(), &buf, 4096, 2);
            let stpl = rng.arena();
            diff_upd(&tpl, 0, &stpl, 0, &format!("E17 pos={pos:#x} i={i}"));
        }
    }
    // Canonical: pos = 0xFFFFFFFF ⇒ 7, 15, 23, 31, 39 over the five iterations.
    let l = libs();
    let mut tpl = vec![0u8; ARENA];
    put_tflac(&mut tpl, 0, 0xFFFF_FFFF, 0, &[0u8; BUF_LEN], 4096, 2);
    let mut a = Arena::from_template(&tpl);
    let mut s = Arena::from_template(&vec![0u8; ARENA]);
    unsafe { (l.c.upd)(a.ptr(), s.ptr() as *const i32) };
    assert_eq!(get_pos(a.bytes(), 0), 39, "E17 canonical pos chain");
}

/// E18 — `t->md5_ctx.total` wraps during the 5 × `+64`.
#[test]
fn err_e18_update_md5_total_wraparound() {
    let mut rng = Rng::new(0xE018);
    for total in [
        u64::MAX,
        u64::MAX - 1,
        u64::MAX - 63,
        u64::MAX - 64,
        u64::MAX - 319,
        u64::MAX - 320,
        u64::MAX - 321,
    ] {
        for i in 0..32 {
            let mut tpl = rng.arena();
            let buf = rng.buf72();
            put_tflac(&mut tpl, 0, rng.u32() % 64, total, &buf, 1152, 2);
            let stpl = rng.arena();
            diff_upd(&tpl, 0, &stpl, 0, &format!("E18 total={total:#x} i={i}"));
        }
    }
    // Canonical: u64::MAX + 320 == 319.
    let l = libs();
    let mut tpl = vec![0u8; ARENA];
    put_tflac(&mut tpl, 0, 0, u64::MAX, &[0u8; BUF_LEN], 1, 1);
    let mut a = Arena::from_template(&tpl);
    let mut s = Arena::from_template(&vec![0u8; ARENA]);
    unsafe { (l.c.upd)(a.ptr(), s.ptr() as *const i32) };
    assert_eq!(get_total(a.bytes(), 0), 319, "E18 canonical total wrap");
}

// ===========================================================================
// E19 / E20 — generic boundaries
// ===========================================================================

/// E19 — out-of-range "enum"-like values across the FFI boundary.  The only
/// mode-valued parameter in the whole API is `tflac_md5_addsample`'s `bits`;
/// the C validates nothing, so every one of the 2^32 patterns is a real input.
/// Fuzzed over the full `u32` range, plus every value one step past each
/// meaningful bound.
#[test]
fn err_e19_bits_full_u32_range_fuzz() {
    let mut rng = Rng::new(0xE019);

    // one step past every documented/meaningful bound
    let boundaries: Vec<u32> = vec![
        0,
        1,
        7,
        8,
        9,
        63,
        64,
        65,
        // 64-byte block == 512 bits, 72-byte array == 576 bits
        511,
        512,
        513,
        575,
        576,
        577,
        // 64*8 == 512 already covered; pos-wrapping bounds
        0x1FFF_FFFF * 8,
        0xFFFF_FFF8,
        0xFFFF_FFF9,
        0xFFFF_FFFE,
        u32::MAX,
        0x8000_0000,
        0x7FFF_FFFF,
    ];
    for bits in boundaries {
        for pos in [0u32, 1, 8, 55, 56, 57, 63, 64, 0xFFFF_FFFF] {
            let mut tpl = rng.arena();
            let buf = rng.buf72();
            put_md5(&mut tpl, 0, pos, rng.u64(), &buf);
            diff_add(&tpl, 0, bits, rng.u64(), &format!("E19 bits={bits:#x} pos={pos:#x}"));
        }
    }

    // full-range fuzz
    for i in 0..3000 {
        let bits = rng.u32();
        let pos = match rng.below(3) {
            0 => rng.u32() % 64,
            1 => rng.u32(),
            _ => 64 + rng.u32() % 64,
        };
        let mut tpl = rng.arena();
        let buf = rng.buf72();
        put_md5(&mut tpl, 0, pos, rng.u64(), &buf);
        diff_add(&tpl, 0, bits, rng.u64(), &format!("E19 fuzz i={i} bits={bits:#x} pos={pos:#x}"));
    }
}

/// E20 — `tflac_pack_u64le` with a misaligned destination and with the last
/// writable 8 bytes of the region.
#[test]
fn err_e20_pack_misaligned_and_last_8_bytes() {
    let mut rng = Rng::new(0xE020);
    let tpl = index_pattern();
    // misaligned
    for mis in 1..8usize {
        for i in 0..64 {
            let n = rng.u64();
            diff_pack(&tpl, mis, n, &format!("E20 mis={mis} i={i}"));
            diff_pack(&tpl, 64 + mis, n, &format!("E20 mis@64={mis} i={i}"));
        }
    }
    // the very last 8 writable bytes (d[7] is the final byte)
    for i in 0..64 {
        let n = rng.u64();
        diff_pack(&tpl, ARENA - 8, n, &format!("E20 last8 i={i}"));
    }
    // and the first byte / offset 0
    for n in [0u64, u64::MAX, 0x0123_4567_89AB_CDEF] {
        diff_pack(&tpl, 0, n, &format!("E20 off=0 n={n:#x}"));
        diff_pack(&tpl, ARENA - 8, n, &format!("E20 last8 n={n:#x}"));
    }
}

/// Extra generic boundary sweep: zero and oversized "lengths" for every entry
/// point, in one place, so nothing depends on the row-by-row tests above.
#[test]
fn err_generic_zero_and_oversized_lengths() {
    let mut rng = Rng::new(0xE0FF);
    // addsample: zero and maximal bits at every pos
    for pos in 0..64u32 {
        for bits in [0u32, u32::MAX] {
            let mut tpl = rng.arena();
            let buf = rng.buf72();
            put_md5(&mut tpl, 0, pos, rng.u64(), &buf);
            diff_add(&tpl, 0, bits, rng.u64(), &format!("Egen add pos={pos} bits={bits:#x}"));
        }
    }
    // update_md5: zero-sized and maximal shapes
    for (cb, ch) in [
        (0u32, 0u32),
        (0, u32::MAX),
        (u32::MAX, 0),
        (u32::MAX, u32::MAX),
        (1, u32::MAX),
        (u32::MAX, 1),
    ] {
        let mut tpl = rng.arena();
        let buf = rng.buf72();
        put_tflac(&mut tpl, 0, 0, 0, &buf, cb, ch);
        let stpl = rng.arena();
        let r = diff_upd(&tpl, 0, &stpl, 0, &format!("Egen upd cb={cb:#x} ch={ch:#x}"));
        assert_eq!(r, cb.wrapping_mul(ch).wrapping_sub(40));
    }
    // pack: n == 0 and n == u64::MAX at every alignment
    let tpl = index_pattern();
    for off in 0..16usize {
        diff_pack(&tpl, off, 0, &format!("Egen pack off={off} n=0"));
        diff_pack(&tpl, off, u64::MAX, &format!("Egen pack off={off} n=MAX"));
    }
}
