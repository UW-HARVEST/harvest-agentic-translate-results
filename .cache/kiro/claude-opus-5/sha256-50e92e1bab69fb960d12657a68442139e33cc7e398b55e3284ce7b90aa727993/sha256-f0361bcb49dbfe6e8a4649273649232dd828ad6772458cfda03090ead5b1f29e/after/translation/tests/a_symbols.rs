//! CONFIGS.md rows 16, 17 — every exported *data* symbol compared byte-for-byte
//! between the C `.so` and the Rust `.so`, plus the default context structs
//! (non-pointer fields) and `pcre2_maketables`.
mod common;
use common::*;
use std::ffi::c_void;

/// (symbol, size) taken mechanically from `nm -D -S --defined-only` on the C .so.
const POD_DATA: &[(&str, usize)] = &[
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
fn pod_data_symbols_are_byte_identical() {
    let (c, r) = pair();
    for &(sym, size) in POD_DATA {
        let cp = c.data(sym);
        let rp = r.data(sym);
        assert!(!cp.is_null() && !rp.is_null(), "{sym}: null address");
        let cs = unsafe { std::slice::from_raw_parts(cp, size) };
        let rs = unsafe { std::slice::from_raw_parts(rp, size) };
        if cs != rs {
            let at = cs.iter().zip(rs).position(|(a, b)| a != b).unwrap();
            panic!(
                "{sym}: first difference at byte {at}: C=0x{:02x} RUST=0x{:02x}\n\
                 C   [{at}..{}] = {:02x?}\nRUST[{at}..{}] = {:02x?}",
                cs[at],
                rs[at],
                (at + 16).min(size),
                &cs[at..(at + 16).min(size)],
                (at + 16).min(size),
                &rs[at..(at + 16).min(size)],
            );
        }
    }
}

#[test]
fn unicode_version_string_matches() {
    let (c, r) = pair();
    // `const char *_pcre2_unicode_version_8` — a pointer; compare the target.
    let cp = unsafe { *(c.data("_pcre2_unicode_version_8") as *const *const u8) };
    let rp = unsafe { *(r.data("_pcre2_unicode_version_8") as *const *const u8) };
    let cs = unsafe { std::ffi::CStr::from_ptr(cp as *const i8) };
    let rs = unsafe { std::ffi::CStr::from_ptr(rp as *const i8) };
    assert_eq!(cs, rs, "unicode version string");
}

#[test]
fn default_compile_context_fields_match() {
    let (c, r) = pair();
    let cp = c.data("_pcre2_default_compile_context_8");
    let rp = r.data("_pcre2_default_compile_context_8");
    // Layout: memctl(24) guard(8) guard_data(8) tables(8) | max_pattern_length(8)
    // max_pattern_compiled_length(8) bsr:u16 newline:u16 parens_nest_limit:u32
    // extra_options:u32 max_varlookbehind:u32 optimization_flags:u32
    let cs = unsafe { std::slice::from_raw_parts(cp, 0x58) };
    let rs = unsafe { std::slice::from_raw_parts(rp, 0x58) };
    assert_eq!(&cs[48..84], &rs[48..84], "compile context scalar fields");
    // tables pointer must point at the respective library's default tables
    let ct = unsafe { *(cp.add(40) as *const *const u8) };
    let rt = unsafe { *(rp.add(40) as *const *const u8) };
    assert_eq!(ct, c.data("_pcre2_default_tables_8"));
    assert_eq!(rt, r.data("_pcre2_default_tables_8"));
    // guard / guard_data must both be NULL
    for off in [24usize, 32] {
        assert!(unsafe { *(cp.add(off) as *const usize) } == 0);
        assert!(unsafe { *(rp.add(off) as *const usize) } == 0);
    }
}

#[test]
fn default_match_context_fields_match() {
    let (c, r) = pair();
    let cp = c.data("_pcre2_default_match_context_8");
    let rp = r.data("_pcre2_default_match_context_8");
    // memctl(24) callout(8) callout_data(8) sub_callout(8) sub_callout_data(8)
    // case_callout(8) case_callout_data(8) => 72; offset_limit(8) heap(4)
    // match(4) depth(4)
    let cs = unsafe { std::slice::from_raw_parts(cp, 0x60) };
    let rs = unsafe { std::slice::from_raw_parts(rp, 0x60) };
    assert_eq!(&cs[72..92], &rs[72..92], "match context scalar fields");
    for off in (24..72).step_by(8) {
        assert_eq!(
            unsafe { *(cp.add(off) as *const usize) },
            0,
            "C match ctx field at {off} should be NULL"
        );
        assert_eq!(
            unsafe { *(rp.add(off) as *const usize) },
            0,
            "Rust match ctx field at {off} should be NULL"
        );
    }
}

#[test]
fn default_convert_context_fields_match() {
    let (c, r) = pair();
    let cp = c.data("_pcre2_default_convert_context_8");
    let rp = r.data("_pcre2_default_convert_context_8");
    let cs = unsafe { std::slice::from_raw_parts(cp, 0x20) };
    let rs = unsafe { std::slice::from_raw_parts(rp, 0x20) };
    assert_eq!(&cs[24..32], &rs[24..32], "convert context scalar fields");
}

#[test]
fn default_memctl_allocators_work_identically() {
    // The memctl in each default context must be a usable malloc/free pair.
    let (c, r) = pair();
    for (a, sym) in [
        (c, "_pcre2_default_compile_context_8"),
        (r, "_pcre2_default_compile_context_8"),
    ] {
        let p = a.data(sym) as *mut MemCtl;
        for size in [0usize, 1, 16, 4096] {
            let m = unsafe { (a.priv_memctl_malloc)(size, p) };
            assert!(!m.is_null(), "{}: memctl_malloc({size}) returned NULL", a.name);
            let mc = unsafe { *p };
            (mc.free.unwrap())(m, mc.memory_data);
        }
    }
}

#[test]
fn maketables_byte_identical() {
    let (c, r) = pair();
    // default gcontext (NULL) and a custom one
    for use_ctx in [false, true] {
        let cg = if use_ctx {
            unsafe { (c.general_context_create)(None, None, std::ptr::null_mut()) }
        } else {
            std::ptr::null_mut()
        };
        let rg = if use_ctx {
            unsafe { (r.general_context_create)(None, None, std::ptr::null_mut()) }
        } else {
            std::ptr::null_mut()
        };
        let ct = unsafe { (c.maketables)(cg) };
        let rt = unsafe { (r.maketables)(rg) };
        assert!(!ct.is_null() && !rt.is_null());
        let cs = unsafe { std::slice::from_raw_parts(ct, 0x440) };
        let rs = unsafe { std::slice::from_raw_parts(rt, 0x440) };
        assert_eq!(cs, rs, "maketables (use_ctx={use_ctx})");
        // ... and identical to the compiled-in default tables
        assert_eq!(cs, unsafe {
            std::slice::from_raw_parts(c.data("_pcre2_default_tables_8"), 0x440)
        });
        unsafe { (c.maketables_free)(cg, ct) };
        unsafe { (r.maketables_free)(rg, rt) };
        if use_ctx {
            unsafe { (c.general_context_free)(cg) };
            unsafe { (r.general_context_free)(rg) };
        }
    }
}

#[test]
fn jit_private_helpers_match() {
    // CONFIGS row 149 (private JIT entry points; no SUPPORT_JIT in this build).
    let (c, r) = pair();
    let ct = unsafe { (c.priv_jit_get_target)() };
    let rt = unsafe { (r.priv_jit_get_target)() };
    assert_eq!(ct.is_null(), rt.is_null(), "jit_get_target nullness");
    if !ct.is_null() {
        assert_eq!(
            unsafe { std::ffi::CStr::from_ptr(ct) },
            unsafe { std::ffi::CStr::from_ptr(rt) }
        );
    }
    assert_eq!(
        unsafe { (c.priv_jit_get_size)(std::ptr::null_mut()) },
        unsafe { (r.priv_jit_get_size)(std::ptr::null_mut()) },
        "jit_get_size(NULL)"
    );
    // free(NULL, ...) must be a harmless no-op in both.
    let mut mc = MemCtl {
        malloc: None,
        free: None,
        memory_data: std::ptr::null_mut(),
    };
    unsafe { (c.priv_jit_free)(std::ptr::null_mut(), &mut mc) };
    unsafe { (r.priv_jit_free)(std::ptr::null_mut(), &mut mc) };
    unsafe { (c.priv_jit_free_rodata)(std::ptr::null_mut(), std::ptr::null_mut()) };
    unsafe { (r.priv_jit_free_rodata)(std::ptr::null_mut(), std::ptr::null_mut()) };
}

#[test]
fn config_all_codes_match() {
    // CONFIGS row 18 / ERRORS rows 27-29, B5.
    let (c, r) = pair();
    for what in [
        0u32, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 100, 1000,
        u32::MAX, u32::MAX - 1,
    ] {
        let mut cb = [0u8; 64];
        let mut rb = [0u8; 64];
        let crc = unsafe { (c.config)(what, cb.as_mut_ptr() as *mut c_void) };
        let rrc = unsafe { (r.config)(what, rb.as_mut_ptr() as *mut c_void) };
        assert_eq!(crc, rrc, "config({what}) rc");
        if crc > 0 {
            assert_eq!(cb, rb, "config({what}) data");
        }
        // NULL probe returns the required length
        let cl = unsafe { (c.config)(what, std::ptr::null_mut()) };
        let rl = unsafe { (r.config)(what, std::ptr::null_mut()) };
        assert_eq!(cl, rl, "config({what}) NULL probe");
    }
}
