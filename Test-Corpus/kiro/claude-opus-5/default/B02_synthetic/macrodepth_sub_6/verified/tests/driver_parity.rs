// CONFIGS.md group M -- end-to-end differential test of the whole program.
//
// `mdmain.c` composes every entry point (`op_<OP>`, `RUN_LOOP`, `helper_call`,
// `helper_ptr`, `use_generated`, `G_OP`, `G_OP_NAME`) and prints four lines.
// Per-function tests cannot see ordering or interleaving bugs in that pipeline,
// so the two executables are compared byte for byte on stdout, stderr and exit
// status.

#[path = "support/mod.rs"]
mod support;

use std::process::Command;
use support::*;

fn both() -> Option<(std::path::PathBuf, std::path::PathBuf)> {
    let c = c_exe_path();
    let r = rust_exe_path();
    if c.exists() && r.exists() {
        Some((c, r))
    } else {
        eprintln!(
            "skipping driver parity: missing {}{}",
            if c.exists() { "" } else { "C exe " },
            if r.exists() { "" } else { "Rust exe" }
        );
        None
    }
}

fn compare(c: &std::path::Path, r: &std::path::Path, args: &[String]) {
    let co = Command::new(c).args(args).output().unwrap();
    let ro = Command::new(r).args(args).output().unwrap();
    assert_eq!(
        co.status.code(),
        ro.status.code(),
        "exit status for {args:?} [OP={OP} REPEAT={REPEAT}]"
    );
    assert_eq!(
        String::from_utf8_lossy(&co.stdout),
        String::from_utf8_lossy(&ro.stdout),
        "stdout for {args:?} [OP={OP} REPEAT={REPEAT}]"
    );
    assert_eq!(
        co.stderr, ro.stderr,
        "stderr for {args:?} [OP={OP} REPEAT={REPEAT}]"
    );
    // Sanity: the configured OP name really is in the output.
    if !co.stdout.is_empty() {
        assert!(
            String::from_utf8_lossy(&co.stdout).contains(&format!("op={OP} ")),
            "C stdout does not report op={OP}: {:?}",
            String::from_utf8_lossy(&co.stdout)
        );
    }
}

/// M-xx -- hand-picked argv shapes: zero, one, negatives, boundaries and the
/// `atoi` oddities, for the active OP x REPEAT configuration.
#[test]
fn m_driver_fixed_inputs() {
    let Some((c, r)) = both() else { return };
    let fixed: &[&[&str]] = &[
        &["7", "3"],
        &["0", "0"],
        &["1", "1"],
        &["-5", "9"],
        &["3", "-4"],
        &["-1", "-1"],
        &["2147483647", "1"],
        &["2147483647", "2147483647"],
        &["-2147483648", "-1"],
        &["-2147483648", "-2147483648"],
        &["46341", "46341"],
        &["65536", "65536"],
        &["  -12abc", "+9"],
        &["99999999999999999999", "3"],
        &["12x", "7"],
        &["abc", "def"],
        &["", ""],
        // extra argv beyond A and B is ignored by the C code
        &["7", "3", "ignored", "also-ignored"],
    ];
    for args in fixed {
        let owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        compare(&c, &r, &owned);
    }
}

/// M-xx -- randomized argv, fixed seed. Covers value-dependent overflow in the
/// composed `summary=` accumulation, which no single hand-picked pair reaches.
#[test]
fn m_driver_randomized_inputs() {
    let Some((c, r)) = both() else { return };
    let mut rng = Rng::new(0x0D_21_4E_12);
    for _ in 0..300 {
        let a = rng.next_i32();
        let b = rng.next_i32();
        compare(&c, &r, &[a.to_string(), b.to_string()]);
    }
}

/// M-xx -- the `argc < 3` usage path, with argv[0] normalised away.
#[test]
fn m_driver_usage_path() {
    let Some((c, r)) = both() else { return };
    for args in [Vec::<String>::new(), vec!["only-one".to_string()]] {
        let co = Command::new(&c).args(&args).output().unwrap();
        let ro = Command::new(&r).args(&args).output().unwrap();
        assert_eq!(co.status.code(), Some(2));
        assert_eq!(co.status.code(), ro.status.code());
        assert!(co.stdout.is_empty() && ro.stdout.is_empty());
        let cn = String::from_utf8_lossy(&co.stderr).replace(c.to_str().unwrap(), "PROG");
        let rn = String::from_utf8_lossy(&ro.stderr).replace(r.to_str().unwrap(), "PROG");
        assert_eq!(cn, rn);
        assert_eq!(cn, "usage: PROG A B\n");
    }
}

/// The four output lines must appear in the same order with the same field
/// names -- a stdout/stderr buffering difference would show up here because the
/// C `printf` stream is fully buffered when redirected while Rust's `println!`
/// is line buffered.
#[test]
fn m_driver_line_structure() {
    let Some((c, r)) = both() else { return };
    let co = Command::new(&c).args(["7", "3"]).output().unwrap();
    let ro = Command::new(&r).args(["7", "3"]).output().unwrap();
    let ct = String::from_utf8_lossy(&co.stdout);
    let rt = String::from_utf8_lossy(&ro.stdout);
    assert_eq!(ct, rt);
    let lines: Vec<&str> = ct.lines().collect();
    assert_eq!(lines.len(), 5, "unexpected line count: {ct:?}");
    assert!(lines[0].starts_with("helper.call="), "{:?}", lines[0]);
    assert!(lines[1].starts_with("helper.ptr="), "{:?}", lines[1]);
    assert!(lines[2].starts_with("gen.acc="), "{:?}", lines[2]);
    assert!(lines[3].starts_with(&format!("op={OP} call=")), "{:?}", lines[3]);
    assert!(lines[4].starts_with("summary="), "{:?}", lines[4]);
    assert!(ct.ends_with('\n'), "missing trailing newline");
}
