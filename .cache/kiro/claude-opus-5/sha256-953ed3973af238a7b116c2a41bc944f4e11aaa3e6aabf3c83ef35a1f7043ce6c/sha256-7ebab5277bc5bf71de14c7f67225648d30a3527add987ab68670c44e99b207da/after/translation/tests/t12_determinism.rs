// Determinism guard: any script whose *C* output differs between two identical
// runs cannot be used for differential testing. This test exists to catch such
// cases explicitly instead of letting them show up as flaky failures.
mod common;

use common::*;

fn nondeterministic(scripts: &[String]) -> Vec<String> {
    let a = Session::new(Side::C, 0);
    let b = Session::new(Side::C, 0);
    let mut bad = Vec::new();
    for s in scripts {
        // Two fresh-ish evaluations, plus a repeat on the same state.
        let r1 = run_script(&a, s);
        let r2 = run_script(&b, s);
        let r3 = run_script(&a, s);
        if r1 != r2 || r1 != r3 {
            bad.push(format!("{} => {:?} / {:?} / {:?}", s, r1.value, r2.value, r3.value));
        }
    }
    bad
}

#[test]
fn date_scripts_are_deterministic() {
    let mut times: Vec<i64> = vec![
        0,
        1,
        -1,
        1_234_567_890_123,
        8_640_000_000_000_000,
        -8_640_000_000_000_000,
        8_640_000_000_000_001,
        -62_167_219_200_000,
    ];
    let mut x: u64 = 0xDA7E ^ 0x9E3779B97F4A7C15;
    for _ in 0..300 {
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        let v = x.wrapping_mul(0x2545F4914F6CDD1D);
        times.push((v % 17_000_000_000_000) as i64 - 8_500_000_000_000);
    }
    const METHODS: &[&str] = &[
        "getTime", "toString", "toDateString", "toTimeString", "toUTCString", "toISOString",
        "toJSON", "toLocaleString", "toLocaleDateString", "toLocaleTimeString",
        "getTimezoneOffset",
    ];
    let mut scripts = Vec::new();
    for t in &times {
        for m in METHODS {
            scripts.push(format!("try{{String(new Date({}).{}())}}catch(e){{e.name}}", t, m));
        }
        scripts.push(format!("String(new Date({}))", t));
    }
    // extreme years, which stress the sprintf buffers in the C original
    for t in [
        "8.64e15", "-8.64e15", "1e15", "-1e15", "2.5e14", "-2.5e14", "1e14", "-1e14",
    ] {
        for m in METHODS {
            scripts.push(format!("try{{String(new Date({}).{}())}}catch(e){{e.name}}", t, m));
        }
    }
    let bad = nondeterministic(&scripts);
    assert!(
        bad.is_empty(),
        "{} date scripts are not deterministic in the C reference:\n{}",
        bad.len(),
        bad.join("\n")
    );
}

#[test]
fn number_and_string_scripts_are_deterministic() {
    let mut scripts = Vec::new();
    for n in [
        "0", "1", "-1", "1e21", "1e-7", "NaN", "Infinity", "1.7976931348623157e308",
        "5e-324", "123.456",
    ] {
        for d in [0, 1, 2, 5, 10, 20, 100] {
            scripts.push(format!("try{{({}).toFixed({})}}catch(e){{e.name}}", n, d));
            scripts.push(format!("try{{({}).toPrecision({})}}catch(e){{e.name}}", n, d));
            scripts.push(format!("try{{({}).toExponential({})}}catch(e){{e.name}}", n, d));
        }
        for r in 2..=36 {
            scripts.push(format!("try{{({}).toString({})}}catch(e){{e.name}}", n, r));
        }
    }
    for s in ["''", "'abc'", "'\\u00e9'", "'\\u4e2d'"] {
        scripts.push(format!("({}).toUpperCase()", s));
        scripts.push(format!("encodeURIComponent({})", s));
        scripts.push(format!("JSON.stringify({})", s));
    }
    let bad = nondeterministic(&scripts);
    assert!(
        bad.is_empty(),
        "{} number/string scripts are not deterministic in the C reference:\n{}",
        bad.len(),
        bad.join("\n")
    );
}
