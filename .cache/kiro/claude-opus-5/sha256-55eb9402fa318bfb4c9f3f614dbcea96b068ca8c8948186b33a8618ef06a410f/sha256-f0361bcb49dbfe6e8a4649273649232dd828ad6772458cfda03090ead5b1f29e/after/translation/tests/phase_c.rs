//! Phase C — error-path differential tests. One test per row of `ERRORS.md`.
//!
//! The C library validates nothing (1 `return`, 1 `if`, 0 asserts, 0 NULL
//! checks, 0 error enums — see `ERRORS.md` for the greps), so each row asserts
//! that the Rust reproduces the C's *non*-rejection exactly: the same wrapped
//! value, the same bytes, or the same fatal signal.

mod common;
use common::*;

use std::os::unix::process::ExitStatusExt;
use std::process::Command;

const N: usize = 300;

// ---------------------------------------------------------------------------
// NULL-pointer rows need a child process, because the C behaviour is a fatal
// signal. The child re-runs THIS test binary with `NULL_CASE`/`NULL_LIB` set;
// `null_deref_child` below is the only test that then does anything.
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
struct Death {
    signal: Option<i32>,
    code: Option<i32>,
    survived: bool,
}

fn run_null_case(case: &str, lib: &str) -> Death {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args(["null_deref_child", "--exact", "--nocapture", "--test-threads=1"])
        .env("NULL_CASE", case)
        .env("NULL_LIB", lib)
        .env_remove("RUST_BACKTRACE")
        .output()
        .expect("spawn child");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    Death {
        signal: out.status.signal(),
        code: out.status.code(),
        survived: stdout.contains("SURVIVED"),
    }
}

fn assert_null_case_matches(case: &str) {
    let c = run_null_case(case, "c");
    let r = run_null_case(case, "rust");
    assert_eq!(
        c, r,
        "[{case}] C and Rust disagree on the NULL-pointer outcome:\n  C   = {c:?}\n  RUST= {r:?}"
    );
    // Sanity: this row is only meaningful if something actually happened.
    assert!(
        c.signal.is_some() || c.survived,
        "[{case}] child neither died from a signal nor reported SURVIVED: {c:?}"
    );
}

/// Not a real test on its own — the driver above re-invokes it with env vars set.
#[test]
fn null_deref_child() {
    let Ok(case) = std::env::var("NULL_CASE") else {
        return;
    };
    let which = std::env::var("NULL_LIB").expect("NULL_LIB");
    let api = if which == "c" { Api::load_c() } else { Api::load_rust() };

    let mut st = Region::from_bytes(&StateBuilder::zeroed().cur_blocksize(1024).channels(2).build());
    let mut sm = Region::from_bytes(&vec![0u8; SAMPLES_LEN]);

    unsafe {
        match case.as_str() {
            "pack_null" => (api.pack_u64le)(std::ptr::null_mut(), 0x0123_4567_89AB_CDEF),
            "addsample_null" => (api.md5_addsample)(std::ptr::null_mut(), 64, 0xDEAD_BEEF),
            "addsample_null_bits0" => (api.md5_addsample)(std::ptr::null_mut(), 0, 0),
            "update_t_null" => {
                let r = (api.update_md5)(std::ptr::null_mut(), sm.as_mut_ptr() as *const i32);
                println!("ret={r}");
            }
            "update_samples_null" => {
                let r = (api.update_md5)(st.as_mut_ptr(), std::ptr::null());
                println!("ret={r}");
            }
            "update_both_null" => {
                let r = (api.update_md5)(std::ptr::null_mut(), std::ptr::null());
                println!("ret={r}");
            }
            other => panic!("unknown NULL_CASE {other}"),
        }
    }
    println!("SURVIVED");
}

// ===========================================================================
// ERRORS.md row 1 — pack_u64le(NULL, n)
// ===========================================================================
#[test]
fn err01_pack_null_pointer() {
    assert_null_case_matches("pack_null");
}

