//! Differential tests for the top-level `main` entry point.
//!
//! Covers CONFIGS.md rows 80–91 and ERRORS.md rows 40–54.
//!
//! Every scenario is checked at TWO levels:
//!
//! 1. **through the exported `main` symbol of both shared objects**, loaded with
//!    `libloading` and called in-process with fd 0/1/2 redirected — this is what
//!    exercises the `#[no_mangle]` `main` wrapper; and
//! 2. **through the two executables**, spawned as child processes — this checks
//!    the artifacts that are actually shipped (`c_src`'s CMake `driver` vs
//!    cargo's `driver`).
//!
//! The two levels are also cross-checked against each other.

mod common;

use common::*;
use core::ffi::{c_char, c_int};
use core::ptr::null_mut;

/// Call the `main` symbol of one shared object with `input` on stdin.
fn call_main(api: &Api, input: &[u8], reset: &dyn Fn()) -> Obs<i64> {
    observe(Some(input), || unsafe {
        reset();
        let name = b"driver\0";
        let mut argv: [*mut c_char; 2] = [name.as_ptr() as *mut c_char, null_mut()];
        (api.main)(1 as c_int, argv.as_mut_ptr()) as i64
    })
}

/// The full differential check for one stdin stream.
#[track_caller]
fn diff_main(what: &str, input: &[u8]) {
    let (c, r) = both();
    let path = observe_stdin_path();

    // Level 1: the exported `main` of each .so.
    let co = call_main(c, input, &|| c_freopen_stdin(&path));
    let ro = call_main(r, input, &|| unsafe { (r.reset_stdin.unwrap())() });
    same(
        &format!("{} [so main] input={:?}", what, preview(input)),
        &co,
        &ro,
    );

    // Level 2: the two executables.
    let ce = run_exe(c_exe_path(), input);
    let re = run_exe(rust_exe_path(), input);
    if ce != re {
        panic!(
            "{} [exe] input={:?}\n  C   : {:?}\n  Rust: {:?}",
            what,
            preview(input),
            ce,
            re
        );
    }

    // Cross-check: `main`'s return value is what the process exits with, and it
    // must have produced the same bytes both ways.
    assert_eq!(
        co.ret as i32, ce.code,
        "{}: .so main returned {} but the executable exited {}",
        what, co.ret, ce.code
    );
    assert_eq!(co.stdout, ce.stdout, "{}: stdout differs .so vs exe", what);
    assert_eq!(co.stderr, ce.stderr, "{}: stderr differs .so vs exe", what);
}

fn preview(input: &[u8]) -> String {
    let s = String::from_utf8_lossy(&input[..input.len().min(160)]).to_string();
    if input.len() > 160 {
        format!("{}...({} bytes)", s, input.len())
    } else {
        s
    }
}

// --------------------------------------------------------------- builders ---

/// Build a well formed stdin stream.
fn stream(op: i64, lengths: &[usize], extra: Option<i64>, seed: u64) -> Vec<u8> {
    let mut g = Rng::new(seed);
    let mut s = format!("{} {}", op, lengths.len());
    for &l in lengths {
        s.push_str(&format!(" {}", l));
        for _ in 0..l {
            s.push_str(&format!(" {}", g.below(256)));
        }
    }
    if let Some(e) = extra {
        s.push_str(&format!(" {}", e));
    }
    s.push('\n');
    s.into_bytes()
}

// ============================================================== row 80 =====

#[test]
fn row80_op_copy() {
    for count in [2usize, 3, 5, 10] {
        for first in [0usize, 1, 2, 3, 100, 255, 256] {
            let mut lens = vec![first];
            for i in 1..count {
                lens.push(i % 7);
            }
            diff_main("row80", &stream(0, &lens, None, 0x80 + first as u64));
        }
    }
}

// ============================================================== row 81 =====

