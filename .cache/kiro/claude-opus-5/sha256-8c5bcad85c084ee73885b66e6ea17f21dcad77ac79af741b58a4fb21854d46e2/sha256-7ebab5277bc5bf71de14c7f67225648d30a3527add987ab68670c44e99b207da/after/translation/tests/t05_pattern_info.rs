//! `pcre2_pattern_info.c`: `pcre2_pattern_info` and `pcre2_callout_enumerate`,
//! plus `pcre2_substring_nametable_scan` / `pcre2_substring_number_from_name`.
mod common;

use common::*;
use std::ffi::c_void;
use std::sync::Mutex;

type PatternInfoFn2 = unsafe extern "C" fn(*const c_void, u32, *mut c_void) -> i32;

/// `PCRE2_INFO_NAMETABLE` yields a pointer; everything else fits in 8 bytes.
const NAMETABLE: u32 = 19;
/// `PCRE2_INFO_FIRSTBITMAP` yields a pointer to 32 bytes (or NULL).
const FIRSTBITMAP: u32 = 7;

#[test]
fn pattern_info_matches() {
    let (c, r) = both::<PatternInfoFn2>("pcre2_pattern_info_8");
    for p in patterns() {
        for &opts in &[
            0u32,
            PCRE2_UTF,
            PCRE2_UCP,
            PCRE2_CASELESS,
            PCRE2_MULTILINE,
            PCRE2_DUPNAMES,
            PCRE2_AUTO_CALLOUT,
            PCRE2_NO_START_OPTIMIZE,
            PCRE2_UTF | PCRE2_UCP | PCRE2_CASELESS,
        ] {
            let Some(pair) = compile_both(p, opts) else {
                continue;
            };
            let show = String::from_utf8_lossy(p).to_string();
            unsafe {
                // NULL `where` returns the required size.
                for what in 0u32..=30 {
                    assert_eq!(
                        c(pair.c, what, std::ptr::null_mut()),
                        r(pair.r, what, std::ptr::null_mut()),
                        "pattern_info({what}) size for {show:?} opts={opts:#x}"
                    );
                }
                for what in 0u32..=30 {
                    if what == NAMETABLE || what == FIRSTBITMAP {
                        continue;
                    }
                    let mut bc = [0xAAu8; 32];
                    let mut br = [0xAAu8; 32];
                    let rc = c(pair.c, what, bc.as_mut_ptr() as *mut c_void);
                    let rr = r(pair.r, what, br.as_mut_ptr() as *mut c_void);
                    assert_eq!(
                        rc, rr,
                        "pattern_info({what}) rc for {show:?} opts={opts:#x}"
                    );
                    assert_bytes_eq(
                        &format!("pattern_info({what}) for {show:?} opts={opts:#x}"),
                        &bc,
                        &br,
                    );
                }

                // Name table: compare the table contents, not the addresses.
                let mut ntc: *const u8 = std::ptr::null();
                let mut ntr: *const u8 = std::ptr::null();
                let rc = c(pair.c, NAMETABLE, &mut ntc as *mut _ as *mut c_void);
                let rr = r(pair.r, NAMETABLE, &mut ntr as *mut _ as *mut c_void);
                assert_eq!(rc, rr, "nametable rc for {show:?}");
                let mut count: u32 = 0;
                let mut size: u32 = 0;
                c(pair.c, 17, &mut count as *mut _ as *mut c_void);
                c(pair.c, 18, &mut size as *mut _ as *mut c_void);
                if count > 0 {
                    let n = (count * size) as usize;
                    assert_bytes_eq(
                        &format!("nametable bytes for {show:?} opts={opts:#x}"),
                        slice_at(ntc, n),
                        slice_at(ntr, n),
                    );
                }

                // First code unit bitmap.
                let mut fbc: *const u8 = std::ptr::null();
                let mut fbr: *const u8 = std::ptr::null();
                let rc = c(pair.c, FIRSTBITMAP, &mut fbc as *mut _ as *mut c_void);
                let rr = r(pair.r, FIRSTBITMAP, &mut fbr as *mut _ as *mut c_void);
                assert_eq!(rc, rr, "firstbitmap rc for {show:?}");
                assert_eq!(
                    fbc.is_null(),
                    fbr.is_null(),
                    "firstbitmap nullness for {show:?}"
                );
                if !fbc.is_null() {
                    assert_bytes_eq(
                        &format!("firstbitmap bytes for {show:?} opts={opts:#x}"),
                        slice_at(fbc, 32),
                        slice_at(fbr, 32),
                    );
                }
            }
        }
    }
}

