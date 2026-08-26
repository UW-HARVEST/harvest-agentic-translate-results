// Phase B — exported DATA symbols.
//
// PCRE2 exports 31 data symbols (Unicode database, opcode lengths, property
// name tables, default contexts, default character tables).  A single wrong
// byte in any of them silently changes matching behaviour for some input, so
// they are compared exhaustively, byte for byte, through `dlsym` on both
// libraries.

mod common;
use common::*;
use std::ffi::{c_void, CStr};
use std::process::Command;

/// (symbol, byte length) for every pointer-free exported data symbol.
/// Lengths are the ELF symbol sizes; `data_symbol_sizes_match` re-derives them
/// with `nm` and fails if either library ever drifts.
const PLAIN_TABLES: &[(&str, usize)] = &[
    ("_pcre2_OP_lengths_8", 173),
    ("_pcre2_callout_end_delims_8", 36),
    ("_pcre2_callout_start_delims_8", 36),
    ("_pcre2_default_tables_8", 1088),
    ("_pcre2_hspace_list_8", 80),
    ("_pcre2_posix_class_maps8", 168),
    ("_pcre2_ucd_boolprop_sets_8", 1528),
    ("_pcre2_ucd_caseless_sets_8", 472),
    ("_pcre2_ucd_digit_sets_8", 312),
    ("_pcre2_ucd_nocase_ranges_8", 336),
    ("_pcre2_ucd_nocase_ranges_size_8", 4),
    ("_pcre2_ucd_records_8", 18756),
    ("_pcre2_ucd_script_sets_8", 1904),
    ("_pcre2_ucd_stage1_8", 17408),
    ("_pcre2_ucd_stage2_8", 80384),
    ("_pcre2_ucd_turkish_dotted_i_caseset_8", 4),
    ("_pcre2_ucp_gbtable_8", 60),
    ("_pcre2_ucp_gentype_8", 120),
    ("_pcre2_utf8_table1", 24),
    ("_pcre2_utf8_table1_size", 4),
    ("_pcre2_utf8_table2", 24),
    ("_pcre2_utf8_table3", 24),
    ("_pcre2_utf8_table4", 64),
    ("_pcre2_utt_8", 3108),
    ("_pcre2_utt_names_8", 3834),
    ("_pcre2_utt_size_8", 8),
    ("_pcre2_vspace_list_8", 32),
];

// CONFIGS row: every pointer-free exported data table, full extent.
#[test]
fn tables_byte_identical() {
    let p = pair();
    let mut total = 0usize;
    for &(sym, len) in PLAIN_TABLES {
        let a = p.c.data(sym);
        let b = p.r.data(sym);
        let (sa, sb) = unsafe {
            (
                std::slice::from_raw_parts(a, len),
                std::slice::from_raw_parts(b, len),
            )
        };
        if sa != sb {
            let at = sa.iter().zip(sb).position(|(x, y)| x != y).unwrap();
            let lo = at.saturating_sub(8);
            let hi = (at + 24).min(len);
            panic!(
                "data symbol `{sym}` differs at byte {at} of {len}\n  C    = {:02x?}\n  rust = {:02x?}",
                &sa[lo..hi],
                &sb[lo..hi]
            );
        }
        total += len;
    }
    assert_eq!(PLAIN_TABLES.len(), 27);
    println!("compared {total} bytes across {} tables", PLAIN_TABLES.len());
}

// CONFIGS row: `_pcre2_unicode_version_8` is a `const char *`; compare the
// string it points at, not the (necessarily different) pointer value.
#[test]
fn unicode_version_string_identical() {
    let p = pair();
    unsafe {
        let a = *(p.c.data("_pcre2_unicode_version_8") as *const *const i8);
        let b = *(p.r.data("_pcre2_unicode_version_8") as *const *const i8);
        assert!(!a.is_null() && !b.is_null());
        assert_eq!(CStr::from_ptr(a), CStr::from_ptr(b), "unicode version differs");
        println!("unicode version = {:?}", CStr::from_ptr(a));
    }
}

// ---------------------------------------------------------- default contexts

#[repr(C)]
struct MemCtl {
    malloc: *mut c_void,
    free: *mut c_void,
    memory_data: *mut c_void,
}

#[repr(C)]
struct DefaultCompileContext {
    memctl: MemCtl,
    stack_guard: *mut c_void,
    stack_guard_data: *mut c_void,
    tables: *const u8,
    max_pattern_length: usize,
    max_pattern_compiled_length: usize,
    bsr_convention: u16,
    newline_convention: u16,
    parens_nest_limit: u32,
    extra_options: u32,
    max_varlookbehind: u32,
    optimization_flags: u32,
}

#[repr(C)]
struct DefaultMatchContext {
    memctl: MemCtl,
    callout: *mut c_void,
    callout_data: *mut c_void,
    substitute_callout: *mut c_void,
    substitute_callout_data: *mut c_void,
    substitute_case_callout: *mut c_void,
    substitute_case_callout_data: *mut c_void,
    offset_limit: usize,
    heap_limit: u32,
    match_limit: u32,
    depth_limit: u32,
}

