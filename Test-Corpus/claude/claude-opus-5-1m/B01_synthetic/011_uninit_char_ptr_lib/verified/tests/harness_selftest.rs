// Negative controls / mutation tests.
//
// A differential suite that always passes is worthless. These tests prove the
// harness and the Phase B/C assertions are actually SENSITIVE: each one builds a
// deliberately WRONG variant of the C source and asserts that the comparison
// used elsewhere in this suite reports a divergence against it.
//
// `c_src/` is never modified -- mutants are generated into `target/`.

mod common;

use common::*;
use std::ffi::c_char;
use std::path::PathBuf;
use std::process::Command;

/// Writes a mutated copy of `c_src/src/driver.c` into `target/` and compiles it.
///
/// Mutants are built at `-O2` deliberately. Two reasons:
///   * a mutant that can reach `bad()` would, at `-O0`, read uninitialized stack
///     residue and may `puts` a wild pointer -- that SIGSEGVs and kills the test
///     process (`catch_unwind` cannot recover from a signal);
///   * at `-O2` `bad()` is the well-defined `printLine(NULL)`, so the mutant's
///     divergence is caused by the mutation alone, not by UB.
fn build_mutant(tag: &str, from: &str, to: &str) -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src = std::fs::read_to_string(manifest.join("c_src/src/driver.c")).expect("read driver.c");
    assert!(
        src.contains(from),
        "mutation anchor {from:?} not found in driver.c"
    );
    let mutated = src.replace(from, to);
    assert_ne!(mutated, src, "mutation {tag} did not change the source");

    let out_dir = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let c_path = out_dir.join(format!("mutant_{tag}.c"));
    let so_path = out_dir.join(format!("libmutant_{tag}.so"));
    std::fs::write(&c_path, mutated).expect("write mutant");

    let status = Command::new("gcc")
        .args(["-O2", "-fPIC", "-shared"])
        .arg("-I")
        .arg(manifest.join("c_src/include"))
        .arg("-o")
        .arg(&so_path)
        .arg(&c_path)
        .status()
        .expect("gcc");
    assert!(status.success(), "failed to build mutant {tag}");
    so_path
}

/// True if `assert_same` reports a divergence for `op`.
fn detects_divergence<F>(a: &Impl, b: &Impl, op: F) -> bool
where
    F: Fn(&Impl) + Copy,
{
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        assert_same(a, b, "selftest", op)
    }))
    .is_err()
}

// ---------------------------------------------------------------------------
// The capture mechanism itself
// ---------------------------------------------------------------------------

#[test]
fn capture_actually_captures_bytes() {
    // If capture_stdout silently returned empty, every differential test would
    // pass vacuously. Pin real, distinguishable byte strings.
    let buf = cstr(b"hello-capture");
    let out = capture_stdout(|| c_default().print_line(buf.as_ptr() as *const c_char));
    assert_eq!(out, b"hello-capture\n", "capture must return the real bytes");

    let good = capture_stdout(|| c_default().good());
    let null = capture_stdout(|| c_default().print_line(std::ptr::null()));
    assert_eq!(good, b"string\n");
    assert!(null.is_empty());
    assert_ne!(good, null, "capture must distinguish different outputs");
}

#[test]
fn capture_is_not_polluted_by_surrounding_output() {
    // Ensure nothing from the harness leaks into a capture window.
    for _ in 0..25 {
        let out = capture_stdout(|| c_default().good());
        assert_eq!(out, b"string\n", "capture window picked up foreign bytes");
    }
    // And an empty capture really is empty.
    for _ in 0..25 {
        let out = capture_stdout(|| {});
        assert!(out.is_empty(), "empty capture returned {}", show(&out));
    }
}

// ---------------------------------------------------------------------------
// Mutation 1: dropped newline -- must be caught
// ---------------------------------------------------------------------------

