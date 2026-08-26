//! Executable-level differential tests: `c_src/src/main.c` vs the `main` in
//! `src/lib.rs` (which is the entry point of the `driver` binary).
//!
//! Both programs are copied next to each other and invoked as `./driver` from
//! their own directory, so `argv[0]` -- which `--help` prints -- is identical.
//! stdout, stderr, the *merged* stream (to catch buffering / interleaving
//! differences) and the exit status are all compared byte for byte.

#![allow(
    clippy::bool_assert_comparison,
    clippy::same_item_push,
    clippy::manual_repeat_n,
    clippy::needless_range_loop,
    clippy::type_complexity
)]

mod common;

use common::*;
use std::ffi::{c_int, OsStr, OsString};
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};


/// `ln -sf`, used instead of copying the executables (see Runner::new).
fn link(src: &std::path::Path, dst: &std::path::Path) {
    let _ = std::fs::remove_file(dst);
    std::os::unix::fs::symlink(src, dst)
        .unwrap_or_else(|e| panic!("symlink {} -> {}: {e}", dst.display(), src.display()));
}

/// Spawning hundreds of children from a multi-threaded test process can
/// transiently fail with ETXTBSY; retry briefly.
fn spawn_retry<F>(mut f: F) -> std::process::Child
where
    F: FnMut() -> std::io::Result<std::process::Child>,
{
    let mut last = None;
    for _ in 0..200 {
        match f() {
            Ok(c) => return c,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::ExecutableFileBusy {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                    last = Some(e);
                    continue;
                }
                panic!("spawn driver: {e:?}");
            }
        }
    }
    panic!("spawn driver kept failing: {last:?}");
}

struct Runner {
    cdir: PathBuf,
    rdir: PathBuf,
    tmp: PathBuf,
    seq: std::cell::Cell<u64>,
}

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    status: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    merged: Vec<u8>,
}

impl Runner {
    fn new(tag: &str) -> Runner {
        let base = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
        let tmp = PathBuf::from(base).join(format!("driver_cli_{}_{tag}", std::process::id()));
        let cdir = tmp.join("c");
        let rdir = tmp.join("r");
        std::fs::create_dir_all(&cdir).unwrap();
        std::fs::create_dir_all(&rdir).unwrap();
        // Symlink (rather than copy) the binaries: the tests run in parallel
        // threads and spawn hundreds of processes, and opening an executable for
        // writing anywhere in the process races with concurrent `exec`
        // (ETXTBSY).  A symlink is enough -- argv[0] is still "./driver".
        link(&c_exe(), &cdir.join("driver"));
        link(&rust_exe(), &rdir.join("driver"));
        Runner {
            cdir,
            rdir,
            tmp,
            seq: std::cell::Cell::new(0),
        }
    }

    fn run_one(&self, dir: &PathBuf, args: &[OsString], stdin_data: &[u8]) -> Outcome {
        let n = self.seq.get();
        self.seq.set(n + 1);
        let merged_path = self.tmp.join(format!("merged_{n}"));
        // separate streams
        let mut child = spawn_retry(|| {
            Command::new("./driver")
                .current_dir(dir)
                .args(args)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
        });
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(stdin_data)
            .or_else(|e| {
                // the program may exit before reading stdin
                if e.kind() == std::io::ErrorKind::BrokenPipe {
                    Ok(())
                } else {
                    Err(e)
                }
            })
            .unwrap();
        drop(child.stdin.take());
        let out = child.wait_with_output().expect("wait driver");

        // merged stream (stdout and stderr share one fd, like a terminal/pipe)
        let f = std::fs::File::create(&merged_path).unwrap();
        let f2 = f.try_clone().unwrap();
        let mut fopt = Some((f, f2));
        let mut child = spawn_retry(|| {
            let (f, f2) = fopt.take().expect("stdio consumed");
            let res = Command::new("./driver")
                .current_dir(dir)
                .args(args)
                .stdin(Stdio::piped())
                .stdout(Stdio::from(f.try_clone().unwrap()))
                .stderr(Stdio::from(f2.try_clone().unwrap()))
                .spawn();
            fopt = Some((f, f2));
            res
        });
        let _ = child.stdin.as_mut().unwrap().write_all(stdin_data);
        drop(child.stdin.take());
        let _ = child.wait().expect("wait driver (merged)");
        let merged = std::fs::read(&merged_path).unwrap_or_default();
        let _ = std::fs::remove_file(&merged_path);

        Outcome {
            status: out.status.code(),
            stdout: out.stdout,
            stderr: out.stderr,
            merged,
        }
    }

