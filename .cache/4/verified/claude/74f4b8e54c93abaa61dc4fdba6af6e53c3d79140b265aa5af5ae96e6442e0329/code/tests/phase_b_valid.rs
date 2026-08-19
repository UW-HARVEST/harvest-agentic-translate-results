//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`.  Every test drives BOTH `.so` files through
//! their exported C symbols and compares the stdout bytes (and, where the C
//! function returns a value, the return value).

mod common;

use common::*;
use std::os::raw::c_int;

// ---------------------------------------------------------------------------
// row 1 — printLine(NULL): the false side of the only branch in the library
// ---------------------------------------------------------------------------
fn row01_print_line_null() {
    diff("row01 printLine(NULL)", |api| api.print_line_null());

    // and it really must produce nothing at all
    let out = capture(|| c_api().print_line_null());
    assert!(out.is_empty(), "C printLine(NULL) wrote {:?}", out);
    let out = capture(|| rust_api().print_line_null());
    assert!(out.is_empty(), "Rust printLine(NULL) wrote {:?}", out);
}

// ---------------------------------------------------------------------------
// row 2 — printLine("")
// ---------------------------------------------------------------------------
fn row02_print_line_empty() {
    diff("row02 printLine(\"\")", |api| api.print_line_bytes(b""));
    let out = capture(|| c_api().print_line_bytes(b""));
    assert_eq!(out, b"\n", "C printLine(\"\") should emit exactly one newline");
}

// ---------------------------------------------------------------------------
// row 3 — every single-byte string 0x01..0xFF (exhaustive)
// ---------------------------------------------------------------------------
fn row03_print_line_every_single_byte() {
    for b in 1u8..=255 {
        let buf = [b];
        diff(&format!("row03 printLine(\\x{b:02x})"), |api| {
            api.print_line_bytes(&buf)
        });
    }
}

// ---------------------------------------------------------------------------
// row 4 — random printable ASCII, random lengths (randomized)
// ---------------------------------------------------------------------------
fn row04_print_line_random_ascii() {
    let mut rng = Rng::new(SEED ^ 4);
    for i in 0..400 {
        let len = rng.range(1, 256);
        let s: Vec<u8> = (0..len).map(|_| rng.range(0x20, 0x7e) as u8).collect();
        diff(&format!("row04 #{i} len={len}"), |api| api.print_line_bytes(&s));
    }
}

// ---------------------------------------------------------------------------
// row 5 — random arbitrary non-NUL bytes, incl. invalid UTF-8 (randomized)
// ---------------------------------------------------------------------------
fn row05_print_line_random_raw_bytes() {
    let mut rng = Rng::new(SEED ^ 5);
    for i in 0..400 {
        let len = rng.range(1, 512);
        let s: Vec<u8> = (0..len).map(|_| rng.byte_nonzero()).collect();
        diff(&format!("row05 #{i} len={len}"), |api| api.print_line_bytes(&s));
    }
    // hand-picked invalid UTF-8 shapes
    let nasty: &[&[u8]] = &[
        &[0x80],
        &[0xbf],
        &[0xc0, 0x80],
        &[0xe0, 0x80, 0x80],
        &[0xf0, 0x80, 0x80, 0x80],
        &[0xf5, 0x80, 0x80, 0x80],
        &[0xfe],
        &[0xff],
        &[0xc3],             // truncated 2-byte sequence
        &[0xe2, 0x82],       // truncated 3-byte sequence
        &[0xed, 0xa0, 0x80], // UTF-16 surrogate encoded as UTF-8
        &[0xff, 0xfe, 0xfd, 0xfc, 0x0a, 0x41],
    ];
    for (i, s) in nasty.iter().enumerate() {
        diff(&format!("row05 invalid-utf8 #{i}"), |api| api.print_line_bytes(s));
    }
}

