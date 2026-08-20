// CONFIGS.md row 36 — differential testing through the real process boundary.
//
// `c_src/CMakeLists.txt` builds an executable, so this is the artefact pair a
// real user runs.  Here standard input is a *pipe* rather than a file (which
// exercises a different stdio buffering path from `tests/ffi_main.rs`) and both
// stdout, stderr and the exit status are compared.

mod common;

use common::{
    c_exe, diff_exe, push_digits, push_n_digits, push_sign, push_ws, run_exe, rust_exe, Rng,
    DIGITS, HEXDIGITS, SEED, WS,
};

fn gen<F: FnMut(&mut Rng, &mut Vec<u8>)>(seed: u64, n: usize, mut f: F) -> Vec<Vec<u8>> {
    let mut rng = Rng::new(seed);
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let mut v = Vec::new();
        f(&mut rng, &mut v);
        out.push(v);
    }
    out
}

/// A hand-picked regression set covering one input per interesting code path.
fn cli_representative_inputs() {
    let inputs: Vec<Vec<u8>> = [
        "", " ", "\n", "\t\x0b\x0c\r ", "-", "+", ".", "..", "0", "-0", "+0", "1", "-1", "0.0",
        "1.5", "-1.5", ".5", "5.", "1e5", "1E-5", "1e", "1e+", "1e309", "-1e309", "1e-400",
        "-1e-400", "0x", "-0x", "+0X", "0x.", "-0x.", "0x1", "0X1", "0x1.8", "0x.8", "0xa.",
        "0x1p3", "0x1P-3", "0x1p", "0x1pa", "0x1p2f3", "0x1p1024", "0x1p-1075", "0x1p-1074",
        "inf", "INF", "-inf", "+inf", "infinity", "INFINITY", "-InFiNiTy", "infi", "infinit",
        "nan", "NAN", "-nan", "nan(1)", "-nan(123)", "n", "na", "i", "in", "abc", "@", "e5",
        "1.7976931348623157e308", "-1.7976931348623157e308", "5e-324", "-5e-324",
        "2.2250738585072014e-308", "9007199254740993", "0.15625", "-0.15625", "0.09375",
        "1 2 3", "  42\n", "\n\n\n7", "1.2.3", "1,000", "1'0", "0x1.fffffffffffff8p1023",
        "123456789012345678901234567890", "0.000000000000000000000000001",
    ]
    .iter()
    .map(|s: &&str| s.as_bytes().to_vec())
    .collect();
    diff_exe("cli representative", &inputs);
}

/// Randomized decimal numbers through the process boundary.
fn cli_random_decimal() {
    let inputs = gen(SEED ^ 101, 250, |rng, v| {
        push_ws(rng, v);
        push_sign(rng, v);
        push_n_digits(rng, v, 1, 12, DIGITS);
        if rng.flip() {
            v.push(b'.');
            push_n_digits(rng, v, 0, 12, DIGITS);
        }
        if rng.flip() {
            v.push(if rng.flip() { b'e' } else { b'E' });
            push_sign(rng, v);
            v.extend_from_slice(format!("{}", rng.range(0, 330)).as_bytes());
        }
    });
    diff_exe("cli random decimal", &inputs);
}

/// Randomized hexadecimal numbers through the process boundary.
fn cli_random_hex() {
    let inputs = gen(SEED ^ 102, 250, |rng, v| {
        push_ws(rng, v);
        push_sign(rng, v);
        v.push(b'0');
        v.push(if rng.flip() { b'x' } else { b'X' });
        push_n_digits(rng, v, 0, 18, HEXDIGITS);
        if rng.flip() {
            v.push(b'.');
            push_n_digits(rng, v, 0, 18, HEXDIGITS);
        }
        if rng.flip() {
            v.push(if rng.flip() { b'p' } else { b'P' });
            push_sign(rng, v);
            v.extend_from_slice(format!("{}", rng.range(0, 1100)).as_bytes());
        }
    });
    diff_exe("cli random hex", &inputs);
}