// ===========================================================================
// ERRORS.md row 2 — pack_u64le to an unaligned destination (accepted)
// ===========================================================================
#[test]
fn err02_pack_unaligned_accepted() {
    let mut rng = Rng::new(0xC002);
    for off in 1..8usize {
        for _ in 0..40 {
            let n = rng.next_u64();
            let st = StateBuilder::random(&mut rng).build();
            let sm = vec![0u8; SAMPLES_LEN];
            run_diff(
                &format!("err02/off={off}"),
                &st,
                &sm,
                &|api, s, _| unsafe { (api.pack_u64le)(s.add(OFF_BUFFER + off), n) },
            );
        }
    }
}

// ===========================================================================
// ERRORS.md row 3 — pack_u64le at buffer[64]: the LAST fully legal 8-byte slot
// ===========================================================================
#[test]
fn err03_pack_last_legal_slot() {
    let mut rng = Rng::new(0xC003);
    for _ in 0..N {
        let n = rng.next_u64();
        let st = StateBuilder::random(&mut rng).build();
        let sm = vec![0u8; SAMPLES_LEN];
        run_diff("err03/buffer[64]", &st, &sm, &|api, s, _| unsafe {
            (api.pack_u64le)(s.add(OFF_BUFFER + BUFFER_LEN - 8), n);
        });
    }
}

// ===========================================================================
// ERRORS.md row 4 — pack_u64le at buffer[65]: ONE STEP PAST the last legal slot
// ===========================================================================
#[test]
fn err04_pack_one_past_last_legal_slot() {
    let mut rng = Rng::new(0xC004);
    for extra in 1..=8usize {
        for _ in 0..40 {
            let n = rng.next_u64();
            let st = StateBuilder::random(&mut rng).build();
            let sm = vec![0u8; SAMPLES_LEN];
            run_diff(
                &format!("err04/buffer[{}]", BUFFER_LEN - 8 + extra),
                &st,
                &sm,
                &|api, s, _| unsafe {
                    (api.pack_u64le)(s.add(OFF_BUFFER + BUFFER_LEN - 8 + extra), n)
                },
            );
        }
    }
}

// ===========================================================================
// ERRORS.md row 5 — addsample(NULL, ...)
// ===========================================================================
#[test]
fn err05_addsample_null_pointer() {
    assert_null_case_matches("addsample_null");
    assert_null_case_matches("addsample_null_bits0");
}

// ===========================================================================
// ERRORS.md row 6 — addsample bits == 0 (zero length, not rejected)
// ===========================================================================
#[test]
fn err06_addsample_zero_bits() {
    let mut rng = Rng::new(0xC006);
    for &pos in &[0u32, 1, 32, 55, 63, 64, 65, 100, 1000, u32::MAX] {
        for _ in 0..40 {
            let total = rng.next_u64();
            let val = rng.next_u64();
            let st = StateBuilder::random(&mut rng).pos(pos).total(total).build();
            let sm = vec![0u8; SAMPLES_LEN];
            let expect_total = total; // += 0
            let expect_pos = if pos >= 64 { pos % 64 } else { pos };
            run_diff(
                &format!("err06/pos={pos}"),
                &st,
                &sm,
                &|api, s, _| unsafe {
                    (api.md5_addsample)(s, 0, val);
                    let seen = std::slice::from_raw_parts(s, SIZEOF_TFLAC);
                    assert_eq!(get_total(seen), expect_total, "{}: total changed", api.which);
                    assert_eq!(get_pos(seen), expect_pos, "{}: pos wrong", api.which);
                },
            );
        }
    }
}

