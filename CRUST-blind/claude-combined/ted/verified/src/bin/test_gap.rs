use ted::gap::{GapBuffer, MEM_ERROR};

#[test]
fn test_create() {
    let gb = GapBuffer::create(10);
    assert_eq!(gb.str_len, 0);
    assert_eq!(gb.gap_len, 10);
    assert_eq!(gb.gap_loc, 0);
    assert_eq!(gb.get_string(), "");
}

#[test]
fn test_insert_three_chars() {
    let mut gb = GapBuffer::create(10);
    assert_eq!(gb.insert_char('a'), 0);
    assert_eq!(gb.insert_char('b'), 0);
    assert_eq!(gb.insert_char('c'), 0);
    assert_eq!(gb.str_len, 3);
    assert_eq!(gb.gap_len, 7);
    assert_eq!(gb.gap_loc, 3);
    assert_eq!(gb.get_string(), "abc");
}

#[test]
fn test_backspace() {
    let mut gb = GapBuffer::create(10);
    gb.insert_char('a');
    gb.insert_char('b');
    gb.insert_char('c');
    gb.backspace();
    assert_eq!(gb.str_len, 2);
    assert_eq!(gb.gap_len, 8);
    assert_eq!(gb.gap_loc, 2);
    assert_eq!(gb.get_string(), "ab");
}

#[test]
fn test_backspace_at_zero_no_change() {
    let mut gb = GapBuffer::create(10);
    gb.backspace();
    assert_eq!(gb.str_len, 0);
    assert_eq!(gb.gap_len, 10);
    assert_eq!(gb.gap_loc, 0);
    assert_eq!(gb.get_string(), "");
}

#[test]
fn test_move_gap_to_zero() {
    let mut gb = GapBuffer::create(10);
    gb.insert_char('a');
    gb.insert_char('b');
    gb.backspace(); // resets to "ab" with str_len=2
    // ensure we're at "ab" still:
    assert_eq!(gb.get_string(), "a");

    let mut gb2 = GapBuffer::create(10);
    gb2.insert_char('a');
    gb2.insert_char('b');
    assert_eq!(gb2.move_gap(0), 0);
    assert_eq!(gb2.str_len, 2);
    assert_eq!(gb2.gap_len, 8);
    assert_eq!(gb2.gap_loc, 0);
    assert_eq!(gb2.get_string(), "ab");
}

#[test]
fn test_insert_after_move_gap() {
    let mut gb = GapBuffer::create(10);
    gb.insert_char('a');
    gb.insert_char('b');
    gb.move_gap(0);
    gb.insert_char('X');
    assert_eq!(gb.str_len, 3);
    assert_eq!(gb.gap_len, 7);
    assert_eq!(gb.gap_loc, 1);
    assert_eq!(gb.get_string(), "Xab");
}

#[test]
fn test_resize_growth() {
    // Capacity 2 forces a resize after inserting first chars (gap closes when gap_len <= 1).
    let mut gb = GapBuffer::create(2);
    gb.insert_char('a');
    gb.insert_char('b');
    gb.insert_char('c');
    assert_eq!(gb.str_len, 3);
    // capacity gets doubled when gap_len <= 1.
    // After 'a': gap_len=1, gap_loc=1, str_len=1
    // Insert 'b': gap_len <= 1, so resize from cap 2 -> 4. After resize gap_len=3.
    //   then insert: str_len=2, gap_len=2, gap_loc=2
    // Insert 'c': gap_len > 1, just insert: str_len=3, gap_len=1, gap_loc=3
    assert_eq!(gb.gap_len, 1);
    assert_eq!(gb.gap_loc, 3);
    assert_eq!(gb.get_string(), "abc");
}

#[test]
fn test_create_from_string() {
    let gb = GapBuffer::create_from_string("hello", 5);
    assert_eq!(gb.str_len, 5);
    assert_eq!(gb.gap_len, 5);
    assert_eq!(gb.gap_loc, 5);
    assert_eq!(gb.get_string(), "hello");
}

#[test]
fn test_create_from_empty_string() {
    let gb = GapBuffer::create_from_string("", 4);
    assert_eq!(gb.str_len, 0);
    assert_eq!(gb.gap_len, 4);
    assert_eq!(gb.gap_loc, 0);
    assert_eq!(gb.get_string(), "");
}

#[test]
fn test_char_at_in_bounds() {
    let gb = GapBuffer::create_from_string("hello", 5);
    assert_eq!(gb.char_at(0), 'h');
    assert_eq!(gb.char_at(1), 'e');
    assert_eq!(gb.char_at(2), 'l');
    assert_eq!(gb.char_at(3), 'l');
    assert_eq!(gb.char_at(4), 'o');
}

#[test]
fn test_char_at_out_of_bounds() {
    let gb = GapBuffer::create_from_string("hello", 5);
    assert_eq!(gb.char_at(5), '\0');
    assert_eq!(gb.char_at(6), '\0');
}

#[test]
fn test_char_at_after_gap_move() {
    let mut gb = GapBuffer::create_from_string("hello", 5);
    gb.move_gap(2);
    gb.insert_char('X');
    // string becomes "heXllo"
    assert_eq!(gb.str_len, 6);
    assert_eq!(gb.gap_len, 4);
    assert_eq!(gb.gap_loc, 3);
    assert_eq!(gb.get_string(), "heXllo");
    assert_eq!(gb.char_at(0), 'h');
    assert_eq!(gb.char_at(1), 'e');
    assert_eq!(gb.char_at(2), 'X');
    assert_eq!(gb.char_at(3), 'l');
    assert_eq!(gb.char_at(4), 'l');
    assert_eq!(gb.char_at(5), 'o');
}

#[test]
fn test_split() {
    // Set up the C scenario: create from "foobar" (6 chars), gap_len 4, then move_gap to 3.
    // Then split.
    let mut gb = GapBuffer::create_from_string("foobar", 4);
    gb.move_gap(3);
    let second = gb.split();

    // After split, the new buffer has the "second half" of the string.
    assert_eq!(second.str_len, 3);
    assert_eq!(second.gap_len, 7);
    assert_eq!(second.gap_loc, 0);
    assert_eq!(second.get_string(), "bar");
}

#[test]
fn test_move_gap_too_far() {
    let mut gb = GapBuffer::create_from_string("abcdef", 4);
    let err = gb.move_gap(100);
    assert_eq!(err, 0);
    assert_eq!(gb.str_len, 6);
    assert_eq!(gb.gap_len, 4);
    assert_eq!(gb.gap_loc, 6);
    assert_eq!(gb.get_string(), "abcdef");
}

#[test]
fn test_mem_error_constant() {
    assert_eq!(MEM_ERROR, 128);
}

fn main() {}
