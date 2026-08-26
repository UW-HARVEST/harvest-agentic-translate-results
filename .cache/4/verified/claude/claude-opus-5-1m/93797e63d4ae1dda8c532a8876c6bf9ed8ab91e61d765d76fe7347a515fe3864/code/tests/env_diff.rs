//! Phase B (rows M22–M25) — environment / plumbing configurations that the C
//! runtime is sensitive to: locale (`%.1f`'s decimal point and `strtol`'s
//! whitespace class), command-line arguments (`main` takes none), and stdout /
//! stdin being regular files instead of pipes (full vs line buffering in the C
//! implementation).

mod common;
use common::*;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn tmp(tag: &str) -> PathBuf {
    let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(dir).join(format!("envdiff-{}-{}.bin", std::process::id(), tag))
}

/// Runs an executable with a chosen environment and argv, stdin from a file and
/// stdout redirected into a regular file; returns (stdout bytes, exit code).
fn run_with(
    path: &Path,
    input: &[u8],
    envs: &[(&str, &str)],
    args: &[&str],
    stdout_to_file: bool,
) -> (Vec<u8>, Option<i32>, Vec<u8>) {
    let in_path = tmp("in");
    std::fs::write(&in_path, input).unwrap();
    let stdin_file = std::fs::File::open(&in_path).unwrap();

    let out_path = tmp("out");
    let mut cmd = Command::new(path);
    cmd.args(args).stdin(Stdio::from(stdin_file)).stderr(Stdio::piped());
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let (stdout, code, stderr);
    if stdout_to_file {
        let f = std::fs::File::create(&out_path).unwrap();
        cmd.stdout(Stdio::from(f));
        let out = cmd.spawn().unwrap().wait_with_output().unwrap();
        stdout = std::fs::read(&out_path).unwrap();
        code = out.status.code();
        stderr = out.stderr;
    } else {
        cmd.stdout(Stdio::piped());
        let out = cmd.spawn().unwrap().wait_with_output().unwrap();
        stdout = out.stdout;
        code = out.status.code();
        stderr = out.stderr;
    }
    let _ = std::fs::remove_file(&in_path);
    let _ = std::fs::remove_file(&out_path);
    (stdout, code, stderr)
}

fn compare(input: &[u8], envs: &[(&str, &str)], args: &[&str], to_file: bool, ctx: &str) {
    let c = run_with(&c_exe(), input, envs, args, to_file);
    let r = run_with(&rust_exe(), input, envs, args, to_file);
    assert_eq!(
        (
            String::from_utf8_lossy(&c.0).to_string(),
            c.1,
            String::from_utf8_lossy(&c.2).to_string()
        ),
        (
            String::from_utf8_lossy(&r.0).to_string(),
            r.1,
            String::from_utf8_lossy(&r.2).to_string()
        ),
        "[{}] mismatch for input {:?}",
        ctx,
        String::from_utf8_lossy(input)
    );
    assert!(!c.0.is_empty(), "[{}] C produced no output", ctx);
}