// ===========================================================================
// ERRORS.md row 7 — addsample bits not a multiple of 8 (truncating division)
// ===========================================================================
#[test]
fn err07_addsample_bits_not_multiple_of_8() {
    let mut rng = Rng::new(0xC007);
    for bits in [1u32, 2, 3, 4, 5, 6, 7, 9, 15, 17, 31, 63, 65, 511, 513, 4095] {
        for &pos in &[0u32, 7, 56, 63, 64, 1000] {
            for _ in 0..6 {
                let total = rng.next_u64();
                let val = rng.next_u64();
                let st = StateBuilder::random(&mut rng).pos(pos).total(total).build();
                let sm = vec![0u8; SAMPLES_LEN];
                let expect_total = total.wrapping_add(bits as u64);
                run_diff(
                    &format!("err07/bits={bits} pos={pos}"),
                    &st,
                    &sm,
                    &|api, s, _| unsafe {
                        (api.md5_addsample)(s, bits, val);
                        let seen = std::slice::from_raw_parts(s, SIZEOF_TFLAC);
                        assert_eq!(
                            get_total(seen),
                            expect_total,
                            "{}: total must add the UNtruncated bits",
                            api.which
                        );
                    },
                );
            }
        }
    }
}

// ===========================================================================
// ERRORS.md row 8 — addsample bits == 0xFFFFFFFF (oversized length)
// ===========================================================================
#[test]
fn err08_addsample_bits_max() {
    let mut rng = Rng::new(0xC008);
    for bits in [u32::MAX, u32::MAX - 1, 0xFFFF_FFF8, 0x8000_0000, 0x7FFF_FFFF] {
        for &pos in &[0u32, 1, 63, 64, 1000, u32::MAX] {
            for _ in 0..8 {
                let total = rng.next_u64();
                let val = rng.next_u64();
                let st = StateBuilder::random(&mut rng).pos(pos).total(total).build();
                let sm = vec![0u8; SAMPLES_LEN];
                let expect_total = total.wrapping_add(bits as u64);
                let expect_pos = {
                    let p = pos.wrapping_add(bits / 8);
                    if p >= 64 { p % 64 } else { p }
                };
                run_diff(
                    &format!("err08/bits={bits:#x} pos={pos}"),
                    &st,
                    &sm,
                    &|api, s, _| unsafe {
                        (api.md5_addsample)(s, bits, val);
                        let seen = std::slice::from_raw_parts(s, SIZEOF_TFLAC);
                        assert_eq!(get_total(seen), expect_total, "{}: total", api.which);
                        assert_eq!(get_pos(seen), expect_pos, "{}: pos", api.which);
                    },
                );
            }
        }
    }
}

// ===========================================================================
// ERRORS.md row 9 — addsample total overflow (wraps mod 2^64, no error)
// ===========================================================================
#[test]
fn err09_addsample_total_overflow() {
    let mut rng = Rng::new(0xC009);
    for _ in 0..N {
        let total = u64::MAX - (rng.next_u64() % 0x40);
        let bits = rng.pick(&[1u32, 8, 63, 64, 65, u32::MAX]);
        let val = rng.next_u64();
        let pos = rng.below(70);
        let st = StateBuilder::random(&mut rng).pos(pos).total(total).build();
        let sm = vec![0u8; SAMPLES_LEN];
        let expect = total.wrapping_add(bits as u64);
        run_diff(
            &format!("err09/total={total:#x} bits={bits}"),
            &st,
            &sm,
            &|api, s, _| unsafe {
                (api.md5_addsample)(s, bits, val);
                let seen = std::slice::from_raw_parts(s, SIZEOF_TFLAC);
                assert_eq!(get_total(seen), expect, "{}: total must wrap mod 2^64", api.which);
            },
        );
    }
}

