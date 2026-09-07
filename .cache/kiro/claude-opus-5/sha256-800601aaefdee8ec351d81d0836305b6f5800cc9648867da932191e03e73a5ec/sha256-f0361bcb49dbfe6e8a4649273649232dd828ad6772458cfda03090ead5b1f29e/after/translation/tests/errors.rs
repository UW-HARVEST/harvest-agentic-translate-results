//! Phase C — error/rejection-path differential tests.
//!
//! One `#[test]` per row of `ERRORS.md` (E1..E13), plus a negative control that
//! proves the harness can actually observe a divergence.
//!
//! E1..E3 (null pointers) are fatal by design: the C performs no validation, so
//! the process dies. They are compared by re-executing this same test binary in a
//! child process and asserting that the C `.so` and the Rust `.so` terminate with
//! the *same* signal — not merely that "both failed".

mod common;
use common::{c_impl, pair, rust_impl, Rng, CANARY, N, SEED};

use std::os::unix::process::ExitStatusExt;
use std::process::Command;

// ---------------------------------------------------------------------------
// Negative control: the harness must be able to see a difference.
// ---------------------------------------------------------------------------

/// Sanity: two distinct libraries really are loaded (different code addresses),
/// and a mismatched expectation really is detected. Without this, an "all green"
/// run could just mean the comparison is vacuous.
#[test]
fn e00_negative_control_harness_detects_divergence() {
    let c = c_impl();
    let r = rust_impl();
    assert_ne!(
        c.f as usize, r.f as usize,
        "C and Rust rgb_to_hsv resolved to the same address — only one library is loaded"
    );

    // Both must actually write something (dest must not stay at the canary).
    let src = [0.9f32, 0.3, 0.1];
    let mut dc = [f32::from_bits(CANARY); 3];
    let mut dr = [f32::from_bits(CANARY); 3];
    unsafe {
        (c.f)(dc.as_mut_ptr(), src.as_ptr());
        (r.f)(dr.as_mut_ptr(), src.as_ptr());
    }
    assert_ne!(dc[0].to_bits(), CANARY, "C never wrote dest[0]");
    assert_ne!(dr[0].to_bits(), CANARY, "Rust never wrote dest[0]");
    assert_eq!(dc.map(f32::to_bits), dr.map(f32::to_bits));

    // And the comparator must reject a deliberately corrupted result.
    let p = pair();
    let (mut cbits, rbits) = p.run(src);
    cbits[0] ^= 1; // perturb one bit
    assert_ne!(
        cbits, rbits,
        "bitwise comparator failed to notice a one-bit difference"
    );
}

// ---------------------------------------------------------------------------
// E1..E3 — null pointers (fatal; compared across child processes).
// ---------------------------------------------------------------------------

const NULL_CASE_ENV: &str = "HARVEST_NULL_CASE";

/// Executed only inside the re-exec'd child: perform the null-pointer call so the
/// parent can observe how the library terminates.
fn maybe_run_null_child() {
    let Ok(spec) = std::env::var(NULL_CASE_ENV) else {
        return;
    };
    let (which, case) = spec.split_once(':').expect("bad HARVEST_NULL_CASE");
    let f = match which {
        "c" => c_impl().f,
        "rust" => rust_impl().f,
        other => panic!("bad impl {other}"),
    };
    let mut dest = [0.0f32; 3];
    let src = [0.25f32, 0.5, 0.75];
    unsafe {
        match case {
            // src == NULL, dest valid
            "src" => f(dest.as_mut_ptr(), std::ptr::null()),
            // dest == NULL, src valid
            "dest" => f(std::ptr::null_mut(), src.as_ptr()),
            // both NULL
            "both" => f(std::ptr::null_mut(), std::ptr::null()),
            other => panic!("bad case {other}"),
        }
    }
    // If we somehow survive, report that distinctly.
    println!("SURVIVED {which}:{case} -> {dest:?}");
    std::process::exit(0);
}

#[derive(Debug, PartialEq, Eq)]
struct Termination {
    code: Option<i32>,
    signal: Option<i32>,
}

