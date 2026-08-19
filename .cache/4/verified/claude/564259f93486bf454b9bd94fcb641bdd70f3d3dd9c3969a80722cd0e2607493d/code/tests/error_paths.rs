// Phase C — error-path / boundary differential tests.
//
// One test per row of ERRORS.md. `driver` returns `void` and validates nothing,
// so there is no in-band error code to compare; the observable "rejection" is
// either the process termination status (invalid-pointer rows, run in a forked
// child so the fault is measurable) or the exact bytes printed (degenerate but
// valid rows).
//
// Every row asserts C and Rust agree on the SAME specific outcome — the same
// signal number or the same printed bytes — never merely "both failed somehow".

mod common;

use common::*;
use std::ffi::c_char;
use std::ptr;

const NUL: *const c_char = ptr::null();

/// A non-null but unmapped, deliberately misaligned pointer.
const GARBAGE: *const c_char = 1usize as *const c_char;

fn cstr(bytes: &'static [u8]) -> *const c_char {
    assert_eq!(*bytes.last().unwrap(), 0, "literal must be NUL-terminated");
    bytes.as_ptr() as *const c_char
}

/// Shared assertion for the invalid-pointer rows: C and Rust must be rejected by
/// the same specific signal, with nothing printed by either.
fn assert_faults_identically(h: &Harness, label: &str, s1: *const c_char, s2: *const c_char) {
    h.assert_fault_parity(label, s1, s2);
}

// ---------------------------------------------------------------------------
// E1-E5 — null pointers
// ---------------------------------------------------------------------------

#[test]
fn err_e1_s1_null() {
    let h = Harness::new();
    assert_faults_identically(&h, "E1 s1 == NULL, s2 non-empty", NUL, cstr(b"abc\0"));
}

#[test]
fn err_e2_s1_null_s2_empty() {
    // s2 is a valid empty reject set, so the fault must come from s1.
    let h = Harness::new();
    assert_faults_identically(&h, "E2 s1 == NULL, s2 == \"\"", NUL, cstr(b"\0"));
}

#[test]
fn err_e3_s2_null() {
    let h = Harness::new();
    assert_faults_identically(&h, "E3 s2 == NULL, s1 non-empty", cstr(b"abc\0"), NUL);
}

#[test]
fn err_e4_s1_empty_s2_null() {
    // The order-of-evaluation row: s1 is immediately empty, so a library that
    // short-circuits on s1 would never touch s2 and would print "0" instead of
    // faulting. C and Rust must agree on which happens.
    let h = Harness::new();
    assert_faults_identically(&h, "E4 s1 == \"\", s2 == NULL", cstr(b"\0"), NUL);
}

#[test]
fn err_e5_both_null() {
    let h = Harness::new();
    assert_faults_identically(&h, "E5 s1 == NULL and s2 == NULL", NUL, NUL);
}

// ---------------------------------------------------------------------------
// E6-E7 — non-null garbage pointers
// ---------------------------------------------------------------------------

#[test]
fn err_e6_s1_garbage_ptr() {
    let h = Harness::new();
    assert_faults_identically(&h, "E6 s1 == (char*)1", GARBAGE, cstr(b"abc\0"));
}

#[test]
fn err_e7_s2_garbage_ptr() {
    let h = Harness::new();
    assert_faults_identically(&h, "E7 s2 == (char*)1", cstr(b"abc\0"), GARBAGE);
}

// ---------------------------------------------------------------------------
// E8-E9 — unterminated buffers running off a guard page
// ---------------------------------------------------------------------------

#[test]
fn err_e8_s1_unterminated_page_edge() {
    // The whole accessible page is 'a' with no NUL, and s2 rejects only '!', so
    // the scan cannot stop before the PROT_NONE page.
    let h = Harness::new();
    let mut gp = GuardedPage::new();
    let page = gp.page;
    let s1 = gp.unterminated(page, b'a');
    assert_faults_identically(
        &h,
        "E8 s1 unterminated, runs into a PROT_NONE page",
        s1,
        cstr(b"!\0"),
    );
}

