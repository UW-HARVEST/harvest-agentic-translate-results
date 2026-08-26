//! Phase B — valid-path differential tests for the lowest-level entry points
//! (`CONFIGS.md` rows C1-C21).
//!
//! Every operation is applied to the C `.so` and the Rust `.so` in the same
//! order so that their (never resettable) cumulative statistics stay in
//! lock-step; only C-vs-Rust equality is asserted.

mod common;

use common::*;

/// `load_text` + full tokenize + `get_stats`, on both libraries.
fn diff_tokenize(text: &[u8]) {
    let p = libs();
    let rc = p.c.load_text(text);
    let rr = p.rust.load_text(text);
    assert_eq!(rc, rr, "load_text rc differs for {}", show(text));

    let tc = p.c.drain_tokens();
    let tr = p.rust.drain_tokens();
    assert_eq!(
        tc.len(),
        tr.len(),
        "token count differs for {}\n C: {:?}\n R: {:?}",
        show(text),
        tc,
        tr
    );
    for (i, (a, b)) in tc.iter().zip(tr.iter()).enumerate() {
        assert_eq!(a, b, "token #{} differs for text {}", i, show(text));
    }

    assert_eq!(
        p.c.stats(),
        p.rust.stats(),
        "stats differ after tokenizing {}",
        show(text)
    );
}

fn diff_all(texts: &[Vec<u8>]) {
    for t in texts {
        diff_tokenize(t);
    }
}

// ---------------------------------------------------------------------------
// C1, C2: empty text and every single byte
// ---------------------------------------------------------------------------

#[test]
fn c1_empty_text() {
    let _g = lock();
    diff_tokenize(b"");
    // ... and immediately again (position already at the end)
    diff_tokenize(b"");
}

#[test]
fn c2_every_single_byte() {
    let _g = lock();
    for b in 1u8..=255 {
        diff_tokenize(&[b]);
    }
}

#[test]
fn c2b_every_byte_pair_of_interesting_alphabet() {
    let _g = lock();
    let alphabet = interesting_bytes();
    for &a in &alphabet {
        for &b in &alphabet {
            diff_tokenize(&[a, b]);
        }
    }
}

#[test]
fn c2c_every_byte_followed_by_letter_and_digit() {
    let _g = lock();
    for b in 1u8..=255 {
        diff_tokenize(&[b, b'a']);
        diff_tokenize(&[b, b'1']);
        diff_tokenize(&[b'a', b]);
        diff_tokenize(&[b'1', b]);
    }
}

// ---------------------------------------------------------------------------
// C3: keywords vs identifiers
// ---------------------------------------------------------------------------

#[test]
fn c3_keywords_and_lookalikes() {
    let _g = lock();
    let mut texts: Vec<Vec<u8>> = Vec::new();
    for k in KEYWORDS.iter() {
        texts.push(k.as_bytes().to_vec());
        texts.push(format!("{} ", k).into_bytes());
        texts.push(format!(" {}", k).into_bytes());
        texts.push(format!("{}x", k).into_bytes());
        texts.push(format!("x{}", k).into_bytes());
        texts.push(format!("_{}", k).into_bytes());
        texts.push(format!("{}_", k).into_bytes());
        texts.push(format!("{}1", k).into_bytes());
        texts.push(k.to_uppercase().into_bytes());
        texts.push(format!("{} {} {}", k, k, k).into_bytes());
        // a prefix of the keyword
        if k.len() > 1 {
            texts.push(k[..k.len() - 1].as_bytes().to_vec());
        }
    }
    texts.push(b"if(x==1){return 0;}else{}".to_vec());
    diff_all(&texts);
}

// ---------------------------------------------------------------------------
// C4: identifiers
// ---------------------------------------------------------------------------

#[test]
fn c4_identifiers() {
    let _g = lock();
    let texts: Vec<Vec<u8>> = vec![
        b"a".to_vec(),
        b"_".to_vec(),
        b"__".to_vec(),
        b"_1".to_vec(),
        b"a1".to_vec(),
        b"a_b_c".to_vec(),
        b"A9z_".to_vec(),
        b"abc def".to_vec(),
        b"abc\tdef".to_vec(),
        b"abc(def)".to_vec(),
        b"x".to_vec(),
        b"xyz".to_vec(),
        b"_leading".to_vec(),
        b"trailing_".to_vec(),
        b"MiXeD_CaSe_123".to_vec(),
    ];
    diff_all(&texts);
}

