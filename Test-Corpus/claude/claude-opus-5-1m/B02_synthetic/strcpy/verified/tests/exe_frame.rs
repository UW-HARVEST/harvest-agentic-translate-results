//! Executable level differential tests for the inputs whose result depends on
//! the *uninitialised* part of the C `main` stack frame (buffers that are not
//! NUL terminated, so `strcmp`/`strlen` read past them).
//!
//! Those left-over bytes come from the dynamic loader and depend on ASLR, on the
//! size of the environment and on the length of the path the program was started
//! with, so they cannot be reproduced from a static table in *every* environment.
//! `src/frame_junk.rs` is a snapshot captured from the compiled C program, and
//! these tests pin the *logic* down independently of that snapshot: the C driver
//! is run under `probe/inject_frame`, a ptrace helper that overwrites every byte
//! of the frame `main` did not initialise with exactly the snapshot the Rust
//! translation uses. Both programs then read identical memory and any difference
//! in their output is a real difference in the translated code.
//!
//! `exe_frame_snapshot_matches_reality` additionally checks the snapshot itself
//! against a fresh capture of the real frame.

mod common;

use common::*;
use driver::frame_junk::FRAME_JUNK;
use std::process::Command;

/// The snapshot must agree with the real frame everywhere except in the bytes
/// that are inherently unstable (the low bytes of the loader's saved stack
/// pointers, which move with ASLR / environment size / argv[0] length).
#[test]
fn exe_frame_snapshot_matches_reality() {
    let dumper = build_helper("dump_frame", "dump_frame.c");
    let driver = c_driver();
    let bp = bp_addr(driver);
    // operation 9 keeps both buffers pristine (no bytes are read from stdin)
    let stdin_file = unique_stdin("9 0 0 0\n");

    let mut disagree_zero = Vec::new();
    let mut disagree_nonzero = Vec::new();
    for _ in 0..8 {
        let out = Command::new(&dumper)
            .arg(driver)
            .arg(&bp)
            .arg(&stdin_file)
            .arg(MODEL_END.to_string())
            .output()
            .expect("run dumper");
        assert!(out.status.success(), "frame dump failed");
        let text = String::from_utf8_lossy(&out.stdout);
        let mut seen = 0usize;
        for line in text.lines() {
            if line.starts_with('#') {
                continue;
            }
            let mut it = line.split_whitespace();
            let off: usize = it.next().unwrap().parse().unwrap();
            let val: u8 = it.next().unwrap().parse().unwrap();
            if off >= MODEL_END {
                continue;
            }
            seen += 1;
            let modelled = FRAME_JUNK[off];
            if (modelled == 0) != (val == 0) {
                if modelled == 0 {
                    disagree_zero.push(off);
                } else {
                    disagree_nonzero.push(off);
                }
            }
        }
        assert_eq!(seen, MODEL_END, "dump is incomplete");
    }
    let _ = std::fs::remove_file(&stdin_file);
    disagree_zero.sort_unstable();
    disagree_zero.dedup();
    disagree_nonzero.sort_unstable();
    disagree_nonzero.dedup();
    println!(
        "frame snapshot vs reality: {} + {} of {MODEL_END} offsets disagree on zero-ness\n  \
         modelled zero but observed non-zero: {disagree_zero:?}\n  \
         modelled non-zero but observed zero: {disagree_nonzero:?}",
        disagree_zero.len(),
        disagree_nonzero.len()
    );
    // The structural layout must match exactly: every byte the snapshot models
    // as zero (the zero filled regions of the frame, which is what `strlen` and
    // `strcmp` actually branch on) has to be zero in reality as well.
    assert!(
        disagree_zero.is_empty(),
        "the snapshot models {disagree_zero:?} as zero but the real frame has non-zero bytes there"
    );
    // The opposite direction is the irreducible environment dependence: the low
    // byte of a saved stack pointer is `addr & 0xff` of a 16 byte aligned
    // address, i.e. zero for one environment out of sixteen. Those bytes are
    // correlated (they all point into the same frame), so a whole group can flip
    // at once; keep the tolerance at ~3% of the frame.
    assert!(
        disagree_nonzero.len() * 32 < MODEL_END,
        "{} of {MODEL_END} modelled non-zero bytes are zero in reality ({disagree_nonzero:?}), \
         which is beyond the environment dependent noise floor",
        disagree_nonzero.len()
    );
}

/// Unterminated buffers of every length: the reads run into the junk that
/// follows them, and the two implementations must agree byte for byte.
#[test]
fn exe_frame_unterminated_lengths() {
    for op in [0i64, 1, 2, 3, 4] {
        for flags in [0u32, 1, 2, 3] {
            for len in [0usize, 1, 2, 3, 5, 6, 7, 8, 9, 16, 17, 31, 32, 33, 64, 100] {
                let a = vec![b'A'; len];
                let b = vec![b'a'; len];
                diff_exe_injected(&exe_case(op, flags, &a, &b), "unterminated equal lengths");
                let b2 = vec![b'A'; len];
                diff_exe_injected(&exe_case(op, flags, &a, &b2), "unterminated identical data");
            }
        }
    }
}