#[repr(C)]
struct DefaultConvertContext {
    memctl: MemCtl,
    glob_separator: u32,
    glob_escape: u32,
}

// CONFIGS row: the three exported default contexts — every non-pointer field.
#[test]
fn default_contexts_identical() {
    let p = pair();
    unsafe {
        assert_eq!(std::mem::size_of::<DefaultCompileContext>(), 88);
        assert_eq!(std::mem::size_of::<DefaultMatchContext>(), 96);
        assert_eq!(std::mem::size_of::<DefaultConvertContext>(), 32);

        let ca = &*(p.c.data("_pcre2_default_compile_context_8") as *const DefaultCompileContext);
        let cb = &*(p.r.data("_pcre2_default_compile_context_8") as *const DefaultCompileContext);
        macro_rules! eqf {
            ($a:expr, $b:expr, $($f:ident),*) => { $(
                assert_eq!($a.$f, $b.$f, "default context field `{}` differs", stringify!($f));
            )* };
        }
        eqf!(
            ca,
            cb,
            max_pattern_length,
            max_pattern_compiled_length,
            bsr_convention,
            newline_convention,
            parens_nest_limit,
            extra_options,
            max_varlookbehind,
            optimization_flags
        );
        // Allocator slots: both must use their own non-null default malloc/free
        // and a NULL user-data pointer.
        for (n, x) in [("C", ca), ("rust", cb)] {
            assert!(!x.memctl.malloc.is_null(), "{n}: default malloc is NULL");
            assert!(!x.memctl.free.is_null(), "{n}: default free is NULL");
            assert!(x.memctl.memory_data.is_null(), "{n}: memory_data not NULL");
            assert!(x.stack_guard.is_null(), "{n}: stack_guard not NULL");
            assert!(x.stack_guard_data.is_null(), "{n}: stack_guard_data not NULL");
        }
        // `tables` must point at that library's own `_pcre2_default_tables_8`.
        assert_eq!(ca.tables, p.c.data("_pcre2_default_tables_8"));
        assert_eq!(cb.tables, p.r.data("_pcre2_default_tables_8"));

        let ma = &*(p.c.data("_pcre2_default_match_context_8") as *const DefaultMatchContext);
        let mb = &*(p.r.data("_pcre2_default_match_context_8") as *const DefaultMatchContext);
        eqf!(ma, mb, offset_limit, heap_limit, match_limit, depth_limit);
        for (n, x) in [("C", ma), ("rust", mb)] {
            assert!(x.callout.is_null(), "{n}: callout not NULL");
            assert!(x.callout_data.is_null(), "{n}: callout_data not NULL");
            assert!(x.substitute_callout.is_null(), "{n}: substitute_callout not NULL");
            assert!(x.substitute_case_callout.is_null(), "{n}: case callout not NULL");
            assert!(!x.memctl.malloc.is_null() && !x.memctl.free.is_null());
        }

        let va = &*(p.c.data("_pcre2_default_convert_context_8") as *const DefaultConvertContext);
        let vb = &*(p.r.data("_pcre2_default_convert_context_8") as *const DefaultConvertContext);
        eqf!(va, vb, glob_separator, glob_escape);
    }
}

// Phase D guard: re-derive every exported data symbol's ELF size with `nm` and
// require the two libraries to agree, so `PLAIN_TABLES` can never silently
// cover a truncated Rust table.
#[test]
fn data_symbol_sizes_match() {
    fn sizes(so: &str) -> Vec<(String, u64)> {
        let out = Command::new("nm")
            .args(["-S", "-D", "--defined-only", so])
            .output()
            .expect("nm not available");
        assert!(out.status.success(), "nm failed on {so}");
        let mut v: Vec<(String, u64)> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| {
                let f: Vec<&str> = l.split_whitespace().collect();
                if f.len() == 4 && (f[2] == "R" || f[2] == "D") {
                    Some((f[3].to_string(), u64::from_str_radix(f[1], 16).ok()?))
                } else {
                    None
                }
            })
            .collect();
        v.sort();
        v
    }
    let root = env!("CARGO_MANIFEST_DIR");
    let c = sizes(&format!("{root}/c_src/build/libpcre2.so"));
    let r = sizes(&format!("{root}/target/release/libpcre2.so"));
    assert_eq!(c.len(), 31, "expected 31 exported data symbols in C");
    assert_eq!(c, r, "exported data symbol names/sizes differ");

    // and every pointer-free one must be in PLAIN_TABLES with the right length
    let ptr_bearing = [
        "_pcre2_default_compile_context_8",
        "_pcre2_default_match_context_8",
        "_pcre2_default_convert_context_8",
        "_pcre2_unicode_version_8",
    ];
    for (sym, len) in &c {
        if ptr_bearing.contains(&sym.as_str()) {
            continue;
        }
        let e = PLAIN_TABLES
            .iter()
            .find(|(s, _)| s == sym)
            .unwrap_or_else(|| panic!("data symbol {sym} not covered by PLAIN_TABLES"));
        assert_eq!(e.1 as u64, *len, "PLAIN_TABLES length wrong for {sym}");
    }
}
