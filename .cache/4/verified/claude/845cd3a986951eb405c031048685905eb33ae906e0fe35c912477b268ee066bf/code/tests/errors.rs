//! Phase C — one differential test per row of `ERRORS.md` (except the rows that
//! need a virgin process, see `tests/uninitialized.rs`, and the driver rows, see
//! `tests/driver_e2e.rs`).
//!
//! Every row asserts the *same* sentinel/error text from both libraries, not
//! merely "both failed somehow".

mod common;

use common::*;
use std::ffi::c_char;

fn init_both() -> &'static Pair {
    let p = libs();
    (p.c.analyzer_init)((p.c.get_tokenizer_ops)());
    (p.rust.analyzer_init)((p.rust.get_tokenizer_ops)());
    p
}

// ---------------------------------------------------------------------------
// E1-E4: tokenizer_load_text
// ---------------------------------------------------------------------------

#[test]
fn e1_load_text_null() {
    let _g = lock();
    let p = libs();
    let (co, ce) = p.c.captured_both(|| {
        let rc = (p.c.tokenizer_load_text)(std::ptr::null());
        assert_eq!(rc, -1, "C tokenizer_load_text(NULL)");
    });
    let (ro, re) = p.rust.captured_both(|| {
        let rc = (p.rust.tokenizer_load_text)(std::ptr::null());
        assert_eq!(rc, -1, "Rust tokenizer_load_text(NULL)");
    });
    assert_eq!(show(&co), show(&ro));
    assert_eq!(show(&ce), show(&re));
    assert!(ce.is_empty(), "E1 must print nothing: {}", show(&ce));

    // the previously loaded buffer must be untouched
    let text = b"abc def";
    assert_eq!(p.c.load_text(text), p.rust.load_text(text));
    assert_eq!(p.c.next(), p.rust.next());
    assert_eq!((p.c.tokenizer_load_text)(std::ptr::null()), -1);
    assert_eq!((p.rust.tokenizer_load_text)(std::ptr::null()), -1);
    assert_eq!(p.c.drain_tokens(), p.rust.drain_tokens());
    assert_eq!(p.c.stats(), p.rust.stats());
}

#[test]
fn e2_e3_e4_load_text_size_limit() {
    let _g = lock();
    let p = libs();
    for n in [8190usize, 8191, 8192, 8193, 20000] {
        let text = vec![b'a'; n];
        let s = cstring(&text);
        let (co, ce) = p.c.captured_both(|| {
            let rc = (p.c.tokenizer_load_text)(s.as_ptr() as *const c_char);
            assert_eq!(rc, if n >= MAX_BUFFER_SIZE { -1 } else { 0 }, "C rc for n={}", n);
        });
        let (ro, re) = p.rust.captured_both(|| {
            let rc = (p.rust.tokenizer_load_text)(s.as_ptr() as *const c_char);
            assert_eq!(rc, if n >= MAX_BUFFER_SIZE { -1 } else { 0 }, "Rust rc for n={}", n);
        });
        assert_eq!(show(&co), show(&ro), "stdout differs for n={}", n);
        assert_eq!(show(&ce), show(&re), "stderr differs for n={}", n);
        if n >= MAX_BUFFER_SIZE {
            assert_eq!(
                show(&ce),
                "Error: Input text too large\\n",
                "unexpected message for n={}",
                n
            );
        } else {
            assert!(ce.is_empty(), "unexpected message for n={}: {}", n, show(&ce));
        }
        // a rejected load must leave the previous buffer in place
        assert_eq!(p.c.drain_tokens(), p.rust.drain_tokens());
        assert_eq!(p.c.stats(), p.rust.stats());
    }
}

// ---------------------------------------------------------------------------
// E5, E6: reading at/after the end of the buffer
// ---------------------------------------------------------------------------

