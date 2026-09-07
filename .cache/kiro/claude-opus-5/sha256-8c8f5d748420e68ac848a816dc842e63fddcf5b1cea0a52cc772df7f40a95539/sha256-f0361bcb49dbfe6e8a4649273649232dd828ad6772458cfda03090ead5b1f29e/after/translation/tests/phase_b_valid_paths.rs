//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Both implementations are driven only
//! through their `.so` exports; the stdout byte stream is compared exactly.

mod common;

use common::*;
use std::ffi::c_char;

const ENTRY_POINTS_VOID: [&str; 3] = ["bad", "good", "driver"];

// ---------------------------------------------------------------------------
// Row 1 — 1-byte string, every value 0x01..=0xFF (exhaustive).
// ---------------------------------------------------------------------------

#[test]
fn cfg_row1_single_byte_exhaustive() {
    for b in 1u8..=255 {
        diff_print_line(&format!("row1 single byte 0x{b:02x}"), &[b]);
    }
}

// ---------------------------------------------------------------------------
// Row 2 — random printable ASCII, len 1..=64.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row2_random_ascii_short() {
    let mut rng = Rng::new(0x5EED_0002);
    for i in 0..512 {
        let len = rng.range(1, 64);
        let s = rng.ascii(len);
        diff_print_line(&format!("row2 iter={i} len={len}"), &s);
    }
}

// ---------------------------------------------------------------------------
// Row 3 — random arbitrary non-NUL bytes (invalid UTF-8 included), len 1..=256.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row3_random_arbitrary_bytes() {
    let mut rng = Rng::new(0x5EED_0003);
    for i in 0..512 {
        let len = rng.range(1, 256);
        let s = rng.bytes(len);
        diff_print_line(&format!("row3 iter={i} len={len}"), &s);
    }
}

// ---------------------------------------------------------------------------
// Row 4 — lengths straddling stdio's 4096-byte buffer.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row4_buffer_boundary_lengths() {
    let mut rng = Rng::new(0x5EED_0004);
    // Around the 4 KiB stdio buffer, and around 1 KiB / 8 KiB for good measure.
    let mut lengths: Vec<usize> = Vec::new();
    for base in [1024usize, 4096, 8192] {
        for delta in -2i64..=2 {
            lengths.push((base as i64 + delta) as usize);
        }
    }
    for &len in &lengths {
        // Two shapes per length: printable and arbitrary bytes.
        let a = rng.ascii(len);
        diff_print_line(&format!("row4 ascii len={len}"), &a);
        let b = rng.bytes(len);
        diff_print_line(&format!("row4 bytes len={len}"), &b);
    }
}

// ---------------------------------------------------------------------------
// Row 5 — very long strings, 64 KiB … 1 MiB.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row5_long_strings() {
    let mut rng = Rng::new(0x5EED_0005);
    for &len in &[64 * 1024usize, 256 * 1024, 1024 * 1024] {
        let s = rng.bytes(len);
        diff_print_line(&format!("row5 len={len}"), &s);
    }
    // A long run of one repeated byte, plus a long run with newlines every 37
    // bytes, to vary how the stream is chunked internally.
    let mut lined = vec![b'x'; 200_000];
    for (i, b) in lined.iter_mut().enumerate() {
        if i % 37 == 36 {
            *b = b'\n';
        }
    }
    diff_print_line("row5 200k with embedded newlines", &lined);
    diff_print_line("row5 100k repeated byte", &vec![0xABu8; 100_000]);
}

// ---------------------------------------------------------------------------
// Row 6 — control bytes, including embedded newlines.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row6_control_bytes() {
    const CONTROLS: [u8; 7] = [b'\n', b'\r', b'\t', 0x0b, 0x0c, 0x08, 0x1b];

    // Hand-picked shapes first.
    diff_print_line("row6 lone newline", b"\n");
    diff_print_line("row6 trailing newline", b"abc\n");
    diff_print_line("row6 leading newline", b"\nabc");
    diff_print_line("row6 only controls", &CONTROLS);
    diff_print_line("row6 crlf", b"a\r\nb\r\nc");

    // Then randomized mixtures of controls and printable text.
    let mut rng = Rng::new(0x5EED_0006);
    for i in 0..256 {
        let len = rng.range(1, 128);
        let s: Vec<u8> = (0..len)
            .map(|_| {
                if rng.below(3) == 0 {
                    CONTROLS[rng.below(CONTROLS.len())]
                } else {
                    rng.range(0x20, 0x7e) as u8
                }
            })
            .collect();
        diff_print_line(&format!("row6 iter={i} len={len}"), &s);
    }
}

// ---------------------------------------------------------------------------
// Row 7 — content that is itself a printf format string.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row7_format_string_content() {
    const SPECS: [&[u8]; 14] = [
        b"%s",
        b"%d",
        b"%n",
        b"%%",
        b"%p",
        b"%x",
        b"%1000000d",
        b"%.2147483647f",
        b"%s%s%s%s%s%s%s%s",
        b"%n%n%n%n",
        b"100%% done",
        b"printf(\"%s\\n\", line)",
        b"%*d",
        b"%hhn",
    ];
    for spec in SPECS {
        diff_print_line(
            &format!("row7 spec={}", String::from_utf8_lossy(spec)),
            spec,
        );
    }

    // Randomized concatenations of specifiers and filler.
    let mut rng = Rng::new(0x5EED_0007);
    for i in 0..256 {
        let parts = rng.range(1, 8);
        let mut s = Vec::new();
        for _ in 0..parts {
            s.extend_from_slice(SPECS[rng.below(SPECS.len())]);
            let filler = rng.ascii_len(0, 8);
            s.extend_from_slice(&filler);
        }
        if s.is_empty() {
            s.push(b'%');
        }
        diff_print_line(&format!("row7 random iter={i}"), &s);
    }
}

