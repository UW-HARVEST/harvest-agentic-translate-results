//! Phase B — valid-path differential tests for the analyzer entry points
//! (`CONFIGS.md` rows C22-C25 and C27-C33).
//!
//! Each library is always driven through *its own* `tokenizer_ops_t`, so both
//! stay in lock-step; the cross-library dispatch of row C26 lives in
//! `tests/cross_ops.rs` (its own process).

mod common;

use common::*;
use std::ffi::c_char;

fn init_both() -> &'static Pair {
    let p = libs();
    (p.c.analyzer_init)((p.c.get_tokenizer_ops)());
    (p.rust.analyzer_init)((p.rust.get_tokenizer_ops)());
    p
}

fn diff_analyze(text: &[u8]) -> CResult {
    let p = libs();
    let rc = p.c.analyze(text);
    let rr = p.rust.analyze(text);
    assert_eq!(rc, rr, "analyze_text differs for {}", show(text));
    assert_eq!(
        p.c.stats(),
        p.rust.stats(),
        "stats differ after analyze_text {}",
        show(text)
    );
    rc
}

fn diff_distribution() -> Vec<u8> {
    let p = libs();
    let c = p.c.captured(|| (p.c.print_token_distribution)());
    let r = p.rust.captured(|| (p.rust.print_token_distribution)());
    assert_eq!(show(&c), show(&r), "print_token_distribution differs");
    c
}

fn diff_score() -> i32 {
    let p = libs();
    let c = (p.c.calculate_complexity_score)();
    let r = (p.rust.calculate_complexity_score)();
    assert_eq!(c, r, "calculate_complexity_score differs");
    c
}

fn diff_find(pattern: &[u8]) -> Vec<u8> {
    let p = libs();
    let c = p.c.find(pattern);
    let r = p.rust.find(pattern);
    assert_eq!(
        show(&c),
        show(&r),
        "find_patterns differs for pattern {}",
        show(pattern)
    );
    assert_eq!(p.c.stats(), p.rust.stats(), "stats differ after find_patterns");
    c
}

// ---------------------------------------------------------------------------
// C22: one token of every class
// ---------------------------------------------------------------------------

#[test]
fn c22_single_token_classes() {
    let _g = lock();
    init_both();
    let texts: Vec<&[u8]> = vec![
        b"",
        b"word",
        b"if",
        b"123",
        b"\"str\"",
        b"'c'",
        b"// comment",
        b"/* comment */",
        b"+",
        b"==",
        b";",
        b"#",
        b"\n",
        b" ",
        b"/",
        b"_",
        b"1.5",
    ];
    for t in texts {
        init_both();
        let r = diff_analyze(t);
        println!("{:>16} -> {:?}", show(t), r);
        diff_distribution();
        diff_score();
    }
}

// ---------------------------------------------------------------------------
// C23: randomized C-like text
// ---------------------------------------------------------------------------

#[test]
fn c23_random_source() {
    let _g = lock();
    let mut rng = Rng::new(0xC23);
    for i in 0..200 {
        if i % 7 == 0 {
            init_both();
        }
        let text = random_source(&mut rng, 50);
        diff_analyze(&text);
        if i % 11 == 0 {
            diff_distribution();
            diff_score();
        }
    }
}

#[test]
fn c23b_random_byte_soup() {
    let _g = lock();
    let mut rng = Rng::new(0xC23B);
    for i in 0..150 {
        if i % 5 == 0 {
            init_both();
        }
        let text = random_soup(&mut rng, 400);
        diff_analyze(&text);
        diff_score();
    }
    diff_distribution();
}

// ---------------------------------------------------------------------------
// C24: repeated analysis without re-init (accumulating state)
// ---------------------------------------------------------------------------

#[test]
fn c24_repeated_analysis_accumulates() {
    let _g = lock();
    init_both();
    let mut rng = Rng::new(0xC24);
    for round in 0..25 {
        let text = random_source(&mut rng, 20);
        diff_analyze(&text);
        diff_score();
        if round % 5 == 0 {
            diff_distribution();
        }
    }
    diff_distribution();
    // the same text over and over: common_word_counts must rise identically
    for _ in 0..5 {
        diff_analyze(b"alpha beta alpha gamma alpha beta");
    }
    diff_distribution();
}