#[test]
fn pattern_info_null_code() {
    let (c, r) = both::<PatternInfoFn2>("pcre2_pattern_info_8");
    unsafe {
        for what in 0u32..=30 {
            let mut bc = [0xAAu8; 32];
            let mut br = [0xAAu8; 32];
            assert_eq!(
                c(std::ptr::null(), what, bc.as_mut_ptr() as *mut c_void),
                r(std::ptr::null(), what, br.as_mut_ptr() as *mut c_void),
                "pattern_info(NULL, {what})"
            );
            assert_bytes_eq(&format!("pattern_info(NULL, {what})"), &bc, &br);
        }
    }
}

/* --------------------------- callout_enumerate --------------------------- */

/// `pcre2_callout_enumerate_block`, mirrored from pcre2.h.
#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct CalloutEnumBlock {
    version: u32,
    pattern_position: PCRE2_SIZE,
    next_item_length: PCRE2_SIZE,
    callout_number: u32,
    callout_string_offset: PCRE2_SIZE,
    callout_string_length: PCRE2_SIZE,
    callout_string: *const u8,
}

#[derive(Debug, PartialEq, Eq)]
struct EnumRecord {
    version: u32,
    pattern_position: PCRE2_SIZE,
    next_item_length: PCRE2_SIZE,
    callout_number: u32,
    callout_string_offset: PCRE2_SIZE,
    callout_string_length: PCRE2_SIZE,
    callout_string: Option<Vec<u8>>,
}

static ENUM_LOG: Mutex<Vec<EnumRecord>> = Mutex::new(Vec::new());

unsafe extern "C" fn enum_cb(blk: *const CalloutEnumBlock, _data: *mut c_void) -> i32 {
    unsafe {
        let b = &*blk;
        let s = if b.callout_string.is_null() {
            None
        } else {
            Some(slice_at(b.callout_string, b.callout_string_length).to_vec())
        };
        ENUM_LOG.lock().unwrap().push(EnumRecord {
            version: b.version,
            pattern_position: b.pattern_position,
            next_item_length: b.next_item_length,
            callout_number: b.callout_number,
            callout_string_offset: b.callout_string_offset,
            callout_string_length: b.callout_string_length,
            callout_string: s,
        });
    }
    0
}

/// Returns a non-zero value on the second callout to exercise early abort.
unsafe extern "C" fn enum_cb_abort(blk: *const CalloutEnumBlock, _data: *mut c_void) -> i32 {
    unsafe {
        let b = &*blk;
        let mut g = ENUM_LOG.lock().unwrap();
        g.push(EnumRecord {
            version: b.version,
            pattern_position: b.pattern_position,
            next_item_length: b.next_item_length,
            callout_number: b.callout_number,
            callout_string_offset: b.callout_string_offset,
            callout_string_length: b.callout_string_length,
            callout_string: None,
        });
        if g.len() >= 2 { 42 } else { 0 }
    }
}

type CalloutEnumFn = unsafe extern "C" fn(
    *const c_void,
    unsafe extern "C" fn(*const CalloutEnumBlock, *mut c_void) -> i32,
    *mut c_void,
) -> i32;

