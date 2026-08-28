//! Level 5: the public API declared in `c_src/include/lib.h` — `str_dups`.
//!
//! `str_dups` has no return value; everything it does that is observable from
//! outside goes to stdout through libc `printf`, so the comparison is a
//! byte-for-byte diff of the captured output.
mod harness;

use harness::*;

fn run_str_dups(num: i32) -> (Vec<u8>, Vec<u8>) {
    let p = pair();
    unsafe {
        (p.c.rand_seed)(0x31415926);
    }
    let c_out = capture_stdout("c", || unsafe { (p.c.str_dups)(num) });
    unsafe {
        (p.rs.rand_seed)(0x31415926);
    }
    let r_out = capture_stdout("rs", || unsafe { (p.rs.str_dups)(num) });
    (c_out, r_out)
}

fn check(num: i32) {
    let (c_out, r_out) = run_str_dups(num);
    assert_eq!(
        c_out,
        r_out,
        "str_dups({num}) stdout mismatch\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out)
    );
    // Guard against a vacuous comparison: str_dups always prints exactly one
    // line, `"a <num>\n"`, for the single string-map entry it creates.
    let expected = format!("a {num}\n");
    assert_eq!(
        String::from_utf8_lossy(&c_out),
        expected,
        "stdout capture looks broken for str_dups({num})"
    );
}

#[test]
fn str_dups_zero_and_small() {
    let _g = global_lock();
    for num in [0i32, 1, 2, 3, 5, 8, 16, 17, 31, 32, 33] {
        check(num);
    }
}

/// Negative counts make the `for (i=0; i < num; ++i)` arena loop body never
/// run, but the map part still executes and prints `num`.
#[test]
fn str_dups_negative_counts() {
    let _g = global_lock();
    for num in [-1i32, -2, -100, i32::MIN] {
        check(num);
    }
}

/// Counts large enough to push the string arena through several block-size
/// steps and into the `len > blocksize` path.
#[test]
fn str_dups_large_counts() {
    let _g = global_lock();
    for num in [64i32, 100, 128, 129, 512, 1000, 4096, 20_000] {
        check(num);
    }
}

#[test]
fn str_dups_boundary_counts() {
    let _g = global_lock();
    // `strkey` formats "test_%d", so these change the key length and therefore
    // where the arena block boundaries fall.
    for num in [9i32, 10, 11, 99, 100, 101, 999, 1000, 1001, 9999, 10_000] {
        check(num);
    }
}

/// Repeated calls must stay identical — `str_dups` frees everything it
/// allocates, but it also mutates the library-global hash seed.
#[test]
fn str_dups_repeated_calls_track_each_other() {
    let _g = global_lock();
    let p = pair();
    unsafe {
        (p.c.rand_seed)(0x31415926);
        (p.rs.rand_seed)(0x31415926);
    }
    for round in 0..25 {
        let num = (round * 13) % 71;
        let c_out = capture_stdout("c", || unsafe { (p.c.str_dups)(num) });
        let r_out = capture_stdout("rs", || unsafe { (p.rs.str_dups)(num) });
        assert_eq!(
            c_out,
            r_out,
            "round {round}: str_dups({num}) stdout mismatch\n  C   : {:?}\n  Rust: {:?}",
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
    }
}

/// The global seed set by `stbds_rand_seed` must not change what `str_dups`
/// prints, and it must not change it differently in the two builds.
#[test]
fn str_dups_under_varied_seeds() {
    let _g = global_lock();
    let p = pair();
    for seed in [0usize, 1, 0x31415926, usize::MAX, 1 << 48] {
        for num in [0i32, 1, 7, 64, 300] {
            unsafe { (p.c.rand_seed)(seed) };
            let c_out = capture_stdout("c", || unsafe { (p.c.str_dups)(num) });
            unsafe { (p.rs.rand_seed)(seed) };
            let r_out = capture_stdout("rs", || unsafe { (p.rs.str_dups)(num) });
            assert_eq!(
                c_out,
                r_out,
                "seed {seed:#x}, str_dups({num}) mismatch\n  C   : {:?}\n  Rust: {:?}",
                String::from_utf8_lossy(&c_out),
                String::from_utf8_lossy(&r_out)
            );
        }
    }
}
