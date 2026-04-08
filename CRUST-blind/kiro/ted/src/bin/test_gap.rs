use ted::gap::GapBuffer;

#[test]
fn test_create_empty() {
    let gb = GapBuffer::create(20);
    assert_eq!(gb.str_len, 0);
    assert_eq!(gb.gap_len, 20);
    assert_eq!(gb.gap_loc, 0);
    assert_eq!(gb.get_string(), "");
}

#[test]
fn test_insert_single() {
    let mut gb = GapBuffer::create(20);
    assert_eq!(gb.insert_char('a'), 0);
    assert_eq!(gb.get_string(), "a");
    assert_eq!(gb.str_len, 1);
    assert_eq!(gb.gap_loc, 1);
    assert_eq!(gb.gap_len, 19);
}

#[test]
fn test_insert_multiple() {
    let mut gb = GapBuffer::create(20);
    for _ in 0..11 {
        assert_eq!(gb.insert_char('a'), 0);
    }
    assert_eq!(gb.get_string(), "aaaaaaaaaaa");
    assert_eq!(gb.str_len, 11);
}

#[test]
fn test_insert_with_resize() {
    let mut gb = GapBuffer::create(20);
    for _ in 0..21 {
        assert_eq!(gb.insert_char('a'), 0);
    }
    assert_eq!(gb.get_string(), "aaaaaaaaaaaaaaaaaaaaa");
    assert_eq!(gb.str_len, 21);
}

#[test]
fn test_backspace() {
    let mut gb = GapBuffer::create(20);
    for _ in 0..21 {
        gb.insert_char('a');
    }
    gb.backspace();
    assert_eq!(gb.get_string(), "aaaaaaaaaaaaaaaaaaaa");
    assert_eq!(gb.str_len, 20);
}

#[test]
fn test_backspace_empty() {
    let mut gb = GapBuffer::create(20);
    gb.backspace();
    assert_eq!(gb.str_len, 0);
    assert_eq!(gb.get_string(), "");
}

#[test]
fn test_move_gap_and_insert() {
    let mut gb = GapBuffer::create(20);
    // Insert 20 'a's
    for _ in 0..20 {
        gb.insert_char('a');
    }
    // Move gap to position 3, insert 'b's
    assert_eq!(gb.move_gap(3), 0);
    for _ in 0..3 {
        gb.insert_char('b');
    }
    assert_eq!(gb.get_string(), "aaabbbaaaaaaaaaaaaaaaaa");

    // Move gap to 0, insert 'c's
    assert_eq!(gb.move_gap(0), 0);
    for _ in 0..3 {
        gb.insert_char('c');
    }
    assert_eq!(gb.get_string(), "cccaaabbbaaaaaaaaaaaaaaaaa");

    // Move gap to end, insert 'd's
    assert_eq!(gb.move_gap(gb.str_len), 0);
    for _ in 0..3 {
        gb.insert_char('d');
    }
    assert_eq!(gb.get_string(), "cccaaabbbaaaaaaaaaaaaaaaaaddd");
}

#[test]
fn test_move_gap_clamps() {
    let mut gb = GapBuffer::create(20);
    for _ in 0..5 {
        gb.insert_char('a');
    }
    // Move beyond str_len should clamp
    assert_eq!(gb.move_gap(100), 0);
    assert_eq!(gb.gap_loc, 5);
}

#[test]
fn test_split() {
    let mut gb = GapBuffer::create(20);
    // Build "cccaaabbbaaaaaaaaaaaaaaaaaddd" like the C test
    for _ in 0..20 {
        gb.insert_char('a');
    }
    gb.move_gap(3);
    for _ in 0..3 {
        gb.insert_char('b');
    }
    gb.move_gap(0);
    for _ in 0..3 {
        gb.insert_char('c');
    }
    gb.move_gap(gb.str_len);
    for _ in 0..3 {
        gb.insert_char('d');
    }
    assert_eq!(gb.get_string(), "cccaaabbbaaaaaaaaaaaaaaaaaddd");

    // Move gap to position 6 and split
    gb.move_gap(6);
    let new_buf = gb.split();

    // In Rust, split() does NOT modify the original (unlike C).
    // The new buffer should have the second half.
    assert_eq!(new_buf.get_string(), "bbbaaaaaaaaaaaaaaaaaddd");

    // After split, manually update original like new_line does
    let capacity = gb.gap_len + gb.str_len;
    gb.str_len = gb.gap_loc;
    gb.gap_len = capacity - gb.str_len;
    assert_eq!(gb.get_string(), "cccaaa");

    // Verify we can still insert into the original after split
    assert_eq!(gb.insert_char('x'), 0);
    assert_eq!(gb.get_string(), "cccaaax");
}

#[test]
fn test_split_insert_into_new() {
    let mut gb = GapBuffer::create(10);
    for _ in 0..5 {
        gb.insert_char('a');
    }
    gb.move_gap(2);
    let mut new_buf = gb.split();
    // new_buf has "aaa", gap at 0
    assert_eq!(new_buf.get_string(), "aaa");
    assert_eq!(new_buf.gap_loc, 0);

    // Insert at beginning of new buffer
    assert_eq!(new_buf.insert_char('y'), 0);
    assert_eq!(new_buf.get_string(), "yaaa");
}

#[test]
fn test_create_from_string() {
    let gb = GapBuffer::create_from_string("ybbbaaaaaaaaaaaaaaaaaddd", 10);
    assert_eq!(gb.get_string(), "ybbbaaaaaaaaaaaaaaaaaddd");
    assert_eq!(gb.str_len, 24);
    assert_eq!(gb.gap_loc, 24);
    assert_eq!(gb.gap_len, 10);
}

#[test]
fn test_create_from_string_then_insert() {
    let mut gb = GapBuffer::create_from_string("ybbbaaaaaaaaaaaaaaaaaddd", 10);
    assert_eq!(gb.insert_char('p'), 0);
    assert_eq!(gb.get_string(), "ybbbaaaaaaaaaaaaaaaaadddp");
}

#[test]
fn test_create_from_string_empty() {
    let gb = GapBuffer::create_from_string("", 10);
    assert_eq!(gb.str_len, 0);
    assert_eq!(gb.gap_len, 10);
    assert_eq!(gb.get_string(), "");
}

#[test]
fn test_char_at() {
    let gb = GapBuffer::create_from_string("abcde", 10);
    assert_eq!(gb.char_at(0), 'a');
    assert_eq!(gb.char_at(4), 'e');
    assert_eq!(gb.char_at(5), '\0'); // out of bounds
}

#[test]
fn test_char_at_empty() {
    let gb = GapBuffer::create(10);
    assert_eq!(gb.char_at(0), '\0');
}

#[test]
fn test_char_at_with_gap_in_middle() {
    let mut gb = GapBuffer::create_from_string("abcde", 10);
    gb.move_gap(2); // gap is between 'b' and 'c'
    assert_eq!(gb.char_at(0), 'a');
    assert_eq!(gb.char_at(1), 'b');
    assert_eq!(gb.char_at(2), 'c');
    assert_eq!(gb.char_at(3), 'd');
    assert_eq!(gb.char_at(4), 'e');
}

#[test]
fn test_backspace_all() {
    let mut gb = GapBuffer::create(20);
    for _ in 0..7 {
        gb.insert_char('a');
    }
    for _ in 0..7 {
        gb.backspace();
    }
    assert_eq!(gb.get_string(), "");
    assert_eq!(gb.str_len, 0);
}

fn main() {}
