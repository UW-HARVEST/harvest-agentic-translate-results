//! Harness smoke test + symbol parity (Phase A / Phase D).
mod common;
use common::*;

/// `dtoa.c` is compiled WITHOUT `MULTIPLE_THREADS`, so `Balloc`'s `freelist`,
/// `p5s` and `dtoa_result` are unsynchronised mutable statics in BOTH libraries.
/// Any test that formats a real number must therefore run exclusively.
fn lock() -> std::sync::MutexGuard<'static, ()> {
    static L: std::sync::Mutex<()> = std::sync::Mutex::new(());
    match L.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    }
}
use std::ffi::c_void;

#[test]
fn both_libraries_load_and_seed_identically() {
    let d = duo();
    let _g = lock();
    unsafe {
        let cs: u32 = d.c.data("hashtable_seed");
        let rs: u32 = d.rs.data("hashtable_seed");
        eq("hashtable_seed", cs, rs);
    }
}

#[test]
fn version_symbols() {
    let d = duo();
    let _g = lock();
    unsafe {
        eq_bytes(
            "jansson_version_str",
            &cstr_bytes((d.c.jansson_version_str)()),
            &cstr_bytes((d.rs.jansson_version_str)()),
        );
        for (a, b, c) in [
            (2, 15, 0),
            (2, 15, 1),
            (2, 14, 0),
            (3, 0, 0),
            (1, 99, 99),
            (0, 0, 0),
            (-1, -1, -1),
            (i32::MIN, 0, 0),
            (i32::MAX, i32::MAX, i32::MAX),
            (2, 15, i32::MIN),
        ] {
            eq(
                &format!("jansson_version_cmp({},{},{})", a, b, c),
                (d.c.jansson_version_cmp)(a, b, c),
                (d.rs.jansson_version_cmp)(a, b, c),
            );
        }
    }
}

#[test]
fn dtoa_divmax_data_symbol() {
    let d = duo();
    let _g = lock();
    unsafe {
        let cv: i32 = d.c.data("dtoa_divmax");
        let rv: i32 = d.rs.data("dtoa_divmax");
        eq("dtoa_divmax", cv, rv);
        eq("dtoa_divmax initial value", cv, 2);
    }
}

/// Phase D: every symbol exported by the C `.so` must also be exported by the
/// Rust `.so`, with the exact same name.
#[test]
fn symbol_parity_nm() {
    let c = c_so_path();
    let r = rust_so_path();
    let syms = |p: &std::path::Path| -> Vec<String> {
        let out = std::process::Command::new("nm")
            .args(["-D", "--defined-only", p.to_str().unwrap()])
            .output()
            .expect("nm");
        assert!(out.status.success(), "nm failed on {}", p.display());
        let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
            .collect();
        v.sort();
        v.dedup();
        v
    };
    let cs = syms(&c);
    let rs = syms(&r);
    let missing: Vec<&String> = cs.iter().filter(|s| !rs.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "{} symbol(s) exported by the C .so are MISSING from the Rust .so: {:?}",
        missing.len(),
        missing
    );
    let extra: Vec<&String> = rs.iter().filter(|s| !cs.contains(s)).collect();
    assert!(
        extra.is_empty(),
        "{} extra symbol(s) in the Rust .so: {:?}",
        extra.len(),
        extra
    );
    assert!(cs.len() >= 130, "expected >= 130 C symbols, got {}", cs.len());
}

#[test]
fn smoke_roundtrip() {
    let d = duo();
    let _g = lock();
    unsafe {
        let src = cs(r#"{"a":[1,2.5,true,false,null,"x"],"b":{"c":3}}"#);
        let cj = (d.c.json_loads)(src.as_ptr(), 0, std::ptr::null_mut());
        let rj = (d.rs.json_loads)(src.as_ptr(), 0, std::ptr::null_mut());
        assert!(!cj.is_null() && !rj.is_null());
        eq("describe", describe(&d.c, cj), describe(&d.rs, rj));
        let (cd, rd) = dumps_both(d, cj, rj, 0);
        eq_bytes("dumps", cd.as_deref().unwrap(), rd.as_deref().unwrap());
        decref(&d.c, cj);
        decref(&d.rs, rj);
    }
}

/// The `va_list` construction used throughout the error-path tests must work
/// against both libraries.
#[test]
fn valist_construction_works() {
    let d = duo();
    let _g = lock();
    unsafe {
        let fmt = cs("{s:i,s:f,s:s}");
        let k1 = cs("i");
        let k2 = cs("f");
        let k3 = cs("s");
        let v3 = cs("hello");
        for l in d.both() {
            let mut va = VaArgs::new()
                .ptr(k1.as_ptr())
                .int(42)
                .ptr(k2.as_ptr())
                .f64(1.5)
                .ptr(k3.as_ptr())
                .ptr(v3.as_ptr());
            let ap = va.build();
            let j = (l.json_vpack_ex)(std::ptr::null_mut(), 0, fmt.as_ptr(), ap);
            assert!(!j.is_null(), "{}: json_vpack_ex returned NULL", l.which);
            let s = (l.json_dumps)(j, JSON_SORT_KEYS);
            assert!(!s.is_null());
            eq_bytes(
                &format!("{} vpack", l.which),
                br#"{"f": 1.5, "i": 42, "s": "hello"}"#,
                &cstr_bytes(s),
            );
            (l.jsonp_free)(s as *mut c_void);
            decref(l, j);
        }
    }
}