// ---------------------------------------------------------------------------
// row 6 — printf directives inside the data must stay literal
// ---------------------------------------------------------------------------
fn row06_print_line_format_specifiers() {
    let fixed: &[&[u8]] = &[
        b"%s",
        b"%d",
        b"%n",
        b"%%",
        b"%1$s",
        b"%s%s%s%s%s%s%s%s",
        b"%n%n%n",
        b"100%",
        b"%",
        b"%.*f",
        b"%p %x %o %c %e %g",
        b"%999999999d",
        b"a%sb%dc%%d",
    ];
    for (i, s) in fixed.iter().enumerate() {
        diff(&format!("row06 fixed #{i}"), |api| api.print_line_bytes(s));
    }

    let mut rng = Rng::new(SEED ^ 6);
    let pieces: &[&[u8]] = &[b"%s", b"%d", b"%n", b"%%", b"%p", b"%x", b"%1$s", b"x", b"-"];
    for i in 0..200 {
        let n = rng.range(1, 12);
        let mut s = Vec::new();
        for _ in 0..n {
            s.extend_from_slice(*rng.pick(pieces));
        }
        diff(&format!("row06 random #{i}"), |api| api.print_line_bytes(&s));
    }
}

// ---------------------------------------------------------------------------
// row 7 — embedded control characters / newlines (line-buffer interaction)
// ---------------------------------------------------------------------------
fn row07_print_line_embedded_control_chars() {
    let fixed: &[&[u8]] = &[
        b"\n",
        b"\n\n\n",
        b"a\nb",
        b"\r\n",
        b"a\r\nb\r\n",
        b"\t\t",
        b"\x0b\x0c",
        b"trailing\n",
        b"\nleading",
        b"mixed\r\n\t\x0b\x0c\n",
    ];
    for (i, s) in fixed.iter().enumerate() {
        diff(&format!("row07 fixed #{i}"), |api| api.print_line_bytes(s));
    }

    let mut rng = Rng::new(SEED ^ 7);
    let ctrl = [b'\n', b'\r', b'\t', 0x0b, 0x0c];
    for i in 0..300 {
        let len = rng.range(1, 200);
        let s: Vec<u8> = (0..len)
            .map(|_| {
                if rng.below(3) == 0 {
                    *rng.pick(&ctrl)
                } else {
                    rng.range(0x20, 0x7e) as u8
                }
            })
            .collect();
        diff(&format!("row07 random #{i} len={len}"), |api| {
            api.print_line_bytes(&s)
        });
    }
}

// ---------------------------------------------------------------------------
// row 8 — lengths straddling stdio / LineWriter buffer boundaries
// ---------------------------------------------------------------------------
fn row08_print_line_buffer_boundaries() {
    let lens = [
        1usize, 2, 63, 64, 65, 127, 128, 129, 511, 512, 513, 1023, 1024, 1025, 2047, 2048, 2049,
        4095, 4096, 4097, 8191, 8192, 8193, 16383, 16384, 16385, 65535, 65536, 65537, 1 << 20,
    ];
    for &len in lens.iter() {
        let s = vec![b'A'; len];
        diff(&format!("row08 len={len} plain"), |api| api.print_line_bytes(&s));

        // same lengths, but with newlines sprinkled in so the line-buffered
        // writer has to flush mid-string
        let mut s2 = vec![b'B'; len];
        for i in (0..len).step_by(97) {
            s2[i] = b'\n';
        }
        diff(&format!("row08 len={len} newlines"), |api| {
            api.print_line_bytes(&s2)
        });
    }
}

// ---------------------------------------------------------------------------
// row 9 — valid multi-byte UTF-8
// ---------------------------------------------------------------------------
fn row09_print_line_valid_utf8() {
    let fixed: &[&str] = &[
        "é",
        "ü ö ä ß",
        "日本語テキスト",
        "Ελληνικά",
        "Здравствуй, мир",
        "\u{1F600}\u{1F4A9}\u{1F1FA}\u{1F1F8}",
        "e\u{0301}a\u{0300}",           // combining marks
        "\u{FEFF}BOM at the start",     // BOM
        "\u{7F}\u{80}\u{7FF}\u{800}",   // encoding-width boundaries
        "\u{FFFF}\u{10000}\u{10FFFF}",  // plane boundaries
        "mixed ascii + ünïcödé + 漢字 + 🚀",
    ];
    for (i, s) in fixed.iter().enumerate() {
        diff(&format!("row09 fixed #{i}"), |api| {
            api.print_line_bytes(s.as_bytes())
        });
    }

    let mut rng = Rng::new(SEED ^ 9);
    for i in 0..200 {
        let n = rng.range(1, 40);
        let mut s = String::new();
        for _ in 0..n {
            // random scalar value, skipping NUL and surrogates
            loop {
                let v = 1 + (rng.next_u32() % 0x10_FFFF);
                if let Some(c) = char::from_u32(v) {
                    s.push(c);
                    break;
                }
            }
        }
        let bytes = s.into_bytes();
        diff(&format!("row09 random #{i}"), |api| api.print_line_bytes(&bytes));
    }
}