// ===========================================================================
// ERRORS.md row 10 — addsample pos overflow (wraps mod 2^32 BEFORE the >=64 test)
// ===========================================================================
#[test]
fn err10_addsample_pos_overflow() {
    let mut rng = Rng::new(0xC00A);
    // Hand-pick a case where the wrap makes the `>= 64` test FALSE even though
    // the mathematical sum is huge: pos = 0xFFFFFFFF, bytes = 1 -> pos = 0.
    let handpicked: &[(u32, u32)] = &[
        (u32::MAX, 8),          // bytes=1 -> 0x100000000 -> 0  -> branch NOT taken
        (u32::MAX, 16),         // bytes=2 -> 1            -> branch NOT taken
        (u32::MAX - 62, 511),   // bytes=63 -> 0           -> branch NOT taken
        (u32::MAX, 64),         // bytes=8 -> 7            -> branch NOT taken
        (0xFFFF_FFC0, 8),       // bytes=1 -> 0xFFFFFFC1   -> branch taken
    ];
    for &(pos, bits) in handpicked {
        let st = StateBuilder::random(&mut rng).pos(pos).build();
        let sm = vec![0u8; SAMPLES_LEN];
        let expect_pos = {
            let p = pos.wrapping_add(bits / 8);
            if p >= 64 { p % 64 } else { p }
        };
        run_diff(
            &format!("err10/pos={pos:#x} bits={bits}"),
            &st,
            &sm,
            &|api, s, _| unsafe {
                (api.md5_addsample)(s, bits, 0xA5A5_5A5A_1234_5678);
                let seen = std::slice::from_raw_parts(s, SIZEOF_TFLAC);
                assert_eq!(get_pos(seen), expect_pos, "{}: wrapped pos", api.which);
            },
        );
    }
    // Randomized sweep of the same class.
    for _ in 0..N {
        let pos = u32::MAX - rng.below(0x4000_0000);
        let bits = rng.next_u32();
        let val = rng.next_u64();
        let st = StateBuilder::random(&mut rng).pos(pos).build();
        let sm = vec![0u8; SAMPLES_LEN];
        run_diff(
            &format!("err10/rand pos={pos:#x} bits={bits:#x}"),
            &st,
            &sm,
            &|api, s, _| unsafe { (api.md5_addsample)(s, bits, val) },
        );
    }
}

// ===========================================================================
// ERRORS.md row 11 — addsample pos == 63, bits == 64: write ends at buffer[70],
//                    the last byte still inside the 72-byte array
// ===========================================================================
#[test]
fn err11_addsample_pos63_write_boundary() {
    let mut rng = Rng::new(0xC00B);
    for _ in 0..N {
        let val = rng.next_u64();
        let st = StateBuilder::random(&mut rng).pos(63).build();
        let sm = vec![0u8; SAMPLES_LEN];
        run_diff("err11/pos=63 bits=64", &st, &sm, &|api, s, _| unsafe {
            (api.md5_addsample)(s, 64, val);
            // The 8 packed bytes must occupy buffer[63..71] exactly.
            let seen = std::slice::from_raw_parts(s, STATE_LEN);
            let got = &seen[OFF_BUFFER + 63..OFF_BUFFER + 71];
            assert_eq!(got, &val.to_le_bytes()[..], "{}: write placement", api.which);
        });
    }
}

// ===========================================================================
// ERRORS.md row 12 — addsample pos >= 64 on entry (outside the ring range)
// ===========================================================================
#[test]
fn err12_addsample_pos_out_of_ring_range() {
    let mut rng = Rng::new(0xC00C);
    for &pos in &[64u32, 65, 71, 72, 100, 127, 128, 129, 1000, 0xFFFF, 0x7FFF_FFFF, u32::MAX] {
        for &bits in &[0u32, 8, 64, 65, 512] {
            for _ in 0..8 {
                let val = rng.next_u64();
                let st = StateBuilder::random(&mut rng).pos(pos).build();
                let sm = vec![0u8; SAMPLES_LEN];
                // The WRITE uses pos % 64 even though the ADD uses the raw pos.
                let write_off = (pos % 64) as usize;
                run_diff(
                    &format!("err12/pos={pos} bits={bits}"),
                    &st,
                    &sm,
                    &|api, s, _| unsafe {
                        let before = std::slice::from_raw_parts(s, STATE_LEN).to_vec();
                        (api.md5_addsample)(s, bits, val);
                        let after = std::slice::from_raw_parts(s, STATE_LEN);
                        // Only meaningful when the carry-down did not overwrite it.
                        if bits / 8 == 0 && pos % 64 == 0 {
                            assert_eq!(
                                &after[OFF_BUFFER + write_off..OFF_BUFFER + write_off + 8],
                                &val.to_le_bytes()[..],
                                "{}: write must use pos%64",
                                api.which
                            );
                        }
                        let _ = before;
                    },
                );
            }
        }
    }
}

