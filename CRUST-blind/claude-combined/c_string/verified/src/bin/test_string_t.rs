use c_string::string_t::{
    new_string, new_string_from_bytes, string_bytes, string_concat, string_copy, string_endswith,
    string_eq, string_find, string_free, string_join_arr, string_len, string_split,
    string_split_by, string_startswith, string_strip, string_substr, string_t_is_space_char,
    StringT, STRING_T_INDEXES_BUFFER_SIZE, STRING_T_SPACE_CHARS_ARR,
};

fn assert_string_eq_bytes(left: &StringT, expected: &str) {
    let exp = expected.as_bytes();
    assert_eq!(left.size, exp.len(), "size mismatch");
    for i in 0..exp.len() {
        assert_eq!(left.bytes[i], exp[i], "byte {} mismatch", i);
    }
}

#[test]
fn test_constants() {
    assert_eq!(STRING_T_INDEXES_BUFFER_SIZE, 512);
    assert_eq!(STRING_T_SPACE_CHARS_ARR, " \t\n\r");
}

#[test]
fn test_new_string() {
    for size in [0usize, 1, 2, 3, 12, 100].iter() {
        let str = new_string(*size);
        assert_eq!(str.size, *size);
        assert_eq!(str.bytes.len(), *size);
        // All bytes should be zero (calloc behavior)
        for b in str.bytes.iter() {
            assert_eq!(*b, 0);
        }
    }
}

#[test]
fn test_new_string_from_bytes() {
    let cases = ["", "test", "some another test"];
    for c in cases.iter() {
        let str = new_string_from_bytes(c);
        assert_eq!(str.size, c.len());
        assert_string_eq_bytes(&str, c);
    }
}

#[test]
fn test_string_len() {
    let cases = [("", 0usize), ("test", 4), ("some another test", 17)];
    for (b, l) in cases.iter() {
        let str = new_string_from_bytes(b);
        assert_eq!(str.size, b.len());
        assert_eq!(string_len(&str), *l);
    }
}

#[test]
fn test_string_bytes() {
    let cases = ["", "test", "some another test"];
    for c in cases.iter() {
        let str = new_string_from_bytes(c);
        let bytes_slice = string_bytes(&str);
        assert_eq!(bytes_slice, *c);
    }
}

#[test]
fn test_string_eq() {
    let lefts = ["", "\n\n", "test", "some another test"];
    let rights = ["", "\n\n", "test", "some another test"];
    for i in 0..4 {
        let l = new_string_from_bytes(lefts[i]);
        let r = new_string_from_bytes(rights[i]);
        assert_eq!(l.size, r.size);
        assert!(string_eq(&l, &r));
    }
    // Inequality cases
    let a = new_string_from_bytes("abc");
    let b = new_string_from_bytes("abd");
    assert!(!string_eq(&a, &b));

    let a2 = new_string_from_bytes("ab");
    let b2 = new_string_from_bytes("abc");
    assert!(!string_eq(&a2, &b2));

    let empty1 = new_string_from_bytes("");
    let empty2 = new_string_from_bytes("");
    assert!(string_eq(&empty1, &empty2));
}

#[test]
fn test_string_copy() {
    let cases = ["", "test", "some another test"];
    for c in cases.iter() {
        let str = new_string_from_bytes(c);
        let copied = string_copy(&str);
        assert_eq!(copied.size, str.size);
        assert!(string_eq(&str, &copied));
        assert_string_eq_bytes(&copied, c);
    }
}

#[test]
fn test_string_concat() {
    let firsts = ["", "first", "some another"];
    let seconds = ["", "second", " test"];
    let res = ["", "firstsecond", "some another test"];
    for i in 0..3 {
        let f = new_string_from_bytes(firsts[i]);
        let s2 = new_string_from_bytes(seconds[i]);
        let r = new_string_from_bytes(res[i]);
        let c = string_concat(&f, &s2);
        assert_eq!(c.size, r.size);
        assert!(string_eq(&c, &r));
        assert_string_eq_bytes(&c, res[i]);
    }
}

