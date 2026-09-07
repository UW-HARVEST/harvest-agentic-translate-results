#![allow(non_snake_case)]

// Differential tests: C `libdriver.so` vs Rust `libdriver.so`.
//
// Both libraries are loaded with `libloading` and every call goes through the
// exported `extern "C"` symbols, so the Rust `#[no_mangle]` wrappers are under
// test too. No Rust function is ever called directly.
//
//   Phase B (valid paths)  -> `cfg_NN_*` tests, one per row of CONFIGS.md
//   Phase C (error paths)  -> `err_NN_*` tests, one per row of ERRORS.md
//   Phase D (symbol parity)-> `symbols_*` tests

mod common;
use common::*;

// ===========================================================================
// Phase D - symbol parity (also asserted programmatically, not just by nm)
// ===========================================================================

/// The exported-symbol sets of the two shared objects must be identical.
/// Derived from `nm -D --defined-only`, so macro-generated names would show up
/// too; nothing is hard-coded except the assertion that the diff is empty.
#[test]
fn symbols_defined_sets_are_identical() {
    let c = defined_symbols(&c_lib_file());
    let r = defined_symbols(&rust_lib_file());
    assert!(!c.is_empty(), "nm found no exported symbols in the C .so");

    let missing: Vec<_> = c.iter().filter(|s| !r.contains(*s)).cloned().collect();
    assert!(
        missing.is_empty(),
        "SYMBOL PARITY FAILURE: the C .so exports symbols the Rust .so does not: {missing:?}\n\
         C   = {c:?}\n Rust = {r:?}"
    );
    assert_eq!(c, r, "exported symbol sets differ\n C   = {c:?}\n Rust = {r:?}");
}

/// Every C symbol must also be resolvable via `dlsym` in the Rust `.so`.
#[test]
fn symbols_rust_exports_every_c_symbol() {
    for name in defined_symbols(&c_lib_file()) {
        assert!(c_lib().exports(&name), "C .so unexpectedly lacks `{name}`");
        assert!(
            rust_lib().exports(&name),
            "SYMBOL PARITY FAILURE: C .so exports `{name}` but the Rust .so does not"
        );
    }
}

