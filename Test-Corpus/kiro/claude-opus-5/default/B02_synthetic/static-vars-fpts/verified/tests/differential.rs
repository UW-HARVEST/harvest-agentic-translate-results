//! Differential tests: run the original C `driver` and the translated Rust
//! `driver` as subprocesses, feed both the same bytes on stdin, and require
//! that stdout, stderr and the exit status all match exactly.
//!
//! Nothing here links against the translation as a library — the binary is
//! driven exactly the way a shell drives it, because that is how the two
//! programs are compared.
//!
//! The C reference binary is located at `c_src/build/driver` if it has already
//! been built; otherwise it is configured and built *out of source* into
//! `target/c_ref` so that `c_src/` is never written to.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Once;

// ---------------------------------------------------------------------------
// Locating the two binaries
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn target_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target")
}

/// Path to the Rust binary under test.
///
/// `CARGO_BIN_EXE_driver` is set by cargo for integration tests and always
/// refers to a freshly built binary. `RUST_DRIVER` overrides it so the same
/// suite can be pointed at `target/release/driver`.
fn rust_binary() -> PathBuf {
    match std::env::var_os("RUST_DRIVER") {
        Some(p) => PathBuf::from(p),
        None => PathBuf::from(env!("CARGO_BIN_EXE_driver")),
    }
}

static C_BUILD: Once = Once::new();

fn c_binary() -> PathBuf {
    let prebuilt = repo_root().join("c_src/build/driver");
    let out_of_tree = target_dir().join("c_ref/driver");

    C_BUILD.call_once(|| {
        if prebuilt.is_file() {
            return;
        }
        let build_dir = target_dir().join("c_ref");
        std::fs::create_dir_all(&build_dir).expect("create c_ref build dir");

        let cfg = Command::new("cmake")
            .arg("-S")
            .arg(repo_root().join("c_src"))
            .arg("-B")
            .arg(&build_dir)
            .output()
            .expect("cmake must be installed to build the C reference");
        assert!(
            cfg.status.success(),
            "cmake configure failed:\n{}",
            String::from_utf8_lossy(&cfg.stderr)
        );

        let build = Command::new("cmake")
            .arg("--build")
            .arg(&build_dir)
            .output()
            .expect("run cmake --build");
        assert!(
            build.status.success(),
            "cmake --build failed:\n{}",
            String::from_utf8_lossy(&build.stderr)
        );
    });

    if prebuilt.is_file() {
        prebuilt
    } else {
        out_of_tree
    }
}

// ---------------------------------------------------------------------------
// File fixtures for menu option 2 ("Load text from file")
// ---------------------------------------------------------------------------

static FIXTURES: Once = Once::new();

fn fixture_dir() -> PathBuf {
    let dir = target_dir().join("fixtures");
    FIXTURES.call_once(|| {
        std::fs::create_dir_all(&dir).expect("create fixture dir");
        let w = |name: &str, bytes: &[u8]| {
            std::fs::write(dir.join(name), bytes).expect("write fixture");
        };

        w("empty", b"");
        w("one_byte", b"a");
        w("code", b"int main(void) {\n    return 0; // done\n}\n");

        // read_file() size checks against MAX_BUFFER_SIZE (8192):
        //   8191 -> loads fine
        //   8192 -> passes read_file, then tokenizer_load_text rejects it
        //           because strlen(text) >= MAX_BUFFER_SIZE
        //   8193 -> read_file reports "Error: File too large"
        w("size_8191", &vec![b'a'; 8191]);
        w("size_8192", &vec![b'a'; 8192]);
        w("size_8193", &vec![b'a'; 8193]);
        w("size_20000", &vec![b'a'; 20000]);

        // Embedded NUL: read_file NUL-terminates at read_size, but analyze_text
        // takes strlen(), so everything from the NUL on is invisible.
        w("with_nul", b"alpha\x00beta gamma\n");

        // Bytes that are not valid UTF-8 must be passed through untouched.
        w("non_utf8", b"\xff\xfe caf\xc3\xa9 \x80\n");

        // Multi-line block comment: advance_char() resets current_column on the
        // newline, so create_token computes a *negative* column for the token.
        w("multiline_comment", b"/*\naaaaaaaaaaaaaaaaaaaa*/ tail\nint x;\n");
        w(
            "multiline_comment_long",
            b"/*llllllllllllllllllllllllllllll\nllllllllllllllllllllllllllllll\nllllllllllllllllllllllllllllll*/\nx\n",
        );

        // > 100 distinct identifiers, to hit the num_common_words < 100 cap.
        let mut many = Vec::new();
        for i in 0..130 {
            many.extend_from_slice(format!("id{:03} ", i).as_bytes());
        }
        many.push(b'\n');
        w("many_words", &many);

        // Enough repetition to make the bubble sort in
        // print_token_distribution() actually reorder things.
        let mut ranked = Vec::new();
        for (word, n) in [("alpha", 2), ("beta", 7), ("gamma", 4), ("delta", 9)] {
            for _ in 0..n {
                ranked.extend_from_slice(word.as_bytes());
                ranked.push(b' ');
            }
        }
        ranked.push(b'\n');
        w("ranked_words", &ranked);

        // Unterminated string literal, and a lone backslash at EOF.
        w("unterminated_string", b"\"never closed\nrest\n");
        w("backslash_eof", b"\"abc\\");

        // Dense operators/punctuation for the complexity-score arithmetic.
        w("dense", b"if(a==b){c++;}else{d--;}while(e<=f){g>>=h;}\n");
    });
    dir
}

