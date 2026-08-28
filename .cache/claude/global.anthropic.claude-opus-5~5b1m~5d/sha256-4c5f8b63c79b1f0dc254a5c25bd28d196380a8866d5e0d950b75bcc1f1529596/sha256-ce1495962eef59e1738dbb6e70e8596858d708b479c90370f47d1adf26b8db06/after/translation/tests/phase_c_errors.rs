//! Phase C — error/rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`. The C library has no error codes at all
//! (its only `return` is `return b;`), so "rejection" is either a fatal signal
//! on an invalid pointer (rows 1–4, checked in a child process so the *same*
//! signal can be compared) or silent wraparound/truncation whose exact result
//! must match bit-for-bit (rows 5–14).

mod common;

use common::*;
use std::os::unix::process::ExitStatusExt;
use std::process::Command;

const SAMPLE_LEN: usize = 160;

/* ================================================================== */
/* Rows 1–4 — NULL pointers (fatal, compared via child processes)      */
/* ================================================================== */

/// Helper: run this same test binary again, in a child process, asking it to
/// perform one null-pointer call against one of the two libraries. Returns
/// `(signal, exit_code)`.
fn run_null_child(target: &str) -> (Option<i32>, Option<i32>) {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args([
            "--exact",
            "child_null_call",
            "--ignored",
            "--test-threads=1",
            "--nocapture",
        ])
        .env("NULL_TARGET", target)
        .env("C_LIB_PATH", c_lib_path())
        .env("RUST_LIB_PATH", rust_lib_path())
        .output()
        .expect("spawn child");
    (out.status.signal(), out.status.code())
}

/// Not run directly — re-invoked by `run_null_child`.
#[test]
#[ignore]
fn child_null_call() {
    let target = match std::env::var("NULL_TARGET") {
        Ok(t) => t,
        Err(_) => return, // invoked by a plain `--ignored` run: do nothing
    };
    let api = both();
    let (lib, what) = target.split_once(':').expect("NULL_TARGET=<c|rs>:<what>");
    let a = if lib == "c" { api.c } else { api.rust };
    let mut samples = vec![0i32; SAMPLE_LEN];
    for (i, s) in samples.iter_mut().enumerate() {
        *s = i as i32;
    }
    let mut arena = Arena::zeroed();
    unsafe {
        match what {
            "pack" => (a.pack_u64le)(std::ptr::null_mut(), 0x0123_4567_89AB_CDEF),
            "addsample" => (a.addsample)(std::ptr::null_mut(), 64, 0xDEAD_BEEF),
            "update_t" => {
                (a.update_md5)(std::ptr::null_mut(), samples.as_ptr());
            }
            "update_samples" => {
                (a.update_md5)(arena.as_ptr(), std::ptr::null());
            }
            other => panic!("unknown NULL_TARGET what={other}"),
        }
    }
    // If we get here the call did NOT fault: report that distinctly.
    std::process::exit(42);
}

fn assert_same_fatal(what: &str) {
    let (csig, ccode) = run_null_child(&format!("c:{what}"));
    let (rsig, rcode) = run_null_child(&format!("rs:{what}"));
    assert_eq!(
        (csig, ccode),
        (rsig, rcode),
        "NULL {what}: C exited with signal={csig:?} code={ccode:?} but Rust exited with \
         signal={rsig:?} code={rcode:?}"
    );
    assert!(
        csig.is_some(),
        "NULL {what}: expected a fatal signal, got exit code {ccode:?} (42 == call returned)"
    );
}

#[test]
fn err_null_pack_u64le() {
    assert_same_fatal("pack");
}

#[test]
fn err_null_addsample() {
    assert_same_fatal("addsample");
}

#[test]
fn err_null_update_md5_t() {
    assert_same_fatal("update_t");
}

#[test]
fn err_null_update_md5_samples() {
    assert_same_fatal("update_samples");
}

/* ================================================================== */
/* Rows 5–14 — silent (non-fatal) rejections: exact result must match  */
/* ================================================================== */

fn pat(seed: u64) -> [u8; BUFFER_LEN] {
    let mut rng = Rng::new(seed);
    let mut b = [0u8; BUFFER_LEN];
    for (i, x) in b.iter_mut().enumerate() {
        *x = (i as u8).wrapping_mul(3) ^ (rng.next_u32() as u8);
    }
    b
}

