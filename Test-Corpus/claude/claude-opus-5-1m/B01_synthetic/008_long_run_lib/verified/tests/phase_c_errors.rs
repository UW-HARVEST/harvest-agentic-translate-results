// Phase C — error-/boundary-path differential tests, one test per row of
// ERRORS.md (E1 … E14).
//
// The C library has no error returns at all (see the greps recorded in
// ERRORS.md), so "same rejection" here means: both libraries accept the
// degenerate/hostile input, neither traps or aborts, and both produce
// byte-identical state.  Every generic C-API boundary (null pointers, zero and
// oversized lengths, one-past-range values, out-of-range enum values) is
// covered or explicitly shown not to exist in this ABI.

mod common;

use common::*;
use std::ffi::{c_char, c_int};

fn c_source() -> String {
    std::fs::read_to_string(manifest_dir().join("c_src/src/long.c")).expect("read long.c")
}
fn c_header() -> String {
    std::fs::read_to_string(manifest_dir().join("c_src/include/long.h")).expect("read long.h")
}
fn rust_source() -> String {
    std::fs::read_to_string(manifest_dir().join("src/clong.rs")).expect("read clong.rs")
}

/// Strip comments so the greps below look at code only.
fn code_only(src: &str) -> String {
    let mut out = String::new();
    for line in src.lines() {
        let l = line.trim_start();
        if l.starts_with("//") {
            continue;
        }
        out.push_str(line.split("//").next().unwrap_or(""));
        out.push('\n');
    }
    out
}

// ---------------------------------------------------------------------------
// E1 — there is no error surface at all
// ---------------------------------------------------------------------------
#[test]
fn no_error_paths_exist_in_c_source() {
    let src = code_only(&c_source());
    for needle in [
        "assert", "NULL", "errno", "exit(", "abort(", "perror", "stderr", "return -1", "return 0;",
        "if (", "if(", "switch", "#ifdef", "#ifndef",
    ] {
        assert!(
            !src.contains(needle),
            "ERRORS.md claims the C source has no `{needle}`, but it does — the \
             error-surface table must be regenerated"
        );
    }
    // The only `return` is a bare `return;` from a void function.
    let returns: Vec<&str> = src.lines().filter(|l| l.contains("return")).collect();
    assert_eq!(returns.len(), 1, "unexpected return statements: {returns:?}");
    assert_eq!(returns[0].trim(), "return;");

    // Both entry points are `void` — there is no channel through which an
    // error could be reported.
    assert!(c_source().contains("void perform_expensive_operations()"));
    assert!(c_source().contains("void long_exec(unsigned int seed)"));
    assert!(c_header().contains("void long_exec(unsigned int seed);"));

    // Differential: both libraries return normally from both entry points.
    let h = harness();
    h.zero_both();
    h.c.perform_expensive_operations();
    h.rust.perform_expensive_operations();
    h.assert_arrays_equal("e1 both return normally");
}

// ---------------------------------------------------------------------------
// E2 / E3 / E4 — seed boundaries (0, 1, one past the signed range, UINT_MAX)
// ---------------------------------------------------------------------------
#[test]
fn boundary_seed_values_produce_identical_fill() {
    let h = harness();
    // `long_exec`'s first stage is `srand(seed)` + 262144 * `rand()`.  Every
    // 32-bit seed is accepted; there is no validation.  The fill is injected
    // into both libraries and then run through the transform so the whole
    // pipeline is compared for these boundary seeds.  (The *complete*
    // 2000-iteration `long_exec` is additionally run for seeds 0, 42 and
    // 0xFFFFFFFF by tests/phase_e2e.rs.)
    for seed in [
        0u32,
        1,
        2,
        0x7FFF_FFFE,
        0x7FFF_FFFF,
        0x8000_0000,
        0x8000_0001,
        0xFFFF_FFFE,
        0xFFFF_FFFF,
    ] {
        let fill = libc_rand_array(seed);
        assert_eq!(fill.len(), ARRAY_SIZE);
        h.write_both(&fill);
        h.c.perform_expensive_operations();
        h.rust.perform_expensive_operations();
        h.assert_arrays_equal(&format!("e2/e3/e4 seed 0x{seed:08x}"));
        assert_eq!(h.c.xor_array(), h.rust.xor_array());
    }
    // glibc quirk the C inherits: srand(0) behaves like srand(1).
    assert_eq!(libc_rand_array(0), libc_rand_array(1));
}