// ---------------------------------------------------------------------------
// M22 — locale environment variables (the program never calls setlocale, so
// `%.1f` must keep using '.' and strtol the "C" whitespace class)
// ---------------------------------------------------------------------------
#[test]
fn cfg_m22_locale_env() {
    let locales = [
        "C",
        "POSIX",
        "en_US.UTF-8",
        "de_DE.UTF-8",
        "fr_FR.UTF-8",
        "tr_TR.UTF-8",
        "ru_RU.UTF-8",
        "not_a_locale.42",
        "",
    ];
    for loc in locales {
        for input in [&b"7\n"[..], &b"-2147483648\n"[..], &b"abc\n"[..], &b""[..]] {
            compare(
                input,
                &[
                    ("LC_ALL", loc),
                    ("LC_NUMERIC", loc),
                    ("LANG", loc),
                    ("LANGUAGE", loc),
                ],
                &[],
                false,
                &format!("M22 locale {:?}", loc),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// M23 — argv is ignored by `int main(void)`
// ---------------------------------------------------------------------------
#[test]
fn cfg_m23_arguments_ignored() {
    for args in [
        vec![],
        vec!["5"],
        vec!["--help"],
        vec!["-x", "-y", "-z"],
        vec!["", ""],
        vec!["a b c", "\n"],
    ] {
        for input in [&b"7\n"[..], &b"zzz\n"[..]] {
            compare(input, &[], &args, false, &format!("M23 args {:?}", args));
        }
    }
}

// ---------------------------------------------------------------------------
// M24 — stdout is a regular file (fully buffered in C) instead of a pipe
// ---------------------------------------------------------------------------
#[test]
fn cfg_m24_stdout_to_file() {
    let inputs: Vec<Vec<u8>> = vec![
        b"7\n".to_vec(),
        b"0\n".to_vec(),
        b"-2147483648\n".to_vec(),
        b"2147483648\n".to_vec(),
        b"abc\n".to_vec(),
        Vec::new(),
        b"  42xyz".to_vec(),
    ];
    for (i, inp) in inputs.iter().enumerate() {
        compare(inp, &[], &[], true, &format!("M24 file #{}", i));
    }
}

// ---------------------------------------------------------------------------
// M25 — stdin is a regular file (seekable, readable in one go) rather than a
// pipe: fgets' read-ahead behaviour differs, the output must not
// ---------------------------------------------------------------------------
#[test]
fn cfg_m25_stdin_from_file() {
    // run_with always feeds stdin from a file, so compare a matrix of shapes
    let mut inputs: Vec<Vec<u8>> = vec![
        b"7\n9\n".to_vec(),
        b"7".to_vec(),
        b"\n".to_vec(),
        vec![b'1'; 250],
    ];
    inputs.push(format!("{}\n", "5".repeat(99)).into_bytes());
    inputs.push(format!(" {}\n", "0".repeat(98)).into_bytes());
    for (i, inp) in inputs.iter().enumerate() {
        compare(inp, &[], &[], false, &format!("M25 stdin file #{}", i));
        compare(inp, &[], &[], true, &format!("M25 stdin+stdout file #{}", i));
    }
}

// ---------------------------------------------------------------------------
// M26 — stdin delivered in slow chunks: fgets must keep reading until it sees
// a newline / EOF, across several read() syscalls
// ---------------------------------------------------------------------------
fn run_exe_chunked(path: &Path, chunks: &[&[u8]], close_without_newline: bool) -> (Vec<u8>, Option<i32>) {
    use std::io::Write;
    use std::time::Duration;
    let mut child = Command::new(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    {
        let mut si = child.stdin.take().unwrap();
        for c in chunks {
            let _ = si.write_all(c);
            let _ = si.flush();
            std::thread::sleep(Duration::from_millis(20));
        }
        if !close_without_newline {
            let _ = si.write_all(b"\n");
            let _ = si.flush();
        }
        // dropping `si` closes the pipe -> EOF for the child
    }
    let out = child.wait_with_output().unwrap();
    (out.stdout, out.status.code())
}

#[test]
fn cfg_m26_chunked_stdin() {
    let cases: Vec<(Vec<&[u8]>, bool)> = vec![
        (vec![b"4", b"2"], false),
        (vec![b"4", b"2"], true),
        (vec![b"-", b"1", b"7"], false),
        (vec![b" ", b" ", b"9"], false),
        (vec![b"a", b"b"], false),
        (vec![b"1", b"2", b"3", b"4", b"5"], true),
        (vec![b""], true),
        (vec![b"7\n", b"8\n"], true),
    ];
    for (i, (chunks, no_nl)) in cases.iter().enumerate() {
        let c = run_exe_chunked(&c_exe(), chunks, *no_nl);
        let r = run_exe_chunked(&rust_exe(), chunks, *no_nl);
        assert_eq!(
            (String::from_utf8_lossy(&c.0).to_string(), c.1),
            (String::from_utf8_lossy(&r.0).to_string(), r.1),
            "M26 chunked #{} ({:?}, no_newline={})",
            i,
            chunks,
            no_nl
        );
        assert!(!c.0.is_empty(), "M26 #{}: C produced no output", i);
    }
}
