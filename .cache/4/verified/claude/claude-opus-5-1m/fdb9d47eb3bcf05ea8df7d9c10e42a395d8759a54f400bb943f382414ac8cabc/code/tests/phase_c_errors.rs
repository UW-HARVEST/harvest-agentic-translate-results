//! Phase C — error/boundary-path differential tests.
//!
//! One test per row of `ERRORS.md` (E1..E15).
//!
//! `c_src/src/lib.c` contains **no** error returns, asserts, range checks, null
//! checks or enums (see the grep census in `ERRORS.md`), so the "error surface"
//! is entirely implicit: degenerate sizes that make the C silently do nothing,
//! and out-of-contract inputs that the C processes unchecked.
//!
//! Rows whose trigger makes the C fault or smash memory are run in **child
//! processes**, and the assertion is that the C child and the Rust child
//! terminate with the *same* signal / exit code — not merely "both failed".

mod common;

use common::*;
use std::io::Write;
use std::path::Path;
use std::process::Command;

// ---------------------------------------------------------------------------
// Deterministic buffer construction (no RNG => reproducible in child procs)
// ---------------------------------------------------------------------------

/// Usable elements handed to `merge_sort` in child-process cases.
const CHILD_N: usize = 8;
/// Extra addressable elements after the usable region, so that
/// "one past the end" style rows stay inside the allocation and therefore
/// remain deterministic instead of faulting at a random address.
const SLACK: usize = 8;

fn fill(i: usize, salt: u64) -> Sprite {
    Sprite {
        texture_id: (i as u64).wrapping_mul(0x1000_0001).wrapping_add(salt),
        // deliberately unsorted so a no-op implementation is detectable
        sort_bits: ((i * 7 + 3) % 11) as i32 - 5,
        pad: (i as u32).wrapping_mul(0x0101_0101) ^ 0xDEAD_0000,
    }
}

fn make(n: usize, salt: u64) -> Vec<Sprite> {
    (0..n).map(|i| fill(i, salt)).collect()
}

// ---------------------------------------------------------------------------
// In-process differential helper
// ---------------------------------------------------------------------------

/// Runs a closure against both implementations and compares the resulting
/// buffers byte-for-byte. The closure receives raw pointers to `a` and `b`
/// so each row can pass nulls, aliases or oversized sizes as needed.
fn diff_inproc<F>(ctx: &str, a0: &[Sprite], b0: &[Sprite], f: F)
where
    F: Fn(&Imp, *mut Sprite, *mut Sprite),
{
    let c = load_c();
    let r = load_rust();

    let (mut ac, mut bc) = (a0.to_vec(), b0.to_vec());
    let (mut ar, mut br) = (a0.to_vec(), b0.to_vec());

    f(&c, ac.as_mut_ptr(), bc.as_mut_ptr());
    f(&r, ar.as_mut_ptr(), br.as_mut_ptr());

    assert_bytes_eq("a", ctx, &ac, &ar);
    assert_bytes_eq("b", ctx, &bc, &br);
}

// ---------------------------------------------------------------------------
// Child-process differential helper (for faulting / memory-smashing rows)
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    signal: Option<i32>,
    code: Option<i32>,
    result: Option<String>,
}

