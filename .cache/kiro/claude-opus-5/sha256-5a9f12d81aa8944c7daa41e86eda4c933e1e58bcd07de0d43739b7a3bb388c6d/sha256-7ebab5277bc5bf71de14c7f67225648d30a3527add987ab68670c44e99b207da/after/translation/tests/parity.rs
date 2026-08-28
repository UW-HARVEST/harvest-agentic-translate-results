// Parity tests: every call goes through the .so exports of BOTH the C library
// and the Rust library (loaded with libloading).  Nothing calls the Rust crate
// directly, so the `#[no_mangle]` wrappers are exercised as well.
//
// Ordered lowest-level first:
//   1. validate_and_normalize   (pure)
//   2. process_octal_string     (pure, writes a buffer)
//   3. find_and_replace_char    (pure, in-place buffer edit)
//   4. add/multiply/subtract/divide  (mutate file-scope statics)
//   5. findrep                  (top level, uses all of the above)

mod common;

use common::*;

// ---------------------------------------------------------------------------
// 1. int validate_and_normalize(int value)
// ---------------------------------------------------------------------------

fn normalize_inputs() -> Vec<i32> {
    let mut v = vec![
        0,
        1,
        -1,
        2,
        63,
        64,
        65,
        99,
        0o100,
        0o777,
        510,
        511,
        512,
        1000,
        -64,
        -511,
        -512,
        -1000,
        i32::MIN,
        i32::MAX,
        i32::MIN + 1,
        i32::MAX - 1,
        0o123,
        0o150,
        0o10,
    ];
    // A deterministic spread of additional values.
    let mut x: i64 = 12345;
    for _ in 0..300 {
        x = (x * 1103515245 + 12345) % 2147483648;
        v.push(x as i32);
        v.push(-(x as i32));
        v.push((x as i32).wrapping_mul(65599));
    }
    v
}

#[test]
fn validate_and_normalize_parity() {
    let p = Pair::fresh();
    let c = p.c_fn1("validate_and_normalize");
    let r = p.r_fn1("validate_and_normalize");
    for v in normalize_inputs() {
        let cv = unsafe { c(v) };
        let rv = unsafe { r(v) };
        assert_eq!(cv, rv, "validate_and_normalize({v})");
    }
}

// ---------------------------------------------------------------------------
// 2. void process_octal_string(char* dest, int octal_val)
// ---------------------------------------------------------------------------

#[test]
fn process_octal_string_parity() {
    let p = Pair::fresh();
    let c = p.c_fnstr("process_octal_string");
    let r = p.r_fnstr("process_octal_string");

    let mut vals = vec![
        0,
        1,
        -1,
        7,
        8,
        63,
        64,
        0o123,
        0o777,
        511,
        512,
        4095,
        i32::MAX,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX - 1,
        -2,
        -8,
        -0o123,
        1234567890,
        -1234567890,
    ];
    let mut x: i64 = 987654321;
    for _ in 0..200 {
        x = (x * 1103515245 + 12345) % 2147483648;
        vals.push(x as i32);
        vals.push(-(x as i32));
    }

    for v in vals {
        let mut cb = new_buf();
        let mut rb = new_buf();
        unsafe {
            c(cb.as_mut_ptr() as *mut i8, v);
            r(rb.as_mut_ptr() as *mut i8, v);
        }
        assert_eq!(
            cb.as_slice(),
            rb.as_slice(),
            "process_octal_string(dest, {v}):\n  C    = {}\n  Rust = {}",
            describe(&cb),
            describe(&rb)
        );
    }
}

// ---------------------------------------------------------------------------
// 3. void find_and_replace_char(char* str, int search_char)
// ---------------------------------------------------------------------------

