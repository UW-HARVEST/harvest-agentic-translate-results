// Crate name `CircularBuffer` happens to match the type name. Import via an
// alias so the test code is unambiguous. Allow `unused_imports` for the case
// when building the binary without `--test` (test fns are gated by `#[test]`).
#[allow(unused_imports)]
use CircularBuffer::circular_buffer::CircularBuffer as CB;

// ---------------------------------------------------------------------------
// Constructors / sizing
// ---------------------------------------------------------------------------

#[test]
fn test_new_capacity_and_size() {
    let cb = CB::new(8);
    assert_eq!(cb.get_capacity(), 8);
    assert_eq!(cb.get_size(), 8);
    assert_eq!(cb.get_data_size(), 0);
}

#[test]
fn test_new_various_sizes() {
    for s in [1usize, 2, 4, 5, 7, 16, 64, 1024] {
        let cb = CB::new(s);
        assert_eq!(cb.get_capacity(), s);
        assert_eq!(cb.get_size(), s);
        assert_eq!(cb.get_data_size(), 0);
    }
}

#[test]
fn test_get_size_equals_get_capacity() {
    let cb = CB::new(7);
    assert_eq!(cb.get_size(), cb.get_capacity());
    assert_eq!(cb.get_size(), 7);
}

// ---------------------------------------------------------------------------
// Reset
// ---------------------------------------------------------------------------

#[test]
fn test_reset_empty_buffer() {
    let mut cb = CB::new(5);
    cb.reset();
    assert_eq!(cb.get_data_size(), 0);
    assert_eq!(cb.get_capacity(), 5);
}

#[test]
fn test_reset_after_push() {
    let mut cb = CB::new(5);
    cb.push(b"abcde", 5);
    assert_eq!(cb.get_data_size(), 5);
    cb.reset();
    assert_eq!(cb.get_data_size(), 0);
    assert_eq!(cb.get_capacity(), 5);
}

#[test]
fn test_push_after_reset() {
    let mut cb = CB::new(5);
    cb.push(b"abcde", 5);
    cb.reset();
    cb.push(b"XYZ", 3);
    assert_eq!(cb.get_data_size(), 3);

    let mut out = [0u8; 10];
    let n = cb.read(10, &mut out);
    assert_eq!(n, 3);
    assert_eq!(&out[..3], b"XYZ");
}

// ---------------------------------------------------------------------------
// Push: zero / empty cases
// ---------------------------------------------------------------------------

#[test]
fn test_push_zero_length_is_noop() {
    let mut cb = CB::new(5);
    cb.push(b"abc", 3);
    assert_eq!(cb.get_data_size(), 3);
    cb.push(b"xyz", 0);
    assert_eq!(cb.get_data_size(), 3);

    let mut out = [0u8; 10];
    let n = cb.read(10, &mut out);
    assert_eq!(n, 3);
    assert_eq!(&out[..3], b"abc");
}

#[test]
fn test_push_zero_length_into_empty() {
    let mut cb = CB::new(5);
    cb.push(b"abc", 0);
    assert_eq!(cb.get_data_size(), 0);
}

// ---------------------------------------------------------------------------
// Push: simple within-capacity
// ---------------------------------------------------------------------------

#[test]
fn test_push_simple_partial() {
    let mut cb = CB::new(5);
    cb.push(b"ABC", 3);
    assert_eq!(cb.get_data_size(), 3);

    let mut out = [0u8; 10];
    let n = cb.read(10, &mut out);
    assert_eq!(n, 3);
    assert_eq!(&out[..3], b"ABC");
}

#[test]
fn test_push_exact_capacity() {
    let mut cb = CB::new(5);
    cb.push(b"ABCDE", 5);
    assert_eq!(cb.get_data_size(), 5);

    let mut out = [0u8; 10];
    let n = cb.read(10, &mut out);
    assert_eq!(n, 5);
    assert_eq!(&out[..5], b"ABCDE");
}

