//! Phase B — valid-path differential tests, one test per row of CONFIGS.md.
//!
//! Both implementations are always reached through their shared libraries
//! (`dlopen` + `dlsym`), never by calling a Rust function directly.

mod common;

use common::*;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------
//
// Rows B01..B06 (in-process `dlopen` calls of `printHexCharLine`) live in
// `tests/inprocess.rs`, which runs with `harness = false` because capturing
// fd 1 is process wide.  Everything here drives the libraries in a *fresh
// process* through `examples/so_runner.rs`, or runs the linked executables.

// ---------------------------------------------------------------------------
// B07 — printHexCharLine in a fresh process (dlopen in so_runner)
// ---------------------------------------------------------------------------

#[test]
fn b07_print_hex_char_line_fresh_process_exhaustive() {
    let a = artifacts();
    for b in 0u16..=255 {
        let v = (b as u8) as i8 as i32;
        let c = run_symbol(
            &a.c_so,
            "printHexCharLine",
            Some(v),
            &StdinSpec::DevNull,
            StdoutSpec::File,
        );
        let r = run_symbol(
            &a.rust_so,
            "printHexCharLine",
            Some(v),
            &StdinSpec::DevNull,
            StdoutSpec::File,
        );
        assert_same(&format!("B07 printHexCharLine({v})"), &c, &r);
    }
}

// ---------------------------------------------------------------------------
// main() rows
// ---------------------------------------------------------------------------

fn compare_main(stdin: &StdinSpec, stdout: StdoutSpec, ctx: &str) {
    let a = artifacts();
    let c = run_symbol(&a.c_so, "main", None, stdin, stdout);
    let r = run_symbol(&a.rust_so, "main", None, stdin, stdout);
    assert_same(ctx, &c, &r);
}

// B08 — stdin = exactly one byte, all 256 values
#[test]
fn b08_main_single_byte_stdin_exhaustive() {
    for b in 0u16..=255 {
        let byte = b as u8;
        compare_main(
            &StdinSpec::File(vec![byte]),
            StdoutSpec::File,
            &format!("B08 stdin=[{byte:#04x}]"),
        );
    }
}

// B09 — stdin = empty file
#[test]
fn b09_main_empty_stdin() {
    compare_main(&StdinSpec::File(Vec::new()), StdoutSpec::File, "B09 empty");
}

// B10 — stdin = two bytes; only the first is converted
#[test]
fn b10_main_two_byte_stdin_exhaustive_first_byte() {
    let mut rng = Rng::new(0x0BAD_C0DE_0BAD_C0DE);
    for b in 0u16..=255 {
        let bytes = vec![b as u8, rng.next_u8()];
        compare_main(
            &StdinSpec::File(bytes.clone()),
            StdoutSpec::File,
            &format!("B10 stdin={}", hex(&bytes)),
        );
    }
}

// B11 — stdin = randomized short buffers
#[test]
fn b11_main_random_short_stdin() {
    let mut rng = Rng::new(Rng::DEFAULT_SEED);
    for i in 0..512 {
        let len = rng.range(3, 64);
        let bytes = rng.bytes(len);
        compare_main(
            &StdinSpec::File(bytes.clone()),
            StdoutSpec::File,
            &format!("B11 #{i} len={len} first={:#04x}", bytes[0]),
        );
    }
}

// B12 — stdin larger than glibc's BUFSIZ and Rust's 8 KiB BufReader
#[test]
fn b12_main_large_stdin() {
    let mut rng = Rng::new(0xFEED_FACE_FEED_FACE);
    for i in 0..32 {
        let len = rng.range(4 * 1024, 64 * 1024);
        let bytes = rng.bytes(len);
        compare_main(
            &StdinSpec::File(bytes.clone()),
            StdoutSpec::File,
            &format!("B12 #{i} len={len} first={:#04x}", bytes[0]),
        );
    }
}

// B13 — leading whitespace must NOT be skipped by "%c"
#[test]
fn b13_main_leading_whitespace_not_skipped() {
    let cases: Vec<&[u8]> = vec![
        b"\n\nA",
        b" A",
        b"\tA",
        b"\r\n",
        b"\n",
        b" ",
        b"\t",
        b"\x0b\x0c",
        b"   \n\t  x",
    ];
    for c in cases {
        compare_main(
            &StdinSpec::File(c.to_vec()),
            StdoutSpec::File,
            &format!("B13 stdin={}", hex(c)),
        );
    }
}

