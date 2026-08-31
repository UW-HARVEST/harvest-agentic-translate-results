//! Lowest level: every exported data symbol must have byte-identical contents.
mod common;

use common::*;

/// (symbol, size in bytes) — sizes taken from `nm -S` on the C .so and verified
/// to be identical in the Rust .so.
const PLAIN_TABLES: &[(&str, usize)] = &[
    ("_pcre2_OP_lengths_8", 0xad),
    ("_pcre2_callout_end_delims_8", 0x24),
    ("_pcre2_callout_start_delims_8", 0x24),
    ("_pcre2_default_tables_8", 0x440),
    ("_pcre2_hspace_list_8", 0x50),
    ("_pcre2_posix_class_maps8", 0xa8),
    ("_pcre2_ucd_boolprop_sets_8", 0x5f8),
    ("_pcre2_ucd_caseless_sets_8", 0x1d8),
    ("_pcre2_ucd_digit_sets_8", 0x138),
    ("_pcre2_ucd_nocase_ranges_8", 0x150),
    ("_pcre2_ucd_nocase_ranges_size_8", 0x4),
    ("_pcre2_ucd_records_8", 0x4944),
    ("_pcre2_ucd_script_sets_8", 0x770),
    ("_pcre2_ucd_stage1_8", 0x4400),
    ("_pcre2_ucd_stage2_8", 0x13a00),
    ("_pcre2_ucd_turkish_dotted_i_caseset_8", 0x4),
    ("_pcre2_ucp_gbtable_8", 0x3c),
    ("_pcre2_ucp_gentype_8", 0x78),
    ("_pcre2_utf8_table1", 0x18),
    ("_pcre2_utf8_table1_size", 0x4),
    ("_pcre2_utf8_table2", 0x18),
    ("_pcre2_utf8_table3", 0x18),
    ("_pcre2_utf8_table4", 0x40),
    ("_pcre2_utt_8", 0xc24),
    ("_pcre2_utt_names_8", 0xefa),
    ("_pcre2_utt_size_8", 0x8),
    ("_pcre2_vspace_list_8", 0x20),
];

#[test]
fn plain_data_tables_match() {
    for &(name, size) in PLAIN_TABLES {
        let (cp, rp) = both_data(name);
        unsafe {
            assert_bytes_eq(name, slice_at(cp, size), slice_at(rp, size));
        }
    }
}

#[test]
fn unicode_version_string_matches() {
    // `const char *PRIV(unicode_version)` — the pointers differ, the strings must not.
    let (cp, rp) = both_data("_pcre2_unicode_version_8");
    unsafe {
        let c = std::ffi::CStr::from_ptr(*(cp as *const *const std::ffi::c_char));
        let r = std::ffi::CStr::from_ptr(*(rp as *const *const std::ffi::c_char));
        assert_eq!(c, r, "unicode_version");
        assert!(!c.to_bytes().is_empty());
    }
}

/// Compare a struct's bytes while skipping ranges that hold addresses (which
/// legitimately differ between the two shared objects).
fn cmp_skipping(name: &str, size: usize, skip: &[(usize, usize)]) {
    let (cp, rp) = both_data(name);
    unsafe {
        let c = slice_at(cp, size).to_vec();
        let r = slice_at(rp, size).to_vec();
        let mut cm = c.clone();
        let mut rm = r.clone();
        for &(off, len) in skip {
            for i in off..off + len {
                cm[i] = 0;
                rm[i] = 0;
            }
        }
        assert_bytes_eq(name, &cm, &rm);
    }
}

#[test]
fn default_compile_context_matches() {
    // memctl.malloc/free (0..16) and tables (40..48) are addresses.
    cmp_skipping("_pcre2_default_compile_context_8", 0x58, &[(0, 16), (40, 8)]);
    // memory_data and stack guard fields must be NULL in both.
    let (cp, rp) = both_data("_pcre2_default_compile_context_8");
    unsafe {
        for base in [cp, rp] {
            for off in [16usize, 24, 32] {
                let v = std::ptr::read_unaligned(base.add(off) as *const usize);
                assert_eq!(v, 0, "expected NULL at offset {off}");
            }
        }
        // `tables` must point at that library's own default_tables.
        let ct = std::ptr::read_unaligned(cp.add(40) as *const *const u8);
        let rt = std::ptr::read_unaligned(rp.add(40) as *const *const u8);
        let (cdt, rdt) = both_data("_pcre2_default_tables_8");
        assert_eq!(ct, cdt, "C default_compile_context.tables");
        assert_eq!(rt, rdt, "Rust default_compile_context.tables");
    }
}

#[test]
fn default_match_context_matches() {
    cmp_skipping("_pcre2_default_match_context_8", 0x60, &[(0, 16)]);
}

#[test]
fn default_convert_context_matches() {
    cmp_skipping("_pcre2_default_convert_context_8", 0x20, &[(0, 16)]);
}