#[test]
fn e5_e6_eof_behaviour() {
    let _g = lock();
    let p = libs();
    for text in [&b""[..], &b"a"[..], &b" "[..], &b"\n"[..]] {
        assert_eq!(p.c.load_text(text), p.rust.load_text(text));
        let tc = p.c.drain_tokens();
        let tr = p.rust.drain_tokens();
        assert_eq!(tc, tr);
        let last = tc.last().unwrap();
        assert_eq!(last.ttype, TOKEN_EOF);
        assert_eq!(last.value, Vec::<u8>::new());
        assert_eq!(last.length, 0);

        // advance_char at the end returns '\0' without counting a character,
        // while create_token keeps counting tokens
        let (l0, t0, c0) = p.c.stats();
        let (l0r, t0r, c0r) = p.rust.stats();
        assert_eq!((l0, t0, c0), (l0r, t0r, c0r));
        for _ in 0..7 {
            assert_eq!(p.c.next(), p.rust.next());
        }
        let (l1, t1, c1) = p.c.stats();
        let (l1r, t1r, c1r) = p.rust.stats();
        assert_eq!((l1, t1, c1), (l1r, t1r, c1r));
        assert_eq!((l1, c1), (l0, c0), "lines/chars must not advance past EOF");
        assert_eq!(t1, t0 + 7, "every EOF token still counts");
    }
}

// ---------------------------------------------------------------------------
// E7-E11, E16, E17b: length clamping
// ---------------------------------------------------------------------------

#[test]
fn e7_e8_e10_length_clamping() {
    let _g = lock();
    let p = libs();

    // identifier of 300 chars -> 255 + 45
    let text = vec![b'a'; 300];
    assert_eq!(p.c.load_text(&text), p.rust.load_text(&text));
    let tc = p.c.drain_tokens();
    assert_eq!(tc, p.rust.drain_tokens());
    assert_eq!(tc[0].length, 255);
    assert_eq!(tc[0].value.len(), 255);
    assert_eq!(tc[1].length, 45);

    // number of 300 digits
    let text = vec![b'7'; 300];
    assert_eq!(p.c.load_text(&text), p.rust.load_text(&text));
    let tc = p.c.drain_tokens();
    assert_eq!(tc, p.rust.drain_tokens());
    assert_eq!(tc[0].length, 255);
    assert_eq!(tc[1].length, 45);

    // scan_string stops at `length < MAX_TOKEN_LENGTH - 2` (254): with 253 body
    // bytes the loop ends exactly on the closing quote, which is still appended
    // (length 255).
    let mut text: Vec<u8> = b"\"".to_vec();
    text.extend_from_slice(&vec![b's'; 253]);
    text.extend_from_slice(b"\"rest");
    assert_eq!(p.c.load_text(&text), p.rust.load_text(&text));
    let tc = p.c.drain_tokens();
    assert_eq!(tc, p.rust.drain_tokens());
    assert_eq!(tc[0].ttype, TOKEN_STRING);
    assert_eq!(tc[0].length, 255);
    assert_eq!(tc[0].value.len(), 255);
    assert!(tc[0].value.ends_with(b"\""), "{}", show(&tc[0].value));

    // a longer literal stops at 254 and never sees its closing quote
    let mut text: Vec<u8> = b"\"".to_vec();
    text.extend_from_slice(&vec![b's'; 400]);
    text.extend_from_slice(b"\"rest");
    assert_eq!(p.c.load_text(&text), p.rust.load_text(&text));
    let tc = p.c.drain_tokens();
    assert_eq!(tc, p.rust.drain_tokens());
    assert_eq!(tc[0].length, 254);
    assert!(tc[0].value.ends_with(b"s"), "{}", show(&tc[0].value));

    // E7: the escape branch pushes two bytes per iteration, so `length` reaches
    // 255 and the closing quote makes it 256 - which create_token clamps to 255
    // (and the clamped copy drops the closing quote again).
    let mut text: Vec<u8> = b"'".to_vec();
    for _ in 0..127 {
        text.extend_from_slice(b"\\x");
    }
    text.extend_from_slice(b"'rest");
    assert_eq!(p.c.load_text(&text), p.rust.load_text(&text));
    let tc = p.c.drain_tokens();
    assert_eq!(tc, p.rust.drain_tokens());
    assert_eq!(tc[0].ttype, TOKEN_STRING);
    assert_eq!(tc[0].length, 255);
    assert_eq!(tc[0].value.len(), 255);
    assert!(tc[0].value.ends_with(b"x"), "{}", show(&tc[0].value));
}