#[test]
fn find_and_replace_char_parity() {
    let p = Pair::fresh();
    let c = p.c_fnstr("find_and_replace_char");
    let r = p.r_fnstr("find_and_replace_char");

    let strings: Vec<&[u8]> = vec![
        b"",
        b"a",
        b"O",
        b"Octal: 0123, Decimal: 83",
        b"Function pointer example with static vars",
        b"XXXX",
        b"no match here",
        b"repeated OOO chars",
        b"trailing O",
        b"O leading",
        b"mixed CASE oO",
        b"\x01\x02\x03\x7f",
        b"high \xc3\xa9 bytes \xff\xfe",
        b"tab\there",
        b"digits 0123456789",
    ];

    // Search values: ASCII, non-ASCII, values outside 0..255 (memchr converts
    // the int to `unsigned char`), and negatives.
    let mut needles: Vec<i32> = vec![
        0,
        b'a' as i32,
        b'O' as i32,
        b'o' as i32,
        b'X' as i32,
        b' ' as i32,
        b'\t' as i32,
        b'0' as i32,
        b'\x01' as i32,
        0x7f,
        0xc3,
        0xff,
        0xfe,
        256,
        256 + b'O' as i32,
        512 + b'a' as i32,
        -1,
        -(b'O' as i32),
        65535,
        65536 + b'X' as i32,
        i32::MAX,
        i32::MIN,
    ];
    for n in 0..256 {
        needles.push(n);
    }

    for s in &strings {
        for &needle in &needles {
            let mut cb = new_buf_with(s);
            let mut rb = new_buf_with(s);
            unsafe {
                c(cb.as_mut_ptr() as *mut i8, needle);
                r(rb.as_mut_ptr() as *mut i8, needle);
            }
            assert_eq!(
                cb.as_slice(),
                rb.as_slice(),
                "find_and_replace_char({:?}, {needle}):\n  C    = {}\n  Rust = {}",
                String::from_utf8_lossy(s),
                describe(&cb),
                describe(&rb)
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 4. The four stateful operations.
//
// Each mutates `accumulator` / `multiplier` / `operation_count`, so the two
// libraries are driven through the *same* call sequence on a fresh pair and
// compared after every single call.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
enum Op {
    Add(i32, i32),
    Mul(i32, i32),
    Sub(i32, i32),
    Div(i32, i32),
}

fn run_op_sequence(ops: &[Op]) {
    let p = Pair::fresh();
    let c_add = p.c_fn2("add_to_accumulator");
    let r_add = p.r_fn2("add_to_accumulator");
    let c_mul = p.c_fn2("multiply_with_multiplier");
    let r_mul = p.r_fn2("multiply_with_multiplier");
    let c_sub = p.c_fn2("subtract_from_accumulator");
    let r_sub = p.r_fn2("subtract_from_accumulator");
    let c_div = p.c_fn2("divide_multiplier");
    let r_div = p.r_fn2("divide_multiplier");
    // `findrep` observes all three statics, so calling it lets us cross-check
    // the hidden state that the four operations mutate.
    let c_findrep = p.c_fn4("findrep");
    let r_findrep = p.r_fn4("findrep");

    for (i, op) in ops.iter().enumerate() {
        let (cv, rv) = unsafe {
            match *op {
                Op::Add(a, b) => (c_add(a, b), r_add(a, b)),
                Op::Mul(a, b) => (c_mul(a, b), r_mul(a, b)),
                Op::Sub(a, b) => (c_sub(a, b), r_sub(a, b)),
                Op::Div(a, b) => (c_div(a, b), r_div(a, b)),
            }
        };
        assert_eq!(cv, rv, "step {i}: {op:?}");
    }

    // Probe the resulting hidden state through findrep.
    let (cv, rv) = unsafe { (c_findrep(0, 0, 0, 0), r_findrep(0, 0, 0, 0)) };
    assert_eq!(cv, rv, "findrep(0,0,0,0) after sequence {ops:?}");
}

#[test]
fn stateful_ops_simple_sequences() {
    run_op_sequence(&[Op::Add(1, 2)]);
    run_op_sequence(&[Op::Mul(3, 4)]);
    run_op_sequence(&[Op::Sub(10, 3)]);
    run_op_sequence(&[Op::Div(0, 0)]); // b == 0: no division happens
    run_op_sequence(&[Op::Div(5, 1)]);
    run_op_sequence(&[Op::Div(5, -1)]); // multiplier is 1 here, safe
    run_op_sequence(&[
        Op::Add(0, 0),
        Op::Mul(0, 0),
        Op::Sub(0, 0),
        Op::Div(0, 0),
    ]);
    run_op_sequence(&[
        Op::Add(i32::MAX, 1),
        Op::Sub(i32::MIN, 1),
        Op::Mul(i32::MAX, 2),
        Op::Div(1, 3),
    ]);
    run_op_sequence(&[
        Op::Mul(i32::MIN, 1),
        Op::Div(0, 2),
        Op::Div(0, 3),
        Op::Div(0, -2),
    ]);
    run_op_sequence(&[
        Op::Add(-5, -7),
        Op::Sub(-100, 100),
        Op::Mul(-3, -4),
        Op::Div(0, -2),
        Op::Add(1000, 2000),
    ]);
}

#[test]
fn stateful_ops_random_sequences() {
    let mut x: i64 = 42;
    let mut next = move || {
        x = x
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407)
            & 0x7fff_ffff_ffff_ffff;
        (x >> 16) as i64
    };

    for _ in 0..40 {
        let len = 3 + (next() % 12) as usize;
        let mut ops = Vec::with_capacity(len);
        for _ in 0..len {
            let a = (next() % 4001 - 2000) as i32;
            let b = (next() % 4001 - 2000) as i32;
            match next() % 4 {
                0 => ops.push(Op::Add(a, b)),
                1 => ops.push(Op::Mul(a % 50, b % 50)),
                2 => ops.push(Op::Sub(a, b)),
                // INT_MIN / -1 traps on x86; the C code performs the division
                // unguarded, so keep the divisor away from -1.
                _ => ops.push(Op::Div(a, if b == -1 || b == 0 { 2 } else { b })),
            }
        }
        run_op_sequence(&ops);
    }
}

// ---------------------------------------------------------------------------
// 5. int findrep(int, int, int, int)
// ---------------------------------------------------------------------------

fn findrep_inputs() -> Vec<(i32, i32, i32, i32)> {
    let mut v = vec![
        (0, 0, 0, 0),
        (1, 0, 0, 0),
        (0, 1, 0, 0),
        (0, 0, 1, 0),
        (0, 0, 0, 1),
        (1, 1, 0, 0),
        (1, 1, 1, 0),
        (1, 1, 1, 1),
        (-1, -1, -1, -1),
        (64, 64, 64, 64),
        (63, 63, 63, 63),
        (65, 65, 65, 65),
        (511, 511, 511, 511),
        (512, 512, 512, 512),
        (i32::MAX, i32::MAX, i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN, i32::MIN, i32::MIN),
        (i32::MIN, i32::MAX, i32::MIN, i32::MAX),
        (0o123, 0o150, 0o777, 0o100),
        (100, 200, 300, 400),
        (-100, -200, -300, -400),
        (1, -1, 1, -1),
        (0, 0, 0o777, 0o777),
        (2, 0, 0, 0),
        (0, 0, 2, 3),
        (0o100, 0, 0, 0),
        (0, 0o777, 0, 0),
    ];
    let mut x: i64 = 20240613;
    for _ in 0..150 {
        let mut n = || {
            x = (x * 1103515245 + 12345) % 2147483648;
            (x - 1073741824) as i32
        };
        v.push((n(), n(), n(), n()));
    }
    // Values chosen to straddle the normalization thresholds.
    for a in [-1, 0, 1, 63, 64, 65, 510, 511, 512] {
        for b in [0, 1, 64, 511, 512, -512] {
            v.push((a, b, b, a));
        }
    }
    v
}

/// Each input gets a fresh pair of libraries, so the statics start from their
/// initial values (accumulator = 0, multiplier = 1, operation_count = 0).
#[test]
fn findrep_parity_fresh_state() {
    for (a, b, c_, d) in findrep_inputs() {
        let p = Pair::fresh();
        let cf = p.c_fn4("findrep");
        let rf = p.r_fn4("findrep");
        let cv = unsafe { cf(a, b, c_, d) };
        let rv = unsafe { rf(a, b, c_, d) };
        assert_eq!(cv, rv, "findrep({a}, {b}, {c_}, {d}) with fresh state");
    }
}

/// Repeated calls on the same library instance: exercises the accumulated
/// static state and the `accumulator > 0150` / `multiplier > 0100` branches.
#[test]
fn findrep_parity_accumulated_state() {
    let inputs = findrep_inputs();
    // Several independent runs, each replaying a slice of the inputs in order.
    for chunk in inputs.chunks(17) {
        let p = Pair::fresh();
        let cf = p.c_fn4("findrep");
        let rf = p.r_fn4("findrep");
        for (i, &(a, b, c_, d)) in chunk.iter().enumerate() {
            let cv = unsafe { cf(a, b, c_, d) };
            let rv = unsafe { rf(a, b, c_, d) };
            assert_eq!(cv, rv, "call #{i} findrep({a}, {b}, {c_}, {d}) (accumulated state)");
        }
    }
}

/// Long run of identical calls: drives accumulator/multiplier far from their
/// initial values and through overflow.
#[test]
fn findrep_parity_long_runs() {
    for seed in [(1, 1, 1, 1), (7, 9, 11, 13), (0o777, 0o777, 0o777, 0o777), (-5, 5, -5, 5), (0, 0, 0, 0)] {
        let p = Pair::fresh();
        let cf = p.c_fn4("findrep");
        let rf = p.r_fn4("findrep");
        for i in 0..500 {
            let cv = unsafe { cf(seed.0, seed.1, seed.2, seed.3) };
            let rv = unsafe { rf(seed.0, seed.1, seed.2, seed.3) };
            assert_eq!(cv, rv, "iteration {i} of findrep{seed:?}");
        }
    }
}

/// Interleave findrep with the low-level operations so the shared statics are
/// mutated from both directions.
#[test]
fn mixed_interleaving_parity() {
    let p = Pair::fresh();
    let cf = p.c_fn4("findrep");
    let rf = p.r_fn4("findrep");
    let c_add = p.c_fn2("add_to_accumulator");
    let r_add = p.r_fn2("add_to_accumulator");
    let c_mul = p.c_fn2("multiply_with_multiplier");
    let r_mul = p.r_fn2("multiply_with_multiplier");
    let c_sub = p.c_fn2("subtract_from_accumulator");
    let r_sub = p.r_fn2("subtract_from_accumulator");
    let c_div = p.c_fn2("divide_multiplier");
    let r_div = p.r_fn2("divide_multiplier");

    let mut x: i64 = 555;
    let mut next = move || {
        x = (x * 1103515245 + 12345) % 2147483648;
        x
    };

    for i in 0..400 {
        let a = (next() % 2001 - 1000) as i32;
        let b = (next() % 2001 - 1000) as i32;
        let (cv, rv) = unsafe {
            match i % 5 {
                0 => (c_add(a, b), r_add(a, b)),
                1 => (c_mul(a % 17, b % 17), r_mul(a % 17, b % 17)),
                2 => (c_sub(a, b), r_sub(a, b)),
                3 => {
                    let d = if b == -1 || b == 0 { 3 } else { b };
                    (c_div(a, d), r_div(a, d))
                }
                _ => (cf(a, b, a ^ b, a.wrapping_add(b)), rf(a, b, a ^ b, a.wrapping_add(b))),
            }
        };
        assert_eq!(cv, rv, "interleaved step {i} (a={a}, b={b})");
    }
}

// ---------------------------------------------------------------------------
// Exported-symbol parity: every symbol the C .so exports must also be exported
// by the Rust .so under the same name, and must be resolvable via dlsym.
// ---------------------------------------------------------------------------

#[test]
fn exported_symbols_match() {
    let c = c_so_path();
    let r = rust_so_path();

    let defined = |path: &std::path::Path| -> Vec<String> {
        let out = std::process::Command::new("nm")
            .arg("-D")
            .arg("--defined-only")
            .arg(path)
            .output()
            .expect("run nm");
        assert!(out.status.success(), "nm failed on {}", path.display());
        let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| {
                let f: Vec<&str> = l.split_whitespace().collect();
                if f.len() >= 3 && matches!(f[1], "T" | "t" | "D" | "B") {
                    Some(f[2].to_string())
                } else {
                    None
                }
            })
            // Ignore toolchain/runtime bookkeeping symbols that are not part of
            // the library's own API surface.
            .filter(|s| {
                !s.starts_with("_")
                    && !s.starts_with("rust_")
                    && s != "atexit"
                    && !s.contains("@")
            })
            .collect();
        v.sort();
        v.dedup();
        v
    };

    let c_syms = defined(&c);
    let r_syms = defined(&r);

    assert!(!c_syms.is_empty(), "no symbols found in C library");

    let missing: Vec<&String> = c_syms.iter().filter(|s| !r_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}\nC: {c_syms:?}\nRust: {r_syms:?}"
    );

    // Each one must also be dynamically resolvable from both libraries.
    let p = Pair::fresh();
    for s in &c_syms {
        let name = std::ffi::CString::new(s.as_str()).unwrap();
        unsafe {
            p.c.get::<*const ()>(name.as_bytes_with_nul())
                .unwrap_or_else(|e| panic!("dlsym C {s}: {e}"));
            p.rust
                .get::<*const ()>(name.as_bytes_with_nul())
                .unwrap_or_else(|e| panic!("dlsym Rust {s}: {e}"));
        }
    }
}