// ===========================================================================
// ERRORS.md row 13 — the carry-down loop's OUT-OF-BOUNDS source read
//                    (buffer[64 + bytes], up to buffer[126])
// ===========================================================================
#[test]
fn err13_addsample_carrydown_oob_source() {
    let mut rng = Rng::new(0xC00D);
    // bits = 512 makes bytes = 64, so the reduced pos equals the original pos
    // and the loop length is exactly `pos` — sweeping pos sweeps the OOB depth.
    for pos in 1..=63u32 {
        for _ in 0..12 {
            // Fully randomized backing region: the stray reads land on bytes the
            // test controls and initialises identically for both libraries.
            let st = StateBuilder::random(&mut rng).pos(pos).build();
            let sm = vec![0u8; SAMPLES_LEN];
            let src = st.clone();
            run_diff(
                &format!("err13/pos={pos} depth=buffer[{}]", 64 + pos - 1),
                &st,
                &sm,
                &|api, s, _| unsafe {
                    (api.md5_addsample)(s, 512, 0);
                    let after = std::slice::from_raw_parts(s, STATE_LEN);
                    // buffer[0..pos] must equal the PRE-CALL buffer[64..64+pos],
                    // except where the 8-byte pack of `val`=0 already landed.
                    let p = pos as usize;
                    for k in 0..p {
                        let mut expect = src[OFF_BUFFER + 64 + k];
                        // pack wrote zeros to buffer[pos .. pos+8]
                        if 64 + k >= p && 64 + k < p + 8 {
                            expect = 0;
                        }
                        assert_eq!(
                            after[OFF_BUFFER + k], expect,
                            "{}: carry-down buffer[{k}] <- buffer[{}]",
                            api.which,
                            64 + k
                        );
                    }
                },
            );
        }
    }
}

// ===========================================================================
// ERRORS.md row 14 — update_md5(NULL, samples)
// ===========================================================================
#[test]
fn err14_update_t_null() {
    assert_null_case_matches("update_t_null");
    assert_null_case_matches("update_both_null");
}

// ===========================================================================
// ERRORS.md row 15 — update_md5(t, NULL)
// ===========================================================================
#[test]
fn err15_update_samples_null() {
    assert_null_case_matches("update_samples_null");
}

// ===========================================================================
// ERRORS.md row 16 — update_md5 product < 40 ⇒ b underflows, no rejection
// ===========================================================================
#[test]
fn err16_update_underflow_return() {
    let mut rng = Rng::new(0xC010);
    let mut pairs: Vec<(u32, u32)> = vec![(0, 0), (0, 5), (5, 0), (1, 1), (4, 8), (39, 1), (1, 39)];
    for a in 0..40u32 {
        pairs.push((a, 1));
    }
    for (cb, ch) in pairs {
        for _ in 0..8 {
            let st = StateBuilder::random(&mut rng)
                .pos(0)
                .cur_blocksize(cb)
                .channels(ch)
                .build();
            let sm = random_samples(&mut rng);
            let expect = cb.wrapping_mul(ch).wrapping_sub(40);
            run_diff(
                &format!("err16/cb={cb} ch={ch}"),
                &st,
                &sm,
                &|api, s, sam| unsafe {
                    let r = (api.update_md5)(s, sam);
                    assert_eq!(r, expect, "{}: expected wrapped {expect:#x}", api.which);
                    r
                },
            );
        }
    }
}

