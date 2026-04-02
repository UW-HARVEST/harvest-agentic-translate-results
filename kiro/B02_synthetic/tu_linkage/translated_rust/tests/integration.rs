use libloading::{Library, Symbol};
use std::ffi::c_int;

fn c_lib() -> Library {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so");
    unsafe { Library::new(path).expect("failed to load C .so") }
}

// All stateful tests in one function to avoid parallel C state corruption.
// C and Rust static state must be called in lockstep (same sequence of calls).
#[test]
fn test_all_functions_sequentially() {
    let lib = c_lib();

    // ── Level 1: target (lib.c) — pure, no state ──
    {
        let c_target: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            unsafe { lib.get(b"target").unwrap() };
        for x in -10..=100 {
            let c_val = unsafe { c_target(x) };
            let r_val = tu_linkage::target(x);
            assert_eq!(c_val, r_val, "target({x}): C={c_val} Rust={r_val}");
        }
        eprintln!("  ✓ target");
    }

    // ── Level 2: call_a_once (a.c) — stateful ──
    {
        let c_fn: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            unsafe { lib.get(b"call_a_once").unwrap() };
        tu_linkage::tu_a::reset_state();
        let inputs = [0, 1, -1, 5, 10, 42, 100, -50, 7, 3, 255, 1000, -999, 0, 17];
        for &x in &inputs {
            let c_val = unsafe { c_fn(x) };
            let r_val = tu_linkage::tu_a::call_a_once(x);
            assert_eq!(c_val, r_val, "call_a_once({x}): C={c_val} Rust={r_val}");
        }
        eprintln!("  ✓ call_a_once");
    }

    // ── Level 2: process_a_stream (a.c) — stateful ──
    {
        let c_fn: Symbol<unsafe extern "C" fn(*const c_int, usize) -> c_int> =
            unsafe { lib.get(b"process_a_stream").unwrap() };
        let test_cases: Vec<Vec<i32>> = vec![
            vec![1, 2, 3],
            vec![0],
            vec![-1, 0, 1],
            vec![10, 20, 30, 40, 50],
            vec![7, 7, 7],
            vec![100, -100, 50, -50],
        ];
        for xs in &test_cases {
            let c_val = unsafe { c_fn(xs.as_ptr(), xs.len()) };
            let r_val = tu_linkage::tu_a::process_a_stream(xs.as_ptr(), xs.len());
            assert_eq!(c_val, r_val, "process_a_stream({xs:?}): C={c_val} Rust={r_val}");
        }
        eprintln!("  ✓ process_a_stream");
    }

    // ── Level 2: call_b_once (b.c) — stateful ──
    {
        let c_fn: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            unsafe { lib.get(b"call_b_once").unwrap() };
        tu_linkage::tu_b::reset_state();
        let inputs = [0, 1, -1, 5, 10, 42, 100, -50, 7, 3, 255, 1000, -999, 0, 17];
        for &x in &inputs {
            let c_val = unsafe { c_fn(x) };
            let r_val = tu_linkage::tu_b::call_b_once(x);
            assert_eq!(c_val, r_val, "call_b_once({x}): C={c_val} Rust={r_val}");
        }
        eprintln!("  ✓ call_b_once");
    }

    // ── Level 2: process_b_stream (b.c) — stateful ──
    {
        let c_fn: Symbol<unsafe extern "C" fn(*const c_int, usize) -> c_int> =
            unsafe { lib.get(b"process_b_stream").unwrap() };
        let test_cases: Vec<Vec<i32>> = vec![
            vec![1, 2, 3],
            vec![0],
            vec![-1, 0, 1],
            vec![10, 20, 30, 40, 50],
            vec![7, 7, 7],
            vec![100, -100, 50, -50],
        ];
        for xs in &test_cases {
            let c_val = unsafe { c_fn(xs.as_ptr(), xs.len()) };
            let r_val = tu_linkage::tu_b::process_b_stream(xs.as_ptr(), xs.len());
            assert_eq!(c_val, r_val, "process_b_stream({xs:?}): C={c_val} Rust={r_val}");
        }
        eprintln!("  ✓ process_b_stream");
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Binary comparison: run both C and Rust binaries, compare stdout.
// Each invocation starts fresh (no stale static state).
// This tests run_engine + vm_print + main end-to-end.
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn test_binary_output_match() {
    use std::process::Command;

    let c_bin = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/driver");
    let r_bin = format!("{}/target/debug/driver", env!("CARGO_MANIFEST_DIR"));

    let status = Command::new("cargo")
        .args(["build", "--bin", "driver"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("cargo build failed");
    assert!(status.success());

    let programs: Vec<Vec<&str>> = vec![
        // push 5, push 3, add, halt
        vec!["0", "5", "0", "3", "1", "10"],
        // push 10, push 20, mul, halt
        vec!["0", "10", "0", "20", "2", "10"],
        // push 7, dup, add, halt
        vec!["0", "7", "3", "1", "10"],
        // push 5, classify, halt
        vec!["0", "5", "5", "10"],
        // push 1, push 2, push 3, process_stream(3), halt
        vec!["0", "1", "0", "2", "0", "3", "9", "3", "10"],
        // push 4, push nonzero, skip 1, push 99, halt
        vec!["0", "4", "0", "1", "6", "1", "0", "99", "10"],
        // push 0 (false cond), skip 1, push 99, halt
        vec!["0", "0", "6", "1", "0", "99", "10"],
        // push 10, push 20, add, dup, classify, halt
        vec!["0", "10", "0", "20", "1", "3", "5", "10"],
        // repeat: push 42, repeat 3 times dup, halt
        vec!["0", "42", "7", "3", "3", "10"],
        // classify2: push 5, classify2, halt
        vec!["0", "5", "8", "10"],
        // push 2, push 3, push 4, drop, add, halt
        vec!["0", "2", "0", "3", "0", "4", "4", "1", "10"],
        // larger program: push several, ops, classify, process_stream
        vec!["0", "10", "0", "20", "0", "30", "1", "1", "5", "0", "1", "0", "2", "9", "2", "10"],
    ];

    for prog in &programs {
        let c_out = Command::new(c_bin).args(prog).output().expect("C binary failed");
        let r_out = Command::new(&r_bin).args(prog).output().expect("Rust binary failed");
        let c_stdout = String::from_utf8_lossy(&c_out.stdout);
        let r_stdout = String::from_utf8_lossy(&r_out.stdout);
        assert_eq!(c_stdout, r_stdout,
            "Binary mismatch for {prog:?}\nC:\n{c_stdout}\nRust:\n{r_stdout}");
    }
    eprintln!("  ✓ binary output match");
}