/// Substitute `{FIX}/name` placeholders with the real fixture directory.
fn expand(input: &str) -> Vec<u8> {
    input
        .replace("{FIX}", fixture_dir().to_str().expect("utf-8 fixture path"))
        .into_bytes()
}

// ---------------------------------------------------------------------------
// The comparison itself
// ---------------------------------------------------------------------------

struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    status: Option<i32>,
}

fn run(bin: &Path, stdin_bytes: &[u8]) -> Outcome {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    let mut stdin = child.stdin.take().expect("piped stdin");
    let bytes = stdin_bytes.to_vec();
    // Write on a helper thread so a program that stops reading cannot deadlock
    // us against a full pipe.
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&bytes);
        let _ = stdin.flush();
        drop(stdin);
    });

    let out = child.wait_with_output().expect("collect child output");
    let _ = writer.join();

    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        status: out.status.code(),
    }
}

fn show(label: &str, bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => format!("{label} ({} bytes):\n{s}", bytes.len()),
        Err(_) => format!("{label} ({} bytes, non-UTF-8):\n{bytes:?}", bytes.len()),
    }
}

/// First differing byte offset, for a pointed error message.
fn first_diff(a: &[u8], b: &[u8]) -> usize {
    a.iter().zip(b.iter()).position(|(x, y)| x != y).unwrap_or_else(|| a.len().min(b.len()))
}

#[track_caller]
fn check_bytes(name: &str, stdin_bytes: &[u8]) {
    let c = run(&c_binary(), stdin_bytes);
    let r = run(&rust_binary(), stdin_bytes);

    if c.stdout != r.stdout {
        let at = first_diff(&c.stdout, &r.stdout);
        panic!(
            "[{name}] stdout differs at byte {at}\nstdin: {:?}\n{}\n{}",
            String::from_utf8_lossy(stdin_bytes),
            show("C stdout", &c.stdout),
            show("Rust stdout", &r.stdout),
        );
    }
    if c.stderr != r.stderr {
        let at = first_diff(&c.stderr, &r.stderr);
        panic!(
            "[{name}] stderr differs at byte {at}\nstdin: {:?}\n{}\n{}",
            String::from_utf8_lossy(stdin_bytes),
            show("C stderr", &c.stderr),
            show("Rust stderr", &r.stderr),
        );
    }
    assert_eq!(
        c.status, r.status,
        "[{name}] exit status differs (C={:?}, Rust={:?}) for stdin {:?}",
        c.status,
        r.status,
        String::from_utf8_lossy(stdin_bytes)
    );
}

#[track_caller]
fn check(name: &str, stdin_text: &str) {
    check_bytes(name, &expand(stdin_text));
}

// ===========================================================================
// Phase A — both programs build and run
// ===========================================================================

#[test]
fn both_binaries_exist_and_run() {
    let c = c_binary();
    let r = rust_binary();
    assert!(c.is_file(), "C binary missing at {}", c.display());
    assert!(r.is_file(), "Rust binary missing at {}", r.display());
    // Banner + menu is printed before any input is consumed.
    let out = run(&c, b"7\n");
    assert!(out.stdout.starts_with(b"Text Analysis and Tokenization System\n"));
    check("smoke", "7\n");
}