// ---------------------------------------------------------------------------
// C25: analyzer_init twice
// ---------------------------------------------------------------------------

#[test]
fn c25_reinit_resets_accumulators() {
    let _g = lock();
    init_both();
    diff_analyze(b"int a; int b; int c; // x\n");
    diff_distribution();
    diff_score();

    init_both();
    // everything must be back to zero (and identical on both sides)
    let out = diff_distribution();
    assert!(
        !out.contains(&b'.'),
        "expected no word lines after re-init: {}",
        show(&out)
    );
    assert_eq!(diff_score(), 0);

    diff_analyze(b"a b c");
    diff_distribution();
    init_both();
    diff_distribution();
}

// ---------------------------------------------------------------------------
// C27: analyzer driven by a stub tokenizer_ops_t
// ---------------------------------------------------------------------------

/// A script of tokens handed to both analyzers through a stub `tokenizer_ops_t`.
static mut SCRIPT: Vec<CToken> = Vec::new();
static mut CURSOR: usize = 0;
static mut STUB_LOAD_RC: i32 = 0;
static mut STUB_STATS: (usize, usize, usize) = (0, 0, 0);

fn set_script(tokens: Vec<CToken>, load_rc: i32, stats: (usize, usize, usize)) {
    unsafe {
        SCRIPT = tokens;
        CURSOR = 0;
        STUB_LOAD_RC = load_rc;
        STUB_STATS = stats;
    }
}

fn rewind_script() {
    unsafe { CURSOR = 0 }
}

fn script_at(i: usize) -> CToken {
    let script: &Vec<CToken> = unsafe { &*std::ptr::addr_of!(SCRIPT) };
    if i < script.len() {
        script[i]
    } else {
        let mut t = CToken::zeroed();
        t.ttype = TOKEN_EOF;
        t
    }
}

extern "C" fn stub_next() -> CToken {
    let i = unsafe { CURSOR };
    unsafe { CURSOR += 1 };
    script_at(i)
}

extern "C" fn stub_peek() -> CToken {
    script_at(unsafe { CURSOR })
}

extern "C" fn stub_reset() {
    rewind_script();
}

extern "C" fn stub_load(_text: *const c_char) -> i32 {
    rewind_script();
    unsafe { STUB_LOAD_RC }
}

extern "C" fn stub_get_stats(lines: *mut usize, tokens: *mut usize, chars: *mut usize) {
    unsafe {
        if !lines.is_null() {
            *lines = STUB_STATS.0;
        }
        if !tokens.is_null() {
            *tokens = STUB_STATS.1;
        }
        if !chars.is_null() {
            *chars = STUB_STATS.2;
        }
    }
}

fn stub_ops() -> COps {
    COps {
        next_token: Some(stub_next),
        peek_token: Some(stub_peek),
        reset: Some(stub_reset),
        load_text: Some(stub_load),
        get_stats: Some(stub_get_stats),
    }
}

fn tok(ttype: i32, value: &[u8], line: i32, column: i32) -> CToken {
    let mut t = CToken::zeroed();
    t.ttype = ttype;
    let n = value.len().min(MAX_TOKEN_LENGTH - 1);
    for i in 0..n {
        t.value[i] = value[i] as c_char;
    }
    t.length = value.len();
    t.line = line;
    t.column = column;
    t
}

