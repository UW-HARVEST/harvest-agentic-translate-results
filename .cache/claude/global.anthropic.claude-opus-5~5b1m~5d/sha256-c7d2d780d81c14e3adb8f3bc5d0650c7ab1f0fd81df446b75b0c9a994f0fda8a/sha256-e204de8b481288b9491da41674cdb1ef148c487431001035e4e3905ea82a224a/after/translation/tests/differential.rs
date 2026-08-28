//! Differential tests: every case runs the C executable built from `c_src/` and
//! the Rust executable built from this crate as subprocesses, feeding both the
//! same stdin, and requires stdout, stderr and the exit status to match byte for
//! byte.
//!
//! The case names below refer to the branches in the C source that they reach.

mod harness;

use harness::{analyze, assert_all, cat, interactive, rep, Rng};

// ---------------------------------------------------------------------------
// main(): the read/parse loop around `fgets` + `sscanf(input, "%d", &choice)`
// ---------------------------------------------------------------------------

#[test]
fn menu_eof_and_empty_input() {
    assert_all(&[
        // `fgets` returns NULL straight away -> the while(1) loop breaks.
        ("empty_stdin", b"".to_vec()),
        // case 7 -> "Goodbye!" and `return 0`.
        ("exit", b"7\n".to_vec()),
        // Last line without a trailing newline: fgets still returns it.
        ("exit_no_newline", b"7".to_vec()),
        ("exit_then_more", b"7\n1\nignored\n".to_vec()),
        // EOF after a valid command, i.e. the loop breaks at the top.
        ("eof_after_command", b"3\n".to_vec()),
        ("eof_after_command_no_newline", b"3".to_vec()),
        ("only_newlines", b"\n\n\n".to_vec()),
    ]);
}

#[test]
fn menu_choice_parsing() {
    assert_all(&[
        // sscanf returns EOF (nothing but whitespace) -> "Invalid input".
        ("blank_line", b"\n7\n".to_vec()),
        ("spaces_only", b"   \n7\n".to_vec()),
        ("tabs_only", b"\t\t\n7\n".to_vec()),
        ("cr_only", b"\r\n7\n".to_vec()),
        ("vtab_only", b"\x0b\n7\n".to_vec()),
        ("formfeed_only", b"\x0c\n7\n".to_vec()),
        // sscanf matching failure -> "Invalid input".
        ("alpha", b"abc\n7\n".to_vec()),
        ("lone_minus", b"-\n7\n".to_vec()),
        ("lone_plus", b"+\n7\n".to_vec()),
        ("double_sign", b"--7\n7\n".to_vec()),
        ("sign_space_digits", b"- 7\n7\n".to_vec()),
        ("dot", b".\n7\n".to_vec()),
        ("nul_first_byte", b"\x007\n7\n".to_vec()),
        // successful conversions
        ("leading_spaces", b"  7\n".to_vec()),
        ("leading_tab", b"\t7\n".to_vec()),
        ("leading_vtab", b"\x0b7\n".to_vec()),
        ("leading_formfeed", b"\x0c7\n".to_vec()),
        ("leading_newlines", b"\n\n7\n".to_vec()),
        ("trailing_junk", b"7abc\n".to_vec()),
        ("trailing_tab", b"7\t\n".to_vec()),
        ("explicit_plus", b"+7\n".to_vec()),
        ("leading_zeros", b"0000000007\n".to_vec()),
        ("hex_like", b"0x7\n7\n".to_vec()),   // %d stops after "0"
        ("float_like", b"1.9\n\n7\n".to_vec()), // %d yields 1
        ("exp_like", b"1e5\n\n7\n".to_vec()),   // %d yields 1
        // out-of-range choices -> "Invalid choice"
        ("choice_zero", b"0\n7\n".to_vec()),
        ("choice_eight", b"8\n7\n".to_vec()),
        ("choice_negative", b"-1\n7\n".to_vec()),
        ("choice_negative_zero", b"-0\n7\n".to_vec()),
        ("choice_99", b"99\n7\n".to_vec()),
        // integer conversion edges: strtol saturation and the long->int cast
        ("int_max", b"2147483647\n7\n".to_vec()),
        ("int_max_plus_one", b"2147483648\n7\n".to_vec()),
        ("int_min", b"-2147483648\n7\n".to_vec()),
        ("two_pow_32", b"4294967296\n7\n".to_vec()),
        ("two_pow_32_plus_1", b"4294967297\n\n7\n".to_vec()),
        ("two_pow_32_plus_7", b"4294967303\n".to_vec()),
        ("long_max", b"9223372036854775807\n7\n".to_vec()),
        ("long_max_plus_one", b"9223372036854775808\n7\n".to_vec()),
        ("long_min", b"-9223372036854775808\n7\n".to_vec()),
        ("two_pow_64_plus_1", b"18446744073709551617\n7\n".to_vec()),
        ("two_pow_64_plus_2", b"18446744073709551618\n7\n".to_vec()),
        ("neg_two_pow_64_plus_1", b"-18446744073709551617\n7\n".to_vec()),
        ("huge_digits", cat(&[&rep(b"9", 400), b"\n7\n"])),
        ("many_leading_zeros", cat(&[&rep(b"0", 200), b"7\n7\n"])),
        // 255 bytes fill the fgets buffer exactly, so the rest becomes the next
        // menu line.
        ("fgets_boundary_254", cat(&[&rep(b" ", 254), b"7\n"])),
        ("fgets_boundary_255", cat(&[&rep(b" ", 255), b"7\n"])),
        ("fgets_boundary_256", cat(&[&rep(b" ", 256), b"7\n"])),
        ("fgets_split_digits", cat(&[b"7", &rep(b"0", 254), b"\n7\n"])),
        ("long_garbage_line", cat(&[&rep(b"x", 300), b"\n7\n"])),
        ("all_choices_in_order", b"1\n\n3\n4\n5\na\n6\n\n7\n".to_vec()),
    ]);
}