fn run_null_case(which: &str, case: &str) -> Termination {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args(["--exact", "--nocapture", "e_null_child_entrypoint"])
        .env(NULL_CASE_ENV, format!("{which}:{case}"))
        .output()
        .expect("spawn child");
    Termination {
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

/// The child entry point. In a normal (parent) run the env var is unset and this
/// test is a no-op; the parent re-execs it with `HARVEST_NULL_CASE` set.
#[test]
fn e_null_child_entrypoint() {
    maybe_run_null_child();
}

/// E1 — `src == NULL`: both libraries must die with the same signal.
#[test]
fn e01_null_src() {
    let c = run_null_case("c", "src");
    let r = run_null_case("rust", "src");
    assert_eq!(
        c.signal,
        Some(libc_sigsegv()),
        "C did not die with SIGSEGV on NULL src: {c:?}"
    );
    assert_eq!(c, r, "C and Rust terminated differently on NULL src");
}

/// E2 — `dest == NULL`: both must die with the same signal.
#[test]
fn e02_null_dest() {
    let c = run_null_case("c", "dest");
    let r = run_null_case("rust", "dest");
    assert_eq!(
        c.signal,
        Some(libc_sigsegv()),
        "C did not die with SIGSEGV on NULL dest: {c:?}"
    );
    assert_eq!(c, r, "C and Rust terminated differently on NULL dest");
}

/// E3 — both pointers NULL: dies on the `src[0]` load, same as E1.
#[test]
fn e03_null_both() {
    let c = run_null_case("c", "both");
    let r = run_null_case("rust", "both");
    assert_eq!(
        c.signal,
        Some(libc_sigsegv()),
        "C did not die with SIGSEGV on NULL/NULL: {c:?}"
    );
    assert_eq!(c, r, "C and Rust terminated differently on NULL/NULL");
    // Loads precede all stores, so this must look exactly like the NULL-src case.
    assert_eq!(c, run_null_case("c", "src"));
}

const fn libc_sigsegv() -> i32 {
    11
}

// ---------------------------------------------------------------------------
// E4/E5 — buffer window: exactly 3 elements read and written.
// ---------------------------------------------------------------------------

/// E4 — the 3-element access window. `dest[3..]` and `src[3..]` guard slots must
/// be left untouched by both libraries (no over-read, no over-write).
#[test]
fn e04_exact_three_element_window() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 104);
    const GUARD: usize = 5;
    for _ in 0..N {
        let core = [rng.range(-2.0, 2.0), rng.range(-2.0, 2.0), rng.range(-2.0, 2.0)];

        // src buffer: 3 real values + guard canaries after them.
        let mut src = [f32::from_bits(CANARY); 3 + GUARD];
        src[..3].copy_from_slice(&core);
        let src_snapshot = src;

        let mut dc = [f32::from_bits(CANARY); 3 + GUARD];
        let mut dr = [f32::from_bits(CANARY); 3 + GUARD];
        unsafe {
            (p.c.f)(dc.as_mut_ptr(), src.as_ptr());
            (p.rust.f)(dr.as_mut_ptr(), src.as_ptr());
        }

        // src is `const float *` — untouched by both.
        assert_eq!(
            src.map(f32::to_bits),
            src_snapshot.map(f32::to_bits),
            "a library mutated its const src buffer"
        );
        // Guards past index 2 untouched, identically, in both.
        for i in 3..3 + GUARD {
            assert_eq!(dc[i].to_bits(), CANARY, "C wrote past dest[2] at index {i}");
            assert_eq!(
                dr[i].to_bits(),
                CANARY,
                "Rust wrote past dest[2] at index {i}"
            );
        }
        assert_eq!(
            dc[..3].iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
            dr[..3].iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
            "divergence on {core:?}"
        );
    }
    eprintln!("[E4] {N} inputs: 3-element window respected identically");
}

