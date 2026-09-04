//! Differential tests for behaviour that depends on how the standard streams
//! are wired up rather than on the input bytes: `stdout` buffering relative to
//! the unbuffered `stderr`, and the process dying from `SIGPIPE`.

mod common;

use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use common::{c_binary, data_path, rust_binary};

/// Run `binary` with both `stdout` and `stderr` pointing at the *same* file, so
/// that the relative order of the block-buffered `stdout` writes and the
/// unbuffered `stderr` writes becomes observable.
fn run_merged(binary: &Path, input: &[u8]) -> (Option<i32>, Vec<u8>) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/difftest-data");
    std::fs::create_dir_all(&dir).expect("cannot create the scratch directory");
    let path = dir.join(format!(
        "merged-{}-{:?}",
        binary.file_name().and_then(|n| n.to_str()).unwrap_or("bin"),
        std::thread::current().id()
    ));

    let file = std::fs::File::create(&path).expect("cannot create the scratch file");
    let dup = file.try_clone().expect("cannot duplicate the scratch file");

    let mut child = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::from(file))
        .stderr(Stdio::from(dup))
        .spawn()
        .unwrap_or_else(|e| panic!("cannot spawn {}: {e}", binary.display()));

    {
        let mut stdin = child.stdin.take().expect("stdin was piped");
        let data = input.to_vec();
        std::thread::spawn(move || {
            let _ = stdin.write_all(&data);
        });
    }
    let status = child.wait().expect("the child failed to run");

    let mut merged = Vec::new();
    let mut file = std::fs::File::open(&path).expect("cannot reopen the scratch file");
    file.seek(SeekFrom::Start(0)).expect("seek failed");
    file.read_to_end(&mut merged).expect("read failed");
    let _ = std::fs::remove_file(&path);

    (status.code(), merged)
}

#[track_caller]
fn assert_same_merged(name: &str, input: &[u8]) {
    let (c_status, c_out) = run_merged(c_binary(), input);
    let (r_status, r_out) = run_merged(rust_binary(), input);
    assert_eq!(
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out),
        "[{name}] merged stdout+stderr differs"
    );
    assert_eq!(c_status, r_status, "[{name}] exit status differs");
}

#[test]
fn stderr_interleaves_with_the_buffered_stdout() {
    assert_same_merged("two_open_errors", b"2\n/nonexistent\n2\n/also-not-here\n7\n");
    assert_same_merged(
        "file_too_large",
        format!("2\n{}\n7\n", data_path("size8193.txt")).as_bytes(),
    );
    assert_same_merged(
        "input_text_too_large",
        format!("2\n{}\n3\n7\n", data_path("size8192.txt")).as_bytes(),
    );
}

#[test]
fn stderr_after_more_than_one_buffer_of_stdout() {
    // Force stdout past the 4096-byte block boundary before the error message.
    let mut input = Vec::from(&b"6\n"[..]);
    for _ in 0..40 {
        input.extend_from_slice(b"aaaa bbbb cccc\n");
    }
    input.extend_from_slice(b"\n2\n/nonexistent\n7\n");
    assert_same_merged("big_then_error", &input);
}

#[test]
fn dies_from_sigpipe_when_stdout_is_closed() {
    use std::os::unix::process::ExitStatusExt;

    let mut input = Vec::from(&b"6\n"[..]);
    for _ in 0..200 {
        input.extend_from_slice(b"aaaa bbbb cccc dddd\n");
    }
    input.extend_from_slice(b"\n7\n");

    let outcome = |binary: &Path| {
        let mut child = Command::new(binary)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("cannot spawn {}: {e}", binary.display()));

        // Drop the read end of the stdout pipe immediately: the child gets
        // SIGPIPE as soon as it flushes.
        drop(child.stdout.take());

        {
            let mut stdin = child.stdin.take().expect("stdin was piped");
            let data = input.clone();
            std::thread::spawn(move || {
                let _ = stdin.write_all(&data);
            });
        }
        let status = child.wait().expect("the child failed to run");
        (status.code(), status.signal())
    };

    assert_eq!(
        outcome(c_binary()),
        outcome(rust_binary()),
        "SIGPIPE handling differs"
    );
}
