//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md` (C1 … C20). Both implementations are driven
//! exclusively through their `.so` exports (`libloading`), lowest-level entry
//! point (`printLine`) first, then the composed ones (`bad`, `good`, `main`).

mod common;

use common::{
    assert_output_is, assert_same_output, assert_same_output_and_ret, assert_same_sequence, impls,
    Op, Rng, SEED,
};
use std::ffi::CString;
use std::os::raw::{c_char, c_int};

/// C1 — `printLine("")`: non-NULL, zero-length payload.
#[test]
fn cfg_c1_print_line_empty() {
    assert_output_is("C1 printLine(\"\")", b"\n", |im| im.print_bytes(b""));
}

/// C2 — `printLine` with every single printable-ASCII byte.
#[test]
fn cfg_c2_print_line_single_ascii() {
    for b in 0x20u8..=0x7e {
        let payload = [b];
        assert_output_is(
            &format!("C2 printLine(single byte {b:#04x})"),
            &[b, b'\n'],
            |im| im.print_bytes(&payload),
        );
    }
}

/// C3 — the exact string literals that appear in the C source.
#[test]
fn cfg_c3_print_line_c_literals() {
    const LITERALS: &[&[u8]] = &[
        b"bad()",
        b"good()",
        b"helperGood()",
        b"helperBad()",
        b"Calling good()...",
        b"Finished good()",
        b"Calling bad()...",
        b"Finished bad()",
    ];
    for lit in LITERALS {
        let mut expected = lit.to_vec();
        expected.push(b'\n');
        assert_output_is(
            &format!("C3 printLine({:?})", String::from_utf8_lossy(lit)),
            &expected,
            |im| im.print_bytes(lit),
        );
    }
}

/// C4 — randomized printable-ASCII payloads, lengths 0..=256.
#[test]
fn cfg_c4_print_line_random_ascii() {
    let mut rng = Rng::new(SEED ^ 4);
    for i in 0..500 {
        let len = rng.range(0, 256);
        let payload = rng.bytes_printable(len);
        assert_same_output(&format!("C4 case {i} len={len}"), |im| {
            im.print_bytes(&payload)
        });
    }
}

/// C5 — randomized full-byte-range payloads (invalid UTF-8 included).
#[test]
fn cfg_c5_print_line_random_bytes() {
    let mut rng = Rng::new(SEED ^ 5);
    for i in 0..500 {
        let len = rng.range(0, 256);
        let payload = rng.bytes_nonzero(len);
        assert_same_output(&format!("C5 case {i} len={len}"), |im| {
            im.print_bytes(&payload)
        });
    }
}

/// C6 — lengths exactly at the stdio buffer boundaries.
#[test]
fn cfg_c6_print_line_buffer_boundaries() {
    let mut rng = Rng::new(SEED ^ 6);
    for len in [4095usize, 4096, 4097, 8191, 8192, 8193, 16383, 16384, 16385] {
        let payload = rng.bytes_printable(len);
        let mut expected = payload.clone();
        expected.push(b'\n');
        assert_output_is(&format!("C6 len={len}"), &expected, |im| {
            im.print_bytes(&payload)
        });
    }
}

/// C7 — very large payloads (64 KiB and 1 MiB).
#[test]
fn cfg_c7_print_line_large() {
    let mut rng = Rng::new(SEED ^ 7);
    for len in [64 * 1024usize, 1024 * 1024] {
        let payload = rng.bytes_nonzero(len);
        let mut expected = payload.clone();
        expected.push(b'\n');
        assert_output_is(&format!("C7 len={len}"), &expected, |im| {
            im.print_bytes(&payload)
        });
    }
}

/// C8 — embedded control bytes and surrounding whitespace.
#[test]
fn cfg_c8_print_line_control_bytes() {
    const CASES: &[&[u8]] = &[
        b"line1\nline2",
        b"a\r\nb",
        b"tab\there",
        b"esc\x1b[31mred\x1b[0m",
        b"del\x7f",
        b"  leading and trailing  ",
        b"\n",
        b"\n\n\n",
        b"\t",
        b"\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f",
        b"mixed\r\n\t\x0b\x0c ok",
    ];
    for case in CASES {
        let mut expected = case.to_vec();
        expected.push(b'\n');
        assert_output_is(&format!("C8 {case:?}"), &expected, |im| im.print_bytes(case));
    }
}

/// C9 — payloads that look like `printf` format strings (must stay data).
#[test]
fn cfg_c9_print_line_format_like() {
    const CASES: &[&[u8]] = &[
        b"%s",
        b"%d",
        b"%n",
        b"%p",
        b"%%",
        b"%s%s%s%s%s%s%s%s",
        b"%n%n%n%n",
        b"%1000000d",
        b"%.*s",
        b"100%% sure",
        b"%hhn%lln",
    ];
    for case in CASES {
        let mut expected = case.to_vec();
        expected.push(b'\n');
        assert_output_is(&format!("C9 {case:?}"), &expected, |im| im.print_bytes(case));
    }
}

