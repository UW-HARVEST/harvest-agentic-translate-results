//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every case drives BOTH the C `.so` and the Rust `.so` through their exported
//! symbols and compares the bytes written to stdout. Rows that can reach the
//! `bad()` defect additionally compare the process termination status, because
//! the C genuinely faults for some caller stacks.

mod common;

use common::*;
use std::ffi::c_char;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// What the C must print for `printLine(payload_as_c_string)`:
/// `printf("%s\n", p)` (which gcc lowers to `puts(p)`) stops at the first NUL.
fn expected(payload: &[u8]) -> Vec<u8> {
    let upto = payload.split(|b| *b == 0).next().unwrap_or(&[]);
    let mut v = upto.to_vec();
    v.push(b'\n');
    v
}

/// Differentially check `printLine` for one payload (a NUL terminator is added).
fn check_print_line(case: &str, payload: &[u8]) {
    let mut buf = payload.to_vec();
    buf.push(0);
    let exp = expected(payload);
    assert_same_and_eq(case, &exp, |api| unsafe {
        api.print_line(buf.as_ptr() as *const c_char)
    });
}

/// Differentially check `driver(v)`. `v == 0` reaches the uninitialized read, so
/// it must run isolated; every non-zero `v` is the well-defined `good()` path.
fn check_driver(case: &str, v: i32) {
    if v == 0 {
        assert_same_isolated(case, |api| unsafe { api.driver(0) });
    } else {
        assert_same_and_eq(case, b"string\n", |api| unsafe { api.driver(v) });
    }
}

const SEED: u64 = 0x5EED_1234_ABCD_0001;

// ---------------------------------------------------------------------------
// C1 — printLine, non-null, empty string
// ---------------------------------------------------------------------------
#[test]
fn cfg_c1_printline_empty() {
    check_print_line("C1/empty", b"");
}

// ---------------------------------------------------------------------------
// C2 — printLine, 1 byte, all 255 non-NUL values
// ---------------------------------------------------------------------------
#[test]
fn cfg_c2_printline_all_single_bytes() {
    for b in 1u8..=255 {
        check_print_line(&format!("C2/byte={b:#04x}"), &[b]);
    }
}

// ---------------------------------------------------------------------------
// C3 — printLine, randomized ASCII, lengths 0..=64
// ---------------------------------------------------------------------------
#[test]
fn cfg_c3_printline_random_ascii() {
    let mut rng = Rng::new(SEED ^ 3);
    for i in 0..512u32 {
        let len = rng.below(65);
        let payload: Vec<u8> = (0..len).map(|_| rng.byte_in(0x20, 0x7e)).collect();
        check_print_line(&format!("C3/#{i}/len={len}"), &payload);
    }
}

// ---------------------------------------------------------------------------
// C4 — printLine, randomized full byte range (non-UTF-8 included)
// ---------------------------------------------------------------------------
#[test]
fn cfg_c4_printline_random_bytes() {
    let mut rng = Rng::new(SEED ^ 4);
    for i in 0..512u32 {
        let len = 1 + rng.below(128);
        let payload: Vec<u8> = (0..len).map(|_| rng.byte_in(0x01, 0xff)).collect();
        assert!(!payload.contains(&0));
        check_print_line(&format!("C4/#{i}/len={len}"), &payload);
    }
}

// ---------------------------------------------------------------------------
// C5 — printLine, embedded NUL at every position 0..=32
// ---------------------------------------------------------------------------
#[test]
fn cfg_c5_printline_embedded_nul_sweep() {
    let mut rng = Rng::new(SEED ^ 5);
    for pos in 0..=32usize {
        let mut payload: Vec<u8> = (0..40).map(|_| rng.byte_in(0x21, 0x7e)).collect();
        payload[pos] = 0;
        check_print_line(&format!("C5/nul@{pos}"), &payload);
    }
}

// ---------------------------------------------------------------------------
// C6 — printLine, printf format specifiers as *data*
// ---------------------------------------------------------------------------
#[test]
fn cfg_c6_printline_format_chars() {
    for p in [
        &b"%s"[..],
        b"%d",
        b"%n",
        b"%%",
        b"%s %s %s %s %s %s %s %s",
        b"%n%n%n%n",
        b"100%",
        b"%",
        b"%1$s",
        b"%.*s",
        b"%p %x %hhn",
        b"\\%s\\",
    ] {
        check_print_line(&format!("C6/{}", String::from_utf8_lossy(p)), p);
    }
}

// ---------------------------------------------------------------------------
// C7 — printLine, whitespace / newline framing
// ---------------------------------------------------------------------------
#[test]
fn cfg_c7_printline_whitespace() {
    for p in [
        &b"\n"[..],
        b"\r\n",
        b"a\nb",
        b"\n\n\n",
        b"\t\ttabbed",
        b"trailing\n",
        b"  ",
        b"\x0b\x0c\r",
    ] {
        check_print_line(&format!("C7/{p:?}"), p);
    }
}

