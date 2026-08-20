//! Phase B/C/D through the **FFI boundary**: both the C source and the Rust
//! translation are built as shared objects, loaded with `libloading`, and their
//! exported `main` symbols are called and compared.
//!
//! `SYMBOLS.md` records that `main` is the only symbol this library exports, so
//! `main` *is* the lowest-level entry point available; the Rust side is never
//! called directly, only through `dlopen`/`dlsym` on `libdriver.so`, which also
//! exercises the `#[no_mangle] extern "C"` export wrapper in `src/lib.rs`.
//!
//! `main` reads fd 0 and writes fd 1, so each invocation happens in a `fork()`ed
//! child with those descriptors redirected to temp files. dlopen/dlsym are done
//! *before* the fork, so the child only performs `open`/`dup2`, the call, and
//! `_exit`. Both tests hold `fd_lock()`, so no two of them fork concurrently.
//!
//! Covers `CONFIGS.md` rows 32-33 and re-checks every input shape from rows
//! 1-23 plus the NUL / error-shaped inputs of `ERRORS.md` rows 5-10 across the
//! FFI boundary.

mod common;

use common::*;
use libloading::{Library, Symbol};
use std::path::Path;

/// `int main(void)` - the C signature, exactly as exported by both `.so`s.
type MainFn = unsafe extern "C" fn() -> libc::c_int;

/// dlopen a shared object and resolve its `main`.
fn load(path: &Path) -> (Library, MainFn) {
    let lib = unsafe { Library::new(path) }
        .unwrap_or_else(|e| panic!("dlopen({}) failed: {}", path.display(), e));
    let f = {
        let sym: Symbol<MainFn> = unsafe { lib.get(b"main\0") }.unwrap_or_else(|e| {
            panic!("dlsym(main) failed in {}: {}", path.display(), e)
        });
        *sym
    };
    (lib, f)
}

/// What one FFI invocation produced.
#[derive(PartialEq, Eq)]
struct SoRun {
    /// The `int` returned by `main`, or `Err(signal)` if the child died.
    ret: Result<i32, i32>,
    stdout: Vec<u8>,
}

impl std::fmt::Debug for SoRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SoRun {{ ret: {:?}, stdout: {} bytes {:?} }}",
            self.ret,
            self.stdout.len(),
            Preview(&self.stdout)
        )
    }
}

/// Call `main` from a loaded `.so` with fd 0 fed from `input` and fd 1 captured.
///
/// `repeat` says how many times to call `main` inside the same child, sharing
/// the same descriptors (used by `so_differential_repeat`).
fn call_main(f: MainFn, input: &[u8], repeat: usize) -> SoRun {
    let in_path = write_temp("so-in", input);
    let out_path = temp_path("so-out");
    std::fs::File::create(&out_path).expect("create out temp");

    // Prepared before the fork: no allocation in the child before `main`.
    let cin = cstr(&in_path);
    let cout = cstr(&out_path);

    let _g = fd_lock().lock().unwrap();

    let ret = unsafe {
        let pid = libc::fork();
        assert!(pid >= 0, "fork() failed");
        if pid == 0 {
            // ---- child ----
            let fd_in = libc::open(cin.as_ptr(), libc::O_RDONLY);
            if fd_in < 0 {
                libc::_exit(101);
            }
            if libc::dup2(fd_in, 0) < 0 {
                libc::_exit(102);
            }
            let fd_out = libc::open(cout.as_ptr(), libc::O_WRONLY | libc::O_TRUNC);
            if fd_out < 0 {
                libc::_exit(103);
            }
            if libc::dup2(fd_out, 1) < 0 {
                libc::_exit(104);
            }

            let mut rc = 0;
            for _ in 0..repeat {
                rc = f();
                // The C runtime flushes stdout when `main` returns from a real
                // program; a dlopen()ing caller has to do it by hand. Harmless
                // for the Rust side, which flushes its own buffer itself.
                libc::fflush(std::ptr::null_mut());
            }
            libc::_exit(rc as i32);
        }
        // ---- parent ----
        let mut status: libc::c_int = 0;
        let w = libc::waitpid(pid, &mut status, 0);
        assert_eq!(w, pid, "waitpid");
        if libc::WIFEXITED(status) {
            Ok(libc::WEXITSTATUS(status))
        } else if libc::WIFSIGNALED(status) {
            Err(libc::WTERMSIG(status))
        } else {
            Err(-1)
        }
    };

    let stdout = std::fs::read(&out_path).expect("read out temp");
    let _ = std::fs::remove_file(&in_path);
    let _ = std::fs::remove_file(&out_path);
    SoRun { ret, stdout }
}

