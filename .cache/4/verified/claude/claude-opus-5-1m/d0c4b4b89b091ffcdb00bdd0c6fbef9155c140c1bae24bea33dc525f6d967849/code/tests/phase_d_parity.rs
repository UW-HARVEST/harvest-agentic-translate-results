//! Phase D — symbol parity between the C `.so` and the Rust `.so`, plus the
//! struct-layout parity that the whole FFI surface depends on.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

/// `nm -D --defined-only <so>` -> the set of exported symbol names.
fn exported_symbols(so: &std::path::Path) -> Option<BTreeSet<String>> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        // "<addr> <type> <name>" or "         <type> <name>"
        let mut it = line.split_whitespace().rev();
        let name = it.next();
        let kind = it.next();
        if let (Some(name), Some(kind)) = (name, kind) {
            if kind.len() == 1 && kind.chars().next().unwrap().is_ascii_uppercase() {
                set.insert(name.to_string());
            }
        }
    }
    Some(set)
}

/// The C `.so` must export exactly the 11 names recorded in SYMBOLS.md, and the
/// Rust `.so` must export every one of them under the exact same name.
#[test]
fn phase_d_symbol_parity() {
    let c_so = c_so_path();
    let r_so = rust_so_path();

    let (Some(c_syms), Some(r_syms)) = (exported_symbols(&c_so), exported_symbols(&r_so)) else {
        eprintln!("`nm` unavailable — falling back to the dlsym parity check only");
        phase_d_symbol_parity_via_dlsym();
        return;
    };

    // The C library's surface must be exactly what SYMBOLS.md claims.
    let expected: BTreeSet<String> = C_SYMBOLS.iter().map(|s| s.to_string()).collect();
    let c_unlisted: Vec<&String> = c_syms
        .iter()
        .filter(|s| !expected.contains(*s) && !is_toolchain_symbol(s))
        .collect();
    assert!(
        c_unlisted.is_empty(),
        "the C .so exports symbols that SYMBOLS.md does not list: {c_unlisted:?}"
    );
    for want in &expected {
        assert!(
            c_syms.contains(want),
            "SYMBOLS.md lists `{want}` but the C .so does not export it"
        );
    }

    // Every C symbol must be present in the Rust .so, byte-for-byte the same name.
    let missing: Vec<&String> = c_syms
        .iter()
        .filter(|s| !is_toolchain_symbol(s) && !r_syms.contains(*s))
        .collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         (Phase A rule: add the #[no_mangle] wrapper, or translate the missing C source.)"
    );
    println!(
        "symbol diff empty: all {} C symbols are exported by the Rust .so",
        expected.len()
    );
}

fn is_toolchain_symbol(s: &str) -> bool {
    s.starts_with("_ITM_")
        || s.starts_with("__cxa_")
        || s.starts_with("__gmon_")
        || s.starts_with("_init")
        || s.starts_with("_fini")
        || s.starts_with("__bss_start")
        || s == "_edata"
        || s == "_end"
}

/// Belt-and-braces: resolve all 11 names through `dlsym` on the *Rust* library.
/// This is what actually proves the `#[no_mangle]` export wrappers exist and are
/// callable, independent of any `nm` availability.
#[test]
fn phase_d_symbol_parity_via_dlsym() {
    let r_so = rust_so_path();
    let lib = unsafe { libloading::Library::new(&r_so) }.expect("dlopen rust .so");
    for name in C_SYMBOLS {
        let mut owned = name.to_string();
        owned.push('\0');
        let sym: Result<libloading::Symbol<*const ()>, _> =
            unsafe { lib.get(owned.as_bytes()) };
        assert!(
            sym.is_ok(),
            "the Rust .so does not export `{name}` (dlsym failed)"
        );
    }
    // And they must all be callable — `both()` resolves every one on both sides.
    let (c, r) = both();
    assert_eq!(c.name, "C");
    assert_eq!(r.name, "Rust");
}

/// The Rust `.so` must not have unresolved non-libc dependencies.
#[test]
fn phase_d_no_missing_dynamic_deps() {
    let r_so = rust_so_path();
    let Ok(out) = Command::new("ldd").args(["-r", r_so.to_str().unwrap()]).output() else {
        eprintln!("`ldd` unavailable — skipping");
        return;
    };
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let bad: Vec<&str> = text
        .lines()
        .filter(|l| l.contains("undefined symbol") || l.contains("not found"))
        .collect();
    assert!(
        bad.is_empty(),
        "the Rust .so has unresolved dynamic symbols:\n{}",
        bad.join("\n")
    );
}

// ---------------------------------------------------------------------------
// Struct layout parity
// ---------------------------------------------------------------------------

const OFF_VALUE: usize = 0;
const OFF_SCALED: usize = 8;
const OFF_RANK: usize = 16;
const STRIDE: usize = SIZE_OF_RESULT;

fn rd_i32(b: &[u8], off: usize) -> i32 {
    i32::from_ne_bytes(b[off..off + 4].try_into().unwrap())
}
fn rd_f64(b: &[u8], off: usize) -> f64 {
    f64::from_bits(u64::from_ne_bytes(b[off..off + 8].try_into().unwrap()))
}

