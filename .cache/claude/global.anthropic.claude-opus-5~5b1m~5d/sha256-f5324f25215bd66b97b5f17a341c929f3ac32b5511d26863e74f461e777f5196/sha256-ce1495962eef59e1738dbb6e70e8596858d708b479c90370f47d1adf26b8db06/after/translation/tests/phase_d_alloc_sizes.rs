//! Allocation-size differential test.
//!
//! The C documents that every OUT pointer is "allocated memory that must be
//! de-allocated by the caller", so the number of bytes asked of `malloc` is part
//! of the observable contract:
//!
//! * `lib.c:77,84,91` — `malloc(match_size + 1)` for `os_major`/`os_minor`/`os_build`
//! * `lib.c:71,95,96,101,105,112,138,143` — `strdup`, i.e. `malloc(strlen + 1)`
//! * `lib.c:24` — `strdup(ARCHS[i])` in `get_os_arch`
//!
//! `malloc_usable_size` is a step function of the request, so an off-by-one in
//! the request is visible whenever the request straddles a 16-byte bin boundary
//! (e.g. `malloc(24)` → 24 usable but `malloc(25)` → 40 usable).
//!
//! The catch: glibc may return a chunk *larger* than the request when it reuses
//! a free chunk, so the value depends on heap history. Comparing C and Rust
//! in-process is therefore invalid — the second call runs against a heap the
//! first call disturbed. Instead each side is measured in its OWN subprocess
//! that performs an IDENTICAL sequence of allocations (both `.so`s are dlopened
//! in both children, in the same order); only the function pointer that is
//! finally called differs, so the two heap histories stay in lockstep.

mod common;

use common::*;

const TARGET_ENV: &str = "DIFF_ALLOC_TARGET";

/// The input corpus. Both children walk it in exactly this order.
///
/// `match_size` is swept across 1..=120 so that every 16-byte bin boundary is
/// straddled for each of `os_major` (`lib.c:77`), `os_minor` (`lib.c:84`) and
/// `os_build` (`lib.c:91`), plus the `strdup`ed fields and `get_os_arch`.
fn corpus() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = Vec::new();
    for n in 1..=120usize {
        let d = vec![b'9'; n];
        let mut push = |parts: &[&[u8]]| {
            let mut x = Vec::new();
            for p in parts {
                x.extend_from_slice(p);
            }
            v.push(x);
        };
        push(&[b"w [Ver: ", &d, b"]"]); // os_major  = malloc(n + 1)
        push(&[b"w [Ver: 1.", &d, b"]"]); // os_minor  = malloc(n + 1)
        push(&[b"w [Ver: 1.2.", &d, b"]"]); // os_build  = malloc(n + 1)
        push(&[b"w [Ver: 1.2.", &d, b".", &d, b"]"]); // multi-dot build group
        push(&[b"h [D: ", &d, b"]"]); // POSIX os_major
        push(&[b"h [D: 1.", &d, b"]"]); // POSIX os_minor
    }
    for n in 0..=120usize {
        let f = vec![b'a'; n];
        let mut push = |parts: &[&[u8]]| {
            let mut x = Vec::new();
            for p in parts {
                x.extend_from_slice(p);
            }
            v.push(x);
        };
        push(&[&f, b" [Ver: 1.2.3]"]); // strdup(os_name), strdup(os_version)
        push(&[b"h [", &f, b": 1.2]"]); // strdup(os_name)
        push(&[b"h [D: ", &f, b"]"]); // strdup(os_version)
        push(&[b"h [D|", &f, b": 1.2]"]); // strdup(os_platform)
        push(&[b"h [D: 1.2 (", &f, b")]"]); // strdup(os_codename)
        push(&[&f, b" x86_64 [D: 1.2]"]); // strdup(os_arch)
        push(&[b"h [", &f, b"]"]); // no-colon strdup(os_name)
    }
    v
}

/// Every ARCHS token, for `get_os_arch`'s own `strdup` (`lib.c:24`).
fn arch_corpus() -> Vec<Vec<u8>> {
    ARCHS.iter().map(|a| a.as_bytes().to_vec()).collect()
}

fn child(target: &str) -> ! {
    let b = both();
    let (parse, arch) = match target {
        "c" => (b.c.parse_uname_string, b.c.get_os_arch),
        "rs" => (b.rs.parse_uname_string, b.rs.get_os_arch),
        other => panic!("unknown target {other}"),
    };
    let mut out = String::new();
    for input in corpus() {
        let o = run_parse_zeroed(parse, &input);
        for sz in &o.sizes {
            out.push_str(&match sz {
                Some(n) => format!("{n},"),
                None => "-,".to_string(),
            });
        }
        out.push('\n');
    }
    for input in arch_corpus() {
        let sz = arch_alloc_size(arch, &input);
        out.push_str(&match sz {
            Some(n) => format!("{n}\n"),
            None => "-\n".to_string(),
        });
    }
    print!("{out}");
    use std::io::Write;
    std::io::stdout().flush().unwrap();
    std::process::exit(0);
}

#[test]
fn alloc_sizes_match_in_lockstep_subprocesses() {
    if let Ok(t) = std::env::var(TARGET_ENV) {
        child(&t);
    }

    let exe = std::env::current_exe().unwrap();
    let run = |target: &str| -> String {
        let out = std::process::Command::new(&exe)
            .args([
                "--exact",
                "alloc_sizes_match_in_lockstep_subprocesses",
                "--nocapture",
            ])
            .env(TARGET_ENV, target)
            .stderr(std::process::Stdio::null())
            .output()
            .expect("spawn child");
        assert!(
            out.status.success(),
            "child {target} failed: {:?}",
            out.status
        );
        String::from_utf8(out.stdout).expect("child stdout is utf8")
    };

    let c = run("c");
    let r = run("rs");

    // Strip libtest's own framing lines; keep only our numeric rows.
    let rows = |s: &str| -> Vec<String> {
        s.lines()
            .filter(|l| {
                !l.is_empty()
                    && l.chars()
                        .all(|ch| ch.is_ascii_digit() || ch == ',' || ch == '-')
            })
            .map(|l| l.to_string())
            .collect()
    };
    let (cr, rr) = (rows(&c), rows(&r));
    let expected = corpus().len() + arch_corpus().len();
    assert_eq!(
        cr.len(),
        expected,
        "C child emitted {} rows, expected {expected}",
        cr.len()
    );
    assert_eq!(rr.len(), cr.len(), "row count differs");

    let inputs = corpus();
    let mut differing = Vec::new();
    for (i, (a, b)) in cr.iter().zip(rr.iter()).enumerate() {
        if a != b {
            let what = if i < inputs.len() {
                format!("parse_uname_string({:?})", String::from_utf8_lossy(&inputs[i]))
            } else {
                format!(
                    "get_os_arch({:?})",
                    String::from_utf8_lossy(&arch_corpus()[i - inputs.len()])
                )
            };
            differing.push(format!("row {i}: {what}\n    C   = {a}\n    Rust= {b}"));
        }
    }
    assert!(
        differing.is_empty(),
        "allocation sizes differ in {} of {} rows (field order: {:?}):\n{}",
        differing.len(),
        cr.len(),
        FIELD_NAMES,
        differing
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );

    // Sanity: the sweep must actually contain a bin-boundary step, otherwise the
    // comparison would be vacuous with respect to off-by-one request sizes.
    let distinct: std::collections::BTreeSet<&String> = cr.iter().collect();
    assert!(
        distinct.len() > 20,
        "the size sweep is too coarse to detect an off-by-one ({} distinct rows)",
        distinct.len()
    );
}
