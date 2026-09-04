//! Randomized differential fuzzing plus process-level behavior that a fixed
//! input list cannot reach (argv, closed stdin, closed stdout / SIGPIPE).

mod harness;

use harness::{assert_same, c_bin, run_one, rust_bin, same};

use std::io::Write;
use std::process::{Command, Stdio};

/// Small deterministic PRNG so failures are reproducible without a dev-dependency.
struct Rng(u64);

impl Rng {
    fn next_u32(&mut self) -> u32 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 32) as u32
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next_u32() as usize) % n
    }
    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
}

const SEPS: [&str; 8] = [" ", "\n", "  ", "\t", "\r\n", " \n ", "\x0b", "\x0c"];

const ODD_TOKENS: [&str; 20] = [
    "abc", "-", "+", "x", ".", "0x10", "3.5", "99999999999999999999",
    "-99999999999999999999", "2147483648", "-2147483648", "4294967296",
    "0001", "+7", "256", "257", "-1", "1e5", "--5", "1_2",
];

fn number(rng: &mut Rng) -> String {
    match rng.below(100) {
        0..=54 => (rng.below(264) as i64 - 3).to_string(),
        55..=69 => rng
            .pick(&[0i64, 1, 2, 255, 256, 257, 100, 101, -1, -2, 2147483647, -2147483648])
            .to_string(),
        70..=84 => (*rng.pick(&ODD_TOKENS)).to_string(),
        _ => {
            // A long decimal literal, well past what an int (or a long) holds,
            // to exercise scanf's saturate-then-truncate conversion.
            let digits = 10 + rng.below(15);
            let mut s = String::new();
            if rng.below(2) == 0 {
                s.push('-');
            }
            s.push((b'1' + rng.below(9) as u8) as char);
            for _ in 1..digits {
                s.push((b'0' + rng.below(10) as u8) as char);
            }
            s
        }
    }
}

/// Mostly well-formed input for a random operation, with occasional corruption.
fn structured(rng: &mut Rng) -> String {
    let op = *rng.pick(&[0i64, 1, 2, 3, 4, 5, 6, 7, -1, 99]);
    let n = *rng.pick(&[1i64, 1, 2, 2, 3, 5, 100, 0, -1, 101]);
    let mut toks = vec![op.to_string(), n.to_string()];
    for _ in 0..n.clamp(0, 101) {
        let ln = *rng.pick(&[0i64, 1, 2, 3, 5, 128, 129, 200, 256, 257, -1]);
        toks.push(ln.to_string());
        for _ in 0..ln.max(0) {
            toks.push(rng.pick(&[0i64, 1, 127, 128, 255, 256, -1]).to_string());
        }
    }
    if op == 3 || op == 5 {
        toks.push(number(rng));
    }
    match rng.below(100) {
        0..=11 => toks.truncate(rng.below(toks.len() + 1)),
        12..=19 => {
            let at = 1 + rng.below(toks.len());
            toks.insert(at, (*rng.pick(&ODD_TOKENS)).to_string());
        }
        20..=24 => toks.push((*rng.pick(&ODD_TOKENS)).to_string()),
        _ => {}
    }
    toks.into_iter()
        .map(|t| t + rng.pick(&SEPS))
        .collect::<String>()
}

/// Free-form token soup.
fn unstructured(rng: &mut Rng) -> String {
    let n = rng.below(13);
    (0..n)
        .map(|_| number(rng) + rng.pick(&SEPS))
        .collect::<String>()
}

#[test]
fn fuzz_structured_inputs() {
    let mut rng = Rng(0x1234_5678_9abc_def1);
    for i in 0..1200 {
        let input = structured(&mut rng);
        same(&format!("fuzz-structured seed=0x123456789abcdef1 iter={i}"), &input);
    }
}

#[test]
fn fuzz_unstructured_inputs() {
    let mut rng = Rng(0x0fed_cba9_8765_4321);
    for i in 0..1200 {
        let input = unstructured(&mut rng);
        same(&format!("fuzz-unstructured seed=0xfedcba9876543210 iter={i}"), &input);
    }
}