#[test]
fn callout_enumerate_matches() {
    let (c, r) = both::<CalloutEnumFn>("pcre2_callout_enumerate_8");
    for p in patterns() {
        for &opts in &[0u32, PCRE2_AUTO_CALLOUT, PCRE2_UTF, PCRE2_AUTO_CALLOUT | PCRE2_UTF] {
            let Some(pair) = compile_both(p, opts) else {
                continue;
            };
            let show = String::from_utf8_lossy(p).to_string();
            unsafe {
                ENUM_LOG.lock().unwrap().clear();
                let rc = c(pair.c, enum_cb, std::ptr::null_mut());
                let log_c = std::mem::take(&mut *ENUM_LOG.lock().unwrap());
                let rr = r(pair.r, enum_cb, std::ptr::null_mut());
                let log_r = std::mem::take(&mut *ENUM_LOG.lock().unwrap());
                assert_eq!(rc, rr, "callout_enumerate rc for {show:?} opts={opts:#x}");
                assert_eq!(
                    log_c, log_r,
                    "callout_enumerate blocks for {show:?} opts={opts:#x}"
                );

                // Early abort from the callback.
                ENUM_LOG.lock().unwrap().clear();
                let rc = c(pair.c, enum_cb_abort, std::ptr::null_mut());
                let log_c = std::mem::take(&mut *ENUM_LOG.lock().unwrap());
                let rr = r(pair.r, enum_cb_abort, std::ptr::null_mut());
                let log_r = std::mem::take(&mut *ENUM_LOG.lock().unwrap());
                assert_eq!(rc, rr, "callout_enumerate abort rc for {show:?}");
                assert_eq!(log_c, log_r, "callout_enumerate abort blocks for {show:?}");
            }
        }
    }
}

/* ------------------------ name table lookups ------------------------ */

type NametableScanFn = unsafe extern "C" fn(
    *const c_void,
    PCRE2_SPTR,
    *mut PCRE2_SPTR,
    *mut PCRE2_SPTR,
) -> i32;
type NumberFromNameFn = unsafe extern "C" fn(*const c_void, PCRE2_SPTR) -> i32;

#[test]
fn substring_name_lookup_matches() {
    let (nsc, nsr) = both::<NametableScanFn>("pcre2_substring_nametable_scan_8");
    let (nnc, nnr) = both::<NumberFromNameFn>("pcre2_substring_number_from_name_8");
    let names: &[&[u8]] = &[
        b"name\0", b"n\0", b"a\0", b"b\0", b"x\0", b"word\0", b"year\0", b"month\0", b"day\0",
        b"nonexistent\0", b"\0",
    ];
    for p in patterns() {
        for &opts in &[0u32, PCRE2_DUPNAMES] {
            let Some(pair) = compile_both(p, opts) else {
                continue;
            };
            let show = String::from_utf8_lossy(p).to_string();
            for nm in names {
                unsafe {
                    assert_eq!(
                        nnc(pair.c, nm.as_ptr()),
                        nnr(pair.r, nm.as_ptr()),
                        "number_from_name({nm:02x?}) for {show:?}"
                    );

                    // Count-only form (NULL first/last).
                    let rc = nsc(pair.c, nm.as_ptr(), std::ptr::null_mut(), std::ptr::null_mut());
                    let rr = nsr(pair.r, nm.as_ptr(), std::ptr::null_mut(), std::ptr::null_mut());
                    assert_eq!(rc, rr, "nametable_scan count {nm:02x?} for {show:?}");

                    // Range form: compare the entry contents.
                    let mut f_c: PCRE2_SPTR = std::ptr::null();
                    let mut l_c: PCRE2_SPTR = std::ptr::null();
                    let mut f_r: PCRE2_SPTR = std::ptr::null();
                    let mut l_r: PCRE2_SPTR = std::ptr::null();
                    let rc = nsc(pair.c, nm.as_ptr(), &mut f_c, &mut l_c);
                    let rr = nsr(pair.r, nm.as_ptr(), &mut f_r, &mut l_r);
                    assert_eq!(rc, rr, "nametable_scan range rc {nm:02x?} for {show:?}");
                    if rc >= 0 {
                        let entry_size = rc as usize;
                        let span_c = l_c.offset_from(f_c);
                        let span_r = l_r.offset_from(f_r);
                        assert_eq!(span_c, span_r, "nametable_scan span for {show:?}");
                        let n = span_c as usize + entry_size;
                        assert_bytes_eq(
                            &format!("nametable_scan entries {nm:02x?} for {show:?}"),
                            slice_at(f_c, n),
                            slice_at(f_r, n),
                        );
                    }
                }
            }
        }
    }
}