// ===========================================================================
// main.c — the menu loop
// ===========================================================================

#[test]
fn empty_stdin_exits_via_fgets_null() {
    // fgets returns NULL immediately: the menu is printed once, then `break`.
    check("empty stdin", "");
}

#[test]
fn exit_choice_prints_goodbye() {
    check("choice 7", "7\n");
    check("choice 7 no trailing newline", "7");
    // Everything after 7 is never read.
    check("input after exit", "7\nthis is never read\n1\n");
}

#[test]
fn unparsable_choice_reports_invalid_input() {
    // sscanf(input, "%d", &choice) != 1
    let cases: [&[u8]; 18] = [
        b"\n", b" \n", b"\t\n", b"abc\n", b"  hello\n", b"+\n", b"-\n", b"--\n", b"+-\n", b".\n",
        b".5\n", b"x7\n", b"\x00\n", b"\x00 7\n", b"\xff\n", b"\r\n", b"\x0b\n", b"\x0c\n",
    ];
    for (i, s) in cases.iter().enumerate() {
        let mut data = s.to_vec();
        data.extend_from_slice(b"7\n");
        check_bytes(&format!("invalid choice #{i}"), &data);
    }
}

#[test]
fn out_of_range_choices_report_invalid_choice() {
    for s in ["0\n", "8\n", "9\n", "99\n", "-1\n", "-7\n", "12345\n", "0x7\n", "0b11\n"] {
        check_bytes(
            &format!("bad choice {s:?}"),
            format!("{s}7\n").as_bytes(),
        );
    }
}

#[test]
fn choice_accepts_leading_whitespace_sign_and_trailing_junk() {
    for s in [" 7\n", "\t7\n", "  +7\n", "07\n", "0000007\n", "7 junk\n", "7x\n", "7.9\n", "7 7\n"] {
        check_bytes(
            &format!("choice {s:?}"),
            format!("{s}7\n").as_bytes(),
        );
    }
}

#[test]
fn choice_integer_overflow_truncates_like_strtol_then_int_cast() {
    // glibc converts with strtol (saturating at LONG_MIN/LONG_MAX) and then
    // stores the low 32 bits into an int.
    for s in [
        "2147483647\n",
        "2147483648\n",
        "-2147483648\n",
        "-2147483649\n",
        "4294967296\n",  // -> 0
        "4294967303\n",  // -> 7, i.e. actually exits!
        "9223372036854775807\n",
        "9223372036854775808\n",  // saturates -> LONG_MAX -> -1
        "-9223372036854775809\n", // saturates -> LONG_MIN -> 0
        "99999999999999999999999\n",
        "-99999999999999999999999\n",
    ] {
        check_bytes(
            &format!("overflow {s:?}"),
            format!("{s}7\n").as_bytes(),
        );
    }
}

#[test]
fn menu_choice_line_longer_than_the_256_byte_fgets_buffer() {
    // fgets stops after 255 bytes, so the remainder is read as the next choice.
    let long = format!("7{}\n", "z".repeat(400));
    check_bytes("long choice line", long.as_bytes());
    let long_digits = format!("{}\n7\n", "1".repeat(300));
    check_bytes("300 digits", long_digits.as_bytes());
}

// ===========================================================================
// main.c case 1 — analyze text typed on stdin
// ===========================================================================

#[test]
fn analyze_empty_text() {
    // Immediate blank line: analyze_text("") -> all zeros.
    check("analyze empty", "1\n\n3\n4\n7\n");
    // EOF instead of a blank line.
    check("analyze then EOF", "1\n");
    check("analyze text then EOF", "1\nint x;");
}

#[test]
fn analyze_single_item() {
    check("one identifier", "1\nx\n\n3\n4\n7\n");
    check("one keyword", "1\nint\n\n3\n4\n7\n");
    check("one number", "1\n42\n\n3\n4\n7\n");
    check("one operator", "1\n+\n\n3\n4\n7\n");
    check("one punctuation", "1\n;\n\n3\n4\n7\n");
}

#[test]
fn analyze_mixed_text() {
    check(
        "mixed",
        "1\nif (x == 1) { /* note */ y++; } // trailing\n\"a string\"\n\n3\n4\n7\n",
    );
}

#[test]
fn analyze_accumulates_stats_across_calls() {
    // total_lines_processed / total_chars_processed are never reset, and they
    // overwrite the per-analysis newline tally.
    check("cumulative", "1\na b\n\n1\nc d\n\n1\n\n\n3\n4\n7\n");
}

