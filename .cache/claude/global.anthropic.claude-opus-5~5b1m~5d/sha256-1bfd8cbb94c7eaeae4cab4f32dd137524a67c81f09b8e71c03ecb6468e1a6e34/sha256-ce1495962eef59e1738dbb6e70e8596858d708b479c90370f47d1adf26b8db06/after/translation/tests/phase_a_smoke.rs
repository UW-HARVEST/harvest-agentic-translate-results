//! Phase A sanity: both `.so`s load, and every symbol the C `.so` exports is
//! resolvable in the Rust `.so` through `dlsym` (the real symbol-parity gate).
mod common;
use common::*;
use std::ffi::{c_char, c_int, c_uint, c_void};
use std::process::Command;

fn nm_defined(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", path.to_str().unwrap()])
        .output()
        .expect("nm not available");
    assert!(out.status.success(), "nm failed on {path:?}");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            if f.len() >= 3 {
                Some(f[2].to_string())
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn symbol_parity_nm() {
    let mut c = nm_defined(&c_so_path());
    let mut r = nm_defined(&rs_so_path());
    c.sort();
    c.dedup();
    r.sort();
    r.dedup();
    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    println!("C exports {} symbols, Rust exports {}", c.len(), r.len());
    assert!(
        missing.is_empty(),
        "{} symbol(s) exported by the C .so are MISSING from the Rust .so: {:?}",
        missing.len(),
        missing
    );
}

#[test]
fn symbol_parity_dlsym() {
    // Every C symbol must be resolvable through dlsym in the Rust .so too.
    let c = nm_defined(&c_so_path());
    let mut bad = Vec::new();
    for name in &c {
        let ok = unsafe { rlib().get::<*const c_void>(name.as_bytes()) }.is_ok();
        if !ok {
            bad.push(name.clone());
        }
    }
    assert!(bad.is_empty(), "not resolvable via dlsym in Rust .so: {bad:?}");
}

#[test]
fn data_symbols_match() {
    unsafe {
        let (c, r) = duo_value::<c_int>("g_debuglevel");
        eqv("g_debuglevel", c, r);
        let (c, r) = duo_value::<c_uint>("g_ZSTD_threading_useless_symbol");
        eqv("g_ZSTD_threading_useless_symbol", c, r);
    }
}

#[test]
fn smoke_roundtrip() {
    unsafe {
        let (vc, vr) = duo::<FnUint0>("ZSTD_versionNumber");
        eqv("ZSTD_versionNumber", vc(), vr());
        let (sc, sr) = duo::<unsafe extern "C" fn() -> *const c_char>("ZSTD_versionString");
        eqv("ZSTD_versionString", cstr(sc()), cstr(sr()));

        let (bc, br) = duo::<FnSizeT1>("ZSTD_compressBound");
        eqv("ZSTD_compressBound(1000)", bc(1000), br(1000));

        let src = gen_class(4, 5000, 1);
        let cap = bc(src.len());
        let (cc, cr) = duo::<FnCompress>("ZSTD_compress");
        let mut oc = vec![0u8; cap];
        let mut or_ = vec![0u8; cap];
        let nc = cc(
            oc.as_mut_ptr() as *mut c_void,
            oc.len(),
            src.as_ptr() as *const c_void,
            src.len(),
            3,
        );
        let nr = cr(
            or_.as_mut_ptr() as *mut c_void,
            or_.len(),
            src.as_ptr() as *const c_void,
            src.len(),
            3,
        );
        eqv("ZSTD_compress ret", nc, nr);
        eqbuf("ZSTD_compress dst", &oc[..nc], &or_[..nr]);

        let (dc, dr) = duo::<FnDecompress>("ZSTD_decompress");
        let mut pc = vec![0u8; src.len()];
        let mut pr = vec![0u8; src.len()];
        let mc = dc(
            pc.as_mut_ptr() as *mut c_void,
            pc.len(),
            oc.as_ptr() as *const c_void,
            nc,
        );
        let mr = dr(
            pr.as_mut_ptr() as *mut c_void,
            pr.len(),
            or_.as_ptr() as *const c_void,
            nr,
        );
        eqv("ZSTD_decompress ret", mc, mr);
        eqbuf("ZSTD_decompress dst", &pc, &pr);
        assert_eq!(&pc[..], &src[..], "round-trip mismatch");
    }
}
