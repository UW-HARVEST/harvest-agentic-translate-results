// CONFIGS.md row 44 — the whole public surface re-run on a heap that is
// pre-filled with a non-zero pattern.
//
// Why this row exists: two buffers in the C are read after only a *partial*
// write, so their tails carry whatever `malloc()` handed out —
//
//   * the 64-byte block from `create_result_string`, of which `snprintf` writes
//     only the formatted prefix plus a NUL, and
//   * `Result.operation[32]`, of which `strcpy` writes only `strlen(lit) + 1`
//     bytes before `printf("Operation performed: %s\n", ...)` walks it.
//
// On a fresh heap those tails are zero-filled kernel pages, so a translation
// bug such as a `strcpy` that forgets its NUL terminator, or an `snprintf`
// bound that is off by one, produces output *identical* to the C and slips
// through every other test.  The interposer in `tests/fixtures/fail_malloc.c`
// fills every fresh block with a chosen non-zero byte, which turns such bugs
// into a visible difference while keeping the comparison meaningful: both
// libraries see the exact same pattern on every allocation.
//
// The sweep also hex-dumps all 64 bytes of every returned block, so the bytes
// past the NUL are compared too, not just the C string.

mod common;

use common::*;

/// Fill bytes worth trying: an arbitrary pattern, the two extremes, an ASCII
/// digit (so a stray byte would look like part of a number) and 0x2c (`,`,
/// a byte that appears in the format string itself).
const FILL_BYTES: &[&str] = &["171", "1", "255", "48", "44", "127", "128"];

fn sweep_with_fill(fill: &str) -> String {
    run_child("sweep", 0, &[("CDIFF_FILL_BYTE", fill)], true)
}

#[test]
fn row44_full_sweep_on_filled_heap() {
    for f in FILL_BYTES {
        let report = sweep_with_fill(f);
        // sanity: the sweep really did drive the whole surface
        assert_c_section_contains(
            &report,
            &[
                "CRS len=0 val=0:",
                "CRS len=70 val=42:",
                "CRS NULL:",
                "MWL 6 7: RET=42",
                "CM 1 6 7 8: RET=13",
                "CM 2 6 7 8: RET=42",
                "CM 3 6 7 8: RET=21",
                "CM 4 6 7 8: RET=21",
                "CM -1 6 7 8: RET=-1",
                "Operation performed: multiplication",
                "CHK 420 420: 1",
                "ADD 420: -2",
                "CAS 3: -12",
                "CAS -1: -1",
                "CMP: 0",
            ],
        );
        assert_sections_match(&report);
    }
}

/// The fill must actually reach the bytes being compared, otherwise the row
/// above would be vacuous.  Two different fill bytes must give two different
/// (but each internally consistent) reports.
#[test]
fn row44_control_fill_is_effective() {
    let a = sweep_with_fill("171");
    let b = sweep_with_fill("1");
    assert_sections_match(&a);
    assert_sections_match(&b);

    assert_ne!(
        c_section(&a),
        c_section(&b),
        "the heap fill does not reach the compared bytes — row 44 would be vacuous"
    );

    // fill 171 == 0xab: the tail after "Operation: , Value: 0\0" must be 0xab.
    let line = c_section(&a)
        .lines()
        .find(|l| l.starts_with("CRS len=0 val=0:"))
        .expect("sweep line missing");
    assert!(
        line.ends_with("abababab"),
        "expected 0xab heap padding in {line}"
    );
    // ... and the same line under fill 1 must end in 0x01 padding.
    let line = c_section(&b)
        .lines()
        .find(|l| l.starts_with("CRS len=0 val=0:"))
        .expect("sweep line missing");
    assert!(
        line.ends_with("01010101"),
        "expected 0x01 heap padding in {line}"
    );
}