// ---------------------------------------------------------------------------
// C5: numbers
// ---------------------------------------------------------------------------

#[test]
fn c5_numbers() {
    let _g = lock();
    let texts: Vec<Vec<u8>> = vec![
        b"0".to_vec(),
        b"9".to_vec(),
        b"123".to_vec(),
        b"007".to_vec(),
        b"1.5".to_vec(),
        b"1.".to_vec(),
        b".5".to_vec(),
        b"..5".to_vec(),
        b"1.2.3".to_vec(),
        b"1..2".to_vec(),
        b"12ab".to_vec(),
        b"1a2".to_vec(),
        b"0.0.0.0".to_vec(),
        b"3.14159 2.71828".to_vec(),
        b"1 2 3".to_vec(),
        b"1+2".to_vec(),
        b"1,2".to_vec(),
    ];
    diff_all(&texts);
}

// ---------------------------------------------------------------------------
// C6: strings
// ---------------------------------------------------------------------------

#[test]
fn c6_strings() {
    let _g = lock();
    let texts: Vec<Vec<u8>> = vec![
        b"\"\"".to_vec(),
        b"\"a\"".to_vec(),
        b"''".to_vec(),
        b"'x'".to_vec(),
        b"\"'\"".to_vec(),
        b"'\"'".to_vec(),
        b"\"a b\tc\"".to_vec(),
        b"\"a\\\"b\"".to_vec(),
        b"\"a\\\\\"".to_vec(),
        b"\"a\\".to_vec(),
        b"\"a\\\n\"".to_vec(),
        b"\"abc".to_vec(),
        b"'abc".to_vec(),
        b"\"abc\ndef\"".to_vec(),
        b"\"\\\"".to_vec(),
        b"\"\\0\"".to_vec(),
        b"\"a\"\"b\"".to_vec(),
        b"'a''b'".to_vec(),
        b"\"a'b\"c".to_vec(),
        b"\\\"a\"".to_vec(),
    ];
    diff_all(&texts);
}

// ---------------------------------------------------------------------------
// C7: comments (note: a lone '/' also enters scan_comment)
// ---------------------------------------------------------------------------

#[test]
fn c7_comments() {
    let _g = lock();
    let texts: Vec<Vec<u8>> = vec![
        b"/".to_vec(),
        b"//".to_vec(),
        b"// text".to_vec(),
        b"// text\nx".to_vec(),
        b"//\n".to_vec(),
        b"///".to_vec(),
        b"/**/".to_vec(),
        b"/* */".to_vec(),
        b"/* a */b".to_vec(),
        b"/*".to_vec(),
        b"/**".to_vec(),
        b"/***/".to_vec(),
        b"/* *".to_vec(),
        b"/* * /".to_vec(),
        b"/*/".to_vec(),
        b"/*/*/".to_vec(),
        b"/* a\nb */c".to_vec(),
        b"a / b".to_vec(),
        b"a/=b".to_vec(),
        b"a//b".to_vec(),
        b"x=y/z".to_vec(),
        b"//*".to_vec(),
        b"/ /".to_vec(),
    ];
    diff_all(&texts);
}

// ---------------------------------------------------------------------------
// C8: operators
// ---------------------------------------------------------------------------

#[test]
fn c8_operators() {
    let _g = lock();
    let mut texts: Vec<Vec<u8>> = Vec::new();
    for &c in OPERATOR_CHARS {
        texts.push(vec![c]);
        for &d in OPERATOR_CHARS {
            texts.push(vec![c, d]);
            texts.push(vec![c, d, b'a']);
        }
        texts.push(vec![c, c, c]);
        texts.push(vec![b'a', c, b'b']);
    }
    for op in TWO_CHAR_OPS.iter() {
        texts.push(op.as_bytes().to_vec());
        texts.push(format!("a{}b", op).into_bytes());
        texts.push(format!("{}{}", op, op).into_bytes());
        texts.push(format!("{}=", op).into_bytes());
    }
    diff_all(&texts);
}

// ---------------------------------------------------------------------------
// C9: whitespace
// ---------------------------------------------------------------------------

