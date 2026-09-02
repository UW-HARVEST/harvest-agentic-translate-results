// Characterisation of the one place where the C program is not a function of
// its input: a completely full `in[1000]` with no NUL byte inside it.
//
// `main` declares `char in[1000] = "";` and the compiler lays it out at
// `rbp-0x3f0` with size `0x3e8`, so the buffer ends exactly at `rbp-8`. The
// initialiser zeroes precisely those 1000 bytes; the 8 bytes at `rbp-8..rbp-1`
// are frame padding that is never written, and `[rbp]` holds the saved frame
// pointer, i.e. a stack address that ASLR randomises on every run.
//
// When `fread` fills all 1000 bytes and none of them is NUL, `strchr` keeps
// scanning into that padding and into the saved `rbp`, so `foo` can report one
// extra 'A' or 'x' depending on bytes the input does not control. Repeated runs
// of the *same* C binary on the *same* input disagree a couple of percent of the
// time.
//
// This test pins down what the Rust program must do about it: reproduce the
// stable, in-bounds answer, and never vary.

mod common;

use common::{c_bin, rust_bin};
use std::io::Write;
use std::process::{Command, Stdio};

const BUF: usize = 1000;
const RUNS: usize = 60;

fn stdout_of(bin: &std::path::Path, input: &[u8]) -> Vec<u8> {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let mut sink = child.stdin.take().expect("piped stdin");
    let data = input.to_vec();
    let w = std::thread::spawn(move || {
        let _ = sink.write_all(&data);
    });
    let out = child.wait_with_output().expect("wait");
    let _ = w.join();
    out.stdout
}

/// Parse "A: <n>\nx: <m>\n".
fn counts(out: &[u8]) -> (i64, i64) {
    let s = String::from_utf8_lossy(out);
    let mut it = s.lines();
    let a = it.next().expect("A line");
    let x = it.next().expect("x line");
    let a = a.strip_prefix("A: ").expect("A prefix").parse().expect("int");
    let x = x.strip_prefix("x: ").expect("x prefix").parse().expect("int");
    (a, x)
}

#[test]
fn full_buffer_rust_is_stable_and_matches_the_in_bounds_answer() {
    // 1000 'A', no NUL anywhere: the in-bounds answer is A: 1000 / x: 0.
    let input = vec![b'A'; BUF];
    let expected = b"A: 1000\nx: 0\n".to_vec();

    // The Rust program never varies.
    for _ in 0..RUNS {
        assert_eq!(
            stdout_of(&rust_bin(), &input),
            expected,
            "Rust must always print the in-bounds answer for a full buffer"
        );
    }

    // The C program agrees most of the time, and when it does not, it only ever
    // over-counts: the out-of-bounds bytes can add matches, never remove them.
    let mut agree = 0usize;
    for _ in 0..RUNS {
        let got = stdout_of(c_bin(), &input);
        if got == expected {
            agree += 1;
        } else {
            let (a, x) = counts(&got);
            assert!(
                a >= 1000 && x >= 0 && a <= 1000 + 16 && x <= 16,
                "unexpected C output for a full buffer: {:?}",
                String::from_utf8_lossy(&got)
            );
        }
    }
    assert!(
        agree * 2 > RUNS,
        "the in-bounds answer should be the C program's usual output, got it \
         {agree}/{RUNS} times"
    );
}

#[test]
fn under_full_buffer_the_c_program_is_deterministic() {
    // 999 bytes: in[999] is still the zero fill, so there is no out-of-bounds
    // read and the C program's output never varies.
    let input = vec![b'A'; BUF - 1];
    let first = stdout_of(c_bin(), &input);
    assert_eq!(first, b"A: 999\nx: 0\n".to_vec());
    for _ in 0..RUNS {
        assert_eq!(
            stdout_of(c_bin(), &input),
            first,
            "C output must be stable when it stays inside its buffer"
        );
        assert_eq!(stdout_of(&rust_bin(), &input), first);
    }
}