#[test]
fn analyze_input_is_capped_at_4096_bytes_by_strncat() {
    // strncat(text, line, MAX_INPUT_SIZE - strlen(text) - 1) caps the buffer at
    // 4095 bytes; whatever comes after is silently dropped mid-token.
    for line_len in [1usize, 63, 253, 254, 255, 256, 300] {
        for n_lines in [1usize, 16, 17, 18, 40] {
            let body: String = std::iter::repeat(format!("{}\n", "a".repeat(line_len)))
                .take(n_lines)
                .collect();
            check_bytes(
                &format!("strncat cap len={line_len} n={n_lines}"),
                format!("1\n{body}\n3\n4\n7\n").as_bytes(),
            );
        }
    }
}

#[test]
fn analyze_line_longer_than_fgets_buffer_is_split() {
    let long = "y".repeat(600);
    check_bytes(
        "600-char line",
        format!("1\n{long}\n\n3\n7\n").as_bytes(),
    );
}

// ===========================================================================
// main.c case 2 / read_file — load text from a file
// ===========================================================================

#[test]
fn load_file_success() {
    check("file: code", "2\n{FIX}/code\n3\n4\n7\n");
    check("file: empty", "2\n{FIX}/empty\n3\n4\n7\n");
    check("file: one byte", "2\n{FIX}/one_byte\n3\n7\n");
    check("file: /dev/null", "2\n/dev/null\n3\n7\n");
}

#[test]
fn load_file_open_failure_writes_to_stderr() {
    check("file: missing", "2\n{FIX}/does_not_exist\n7\n");
    check("file: empty filename", "2\n\n7\n");
    check("file: nested missing", "2\n/nonexistent/dir/file\n7\n");
    // strcspn only trims the newline, so surrounding spaces stay in the name.
    check("file: name with spaces", "2\n {FIX}/code \n7\n");
}

#[test]
fn load_file_directory() {
    // fopen() on a directory succeeds on Linux; the later read fails.
    check("file: directory", "2\n{FIX}\n3\n7\n");
}

#[test]
fn load_file_size_boundaries() {
    check("file: 8191 bytes", "2\n{FIX}/size_8191\n3\n4\n7\n");
    // Exactly MAX_BUFFER_SIZE: read_file accepts it, tokenizer_load_text does
    // not -> two lines on stderr and a zeroed result.
    check("file: 8192 bytes", "2\n{FIX}/size_8192\n3\n4\n7\n");
    // Larger than MAX_BUFFER_SIZE: "Error: File too large".
    check("file: 8193 bytes", "2\n{FIX}/size_8193\n7\n");
    check("file: 20000 bytes", "2\n{FIX}/size_20000\n7\n");
}

#[test]
fn load_file_with_embedded_nul_and_non_utf8_bytes() {
    check("file: NUL", "2\n{FIX}/with_nul\n3\n5\n\n7\n");
    check("file: non-UTF-8", "2\n{FIX}/non_utf8\n3\n5\n\n7\n");
    // A raw 0xFF byte as the search pattern (not valid UTF-8).
    let mut data = expand("2\n{FIX}/non_utf8\n5\n");
    data.extend_from_slice(b"\xff\n7\n");
    check_bytes("pattern: raw 0xFF byte", &data);
}

#[test]
fn load_file_eof_at_filename_prompt() {
    // `break` leaves only the switch, so the menu is printed once more.
    check("EOF at filename", "2\n");
}

#[test]
fn load_file_tracks_more_than_100_distinct_words() {
    check("track_word cap", "2\n{FIX}/many_words\n3\n4\n7\n");
}

// ===========================================================================
// main.c case 3 / print_token_distribution
// ===========================================================================

#[test]
fn token_distribution_before_any_analysis() {
    // All counts zero: only the two headers are printed.
    check("distribution empty", "3\n7\n");
}

#[test]
fn token_distribution_sorting_and_top_10_limit() {
    check("distribution ranked", "2\n{FIX}/ranked_words\n3\n7\n");
    // The bubble sort mutates the stored arrays, so calling it twice re-sorts
    // an already sorted array.
    check("distribution twice", "2\n{FIX}/ranked_words\n3\n3\n7\n");
    for n in [1usize, 9, 10, 11, 25] {
        let words: Vec<String> = (0..n).map(|i| format!("t{i}")).collect();
        check_bytes(
            &format!("top-10 limit with {n} words"),
            format!("1\n{}\n\n3\n7\n", words.join(" ")).as_bytes(),
        );
    }
}