#[test]
fn fuzz_digit_heavy_byte_soup() {
    // Random bytes from a digit/whitespace/sign alphabet: exercises the scanf
    // tokenizer far more densely than well-formed input does.
    const ALPHA: &[u8] = b"0123456789 \n\t\r-+.xe";
    let mut rng = Rng(0xdead_beef_cafe_0001);
    for i in 0..1500 {
        let n = rng.below(81);
        let input: Vec<u8> = (0..n).map(|_| *rng.pick(ALPHA)).collect();
        assert_same(&format!("fuzz-digitsoup iter={i}"), &input);
    }
}

#[test]
fn fuzz_arbitrary_byte_soup() {
    // Full 0..=127 alphabet, including NUL and control characters.
    let alpha: Vec<u8> = (0u8..128).collect();
    let mut rng = Rng(0x00c0_ffee_0000_0007);
    for i in 0..900 {
        let n = rng.below(61);
        let input: Vec<u8> = (0..n).map(|_| *rng.pick(&alpha)).collect();
        assert_same(&format!("fuzz-bytesoup iter={i}"), &input);
    }
}

// ===================================================================
// Process-level behavior
// ===================================================================

#[test]
fn command_line_arguments_are_ignored() {
    // main() takes argc/argv but never reads them.
    for args in [vec![], vec!["foo"], vec!["-h"], vec!["1", "2", "3"]] {
        let input = b"6 1 2 5 6\n";
        let mut runs = Vec::new();
        for exe in [c_bin(), rust_bin()] {
            let mut child = Command::new(&exe)
                .args(&args)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("spawn");
            child.stdin.take().unwrap().write_all(input).unwrap();
            runs.push(child.wait_with_output().expect("wait"));
        }
        assert_eq!(runs[0].stdout, runs[1].stdout, "stdout with args {args:?}");
        assert_eq!(runs[0].stderr, runs[1].stderr, "stderr with args {args:?}");
        assert_eq!(
            runs[0].status.code(),
            runs[1].status.code(),
            "status with args {args:?}"
        );
    }
}

#[test]
fn stdin_at_eof_immediately() {
    // Equivalent to `< /dev/null`.
    assert_same("stdin eof", b"");
}

#[test]
fn closed_stdout_matches_sigpipe_disposition() {
    // The C runs with SIGPIPE at SIG_DFL, so a reader that goes away kills it
    // with signal 13. Rust's runtime ignores SIGPIPE unless it is restored,
    // which would otherwise make the Rust program exit 0 here.
    let mut input = String::from("1 100");
    for b in 0..100usize {
        input.push_str(" 256");
        for k in 0..256usize {
            input.push_str(&format!(" {}", (b * 7 + k) % 256));
        }
    }

    let mut results = Vec::new();
    for exe in [c_bin(), rust_bin()] {
        // `head -c 1` exits after one byte, closing the read end of the pipe.
        let mut producer = Command::new(&exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn producer");

        let mut stdin = producer.stdin.take().unwrap();
        let data = input.clone().into_bytes();
        let writer = std::thread::spawn(move || {
            let _ = stdin.write_all(&data);
        });

        let stdout = producer.stdout.take().unwrap();
        let consumer = Command::new("head")
            .args(["-c", "1"])
            .stdin(stdout)
            .stdout(Stdio::null())
            .status()
            .expect("spawn head");
        assert!(consumer.success());

        let status = producer.wait().expect("wait producer");
        let _ = writer.join();

        #[cfg(unix)]
        let sig = {
            use std::os::unix::process::ExitStatusExt;
            status.signal()
        };
        #[cfg(not(unix))]
        let sig = None;

        results.push((status.code(), sig));
    }

    assert_eq!(
        results[0], results[1],
        "closed-stdout disposition differs: C={:?} Rust={:?}",
        results[0], results[1]
    );
}

#[test]
fn both_binaries_exist_and_run() {
    // Guards the harness itself: a comparison against a program that did not
    // build measures nothing.
    let c = c_bin();
    let r = rust_bin();
    assert!(c.exists(), "C binary missing at {}", c.display());
    assert!(r.exists(), "Rust binary missing at {}", r.display());
    let cr = run_one(&c, b"6 1 3 1 2 3\n");
    let rr = run_one(&r, b"6 1 3 1 2 3\n");
    assert_eq!(cr.code, Some(0), "C smoke run failed: {cr:?}");
    assert_eq!(rr.code, Some(0), "Rust smoke run failed: {rr:?}");
    assert_eq!(cr.stdout, rr.stdout);
    assert!(!cr.stdout.is_empty(), "smoke run produced no output");
}