/// C10 — `printLine(NULL)`: the guarded branch.
#[test]
fn cfg_c10_print_line_null() {
    assert_output_is("C10 printLine(NULL)", b"", |im| {
        im.print_line(std::ptr::null())
    });
}

/// C11 — `bad()`.
#[test]
fn cfg_c11_bad_single() {
    assert_output_is("C11 bad()", b"bad()\n", |im| im.bad());
}

/// C12 — `good()` (must also emit the `static helperGood()` line).
#[test]
fn cfg_c12_good_single() {
    assert_output_is("C12 good()", b"good()\nhelperGood()\n", |im| im.good());
}

const MAIN_OUTPUT: &[u8] = b"Calling good()...\ngood()\nhelperGood()\nFinished good()\nCalling bad()...\nbad()\nFinished bad()\n";

fn call_main(im: &common::Impl, argc: c_int, args: &[&str]) -> c_int {
    let owned: Vec<CString> = args.iter().map(|a| CString::new(*a).unwrap()).collect();
    let mut ptrs: Vec<*mut c_char> = owned.iter().map(|s| s.as_ptr() as *mut c_char).collect();
    ptrs.push(std::ptr::null_mut());
    im.main(argc, ptrs.as_mut_ptr())
}

/// C13 — `main(1, {"driver", NULL})`.
#[test]
fn cfg_c13_main_normal() {
    assert_output_is("C13 main(1, argv)", MAIN_OUTPUT, |im| {
        call_main(im, 1, &["driver"]);
    });
    assert_same_output_and_ret("C13 main return value", |im| call_main(im, 1, &["driver"]));
}

/// C14 — `main` with extra arguments (ignored by the C body).
#[test]
fn cfg_c14_main_with_args() {
    let args = ["driver", "--verbose", "input.txt", "-n", "42"];
    assert_output_is("C14 main(5, argv)", MAIN_OUTPUT, |im| {
        call_main(im, 5, &args);
    });
    assert_same_output_and_ret("C14 main(5, argv) return value", |im| {
        call_main(im, 5, &args)
    });
}

/// C15 — `main` invoked repeatedly: no state may accumulate.
#[test]
fn cfg_c15_main_repeated() {
    let mut expected = Vec::new();
    for _ in 0..3 {
        expected.extend_from_slice(MAIN_OUTPUT);
    }
    assert_output_is("C15 main x3", &expected, |im| {
        for _ in 0..3 {
            call_main(im, 1, &["driver"]);
        }
    });
    assert_same_output_and_ret("C15 main x3 return values", |im| {
        (0..3)
            .map(|_| call_main(im, 1, &["driver"]))
            .collect::<Vec<_>>()
    });
}

/// C16 — randomized interleaved call sequences over the whole public API.
#[test]
fn cfg_c16_random_call_sequences() {
    let mut rng = Rng::new(SEED ^ 16);
    for seq in 0..200 {
        let n = rng.range(1, 20);
        let mut ops = Vec::with_capacity(n);
        for _ in 0..n {
            ops.push(match rng.below(6) {
                0 => {
                    let len = rng.range(0, 64);
                    Op::PrintLine(rng.bytes_printable(len))
                }
                1 => {
                    let len = rng.range(0, 64);
                    Op::PrintLine(rng.bytes_nonzero(len))
                }
                2 => Op::PrintNull,
                3 => Op::Bad,
                4 => Op::Good,
                _ => {
                    let argc = rng.range(0, 3) as c_int;
                    let args: Vec<Vec<u8>> = (0..argc.max(0) as usize)
                        .map(|_| {
                            let len = rng.range(1, 8);
                            rng.bytes_printable(len)
                        })
                        .collect();
                    Op::Main(argc, args)
                }
            });
        }
        assert_same_sequence(&format!("C16 sequence {seq} ({} ops)", ops.len()), &ops);
    }
}

/// C17 — alternating `bad()` / `good()` calls.
#[test]
fn cfg_c17_bad_good_alternating() {
    let mut expected = Vec::new();
    for i in 0..50 {
        if i % 2 == 0 {
            expected.extend_from_slice(b"bad()\n");
        } else {
            expected.extend_from_slice(b"good()\nhelperGood()\n");
        }
    }
    assert_output_is("C17 bad/good x50", &expected, |im| {
        for i in 0..50 {
            if i % 2 == 0 {
                im.bad();
            } else {
                im.good();
            }
        }
    });
}