/// E5 — oversized input: elements at index >= 3 are ignored, so padding them with
/// arbitrary garbage must not change either library's output.
#[test]
fn e05_oversized_src_ignored() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 105);
    for _ in 0..N {
        let core = [rng.raw_f32(), rng.raw_f32(), rng.raw_f32()];
        let (base_c, base_r) = p.run(core);
        assert_eq!(base_c, base_r, "baseline divergence on {core:?}");

        // Same 3 values, followed by 5 arbitrary junk floats.
        let mut padded = [0.0f32; 8];
        padded[..3].copy_from_slice(&core);
        for slot in padded[3..].iter_mut() {
            *slot = rng.raw_f32();
        }

        let mut dc = [f32::from_bits(CANARY); 3];
        let mut dr = [f32::from_bits(CANARY); 3];
        unsafe {
            (p.c.f)(dc.as_mut_ptr(), padded.as_ptr());
            (p.rust.f)(dr.as_mut_ptr(), padded.as_ptr());
        }
        assert_eq!(dc.map(f32::to_bits), base_c, "C used padding bytes");
        assert_eq!(dr.map(f32::to_bits), base_r, "Rust used padding bytes");
    }
    eprintln!("[E5] {N} inputs: trailing elements ignored by both");
}

// ---------------------------------------------------------------------------
// E6/E7 — aliasing and partial overlap (no `restrict` in the C signature).
// ---------------------------------------------------------------------------

/// E6 — `dest == src`: all loads precede all stores, so the in-place result must
/// equal the non-aliased result, identically in C and Rust.
#[test]
fn e06_full_aliasing_in_place() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 106);
    for _ in 0..N {
        let core = [rng.raw_f32(), rng.raw_f32(), rng.raw_f32()];
        let (base_c, base_r) = p.run(core);
        assert_eq!(base_c, base_r, "baseline divergence on {core:?}");

        let mut bc = core;
        let mut br = core;
        unsafe {
            (p.c.f)(bc.as_mut_ptr(), bc.as_ptr());
            (p.rust.f)(br.as_mut_ptr(), br.as_ptr());
        }
        assert_eq!(bc.map(f32::to_bits), br.map(f32::to_bits), "in-place divergence");
        assert_eq!(
            bc.map(f32::to_bits),
            base_c,
            "in-place result differs from non-aliased result (C)"
        );
    }
    eprintln!("[E6] {N} inputs: full aliasing matches");
}

/// E7 — partial overlap `dest = buf+1` / `dest = buf-1` (src at the other offset).
#[test]
fn e07_partial_overlap() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 107);
    for _ in 0..N {
        let core = [rng.range(-4.0, 4.0), rng.range(-4.0, 4.0), rng.range(-4.0, 4.0)];
        let (base, _) = p.run(core);

        // Layout: [pad, s0, s1, s2] with dest = &buf[0] (dest == src - 1).
        for shift in [0usize, 1usize] {
            let mut bc = [f32::from_bits(CANARY); 4];
            let mut br = [f32::from_bits(CANARY); 4];
            // src occupies indices 1..4, dest starts at `shift` (0 -> src-1, 1 -> src).
            bc[1..4].copy_from_slice(&core);
            br[1..4].copy_from_slice(&core);
            unsafe {
                (p.c.f)(bc.as_mut_ptr().add(shift), bc.as_ptr().add(1));
                (p.rust.f)(br.as_mut_ptr().add(shift), br.as_ptr().add(1));
            }
            assert_eq!(
                bc.map(f32::to_bits),
                br.map(f32::to_bits),
                "overlap(shift={shift}) divergence on {core:?}"
            );
            assert_eq!(
                [
                    bc[shift].to_bits(),
                    bc[shift + 1].to_bits(),
                    bc[shift + 2].to_bits()
                ],
                base,
                "overlap(shift={shift}) result differs from non-aliased result"
            );
            if shift == 1 {
                // dest == src exactly; the leading pad slot must be preserved.
                assert_eq!(bc[0].to_bits(), CANARY);
                assert_eq!(br[0].to_bits(), CANARY);
            } else {
                // dest = src-1; the trailing slot must be preserved.
                assert_eq!(bc[3].to_bits(), core[2].to_bits());
                assert_eq!(br[3].to_bits(), core[2].to_bits());
            }
        }
    }
    eprintln!("[E7] {N} inputs x 2 overlap layouts: matched");
}

// ---------------------------------------------------------------------------
// E8 — misaligned float pointers (no alignment check in the C).
// ---------------------------------------------------------------------------

