//! Phase C — error/rejection-path differential tests (`ERRORS.md` rows E1..E7)
//! plus the read/write-exactness rows C13/C14 from `CONFIGS.md`, which need the
//! same guard-page + fork harness.
//!
//! `md5_digest` has no in-band error surface (no return value, no status
//! out-param, no checks — see `ERRORS.md`), so "rejection" for this API means a
//! memory fault. Each row forks a child per implementation and compares
//! (terminating signal, bytes committed to a MAP_SHARED output buffer before
//! the fault). Comparing the committed prefix — not just "both crashed" — is
//! what makes these differential rather than smoke tests.

mod common;

use common::*;

/// Calls `f(m, out)` in a forked child; reports how the child died.
fn child_call(f: Md5DigestFn, m: *const Md5, out: *mut u8) -> Outcome {
    run_in_child(move || unsafe { f(m, out) })
}

fn assert_same_outcome(c_out: Outcome, r_out: Outcome, ctx: &str) {
    assert_eq!(
        c_out, r_out,
        "[{ctx}] termination differs: C={c_out:?} Rust={r_out:?}"
    );
}

// ---------------------------------------------------------------------------
// E1 — m == NULL, out valid
// ---------------------------------------------------------------------------
#[test]
fn e01_null_m() {
    let (c, r) = both();
    let mut out = [0u8; 16];
    let co = child_call(c.md5_digest, std::ptr::null(), out.as_mut_ptr());
    let ro = child_call(r.md5_digest, std::ptr::null(), out.as_mut_ptr());
    assert_same_outcome(co, ro, "E1 m=NULL");
    assert_eq!(co, Outcome::segv(), "E1: C is expected to fault with SIGSEGV");
}

// ---------------------------------------------------------------------------
// E2 — out == NULL, m valid
// ---------------------------------------------------------------------------
#[test]
fn e02_null_out() {
    let (c, r) = both();
    let m = Md5::new(1, 2, 3, 4);
    let co = child_call(c.md5_digest, &m as *const Md5, std::ptr::null_mut());
    let ro = child_call(r.md5_digest, &m as *const Md5, std::ptr::null_mut());
    assert_same_outcome(co, ro, "E2 out=NULL");
    assert_eq!(co, Outcome::segv(), "E2: C is expected to fault with SIGSEGV");
}

// ---------------------------------------------------------------------------
// E3 — both NULL
// ---------------------------------------------------------------------------
#[test]
fn e03_both_null() {
    let (c, r) = both();
    let co = child_call(c.md5_digest, std::ptr::null(), std::ptr::null_mut());
    let ro = child_call(r.md5_digest, std::ptr::null(), std::ptr::null_mut());
    assert_same_outcome(co, ro, "E3 both NULL");
    assert_eq!(co, Outcome::segv(), "E3: C is expected to fault with SIGSEGV");
}

// ---------------------------------------------------------------------------
// E4 — m points into an unreadable (PROT_NONE) page; out is valid.
//      Also verifies that no output byte is produced before the fault.
// ---------------------------------------------------------------------------
#[test]
fn e04_unreadable_m() {
    let (c, r) = both();
    let src = Guarded::new();
    let dst = Guarded::new();
    let m_ptr = src.rw() as *const Md5;
    src.make_none(); // whole page inaccessible

    let run = |f: Md5DigestFn| -> (Outcome, Vec<u8>) {
        dst.fill(GUARD);
        let o = child_call(f, m_ptr, dst.rw());
        (o, dst.read(0, 16))
    };
    let (co, cb) = run(c.md5_digest);
    let (ro, rb) = run(r.md5_digest);
    assert_same_outcome(co, ro, "E4 unreadable m");
    assert_eq!(cb, rb, "E4: committed output bytes differ");
    assert_eq!(co, Outcome::segv(), "E4: C is expected to fault with SIGSEGV");
    assert_eq!(cb, vec![GUARD; 16], "E4: C wrote output despite unreadable m");
}