#[test]
fn row81_op_reverse() {
    let mut rng = Rng::new(0x81);
    for count in [1usize, 2, 3, 7, 25] {
        for _ in 0..5 {
            let lens: Vec<usize> = (0..count)
                .map(|i| match i % 4 {
                    0 => 0,
                    1 => 1,
                    2 => rng.below(257),
                    _ => 256,
                })
                .collect();
            diff_main("row81", &stream(1, &lens, None, rng.next_u64()));
        }
    }
}

// ============================================================== row 82 =====

#[test]
fn row82_op_merge() {
    let mut rng = Rng::new(0x82);
    // combined length from 0 up to and past the 256 limit
    for &(l1, l2) in &[
        (0usize, 0usize),
        (0, 1),
        (1, 0),
        (1, 1),
        (128, 128),
        (255, 1),
        (1, 255),
        (256, 0),
        (0, 256),
        (200, 56),
        (256, 1),
        (1, 256),
        (200, 200),
        (256, 256),
    ] {
        diff_main("row82", &stream(2, &[l1, l2], None, rng.next_u64()));
        // with extra buffers behind the first two (which are ignored)
        diff_main(
            "row82/extra",
            &stream(2, &[l1, l2, 3, 0, 7], None, rng.next_u64()),
        );
    }
    for _ in 0..40 {
        let l1 = rng.below(257);
        let l2 = rng.below(257);
        diff_main("row82/rand", &stream(2, &[l1, l2], None, rng.next_u64()));
    }
}

// ============================================================== row 83 =====

#[test]
fn row83_op_split() {
    let mut rng = Rng::new(0x83);
    for len in [0usize, 1, 2, 3, 10, 255, 256] {
        let positions: Vec<i64> = vec![
            0,
            1,
            (len as i64) / 2,
            len as i64 - 1,
            len as i64,
            len as i64 + 1,
            -1,
            -100,
            i32::MIN as i64,
            i32::MAX as i64,
            257,
        ];
        for p in positions {
            diff_main("row83", &stream(3, &[len], Some(p), rng.next_u64()));
        }
    }
    for _ in 0..40 {
        let len = rng.below(257);
        let p = rng.range(-300, 300);
        diff_main("row83/rand", &stream(3, &[len], Some(p), rng.next_u64()));
    }
}

// ============================================================== row 84 =====

#[test]
fn row84_op_interleave() {
    let mut rng = Rng::new(0x84);
    for &(l1, l2) in &[
        (0usize, 0usize),
        (0, 5),
        (5, 0),
        (1, 1),
        (3, 7),
        (7, 3),
        (128, 128),
        (255, 1),
        (256, 0),
        (0, 256),
        (129, 128),
        (256, 256),
    ] {
        diff_main("row84", &stream(4, &[l1, l2], None, rng.next_u64()));
    }
    for _ in 0..40 {
        let l1 = rng.below(200);
        let l2 = rng.below(200);
        diff_main("row84/rand", &stream(4, &[l1, l2], None, rng.next_u64()));
    }
}

// ============================================================== row 85 =====

#[test]
fn row85_op_rotate() {
    let mut rng = Rng::new(0x85);
    let positions: [i64; 14] = [
        0,
        1,
        2,
        7,
        255,
        256,
        257,
        -1,
        -7,
        -256,
        -257,
        i32::MIN as i64,
        i32::MAX as i64,
        2147483648,
    ];
    for p in positions {
        for count in [1usize, 2, 4] {
            let lens: Vec<usize> = (0..count).map(|i| [0usize, 1, 5, 256][i % 4]).collect();
            diff_main("row85", &stream(5, &lens, Some(p), rng.next_u64()));
        }
    }
    for _ in 0..40 {
        let count = 1 + rng.below(4);
        let lens: Vec<usize> = (0..count).map(|_| rng.below(257)).collect();
        let p = rng.range(-2000, 2000);
        diff_main("row85/rand", &stream(5, &lens, Some(p), rng.next_u64()));
    }
}

// ============================================================== row 86 =====

#[test]
fn row86_op_checksum() {
    let mut rng = Rng::new(0x86);
    for count in [1usize, 2, 3, 17] {
        let lens: Vec<usize> = (0..count).map(|_| rng.below(257)).collect();
        diff_main("row86", &stream(6, &lens, None, rng.next_u64()));
    }
}

