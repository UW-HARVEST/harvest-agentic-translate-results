use c_string::string_t::*;

// --- new_string ---
#[test]
fn test_new_string() {
    for &sz in &[0usize, 1, 2, 3, 12, 100] {
        let s = new_string(sz);
        assert_eq!(s.size, sz);
        assert_eq!(s.bytes.len(), sz);
    }
}

// --- new_string_from_bytes ---
#[test]
fn test_new_string_from_bytes() {
    for &b in &["", "test", "some another test"] {
        let s = new_string_from_bytes(b);
        assert_eq!(s.size, b.len());
        assert_eq!(string_bytes(&s), b);
    }
}

// --- string_len ---
#[test]
fn test_string_len() {
    let cases = [("", 0), ("test", 4), ("some another test", 17)];
    for &(b, expected) in &cases {
        let s = new_string_from_bytes(b);
        assert_eq!(string_len(&s), expected);
    }
}

// --- string_bytes ---
#[test]
fn test_string_bytes() {
    for &b in &["", "test", "some another test"] {
        let s = new_string_from_bytes(b);
        assert_eq!(string_bytes(&s), b);
    }
}

// --- string_eq ---
#[test]
fn test_string_eq_equal() {
    for &b in &["", "\n\n", "test", "some another test"] {
        let a = new_string_from_bytes(b);
        let c = new_string_from_bytes(b);
        assert_eq!(string_eq(&a, &c), true);
    }
}

#[test]
fn test_string_eq_unequal() {
    let a = new_string_from_bytes("hello");
    let b = new_string_from_bytes("world");
    assert_eq!(string_eq(&a, &b), false);

    let c = new_string_from_bytes("hell");
    assert_eq!(string_eq(&a, &c), false);
}

// --- string_copy ---
#[test]
fn test_string_copy() {
    for &b in &["", "test", "some another test"] {
        let s = new_string_from_bytes(b);
        let cp = string_copy(&s);
        assert_eq!(string_eq(&s, &cp), true);
        assert_eq!(cp.size, s.size);
        assert_eq!(string_bytes(&cp), b);
    }
}

// --- string_concat ---
#[test]
fn test_string_concat() {
    let cases = [("", "", ""), ("first", "second", "firstsecond"), ("some another", " test", "some another test")];
    for &(a, b, expected) in &cases {
        let sa = new_string_from_bytes(a);
        let sb = new_string_from_bytes(b);
        let res = string_concat(&sa, &sb);
        let exp = new_string_from_bytes(expected);
        assert_eq!(string_eq(&res, &exp), true);
        assert_eq!(res.size, expected.len());
    }
}

// --- string_substr ---
#[test]
fn test_string_substr() {
    let cases = [("", 0, 0, ""), ("vfv\n\n", 1, 2, "fv"), ("test string", 5, 3, "str"), (" some another test  ", 0, 5, " some")];
    for &(input, pos, len, expected) in &cases {
        let s = new_string_from_bytes(input);
        let sub = string_substr(&s, pos, len);
        assert_eq!(sub.size, expected.len());
        assert_eq!(string_bytes(&sub), expected);
    }
}

// --- string_startswith ---
#[test]
fn test_string_startswith() {
    let cases = [("", "", true), ("vfv\n\n", "vfv", true), ("test string", "test string", true),
                 (" some another test  ", " some", true), ("1234", "2", false), ("hi", "hello", false)];
    for &(input, prefix, expected) in &cases {
        let s = new_string_from_bytes(input);
        assert_eq!(string_startswith(&s, prefix), expected);
    }
}

// --- string_endswith ---
#[test]
fn test_string_endswith() {
    let cases = [("", "", true), ("vfv\n\n", "fv\n\n", true), ("test string", "test string", true),
                 (" some another test  ", "test  ", true), ("1234", "2", false), ("hi", "hello", false)];
    for &(input, suffix, expected) in &cases {
        let s = new_string_from_bytes(input);
        assert_eq!(string_endswith(&s, suffix), expected);
    }
}