/// Row 5 — `bits == 0`.
#[test]
fn err_bits_zero() {
    let mut rng = Rng::new(0xE005);
    for pos in [0u32, 1, 7, 8, 56, 63, 64, 65, 1000] {
        for i in 0..64 {
            let buf = pat(0xE005_0000 + pos as u64 + i as u64);
            let arena = tflac_arena(0xE005_1000 + i as u64, pos, rng.next_u64(), 0, 0, Some(&buf));
            diff_addsample(&format!("bits=0 pos={pos}"), &arena, 0, rng.interesting_u64());
        }
    }
}

/// Row 6 — `bits` not a multiple of 8 (truncating division).
#[test]
fn err_bits_not_multiple_of_8() {
    let mut rng = Rng::new(0xE006);
    let odd: Vec<u32> = vec![1, 2, 3, 4, 5, 6, 7, 9, 15, 17, 23, 31, 33, 63, 65, 71, 127, 129];
    for bits in odd {
        for pos in [0u32, 1, 7, 8, 56, 57, 63, 64, 130] {
            for i in 0..8 {
                let buf = pat(0xE006_0000 + bits as u64 * 31 + pos as u64 + i as u64);
                let arena =
                    tflac_arena(0xE006_1000 + i as u64, pos, rng.next_u64(), 0, 0, Some(&buf));
                diff_addsample(
                    &format!("bits={bits} (not %8) pos={pos}"),
                    &arena,
                    bits,
                    rng.interesting_u64(),
                );
            }
        }
    }
}

/// Row 7 — huge `bits`, including `u32::MAX` (one step past every sane range).
#[test]
fn err_bits_huge() {
    let mut rng = Rng::new(0xE007);
    let huge: Vec<u32> = vec![
        u32::MAX,
        u32::MAX - 1,
        0x8000_0000,
        0x7FFF_FFFF,
        0xFFFF_FFF8,
        1 << 24,
        1 << 20,
        100_000,
        512,
        520,
    ];
    for bits in huge {
        for pos in [0u32, 1, 7, 8, 56, 63, 64, 1000, u32::MAX] {
            for i in 0..8 {
                let buf = pat(0xE007_0000 + bits as u64 + pos as u64 + i as u64);
                let arena =
                    tflac_arena(0xE007_1000 + i as u64, pos, rng.next_u64(), 0, 0, Some(&buf));
                diff_addsample(
                    &format!("bits={bits} pos={pos}"),
                    &arena,
                    bits,
                    rng.interesting_u64(),
                );
            }
        }
    }
}

/// Row 8 — `m->pos >= 64` on entry (never sanitised by the C).
#[test]
fn err_pos_out_of_range() {
    let mut rng = Rng::new(0xE008);
    for pos in [
        64u32, 65, 66, 71, 72, 73, 100, 127, 128, 129, 191, 192, 1000, 0xFFFF, 0x0100_0000,
    ] {
        for bits in [0u32, 8, 64, 65, 512] {
            for i in 0..8 {
                let buf = pat(0xE008_0000 + pos as u64 + bits as u64 + i as u64);
                let arena =
                    tflac_arena(0xE008_1000 + i as u64, pos, rng.next_u64(), 0, 0, Some(&buf));
                diff_addsample(
                    &format!("pos={pos} bits={bits}"),
                    &arena,
                    bits,
                    rng.interesting_u64(),
                );
            }
        }
    }
}

/// Row 9 — `m->pos` at/near `u32::MAX` so `pos += bytes` overflows.
#[test]
fn err_pos_u32_max() {
    let mut rng = Rng::new(0xE009);
    for pos in [
        u32::MAX,
        u32::MAX - 1,
        u32::MAX - 7,
        u32::MAX - 8,
        u32::MAX - 63,
        0xFFFF_FFC0,
    ] {
        for bits in [0u32, 8, 16, 64, 65, 512, u32::MAX] {
            for i in 0..8 {
                let buf = pat(0xE009_0000 + bits as u64 + i as u64);
                let arena =
                    tflac_arena(0xE009_1000 + i as u64, pos, rng.next_u64(), 0, 0, Some(&buf));
                diff_addsample(
                    &format!("pos={pos} bits={bits}"),
                    &arena,
                    bits,
                    rng.interesting_u64(),
                );
            }
        }
    }
}

