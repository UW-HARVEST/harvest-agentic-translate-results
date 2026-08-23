mod common;
use common::*;

#[test]
fn t_symbols_present() {
    let p = libs();
    // every symbol the C .so exports must resolve in the Rust .so
    let out = std::process::Command::new("nm")
        .args(["-D", "--defined-only", c_so_path().to_str().unwrap()])
        .output()
        .expect("nm");
    let text = String::from_utf8_lossy(&out.stdout);
    let mut missing = vec![];
    let mut n = 0;
    for line in text.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() < 3 {
            continue;
        }
        let name = f[2];
        if name.starts_with('_') {
            continue;
        }
        n += 1;
        if !p.rs.has(name) {
            missing.push(name.to_string());
        }
    }
    assert!(n > 200, "expected >200 C symbols, saw {n}");
    assert!(missing.is_empty(), "missing from Rust .so: {missing:?}");
}

#[test]
fn t_hello() {
    diff_dostring(0, "print('hello')");
}

#[test]
fn t_basic_arith() {
    for src in [
        "1+1",
        "print(1+1)",
        "print(0.1+0.2)",
        "print(1/3)",
        "print(1e300*1e300)",
        "print(-0)",
        "print(1/-0)",
        "print((0/0))",
        "print([1,2,3].join('-'))",
        "print(JSON.stringify({a:1,b:[1,2,{c:3}]}))",
        "print('abc'.toUpperCase())",
        "print(/a(b)c/.exec('xxabcyy'))",
        "print(Math.sqrt(2), Math.PI, Math.E)",
        "print(typeof undefined, typeof null, typeof 1, typeof 'x', typeof {}, typeof [])",
    ] {
        diff_dostring(0, src);
        diff_eval(0, src);
    }
}

#[test]
fn t_number_formatting() {
    let mut rng = Rng::new(0x1234_5678);
    for _ in 0..400 {
        let x = rng.f64_sane();
        let src = format!("print(({}))", fmt_lit(x));
        diff_dostring(0, &src);
    }
}

/// Render an f64 as a JS numeric literal that round-trips exactly.
pub fn fmt_lit(x: f64) -> String {
    if x.is_nan() {
        return "0/0".to_string();
    }
    if x.is_infinite() {
        return if x > 0.0 {
            "Infinity".to_string()
        } else {
            "-Infinity".to_string()
        };
    }
    // hex-free exact decimal via {:?} then wrap negatives
    let s = format!("{x:?}");
    if s.starts_with('-') {
        format!("({s})")
    } else {
        s
    }
}