// --- string_find ---
#[test]
fn test_string_find() {
    // C returns int: 0 for empty-in-empty, -1 for not found, position otherwise
    // Rust returns Option<usize>: Some(0), None, Some(pos)
    let s1 = new_string_from_bytes("");
    assert_eq!(string_find(&s1, ""), Some(0));
    assert_eq!(string_find(&s1, "x"), None);

    let s2 = new_string_from_bytes("hello");
    assert_eq!(string_find(&s2, ""), Some(0));
    assert_eq!(string_find(&s2, "llo"), Some(2));
    assert_eq!(string_find(&s2, "xyz"), None);
    assert_eq!(string_find(&s2, "hello"), Some(0));
    assert_eq!(string_find(&s2, "helloo"), None);

    let s3 = new_string_from_bytes("vfv\n\n");
    assert_eq!(string_find(&s3, "\n"), Some(3));

    let s4 = new_string_from_bytes("test string");
    assert_eq!(string_find(&s4, "no"), None);
    assert_eq!(string_find(&s4, ""), Some(0));

    let s5 = new_string_from_bytes(" some another test  ");
    assert_eq!(string_find(&s5, "another"), Some(6));
}

// --- string_strip ---
#[test]
fn test_string_strip() {
    // Normal cases
    let cases = [("", ""), ("vfv\n\n", "vfv"), ("  test\t", "test"), (" some another test  ", "some another test")];
    for &(input, expected) in &cases {
        let s = new_string_from_bytes(input);
        let stripped = string_strip(&s);
        assert_eq!(string_bytes(&stripped), expected);
        assert_eq!(stripped.size, expected.len());
    }
}

#[test]
fn test_string_strip_all_whitespace() {
    // C behavior: all-whitespace returns copy of original (start_pos >= end_pos)
    let s = new_string_from_bytes("   ");
    let stripped = string_strip(&s);
    assert_eq!(string_bytes(&stripped), "   ");
    assert_eq!(stripped.size, 3);

    let s2 = new_string_from_bytes("\t\n\r ");
    let stripped2 = string_strip(&s2);
    assert_eq!(string_bytes(&stripped2), "\t\n\r ");
    assert_eq!(stripped2.size, 4);
}

// --- string_split ---
#[test]
fn test_string_split_empty() {
    let s = new_string_from_bytes("");
    let mut arr_size = 0;
    let arr = string_split(&s, &mut arr_size);
    assert_eq!(arr_size, 1);
    assert_eq!(arr.len(), 1);
    assert_eq!(string_bytes(&arr[0]), "");
    assert_eq!(arr[0].size, 0);
}

#[test]
fn test_string_split_single_word() {
    let s = new_string_from_bytes("hello");
    let mut arr_size = 0;
    let arr = string_split(&s, &mut arr_size);
    assert_eq!(arr_size, 1);
    assert_eq!(string_bytes(&arr[0]), "hello");
    assert_eq!(arr[0].size, 5);
}

#[test]
fn test_string_split_leading_space() {
    // C: " some string 124!" -> ["", "some", "string", "124!"], arr_size=4
    let s = new_string_from_bytes(" some string 124!");
    let mut arr_size = 0;
    let arr = string_split(&s, &mut arr_size);
    assert_eq!(arr_size, 4);
    let expected = ["", "some", "string", "124!"];
    for (i, exp) in expected.iter().enumerate() {
        assert_eq!(string_bytes(&arr[i]), *exp);
        assert_eq!(arr[i].size, exp.len());
    }
}

#[test]
fn test_string_split_no_leading_space() {
    let s = new_string_from_bytes("Some github account: vnkrtv");
    let mut arr_size = 0;
    let arr = string_split(&s, &mut arr_size);
    assert_eq!(arr_size, 4);
    let expected = ["Some", "github", "account:", "vnkrtv"];
    for (i, exp) in expected.iter().enumerate() {
        assert_eq!(string_bytes(&arr[i]), *exp);
        assert_eq!(arr[i].size, exp.len());
    }
}

// --- string_split_by ---
#[test]
fn test_string_split_by_small_input() {
    // size <= split_len returns copy
    let s = new_string_from_bytes("W");
    let mut arr_size = 0;
    let arr = string_split_by(&s, &mut arr_size, "W");
    assert_eq!(arr_size, 1);
    assert_eq!(string_bytes(&arr[0]), "W");
    assert_eq!(arr[0].size, 1);

    let s2 = new_string_from_bytes("");
    let mut arr_size2 = 0;
    let arr2 = string_split_by(&s2, &mut arr_size2, "W");
    assert_eq!(arr_size2, 1);
    assert_eq!(string_bytes(&arr2[0]), "");
    assert_eq!(arr2[0].size, 0);
}

