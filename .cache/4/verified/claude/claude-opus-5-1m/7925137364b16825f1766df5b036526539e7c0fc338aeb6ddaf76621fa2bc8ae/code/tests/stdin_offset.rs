// CONFIGS.md rows C26/C27 — how much of stdin the process consumes.
//
// This is observable: `{ driver; cat; } < file` shows what the *next* reader of
// the same descriptor sees.  glibc gives back the unused read-ahead of a
// seekable descriptor when the process exits (_IO_cleanup -> _IO_SYNC), and its
// stream buffer is `st_blksize` bytes rather than Rust's 8 KiB, so both the file
// offset and the bytes remaining in a pipe are part of the behaviour that has to
// match.
//
// The tests reproduce the shell scenario without a shell: the child gets a
// `dup()` of our own descriptor (which shares the file offset for files, and is
// the very same pipe for pipes), and afterwards we read what is left.

mod common;

use common::*;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};

/// Runs `bin` with a regular file as stdin and returns what a subsequent reader
/// of the same descriptor would see.
fn leftover_after_file(bin: &Path, content: &[u8]) -> Vec<u8> {
    let dir = std::env::temp_dir().join("driver-difftest");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!(
        "offset-{}-{:p}",
        std::process::id(),
        content.as_ptr()
    ));
    std::fs::write(&path, content).unwrap();

    let mut mine = std::fs::File::open(&path).unwrap();
    let childs = mine.try_clone().unwrap(); // dup(): shares the file offset

    let status = Command::new(bin)
        .stdin(Stdio::from(childs))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("spawn");
    assert!(status.success(), "child failed: {status:?}");

    let mut rest = Vec::new();
    mine.read_to_end(&mut rest).unwrap();
    std::fs::remove_file(&path).ok();
    rest
}

/// Same, with a pipe: whatever the child swallowed is gone for good.
fn leftover_after_pipe(bin: &Path, content: &[u8]) -> Vec<u8> {
    assert!(content.len() < 60_000, "keep below the pipe capacity");
    let (reader, mut writer) = std::io::pipe().unwrap();
    let childs = reader.try_clone().unwrap();
    writer.write_all(content).unwrap();
    drop(writer); // EOF once drained

    let status = Command::new(bin)
        .stdin(Stdio::from(childs))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("spawn");
    assert!(status.success(), "child failed: {status:?}");

    let mut rest = Vec::new();
    let mut r = reader;
    r.read_to_end(&mut rest).unwrap();
    rest
}

fn cases() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = vec![
        b"1 2 3 REST-OF-DATA".to_vec(),
        b"1 2 3\nsecond line\nthird line\n".to_vec(),
        b"1 2 3".to_vec(),
        b"1 2".to_vec(),
        b"1".to_vec(),
        b"".to_vec(),
        b"abc def ghi".to_vec(),
        b"   \t\n  1 2 3 tail".to_vec(),
        b"0 2 3 unread".to_vec(),
        b"1 x 3 unread".to_vec(),
        b"-".to_vec(),
        b"1 2 3 4 5 6 7 8 9".to_vec(),
    ];
    // Sizes straddling st_blksize (4096) and Rust's old 8 KiB buffer.
    for n in [4000usize, 4095, 4096, 4097, 8191, 8192, 8193, 20000, 50000] {
        let mut s = b"1 2 3 ".to_vec();
        s.extend(std::iter::repeat(b'X').take(n));
        v.push(s);
        // ...and the tokens *after* a long whitespace run.
        let mut s = vec![b' '; n];
        s.extend_from_slice(b"1 2 3 TAIL");
        v.push(s);
    }
    v
}

/// C26 — seekable stdin: the file offset left behind must be identical.
#[test]
fn c26_file_offset_left_behind() {
    for content in cases() {
        let c = leftover_after_file(&c_bin(), &content);
        let r = leftover_after_file(Path::new(RUST_BIN), &content);
        assert_eq!(
            c.len(),
            r.len(),
            "C26 leftover length differs for a {}-byte file (C left {}, Rust left {})",
            content.len(),
            c.len(),
            r.len()
        );
        assert!(c == r, "C26 leftover bytes differ for a {}-byte file", content.len());
    }
}

/// C27 — pipe stdin: the bytes still queued in the pipe must be identical.
#[test]
fn c27_pipe_bytes_left_behind() {
    for content in cases() {
        if content.len() >= 60_000 {
            continue;
        }
        let c = leftover_after_pipe(&c_bin(), &content);
        let r = leftover_after_pipe(Path::new(RUST_BIN), &content);
        assert_eq!(
            c.len(),
            r.len(),
            "C27 leftover length differs for a {}-byte pipe (C left {}, Rust left {})",
            content.len(),
            c.len(),
            r.len()
        );
        assert!(c == r, "C27 leftover bytes differ for a {}-byte pipe", content.len());
    }
}

/// Randomized version of both rows.
#[test]
fn c26_c27_randomized() {
    let mut rng = Rng::new(0x0FF5);
    for _ in 0..60 {
        let mut content = format!("{} {} {} ", rng.next_i32(), rng.next_i32(), rng.next_i32())
            .into_bytes();
        let tail = rng.below(9000) as usize;
        content.extend((0..tail).map(|_| *rng.choose(b"abc 0123456789\n")));
        let c = leftover_after_file(&c_bin(), &content);
        let r = leftover_after_file(Path::new(RUST_BIN), &content);
        assert_eq!(c, r, "randomized C26 leftover differs (len {})", content.len());
        let c = leftover_after_pipe(&c_bin(), &content);
        let r = leftover_after_pipe(Path::new(RUST_BIN), &content);
        assert_eq!(c, r, "randomized C27 leftover differs (len {})", content.len());
    }
}
