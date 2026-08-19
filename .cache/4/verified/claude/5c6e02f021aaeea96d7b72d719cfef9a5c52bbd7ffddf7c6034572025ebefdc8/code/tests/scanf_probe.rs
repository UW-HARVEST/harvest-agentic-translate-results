//! Supplementary white-box differential test for the `scanf("%d", &x)`
//! emulation in `src/translated.rs`.
//!
//! ## Why this test exists
//!
//! `c_src/src/main.c` does:
//!
//! ```c
//! int x = 0;
//! scanf("%d", &x);
//! if (x) { good(); } else { bad(); }
//! ```
//!
//! and **both** `good()` and `bad()` print `data[0]`, which is `0` in either
//! branch. The program's stdout is therefore `0\n` for *every* possible stdin,
//! which means the ~70 lines of scanf emulation in the Rust translation are
//! completely invisible to the black-box `main` corpus in
//! `tests/subprocess_diff.rs` (that corpus proves the *observable* behaviour
//! matches, and it does — see rows 20-37). A silent divergence in the scanner
//! would still be a translation bug, so it is compared directly here.
//!
//! ## How
//!
//! * C side: `tests/c_ref/scan_probe.c` re-issues the identical two statements
//!   (`int x = 0; scanf("%d", &x);`) and prints `"<retval> <x>\n"`. That file is
//!   a purpose-built reference harness — it is **not** part of `c_src/`, which is
//!   left untouched.
//! * Rust side: this test binary re-executes itself with `SCAN_PROBE=1` and runs
//!   `Scanner::scan_int` on stdin, printing the same two values.
//!
//! Both sides run in a fresh child process per input (a scanner reads and
//! buffers stdin, so it can only be exercised once per process).
//!
//! Note: unlike every other test here, the Rust side is reached through the
//! library crate rather than through `dlsym` on the `.so`. There is no C symbol
//! for this — `scanf` is glibc's, and the emulation is an internal helper of the
//! translation — so there is no `#[no_mangle]` export wrapper to exercise, and
//! adding a synthetic export would break the exact C/Rust symbol-set equality
//! that `tests/symbols.rs` asserts. All five real exported symbols are covered
//! through the `.so` in the other test files.
//!
//! Uses `harness = false` so that `main()` can dispatch into the probe role.

mod common;

// The translation is compiled straight into this test binary from the same file
// the cdylib and the `driver` binary use. It cannot be reached through the
// `driver` library crate here: that crate carries the `#[no_mangle] extern "C"
// fn main` export wrapper, which would collide with this test binary's own
// `main` symbol at link time.
#[allow(dead_code)]
#[path = "../src/translated.rs"]
mod translated;

use common::*;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

// ---------------------------------------------------------------------------
// probe role
// ---------------------------------------------------------------------------

fn probe_main() -> ! {
    // Mirrors `int x = 0; int r = scanf("%d", &x); printf("%d %d\n", r, x);`
    let mut scanner = translated::Scanner::new();
    let mut x: i32 = 0;
    let r = scanner.scan_int(&mut x);
    let mut out = std::io::stdout();
    let _ = out.write_all(format!("{r} {x}\n").as_bytes());
    let _ = out.flush();
    std::process::exit(0);
}

// ---------------------------------------------------------------------------
// parent
// ---------------------------------------------------------------------------

fn build_c_probe() -> PathBuf {
    let dir = manifest_dir().join("c_build");
    std::fs::create_dir_all(&dir).expect("create c_build");
    let src = manifest_dir().join("tests/c_ref/scan_probe.c");
    assert!(src.exists(), "missing {}", src.display());
    let out = dir.join("scan_probe_c");

    let fresh = match (out.metadata(), src.metadata()) {
        (Ok(o), Ok(s)) => matches!((o.modified(), s.modified()), (Ok(om), Ok(sm)) if om >= sm),
        _ => false,
    };
    if fresh {
        return out;
    }
    let tmp = dir.join(format!("scan_probe_c.{}.tmp", std::process::id()));
    let st = Command::new("gcc")
        .arg("-o")
        .arg(&tmp)
        .arg(&src)
        .output()
        .expect("spawn gcc");
    assert!(
        st.status.success(),
        "gcc failed: {}",
        String::from_utf8_lossy(&st.stderr)
    );
    let _ = std::fs::rename(&tmp, &out);
    let _ = std::fs::remove_file(&tmp);
    out
}

fn run_with_stdin(cmd: &mut Command, stdin_bytes: &[u8]) -> (Vec<u8>, Option<i32>) {
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(stdin_bytes)
        .ok();
    let out = child.wait_with_output().expect("wait");
    (out.stdout, out.status.code())
}

