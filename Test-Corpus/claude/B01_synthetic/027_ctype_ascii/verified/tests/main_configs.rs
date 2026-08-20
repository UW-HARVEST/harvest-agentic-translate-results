//! Phase B -- valid-path differential tests for the `main` entry point of both
//! shared objects (`int main()`: `getchar()` + `driver`).
//!
//! Covers rows 22-30 of CONFIGS.md.

mod common;

use common::*;

#[test]
fn cfg_22_main_single_byte_all() {
    // stdin = seekable file holding exactly one byte, every possible byte.
    let mut vals: Vec<u8> = (0..=255u8).collect();
    let mut rng = Rng::new(SEED ^ 0x2222);
    rng.shuffle(&mut vals);
    for b in vals {
        assert_same(
            &format!("cfg_22/byte 0x{b:02x}"),
            &[Op::Main],
            &Cfg::stdin_file(&[b]),
        );
    }
}

#[test]
fn cfg_23_main_multibyte_file() {
    // Only the first byte is consumed; the rest must not change the output.
    let mut rng = Rng::new(SEED ^ 0x2323);
    for i in 0..64 {
        let len = 2 + rng.below(4095);
        let data = rng.bytes(len);
        let run = assert_same(
            &format!("cfg_23/len {len} @{i}"),
            &[Op::Main],
            &Cfg::stdin_file(&data),
        );
        // Cross-check: identical to feeding just the first byte.
        let single = assert_same(
            &format!("cfg_23/first-byte-only @{i}"),
            &[Op::Main],
            &Cfg::stdin_file(&data[..1]),
        );
        assert_eq!(run.stdout, single.stdout, "only the first byte may matter");
    }
}

#[test]
fn cfg_24_main_large_file() {
    // stdin larger than any stdio buffer (glibc 4 KiB, Rust 8 KiB).
    let mut rng = Rng::new(SEED ^ 0x2424);
    for i in 0..8 {
        let len = 8192 + rng.below(12_000);
        let data = rng.bytes(len);
        assert_same(
            &format!("cfg_24/len {len} @{i}"),
            &[Op::Main],
            &Cfg::stdin_file(&data),
        );
    }
}

#[test]
fn cfg_25_main_stdin_pipe() {
    // Non-seekable stdin.
    let mut rng = Rng::new(SEED ^ 0x2525);
    for i in 0..64 {
        let len = 1 + rng.below(2048);
        let data = rng.bytes(len);
        assert_same(
            &format!("cfg_25/pipe len {len} @{i}"),
            &[Op::Main],
            &Cfg::stdin_pipe(&data),
        );
    }
}

#[test]
fn cfg_26_main_both_pipes() {
    let mut rng = Rng::new(SEED ^ 0x2626);
    for i in 0..64 {
        let len = 1 + rng.below(512);
        let data = rng.bytes(len);
        assert_same(
            &format!("cfg_26/both pipes len {len} @{i}"),
            &[Op::Main],
            &Cfg::stdin_pipe(&data).with_stdout(StdoutSpec::Pipe),
        );
    }
}

#[test]
fn cfg_27_main_eof_both_stdout_shapes() {
    for (name, cfg) in [
        ("file/file", Cfg::stdin_file(&[])),
        (
            "file/pipe",
            Cfg::stdin_file(&[]).with_stdout(StdoutSpec::Pipe),
        ),
        ("pipe/file", Cfg::stdin_pipe(&[])),
        (
            "pipe/pipe",
            Cfg::stdin_pipe(&[]).with_stdout(StdoutSpec::Pipe),
        ),
        (
            "devnull/file",
            Cfg {
                stdin: StdinSpec::DevNull,
                ..Cfg::default()
            },
        ),
    ] {
        for _ in 0..4 {
            assert_same(&format!("cfg_27/{name}"), &[Op::Main], &cfg);
        }
    }
}

#[test]
fn cfg_28_main_host_locale_latin1() {
    // The caller's locale classifies 0x80..0xFF as alphabetic; main() ->
    // driver() forces the "C" locale, so the output must not change.
    let mut rng = Rng::new(SEED ^ 0x2828);
    for loc in ["en_US.iso88591", "de_DE.iso88591", "en_US.utf8", "C.utf8"] {
        if !locale_available(loc) {
            continue;
        }
        for _ in 0..16 {
            let first = 0x80u8 + rng.below(0x80) as u8;
            let mut data = vec![first];
            let extra = rng.below(8);
            data.extend(rng.bytes(extra));
            let cfg = Cfg::stdin_file(&data).with_locale(loc);
            let run = assert_same(&format!("cfg_28/{loc} 0x{first:02x}"), &[Op::Main], &cfg);
            let plain = assert_same(
                &format!("cfg_28/C 0x{first:02x}"),
                &[Op::Main],
                &Cfg::stdin_file(&data),
            );
            assert_eq!(
                run.stdout, plain.stdout,
                "the caller's locale must not leak into the output"
            );
        }
    }
}

#[test]
fn cfg_29_main_twice_same_process() {
    // Two (and three) calls in one process: successive bytes are consumed.
    let mut rng = Rng::new(SEED ^ 0x2929);
    for i in 0..48 {
        let len = 1 + rng.below(64);
        let data = rng.bytes(len);
        assert_same(
            &format!("cfg_29/file x2 len {len} @{i}"),
            &[Op::Main, Op::Main],
            &Cfg::stdin_file(&data),
        );
        assert_same(
            &format!("cfg_29/pipe x3 len {len} @{i}"),
            &[Op::Main, Op::Main, Op::Main],
            &Cfg::stdin_pipe(&data),
        );
    }
    // Past the end of input every further call must behave like EOF.
    assert_same(
        "cfg_29/exhausted",
        &[Op::Main, Op::Main, Op::Main, Op::Main],
        &Cfg::stdin_file(&[b'x', b'y']),
    );
}

#[test]
fn cfg_30_main_then_driver_mixed() {
    // Mixed use of both entry points inside one process.
    let mut rng = Rng::new(SEED ^ 0x3030);
    for i in 0..32 {
        let len = 1 + rng.below(16);
        let data = rng.bytes(len);
        let mut ops = vec![Op::Main];
        for _ in 0..rng.below(6) {
            ops.push(Op::Driver(rng.byte() as i8));
        }
        ops.push(Op::Main);
        ops.push(Op::DriverInt(rng.next_i32()));
        assert_same(
            &format!("cfg_30/mixed @{i}"),
            &ops,
            &Cfg::stdin_file(&data),
        );
        assert_same(
            &format!("cfg_30/mixed-pipe @{i}"),
            &ops,
            &Cfg::stdin_pipe(&data).with_stdout(StdoutSpec::Pipe),
        );
    }
}