#[test]
fn c9_whitespace() {
    let _g = lock();
    let texts: Vec<Vec<u8>> = vec![
        b" ".to_vec(),
        b"  ".to_vec(),
        b"\t".to_vec(),
        b"\x0b".to_vec(),
        b"\x0c".to_vec(),
        b"\r".to_vec(),
        b" \t\x0b\x0c\r".to_vec(),
        b"  a  ".to_vec(),
        b"\t\ta\t\t".to_vec(),
        b"a \t b".to_vec(),
        b"\r\n".to_vec(),
        b"a\r\nb".to_vec(),
        b" \n ".to_vec(),
        b"\n \n".to_vec(),
        b"          x".to_vec(),
    ];
    diff_all(&texts);
}

// ---------------------------------------------------------------------------
// C10: multi-line input
// ---------------------------------------------------------------------------

#[test]
fn c10_multiline() {
    let _g = lock();
    let mut texts: Vec<Vec<u8>> = vec![
        b"\n".to_vec(),
        b"\n\n".to_vec(),
        b"\n\n\n".to_vec(),
        b"a\n".to_vec(),
        b"\na".to_vec(),
        b"a\nb".to_vec(),
        b"a\n\nb".to_vec(),
        b"a\r\nb\r\n".to_vec(),
        b"if\nelse\nwhile\n".to_vec(),
        b"1\n2\n3\n4\n5".to_vec(),
    ];
    let mut many = Vec::new();
    for i in 0..200 {
        many.extend_from_slice(format!("line{} = {};\n", i, i).as_bytes());
    }
    texts.push(many);
    let mut blank = Vec::new();
    for _ in 0..300 {
        blank.push(b'\n');
    }
    texts.push(blank);
    diff_all(&texts);
}

// ---------------------------------------------------------------------------
// C11: token-length boundaries
// ---------------------------------------------------------------------------

fn rep(c: u8, n: usize) -> Vec<u8> {
    vec![c; n]
}

#[test]
fn c11_token_length_boundaries() {
    let _g = lock();
    let mut texts: Vec<Vec<u8>> = Vec::new();
    for n in [1usize, 2, 253, 254, 255, 256, 257, 300, 511, 512, 513] {
        // identifiers
        texts.push(rep(b'a', n));
        let mut t = rep(b'a', n);
        t.extend_from_slice(b" z");
        texts.push(t);
        // numbers
        texts.push(rep(b'7', n));
        // identifiers made of digits and letters
        let mut t: Vec<u8> = b"x".to_vec();
        t.extend_from_slice(&rep(b'9', n));
        texts.push(t);
        // // comment
        let mut t: Vec<u8> = b"//".to_vec();
        t.extend_from_slice(&rep(b'c', n));
        texts.push(t);
        let mut t: Vec<u8> = b"//".to_vec();
        t.extend_from_slice(&rep(b'c', n));
        t.extend_from_slice(b"\nafter");
        texts.push(t);
        // /* comment */
        let mut t: Vec<u8> = b"/*".to_vec();
        t.extend_from_slice(&rep(b'c', n));
        t.extend_from_slice(b"*/after");
        texts.push(t);
        let mut t: Vec<u8> = b"/*".to_vec();
        t.extend_from_slice(&rep(b'c', n));
        texts.push(t);
        // strings, terminated and not
        let mut t: Vec<u8> = b"\"".to_vec();
        t.extend_from_slice(&rep(b's', n));
        t.extend_from_slice(b"\"tail");
        texts.push(t);
        let mut t: Vec<u8> = b"\"".to_vec();
        t.extend_from_slice(&rep(b's', n));
        texts.push(t);
        // string full of escapes (two buffer slots per iteration)
        let mut t: Vec<u8> = b"'".to_vec();
        for _ in 0..n {
            t.extend_from_slice(b"\\x");
        }
        t.extend_from_slice(b"'rest");
        texts.push(t);
        // string whose escape lands exactly on the bound
        let mut t: Vec<u8> = b"\"".to_vec();
        t.extend_from_slice(&rep(b'q', n));
        t.extend_from_slice(b"\\\"\"");
        texts.push(t);
    }
    diff_all(&texts);
}

// ---------------------------------------------------------------------------
// C12: buffer-length boundaries
// ---------------------------------------------------------------------------