#[test]
fn test_push_more_than_capacity_keeps_last_n() {
    // C behavior: when pushing more bytes than capacity, only the last
    // `capacity` bytes are kept.
    let mut cb = CB::new(4);
    cb.push(b"0123456789", 10);
    assert_eq!(cb.get_data_size(), 4);

    let mut out = [0u8; 10];
    let n = cb.read(10, &mut out);
    assert_eq!(n, 4);
    assert_eq!(&out[..4], b"6789");
}

#[test]
fn test_two_pushes_fill_exactly() {
    // size 8: push 5 then push 3 → buffer is exactly full with no wrap.
    let mut cb = CB::new(8);
    cb.push(b"01234", 5);
    assert_eq!(cb.get_data_size(), 5);
    cb.push(b"567", 3);
    assert_eq!(cb.get_data_size(), 8);

    let mut out = [0u8; 16];
    let n = cb.read(16, &mut out);
    assert_eq!(n, 8);
    assert_eq!(&out[..8], b"01234567");
}

// ---------------------------------------------------------------------------
// Pop on empty / zero length
// ---------------------------------------------------------------------------

#[test]
fn test_pop_on_empty_returns_zero() {
    let mut cb = CB::new(5);
    let mut out = [0u8; 10];
    let n = cb.pop(5, &mut out);
    assert_eq!(n, 0);
    assert_eq!(cb.get_data_size(), 0);
}

#[test]
fn test_pop_zero_length_returns_zero() {
    let mut cb = CB::new(5);
    cb.push(b"abc", 3);
    let mut out = [0u8; 10];
    let n = cb.pop(0, &mut out);
    assert_eq!(n, 0);
    assert_eq!(cb.get_data_size(), 3);
}

#[test]
fn test_read_on_empty_returns_zero() {
    let cb = CB::new(5);
    let mut out = [0u8; 10];
    let n = cb.read(5, &mut out);
    assert_eq!(n, 0);
}

#[test]
fn test_read_zero_length_returns_zero() {
    let mut cb = CB::new(5);
    cb.push(b"abc", 3);
    let mut out = [0u8; 10];
    let n = cb.read(0, &mut out);
    assert_eq!(n, 0);
    assert_eq!(cb.get_data_size(), 3);
}

// ---------------------------------------------------------------------------
// Read does not advance / Pop does
// ---------------------------------------------------------------------------

#[test]
fn test_read_does_not_consume() {
    let mut cb = CB::new(8);
    cb.push(b"ABCDEFGH", 8);

    let mut out = [0u8; 10];
    let n = cb.read(4, &mut out);
    assert_eq!(n, 4);
    assert_eq!(&out[..4], b"ABCD");
    assert_eq!(cb.get_data_size(), 8);

    // Read again — should yield the same data and not advance.
    let mut out2 = [0u8; 10];
    let n2 = cb.read(4, &mut out2);
    assert_eq!(n2, 4);
    assert_eq!(&out2[..4], b"ABCD");
    assert_eq!(cb.get_data_size(), 8);
}

#[test]
fn test_pop_consumes() {
    let mut cb = CB::new(8);
    cb.push(b"ABCDEFGH", 8);

    let mut out = [0u8; 10];
    let n = cb.pop(4, &mut out);
    assert_eq!(n, 4);
    assert_eq!(&out[..4], b"ABCD");
    assert_eq!(cb.get_data_size(), 4);

    // Subsequent read sees only "EFGH"
    let mut out2 = [0u8; 10];
    let n2 = cb.read(10, &mut out2);
    assert_eq!(n2, 4);
    assert_eq!(&out2[..4], b"EFGH");
}

#[test]
fn test_pop_more_than_available_returns_available() {
    let mut cb = CB::new(8);
    cb.push(b"abcde", 5);
    let mut out = [0u8; 30];
    let n = cb.pop(30, &mut out);
    assert_eq!(n, 5);
    assert_eq!(&out[..5], b"abcde");
    assert_eq!(cb.get_data_size(), 0);
}