// ---------------------------------------------------------------------------
// row 10 — printIntLine boundary integers
// ---------------------------------------------------------------------------
fn row10_print_int_line_boundaries() {
    let vals = [
        0i32,
        1,
        -1,
        2,
        -2,
        i32::MAX,
        i32::MAX - 1,
        i32::MIN,
        i32::MIN + 1,
        i16::MAX as i32,
        i16::MIN as i32,
        u16::MAX as i32,
        i8::MAX as i32,
        i8::MIN as i32,
        u8::MAX as i32,
    ];
    for &v in vals.iter() {
        diff(&format!("row10 printIntLine({v})"), |api| unsafe {
            (api.print_int_line)(v as c_int)
        });
    }
}

// ---------------------------------------------------------------------------
// row 11 — printIntLine full-range random i32 (randomized)
// ---------------------------------------------------------------------------
fn row11_print_int_line_random_full_range() {
    let mut rng = Rng::new(SEED ^ 11);
    for i in 0..4096 {
        let v = rng.next_i32();
        diff(&format!("row11 #{i} v={v}"), |api| unsafe {
            (api.print_int_line)(v as c_int)
        });
    }
}

// ---------------------------------------------------------------------------
// row 12 — digit-count transitions and small values (randomized)
// ---------------------------------------------------------------------------
fn row12_print_int_line_digit_transitions() {
    let mut vals: Vec<i32> = Vec::new();
    let mut p: i64 = 1;
    while p <= 1_000_000_000 {
        for d in [-1i64, 0, 1] {
            let v = p + d;
            if v >= i32::MIN as i64 && v <= i32::MAX as i64 {
                vals.push(v as i32);
                vals.push(-(v as i32));
            }
        }
        p *= 10;
    }
    let mut rng = Rng::new(SEED ^ 12);
    for _ in 0..500 {
        vals.push(rng.range(0, 2000) as i32 - 1000);
    }
    for (i, &v) in vals.iter().enumerate() {
        diff(&format!("row12 #{i} v={v}"), |api| unsafe {
            (api.print_int_line)(v as c_int)
        });
    }
}

// ---------------------------------------------------------------------------
// rows 13 & 14 — bad() / good()
// ---------------------------------------------------------------------------
fn row13_bad_single_call() {
    diff("row13 bad()", |api| unsafe { (api.bad)() });
    // the discarded `intOne + intTwo;` means intSum stays 0 — reproduce, don't fix
    let out = capture(|| unsafe { (c_api().bad)() });
    assert_eq!(out, b"0\n0\n", "C bad() reference output changed");
}

fn row14_good_single_call() {
    diff("row14 good()", |api| unsafe { (api.good)() });
    let out = capture(|| unsafe { (c_api().good)() });
    assert_eq!(out, b"0\n2\n", "C good() reference output changed");
}

// ---------------------------------------------------------------------------
// row 15 — repeated / interleaved bad() and good() (randomized)
// ---------------------------------------------------------------------------
fn row15_bad_good_interleaved() {
    let mut rng = Rng::new(SEED ^ 15);
    for i in 0..100 {
        let n = rng.range(1, 20);
        let plan: Vec<bool> = (0..n).map(|_| rng.below(2) == 0).collect();
        diff(&format!("row15 #{i} n={n}"), |api| {
            for &is_bad in plan.iter() {
                unsafe {
                    if is_bad {
                        (api.bad)()
                    } else {
                        (api.good)()
                    }
                }
            }
        });
    }
}