/// Drive `init_result_array` on a raw byte buffer through both libraries and
/// check that the bytes land at the C-mandated offsets (`Result` = 24 bytes with
/// `value` @0, `scaled` @8, `rank` @16; `ResultArray::count` @240).
#[test]
fn phase_d_layout_parity() {
    let (c, r) = both();
    let vals: Vec<i32> = (0..10).map(|i| 100 + i).collect();

    let run = |api: &Api| -> Box<PaddedArray> {
        let mut p = PaddedArray::new_filled(0x00);
        unsafe { (api.init_result_array)(p.as_arr_ptr(), vals.as_ptr(), 10) };
        p
    };
    let cp = run(c);
    let rp = run(r);

    for (tag, p) in [("C", &cp), ("Rust", &rp)] {
        for i in 0..10usize {
            let base = i * STRIDE;
            assert_eq!(
                rd_i32(&p.bytes, base + OFF_VALUE),
                100 + i as i32,
                "{tag}: Result.value must sit at offset {} of element {i}",
                OFF_VALUE
            );
            assert_eq!(
                rd_f64(&p.bytes, base + OFF_SCALED),
                (100 + i as i32) as f64 * 1.5,
                "{tag}: Result.scaled must sit at offset {OFF_SCALED} of element {i}"
            );
            assert_eq!(
                rd_i32(&p.bytes, base + OFF_RANK),
                i as i32,
                "{tag}: Result.rank must sit at offset {OFF_RANK} of element {i}"
            );
        }
        assert_eq!(
            rd_i32(&p.bytes, OFFSET_OF_COUNT),
            10,
            "{tag}: ResultArray.count must sit at offset {OFFSET_OF_COUNT}"
        );
        // Nothing beyond sizeof(ResultArray) may be touched.
        assert!(
            p.bytes[SIZE_OF_RESULT_ARRAY..].iter().all(|&b| b == 0),
            "{tag}: wrote past sizeof(ResultArray) = {SIZE_OF_RESULT_ARRAY}"
        );
    }

    // And both libraries must agree on every non-padding byte.
    for i in 0..10usize {
        let base = i * STRIDE;
        assert_eq!(
            &cp.bytes[base..base + 20],
            &rp.bytes[base..base + 20],
            "element {i}: C and Rust disagree on the non-padding bytes"
        );
    }
    assert_eq!(
        &cp.bytes[OFFSET_OF_COUNT..OFFSET_OF_COUNT + 4],
        &rp.bytes[OFFSET_OF_COUNT..OFFSET_OF_COUNT + 4],
        "count field bytes differ"
    );
}

/// The Rust type definitions used by the tests must match the sizes the C ABI
/// uses, otherwise every other test in the suite would be reading garbage.
#[test]
fn phase_d_rust_side_type_sizes() {
    assert_eq!(std::mem::size_of::<CResult>(), 24);
    assert_eq!(std::mem::align_of::<CResult>(), 8);
    assert_eq!(std::mem::size_of::<CResultArray>(), SIZE_OF_RESULT_ARRAY);
    assert_eq!(std::mem::align_of::<CResultArray>(), 8);
    assert_eq!(std::mem::offset_of!(CResultArray, count), OFFSET_OF_COUNT);
    assert_eq!(std::mem::offset_of!(CResult, value), OFF_VALUE);
    assert_eq!(std::mem::offset_of!(CResult, scaled), OFF_SCALED);
    assert_eq!(std::mem::offset_of!(CResult, rank), OFF_RANK);
    // A function pointer and Option<fn> must be ABI-identical (the
    // `operation_func` parameter of process_with_foreach).
    assert_eq!(
        std::mem::size_of::<OpFnOpt>(),
        std::mem::size_of::<OpFn>(),
        "Option<fn> must be a bare, nullable function pointer"
    );
}

/// The four exported operations must be reachable as function *pointers* that
/// the other library can call — i.e. the C library can invoke Rust's
/// `add_operation` and vice versa, through `process_with_foreach`.
#[test]
fn phase_d_cross_library_callbacks() {
    let (c, r) = both();
    let mut rng = Rng::new(0xD_0000);
    let pairs: [(&str, OpFn, OpFn); 8] = [
        ("C.add into Rust.foreach", c.add_operation, c.add_operation),
        ("Rust.add into C.foreach", r.add_operation, r.add_operation),
        ("C.mul", c.multiply_operation, c.multiply_operation),
        ("Rust.mul", r.multiply_operation, r.multiply_operation),
        ("C.sub", c.subtract_operation, c.subtract_operation),
        ("Rust.sub", r.subtract_operation, r.subtract_operation),
        ("C.mod", c.modulo_operation, c.modulo_operation),
        ("Rust.mod", r.modulo_operation, r.modulo_operation),
    ];
    // Using the *same* callback on both sides isolates the loop/write-back logic
    // from the operation itself.
    for (tag, cb, _) in pairs {
        for _ in 0..256 {
            let n = rng.range_i32(0, 10) as usize;
            let vals: Vec<i32> = (0..n).map(|_| rng.interesting_i32()).collect();
            let mut ca = CResultArray::from_values(&vals);
            let mut ra = CResultArray::from_values(&vals);
            let cv = unsafe { (c.process_with_foreach)(&mut ca, Some(cb)) };
            let rv = unsafe { (r.process_with_foreach)(&mut ra, Some(cb)) };
            eq_i32("D cross-lib callback", (tag, n), cv, rv);
            eq_arrays("D cross-lib callback", (tag, n), &ca, &ra);
        }
    }
}