#[test]
fn test_string_substr() {
    let bytes = ["", "vfv\n\n", "test string", " some another test  "];
    let starts = [0usize, 1, 5, 0];
    let lens = [0usize, 2, 3, 5];
    let expected = ["", "fv", "str", " some"];

    for i in 0..4 {
        let str = new_string_from_bytes(bytes[i]);
        let sub = string_substr(&str, starts[i], lens[i]);
        let exp = new_string_from_bytes(expected[i]);
        assert_eq!(sub.size, exp.size);
        assert!(string_eq(&sub, &exp));
        assert_string_eq_bytes(&sub, expected[i]);
    }

    // substr with len=0 from middle
    let str = new_string_from_bytes("hello world");
    let sub = string_substr(&str, 5, 0);
    assert_eq!(sub.size, 0);
}

#[test]
fn test_string_startswith() {
    let bytes = ["", "vfv\n\n", "test string", " some another test  ", "1234"];
    let prefixes = ["", "vfv", "test string", " some", "2"];
    let res = [true, true, true, true, false];
    for i in 0..5 {
        let str = new_string_from_bytes(bytes[i]);
        assert_eq!(string_startswith(&str, prefixes[i]), res[i]);
    }

    // Additional cases verified by C probe
    let s1 = new_string_from_bytes("hello world");
    assert!(string_startswith(&s1, ""));
    assert!(string_startswith(&s1, "he"));
    assert!(!string_startswith(&s1, "hi"));
    // Prefix longer than string
    assert!(!string_startswith(&s1, "hello world!"));
}

#[test]
fn test_string_endswith() {
    let bytes = ["", "vfv\n\n", "test string", " some another test  ", "1234"];
    let suffixes = ["", "fv\n\n", "test string", "test  ", "2"];
    let res = [true, true, true, true, false];
    for i in 0..5 {
        let str = new_string_from_bytes(bytes[i]);
        assert_eq!(string_endswith(&str, suffixes[i]), res[i]);
    }

    let s1 = new_string_from_bytes("hello world");
    assert!(string_endswith(&s1, ""));
    assert!(string_endswith(&s1, "ld"));
    assert!(!string_endswith(&s1, "lD"));
    assert!(!string_endswith(&s1, "HELLO WORLD!"));
}

#[test]
fn test_string_find() {
    let bytes = ["", "vfv\n\n", "test string", "test string", " some another test  "];
    let chars = ["", "\n", "no", "", "another"];
    let expected = [Some(0usize), Some(3), None, Some(0), Some(6)];
    for i in 0..5 {
        let str = new_string_from_bytes(bytes[i]);
        assert_eq!(string_find(&str, chars[i]), expected[i]);
    }

    // Additional cases verified from C probe
    let s1 = new_string_from_bytes("hello world");
    assert_eq!(string_find(&s1, "world"), Some(6));
    assert_eq!(string_find(&s1, "hello"), Some(0));
    assert_eq!(string_find(&s1, "xx"), None);
    assert_eq!(string_find(&s1, ""), Some(0));
    assert_eq!(string_find(&s1, "o"), Some(4));
}

#[test]
fn test_string_strip() {
    let bytes = ["", "vfv\n\n", "  test\t", " some another test  "];
    let stripped = ["", "vfv", "test", "some another test"];
    for i in 0..4 {
        let str = new_string_from_bytes(bytes[i]);
        let result = string_strip(&str);
        let expected = new_string_from_bytes(stripped[i]);
        assert_eq!(result.size, expected.size);
        assert!(string_eq(&result, &expected));
        assert_string_eq_bytes(&result, stripped[i]);
    }

    // Additional cases verified from C probe
    let s1 = new_string_from_bytes(" a ");
    let r1 = string_strip(&s1);
    // Per C behavior: when start_pos >= end_pos, return string_copy(str). " a " yields " a "
    assert_string_eq_bytes(&r1, " a ");

    let s2 = new_string_from_bytes("a");
    let r2 = string_strip(&s2);
    assert_string_eq_bytes(&r2, "a");

    let s3 = new_string_from_bytes("ab");
    let r3 = string_strip(&s3);
    assert_string_eq_bytes(&r3, "ab");

    let s4 = new_string_from_bytes(" abc ");
    let r4 = string_strip(&s4);
    assert_string_eq_bytes(&r4, "abc");

    let s5 = new_string_from_bytes("   ");
    let r5 = string_strip(&s5);
    assert_string_eq_bytes(&r5, "   ");
}