/// Randomized malformed input through the process boundary.
fn cli_random_garbage() {
    const ALPHABET: &[u8] = b"0123456789abcdefxXpPeE+-., \t\nnif()\0";
    let inputs = gen(SEED ^ 103, 300, |rng, v| {
        let n = rng.below(8) as usize;
        for _ in 0..n {
            v.push(*rng.pick(ALPHABET));
        }
    });
    diff_exe("cli random garbage", &inputs);
}

/// Big inputs through the pipe: more than one pipe buffer (64 KiB) of leading
/// white space, and a very long digit string.
fn cli_large_inputs() {
    let mut inputs: Vec<Vec<u8>> = Vec::new();

    let mut ws = vec![b' '; 200_000];
    ws.extend_from_slice(b"2.5");
    inputs.push(ws);

    let mut mixed = Vec::new();
    for i in 0..100_000 {
        mixed.push(WS[i % WS.len()]);
    }
    mixed.extend_from_slice(b"-0x1.8p4");
    inputs.push(mixed);

    let mut rng = Rng::new(SEED ^ 104);
    let mut digits = Vec::new();
    push_digits(&mut rng, &mut digits, 20_000, DIGITS);
    inputs.push(digits.clone());

    let mut with_dot = digits;
    with_dot.insert(1, b'.');
    with_dot.extend_from_slice(b"e-300");
    inputs.push(with_dot);

    // 200 000 bytes of trailing input that must never be looked at
    let mut trailing = b"3.25".to_vec();
    trailing.extend(std::iter::repeat(b'9').take(200_000));
    inputs.push(trailing);

    inputs.push(vec![b'\n'; 300_000]);

    diff_exe("cli large inputs", &inputs);
}

/// Standard input closed / at EOF immediately, and standard input being
/// `/dev/null`: `scanf` reports an input failure and `f` keeps `0.0`.
fn cli_no_stdin() {
    use std::process::{Command, Stdio};
    for (label, mk) in [
        ("null", (|| Stdio::null()) as fn() -> Stdio),
        ("closed-ish", (|| Stdio::null()) as fn() -> Stdio),
    ] {
        let mut outs = Vec::new();
        for exe in [c_exe(), rust_exe()] {
            let out = Command::new(&exe)
                .stdin(mk())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .unwrap_or_else(|e| panic!("spawn {}: {e}", exe.display()));
            outs.push((out.stdout, out.stderr, out.status.code()));
        }
        assert_eq!(
            outs[0], outs[1],
            "stdin={label}: C {:?} vs Rust {:?}",
            String::from_utf8_lossy(&outs[0].0),
            String::from_utf8_lossy(&outs[1].0)
        );
        assert_eq!(
            outs[0].0,
            b"0 0x0p+0 0.0000\n".to_vec(),
            "stdin={label}: unexpected C output"
        );
    }
}

/// A directly pinned end-to-end expectation, so that a harness that silently
/// compared nothing at all could not pass.
fn cli_self_check() {
    let r = run_exe(&c_exe(), b"1.5");
    assert_eq!(
        r.stdout, b"3ff8000000000000 0x1.8p+0 1.5000\n",
        "C reference output changed: {:?}",
        String::from_utf8_lossy(&r.stdout)
    );
    let r = run_exe(&rust_exe(), b"1.5");
    assert_eq!(
        r.stdout, b"3ff8000000000000 0x1.8p+0 1.5000\n",
        "Rust output changed: {:?}",
        String::from_utf8_lossy(&r.stdout)
    );
    let r = run_exe(&c_exe(), b"");
    assert_eq!(r.stdout, b"0 0x0p+0 0.0000\n");
    assert_eq!(r.code, Some(0));
}

fn main() {
    common::run_suite(
        "cli_diff",
        &[
            ("cli_self_check", cli_self_check),
            ("cli_representative_inputs", cli_representative_inputs),
            ("cli_random_decimal", cli_random_decimal),
            ("cli_random_hex", cli_random_hex),
            ("cli_random_garbage", cli_random_garbage),
            ("cli_large_inputs", cli_large_inputs),
            ("cli_no_stdin", cli_no_stdin),
        ],
    );
}
