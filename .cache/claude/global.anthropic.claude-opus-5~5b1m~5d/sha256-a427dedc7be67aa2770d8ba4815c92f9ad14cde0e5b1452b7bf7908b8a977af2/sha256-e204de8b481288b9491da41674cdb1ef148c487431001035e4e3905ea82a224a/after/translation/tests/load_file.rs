//! Differential tests for menu entry `2` (load text from a file), i.e. every
//! branch of `read_file` plus the size limits of `tokenizer_load_text`.

mod common;

use common::{assert_same, data_dir, data_path};

fn load(path: &str) -> Vec<u8> {
    format!("2\n{path}\n3\n4\n7\n").into_bytes()
}

#[test]
fn eof_right_after_the_choice() {
    // fgets returns NULL, the C code `break`s out of the switch and the outer
    // loop then hits EOF as well.
    assert_same("eof_after_2", b"2\n");
}

#[test]
fn missing_file() {
    assert_same("missing", &load("/nonexistent/definitely/not/here"));
    assert_same("missing_relative", &load("no_such_file_here.txt"));
}

#[test]
fn empty_filename() {
    // strcspn turns the bare newline into an empty string; fopen("") fails.
    assert_same("empty_filename", b"2\n\n3\n7\n");
    assert_same("blank_filename", b"2\n   \n3\n7\n");
    assert_same("tab_filename", b"2\n\t\n3\n7\n");
}

#[test]
fn small_file() {
    assert_same("small_c", &load(&data_path("small.c")));
    assert_same("code_c", &load(&data_path("code.c")));
    assert_same("one_word", &load(&data_path("one_word.txt")));
    assert_same("newlines", &load(&data_path("newlines.txt")));
}

#[test]
fn empty_file() {
    assert_same("empty_file", &load(&data_path("empty.txt")));
}

#[test]
fn size_limits() {
    // read_file rejects anything strictly larger than MAX_BUFFER_SIZE (8192)
    // with "File too large"; tokenizer_load_text then rejects a length of
    // exactly 8192 with "Input text too large", which analyze_text reports as
    // "Failed to load text".
    assert_same("size_8191", &load(&data_path("size8191.txt")));
    assert_same("size_8192", &load(&data_path("size8192.txt")));
    assert_same("size_8193", &load(&data_path("size8193.txt")));
    assert_same("size_4096", &load(&data_path("size4096.txt")));
}

#[test]
fn files_containing_nul_bytes() {
    // The content is used as a C string, so a NUL truncates it.
    assert_same("nul_middle", &load(&data_path("nul_middle.bin")));
    assert_same("nul_first", &load(&data_path("nul_first.bin")));
}

#[test]
fn file_with_high_bytes() {
    assert_same("high_bytes", &load(&data_path("high_bytes.bin")));
}

#[test]
fn directory_instead_of_file() {
    // fopen() succeeds on a directory, fread() then fails with EISDIR.
    assert_same("a_directory", &load(&data_path("a_directory")));
    assert_same("data_dir", &load(data_dir().to_str().expect("utf-8 path")));
    assert_same("root_dir", &load("/"));
}

#[test]
fn unreadable_file() {
    assert_same("no_permission", &load(&data_path("no_permission.txt")));
}

#[test]
fn procfs_file_reports_size_zero() {
    // SEEK_END is refused on these streams, so ftell reports the unchanged
    // position 0 and the program analyses an empty string.
    assert_same("proc_self_status", &load("/proc/self/status"));
    assert_same("proc_version", &load("/proc/version"));
}

#[test]
fn character_devices() {
    assert_same("dev_null", &load("/dev/null"));
    assert_same("dev_zero", &load("/dev/zero"));
}

#[test]
fn non_seekable_stream() {
    // On a FIFO both fseek and ftell fail, so `size` stays -1, malloc(0)
    // succeeds and fread reports 0 bytes: an empty analysis, no error message.
    let fifo_c = data_dir().join("fifo_c");
    let fifo_r = data_dir().join("fifo_r");
    for path in [&fifo_c, &fifo_r] {
        let _ = std::fs::remove_file(path);
        let status = std::process::Command::new("mkfifo")
            .arg(path)
            .status()
            .expect("mkfifo must be available");
        assert!(status.success(), "mkfifo failed for {}", path.display());
    }

    let feed = |path: std::path::PathBuf| {
        std::thread::spawn(move || {
            // Opening for writing blocks until the program opens the read end.
            let _ = std::fs::write(&path, b"int x = 1;\n");
        })
    };

    let writer_c = feed(fifo_c.clone());
    let c = common::run(
        common::c_binary(),
        &load(fifo_c.to_str().expect("utf-8 path")),
    );
    let _ = writer_c.join();

    let writer_r = feed(fifo_r.clone());
    let r = common::run(
        common::rust_binary(),
        &load(fifo_r.to_str().expect("utf-8 path")),
    );
    let _ = writer_r.join();

    // The absolute paths differ, so compare everything after the prompt that
    // echoes the file name.  Neither program prints the name on stdout, so the
    // streams must match verbatim here.
    assert_eq!(
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout),
        "fifo: stdout differs"
    );
    assert_eq!(
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr),
        "fifo: stderr differs"
    );
    assert_eq!((c.status, c.signal), (r.status, r.signal), "fifo: exit differs");

    let _ = std::fs::remove_file(&fifo_c);
    let _ = std::fs::remove_file(&fifo_r);
}

#[test]
fn filename_longer_than_the_buffer() {
    // fgets truncates the name at 255 bytes; the tail becomes the next choice.
    let mut input = Vec::from(&b"2\n"[..]);
    input.extend(std::iter::repeat(b'n').take(300));
    input.extend_from_slice(b"\n7\n");
    assert_same("long_filename", &input);

    // Exactly 255 bytes of name: no newline is seen, so strcspn keeps them all.
    let mut input = Vec::from(&b"2\n"[..]);
    input.extend(std::iter::repeat(b'n').take(255));
    input.extend_from_slice(b"\n7\n");
    assert_same("filename_255", &input);
}

#[test]
fn filename_with_nul_and_high_bytes() {
    assert_same("filename_nul", b"2\n\0abc\n7\n");
    assert_same("filename_high", b"2\n\xff\xfe\n7\n");
}

#[test]
fn file_then_more_analysis() {
    let path = data_path("code.c");
    let input = format!("2\n{path}\n1\nint y;\n\n3\n4\n5\nint\n7\n");
    assert_same("file_then_text", input.as_bytes());
}