// ============================================================== row 87 =====

#[test]
fn row87_maximum_buffer_count_large_output() {
    // 100 buffers x 256 bytes: ~100 KB of stdout, which crosses glibc's 4096
    // byte stdout buffer many times over.
    let lens: Vec<usize> = vec![256; 100];
    diff_main("row87/reverse", &stream(1, &lens, None, 0x87));
    diff_main("row87/checksum", &stream(6, &lens, None, 0x88));
    diff_main("row87/rotate", &stream(5, &lens, Some(101), 0x89));
    // 100 is the maximum; 101 must be rejected (ERRORS row 43)
    let lens101: Vec<usize> = vec![1; 101];
    diff_main("row87/101", &stream(6, &lens101, None, 0x8A));
    // and a mixed set of lengths at the maximum count
    let mut g = Rng::new(0x8B);
    let mixed: Vec<usize> = (0..100).map(|_| g.below(257)).collect();
    diff_main("row87/mixed", &stream(1, &mixed, None, 0x8C));
}

// ============================================================== row 88 =====

#[test]
fn row88_extra_trailing_tokens_ignored() {
    for op in [0i64, 1, 2, 3, 4, 5, 6] {
        let base = stream(op, &[3, 3], Some(1), 0x88);
        let mut with_tail = base.clone();
        with_tail.extend_from_slice(b" 9 9 9 9 9\n\n 7\n");
        diff_main("row88", &with_tail);
        let mut with_junk = base.clone();
        with_junk.extend_from_slice(b" xyz\n");
        diff_main("row88/junk-tail", &with_junk);
    }
}

// ============================================================== row 89 =====

#[test]
fn row89_whitespace_and_number_formats() {
    let ws: [&str; 8] = [" ", "\t", "\n", "\r\n", "\x0b", "\x0c", "   ", " \t\n\x0b\x0c\r "];
    for w in ws {
        let s = format!("1{w}2{w}3{w}1{w}2{w}3{w}2{w}9{w}8{w}");
        diff_main("row89/sep", s.as_bytes());
    }
    diff_main("row89/plus", b"+1 +2 +3 +1 +2 +3 +2 +9 +8\n");
    diff_main("row89/zeros", b"001 0002 003 1 2 3 02 9 8\n");
    diff_main("row89/leading-ws", b"\n\n\t   6 1 2 3 4\n");
    diff_main("row89/no-eol", b"6 1 2 3 4");
    diff_main("row89/mixed", b"\x0c+06\r\n01\t002\x0b7\x0c8\n");
}

// ============================================================== row 90 =====

#[test]
fn row90_scanf_overflow_values() {
    // `%d` conversion of tokens that overflow `long` (glibc saturates at
    // LONG_MAX/LONG_MIN and then truncates to `int`) and of tokens that only
    // overflow `int`.
    let toks = [
        "2147483647",
        "2147483648",
        "-2147483648",
        "-2147483649",
        "4294967296",
        "4294967295",
        "-4294967296",
        "9223372036854775807",
        "9223372036854775808",
        "-9223372036854775808",
        "-9223372036854775809",
        "99999999999999999999",
        "-99999999999999999999",
        "184467440737095516161",
        "00000000000000000006",
    ];
    for t in toks {
        // as the operation
        diff_main("row90/op", format!("{} 2 1 5 1 6 2\n", t).into_bytes().as_slice());
        // as the buffer count
        diff_main("row90/count", format!("6 {} 1 5\n", t).into_bytes().as_slice());
        // as a buffer length
        diff_main("row90/len", format!("6 1 {} 1 2 3\n", t).into_bytes().as_slice());
        // as a byte value
        diff_main("row90/byte", format!("6 1 2 {} 4\n", t).into_bytes().as_slice());
        // as the split position
        diff_main("row90/split", format!("3 1 4 1 2 3 4 {}\n", t).into_bytes().as_slice());
        // as the rotation amount
        diff_main("row90/rot", format!("5 1 4 1 2 3 4 {}\n", t).into_bytes().as_slice());
    }
}

