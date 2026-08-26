//! Phase A / Phase D — exported-symbol parity between the C `.so` and the Rust
//! `.so`, plus a mechanical re-verification of every static data table.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

fn dyn_syms(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", path.to_str().unwrap()])
        .output()
        .expect("running `nm` failed");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let _addr = it.next()?;
            let kind = it.next()?;
            let name = it.next()?;
            // Only globally-visible code/data, and ignore the toolchain-injected
            // symbols that are not part of the library's own API surface.
            if !matches!(kind, "T" | "D" | "B" | "R" | "W" | "V") {
                return None;
            }
            if name.starts_with("_") || name.starts_with("rust_") {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}

/// Every symbol the C `.so` exports must also be exported by the Rust `.so`.
#[test]
fn symbol_diff_is_empty() {
    let c = dyn_syms(&common::c_so_path());
    let r = dyn_syms(&common::rust_so_path());

    assert!(!c.is_empty(), "no symbols found in the C .so");

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C   ({}): {c:?}\n\
         Rust({}): {r:?}",
        c.len(),
        r.len()
    );

    // Informational: report the exact set we verified.
    println!("verified {} exported C symbols: {:?}", c.len(), c);

    // Extra symbols on the Rust side would mean the translation leaks internals
    // that are `static` in C.
    let extra: Vec<&String> = r.difference(&c).collect();
    assert!(
        extra.is_empty(),
        "Rust .so exports symbols the C .so does not (C `static` items must stay private): {extra:?}"
    );
}

/// Every exported symbol must actually be loadable & callable through both
/// `.so`s with the C signature (this is what the rest of the suite relies on).
#[test]
fn all_symbols_loadable_from_both() {
    let _ = common::both();
}

/// Re-derive the four `static` tables straight from `c_src/src/lib.c` and check
/// them against `src/tables.rs`, element by element.
#[test]
fn static_tables_match_c_source() {
    fn hex_nums(s: &str) -> Vec<u32> {
        let bytes = s.as_bytes();
        let mut out = Vec::new();
        let mut i = 0;
        while i + 1 < bytes.len() {
            if bytes[i] == b'0' && (bytes[i + 1] == b'x' || bytes[i + 1] == b'X') {
                let start = i + 2;
                let mut j = start;
                while j < bytes.len() && (bytes[j] as char).is_ascii_hexdigit() {
                    j += 1;
                }
                if j > start {
                    out.push(u32::from_str_radix(&s[start..j], 16).unwrap());
                }
                i = j;
            } else {
                i += 1;
            }
        }
        out
    }

    /// Extract the initialiser of `decl` up to (but excluding) `next_decl`.
    fn section<'a>(src: &'a str, decl: &str, next_decl: Option<&str>) -> &'a str {
        let start = src
            .find(decl)
            .unwrap_or_else(|| panic!("declaration `{decl}` not found"));
        let end = match next_decl {
            Some(n) => src[start..]
                .find(n)
                .unwrap_or_else(|| panic!("declaration `{n}` not found after `{decl}`"))
                + start,
            None => src.len(),
        };
        &src[start..end]
    }

    let c_src = std::fs::read_to_string(common::manifest_dir().join("c_src/src/lib.c")).unwrap();
    let rs_src = std::fs::read_to_string(common::manifest_dir().join("src/tables.rs")).unwrap();

    let cases: [(&str, &str, Option<&str>, &str, Option<&str>, usize); 4] = [
        (
            "tflac_crc16_tables",
            "tflac_crc16_tables[8][256]",
            Some("tflac_u32 f7("),
            "TFLAC_CRC16_TABLES",
            Some("M_MANTISSA"),
            8 * 256,
        ),
        (
            "m__mantissa",
            "m__mantissa[2048]",
            Some("m__offset[64]"),
            "M_MANTISSA",
            Some("M_OFFSET"),
            2048,
        ),
        (
            "m__offset",
            "m__offset[64]",
            Some("m__exponent[64]"),
            "M_OFFSET",
            Some("M_EXPONENT"),
            64,
        ),
        (
            "m__exponent",
            "m__exponent[64]",
            Some("float f10("),
            "M_EXPONENT",
            None,
            64,
        ),
    ];

    for (label, c_decl, c_next, rs_decl, rs_next, expected_len) in cases {
        let cv = hex_nums(section(&c_src, c_decl, c_next));
        let rv = hex_nums(section(&rs_src, rs_decl, rs_next));
        assert_eq!(cv.len(), expected_len, "{label}: unexpected C element count");
        assert_eq!(
            rv.len(),
            expected_len,
            "{label}: Rust table has {} elements, expected {expected_len}",
            rv.len()
        );
        for (i, (a, b)) in cv.iter().zip(rv.iter()).enumerate() {
            assert_eq!(a, b, "{label}[{i}]: C = 0x{a:08x}, Rust = 0x{b:08x}");
        }
        println!("{label}: {expected_len} values identical");
    }
}
