//! Phase C — error/rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`. The C library has no error codes, no
//! sentinels, no asserts and no enums, so "the same error/rejection" means:
//! the same *observable* result of the boundary condition — identical return
//! value and identical byte image of the mutated context — plus, for the
//! genuinely-UB null-pointer case, the same fatal signal in a forked child.

mod common;
use common::*;

const ZERO: [u8; BUF_LEN] = [0u8; BUF_LEN];

/* ------------------------------------------------------------------ */
/* Row 1 — the library's only branch: fold-down TAKEN                  */
/* ------------------------------------------------------------------ */

#[test]
fn err01_folddown_branch_taken() {
    let mut rng = Rng::new(0xE001);
    let mut hit = 0usize;
    for _ in 0..4000 {
        let buf = rng.fill_buf();
        let pos = 56 + rng.below(8); // 56..63 -> pos+8 >= 64
        let out = diff_addsample("err01", Md5Ctx::new(pos, rng.next_u64(), &buf), 64, rng.next_u64());
        assert!(out.pos() < 64, "err01: fold-down must leave pos < 64");
        hit += 1;
    }
    assert!(hit > 0);
}

/* ------------------------------------------------------------------ */
/* Row 2 — fold-down branch NOT taken                                  */
/* ------------------------------------------------------------------ */

#[test]
fn err02_folddown_branch_not_taken() {
    let mut rng = Rng::new(0xE002);
    for _ in 0..4000 {
        let buf = rng.fill_buf();
        let pos = rng.below(56); // pos + 8 < 64
        let out = diff_addsample("err02", Md5Ctx::new(pos, rng.next_u64(), &buf), 64, rng.next_u64());
        assert_eq!(out.pos(), pos + 8, "err02: pos must be the raw sum");
        // With pos < 56 the 8-byte store ends at index <= 62, so the spill-over
        // tail (buffer[64..72]) is never written and no fold-down happens.
        assert_eq!(
            &out.buffer()[64..72],
            &buf[64..72],
            "err02: tail must be untouched when the fold-down branch is not taken"
        );
    }
}

/* ------------------------------------------------------------------ */
/* Row 3 — fold-down with pos % 64 == 0 -> `while (bytes--)` count 0   */
/* ------------------------------------------------------------------ */