// ==================================================== ERRORS rows 40–54 =====

#[test]
fn errors_row40_operation_scan_failure() {
    for input in [
        &b""[..], b" ", b"\n", b"\t\r\n ", b"x", b"abc", b"-", b"+", b"-x", b"+x", b".", b".5",
        b"/", b"\x00", b"\xff\xfe",
    ] {
        diff_main("ERRORS row40", input);
    }
    // exact message
    let ce = run_exe(c_exe_path(), b"");
    assert_eq!(ce.code, 1);
    assert_eq!(ce.stderr, b"Error: Failed to read operation\n".to_vec());
    assert!(ce.stdout.is_empty());
}

#[test]
fn errors_row41_buffer_count_scan_failure() {
    for input in [
        &b"6"[..], b"6 ", b"6\n", b"6 x", b"0 x", b"6 -", b"6 +", b"6 .", b"0x10", b"6 abc",
    ] {
        diff_main("ERRORS row41", input);
    }
    let ce = run_exe(c_exe_path(), b"6 x");
    assert_eq!(ce.code, 1);
    assert_eq!(ce.stderr, b"Error: Failed to read buffer count\n".to_vec());
}

#[test]
fn errors_row42_43_buffer_count_out_of_range() {
    for count in [
        "0",
        "-1",
        "-2",
        "-100",
        "-2147483648",
        "101",
        "102",
        "1000",
        "2147483647",
        "2147483648",
        "99999999999999999999",
        "-99999999999999999999",
    ] {
        for op in [0i64, 1, 6] {
            diff_main(
                "ERRORS row42/43",
                format!("{} {} 1 5\n", op, count).into_bytes().as_slice(),
            );
        }
    }
    for (tok, shown) in [("0", "0"), ("-1", "-1"), ("101", "101")] {
        let ce = run_exe(c_exe_path(), format!("6 {}\n", tok).as_bytes());
        assert_eq!(ce.code, 1);
        assert_eq!(
            ce.stderr,
            format!("Error: Invalid buffer count {}\n", shown).into_bytes()
        );
    }
    // 1 and 100 are the accepted boundaries
    for n in [1usize, 100] {
        let lens: Vec<usize> = vec![0; n];
        diff_main("ERRORS row42/43 boundary", &stream(6, &lens, None, 0x42));
    }
}

#[test]
fn errors_row45_read_buffer_failure_inside_main() {
    // truncated / invalid buffer data at various positions
    for input in [
        &b"6 3 1 5 1 5"[..],  // third buffer's length missing
        b"6 3 1 5 1",         // second buffer's byte missing
        b"6 2 -1 5",          // negative length
        b"6 2 257 5",         // length past the maximum
        b"6 2 2 1",           // byte missing
        b"6 2 2 1 2 x",       // junk where the second length goes
        b"6 1 3 1 2",         // short data
        b"1 4 2 1 2 2 3 4 x", // junk mid-stream
        b"6 100 1 1",         // far too little data for 100 buffers
    ] {
        diff_main("ERRORS row45", input);
    }
}

#[test]
fn errors_row46_47_50_operations_needing_two_buffers() {
    for (op, msg) in [
        (0i64, "Error: Copy needs at least 2 buffers\n"),
        (2, "Error: Merge needs at least 2 buffers\n"),
        (4, "Error: Interleave needs at least 2 buffers\n"),
    ] {
        for len in [0usize, 1, 5, 256] {
            let input = stream(op, &[len], None, 0x46);
            diff_main("ERRORS row46/47/50", &input);
            let ce = run_exe(c_exe_path(), &input);
            assert_eq!(ce.code, 1);
            assert_eq!(ce.stderr, msg.as_bytes().to_vec());
            assert!(ce.stdout.is_empty());
        }
        // two buffers is enough
        let ok = stream(op, &[2, 2], None, 0x47);
        let ce = run_exe(c_exe_path(), &ok);
        assert_eq!(ce.code, 0, "op {} with 2 buffers must succeed", op);
    }
}