    /// Run both programs and assert byte-identical behaviour.
    fn check(&self, args: &[&str], stdin_data: &[u8]) {
        let owned: Vec<OsString> = args.iter().map(|s| OsString::from(*s)).collect();
        self.check_os(&owned, stdin_data);
    }

    fn check_os(&self, args: &[OsString], stdin_data: &[u8]) {
        let c = self.run_one(&self.cdir, args, stdin_data);
        let r = self.run_one(&self.rdir, args, stdin_data);
        let ctx = format!(
            "argv={:?} stdin={:?}",
            args.iter()
                .map(|a| String::from_utf8_lossy(a.as_bytes()).to_string())
                .collect::<Vec<_>>(),
            String::from_utf8_lossy(&stdin_data[..stdin_data.len().min(120)])
        );
        assert_eq!(
            c.status, r.status,
            "exit status differs for {ctx}\nC stdout: {}\nR stdout: {}",
            String::from_utf8_lossy(&c.stdout),
            String::from_utf8_lossy(&r.stdout)
        );
        assert_eq!(
            String::from_utf8_lossy(&c.stdout),
            String::from_utf8_lossy(&r.stdout),
            "stdout differs for {ctx}"
        );
        assert_eq!(
            String::from_utf8_lossy(&c.stderr),
            String::from_utf8_lossy(&r.stderr),
            "stderr differs for {ctx}"
        );
        assert_eq!(
            String::from_utf8_lossy(&c.merged),
            String::from_utf8_lossy(&r.merged),
            "merged stdout+stderr differs for {ctx}"
        );
        assert_eq!(c.stdout, r.stdout, "stdout bytes differ for {ctx}");
        assert_eq!(c.stderr, r.stderr, "stderr bytes differ for {ctx}");
        assert_eq!(c.merged, r.merged, "merged bytes differ for {ctx}");
    }
}

/// CONFIGS.md row 56 -- argv shapes.
#[test]
fn cli_fixed_cases() {
    let run = Runner::new("fixed");
    let cases: Vec<Vec<&str>> = vec![
        vec![],
        vec!["--help"],
        vec!["--help", "1", "2", "3"],
        vec!["1", "2", "--help", "3"],
        vec!["--help", "--stdin"],
        vec!["--stdin"],
        vec!["--stdin", "--stdin"],
        vec![""],
        vec![" "],
        vec!["  12  "],
        vec![" 12"],
        vec!["\t12"],
        vec!["12 "],
        vec!["abc"],
        vec!["12abc"],
        vec!["0x10"],
        vec!["+-5"],
        vec!["-"],
        vec!["+"],
        vec!["-0"],
        vec!["+7"],
        vec!["007"],
        vec!["--Stdin", "--STDIN", "--help=x", "-h"],
        vec!["99999999999999999999", "3"],
        vec!["-99999999999999999999", "3"],
        vec!["2147483648", "3"],
        vec!["-2147483649", "3"],
        vec!["4294967296", "3"],
        vec!["2147483647"],
        vec!["-2147483648"],
        // opcode level cases (same programs as the library tests)
        vec!["10"],
        vec!["11"],
        vec!["-1"],
        vec!["0"],
        vec!["0", "42"],
        vec!["1"],
        vec!["0", "1", "1"],
        vec!["0", "1", "0", "2", "1"],
        vec!["2"],
        vec!["0", "3", "0", "4", "2"],
        vec!["3"],
        vec!["4"],
        vec!["0", "9", "4"],
        vec!["5"],
        vec!["0", "7", "5"],
        vec!["0", "-3", "5"],
        vec!["8"],
        vec!["0", "12345", "8", "8", "8"],
        vec!["6"],
        vec!["6", "1"],
        vec!["0", "1", "6", "99"],
        vec!["0", "1", "6", "-1", "0", "5"],
        vec!["0", "0", "6", "2", "0", "7", "0", "8"],
        vec!["0", "1", "6", "0", "0", "8"],
        vec!["0", "1", "6", "2", "0", "7", "0", "8"],
        vec!["0", "1", "6", "4", "0", "7", "0", "8"],
        vec!["7"],
        vec!["7", "3"],
        vec!["7", "0", "3"],
        vec!["7", "-5", "3"],
        vec!["7", "4", "3"],
        vec!["0", "6", "7", "3", "5"],
        vec!["7", "3", "0"],
        vec!["7", "2", "10"],
        vec!["7", "2", "11"],
        vec!["7", "2", "7"],
        vec!["9"],
        vec!["9", "-1"],
        vec!["9", "1"],
        vec!["9", "0"],
        vec!["0", "5", "9", "1"],
        vec!["0", "5", "0", "6", "9", "2"],
        vec!["0", "1", "0", "2", "0", "3", "0", "4", "9", "4"],
        vec!["0", "1", "0", "2", "0", "3", "0", "4", "9", "2"],
        vec!["0", "-7", "0", "-8", "9", "2"],
        vec!["0", "2147483647", "0", "-2147483648", "9", "2"],
        vec![
            "0", "100", "3", "3", "3", "3", "9", "3", "9", "2", "5", "8", "1", "2", "10",
        ],
    ];
    for case in cases {
        run.check(&case, b"");
    }
    // a non-UTF-8 argument
    let bad = OsString::from(OsStr::from_bytes(b"\xff\xfe\x80"));
    run.check_os(&[bad, OsString::from("3")], b"");
    // an argument with an embedded newline
    run.check(&["3\n4"], b"");
}