#[test]
fn mutant_missing_newline_is_detected() {
    let so = build_mutant("nonewline", r#"printf("%s\n", line)"#, r#"printf("%s", line)"#);
    let mutant = Impl::load(&so, "C-mutant(no newline)");

    let buf = cstr(b"payload");
    assert!(
        detects_divergence(&mutant, rust(), |imp| {
            imp.print_line(buf.as_ptr() as *const c_char)
        }),
        "harness FAILED to detect a missing-newline mutation"
    );

    // And the real C build must agree with Rust on the very same input.
    diff("selftest printLine baseline", |imp| {
        imp.print_line(buf.as_ptr() as *const c_char)
    });
}

// ---------------------------------------------------------------------------
// Mutation 2: `useGood > 0` instead of `if (useGood)`
// ---------------------------------------------------------------------------

#[test]
fn mutant_signed_truthiness_is_detected() {
    // This is the classic translation bug for `if (useGood)`. It behaves
    // identically for driver(1), so only the negative boundary values in
    // CONFIGS.md row 16 / ERRORS.md row 9 can catch it. Prove they do.
    let so = build_mutant("gtzero", "if (useGood)", "if (useGood > 0)");
    let mutant = Impl::load(&so, "C-mutant(useGood > 0)");

    // Indistinguishable on the happy path ...
    assert!(
        !detects_divergence(&mutant, rust(), |imp| imp.driver(1)),
        "mutant should be indistinguishable from Rust for driver(1)"
    );

    // ... but caught by the negative boundary values.
    for v in [-1i32, i32::MIN, -42] {
        assert!(
            detects_divergence(&mutant, rust(), |imp| imp.driver(v)),
            "harness FAILED to detect the `useGood > 0` mutation at driver({v})"
        );
    }
}

// ---------------------------------------------------------------------------
// Mutation 3: dropped NULL guard -- the library's only error branch
// ---------------------------------------------------------------------------

#[test]
fn mutant_null_guard_side_effect_is_detected() {
    // NOTE: naively deleting the guard (`if (1)`) is NOT usable as a mutant --
    // gcc rewrites `printf("%s\n", line)` into `puts(line)`, and `puts(NULL)`
    // dereferences null, so that mutant SIGSEGVs and takes the whole test
    // process with it. Instead, give the NULL path an observable, safe side
    // effect: it must be caught by ERRORS.md row 1.
    let so = build_mutant(
        "nullmark",
        r#"    if (line != NULL)
    {
        printf("%s\n", line);
    }
}"#,
        r#"    if (line != NULL)
    {
        printf("%s\n", line);
    }
    else
    {
        printf("(nil)\n");
    }
}"#,
    );
    let mutant = Impl::load(&so, "C-mutant(NULL prints marker)");

    assert!(
        detects_divergence(&mutant, rust(), |imp| imp.print_line(std::ptr::null())),
        "harness FAILED to detect a changed NULL-guard path"
    );

    // Non-NULL inputs stay identical, confirming the mutation is targeted.
    let buf = cstr(b"still-fine");
    assert!(
        !detects_divergence(&mutant, rust(), |imp| {
            imp.print_line(buf.as_ptr() as *const c_char)
        }),
        "nullmark mutant should still agree on non-NULL input"
    );
}

/// Documents the crash discovered while building the mutants above: the C
/// library's `NULL` guard is load-bearing, because gcc lowers
/// `printf("%s\n", line)` to `puts(line)` and `puts(NULL)` segfaults. Both
/// implementations must therefore keep the guard -- which is exactly what
/// ERRORS.md row 1 verifies.
#[test]
fn null_guard_is_load_bearing_in_both_implementations() {
    for imp in [c_default(), c_o2(), rust()] {
        let out = capture_stdout(|| imp.print_line(std::ptr::null()));
        assert!(
            out.is_empty(),
            "{}: printLine(NULL) must be guarded and emit nothing, got {}",
            imp.name,
            show(&out)
        );
    }
}

// ---------------------------------------------------------------------------
// Mutation 4: swapped good/bad branches
// ---------------------------------------------------------------------------

#[test]
fn mutant_swapped_branches_is_detected() {
    let so = build_mutant(
        "swap",
        "    if (useGood)\n    {\n        good();\n    }\n    else\n    {\n        bad();\n    }",
        "    if (useGood)\n    {\n        bad();\n    }\n    else\n    {\n        good();\n    }",
    );
    let mutant = Impl::load(&so, "C-mutant(swapped branches)");

    assert!(
        detects_divergence(&mutant, rust(), |imp| imp.driver(1)),
        "harness FAILED to detect swapped good()/bad() branches"
    );
}