/// Global text/data symbols defined by a shared object, via `nm -D`, excluding
/// the toolchain-generated ELF housekeeping entries that are not part of the
/// library's own API surface.
fn defined_symbols(so: &std::path::Path) -> std::collections::BTreeSet<String> {
    let out = std::process::Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", so.display()));
    assert!(out.status.success(), "nm failed on {}", so.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
            // Keep global text and data; drop ELF/runtime bookkeeping.
            if !matches!(kind, "T" | "D" | "B" | "R" | "W") {
                return None;
            }
            const IGNORED: &[&str] = &[
                "_init",
                "_fini",
                "__bss_start",
                "_edata",
                "_end",
                "rust_eh_personality",
                "__rust_no_alloc_shim_is_unstable_v2",
            ];
            if IGNORED.contains(&name) || name.starts_with("__rust") || name.starts_with("_ZN") {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}

// ===========================================================================
// Phase B - CONFIGS.md rows 1-14: the low-level entry point `foo`
// ===========================================================================

/// Row 1: empty haystack, needle swept over every legal byte `1..=255`.
#[test]
fn cfg_01_foo_empty_haystack_all_needles() {
    for needle in 1u8..=255 {
        let n = diff_foo(b"", needle, "row1 empty haystack");
        assert_eq!(n, 0, "empty haystack must never match");
    }
}

/// Row 2: full 255x255 matrix of one-byte haystack x needle.
#[test]
fn cfg_02_foo_single_byte_matrix() {
    for hay in 1u8..=255 {
        for needle in 1u8..=255 {
            let n = diff_foo(&[hay], needle, "row2 1x1 matrix");
            assert_eq!(n, (hay == needle) as i32, "hay={hay:#04x} needle={needle:#04x}");
        }
    }
}

/// Row 3: random printable-ASCII haystacks, needle `'A'` (driver's first needle).
#[test]
fn cfg_03_foo_random_ascii_needle_A() {
    let mut rng = Rng::new(0x03_A);
    let alphabet: Vec<u8> = (0x20u8..0x7f).collect();
    for i in 0..2000 {
        let len = rng.range(1, 64);
        let hay = rng.bytes_from(&alphabet, len);
        let n = diff_foo(&hay, b'A', &format!("row3 iter {i}"));
        assert_eq!(n, count_bytes(&hay, b'A'));
    }
}

/// Row 4: random printable-ASCII haystacks, needle `'x'` (driver's second needle).
#[test]
fn cfg_04_foo_random_ascii_needle_x() {
    let mut rng = Rng::new(0x04_78);
    let alphabet: Vec<u8> = (0x20u8..0x7f).collect();
    for i in 0..2000 {
        let len = rng.range(1, 64);
        let hay = rng.bytes_from(&alphabet, len);
        let n = diff_foo(&hay, b'x', &format!("row4 iter {i}"));
        assert_eq!(n, count_bytes(&hay, b'x'));
    }
}

/// Row 5: needle guaranteed ABSENT (haystack from a disjoint alphabet) -> 0 path.
#[test]
fn cfg_05_foo_needle_guaranteed_absent() {
    let mut rng = Rng::new(0x05_00);
    for i in 0..2000 {
        let needle = rng.range(1, 127) as u8;
        let alphabet: Vec<u8> = (1u8..=255).filter(|&b| b != needle).collect();
        let len = rng.range(0, 128);
        let hay = rng.bytes_from(&alphabet, len);
        let n = diff_foo(&hay, needle, &format!("row5 iter {i}"));
        assert_eq!(n, 0, "needle was excluded from the alphabet");
    }
}

/// Row 6: exactly one match, at a random position (including first and last).
#[test]
fn cfg_06_foo_single_match_random_position() {
    let mut rng = Rng::new(0x06_01);
    for i in 0..2000 {
        let needle = rng.range(1, 255) as u8;
        let filler: Vec<u8> = (1u8..=255).filter(|&b| b != needle).collect();
        let len = rng.range(1, 64);
        let mut hay = rng.bytes_from(&filler, len);
        let pos = rng.below(len);
        hay[pos] = needle;
        let n = diff_foo(&hay, needle, &format!("row6 iter {i} pos={pos}/{len}"));
        assert_eq!(n, 1);
    }
    // Explicit first-byte and last-byte boundaries.
    for len in 1..=32 {
        let mut hay = vec![b'.'; len];
        hay[0] = b'A';
        assert_eq!(diff_foo(&hay, b'A', "row6 first byte"), 1);
        let mut hay = vec![b'.'; len];
        hay[len - 1] = b'A';
        assert_eq!(diff_foo(&hay, b'A', "row6 last byte"), 1);
    }
}

/// Row 7: haystack is the needle repeated N times (all matches, consecutive).
#[test]
fn cfg_07_foo_all_matches_consecutive() {
    let mut rng = Rng::new(0x07_FF);
    for n in 1usize..=64 {
        let needle = rng.nonzero_byte();
        let hay = vec![needle; n];
        assert_eq!(diff_foo(&hay, needle, &format!("row7 n={n}")), n as i32);
    }
}

/// Row 8: matches separated by exactly one non-matching byte ("AxAxAx...").
#[test]
fn cfg_08_foo_alternating_matches() {
    for reps in 1usize..=64 {
        let mut hay = Vec::new();
        for _ in 0..reps {
            hay.extend_from_slice(b"Ax");
        }
        assert_eq!(diff_foo(&hay, b'A', &format!("row8 reps={reps}")), reps as i32);
        assert_eq!(diff_foo(&hay, b'x', &format!("row8 reps={reps}")), reps as i32);
        // Odd length variant (trailing match).
        hay.push(b'A');
        assert_eq!(diff_foo(&hay, b'A', &format!("row8 odd reps={reps}")), reps as i32 + 1);
    }
}

/// Row 9: two-symbol alphabet with random density -> many matches, random clustering.
#[test]
fn cfg_09_foo_two_symbol_alphabet_random_density() {
    let mut rng = Rng::new(0x09_2A);
    for i in 0..2000 {
        let needle = rng.nonzero_byte();
        let mut other = rng.nonzero_byte();
        if other == needle {
            other = needle.wrapping_add(1).max(1);
        }
        let len = rng.range(0, 256);
        let density = rng.range(0, 100);
        let hay: Vec<u8> = (0..len)
            .map(|_| if rng.range(1, 100) <= density { needle } else { other })
            .collect();
        let n = diff_foo(&hay, needle, &format!("row9 iter {i} density={density}"));
        assert_eq!(n, count_bytes(&hay, needle));
    }
}

/// Row 10: full byte-range haystacks (non-UTF-8 included) and random needles.
#[test]
fn cfg_10_foo_full_byte_range() {
    let mut rng = Rng::new(0x10_FF);
    for i in 0..3000 {
        let len = rng.range(0, 256);
        let hay = rng.nonzero_bytes(len);
        let needle = rng.nonzero_byte();
        let n = diff_foo(&hay, needle, &format!("row10 iter {i}"));
        assert_eq!(n, count_bytes(&hay, needle));
    }
}

/// Row 11: high-bit needles passed as negative `signed char` values.
#[test]
fn cfg_11_foo_negative_char_needles() {
    let mut rng = Rng::new(0x11_80);
    for raw in 0x80u8..=0xff {
        // The haystack deliberately contains the high-bit byte many times.
        let mut hay = Vec::new();
        for _ in 0..rng.range(1, 40) {
            hay.push(raw);
            hay.push(rng.range(0x20, 0x7e) as u8);
        }
        let expected = count_bytes(&hay, raw);
        assert_eq!(
            diff_foo(&hay, raw, &format!("row11 raw={raw:#04x} as i8={}", raw as i8)),
            expected
        );
    }
    // Mixed high-bit and low-bit bytes, randomized.
    for i in 0..1000 {
        let len = rng.range(1, 128);
        let hay: Vec<u8> = (0..len).map(|_| rng.nonzero_byte()).collect();
        let needle = rng.range(0x80, 0xff) as u8;
        let n = diff_foo(&hay, needle, &format!("row11 mixed iter {i}"));
        assert_eq!(n, count_bytes(&hay, needle));
    }
}

/// Row 12: case sensitivity - `'a'` must not match `'A'`.
#[test]
fn cfg_12_foo_case_sensitivity() {
    for n in 1usize..=32 {
        let upper = vec![b'A'; n];
        let lower = vec![b'a'; n];
        assert_eq!(diff_foo(&upper, b'A', "row12"), n as i32);
        assert_eq!(diff_foo(&upper, b'a', "row12"), 0);
        assert_eq!(diff_foo(&lower, b'a', "row12"), n as i32);
        assert_eq!(diff_foo(&lower, b'A', "row12"), 0);
    }
    // 'x' vs 'X' too (driver's second needle).
    let mixed = b"xXxXxXxX";
    assert_eq!(diff_foo(mixed, b'x', "row12"), 4);
    assert_eq!(diff_foo(mixed, b'X', "row12"), 4);
}

/// Row 13: long haystacks (4 KiB and 64 KiB).
#[test]
fn cfg_13_foo_long_haystacks() {
    let mut rng = Rng::new(0x13_00);
    for &len in &[4096usize, 65536] {
        for i in 0..20 {
            let hay = rng.nonzero_bytes(len);
            let needle = rng.nonzero_byte();
            let n = diff_foo(&hay, needle, &format!("row13 len={len} iter {i}"));
            assert_eq!(n, count_bytes(&hay, needle));
        }
    }
}

/// Row 14: oversized haystack - 1 MiB, ~50% match density.
#[test]
fn cfg_14_foo_oversized_haystack() {
    let mut rng = Rng::new(0x14_00);
    let len = 1usize << 20;
    let hay: Vec<u8> = (0..len).map(|_| if rng.below(2) == 0 { b'A' } else { b'.' }).collect();
    let expected = count_bytes(&hay, b'A');
    assert_eq!(diff_foo(&hay, b'A', "row14 1MiB"), expected);
    assert!(expected > 400_000, "expected a large count, got {expected}");
    // Uniform 1 MiB of a single matching byte -> maximal count.
    let hay = vec![b'A'; len];
    assert_eq!(diff_foo(&hay, b'A', "row14 1MiB all match"), len as i32);
}

// (CONFIGS.md rows 15-22 - the `driver` stdout comparisons - live in
// tests/driver_stdout.rs, which runs without the libtest harness so that
// nothing but the library's own output can land in the fd-1 capture.)

// ===========================================================================
// Phase C - ERRORS.md rows 1-13
// ===========================================================================

/// ERRORS row 1: needle absent -> first `strchr` returns NULL -> 0.
#[test]
fn err_01_needle_absent() {
    let mut rng = Rng::new(0xE0_01);
    for i in 0..1000 {
        let needle = rng.nonzero_byte();
        let alphabet: Vec<u8> = (1u8..=255).filter(|&b| b != needle).collect();
        let len = rng.range(1, 128);
        let hay = rng.bytes_from(&alphabet, len);
        assert_eq!(diff_foo(&hay, needle, &format!("err1 iter {i}")), 0);
    }
    // And the wide-needle spelling of the same rejection.
    assert_eq!(diff_foo_wide(b"hello", b'Z' as i32, "err1 wide"), 0);
}

/// ERRORS row 2: empty haystack.
#[test]
fn err_02_empty_string() {
    for needle in 1u8..=255 {
        assert_eq!(diff_foo(b"", needle, "err2"), 0);
    }
    assert_eq!(diff_foo_wide(b"", 1, "err2 wide"), 0);
}

/// ERRORS row 3: match on the very last byte - `s++` must land exactly on the
/// NUL terminator and the next `strchr` must return NULL (no over-read).
#[test]
fn err_03_match_at_last_byte() {
    for len in 1usize..=64 {
        let mut hay = vec![b'.'; len];
        hay[len - 1] = b'A';
        assert_eq!(diff_foo(&hay, b'A', &format!("err3 len={len}")), 1);
    }
    // Trailing run of matches, so several increments happen near the terminator.
    for run in 1usize..=32 {
        let mut hay = vec![b'.'; 8];
        hay.extend(std::iter::repeat(b'A').take(run));
        assert_eq!(diff_foo(&hay, b'A', &format!("err3 run={run}")), run as i32);
    }
}

/// ERRORS row 4: single-character haystack equal to the needle.
#[test]
fn err_04_single_char_match() {
    for b in 1u8..=255 {
        assert_eq!(diff_foo(&[b], b, &format!("err4 b={b:#04x}")), 1);
    }
}

/// ERRORS row 5: consecutive matches - no double counting, no skipping.
#[test]
fn err_05_consecutive_matches() {
    for n in 1usize..=128 {
        assert_eq!(diff_foo(&vec![b'A'; n], b'A', &format!("err5 n={n}")), n as i32);
    }
    // Runs embedded in filler.
    let hay = b"..AAA..A....AAAAA.A";
    assert_eq!(diff_foo(hay, b'A', "err5 embedded"), count_bytes(hay, b'A'));
}

/// ERRORS row 6: needle one step past the positive `char` range, i.e. negative
/// `signed char` values `-128..=-1` (bytes `0x80..=0xFF`).
#[test]
fn err_06_negative_char_needle() {
    let mut rng = Rng::new(0xE0_06);
    for raw in 0x80u8..=0xff {
        let mut hay = rng.nonzero_bytes(64);
        hay[rng.below(64)] = raw;
        let expected = count_bytes(&hay, raw);
        assert_eq!(diff_foo(&hay, raw, &format!("err6 raw={raw:#04x}")), expected);
    }
    // The two extremes, on haystacks made only of those bytes.
    for raw in [0x80u8, 0xff] {
        assert_eq!(diff_foo(&[raw; 10], raw, "err6 extreme"), 10);
        let other = if raw == 0xff { 0x80 } else { 0xff };
        assert_eq!(diff_foo(&[other; 10], raw, "err6 extreme miss"), 0);
    }
    // 0x7f / 0x80 boundary pair.
    assert_eq!(diff_foo(b"\x7f\x80\x7f\x80", 0x7f, "err6 boundary"), 2);
    assert_eq!(diff_foo(b"\x7f\x80\x7f\x80", 0x80, "err6 boundary"), 2);
}

/// ERRORS row 7: out-of-range integer needle pushed across the FFI boundary as
/// a full-width `int` (the "out-of-range enum value" class). C performs no
/// check; both libraries must agree on whatever the ABI yields.
#[test]
fn err_07_needle_int_out_of_char_range() {
    let hay = b"AAxx\x41\x80\xff. ";
    let cases: &[i32] = &[
        0, 1, 0x41, 0x78, 0x7f, 0x80, 0xff, 0x100, 0x141, 0x178, 256, 257, 511, 512, 1000, 65_601,
        0x7fff_ffff, -1, -2, -128, -129, -256, -1000, -65_535, i32::MIN, i32::MIN + 1,
    ];
    for &c in cases {
        if c == 0 {
            continue; // NUL needle is undefined behaviour - see err_08.
        }
        // Skip values whose low byte is 0: they degenerate into the NUL-needle
        // case inside strchr and are therefore also undefined.
        if (c as u8) == 0 {
            continue;
        }
        diff_foo_wide(hay, c, &format!("err7 needle={c}"));
    }
    // Randomized sweep of arbitrary 32-bit needles.
    let mut rng = Rng::new(0xE0_07);
    for i in 0..2000 {
        let c = rng.next_u64() as i32;
        if (c as u8) == 0 {
            continue;
        }
        let len = rng.range(1, 64);
        let hay = rng.nonzero_bytes(len);
        diff_foo_wide(&hay, c, &format!("err7 random iter {i} needle={c}"));
    }
}

/// ERRORS row 8: `c == 0`.
///
/// `strchr(s, 0)` SUCCEEDS (it returns a pointer to the terminator), the count
/// is incremented, and `s++` then steps PAST the terminator, so the C loop
/// reads out of bounds and can only stop on a fault. This is undefined
/// behaviour with no defined C result, so there is nothing to be byte-identical
/// to and the case is deliberately not invoked.
///
/// What IS asserted here is that the Rust translation is structurally the same
/// program: it calls the same libc `strchr` (rather than, say, a bounded
/// `CStr::to_bytes().iter()` scan that would terminate and return 1) and it
/// applies the same post-increment. Both libraries therefore import `strchr`
/// from libc, and the near-terminator behaviour they share is pinned by
/// `err_03`.
#[test]
fn err_08_nul_needle_is_undefined_behaviour() {
    let c_so = c_lib_file();
    let out = std::process::Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(&c_so)
        .output()
        .expect("run nm on the C .so");
    let c_undef = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        c_undef.contains("strchr"),
        "expected the C .so to import strchr, got:\n{c_undef}"
    );

    // The Rust .so must import the very same libc routine.
    let rust_so = rust_lib_file();
    let out = std::process::Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(&rust_so)
        .output()
        .expect("run nm on the Rust .so");
    let r_undef = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        r_undef.contains("strchr"),
        "the Rust .so must delegate to libc strchr so that its scanning \
         semantics (including the undefined c==0 case) are identical to the C; \
         undefined symbols were:\n{r_undef}"
    );
    assert!(
        r_undef.contains("printf"),
        "the Rust .so must use libc printf for identical stdout formatting; got:\n{r_undef}"
    );
}