fn run_child(lib: &Path, case: &str) -> Outcome {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args([
            "zzz_child_worker",
            "--exact",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("DIFF_CHILD_LIB", lib)
        .env("DIFF_CHILD_CASE", case)
        .output()
        .expect("spawn child");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let result = stdout
        .lines()
        .find(|l| l.starts_with("RESULT "))
        .map(|l| l["RESULT ".len()..].to_string());
    Outcome {
        signal: out.status.signal(),
        code: out.status.code(),
        result,
    }
}

/// Asserts both implementations terminate identically for an out-of-contract
/// call. `compare_memory` is false for rows whose post-state is ASLR-dependent
/// (a huge backward `memmove` smears over unrelated mappings), where only the
/// termination status is a well-defined observable.
fn diff_child(case: &str, compare_memory: bool) -> Outcome {
    let c = run_child(&c_so_path(), case);
    let r = run_child(&rust_so_path(), case);

    assert_eq!(
        (c.signal, c.code),
        (r.signal, r.code),
        "case `{case}`: termination status differs.\n  C    = {c:?}\n  Rust = {r:?}"
    );
    if compare_memory {
        assert_eq!(
            c.result, r.result,
            "case `{case}`: post-call memory differs.\n  C    = {:?}\n  Rust = {:?}",
            c.result, r.result
        );
    }
    eprintln!(
        "  case {case:<18} signal={:?} code={:?} memcmp={}",
        c.signal,
        c.code,
        if compare_memory { "yes" } else { "status-only" }
    );
    c
}

/// The child worker. Does nothing in the parent process (no env vars set), so
/// it is a harmless no-op test there.
#[test]
fn zzz_child_worker() {
    let (Ok(libp), Ok(case)) = (
        std::env::var("DIFF_CHILD_LIB"),
        std::env::var("DIFF_CHILD_CASE"),
    ) else {
        return; // parent process
    };

    let imp = load_path("child", Path::new(&libp));
    let mut a = make(CHILD_N + SLACK, 0x11);
    let mut b = make(CHILD_N + SLACK, 0x22);

    unsafe {
        if case == "e4_null_scratch" {
            imp.call(a.as_mut_ptr(), std::ptr::null_mut(), 1);
        } else if case == "e5_null_source" {
            imp.call(std::ptr::null_mut(), b.as_mut_ptr(), 1);
        } else if case == "e2_both_null_size0" {
            imp.call(std::ptr::null_mut(), std::ptr::null_mut(), 0);
        } else if let Some(n) = case.strip_prefix("size:") {
            let n: i32 = n.parse().expect("size");
            imp.call(a.as_mut_ptr(), b.as_mut_ptr(), n);
        } else {
            panic!("unknown child case `{case}`");
        }
    }

    println!("RESULT a={} b={}", hex(&a), hex(&b));
    std::io::stdout().flush().ok();
    // Exit before libtest prints its summary so the parent sees a clean status.
    std::process::exit(0);
}

// ===========================================================================
// E1 — size == 0, valid pointers: nothing is written
// ===========================================================================
#[test]
fn err_e1_size_zero_no_writes() {
    let a0 = make(8, 0xA1);
    let b0 = make(8, 0xB1);
    diff_inproc("E1 size=0", &a0, &b0, |imp, a, b| unsafe {
        imp.call(a, b, 0)
    });

    // and confirm the documented behaviour: neither buffer is touched at all
    let c = load_c();
    let r = load_rust();
    for imp in [&c, &r] {
        let (mut a, mut b) = (a0.clone(), b0.clone());
        unsafe { imp.call(a.as_mut_ptr(), b.as_mut_ptr(), 0) };
        assert_eq!(a, a0, "{}: size=0 modified `a`", imp.name);
        assert_eq!(b, b0, "{}: size=0 modified `b`", imp.name);
    }
}

// ===========================================================================
// E2 — size == 0 with NULL pointers: must not dereference
// ===========================================================================
#[test]
fn err_e2_size_zero_null_pointers() {
    // In-process: neither implementation may dereference the null pointers.
    let c = load_c();
    let r = load_rust();
    for imp in [&c, &r] {
        unsafe { imp.call(std::ptr::null_mut(), std::ptr::null_mut(), 0) };
    }
    // Also cross-checked in child processes so a crash would be visible as a
    // signal rather than taking the test runner down.
    let out = diff_child("e2_both_null_size0", true);
    assert_eq!(out.signal, None, "size=0 with NULL pointers must not fault");
    assert_eq!(out.code, Some(0));
}

// ===========================================================================
// E3 — size == 1: recursion base case, b becomes a copy of a
// ===========================================================================
#[test]
fn err_e3_size_one_base_case() {
    for salt in 0..64u64 {
        let a0 = make(4, salt * 7 + 1);
        let b0 = make(4, salt * 13 + 2);
        diff_inproc(&format!("E3 size=1 salt={salt}"), &a0, &b0, |imp, a, b| {
            unsafe { imp.call(a, b, 1) }
        });
    }
    // documented behaviour: a unchanged, b[0] == a[0] (padding included)
    let a0 = make(4, 5);
    let b0 = make(4, 9);
    let c = load_c();
    let r = load_rust();
    for imp in [&c, &r] {
        let (mut a, mut b) = (a0.clone(), b0.clone());
        unsafe { imp.call(a.as_mut_ptr(), b.as_mut_ptr(), 1) };
        assert_eq!(a, a0, "{}: size=1 must leave `a` untouched", imp.name);
        assert_eq!(b[0], a0[0], "{}: size=1 must copy a[0] into b[0]", imp.name);
        assert_eq!(&b[1..], &b0[1..], "{}: size=1 wrote past b[0]", imp.name);
    }
}

// ===========================================================================
// E4 / E5 — NULL scratch / NULL source with size == 1 => identical fault
// ===========================================================================
#[test]
fn err_e4_null_scratch_faults_identically() {
    let out = diff_child("e4_null_scratch", false);
    assert_eq!(
        out.signal,
        Some(11),
        "expected SIGSEGV writing through a NULL scratch pointer, got {out:?}"
    );
}

#[test]
fn err_e5_null_source_faults_identically() {
    let out = diff_child("e5_null_source", false);
    assert_eq!(
        out.signal,
        Some(11),
        "expected SIGSEGV reading through a NULL source pointer, got {out:?}"
    );
}

// ===========================================================================
// E6 — negative size: the int -> size_t sign-extension trap
// ===========================================================================
#[test]
fn err_e6_negative_size_faults_identically() {
    // Small negatives: `sizeof * size` sign-extends to ~2^64 bytes. glibc's
    // memmove treats src/dst as overlapping and copies BACKWARD from a wrapped
    // address, which happens not to fault -- the process survives with status
    // 0. The smeared memory is ASLR-dependent, so only the (stable)
    // termination status is compared; see ERRORS.md.
    for n in [-1, -2, -3] {
        let out = diff_child(&format!("size:{n}"), false);
        assert_eq!(
            (out.signal, out.code),
            (None, Some(0)),
            "size={n}: expected both to survive with status 0, got {out:?}"
        );
    }
    // Larger negatives fault deterministically in both implementations.
    for n in [-1000, -65536, -(1 << 20)] {
        let out = diff_child(&format!("size:{n}"), false);
        assert_eq!(
            out.signal,
            Some(11),
            "size={n}: expected identical SIGSEGV, got {out:?}"
        );
    }
}

// ===========================================================================
// E7 — size == INT_MIN
// ===========================================================================
#[test]
fn err_e7_int_min_size_faults_identically() {
    let out = diff_child(&format!("size:{}", i32::MIN), false);
    assert_eq!(
        out.signal,
        Some(11),
        "size=INT_MIN: expected identical SIGSEGV, got {out:?}"
    );
}

// ===========================================================================
// E8 / E14 — size one step past the real buffer length (both buffers)
// ===========================================================================
#[test]
fn err_e8_size_one_past_buffer() {
    // `SLACK` extra addressable elements keep the out-of-bounds element inside
    // the allocation, so the row is deterministic and can be compared
    // byte-for-byte rather than only by crash signal.
    for n in 0..=16usize {
        let a0 = make(n + SLACK, 0xE8);
        let b0 = make(n + SLACK, 0x8E);
        let size = (n + 1) as i32; // one past the "real" length n
        diff_inproc(
            &format!("E8 real_len={n} size={size}"),
            &a0,
            &b0,
            |imp, a, b| unsafe { imp.call(a, b, size) },
        );
    }
}

// ===========================================================================
// E9 — aliased buffers (a == b)
// ===========================================================================
#[test]
fn err_e9_aliased_buffers() {
    let c = load_c();
    let r = load_rust();
    for n in 0..=32i32 {
        let a0 = make(n.max(0) as usize + SLACK, 0xA9);

        let mut ac = a0.clone();
        let mut ar = a0.clone();
        // Same pointer passed as BOTH the source and the scratch buffer.
        unsafe { c.call(ac.as_mut_ptr(), ac.as_mut_ptr(), n) };
        unsafe { r.call(ar.as_mut_ptr(), ar.as_mut_ptr(), n) };
        assert_bytes_eq("aliased a==b", &format!("E9 size={n}"), &ac, &ar);
    }
}

// ===========================================================================
// E10 — partially overlapping buffers (b == a + 1)
// ===========================================================================
#[test]
fn err_e10_partially_overlapping_buffers() {
    let c = load_c();
    let r = load_rust();
    for n in 0..=32i32 {
        let a0 = make(n.max(0) as usize + SLACK, 0x1A);

        let mut ac = a0.clone();
        let mut ar = a0.clone();
        unsafe {
            let p = ac.as_mut_ptr();
            c.call(p, p.add(1), n);
        }
        unsafe {
            let p = ar.as_mut_ptr();
            r.call(p, p.add(1), n);
        }
        assert_bytes_eq("overlap b=a+1", &format!("E10 size={n}"), &ac, &ar);
    }

    // and the mirror: source one past the scratch buffer
    for n in 0..=32i32 {
        let a0 = make(n.max(0) as usize + SLACK, 0x2B);
        let mut ac = a0.clone();
        let mut ar = a0.clone();
        unsafe {
            let p = ac.as_mut_ptr();
            c.call(p.add(1), p, n);
        }
        unsafe {
            let p = ar.as_mut_ptr();
            r.call(p.add(1), p, n);
        }
        assert_bytes_eq("overlap a=b+1", &format!("E10m size={n}"), &ac, &ar);
    }
}

// ===========================================================================
// E11 — signed comparison at the sort_bits extremes
// ===========================================================================
#[test]
fn err_e11_sort_bits_signed_extremes() {
    let extremes = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    let mut rng = Rng::new(SEED ^ 0xE11);

    // exhaustive over all ordered pairs at size 2 ...
    for &x in &extremes {
        for &y in &extremes {
            let a0 = vec![
                Sprite {
                    texture_id: 1,
                    sort_bits: x,
                    pad: 0xAAAA_AAAA,
                },
                Sprite {
                    texture_id: 2,
                    sort_bits: y,
                    pad: 0x5555_5555,
                },
            ];
            let b0 = make(2, 0x77);
            diff_inproc(&format!("E11 pair ({x},{y})"), &a0, &b0, |imp, a, b| {
                unsafe { imp.call(a, b, 2) }
            });
        }
    }

    // ... and randomized larger arrays drawn only from the extremes
    for size in 2..=32i32 {
        for _ in 0..20 {
            let a0: Vec<Sprite> = (0..size as usize)
                .map(|i| Sprite {
                    texture_id: rng.next_u64(),
                    sort_bits: extremes[rng.below(extremes.len() as u64) as usize],
                    pad: (i as u32) ^ 0xF0F0,
                })
                .collect();
            let b0 = make(size as usize, 0x33);
            diff_inproc(&format!("E11 rand size={size}"), &a0, &b0, |imp, a, b| {
                unsafe { imp.call(a, b, size) }
            });
        }
    }
}

// ===========================================================================
// E12 — texture_id must NEVER affect ordering (lib.c:9 is dead code)
// ===========================================================================
#[test]
fn err_e12_texture_id_never_affects_order() {
    let c = load_c();
    let r = load_rust();
    let mut rng = Rng::new(SEED ^ 0xE12);

    for size in 2..=48i32 {
        for _ in 0..20 {
            // Fixed sort_bits; only texture_id varies between the two runs.
            let keys: Vec<i32> = (0..size as usize)
                .map(|_| rng.below(4) as i32)
                .collect();

            let mk = |tex: &dyn Fn(usize) -> u64| -> Vec<Sprite> {
                (0..size as usize)
                    .map(|i| Sprite {
                        texture_id: tex(i),
                        sort_bits: keys[i],
                        pad: 0,
                    })
                    .collect()
            };

            let ascending = mk(&|i| i as u64);
            let descending = mk(&|i| (size as usize - i) as u64);
            let extreme = mk(&|i| if i % 2 == 0 { 0 } else { u64::MAX });

            for (label, input) in [
                ("asc", &ascending),
                ("desc", &descending),
                ("extreme", &extreme),
            ] {
                let b0 = make(size as usize, 0x9C);
                diff_inproc(
                    &format!("E12 {label} size={size}"),
                    input,
                    &b0,
                    |imp, a, b| unsafe { imp.call(a, b, size) },
                );
            }

            // The dead-branch invariant: the *permutation chosen* depends only
            // on sort_bits, so the sequence of sort_bits in the output is
            // identical no matter what texture_ids are attached, and elements
            // with equal keys keep their original relative order (stability).
            for imp in [&c, &r] {
                let mut outs = Vec::new();
                for input in [&ascending, &descending, &extreme] {
                    let mut a = input.clone();
                    let mut b = make(size as usize, 0x9C);
                    unsafe { imp.call(a.as_mut_ptr(), b.as_mut_ptr(), size) };
                    outs.push(a);
                }
                let key_seq: Vec<Vec<i32>> = outs
                    .iter()
                    .map(|o| o.iter().map(|s| s.sort_bits).collect())
                    .collect();
                assert_eq!(
                    key_seq[0], key_seq[1],
                    "{}: texture_id changed the sort_bits ordering (dead branch became live!)",
                    imp.name
                );
                assert_eq!(
                    key_seq[0], key_seq[2],
                    "{}: extreme texture_ids changed the ordering",
                    imp.name
                );
                // stability: ascending texture_id == original index, so a
                // stable sort must leave those ids ascending within each key
                let asc_out = &outs[0];
                for w in asc_out.windows(2) {
                    if w[0].sort_bits == w[1].sort_bits {
                        assert!(
                            w[0].texture_id < w[1].texture_id,
                            "{}: sort is not stable / texture_id used as tiebreak",
                            imp.name
                        );
                    }
                }
            }
        }
    }
}

// ===========================================================================
// E13 — the `int size` domain: no enum exists, every int is accepted
// ===========================================================================
#[test]
fn err_e13_int_size_domain_sweep() {
    // In-contract sizes: compare memory byte-for-byte.
    for n in 0..=(CHILD_N + SLACK) as i32 {
        let out = diff_child(&format!("size:{n}"), true);
        assert_eq!(
            (out.signal, out.code),
            (None, Some(0)),
            "size={n} must be handled without faulting, got {out:?}"
        );
    }
    // Out-of-contract sizes, i.e. one step past a documented range and beyond.
    // Only the termination status is well defined; it must match exactly.
    for n in [
        (CHILD_N + SLACK) as i32 + 1,
        100,
        1 << 12,
        1 << 20,
        1 << 28,
        i32::MAX - 1,
        i32::MAX,
        -1,
        i32::MIN + 1,
        i32::MIN,
    ] {
        diff_child(&format!("size:{n}"), false);
    }
}

// ===========================================================================
// E15 — (lo + hi) signed overflow near INT_MAX: documented, not runtime-testable
// ===========================================================================
#[test]
fn err_e15_split_overflow_documented_not_testable() {
    // Reaching `(lo + hi)` overflow requires size ~ INT_MAX, i.e. two
    // INT_MAX*16 == 34 GiB buffers. It is discharged by inspecting codegen:
    //   gcc -O0 : add %edx,%eax ; mov %eax,%edx ; shr $0x1f,%edx ;
    //             add %edx,%eax ; sar %eax        (two's-complement add,
    //                                              round-toward-zero divide)
    //   Rust    : lo.wrapping_add(hi) / 2         (identical semantics)
    // Assert here that the Rust source really does use wrapping_add, so this
    // row fails loudly if someone "fixes" it into a checked add later.
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"))
        .expect("read src/lib.rs");
    let code: String = src
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        code.contains("lo.wrapping_add(hi) / 2"),
        "src/lib.rs no longer computes split as `lo.wrapping_add(hi) / 2`; the \
         (lo+hi) overflow behaviour may no longer match gcc's two's-complement add"
    );
    eprintln!("E15 discharged by codegen inspection (see ERRORS.md)");
}
