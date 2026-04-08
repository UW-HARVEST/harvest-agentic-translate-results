use c_string::string_t::*;

// Helper to make a StringT and get its bytes as &str
fn s(text: &str) -> StringT {
    new_string_from_bytes(text)
}
fn bytes(st: &StringT) -> &str {
    string_bytes(st)
}

// ---- new_string ----
#[test]
fn test_new_string() {
    for &size in &[0, 1, 2, 3, 12, 100] {
        let st = new_string(size);
        assert_eq!(st.size, size);
        assert_eq!(st.bytes.len(), size);
    }
}

// ---- new_string_from_bytes ----
#[test]
fn test_new_string_from_bytes() {
    for &text in &["", "test", "some another test"] {
        let st = s(text);
        assert_eq!(st.size, text.len());
        assert_eq!(bytes(&st), text);
    }
}

// ---- string_len ----
#[test]
fn test_string_len() {
    let cases = [("", 0), ("test", 4), ("some another test", 17)];
    for &(text, expected) in &cases {
        assert_eq!(string_len(&s(text)), expected);
    }
}

// ---- string_bytes ----
#[test]
fn test_string_bytes() {
    for &text in &["", "test", "some another test"] {
        assert_eq!(string_bytes(&s(text)), text);
    }
}

// ---- string_eq ----
#[test]
fn test_string_eq() {
    // Equal pairs
    for &text in &["", "\n\n", "test", "some another test"] {
        assert!(string_eq(&s(text), &s(text)));
    }
    // Unequal
    assert!(!string_eq(&s("abc"), &s("def")));
    assert!(!string_eq(&s("abc"), &s("ab")));
}

// ---- string_copy ----
#[test]
fn test_string_copy() {
    for &text in &["", "test", "some another test"] {
        let st = s(text);
        let cp = string_copy(&st);
        assert!(string_eq(&st, &cp));
    }
}

// ---- string_concat ----
#[test]
fn test_string_concat() {
    let cases = [("", "", ""), ("first", "second", "firstsecond"), ("some another", " test", "some another test")];
    for &(a, b, expected) in &cases {
        let result = string_concat(&s(a), &s(b));
        assert!(string_eq(&result, &s(expected)));
    }
}

// ---- string_substr ----
#[test]
fn test_string_substr() {
    let cases = [("", 0, 0, ""), ("vfv\n\n", 1, 2, "fv"), ("test string", 5, 3, "str"), (" some another test  ", 0, 5, " some")];
    for &(text, pos, len, expected) in &cases {
        let result = string_substr(&s(text), pos, len);
        assert!(string_eq(&result, &s(expected)));
    }
}

// ---- string_startswith ----
#[test]
fn test_string_startswith() {
    assert!(string_startswith(&s(""), ""));
    assert!(string_startswith(&s("vfv\n\n"), "vfv"));
    assert!(string_startswith(&s("test string"), "test string"));
    assert!(string_startswith(&s(" some another test  "), " some"));
    assert!(!string_startswith(&s("1234"), "2"));
    assert!(!string_startswith(&s("ab"), "abcdef"));
}

// ---- string_endswith ----
#[test]
fn test_string_endswith() {
    assert!(string_endswith(&s(""), ""));
    assert!(string_endswith(&s("vfv\n\n"), "fv\n\n"));
    assert!(string_endswith(&s("test string"), "test string"));
    assert!(string_endswith(&s(" some another test  "), "test  "));
    assert!(!string_endswith(&s("1234"), "2"));
    assert!(!string_endswith(&s("ab"), "abcdef"));
}

// ---- string_find ----
#[test]
fn test_string_find() {
    // C returns 0 for empty search, Rust returns Some(0)
    assert_eq!(string_find(&s(""), ""), Some(0));
    assert_eq!(string_find(&s("test string"), ""), Some(0));
    // Normal finds
    assert_eq!(string_find(&s("vfv\n\n"), "\n"), Some(3));
    assert_eq!(string_find(&s("test string"), "str"), Some(5));
    assert_eq!(string_find(&s(" some another test  "), "another"), Some(6));
    // Not found: C returns -1, Rust returns None
    assert_eq!(string_find(&s("test string"), "no"), None);
}

// ---- string_strip ----
#[test]
fn test_string_strip() {
    let cases = [
        ("", ""),
        ("vfv\n\n", "vfv"),
        ("  test\t", "test"),
        (" some another test  ", "some another test"),
    ];
    for &(input, expected) in &cases {
        let result = string_strip(&s(input));
        assert!(string_eq(&result, &s(expected)));
    }
    // All spaces: C returns copy of original
    let all_spaces = s("   ");
    let stripped = string_strip(&all_spaces);
    assert!(string_eq(&stripped, &all_spaces));
    assert_eq!(stripped.size, 3);
}