#[test]
fn errors_row48_split_position_scan_failure() {
    for input in [
        &b"3 1 2 7 8"[..], // no split position token at all
        b"3 1 2 7 8 ",
        b"3 1 2 7 8\n",
        b"3 1 2 7 8 x",
        b"3 1 2 7 8 -",
        b"3 1 2 7 8 +",
        b"3 1 0",
        b"3 1 0 x",
    ] {
        diff_main("ERRORS row48", input);
    }
    let ce = run_exe(c_exe_path(), b"3 1 2 7 8");
    assert_eq!(ce.code, 1);
    assert_eq!(ce.stderr, b"Error: Failed to read split position\n".to_vec());
}

#[test]
fn errors_row49_split_position_past_length() {
    for len in [0usize, 1, 5, 256] {
        for delta in [1i64, 2, 100] {
            let input = stream(3, &[len], Some(len as i64 + delta), 0x49);
            diff_main("ERRORS row49", &input);
            let ce = run_exe(c_exe_path(), &input);
            assert_eq!(ce.code, 1);
            assert_eq!(
                ce.stderr,
                format!(
                    "Error: Split position {} exceeds length {}\n",
                    len as i64 + delta,
                    len
                )
                .into_bytes()
            );
        }
        // negative positions sign-extend into a huge size_t
        for neg in [-1i64, -2, -100, i32::MIN as i64] {
            let input = stream(3, &[len], Some(neg), 0x4A);
            diff_main("ERRORS row49/neg", &input);
            let ce = run_exe(c_exe_path(), &input);
            assert_eq!(ce.code, 1);
            assert_eq!(
                ce.stderr,
                format!(
                    "Error: Split position {} exceeds length {}\n",
                    neg as i32 as isize as usize,
                    len
                )
                .into_bytes()
            );
        }
    }
}

#[test]
fn errors_row51_rotation_amount_scan_failure() {
    for input in [
        &b"5 1 2 7 8"[..],
        b"5 1 2 7 8 ",
        b"5 1 2 7 8\n",
        b"5 1 2 7 8 x",
        b"5 1 2 7 8 -",
        b"5 2 0 0",
        b"5 1 0 zzz",
    ] {
        diff_main("ERRORS row51", input);
    }
    let ce = run_exe(c_exe_path(), b"5 1 2 7 8");
    assert_eq!(ce.code, 1);
    assert_eq!(ce.stderr, b"Error: Failed to read rotation amount\n".to_vec());
}

#[test]
fn errors_row52_unknown_operation() {
    for op in [
        "7",
        "8",
        "9",
        "42",
        "-1",
        "-2",
        "-100",
        "1000",
        "2147483647",
        "-2147483648",
        "2147483648",
        "99999999999999999999",
    ] {
        let input = format!("{} 2 1 5 1 6\n", op).into_bytes();
        diff_main("ERRORS row52", &input);
        let ce = run_exe(c_exe_path(), &input);
        assert_eq!(ce.code, 1);
        assert!(
            String::from_utf8_lossy(&ce.stderr).starts_with("Error: Unknown operation "),
            "stderr was {:?}",
            String::from_utf8_lossy(&ce.stderr)
        );
    }
}

#[test]
fn errors_row53_54_combined_length_over_maximum_in_main() {
    for &(l1, l2) in &[(256usize, 1usize), (1, 256), (200, 200), (256, 256), (129, 128)] {
        let m = stream(2, &[l1, l2], None, 0x53);
        diff_main("ERRORS row53", &m);
        let ce = run_exe(c_exe_path(), &m);
        assert_eq!(ce.code, 1);
        assert_eq!(
            ce.stderr,
            format!("Error: Merged length {} exceeds maximum\n", l1 + l2).into_bytes()
        );

        let i = stream(4, &[l1, l2], None, 0x54);
        diff_main("ERRORS row54", &i);
        let ce = run_exe(c_exe_path(), &i);
        assert_eq!(ce.code, 1);
        assert_eq!(
            ce.stderr,
            b"Error: Interleaved length exceeds maximum\n".to_vec()
        );
    }
}

