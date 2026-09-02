//! Phase B/C — `read_file()` in `c_src/src/main.c` (menu choice 2) and the
//! `tokenizer_load_text()` size check it feeds into.

mod common;
use common::{assert_same, fixture, scratch};

use std::path::Path;

/// Menu choice 2 with `path` as the filename, then a report and exit.
fn load(path: &Path, tail: &[u8]) -> Vec<u8> {
    let mut input = b"2\n".to_vec();
    input.extend_from_slice(path.to_str().expect("utf-8 scratch path").as_bytes());
    input.push(b'\n');
    input.extend_from_slice(tail);
    input
}

#[test]
fn missing_file_reports_to_stderr() {
    let dir = scratch("missing");
    let path = dir.join("does-not-exist");
    assert_same("file-missing", &load(&path, b"7\n"));
}

#[test]
fn empty_filename_fails_to_open() {
    assert_same("file-empty-name", b"2\n\n7\n");
}

#[test]
fn filename_is_truncated_at_the_first_newline_only() {
    let dir = scratch("trunc");
    let path = fixture(&dir, "plain.txt", b"int x = 1;\n");
    // Trailing spaces are kept, so the open fails.
    let mut input = b"2\n".to_vec();
    input.extend_from_slice(path.to_str().unwrap().as_bytes());
    input.extend_from_slice(b"  \n7\n");
    assert_same("file-trailing-space", &input);
}

#[test]
fn filename_containing_a_nul_is_truncated() {
    let dir = scratch("nulname");
    let path = fixture(&dir, "plain.txt", b"int x = 1;\n");
    let full = path.to_str().unwrap().as_bytes().to_vec();
    let mut input = b"2\n".to_vec();
    input.extend_from_slice(&full[..full.len() - 4]);
    input.push(0);
    input.extend_from_slice(&full[full.len() - 4..]);
    input.extend_from_slice(b"\n7\n");
    assert_same("file-nul-name", &input);
}

#[test]
fn eof_at_the_filename_prompt_reprints_the_menu() {
    // The C's `break` leaves the switch, not the while loop.
    assert_same("file-eof", b"2\n");
    assert_same("file-eof-no-newline", b"2");
}

#[test]
fn empty_file() {
    let dir = scratch("emptyfile");
    let path = fixture(&dir, "empty", b"");
    assert_same("file-empty", &load(&path, b"3\n4\n7\n"));
}

#[test]
fn single_byte_file() {
    let dir = scratch("onebyte");
    let path = fixture(&dir, "one", b"x");
    assert_same("file-one-byte", &load(&path, b"3\n7\n"));
}

#[test]
fn small_source_file() {
    let dir = scratch("small");
    let path = fixture(
        &dir,
        "src.c",
        b"int main(void) {\n    // entry\n    return 0;\n}\n",
    );
    assert_same("file-small", &load(&path, b"3\n4\n5\nreturn\n7\n"));
}

#[test]
fn file_size_boundaries_around_max_buffer_size() {
    let dir = scratch("sizes");

    // 8191 bytes: fits, strlen < MAX_BUFFER_SIZE, analyzed normally.
    let ok = fixture(&dir, "n8191", &vec![b'a'; 8191]);
    assert_same("file-8191", &load(&ok, b"7\n"));

    // 8192 bytes: `size > MAX_BUFFER_SIZE` is false, so read_file succeeds,
    // but tokenizer_load_text rejects `length >= MAX_BUFFER_SIZE` and
    // analyze_text then reports its own failure. Both messages go to stderr
    // and a zeroed result is still printed.
    let edge = fixture(&dir, "n8192", &vec![b'a'; 8192]);
    assert_same("file-8192", &load(&edge, b"3\n4\n7\n"));

    // 8193 bytes: rejected by read_file with "File too large".
    let too_big = fixture(&dir, "n8193", &vec![b'a'; 8193]);
    assert_same("file-8193", &load(&too_big, b"7\n"));

    // Much larger, same path.
    let huge = fixture(&dir, "huge", &vec![b'b'; 100_000]);
    assert_same("file-huge", &load(&huge, b"7\n"));
}

#[test]
fn embedded_nul_truncates_the_file_contents() {
    let dir = scratch("nulfile");
    let path = fixture(&dir, "nul", b"abc\x00def ghi\n");
    assert_same("file-nul-content", &load(&path, b"3\n7\n"));

    let leading = fixture(&dir, "nul-first", b"\x00abc\n");
    assert_same("file-nul-leading", &load(&leading, b"3\n7\n"));
}

#[test]
fn binary_file_contents() {
    let dir = scratch("binary");
    let bytes: Vec<u8> = (1u8..=255).collect();
    let path = fixture(&dir, "bin", &bytes);
    assert_same("file-binary", &load(&path, b"3\n4\n7\n"));
}

#[test]
fn a_directory_opens_but_cannot_be_read() {
    // fopen()/open() on a directory succeeds with O_RDONLY; the read then
    // fails with EISDIR, so the contents come out empty.
    let dir = scratch("asdir");
    fixture(&dir, "inner", b"x");
    assert_same("file-directory", &load(&dir, b"3\n7\n"));
}

#[test]
fn dev_null_reads_as_an_empty_file() {
    assert_same("file-dev-null", b"2\n/dev/null\n3\n7\n");
}

#[test]
fn a_zero_length_stat_but_readable_file() {
    // procfs reports size 0, so ftell() yields 0 and fread() copies nothing
    // even though the file has contents.
    if Path::new("/proc/version").exists() {
        assert_same("file-proc-version", b"2\n/proc/version\n3\n7\n");
    }
}

#[test]
fn loading_a_file_twice_accumulates_statistics() {
    let dir = scratch("twice");
    let path = fixture(&dir, "src.c", b"if (a) { b++; }\n");
    let mut input = load(&path, b"");
    input.extend_from_slice(&load(&path, b"3\n4\n7\n"));
    assert_same("file-twice", &input);
}

#[test]
fn a_loaded_file_can_then_be_pattern_searched() {
    let dir = scratch("patfile");
    let path = fixture(&dir, "src.c", b"alpha beta alpha\n/* alpha */\n");
    assert_same("file-then-pattern", &load(&path, b"5\nalpha\n7\n"));
}

#[test]
fn overlong_filename_is_split_by_fgets() {
    // fgets stops after 255 bytes; the remainder becomes the next menu line.
    let mut input = b"2\n/tmp/".to_vec();
    input.extend(std::iter::repeat(b'z').take(400));
    input.extend_from_slice(b"\n7\n");
    assert_same("file-overlong-name", &input);
}