#[test]
fn e9_second_decimal_point() {
    let _g = lock();
    let p = libs();
    for text in [&b"1.2.3"[..], &b"1..2"[..], &b"0.0.0"[..], &b"9.9.9.9"[..]] {
        assert_eq!(p.c.load_text(text), p.rust.load_text(text));
        let tc = p.c.drain_tokens();
        assert_eq!(tc, p.rust.drain_tokens());
        assert_eq!(tc[0].ttype, TOKEN_NUMBER);
        assert_eq!(tc[1].ttype, TOKEN_PUNCTUATION, "{}", show(text));
        assert_eq!(tc[1].value, b".".to_vec());
    }
}

#[test]
fn e11_e12_e13_e14_unterminated_strings() {
    let _g = lock();
    let p = libs();
    let cases: Vec<&[u8]> = vec![
        b"\"abc",
        b"'abc",
        b"\"abc\ndef\"",
        b"\"ab\\",
        b"\"ab\\\"",
        b"'a\\'",
        b"\"",
        b"'",
        b"\"\\",
    ];
    for text in cases {
        assert_eq!(p.c.load_text(text), p.rust.load_text(text));
        let tc = p.c.drain_tokens();
        assert_eq!(tc, p.rust.drain_tokens(), "text {}", show(text));
        assert_eq!(tc[0].ttype, TOKEN_STRING, "text {}", show(text));
        println!("{:>12} -> {:?}", show(text), tc[0]);
    }
}

#[test]
fn e15_e16_e17_e17b_e17c_comments() {
    let _g = lock();
    let p = libs();

    // E17c: a lone '/' becomes a COMMENT, never an OPERATOR
    for text in [&b"/"[..], &b"a / b"[..], &b"/="[..], &b"/x"[..], &b"/ /"[..]] {
        assert_eq!(p.c.load_text(text), p.rust.load_text(text));
        let tc = p.c.drain_tokens();
        assert_eq!(tc, p.rust.drain_tokens());
        let slash = tc.iter().find(|t| t.value.starts_with(b"/")).unwrap();
        assert_eq!(slash.ttype, TOKEN_COMMENT, "text {}", show(text));
    }

    // E15/E16: // comment ends at the newline or the 255-byte bound
    let mut text: Vec<u8> = b"//".to_vec();
    text.extend_from_slice(&vec![b'c'; 400]);
    text.extend_from_slice(b"\nx");
    assert_eq!(p.c.load_text(&text), p.rust.load_text(&text));
    let tc = p.c.drain_tokens();
    assert_eq!(tc, p.rust.drain_tokens());
    assert_eq!(tc[0].ttype, TOKEN_COMMENT);
    assert_eq!(tc[0].length, 255);

    // E17/E17b: unterminated /* comment
    for text in [
        b"/*".to_vec(),
        b"/**".to_vec(),
        b"/* abc".to_vec(),
        {
            let mut t = b"/*".to_vec();
            t.extend_from_slice(&vec![b'c'; 400]);
            t.extend_from_slice(b"*/tail");
            t
        },
    ] {
        assert_eq!(p.c.load_text(&text), p.rust.load_text(&text));
        let tc = p.c.drain_tokens();
        assert_eq!(tc, p.rust.drain_tokens(), "text {}", show(&text));
        assert_eq!(tc[0].ttype, TOKEN_COMMENT);
        assert!(tc[0].length <= 255);
    }
}

// ---------------------------------------------------------------------------
// E18: get_stats with NULL out-parameters
// ---------------------------------------------------------------------------