/// ERRORS rows 9 and 10: `in == NULL` for `foo` and for `driver`.
///
/// `strchr(NULL, c)` dereferences a null pointer, so the C aborts with a fatal
/// signal. The assertion is that BOTH libraries are rejected the same way -
/// same fatal signal - rather than one returning while the other dies. Each
/// call happens in a forked child process (`current_exe` re-invoked with
/// `DRIVER_DIFF_CRASH` set) so the test runner survives.
#[test]
fn err_09_null_pointer_segfaults_in_both() {
    use std::os::unix::process::ExitStatusExt;

    let mut results = Vec::new();
    for target in ["c-foo", "rust-foo", "c-driver", "rust-driver"] {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "--nocapture", "crash_child_do_not_run_directly"])
            .env("DRIVER_DIFF_CRASH", target)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .expect("spawn crash child");
        results.push((target, status.signal(), status.code()));
    }

    for (target, sig, code) in &results {
        assert!(
            sig.is_some(),
            "{target}: expected a fatal signal from the NULL-pointer call, \
             but the child exited normally with code {code:?}"
        );
    }
    // C `foo` and Rust `foo` must be rejected with the SAME signal, likewise driver.
    assert_eq!(
        results[0].1, results[1].1,
        "foo(NULL): C died with signal {:?} but Rust died with signal {:?}",
        results[0].1, results[1].1
    );
    assert_eq!(
        results[2].1, results[3].1,
        "driver(NULL): C died with signal {:?} but Rust died with signal {:?}",
        results[2].1, results[3].1
    );
}