#[test]
fn test_read_more_than_available_returns_available() {
    let mut cb = CB::new(8);
    cb.push(b"abcde", 5);
    let mut out = [0u8; 30];
    let n = cb.read(30, &mut out);
    assert_eq!(n, 5);
    assert_eq!(&out[..5], b"abcde");
    assert_eq!(cb.get_data_size(), 5);
}

// ---------------------------------------------------------------------------
// Wrap-around scenarios (data spans end of buffer)
// ---------------------------------------------------------------------------

#[test]
fn test_wrap_around_push_then_read() {
    // size 5: fill, pop 3, push 2 → contents wrap around end of underlying buffer.
    let mut cb = CB::new(5);
    cb.push(b"ABCDE", 5);

    let mut out = [0u8; 10];
    let n = cb.pop(3, &mut out);
    assert_eq!(n, 3);
    assert_eq!(&out[..3], b"ABC");
    assert_eq!(cb.get_data_size(), 2);

    cb.push(b"XY", 2);
    assert_eq!(cb.get_data_size(), 4);

    let mut out2 = [0u8; 10];
    let n2 = cb.read(10, &mut out2);
    assert_eq!(n2, 4);
    assert_eq!(&out2[..4], b"DEXY");
}

#[test]
fn test_wrap_around_pop_drains_to_empty() {
    // After wrap, pop everything in one call.
    let mut cb = CB::new(5);
    cb.push(b"ABCDE", 5);
    let mut tmp = [0u8; 10];
    cb.pop(3, &mut tmp);
    cb.push(b"XY", 2);

    let mut out = [0u8; 10];
    let n = cb.pop(4, &mut out);
    assert_eq!(n, 4);
    assert_eq!(&out[..4], b"DEXY");
    assert_eq!(cb.get_data_size(), 0);

    // Subsequent pop returns 0.
    let mut out2 = [0u8; 10];
    let n2 = cb.pop(4, &mut out2);
    assert_eq!(n2, 0);
    assert_eq!(cb.get_data_size(), 0);
}

#[test]
fn test_wrap_around_partial_pop_then_read() {
    // Wrap, pop 2 across boundary, then read remainder.
    let mut cb = CB::new(5);
    cb.push(b"ABCDE", 5);
    let mut tmp = [0u8; 10];
    cb.pop(3, &mut tmp); // head=3, tail=4
    cb.push(b"XY", 2); // head=3, tail=1, contents DEXY

    let mut out = [0u8; 10];
    let n = cb.pop(2, &mut out); // pops "DE"
    assert_eq!(n, 2);
    assert_eq!(&out[..2], b"DE");
    assert_eq!(cb.get_data_size(), 2);

    let mut out2 = [0u8; 10];
    let n2 = cb.read(10, &mut out2);
    assert_eq!(n2, 2);
    assert_eq!(&out2[..2], b"XY");
}

#[test]
fn test_push_overlaps_head_resets_head() {
    // size 8: fill 8, pop 4, push 4 → buffer becomes "4567abcd" wrapping.
    let mut cb = CB::new(8);
    cb.push(b"01234567", 8);
    let mut tmp = [0u8; 16];
    cb.pop(4, &mut tmp); // pops "0123", left "4567"
    assert_eq!(cb.get_data_size(), 4);

    cb.push(b"abcd", 4);
    assert_eq!(cb.get_data_size(), 8);

    let mut out = [0u8; 16];
    let n = cb.read(16, &mut out);
    assert_eq!(n, 8);
    assert_eq!(&out[..8], b"4567abcd");
}