#[test]
fn e18_get_stats_null_out_params() {
    let _g = lock();
    let p = libs();
    let text = b"a b c\n1 2 3\n";
    assert_eq!(p.c.load_text(text), p.rust.load_text(text));
    assert_eq!(p.c.drain_tokens(), p.rust.drain_tokens());

    // all NULL: nothing must be written, nothing must crash
    (p.c.tokenizer_get_stats)(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
    (p.rust.tokenizer_get_stats)(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());

    const SENTINEL: usize = 0xdead_beef;
    for mask in 0..8u32 {
        let mut c = [SENTINEL; 3];
        let mut r = [SENTINEL; 3];
        let cp: [*mut usize; 3] = [
            if mask & 1 != 0 { &mut c[0] } else { std::ptr::null_mut() },
            if mask & 2 != 0 { &mut c[1] } else { std::ptr::null_mut() },
            if mask & 4 != 0 { &mut c[2] } else { std::ptr::null_mut() },
        ];
        let rp: [*mut usize; 3] = [
            if mask & 1 != 0 { &mut r[0] } else { std::ptr::null_mut() },
            if mask & 2 != 0 { &mut r[1] } else { std::ptr::null_mut() },
            if mask & 4 != 0 { &mut r[2] } else { std::ptr::null_mut() },
        ];
        (p.c.tokenizer_get_stats)(cp[0], cp[1], cp[2]);
        (p.rust.tokenizer_get_stats)(rp[0], rp[1], rp[2]);
        assert_eq!(c, r, "mask {}", mask);
        for i in 0..3 {
            let written = mask & (1 << i) != 0;
            assert_eq!(
                c[i] != SENTINEL,
                written,
                "mask {} slot {} written-ness",
                mask,
                i
            );
        }
    }
}

// ---------------------------------------------------------------------------
// E19, E20: analyze_text rejections
// ---------------------------------------------------------------------------

#[test]
fn e19_analyze_text_null() {
    let _g = lock();
    let p = init_both();
    let (co, ce) = p.c.captured_both(|| {
        let r = (p.c.analyze_text)(std::ptr::null());
        assert_eq!(r, CResult::default(), "C analyze_text(NULL) result");
    });
    let (ro, re) = p.rust.captured_both(|| {
        let r = (p.rust.analyze_text)(std::ptr::null());
        assert_eq!(r, CResult::default(), "Rust analyze_text(NULL) result");
    });
    assert_eq!(show(&co), show(&ro));
    assert_eq!(show(&ce), show(&re));
    assert_eq!(show(&ce), "Error: Failed to load text\\n");
    assert!(co.is_empty());
}

#[test]
fn e20_analyze_text_too_large() {
    let _g = lock();
    let p = init_both();
    for n in [8191usize, 8192, 9000] {
        let text = vec![b'z'; n];
        let s = cstring(&text);
        let (co, ce) = p.c.captured_both(|| {
            let _ = (p.c.analyze_text)(s.as_ptr() as *const c_char);
        });
        let (ro, re) = p.rust.captured_both(|| {
            let _ = (p.rust.analyze_text)(s.as_ptr() as *const c_char);
        });
        assert_eq!(show(&co), show(&ro), "stdout for n={}", n);
        assert_eq!(show(&ce), show(&re), "stderr for n={}", n);
        if n >= MAX_BUFFER_SIZE {
            assert_eq!(
                show(&ce),
                "Error: Input text too large\\nError: Failed to load text\\n",
                "n={}",
                n
            );
        } else {
            assert!(ce.is_empty(), "n={}: {}", n, show(&ce));
        }
    }
}

// ---------------------------------------------------------------------------
// E21, E22: track_word limits
// ---------------------------------------------------------------------------

#[test]
fn e21_more_than_100_distinct_words() {
    let _g = lock();
    let p = init_both();
    let mut text = Vec::new();
    for i in 0..150 {
        text.extend_from_slice(format!("word{} ", i).as_bytes());
    }
    // repeat the 101st word: it must be dropped, but still counted as a word
    text.extend_from_slice(b"word100 word100 word0 ");

    let rc = p.c.analyze(&text);
    let rr = p.rust.analyze(&text);
    assert_eq!(rc, rr);
    assert_eq!(rc.word_count, 153);

    let c = p.c.captured(|| (p.c.print_token_distribution)());
    let r = p.rust.captured(|| (p.rust.print_token_distribution)());
    assert_eq!(show(&c), show(&r));
    // the top-10 list can only mention words that made it into the table
    assert!(!c.windows(8).any(|w| w == b"word100:"), "{}", show(&c));
}

#[test]
fn e22_word_longer_than_255() {
    let _g = lock();
    let p = init_both();
    let mut text = Vec::new();
    for n in [254usize, 255, 256, 300] {
        text.extend_from_slice(&vec![b'w'; n]);
        text.push(b' ');
    }
    let rc = p.c.analyze(&text);
    assert_eq!(rc, p.rust.analyze(&text));
    let c = p.c.captured(|| (p.c.print_token_distribution)());
    let r = p.rust.captured(|| (p.rust.print_token_distribution)());
    assert_eq!(show(&c), show(&r));
}

// ---------------------------------------------------------------------------
// E23, E25: find_patterns rejections
// ---------------------------------------------------------------------------

#[test]
fn e23_find_patterns_null() {
    let _g = lock();
    let p = init_both();
    p.c.analyze(b"alpha beta");
    p.rust.analyze(b"alpha beta");

    let (co, ce) = p.c.captured_both(|| (p.c.find_patterns)(std::ptr::null()));
    let (ro, re) = p.rust.captured_both(|| (p.rust.find_patterns)(std::ptr::null()));
    assert_eq!(show(&co), show(&ro));
    assert_eq!(show(&ce), show(&re));
    assert!(co.is_empty(), "NULL pattern must print nothing: {}", show(&co));
    assert!(ce.is_empty());
    // and the tokenizer must not have been touched
    assert_eq!(p.c.stats(), p.rust.stats());
}

#[test]
fn e25_find_patterns_empty_pattern() {
    let _g = lock();
    let p = init_both();
    p.c.analyze(b"a bb ccc");
    p.rust.analyze(b"a bb ccc");
    let c = p.c.find(b"");
    let r = p.rust.find(b"");
    assert_eq!(show(&c), show(&r));
    // strstr with an empty needle matches every token
    assert_eq!(
        c.windows(5).filter(|w| *w == b"Line ").count(),
        3,
        "{}",
        show(&c)
    );
    assert!(c.ends_with(b"Found 3 occurrences\n"), "{}", show(&c));
}

// ---------------------------------------------------------------------------
// E26-E29: analyzer reporting edge cases
// ---------------------------------------------------------------------------

#[test]
fn e26_negative_score_clamped() {
    let _g = lock();
    let p = init_both();
    for text in [
        &b"// one"[..],
        &b"// one\n// two\n// three\n"[..],
        &b"/*a*//*b*//*c*/"[..],
    ] {
        (p.c.analyzer_init)((p.c.get_tokenizer_ops)());
        (p.rust.analyzer_init)((p.rust.get_tokenizer_ops)());
        p.c.analyze(text);
        p.rust.analyze(text);
        let sc = (p.c.calculate_complexity_score)();
        let sr = (p.rust.calculate_complexity_score)();
        assert_eq!(sc, sr);
        assert_eq!(sc, 0, "score must be clamped for {}", show(text));
    }
}

#[test]
fn e27_e28_empty_distribution() {
    let _g = lock();
    let p = init_both();
    let c = p.c.captured(|| (p.c.print_token_distribution)());
    let r = p.rust.captured(|| (p.rust.print_token_distribution)());
    assert_eq!(show(&c), show(&r));
    assert_eq!(
        show(&c),
        "\\n=== Token Distribution ===\\n\\n=== Most Common Words ===\\n"
    );
}

#[test]
fn e29_top_ten_only() {
    let _g = lock();
    let p = init_both();
    let mut text = Vec::new();
    for i in 0..25 {
        for _ in 0..(25 - i) {
            text.extend_from_slice(format!("w{} ", i).as_bytes());
        }
    }
    p.c.analyze(&text);
    p.rust.analyze(&text);
    let c = p.c.captured(|| (p.c.print_token_distribution)());
    let r = p.rust.captured(|| (p.rust.print_token_distribution)());
    assert_eq!(show(&c), show(&r));
    let lines = c.split(|&b| b == b'\n').filter(|l| l.contains(&b'.')).count();
    assert_eq!(lines, 10, "only the top 10 words are printed:\n{}", show(&c));
}

// ---------------------------------------------------------------------------
// E30-E32b: read_file rejections
// ---------------------------------------------------------------------------

fn diff_read_file_err(path: &[u8]) -> (Vec<u8>, Vec<u8>, bool) {
    let p = libs();
    let s = cstring(path);
    let mut c_null = false;
    let mut r_null = false;
    let (co, ce) = p.c.captured_both(|| {
        let q = (p.c.read_file)(s.as_ptr() as *const c_char);
        c_null = q.is_null();
        if !q.is_null() {
            c_free(q);
        }
    });
    let (ro, re) = p.rust.captured_both(|| {
        let q = (p.rust.read_file)(s.as_ptr() as *const c_char);
        r_null = q.is_null();
        if !q.is_null() {
            c_free(q);
        }
    });
    assert_eq!(show(&co), show(&ro), "read_file stdout for {}", show(path));
    assert_eq!(show(&ce), show(&re), "read_file stderr for {}", show(path));
    assert_eq!(c_null, r_null, "read_file NULL-ness for {}", show(path));
    (co, ce, c_null)
}

#[test]
fn e30_read_file_missing() {
    let _g = lock();
    let path = b"/definitely/not/here/xyz.txt";
    let (_, ce, isnull) = diff_read_file_err(path);
    assert!(isnull);
    assert_eq!(
        show(&ce),
        "Error: Could not open file '/definitely/not/here/xyz.txt'\\n"
    );

    // relative, empty and odd names
    for p in [&b""[..], &b"."[..], &b".."[..], &b"/"[..], &b"\n"[..], &b" "[..]] {
        diff_read_file_err(p);
    }
}

#[test]
fn e31_read_file_null_filename() {
    let _g = lock();
    let p = libs();
    let mut c_null = false;
    let mut r_null = false;
    let (co, ce) = p.c.captured_both(|| {
        let q = (p.c.read_file)(std::ptr::null());
        c_null = q.is_null();
        if !q.is_null() {
            c_free(q);
        }
    });
    let (ro, re) = p.rust.captured_both(|| {
        let q = (p.rust.read_file)(std::ptr::null());
        r_null = q.is_null();
        if !q.is_null() {
            c_free(q);
        }
    });
    assert_eq!(show(&co), show(&ro));
    assert_eq!(show(&ce), show(&re));
    assert!(c_null && r_null);
    assert_eq!(show(&ce), "Error: Could not open file '(null)'\\n");
}

#[test]
fn e31b_read_file_unreadable() {
    let _g = lock();
    let path = std::env::temp_dir().join(format!("ta_noperm_{}", std::process::id()));
    std::fs::write(&path, b"secret").unwrap();
    let mut perms = std::fs::metadata(&path).unwrap().permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o000);
    }
    std::fs::set_permissions(&path, perms).unwrap();

    let bytes = path.as_os_str().as_encoded_bytes().to_vec();
    let (_, ce, isnull) = diff_read_file_err(&bytes);
    // running as root would make the open succeed; accept either, but both
    // libraries must agree (already asserted by diff_read_file_err)
    if isnull {
        assert_eq!(
            show(&ce),
            format!("Error: Could not open file '{}'\\n", show(&bytes))
        );
    }
    let _ = std::fs::remove_file(&path);
}

