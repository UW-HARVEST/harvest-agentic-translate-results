//! Cross-process differential test: allocation-pattern equality.
//!
//! The in-process tests in `valid_paths.rs` / `errors.rs` force the allocator
//! state before every heap-sensitive call, because both `.so`s share one libc
//! heap. This test instead loads only ONE implementation per process — the way a
//! real consumer does — seeds the tcache once at the start, and then runs a long
//! fixed script with NO further host allocations. From that point the entire
//! result sequence is decided by the library's own `malloc`/`free` pattern, so
//! the two children can only agree if the C and the Rust perform the same number
//! of allocations, of the same size, freed in the same order, at the same points
//! in the call sequence. An extra or missing `malloc` anywhere shows up as a
//! phase shift in the sequence.
//!
//! Why the seed is still needed even in a fresh process: `dlopen` itself
//! allocates (link maps, TLS, path strings), and the 400 KB Rust `cdylib` has a
//! different load-time allocation footprint than the 16 KB C `.so`. That leaves
//! the 32-byte tcache bin in a different state before the first library call, so
//! an unseeded comparison measures the *dynamic loader*, not the library. See
//! VERIFICATION.md for the full argument.

mod common;

use common::Heap;
use std::env;
use std::ffi::c_int;
use std::process::Command;

const ENV_KEY: &str = "HARVEST_FRESH_IMPL";
const MARKER: &str = "FRESH_SEQ:";
const CAP: usize = 4096;

/// A fixed script of calls, run identically by both children.
///
/// IMPORTANT: this function must not allocate. `out` is pre-sized by the caller
/// so no `Vec` growth happens, which would perturb the same tcache bin the
/// library uses and make the sequence depend on the harness instead of on the
/// library.
fn run_script(imp: &common::Impl, out: &mut Vec<i32>) {
    // Pure calls first (no heap traffic), then the allocating ones, so any
    // difference in allocation pattern shows up as a shifted sequence.
    for op in -2..=5i32 {
        out.push(unsafe { (imp.apply_bitmask)(0x1234_5678, op) });
    }
    let s = b"Hello\0";
    out.push(unsafe { (imp.process_string)(s.as_ptr() as *const _) });

    for i in 0..40i32 {
        out.push(unsafe { (imp.compare_allocations)(i - 20, i) });
    }
    for i in 0..40i32 {
        out.push(unsafe { (imp.arity4)(i - 20, i, i % 3 - 1, i - 5) });
    }
    for i in 0..20i32 {
        out.push(unsafe { (imp.arity2)(i - 10, i) });
        out.push(unsafe { (imp.arity3)(i - 10, i, i - 3) });
    }
    let mut params: [c_int; 4] = [7, -13, 5, 21];
    for len in 0..=12i32 {
        out.push(unsafe { (imp.arity)(len, params.as_mut_ptr()) });
    }
    // Interleave non-allocating calls between allocating ones to confirm
    // neither implementation sneaks in a hidden allocation.
    for i in 0..20i32 {
        let mut buf: [c_int; 4] = [i, i + 1, i + 2, i + 3];
        unsafe { (imp.shift_array)(buf.as_mut_ptr(), 4, 1) };
        out.extend_from_slice(&buf);
        let mut m = [0i32; 12];
        unsafe { (imp.init_matrix)(m.as_mut_ptr()) };
        out.push(m.iter().fold(0i32, |a, b| a.wrapping_add(*b)));
        out.push(unsafe { (imp.arity4)(i, -i, 3, 0) });
    }
    assert!(out.len() <= CAP, "run_script would have reallocated `out`");
}

fn child_main(which: &str) {
    let imp = match which {
        "c" => common::load_c(),
        "rust" => common::load_rust(),
        other => panic!("unknown impl {other}"),
    };
    // Allocate the output buffer BEFORE seeding, so the seeded tcache state is
    // the last thing that happens before the first library call.
    let mut out: Vec<i32> = Vec::with_capacity(CAP);
    for order in [Heap::Ascending, Heap::Descending] {
        common::seed_heap(order);
        run_script(&imp, &mut out);
    }
    let joined: Vec<String> = out.iter().map(|v| v.to_string()).collect();
    println!("{MARKER}{}", joined.join(","));
}

fn spawn_child(which: &str) -> Vec<i32> {
    let exe = env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args(["fresh_process_sequences_match", "--exact", "--nocapture"])
        .env(ENV_KEY, which)
        .output()
        .expect("spawn child");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "child ({which}) failed: {stdout}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let line = stdout
        .lines()
        .find(|l| l.starts_with(MARKER))
        .unwrap_or_else(|| panic!("child ({which}) produced no sequence:\n{stdout}"));
    line[MARKER.len()..]
        .split(',')
        .map(|t| t.parse::<i32>().expect("int"))
        .collect()
}

#[test]
fn fresh_process_sequences_match() {
    if let Ok(which) = env::var(ENV_KEY) {
        child_main(&which);
        return;
    }
    let c = spawn_child("c");
    let r = spawn_child("rust");
    assert!(!c.is_empty(), "empty C sequence");
    assert_eq!(
        c.len(),
        r.len(),
        "sequence lengths differ: C={} Rust={}",
        c.len(),
        r.len()
    );
    if c != r {
        let first = c
            .iter()
            .zip(r.iter())
            .position(|(a, b)| a != b)
            .expect("difference");
        panic!(
            "fresh-process sequences diverge at index {first}: C={} Rust={}\n\
             C  : {:?}\nRust: {:?}",
            c[first],
            r[first],
            &c[first.saturating_sub(4)..(first + 5).min(c.len())],
            &r[first.saturating_sub(4)..(first + 5).min(r.len())],
        );
    }
    // Run each child a second time to confirm the sequences are reproducible
    // (i.e. the allocator behavior is deterministic, not just coincidentally
    // equal on one run).
    assert_eq!(spawn_child("c"), c, "C sequence is not reproducible");
    assert_eq!(spawn_child("rust"), r, "Rust sequence is not reproducible");
}
