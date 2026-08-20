//! Phase B / Phase C — the exported `main` called repeatedly in one process.
//!
//! Rows C26–C28 and E18 of CONFIGS.md / ERRORS.md. `main` is a public symbol of
//! the shared library, so an external consumer can call it more than once. Each
//! call continues reading the *same* stdin stream, which makes visible exactly
//! where the previous `scanf("%d")` left the stream positioned — glibc returns
//! the terminating (or mismatching) character with `ungetc`, and an
//! already-consumed sign is not returned.

mod common;

use common::*;

/// The line `driver(x)` prints for a given `x`.
fn line(x: i32) -> String {
    let mut s = String::new();
    for b in x.to_le_bytes() {
        s.push_str(&format!("{b:02x}"));
    }
    s.push_str("03000000");
    s.push_str("0000000000000040");
    s.push('\n');
    s
}

/// Expected concatenated output for a sequence of `x` values.
fn lines(xs: &[i32]) -> String {
    xs.iter().map(|&x| line(x)).collect()
}

/// C26 — two `main` calls: the second sees whatever the first left behind.
///
/// The expected values were measured from glibc with successive
/// `fscanf("%d")` calls on one stream.
#[test]
fn c26_two_calls_stream_position() {
    // (stdin, x of call 1, x of call 2)
    let cases: [(&str, i32, i32); 16] = [
        // The terminating non-digit is pushed back, so the second conversion
        // fails on it and x stays 0.
        ("12x34", 12, 0),
        // Whitespace terminators are skipped by the second conversion.
        ("12 34", 12, 34),
        ("12\n34", 12, 34),
        ("12\t34", 12, 34),
        // A pushed-back '-' is reused as the second number's sign.
        ("5-6", 5, -6),
        ("5+6", 5, 6),
        // 'x' terminates the first conversion and blocks the second.
        ("0x10", 0, 0),
        (".5", 0, 0),
        // A mismatching character is pushed back, an already-eaten sign is not.
        ("- 5", 0, 5),
        ("+ 9", 0, 9),
        ("--3", 0, -3),
        ("-x5", 0, 0),
        ("abc7", 0, 0),
        // EOF right after the digits: the second call gets EOF, x stays 0.
        ("12", 12, 0),
        ("", 0, 0),
        ("   ", 0, 0),
    ];
    for (input, x1, x2) in cases {
        let run = assert_main_n_same(input.as_bytes(), 2, &format!("C26 {input:?}"));
        assert_eq!(
            String::from_utf8_lossy(&run.stdout),
            lines(&[x1, x2]),
            "C26: the C library's result for {input:?} is not the measured one"
        );
        assert_eq!(run.returns, vec![0, 0], "C26: main() must return 0");
    }
}

/// C27 — three or more calls, consuming a whole list of numbers.
#[test]
fn c27_many_calls_consume_a_list() {
    let cases: [(&str, [i32; 4]); 8] = [
        ("1 2 3 4", [1, 2, 3, 4]),
        ("1\n2\n3\n4\n", [1, 2, 3, 4]),
        ("  -1   +2 \t 3 \r\n -4 ", [-1, 2, 3, -4]),
        // Runs out of input: the remaining calls see EOF and print 0.
        ("7 8", [7, 8, 0, 0]),
        ("42", [42, 0, 0, 0]),
        ("", [0, 0, 0, 0]),
        // A blocking non-digit stops every later conversion.
        ("1 2 x 3", [1, 2, 0, 0]),
        // Signs and overflow mixed into a list.
        ("2147483648 -9223372036854775809 0 -1", [i32::MIN, 0, 0, -1]),
    ];
    for (input, xs) in cases {
        let run = assert_main_n_same(input.as_bytes(), 4, &format!("C27 {input:?}"));
        assert_eq!(
            String::from_utf8_lossy(&run.stdout),
            lines(&xs),
            "C27: the C library's result for {input:?} is not the measured one"
        );
        assert_eq!(run.returns, vec![0; 4], "C27: main() must return 0");
    }
}

