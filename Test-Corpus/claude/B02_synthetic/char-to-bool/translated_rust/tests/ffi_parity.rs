// Integration test that compares the Rust translation against the original C
// implementation through their respective shared-library exports.
//
// Both shared libraries expose:
//
//     int process_decisions(char *decision_string, size_t length,
//                           int operation, int param);
//
// We `dlopen` both .so files via libloading and call the symbol with the
// same inputs, then assert byte-equivalent return codes.

use libloading::{Library, Symbol};
use std::os::raw::c_int;
use std::path::PathBuf;

type ProcessDecisionsFn =
    unsafe extern "C" fn(*mut u8, usize, c_int, c_int) -> c_int;

struct Backends {
    _c_lib: Library,
    _rust_lib: Library,
    c_fn: ProcessDecisionsFn,
    rust_fn: ProcessDecisionsFn,
}

impl Backends {
    fn load() -> Self {
        // Locate the C .so built by the harness setup. We expect it at
        // c_src/build_so/libdriver_c.so relative to the crate root.
        let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = crate_root.join("c_src/build_so/libdriver_c.so");
        let rust_path = crate_root.join("target/release/libdriver.so");
        assert!(c_path.exists(), "C library not found at {:?}", c_path);
        assert!(
            rust_path.exists(),
            "Rust library not found at {:?} — did you run `cargo build --release`?",
            rust_path
        );

        unsafe {
            let c_lib = Library::new(&c_path).expect("dlopen C lib");
            let rust_lib = Library::new(&rust_path).expect("dlopen Rust lib");
            let c_sym: Symbol<ProcessDecisionsFn> =
                c_lib.get(b"process_decisions\0").expect("C symbol");
            let rust_sym: Symbol<ProcessDecisionsFn> =
                rust_lib.get(b"process_decisions\0").expect("Rust symbol");
            let c_fn: ProcessDecisionsFn = *c_sym;
            let rust_fn: ProcessDecisionsFn = *rust_sym;
            Backends {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c_fn,
                rust_fn,
            }
        }
    }

    fn call(&self, input: &[u8], length: usize, op: c_int, param: c_int) -> (c_int, c_int) {
        // Each backend gets its own copy of the buffer because operation 3
        // mutates the buffer in place.
        let mut c_buf = input.to_vec();
        let mut rust_buf = input.to_vec();
        let c_ptr = if c_buf.is_empty() {
            std::ptr::null_mut()
        } else {
            c_buf.as_mut_ptr()
        };
        let r_ptr = if rust_buf.is_empty() {
            std::ptr::null_mut()
        } else {
            rust_buf.as_mut_ptr()
        };
        let c_ret = unsafe { (self.c_fn)(c_ptr, length, op, param) };
        let r_ret = unsafe { (self.rust_fn)(r_ptr, length, op, param) };
        (c_ret, r_ret)
    }
}

fn assert_match(b: &Backends, input: &[u8], length: usize, op: c_int, param: c_int) {
    let (c, r) = b.call(input, length, op, param);
    assert_eq!(
        c, r,
        "mismatch: input={:?} length={} op={} param={} -> C={} Rust={}",
        std::str::from_utf8(input).unwrap_or("<non-utf8>"),
        length,
        op,
        param,
        c,
        r
    );
}

#[test]
fn null_or_empty_returns_minus1() {
    let b = Backends::load();
    // Empty buffer with length 0
    let (c, r) = b.call(b"", 0, 0, 0);
    assert_eq!(c, -1);
    assert_eq!(r, -1);

    // Non-empty buffer but length=0 should also return -1.
    assert_match(&b, b"yyy", 0, 0, 0);
    assert_match(&b, b"yyy", 0, 1, 0);
    assert_match(&b, b"yyy", 0, 2, 0);
    assert_match(&b, b"yyy", 0, 3, 0);
}

#[test]
fn invalid_operation() {
    let b = Backends::load();
    for op in [-1, 4, 5, 100, i32::MIN, i32::MAX] {
        assert_match(&b, b"yyy", 3, op, 0);
    }
}

#[test]
fn op0_apply_permissions_short_input() {
    let b = Backends::load();
    // length < 3 => -2
    assert_match(&b, b"y", 1, 0, 0);
    assert_match(&b, b"yn", 2, 0, 0);
}

#[test]
fn op0_apply_permissions_all_combinations() {
    let b = Backends::load();
    let chars = [b'y', b'Y', b'n', b'N', b'x', b'0'];
    for &c1 in &chars {
        for &c2 in &chars {
            for &c3 in &chars {
                let buf = [c1, c2, c3, b'x', b'x'];
                assert_match(&b, &buf, 3, 0, 0);
                assert_match(&b, &buf, 5, 0, 0); // extra chars ignored
            }
        }
    }
}

#[test]
fn op1_evaluate_conditions_short_input() {
    let b = Backends::load();
    assert_match(&b, b"y", 1, 1, 0);
    assert_match(&b, b"yn", 2, 1, 0);
}