/// `create_token` computes `column = current_column - token.length` in
/// `size_t` arithmetic and stores it in an `int`: a multi-line comment resets
/// `current_column` in the middle of the token, so the column goes negative.
#[test]
fn c11b_negative_columns_from_multiline_comments() {
    let _g = lock();
    let p = libs();
    let mut texts: Vec<Vec<u8>> = Vec::new();
    for lead in [0usize, 1, 5] {
        for body in [1usize, 10, 100, 250, 251, 252, 253, 254, 300] {
            let mut t: Vec<u8> = vec![b' '; lead];
            t.extend_from_slice(b"/*\n");
            t.extend_from_slice(&vec![b'c'; body]);
            t.extend_from_slice(b"*/after");
            texts.push(t);
            // newline later inside the comment
            let mut t: Vec<u8> = vec![b' '; lead];
            t.extend_from_slice(b"/*");
            t.extend_from_slice(&vec![b'c'; body]);
            t.push(b'\n');
            t.extend_from_slice(&vec![b'd'; body]);
            t.extend_from_slice(b"*/after");
            texts.push(t);
            // several newlines
            let mut t: Vec<u8> = b"/*".to_vec();
            for _ in 0..5 {
                t.extend_from_slice(&vec![b'e'; body / 5 + 1]);
                t.push(b'\n');
            }
            t.extend_from_slice(b"*/x");
            texts.push(t);
        }
    }
    let mut saw_negative = false;
    for t in &texts {
        assert_eq!(p.c.load_text(t), p.rust.load_text(t));
        let tc = p.c.drain_tokens();
        assert_eq!(tc, p.rust.drain_tokens(), "text {}", show(t));
        if tc.iter().any(|tok| tok.column < 0) {
            saw_negative = true;
        }
        assert_eq!(p.c.stats(), p.rust.stats());
    }
    assert!(
        saw_negative,
        "expected at least one negative column in this corpus"
    );
}

#[test]
fn c12_buffer_length_boundaries() {
    let _g = lock();
    for n in [1usize, 2, 4095, 4096, 8189, 8190, 8191] {
        // one long identifier
        diff_tokenize(&rep(b'a', n));
        // many short tokens
        let mut t = Vec::with_capacity(n);
        while t.len() + 2 <= n {
            t.extend_from_slice(b"a ");
        }
        while t.len() < n {
            t.push(b'b');
        }
        diff_tokenize(&t);
        // newline heavy
        let mut t = Vec::with_capacity(n);
        while t.len() < n {
            t.push(if t.len() % 3 == 0 { b'\n' } else { b'x' });
        }
        diff_tokenize(&t);
    }
}

// ---------------------------------------------------------------------------
// C13: high bytes (signed char passed to ctype functions)
// ---------------------------------------------------------------------------

#[test]
fn c13_high_bytes() {
    let _g = lock();
    let mut texts: Vec<Vec<u8>> = Vec::new();
    for b in 0x80u8..=0xff {
        texts.push(vec![b]);
        texts.push(vec![b, b'a']);
        texts.push(vec![b'a', b, b'b']);
        texts.push(vec![b, b'\n', b]);
        texts.push(vec![b'"', b, b'"']);
    }
    // UTF-8 text
    texts.push("héllo wörld — ünïcode".as_bytes().to_vec());
    texts.push("日本語のテキスト".as_bytes().to_vec());
    diff_all(&texts);
}

// ---------------------------------------------------------------------------
// C14, C15: randomized inputs
// ---------------------------------------------------------------------------

#[test]
fn c14_random_byte_soup() {
    let _g = lock();
    let mut rng = Rng::new(0xC14);
    for _ in 0..200 {
        let t = random_soup(&mut rng, 600);
        diff_tokenize(&t);
    }
}

#[test]
fn c15_random_c_like_source() {
    let _g = lock();
    let mut rng = Rng::new(0xC15);
    for _ in 0..200 {
        let t = random_source(&mut rng, 60);
        diff_tokenize(&t);
    }
}

#[test]
fn c15b_random_long_source() {
    let _g = lock();
    let mut rng = Rng::new(0xC15B);
    for _ in 0..25 {
        let mut t = Vec::new();
        while t.len() < 6000 {
            t.extend_from_slice(&random_source(&mut rng, 40));
        }
        t.truncate(8191);
        diff_tokenize(&t);
    }
}

// ---------------------------------------------------------------------------
// C16: peek/next interleavings
// ---------------------------------------------------------------------------