// ---------------------------------------------------------------------------
// C8 — printLine, long payloads crossing stdio buffer sizes
// ---------------------------------------------------------------------------
#[test]
fn cfg_c8_printline_long() {
    for &len in &[1024usize, 4095, 4096, 4097, 65535, 65536, 65537, 1 << 20] {
        let payload: Vec<u8> = (0..len).map(|i| b'A' + (i % 26) as u8).collect();
        check_print_line(&format!("C8/len={len}"), &payload);
    }
}

// ---------------------------------------------------------------------------
// C9 — printLine, pointer into the middle of a buffer
// ---------------------------------------------------------------------------
#[test]
fn cfg_c9_printline_offset_pointer() {
    let buf: Vec<u8> = b"0123456789abcdefghijklmnopqrstuvwxyz\0".to_vec();
    for off in 0..buf.len() {
        let tail = &buf[off..];
        let exp = expected(&tail[..tail.len() - 1]);
        assert_same_and_eq(&format!("C9/off={off}"), &exp, |api| unsafe {
            api.print_line(buf.as_ptr().add(off) as *const c_char)
        });
    }
}

// ---------------------------------------------------------------------------
// C10 — printLine, NULL (the false branch)
// ---------------------------------------------------------------------------
#[test]
fn cfg_c10_printline_null() {
    // Isolated first: if the NULL check were ever lost, `puts(NULL)` faults, and
    // running in a child turns that into a readable assertion rather than taking
    // the test process down.
    assert_same_and_eq_isolated("C10/null", b"", |api| unsafe {
        api.print_line(std::ptr::null())
    });
    assert_same_and_eq("C10/null-inproc", b"", |api| unsafe {
        api.print_line(std::ptr::null())
    });
}

// ---------------------------------------------------------------------------
// C11 — good()
// ---------------------------------------------------------------------------
#[test]
fn cfg_c11_good() {
    assert_same_and_eq("C11/good", b"string\n", |api| unsafe { api.good() });
    // and repeatedly, to prove the literal is not consumed
    for i in 0..8 {
        assert_same_and_eq(&format!("C11/good#{i}"), b"string\n", |api| unsafe {
            api.good()
        });
    }
}

// ---------------------------------------------------------------------------
// C12 — bad() (uninitialized `data`)
// ---------------------------------------------------------------------------
#[test]
fn cfg_c12_bad() {
    for i in 0..8 {
        assert_same_isolated(&format!("C12/bad#{i}"), |api| unsafe { api.bad() });
    }
}

// ---------------------------------------------------------------------------
// C13 — driver(1): the composed good pipeline
// ---------------------------------------------------------------------------
#[test]
fn cfg_c13_driver_one() {
    check_driver("C13/driver(1)", 1);
}

// ---------------------------------------------------------------------------
// C14 — driver(0): the composed bad pipeline
// ---------------------------------------------------------------------------
#[test]
fn cfg_c14_driver_zero() {
    for i in 0..4 {
        assert_same_isolated(&format!("C14/driver(0)#{i}"), |api| unsafe {
            api.driver(0)
        });
    }
}

// ---------------------------------------------------------------------------
// C15 — driver over 512 randomized i32 plus the small neighbourhood of 0
// ---------------------------------------------------------------------------
#[test]
fn cfg_c15_driver_random_i32() {
    let mut rng = Rng::new(SEED ^ 15);
    for i in 0..512u32 {
        let v = rng.next_i32();
        check_driver(&format!("C15/#{i}/v={v}"), v);
    }
    for v in -4..=4i32 {
        check_driver(&format!("C15/near-zero/v={v}"), v);
    }
}

// ---------------------------------------------------------------------------
// C16 — driver at the i32 boundaries
// ---------------------------------------------------------------------------
#[test]
fn cfg_c16_driver_boundaries() {
    for v in [
        i32::MIN,
        i32::MIN + 1,
        -65537,
        -65536,
        -256,
        -1,
        0,
        1,
        255,
        256,
        65535,
        65536,
        i32::MAX - 1,
        i32::MAX,
    ] {
        check_driver(&format!("C16/v={v}"), v);
    }
}

// ---------------------------------------------------------------------------
// C17 — interleaved low-level + top-level sequence (no hidden state)
// ---------------------------------------------------------------------------
#[test]
fn cfg_c17_interleaved_sequence() {
    let mut rng = Rng::new(SEED ^ 17);
    // Build one deterministic script, then replay it against both libraries in a
    // single capture each: this checks the *whole stream*, including framing
    // between consecutive calls.
    #[derive(Clone)]
    enum Step {
        Line(Vec<u8>),
        Null,
        Good,
        Driver(i32),
    }
    let mut script = Vec::new();
    let mut expect = Vec::new();
    for _ in 0..256 {
        match rng.below(4) {
            0 => {
                let len = rng.below(24);
                let mut p: Vec<u8> = (0..len).map(|_| rng.byte_in(0x21, 0x7e)).collect();
                expect.extend_from_slice(&expected(&p));
                p.push(0);
                script.push(Step::Line(p));
            }
            1 => script.push(Step::Null),
            2 => {
                expect.extend_from_slice(b"string\n");
                script.push(Step::Good);
            }
            _ => {
                // never 0: driver(0) would reach the defect, covered by C14/C19
                let mut v = rng.next_i32();
                if v == 0 {
                    v = 1;
                }
                expect.extend_from_slice(b"string\n");
                script.push(Step::Driver(v));
            }
        }
    }
    let script = script;
    assert_same_and_eq("C17/interleaved", &expect, |api| {
        for step in &script {
            unsafe {
                match step {
                    Step::Line(p) => api.print_line(p.as_ptr() as *const c_char),
                    Step::Null => api.print_line(std::ptr::null()),
                    Step::Good => api.good(),
                    Step::Driver(v) => api.driver(*v),
                }
            }
        }
    });
}