/// Row 10 — `m->total` overflow.
#[test]
fn err_total_overflow() {
    let mut rng = Rng::new(0xE010);
    for total in [
        u64::MAX,
        u64::MAX - 1,
        u64::MAX - 63,
        u64::MAX - 64,
        u64::MAX - 65,
        0xFFFF_FFFF_FFFF_FFF0,
    ] {
        for bits in [0u32, 1, 8, 64, 65, u32::MAX] {
            for i in 0..8 {
                let buf = pat(0xE010_0000 + bits as u64 + i as u64);
                let arena = tflac_arena(
                    0xE010_1000 + i as u64,
                    rng.below(70),
                    total,
                    0,
                    0,
                    Some(&buf),
                );
                diff_addsample(
                    &format!("total={total:#x} bits={bits}"),
                    &arena,
                    bits,
                    rng.interesting_u64(),
                );
            }
        }
    }
}

/// Row 11 — `cur_blocksize * channels` overflows u32.
#[test]
fn err_b_product_overflow() {
    let mut rng = Rng::new(0xE011);
    let pairs = [
        (0x1000_0000u32, 0x11u32),
        (0xFFFF_FFFF, 2),
        (0xFFFF_FFFF, u32::MAX),
        (65536, 65536),
        (65537, 65537),
        (0x8000_0000, 2),
        (0xABCD_EF01, 0x1234_5678),
    ];
    for (bs, ch) in pairs {
        for i in 0..16 {
            let buf = pat(0xE011_0000 + i as u64);
            let arena = tflac_arena(
                0xE011_1000 + i as u64,
                rng.below(70),
                rng.next_u64(),
                bs,
                ch,
                Some(&buf),
            );
            let samples = random_samples(&mut rng, SAMPLE_LEN);
            diff_update(&format!("overflow bs={bs} ch={ch}"), &arena, &samples);
        }
    }
}

/// Row 12 — `b` underflow (`product < 40`, including 0).
#[test]
fn err_b_underflow() {
    let mut rng = Rng::new(0xE012);
    for (bs, ch) in [
        (0u32, 0u32),
        (0, 1),
        (1, 0),
        (1, 1),
        (2, 2),
        (8, 4),
        (39, 1),
        (1, 39),
        (0, u32::MAX),
        (u32::MAX, 0),
    ] {
        for i in 0..16 {
            let buf = pat(0xE012_0000 + bs as u64 + ch as u64 + i as u64);
            let arena = tflac_arena(
                0xE012_1000 + i as u64,
                rng.below(70),
                rng.next_u64(),
                bs,
                ch,
                Some(&buf),
            );
            let samples = random_samples(&mut rng, SAMPLE_LEN);
            diff_update(&format!("underflow bs={bs} ch={ch}"), &arena, &samples);
        }
    }
}

/// Row 13 — the fixed 5×stride-32 read pattern: exactly `samples[0..136]` are
/// touched. Compared against an oversized array so both libraries read the
/// same defined memory, and verified that element 136 onwards is irrelevant.
#[test]
fn err_samples_stride_reads() {
    let api = both();
    let mut rng = Rng::new(0xE013);
    for trial in 0..128 {
        let mut base = random_samples(&mut rng, 1024);
        let buf = pat(0xE013_0000 + trial as u64);
        let arena = tflac_arena(
            0xE013_1000 + trial as u64,
            rng.below(70),
            rng.next_u64(),
            rng.next_u32(),
            rng.below(9),
            Some(&buf),
        );
        diff_update("stride reads", &arena, &base);

        // Everything at/after index 136 must be irrelevant to both libs.
        let mut ca = arena.clone_arena();
        let r_c = unsafe { (api.c.update_md5)(ca.as_ptr(), base.as_ptr()) };
        for s in base[136..].iter_mut() {
            *s = rng.next_i32();
        }
        let mut ra = arena.clone_arena();
        let r_r = unsafe { (api.rust.update_md5)(ra.as_ptr(), base.as_ptr()) };
        assert_eq!(r_c, r_r, "samples[136..] must not influence the return value");
        assert_eq!(
            ca.bytes(),
            ra.bytes(),
            "samples[136..] must not influence the md5 state"
        );
    }
}

/// Row 14 — unaligned destination for `tflac_pack_u64le`.
#[test]
fn err_pack_unaligned() {
    let mut rng = Rng::new(0xE014);
    for off in 0usize..64 {
        for i in 0..32 {
            let arena = Arena::new(0xE014_0000 + off as u64 * 7 + i as u64);
            diff_pack(&format!("unaligned off={off}"), &arena, off, rng.interesting_u64());
        }
    }
    // Also the very last legal 8-byte window of the arena.
    for i in 0..64 {
        let arena = Arena::new(0xE014_9000 + i as u64);
        diff_pack("arena tail", &arena, ARENA_BYTES - 8, rng.interesting_u64());
        let arena2 = Arena::new(0xE014_A000 + i as u64);
        diff_pack("arena tail-3", &arena2, ARENA_BYTES - 11, rng.interesting_u64());
    }
}