#[test]
fn e31c_read_file_directory() {
    let _g = lock();
    let dir = std::env::temp_dir().join(format!("ta_dir_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let (_, _, isnull) = diff_read_file_err(dir.as_os_str().as_encoded_bytes());
    assert!(!isnull, "fopen(directory) succeeds on Linux");
    let _ = std::fs::remove_dir(&dir);
}

#[test]
fn e32_read_file_too_large() {
    let _g = lock();
    for n in [8192usize, 8193, 8194, 20000] {
        let path = std::env::temp_dir().join(format!("ta_big_{}_{}", std::process::id(), n));
        std::fs::write(&path, vec![b'x'; n]).unwrap();
        let bytes = path.as_os_str().as_encoded_bytes().to_vec();
        let (_, ce, isnull) = diff_read_file_err(&bytes);
        if n > MAX_BUFFER_SIZE {
            assert!(isnull, "n={} must be rejected", n);
            assert_eq!(show(&ce), "Error: File too large\\n", "n={}", n);
        } else {
            assert!(!isnull, "n={} is exactly at the limit and is accepted", n);
            assert!(ce.is_empty(), "n={}: {}", n, show(&ce));
        }
        let _ = std::fs::remove_file(&path);
    }
}

#[test]
fn e32b_read_file_8192_then_analyze() {
    let _g = lock();
    let p = init_both();
    let path = std::env::temp_dir().join(format!("ta_8192_{}", std::process::id()));
    std::fs::write(&path, vec![b'q'; 8192]).unwrap();
    let name = cstring(path.as_os_str().as_encoded_bytes());

    let (co, ce) = p.c.captured_both(|| {
        let content = (p.c.read_file)(name.as_ptr() as *const c_char);
        assert!(!content.is_null());
        let r = (p.c.analyze_text)(content);
        (p.c.print_analysis_result)(r);
        c_free(content);
    });
    let (ro, re) = p.rust.captured_both(|| {
        let content = (p.rust.read_file)(name.as_ptr() as *const c_char);
        assert!(!content.is_null());
        let r = (p.rust.analyze_text)(content);
        (p.rust.print_analysis_result)(r);
        c_free(content);
    });
    assert_eq!(show(&co), show(&ro));
    assert_eq!(show(&ce), show(&re));
    assert_eq!(
        show(&ce),
        "Error: Input text too large\\nError: Failed to load text\\n"
    );
    let _ = std::fs::remove_file(&path);
}

// ---------------------------------------------------------------------------
// Generic FFI boundaries: embedded NUL bytes and non-UTF-8 paths
// ---------------------------------------------------------------------------

#[test]
fn embedded_nul_truncates_like_strlen() {
    let _g = lock();
    let p = init_both();
    // The C code measures its inputs with strlen/strncpy, so everything from the
    // first NUL on is invisible.
    let cases: Vec<Vec<u8>> = vec![
        b"abc\0def".to_vec(),
        b"\0abc".to_vec(),
        b"a\0".to_vec(),
        b"if (a)\0 else".to_vec(),
        b"\0".to_vec(),
    ];
    for raw in &cases {
        // tokenizer_load_text + full tokenize
        let mut buf = raw.clone();
        buf.push(0);
        let rc = (p.c.tokenizer_load_text)(buf.as_ptr() as *const c_char);
        let rr = (p.rust.tokenizer_load_text)(buf.as_ptr() as *const c_char);
        assert_eq!(rc, rr);
        assert_eq!(p.c.drain_tokens(), p.rust.drain_tokens(), "{}", show(raw));
        assert_eq!(p.c.stats(), p.rust.stats());

        // analyze_text
        let ac = (p.c.analyze_text)(buf.as_ptr() as *const c_char);
        let ar = (p.rust.analyze_text)(buf.as_ptr() as *const c_char);
        assert_eq!(ac, ar, "analyze_text for {}", show(raw));

        // find_patterns with an embedded NUL in the pattern
        let c = p.c.captured(|| (p.c.find_patterns)(buf.as_ptr() as *const c_char));
        let r = p
            .rust
            .captured(|| (p.rust.find_patterns)(buf.as_ptr() as *const c_char));
        assert_eq!(show(&c), show(&r), "find_patterns for {}", show(raw));
    }
}

#[test]
fn non_utf8_filenames() {
    let _g = lock();
    // a path that is not valid UTF-8 must be passed through byte for byte
    diff_read_file_err(b"/tmp/ta_no_such_\xff\xfe_file");
    diff_read_file_err(b"\xff");
    diff_read_file_err(b"/\x80\x81");

    let mut name = std::env::temp_dir().as_os_str().as_encoded_bytes().to_vec();
    name.extend_from_slice(format!("/ta_utf8_{}_", std::process::id()).as_bytes());
    name.extend_from_slice(&[0xff, 0xfe, 0x80]);
    {
        use std::os::unix::ffi::OsStrExt;
        let path = std::path::PathBuf::from(std::ffi::OsStr::from_bytes(&name));
        std::fs::write(&path, b"int weird_name = 1;\n").unwrap();
        let got = diff_read_file_err(&name);
        assert!(!got.2, "the file exists, so read_file must succeed");
        let _ = std::fs::remove_file(&path);
    }
}

// ---------------------------------------------------------------------------
// Out-of-range enum values across the FFI boundary
// ---------------------------------------------------------------------------

static mut STUB_TYPE: i32 = 0;

extern "C" fn one_token_next() -> CToken {
    let mut t = CToken::zeroed();
    let ty = unsafe { STUB_TYPE };
    if ty == i32::MIN {
        t.ttype = TOKEN_EOF;
        return t;
    }
    unsafe { STUB_TYPE = i32::MIN };
    t.ttype = ty;
    t.value[0] = b'q' as c_char;
    t.length = 1;
    t.line = 3;
    t.column = 4;
    t
}

extern "C" fn one_token_reset() {}

extern "C" fn one_token_load(_t: *const c_char) -> i32 {
    0
}

extern "C" fn one_token_stats(l: *mut usize, t: *mut usize, c: *mut usize) {
    unsafe {
        if !l.is_null() {
            *l = 1;
        }
        if !t.is_null() {
            *t = 2;
        }
        if !c.is_null() {
            *c = 3;
        }
    }
}

#[test]
fn enum_out_of_range_via_custom_ops() {
    let _g = lock();
    let p = libs();
    let ops = COps {
        next_token: Some(one_token_next),
        peek_token: Some(one_token_next),
        reset: Some(one_token_reset),
        load_text: Some(one_token_load),
        get_stats: Some(one_token_stats),
    };

    // token_type_counts[] has 20 entries: 12..=19 are outside token_type_t but
    // still inside the array the C code indexes, so the behaviour is defined.
    for ty in 1..20i32 {
        (p.c.analyzer_init)(ops);
        (p.rust.analyzer_init)(ops);

        unsafe { STUB_TYPE = ty };
        let rc = p.c.analyze(b"x");
        unsafe { STUB_TYPE = ty };
        let rr = p.rust.analyze(b"x");
        assert_eq!(rc, rr, "analyze_text differs for token type {}", ty);

        let c = p.c.captured(|| (p.c.print_token_distribution)());
        let r = p.rust.captured(|| (p.rust.print_token_distribution)());
        assert_eq!(show(&c), show(&r), "distribution differs for type {}", ty);

        assert_eq!(
            (p.c.calculate_complexity_score)(),
            (p.rust.calculate_complexity_score)(),
            "score differs for type {}",
            ty
        );

        unsafe { STUB_TYPE = ty };
        let c = p.c.find(b"q");
        unsafe { STUB_TYPE = ty };
        let r = p.rust.find(b"q");
        assert_eq!(show(&c), show(&r), "find_patterns differs for type {}", ty);
    }

    init_both();
}