// ---- string_split ----
#[test]
fn test_string_split() {
    // Empty string -> 1 part: ""
    let mut sz = 0;
    let arr = string_split(&s(""), &mut sz);
    assert_eq!(sz, 1);
    assert!(string_eq(&arr[0], &s("")));

    // Single word
    let arr = string_split(&s("1"), &mut sz);
    assert_eq!(sz, 1);
    assert!(string_eq(&arr[0], &s("1")));

    let arr = string_split(&s("some"), &mut sz);
    assert_eq!(sz, 1);
    assert!(string_eq(&arr[0], &s("some")));

    // Leading space: " some string 124!" -> ["", "some", "string", "124!"]
    let arr = string_split(&s(" some string 124!"), &mut sz);
    assert_eq!(sz, 4);
    let expected = ["", "some", "string", "124!"];
    for (i, &e) in expected.iter().enumerate() {
        assert!(string_eq(&arr[i], &s(e)), "split part {} mismatch: got '{}' expected '{}'", i, bytes(&arr[i]), e);
    }

    // Normal sentence
    let arr = string_split(&s("Some github account: vnkrtv"), &mut sz);
    assert_eq!(sz, 4);
    let expected = ["Some", "github", "account:", "vnkrtv"];
    for (i, &e) in expected.iter().enumerate() {
        assert!(string_eq(&arr[i], &s(e)));
    }
}

// ---- string_split_by ----
#[test]
fn test_string_split_by() {
    let mut sz = 0;

    // Size <= split_len: returns copy
    let arr = string_split_by(&s(""), &mut sz, "W");
    assert_eq!(sz, 1);
    assert!(string_eq(&arr[0], &s("")));

    let arr = string_split_by(&s("1"), &mut sz, "W");
    assert_eq!(sz, 1);
    assert!(string_eq(&arr[0], &s("1")));

    let arr = string_split_by(&s("W"), &mut sz, "W");
    assert_eq!(sz, 1);
    assert!(string_eq(&arr[0], &s("W")));

    // No delimiter found
    let arr = string_split_by(&s("some"), &mut sz, "W");
    assert_eq!(sz, 1);
    assert!(string_eq(&arr[0], &s("some")));

    // Leading delimiter: "WsomeWstringW124!" -> ["", "some", "string", "124!"]
    let arr = string_split_by(&s("WsomeWstringW124!"), &mut sz, "W");
    assert_eq!(sz, 4);
    let expected = ["", "some", "string", "124!"];
    for (i, &e) in expected.iter().enumerate() {
        assert!(string_eq(&arr[i], &s(e)), "split_by part {} mismatch: got '{}' expected '{}'", i, bytes(&arr[i]), e);
    }

    // Normal split
    let arr = string_split_by(&s("SomeWgithubWaccount:Wvnkrtv"), &mut sz, "W");
    assert_eq!(sz, 4);
    let expected = ["Some", "github", "account:", "vnkrtv"];
    for (i, &e) in expected.iter().enumerate() {
        assert!(string_eq(&arr[i], &s(e)));
    }

    // Multi-char delimiter: "helloWORLDfooWORLDbar" by "WORLD" -> 2 parts: "hello", "foo"
    let arr = string_split_by(&s("helloWORLDfooWORLDbar"), &mut sz, "WORLD");
    assert_eq!(sz, 2);
    assert!(string_eq(&arr[0], &s("hello")));
    assert!(string_eq(&arr[1], &s("foo")));
}

// ---- string_join_arr ----
#[test]
fn test_string_join_arr() {
    // Single element
    let arr = vec![s("")];
    let result = string_join_arr(&arr, 1, " ");
    assert!(string_eq(&result, &s("")));

    let arr = vec![s("1")];
    let result = string_join_arr(&arr, 1, " ");
    assert!(string_eq(&result, &s("1")));

    // Two elements with space
    let arr = vec![s("some"), s("string")];
    let result = string_join_arr(&arr, 2, " ");
    assert!(string_eq(&result, &s("some string")));

    // Three elements with newline
    let arr = vec![s(""), s("some"), s("string")];
    let result = string_join_arr(&arr, 3, "\n");
    assert!(string_eq(&result, &s("\nsome\nstring")));

    // Custom separator
    let arr = vec![s("some"), s("string")];
    let result = string_join_arr(&arr, 2, "SOME");
    assert!(string_eq(&result, &s("someSOMEstring")));

    // Empty separator
    let arr = vec![s("abc"), s("def")];
    let result = string_join_arr(&arr, 2, "");
    assert!(string_eq(&result, &s("abcdef")));
}

// ---- string_t_is_space_char ----
#[test]
fn test_is_space_char() {
    assert!(string_t_is_space_char(b' '));
    assert!(string_t_is_space_char(b'\t'));
    assert!(string_t_is_space_char(b'\n'));
    assert!(string_t_is_space_char(b'\r'));
    assert!(!string_t_is_space_char(b'a'));
    assert!(!string_t_is_space_char(b'0'));
}

// ---- string_free (no-op, just ensure it doesn't panic) ----
#[test]
fn test_string_free() {
    let st = s("test");
    string_free(st);
}

fn main() {}
