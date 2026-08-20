//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! `bitwriter_add` has a single unconditional `return 0;` and no validation, so
//! most rows assert the *absence* of a rejection: both implementations must
//! return exactly `0` **and** agree on the full struct post-state. A Rust
//! translation that added a range check, saturated an overflow, or panicked on
//! the `63 - bw->bits` underflow would fail these tests.
//!
//! Row 13 (`bw == NULL`) is verified out-of-process because the expected
//! outcome is a `SIGSEGV`.

mod common;

use common::{load_c, load_pair, load_rust, Bw, Pair, Rng};
use std::ffi::c_int;

/// Assert both sides return exactly `0` (no error signalling) and agree.
fn assert_no_rejection(p: &Pair, ctx: &str, pre: &Bw, bits: u32, val: u64) {
    let (c, r) = p.call(pre, bits, val);
    assert_eq!(
        c.rc, 0,
        "[{ctx}] C returned {} instead of 0 — the C source has exactly one \
         `return 0;` so this can never happen (bits={bits}, val={val:#x})",
        c.rc
    );
    assert_eq!(
        r.rc, 0,
        "[{ctx}] Rust returned {} but C returns 0 — Rust invented an error path \
         (bits={bits}, val={val:#x})",
        r.rc
    );
    p.assert_same(ctx, pre, bits, val);
}

fn pre(rng: &mut Rng, bwbits: u32) -> Bw {
    Bw {
        val: rng.interesting_u64(),
        bits: bwbits,
        pos: rng.next_u32(),
        len: rng.next_u32(),
        tot: rng.next_u32(),
        buffer: rng.next_u64() as *mut u8,
    }
}

// ---------------------------------------------------------------------------
// Row 1 — nominal success path returns 0
// ---------------------------------------------------------------------------

#[test]
fn err01_nominal_returns_zero() {
    let p = load_pair();
    let mut rng = Rng::new(0xE001);
    for _ in 0..50_000 {
        let bwbits = rng.range(0, 63);
        let bits = rng.range(0, 63 - bwbits);
        let s = pre(&mut rng, bwbits);
        assert_no_rejection(&p, "err01 nominal", &s, bits, rng.interesting_u64());
    }
}

// ---------------------------------------------------------------------------
// Row 2 — bits == 0 (degenerate width, out-of-range `64 - bits` shift)
// ---------------------------------------------------------------------------

#[test]
fn err02_bits_zero_not_rejected() {
    let p = load_pair();
    let mut rng = Rng::new(0xE002);
    for bwbits in [0u32, 1, 31, 62, 63, 64, 65, 128, u32::MAX] {
        for _ in 0..2_000 {
            let s = pre(&mut rng, bwbits);
            assert_no_rejection(&p, "err02 bits=0", &s, 0, rng.interesting_u64());
        }
    }
}

// ---------------------------------------------------------------------------
// Row 3 — bits == 64 (exactly 8 * sizeof(tflac_uint))
// ---------------------------------------------------------------------------

#[test]
fn err03_bits_equal_width_not_rejected() {
    let p = load_pair();
    let mut rng = Rng::new(0xE003);
    for bwbits in [0u32, 1, 31, 62, 63, 64, 65, 128, u32::MAX] {
        for _ in 0..2_000 {
            let s = pre(&mut rng, bwbits);
            assert_no_rejection(&p, "err03 bits=64", &s, 64, rng.interesting_u64());
        }
    }
}

// ---------------------------------------------------------------------------
// Row 4 — bits == 65, one step past the maximum representable width
// ---------------------------------------------------------------------------