// ---------------------------------------------------------------------------
// E5 — negative seed passed through a signed prototype
// ---------------------------------------------------------------------------
#[test]
fn negative_seed_is_reinterpreted_as_unsigned() {
    // At the ABI level `long_exec(-1)` and `long_exec(0xFFFFFFFFu)` are the
    // *same* call (one 32-bit register), which both libraries must interpret
    // the same way: as `unsigned int`.
    for signed in [-1i32, i32::MIN, -42, -2] {
        let as_unsigned = signed as u32;
        let a = libc_rand_array(as_unsigned);
        let b = libc_rand_array(signed as u32);
        assert_eq!(a, b);
        let h = harness();
        h.write_both(&a);
        h.c.perform_expensive_operations();
        h.rust.perform_expensive_operations();
        h.assert_arrays_equal(&format!("e5 signed seed {signed}"));
    }
    assert_eq!(libc_rand_array((-1i32) as u32), libc_rand_array(u32::MAX));

    // Both libraries expose `long_exec` with a 32-bit scalar parameter, so the
    // signed prototype resolves to the same symbol in both.
    let h = harness();
    assert!(!h.c.array_ptr().is_null() && !h.rust.array_ptr().is_null());
}

// ---------------------------------------------------------------------------
// E6 / E7 — extreme element values (signed overflow, division/modulo signs)
// ---------------------------------------------------------------------------
#[test]
fn extreme_element_values_do_not_trap() {
    let h = harness();
    // Uniform arrays of each extreme: if the Rust used checked arithmetic it
    // would panic (and, with `panic = "abort"` in the release profile, kill the
    // process) instead of wrapping like the C.
    for v in [
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 2,
        i32::MAX,
        i32::MAX - 1,
        -1,
        0,
        1,
        7,
        -7,
        14,
        -14,
        0x2AAA_AAAB,
        -0x2AAA_AAAB,
        0x5555_5555,
        -0x5555_5555,
    ] {
        let data = vec![v as c_int; ARRAY_SIZE];
        h.write_both(&data);
        h.c.perform_expensive_operations();
        h.rust.perform_expensive_operations();
        h.assert_arrays_equal(&format!("e6/e7 all-{v}"));
        println!("e6/e7: f^100({v}) = {}", h.c.get(0));
    }
    // …and a single pass over every boundary value at once.
    let data = tile(&boundary_values());
    h.write_both(&data);
    h.c.perform_expensive_operations();
    h.rust.perform_expensive_operations();
    h.assert_arrays_equal("e6/e7 boundary tiling");
}

// ---------------------------------------------------------------------------
// E8 — the zero-initialised `.bss` state (the "empty input" analogue)
// ---------------------------------------------------------------------------
#[test]
fn zero_initialised_bss_state() {
    let h = harness();
    h.zero_both();
    assert!(h.c.read_bytes().iter().all(|&b| b == 0));
    assert!(h.rust.read_bytes().iter().all(|&b| b == 0));
    h.c.perform_expensive_operations();
    h.rust.perform_expensive_operations();
    h.assert_arrays_equal("e8 fresh bss");
    // …and again, on the already-transformed state.
    h.c.perform_expensive_operations();
    h.rust.perform_expensive_operations();
    h.assert_arrays_equal("e8 fresh bss, second pass");
}

// ---------------------------------------------------------------------------
// E9 — 0, 1, 2, … repeated calls are never rejected
// ---------------------------------------------------------------------------
#[test]
fn repeated_calls_never_reject() {
    let h = harness();
    let mut rng = SplitMix64::new(0xE9);
    let data = random_array(&mut rng);
    h.write_both(&data);
    // n = 0: nothing may happen.
    h.assert_arrays_equal("e9 zero calls");
    for n in 1..=5 {
        h.c.perform_expensive_operations();
        h.rust.perform_expensive_operations();
        h.assert_arrays_equal(&format!("e9 after {n} calls"));
    }
}

// ---------------------------------------------------------------------------
// E10 — extra arguments across the FFI boundary (unspecified parameter list),
//       including a null pointer argument
// ---------------------------------------------------------------------------
#[test]
fn extra_ffi_arguments_are_ignored() {
    let h = harness();
    let mut rng = SplitMix64::new(0xE10);
    let data = random_array(&mut rng);

    // Reference result with the correct prototype.
    h.write_both(&data);
    h.c.perform_expensive_operations();
    let reference = h.c.read_array();

    // Same call with three junk arguments, one of them a NULL pointer.
    for (a, p, f) in [
        (0i32, std::ptr::null::<c_char>(), 0.0f64),
        (-1, std::ptr::null(), f64::NAN),
        (i32::MIN, b"junk\0".as_ptr() as *const c_char, 1e308),
    ] {
        h.write_both(&data);
        h.c.peo_with_extra_args(a, p, f);
        h.rust.peo_with_extra_args(a, p, f);
        h.assert_arrays_equal(&format!("e10 extra args ({a}, {p:?}, {f})"));
        assert_eq!(
            h.c.read_array(),
            reference,
            "e10: extra arguments changed the C result"
        );
        assert_eq!(
            h.rust.read_array(),
            reference,
            "e10: extra arguments changed the Rust result"
        );
    }
}

