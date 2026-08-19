//! Build-configuration axis: the C translation unit compiled at every gcc
//! optimization level.
//!
//! `c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE`, so the reference build is
//! unoptimized — that is the configuration everything else here compares
//! against.  This target additionally pins down that the observable contract
//! does not shift when the optimizer is enabled, which matters specifically
//! because `helperBad()` returns the address of a local: gcc replaces that with
//! a literal `NULL` return, and we assert it keeps doing so at `-O0`, `-O1`,
//! `-O2`, `-O3` and `-Os`.  If a future toolchain ever changed that, this test
//! is where it would surface, instead of silently invalidating the translation.

mod common;

use common::{assert_same, rust_so, show, so_main, so_void, Rng, SEED};
use std::path::PathBuf;

const LEVELS: [(&str, &str); 5] = [
    ("O0", "-O0"),
    ("O1", "-O1"),
    ("O2", "-O2"),
    ("O3", "-O3"),
    ("Os", "-Os"),
];

fn c_objects() -> Vec<(&'static str, PathBuf)> {
    LEVELS
        .iter()
        .map(|(tag, flag)| (*tag, common::c_so_with_flags(tag, &[flag])))
        .collect()
}

#[test]
fn bad_takes_the_null_branch_at_every_optimization_level() {
    let rust = rust_so();
    let r = so_void(&rust, "bad");
    for (tag, so) in c_objects() {
        let c = so_void(&so, "bad");
        assert!(
            c.stdout.is_empty(),
            "gcc {tag}: bad() unexpectedly produced output \"{}\" — helperBad() no longer \
             returns NULL, so the translation's premise would need revisiting",
            show(&c.stdout)
        );
        assert_same(&format!("bad() at {tag}"), b"", &c, &r);
    }
}

#[test]
fn good_matches_at_every_optimization_level() {
    let rust = rust_so();
    let r = so_void(&rust, "good");
    for (tag, so) in c_objects() {
        let c = so_void(&so, "good");
        assert_eq!(c.stdout, b"helperGood1 string\n", "gcc {tag}");
        assert_same(&format!("good() at {tag}"), b"", &c, &r);
    }
}

#[test]
fn print_line_matches_at_every_optimization_level() {
    let rust = rust_so();
    let mut rng = Rng::new(SEED ^ 0xAA);
    let alphabet: Vec<u8> = (0x01u8..=0xff).collect();
    let mut corpus: Vec<Vec<u8>> = vec![Vec::new(), b"x".to_vec(), b"a\nb".to_vec()];
    for _ in 0..24 {
        let len = rng.range(1, 200) as usize;
        corpus.push(rng.bytes(len, &alphabet));
    }

    let objs = c_objects();
    for input in &corpus {
        let r = common::so_print_line(&rust, input);
        for (tag, so) in &objs {
            let c = common::so_print_line(so, input);
            assert_same(&format!("printLine at {tag}"), input, &c, &r);
        }
    }
    // NULL as well.
    let r = common::so_print_line_null(&rust);
    for (tag, so) in &objs {
        let c = common::so_print_line_null(so);
        assert_same(&format!("printLine(NULL) at {tag}"), b"<null>", &c, &r);
        assert!(c.stdout.is_empty(), "gcc {tag}");
    }
}

#[test]
fn main_matches_at_every_optimization_level() {
    let rust = rust_so();
    let mut rng = Rng::new(SEED ^ 0xBB);
    let mut corpus: Vec<Vec<u8>> = vec![
        Vec::new(),
        b"0".to_vec(),
        b"1".to_vec(),
        b"-1".to_vec(),
        b"abc".to_vec(),
        b"+".to_vec(),
        b"-".to_vec(),
        b"0x10".to_vec(),
        b"  \t\n7 ".to_vec(),
        b"4294967296".to_vec(),
        b"99999999999999999999".to_vec(),
        b"-99999999999999999999".to_vec(),
        b"2147483648".to_vec(),
        b"-9223372036854775808".to_vec(),
        vec![0x00],
        vec![0xff],
    ];
    let soup: Vec<u8> = (0x00u8..=0xff).collect();
    for _ in 0..64 {
        let len = rng.range(0, 12) as usize;
        corpus.push(rng.bytes(len, &soup));
    }

    let objs = c_objects();
    for input in &corpus {
        let r = so_main(&rust, input);
        for (tag, so) in &objs {
            let c = so_main(so, input);
            assert_same(&format!("main at {tag}"), input, &c, &r);
        }
    }
}