#[test]
fn err04_bits_one_past_max_not_rejected() {
    let p = load_pair();
    let mut rng = Rng::new(0xE004);
    for bits in [65u32, 66, 67, 96, 127, 128, 129] {
        for bwbits in [0u32, 1, 63, 64, 65, u32::MAX] {
            for _ in 0..500 {
                let s = pre(&mut rng, bwbits);
                assert_no_rejection(&p, "err04 bits>64", &s, bits, rng.interesting_u64());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 5 — bits == UINT32_MAX (grossly oversized width)
// ---------------------------------------------------------------------------

#[test]
fn err05_bits_uint32_max_not_rejected() {
    let p = load_pair();
    let mut rng = Rng::new(0xE005);
    for bits in [u32::MAX, u32::MAX - 1, u32::MAX - 63, 0x8000_0000, 0xFFFF_0000] {
        for bwbits in [0u32, 1, 63, 64, 65, 1000, u32::MAX] {
            for _ in 0..300 {
                let s = pre(&mut rng, bwbits);
                assert_no_rejection(&p, "err05 bits=UINT32_MAX", &s, bits, rng.interesting_u64());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 6 — `bw->bits + bits` wraps past 2^32 into `< 64`, loop skipped
// ---------------------------------------------------------------------------

#[test]
fn err06_guard_sum_wraparound_not_rejected() {
    let p = load_pair();
    let mut rng = Rng::new(0xE006);
    for bwbits in [64u32, 65, 128, 4096, 0x8000_0000, u32::MAX] {
        for target in 0u32..64 {
            let bits = target.wrapping_sub(bwbits);
            for _ in 0..4 {
                let s = pre(&mut rng, bwbits);
                assert_no_rejection(&p, "err06 guard wraparound", &s, bits, rng.interesting_u64());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 7 — bw->bits == 64: `63 - bw->bits` underflows, must NOT be rejected
// ---------------------------------------------------------------------------

#[test]
fn err07_bwbits_64_underflow_not_rejected() {
    let p = load_pair();
    let mut rng = Rng::new(0xE007);
    for bits in [0u32, 1, 2, 32, 63, 64, 65, 128, u32::MAX] {
        for _ in 0..1_000 {
            let s = pre(&mut rng, 64);
            assert_no_rejection(&p, "err07 bw->bits=64 underflow", &s, bits, rng.interesting_u64());
        }
    }
}

// ---------------------------------------------------------------------------
// Row 8 — bw->bits == UINT32_MAX
// ---------------------------------------------------------------------------

#[test]
fn err08_bwbits_uint32_max_not_rejected() {
    let p = load_pair();
    let mut rng = Rng::new(0xE008);
    for bwbits in [u32::MAX, u32::MAX - 1, 0x8000_0000, 0xFFFF_FF00] {
        for bits in [0u32, 1, 63, 64, 65, u32::MAX] {
            for _ in 0..300 {
                let s = pre(&mut rng, bwbits);
                assert_no_rejection(&p, "err08 bw->bits=UINT32_MAX", &s, bits, rng.interesting_u64());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 9 — the `i < 100` cap is reached and is NOT reported as an error
// ---------------------------------------------------------------------------

#[test]
fn err09_iteration_cap_is_not_an_error() {
    let p = load_pair();
    let mut rng = Rng::new(0xE009);
    // bw->bits == 63, bits == 1 => b == 0 => 100 no-progress spins.
    for _ in 0..20_000 {
        let s = pre(&mut rng, 63);
        assert_no_rejection(&p, "err09 cap (bw->bits=63,bits=1)", &s, 1, rng.interesting_u64());
    }
    // Other stalling shapes.
    for bwbits in [63u32, 64, 65, 1000, u32::MAX] {
        for bits in [0u32, 1, 2, 64, u32::MAX] {
            for _ in 0..400 {
                let s = pre(&mut rng, bwbits);
                assert_no_rejection(&p, "err09 cap (other)", &s, bits, rng.interesting_u64());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 10 — bw->tot overflow silently wraps
// ---------------------------------------------------------------------------

#[test]
fn err10_tot_overflow_not_rejected() {
    let p = load_pair();
    let mut rng = Rng::new(0xE010);
    for tot in [u32::MAX, u32::MAX - 1, u32::MAX - 63, 0xFFFF_FF00] {
        for bits in [1u32, 2, 63, 64, 65, 128, u32::MAX] {
            for bwbits in [0u32, 63, 64, u32::MAX] {
                for _ in 0..40 {
                    let mut s = pre(&mut rng, bwbits);
                    s.tot = tot;
                    assert_no_rejection(&p, "err10 tot overflow", &s, bits, rng.interesting_u64());
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 11 — capacity violation (pos > len, len == 0) is NOT rejected
// ---------------------------------------------------------------------------

#[test]
fn err11_capacity_violation_not_rejected() {
    let p = load_pair();
    let mut rng = Rng::new(0xE011);
    for &(pos, len) in &[
        (0u32, 0u32),
        (1, 0),
        (u32::MAX, 0),
        (u32::MAX, u32::MAX - 1),
        (0xFFFF_FFFF, 1),
        (10, 3),
    ] {
        for bits in [0u32, 1, 8, 64, 65, u32::MAX] {
            for bwbits in [0u32, 63, 64, u32::MAX] {
                for _ in 0..40 {
                    let mut s = pre(&mut rng, bwbits);
                    s.pos = pos;
                    s.len = len;
                    assert_no_rejection(
                        &p,
                        "err11 capacity violation",
                        &s,
                        bits,
                        rng.interesting_u64(),
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 12 — buffer == NULL (no output buffer) is NOT rejected
// ---------------------------------------------------------------------------

#[test]
fn err12_null_buffer_not_rejected() {
    let p = load_pair();
    let mut rng = Rng::new(0xE012);
    for bits in [0u32, 1, 8, 63, 64, 65, 128, u32::MAX] {
        for bwbits in [0u32, 1, 62, 63, 64, 65, u32::MAX] {
            for _ in 0..200 {
                let mut s = pre(&mut rng, bwbits);
                s.buffer = std::ptr::null_mut();
                assert_no_rejection(&p, "err12 buffer=NULL", &s, bits, rng.interesting_u64());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 13 — bw == NULL: both sides must die with the SAME signal (SIGSEGV)
// ---------------------------------------------------------------------------

/// Env var telling the re-executed child which `.so` to null-deref.
const NULL_ENV: &str = "BITWRITER_NULL_DEREF_TARGET";

/// Child half of row 13. When `BITWRITER_NULL_DEREF_TARGET` is set this test
/// performs the forbidden call and is expected to be killed by a signal; when
/// the variable is absent it is an inert no-op so a normal `cargo test` run is
/// unaffected.
#[test]
fn err13_null_pointer_child_worker() {
    let Ok(which) = std::env::var(NULL_ENV) else {
        return; // parent-side run: nothing to do
    };
    let imp = match which.as_str() {
        "c" => load_c(),
        "rust" => load_rust(),
        other => panic!("unknown {NULL_ENV}={other}"),
    };
    let f = imp.f;
    // `bw->tot += bits` dereferences address 0x14 unconditionally.
    let rc: c_int = unsafe { f(std::ptr::null_mut(), 8, 0xDEAD_BEEF) };
    // Reaching here means no fault occurred; report it distinguishably.
    println!("NO_FAULT rc={rc}");
    std::process::exit(77);
}

#[derive(Debug, PartialEq, Eq)]
struct Death {
    signal: Option<i32>,
    code: Option<i32>,
}

fn run_null_deref(which: &str) -> Death {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().expect("current_exe");
    let out = std::process::Command::new(exe)
        .args(["--exact", "err13_null_pointer_child_worker", "--test-threads=1", "--nocapture"])
        .env(NULL_ENV, which)
        .env("RUST_BACKTRACE", "0")
        .output()
        .expect("failed to re-exec test binary");
    Death { signal: out.status.signal(), code: out.status.code() }
}

#[test]
fn err13_null_pointer_same_fatal_signal() {
    // Make sure both `.so`s are loadable before we go fault-hunting.
    let _ = load_pair();

    let c = run_null_deref("c");
    let r = run_null_deref("rust");

    assert_eq!(
        c, r,
        "ERRORS.md row 13: C and Rust must fail identically on a NULL `bw`. \
         C = {c:?}, Rust = {r:?}"
    );
    // And it must actually be the expected fatal memory fault, not exit(77).
    assert_eq!(
        c.signal,
        Some(libc_sigsegv()),
        "expected SIGSEGV from the unconditional `bw->tot += bits` deref, got {c:?}"
    );
    assert_ne!(c.code, Some(77), "row 13: the null deref did not fault at all");
}

fn libc_sigsegv() -> i32 {
    11 // SIGSEGV on Linux
}

// ---------------------------------------------------------------------------
// Row 14 — misaligned `bw` pointer
// ---------------------------------------------------------------------------

#[test]
fn err14_misaligned_struct_pointer() {
    // Hand both implementations a deliberately odd-addressed struct.
    let c = load_c();
    let r = load_rust();
    let mut rng = Rng::new(0xE014);
    let mut backing = vec![0u8; 128];

    for off in [1usize, 2, 3, 4, 5, 6, 7] {
        for bits in [0u32, 1, 13, 64, 65, u32::MAX] {
            for bwbits in [0u32, 63, 64, u32::MAX] {
                let s = pre(&mut rng, bwbits);
                let val = rng.interesting_u64();
                let src = s.bytes();

                // C run.
                backing[..].fill(0);
                backing[off..off + 32].copy_from_slice(&src);
                let cp = unsafe { backing.as_mut_ptr().add(off) } as *mut Bw;
                let crc = unsafe { (c.f)(cp, bits, val) };
                let mut cpost = [0u8; 32];
                cpost.copy_from_slice(&backing[off..off + 32]);
                let cguard = (backing[..off].to_vec(), backing[off + 32..].to_vec());

                // Rust run.
                backing[..].fill(0);
                backing[off..off + 32].copy_from_slice(&src);
                let rp = unsafe { backing.as_mut_ptr().add(off) } as *mut Bw;
                let rrc = unsafe { (r.f)(rp, bits, val) };
                let mut rpost = [0u8; 32];
                rpost.copy_from_slice(&backing[off..off + 32]);
                let rguard = (backing[..off].to_vec(), backing[off + 32..].to_vec());

                assert_eq!(crc, rrc, "err14 rc mismatch (off={off} bits={bits})");
                assert_eq!(
                    cpost, rpost,
                    "err14 post-state mismatch (off={off} bits={bits} bw->bits={bwbits})"
                );
                assert_eq!(cguard, rguard, "err14 out-of-struct write mismatch (off={off})");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 15 — no enum parameters exist; the integer domains are already total
// ---------------------------------------------------------------------------

/// `ERRORS.md` row 15: the C API declares no `enum`, so there is no "invalid
/// variant" to smuggle across the FFI boundary. The nearest equivalent is
/// passing values with no meaningful interpretation in each integer parameter,
/// which this test does exhaustively at the extremes of both domains.
#[test]
fn err15_no_enum_params_full_integer_domain() {
    let p = load_pair();
    let extremes_u32: [u32; 12] = [
        0,
        1,
        2,
        63,
        64,
        65,
        0x7FFF_FFFF,
        0x8000_0000,
        0x8000_0001,
        0xFFFF_FFFE,
        u32::MAX,
        0xDEAD_BEEF,
    ];
    let extremes_u64: [u64; 8] = [
        0,
        1,
        u64::MAX,
        u64::MAX - 1,
        1u64 << 63,
        (1u64 << 63) | 1,
        0x7FFF_FFFF_FFFF_FFFF,
        0xDEAD_BEEF_DEAD_BEEF,
    ];
    for &bits in &extremes_u32 {
        for &bwbits in &extremes_u32 {
            for &val in &extremes_u64 {
                for &bwval in &extremes_u64 {
                    let s = Bw {
                        val: bwval,
                        bits: bwbits,
                        pos: u32::MAX,
                        len: u32::MAX,
                        tot: u32::MAX,
                        buffer: usize::MAX as *mut u8,
                    };
                    assert_no_rejection(&p, "err15 full integer domain", &s, bits, val);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Generic boundary sweep required by Phase C, beyond the table
// ---------------------------------------------------------------------------

/// "Zero and oversized lengths, and values one step past a documented valid
/// range" — swept densely around every boundary constant in the source.
#[test]
fn err_generic_boundaries_one_step_past() {
    let p = load_pair();
    let mut rng = Rng::new(0xE0FF);
    // Every boundary the source mentions: 0, 1, 63, 64 (= 8*sizeof), 100 (loop
    // cap), 2^32-1 — plus one step either side of each.
    let mut probes: Vec<u32> = Vec::new();
    for anchor in [0u32, 1, 63, 64, 100, 128, 0x8000_0000, u32::MAX] {
        for d in -3i64..=3 {
            probes.push((anchor as i64).wrapping_add(d) as u32);
        }
    }
    probes.sort_unstable();
    probes.dedup();

    for &bits in &probes {
        for &bwbits in &probes {
            for _ in 0..3 {
                let mut s = pre(&mut rng, bwbits);
                s.tot = if rng.below(2) == 0 { u32::MAX } else { 0 };
                assert_no_rejection(&p, "err generic boundaries", &s, bits, rng.interesting_u64());
            }
        }
    }
}
