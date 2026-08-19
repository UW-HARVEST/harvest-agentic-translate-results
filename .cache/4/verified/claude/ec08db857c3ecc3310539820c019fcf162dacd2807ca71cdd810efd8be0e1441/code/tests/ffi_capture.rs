//! Phase B/C — differential tests across the FFI boundary (`CONFIGS.md` rows
//! C37–C40, `ERRORS.md` row E20).
//!
//! Both sides are loaded with `libloading` and driven only through their exported
//! C-ABI symbols, exactly as an external caller would:
//!
//! * the C side is `gcc -O2 -shared -fPIC c_src/src/main.c` (the untouched C
//!   source, compiled out of tree);
//! * the Rust side is the `ffi/` cdylib, i.e. the `#[no_mangle] extern "C"`
//!   export wrappers around the translated code.
//!
//! `run()` is reached here at call depths and with `int` arguments the process
//! entry point can never produce, which is what exercises the accumulated
//! file-scope `static house_t the_house` and the `int` wraparound in `add_floor`
//! and `add_bedrooms`.
//!
//! ## Why this file contains exactly one `#[test]`
//!
//! Comparing `run()` means capturing what the loaded objects write to **file
//! descriptor 1**, which is process-global. libtest writes its own progress lines
//! (`test foo ... ok`) straight to fd 1 from the main thread, so any *other* test
//! finishing while fd 1 is redirected would inject its progress line into the
//! captured transcript. Cargo runs test *binaries* sequentially, so keeping the
//! capture work in a single test of its own binary removes the race entirely.
//! (The `run_sequence` shape check below also fails loudly if anything foreign
//! ever does end up in a capture.)

mod common;

use common::*;
use libloading::{Library, Symbol};
use std::os::raw::c_int;
use std::path::{Path, PathBuf};

type RunFn = unsafe extern "C" fn(c_int);

struct Sos {
    c: PathBuf,
    rust: PathBuf,
    dir: PathBuf,
}

/// Load a *fresh* instance of `so` (a unique inode, so `dlopen` gives freshly
/// initialised globals) and drive `run()` with `args`, returning everything the
/// object wrote to fd 1.
fn run_sequence(so: &Path, dir: &Path, tag: &str, args: &[i32]) -> Vec<u8> {
    let fresh = fresh_copy(so, dir, tag);
    let lib = unsafe { Library::new(&fresh) }.expect("dlopen");
    let f: Symbol<RunFn> = unsafe { lib.get(b"run\0") }.expect("dlsym run");
    let out = dir.join(format!("cap_{tag}.bin"));
    let bytes = capture_fd1(&out, || {
        for &a in args {
            unsafe { f(a) };
        }
    });
    drop(lib);
    let _ = std::fs::remove_file(&fresh);

    // Every `run()` call prints exactly four lines of a fixed shape. Verifying
    // that here means a capture polluted by some other writer can never be
    // mistaken for a translation difference.
    let expected_lines = 4 * args.len();
    let lines: Vec<&[u8]> = bytes
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .collect();
    assert_eq!(
        lines.len(),
        expected_lines,
        "capture for tag={tag} has {} lines, expected {expected_lines}",
        lines.len()
    );
    for (i, l) in lines.iter().enumerate() {
        assert!(
            l.starts_with(b"The house has ") && l.ends_with(b" bathrooms"),
            "capture for tag={tag} line {i} is not a house line: {}",
            describe(l)
        );
    }
    bytes
}

/// Compare a whole `run()` call sequence between the two shared objects.
fn compare_sequence(sos: &Sos, row: &str, tag: &str, args: &[i32]) {
    let c = run_sequence(&sos.c, &sos.dir, &format!("c_{tag}"), args);
    let r = run_sequence(&sos.rust, &sos.dir, &format!("r_{tag}"), args);
    if c != r {
        let cl: Vec<&[u8]> = c.split(|&b| b == b'\n').collect();
        let rl: Vec<&[u8]> = r.split(|&b| b == b'\n').collect();
        let mut i = 0;
        while i < cl.len() && i < rl.len() && cl[i] == rl[i] {
            i += 1;
        }
        panic!(
            "[{row}] FFI divergence for tag={tag} args[..8]={:?} (len {})\n\
             first differing line {i}:\n  C: {}\n  R: {}\n\
             total C bytes {} / R bytes {}",
            &args[..args.len().min(8)],
            args.len(),
            cl.get(i).map(|s| describe(s)).unwrap_or_default(),
            rl.get(i).map(|s| describe(s)).unwrap_or_default(),
            c.len(),
            r.len()
        );
    }
}