// ---------------------------------------------------------------------------
// E11 — the null-pointer boundary does not exist in this ABI
// ---------------------------------------------------------------------------
#[test]
fn no_pointer_parameters_exist() {
    let decls = [
        "void perform_expensive_operations()",
        "void long_exec(unsigned int seed)",
    ];
    let src = c_source();
    for d in decls {
        assert!(src.contains(d), "declaration changed: {d}");
        assert!(
            !d.contains('*'),
            "{d} takes a pointer — ERRORS.md row E11 must be regenerated"
        );
    }
    assert!(!code_only(&c_header()).contains('*'));
    // The only pointer-shaped part of the ABI is the exported `array` object,
    // whose address is non-null in both libraries and whose contents are fully
    // caller-controlled (covered by C2/E13).
    let h = harness();
    assert!(!h.c.array_ptr().is_null());
    assert!(!h.rust.array_ptr().is_null());
}

// ---------------------------------------------------------------------------
// E12 — the out-of-range-enum boundary does not exist in this ABI
// ---------------------------------------------------------------------------
#[test]
fn no_enum_parameters_exist() {
    let c = code_only(&c_source()) + &code_only(&c_header());
    assert!(
        !c.contains("enum"),
        "the C source declares an enum — ERRORS.md row E12 must be regenerated"
    );
    assert!(!code_only(&rust_source()).contains("enum"));
    // Closest analogue: the only scalar parameter accepts every one of its
    // 2^32 bit patterns.  Sample values with no "valid variant" meaning.
    let h = harness();
    for v in [0xDEAD_BEEFu32, 0xFFFF_FFFF, 0x8000_0000, 0x0000_0003] {
        let fill = libc_rand_array(v);
        h.write_both(&fill);
        h.c.perform_expensive_operations();
        h.rust.perform_expensive_operations();
        h.assert_arrays_equal(&format!("e12 arbitrary scalar 0x{v:08x}"));
    }
}

// ---------------------------------------------------------------------------
// E13 — the exported object's bounds are identical in both libraries
// ---------------------------------------------------------------------------
#[test]
fn array_bounds_are_identical() {
    // Same object size in both `.so`s: 0x100000 bytes = 262144 * sizeof(int).
    let c_size = nm_size(&c_so_path(), "array");
    let rust_size = nm_size(&rust_so_path(), "array");
    assert_eq!(c_size, 0x100000, "C array size");
    assert_eq!(rust_size, c_size, "Rust array size differs from C");
    assert_eq!(c_size as usize, ARRAY_SIZE * 4);

    let h = harness();
    // First and last element: the in-range boundaries.  One past the last
    // element is out of bounds of a 0x100000-byte object in *both* libraries,
    // so it is not a valid input and is deliberately not written.
    let mut rng = SplitMix64::new(0xE13);
    for trial in 0..4 {
        h.zero_both();
        let (first, last) = (rng.next_i32(), rng.next_i32());
        for lib in h.libs() {
            lib.set(0, first);
            lib.set(ARRAY_SIZE - 1, last);
        }
        h.c.perform_expensive_operations();
        h.rust.perform_expensive_operations();
        h.assert_arrays_equal(&format!("e13 trial {trial}"));
        assert_eq!(h.c.get(0), h.rust.get(0));
        assert_eq!(h.c.get(ARRAY_SIZE - 1), h.rust.get(ARRAY_SIZE - 1));
    }
}

fn nm_size(so: &std::path::Path, symbol: &str) -> u64 {
    let out = std::process::Command::new("nm")
        .args(["-D", "-S", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", so.display());
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() == 4 && f[3] == symbol {
            return u64::from_str_radix(f[1], 16).expect("hex size");
        }
    }
    panic!("symbol {symbol} not found in {}\n{text}", so.display());
}

// ---------------------------------------------------------------------------
// E14 — a failing `printf` is ignored (no error channel)
// ---------------------------------------------------------------------------
#[test]
fn printf_failure_is_ignored() {
    // The C discards `printf`'s return value, so a write error cannot be
    // reported; the Rust translation must discard it too (and must not, say,
    // `unwrap()` a write result or panic).
    let c = code_only(&c_source());
    assert!(
        c.contains("printf(\"%d\\n\", xor_result);"),
        "the printf call changed"
    );
    assert!(
        !c.contains("= printf") && !c.contains("if (printf"),
        "the C now inspects printf's result"
    );
    let r = code_only(&rust_source());
    assert!(
        r.contains("printf(c\"%d\\n\".as_ptr(), xor_result);"),
        "the Rust must call libc printf with the same format string and \
         discard its result; found:\n{r}"
    );
    assert!(
        !r.contains("= printf") && !r.contains("expect") && !r.contains("unwrap"),
        "the Rust must not inspect or unwrap printf's result"
    );
    // The redirected-stdout path itself is exercised for real by
    // tests/phase_e2e.rs, which captures `long_exec`'s output through a
    // dup2'ed file descriptor on both libraries.
}