// ---------------------------------------------------------------------------
// row 16 — main(1, ["driver"]) : output AND return value
// ---------------------------------------------------------------------------
fn row16_main_argc1() {
    diff_with("row16 main(1, argv)", |api| {
        let argv = Argv::new(&["driver"]);
        api.call_main(argv.argc(), argv.as_ptr())
    });

    let (out, ret) = capture_ret(|| {
        let argv = Argv::new(&["driver"]);
        c_api().call_main(argv.argc(), argv.as_ptr())
    });
    assert_eq!(ret, 0, "C main must return 0");
    assert_eq!(
        out, EXPECTED_PROGRAM_OUTPUT,
        "C main reference output changed"
    );
}

// ---------------------------------------------------------------------------
// row 17 — main(0, NULL)
// ---------------------------------------------------------------------------
fn row17_main_argc0_null_argv() {
    diff_with("row17 main(0, NULL)", |api| {
        api.call_main(0, std::ptr::null_mut())
    });
}

// ---------------------------------------------------------------------------
// row 18 — main with many argv entries
// ---------------------------------------------------------------------------
fn row18_main_many_args() {
    let args: Vec<String> = (0..64).map(|i| format!("arg{i}")).collect();
    let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    diff_with("row18 main(64, argv)", |api| {
        let argv = Argv::new(&refs);
        api.call_main(argv.argc(), argv.as_ptr())
    });
}

// ---------------------------------------------------------------------------
// row 19 — main called twice
// ---------------------------------------------------------------------------
fn row19_main_twice() {
    diff_with("row19 main twice", |api| {
        let argv = Argv::new(&["driver"]);
        let a = api.call_main(argv.argc(), argv.as_ptr());
        let b = api.call_main(argv.argc(), argv.as_ptr());
        (a, b)
    });

    let (out, _) = capture_ret(|| {
        let argv = Argv::new(&["driver"]);
        let a = c_api().call_main(argv.argc(), argv.as_ptr());
        let b = c_api().call_main(argv.argc(), argv.as_ptr());
        (a, b)
    });
    let mut expect = EXPECTED_PROGRAM_OUTPUT.to_vec();
    expect.extend_from_slice(EXPECTED_PROGRAM_OUTPUT);
    assert_eq!(out, expect);
}

// ---------------------------------------------------------------------------
// row 20 — randomized interleaving of all five entry points in one capture
// ---------------------------------------------------------------------------
#[derive(Clone)]
enum Op {
    Line(Vec<u8>),
    LineNull,
    Int(i32),
    Bad,
    Good,
    Main,
}

fn row20_mixed_sequences() {
    let mut rng = Rng::new(SEED ^ 20);
    for t in 0..150 {
        let n = rng.range(1, 25);
        let mut plan = Vec::with_capacity(n);
        for _ in 0..n {
            plan.push(match rng.below(6) {
                0 => {
                    let len = rng.below(40);
                    Op::Line((0..len).map(|_| rng.byte_nonzero()).collect())
                }
                1 => Op::LineNull,
                2 => Op::Int(rng.next_i32()),
                3 => Op::Bad,
                4 => Op::Good,
                _ => Op::Main,
            });
        }
        diff_with(&format!("row20 #{t} n={n}"), |api| {
            let mut rets = Vec::new();
            for op in plan.iter() {
                match op {
                    Op::Line(b) => api.print_line_bytes(b),
                    Op::LineNull => api.print_line_null(),
                    Op::Int(v) => unsafe { (api.print_int_line)(*v as c_int) },
                    Op::Bad => unsafe { (api.bad)() },
                    Op::Good => unsafe { (api.good)() },
                    Op::Main => {
                        let argv = Argv::new(&["driver", "x"]);
                        rets.push(api.call_main(argv.argc(), argv.as_ptr()));
                    }
                }
            }
            rets
        });
    }
}

// ---------------------------------------------------------------------------
// rows 21 & 22 — process level: the C executable vs the Rust executable
// ---------------------------------------------------------------------------
use std::io::Write as _;
use std::process::{Command, Stdio};