#[test]
fn test_string_split() {
    let bytes = [
        "",
        "1",
        "some",
        " some string 124!",
        "Some github account: vnkrtv",
    ];
    let expected_sizes = [1usize, 1, 1, 4, 4];
    let expected: Vec<Vec<&str>> = vec![
        vec![""],
        vec!["1"],
        vec!["some"],
        vec!["", "some", "string", "124!"],
        vec!["Some", "github", "account:", "vnkrtv"],
    ];

    for idx in 0..5 {
        let str = new_string_from_bytes(bytes[idx]);
        let mut arr_size = 0usize;
        let arr = string_split(&str, &mut arr_size);
        assert_eq!(arr_size, expected_sizes[idx]);
        assert_eq!(arr.len(), expected_sizes[idx]);
        for j in 0..expected_sizes[idx] {
            assert_string_eq_bytes(&arr[j], expected[idx][j]);
        }
    }

    // Additional case verified from C probe
    let s_extra = new_string_from_bytes("hello world");
    let mut arr_size = 0usize;
    let arr = string_split(&s_extra, &mut arr_size);
    assert_eq!(arr_size, 2);
    assert_string_eq_bytes(&arr[0], "hello");
    assert_string_eq_bytes(&arr[1], "world");

    // Trailing whitespace: "hello world " -> ["hello", "world"]
    let s_extra2 = new_string_from_bytes("hello world ");
    let mut arr_size2 = 0usize;
    let arr2 = string_split(&s_extra2, &mut arr_size2);
    assert_eq!(arr_size2, 2);
    assert_string_eq_bytes(&arr2[0], "hello");
    assert_string_eq_bytes(&arr2[1], "world");
}

#[test]
fn test_string_split_by() {
    let split_chars = "W";
    let bytes = [
        "",
        "1",
        "some",
        "WsomeWstringW124!",
        "SomeWgithubWaccount:Wvnkrtv",
    ];
    let expected_sizes = [1usize, 1, 1, 4, 4];
    let expected: Vec<Vec<&str>> = vec![
        vec![""],
        vec!["1"],
        vec!["some"],
        vec!["", "some", "string", "124!"],
        vec!["Some", "github", "account:", "vnkrtv"],
    ];

    for idx in 0..5 {
        let str = new_string_from_bytes(bytes[idx]);
        let mut arr_size = 0usize;
        let arr = string_split_by(&str, &mut arr_size, split_chars);
        assert_eq!(arr_size, expected_sizes[idx]);
        assert_eq!(arr.len(), expected_sizes[idx]);
        for j in 0..expected_sizes[idx] {
            assert_string_eq_bytes(&arr[j], expected[idx][j]);
        }
    }

    // Edge case: separator length equals string length
    let s_eq = new_string_from_bytes("AB");
    let mut n = 0usize;
    let arr = string_split_by(&s_eq, &mut n, "AB");
    assert_eq!(n, 1);
    assert_string_eq_bytes(&arr[0], "AB");
}

#[test]
fn test_string_join_arr() {
    let arrs: Vec<Vec<StringT>> = vec![
        vec![new_string_from_bytes("")],
        vec![new_string_from_bytes("1")],
        vec![new_string_from_bytes("some"), new_string_from_bytes("string")],
        vec![
            new_string_from_bytes(""),
            new_string_from_bytes("some"),
            new_string_from_bytes("string"),
        ],
        vec![new_string_from_bytes("some"), new_string_from_bytes("string")],
    ];
    let arr_size = [1usize, 1, 2, 3, 2];
    let space_chars = [" ", " ", " ", "\n", "SOME"];
    let expected = ["", "1", "some string", "\nsome\nstring", "someSOMEstring"];

    for idx in 0..5 {
        let res = string_join_arr(&arrs[idx], arr_size[idx], space_chars[idx]);
        let exp = new_string_from_bytes(expected[idx]);
        assert_eq!(res.size, exp.size);
        assert!(string_eq(&res, &exp));
        assert_string_eq_bytes(&res, expected[idx]);
    }
}

#[test]
fn test_string_t_is_space_char() {
    assert!(string_t_is_space_char(b' '));
    assert!(string_t_is_space_char(b'\t'));
    assert!(string_t_is_space_char(b'\n'));
    assert!(string_t_is_space_char(b'\r'));
    assert!(!string_t_is_space_char(b'a'));
    assert!(!string_t_is_space_char(b'0'));
    assert!(!string_t_is_space_char(0));
    assert!(!string_t_is_space_char(b'_'));
}

#[test]
fn test_string_free() {
    // string_free in Rust is a no-op (Drop handles deallocation), but should
    // accept an owned StringT and not panic.
    let s = new_string_from_bytes("hello");
    string_free(s);
    let s2 = new_string(0);
    string_free(s2);
}

fn main() {}