#[test]
fn ffi_run_differential() {
    let dir = scratch("ffi");
    let sos = Sos {
        c: c_so().to_path_buf(),
        rust: rust_so().to_path_buf(),
        dir: dir.clone(),
    };
    let _guard = fd_guard();

    // --- C37: one call per fresh load, boundary arguments -------------------
    for (i, &v) in BOUNDARY_I32.iter().enumerate() {
        compare_sequence(&sos, "C37", &format!("single{i}"), &[v]);
    }

    // --- C38: two calls per fresh load, i.e. exactly what `main()` does -----
    for (i, &v) in BOUNDARY_I32.iter().enumerate() {
        compare_sequence(&sos, "C38", &format!("double{i}"), &[v, v]);
    }

    // --- C39: long randomized sequence on a single load ---------------------
    let mut rng = Rng::new(0xFEED_BEEF);
    let args: Vec<i32> = (0..2000).map(|_| rng.i32()).collect();
    compare_sequence(&sos, "C39", "randseq", &args);

    // --- C39: many short randomized sequences, fresh state each time --------
    let mut rng = Rng::new(0xC0FF_EE01);
    for i in 0..20 {
        let n = 1 + rng.below(12);
        let args: Vec<i32> = (0..n)
            .map(|_| {
                if rng.below(2) == 0 {
                    *rng.pick(BOUNDARY_I32)
                } else {
                    rng.i32()
                }
            })
            .collect();
        compare_sequence(&sos, "C39", &format!("small{i}"), &args);
    }

    // --- C40: boundary arguments in random order on a single load -----------
    let mut rng = Rng::new(0x0BAD_F00D);
    let args: Vec<i32> = (0..400).map(|_| *rng.pick(BOUNDARY_I32)).collect();
    compare_sequence(&sos, "C40", "boundseq", &args);

    // --- C39: deep sequence, pushing `floors` and `bathrooms` far forward ---
    compare_sequence(&sos, "C39", "deepzero", &vec![0; 3000]);

    // --- C40: deep sequence that cycles `bedrooms` through wraparound -------
    let args: Vec<i32> = (0..1500)
        .map(|i| if i % 2 == 0 { i32::MAX } else { i32::MIN })
        .collect();
    compare_sequence(&sos, "C40", "deepwrap", &args);

    // --- C41: the FFI path and the process path are the same ground truth ---
    // Two `run()` calls through `dlopen` must equal what each executable prints.
    let c_exe = c_exe().to_path_buf();
    let r_exe = rust_exe();
    let mut cross = Vec::new();
    for x in [7i32, 0, -1, 42, i32::MAX, i32::MIN, i32::MAX - 4] {
        let c_so = run_sequence(&sos.c, &sos.dir, &format!("xc{x}"), &[x, x]);
        let r_so = run_sequence(&sos.rust, &sos.dir, &format!("xr{x}"), &[x, x]);
        cross.push((x, c_so, r_so));
    }
    // fd 1 is free again before spawning child processes.
    drop(_guard);
    for (x, c_so, r_so) in cross {
        let stdin = format!("{x}\n");
        let c_proc = run_stdin_file(&c_exe, &dir, stdin.as_bytes());
        let r_proc = run_stdin_file(&r_exe, &dir, stdin.as_bytes());
        assert_eq!(
            c_so, c_proc.stdout,
            "[C41] C: exported run() twice != the C executable for x={x}"
        );
        assert_eq!(
            r_so, r_proc.stdout,
            "[C41] Rust: exported run() twice != the Rust executable for x={x}"
        );
        assert_eq!(c_so, r_so, "[C41] C .so vs Rust .so for x={x}");
    }
}