// ---------------------------------------------------------------------------
// case 1: analyze_text() + the tokenizer's scanning branches
// ---------------------------------------------------------------------------

#[test]
fn analyze_basic_and_empty() {
    assert_all(&[
        // empty line immediately -> analyze_text("")
        ("analyze_empty", b"1\n\n7\n".to_vec()),
        // EOF while collecting text
        ("analyze_eof", b"1\n".to_vec()),
        ("analyze_eof_mid_text", b"1\nabc".to_vec()),
        ("analyze_single_word", analyze(b"hello")),
        ("analyze_single_keyword", analyze(b"int")),
        ("analyze_single_number", analyze(b"42")),
        ("analyze_single_char", analyze(b"a")),
        ("analyze_whitespace_only", analyze(b" \t\x0b\x0c\r")),
        ("analyze_newlines_only", cat(&[b"1\n", b"\n"])),
        ("analyze_twice", b"1\nint a;\n\n1\nint b;\n\n7\n".to_vec()),
    ]);
}

#[test]
fn tokenizer_token_kinds() {
    assert_all(&[
        // keywords (all 31 of them) vs identifiers
        (
            "keywords_all",
            analyze(
                b"if else while for return int char float double void struct typedef \
                  const static extern auto register sizeof break continue switch case \
                  default do goto enum union signed unsigned long short",
            ),
        ),
        ("keyword_prefixes", analyze(b"i in intx iff returns Int INT")),
        ("identifiers_underscore", analyze(b"_a _1 __ _ a_b_c x9")),
        // numbers: scan_number's second-decimal-point break
        ("number_int", analyze(b"12345")),
        ("number_decimal", analyze(b"1.5")),
        ("number_trailing_dot", analyze(b"1.")),
        ("number_leading_dot", analyze(b".5")),
        ("number_two_dots", analyze(b"1.2.3")),
        ("number_many_dots", analyze(b"1.2.3.4.5")),
        ("number_dot_dot", analyze(b"..")),
        ("number_then_ident", analyze(b"123abc")),
        // strings: both quote characters, escapes, unterminated, newline stop
        ("string_double", analyze(b"\"abc\"")),
        ("string_single", analyze(b"'a'")),
        ("string_empty", analyze(b"\"\"")),
        ("string_unterminated", analyze(b"\"abc")),
        ("string_lone_quote", analyze(b"\"")),
        ("string_lone_apostrophe", analyze(b"'")),
        ("string_escaped_quote", analyze(b"\"a\\\"b\"")),
        ("string_escape_at_eof", analyze(b"\"a\\")),
        ("string_backslash_only", analyze(b"\"\\\\\"")),
        ("string_apostrophe_inside", analyze(b"\"it's\"")),
        ("string_quote_inside_single", analyze(b"'\"'")),
        ("string_stops_at_newline", cat(&[b"1\n\"abc\ndef\"\n\n7\n"])),
        // comments: `//`, `/* */`, unterminated, and the quirk that a lone `/`
        // also enters scan_comment (peek_char() still returns the '/' itself)
        ("comment_line", analyze(b"// hello")),
        ("comment_line_empty", analyze(b"//")),
        ("comment_block", analyze(b"/*x*/")),
        ("comment_block_empty", analyze(b"/**/")),
        ("comment_block_unterminated", analyze(b"/* never closed")),
        ("comment_block_multiline", cat(&[b"1\n/*\nabc*/\n\n7\n"])),
        ("comment_block_many_lines", cat(&[b"1\n/*", &rep(b"line\n", 20), b"*/\n\n7\n"])),
        ("comment_block_stars", analyze(&cat(&[b"/*", &rep(b"*", 20), b"/"]))),
        ("comment_slash_alone", analyze(b"/")),
        ("comment_slash_between", analyze(b"a / b")),
        ("comment_slash_at_eof", analyze(b"a/")),
        ("comment_slash_slash_star", analyze(b"//*")),
        ("comment_star_slash", analyze(b"*/")),
        ("comment_slash_star_slash", analyze(b"/*/")),
        // operators: every one- and two-character form
        (
            "operators_two_char",
            analyze(b"== != <= >= && || ++ -- -> << >>"),
        ),
        (
            "operators_one_char",
            analyze(b"+ - * % = < > ! & | ^ ~ ? :"),
        ),
        ("operators_glued", analyze(b"a==b!=c<=d>=e&&f||g++h--i->j<<k>>l")),
        ("operators_near_miss", analyze(b"=! <& >| +- -+ <> ><")),
        // punctuation
        ("punctuation_all", analyze(b"(){}[];,.")),
        // unknown characters -> TOKEN_ERROR
        ("unknown_at", analyze(b"@")),
        ("unknown_hash", analyze(b"#")),
        ("unknown_dollar", analyze(b"$a")),
        ("unknown_backtick", analyze(b"`")),
        ("unknown_backslash", analyze(b"\\")),
        ("unknown_del", analyze(b"\x7f")),
        ("unknown_soh", analyze(b"\x01")),
        // bytes with the high bit set: isalpha()/isspace() see a negative
        // `char` and must classify them as "other" in the C locale.
        ("high_byte_80", analyze(b"\x80")),
        ("high_byte_ff", analyze(b"\xff")),
        ("high_byte_utf8", analyze(b"\xc3\xa9\xe2\x82\xac")),
        ("high_byte_nbsp", analyze(b"\xa0a\xa0")),
        // NUL byte inside a line: strncat stops there
        ("nul_inside_line", b"1\nab\x00cd\n\n7\n".to_vec()),
        ("nul_starts_line", b"1\n\x00abc\n\n7\n".to_vec()),
        // a realistic C fragment exercising many branches at once
        (
            "c_snippet",
            cat(&[
                b"1\n",
                b"int main(void) {\n",
                b"    /* multi\n       line */\n",
                b"    char *s = \"hi\\n\";\n",
                b"    if (x >= 1 && y != 2) { x++; }\n",
                b"    return 0; // done\n",
                b"}\n",
                b"\n7\n",
            ]),
        ),
    ]);
}