/// CONFIGS.md row 57 -- stdin shapes.
#[test]
fn cli_stdin_shapes() {
    let run = Runner::new("stdin");
    let long_line = {
        let mut v = Vec::new();
        for _ in 0..1200 {
            v.extend_from_slice(b"3 ");
        }
        v.extend_from_slice(b"1\n");
        v
    };
    let split_token = {
        // a 4090 digit number straddling the 4096 byte fgets boundary
        let mut v = Vec::new();
        v.extend_from_slice(b"0 ");
        v.extend(std::iter::repeat(b'1').take(4090));
        v.extend_from_slice(b" 3 3\n");
        v
    };
    let huge_ws = {
        let mut v = Vec::new();
        v.extend(std::iter::repeat(b' ').take(5000));
        v.extend_from_slice(b"0 5 9 1\n");
        v
    };
    let inputs: Vec<(&str, Vec<u8>)> = vec![
        ("empty", b"".to_vec()),
        ("newline", b"\n".to_vec()),
        ("newlines", b"\n\n\n".to_vec()),
        ("spaces", b"    ".to_vec()),
        ("simple", b"0 5 1 10\n".to_vec()),
        ("multiline", b"0 5\n1\n10\n".to_vec()),
        ("crlf", b"0\t5\r\n3 3 1 1\n".to_vec()),
        ("no trailing nl", b"  0   5   9 2  ".to_vec()),
        ("junk", b"abc 5 12x 7\n".to_vec()),
        (
            "overflow",
            b"99999999999999999999 -99999999999999999999 5\n".to_vec(),
        ),
        ("nul first", b"\0 0 5 1\n".to_vec()),
        ("nul middle", b"0 5 \0 1 1\n0 6 1\n".to_vec()),
        ("only nul", b"\0".to_vec()),
        ("long line", long_line),
        ("split token", split_token),
        ("leading whitespace", huge_ws),
        ("tabs only", b"\t\t\t\n".to_vec()),
        ("mixed", b" 0 12 \t 5 \r 8 \n 9 2 \n 10 \n".to_vec()),
    ];
    for (name, data) in inputs {
        run.check(&["--stdin"], &data);
        run.check(&["--stdin", "0", "77"], &data);
        run.check(&["0", "77"], &data); // stdin ignored without --stdin
        run.check(&["--stdin", "--help"], &data);
        let _ = name;
    }
}

/// CONFIGS.md row 58 -- randomised programs through argv and through stdin.
#[test]
fn cli_random_programs() {
    let run = Runner::new("random");
    let mut rng = Rng::new(0xC11_5EED);
    for _ in 0..400 {
        let code: Vec<c_int> = rng.program(14);
        let args: Vec<String> = code.iter().map(|v| v.to_string()).collect();
        let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        run.check(&refs, b"");
    }
    for _ in 0..100 {
        let code: Vec<c_int> = rng.program(20);
        let mut data = Vec::new();
        for (i, v) in code.iter().enumerate() {
            data.extend_from_slice(v.to_string().as_bytes());
            data.push(if i % 4 == 3 { b'\n' } else { b' ' });
        }
        data.push(b'\n');
        run.check(&["--stdin"], &data);
    }
    // completely unbiased garbage on the command line
    for _ in 0..100 {
        let n = 1 + rng.below(6) as usize;
        let args: Vec<String> = (0..n).map(|_| rng.value().to_string()).collect();
        let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        run.check(&refs, b"");
    }
}
