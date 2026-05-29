use ted::gap::{GapBuffer, MEM_ERROR};

#[test]
fn test_create() {
    let gb = GapBuffer::create(20);
    assert_eq!(gb.str_len, 0);
    assert_eq!(gb.gap_loc, 0);
    assert_eq!(gb.gap_len, 20);
    assert_eq!(gb.buffer.len(), 20);
    assert_eq!(gb.get_string(), "");
}

#[test]
fn test_insert_char_single() {
    let mut gb = GapBuffer::create(20);
    let err = gb.insert_char('a');
    assert_eq!(err, 0);
    assert_eq!(gb.str_len, 1);
    assert_eq!(gb.gap_loc, 1);
    assert_eq!(gb.gap_len, 19);
    assert_eq!(gb.get_string(), "a");
}

#[test]
fn test_insert_char_multiple() {
    let mut gb = GapBuffer::create(20);
    for _ in 0..11 {
        let err = gb.insert_char('a');
        assert_eq!(err, 0);
    }
    assert_eq!(gb.str_len, 11);
    assert_eq!(gb.gap_loc, 11);
    assert_eq!(gb.gap_len, 9);
    assert_eq!(gb.get_string(), "aaaaaaaaaaa");
}

#[test]
fn test_insert_char_resize() {
    let mut gb = GapBuffer::create(20);
    for _ in 0..21 {
        let err = gb.insert_char('a');
        assert_eq!(err, 0);
    }
    // After 21 inserts (resize once at insert #20):
    // capacity grew from 20 -> 40
    assert_eq!(gb.str_len, 21);
    assert_eq!(gb.gap_loc, 21);
    assert_eq!(gb.gap_len, 19);
    assert_eq!(gb.get_string(), "aaaaaaaaaaaaaaaaaaaaa");
}

#[test]
fn test_backspace_basic() {
    let mut gb = GapBuffer::create(20);
    for _ in 0..5 {
        gb.insert_char('a');
    }
    gb.backspace();
    assert_eq!(gb.str_len, 4);
    assert_eq!(gb.gap_loc, 4);
    assert_eq!(gb.gap_len, 16);
    assert_eq!(gb.get_string(), "aaaa");
}

#[test]
fn test_backspace_underflow_protected() {
    let mut gb = GapBuffer::create(20);
    for _ in 0..5 {
        gb.insert_char('a');
    }
    for _ in 0..7 {
        // 7 backspaces — last 2 are no-ops because gap_loc is 0
        gb.backspace();
    }
    assert_eq!(gb.str_len, 0);
    assert_eq!(gb.gap_loc, 0);
    assert_eq!(gb.gap_len, 20);
    assert_eq!(gb.get_string(), "");
}

#[test]
fn test_move_gap_basic() {
    let mut gb = GapBuffer::create(20);
    for _ in 0..5 {
        gb.insert_char('a');
    }
    let err = gb.move_gap(2);
    assert_eq!(err, 0);
    assert_eq!(gb.str_len, 5);
    assert_eq!(gb.gap_loc, 2);
    assert_eq!(gb.gap_len, 15);
    assert_eq!(gb.get_string(), "aaaaa");
}

#[test]
fn test_move_gap_forward_then_backward() {
    let mut gb = GapBuffer::create(20);
    for _ in 0..5 {
        gb.insert_char('a');
    }
    // start at gap_loc=5; move to 2
    let err = gb.move_gap(2);
    assert_eq!(err, 0);
    assert_eq!(gb.gap_loc, 2);
    // move back to 4 (forward from gap)
    let err = gb.move_gap(4);
    assert_eq!(err, 0);
    assert_eq!(gb.str_len, 5);
    assert_eq!(gb.gap_loc, 4);
    assert_eq!(gb.gap_len, 15);
    assert_eq!(gb.get_string(), "aaaaa");
}

#[test]
fn test_move_gap_clamp_above() {
    let mut gb = GapBuffer::create(20);
    for _ in 0..5 {
        gb.insert_char('a');
    }
    // location > str_len -> clamp to str_len (5)
    let err = gb.move_gap(100);
    assert_eq!(err, 0);
    assert_eq!(gb.gap_loc, 5);
    assert_eq!(gb.str_len, 5);
    assert_eq!(gb.gap_len, 15);
}

#[test]
fn test_move_gap_to_zero() {
    let mut gb = GapBuffer::create(20);
    for _ in 0..5 {
        gb.insert_char('a');
    }
    let err = gb.move_gap(0);
    assert_eq!(err, 0);
    assert_eq!(gb.gap_loc, 0);
    assert_eq!(gb.str_len, 5);
    assert_eq!(gb.gap_len, 15);
    assert_eq!(gb.get_string(), "aaaaa");
}

#[test]
fn test_get_string_empty() {
    let gb = GapBuffer::create(20);
    assert_eq!(gb.get_string(), "");
}

#[test]
fn test_split_middle() {
    // build "hello world" (11 chars), gap at 5
    let mut gb = GapBuffer::create(30);
    for c in "hello world".chars() {
        gb.insert_char(c);
    }
    gb.move_gap(5);
    assert_eq!(gb.gap_loc, 5);
    assert_eq!(gb.str_len, 11);

    let new_gb = gb.split();
    // Original after split: only first half remains. C version mutates original.
    // Check second half:
    assert_eq!(new_gb.str_len, 6);
    assert_eq!(new_gb.gap_loc, 0);
    assert_eq!(new_gb.gap_len, 30 - 6);
    assert_eq!(new_gb.get_string(), " world");
}