/// Helper for `err_09`. Does nothing unless `DRIVER_DIFF_CRASH` is set, so a
/// normal test run simply passes it.
#[test]
fn crash_child_do_not_run_directly() {
    let Ok(target) = std::env::var("DRIVER_DIFF_CRASH") else {
        return;
    };
    let null: *const std::os::raw::c_char = std::ptr::null();
    unsafe {
        match target.as_str() {
            "c-foo" => {
                std::hint::black_box((c_lib().foo())(null, b'A' as i8));
            }
            "rust-foo" => {
                std::hint::black_box((rust_lib().foo())(null, b'A' as i8));
            }
            "c-driver" => (c_lib().driver())(null),
            "rust-driver" => (rust_lib().driver())(null),
            other => panic!("unknown crash target {other}"),
        }
    }
    // Reached only if the NULL dereference did not fault.
    println!("no fault for {target}");
}

/// ERRORS row 11: oversized input, far beyond any plausible internal buffer.
#[test]
fn err_11_oversized_input() {
    let mut rng = Rng::new(0xE0_11);
    let len = 1usize << 20;
    let hay: Vec<u8> = (0..len).map(|_| if rng.below(2) == 0 { b'Z' } else { b'q' }).collect();
    let expected = count_bytes(&hay, b'Z');
    assert_eq!(diff_foo(&hay, b'Z', "err11 1MiB"), expected);
    // Long haystack with no match at all (worst case for the scan).
    let hay = vec![b'q'; len];
    assert_eq!(diff_foo(&hay, b'Z', "err11 1MiB no match"), 0);
}