#[test]
fn c16_peek_next_interleavings() {
    let _g = lock();
    let p = libs();
    let mut rng = Rng::new(0xC16);

    for round in 0..40 {
        let text = if round % 2 == 0 {
            random_source(&mut rng, 30)
        } else {
            random_soup(&mut rng, 120)
        };
        assert_eq!(p.c.load_text(&text), p.rust.load_text(&text));

        for step in 0..400 {
            let op = rng.below(10);
            match op {
                0..=4 => {
                    let a = p.c.next();
                    let b = p.rust.next();
                    assert_eq!(a, b, "next differs at step {} for {}", step, show(&text));
                }
                5..=8 => {
                    let a = p.c.peek();
                    let b = p.rust.peek();
                    assert_eq!(a, b, "peek differs at step {} for {}", step, show(&text));
                }
                _ => {
                    let a = p.c.stats();
                    let b = p.rust.stats();
                    assert_eq!(a, b, "stats differ at step {} for {}", step, show(&text));
                }
            }
        }
        assert_eq!(p.c.stats(), p.rust.stats());
    }
}

#[test]
fn c16b_peek_is_idempotent_and_consumed_by_next() {
    let _g = lock();
    let p = libs();
    let text = b"int main(void) { return 42; }";
    assert_eq!(p.c.load_text(text), p.rust.load_text(text));
    for _ in 0..12 {
        // three peeks in a row, then a next
        for _ in 0..3 {
            assert_eq!(p.c.peek(), p.rust.peek());
        }
        assert_eq!(p.c.next(), p.rust.next());
        assert_eq!(p.c.stats(), p.rust.stats());
    }
}

// ---------------------------------------------------------------------------
// C17: reset
// ---------------------------------------------------------------------------

#[test]
fn c17_reset() {
    let _g = lock();
    let p = libs();

    // reset before any load
    (p.c.tokenizer_reset)();
    (p.rust.tokenizer_reset)();
    assert_eq!(p.c.stats(), p.rust.stats());

    let text = b"if (a >= 10) { /* c */ return \"s\"; } // tail\n";
    assert_eq!(p.c.load_text(text), p.rust.load_text(text));

    // reset mid-stream
    for _ in 0..3 {
        assert_eq!(p.c.next(), p.rust.next());
    }
    (p.c.tokenizer_reset)();
    (p.rust.tokenizer_reset)();
    assert_eq!(p.c.drain_tokens(), p.rust.drain_tokens());
    assert_eq!(p.c.stats(), p.rust.stats());

    // reset at EOF, twice
    (p.c.tokenizer_reset)();
    (p.rust.tokenizer_reset)();
    (p.c.tokenizer_reset)();
    (p.rust.tokenizer_reset)();
    assert_eq!(p.c.drain_tokens(), p.rust.drain_tokens());

    // reset discards the lookahead
    (p.c.tokenizer_reset)();
    (p.rust.tokenizer_reset)();
    assert_eq!(p.c.peek(), p.rust.peek());
    (p.c.tokenizer_reset)();
    (p.rust.tokenizer_reset)();
    assert_eq!(p.c.next(), p.rust.next());
    assert_eq!(p.c.stats(), p.rust.stats());
}

// ---------------------------------------------------------------------------
// C18: get_stats out-parameter combinations after every step
// ---------------------------------------------------------------------------