/* ================================================================== */
/* Generic FFI boundary sweeps (beyond the table)                       */
/* ================================================================== */

/// No enum parameters exist in this API, so the "out-of-range enum variant"
/// class degenerates to "arbitrary `u32` in a parameter position". Sweep the
/// whole exponent range of `bits` plus randomized full-range values.
#[test]
fn err_full_range_bits_sweep() {
    let mut rng = Rng::new(0xE0FF);
    let mut bits_values: Vec<u32> = Vec::new();
    for b in 0..32 {
        bits_values.push(1u32 << b);
        bits_values.push((1u32 << b).wrapping_sub(1));
        bits_values.push((1u32 << b).wrapping_add(1));
    }
    for _ in 0..64 {
        bits_values.push(rng.next_u32());
    }
    for bits in bits_values {
        for pos in [0u32, 7, 63, 64, 0xFFFF_FFFF] {
            let buf = pat(0xE0FF_0000 + bits as u64);
            let arena = tflac_arena(0xE0FF_1000, pos, rng.next_u64(), 0, 0, Some(&buf));
            diff_addsample(
                &format!("sweep bits={bits} pos={pos}"),
                &arena,
                bits,
                rng.interesting_u64(),
            );
        }
    }
}

/// Row 15 — a *misaligned* `tflac` / `tflac_md5` pointer. The C ABI asks for
/// 8-byte alignment but the C code itself performs ordinary loads that work
/// unaligned on this target, so the Rust must not behave differently (e.g. a
/// caller handing over a `malloc`ed-then-offset or `Vec<u8>`-backed struct).
#[test]
fn err_misaligned_struct_pointer() {
    let mut rng = Rng::new(0xE015);
    // Sample bytes with a deliberately odd start offset as well.
    let mut sample_bytes = vec![0u8; 4 * 1024];
    for b in sample_bytes.iter_mut() {
        *b = rng.next_u32() as u8;
    }

    for struct_off in 1usize..=8 {
        for i in 0..16 {
            let mut arena = Arena::new(0xE015_0000 + struct_off as u64 * 97 + i as u64);
            // Lay the struct fields out by hand at `struct_off`.
            arena.set_u32_at(struct_off + OFF_POS, rng.below(200));
            arena.set_u64_at(struct_off + OFF_TOTAL, rng.next_u64());
            arena.set_u32_at(struct_off + OFF_CUR_BLOCKSIZE, rng.next_u32());
            arena.set_u32_at(struct_off + OFF_CHANNELS, rng.below(9));
            let buf: Vec<u8> = (0..BUFFER_LEN).map(|k| (k as u8) ^ 0x3C).collect();
            arena.set_bytes_at(struct_off + OFF_BUFFER, &buf);

            for bits in [0u32, 8, 64, 65, 512, u32::MAX] {
                diff_addsample_off(
                    &format!("misaligned struct off={struct_off}"),
                    &arena,
                    struct_off,
                    bits,
                    rng.interesting_u64(),
                );
            }
            for soff in [0usize, 1, 2, 3, 4, 7] {
                diff_update_off(
                    &format!("misaligned struct off={struct_off}"),
                    &arena,
                    struct_off,
                    &sample_bytes,
                    soff,
                );
            }
        }
    }
}

/// Zero-sized / degenerate `tflac` state combined with degenerate samples.
#[test]
fn err_degenerate_state() {
    let zeros = [0u8; BUFFER_LEN];
    let samples = vec![0i32; SAMPLE_LEN];
    for pos in [0u32, 63, 64, u32::MAX] {
        for total in [0u64, u64::MAX] {
            let mut arena = Arena::zeroed();
            arena.set_pos(pos);
            arena.set_total(total);
            arena.set_buffer(&zeros);
            arena.set_cur_blocksize(0);
            arena.set_channels(0);
            diff_update(&format!("degenerate pos={pos} total={total}"), &arena, &samples);
            diff_addsample(&format!("degenerate pos={pos} total={total}"), &arena, 0, 0);
            diff_addsample(
                &format!("degenerate pos={pos} total={total} bits=max"),
                &arena,
                u32::MAX,
                u64::MAX,
            );
        }
    }
}