// ===========================================================================
// ERRORS.md row 17 — update_md5 product == 40 ⇒ returns 0 (ambiguous with "done")
// ===========================================================================
#[test]
fn err17_update_product_exactly_40() {
    let mut rng = Rng::new(0xC011);
    for (cb, ch) in [(1u32, 40u32), (2, 20), (4, 10), (5, 8), (8, 5), (10, 4), (20, 2), (40, 1)] {
        for _ in 0..20 {
            let st = StateBuilder::random(&mut rng)
                .pos(0)
                .cur_blocksize(cb)
                .channels(ch)
                .build();
            let sm = random_samples(&mut rng);
            run_diff(
                &format!("err17/cb={cb} ch={ch}"),
                &st,
                &sm,
                &|api, s, sam| unsafe {
                    let r = (api.update_md5)(s, sam);
                    assert_eq!(r, 0, "{}: must return 0", api.which);
                    r
                },
            );
        }
    }
}

// ===========================================================================
// ERRORS.md row 18 — update_md5 product overflows u32
// ===========================================================================
#[test]
fn err18_update_product_overflow() {
    let mut rng = Rng::new(0xC012);
    let mut pairs: Vec<(u32, u32)> = vec![
        (0x1_0000, 0x1_0000),
        (u32::MAX, 3),
        (u32::MAX, u32::MAX),
        (0x8000_0000, 2),
        (0x8000_0000, 3),
        (0xFFFF, 0x1_0001),
    ];
    for _ in 0..60 {
        pairs.push((rng.next_u32(), rng.next_u32()));
    }
    for (cb, ch) in pairs {
        let st = StateBuilder::random(&mut rng)
            .pos(0)
            .cur_blocksize(cb)
            .channels(ch)
            .build();
        let sm = random_samples(&mut rng);
        let expect = cb.wrapping_mul(ch).wrapping_sub(40);
        run_diff(
            &format!("err18/cb={cb} ch={ch}"),
            &st,
            &sm,
            &|api, s, sam| unsafe {
                let r = (api.update_md5)(s, sam);
                assert_eq!(r, expect, "{}: product must wrap mod 2^32", api.which);
                r
            },
        );
    }
}

// ===========================================================================
// ERRORS.md row 19 — update_md5 reads 136 elements with no length parameter:
//   a caller who supplies fewer gets a silent over-read of adjacent memory.
//   Made deterministic by giving the "short array" a controlled tail.
// ===========================================================================
#[test]
fn err19_update_over_reads_short_array() {
    let mut rng = Rng::new(0xC013);
    for short_len in [0usize, 1, 7, 8, 9, 32, 40, 64, 100, 135] {
        for _ in 0..20 {
            // The caller's "array" is only `short_len` elements; everything past
            // it is adjacent memory, which the test fills identically for both
            // libraries so the over-read is observable and comparable.
            let mut xs = vec![0i32; SAMPLES_ELEMS];
            for (i, x) in xs.iter_mut().enumerate() {
                *x = if i < short_len { rng.next_i32() } else { rng.next_i32() };
            }
            let sm = samples_from_i32(&xs);
            let st = StateBuilder::random(&mut rng)
                .pos(0)
                .cur_blocksize(short_len.max(1) as u32)
                .channels(1)
                .build();
            run_diff(
                &format!("err19/short_len={short_len}"),
                &st,
                &sm,
                &|api, s, sam| unsafe { (api.update_md5)(s, sam) },
            );
        }
    }
}