fn run_exe(exe: &std::path::Path, args: &[&str], stdin_data: &[u8]) -> (Vec<u8>, Vec<u8>, i32) {
    let mut child = Command::new(exe)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", exe.display()));
    {
        let mut si = child.stdin.take().unwrap();
        let _ = si.write_all(stdin_data);
        let _ = si.flush();
    }
    let out = child.wait_with_output().expect("wait");
    (out.stdout, out.stderr, out.status.code().unwrap_or(-1))
}

fn row21_executable_no_args() {
    let c = c_executable();
    let r = rust_executable();
    let (co, ce, cs) = run_exe(&c, &[], b"");
    let (ro, re, rs) = run_exe(&r, &[], b"");
    assert_bytes_eq("row21 stdout", &co, &ro);
    assert_bytes_eq("row21 stderr", &ce, &re);
    assert_eq!(cs, rs, "row21 exit status: C={cs} Rust={rs}");
    assert_eq!(co, EXPECTED_PROGRAM_OUTPUT, "C program reference output changed");
    assert_eq!(cs, 0);
}

fn row22_executable_with_args_and_stdin() {
    let c = c_executable();
    let r = rust_executable();
    let stdin_data = vec![b'z'; 4096];
    for args in [
        vec!["one"],
        vec!["--flag", "value"],
        vec!["", "  ", "\t"],
        (0..40).map(|_| "arg").collect::<Vec<_>>(),
    ] {
        let (co, ce, cs) = run_exe(&c, &args, &stdin_data);
        let (ro, re, rs) = run_exe(&r, &args, &stdin_data);
        assert_bytes_eq(&format!("row22 stdout args={args:?}"), &co, &ro);
        assert_bytes_eq(&format!("row22 stderr args={args:?}"), &ce, &re);
        assert_eq!(cs, rs, "row22 exit status args={args:?}");
    }
}

// ---------------------------------------------------------------------------
// runner — one entry per CONFIGS.md row, executed sequentially
// ---------------------------------------------------------------------------
fn main() {
    println!(
        "C   .so: {}\nRust .so: {}",
        // touch both libraries first so load failures are reported up front
        {
            let _ = c_api();
            "c_build/libdriver_c.so"
        },
        {
            let _ = rust_api();
            rust_shared_object().display().to_string()
        }
    );

    let mut s = Suite::new("Phase B — valid paths (CONFIGS.md)");
    s.run("row01 printLine(NULL)", row01_print_line_null);
    s.run("row02 printLine(\"\")", row02_print_line_empty);
    s.run("row03 printLine single byte 0x01..0xFF", row03_print_line_every_single_byte);
    s.run("row04 printLine random ASCII", row04_print_line_random_ascii);
    s.run("row05 printLine random raw bytes", row05_print_line_random_raw_bytes);
    s.run("row06 printLine printf directives as data", row06_print_line_format_specifiers);
    s.run("row07 printLine embedded control chars", row07_print_line_embedded_control_chars);
    s.run("row08 printLine buffer boundaries", row08_print_line_buffer_boundaries);
    s.run("row09 printLine valid multi-byte UTF-8", row09_print_line_valid_utf8);
    s.run("row10 printIntLine boundary ints", row10_print_int_line_boundaries);
    s.run("row11 printIntLine random full-range i32", row11_print_int_line_random_full_range);
    s.run("row12 printIntLine digit transitions", row12_print_int_line_digit_transitions);
    s.run("row13 bad()", row13_bad_single_call);
    s.run("row14 good()", row14_good_single_call);
    s.run("row15 bad()/good() interleaved", row15_bad_good_interleaved);
    s.run("row16 main(1, argv)", row16_main_argc1);
    s.run("row17 main(0, NULL)", row17_main_argc0_null_argv);
    s.run("row18 main(64, argv)", row18_main_many_args);
    s.run("row19 main twice", row19_main_twice);
    s.run("row20 mixed sequences of all 5 entry points", row20_mixed_sequences);
    s.run("row21 executable, no args", row21_executable_no_args);
    s.run("row22 executable, args + stdin", row22_executable_with_args_and_stdin);
    s.finish();
}