#[test]
fn tokenizer_token_length_limits() {
    // MAX_TOKEN_LENGTH is 256; scan_* stop at 255 (254 for strings and block
    // comments) and create_token clamps token.length to 255.
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    for n in [1usize, 2, 100, 252, 253, 254, 255, 256, 257, 258, 300, 511, 512] {
        cases.push((format!("ident_{}", n), analyze(&rep(b"a", n))));
        cases.push((format!("number_{}", n), analyze(&rep(b"1", n))));
        cases.push((format!("string_{}", n), analyze(&cat(&[b"\"", &rep(b"s", n), b"\""]))));
        cases.push((format!("comment_line_{}", n), analyze(&cat(&[b"//", &rep(b"c", n)]))));
        cases.push((
            format!("comment_block_{}", n),
            analyze(&cat(&[b"/*", &rep(b"m", n), b"*/"])),
        ));
        cases.push((
            format!("string_escapes_{}", n),
            analyze(&cat(&[b"\"", &rep(b"\\a", n), b"\""])),
        ));
        // same shapes through the interactive tokenizer, which prints the
        // (possibly truncated) token text plus its line/column.
        cases.push((format!("i_ident_{}", n), interactive(&rep(b"a", n))));
        cases.push((
            format!("i_string_{}", n),
            interactive(&cat(&[b"\"", &rep(b"s", n), b"\""])),
        ));
        cases.push((
            format!("i_string_escapes_{}", n),
            interactive(&cat(&[b"\"", &rep(b"\\a", n), b"\""])),
        ));
        cases.push((
            format!("i_comment_block_{}", n),
            interactive(&cat(&[b"/*", &rep(b"m", n), b"*/"])),
        ));
    }
    // an escape pair landing exactly on scan_string's `length < MAX-2` bound
    for pad in [249usize, 250, 251, 252, 253, 254] {
        cases.push((
            format!("i_escape_at_limit_{}", pad),
            interactive(&cat(&[b"\"", &rep(b"z", pad), b"\\q\""])),
        ));
    }
    let cases: Vec<(&str, Vec<u8>)> =
        cases.iter().map(|(n, v)| (n.as_str(), v.clone())).collect();
    assert_all(&cases);
}