/// E8 — pointers offset by 1..3 bytes. The C does no alignment check; on x86-64
/// the unaligned accesses succeed and both libraries must agree bit-for-bit.
#[test]
fn e08_misaligned_pointers() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 108);
    // 4-byte-aligned backing storage with slack for a 3-byte skew.
    #[repr(align(16))]
    struct Buf([u8; 32]);

    for _ in 0..N {
        let core = [rng.range(-3.0, 3.0), rng.range(-3.0, 3.0), rng.range(-3.0, 3.0)];
        let (base, _) = p.run(core);
        for off in 1usize..=3 {
            let mut sbuf = Buf([0u8; 32]);
            let mut dbc = Buf([0u8; 32]);
            let mut dbr = Buf([0u8; 32]);
            for (i, v) in core.iter().enumerate() {
                sbuf.0[off + i * 4..off + i * 4 + 4].copy_from_slice(&v.to_le_bytes());
            }
            unsafe {
                let sp = sbuf.0.as_ptr().add(off) as *const f32;
                (p.c.f)(dbc.0.as_mut_ptr().add(off) as *mut f32, sp);
                (p.rust.f)(dbr.0.as_mut_ptr().add(off) as *mut f32, sp);
            }
            let rd = |b: &Buf| -> [u32; 3] {
                let mut o = [0u32; 3];
                for i in 0..3 {
                    let mut w = [0u8; 4];
                    w.copy_from_slice(&b.0[off + i * 4..off + i * 4 + 4]);
                    o[i] = u32::from_le_bytes(w);
                }
                o
            };
            let (gc, gr) = (rd(&dbc), rd(&dbr));
            assert_eq!(gc, gr, "misaligned(off={off}) divergence on {core:?}");
            assert_eq!(
                gc, base,
                "misaligned(off={off}) result differs from aligned result"
            );
        }
    }
    eprintln!("[E8] {N} inputs x 3 byte offsets: matched");
}

// ---------------------------------------------------------------------------
// E9 — out-of-domain values (no range check exists).
// ---------------------------------------------------------------------------

/// E9 — components outside the nominal `[0,1]` RGB domain, including one step
/// past each endpoint. No rejection happens; `s`/`v` may leave `[0,1]`.
#[test]
fn e09_out_of_domain_no_range_check() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 109);

    // One-step-past-the-endpoints, exhaustively.
    let edge: [f32; 6] = [
        f32::from_bits(0x8000_0001),          // just below 0
        0.0,
        1.0,
        f32::from_bits(1.0f32.to_bits() + 1), // just above 1
        -1.0,
        2.0,
    ];
    let mut inputs = Vec::new();
    for &r in &edge {
        for &g in &edge {
            for &b in &edge {
                inputs.push([r, g, b]);
            }
        }
    }
    // Plus randomized far-out-of-range values.
    for _ in 0..N {
        inputs.push([
            rng.range(-1e6, 1e6),
            rng.range(-1e6, 1e6),
            rng.range(-1e6, 1e6),
        ]);
    }
    p.assert_batch("E9", inputs.clone());

    // And confirm the C really does produce out-of-nominal-range output for at
    // least one of these (i.e. there is genuinely no clamping to reject).
    let (out, _) = p.run([1.0, -1.0, 0.0]);
    let s = f32::from_bits(out[1]);
    assert!(s > 1.0, "expected unclamped s > 1, got {s}");
}

// ---------------------------------------------------------------------------
// E10..E13 — non-finite, arbitrary bit patterns, underflow/overflow.
// ---------------------------------------------------------------------------

/// E10 — NaN inputs, including a signalling NaN bit pattern, in every subset of
/// the three channels (all 7 non-empty masks).
#[test]
fn e10_nan_all_masks_incl_signalling() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 110);
    // quiet NaN, negative quiet NaN, signalling NaN (mantissa MSB clear), max payload
    let nans: [f32; 5] = [
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7F80_0001), // sNaN
        f32::from_bits(0xFF80_0001), // negative sNaN
        f32::from_bits(0x7FFF_FFFF), // qNaN, all payload bits set
    ];
    let mut inputs = Vec::new();
    for mask in 1u8..8 {
        for &nan in &nans {
            for _ in 0..N / 4 {
                let mut v = [0.0f32; 3];
                for i in 0..3 {
                    v[i] = if mask & (1 << i) != 0 {
                        nan
                    } else {
                        rng.range(-2.0, 2.0)
                    };
                }
                inputs.push(v);
            }
        }
    }
    p.assert_batch("E10", inputs);
}

