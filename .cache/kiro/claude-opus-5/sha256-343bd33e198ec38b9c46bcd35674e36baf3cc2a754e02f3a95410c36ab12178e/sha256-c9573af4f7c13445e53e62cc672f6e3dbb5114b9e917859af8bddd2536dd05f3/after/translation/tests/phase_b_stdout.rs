//! Phase B row C34 — stdout must be byte-identical between the C and Rust
//! shared objects.
//!
//! This lives in its own test binary on purpose: the capture works by
//! redirecting fd 1, which is process-wide, so it must not run concurrently
//! with other tests that make the libraries log.

mod common;

use common::*;

// ---------------------------------------------------------------------------
// C34 — stdout must be byte-identical, including line order.
// ---------------------------------------------------------------------------

#[test]
fn c34_stdout_byte_identical() {
    let libs = Pair::load();
    let (cf, rf) = libs.gotomach();
    warm_up_stdout();

    // (iterations, seed, mode, threshold, human label)
    let cases: &[(i32, i32, i32, i32, &str)] = &[
        (0, 0, 0, 0, "empty"),
        (4, 7, 0, i32::MAX, "valid-mode0"),
        (4, 7, 1, i32::MAX, "valid-mode1"),
        (4, 7, 2, i32::MAX, "valid-mode2"),
        (4, 7, 0, i32::MIN, "valid-nostore"),
        (-1, 0, 0, 0, "bad-iterations-negative"),
        (65536, 0, 0, 0, "bad-iterations-too-big"),
        (i32::MIN, 0, 0, 0, "bad-iterations-intmin"),
        (i32::MAX, 0, 0, 0, "bad-iterations-intmax"),
        (4, -1, 0, 0, "bad-seed-negative"),
        (4, 65536, 0, 0, "bad-seed-too-big"),
        (4, i32::MIN, 0, 0, "bad-seed-intmin"),
        (4, 7, -1, i32::MAX, "bad-mode-negative"),
        (4, 7, 3, i32::MAX, "bad-mode-3"),
        (4, 7, i32::MIN, i32::MAX, "bad-mode-intmin"),
        (4, 7, i32::MAX, i32::MAX, "bad-mode-intmax"),
        (65535, 1, 0, i32::MAX, "max-count-ceiling"),
    ];

    for &(it, s, m, t, label) in cases {
        let c_out = capture_stdout(&format!("c-{label}"), || {
            unsafe { cf(it, s, m, t) };
        });
        let r_out = capture_stdout(&format!("r-{label}"), || {
            unsafe { rf(it, s, m, t) };
        });
        if c_out != r_out {
            panic!(
                "[C34] stdout differs for {label} \
                 (iterations={it}, seed={s}, mode={m}, threshold={t})\n\
                 --- C ({} bytes) ---\n{}\n--- Rust ({} bytes) ---\n{}",
                c_out.len(),
                String::from_utf8_lossy(&c_out),
                r_out.len(),
                String::from_utf8_lossy(&r_out),
            );
        }
        assert!(
            !c_out.is_empty(),
            "[C34] capture produced nothing for {label} — the harness is broken"
        );
    }
}