#[test]
fn tokenizer_column_arithmetic() {
    // create_token computes `current_column - token.length` in size_t and then
    // truncates to int, so tokens that span a newline get a negative column.
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    cases.push(("newline_token_column".into(), interactive(b"a\nb")));
    cases.push(("block_comment_newline".into(), interactive(b"/*\n*/")));
    cases.push(("block_comment_blank_lines".into(), interactive(b"/*\n\n\n\nx*/")));
    for n in [1usize, 2, 5, 20] {
        cases.push((
            format!("block_comment_{}_newlines", n),
            interactive(&cat(&[b"/*", &rep(b"\n", n), b"*/"])),
        ));
    }
    cases.push((
        "block_comment_wide_lines".into(),
        interactive(&cat(&[b"/*", &rep(b"aaaaaaaaaaaaaaaaaaaa\n", 6), b"*/"])),
    ));
    cases.push((
        "block_comment_long_across_lines".into(),
        interactive(&cat(&[b"/*", &rep(b"abc\n", 60), b"*/"])),
    ));
    // find_patterns prints line/column too
    cases.push(("pattern_negative_column".into(), b"1\n/*\nabc*/\n\n5\n*\n7\n".to_vec()));
    let cases: Vec<(&str, Vec<u8>)> =
        cases.iter().map(|(n, v)| (n.as_str(), v.clone())).collect();
    assert_all(&cases);
}

#[test]
fn analyze_input_buffer_limits() {
    // main()'s `char text[MAX_INPUT_SIZE]` is 4096 bytes and is filled with
    // strncat(text, line, MAX_INPUT_SIZE - strlen(text) - 1), so input is
    // truncated at 4095 bytes.
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    for n in [4090usize, 4093, 4094, 4095, 4096, 4097, 4100, 5000] {
        cases.push((format!("single_line_{}", n), analyze(&rep(b"x", n))));
    }
    for n in [253usize, 254, 255, 256, 257] {
        // lines that straddle the 256-byte fgets buffer
        cases.push((format!("fgets_line_{}", n), analyze(&rep(b"y", n))));
        cases.push((
            format!("fgets_line_{}_no_newline", n),
            cat(&[b"1\n", &rep(b"y", n)]),
        ));
    }
    cases.push((
        "many_lines_overflow".into(),
        cat(&[b"1\n", &rep(b"abcdefgh\n", 600), b"\n7\n"]),
    ));
    cases.push((
        "truncated_mid_token".into(),
        cat(&[b"1\n", &rep(b"a", 4090), b"\n", &rep(b"b", 300), b"\n\n7\n"]),
    ));
    cases.push((
        "many_short_lines".into(),
        cat(&[b"1\n", &rep(b"x\n", 300), b"\n7\n"]),
    ));
    cases.push((
        "blank_line_ends_early".into(),
        b"1\nfirst\n\nsecond\n7\n".to_vec(),
    ));
    let cases: Vec<(&str, Vec<u8>)> =
        cases.iter().map(|(n, v)| (n.as_str(), v.clone())).collect();
    assert_all(&cases);
}

// ---------------------------------------------------------------------------
// case 2: read_file() + tokenizer_load_text()
// ---------------------------------------------------------------------------