// ---------------------------------------------------------------------------
// C18 — mid-level entry points repeated 100x (no drift)
// ---------------------------------------------------------------------------
#[test]
fn cfg_c18_repeat_no_drift() {
    let mut expect = Vec::new();
    for _ in 0..100 {
        expect.extend_from_slice(b"string\n"); // good()
        expect.extend_from_slice(b"string\n"); // driver(1)
        expect.extend_from_slice(b"x\n"); // printLine("x")
    }
    assert_same_and_eq("C18/repeat", &expect, |api| {
        let s = b"x\0";
        for _ in 0..100 {
            unsafe {
                api.good();
                api.driver(1);
                api.print_line(s.as_ptr() as *const c_char);
            }
        }
    });
    // `bad()` repeated, isolated (it may fault).
    assert_same_isolated("C18/repeat-bad", |api| unsafe {
        for _ in 0..100 {
            api.bad();
        }
    });
}

// ---------------------------------------------------------------------------
// C19 — the axis `bad()`'s indeterminate read is sensitive to:
//       a deliberately dirtied caller stack
// ---------------------------------------------------------------------------
#[test]
fn cfg_c19_dirty_stack_matrix() {
    for fill in [
        0u64,
        1,
        0x4141_4141_4141_4141,
        0xdead_beef_dead_beef,
        u64::MAX,
    ] {
        for depth in 0..3u32 {
            assert_same_isolated(&format!("C19/bad f={fill:#x} d={depth}"), |api| {
                dirty_stack(fill, depth);
                unsafe { api.bad() }
            });
            assert_same_isolated(&format!("C19/driver0 f={fill:#x} d={depth}"), |api| {
                dirty_stack(fill, depth);
                unsafe { api.driver(0) }
            });
            // the well-defined paths must be unaffected by stack contents
            assert_same_isolated(&format!("C19/driver1 f={fill:#x} d={depth}"), |api| {
                dirty_stack(fill, depth);
                unsafe { api.driver(1) }
            });
            assert_same_isolated(&format!("C19/good f={fill:#x} d={depth}"), |api| {
                dirty_stack(fill, depth);
                unsafe { api.good() }
            });
        }
    }
}

// ---------------------------------------------------------------------------
// C19b — `good()` immediately followed by `bad()`: the C's `good` spills the
//        string literal's address into exactly the slot `bad` reads, so the C
//        prints `string` twice. This is the sharpest test of frame-layout parity.
// ---------------------------------------------------------------------------
#[test]
fn cfg_c19b_good_then_bad_frame_aliasing() {
    assert_same_isolated("C19b/good-then-bad", |api| unsafe {
        api.good();
        api.bad();
    });
    assert_same_isolated("C19b/driver1-then-driver0", |api| unsafe {
        api.driver(1);
        api.driver(0);
    });
    assert_same_isolated("C19b/line-then-bad", |api| unsafe {
        let s = b"AAAAAAAAAAAAAAAA\0";
        api.print_line(s.as_ptr() as *const c_char);
        api.bad();
    });
}

// ---------------------------------------------------------------------------
// C20 — printLine fuzz over a mix of all shapes
// ---------------------------------------------------------------------------
#[test]
fn cfg_c20_printline_fuzz_mixed() {
    let mut rng = Rng::new(SEED ^ 20);
    for i in 0..1024u32 {
        let shape = rng.below(6);
        let len = rng.below(96);
        let payload: Vec<u8> = match shape {
            0 => Vec::new(),
            1 => (0..len).map(|_| rng.byte_in(0x20, 0x7e)).collect(),
            2 => (0..len).map(|_| rng.byte_in(0x80, 0xff)).collect(),
            3 => (0..len).map(|_| rng.byte_in(0x01, 0xff)).collect(),
            4 => {
                let mut v: Vec<u8> = (0..len.max(1)).map(|_| rng.byte_in(0x21, 0x7e)).collect();
                let p = rng.below(v.len());
                v[p] = 0; // embedded NUL
                v
            }
            _ => {
                let mut v = Vec::new();
                for _ in 0..len {
                    match rng.below(3) {
                        0 => v.push(b'%'),
                        1 => v.push(b'\n'),
                        _ => v.push(rng.byte_in(0x21, 0x7e)),
                    }
                }
                v
            }
        };
        check_print_line(&format!("C20/#{i}/shape={shape}"), &payload);
    }
}
