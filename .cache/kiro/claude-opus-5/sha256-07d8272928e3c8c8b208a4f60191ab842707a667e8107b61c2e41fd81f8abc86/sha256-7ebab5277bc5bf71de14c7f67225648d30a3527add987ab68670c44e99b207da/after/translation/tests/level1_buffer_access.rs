//! Level 1: the buffer-access layer — `can_access_at_index` / `buffer_at_offset`
//! semantics as observed through `parse_number` (guard clauses, truncated
//! lengths, non-zero offsets, empty scans).

mod common;

use common::Harness;

#[test]
fn null_input_buffer_returns_false() {
    Harness::new().check_null_input_buffer();
}

#[test]
fn null_content_returns_false() {
    let h = Harness::new();
    for &(len, off) in &[(0usize, 0usize), (10, 0), (10, 5), (10, 10), (0, 4)] {
        h.check_null_content(len, off);
    }
}

#[test]
fn zero_length_buffer() {
    let h = Harness::new();
    // Non-NULL content but length 0: nothing is accessible, strtod("") fails.
    h.check_raw(b"123", 0, 0, 0);
    h.check_raw(b"", 0, 0, 0);
}

#[test]
fn offset_at_or_past_end() {
    let h = Harness::new();
    h.check_raw(b"123", 3, 3, 0);
    h.check_raw(b"123", 3, 4, 0);
    h.check_raw(b"123", 3, 100, 0);
}

#[test]
fn truncated_lengths_stop_the_scan() {
    let h = Harness::new();
    for s in [
        &b"1234567890"[..],
        b"-1.5e+10",
        b"0.0001",
        b"1e",
        b"1e-",
        b"1e+",
        b"..",
        b"+-+-",
        b"1.2.3",
        b"9999999999999999999999",
    ] {
        h.check_all_lengths(s);
    }
}

#[test]
fn non_zero_offsets() {
    let h = Harness::new();
    for s in [
        &b"[1,2,3]"[..],
        b"  12.5  ",
        b"abc123def",
        b"{\"a\":-4.25e3}",
        b"1.5,2.5",
        b"...1...",
        b"e1e1e1",
    ] {
        h.check_all_offsets(s);
    }
}

#[test]
fn terminator_characters_end_the_scan() {
    let h = Harness::new();
    // Every byte that is *not* in the accepted set must terminate the loop.
    for b in 0u8..=255 {
        let buf = [b'1', b, b'2'];
        h.check_raw(&buf, 3, 0, 0);
        let buf2 = [b, b'1'];
        h.check_raw(&buf2, 2, 0, 0);
    }
}

#[test]
fn buffer_fields_other_than_offset_are_untouched() {
    let h = Harness::new();
    // depth/length must come back unchanged; the harness compares them.
    for depth in [0usize, 1, 42, usize::MAX] {
        h.check_raw(b"3.14159", 7, 0, depth);
    }
}

#[test]
fn no_nul_terminator_needed() {
    let h = Harness::new();
    // The C code copies exactly `number_string_length` bytes; the input need not
    // contain a NUL. Use a buffer whose numeric run reaches the very end.
    let s = b"12345";
    h.check_raw(s, s.len(), 0, 0);
    let s = b"-7.5e2";
    h.check_raw(s, s.len(), 0, 0);
}

#[test]
fn offset_wraps_in_can_access_at_index() {
    let h = Harness::new();
    // `(offset + index) < length` is unsigned arithmetic in C. With a huge
    // offset the very first comparison is false, so nothing is ever read and
    // both sides must agree on the "empty scan" outcome.
    for off in [usize::MAX, usize::MAX - 1, usize::MAX - 3, usize::MAX / 2] {
        h.check_raw(b"12345", 5, off, 0);
    }
}

#[test]
fn length_larger_than_scanned_run() {
    let h = Harness::new();
    // A terminator well before `length` - the scan must stop at the terminator,
    // not at `length`.
    let buf = b"7,000000000000000000";
    h.check_raw(buf, buf.len(), 0, 0);
    let buf = b"1 2345678901234567890";
    h.check_raw(buf, buf.len(), 0, 0);
}