// ============================================================== row 91 =====

/// Generate a stdin stream that is usually structurally valid but freely mixes
/// in boundary values, junk tokens and every whitespace form.
fn fuzz_input(g: &mut Rng) -> Vec<u8> {
    let interesting: [i64; 22] = [
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        -1,
        -2,
        100,
        101,
        255,
        256,
        257,
        1000,
        2147483647,
        -2147483648,
        2147483648,
        4294967296,
        -4294967296,
        i64::MAX,
    ];
    let seps: [&str; 8] = [" ", "\n", "\t", "  ", " \n ", "\r\n", "\x0b", "\x0c"];

    let mut toks: Vec<String> = Vec::new();
    let mut push = |t: i64, g: &mut Rng| {
        let mut s = String::new();
        if t >= 0 && g.below(12) == 0 {
            s.push('+');
        } else if t >= 0 && g.below(14) == 0 {
            s.push('0');
        }
        s.push_str(&t.to_string());
        // Occasionally grow the token past the range of `long`, so glibc's
        // strtol saturation + truncation to `int` is exercised.
        if g.below(25) == 0 {
            for _ in 0..(1 + g.below(6)) {
                s.push(char::from(b'0' + g.below(10) as u8));
            }
        }
        toks.push(s);
    };

    if g.below(100) < 75 {
        // structured
        let op = if g.below(5) == 0 {
            g.pick(&interesting)
        } else {
            g.below(7) as i64
        };
        push(op, g);
        let count = if g.below(10) == 0 {
            g.pick(&interesting)
        } else {
            1 + g.below(5) as i64
        };
        push(count, g);
        let n = if (1..=100).contains(&count) {
            count as usize
        } else {
            1 + g.below(3)
        };
        for _ in 0..n {
            let len = if g.below(8) == 0 {
                g.pick(&interesting)
            } else {
                match g.below(4) {
                    0 => 0,
                    1 => 1,
                    2 => g.below(20) as i64,
                    _ => g.below(257) as i64,
                }
            };
            push(len, g);
            if (0..=256).contains(&len) {
                for _ in 0..len {
                    let b = if g.below(8) == 0 {
                        g.pick(&interesting)
                    } else {
                        g.below(256) as i64
                    };
                    push(b, g);
                }
            }
        }
        if g.below(10) < 9 {
            let e = if g.below(3) == 0 {
                g.pick(&interesting)
            } else {
                g.range(-300, 300)
            };
            push(e, g);
        }
        if g.below(5) == 0 {
            let e = g.pick(&interesting);
            push(e, g);
        }
    } else {
        for _ in 0..g.below(12) {
            let t = g.pick(&interesting);
            push(t, g);
        }
    }

    let mut s = String::new();
    for (i, t) in toks.iter().enumerate() {
        if i > 0 {
            s.push_str(g.pick(&seps));
        }
        s.push_str(t);
    }
    if g.below(10) == 0 {
        s.push_str(g.pick(&[" x", " abc", " -", " +", " 1.5", " 0x10", "\n\n", " --3"]));
    }
    if g.below(4) == 0 {
        s.insert(0, ' ');
    }
    if g.below(3) != 0 {
        s.push('\n');
    }
    s.into_bytes()
}

#[test]
fn row91_randomized_full_program_fuzz() {
    let mut g = Rng::new(0x91);
    for i in 0..1500 {
        let input = fuzz_input(&mut g);
        diff_main(&format!("row91/#{}", i), &input);
    }
}

#[test]
fn row91b_randomized_full_program_fuzz_second_seed() {
    let mut g = Rng::new(0xC0FFEE);
    for i in 0..1500 {
        let input = fuzz_input(&mut g);
        diff_main(&format!("row91b/#{}", i), &input);
    }
}