#[test]
fn c27_stub_ops_token_scripts() {
    let _g = lock();
    let p = libs();
    let mut rng = Rng::new(0xC27);

    let words: [&[u8]; 6] = [b"alpha", b"beta", b"gamma", b"", b"x", b"averylongword"];

    for round in 0..40 {
        // a random token script (types 0..19 - 12..19 are outside the enum but
        // inside the 20-element counter array the C code indexes)
        let mut script = Vec::new();
        let n = rng.below(30);
        for i in 0..n {
            let ty = match rng.below(10) {
                0..=7 => rng.below(12) as i32,
                _ => rng.range(12, 19) as i32,
            };
            if ty == TOKEN_EOF {
                continue;
            }
            script.push(tok(
                ty,
                words[rng.below(words.len())],
                (i % 7) as i32 + 1,
                (i % 13) as i32,
            ));
        }
        let stats = (
            rng.below(1000),
            rng.below(1000),
            rng.below(1000),
        );
        set_script(script, 0, stats);

        (p.c.analyzer_init)(stub_ops());
        (p.rust.analyzer_init)(stub_ops());

        for _ in 0..(1 + round % 3) {
            rewind_script();
            let rc = p.c.analyze(b"ignored");
            rewind_script();
            let rr = p.rust.analyze(b"ignored");
            assert_eq!(rc, rr, "stub analyze_text differs (round {})", round);
        }

        let c = p.c.captured(|| (p.c.print_token_distribution)());
        let r = p.rust.captured(|| (p.rust.print_token_distribution)());
        assert_eq!(show(&c), show(&r), "stub distribution differs");

        assert_eq!(
            (p.c.calculate_complexity_score)(),
            (p.rust.calculate_complexity_score)(),
            "stub score differs"
        );

        rewind_script();
        let c = p.c.captured(|| {
            let s = cstring(b"a");
            (p.c.find_patterns)(s.as_ptr() as *const c_char)
        });
        rewind_script();
        let r = p.rust.captured(|| {
            let s = cstring(b"a");
            (p.rust.find_patterns)(s.as_ptr() as *const c_char)
        });
        assert_eq!(show(&c), show(&r), "stub find_patterns differs");
    }

    // restore the real ops for whatever runs next
    init_both();
}

// ---------------------------------------------------------------------------
// C28: complexity score mixes
// ---------------------------------------------------------------------------

#[test]
fn c28_complexity_scores() {
    let _g = lock();
    let cases: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"word".to_vec(),
        b"if".to_vec(),
        b"if if if".to_vec(),
        b"+ - * %".to_vec(),
        b"();".to_vec(),
        b"(((((((((".to_vec(),                 // 9 punctuation -> 9/10 == 0
        b"((((((((((".to_vec(),                // 10 punctuation -> 1
        b"(((((((((((((((((((((((((".to_vec(), // 25 -> 2
        b"// c".to_vec(),                      // score -1 -> clamped to 0
        b"// a\n// b\n// c\n".to_vec(),        // -3 -> 0
        b"if // c".to_vec(),                   // 2-1 = 1
        b"if + // c\n// d".to_vec(),
        b"int a = 1; if (a == 2) { return a++; } // done\n".to_vec(),
    ];
    for c in &cases {
        init_both();
        diff_analyze(c);
        let s = diff_score();
        println!("{:>20} -> score {}", show(c), s);
    }

    // accumulating across several analyses without re-init
    init_both();
    for c in &cases {
        diff_analyze(c);
        diff_score();
    }
}

// ---------------------------------------------------------------------------
// C29: token distribution / most-common-words printing
// ---------------------------------------------------------------------------

fn distinct_words(n: usize) -> Vec<u8> {
    let mut v = Vec::new();
    for i in 0..n {
        v.extend_from_slice(format!("w{} ", i).as_bytes());
    }
    v
}

#[test]
fn c29_distribution_word_counts() {
    let _g = lock();
    for n in [0usize, 1, 2, 9, 10, 11, 12, 99, 100, 101, 150] {
        init_both();
        diff_analyze(&distinct_words(n));
        let out = diff_distribution();
        println!("n={} -> {} bytes", n, out.len());
        // printed twice: the bubble sort mutates the arrays in place
        diff_distribution();
    }

    // ties and repeats (bubble-sort order for equal counts)
    init_both();
    diff_analyze(b"a b c a b a d e d");
    diff_distribution();
    diff_distribution();

    init_both();
    diff_analyze(b"z y x w v u t s r q p o n m l k j i h g f e d c b a");
    diff_distribution();

    // many repeats of a few words
    init_both();
    let mut text = Vec::new();
    for i in 0..300 {
        text.extend_from_slice(format!("w{} ", i % 17).as_bytes());
    }
    diff_analyze(&text);
    diff_distribution();

    // long words (truncated at MAX_TOKEN_LENGTH-1 by track_word)
    init_both();
    let mut text = Vec::new();
    for n in [250usize, 254, 255, 256, 260, 300] {
        text.extend_from_slice(&vec![b'q'; n]);
        text.push(b' ');
    }
    diff_analyze(&text);
    diff_distribution();
}

