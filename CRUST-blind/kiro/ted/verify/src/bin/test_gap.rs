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
fn test_insert_single_char() {
    let mut gb = GapBuffer::create(20);
    let err = gb.insert_char('a');
    assert_eq!(err, 0);
    assert_eq!(gb.str_len, 1);
    assert_eq!(gb.gap_loc, 1);
    assert_eq!(gb.gap_len, 19);
    assert_eq!(gb.get_string(), "a");
}

#[test]
fn test_insert_11_chars() {
    let mut gb = GapBuffer::create(20);
    for _ in 0..11 {
        assert_eq!(gb.insert_char('a'), 0);
    }
    assert_eq!(gb.str_len, 11);
    assert_eq!(gb.get_string(), "aaaaaaaaaaa");
}

#[test]
fn test_insert_with_resize() {
    let mut gb = GapBuffer::create(20);
    for _ in 0..21 {
        assert_eq!(gb.insert_char('a'), 0);
    }
    assert_eq!(gb.str_len, 21);
    assert_eq!(gb.get_string(), "aaaaaaaaaaaaaaaaaaaaa");
}

#[test]
fn test_backspace() {
    let mut gb = GapBuffer::create(20);
    for _ in 0..21 {
        gb.insert_char('a');
    }
    gb.backspace();
    assert_eq!(gb.str_len, 20);
    assert_eq!(gb.get_string(), "aaaaaaaaaaaaaaaaaaaa");
}

#[test]
fn test_backspace_empty() {
    let mut gb = GapBuffer::create(20);
    gb.backspace();
    assert_eq!(gb.str_len, 0);
    assert_eq!(gb.gap_loc, 0);
    assert_eq!(gb.get_string(), "");
}

#[test]
fn test_move_gap_and_insert() {
    let mut gb = GapBuffer::create(20);
    for _ in 0..20 {
        gb.insert_char('a');
    }
    assert_eq!(gb.move_gap(3), 0);
    for _ in 0..3 {
        assert_eq!(gb.insert_char('b'), 0);
    }
    assert_eq!(gb.get_string(), "aaabbbaaaaaaaaaaaaaaaaa");
}

#[test]
fn test_move_gap_to_start() {
    let mut gb = GapBuffer::create(20);
    for _ in 0..20 {
        gb.insert_char('a');
    }
    gb.move_gap(3);
    for _ in 0..3 {
        gb.insert_char('b');
    }
    assert_eq!(gb.move_gap(0), 0);
    for _ in 0..3 {
        gb.insert_char('c');
    }
    assert_eq!(gb.get_string(), "cccaaabbbaaaaaaaaaaaaaaaaa");
}

#[test]
fn test_move_gap_to_end() {
    let mut gb = GapBuffer::create(20);
    for _ in 0..20 {
        gb.insert_char('a');
    }
    gb.move_gap(3);
    for _ in 0..3 { gb.insert_char('b'); }
    gb.move_gap(0);
    for _ in 0..3 { gb.insert_char('c'); }
    assert_eq!(gb.move_gap(gb.str_len), 0);
    for _ in 0..3 { gb.insert_char('d'); }
    assert_eq!(gb.get_string(), "cccaaabbbaaaaaaaaaaaaaaaaaddd");
}

#[test]
fn test_split() {
    let mut gb = GapBuffer::create(20);
    for _ in 0..20 { gb.insert_char('a'); }
    gb.move_gap(3);
    for _ in 0..3 { gb.insert_char('b'); }
    gb.move_gap(0);
    for _ in 0..3 { gb.insert_char('c'); }
    gb.move_gap(gb.str_len);
    for _ in 0..3 { gb.insert_char('d'); }
    // "cccaaabbbaaaaaaaaaaaaaaaaaddd" len=28
    gb.move_gap(6);
    let gb2 = gb.split();
    assert_eq!(gb.get_string(), "cccaaa");
    assert_eq!(gb2.get_string(), "bbbaaaaaaaaaaaaaaaaaddd");
}

