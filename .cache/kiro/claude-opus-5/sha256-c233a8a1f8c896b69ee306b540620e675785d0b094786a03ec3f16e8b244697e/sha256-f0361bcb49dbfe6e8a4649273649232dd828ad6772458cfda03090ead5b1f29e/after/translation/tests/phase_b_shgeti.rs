//! Phase B — CONFIGS.md rows 58-63: the `sh_geti` driver, end to end, with the
//! stdout it produces compared byte-for-byte between the C and Rust `.so`s.
//!
//! `sh_geti` is the only entry point in `c_src/include/lib.h`; it drives the
//! whole library (arena, both string-hash modes, put/get/del, growth, shrink)
//! and `printf`s each surviving entry.  It also contains the C's own
//! `assert`s, so a mismatch in any internal invariant kills the process.

mod common;
use common::*;

fn run_pair(b: &Both, num: i32, seed: usize) {
    // both libraries must start from the same global hash-seed state
    seed_both(b, seed);
    let cout = capture_stdout(|| unsafe { (b.c.sh_geti)(num) });
    seed_both(b, seed);
    let rout = capture_stdout(|| unsafe { (b.r.sh_geti)(num) });
    assert_eq!(
        String::from_utf8_lossy(&cout),
        String::from_utf8_lossy(&rout),
        "sh_geti({num}) seed={seed:#x}: stdout differs ({} vs {} bytes)",
        cout.len(),
        rout.len()
    );
    assert_eq!(cout, rout, "sh_geti({num}) seed={seed:#x}: stdout bytes differ");
}

/// row 58 — `num == 0`
#[test]
fn row_58_sh_geti_zero() {
    let (_g, b) = both();
    run_pair(b, 0, 0x3141_5926);
    // nothing to print
    seed_both(b, 0x3141_5926);
    let out = capture_stdout(|| unsafe { (b.c.sh_geti)(0) });
    assert!(out.is_empty(), "expected no output for num=0, got {out:?}");
}

/// rows 59-60 — small counts, odd and even, crossing several table growths
#[test]
fn row_59_60_sh_geti_small() {
    let (_g, b) = both();
    for num in 1..=40i32 {
        run_pair(b, num, 0x3141_5926);
    }
    // Guard against a vacuously-passing capture: `sh_geti` must actually print.
    seed_both(b, 0x3141_5926);
    let out = capture_stdout(|| unsafe { (b.c.sh_geti)(8) });
    let text = String::from_utf8_lossy(&out).to_string();
    assert!(
        text.contains("test_0 0") && text.contains("test_4 12") && out.len() > 40,
        "stdout capture looks broken; got {text:?}"
    );
    // `sh_geti` runs two passes (SH_STRDUP then SH_ARENA) and prints
    // shlen(strmap) lines per pass, i.e. 4 lines each for num == 8.
    assert_eq!(
        text.lines().count(),
        8,
        "expected 8 printed lines for num=8, got {text:?}"
    );
}

/// row 61 — larger counts (arena spills past 512 bytes of keys, repeated
/// shrink/rebuild of the hash index)
#[test]
fn row_61_sh_geti_large() {
    let (_g, b) = both();
    for num in [50i32, 64, 65, 100, 127, 128, 129, 200, 257, 500] {
        run_pair(b, num, 0x3141_5926);
    }
}

/// row 62 / ERRORS.md #46 — negative counts
#[test]
fn row_62_sh_geti_negative() {
    let (_g, b) = both();
    for num in [-1i32, -2, -7, -1000, i32::MIN, i32::MIN + 1] {
        run_pair(b, num, 0x3141_5926);
        seed_both(b, 0x3141_5926);
        let out = capture_stdout(|| unsafe { (b.r.sh_geti)(num) });
        assert!(out.is_empty(), "expected no output for num={num}");
    }
}

/// row 63 — the same driver under many different starting hash seeds.  The seed
/// changes the probe order, hence the element order, hence the printed order.
#[test]
fn row_63_sh_geti_seeds() {
    let (_g, b) = both();
    let mut rng = Rng::new(0x630);
    let mut seeds: Vec<usize> = vec![0, 1, 2, 3, usize::MAX, usize::MAX - 1, 0x3141_5926];
    for _ in 0..25 {
        seeds.push(rng.next_u64() as usize);
    }
    for s in seeds {
        for num in [0i32, 1, 3, 8, 17, 33, 100] {
            run_pair(b, num, s);
        }
    }
}

/// Consecutive calls without re-seeding: the global hash seed keeps advancing,
/// so this checks that the seed *state machine* stays in lock-step across a
/// long run rather than only at a fresh seed.
///
/// The whole sequence runs inside a single capture (i.e. a single forked
/// child) so that the seed really does advance from call to call.
#[test]
fn sh_geti_seed_state_machine() {
    let (_g, b) = both();
    const NUMS: [i32; 10] = [1, 2, 5, 9, 16, 31, 64, 3, 0, 7];

    seed_both(b, 0x3141_5926);
    let cout = capture_stdout(|| unsafe {
        for n in NUMS {
            (b.c.sh_geti)(n);
            println_bar();
        }
    });
    seed_both(b, 0x3141_5926);
    let rout = capture_stdout(|| unsafe {
        for n in NUMS {
            (b.r.sh_geti)(n);
            println_bar();
        }
    });
    assert_eq!(
        String::from_utf8_lossy(&cout),
        String::from_utf8_lossy(&rout),
        "seed state machine diverged across consecutive sh_geti calls"
    );
    assert!(cout.len() > 100, "expected substantial output, got {} bytes", cout.len());

    // The seed advance is also directly observable: after the same sequence of
    // table creations, the next table's `seed` field must match.
    seed_both(b, 0x3141_5926);
    unsafe {
        let mut cs = Vec::new();
        let mut rs = Vec::new();
        for _ in 0..40 {
            let c = (b.c.shmode_func)(16, 3);
            let r = (b.r.shmode_func)(16, 3);
            cs.push(snap(c, 16, 0, KeyKind::Raw, 0, 0).seed);
            rs.push(snap(r, 16, 0, KeyKind::Raw, 0, 0).seed);
            (b.c.hmfree_func)(hash_to_arr(c, 16), 16);
            (b.r.hmfree_func)(hash_to_arr(r, 16), 16);
        }
        assert_eq!(cs, rs, "hash-seed sequence diverged");
        assert!(cs.windows(2).any(|w| w[0] != w[1]), "seed never advanced");
    }
}

/// separator written by the child so the two runs are compared as a unit
fn println_bar() {
    use std::io::Write;
    let mut so = std::io::stdout();
    let _ = so.write_all(b"|\n");
    let _ = so.flush();
}
