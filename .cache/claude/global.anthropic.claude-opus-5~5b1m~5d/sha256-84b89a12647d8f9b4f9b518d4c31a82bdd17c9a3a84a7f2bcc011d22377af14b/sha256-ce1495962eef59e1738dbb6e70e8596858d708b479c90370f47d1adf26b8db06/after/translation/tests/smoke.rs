//! Harness sanity check + Phase D symbol-parity gate.
mod common;

use common::*;
use std::os::raw::{c_char, c_int};
use std::process::Command;

#[test]
fn harness_loads_both_libraries() {
    let l = libs();
    unsafe {
        let (c, r) = l.sym::<FnVoidToInt>("LZ4_versionNumber");
        assert_eq!(c(), r(), "LZ4_versionNumber differs");
    }
}

#[test]
fn roundtrip_smoke() {
    let l = libs();
    let mut rng = Rng::new(1);
    let src = gen_textlike(&mut rng, 4096);
    unsafe {
        let (cb_c, cb_r) = l.sym::<FnCompressBound>("LZ4_compressBound");
        assert_eq!(cb_c(src.len() as c_int), cb_r(src.len() as c_int));
        let bound = cb_c(src.len() as c_int) as usize;

        let (comp_c, comp_r) = l.sym::<FnCompressDefault>("LZ4_compress_default");
        let mut dc = vec![0u8; bound];
        let mut dr = vec![0u8; bound];
        let rc = comp_c(
            src.as_ptr() as *const c_char,
            dc.as_mut_ptr() as *mut c_char,
            src.len() as c_int,
            bound as c_int,
        );
        let rr = comp_r(
            src.as_ptr() as *const c_char,
            dr.as_mut_ptr() as *mut c_char,
            src.len() as c_int,
            bound as c_int,
        );
        same_int_and_bytes("LZ4_compress_default smoke", rc, rr, &dc, &dr);
        assert!(rc > 0);

        let (dec_c, dec_r) = l.sym::<FnDecompressSafe>("LZ4_decompress_safe");
        let mut oc = vec![0u8; src.len()];
        let mut or = vec![0u8; src.len()];
        let dcr = dec_c(
            dc.as_ptr() as *const c_char,
            oc.as_mut_ptr() as *mut c_char,
            rc,
            src.len() as c_int,
        );
        let drr = dec_r(
            dr.as_ptr() as *const c_char,
            or.as_mut_ptr() as *mut c_char,
            rr,
            src.len() as c_int,
        );
        same_int_and_bytes("LZ4_decompress_safe smoke", dcr, drr, &oc, &or);
        assert_eq!(oc, src);
    }
}

/// Phase D gate: every dynamic symbol the C `.so` exports must also be exported
/// by the Rust `.so`, with the exact same name.
#[test]
fn phase_d_symbol_parity() {
    fn syms(path: &str) -> Vec<String> {
        let out = Command::new("nm")
            .args(["-D", "--defined-only", path])
            .output()
            .expect("run nm");
        assert!(out.status.success(), "nm failed on {path}");
        let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().nth(2).map(str::to_string))
            .collect();
        v.sort();
        v.dedup();
        v
    }

    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/");
    let c = syms(&format!("{root}../c_src/build/liblz4.so"));
    let r = syms(&format!("{root}target/release/liblz4.so"));
    assert!(c.len() > 100, "expected many C symbols, got {}", c.len());

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "{} symbols exported by C .so are MISSING from Rust .so: {:?}",
        missing.len(),
        missing
    );

    let extra: Vec<&String> = r.iter().filter(|s| !c.contains(s)).collect();
    assert!(extra.is_empty(), "Rust .so exports extra symbols: {extra:?}");
}