#[test]
fn c18_stats_scripts() {
    let _g = lock();
    let p = libs();
    let mut rng = Rng::new(0xC18);

    for _ in 0..30 {
        let text = random_source(&mut rng, 25);
        assert_eq!(p.c.load_text(&text), p.rust.load_text(&text));
        for _ in 0..60 {
            match rng.below(4) {
                0 => assert_eq!(p.c.next(), p.rust.next()),
                1 => assert_eq!(p.c.peek(), p.rust.peek()),
                2 => {
                    (p.c.tokenizer_reset)();
                    (p.rust.tokenizer_reset)();
                }
                _ => {
                    let t2 = random_source(&mut rng, 10);
                    assert_eq!(p.c.load_text(&t2), p.rust.load_text(&t2));
                }
            }
            // every combination of NULL out-parameters
            for mask in 0..8u32 {
                let mut lc = [0usize; 3];
                let mut lr = [0usize; 3];
                let pc: [*mut usize; 3] = [
                    if mask & 1 != 0 { &mut lc[0] } else { std::ptr::null_mut() },
                    if mask & 2 != 0 { &mut lc[1] } else { std::ptr::null_mut() },
                    if mask & 4 != 0 { &mut lc[2] } else { std::ptr::null_mut() },
                ];
                let pr: [*mut usize; 3] = [
                    if mask & 1 != 0 { &mut lr[0] } else { std::ptr::null_mut() },
                    if mask & 2 != 0 { &mut lr[1] } else { std::ptr::null_mut() },
                    if mask & 4 != 0 { &mut lr[2] } else { std::ptr::null_mut() },
                ];
                (p.c.tokenizer_get_stats)(pc[0], pc[1], pc[2]);
                (p.rust.tokenizer_get_stats)(pr[0], pr[1], pr[2]);
                assert_eq!(lc, lr, "get_stats mask {} differs", mask);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C19: repeated load_text
// ---------------------------------------------------------------------------

#[test]
fn c19_repeated_loads() {
    let _g = lock();
    let p = libs();

    let seq: Vec<Vec<u8>> = vec![
        b"a_very_long_first_text with several tokens 123".to_vec(),
        b"short".to_vec(),
        b"".to_vec(),
        b"x".to_vec(),
        b"".to_vec(),
        b"even longer text than the first one, with \"strings\" and /* comments */"
            .to_vec(),
        b"tiny".to_vec(),
    ];
    for text in &seq {
        assert_eq!(p.c.load_text(text), p.rust.load_text(text));
        assert_eq!(
            p.c.drain_tokens(),
            p.rust.drain_tokens(),
            "tokens differ for {}",
            show(text)
        );
        assert_eq!(p.c.stats(), p.rust.stats());
    }

    // load without draining, several times in a row
    for text in &seq {
        assert_eq!(p.c.load_text(text), p.rust.load_text(text));
        assert_eq!(p.c.next(), p.rust.next());
    }
    assert_eq!(p.c.stats(), p.rust.stats());
}

// ---------------------------------------------------------------------------
// C20: reading past EOF
// ---------------------------------------------------------------------------

#[test]
fn c20_past_eof() {
    let _g = lock();
    let p = libs();
    for text in [&b""[..], &b"a"[..], &b"a b"[..], &b"\n"[..], &b"  "[..]] {
        assert_eq!(p.c.load_text(text), p.rust.load_text(text));
        assert_eq!(p.c.drain_tokens(), p.rust.drain_tokens());
        for i in 0..5 {
            assert_eq!(p.c.next(), p.rust.next(), "extra next #{}", i);
            assert_eq!(p.c.peek(), p.rust.peek(), "extra peek #{}", i);
            assert_eq!(p.c.stats(), p.rust.stats(), "stats after extra call #{}", i);
        }
    }
}

// ---------------------------------------------------------------------------
// C21: drive everything through the function pointers of get_tokenizer_ops
// ---------------------------------------------------------------------------

#[test]
fn c21_through_ops_pointers() {
    let _g = lock();
    let p = libs();
    let ops_c = (p.c.get_tokenizer_ops)();
    let ops_r = (p.rust.get_tokenizer_ops)();
    let mut rng = Rng::new(0xC21);

    for _ in 0..40 {
        let text = random_source(&mut rng, 30);
        let s = cstring(&text);
        let rc = (ops_c.load_text.unwrap())(s.as_ptr() as *const std::ffi::c_char);
        let rr = (ops_r.load_text.unwrap())(s.as_ptr() as *const std::ffi::c_char);
        assert_eq!(rc, rr);

        loop {
            let a = (ops_c.next_token.unwrap())().view();
            let b = (ops_r.next_token.unwrap())().view();
            assert_eq!(a, b, "ops next_token differs for {}", show(&text));
            if a.ttype == TOKEN_EOF {
                break;
            }
            if rng.chance(4) {
                assert_eq!(
                    (ops_c.peek_token.unwrap())().view(),
                    (ops_r.peek_token.unwrap())().view()
                );
            }
        }

        (ops_c.reset.unwrap())();
        (ops_r.reset.unwrap())();

        let mut lc = [0usize; 3];
        let mut lr = [0usize; 3];
        (ops_c.get_stats.unwrap())(&mut lc[0], &mut lc[1], &mut lc[2]);
        (ops_r.get_stats.unwrap())(&mut lr[0], &mut lr[1], &mut lr[2]);
        assert_eq!(lc, lr, "ops get_stats differs");
    }
}