fn c_probe(exe: &Path, input: &[u8]) -> (Vec<u8>, Option<i32>) {
    run_with_stdin(&mut Command::new(exe), input)
}

fn rust_probe(input: &[u8]) -> (Vec<u8>, Option<i32>) {
    let me = std::env::current_exe().expect("current_exe");
    let mut cmd = Command::new(me);
    cmd.env("SCAN_PROBE", "1");
    run_with_stdin(&mut cmd, input)
}

fn extra_inputs() -> Vec<(&'static str, String, Vec<u8>)> {
    let mut v: Vec<(&'static str, String, Vec<u8>)> = Vec::new();
    let mut rng = Rng::new(SEED ^ 0x5CAF);

    // Dense fuzz over the alphabet that actually matters for %d.
    let alphabet: Vec<u8> = b"0123456789+-  \t\n\r\x0b\x0cxX.,eE'_%\xff\x01aZ".to_vec();
    for i in 0..1500 {
        let len = rng.range_usize(0, 20);
        let s: Vec<u8> = (0..len).map(|_| *rng.pick(&alphabet)).collect();
        v.push(("probe-fuzz", format!("fuzz #{i}"), s));
    }

    // Every i32 boundary and random i32/i64/i128 values rendered as decimal,
    // with and without an explicit sign and with random leading whitespace.
    let add_decimal = |v: &mut Vec<(&'static str, String, Vec<u8>)>, s: String| {
        v.push(("probe-decimal", format!("{s:?}"), s.clone().into_bytes()));
        v.push((
            "probe-decimal",
            format!("\" {s}\""),
            format!(" {s}").into_bytes(),
        ));
        v.push((
            "probe-decimal",
            format!("\"{s}\\n\""),
            format!("{s}\n").into_bytes(),
        ));
    };
    for k in 0..64u32 {
        let p = 1u128 << k;
        for d in [-1i128, 0, 1] {
            let m = p as i128 + d;
            add_decimal(&mut v, m.to_string());
            add_decimal(&mut v, (-m).to_string());
        }
    }
    for _ in 0..400 {
        add_decimal(&mut v, (rng.next_u64() as i64).to_string());
        add_decimal(&mut v, (rng.next_u32() as i32).to_string());
        let hi = rng.next_u64() as u128;
        let lo = rng.next_u64() as u128;
        add_decimal(&mut v, ((hi << 64) | lo).to_string());
    }
    // Digit runs around the 19/20-digit strtol saturation edge.
    for nd in [17usize, 18, 19, 20, 21, 25, 40, 200] {
        for lead in ["", "-", "+"] {
            let mut s = lead.to_string();
            s.push_str(&"9".repeat(nd));
            add_decimal(&mut v, s);
            let mut s = lead.to_string();
            s.push('1');
            s.push_str(&"0".repeat(nd - 1));
            add_decimal(&mut v, s);
            let mut s = lead.to_string();
            for _ in 0..nd {
                s.push((b'0' + rng.below(10) as u8) as char);
            }
            add_decimal(&mut v, s);
        }
    }
    v
}

fn main() {
    if std::env::var_os("SCAN_PROBE").is_some() {
        probe_main();
    }

    let c_exe = build_c_probe();

    let mut cases = stdin_corpus();
    cases.extend(extra_inputs());
    let total = cases.len();

    println!("\nrunning scanf probe over {total} inputs");
    let mut failures: Vec<String> = Vec::new();

    for (row, case, input) in cases {
        let (c_out, c_code) = c_probe(&c_exe, &input);
        let (r_out, r_code) = rust_probe(&input);
        if c_out != r_out || c_code != r_code {
            failures.push(format!(
                "[{row}] {case}: stdin={}\n    C    -> {:?} (exit {:?})\n    Rust -> {:?} (exit {:?})",
                show(&input),
                String::from_utf8_lossy(&c_out),
                c_code,
                String::from_utf8_lossy(&r_out),
                r_code
            ));
            if failures.len() >= 25 {
                break;
            }
        }
    }

    if failures.is_empty() {
        println!("test scanf_emulation_matches_glibc ... ok ({total} inputs)");
        println!("\ntest result: ok. 1 passed; 0 failed\n");
    } else {
        println!("test scanf_emulation_matches_glibc ... FAILED");
        println!("\n{} divergence(s):\n", failures.len());
        for f in &failures {
            println!("{f}");
        }
        println!("\ntest result: FAILED. 0 passed; 1 failed\n");
        std::process::exit(101);
    }
}