#[test]
fn test_push_overflows_head_pointer_advances() {
    // size 5: fill, pop 2, push 4 → 4 new bytes overwrite some old data.
    // After push 5 ABCDE: head=0, tail=4, ds=5 contents ABCDE
    // After pop 2: head=2, tail=4, ds=3 contents CDE
    // Push 4 "wxyz": writes overwriting; result should be the last 5 bytes
    // in FIFO order: from "CDE" + "wxyz" = "CDEwxyz" → last 5 = "Ewxyz".
    let mut cb = CB::new(5);
    cb.push(b"ABCDE", 5);
    let mut tmp = [0u8; 10];
    cb.pop(2, &mut tmp);
    assert_eq!(cb.get_data_size(), 3);

    cb.push(b"wxyz", 4);
    assert_eq!(cb.get_data_size(), 5);

    let mut out = [0u8; 10];
    let n = cb.read(10, &mut out);
    assert_eq!(n, 5);
    assert_eq!(&out[..5], b"Ewxyz");
}

// ---------------------------------------------------------------------------
// Reproduce the original C test sequence exactly
// ---------------------------------------------------------------------------

#[test]
fn test_full_c_sequence() {
    let mut cb = CB::new(8);
    let a = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut offset = 0usize;
    let mut b = [0u8; 128];

    // push 3 bytes "012"
    cb.push(&a[offset..], 3);
    offset += 3;
    assert_eq!(cb.get_data_size(), 3);

    // push 7 bytes "3456789" → buffer holds last 8: "23456789"
    cb.push(&a[offset..], 7);
    offset += 7;
    assert_eq!(cb.get_data_size(), 8);

    // pop 3 bytes → "234"
    b.fill(0);
    let out = cb.pop(3, &mut b);
    assert_eq!(out, 3);
    assert_eq!(&b[..3], b"234");
    assert_eq!(cb.get_data_size(), 5);

    // read 2 bytes → "56", does not consume
    b.fill(0);
    let out = cb.read(2, &mut b);
    assert_eq!(out, 2);
    assert_eq!(&b[..2], b"56");
    assert_eq!(cb.get_data_size(), 5);

    // push 10 bytes "abcdefghij" → keeps last 8: "cdefghij"
    cb.push(&a[offset..], 10);
    offset += 10;
    assert_eq!(cb.get_data_size(), 8);

    // pop 3 bytes → "cde"
    b.fill(0);
    let out = cb.pop(3, &mut b);
    assert_eq!(out, 3);
    assert_eq!(&b[..3], b"cde");
    assert_eq!(cb.get_data_size(), 5);

    // pop 30 bytes, but only 5 available → "fghij"
    b.fill(0);
    let out = cb.pop(30, &mut b);
    assert_eq!(out, 5);
    assert_eq!(&b[..5], b"fghij");
    assert_eq!(cb.get_data_size(), 0);

    // push 5 bytes "klmno"
    cb.push(&a[offset..], 5);
    offset += 5;
    assert_eq!(cb.get_data_size(), 5);

    // pop 2 bytes → "kl"
    b.fill(0);
    let out = cb.pop(2, &mut b);
    assert_eq!(out, 2);
    assert_eq!(&b[..2], b"kl");
    assert_eq!(cb.get_data_size(), 3);

    // push 10 bytes "pqrstuvwxy" → keeps last 8: "rstuvwxy"
    cb.push(&a[offset..], 10);
    offset += 10;
    assert_eq!(cb.get_data_size(), 8);

    // pop 6 bytes → "rstuvw"
    b.fill(0);
    let out = cb.pop(6, &mut b);
    assert_eq!(out, 6);
    assert_eq!(&b[..6], b"rstuvw");
    assert_eq!(cb.get_data_size(), 2);

    // push 4 bytes "zABC" → contents become "xyzABC"
    cb.push(&a[offset..], 4);
    let _ = offset; // keep symmetric
    assert_eq!(cb.get_data_size(), 6);

    let mut readout = [0u8; 16];
    let n = cb.read(16, &mut readout);
    assert_eq!(n, 6);
    assert_eq!(&readout[..6], b"xyzABC");
}

// ---------------------------------------------------------------------------
// inter_read direct invocation
// ---------------------------------------------------------------------------