// ---------------------------------------------------------------------------
// C30, C31, C32: find_patterns
// ---------------------------------------------------------------------------

#[test]
fn c30_find_patterns_fixed() {
    let _g = lock();
    init_both();
    let text = b"int alpha = 1; // alpha comment\nchar *s = \"alpha beta\";\nif (alpha == 2) { alpha++; }\n";
    diff_analyze(text);

    let patterns: Vec<&[u8]> = vec![
        b"",
        b"a",
        b"alpha",
        b"alphabet",
        b"==",
        b"\"",
        b"\\",
        b"//",
        b"/*",
        b"int",
        b"IF",
        b"if",
        b";",
        b"\n",
        b" ",
        b"1",
        b"nonexistent-pattern",
        &[0x80],
        &[0xff, 0xfe],
    ];
    for pat in patterns {
        let out = diff_find(pat);
        println!("pattern {:>22} -> {} bytes", show(pat), out.len());
    }
}

#[test]
fn c31_find_patterns_repeated() {
    let _g = lock();
    init_both();
    diff_analyze(b"aa ab ba bb // aa\n\"ab\"\n");
    for _ in 0..4 {
        diff_find(b"a");
        diff_find(b"b");
        diff_find(b"");
    }
    // interleaved with resets and re-analysis
    (libs().c.tokenizer_reset)();
    (libs().rust.tokenizer_reset)();
    diff_find(b"a");
    diff_analyze(b"cc dd cc");
    diff_find(b"c");
}

#[test]
fn c32_find_patterns_over_bare_load() {
    let _g = lock();
    let p = init_both();
    // no analyze_text: find_patterns re-scans whatever tokenizer_load_text left
    for text in [
        &b""[..],
        &b"one two three"[..],
        &b"if (x) /* c */ return \"s\";"[..],
    ] {
        assert_eq!(p.c.load_text(text), p.rust.load_text(text));
        diff_find(b"");
        diff_find(b"t");
        diff_find(b"xyz");
    }
}

#[test]
fn c30b_find_patterns_random() {
    let _g = lock();
    init_both();
    let mut rng = Rng::new(0xC30);
    for i in 0..200 {
        let text = if i % 3 == 0 {
            random_soup(&mut rng, 200)
        } else {
            random_source(&mut rng, 25)
        };
        diff_analyze(&text);
        // a pattern that is likely to match, plus a random one
        let mut pat: Vec<u8> = Vec::new();
        if !text.is_empty() {
            let start = rng.below(text.len());
            let len = rng.range(1, 4).min(text.len() - start);
            pat.extend_from_slice(&text[start..start + len]);
            pat.retain(|&b| b != 0);
        }
        diff_find(&pat);
        let random_pat = random_soup(&mut rng, 3);
        diff_find(&random_pat);
    }
}

// ---------------------------------------------------------------------------
// C33: analyze_text at the buffer-size boundary
// ---------------------------------------------------------------------------

#[test]
fn c33_analyze_text_size_boundary() {
    let _g = lock();
    init_both();
    for n in [8189usize, 8190, 8191] {
        let mut text = Vec::with_capacity(n);
        while text.len() < n {
            text.extend_from_slice(b"ab ");
        }
        text.truncate(n);
        init_both();
        let r = diff_analyze(&text);
        println!("n={} -> {:?}", n, r);
        diff_distribution();
        diff_score();
    }
}