#[test]
fn token_distribution_ties_keep_insertion_order() {
    // The swap is on `<`, not `<=`, so equal counts are not reordered.
    check("ties", "1\np q r p q r s\n\n3\n7\n");
}

// ===========================================================================
// main.c case 4 / calculate_complexity_score
// ===========================================================================

#[test]
fn complexity_score_branches() {
    // score < 10 -> Low
    check("complexity low", "1\nif x\n\n4\n7\n");
    check("complexity zero", "4\n7\n");
    // 10 <= score < 50 -> Medium
    let med = std::iter::repeat("if").take(10).collect::<Vec<_>>().join(" ");
    check_bytes(
        "complexity medium",
        format!("1\n{med}\n\n4\n7\n").as_bytes(),
    );
    // score >= 50 -> High
    let high = std::iter::repeat("if").take(30).collect::<Vec<_>>().join(" ");
    check_bytes("complexity high", format!("1\n{high}\n\n4\n7\n").as_bytes());
    // Comments subtract; the negative result is clamped back to 0.
    let cmts = std::iter::repeat("/*c*/").take(20).collect::<Vec<_>>().join(" ");
    check_bytes("complexity clamped", format!("1\n{cmts}\n\n4\n7\n").as_bytes());
    // Punctuation contributes count/10 (truncating division).
    check_bytes(
        "complexity punctuation",
        format!("1\n{}\n\n3\n4\n7\n", "(){}[];,.".repeat(5)).as_bytes(),
    );
    check("complexity dense", "2\n{FIX}/dense\n3\n4\n7\n");
}

#[test]
fn complexity_score_exactly_at_the_thresholds() {
    // 5 keywords * 2 = 10 -> Medium; 4 keywords * 2 = 8 -> Low.
    check("score 8", "1\nif if if if\n\n4\n7\n");
    check("score 10", "1\nif if if if if\n\n4\n7\n");
    // 25 keywords * 2 = 50 -> High; 24 -> 48 -> Medium.
    let k24 = std::iter::repeat("if").take(24).collect::<Vec<_>>().join(" ");
    let k25 = std::iter::repeat("if").take(25).collect::<Vec<_>>().join(" ");
    check_bytes("score 48", format!("1\n{k24}\n\n4\n7\n").as_bytes());
    check_bytes("score 50", format!("1\n{k25}\n\n4\n7\n").as_bytes());
}

// ===========================================================================
// main.c case 5 / find_patterns
// ===========================================================================

#[test]
fn find_pattern_matches_and_misses() {
    check("pattern found", "1\nint a = 1; int b = 2;\n\n5\nint\n7\n");
    check("pattern missing", "1\nint a = 1;\n\n5\nzzzz\n7\n");
    check("pattern operator", "1\nint a = 1;\n\n5\n=\n7\n");
    // strstr(value, "") is never NULL, so an empty pattern matches every token.
    check("pattern empty", "1\na b c\n\n5\n\n7\n");
}

#[test]
fn find_pattern_before_any_text_is_loaded() {
    // reset() rewinds an empty buffer -> immediately EOF -> 0 occurrences.
    check("pattern with no text", "5\nx\n7\n");
    check("pattern empty with no text", "5\n\n7\n");
}

#[test]
fn find_pattern_eof_at_prompt() {
    check("EOF at pattern", "5\n");
}

#[test]
fn find_pattern_reports_negative_columns_for_multiline_comments() {
    // advance_char() resets current_column at '\n', so
    // `current_column - token.length` goes negative for a token spanning lines.
    check("negative column", "2\n{FIX}/multiline_comment\n5\na\n5\n*\n7\n");
    check("negative column long", "2\n{FIX}/multiline_comment_long\n5\nl\n3\n7\n");
}

#[test]
fn find_pattern_uses_whatever_buffer_was_loaded_last() {
    check("pattern after option 6", "6\nint a; int b;\n\n5\nint\n7\n");
    check("pattern after file", "2\n{FIX}/code\n5\nreturn\n7\n");
}

// ===========================================================================
// main.c case 6 / interactive_tokenizer
// ===========================================================================

