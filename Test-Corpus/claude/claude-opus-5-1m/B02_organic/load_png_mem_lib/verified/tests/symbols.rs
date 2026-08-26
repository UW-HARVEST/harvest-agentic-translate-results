//! Phase D — symbol parity, plus CONFIGS.md rows 69 & 70.

mod common;

use common::*;
use std::process::Command;

fn nm_defined(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("nm must be available");
    assert!(out.status.success(), "nm failed on {}", path.display());
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
        .collect();
    v.sort();
    v.dedup();
    v
}

/// Every symbol the C `.so` exports must also be exported by the Rust `.so`.
#[test]
fn symbol_parity() {
    let c = nm_defined(&c_so_path());
    let r = nm_defined(&rust_so_path());
    assert_eq!(c.len(), 9, "unexpected C symbol count: {c:?}");
    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}\n\
         C: {c:?}\nRust: {r:?}"
    );
    // and nothing surprising is missing the other way for the 9 public names
    for name in [
        "load_png_mem",
        "cp_inflate",
        "cp_error_reason",
        "cp_fixed_table",
        "cp_permutation_order",
        "cp_len_extra_bits",
        "cp_len_base",
        "cp_dist_extra_bits",
        "cp_dist_base",
    ] {
        assert!(c.contains(&name.to_string()), "C .so lost {name}");
        assert!(r.contains(&name.to_string()), "Rust .so lost {name}");
    }
}

/// CONFIGS.md row 69 — the six writable data tables must be byte identical.
#[test]
fn data_table_contents() {
    let p = pair();
    let tables: [(&str, fn(&Impl) -> *mut u8, usize); 6] = [
        ("cp_fixed_table", |i| i.fixed_table, 288 + 32),
        ("cp_permutation_order", |i| i.permutation_order, 19),
        ("cp_len_extra_bits", |i| i.len_extra_bits, 29 + 2),
        ("cp_len_base", |i| i.len_base, (29 + 2) * 4),
        ("cp_dist_extra_bits", |i| i.dist_extra_bits, 30 + 2),
        ("cp_dist_base", |i| i.dist_base, (30 + 2) * 4),
    ];
    for (name, get, len) in tables {
        let a = unsafe { std::slice::from_raw_parts(get(&p.c), len) };
        let b = unsafe { std::slice::from_raw_parts(get(&p.rust), len) };
        assert_eq!(a, b, "table {name} differs (len {len})");
    }
}

/// CONFIGS.md row 70 — `cp_error_reason` lifecycle.
#[test]
fn error_reason_lifecycle() {
    let p = pair();
    // A successful call must leave cp_error_reason untouched.
    let png = tiny_png();
    for im in [&p.c, &p.rust] {
        im.clear_error();
        assert_eq!(im.error(), None, "{}: clear_error failed", im.name);
        let r = call_load_png(im, &png, png.len() as i32);
        assert!(r.ok, "{}: tiny_png should decode", im.name);
        assert_eq!(
            r.err, None,
            "{}: a successful call must not set cp_error_reason",
            im.name
        );
    }
    // A failing call sets the identical string in both.
    let bad = vec![0u8; 64];
    let a = call_load_png(&p.c, &bad, 64);
    let b = call_load_png(&p.rust, &bad, 64);
    assert_eq!(a.err, b.err);
    assert_eq!(
        a.err.as_deref(),
        Some("incorrect file signature (is this a png file?)")
    );
    // The value persists across a following successful call.
    let a2 = call_load_png_no_clear(&p.c, &png);
    let b2 = call_load_png_no_clear(&p.rust, &png);
    assert!(a2.ok && b2.ok);
    assert_eq!(a2.err, b2.err);
    assert_eq!(
        a2.err.as_deref(),
        Some("incorrect file signature (is this a png file?)"),
        "cp_error_reason must survive a successful call"
    );
}

fn call_load_png_no_clear(im: &Impl, png: &[u8]) -> PngResult {
    let img = unsafe { (im.load_png_mem)(png.as_ptr(), png.len() as i32) };
    let ok = !img.pix.is_null();
    let mut pixels = Vec::new();
    if ok {
        let n = (img.w as i64 * img.h as i64 * 4) as usize;
        pixels = unsafe { std::slice::from_raw_parts(img.pix, n) }.to_vec();
        unsafe { libc::free(img.pix as *mut std::ffi::c_void) };
    }
    PngResult {
        w: img.w,
        h: img.h,
        ok,
        pixels,
        err: im.error(),
    }
}

/// A minimal, definitely-valid 2x2 RGBA PNG built from scratch.
pub fn tiny_png() -> Vec<u8> {
    let (w, h, ct) = (2usize, 2usize, 6u8);
    let bpp = bpp_of(ct);
    let raw = scanlines(w, h, bpp, &[0], &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
    let z = zlib(&deflate_literals_fixed(&raw), &raw);
    build_png(
        &PNG_SIG,
        &[
            Chunk::new(b"IHDR", ihdr(w as u32, h as u32, 8, ct, 0, 0, 0)),
            Chunk::new(b"IDAT", z),
            Chunk::new(b"IEND", vec![]),
        ],
    )
}