#[test]
fn file_loading() {
    assert_all(&[
        // fopen failure -> "Error: Could not open file '...'" on stderr
        ("file_missing", b"2\nno_such_file_xyz\n7\n".to_vec()),
        ("file_empty_name", b"2\n\n7\n".to_vec()),
        ("file_name_with_spaces", b"2\n data/small.txt \n7\n".to_vec()),
        ("file_name_too_long", cat(&[b"2\n", &rep(b"n", 300), b"\n7\n"])),
        ("file_no_permission", b"2\ndata/noperm.txt\n7\n".to_vec()),
        ("file_trailing_slash", b"2\ndata/small.txt/\n7\n".to_vec()),
        // successful reads
        ("file_small", b"2\ndata/small.txt\n7\n".to_vec()),
        ("file_empty", b"2\ndata/empty.txt\n7\n".to_vec()),
        ("file_no_trailing_newline", b"2\ndata/noeol.txt\n7\n".to_vec()),
        ("file_crlf", b"2\ndata/crlf.txt\n7\n".to_vec()),
        ("file_only_newlines", b"2\ndata/only_newlines.txt\n3\n7\n".to_vec()),
        ("file_high_bytes", b"2\ndata/highbytes.txt\n7\n".to_vec()),
        ("file_binary", b"2\ndata/binary.bin\n3\n7\n".to_vec()),
        ("file_4096", b"2\ndata/size4096.txt\n7\n".to_vec()),
        ("file_8000", b"2\ndata/size8000.txt\n7\n".to_vec()),
        ("file_words", b"2\ndata/words.txt\n3\n4\n7\n".to_vec()),
        // NUL bytes: strlen() < st_size, so buffer_length shrinks
        ("file_nul_middle", b"2\ndata/nul.txt\n7\n".to_vec()),
        ("file_nul_first", b"2\ndata/nul_first.txt\n7\n".to_vec()),
        ("file_nul_first_8192", b"2\ndata/nul_first_8192.txt\n3\n7\n".to_vec()),
        // MAX_BUFFER_SIZE boundary
        ("file_8191", b"2\ndata/exact8191.txt\n7\n".to_vec()),
        // 8192 passes read_file's `size > MAX_BUFFER_SIZE` test but is rejected
        // by tokenizer_load_text's `length >= MAX_BUFFER_SIZE`, producing two
        // stderr lines and a zeroed result.
        ("file_8192", b"2\ndata/exact8192.txt\n7\n".to_vec()),
        ("file_8192_then_more", b"2\ndata/exact8192.txt\n3\n4\n7\n".to_vec()),
        // 8193 -> "Error: File too large" and no analysis at all
        ("file_over_8192", b"2\ndata/over8192.txt\n7\n".to_vec()),
        ("file_over_then_analyze", b"2\ndata/over8192.txt\n1\nabc\n\n7\n".to_vec()),
        // non-regular / zero-length-but-readable files: glibc's fseek(SEEK_END)
        // uses fstat for regular files, so procfs reports size 0.
        ("file_dev_null", b"2\n/dev/null\n7\n".to_vec()),
        ("file_dev_zero", b"2\n/dev/zero\n7\n".to_vec()),
        ("file_proc_status", b"2\n/proc/self/status\n7\n".to_vec()),
        ("file_proc_version", b"2\n/proc/version\n7\n".to_vec()),
        ("file_directory", b"2\ndata\n7\n".to_vec()),
        ("file_directory_dot", b"2\n.\n7\n".to_vec()),
        // EOF at the filename prompt: the switch `break`s, the loop re-prints
        // the menu and only then sees EOF.
        ("file_eof_at_prompt", b"2".to_vec()),
        ("file_eof_at_prompt_nl", b"2\n".to_vec()),
        // repeated loads accumulate the tokenizer's static totals
        ("file_twice", b"2\ndata/small.txt\n2\ndata/small.txt\n3\n7\n".to_vec()),
        ("file_then_pattern", b"2\ndata/small.txt\n5\nx\n7\n".to_vec()),
        ("file_then_analyze", b"2\ndata/small.txt\n1\nint y;\n\n3\n7\n".to_vec()),
    ]);
}

// ---------------------------------------------------------------------------
// case 3: print_token_distribution()
// ---------------------------------------------------------------------------

#[test]
fn token_distribution() {
    let many = |n: usize| -> Vec<u8> {
        let words: Vec<String> = (0..n).map(|i| format!("w{:03}", i)).collect();
        cat(&[b"1\n", words.join(" ").as_bytes(), b"\n\n3\n7\n"])
    };
    assert_all(&[
        // nothing analysed yet: every counter is zero, so only the headers show
        ("dist_fresh", b"3\n7\n".to_vec()),
        ("dist_repeated_fresh", b"3\n3\n3\n7\n".to_vec()),
        ("dist_after_empty_analyze", b"1\n\n3\n7\n".to_vec()),
        (
            "dist_after_mixed",
            b"1\nfor (i = 0; i < 10; i++) { a[i] = b[i]; } // loop\n\n3\n7\n".to_vec(),
        ),
        // the common-word table holds at most 100 entries
        ("dist_99_words", many(99)),
        ("dist_100_words", many(100)),
        ("dist_101_words", many(101)),
        ("dist_150_words", many(150)),
        (
            "dist_table_full_then_repeat",
            cat(&[
                b"1\n",
                (0..120)
                    .map(|i| format!("w{:03}", i))
                    .collect::<Vec<_>>()
                    .join(" ")
                    .as_bytes(),
                b" w000 w000\n\n3\n7\n",
            ]),
        ),
        // bubble sort: distinct counts, ties, and repeated sorting
        (
            "dist_distinct_counts",
            b"1\naa bb aa cc cc cc dd dd\n\n3\n7\n".to_vec(),
        ),
        ("dist_all_ties", b"1\nq w e r t y u i o p a s d f\n\n3\n7\n".to_vec()),
        ("dist_sorted_twice", b"1\naa bb aa cc bb aa\n\n3\n3\n7\n".to_vec()),
        (
            "dist_analyze_after_sort",
            b"1\naa bb\n\n3\n1\ncc aa\n\n3\n7\n".to_vec(),
        ),
        // words longer than MAX_TOKEN_LENGTH-1 get truncated in the table
        (
            "dist_long_words",
            cat(&[b"1\n", &rep(b"a", 260), b" ", &rep(b"b", 300), b"\n\n3\n7\n"]),
        ),
        // exactly 10 vs more than 10 distinct words (the "top 10" limit)
        ("dist_9_words", many(9)),
        ("dist_10_words", many(10)),
        ("dist_11_words", many(11)),
        ("dist_after_interactive", b"6\nint a;\n\n3\n7\n".to_vec()),
    ]);
}

