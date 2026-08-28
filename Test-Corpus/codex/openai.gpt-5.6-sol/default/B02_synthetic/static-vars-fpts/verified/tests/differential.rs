use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn run(program: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("failed to start {}: {error}", program.display()));

    child
        .stdin
        .take()
        .expect("child stdin was not piped")
        .write_all(input)
        .expect("failed to write child stdin");

    child.wait_with_output().expect("failed to wait for child")
}

fn assert_programs_match(name: &str, input: &[u8]) {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let c_program = manifest_dir.join("../c_src/build/driver");
    let rust_program = PathBuf::from(env!("CARGO_BIN_EXE_driver"));

    let c = run(&c_program, input);
    let rust = run(&rust_program, input);

    assert_eq!(rust.status, c.status, "{name}: exit status differs");
    assert_eq!(rust.stdout, c.stdout, "{name}: stdout differs");
    assert_eq!(rust.stderr, c.stderr, "{name}: stderr differs");
}

fn menu_input(choice: u8, payload: &[u8]) -> Vec<u8> {
    let mut input = vec![choice, b'\n'];
    input.extend_from_slice(payload);
    input
}

#[test]
fn menu_and_analysis_paths_match() {
    let cases: &[(&str, &[u8])] = &[
        ("menu_eof", b""),
        ("exit", b"7\n"),
        ("invalid_input", b"not a number\n7\n"),
        ("signed_choice_with_trailing_text", b"  +7 trailing text\n"),
        ("invalid_choice_low", b"0\n7\n"),
        ("invalid_choice_high", b"8\n7\n"),
        (
            "overflowing_positive_choice",
            b"999999999999999999999999999999\n7\n",
        ),
        (
            "overflowing_negative_choice",
            b"-999999999999999999999999999999\n7\n",
        ),
        ("empty_analysis", b"1\n\n7\n"),
        ("single_identifier", b"1\nitem\n\n7\n"),
        (
            "all_analysis_token_classes",
            b"1\nint _id = 12.5.6 + \"a\\\"b\"; // comment\n/* block\ncomment */ @\n\n3\n7\n",
        ),
        ("analysis_stops_at_eof", b"1\nidentifier"),
        ("distribution_before_analysis", b"3\n7\n"),
        ("low_complexity", b"4\n7\n"),
        ("medium_complexity", b"1\nif if if if if\n\n4\n7\n"),
        (
            "high_complexity",
            b"1\nif if if if if if if if if if if if if if if if if if if if if if if if if\n\n4\n7\n",
        ),
        ("comments_clamp_complexity_to_zero", b"1\n/ / / /\n\n4\n7\n"),
        (
            "punctuation_contributes_to_complexity",
            b"1\n(){}[];,..()\n\n4\n7\n",
        ),
        ("filename_eof", b"2\n"),
        ("empty_filename", b"2\n\n7\n"),
        ("pattern_eof", b"5\n"),
        ("pattern_before_analysis", b"5\nanything\n7\n"),
        (
            "pattern_found_missing_and_empty",
            b"1\nalpha alphabet beta\n\n5\nalpha\n5\nz\n5\n\n7\n",
        ),
    ];

    for (name, input) in cases {
        assert_programs_match(name, input);
    }

    assert_programs_match("nul_terminates_menu_input", b"7\0ignored\n");
    assert_programs_match("nul_terminates_collected_text", b"1\nbefore\0after\n\n7\n");
}