/// The buffers are adjacent in the frame (`ref_buffer` first, `input_buffer`
/// 1024 bytes above it), so a full 1024 byte unterminated buffer makes the reads
/// cross from one array into the other and then into the locals of `main`.
#[test]
fn exe_frame_full_buffers() {
    for op in [0i64, 1, 2, 3, 4] {
        for flags in [0u32, 1, 2, 3] {
            for (la, lb) in [
                (1024usize, 0usize),
                (1024, 1),
                (1024, 1024),
                (1023, 1024),
                (1024, 1023),
                (0, 1024),
                (1, 1024),
            ] {
                let a = vec![b'A'; la];
                let b = vec![b'a'; lb];
                diff_exe_injected(&exe_case(op, flags, &a, &b), "full buffers");
            }
        }
    }
}

#[test]
fn exe_frame_random() {
    let rng = Rng::new(9090);
    let words: [&[u8]; 12] = [
        b"START", b"STOP", b"PAUSE", b"RESUME", b"RESET", b"ADMIN", b"VALID", b"OK", b"NONE",
        b"EMPTY", b"abc", b"",
    ];
    for _ in 0..400 {
        let op = rng.pick(&[0i64, 1, 2, 3, 4, 0, 1, 2, 3, 4, 5, -1, 7]);
        let flags = rng.pick(&[0u32, 1, 2, 3, 4, 0xFFFF_FFFF]);
        let make = || -> Vec<u8> {
            match rng.below(6) {
                0 => {
                    let mut v = rng.pick(&words).to_vec();
                    if rng.bool() {
                        v.push(0);
                    }
                    v
                }
                1 => (0..rng.below(24)).map(|_| rng.byte()).collect(),
                2 => {
                    let mut v: Vec<u8> = (0..rng.below(24)).map(|_| rng.byte() | 1).collect();
                    v.push(0);
                    v
                }
                3 => {
                    let n = rng.pick(&[
                        0usize, 1, 2, 3, 7, 8, 15, 16, 33, 64, 200, 512, 1023, 1024,
                    ]);
                    vec![rng.pick(&[b'A', b'a', b'z', 1u8, 255, b' ', b':', b'|']); n]
                }
                4 => {
                    let mut v = rng.pick(&words).to_vec();
                    v.extend((0..rng.below(5)).map(|_| rng.byte()));
                    v
                }
                _ => (0..rng.below(40))
                    .map(|_| rng.pick(&[0u8, 65, 97, 42, 58, 124, 95, 32]))
                    .collect(),
            }
        };
        let a = make();
        let b = make();
        diff_exe_injected(&exe_case(op, flags, &a, &b), "random with controlled frame");
    }
}

/// The unbounded loop in `match_pattern` (`text_len - pattern_len` underflows)
/// walks up the stack until the process dies; with a pattern that does not occur
/// above the frame both programs must die the same way and print nothing.
#[test]
fn exe_frame_unbounded_loop() {
    for (text, pattern) in [
        (b"AB\0".as_slice(), b"ABCDEFGH\0".as_slice()),
        (b"\0", b"\xf7\x3b\xa9\xd1\x5c\xe2\x84\x91\0".as_slice()),
        (b"abc\0", b"abcdefghijklmnop\0"),
        (b"\0", b"\xd3\x7a\x0e\0".as_slice()),
    ] {
        let r = diff_exe_injected(&exe_case(4, 2, text, pattern), "unbounded loop");
        assert_eq!(r.code, 128 + 11, "expected SIGSEGV: {r:?}");
        assert!(r.stdout.is_empty());
    }
}

/// A one byte pattern is always found in the junk right above the text, so the
/// unbounded loop returns `10 + <offset of the first occurrence>`: this pins the
/// modelled frame contents down byte for byte.
#[test]
fn exe_frame_unbounded_loop_finds_junk_byte() {
    // bytes that occur in the part of the frame the injector controls
    let window = &FRAME_JUNK[1025..2090];
    let mut candidates: Vec<u8> = Vec::new();
    for &b in window {
        if b != 0 && !candidates.contains(&b) {
            candidates.push(b);
        }
        if candidates.len() == 12 {
            break;
        }
    }
    assert_eq!(candidates.len(), 12, "not enough distinct junk bytes");
    for b in candidates {
        let first = (1..1066usize)
            .find(|&k| FRAME_JUNK[1024 + k] == b)
            .expect("byte occurs in the window");
        let r = diff_exe_injected(
            &exe_case(4, 2, b"\0", &[b, 0]),
            "unbounded loop, 1 byte pattern",
        );
        assert_eq!(r.code, 0, "expected a normal exit: {r:?}");
        assert_eq!(
            String::from_utf8_lossy(&r.stdout),
            format!("{}\n", 10 + first),
            "byte {b:#02x} must be found at offset {first} above the text"
        );
    }
}
