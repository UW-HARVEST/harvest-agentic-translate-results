// Phase D — ABI / struct-layout parity.
//
// The tests in Phase B/C drive each library end to end in isolation, which
// cannot catch a `sizeof(Task)` or field-offset mismatch: each library would
// simply be self-consistent. These tests deliberately MIX the two libraries in
// one pipeline — allocate with C, populate with Rust, render with C, and vice
// versa — so any layout, stride or alignment disagreement shows up as different
// bytes on stdout.
//
// The loggers are deliberately left uninitialised in every run, so the only
// output is `print_tasks`'s and there is no cross-library stdio interleaving to
// make the comparison nondeterministic.
mod harness;

use harness::*;
use std::ffi::{c_int, CString};

struct Case {
    descs: Vec<CString>,
    prios: Vec<c_int>,
}

fn make_case(seed: u64, n: usize) -> Case {
    let mut rng = Rng::new(seed);
    let mut descs = Vec::new();
    let mut prios = Vec::new();
    for _ in 0..n {
        descs.push(cstr(&rng.printable_range(0, 300)));
        prios.push(rng.i32());
    }
    Case { descs, prios }
}

/// Run one pipeline with a chosen library for each individual step.
unsafe fn pipeline(
    creator: &Api,
    adder: &Api,
    printer: &Api,
    destroyer: &Api,
    case: &Case,
    scratch: &Scratch,
    label: &str,
) -> (Vec<u8>, Vec<u8>, i32, i32, Vec<(Vec<u8>, i32)>) {
    let cap = Capture::begin(
        scratch.path(&format!("{label}.out")),
        scratch.path(&format!("{label}.err")),
    );
    let m = (creator.create_task_manager)();
    assert!(!m.is_null(), "{label}: create_task_manager returned NULL");
    for (d, p) in case.descs.iter().zip(&case.prios) {
        (adder.add_task)(m, d.as_ptr(), *p);
    }
    (printer.print_tasks)(m);
    let max_tasks = (*m).max_tasks;
    let task_count = (*m).task_count;
    let mut tasks = Vec::new();
    for i in 0..task_count {
        let t = (*m).tasks.offset(i as isize);
        tasks.push((
            std::slice::from_raw_parts((*t).description.as_ptr() as *const u8, 256).to_vec(),
            (*t).priority,
        ));
    }
    (destroyer.destroy_task_manager)(m);
    let (out, err) = cap.end();
    (out, err, max_tasks, task_count, tasks)
}

#[test]
fn phase_d_cross_library_struct_layout() {
    let _g = guard();
    for (seed, n, max) in [
        (1u64, 1usize, "4"),
        (2, 4, "4"),
        (3, 8, "8"),
        (4, 16, "32"),
        (5, 3, "3"),
    ] {
        let libs = fresh("abi");
        let scratch = Scratch::new("abi");
        set_env("MAX_TASKS", Some(max));
        set_env("LOG_FILE", None); // loggers stay uninitialised ⇒ no log output
        let case = make_case(seed, n);

        unsafe {
            let c = &libs.c;
            let r = &libs.rust;
            let all_c = pipeline(c, c, c, c, &case, &scratch, "all_c");
            let all_r = pipeline(r, r, r, r, &case, &scratch, "all_r");
            // C allocates, Rust fills, C renders and frees.
            let c_r_c = pipeline(c, r, c, c, &case, &scratch, "c_r_c");
            // Rust allocates, C fills, Rust renders and frees.
            let r_c_r = pipeline(r, c, r, r, &case, &scratch, "r_c_r");
            // Fully interleaved.
            let mix = pipeline(c, r, r, c, &case, &scratch, "mix");

            for (label, got) in [
                ("all_rust", &all_r),
                ("C-create/Rust-add/C-print", &c_r_c),
                ("Rust-create/C-add/Rust-print", &r_c_r),
                ("interleaved", &mix),
            ] {
                assert_eq!(
                    all_c.0, got.0,
                    "stdout differs for `{label}` (seed {seed}, n {n}, MAX_TASKS {max})"
                );
                assert_eq!(all_c.1, got.1, "stderr differs for `{label}`");
                assert_eq!(all_c.2, got.2, "max_tasks differs for `{label}`");
                assert_eq!(all_c.3, got.3, "task_count differs for `{label}`");
                assert_eq!(all_c.4, got.4, "Task contents differ for `{label}`");
            }
            // Sanity: the pipeline really did something.
            assert!(
                !all_c.0.is_empty(),
                "expected print_tasks output for seed {seed}"
            );
        }
    }
}

/// `driver` is the composed entry point; run it in each library back to back on
/// the same input with the same environment and require identical stdout. This
/// also confirms the two `.so`s can be loaded into one process simultaneously
/// without their `static` state colliding.
#[test]
fn phase_d_both_libraries_loaded_together() {
    let _g = guard();
    let libs = fresh("together");
    let scratch = Scratch::new("together");
    let mut rng = Rng::new(0xD00D);
    for i in 0..25u64 {
        let n = rng.range(0, 12);
        let mut buf: Vec<u8> = Vec::new();
        for k in 0..n {
            if k > 0 {
                buf.push(b'\n');
            }
            buf.extend_from_slice(&rng.printable_range(0, 120));
        }
        let input = cstr(&buf);
        let max = rng.range(0, 15).to_string();

        unsafe {
            // C first, then Rust — deliberately in the same process, each with
            // its own $LOG_FILE so the log bytes are attributable.
            set_env("MAX_TASKS", Some(&max));
            let clog = scratch.path(&format!("c{i}.log"));
            set_env("LOG_FILE", Some(clog.to_str().unwrap()));
            let cap = Capture::begin(scratch.path("c.out"), scratch.path("c.err"));
            let rc_c = (libs.c.driver)(input.as_ptr());
            let (out_c, err_c) = cap.end();

            let rlog = scratch.path(&format!("r{i}.log"));
            set_env("LOG_FILE", Some(rlog.to_str().unwrap()));
            let cap = Capture::begin(scratch.path("r.out"), scratch.path("r.err"));
            let rc_r = (libs.rust.driver)(input.as_ptr());
            let (out_r, err_r) = cap.end();

            assert_eq!(rc_c, rc_r, "driver rc differs (iter {i}, MAX_TASKS {max})");
            assert_eq!(out_c, out_r, "driver stdout differs (iter {i})");
            assert_eq!(err_c, err_r, "driver stderr differs (iter {i})");
            let lc = std::fs::read(&clog).unwrap_or_default();
            let lr = std::fs::read(&rlog).unwrap_or_default();
            assert_eq!(
                String::from_utf8_lossy(&lc),
                String::from_utf8_lossy(&lr),
                "driver log differs (iter {i})"
            );
        }
    }
}