// B14 — stdin is a pipe (not seekable)
#[test]
fn b14_main_stdin_is_pipe() {
    let mut rng = Rng::new(0xA5A5_5A5A_A5A5_5A5A);
    for i in 0..128 {
        let len = rng.range(1, 1024);
        let bytes = rng.bytes(len);
        compare_main(
            &StdinSpec::Pipe(bytes.clone()),
            StdoutSpec::File,
            &format!("B14 #{i} len={len} first={:#04x}", bytes[0]),
        );
    }
    // plus the empty pipe (immediate EOF)
    compare_main(&StdinSpec::Pipe(Vec::new()), StdoutSpec::File, "B14 empty");
}

// B15 — stdin = /dev/null
#[test]
fn b15_main_stdin_devnull() {
    compare_main(&StdinSpec::DevNull, StdoutSpec::File, "B15 /dev/null");
}

// B16 — stdout = pipe
#[test]
fn b16_main_stdout_is_pipe() {
    let mut rng = Rng::new(0x5EED_1234_5EED_1234);
    for i in 0..128 {
        let len = rng.range(1, 64);
        let bytes = rng.bytes(len);
        compare_main(
            &StdinSpec::File(bytes.clone()),
            StdoutSpec::Pipe,
            &format!("B16 #{i} first={:#04x}", bytes[0]),
        );
    }
}

// B17 — stdout = /dev/null (only the exit status is observable)
#[test]
fn b17_main_stdout_devnull() {
    let mut rng = Rng::new(0x9999_1111_2222_3333);
    for _ in 0..16 {
        let len = rng.range(0, 32);
        let bytes = rng.bytes(len);
        compare_main(&StdinSpec::File(bytes), StdoutSpec::DevNull, "B17");
    }
}

// B18 — exit status is always 0 for both implementations
#[test]
fn b18_main_exit_status_is_always_zero() {
    let a = artifacts();
    let mut rng = Rng::new(0x7777_8888_9999_AAAA);
    for _ in 0..64 {
        let len = rng.range(0, 16);
        let bytes = rng.bytes(len);
        let stdin = StdinSpec::File(bytes);
        let c = run_symbol(&a.c_so, "main", None, &stdin, StdoutSpec::File);
        let r = run_symbol(&a.rust_so, "main", None, &stdin, StdoutSpec::File);
        assert_eq!(c.status, Some(0), "C main must exit 0");
        assert_eq!(r.status, Some(0), "Rust main must exit 0");
        assert_same("B18", &c, &r);
    }
}

// B21 — stdin is a terminal (glibc switches `stdin` to line buffering)
#[test]
fn b21_main_stdin_is_a_tty() {
    use std::ffi::OsStr;
    let a = artifacts();
    let cases: Vec<&[u8]> = vec![
        b"A\n",
        b"\n",
        b" \n",
        b"z\n",
        b"0\n",
        b"\t\n",
        b"~\n",
        b"hello\n",
        b"\x04\n", // EOT: the line discipline turns this into EOF
    ];
    for input in cases {
        let c = run_on_pty(
            &a.runner,
            &[a.c_so.as_os_str(), OsStr::new("main")],
            input,
        );
        let r = run_on_pty(
            &a.runner,
            &[a.rust_so.as_os_str(), OsStr::new("main")],
            input,
        );
        assert_same(&format!("B21 tty stdin={}", hex(input)), &c, &r);

        // ... and the same through the two executables.
        let ce = run_on_pty(&a.c_exe, &[], input);
        let re = run_on_pty(&a.rust_exe, &[], input);
        assert_same(&format!("B21 tty exe stdin={}", hex(input)), &ce, &re);
        assert_same(&format!("B21 tty so-vs-exe stdin={}", hex(input)), &c, &ce);

        // Ground truth, so the row cannot pass vacuously.
        let want = match input {
            b"A\n" => Some("42\n"),
            b"\n" => Some("0b\n"),
            b" \n" => Some("21\n"),
            b"\t\n" => Some("0a\n"),
            b"z\n" => Some("7b\n"),
            b"\x04\n" => Some("21\n"), // EOF before any conversion
            _ => None,
        };
        if let Some(want) = want {
            assert_eq!(
                String::from_utf8_lossy(&c.stdout),
                want,
                "B21: unexpected C output for {}",
                hex(input)
            );
        }
    }
}

