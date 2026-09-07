//! Phase D — harness self-check plus symbol parity, executed as a test so the
//! Phase A/D claims are machine-verified rather than asserted in prose.

mod common;

use common::*;
use std::ffi::c_int;
use std::process::Command;

const C_SYMBOLS: [&str; 10] = [
    "increment_counter",
    "decrement_counter",
    "multiply_counter",
    "reset_counter",
    "is_string_empty",
    "find_char_in_buffer",
    "create_buffer",
    "validate_uint16_range",
    "apply_operation",
    "charinbuf",
];

fn dyn_defined(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(path)
        .output()
        .expect("nm available");
    assert!(out.status.success(), "nm failed on {}", path.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let name = it.next()?;
            let kind = it.next()?;
            (kind == "T").then(|| name.to_string())
        })
        .collect()
}

/// The stdout-capture harness itself must work, otherwise every byte comparison
/// in Phase B/C is vacuous.
#[test]
fn d0_capture_harness_is_not_vacuous() {
    let _g = guard();
    let p = pair();
    let (rc, oc) = capture(|| unsafe { (p.c.charinbuf)(3, 7, 3, 4) });
    assert_eq!(rc, 35, "reset 7, +3 = 10, *4 = 40, -5 = 35");
    let text = String::from_utf8(oc).expect("stdout is UTF-8 here");
    let expected = "Mode 3: Function pointers with static counter\n\
                    Counter reset to: 7\n\
                    Counter after increment by 3: 10\n\
                    Counter after multiply by 4: 40\n\
                    Counter after decrement by 5: 35\n\
                    Final static counter value: 35\n";
    assert_eq!(text, expected, "C mode 3 stdout, captured verbatim");

    let (rr, or) = capture(|| unsafe { (p.rs.charinbuf)(3, 7, 3, 4) });
    assert_eq!(rr, 35);
    assert_eq!(String::from_utf8(or).unwrap(), expected);

    // And mode 0, which uses %u for UINT16_MAX.
    let (r0, o0) = capture(|| unsafe { (p.c.charinbuf)(0, 300, 0, 0) });
    let (r1, o1) = capture(|| unsafe { (p.rs.charinbuf)(0, 300, 0, 0) });
    assert_eq!((r0, r1), (300, 300));
    assert_eq!(
        String::from_utf8_lossy(&o0),
        "Mode 0: UINT16_MAX validation\n\
         Checking if value 300 is within uint16_t range...\n\
         Value 300 is valid (0 <= value <= 65535)\n\
         UINT16_MAX constant value: 65535\n"
    );
    assert_eq!(o0, o1);

    // Mode 2 and mode 4, whose messages embed a heap string / %c / %zu.
    let (r2, o2) = capture(|| unsafe { (p.c.charinbuf)(2, 0, 0, 0) });
    let (r3, o3) = capture(|| unsafe { (p.rs.charinbuf)(2, 0, 0, 0) });
    assert_eq!((r2, r3), (23, 23));
    assert_eq!(
        String::from_utf8_lossy(&o2),
        "Mode 2: Dynamic memory allocation and free\n\
         Buffer allocated: 'Testing malloc and free'\n\
         Buffer length: 23\n\
         Buffer freed successfully\n"
    );
    assert_eq!(o2, o3);

    let (r4, o4) = capture(|| unsafe { (p.c.charinbuf)(4, 0, 0, 0) });
    let (r5, o5) = capture(|| unsafe { (p.rs.charinbuf)(4, 0, 0, 0) });
    assert_eq!((r4, r5), (21, 21));
    assert_eq!(
        String::from_utf8_lossy(&o4),
        "Mode 4: Using memchr to find character\n\
         Searching for 'X' in: 'Search for character X in this buffer'\n\
         Found 'X' at position: 21\n"
    );
    assert_eq!(o4, o5);

    let (r6, o6) = capture(|| unsafe { (p.c.charinbuf)(1, 0, 0, 0) });
    let (r7, o7) = capture(|| unsafe { (p.rs.charinbuf)(1, 0, 0, 0) });
    assert_eq!((r6, r7), (10, 10));
    assert_eq!(
        String::from_utf8_lossy(&o6),
        "Mode 1: String empty check by dereference\n\
         Test string is empty (checked with *string)\n\
         Non-empty string correctly identified\n"
    );
    assert_eq!(o6, o7);
}

#[test]
fn d1_symbol_parity() {
    let p = pair();
    let c = dyn_defined(&p.c.path);
    let rs = dyn_defined(&p.rs.path);

    // The C surface is exactly the ten documented symbols.
    let mut c_sorted = c.clone();
    c_sorted.sort();
    let mut expect: Vec<String> = C_SYMBOLS.iter().map(|s| s.to_string()).collect();
    expect.sort();
    assert_eq!(
        c_sorted, expect,
        "the C .so's exported surface changed; SYMBOLS.md must be regenerated"
    );

    let missing: Vec<&String> = c.iter().filter(|s| !rs.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} C symbol(s): {missing:?}",
        missing.len()
    );
}

#[test]
fn d2_every_symbol_is_callable_through_the_rust_so() {
    let _g = guard();
    let p = pair();
    // `pair()` already resolved all ten symbols via `libloading` (it panics on a
    // missing one), and each is invoked at least once here so an exported but
    // non-functional stub cannot pass.
    let s = cstring(b"z");
    unsafe {
        assert_eq!((p.rs.reset_counter)(4), (p.c.reset_counter)(4));
        assert_eq!((p.rs.increment_counter)(1), (p.c.increment_counter)(1));
        assert_eq!((p.rs.decrement_counter)(2), (p.c.decrement_counter)(2));
        assert_eq!((p.rs.multiply_counter)(3), (p.c.multiply_counter)(3));
        assert_eq!(
            (p.rs.is_string_empty)(s.as_ptr()),
            (p.c.is_string_empty)(s.as_ptr())
        );
        assert_eq!(
            (p.rs.find_char_in_buffer)(s.as_ptr(), 1, b'z' as i8),
            (p.c.find_char_in_buffer)(s.as_ptr(), 1, b'z' as i8)
        );
        let a = (p.rs.create_buffer)(s.as_ptr());
        let b = (p.c.create_buffer)(s.as_ptr());
        assert_eq!(read_cstr(a), read_cstr(b));
        libc_free(a);
        libc_free(b);
        assert_eq!(
            (p.rs.validate_uint16_range)(70000),
            (p.c.validate_uint16_range)(70000)
        );
        let op: CounterFn = p.rs.reset_counter;
        let cop: CounterFn = p.c.reset_counter;
        assert_eq!(
            (p.rs.apply_operation)(Some(op), 9),
            (p.c.apply_operation)(Some(cop), 9)
        );
    }
    let (rc, oc) = capture(|| unsafe { (p.c.charinbuf)(0, 5, 0, 0) });
    let (rr, or) = capture(|| unsafe { (p.rs.charinbuf)(0, 5, 0, 0) });
    assert_eq!((rc, oc), (rr, or));
}

#[test]
fn d3_no_cargo_features_exist() {
    // Phase D requires repeating B and C under every feature combination. There
    // are none: assert that, so the claim cannot silently rot.
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let toml = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    assert!(
        !toml.contains("[features]"),
        "a [features] table appeared; Phases B and C must be re-run per combination"
    );
    let c = std::fs::read_to_string(root.join("../c_src/src/lib.c")).unwrap();
    for token in ["#if", "#ifdef", "#ifndef", "#elif"] {
        assert!(
            !c.contains(token),
            "the C source gained a `{token}` conditional; CONFIGS.md must gain rows"
        );
    }
    let _: c_int = 0;
}
