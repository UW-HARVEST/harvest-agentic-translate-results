// Phase D — symbol parity between the two shared objects, enforced in CI rather
// than by a one-off shell command.
//
// Two independent checks:
//   1. `nm -D --defined-only` on the C .so, every symbol of which must also be
//      exported by the Rust .so (exact name).
//   2. every one of those names must actually be `dlsym`-able out of the Rust
//      .so — a name can appear in `nm` output and still be unusable, and this
//      also proves the `#[unsafe(no_mangle)]` wrappers really are the linker
//      symbols.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

/// The ten symbols `src/lib.c` exports (nine non-static helpers + `charinbuf`).
/// Hard-coded as a floor so the test fails if `nm` is unavailable AND the C .so
/// were somehow to export nothing.
const EXPECTED: [&str; 10] = [
    "apply_operation",
    "charinbuf",
    "create_buffer",
    "decrement_counter",
    "find_char_in_buffer",
    "increment_counter",
    "is_string_empty",
    "multiply_counter",
    "reset_counter",
    "validate_uint16_range",
];

fn defined_dynamic_symbols(so: &std::path::Path) -> Option<BTreeSet<String>> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    Some(
        text.lines()
            .filter_map(|l| l.split_whitespace().nth(2))
            .map(|s| s.to_string())
            .collect(),
    )
}

#[test]
fn phase_d_every_c_symbol_is_exported_by_rust() {
    let _g = gate();
    let (c, r) = apis();

    let (Some(c_syms), Some(r_syms)) = (
        defined_dynamic_symbols(&c.path),
        defined_dynamic_symbols(&r.path),
    ) else {
        eprintln!("nm unavailable — relying on the dlsym test instead");
        return;
    };

    assert!(
        c_syms.len() >= EXPECTED.len(),
        "C .so exports only {} symbols: {:?}",
        c_syms.len(),
        c_syms
    );

    // Rust exports plenty of extra Rust-runtime symbols; only the C surface has
    // to be a subset.
    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {:?}\n\
         C   .so: {}\n\
         Rust.so: {}",
        missing.len(),
        missing,
        c.path.display(),
        r.path.display()
    );

    // And the C surface is exactly the ten symbols we documented in SYMBOLS.md.
    let expected: BTreeSet<String> = EXPECTED.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        c_syms, expected,
        "the C .so's exported surface changed; SYMBOLS.md needs regenerating"
    );
}

#[test]
fn phase_d_every_symbol_is_dlsym_able_from_both() {
    let _g = gate();
    let (c, r) = apis();

    for name in EXPECTED {
        for (label, path) in [("C", &c.path), ("Rust", &r.path)] {
            let lib = unsafe { libloading::Library::new(path) }.expect("dlopen");
            let mut cname = name.as_bytes().to_vec();
            cname.push(0);
            let found = unsafe {
                lib.get::<unsafe extern "C" fn()>(&cname)
            }
            .is_ok();
            assert!(
                found,
                "{label} .so ({}) does not export `{name}`",
                path.display()
            );
        }
    }
}

/// Guards against a stub: a symbol that exists but does nothing. Each of the ten
/// is called through the Rust .so and required to behave like the C one — a
/// `unimplemented!()`/no-op stub would diverge here.
#[test]
fn phase_d_no_symbol_is_a_stub() {
    let _g = gate();
    let (c, r) = apis();
    let s = b"stub probe\0";
    let p = s.as_ptr().cast::<std::ffi::c_char>();

    (c.reset_counter)(0);
    (r.reset_counter)(0);

    // reset / increment / decrement / multiply must all move real state.
    assert_eq!((c.reset_counter)(10), (r.reset_counter)(10));
    assert_eq!((c.increment_counter)(5), (r.increment_counter)(5));
    assert_eq!((c.decrement_counter)(2), (r.decrement_counter)(2));
    assert_eq!((c.multiply_counter)(3), (r.multiply_counter)(3));
    assert_eq!(c.peek_counter(), r.peek_counter());
    assert_eq!(c.peek_counter(), 39, "10+5-2=13, *3=39");

    assert_eq!(
        (c.validate_uint16_range)(65535),
        (r.validate_uint16_range)(65535)
    );
    assert_eq!((c.validate_uint16_range)(65535), 1);

    unsafe {
        assert_eq!((c.is_string_empty)(p), (r.is_string_empty)(p));
        assert_eq!((c.is_string_empty)(p), 0);

        let cb = (c.create_buffer)(p);
        let rb = (r.create_buffer)(p);
        assert!(!cb.is_null() && !rb.is_null());
        assert_eq!(libc_strlen(cb), 10);
        assert_eq!(libc_strlen(rb), 10);
        libc_free(cb);
        libc_free(rb);

        let cf = (c.find_char_in_buffer)(p, 10, b'p' as std::ffi::c_char);
        let rf = (r.find_char_in_buffer)(p, 10, b'p' as std::ffi::c_char);
        assert_eq!(cf.offset_from(p), 5, "'p' of \"probe\" is at index 5");
        assert_eq!(rf.offset_from(p), 5);

        assert_eq!(
            (c.apply_operation)(c.p_reset, 77),
            (r.apply_operation)(r.p_reset, 77)
        );
        assert_eq!(c.peek_counter(), 77);
        assert_eq!(r.peek_counter(), 77);
    }

    // charinbuf: every mode must produce real output.
    for mode in 0..=4 {
        let (cv, cout) = capture(|| (c.charinbuf)(mode, 7, 3, 2));
        let (rv, rout) = capture(|| (r.charinbuf)(mode, 7, 3, 2));
        assert!(
            !cout.is_empty(),
            "C charinbuf(mode={mode}) printed nothing"
        );
        assert!(
            !rout.is_empty(),
            "Rust charinbuf(mode={mode}) printed nothing — stub?"
        );
        assert_same_call(&format!("stub check mode {mode}"), (cv, cout), (rv, rout));
    }
}