#[test]
fn err03_folddown_zero_length_copy() {
    let mut rng = Rng::new(0xE003);
    // Every (pos, bits) pair whose byte sum is an exact multiple of 64.
    let cases: [(u32, u32); 8] =
        [(56, 64), (0, 512), (32, 256), (48, 128), (64, 0), (128, 0), (63, 8), (1, 504)];
    for &(pos, bits) in &cases {
        for _ in 0..500 {
            let buf = rng.fill_buf();
            let before = Md5Ctx::new(pos, rng.next_u64(), &buf);
            let out = diff_addsample("err03", before, bits, rng.next_u64());
            if (pos.wrapping_add(bits / 8)) >= 64 && (pos.wrapping_add(bits / 8)) % 64 == 0 {
                assert_eq!(out.pos(), 0);
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/* Row 4 — bits == 0 (zero length)                                     */
/* ------------------------------------------------------------------ */

#[test]
fn err04_bits_zero() {
    let mut rng = Rng::new(0xE004);
    for &pos in &[0u32, 1, 8, 55, 56, 63, 64, 65, 127, 0xFFFF_FFFF] {
        for _ in 0..300 {
            let buf = rng.fill_buf();
            let total = rng.next_u64();
            let out = diff_addsample("err04", Md5Ctx::new(pos, total, &buf), 0, rng.next_u64());
            assert_eq!(out.total(), total, "err04: total must be unchanged when bits == 0");
            if pos < 64 {
                assert_eq!(out.pos(), pos, "err04: pos must be unchanged when bits == 0");
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/* Row 5 — bits not a multiple of 8 (silent truncation of bits/8)      */
/* ------------------------------------------------------------------ */

#[test]
fn err05_bits_not_multiple_of_8() {
    let mut rng = Rng::new(0xE005);
    for bits in [1u32, 2, 3, 4, 5, 6, 7, 9, 10, 15, 17, 23, 25, 31, 33, 39, 63, 65, 71, 127, 129] {
        for _ in 0..300 {
            let buf = rng.fill_buf();
            let total = rng.next_u64();
            let out = diff_addsample("err05", Md5Ctx::new(rng.below(70), total, &buf), bits, rng.next_u64());
            assert_eq!(
                out.total(),
                total.wrapping_add(bits as u64),
                "err05: total gains the untruncated bit count"
            );
        }
    }
}

/* ------------------------------------------------------------------ */
/* Row 6 — oversized bits (0xFFFFFFFF)                                 */
/* ------------------------------------------------------------------ */

#[test]
fn err06_bits_oversized() {
    let mut rng = Rng::new(0xE006);
    for bits in [u32::MAX, u32::MAX - 1, 0xFFFF_FFF8, 0xFFFF_FF00, 0x8000_0000, 0x8000_0007] {
        for &pos in &[0u32, 1, 56, 63, 64, 65, 0xFFFF_FFFF] {
            for _ in 0..100 {
                let buf = rng.fill_buf();
                diff_addsample("err06", Md5Ctx::new(pos, rng.next_u64(), &buf), bits, rng.next_u64());
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/* Row 7 — pos outside the documented [0,63] range                     */
/* ------------------------------------------------------------------ */

#[test]
fn err07_pos_out_of_range() {
    let mut rng = Rng::new(0xE007);
    // one step past the valid range, and far beyond it
    let positions = [64u32, 65, 66, 71, 72, 73, 127, 128, 129, 255, 256, 4095, 4096,
        0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFE, 0xFFFF_FFFF];
    for &pos in &positions {
        for &bits in &[0u32, 1, 8, 64, 65, u32::MAX] {
            for _ in 0..100 {
                let buf = rng.fill_buf();
                diff_addsample("err07", Md5Ctx::new(pos, rng.next_u64(), &buf), bits, rng.next_u64());
            }
        }
    }
    // exhaustive pos sweep 0..=256 at bits = 64
    for pos in 0..=256u32 {
        let buf = rng.fill_buf();
        diff_addsample("err07/sweep", Md5Ctx::new(pos, rng.next_u64(), &buf), 64, rng.next_u64());
    }
}

/* ------------------------------------------------------------------ */
/* Row 8 — pos + bits/8 overflows u32                                  */
/* ------------------------------------------------------------------ */

#[test]
fn err08_pos_add_overflows_u32() {
    let mut rng = Rng::new(0xE008);
    let cases: [(u32, u32); 8] = [
        (0xFFFF_FFFF, 64),   // -> 0x00000007, branch NOT taken
        (0xFFFF_FFFF, 8),    // -> 0x00000000
        (0xFFFF_FFF8, 64),   // -> 0x00000000
        (0xFFFF_FE00, 4096), // -> wraps
        (0xFFFF_FFFF, u32::MAX),
        (0x8000_0000, u32::MAX),
        (0xFFFF_FFFF, 512),
        (0xFFFF_FF00, 0xFFFF_FFFF),
    ];
    for &(pos, bits) in &cases {
        for _ in 0..300 {
            let buf = rng.fill_buf();
            let out = diff_addsample("err08", Md5Ctx::new(pos, rng.next_u64(), &buf), bits, rng.next_u64());
            assert_eq!(
                out.pos(),
                {
                    let s = pos.wrapping_add(bits / 8);
                    if s >= 64 { s % 64 } else { s }
                },
                "err08: unsigned wraparound semantics"
            );
        }
    }
}

/* ------------------------------------------------------------------ */
/* Row 9 — total overflows u64                                         */
/* ------------------------------------------------------------------ */

#[test]
fn err09_total_overflows_u64() {
    let mut rng = Rng::new(0xE009);
    for &total in &[u64::MAX, u64::MAX - 1, u64::MAX - 63, u64::MAX - 64, u64::MAX - 65,
        0xFFFF_FFFF_FFFF_FF00] {
        for &bits in &[1u32, 64, 65, 66, 256, u32::MAX] {
            for _ in 0..100 {
                let buf = rng.fill_buf();
                let out = diff_addsample("err09", Md5Ctx::new(rng.below(70), total, &buf), bits, rng.next_u64());
                assert_eq!(out.total(), total.wrapping_add(bits as u64));
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/* Row 10 — pack_u64le boundary values                                 */
/* ------------------------------------------------------------------ */

#[test]
fn err10_pack_boundary_values() {
    for pat in [&[0x00u8][..], &[0xFF][..], &[0xA5][..]] {
        let buf = PackBuf::new(pat);
        for off in 0..=64usize {
            diff_pack("err10/0", buf, off, 0);
            diff_pack("err10/max", buf, off, u64::MAX);
            diff_pack("err10/one", buf, off, 1);
            diff_pack("err10/msb", buf, off, 1u64 << 63);
        }
    }
}

/* ------------------------------------------------------------------ */
/* Row 11 — pack_u64le at the very end of the 72-byte buffer           */
/* ------------------------------------------------------------------ */

#[test]
fn err11_pack_at_buffer_end() {
    let mut rng = Rng::new(0xE011);
    for pat in [&[0x00u8][..], &[0xFF][..], &[0x5A][..]] {
        let buf = PackBuf::new(pat);
        for _ in 0..3000 {
            // offset 64 fills buffer[64..71] exactly; the 32-byte guard region
            // that follows must stay pristine in BOTH implementations.
            diff_pack("err11", buf, 64, rng.next_u64());
        }
    }
    // explicit guard check
    let p = pair();
    let base = PackBuf::new(&[0x11]);
    let mut cb = base;
    let mut rb = base;
    unsafe {
        (p.c.pack_u64le)(cb.as_mut_ptr().add(64), u64::MAX);
        (p.rs.pack_u64le)(rb.as_mut_ptr().add(64), u64::MAX);
    }
    assert_eq!(cb, rb);
    assert_eq!(&cb.0[72..], &[0x11u8; 32][..], "C wrote past the 72-byte buffer");
    assert_eq!(&rb.0[72..], &[0x11u8; 32][..], "Rust wrote past the 72-byte buffer");
}

/* ------------------------------------------------------------------ */
/* Row 12 — b == 0 -> return 0xFFFFFFD8 (NOT a sentinel)               */
/* ------------------------------------------------------------------ */

#[test]
fn err12_update_b_zero_returns_wrapped() {
    let samples = vec![0i32; UPDATE_MD5_MIN_SAMPLES];
    for &(cbs, ch) in &[(0u32, 0u32), (0, 1), (1, 0), (0, u32::MAX), (u32::MAX, 0), (0, 0x1000)] {
        let (ret, _) = diff_update("err12", TflacCtx::new(0, 0, &ZERO, cbs, ch), &samples);
        assert_eq!(ret, 0u32.wrapping_sub(40));
        assert_eq!(ret, 0xFFFF_FFD8);
    }
}

/* ------------------------------------------------------------------ */
/* Row 13 — 0 < b < 40 partial underflow                               */
/* ------------------------------------------------------------------ */

#[test]
fn err13_update_partial_underflow() {
    let mut rng = Rng::new(0xE013);
    for prod in 1..40u32 {
        for _ in 0..30 {
            let buf = rng.fill_buf();
            let samples = rng.samples(UPDATE_MD5_MIN_SAMPLES);
            let (ret, _) =
                diff_update("err13", TflacCtx::new(rng.below(70), rng.next_u64(), &buf, prod, 1), &samples);
            assert_eq!(ret, prod.wrapping_sub(40));
            assert!(ret > 0xFFFF_FF00, "err13: must have underflowed");
        }
    }
}

/* ------------------------------------------------------------------ */
/* Row 14 — cur_blocksize * channels overflows u32                     */
/* ------------------------------------------------------------------ */

#[test]
fn err14_update_product_overflow() {
    let mut rng = Rng::new(0xE014);
    for _ in 0..2000 {
        let cbs = rng.next_u32();
        let ch = rng.next_u32();
        let buf = rng.fill_buf();
        let samples = rng.samples(UPDATE_MD5_MIN_SAMPLES);
        let (ret, _) =
            diff_update("err14", TflacCtx::new(rng.below(70), rng.next_u64(), &buf, cbs, ch), &samples);
        assert_eq!(ret, cbs.wrapping_mul(ch).wrapping_sub(40));
    }
    for &(cbs, ch) in &[(0x1_0000u32, 0x1_0000u32), (u32::MAX, u32::MAX), (u32::MAX, 2),
        (0x8000_0000, 0x8000_0000), (0xFFFF, 0x1_0001)] {
        let samples = vec![-1i32; UPDATE_MD5_MIN_SAMPLES];
        let (ret, _) = diff_update("err14/fixed", TflacCtx::new(0, 0, &ZERO, cbs, ch), &samples);
        assert_eq!(ret, cbs.wrapping_mul(ch).wrapping_sub(40));
    }
}

/* ------------------------------------------------------------------ */
/* Row 15 — negative samples: sign-extension then & 0xFF               */
/* ------------------------------------------------------------------ */

#[test]
fn err15_update_negative_samples_sign_extension() {
    let n = UPDATE_MD5_MIN_SAMPLES;
    let cases: Vec<(&str, Vec<i32>)> = vec![
        ("all -1", vec![-1i32; n]),
        ("all INT32_MIN", vec![i32::MIN; n]),
        ("all INT32_MAX", vec![i32::MAX; n]),
        ("-256 (low byte 0)", vec![-256i32; n]),
        ("-255", vec![-255i32; n]),
        ("alternating", (0..n).map(|i| if i % 2 == 0 { i32::MIN } else { -1 }).collect()),
        ("descending", (0..n).map(|i| -1 - (i as i32) * 7).collect()),
    ];
    for (name, samples) in &cases {
        let (_, out) = diff_update(name, TflacCtx::new(0, 0, &ZERO, 4096, 2), samples);
        // Only the low byte of each sample may survive the `& 0xFF` mask.
        let expect_byte = (samples[0] as u32 & 0xFF) as u8;
        if samples.iter().all(|&s| (s as u32 & 0xFF) as u8 == expect_byte) {
            assert_eq!(
                out.md5().buffer()[0], expect_byte,
                "err15 [{name}]: only the low byte must survive"
            );
        }
    }
}

/* ------------------------------------------------------------------ */
/* Row 16 — the 136-sample minimum / 32-element stride                 */
/* ------------------------------------------------------------------ */

#[test]
fn err16_update_reads_exactly_136_samples() {
    let mut rng = Rng::new(0xE016);
    for _ in 0..500 {
        let buf = rng.fill_buf();
        let start = TflacCtx::new(rng.below(70), rng.next_u64(), &buf, rng.next_u32(), rng.next_u32());

        // Exactly-136 allocation: any read past index 135 would be a heap
        // overrun visible under ASan / valgrind and would differ between impls.
        let exact: Vec<i32> = rng.samples(UPDATE_MD5_MIN_SAMPLES);
        let (r_exact, s_exact) = diff_update("err16/exact", start, &exact);

        // Same 40 read slots, everything else poisoned -> identical result.
        let mut poisoned = vec![0i32; 1024];
        for i in 0..1024 {
            poisoned[i] = if i < UPDATE_MD5_MIN_SAMPLES && i % 32 < 8 {
                exact[i]
            } else {
                rng.next_i32()
            };
        }
        let (r_pois, s_pois) = diff_update("err16/poisoned", start, &poisoned);
        assert_eq!(r_exact, r_pois, "err16: a never-read slot changed the return value");
        assert_eq!(s_exact, s_pois, "err16: a never-read slot changed the context");
    }
}

/* ------------------------------------------------------------------ */
/* Row 17 — NULL pointers (undefined behaviour in C)                   */
/* ------------------------------------------------------------------ */

/// Runs `f` in a forked child and returns the raw `wait` status, so a fatal
/// signal can be compared between the two implementations without taking the
/// test harness down with it.
#[cfg(unix)]
fn fork_status(f: impl FnOnce()) -> i32 {
    unsafe {
        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            f();
            libc::_exit(0); // survived -> distinguishable from a crash
        }
        let mut status: i32 = 0;
        assert!(libc::waitpid(pid, &mut status, 0) == pid);
        // normalise to (signal, exit_code)
        if libc::WIFSIGNALED(status) {
            -libc::WTERMSIG(status)
        } else {
            libc::WEXITSTATUS(status)
        }
    }
}

#[cfg(unix)]
#[test]
fn err17_null_pointers_behave_identically() {
    let p = pair();

    let cases: Vec<(&str, i32, i32)> = vec![
        (
            "pack_u64le(NULL, n)",
            fork_status(|| unsafe { (p.c.pack_u64le)(std::ptr::null_mut(), 0xDEAD_BEEF) }),
            fork_status(|| unsafe { (p.rs.pack_u64le)(std::ptr::null_mut(), 0xDEAD_BEEF) }),
        ),
        (
            "md5_addsample(NULL, 64, v)",
            fork_status(|| unsafe { (p.c.md5_addsample)(std::ptr::null_mut(), 64, 1) }),
            fork_status(|| unsafe { (p.rs.md5_addsample)(std::ptr::null_mut(), 64, 1) }),
        ),
        (
            "update_md5(NULL, samples)",
            {
                let s = vec![0i32; UPDATE_MD5_MIN_SAMPLES];
                fork_status(|| unsafe {
                    (p.c.update_md5)(std::ptr::null_mut(), s.as_ptr());
                })
            },
            {
                let s = vec![0i32; UPDATE_MD5_MIN_SAMPLES];
                fork_status(|| unsafe {
                    (p.rs.update_md5)(std::ptr::null_mut(), s.as_ptr());
                })
            },
        ),
        (
            "update_md5(t, NULL)",
            {
                let mut t = TflacCtx::new(0, 0, &ZERO, 1, 1);
                fork_status(move || unsafe {
                    (p.c.update_md5)(t.as_mut_ptr(), std::ptr::null());
                })
            },
            {
                let mut t = TflacCtx::new(0, 0, &ZERO, 1, 1);
                fork_status(move || unsafe {
                    (p.rs.update_md5)(t.as_mut_ptr(), std::ptr::null());
                })
            },
        ),
    ];

    let debug_build = rust_so_is_debug();
    for (name, cs, rs) in cases {
        assert!(cs < 0, "err17 [{name}]: C survived a NULL deref (exit code {cs})");
        assert!(rs < 0, "err17 [{name}]: Rust survived a NULL deref (exit code {rs})");
        if debug_build {
            // An unoptimised Rust cdylib has `debug_assertions` on, so the
            // null dereference is caught and turned into an aborting panic
            // (SIGABRT) instead of the SIGSEGV the C takes. That is debug
            // instrumentation, not an ABI difference; the shipped (release)
            // artifact is asserted for exact signal equality below.
            eprintln!(
                "err17 [{name}]: debug cdylib — C signal {} / Rust signal {} (both fatal)",
                -cs, -rs
            );
        } else {
            assert_eq!(
                cs, rs,
                "err17 [{name}]: release build must die identically — \
                 C signal {} vs Rust signal {}",
                -cs, -rs
            );
            eprintln!("err17 [{name}]: both died with signal {}", -cs);
        }
    }
}

/* ------------------------------------------------------------------ */
/* Row 18 — out-of-range "enum" values                                 */
/* ------------------------------------------------------------------ */

#[test]
fn err18_no_enums_bits_is_the_unconstrained_int_parameter() {
    // The library declares no enums, so the equivalent class of input is the
    // unconstrained `tflac_u32 bits`. Exhaustively sweep every value in
    // 0..=1024 plus the top of the range, at several starting positions.
    let mut rng = Rng::new(0xE018);
    for bits in 0..=1024u32 {
        let buf = rng.fill_buf();
        diff_addsample("err18/sweep", Md5Ctx::new(rng.below(200), rng.next_u64(), &buf), bits, rng.next_u64());
    }
    for bits in (u32::MAX - 512)..=u32::MAX {
        let buf = rng.fill_buf();
        diff_addsample("err18/top", Md5Ctx::new(rng.below(200), rng.next_u64(), &buf), bits, rng.next_u64());
    }
}

/* ------------------------------------------------------------------ */
/* Generic boundaries required by Phase C                              */
/* ------------------------------------------------------------------ */

#[test]
fn errgen_exhaustive_pos_times_bytecount_grid() {
    // Full 2-D grid over the only branch condition in the library:
    // pos in 0..=71 (the whole buffer) x bits in {0,8,...,576}.
    let mut rng = Rng::new(0xE0FF);
    for pos in 0..=71u32 {
        for step in 0..=72u32 {
            let buf = rng.fill_buf();
            diff_addsample(
                "errgen/grid",
                Md5Ctx::new(pos, rng.next_u64(), &buf),
                step * 8,
                rng.next_u64(),
            );
        }
    }
}

#[test]
fn errgen_update_md5_full_field_grid() {
    // cur_blocksize x channels over interesting boundary values, crossed with
    // interesting initial md5 positions.
    let mut rng = Rng::new(0xE0FE);
    let vals = [0u32, 1, 2, 5, 8, 39, 40, 41, 0xFFFF, 0x1_0000, 0x8000_0000, u32::MAX];
    for &cbs in &vals {
        for &ch in &vals {
            for &pos in &[0u32, 56, 63, 64, 0xFFFF_FFFF] {
                let buf = rng.fill_buf();
                let samples = rng.samples(UPDATE_MD5_MIN_SAMPLES);
                let (ret, _) =
                    diff_update("errgen/grid", TflacCtx::new(pos, rng.next_u64(), &buf, cbs, ch), &samples);
                assert_eq!(ret, cbs.wrapping_mul(ch).wrapping_sub(40));
            }
        }
    }
}