// ---------------------------------------------------------------------------
// Row 8 — a long random call sequence against one accumulating stream.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row8_random_call_sequence() {
    let mut rng = Rng::new(0x5EED_0008);

    // Build the script once so both implementations replay it identically.
    // `None` means printLine(NULL).
    let script: Vec<Option<Vec<u8>>> = (0..256)
        .map(|_| match rng.below(8) {
            0 => None,
            1 => Some(Vec::new()),
            2 => Some(rng.bytes_len(1, 4096)),
            _ => Some(rng.ascii_len(1, 80)),
        })
        .collect();

    diff("row8 random call sequence", |which| {
        let f = sym_print_line(which);
        for item in &script {
            match item {
                None => unsafe { f(std::ptr::null()) },
                Some(payload) => {
                    let mut buf = payload.clone();
                    buf.push(0);
                    unsafe { f(buf.as_ptr() as *const c_char) };
                }
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Row 9 — interleaved with the caller's own writes to fd 1.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row9_interleaved_with_caller_output() {
    diff("row9 interleaved with caller output", |which| {
        let f = sym_print_line(which);
        let driver = sym_void(which, "driver");
        raw_write_stdout(b"<<caller preamble>>\n");
        unsafe { f(b"library line 1\0".as_ptr() as *const c_char) };
        raw_write_stdout(b"<<caller middle>>\n");
        unsafe { driver() };
        raw_write_stdout(b"<<caller epilogue>>\n");
        unsafe { f(b"library line 2\0".as_ptr() as *const c_char) };
    });
}

// ---------------------------------------------------------------------------
// Rows 10-12 — each level-1/level-2 entry point on a fresh stream.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row10_bad() {
    diff("row10 bad()", |which| {
        let f = sym_void(which, "bad");
        unsafe { f() };
    });
}

#[test]
fn cfg_row11_good() {
    diff("row11 good()", |which| {
        let f = sym_void(which, "good");
        unsafe { f() };
    });
}

#[test]
fn cfg_row12_driver() {
    diff("row12 driver()", |which| {
        let f = sym_void(which, "driver");
        unsafe { f() };
    });
}

// ---------------------------------------------------------------------------
// Row 13 — repeated invocations (no hidden state).
// ---------------------------------------------------------------------------

#[test]
fn cfg_row13_repeated_invocations() {
    for name in ENTRY_POINTS_VOID {
        diff(&format!("row13 {name}() x16"), |which| {
            let f = sym_void(which, name);
            for _ in 0..16 {
                unsafe { f() };
            }
        });
    }
    diff("row13 printLine x16", |which| {
        let f = sym_print_line(which);
        for i in 0..16 {
            let s = format!("repeat {i}\0");
            unsafe { f(s.as_ptr() as *const c_char) };
        }
    });
}

// ---------------------------------------------------------------------------
// Row 14 — randomly interleaved sequence over ALL four entry points.
// ---------------------------------------------------------------------------

#[derive(Clone)]
enum Op {
    PrintLine(Option<Vec<u8>>),
    Bad,
    Good,
    Driver,
}

fn random_ops(seed: u64, n: usize) -> Vec<Op> {
    let mut rng = Rng::new(seed);
    (0..n)
        .map(|_| match rng.below(6) {
            0 => Op::Bad,
            1 => Op::Good,
            2 => Op::Driver,
            3 => Op::PrintLine(None),
            4 => Op::PrintLine(Some(Vec::new())),
            _ => Op::PrintLine(Some(rng.bytes_len(1, 200))),
        })
        .collect()
}

fn replay(which: Impl, ops: &[Op]) {
    let print_line = sym_print_line(which);
    let bad = sym_void(which, "bad");
    let good = sym_void(which, "good");
    let driver = sym_void(which, "driver");
    for op in ops {
        match op {
            Op::Bad => unsafe { bad() },
            Op::Good => unsafe { good() },
            Op::Driver => unsafe { driver() },
            Op::PrintLine(None) => unsafe { print_line(std::ptr::null()) },
            Op::PrintLine(Some(payload)) => {
                let mut buf = payload.clone();
                buf.push(0);
                unsafe { print_line(buf.as_ptr() as *const c_char) };
            }
        }
    }
}

#[test]
fn cfg_row14_all_entry_points_interleaved() {
    for seed in [0x5EED_0014u64, 0xA11C_0E14, 0x0BAD_F00D] {
        let ops = random_ops(seed, 256);
        diff(&format!("row14 seed={seed:#x}"), |which| replay(which, &ops));
    }
}

// ---------------------------------------------------------------------------
// Row 15 — driver() sharing one stream with the level-0 primitive.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row15_driver_mixed_with_primitives() {
    diff("row15 driver mixed with primitives", |which| {
        let print_line = sym_print_line(which);
        let bad = sym_void(which, "bad");
        let good = sym_void(which, "good");
        let driver = sym_void(which, "driver");
        unsafe {
            print_line(b"before\0".as_ptr() as *const c_char);
            driver();
            good();
            print_line(std::ptr::null());
            bad();
            driver();
            print_line(b"after\0".as_ptr() as *const c_char);
        }
    });
}

// ---------------------------------------------------------------------------
// Row 16 — invoked from a secondary thread.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row16_called_from_secondary_thread() {
    let ops = random_ops(0x5EED_0016, 64);
    diff("row16 secondary thread", |which| {
        let ops = ops.clone();
        std::thread::spawn(move || replay(which, &ops))
            .join()
            .expect("worker thread panicked");
    });
}