/// ERRORS row 12 (`foo` half): invalid-UTF-8 input must be treated as plain
/// bytes. The `driver` half is in tests/driver_stdout.rs.
#[test]
fn err_12_invalid_utf8_input() {
    // Lone continuation bytes, truncated sequences, overlong forms, surrogates.
    let cases: &[&[u8]] = &[
        b"\x80",
        b"\xbf\xbf\xbf",
        b"\xc3",
        b"\xc3\x28",
        b"\xe2\x82",
        b"\xf0\x9f\x92",
        b"\xf8\xa1\xa1\xa1\xa1",
        b"\xc0\xaf",
        b"\xed\xa0\x80",
        b"A\xffx\xfeA\xfdx",
        b"\xff\xfe\xfd\xfc\xfb\xfa",
    ];
    for hay in cases {
        for needle in 1u8..=255 {
            let n = diff_foo(hay, needle, "err12 invalid utf8");
            assert_eq!(n, count_bytes(hay, needle));
        }
    }
    // Randomized invalid-UTF-8 soup.
    let mut rng = Rng::new(0xE0_12);
    for i in 0..500 {
        let len = rng.range(1, 64);
        let hay: Vec<u8> = (0..len).map(|_| rng.range(0x80, 0xff) as u8).collect();
        let needle = rng.range(0x80, 0xff) as u8;
        let n = diff_foo(&hay, needle, &format!("err12 rand {i}"));
        assert_eq!(n, count_bytes(&hay, needle));
    }
}

/// ERRORS row 13 (`foo` half): `%`-bearing input is ordinary data. The stdout
/// side (input must never be used as a format string) is in
/// tests/driver_stdout.rs.
#[test]
fn err_13_format_specifiers_in_input() {
    let cases: &[&[u8]] = &[
        b"%s",
        b"%n",
        b"%d %d %d %d %d",
        b"%s%s%s%s%s%s%s%s",
        b"%n%n%n%n",
        b"A%sx%nA",
        b"%%%%",
        b"%1000000d",
        b"%.999999f",
        b"%p %p %p",
        b"A: %d\nx: %d\n",
    ];
    for hay in cases {
        for needle in [b'A', b'x', b'%', b'n', b's', b'd'] {
            assert_eq!(diff_foo(hay, needle, "err13 foo"), count_bytes(hay, needle));
        }
    }
}