#[test]
fn op1_evaluate_conditions_all_logic_ops() {
    let b = Backends::load();
    let chars = [b'y', b'Y', b'n', b'N', b'?'];
    for &c1 in &chars {
        for &c2 in &chars {
            for &c3 in &chars {
                let buf = [c1, c2, c3];
                for logic_op in [-1, 0, 1, 2, 3, 4, 99] {
                    assert_match(&b, &buf, 3, 1, logic_op);
                }
            }
        }
    }
}

#[test]
fn op2_configure_flags_small_lengths() {
    let b = Backends::load();
    let chars = [b'y', b'n'];
    // Exhaust all 1..=6 length boolean strings.
    for len in 1..=6usize {
        for mask in 0u32..(1u32 << len) {
            let mut buf = Vec::with_capacity(len);
            for i in 0..len {
                let bit = (mask >> i) & 1;
                buf.push(chars[bit as usize]);
            }
            assert_match(&b, &buf, len, 2, 0);
        }
    }
}

#[test]
fn op2_configure_flags_alternating_and_runs() {
    let b = Backends::load();
    let inputs: &[&[u8]] = &[
        b"ynynynyn",
        b"nynynyny",
        b"yyyy",
        b"nnnn",
        b"yyynnnyy",
        b"ynyyyyyn",
        b"ynynynynyn",
        b"nyynyynyy",
        b"yynnyynn",
        b"nynnyyny",
    ];
    for &s in inputs {
        assert_match(&b, s, s.len(), 2, 0);
    }
}

#[test]
fn op2_configure_flags_long_input_capped_at_32() {
    let b = Backends::load();
    // length > 32 should be capped internally
    let mut buf = vec![b'y'; 40];
    for i in (0..buf.len()).step_by(2) {
        buf[i] = b'n';
    }
    for len in [32usize, 33, 35, 40] {
        assert_match(&b, &buf[..len], len, 2, 0);
    }
}

#[test]
fn op3_validate_sequence_examples() {
    let b = Backends::load();
    let inputs: &[&[u8]] = &[
        // Single-char: starts with y => returns 1 (transitions == 0, len <= 3)
        b"y",
        b"n",  // -10 (doesn't start with y)
        b"yn", // ok, len <=3
        b"ynyn",
        b"ynynynynynyn",
        b"yyyyy",            // 4 consecutive same => -12
        b"yyyn",             // ok start/end
        b"ynnnnn",           // 4 consecutive => -12
        b"yny",              // ends with y, len>1 => -11
        b"ynynyn",
        b"ynyynyyn",
        b"ynynynynyny",      // ends with y => -11
        b"ynynynynynyn",     // long sequence
        b"ynnyynynyny",      // ends with y
        b"ynyyyn",
        b"yynnyynn",
        b"yynyynyynyynnyy",  // various
        b"y",
    ];
    for &s in inputs {
        assert_match(&b, s, s.len(), 3, 0);
    }
}

#[test]
fn op3_validate_sequence_exhaustive_short() {
    let b = Backends::load();
    let chars = [b'y', b'n', b'Y', b'N', b'?'];
    // Exhaust 1..=4 length combinations of {y,n,Y,N,?}
    for len in 1..=4usize {
        let total = chars.len().pow(len as u32);
        for n in 0..total {
            let mut idx = n;
            let mut buf = Vec::with_capacity(len);
            for _ in 0..len {
                buf.push(chars[idx % chars.len()]);
                idx /= chars.len();
            }
            assert_match(&b, &buf, len, 3, 0);
        }
    }
}

#[test]
fn op3_validate_sequence_medium_and_long() {
    let b = Backends::load();
    // Medium length 4..=10 random patterns plus some structured ones.
    let cases: &[&[u8]] = &[
        b"ynyn",
        b"ynyny",
        b"ynynyn",
        b"ynynyny",       // ends with y => -11
        b"ynynynyn",
        b"ynynynyny",     // ends with y => -11
        b"ynynynynyn",
        b"yynnyynnyy",    // ends with y => -11
        b"yynnyynnyn",
        b"ynyyynyynyn",   // length 11
        b"ynynynynynynyn", // length 14
        b"ynynyynnyynyynnyynnyynnyy", // longer
        b"ynyyynyyynyn",
        b"yyynnnyy",      // 3 consecutive ok
        b"yyyynn",        // 4 consecutive => -12
    ];
    for &c in cases {
        assert_match(&b, c, c.len(), 3, 0);
    }
}

#[test]
fn op3_unusual_characters() {
    let b = Backends::load();
    // Bytes that are not y/n/Y/N => parse_bool returns false => bool=0
    let cases: &[&[u8]] = &[
        b"yxxxxxn",
        b"y\0\0\0n",
        b"y???n",
        b"YnYnYn",
        b"Y",
        b"YYYn",
    ];
    for &c in cases {
        assert_match(&b, c, c.len(), 3, 0);
    }
}