// ---------------------------------------------------------------------------
// B19 / B20 — the linked executables, end to end
// ---------------------------------------------------------------------------

#[test]
fn b19_executables_end_to_end() {
    let a = artifacts();
    // exhaustive single byte
    for b in 0u16..=255 {
        let stdin = StdinSpec::File(vec![b as u8]);
        let c = run_exe(&a.c_exe, &stdin, StdoutSpec::File);
        let r = run_exe(&a.rust_exe, &stdin, StdoutSpec::File);
        assert_same(&format!("B19 exe stdin=[{b:#04x}]"), &c, &r);
    }
    // randomized multi-byte
    let mut rng = Rng::new(Rng::DEFAULT_SEED ^ 0xFFFF);
    for i in 0..256 {
        let len = rng.range(0, 300);
        let bytes = rng.bytes(len);
        let stdin = StdinSpec::File(bytes.clone());
        let c = run_exe(&a.c_exe, &stdin, StdoutSpec::File);
        let r = run_exe(&a.rust_exe, &stdin, StdoutSpec::File);
        assert_same(&format!("B19 exe #{i} len={len}"), &c, &r);
    }
    // also through the CMake-built binary if it is present
    let cmake_exe = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/driver");
    if cmake_exe.is_file() {
        for b in [0u8, 0x20, 0x7f, 0x80, 0xff] {
            let stdin = StdinSpec::File(vec![b]);
            let c = run_exe(&cmake_exe, &stdin, StdoutSpec::File);
            let r = run_exe(&a.rust_exe, &stdin, StdoutSpec::File);
            assert_same(&format!("B19 cmake exe stdin=[{b:#04x}]"), &c, &r);
        }
    }
}

#[test]
fn b20_release_executable_end_to_end() {
    let a = artifacts();
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out = std::process::Command::new(env!("CARGO"))
        .current_dir(&manifest)
        .args(["build", "--offline", "--release", "--bin", "driver"])
        .output()
        .expect("cargo build --release");
    assert!(
        out.status.success(),
        "cargo build --release failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let release_exe = manifest.join("target/release/driver");
    assert!(release_exe.is_file(), "missing {}", release_exe.display());

    let mut rng = Rng::new(0xBEEF_0001_BEEF_0001);
    let mut cases: Vec<Vec<u8>> = vec![
        Vec::new(),
        vec![0x00],
        vec![0x20],
        vec![0x7f],
        vec![0x80],
        vec![0xff],
        b"\n".to_vec(),
        b"hello world".to_vec(),
    ];
    for _ in 0..64 {
        let len = rng.range(0, 128);
        cases.push(rng.bytes(len));
    }
    for (i, bytes) in cases.iter().enumerate() {
        let stdin = StdinSpec::File(bytes.clone());
        let c = run_exe(&a.c_exe, &stdin, StdoutSpec::File);
        let r = run_exe(&release_exe, &stdin, StdoutSpec::File);
        assert_same(&format!("B20 release exe #{i}"), &c, &r);
    }
}

// ---------------------------------------------------------------------------
// C2 — the -O2 build of the C library must behave the same too
// ---------------------------------------------------------------------------

#[test]
fn c2_optimised_c_library_matches() {
    // (the in-process `printHexCharLine` half of row C2 lives in
    // tests/inprocess.rs)
    let a = artifacts();
    for b in 0u16..=255 {
        let stdin = StdinSpec::File(vec![b as u8]);
        let c = run_symbol(&a.c_so_o2, "main", None, &stdin, StdoutSpec::File);
        let r = run_symbol(&a.rust_so, "main", None, &stdin, StdoutSpec::File);
        assert_same(&format!("C2 main stdin=[{b:#04x}]"), &c, &r);
    }
    for v in [0i32, 1, 0x7f, -128, -1, 0x1ff, i32::MIN, i32::MAX] {
        let c = run_symbol(
            &a.c_so_o2,
            "printHexCharLine",
            Some(v),
            &StdinSpec::DevNull,
            StdoutSpec::File,
        );
        let r = run_symbol(
            &a.rust_so,
            "printHexCharLine",
            Some(v),
            &StdinSpec::DevNull,
            StdoutSpec::File,
        );
        assert_same(&format!("C2 printHexCharLine({v})"), &c, &r);
    }
}