// ===========================================================================
// ERRORS.md row 20 — negative samples: sign-extend then mask ⇒ -1 == 0xFF
// ===========================================================================
#[test]
fn err20_update_negative_samples_alias() {
    let mut rng = Rng::new(0xC014);
    for _ in 0..N {
        // Two arrays that differ only above the low byte, one negative.
        let low: Vec<i32> = (0..SAMPLES_ELEMS).map(|_| (rng.next_u32() & 0xFF) as i32).collect();
        let neg: Vec<i32> = low.iter().map(|&x| (x as u32 | 0xFFFF_FF00) as i32).collect();

        let st = StateBuilder::random(&mut rng)
            .pos(0)
            .cur_blocksize(rng.range(1, 4096))
            .channels(rng.range(1, 8))
            .build();
        let sm_low = samples_from_i32(&low);
        let sm_neg = samples_from_i32(&neg);

        run_diff("err20/low", &st, &sm_low, &|api, s, sam| unsafe {
            (api.update_md5)(s, sam)
        });
        run_diff("err20/neg", &st, &sm_neg, &|api, s, sam| unsafe {
            (api.update_md5)(s, sam)
        });

        // Both libraries must ALSO agree that the two arrays are indistinguishable.
        for api in [&apis().0, &apis().1] {
            let mut a = Region::from_bytes(&st);
            let mut sa = Region::from_bytes(&sm_low);
            let ra = unsafe { (api.update_md5)(a.as_mut_ptr(), sa.as_mut_ptr() as *const i32) };
            let mut b = Region::from_bytes(&st);
            let mut sb = Region::from_bytes(&sm_neg);
            let rb = unsafe { (api.update_md5)(b.as_mut_ptr(), sb.as_mut_ptr() as *const i32) };
            assert_eq!(ra, rb, "{}: sign-extension aliasing", api.which);
            assert_eq!(a.bytes(), b.bytes(), "{}: sign-extension aliasing state", api.which);
        }
    }
}

// ===========================================================================
// ERRORS.md row 21 — pos >= 56 so the OOB carry-down fires INSIDE update_md5's
//                    5-iteration loop
// ===========================================================================
#[test]
fn err21_update_carrydown_inside_loop() {
    let mut rng = Rng::new(0xC015);
    for pos in 56..=127u32 {
        for _ in 0..6 {
            let st = StateBuilder::random(&mut rng)
                .pos(pos)
                .total(rng.next_u64())
                .cur_blocksize(rng.range(1, 4096))
                .channels(rng.range(1, 8))
                .build();
            let sm = random_samples(&mut rng);
            run_diff(
                &format!("err21/pos={pos}"),
                &st,
                &sm,
                &|api, s, sam| unsafe { (api.update_md5)(s, sam) },
            );
        }
    }
}

// ===========================================================================
// Generic FFI-boundary fuzz: there are no enum parameters in this API
// (`grep enum c_src/` ⇒ 0 matches), so the equivalent "value with no valid
// variant" input is an arbitrary u32 in the `bits` parameter and an arbitrary
// u32 in `pos`/`cur_blocksize`/`channels`. Sweep them fully at random.
// ===========================================================================
#[test]
fn err22_full_range_scalar_fuzz() {
    let mut rng = Rng::new(0xC016);
    for i in 0..2000 {
        let bits = rng.next_u32();
        let val = rng.next_u64();
        let pos = rng.next_u32();
        let total = rng.next_u64();
        let st = StateBuilder::random(&mut rng)
            .pos(pos)
            .total(total)
            .cur_blocksize(rng.next_u32())
            .channels(rng.next_u32())
            .build();
        let sm = random_samples(&mut rng);
        run_diff(
            &format!("err22/{i} bits={bits:#x} pos={pos:#x}"),
            &st,
            &sm,
            &|api, s, sam| unsafe {
                (api.md5_addsample)(s, bits, val);
                let a = (api.update_md5)(s, sam);
                (api.pack_u64le)(s.add(OFF_BUFFER + (val % 65) as usize), val);
                let b = (api.update_md5)(s, sam.add(64));
                (a, b)
            },
        );
    }
}