#[test]
fn interactive_tokenizer_empty_and_basic() {
    check("interactive empty", "6\n\n7\n");
    check("interactive EOF", "6\n");
    check("interactive basic", "6\nint x = 1;\n\n7\n");
    check("interactive EOF mid block", "6\nint x = 1;\n");
}

#[test]
fn interactive_tokenizer_truncates_after_101_tokens() {
    // count is incremented before `if (count > 100)`, so 101 tokens print
    // before the truncation notice.
    for n in [99usize, 100, 101, 102, 150] {
        let words: Vec<String> = (0..n).map(|i| format!("a{i}")).collect();
        check_bytes(
            &format!("interactive {n} tokens"),
            format!("6\n{}\n\n7\n", words.join(" ")).as_bytes(),
        );
    }
}

#[test]
fn interactive_tokenizer_leaves_state_for_later_options() {
    // Option 6 advances the shared char/line counters without touching the
    // analyzer's per-type counts.
    check("6 then 1", "6\nint a; int b;\n\n1\nx y\n\n3\n4\n7\n");
    check("6 then 3", "6\nint a; int b;\n\n3\n4\n7\n");
}

// ===========================================================================
// tokenizer.c — one case per scanning branch
// ===========================================================================

#[test]
fn tokenize_identifiers_and_keywords() {
    check(
        "words",
        "6\n_a a_1 1a 1_a __ _ 9z if iff If IF else elsex return returns short longer\n\n3\n7\n",
    );
    // Every keyword in the table.
    let kws = "if else while for return int char float double void struct typedef const \
               static extern auto register sizeof break continue switch case default do \
               goto enum union signed unsigned long short";
    check_bytes("all keywords", format!("1\n{kws}\n\n3\n4\n7\n").as_bytes());
}

#[test]
fn tokenize_numbers() {
    // A second '.' breaks out of scan_number, so "1.2.3" is three tokens.
    check("numbers", "6\n1 42 007 1.5 1.2.3 4. .5 1..2 0.0.0\n\n3\n7\n");
    check_bytes(
        "long number",
        format!("6\n{}\n\n7\n", "7".repeat(300)).as_bytes(),
    );
    check_bytes(
        "many dots",
        format!("6\n{}\n\n7\n", "1.".repeat(200)).as_bytes(),
    );
}

#[test]
fn tokenize_strings() {
    check("strings", "6\n\"abc\" 'x' \"it's\" 'say \"hi\"'\n\n3\n7\n");
    check("string escapes", "6\n\"a\\\"b\" 'c\\'d' \"a\\\\b\"\n\n3\n7\n");
    // Unterminated by newline, and unterminated by end of buffer.
    check("string unterminated", "6\n\"never closed\n\n7\n");
    check("string file unterminated", "2\n{FIX}/unterminated_string\n3\n7\n");
    check("string backslash at EOF", "2\n{FIX}/backslash_eof\n3\n7\n");
    check("empty string literal", "6\n'' \"\"\n\n7\n");
}

#[test]
fn tokenize_string_length_cap() {
    // The loop stops at MAX_TOKEN_LENGTH - 2, and the escape branch appends two
    // bytes at once, so the assembled buffer can reach 256 before
    // create_token() truncates the stored value to 255.
    for n in 245..258 {
        check_bytes(
            &format!("escaped string len {n}"),
            format!("6\n\"{}\\z\"\n\n7\n", "a".repeat(n)).as_bytes(),
        );
    }
    for n in 250..260 {
        check_bytes(
            &format!("plain string len {n}"),
            format!("6\n\"{}\"\n\n7\n", "b".repeat(n)).as_bytes(),
        );
    }
    check_bytes(
        "all-escape string",
        format!("6\n\"{}\"\n\n7\n", "\\\\".repeat(200)).as_bytes(),
    );
}

#[test]
fn tokenize_comments() {
    check("line comment", "6\n// a line comment\n\n3\n7\n");
    check("block comment", "6\n/* a block */\n\n3\n7\n");
    check("block unterminated", "6\n/* never closed\n\n3\n7\n");
    // scan_comment() compares peek_char() against '/' and '*' *before*
    // consuming, so the test is really just `c == '/'` and every single slash
    // becomes a COMMENT token.
    check("lone slash is a comment", "6\na / b /\n\n3\n4\n7\n");
    check("division looks like a comment", "1\nx = a / b / c\n\n3\n4\n7\n");
    check("block with stars", "6\n/*** stars ***/\n\n3\n7\n");
    check_bytes(
        "block of stars",
        format!("6\n/*{}/\n\n7\n", "*".repeat(260)).as_bytes(),
    );
    for n in [252usize, 253, 254, 255, 256, 260] {
        check_bytes(
            &format!("line comment len {n}"),
            format!("6\n//{}\n\n7\n", "c".repeat(n)).as_bytes(),
        );
        check_bytes(
            &format!("block comment len {n}"),
            format!("6\n/*{}*/\n\n7\n", "d".repeat(n)).as_bytes(),
        );
    }
    check("multiline block comment", "2\n{FIX}/multiline_comment\n3\n4\n7\n");
}