/// Every input shape from `CONFIGS.md`, as (label, bytes).
fn cases() -> Vec<(String, Vec<u8>)> {
    let mut v: Vec<(String, Vec<u8>)> = Vec::new();
    let mut push = |n: &str, b: Vec<u8>| v.push((n.to_string(), b));

    // rows 1-5
    push("row01_empty", vec![]);
    push("row02_single_newline", b"\n".to_vec());
    push("row02_many_newlines", vec![b'\n'; 300]);
    push("row03_one_line_nl", b"hello\n".to_vec());
    push("row04_one_line_no_nl", b"hello".to_vec());
    push(
        "row05_many_short_lines",
        (0..200).flat_map(|i| format!("line {}\n", i).into_bytes()).collect(),
    );
    // rows 7-11: the 127-byte chunk boundary
    for len in [1usize, 125, 126, 127, 128, 129, 253, 254, 255, 300, 381] {
        let mut a = vec![b'a'; len];
        push(&format!("row07_11_len{}_no_nl", len), a.clone());
        a.push(b'\n');
        push(&format!("row07_11_len{}_nl", len), a);
    }
    // rows 12-16: NUL handling
    push("row12_nul_middle", b"ab\x00cd\nxy\n".to_vec());
    push("row13_nul_leading", b"\x00abc\ndef\n".to_vec());
    push("row14_nul_before_nl", b"abc\x00\ndef\n".to_vec());
    for at in [125usize, 126, 127, 128] {
        let mut b = vec![b'a'; at];
        b.push(0);
        b.extend_from_slice(b"tail\nsecond\n");
        push(&format!("row15_nul_at_{}", at), b);
    }
    for n in [1usize, 127, 128, 300] {
        push(&format!("row16_all_nuls_{}", n), vec![0u8; n]);
    }
    // rows 18-20
    push("row18_crlf", b"line1\r\nline2\r\n".to_vec());
    push("row19_all_bytes", (0..=255u8).collect());
    push(
        "row19_all_bytes_x10",
        (0..10).flat_map(|_| (0..=255u8).collect::<Vec<u8>>()).collect(),
    );
    push("row20_invalid_utf8", b"\xff\xfe\xc3\x28\n\x80\x81\n".to_vec());
    // rows 22-23: stdio buffer boundary and multi-block
    for len in [4095usize, 4096, 4097, 8192] {
        push(&format!("row22_len{}", len), vec![b'k'; len]);
        let mut lined = Vec::with_capacity(len);
        while lined.len() < len {
            lined.extend_from_slice(b"0123456789012345678901234567890123456789012345678\n");
        }
        lined.truncate(len);
        push(&format!("row22_lined_len{}", len), lined);
    }
    push("row23_large_1mb_lines", {
        let mut b = Vec::with_capacity(1_000_000);
        while b.len() < 1_000_000 {
            b.extend_from_slice(b"the quick brown fox jumps over the lazy dog\n");
        }
        b
    });
    push("row23_large_1mb_no_nl", vec![b'w'; 1_000_000]);

    // rows 6/17/21: randomized, fixed seed
    let mut rng = Rng::new(SEED ^ 0x50);
    for case in 0..120 {
        let len = rng.below(900) as usize;
        let mode = rng.below(3);
        let b: Vec<u8> = (0..len)
            .map(|_| match mode {
                0 => rng.byte(),                                   // pure binary
                1 => {
                    if rng.bool_pct(12) {
                        b'\n'
                    } else if rng.bool_pct(12) {
                        0
                    } else {
                        b'A' + (rng.below(26) as u8)
                    }
                }
                _ => {
                    if rng.bool_pct(8) {
                        b'\n'
                    } else {
                        b'a' + (rng.below(26) as u8)
                    }
                }
            })
            .collect();
        push(&format!("row06_17_21_random{}", case), b);
    }
    v
}

/// `CONFIGS.md` row 32 + the FFI half of `ERRORS.md` row 15: for every input
/// shape, the C `.so` and the Rust `.so` must return the same `int` and write
/// the same bytes to fd 1.
#[test]
fn so_differential_all() {
    let (_clib, cmain) = load(&c_so());
    let (_rlib, rmain) = load(&rust_so());

    let all = cases();
    assert!(all.len() > 100, "expected a broad case list, got {}", all.len());

    for (name, input) in &all {
        let c = call_main(cmain, input, 1);
        let r = call_main(rmain, input, 1);
        assert_eq!(
            c, r,
            "\nFFI divergence in case {} ({} bytes: {:?})\n  C .so : {:?}\n  Rust  : {:?}\n  first differing byte: {:?}\n",
            name,
            input.len(),
            Preview(input),
            c,
            r,
            first_diff(&c.stdout, &r.stdout)
        );
        // `main` must always return exactly 0 (ERRORS.md row 15).
        assert_eq!(c.ret, Ok(0), "C main returned non-zero for {}", name);
        assert_eq!(r.ret, Ok(0), "Rust main returned non-zero for {}", name);
        // and the C ground truth must match the fgets/fputs model.
        assert_eq!(
            c.stdout,
            model(input),
            "case {}: C .so output disagrees with the fgets/fputs model",
            name
        );
    }
    eprintln!("so_differential_all: {} cases compared via dlopen", all.len());
}

/// `CONFIGS.md` row 33: calling the exported `main` twice inside the same loaded
/// image must not leak state or double-emit. The descriptors are deliberately
/// *not* reset between the calls, so on the second call the stream is genuinely
/// at end of input - which is the only situation a single-`main` program can
/// ever be in - and both sides must agree that it produces nothing more.
#[test]
fn so_differential_repeat() {
    let (_clib, cmain) = load(&c_so());
    let (_rlib, rmain) = load(&rust_so());

    for input in [
        &b""[..],
        &b"one\n"[..],
        &b"one\ntwo\nthree"[..],
        &b"\x00nul\nafter\n"[..],
        &vec![b'r'; 500][..],
    ] {
        let c1 = call_main(cmain, input, 1);
        let r1 = call_main(rmain, input, 1);
        assert_eq!(c1, r1, "single call diverged for {:?}", Preview(input));

        let c2 = call_main(cmain, input, 2);
        let r2 = call_main(rmain, input, 2);
        assert_eq!(
            c2, r2,
            "second call diverged for {:?}\n  C: {:?}\n  R: {:?}",
            Preview(input),
            c2,
            r2
        );
        // Calling twice must not produce more output than calling once.
        assert_eq!(
            c2.stdout, c1.stdout,
            "sanity: the C .so emits nothing extra on a second call"
        );
        assert_eq!(c2.ret, Ok(0));
        assert_eq!(r2.ret, Ok(0));
    }
}