// ---------------------------------------------------------------------------
// E5 — out points into a read-only page; m is valid.
// ---------------------------------------------------------------------------
#[test]
fn e05_readonly_out() {
    let (c, r) = both();
    let dst = Guarded::new();
    dst.fill(GUARD);
    let out_ptr = dst.rw();
    dst.make_readonly();
    let m = Md5::new(0x1111_1111, 0x2222_2222, 0x3333_3333, 0x4444_4444);

    let co = child_call(c.md5_digest, &m as *const Md5, out_ptr);
    let ro = child_call(r.md5_digest, &m as *const Md5, out_ptr);
    assert_same_outcome(co, ro, "E5 read-only out");
    assert_eq!(co, Outcome::segv(), "E5: C is expected to fault with SIGSEGV");
    assert_eq!(
        dst.read(0, 16),
        vec![GUARD; 16],
        "E5: read-only page was modified"
    );
}

// ---------------------------------------------------------------------------
// E6 — out buffer too short: only k writable bytes (k = 0..15) before a
//      PROT_NONE guard page. Compares the fault AND the committed prefix.
// ---------------------------------------------------------------------------
#[test]
fn e06_short_out_buffer() {
    let (c, r) = both();
    let dst = Guarded::new();
    let mut rng = Rng::new(SEED ^ 0xE6);

    for k in 0..16usize {
        for i in 0..3 {
            let m = rng.state();
            let expect_prefix = &expected_le(&m)[..k];

            let run = |f: Md5DigestFn| -> (Outcome, Vec<u8>) {
                dst.fill(GUARD);
                let out_ptr = unsafe { dst.rw_end().sub(k) };
                let o = child_call(f, &m as *const Md5, out_ptr);
                let tail = if k == 0 {
                    Vec::new()
                } else {
                    dst.read(dst.page() - k, k)
                };
                (o, tail)
            };

            let (co, cb) = run(c.md5_digest);
            let (ro, rb) = run(r.md5_digest);
            let ctx = format!("E6 k={k} i={i} m={m:08x?}");
            assert_same_outcome(co, ro, &ctx);
            assert_eq!(cb, rb, "[{ctx}] committed prefix differs");
            assert_eq!(co, Outcome::segv(), "[{ctx}] C should fault");
            assert_eq!(
                cb,
                expect_prefix.to_vec(),
                "[{ctx}] C committed prefix is not the first {k} digest bytes"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// E7 — m readable for only k bytes (k = 0..15) before a PROT_NONE guard page.
//      The C loads each field with a single 4-byte load, so a partially
//      readable field yields no bytes at all for that field.
// ---------------------------------------------------------------------------
#[test]
fn e07_short_m_buffer() {
    let (c, r) = both();
    let src = Guarded::new();
    let dst = Guarded::new();

    // Deterministic content for whatever part of the struct stays readable.
    for off in 0..src.page() {
        src.write_at(off, &[(off as u8).wrapping_mul(37).wrapping_add(3)]);
    }

    for k in 0..16usize {
        let m_ptr = unsafe { src.rw_end().sub(k) } as *const Md5;

        let run = |f: Md5DigestFn| -> (Outcome, Vec<u8>) {
            dst.fill(GUARD);
            let o = child_call(f, m_ptr, dst.rw());
            (o, dst.read(0, 16))
        };

        let (co, cb) = run(c.md5_digest);
        let (ro, rb) = run(r.md5_digest);
        let ctx = format!("E7 k={k}");
        assert_same_outcome(co, ro, &ctx);
        assert_eq!(
            cb, rb,
            "[{ctx}] committed output differs\n  C   : {cb:02x?}\n  Rust: {rb:02x?}"
        );
        assert_eq!(co, Outcome::segv(), "[{ctx}] C should fault");

        // Whole readable fields must have been serialised; the rest untouched.
        let committed = 4 * (k / 4);
        assert!(
            cb[..committed].iter().all(|&b| b != GUARD) || committed == 0 || k == 0,
            "[{ctx}] expected {committed} committed bytes, got {cb:02x?}"
        );
        assert_eq!(
            &cb[committed..],
            &vec![GUARD; 16 - committed][..],
            "[{ctx}] bytes past the last wholly readable field were written"
        );
    }
}

// ---------------------------------------------------------------------------
// C13 — write exactness: out ends exactly at a PROT_NONE guard page,
//       and starts exactly at one. Must NOT fault.
// ---------------------------------------------------------------------------
#[test]
fn c13_write_exactness_against_guard_pages() {
    let (c, r) = both();
    let dst = Guarded::new();
    let mut rng = Rng::new(SEED ^ 0x13);

    for i in 0..8 {
        let m = rng.state();
        // (a) out[15] is the last writable byte -> proves no 17th byte written
        // (b) out[0] is the first writable byte -> proves nothing before out[0]
        for (case, out_off) in [("tail", dst.page() - 16), ("head", 0)] {
            let run = |f: Md5DigestFn| -> (Outcome, Vec<u8>) {
                dst.fill(GUARD);
                let out_ptr = unsafe { dst.rw().add(out_off) };
                let o = child_call(f, &m as *const Md5, out_ptr);
                (o, dst.read(out_off, 16))
            };
            let (co, cb) = run(c.md5_digest);
            let (ro, rb) = run(r.md5_digest);
            let ctx = format!("C13 {case} i={i} m={m:08x?}");
            assert_same_outcome(co, ro, &ctx);
            assert_eq!(co, Outcome::ok(), "[{ctx}] C faulted unexpectedly");
            assert_eq!(cb, rb, "[{ctx}] output differs");
            assert_eq!(cb, expected_le(&m).to_vec(), "[{ctx}] reference sanity");
        }
    }
}

// ---------------------------------------------------------------------------
// C14 — read exactness: m occupies the last (and first) 16 bytes of the
//       readable page, flanked by PROT_NONE. Must NOT fault.
// ---------------------------------------------------------------------------
#[test]
fn c14_read_exactness_against_guard_pages() {
    let (c, r) = both();
    let src = Guarded::new();
    let dst = Guarded::new();
    let mut rng = Rng::new(SEED ^ 0x14);

    for i in 0..8 {
        let m = rng.state();
        for (case, m_off) in [("tail", src.page() - 16), ("head", 0)] {
            src.fill(0);
            src.write_at(m_off, &m.to_bytes());
            let m_ptr = unsafe { src.rw().add(m_off) } as *const Md5;
            let run = |f: Md5DigestFn| -> (Outcome, Vec<u8>) {
                dst.fill(GUARD);
                let o = child_call(f, m_ptr, dst.rw());
                (o, dst.read(0, 16))
            };
            let (co, cb) = run(c.md5_digest);
            let (ro, rb) = run(r.md5_digest);
            let ctx = format!("C14 {case} i={i} m={m:08x?}");
            assert_same_outcome(co, ro, &ctx);
            assert_eq!(co, Outcome::ok(), "[{ctx}] C faulted unexpectedly");
            assert_eq!(cb, rb, "[{ctx}] output differs");
            assert_eq!(cb, expected_le(&m).to_vec(), "[{ctx}] reference sanity");
        }
    }
}

// ---------------------------------------------------------------------------
// E8 / E9 documentation guards: the C API has no enum, flag, or length
// parameter, so there is no out-of-range value to pass across the FFI
// boundary. This test pins that fact to the header so the rows in ERRORS.md
// cannot silently rot if the API ever grows one.
// ---------------------------------------------------------------------------
#[test]
fn e08_e09_no_enum_or_length_parameters_in_api() {
    let hdr = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/c_src/include/lib.h"
    ))
    .expect("read lib.h");
    assert!(
        !hdr.contains("enum"),
        "ERRORS.md row E8 assumed there is no enum in the public API"
    );
    let protos: Vec<&str> = hdr.lines().filter(|l| l.contains("md5_digest")).collect();
    assert_eq!(protos.len(), 1, "expected exactly one public prototype");
    assert_eq!(
        protos[0].trim(),
        "void md5_digest(const tflac_md5 *m, tflac_u8 out[16]);",
        "public prototype changed; ERRORS.md/CONFIGS.md must be re-derived"
    );
    // Struct field count pinned (CONFIGS.md axis A3): four `tflac_u32 <name>;`
    // declarations (the typedef line reads `tflac_u32;`, without a space).
    assert_eq!(hdr.matches("tflac_u32 ").count(), 4, "field count");
    for f in ["tflac_u32 a;", "tflac_u32 b;", "tflac_u32 c;", "tflac_u32 d;"] {
        assert!(hdr.contains(f), "missing field declaration `{f}`");
    }
}
