//! `siphash(int)` stdout differential rows: `CONFIGS.md` C19–C22 and
//! `ERRORS.md` E12–E13.
//!
//! This target deliberately contains exactly ONE `#[test]` function. Comparing
//! `siphash`'s output requires redirecting the process-wide fd 1, which is not
//! safe while other libtest threads are writing progress lines to it; one test
//! per binary removes the race entirely.

mod common;

use common::*;

fn line_count(v: &[u8]) -> usize {
    v.iter().filter(|&&b| b == b'\n').count()
}

/// Compare the full stdout of `siphash(init)` between the C and Rust `.so`s.
fn check(p: &Pair, init: i32, row: &str) {
    let cf = p.c_siphash();
    let rf = p.r_siphash();
    let cout = capture_stdout("c", || unsafe { cf(init) });
    let rout = capture_stdout("r", || unsafe { rf(init) });
    assert!(
        !cout.is_empty(),
        "[{row}] C siphash({init}) produced no output"
    );
    assert_eq!(
        line_count(&cout),
        64,
        "[{row}] C siphash({init}) should print 64 rows, got {}",
        line_count(&cout)
    );
    assert_eq!(
        String::from_utf8_lossy(&cout),
        String::from_utf8_lossy(&rout),
        "[{row}] siphash({init}) stdout mismatch for {}",
        describe(p)
    );
    // byte-for-byte, not just as text
    assert_eq!(
        cout,
        rout,
        "[{row}] siphash({init}) raw byte mismatch for {}",
        describe(p)
    );
}

#[test]
fn siphash_stdout_all_rows() {
    let mut rng = Rng::new(0xC22_0000_0001);
    for p in pairs() {
        // C19 — default invocation.
        check(&p, 0, "C19");

        // C20 — byte-class boundaries and `unsigned char` truncation.
        for init in [
            1i32, -1, 42, 127, 128, 192, 255, 256, -256, 0x7A, 0xF9, 250, -128, -129,
        ] {
            check(&p, init, "C20");
        }

        // C21 / E12 — int extremes, incl. `z++` signed-overflow wrap.
        for init in [
            i32::MAX,
            i32::MIN,
            i32::MAX - 63,
            i32::MAX - 1,
            i32::MIN + 1,
            -2147483647,
            65536,
            -65536,
            0x0100_0000,
            -0x0100_0000,
        ] {
            check(&p, init, "C21/E12");
        }

        // E13 — exhaustive over all 256 residues of `init`, which is the full
        // behavioural domain of `mem[i] = (unsigned char)(init + i)`.
        for r in 0..256i32 {
            check(&p, r, "E13");
        }

        // C22 — randomized `init` across the whole `int` range.
        for _ in 0..200 {
            let init = rng.next_u64() as i32;
            check(&p, init, "C22");
        }
    }
}
