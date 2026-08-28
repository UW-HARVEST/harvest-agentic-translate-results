//! End-to-end check against the *genuine* CMake artifact.
//!
//! `c_src/CMakeLists.txt` builds an executable (`driver`) from the same two
//! translation units. This test runs that real executable and compares its
//! stdout, stderr and exit status against the Rust `.so`'s exported `main`
//! called through `dlsym` with the same `argv`. It closes the loop on the whole
//! "compile the C as a shared library" step: if the `.so` used by the other
//! tests did not faithfully represent the CMake build, this test would diverge.
//!
//! Skipped (with a printed note) when `MD_C_EXE` is not set.

mod common;

use common::*;
use std::process::Command;

fn c_exe() -> Option<String> {
    std::env::var("MD_C_EXE").ok().filter(|p| !p.is_empty())
}

fn run_exe(exe: &str, args: &[&str]) -> (i32, Vec<u8>, Vec<u8>) {
    let out = Command::new(exe).args(args).output().expect("spawn driver");
    (
        out.status.code().unwrap_or(-1),
        out.stdout.clone(),
        out.stderr.clone(),
    )
}

#[test]
fn e2e_cmake_executable_matches_rust_so() {
    let exe = match c_exe() {
        Some(e) => e,
        None => {
            eprintln!("MD_C_EXE not set — skipping the CMake-executable comparison");
            return;
        }
    };
    let p = Pair::load();

    let arg_sets: Vec<Vec<&str>> = vec![
        vec!["3", "4"],
        vec!["0", "0"],
        vec!["-7", "9"],
        vec!["2147483647", "1"],
        vec!["-2147483648", "-1"],
        vec!["abc", "12x"],
        vec!["99999999999999", "-99999999999999"],
        vec!["5", "6", "extra"],
        vec!["1"],
        vec![],
    ];

    for args in arg_sets {
        let (code, out, err) = run_exe(&exe, &args);

        // Same argv the shell handed the executable, argv[0] included: it is
        // printed verbatim in the usage message.
        let mut argv: Vec<&str> = vec![exe.as_str()];
        argv.extend(args.iter().copied());

        // C .so's main, for a three-way check
        let (rc_c, cap_c) = {
            let items: Vec<Option<&str>> = argv.iter().map(|s| Some(*s)).collect();
            let mut av = Argv::new(&items);
            capture(|| unsafe { (p.c.main_fn())(argv.len() as i32, av.as_ptr()) })
        };
        let (rc_r, cap_r) = {
            let items: Vec<Option<&str>> = argv.iter().map(|s| Some(*s)).collect();
            let mut av = Argv::new(&items);
            capture(|| unsafe { (p.rs.main_fn())(argv.len() as i32, av.as_ptr()) })
        };

        assert_eq!(
            code, rc_c,
            "CMake exe exit code differs from the C .so's main for args {args:?}"
        );
        assert_eq!(
            out,
            cap_c.out,
            "CMake exe stdout differs from the C .so's main for args {args:?}:\n exe: {:?}\n so : {:?}",
            String::from_utf8_lossy(&out),
            String::from_utf8_lossy(&cap_c.out)
        );
        assert_eq!(
            err,
            cap_c.err,
            "CMake exe stderr differs from the C .so's main for args {args:?}"
        );

        assert_eq!(
            code, rc_r,
            "CMake exe exit code vs Rust .so main for args {args:?} [OP={} REPEAT={}]",
            p.op, p.repeat
        );
        assert_eq!(
            out,
            cap_r.out,
            "CMake exe stdout vs Rust .so main for args {args:?} [OP={} REPEAT={}]:\n exe : {:?}\n rust: {:?}",
            p.op,
            p.repeat,
            String::from_utf8_lossy(&out),
            String::from_utf8_lossy(&cap_r.out)
        );
        assert_eq!(
            err,
            cap_r.err,
            "CMake exe stderr vs Rust .so main for args {args:?} [OP={} REPEAT={}]:\n exe : {:?}\n rust: {:?}",
            p.op,
            p.repeat,
            String::from_utf8_lossy(&err),
            String::from_utf8_lossy(&cap_r.err)
        );
    }
}
