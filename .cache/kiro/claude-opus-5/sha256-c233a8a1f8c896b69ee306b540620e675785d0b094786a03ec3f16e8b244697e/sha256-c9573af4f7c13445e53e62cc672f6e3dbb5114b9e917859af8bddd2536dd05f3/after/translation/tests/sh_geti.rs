//! Phase B rows 53-57: the top-level `sh_geti` driver.
//!
//! `sh_geti` has no return value; its entire observable output is what it
//! `printf`s to stdout (plus "did it abort?").  Both libraries are driven with
//! stdout redirected to a file and the bytes are compared exactly.

mod common;
use common::*;

use std::ffi::c_int;

fn run_both(p: &Pair, nums: &[c_int], tag: &str) {
    unsafe {
        // Reset the process-global hash seed in BOTH libraries so the tables
        // they build are seeded identically.
        (p.c.rand_seed)(0x3141_5926);
        (p.r.rand_seed)(0x3141_5926);
        let cout = capture_stdout(&format!("c_{tag}"), || {
            for &n in nums {
                (p.c.sh_geti)(n);
            }
        });
        (p.c.rand_seed)(0x3141_5926);
        (p.r.rand_seed)(0x3141_5926);
        let rout = capture_stdout(&format!("r_{tag}"), || {
            for &n in nums {
                (p.r.sh_geti)(n);
            }
        });
        if cout != rout {
            let cs = String::from_utf8_lossy(&cout);
            let rs = String::from_utf8_lossy(&rout);
            let mut msg = format!(
                "sh_geti stdout DIVERGENCE ({tag}, nums={nums:?})\nC bytes={} RUST bytes={}\n",
                cout.len(),
                rout.len()
            );
            for (i, (a, b)) in cs.lines().zip(rs.lines()).enumerate() {
                if a != b {
                    msg.push_str(&format!("first differing line {i}:\n  C: {a:?}\n  R: {b:?}\n"));
                    break;
                }
            }
            msg.push_str(&format!(
                "--- C (first 800) ---\n{}\n--- RUST (first 800) ---\n{}\n",
                &cs[..cs.len().min(800)],
                &rs[..rs.len().min(800)]
            ));
            panic!("{msg}");
        }
    }
}

#[test]
fn cfg53_sh_geti_nonpositive() {
    let _g = serial();
    let p = pair();
    for n in [0, -1, -2, i32::MIN, i32::MIN + 1] {
        run_both(p, &[n], &format!("np{n}"));
    }
    // and: the output must actually be empty for these
    unsafe {
        (p.c.rand_seed)(0x3141_5926);
        let out = capture_stdout("np_empty", || (p.c.sh_geti)(0));
        assert!(out.is_empty(), "sh_geti(0) printed {:?}", String::from_utf8_lossy(&out));
    }
}

#[test]
fn cfg54_sh_geti_small() {
    let _g = serial();
    let p = pair();
    for n in 1..=8 {
        run_both(p, &[n], &format!("s{n}"));
    }
}

#[test]
fn cfg55_sh_geti_grow_shrink_rebuild() {
    let _g = serial();
    let p = pair();
    for n in [9, 12, 16, 17, 32, 33, 64, 100] {
        run_both(p, &[n], &format!("g{n}"));
    }
}

#[test]
fn cfg56_sh_geti_deep() {
    let _g = serial();
    let p = pair();
    for n in [200, 500] {
        run_both(p, &[n], &format!("d{n}"));
    }
}

#[test]
fn cfg57_sh_geti_repeated_calls() {
    let _g = serial();
    let p = pair();
    // The global hash seed has advanced by the time the 2nd/3rd call runs, so
    // this checks the seed-advance sequence agrees between the libraries.
    run_both(p, &[4, 4, 4], "rep444");
    run_both(p, &[1, 7, 3, 16, 0, 5], "rep_mixed");
    run_both(p, &[33, 2, 33], "rep_33_2_33");
}

/// Sanity: `sh_geti` really does print something for n > 0 (otherwise rows
/// 54-57 would be comparing two empty strings and prove nothing).
#[test]
fn cfg54b_sh_geti_output_is_nonempty_and_wellformed() {
    let _g = serial();
    let p = pair();
    unsafe {
        (p.c.rand_seed)(0x3141_5926);
        let cout = capture_stdout("wf_c", || (p.c.sh_geti)(8));
        (p.r.rand_seed)(0x3141_5926);
        let rout = capture_stdout("wf_r", || (p.r.sh_geti)(8));
        assert!(!cout.is_empty(), "C sh_geti(8) printed nothing");
        assert_eq!(cout, rout);
        let s = String::from_utf8_lossy(&cout);
        // 2 passes over the j loop, 4 even keys each -> 8 lines of "test_N M"
        let lines: Vec<&str> = s.lines().collect();
        assert_eq!(lines.len(), 8, "unexpected line count: {s:?}");
        for l in &lines {
            let mut it = l.split(' ');
            let k = it.next().unwrap();
            let v: i64 = it.next().unwrap().parse().unwrap();
            assert!(k.starts_with("test_"), "bad key in {l:?}");
            let n: i64 = k["test_".len()..].parse().unwrap();
            assert_eq!(v, n * 3, "value must be key*3 in {l:?}");
        }
    }
}
