//! Phase B — the top-level driver `sh_geti(int)`.
//!
//! `sh_geti` is the library's only stdout-producing entry point (the
//! `printf("%s %d\n", strmap[z], strmap[z].value)` at lib.c:968, which passes
//! the whole `{char *key; int value;}` struct by value where `%s` expects a
//! `char *`).  Its stdout is compared byte-for-byte between the C and the Rust
//! shared object.
//!
//! CONFIGS.md rows 29, 30.

mod common;

use common::*;

fn run_pair(p: &Pair, seed: usize, num: i32) -> (Vec<u8>, Vec<u8>) {
    unsafe {
        (p.c.rand_seed)(seed);
        let out_c = capture_stdout("c", || (p.c.sh_geti)(num));
        (p.r.rand_seed)(seed);
        let out_r = capture_stdout("r", || (p.r.sh_geti)(num));
        (out_c, out_r)
    }
}

#[test]
fn cfg_29_sh_geti_stdout() {
    let (_g, p) = libs();
    let nums: [i32; 24] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 65, 100, 127, 128, 255, 257,
    ];
    for num in nums {
        let (c, r) = run_pair(p, 0x3141_5926, num);
        assert_eq!(
            String::from_utf8_lossy(&c),
            String::from_utf8_lossy(&r),
            "sh_geti({num}) stdout differs"
        );
        assert_eq!(c, r, "sh_geti({num}) stdout bytes differ");
    }
}

#[test]
fn cfg_30_sh_geti_seeded_stdout() {
    let (_g, p) = libs();
    let seeds: [usize; 7] =
        [0, 1, 7, 42, 0xDEAD_BEEF, usize::MAX, 0x8000_0000_0000_0001];
    for &seed in &seeds {
        for num in [0i32, 1, 2, 5, 16, 33, 64, 100] {
            let (c, r) = run_pair(p, seed, num);
            assert_eq!(
                String::from_utf8_lossy(&c),
                String::from_utf8_lossy(&r),
                "sh_geti({num}) with rand_seed({seed:#x}) stdout differs"
            );
            assert_eq!(c, r);
        }
    }
}

/// `sh_geti` mutates the process-global `stbds_hash_seed`; consecutive calls
/// without re-seeding must therefore stay in lockstep between the two
/// libraries as well.
#[test]
fn cfg_29b_sh_geti_consecutive_no_reseed() {
    let (_g, p) = libs();
    unsafe {
        (p.c.rand_seed)(0x3141_5926);
        (p.r.rand_seed)(0x3141_5926);
        for round in 0..12 {
            let num = [4i32, 17, 40, 9, 64, 3][round % 6];
            let out_c = capture_stdout("c", || (p.c.sh_geti)(num));
            let out_r = capture_stdout("r", || (p.r.sh_geti)(num));
            assert_eq!(
                String::from_utf8_lossy(&out_c),
                String::from_utf8_lossy(&out_r),
                "round {round}: sh_geti({num}) diverged without reseeding"
            );
        }
    }
}