#[test]
fn err_e9_s2_unterminated_page_edge() {
    // The reject set has no NUL, so building it must run into the guard page.
    let h = Harness::new();
    let mut gp = GuardedPage::new();
    let page = gp.page;
    let s2 = gp.unterminated(page, b'Z');
    assert_faults_identically(
        &h,
        "E9 s2 unterminated, runs into a PROT_NONE page",
        cstr(b"abc\0"),
        s2,
    );
}

// ---------------------------------------------------------------------------
// E10-E12 — zero-length and oversized-length boundaries (valid, not errors)
// ---------------------------------------------------------------------------

#[test]
fn err_e10_zero_length_both_empty() {
    let h = Harness::new();
    let cases = vec![Case::new(b"", b"")];
    h.assert_same("E10 both arguments zero-length", &cases);
    assert_eq!(
        h.capture_c(&[cases[0].ptrs()]),
        b"0\n",
        "E10 must be accepted and print 0"
    );
}

#[test]
fn err_e11_zero_length_reject_set() {
    // Empty reject set: nothing can be rejected, so the answer is strlen(s1).
    let h = Harness::new();
    let mut rng = Rng::new(SEED ^ 0xE11);
    let mut cases = Vec::new();
    let mut expected = String::new();
    for _ in 0..300 {
        let n = rng.range(1, 200);
        cases.push(Case::raw(rng.string_full_domain(n), vec![0]));
        expected.push_str(&format!("{n}\n"));
    }
    h.assert_same("E11 zero-length reject set", &cases);
    let ptrs: Vec<_> = cases.iter().map(|c| c.ptrs()).collect();
    assert_eq!(
        String::from_utf8_lossy(&h.capture_c(&ptrs)),
        expected,
        "E11 empty s2 must yield strlen(s1)"
    );
}

#[test]
fn err_e12_oversized_length() {
    // 1 MiB with no rejected byte: the largest in-range result, and the widest
    // %zu the library can realistically print.
    let h = Harness::new();
    let mut s1 = vec![b'a'; 1024 * 1024];
    s1.push(0);
    let cases = vec![Case::raw(s1, vec![b'!', 0])];
    h.assert_same("E12 oversized (1 MiB) input", &cases);
    assert_eq!(h.capture_c(&[cases[0].ptrs()]), b"1048576\n");
    assert_eq!(h.capture_rs(&[cases[0].ptrs()]), b"1048576\n");
}

// ---------------------------------------------------------------------------
// E13-E14 — one step past the byte range / sign-extension boundary
// ---------------------------------------------------------------------------

#[test]
fn err_e13_full_byte_domain_reject() {
    // s2 holds every one of the 255 legal non-NUL byte values. Any non-empty s1
    // must therefore yield 0. This is the "value with no valid variant" analogue
    // for this API: it proves the reject table spans the entire unsigned char
    // domain with no off-by-one at either end (0x01 and 0xFF).
    let h = Harness::new();
    let mut rng = Rng::new(SEED ^ 0xE13);
    let mut full = all_nonzero_bytes();
    assert_eq!(full.len(), 255, "there are exactly 255 legal non-NUL bytes");
    full.push(0);

    let mut cases = Vec::new();
    // Every single byte value on its own, plus random strings.
    for b in 1u16..=255 {
        cases.push(Case::raw(vec![b as u8, 0], full.clone()));
    }
    for _ in 0..200 {
        let n = rng.range(1, 120);
        cases.push(Case::raw(rng.string_full_domain(n), full.clone()));
    }
    h.assert_same("E13 full 255-byte reject domain", &cases);
    let ptrs: Vec<_> = cases.iter().map(|c| c.ptrs()).collect();
    assert_eq!(
        h.capture_c(&ptrs),
        "0\n".repeat(cases.len()).into_bytes(),
        "E13 every byte is rejected, so every result must be 0"
    );
}