#[test]
fn test_string_split_by_single_char() {
    let s = new_string_from_bytes("WsomeWstringW124!");
    let mut arr_size = 0;
    let arr = string_split_by(&s, &mut arr_size, "W");
    assert_eq!(arr_size, 4);
    let expected = ["", "some", "string", "124!"];
    for (i, exp) in expected.iter().enumerate() {
        assert_eq!(string_bytes(&arr[i]), *exp);
        assert_eq!(arr[i].size, exp.len());
    }
}

#[test]
fn test_string_split_by_no_leading_delim() {
    let s = new_string_from_bytes("SomeWgithubWaccount:Wvnkrtv");
    let mut arr_size = 0;
    let arr = string_split_by(&s, &mut arr_size, "W");
    assert_eq!(arr_size, 4);
    let expected = ["Some", "github", "account:", "vnkrtv"];
    for (i, exp) in expected.iter().enumerate() {
        assert_eq!(string_bytes(&arr[i]), *exp);
        assert_eq!(arr[i].size, exp.len());
    }
}

#[test]
fn test_string_split_by_multi_char_delim() {
    // C: "helloABworldABfoo" by "AB" -> ["hello", "world", "fo"], arr_size=3
    // Note: last element is "fo" not "foo" — this is the C behavior
    let s = new_string_from_bytes("helloABworldABfoo");
    let mut arr_size = 0;
    let arr = string_split_by(&s, &mut arr_size, "AB");
    assert_eq!(arr_size, 3);
    let expected = ["hello", "world", "fo"];
    for (i, exp) in expected.iter().enumerate() {
        assert_eq!(string_bytes(&arr[i]), *exp);
        assert_eq!(arr[i].size, exp.len());
    }
}

// --- string_join_arr ---
#[test]
fn test_string_join_arr_single() {
    let arr = vec![new_string_from_bytes("")];
    let res = string_join_arr(&arr, 1, " ");
    assert_eq!(string_bytes(&res), "");
    assert_eq!(res.size, 0);

    let arr2 = vec![new_string_from_bytes("1")];
    let res2 = string_join_arr(&arr2, 1, " ");
    assert_eq!(string_bytes(&res2), "1");
    assert_eq!(res2.size, 1);
}

#[test]
fn test_string_join_arr_multiple() {
    let arr = vec![new_string_from_bytes("some"), new_string_from_bytes("string")];
    let res = string_join_arr(&arr, 2, " ");
    assert_eq!(string_bytes(&res), "some string");
    assert_eq!(res.size, 11);
}

#[test]
fn test_string_join_arr_newline_sep() {
    let arr = vec![new_string_from_bytes(""), new_string_from_bytes("some"), new_string_from_bytes("string")];
    let res = string_join_arr(&arr, 3, "\n");
    assert_eq!(string_bytes(&res), "\nsome\nstring");
    assert_eq!(res.size, 12);
}

#[test]
fn test_string_join_arr_multi_char_sep() {
    let arr = vec![new_string_from_bytes("some"), new_string_from_bytes("string")];
    let res = string_join_arr(&arr, 2, "SOME");
    assert_eq!(string_bytes(&res), "someSOMEstring");
    assert_eq!(res.size, 14);
}

// --- string_t_is_space_char ---
#[test]
fn test_is_space_char() {
    assert_eq!(string_t_is_space_char(b' '), true);
    assert_eq!(string_t_is_space_char(b'\t'), true);
    assert_eq!(string_t_is_space_char(b'\n'), true);
    assert_eq!(string_t_is_space_char(b'\r'), true);
    assert_eq!(string_t_is_space_char(b'a'), false);
    assert_eq!(string_t_is_space_char(b'0'), false);
}

// --- string_free (no-op in Rust, just ensure it doesn't panic) ---
#[test]
fn test_string_free() {
    let s = new_string_from_bytes("test");
    string_free(s);
}

fn main() {}