// ---------------------------------------------------------------------------
// case 4: calculate_complexity_score()
// ---------------------------------------------------------------------------

#[test]
fn complexity_score() {
    assert_all(&[
        ("score_fresh", b"4\n7\n".to_vec()),
        ("score_low_zero", b"1\nabc\n\n4\n7\n".to_vec()),
        // keywords*2 + operators + punctuation/10 - comments, clamped at 0
        ("score_9_low", cat(&[b"1\n", &rep(b"if ", 4), b"+\n\n4\n7\n"])),
        ("score_10_medium", cat(&[b"1\n", &rep(b"if ", 5), b"\n\n4\n7\n"])),
        ("score_49_medium", cat(&[b"1\n", &rep(b"if ", 24), b"+\n\n4\n7\n"])),
        ("score_50_high", cat(&[b"1\n", &rep(b"if ", 25), b"\n\n4\n7\n"])),
        ("score_high", cat(&[b"1\n", &rep(b"if int char ", 20), b"\n\n4\n7\n"])),
        // punctuation integer division
        ("score_punct_9", cat(&[b"1\n", &rep(b"(", 9), b"\n\n4\n7\n"])),
        ("score_punct_10", cat(&[b"1\n", &rep(b"(", 10), b"\n\n4\n7\n"])),
        ("score_punct_25", cat(&[b"1\n", &rep(b"(", 25), b"\n\n4\n7\n"])),
        // comments push the score negative -> clamped to 0
        ("score_negative_clamped", cat(&[b"1\n", &rep(b"//x\n", 10), b"\n4\n7\n"])),
        ("score_comment_vs_keyword", b"1\nif //a\n//b\n//c\n\n4\n7\n".to_vec()),
        ("score_mixed", b"1\nif(a){b=1;}else{c=2;}//k\n\n4\n7\n".to_vec()),
        ("score_repeated", b"1\nint a = 1; // x\n\n3\n4\n3\n4\n7\n".to_vec()),
        ("score_accumulates", b"1\nif\n\n4\n1\nif\n\n4\n7\n".to_vec()),
    ]);
}

// ---------------------------------------------------------------------------
// case 5: find_patterns()
// ---------------------------------------------------------------------------

#[test]
fn find_pattern() {
    assert_all(&[
        // no text loaded yet: the tokenizer buffer is empty
        ("pattern_fresh", b"5\nx\n7\n".to_vec()),
        ("pattern_fresh_empty", b"5\n\n7\n".to_vec()),
        // strstr with an empty needle matches every token
        ("pattern_empty", b"1\nabc def\n\n5\n\n7\n".to_vec()),
        ("pattern_found_multiple", b"1\nabc abcd xabc\n\n5\nabc\n7\n".to_vec()),
        ("pattern_not_found", b"1\nabc\n\n5\nzzz\n7\n".to_vec()),
        ("pattern_longer_than_token", cat(&[b"1\nabc\n\n5\n", &rep(b"y", 300), b"\n7\n"])),
        ("pattern_255", cat(&[b"1\n", &rep(b"a", 300), b"\n\n5\n", &rep(b"a", 254), b"\n7\n"])),
        // punctuation / operator / whitespace needles
        ("pattern_paren", b"1\n(a)\n\n5\n(\n7\n".to_vec()),
        ("pattern_operator", b"1\na == b\n\n5\n==\n7\n".to_vec()),
        ("pattern_backslash", b"1\n\"a\\\\b\"\n\n5\n\\\n7\n".to_vec()),
        ("pattern_cr", b"1\na\rb\n\n5\n\r\n7\n".to_vec()),
        ("pattern_high_byte", b"1\n\x80x\n\n5\n\x80\n7\n".to_vec()),
        // the pattern is truncated at the first NUL by strcspn/strstr
        ("pattern_nul_prefix", b"1\nabc\n\n5\n\x00abc\n7\n".to_vec()),
        ("pattern_nul_middle", b"1\nabc\n\n5\nab\x00c\n7\n".to_vec()),
        // find_patterns() calls reset(), which rewinds without clearing the
        // cumulative statistics, so a later analyze reports a bigger char count
        ("pattern_then_analyze", b"1\nint a;\n\n5\na\n1\nint a;\n\n7\n".to_vec()),
        ("pattern_twice", b"1\na b c\n\n5\na\n5\nb\n7\n".to_vec()),
        ("pattern_three_times", b"1\nabcd\n\n5\nabc\n5\nabc\n5\nd\n7\n".to_vec()),
        // EOF at the pattern prompt
        ("pattern_eof_at_prompt", b"5".to_vec()),
        ("pattern_eof_at_prompt_nl", b"5\n".to_vec()),
        ("pattern_after_file", b"2\ndata/small.txt\n5\nx\n7\n".to_vec()),
        ("pattern_newline_token", b"1\na\nb\n\n5\n\n7\n".to_vec()),
    ]);
}