/// C28 — randomized token streams read by repeated `main` calls (fixed seed).
/// This is where a wrong stream position shows up as a cascading divergence.
#[test]
fn c28_random_token_streams() {
    let mut rng = Rng::new(0xC28);
    const WS: [&str; 6] = [" ", "\t", "\n", "\x0b", "\x0c", "\r"];
    const ODD: [&str; 8] = ["x", ".", ",", "-", "+", "abc", "0x", "!"];

    for i in 0..192 {
        let mut input = String::new();
        let tokens = rng.below(6) as usize;
        for _ in 0..tokens {
            // Separator (sometimes absent, which merges tokens).
            for _ in 0..rng.below(3) {
                input.push_str(*rng.pick(&WS));
            }
            match rng.below(10) {
                // A number of some magnitude class.
                0..=6 => {
                    if rng.below(3) == 0 {
                        input.push(if rng.below(2) == 0 { '-' } else { '+' });
                    }
                    match rng.below(4) {
                        0 => input.push_str(&rng.below(1000).to_string()),
                        1 => input.push_str(&(rng.next_u32() as i32).to_string()),
                        2 => input.push_str(&rng.next_u64().to_string()),
                        _ => input.push_str(&format!("{}{}", rng.next_u64(), rng.next_u64())),
                    }
                }
                // Something that does not match, to exercise pushback.
                _ => input.push_str(*rng.pick(&ODD)),
            }
        }
        for _ in 0..rng.below(3) {
            input.push_str(*rng.pick(&WS));
        }

        let n = 1 + rng.below(5) as usize;
        assert_main_n_same(
            input.as_bytes(),
            n,
            &format!("C28 iteration {i}: {input:?} x{n}"),
        );
    }
}

/// E18 — a rejected conversion must leave the stream *exactly* where the C
/// leaves it: repeating a failing conversion many times must keep failing on
/// the same character and must never make progress.
#[test]
fn e18_failed_conversion_does_not_consume() {
    for input in ["abc", "x1", "-x", "+.", "0x10", ".5", ",", "\0", "\u{ff}9", "--"] {
        let run = assert_main_n_same(input.as_bytes(), 5, &format!("E18 {input:?}"));
        // Every call after the blocking character must print the same line.
        let first = &run.stdout[..33];
        for k in 1..5 {
            let this = &run.stdout[k * 33..(k + 1) * 33];
            if input == "0x10" || input == ".5" {
                // The first call converts a digit ('0'), later ones fail: only
                // calls 2.. are required to be identical to each other.
                if k >= 2 {
                    assert_eq!(
                        this,
                        &run.stdout[33..66],
                        "E18 {input:?}: call {} differs from call 2",
                        k + 1
                    );
                }
            } else {
                assert_eq!(this, first, "E18 {input:?}: call {} made progress", k + 1);
            }
        }
    }
}

/// E19 — the stdin stream reaches end-of-file and *then* grows.
///
/// C's end-of-file indicator is sticky: once `stdin` has seen EOF, every later
/// `scanf` fails without reading again, because the C code never calls
/// `clearerr`. Verified against glibc:
///
/// ```text
/// file "5": fscanf -> 5 | fscanf -> EOF | (append " 7") fscanf -> EOF
///                                       | clearerr, fscanf -> 7
/// ```
///
/// So every call after the first EOF must print `x = 0`, even though more data
/// is available on the descriptor.
#[test]
fn e19_sticky_eof_on_a_growing_stdin() {
    // "5" then EOF, and " 7" appears after each call.
    let run = assert_main_growing_same(b"5", " 7", 4, "E19 digits then EOF");
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        lines(&[5, 0, 0, 0]),
        "E19: the C library's sticky-EOF behaviour is not the measured one"
    );

    // Empty to begin with: the very first call already sees EOF.
    let run = assert_main_growing_same(b"", "42 ", 3, "E19 empty then growing");
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        lines(&[0, 0, 0]),
        "E19: an initially empty stdin must stay at EOF"
    );

    // The complement of the sticky-EOF case: with a trailing separator every
    // conversion stops *before* EOF, so the indicator is never set and each
    // call happily reads the data that was appended after the previous one.
    let run = assert_main_growing_same(b"5 ", "7 ", 4, "E19 terminator before EOF");
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        lines(&[5, 7, 7, 7]),
        "E19: a stream that never reaches EOF must keep making progress"
    );

    // Whitespace-only input: the whitespace skip consumes everything and hits
    // EOF, which freezes the stream.
    let run = assert_main_growing_same(b" ", "1 ", 3, "E19 whitespace then EOF");
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        lines(&[0, 0, 0]),
        "E19: whitespace-only input must reach the sticky EOF state"
    );
}