/// A run with the heap fill *and* an armed malloc failure: the error branches
/// must still agree byte for byte when the heap is not zero-filled.
#[test]
fn row44_filled_heap_with_malloc_failure() {
    for (scenario, size, needle) in [
        ("crs", 64u64, "RET_PTR=<NULL>"),
        ("mwl", 64, "RET=0 LOG=<NULL>"),
        ("cm2", 64, "Log message creation failed"),
        ("cm1", 40, "Failed to allocate result tracker"),
        ("cm4", 40, "Failed to allocate result tracker"),
        ("cas", 12, "Memory allocation failed"),
    ] {
        for fill in ["171", "255"] {
            let report = run_child(scenario, size, &[("CDIFF_FILL_BYTE", fill)], true);
            assert_c_section_contains(&report, &[needle]);
            assert_sections_match(&report);
        }
    }
}

/// glibc's own `MALLOC_PERTURB_` as a second, independent poisoning mechanism.
/// It cannot be used for the raw-tail dumps (the tcache fast path bypasses
/// alloc_perturb, so recycled chunks keep history-dependent bytes), but it does
/// exercise a differently laid-out heap end to end, so the *printed* output and
/// return values must still match.
#[test]
fn row44_malloc_perturb_variant() {
    for p in ["170", "1", "255"] {
        let report = run_child("cm2", 0, &[("MALLOC_PERTURB_", p)], false);
        assert_c_section_contains(&report, &["Mode 2: Operation: multiply, Value: 42", "RET=42"]);
        assert_sections_match(&report);
        for scenario in ["cm1", "cm3", "cm4", "cm9"] {
            let report = run_child(scenario, 0, &[("MALLOC_PERTURB_", p)], false);
            assert_sections_match(&report);
        }
    }
}

// ===========================================================================
// CONFIGS.md row 45 — malloc() request-size sequences
//
// Some divergences leave the observable *result* unchanged while asking the
// allocator for a different amount of memory.  The canonical example is
// `count * sizeof(int)` in `copy_and_sum`: the C converts the `int` to `size_t`
// (sign extension), so `count == -1` asks for 18446744073709551612 bytes.  A
// translation that zero-extends asks for 17179869180 bytes instead — both fail
// on any normal host, so the return value and the printed message are identical
// and every outcome-based test passes.  Comparing the logged request sizes is
// what makes that class of bug visible.
// ===========================================================================

#[test]
fn row45_malloc_request_sizes_match() {
    let report = run_child("sizes", 0, &[], true);
    assert_c_section_contains(
        &report,
        &[
            // create_result_string always asks for exactly 64 bytes
            "SIZES crs len=0: [64]",
            "SIZES crs len=70: [64]",
            "SIZES mwl 6 7: [64]",
            // copy_and_sum: count * sizeof(int), sign-extended through size_t
            "SIZES cas 0: [0]",
            "SIZES cas 1: [4]",
            "SIZES cas 3: [12]",
            "SIZES cas 17: [68]",
            "SIZES cas 64: [256]",
            "SIZES cas -1: [18446744073709551612]",
            "SIZES cas -2: [18446744073709551608]",
            "SIZES cas -1024: [18446744073709547520]",
            "SIZES cas -2147483648: [18446744065119617024]",
            // complexmode: 40-byte Result tracker, plus mode-specific blocks
            "SIZES cm 1: [40]",
            "SIZES cm 2: [40,64]",
            "SIZES cm 3: [40,12]",
            "SIZES cm 4: [40]",
            "SIZES cm 0: [40]",
            "SIZES cm -1: [40]",
        ],
    );
    assert_sections_match(&report);
}

/// The same size log with the heap fill and a failure armed, so the size
/// sequence is compared on the error paths too (a failed malloc is still
/// logged).
#[test]
fn row45_malloc_request_sizes_under_injection() {
    for size in [40u64, 64, 12] {
        let report = run_child("sizes", size, &[("CDIFF_FILL_BYTE", "171")], true);
        assert!(
            c_section(&report).contains("SIZES cm 2: [40"),
            "size log missing:\n{report}"
        );
        assert_sections_match(&report);
    }
}