// ---------------------------------------------------------------------------
// case 6: interactive_tokenizer()
// ---------------------------------------------------------------------------

#[test]
fn interactive_tokenizer_cases() {
    let n_tokens = |n: usize| -> Vec<u8> {
        let words: Vec<&[u8]> = std::iter::repeat(b"z" as &[u8]).take(n).collect();
        let joined = words.join(&b' ');
        interactive(&joined)
    };
    let mut cases: Vec<(String, Vec<u8>)> = vec![
        ("interactive_empty_line".into(), b"6\n\n7\n".to_vec()),
        ("interactive_eof".into(), b"6\n".to_vec()),
        ("interactive_eof_no_newline".into(), b"6".to_vec()),
        ("interactive_simple".into(), interactive(b"int x = 1;")),
        ("interactive_newlines".into(), b"6\na\nb\nc\n\n7\n".to_vec()),
        ("interactive_high_bytes".into(), interactive(b"\x80\x81\xff")),
        ("interactive_nul".into(), b"6\na\x00b\n\n7\n".to_vec()),
        ("interactive_then_dist".into(), b"6\nint a;\n\n3\n4\n7\n".to_vec()),
        ("interactive_then_analyze".into(), b"6\nint a;\n\n1\nint b;\n\n3\n7\n".to_vec()),
        (
            "interactive_all_kinds".into(),
            interactive(b"int i=0; /*c*/ \"s\" 'q' // done"),
        ),
        (
            "interactive_overflow_buffer".into(),
            cat(&[b"6\n", &rep(b"ab\n", 600), b"\n7\n"]),
        ),
    ];
    // `if (count > 100)` truncates after the 101st token
    for n in [1usize, 2, 50, 99, 100, 101, 102, 103, 200] {
        cases.push((format!("interactive_{}_tokens", n), n_tokens(n)));
    }
    cases.push((
        "interactive_101_newline_tokens".into(),
        cat(&[b"6\n", &rep(b"a\n", 60), b"\n7\n"]),
    ));
    let cases: Vec<(&str, Vec<u8>)> =
        cases.iter().map(|(n, v)| (n.as_str(), v.clone())).collect();
    assert_all(&cases);
}

// ---------------------------------------------------------------------------
// whole sessions: the static state in tokenizer.c / analyzer.c is never reset,
// so command order is observable.
// ---------------------------------------------------------------------------

#[test]
fn sessions() {
    assert_all(&[
        (
            "session_full",
            cat(&[
                b"1\nint main() { return 0; }\n\n",
                b"3\n4\n5\nmain\n",
                b"6\nfloat f = 1.5;\n\n",
                b"3\n4\n",
                b"2\ndata/small.txt\n3\n4\n5\nx\n7\n",
            ]),
        ),
        ("session_no_exit", b"1\nabc\n\n3\n4\n".to_vec()),
        (
            "session_errors_mixed",
            b"2\nmissing\n2\ndata/over8192.txt\n2\ndata/exact8192.txt\n3\n7\n".to_vec(),
        ),
        (
            "session_invalid_between",
            b"abc\n1\nint\n\nxyz\n3\n\n4\n99\n7\n".to_vec(),
        ),
        (
            "session_char_count_accumulates",
            b"1\nabc\n\n1\nabc\n\n1\nabc\n\n7\n".to_vec(),
        ),
        (
            "session_pattern_between_analyses",
            b"1\nabc\n\n5\na\n1\nabc\n\n5\na\n7\n".to_vec(),
        ),
        (
            "session_dist_between_analyses",
            b"1\naa bb\n\n3\n1\naa cc\n\n3\n1\ndd\n\n3\n7\n".to_vec(),
        ),
    ]);
}

#[test]
fn every_byte_value() {
    // Every byte except '\n' (which would terminate the input block) and NUL
    // handling is covered separately; run the whole range through the tokenizer
    // via both the analyzer and the interactive printer.
    let all: Vec<u8> = (1u16..256).map(|b| b as u8).filter(|&b| b != b'\n').collect();
    let mut cases: Vec<(String, Vec<u8>)> = vec![
        ("all_bytes_analyze".into(), analyze(&all)),
        ("all_bytes_interactive".into(), interactive(&all)),
        ("all_bytes_pattern".into(), cat(&[b"1\n", &all, b"\n\n5\n@\n7\n"])),
    ];
    // one case per byte, so a single misclassified character is pinpointed
    for b in 1u16..256 {
        let b = b as u8;
        if b == b'\n' {
            continue;
        }
        cases.push((format!("byte_{:02x}", b), analyze(&[b'a', b, b'b'])));
    }
    let cases: Vec<(&str, Vec<u8>)> =
        cases.iter().map(|(n, v)| (n.as_str(), v.clone())).collect();
    assert_all(&cases);
}