#[test]
fn test_split_insert_first_half() {
    let mut gb = GapBuffer::create(20);
    for _ in 0..20 { gb.insert_char('a'); }
    gb.move_gap(3);
    for _ in 0..3 { gb.insert_char('b'); }
    gb.move_gap(0);
    for _ in 0..3 { gb.insert_char('c'); }
    gb.move_gap(gb.str_len);
    for _ in 0..3 { gb.insert_char('d'); }
    gb.move_gap(6);
    let _gb2 = gb.split();
    gb.insert_char('x');
    assert_eq!(gb.get_string(), "cccaaax");
}

#[test]
fn test_split_insert_second_half() {
    let mut gb = GapBuffer::create(20);
    for _ in 0..20 { gb.insert_char('a'); }
    gb.move_gap(3);
    for _ in 0..3 { gb.insert_char('b'); }
    gb.move_gap(0);
    for _ in 0..3 { gb.insert_char('c'); }
    gb.move_gap(gb.str_len);
    for _ in 0..3 { gb.insert_char('d'); }
    gb.move_gap(6);
    let mut gb2 = gb.split();
    gb2.insert_char('y');
    assert_eq!(gb2.get_string(), "ybbbaaaaaaaaaaaaaaaaaddd");
}

#[test]
fn test_create_from_string() {
    let gb = GapBuffer::create_from_string("ybbbaaaaaaaaaaaaaaaaaddd", 10);
    assert_eq!(gb.str_len, 24);
    assert_eq!(gb.gap_loc, 24);
    assert_eq!(gb.gap_len, 10);
    assert_eq!(gb.get_string(), "ybbbaaaaaaaaaaaaaaaaaddd");
}

#[test]
fn test_create_from_string_insert() {
    let mut gb = GapBuffer::create_from_string("ybbbaaaaaaaaaaaaaaaaaddd", 10);
    assert_eq!(gb.insert_char('p'), 0);
    assert_eq!(gb.get_string(), "ybbbaaaaaaaaaaaaaaaaadddp");
}

#[test]
fn test_create_from_empty_string() {
    let gb = GapBuffer::create_from_string("", 10);
    assert_eq!(gb.str_len, 0);
    assert_eq!(gb.gap_len, 10);
    assert_eq!(gb.get_string(), "");
}

#[test]
fn test_backspace_all() {
    let mut gb = GapBuffer::create(20);
    for _ in 0..7 { gb.insert_char('a'); }
    for _ in 0..7 { gb.backspace(); }
    assert_eq!(gb.str_len, 0);
    assert_eq!(gb.get_string(), "");
}

#[test]
fn test_char_at_empty() {
    let gb = GapBuffer::create(10);
    assert_eq!(gb.char_at(0), '\0');
}

#[test]
fn test_char_at() {
    let mut gb = GapBuffer::create(10);
    gb.insert_char('H');
    gb.insert_char('e');
    gb.insert_char('l');
    assert_eq!(gb.char_at(0), 'H');
    assert_eq!(gb.char_at(1), 'e');
    assert_eq!(gb.char_at(2), 'l');
    assert_eq!(gb.char_at(3), '\0');
}

#[test]
fn test_char_at_after_move() {
    let mut gb = GapBuffer::create(10);
    gb.insert_char('H');
    gb.insert_char('e');
    gb.insert_char('l');
    gb.move_gap(1);
    assert_eq!(gb.char_at(0), 'H');
    assert_eq!(gb.char_at(1), 'e');
    assert_eq!(gb.char_at(2), 'l');
}

#[test]
fn test_move_gap_beyond_str_len() {
    let mut gb = GapBuffer::create(10);
    gb.insert_char('a');
    gb.insert_char('b');
    gb.move_gap(100);
    assert_eq!(gb.gap_loc, 2);
    assert_eq!(gb.get_string(), "ab");
}

fn main() {}