#[test]
fn tokenizer_paths_and_limits_match() {
    let cases: &[(&str, &[u8])] = &[
        ("empty_tokenizer", b"6\n\n7\n"),
        ("single_token", b"6\nname\n\n7\n"),
        (
            "all_tokenizer_classes",
            b"6\nif ident _x 12 1.2.3 \"closed\" 'x' \"unterminated\n// line\n/* closed */ /* open\n== != <= >= && || ++ -- -> << >> + - * % ^ ~ ? : (){}[];,.\n/ @\n\n7\n",
        ),
        (
            "operators_punctuation_and_error",
            b"6\n== != <= >= && || ++ -- -> << >> = ! < > & | + - * % ^ ~ ? : (){}[];,.\n/ @\n\n7\n",
        ),
        (
            "all_c_whitespace",
            b"6\nalpha \t\x0b\x0c\r beta\n\n7\n",
        ),
        (
            "escaped_newline_inside_string",
            b"6\n\"left\\\nright\" tail\n\n7\n",
        ),
        ("tokenizer_stops_at_eof", b"6\nlast"),
        ("string_escape_at_eof", b"6\n\"abc\\"),
    ];

    for (name, input) in cases {
        assert_programs_match(name, input);
    }

    let tokens = (0..101)
        .map(|index| format!("t{index}"))
        .collect::<Vec<_>>()
        .join(" ");
    let input = format!("6\n{tokens}\n\n7\n");
    assert_programs_match("token_output_truncates_after_101", input.as_bytes());

    let long_identifier = "a".repeat(300);
    let input = format!("6\n{long_identifier}\n\n7\n");
    assert_programs_match("token_length_limit", input.as_bytes());

    let long_number = "1".repeat(300);
    let input = format!("6\n{long_number}\n\n7\n");
    assert_programs_match("number_length_limit", input.as_bytes());

    let long_string = "s".repeat(300);
    let input = format!("6\n\"{long_string}\"\n\n7\n");
    assert_programs_match("string_length_limit", input.as_bytes());

    let long_line_comment = "c".repeat(300);
    let input = format!("6\n//{long_line_comment}\n\n7\n");
    assert_programs_match("line_comment_length_limit", input.as_bytes());

    let long_block_comment = "c".repeat(300);
    let input = format!("6\n/*{long_block_comment}*/\n\n7\n");
    assert_programs_match("block_comment_length_limit", input.as_bytes());

    let max_text = "a".repeat(4095);
    let input = format!("1\n{max_text}\n\n7\n");
    assert_programs_match("maximum_collected_text", input.as_bytes());

    let many_words = (0..101)
        .map(|index| format!("word{index}"))
        .collect::<Vec<_>>()
        .join(" ");
    let input = format!("1\nword50 word50 {many_words}\n\n3\n7\n");
    assert_programs_match("common_word_capacity_and_top_ten", input.as_bytes());
}

#[test]
fn file_paths_and_size_boundaries_match() {
    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("differential-fixtures");
    fs::create_dir_all(&fixture_dir).expect("failed to create fixture directory");

    let normal = fixture_dir.join("normal.txt");
    let max = fixture_dir.join("max-8191.txt");
    let analyzer_error = fixture_dir.join("analyzer-error-8192.txt");
    let file_error = fixture_dir.join("file-error-8193.txt");
    let binary = fixture_dir.join("binary-nul.txt");
    fs::write(&normal, b"int main() {\nreturn 1;\n}\n").expect("failed to write normal fixture");
    fs::write(&max, vec![b'a'; 8191]).expect("failed to write maximum fixture");
    fs::write(&analyzer_error, vec![b'a'; 8192]).expect("failed to write analyzer error fixture");
    fs::write(&file_error, vec![b'a'; 8193]).expect("failed to write oversized fixture");
    fs::write(&binary, b"before\0after\n").expect("failed to write binary fixture");

    let missing = fixture_dir.join("does-not-exist.txt");
    let _ = fs::remove_file(&missing);

    for (name, path) in [
        ("normal_file", normal),
        ("maximum_analyzable_file", max),
        ("analyzer_load_error", analyzer_error),
        ("file_too_large_error", file_error),
        ("binary_file_stops_at_nul", binary),
        ("file_open_error", missing),
    ] {
        let mut payload = path.as_os_str().as_encoded_bytes().to_vec();
        payload.extend_from_slice(b"\n7\n");
        let input = menu_input(b'2', &payload);
        assert_programs_match(name, &input);
    }
}