// ---------------------------------------------------------------------------
// deterministic randomised sweep
// ---------------------------------------------------------------------------

#[test]
fn randomised_sessions() {
    const FRAGMENTS: &[&[u8]] = &[
        b"int", b"char", b"if", b"else", b"while", b"return", b"static", b"typedef",
        b"abc", b"x", b"_y", b"z9", b"var_name",
        b"0", b"42", b"1.5", b"1.2.3", b"3.", b".5",
        b"\"str\"", b"\"unterminated", b"\"esc\\\"q\"", b"'c'", b"'", b"\"",
        b"//line", b"/*block*/", b"/*unterminated", b"*/", b"/", b"//",
        b"==", b"!=", b"<=", b">=", b"&&", b"||", b"++", b"--", b"->", b"<<", b">>",
        b"=", b"<", b">", b"!", b"&", b"|", b"^", b"~", b"?", b":", b"%", b"*", b"+", b"-",
        b"(", b")", b"{", b"}", b"[", b"]", b";", b",", b".",
        b"@", b"#", b"$", b"`", b"\\", b"\x7f", b"\x80", b"\xff", b"\x01",
        b" ", b"\t", b"\x0b", b"\x0c", b"\r", b"\x00",
    ];
    const FILES: &[&[u8]] = &[
        b"data/small.txt", b"data/empty.txt", b"data/nul.txt", b"data/exact8191.txt",
        b"data/exact8192.txt", b"data/over8192.txt", b"data/words.txt",
        b"data/binary.bin", b"data/noperm.txt", b"data", b".", b"/dev/null",
        b"/proc/self/status", b"missing_file", b"",
    ];
    const CHOICES: &[&[u8]] = &[
        b"1", b"2", b"3", b"4", b"5", b"6", b"7", b"0", b"8", b"-1", b"99", b"",
        b"abc", b" 3 ", b"3junk", b"+4", b"2147483648", b"99999999999999999999",
        b"0000001", b"\t5", b"1.9",
    ];

    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    for seed in 1u64..=200 {
        let mut rng = Rng::new(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let mut input: Vec<u8> = Vec::new();
        for _ in 0..1 + rng.below(5) {
            let choice: &[u8] = *rng.pick(CHOICES);
            input.extend_from_slice(choice);
            input.push(b'\n');
            let trimmed: &[u8] = choice.trim_ascii();
            if trimmed == b"1" || trimmed == b"6" {
                for _ in 0..rng.below(6) {
                    for _ in 0..rng.below(10) {
                        input.extend_from_slice(*rng.pick(FRAGMENTS));
                    }
                    input.push(b'\n');
                }
                if rng.below(10) == 0 {
                    input.extend_from_slice(&rep(b"q", 4000 + rng.below(400)));
                    input.push(b'\n');
                }
                input.push(b'\n');
            } else if trimmed == b"2" {
                input.extend_from_slice(*rng.pick(FILES));
                input.push(b'\n');
            } else if trimmed == b"5" {
                let frag: &[u8] = *rng.pick(FRAGMENTS);
                let cleaned: Vec<u8> = frag.iter().copied().filter(|&b| b != b'\n').collect();
                input.extend_from_slice(&cleaned);
                input.push(b'\n');
            }
        }
        if rng.below(10) < 7 {
            input.extend_from_slice(b"7\n");
        }
        cases.push((format!("session_seed_{}", seed), input));
    }
    let cases: Vec<(&str, Vec<u8>)> =
        cases.iter().map(|(n, v)| (n.as_str(), v.clone())).collect();
    assert_all(&cases);
}

#[test]
fn randomised_raw_bytes() {
    // Unstructured input, to shake out fgets / sscanf edge cases.
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    const ALPHABET: &[u8] = b"1234567\n \t/*\"'\\.;=<>+-@\x00abcxyz_";
    for seed in 1u64..=150 {
        let mut rng = Rng::new(seed.wrapping_mul(0xD1B5_4A32_D192_ED03) ^ 0x5DEE_CE66);
        let mode = rng.below(3);
        let n = rng.below(400);
        let mut input: Vec<u8> = Vec::with_capacity(n);
        match mode {
            0 => {
                for _ in 0..n {
                    input.push((rng.below(256)) as u8);
                }
            }
            1 => {
                for _ in 0..n {
                    input.push(*rng.pick(ALPHABET));
                }
            }
            _ => {
                for _ in 0..1 + rng.below(20) {
                    input.push(b"1234567"[rng.below(7)]);
                    for _ in 0..rng.below(40) {
                        input.push((rng.below(256)) as u8);
                    }
                    input.push(b'\n');
                }
            }
        }
        cases.push((format!("raw_seed_{}", seed), input));
    }
    let cases: Vec<(&str, Vec<u8>)> =
        cases.iter().map(|(n, v)| (n.as_str(), v.clone())).collect();
    assert_all(&cases);
}