#[test]
fn test_create_from_string() {
    let gb = GapBuffer::create_from_string("hello", 10);
    assert_eq!(gb.str_len, 5);
    assert_eq!(gb.gap_loc, 5);
    assert_eq!(gb.gap_len, 10);
    assert_eq!(gb.get_string(), "hello");
}

#[test]
fn test_create_from_empty_string() {
    let gb = GapBuffer::create_from_string("", 10);
    assert_eq!(gb.str_len, 0);
    assert_eq!(gb.gap_loc, 0);
    assert_eq!(gb.gap_len, 10);
    assert_eq!(gb.get_string(), "");
}

#[test]
fn test_create_from_string_then_insert() {
    let mut gb = GapBuffer::create_from_string("hello", 10);
    let err = gb.insert_char('!');
    assert_eq!(err, 0);
    assert_eq!(gb.str_len, 6);
    assert_eq!(gb.gap_loc, 6);
    assert_eq!(gb.gap_len, 9);
    assert_eq!(gb.get_string(), "hello!");
}

#[test]
fn test_char_at_basic() {
    let gb = GapBuffer::create_from_string("hello", 10);
    assert_eq!(gb.char_at(0), 'h');
    assert_eq!(gb.char_at(1), 'e');
    assert_eq!(gb.char_at(2), 'l');
    assert_eq!(gb.char_at(3), 'l');
    assert_eq!(gb.char_at(4), 'o');
    // out of range should return null byte
    assert_eq!(gb.char_at(5), '\0');
    assert_eq!(gb.char_at(100), '\0');
}

#[test]
fn test_char_at_with_gap_in_middle() {
    let mut gb = GapBuffer::create_from_string("hello", 10);
    gb.move_gap(2);
    // The string is still "hello" logically
    assert_eq!(gb.char_at(0), 'h');
    assert_eq!(gb.char_at(1), 'e');
    assert_eq!(gb.char_at(2), 'l');
    assert_eq!(gb.char_at(3), 'l');
    assert_eq!(gb.char_at(4), 'o');
    assert_eq!(gb.char_at(5), '\0');
}

#[test]
fn test_char_at_empty() {
    let gb = GapBuffer::create(10);
    assert_eq!(gb.char_at(0), '\0');
    assert_eq!(gb.char_at(5), '\0');
}

#[test]
fn test_official_runtests_gapbuffer_sequence() {
    // Mirrors the C runtests.c TestGapBuffer suite to a strong degree
    let mut buffer = GapBuffer::create(20);

    // Test 1, empty string
    assert_eq!(buffer.get_string(), "");

    // Test 2 insert char 'a'
    let err = buffer.insert_char('a');
    assert_eq!(err, 0);
    assert_eq!(buffer.get_string(), "a");

    // Test 2.1 insert 10 more 'a' -> "aaaaaaaaaaa"
    for _ in 0..10 {
        let err = buffer.insert_char('a');
        assert_eq!(err, 0);
    }
    assert_eq!(buffer.get_string(), "aaaaaaaaaaa");

    // Test 2.2 insert 10 more 'a' (resize) -> 21 'a'
    for _ in 0..10 {
        let err = buffer.insert_char('a');
        assert_eq!(err, 0);
    }
    assert_eq!(buffer.get_string(), "aaaaaaaaaaaaaaaaaaaaa");

    // Test 3 backspace -> 20 'a'
    buffer.backspace();
    assert_eq!(buffer.get_string(), "aaaaaaaaaaaaaaaaaaaa");

    // Test 4 move gap to 3 then insert 3 'b' -> "aaabbbaaaaaaaaaaaaaaaaa"
    let err = buffer.move_gap(3);
    assert_eq!(err, 0);
    for _ in 0..3 {
        let err = buffer.insert_char('b');
        assert_eq!(err, 0);
    }
    assert_eq!(buffer.get_string(), "aaabbbaaaaaaaaaaaaaaaaa");

    // Test 4.1 move gap to 0, insert 3 'c'
    let err = buffer.move_gap(0);
    assert_eq!(err, 0);
    for _ in 0..3 {
        let err = buffer.insert_char('c');
        assert_eq!(err, 0);
    }
    assert_eq!(buffer.get_string(), "cccaaabbbaaaaaaaaaaaaaaaaa");

    // Test 4.2 move gap to str_len, insert 3 'd'
    let err = buffer.move_gap(buffer.str_len);
    assert_eq!(err, 0);
    for _ in 0..3 {
        let err = buffer.insert_char('d');
        assert_eq!(err, 0);
    }
    assert_eq!(buffer.get_string(), "cccaaabbbaaaaaaaaaaaaaaaaaddd");

    // Test 5 move gap to 6 then split
    let err = buffer.move_gap(6);
    assert_eq!(err, 0);
    let buffer2 = buffer.split();
    // The Rust split() is non-mutating on `&self`, so original is unchanged.
    // C's version mutates the original but the new buffer's contents must match.
    assert_eq!(buffer2.get_string(), "bbbaaaaaaaaaaaaaaaaaddd");

    let _ = MEM_ERROR;
}

#[test]
fn test_get_string_with_gap_in_middle() {
    let mut gb = GapBuffer::create_from_string("abcdef", 10);
    gb.move_gap(3);
    // string before/after gap should still concatenate
    assert_eq!(gb.get_string(), "abcdef");
    assert_eq!(gb.str_len, 6);
    assert_eq!(gb.gap_loc, 3);
}

fn main() {}