/// C18 — end-to-end executable comparison, to a file and through a pipe.
#[test]
fn cfg_c18_executables_end_to_end() {
    use std::process::{Command, Stdio};

    let c_exe = common::c_exe_path();
    let rust_exe = env!("CARGO_BIN_EXE_driver");

    // (a) stdout is a pipe -> C stdio is fully buffered.
    let run_piped = |exe: &str| {
        Command::new(exe)
            .stdin(Stdio::null())
            .output()
            .unwrap_or_else(|e| panic!("failed to run {exe}: {e}"))
    };
    let c_out = run_piped(c_exe.to_str().unwrap());
    let r_out = run_piped(rust_exe);
    assert_eq!(
        c_out.stdout, r_out.stdout,
        "C18 piped stdout mismatch\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c_out.stdout),
        String::from_utf8_lossy(&r_out.stdout)
    );
    assert_eq!(c_out.stderr, r_out.stderr, "C18 piped stderr mismatch");
    assert_eq!(
        c_out.status.code(),
        r_out.status.code(),
        "C18 exit status mismatch"
    );
    assert_eq!(c_out.stdout, MAIN_OUTPUT, "C18 reference output changed");
    assert_eq!(c_out.status.code(), Some(0), "C18 C exit status");

    // (b) stdout is a regular file -> same buffering class, different fd type.
    let run_to_file = |exe: &str, tag: &str| {
        let path = std::env::temp_dir().join(format!("cdiff-exe-{}-{tag}.out", std::process::id()));
        let f = std::fs::File::create(&path).unwrap();
        let status = Command::new(exe)
            .stdin(Stdio::null())
            .stdout(f)
            .status()
            .unwrap_or_else(|e| panic!("failed to run {exe}: {e}"));
        let bytes = std::fs::read(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        (status.code(), bytes)
    };
    let (c_code, c_bytes) = run_to_file(c_exe.to_str().unwrap(), "c");
    let (r_code, r_bytes) = run_to_file(rust_exe, "rust");
    assert_eq!(c_bytes, r_bytes, "C18 file-redirected stdout mismatch");
    assert_eq!(c_code, r_code, "C18 file-redirected exit status mismatch");
    assert_eq!(c_bytes, MAIN_OUTPUT, "C18 file reference output changed");

    // (c) arguments are ignored by the C program.
    let with_args = |exe: &str| {
        Command::new(exe)
            .args(["a", "b", "c"])
            .stdin(Stdio::null())
            .output()
            .unwrap()
    };
    let c_args = with_args(c_exe.to_str().unwrap());
    let r_args = with_args(rust_exe);
    assert_eq!(c_args.stdout, r_args.stdout, "C18 argv stdout mismatch");
    assert_eq!(
        c_args.status.code(),
        r_args.status.code(),
        "C18 argv exit status mismatch"
    );
}

/// C19 — randomized payloads biased towards byte-range edge values.
#[test]
fn cfg_c19_print_line_edge_byte_mix() {
    let mut rng = Rng::new(SEED ^ 19);
    const EDGES: [u8; 8] = [0x01, 0x0a, 0x0d, 0x1b, 0x7e, 0x7f, 0x80, 0xff];
    for i in 0..300 {
        let len = rng.range(0, 96);
        let payload: Vec<u8> = (0..len)
            .map(|_| {
                if rng.below(100) < 70 {
                    EDGES[rng.below(EDGES.len() as u64) as usize]
                } else {
                    rng.nonzero_byte()
                }
            })
            .collect();
        assert_same_output(&format!("C19 case {i} len={len}"), |im| {
            im.print_bytes(&payload)
        });
    }
}

/// C20 — `main` with hostile `argv` contents that must never be read.
#[test]
fn cfg_c20_main_hostile_argv() {
    let big = "x".repeat(70_000);
    let args: Vec<&str> = vec!["\u{1f600}emoji", "%n%n%n", &big, " "];
    assert_output_is("C20 main hostile argv", MAIN_OUTPUT, |im| {
        call_main(im, args.len() as c_int, &args);
    });
    assert_same_output_and_ret("C20 main hostile argv return value", |im| {
        call_main(im, args.len() as c_int, &args)
    });

    // Non-UTF-8 argv bytes, via the sequence driver.
    let ops = vec![Op::Main(
        2,
        vec![vec![0x80, 0xfe, 0xff], vec![0x01, 0x7f, 0xc3]],
    )];
    assert_same_sequence("C20 main non-UTF-8 argv", &ops);

    // Sanity: both libraries really were exercised.
    let (c, r) = impls();
    assert_eq!(c.name, "C");
    assert_eq!(r.name, "Rust");
}