#[test]
fn test_inter_read_no_consume() {
    let mut cb = CB::new(5);
    cb.push(b"ABCDE", 5);
    let mut out = [0u8; 10];
    let n = cb.inter_read(3, &mut out, false);
    assert_eq!(n, 3);
    assert_eq!(&out[..3], b"ABC");
    // No consumption.
    assert_eq!(cb.get_data_size(), 5);

    // Read again — same content.
    let mut out2 = [0u8; 10];
    let n2 = cb.inter_read(3, &mut out2, false);
    assert_eq!(n2, 3);
    assert_eq!(&out2[..3], b"ABC");
    assert_eq!(cb.get_data_size(), 5);
}

#[test]
fn test_inter_read_consume() {
    let mut cb = CB::new(5);
    cb.push(b"ABCDE", 5);
    let mut out = [0u8; 10];
    let n = cb.inter_read(3, &mut out, true);
    assert_eq!(n, 3);
    assert_eq!(&out[..3], b"ABC");
    assert_eq!(cb.get_data_size(), 2);

    // Reading again advances further.
    let mut out2 = [0u8; 10];
    let n2 = cb.inter_read(10, &mut out2, true);
    assert_eq!(n2, 2);
    assert_eq!(&out2[..2], b"DE");
    assert_eq!(cb.get_data_size(), 0);
}

#[test]
fn test_inter_read_zero_length() {
    let mut cb = CB::new(5);
    cb.push(b"ABC", 3);
    let mut out = [0u8; 10];
    let n = cb.inter_read(0, &mut out, true);
    assert_eq!(n, 0);
    assert_eq!(cb.get_data_size(), 3);
}

#[test]
fn test_inter_read_on_empty() {
    let mut cb = CB::new(5);
    let mut out = [0u8; 10];
    let n = cb.inter_read(5, &mut out, true);
    assert_eq!(n, 0);
    assert_eq!(cb.get_data_size(), 0);

    let n2 = cb.inter_read(5, &mut out, false);
    assert_eq!(n2, 0);
    assert_eq!(cb.get_data_size(), 0);
}

// ---------------------------------------------------------------------------
// Repeated cycles: drain and refill
// ---------------------------------------------------------------------------

#[test]
fn test_drain_and_refill_cycle() {
    let mut cb = CB::new(4);

    cb.push(b"abcd", 4);
    assert_eq!(cb.get_data_size(), 4);

    let mut out = [0u8; 10];
    let n = cb.pop(4, &mut out);
    assert_eq!(n, 4);
    assert_eq!(&out[..4], b"abcd");
    assert_eq!(cb.get_data_size(), 0);

    cb.push(b"WXYZ", 4);
    assert_eq!(cb.get_data_size(), 4);

    let mut out2 = [0u8; 10];
    let n2 = cb.pop(4, &mut out2);
    assert_eq!(n2, 4);
    assert_eq!(&out2[..4], b"WXYZ");
    assert_eq!(cb.get_data_size(), 0);
}

// ---------------------------------------------------------------------------
// print() — just ensure it does not panic. Output is to stdout only.
// ---------------------------------------------------------------------------

#[test]
fn test_print_does_not_panic_empty() {
    let cb = CB::new(5);
    cb.print(false);
    cb.print(true);
}

#[test]
fn test_print_does_not_panic_with_data() {
    let mut cb = CB::new(5);
    cb.push(b"ABC", 3);
    cb.print(false);
    cb.print(true);
}

#[test]
fn test_print_does_not_panic_when_wrapped() {
    let mut cb = CB::new(5);
    cb.push(b"ABCDE", 5);
    let mut tmp = [0u8; 10];
    cb.pop(3, &mut tmp);
    cb.push(b"XY", 2); // wrapped
    cb.print(false);
    cb.print(true);
}

// ---------------------------------------------------------------------------
// free() consumes self — confirm it compiles & runs.
// ---------------------------------------------------------------------------

#[test]
fn test_free_consumes() {
    let cb = CB::new(8);
    cb.free();
}

fn main() {}