#[test]
fn err_e14_high_bit_sign_extension() {
    // 0x80..=0xFF are negative when `char` is signed (x86-64). A reject table
    // indexed by a sign-extended char would index out of bounds (Rust panic) or
    // read the wrong slot. Both directions are covered: high bytes in s1 only,
    // in s2 only, and in both.
    let h = Harness::new();
    let mut rng = Rng::new(SEED ^ 0xE14);
    let high: Vec<u8> = (0x80u16..=0xFF).map(|b| b as u8).collect();
    let mut cases = Vec::new();

    // Each high byte individually rejected by itself -> 0.
    for &b in &high {
        cases.push(Case::raw(vec![b, 0], vec![b, 0]));
    }
    // Each high byte in s1, ASCII reject set -> 1.
    for &b in &high {
        cases.push(Case::raw(vec![b, 0], vec![b'a', 0]));
    }
    // Mixed random strings over the high range.
    for _ in 0..400 {
        let n1 = rng.range(1, 80);
        let n2 = rng.range(1, 40);
        cases.push(Case::raw(rng.string_from(n1, &high), rng.string_from(n2, &high)));
    }
    h.assert_same("E14 high-bit / signed-char boundary", &cases);

    // Pin the two deterministic halves so the row cannot pass vacuously.
    let self_reject: Vec<_> = high.iter().map(|&b| Case::raw(vec![b, 0], vec![b, 0])).collect();
    let ptrs: Vec<_> = self_reject.iter().map(|c| c.ptrs()).collect();
    assert_eq!(h.capture_c(&ptrs), "0\n".repeat(high.len()).into_bytes());
    let ascii_reject: Vec<_> =
        high.iter().map(|&b| Case::raw(vec![b, 0], vec![b'a', 0])).collect();
    let ptrs: Vec<_> = ascii_reject.iter().map(|c| c.ptrs()).collect();
    assert_eq!(h.capture_c(&ptrs), "1\n".repeat(high.len()).into_bytes());
}

// ---------------------------------------------------------------------------
// E15 — embedded NUL terminates both arguments
// ---------------------------------------------------------------------------

#[test]
fn err_e15_embedded_nul_terminates() {
    let h = Harness::new();
    // s1 = "ab" + NUL + "XY" + NUL ; s2 = "XY" + NUL + "ab" + NUL.
    // If either library looked past the first NUL, the answer would change:
    //   correct  -> reject set {X,Y}, s1 = "ab", no match  -> 2
    //   if s2 over-read -> reject set includes a,b        -> 0
    let cases = vec![
        Case::raw(b"ab\0XY\0".to_vec(), b"XY\0ab\0".to_vec()),
        Case::raw(b"\0abc\0".to_vec(), b"\0xyz\0".to_vec()),
        Case::raw(b"abc\0".to_vec(), b"\0abc\0".to_vec()),
    ];
    h.assert_same("E15 embedded NUL terminates both arguments", &cases);
    let ptrs: Vec<_> = cases.iter().map(|c| c.ptrs()).collect();
    assert_eq!(
        String::from_utf8_lossy(&h.capture_c(&ptrs)),
        // "ab" vs {X,Y} -> 2 ; empty s1 -> 0 ; "abc" vs empty s2 -> 3
        "2\n0\n3\n",
        "E15 bytes after the first NUL must be invisible"
    );
}

// ---------------------------------------------------------------------------
// Generic FFI-boundary sweep: many invalid pointer shapes at once
// ---------------------------------------------------------------------------

#[test]
fn err_generic_invalid_pointer_matrix() {
    // Cross-product of "bad" pointer values against a valid string, to make sure
    // C and Rust agree on every combination and not just the hand-picked ones.
    let h = Harness::new();
    let valid = cstr(b"hello world\0");
    let bad: [(&str, *const c_char); 5] = [
        ("NULL", ptr::null()),
        ("(char*)1", 1usize as *const c_char),
        ("(char*)8", 8usize as *const c_char),
        ("(char*)0xdead", 0xdeadusize as *const c_char),
        // Non-canonical / kernel-space address.
        ("0xffff_ffff_ffff_f000", 0xffff_ffff_ffff_f000usize as *const c_char),
    ];

    for (name, p) in bad {
        assert_faults_identically(&h, &format!("generic: s1 = {name}"), p, valid);
        assert_faults_identically(&h, &format!("generic: s2 = {name}"), valid, p);
        for (name2, q) in bad {
            assert_faults_identically(&h, &format!("generic: s1 = {name}, s2 = {name2}"), p, q);
        }
    }
}