#[test]
fn tokenize_operators() {
    // Both the two-character forms and the single-character fallbacks.
    check(
        "operators",
        "6\n== != <= >= && || ++ -- -> << >> + - * % = < > ! & | ^ ~ ? : =! <- >+ &| |&\n\n3\n4\n7\n",
    );
}

#[test]
fn tokenize_punctuation() {
    check("punctuation", "6\n( ) { } [ ] ; , .\n\n3\n4\n7\n");
}

#[test]
fn tokenize_unknown_bytes_become_error_tokens() {
    check_bytes(
        "error tokens",
        b"6\n@ # $ ` \x01 \x80 \xff \xc3\xa9 \x7f\n\n3\n4\n7\n",
    );
}

#[test]
fn tokenize_whitespace_handling() {
    // skip_whitespace() skips every isspace() byte except '\n'.
    check_bytes("whitespace variants", b"1\na\x0bb\x0cc\rd\te f\n\n3\n4\n7\n");
    check_bytes("whitespace only", b"1\n \t\x0b\x0c\r\n\n3\n4\n7\n");
    check_bytes("leading whitespace", b"6\n\t\t  x\n\n7\n");
    check_bytes("CRLF", b"1\r\nint x\r\n\r\n3\n7\r\n");
}

#[test]
fn tokenize_word_length_cap() {
    // scan_word() stops at MAX_TOKEN_LENGTH - 1 and the rest of the run becomes
    // a second token.
    for n in [254usize, 255, 256, 257, 600] {
        check_bytes(
            &format!("identifier len {n}"),
            format!("6\n{}\n\n7\n", "i".repeat(n)).as_bytes(),
        );
    }
}

#[test]
fn tokenizer_rejects_input_at_or_above_max_buffer_size() {
    // The only reachable route is a file of exactly MAX_BUFFER_SIZE bytes:
    // "Error: Input text too large" from the tokenizer, then
    // "Error: Failed to load text" from the analyzer.
    check("load_text too large", "2\n{FIX}/size_8192\n7\n");
}

#[test]
fn nul_bytes_on_stdin() {
    // fgets keeps NUL bytes, but every consumer treats the buffer as a C string.
    check_bytes("NUL in analyzed text", b"1\nab\x00cd\n\n3\n4\n7\n");
    check_bytes("NUL in choice", b"7\x00abc\n");
    check_bytes("NUL then digit", b"\x007\n7\n");
    check_bytes("NUL in pattern", b"1\nabc\n\n5\na\x00b\n7\n");
}

// ===========================================================================
// Longer sequences that mix options and cross-contaminate state
// ===========================================================================

#[test]
fn full_menu_walkthrough() {
    check(
        "walkthrough",
        "1\nif (x == 1) { /*c*/ y++; }\n\n3\n4\n5\nx\n6\nz--\n\n1\n\n\n3\n4\n5\n\n7\n",
    );
}

#[test]
fn every_option_visited_at_least_once_in_order() {
    check(
        "1..7 in order",
        "1\nint a;\n\n2\n{FIX}/code\n3\n4\n5\na\n6\nb\n\n7\n",
    );
}

#[test]
fn interleaved_invalid_and_valid_choices() {
    check("interleaved", "abc\n0\n99\n1\nint x;\n\n-1\n\n3\n4\n7\n");
}

#[test]
fn repeated_pattern_searches_do_not_disturb_counts() {
    check("repeated searches", "1\na b c\n\n5\na\n5\nb\n5\n\n3\n4\n7\n");
}

#[test]
fn large_realistic_input() {
    let body: String = std::iter::repeat("int a = 1; b++; // note\n").take(150).collect();
    check_bytes(
        "large input",
        format!("1\n{body}\n3\n4\n5\na\n7\n").as_bytes(),
    );
}