/// E11 — infinities in every subset, both signs, exhaustively plus randomized.
#[test]
fn e11_infinities_all_masks() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 111);
    let infs = [f32::INFINITY, f32::NEG_INFINITY];
    let mut inputs = Vec::new();
    for mask in 1u8..8 {
        for &pos in &infs {
            for &neg in &infs {
                for _ in 0..N / 8 {
                    let mut v = [0.0f32; 3];
                    let mut alt = false;
                    for i in 0..3 {
                        v[i] = if mask & (1 << i) != 0 {
                            alt = !alt;
                            if alt {
                                pos
                            } else {
                                neg
                            }
                        } else {
                            rng.range(-9.0, 9.0)
                        };
                    }
                    inputs.push(v);
                }
            }
        }
    }
    p.assert_batch("E11", inputs);
}

/// E12 — the "bit pattern with no valid meaning" class. This API has no enum or
/// flag parameter (the header declares only `float*`), so the analogue is an
/// arbitrary 32-bit pattern per channel: all are accepted, none is rejected.
#[test]
fn e12_arbitrary_bit_patterns() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 112);
    p.assert_batch(
        "E12",
        (0..N * 25).map(|_| [rng.raw_f32(), rng.raw_f32(), rng.raw_f32()]),
    );

    // Also sweep the exponent/sign space systematically so no float class is missed.
    let mut inputs = Vec::new();
    for sign in [0u32, 0x8000_0000] {
        for exp in 0u32..=255 {
            for mant in [0u32, 1, 0x0040_0000, 0x007F_FFFF] {
                inputs.push(f32::from_bits(sign | (exp << 23) | mant));
            }
        }
    }
    let mut triples = Vec::new();
    for _ in 0..N * 2 {
        triples.push([
            inputs[rng.below(inputs.len())],
            inputs[rng.below(inputs.len())],
            inputs[rng.below(inputs.len())],
        ]);
    }
    p.assert_batch("E12-classes", triples);
}

/// E13 — denormal underflow (`delta` collapses to a subnormal or 0) and
/// overflow (`delta` saturates to `+inf`) boundaries.
#[test]
fn e13_underflow_and_overflow_of_delta() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 113);
    let mut inputs = Vec::new();

    // Underflow: three neighbouring subnormals -> delta is a tiny subnormal.
    for _ in 0..N {
        let base = rng.next_u32() & 0x0000_00FF;
        let a = f32::from_bits(base);
        let b = f32::from_bits(base + 1);
        let c = f32::from_bits(base + 2);
        inputs.push([c, b, a]);
        inputs.push([a, c, b]);
    }
    // Underflow to exactly 0: identical subnormals.
    for _ in 0..N / 2 {
        let v = f32::from_bits(rng.next_u32() & 0x0000_FFFF);
        inputs.push([v, v, v]);
    }
    // Overflow: max - min saturates past f32::MAX.
    for _ in 0..N {
        let hi = f32::MAX * rng.range(0.5, 1.0);
        let lo = f32::MIN * rng.range(0.5, 1.0);
        inputs.push([hi, lo, rng.range(-1.0, 1.0)]);
        inputs.push([lo, hi, rng.range(-1.0, 1.0)]);
        inputs.push([rng.range(-1.0, 1.0), lo, hi]);
    }
    // Tiny max with large-ish delta -> ratio overflow to +inf.
    for _ in 0..N / 2 {
        let tiny = f32::from_bits(1 + (rng.next_u32() & 0xF));
        inputs.push([tiny, -f32::MAX, -1.0]);
        inputs.push([-f32::MAX, tiny, -1.0]);
    }
    p.assert_batch("E13", inputs);
}
