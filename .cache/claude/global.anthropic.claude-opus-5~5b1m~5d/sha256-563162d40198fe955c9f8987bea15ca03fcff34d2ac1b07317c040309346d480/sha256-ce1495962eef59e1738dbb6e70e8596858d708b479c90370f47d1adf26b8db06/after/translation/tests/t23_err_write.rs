//! Phase C — write-path rejections.
//!
//! On the write side `png_error`, `png_app_error`, `png_app_warning` and
//! `png_benign_error` are all FATAL in this build (`PNG_RELEASE_BUILD == 0` and
//! `PNG_BENIGN_WRITE_ERRORS_SUPPORTED` is off), and `png_error` must not
//! return.  Rust cannot `setjmp`, so each case runs in a SUB-PROCESS: the test
//! binary re-executes itself once for the C library and once for the Rust
//! library with an error handler that prints the message and `exit(70)`s.  The
//! parent then compares the two transcripts — the ordered list of warnings, the
//! error message and the exit code — so a divergence in the message text, in
//! the *order* of the diagnostics, or in whether the call was fatal at all is
//! caught.
mod common;

use common::api::{apis, Api};
use common::harness::*;
use common::pngbuild as pb;
use common::*;
use std::ffi::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// the child: performs one named case against one library
// ---------------------------------------------------------------------------

/// Fresh write struct with the recording callbacks installed.
unsafe fn new_write(a: &Api) -> (png_structp, png_infop) {
    let p = (a.png_create_write_struct)(
        PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
        std::ptr::null_mut(),
        Some(error_cb),
        Some(warn_cb),
    );
    assert!(!p.is_null());
    let info = (a.png_create_info_struct)(p);
    assert!(!info.is_null());
    (a.png_set_write_fn)(p, std::ptr::null_mut(), Some(write_cb), Some(flush_cb));
    (p, info)
}

/// A minimal valid header so that row writing can be reached.
unsafe fn hdr(a: &Api, p: png_structp, info: png_infop, w: u32, h: u32, bd: c_int, ct: c_int) {
    (a.png_set_IHDR)(p, info, w, h, bd, ct, 0, 0, 0);
    if ct == PNG_COLOR_TYPE_PALETTE {
        let pal = [png_color { red: 1, green: 2, blue: 3 }; 4];
        (a.png_set_PLTE)(p, info, pal.as_ptr(), 4);
    }
}

fn run_case(a: &Api, case: &str) {
    unsafe {
        match case {
            // ---------------- png_create_write_struct ----------------
            "create-version-null" => {
                let p = (a.png_create_write_struct)(
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    Some(error_cb),
                    Some(warn_cb),
                );
                emit(format!("RET:{}", if p.is_null() { "NULL" } else { "ptr" }));
            }
            "create-version-empty" => {
                let p = (a.png_create_write_struct)(
                    c"".as_ptr(),
                    std::ptr::null_mut(),
                    Some(error_cb),
                    Some(warn_cb),
                );
                emit(format!("RET:{}", if p.is_null() { "NULL" } else { "ptr" }));
            }
            "create-version-major-mismatch" => {
                let p = (a.png_create_write_struct)(
                    c"2.6.59".as_ptr(),
                    std::ptr::null_mut(),
                    Some(error_cb),
                    Some(warn_cb),
                );
                emit(format!("RET:{}", if p.is_null() { "NULL" } else { "ptr" }));
            }
            "create-version-minor-mismatch" => {
                let p = (a.png_create_write_struct)(
                    c"1.5.59".as_ptr(),
                    std::ptr::null_mut(),
                    Some(error_cb),
                    Some(warn_cb),
                );
                emit(format!("RET:{}", if p.is_null() { "NULL" } else { "ptr" }));
            }
            "create-version-garbage" => {
                let p = (a.png_create_write_struct)(
                    c"garbage".as_ptr(),
                    std::ptr::null_mut(),
                    Some(error_cb),
                    Some(warn_cb),
                );
                emit(format!("RET:{}", if p.is_null() { "NULL" } else { "ptr" }));
            }

            // ---------------- png_set_IHDR / png_check_IHDR ----------------
            _ if case.starts_with("ihdr:") => {
                let f: Vec<&str> = case[5..].split(',').collect();
                let (p, info) = new_write(a);
                (a.png_set_IHDR)(
                    p,
                    info,
                    f[0].parse().unwrap(),
                    f[1].parse().unwrap(),
                    f[2].parse().unwrap(),
                    f[3].parse().unwrap(),
                    f[4].parse().unwrap(),
                    f[5].parse().unwrap(),
                    f[6].parse().unwrap(),
                );
                emit("set_IHDR returned");
                (a.png_write_info)(p, info);
                emit("write_info returned");
            }

            // ---------------- writing without a header ----------------
            "write-info-no-ihdr" => {
                let (p, info) = new_write(a);
                (a.png_write_info)(p, info);
                emit("write_info returned");
            }
            "write-row-before-info" => {
                let (p, info) = new_write(a);
                hdr(a, p, info, 4, 2, 8, 2);
                let row = [0u8; 12];
                (a.png_write_row)(p, row.as_ptr());
                emit("write_row returned");
            }
            "write-end-before-info" => {
                let (p, info) = new_write(a);
                hdr(a, p, info, 4, 2, 8, 2);
                (a.png_write_end)(p, info);
                emit("write_end returned");
            }
            "write-too-many-rows" => {
                let (p, info) = new_write(a);
                hdr(a, p, info, 4, 2, 8, 2);
                (a.png_write_info)(p, info);
                let row = [0u8; 12];
                for i in 0..5 {
                    (a.png_write_row)(p, row.as_ptr());
                    emit(format!("row {i} ok"));
                }
            }
            "write-end-too-few-rows" => {
                let (p, info) = new_write(a);
                hdr(a, p, info, 4, 4, 8, 2);
                (a.png_write_info)(p, info);
                let row = [0u8; 12];
                (a.png_write_row)(p, row.as_ptr());
                (a.png_write_end)(p, info);
                emit("write_end returned");
            }
            // NOTE: `png_write_image(p, NULL)` is NOT a testable rejection --
            // pngwrite.c dereferences `image[i]` with no NULL check, so the C
            // segfaults.  There is no check to compare against.
            "write-png-no-rows" => {
                let (p, info) = new_write(a);
                hdr(a, p, info, 4, 2, 8, 2);
                (a.png_write_png)(p, info, 0, std::ptr::null_mut());
                emit("write_png returned");
            }
            // NOTE: `png_write_row(p, NULL)` is likewise not a rejection -- the C
            // has no NULL check and dereferences the row pointer.
            "write-rows-zero" => {
                let (p, info) = new_write(a);
                hdr(a, p, info, 4, 2, 8, 2);
                (a.png_write_info)(p, info);
                (a.png_write_rows)(p, std::ptr::null_mut(), 0);
                emit("write_rows(NULL,0) returned");
            }

            // ---------------- PLTE ----------------
            _ if case.starts_with("plte:") => {
                let f: Vec<&str> = case[5..].split(',').collect();
                let bd: c_int = f[0].parse().unwrap();
                let ct: c_int = f[1].parse().unwrap();
                let n: c_int = f[2].parse().unwrap();
                let (p, info) = new_write(a);
                (a.png_set_IHDR)(p, info, 4, 2, bd, ct, 0, 0, 0);
                let pal = vec![png_color { red: 1, green: 2, blue: 3 }; 300];
                (a.png_set_PLTE)(p, info, pal.as_ptr(), n);
                emit("set_PLTE returned");
                (a.png_write_info)(p, info);
                emit("write_info returned");
            }
            "plte-null" => {
                let (p, info) = new_write(a);
                (a.png_set_IHDR)(p, info, 4, 2, 8, 3, 0, 0, 0);
                (a.png_set_PLTE)(p, info, std::ptr::null(), 4);
                emit("set_PLTE(NULL) returned");
            }
            "plte-missing" => {
                let (p, info) = new_write(a);
                (a.png_set_IHDR)(p, info, 4, 2, 8, 3, 0, 0, 0);
                (a.png_write_info)(p, info);
                emit("write_info returned");
            }
            "palette-index-out-of-range" => {
                let (p, info) = new_write(a);
                (a.png_set_IHDR)(p, info, 4, 2, 8, 3, 0, 0, 0);
                let pal = [png_color { red: 1, green: 2, blue: 3 }; 2];
                (a.png_set_PLTE)(p, info, pal.as_ptr(), 2);
                (a.png_set_check_for_invalid_index)(p, 1);
                (a.png_write_info)(p, info);
                let row = [200u8, 201, 202, 203];
                (a.png_write_row)(p, row.as_ptr());
                (a.png_write_row)(p, row.as_ptr());
                emit("rows written");
                (a.png_write_end)(p, info);
                emit("write_end returned");
            }

            // ---------------- tRNS ----------------
            _ if case.starts_with("trns:") => {
                let f: Vec<&str> = case[5..].split(',').collect();
                let ct: c_int = f[0].parse().unwrap();
                let n: c_int = f[1].parse().unwrap();
                let use_alpha: bool = f[2] == "1";
                let (p, info) = new_write(a);
                (a.png_set_IHDR)(p, info, 4, 2, 8, ct, 0, 0, 0);
                if ct == 3 {
                    let pal = [png_color { red: 1, green: 2, blue: 3 }; 4];
                    (a.png_set_PLTE)(p, info, pal.as_ptr(), 4);
                }
                let alpha = vec![0x80u8; 300];
                let col = png_color_16 {
                    index: 0,
                    red: 1,
                    green: 2,
                    blue: 3,
                    gray: 4,
                };
                (a.png_set_tRNS)(
                    p,
                    info,
                    if use_alpha {
                        alpha.as_ptr()
                    } else {
                        std::ptr::null()
                    },
                    n,
                    if use_alpha { std::ptr::null() } else { &col },
                );
                emit("set_tRNS returned");
                (a.png_write_info)(p, info);
                emit("write_info returned");
            }

            // ---------------- other chunk setters ----------------
            _ if case.starts_with("srgb:") => {
                let i: c_int = case[5..].parse().unwrap();
                let (p, info) = new_write(a);
                hdr(a, p, info, 4, 2, 8, 2);
                (a.png_set_sRGB)(p, info, i);
                emit("set_sRGB returned");
                (a.png_write_info)(p, info);
                emit("write_info returned");
            }
            _ if case.starts_with("gama:") => {
                let g: i32 = case[5..].parse().unwrap();
                let (p, info) = new_write(a);
                hdr(a, p, info, 4, 2, 8, 2);
                (a.png_set_gAMA_fixed)(p, info, g);
                emit("set_gAMA returned");
                (a.png_write_info)(p, info);
                emit("write_info returned");
            }
            _ if case.starts_with("sbit:") => {
                let f: Vec<&str> = case[5..].split(',').collect();
                let ct: c_int = f[0].parse().unwrap();
                let bd: c_int = f[1].parse().unwrap();
                let v: u8 = f[2].parse().unwrap();
                let (p, info) = new_write(a);
                hdr(a, p, info, 4, 2, bd, ct);
                let s = png_color_8 {
                    red: v,
                    green: v,
                    blue: v,
                    gray: v,
                    alpha: v,
                };
                (a.png_set_sBIT)(p, info, &s);
                emit("set_sBIT returned");
                (a.png_write_info)(p, info);
                emit("write_info returned");
            }
            _ if case.starts_with("scal:") => {
                let f: Vec<&str> = case[5..].split(',').collect();
                let (p, info) = new_write(a);
                hdr(a, p, info, 4, 2, 8, 2);
                (a.png_set_sCAL_fixed)(
                    p,
                    info,
                    f[0].parse().unwrap(),
                    f[1].parse().unwrap(),
                    f[2].parse().unwrap(),
                );
                emit("set_sCAL returned");
                (a.png_write_info)(p, info);
                emit("write_info returned");
            }
            _ if case.starts_with("scal-s:") => {
                let f: Vec<&str> = case[7..].split(',').collect();
                let (p, info) = new_write(a);
                hdr(a, p, info, 4, 2, 8, 2);
                let w = std::ffi::CString::new(f[1]).unwrap();
                let h = std::ffi::CString::new(f[2]).unwrap();
                (a.png_set_sCAL_s)(p, info, f[0].parse().unwrap(), w.as_ptr(), h.as_ptr());
                emit("set_sCAL_s returned");
                (a.png_write_info)(p, info);
                emit("write_info returned");
            }
            _ if case.starts_with("pcal:") => {
                let f: Vec<&str> = case[5..].split(',').collect();
                let (p, info) = new_write(a);
                hdr(a, p, info, 4, 2, 8, 2);
                let mut params: Vec<*mut c_char> = Vec::new();
                let keep: Vec<std::ffi::CString> = (0..f[3].parse::<usize>().unwrap())
                    .map(|i| std::ffi::CString::new(format!("{i}.0")).unwrap())
                    .collect();
                for k in &keep {
                    params.push(k.as_ptr() as *mut c_char);
                }
                (a.png_set_pCAL)(
                    p,
                    info,
                    c"purpose".as_ptr(),
                    f[0].parse().unwrap(),
                    f[1].parse().unwrap(),
                    f[2].parse().unwrap(),
                    f[3].parse().unwrap(),
                    c"units".as_ptr(),
                    if params.is_empty() {
                        std::ptr::null_mut()
                    } else {
                        params.as_mut_ptr()
                    },
                );
                emit("set_pCAL returned");
                (a.png_write_info)(p, info);
                emit("write_info returned");
            }
            _ if case.starts_with("iccp:") => {
                let f: Vec<&str> = case[5..].split(',').collect();
                let comp: c_int = f[0].parse().unwrap();
                let proflen: u32 = f[1].parse().unwrap();
                let (p, info) = new_write(a);
                hdr(a, p, info, 4, 2, 8, 2);
                let prof = vec![0u8; proflen.max(1) as usize];
                (a.png_set_iCCP)(p, info, c"icc".as_ptr(), comp, prof.as_ptr(), proflen);
                emit("set_iCCP returned");
                (a.png_write_info)(p, info);
                emit("write_info returned");
            }
            "iccp-null-name" => {
                let (p, info) = new_write(a);
                hdr(a, p, info, 4, 2, 8, 2);
                let prof = vec![0u8; 132];
                (a.png_set_iCCP)(p, info, std::ptr::null(), 0, prof.as_ptr(), 132);
                emit("set_iCCP returned");
            }
            "iccp-null-profile" => {
                let (p, info) = new_write(a);
                hdr(a, p, info, 4, 2, 8, 2);
                (a.png_set_iCCP)(p, info, c"icc".as_ptr(), 0, std::ptr::null(), 132);
                emit("set_iCCP returned");
            }
            _ if case.starts_with("splt:") => {
                let f: Vec<&str> = case[5..].split(',').collect();
                let depth: u8 = f[0].parse().unwrap();
                let nent: i32 = f[1].parse().unwrap();
                let (p, info) = new_write(a);
                hdr(a, p, info, 4, 2, 8, 2);
                let mut entries = vec![
                    png_sPLT_entry {
                        red: 1,
                        green: 2,
                        blue: 3,
                        alpha: 4,
                        frequency: 5,
                    };
                    8
                ];
                let name = std::ffi::CString::new("splt").unwrap();
                let mut s = png_sPLT_t {
                    name: name.as_ptr() as *mut c_char,
                    depth,
                    entries: entries.as_mut_ptr(),
                    nentries: nent,
                };
                (a.png_set_sPLT)(p, info, &s, 1);
                emit("set_sPLT returned");
                (a.png_write_info)(p, info);
                emit("write_info returned");
            }
            _ if case.starts_with("text:") => {
                let f: Vec<&str> = case[5..].split(';').collect();
                let comp: c_int = f[0].parse().unwrap();
                let key = f[1].to_string();
                let (p, info) = new_write(a);
                hdr(a, p, info, 4, 2, 8, 2);
                let mut k = key.into_bytes();
                k.push(0);
                let mut v = b"value\0".to_vec();
                let t = png_text {
                    compression: comp,
                    key: k.as_mut_ptr() as *mut c_char,
                    text: v.as_mut_ptr() as *mut c_char,
                    text_length: 0,
                    itxt_length: 0,
                    lang: std::ptr::null_mut(),
                    lang_key: std::ptr::null_mut(),
                };
                let r = (a.png_set_text_2)(p, info, &t, 1);
                emit(format!("set_text_2:{r}"));
                (a.png_write_info)(p, info);
                emit("write_info returned");
            }
            "text-null-key" => {
                let (p, info) = new_write(a);
                hdr(a, p, info, 4, 2, 8, 2);
                let mut v = b"value\0".to_vec();
                let t = png_text {
                    compression: PNG_TEXT_COMPRESSION_NONE,
                    key: std::ptr::null_mut(),
                    text: v.as_mut_ptr() as *mut c_char,
                    text_length: 0,
                    itxt_length: 0,
                    lang: std::ptr::null_mut(),
                    lang_key: std::ptr::null_mut(),
                };
                let r = (a.png_set_text_2)(p, info, &t, 1);
                emit(format!("set_text_2:{r}"));
            }
            _ if case.starts_with("time:") => {
                let f: Vec<&str> = case[5..].split(',').collect();
                let (p, info) = new_write(a);
                hdr(a, p, info, 4, 2, 8, 2);
                let t = png_time {
                    year: f[0].parse().unwrap(),
                    month: f[1].parse().unwrap(),
                    day: f[2].parse().unwrap(),
                    hour: f[3].parse().unwrap(),
                    minute: f[4].parse().unwrap(),
                    second: f[5].parse().unwrap(),
                };
                (a.png_set_tIME)(p, info, &t);
                emit("set_tIME returned");
                (a.png_write_info)(p, info);
                emit("write_info returned");
            }
            _ if case.starts_with("hist:") => {
                let n: c_int = case[5..].parse().unwrap();
                let (p, info) = new_write(a);
                (a.png_set_IHDR)(p, info, 4, 2, 8, 3, 0, 0, 0);
                if n > 0 {
                    let pal = vec![png_color { red: 1, green: 2, blue: 3 }; n as usize];
                    (a.png_set_PLTE)(p, info, pal.as_ptr(), n);
                }
                let h = vec![7u16; 300];
                (a.png_set_hIST)(p, info, h.as_ptr());
                emit("set_hIST returned");
                (a.png_write_info)(p, info);
                emit("write_info returned");
            }
            "hist-no-plte" => {
                let (p, info) = new_write(a);
                (a.png_set_IHDR)(p, info, 4, 2, 8, 3, 0, 0, 0);
                let h = vec![7u16; 300];
                (a.png_set_hIST)(p, info, h.as_ptr());
                emit("set_hIST returned");
            }
            _ if case.starts_with("cicp:") => {
                let f: Vec<&str> = case[5..].split(',').collect();
                let (p, info) = new_write(a);
                hdr(a, p, info, 4, 2, 8, 2);
                (a.png_set_cICP)(
                    p,
                    info,
                    f[0].parse().unwrap(),
                    f[1].parse().unwrap(),
                    f[2].parse().unwrap(),
                    f[3].parse().unwrap(),
                );
                emit("set_cICP returned");
                (a.png_write_info)(p, info);
                emit("write_info returned");
            }
            _ if case.starts_with("exif:") => {
                let n: i64 = case[5..].parse().unwrap();
                let (p, info) = new_write(a);
                hdr(a, p, info, 4, 2, 8, 2);
                let mut d = vec![b'I', b'I', 0x2a, 0, 8, 0, 0, 0];
                (a.png_set_eXIf_1)(p, info, n as u32, d.as_mut_ptr());
                emit("set_eXIf_1 returned");
                (a.png_write_info)(p, info);
                emit("write_info returned");
            }

            // ---------------- compression parameters ----------------
            _ if case.starts_with("clevel:") => {
                let v: c_int = case[7..].parse().unwrap();
                let (p, info) = new_write(a);
                (a.png_set_compression_level)(p, v);
                emit("set returned");
                hdr(a, p, info, 4, 2, 8, 2);
                (a.png_write_info)(p, info);
                let row = [0u8; 12];
                (a.png_write_row)(p, row.as_ptr());
                (a.png_write_row)(p, row.as_ptr());
                (a.png_write_end)(p, info);
                emit(format!("bytes:{}", out_len()));
            }
            _ if case.starts_with("cmem:") => {
                let v: c_int = case[5..].parse().unwrap();
                let (p, info) = new_write(a);
                (a.png_set_compression_mem_level)(p, v);
                emit("set returned");
                hdr(a, p, info, 4, 2, 8, 2);
                (a.png_write_info)(p, info);
                let row = [0u8; 12];
                (a.png_write_row)(p, row.as_ptr());
                (a.png_write_row)(p, row.as_ptr());
                (a.png_write_end)(p, info);
                emit(format!("bytes:{}", out_len()));
            }
            _ if case.starts_with("cwbits:") => {
                let v: c_int = case[7..].parse().unwrap();
                let (p, info) = new_write(a);
                (a.png_set_compression_window_bits)(p, v);
                emit("set returned");
                hdr(a, p, info, 4, 2, 8, 2);
                (a.png_write_info)(p, info);
                let row = [0u8; 12];
                (a.png_write_row)(p, row.as_ptr());
                (a.png_write_row)(p, row.as_ptr());
                (a.png_write_end)(p, info);
                emit(format!("bytes:{}", out_len()));
            }
            _ if case.starts_with("cmethod:") => {
                let v: c_int = case[8..].parse().unwrap();
                let (p, info) = new_write(a);
                (a.png_set_compression_method)(p, v);
                emit("set returned");
                hdr(a, p, info, 4, 2, 8, 2);
                (a.png_write_info)(p, info);
                let row = [0u8; 12];
                (a.png_write_row)(p, row.as_ptr());
                (a.png_write_row)(p, row.as_ptr());
                (a.png_write_end)(p, info);
                emit(format!("bytes:{}", out_len()));
            }
            _ if case.starts_with("cstrategy:") => {
                let v: c_int = case[10..].parse().unwrap();
                let (p, info) = new_write(a);
                (a.png_set_compression_strategy)(p, v);
                emit("set returned");
                hdr(a, p, info, 4, 2, 8, 2);
                (a.png_write_info)(p, info);
                let row = [0u8; 12];
                (a.png_write_row)(p, row.as_ptr());
                (a.png_write_row)(p, row.as_ptr());
                (a.png_write_end)(p, info);
                emit(format!("bytes:{}", out_len()));
            }
            _ if case.starts_with("cbuf:") => {
                let v: usize = case[5..].parse().unwrap();
                let (p, info) = new_write(a);
                (a.png_set_compression_buffer_size)(p, v);
                emit("set returned");
                hdr(a, p, info, 4, 2, 8, 2);
                (a.png_write_info)(p, info);
                let row = [0u8; 12];
                (a.png_write_row)(p, row.as_ptr());
                (a.png_write_row)(p, row.as_ptr());
                (a.png_write_end)(p, info);
                emit(format!("bytes:{}", out_len()));
            }
            _ if case.starts_with("tclevel:") => {
                let v: c_int = case[8..].parse().unwrap();
                let (p, info) = new_write(a);
                (a.png_set_text_compression_level)(p, v);
                emit("set returned");
            }
            _ if case.starts_with("tcmem:") => {
                let v: c_int = case[6..].parse().unwrap();
                let (p, _info) = new_write(a);
                (a.png_set_text_compression_mem_level)(p, v);
                emit("set returned");
            }
            _ if case.starts_with("tcwbits:") => {
                let v: c_int = case[8..].parse().unwrap();
                let (p, _info) = new_write(a);
                (a.png_set_text_compression_window_bits)(p, v);
                emit("set returned");
            }
            _ if case.starts_with("tcmethod:") => {
                let v: c_int = case[9..].parse().unwrap();
                let (p, _info) = new_write(a);
                (a.png_set_text_compression_method)(p, v);
                emit("set returned");
            }
            _ if case.starts_with("tcstrategy:") => {
                let v: c_int = case[11..].parse().unwrap();
                let (p, _info) = new_write(a);
                (a.png_set_text_compression_strategy)(p, v);
                emit("set returned");
            }

            // ---------------- png_set_filter ----------------
            _ if case.starts_with("filter:") => {
                let f: Vec<&str> = case[7..].split(',').collect();
                let method: c_int = f[0].parse().unwrap();
                let filters: c_int = f[1].parse().unwrap();
                let after_start: bool = f[2] == "1";
                let (p, info) = new_write(a);
                hdr(a, p, info, 4, 2, 8, 2);
                if after_start {
                    (a.png_write_info)(p, info);
                }
                (a.png_set_filter)(p, method, filters);
                emit("set_filter returned");
                if !after_start {
                    (a.png_write_info)(p, info);
                }
                let row = [0u8; 12];
                (a.png_write_row)(p, row.as_ptr());
                (a.png_write_row)(p, row.as_ptr());
                (a.png_write_end)(p, info);
                emit(format!("bytes:{}", out_len()));
            }
            _ if case.starts_with("filter-heur:") => {
                let f: Vec<&str> = case[12..].split(',').collect();
                let (p, _info) = new_write(a);
                (a.png_set_filter_heuristics)(
                    p,
                    f[0].parse().unwrap(),
                    f[1].parse().unwrap(),
                    std::ptr::null(),
                    std::ptr::null(),
                );
                emit("set_filter_heuristics returned");
            }

            // ---------------- write transforms ----------------
            _ if case.starts_with("filler:") => {
                let f: Vec<&str> = case[7..].split(',').collect();
                let bd: c_int = f[0].parse().unwrap();
                let ct: c_int = f[1].parse().unwrap();
                let flags: c_int = f[2].parse().unwrap();
                let (p, info) = new_write(a);
                hdr(a, p, info, 4, 2, bd, ct);
                (a.png_write_info)(p, info);
                (a.png_set_filler)(p, 0xffff, flags);
                emit("set_filler returned");
            }
            _ if case.starts_with("shift:") => {
                let f: Vec<&str> = case[6..].split(',').collect();
                let bd: c_int = f[0].parse().unwrap();
                let ct: c_int = f[1].parse().unwrap();
                let v: u8 = f[2].parse().unwrap();
                let (p, info) = new_write(a);
                hdr(a, p, info, 4, 2, bd, ct);
                (a.png_write_info)(p, info);
                let s = png_color_8 {
                    red: v,
                    green: v,
                    blue: v,
                    gray: v,
                    alpha: v,
                };
                (a.png_set_shift)(p, &s);
                emit("set_shift returned");
            }
            "swap-after-start" => {
                let (p, info) = new_write(a);
                hdr(a, p, info, 4, 2, 8, 2);
                (a.png_write_info)(p, info);
                let row = [0u8; 12];
                (a.png_write_row)(p, row.as_ptr());
                (a.png_set_swap)(p);
                emit("set_swap returned");
            }
            "invert-mono-after-start" => {
                let (p, info) = new_write(a);
                hdr(a, p, info, 4, 2, 8, 2);
                (a.png_write_info)(p, info);
                let row = [0u8; 12];
                (a.png_write_row)(p, row.as_ptr());
                (a.png_set_invert_mono)(p);
                emit("set_invert_mono returned");
            }

            // ---------------- raw chunk API ----------------
            "chunk-start-bad-name" => {
                let (p, _info) = new_write(a);
                (a.png_write_sig)(p);
                (a.png_write_chunk_start)(p, b"12345".as_ptr(), 0);
                emit("chunk_start returned");
                (a.png_write_chunk_end)(p);
                emit("chunk_end returned");
            }
            "chunk-start-huge-length" => {
                let (p, _info) = new_write(a);
                (a.png_write_sig)(p);
                (a.png_write_chunk_start)(p, b"prVt".as_ptr(), 0xffff_ffff);
                emit("chunk_start returned");
            }
            "chunk-data-more-than-declared" => {
                let (p, _info) = new_write(a);
                (a.png_write_sig)(p);
                (a.png_write_chunk_start)(p, b"prVt".as_ptr(), 2);
                (a.png_write_chunk_data)(p, b"abcdef".as_ptr(), 6);
                emit("chunk_data returned");
                (a.png_write_chunk_end)(p);
                emit(format!("bytes:{}", out_len()));
            }
            "chunk-data-null" => {
                let (p, _info) = new_write(a);
                (a.png_write_sig)(p);
                (a.png_write_chunk_start)(p, b"prVt".as_ptr(), 4);
                (a.png_write_chunk_data)(p, std::ptr::null(), 4);
                emit("chunk_data(NULL) returned");
                (a.png_write_chunk_end)(p);
                emit(format!("bytes:{}", out_len()));
            }
            "chunk-end-without-start" => {
                let (p, _info) = new_write(a);
                (a.png_write_sig)(p);
                (a.png_write_chunk_end)(p);
                emit(format!("bytes:{}", out_len()));
            }

            // ---------------- NULL / missing IO ----------------
            // NOTE: never calling `png_set_write_fn` is not a rejection either:
            // `png_create_write_struct` installs `png_default_write_data`, which
            // calls `fwrite` on a NULL `FILE*`.  The "Call to NULL write
            // function" branch is reached by the `write-fn-null` case below.
            // NOTE: `png_write_data`'s "Call to NULL write function" branch
            // (pngwio.c:40) is UNREACHABLE through the public API: passing NULL
            // to `png_set_write_fn` makes it install `png_default_write_data`,
            // which then calls `fwrite` on the NULL `io_ptr`.  Recorded here so
            // the row is accounted for rather than silently skipped.
            "read-fn-on-write-struct" => {
                let p = (a.png_create_write_struct)(
                    PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
                    std::ptr::null_mut(),
                    Some(error_cb),
                    Some(warn_cb),
                );
                (a.png_set_read_fn)(p, std::ptr::null_mut(), Some(read_cb));
                emit("set_read_fn on write struct returned");
            }
            "write-fn-on-read-struct" => {
                let p = (a.png_create_read_struct)(
                    PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
                    std::ptr::null_mut(),
                    Some(error_cb),
                    Some(warn_cb),
                );
                (a.png_set_write_fn)(p, std::ptr::null_mut(), Some(write_cb), Some(flush_cb));
                emit("set_write_fn on read struct returned");
            }

            // ---------------- error / warning dispatchers ----------------
            "png_error-direct" => {
                let (p, _info) = new_write(a);
                (a.png_error)(p, c"deliberate error".as_ptr());
                emit("png_error returned (should not happen)");
            }
            "png_error-null-message" => {
                let (p, _info) = new_write(a);
                (a.png_error)(p, std::ptr::null());
                emit("png_error returned (should not happen)");
            }
            "png_error-long-message" => {
                let (p, _info) = new_write(a);
                let m = std::ffi::CString::new("x".repeat(500)).unwrap();
                (a.png_error)(p, m.as_ptr());
                emit("png_error returned (should not happen)");
            }
            "png_warning-direct" => {
                let (p, _info) = new_write(a);
                (a.png_warning)(p, c"deliberate warning".as_ptr());
                emit("png_warning returned");
            }
            "png_warning-null" => {
                let (p, _info) = new_write(a);
                (a.png_warning)(p, std::ptr::null());
                emit("png_warning returned");
            }
            "png_warning-long" => {
                let (p, _info) = new_write(a);
                let m = std::ffi::CString::new("y".repeat(500)).unwrap();
                (a.png_warning)(p, m.as_ptr());
                emit("png_warning returned");
            }
            "png_benign_error-write" => {
                let (p, _info) = new_write(a);
                (a.png_benign_error)(p, c"benign on write".as_ptr());
                emit("png_benign_error returned");
            }
            "png_benign_error-write-allowed" => {
                let (p, _info) = new_write(a);
                (a.png_set_benign_errors)(p, 1);
                (a.png_benign_error)(p, c"benign on write".as_ptr());
                emit("png_benign_error returned");
            }
            "png_benign_error-read" => {
                let p = (a.png_create_read_struct)(
                    PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
                    std::ptr::null_mut(),
                    Some(error_cb),
                    Some(warn_cb),
                );
                (a.png_benign_error)(p, c"benign on read".as_ptr());
                emit("png_benign_error returned");
            }
            "png_chunk_error-write" => {
                let (p, _info) = new_write(a);
                (a.png_chunk_error)(p, c"chunk problem".as_ptr());
                emit("png_chunk_error returned (should not happen)");
            }
            "png_chunk_warning-write" => {
                let (p, _info) = new_write(a);
                (a.png_chunk_warning)(p, c"chunk problem".as_ptr());
                emit("png_chunk_warning returned");
            }
            "png_chunk_benign_error-read" => {
                let p = (a.png_create_read_struct)(
                    PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
                    std::ptr::null_mut(),
                    Some(error_cb),
                    Some(warn_cb),
                );
                (a.png_chunk_benign_error)(p, c"chunk benign".as_ptr());
                emit("png_chunk_benign_error returned");
            }
            "png_longjmp-no-jmpbuf" => {
                let (p, _info) = new_write(a);
                (a.png_longjmp)(p, 1);
                emit("png_longjmp returned (should not happen)");
            }
            "png_error-null-struct" => {
                (a.png_error)(std::ptr::null_mut(), c"no struct".as_ptr());
                emit("png_error(NULL) returned (should not happen)");
            }
            "png_warning-null-struct" => {
                (a.png_warning)(std::ptr::null_mut(), c"no struct".as_ptr());
                emit("png_warning(NULL) returned");
            }

            // ---------------- png_set_longjmp_fn ----------------
            "set_longjmp-null-fn" => {
                let (p, _info) = new_write(a);
                let r = (a.png_set_longjmp_fn)(p, None, 200);
                emit(format!("ret:{}", if r.is_null() { "NULL" } else { "ptr" }));
            }
            "set_longjmp-zero-size" => {
                let (p, _info) = new_write(a);
                let r = (a.png_set_longjmp_fn)(p, Some(dummy_longjmp), 0);
                emit(format!("ret:{}", if r.is_null() { "NULL" } else { "ptr" }));
            }
            "set_longjmp-null-struct" => {
                let r = (a.png_set_longjmp_fn)(std::ptr::null_mut(), Some(dummy_longjmp), 200);
                emit(format!("ret:{}", if r.is_null() { "NULL" } else { "ptr" }));
            }

            // ---------------- memory ----------------
            _ if case.starts_with("malloc:") => {
                let n: usize = case[7..].parse().unwrap();
                let (p, _info) = new_write(a);
                let q = (a.png_malloc_warn)(p, n);
                emit(format!("malloc_warn:{}", if q.is_null() { "NULL" } else { "ptr" }));
                if !q.is_null() {
                    (a.png_free)(p, q);
                }
                let q = (a.png_calloc)(p, n);
                emit(format!("calloc:{}", if q.is_null() { "NULL" } else { "ptr" }));
                if !q.is_null() {
                    (a.png_free)(p, q);
                }
                let q = (a.png_malloc)(p, n);
                emit(format!("malloc:{}", if q.is_null() { "NULL" } else { "ptr" }));
            }
            "free-null" => {
                let (p, _info) = new_write(a);
                (a.png_free)(p, std::ptr::null_mut());
                (a.png_free_default)(p, std::ptr::null_mut());
                emit("free(NULL) ok");
            }

            // ---------------- png_permit_mng_features / png_set_option ----
            _ if case.starts_with("mng:") => {
                let v: u32 = case[4..].parse().unwrap();
                let (p, _info) = new_write(a);
                let r = (a.png_permit_mng_features)(p, v);
                emit(format!("mng:{r}"));
            }
            _ if case.starts_with("option:") => {
                let f: Vec<&str> = case[7..].split(',').collect();
                let (p, _info) = new_write(a);
                let r = (a.png_set_option)(p, f[0].parse().unwrap(), f[1].parse().unwrap());
                emit(format!("option:{r}"));
            }

            other => {
                emit(format!("UNKNOWN CASE {other}"));
                std::process::exit(3);
            }
        }
    }
    child_finish();
}

pub unsafe extern "C" fn dummy_longjmp(_b: *mut c_void, _v: c_int) {
    // A longjmp replacement that must not return; exiting is the only portable
    // choice from Rust and is identical for both libraries.
    println!("@@LONGJMP");
    std::process::exit(71);
}

/// The sub-process entry point.  Does nothing in the parent.
#[test]
fn harness_child() {
    let Some((case, which)) = child_case() else {
        return;
    };
    set_child_mode(true);
    let b = apis();
    let a = if which == "c" { &b.c } else { &b.rs };
    set_cur_is_c(which == "c");
    reset_all();
    run_case(a, &case);
}

// ---------------------------------------------------------------------------
// the parent: one #[test] per group of ERRORS.md rows
// ---------------------------------------------------------------------------

fn run_all(cases: &[String]) {
    for c in cases {
        diff_case(c);
    }
}

#[test]
fn create_struct_version_rejections() {
    run_all(&[
        "create-version-null".into(),
        "create-version-empty".into(),
        "create-version-major-mismatch".into(),
        "create-version-minor-mismatch".into(),
        "create-version-garbage".into(),
    ]);
}

#[test]
fn ihdr_rejections() {
    let mut cases = Vec::new();
    // every bit depth x colour type, valid and invalid
    for bd in [0i32, 1, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17, 32, -1] {
        for ct in [0i32, 1, 2, 3, 4, 5, 6, 7, 8, -1] {
            cases.push(format!("ihdr:4,2,{bd},{ct},0,0,0"));
        }
    }
    // dimensions
    for (w, h) in [
        (0u32, 2u32),
        (4, 0),
        (0, 0),
        (0x8000_0000, 2),
        (2, 0x8000_0000),
        (0xffff_ffff, 1),
        (1_000_001, 1),
        (1, 1_000_001),
        (1_000_000, 1),
        (1, 1),
    ] {
        cases.push(format!("ihdr:{w},{h},8,2,0,0,0"));
    }
    // interlace / compression / filter methods
    for il in [0i32, 1, 2, 3, 255, -1] {
        cases.push(format!("ihdr:4,2,8,2,{il},0,0"));
    }
    for cm in [0i32, 1, 2, 8, 255, -1] {
        cases.push(format!("ihdr:4,2,8,2,0,{cm},0"));
    }
    for fm in [0i32, 1, 2, 64, 255, -1] {
        cases.push(format!("ihdr:4,2,8,2,0,0,{fm}"));
    }
    run_all(&cases);
}

#[test]
fn missing_and_extra_data_rejections() {
    run_all(&[
        "write-info-no-ihdr".into(),
        "write-row-before-info".into(),
        "write-end-before-info".into(),
        "write-too-many-rows".into(),
        "write-end-too-few-rows".into(),
        "write-png-no-rows".into(),
        "write-rows-zero".into(),
        "read-fn-on-write-struct".into(),
        "write-fn-on-read-struct".into(),
    ]);
}

#[test]
fn plte_rejections() {
    let mut cases = Vec::new();
    for &(bd, ct) in &[(1i32, 3i32), (2, 3), (4, 3), (8, 3), (8, 2), (8, 0), (8, 6), (16, 2)] {
        for n in [-1i32, 0, 1, 2, 3, 4, 5, 16, 17, 255, 256, 257, 300] {
            cases.push(format!("plte:{bd},{ct},{n}"));
        }
    }
    cases.push("plte-null".into());
    cases.push("plte-missing".into());
    cases.push("palette-index-out-of-range".into());
    run_all(&cases);
}

#[test]
fn trns_rejections() {
    let mut cases = Vec::new();
    for ct in [0i32, 2, 3, 4, 6] {
        for n in [-1i32, 0, 1, 2, 4, 5, 256, 257, 300] {
            for alpha in [0, 1] {
                cases.push(format!("trns:{ct},{n},{alpha}"));
            }
        }
    }
    run_all(&cases);
}

#[test]
fn chunk_setter_rejections() {
    let mut cases = Vec::new();
    for i in [-1i32, 0, 1, 2, 3, 4, 5, 255] {
        cases.push(format!("srgb:{i}"));
    }
    for g in [i32::MIN, -1, 0, 1, 100000, 0x7fff_ffff] {
        cases.push(format!("gama:{g}"));
    }
    for &(ct, bd) in &[(0i32, 1i32), (0, 8), (0, 16), (2, 8), (2, 16), (3, 8), (4, 8), (6, 16)] {
        for v in [0u8, 1, 8, 9, 16, 17, 255] {
            cases.push(format!("sbit:{ct},{bd},{v}"));
        }
    }
    for u in [0i32, 1, 2, 3, 255, -1] {
        cases.push(format!("scal:{u},100000,200000"));
    }
    for (w, h) in [(0i32, 100000i32), (100000, 0), (-1, 100000), (100000, -1)] {
        cases.push(format!("scal:1,{w},{h}"));
    }
    for s in ["0", "-1.0", "abc", "", "1.0"] {
        cases.push(format!("scal-s:1,{s},1.0"));
        cases.push(format!("scal-s:1,1.0,{s}"));
    }
    // pCAL: X0,X1,type,nparams
    for ty in [-1i32, 0, 1, 2, 3, 4, 255] {
        cases.push(format!("pcal:0,255,{ty},2"));
    }
    for np in [-1i32, 0, 1, 2, 3, 4] {
        cases.push(format!("pcal:0,255,0,{np}"));
    }
    cases.push("pcal:5,5,0,0".into());
    // iCCP: compression type, profile length
    for comp in [-1i32, 0, 1, 2, 255] {
        cases.push(format!("iccp:{comp},132"));
    }
    for len in [0u32, 1, 4, 127, 128, 132, 0xffff_ffff] {
        cases.push(format!("iccp:0,{len}"));
    }
    cases.push("iccp-null-name".into());
    cases.push("iccp-null-profile".into());
    // sPLT depth, nentries
    for d in [0u8, 1, 4, 8, 16, 32, 255] {
        cases.push(format!("splt:{d},4"));
    }
    for n in [-1i32, 0, 1, 8] {
        cases.push(format!("splt:8,{n}"));
    }
    // text
    for comp in [-4i32, -3, -2, -1, 0, 1, 2, 3, 255] {
        cases.push(format!("text:{comp};Title"));
    }
    for key in ["", " Key", "Key ", "Ke  y", "Ke\ty", &"K".repeat(200)] {
        cases.push(format!("text:-1;{key}"));
    }
    cases.push("text-null-key".into());
    // tIME
    for t in [
        "2024,0,1,0,0,0",
        "2024,13,1,0,0,0",
        "2024,1,0,0,0,0",
        "2024,1,32,0,0,0",
        "2024,1,1,24,0,0",
        "2024,1,1,0,60,0",
        "2024,1,1,0,0,61",
        "65535,255,255,255,255,255",
        "2024,2,29,23,59,60",
    ] {
        cases.push(format!("time:{t}"));
    }
    // hIST
    for n in [0i32, 1, 2, 4, 255, 256] {
        cases.push(format!("hist:{n}"));
    }
    cases.push("hist-no-plte".into());
    // cICP
    for mc in [0u8, 1, 2, 255] {
        cases.push(format!("cicp:9,16,{mc},1"));
    }
    for vf in [0u8, 1, 2, 255] {
        cases.push(format!("cicp:9,16,0,{vf}"));
    }
    // eXIf
    for n in [0i64, 1, 2, 3, 4, 8, 100] {
        cases.push(format!("exif:{n}"));
    }
    run_all(&cases);
}

#[test]
fn compression_parameter_rejections() {
    let mut cases = Vec::new();
    for v in [-2i32, -1, 0, 1, 9, 10, 100, i32::MAX, i32::MIN] {
        cases.push(format!("clevel:{v}"));
        cases.push(format!("tclevel:{v}"));
    }
    for v in [-1i32, 0, 1, 8, 9, 10, 100] {
        cases.push(format!("cmem:{v}"));
        cases.push(format!("tcmem:{v}"));
    }
    for v in [-1i32, 0, 7, 8, 9, 15, 16, 100] {
        cases.push(format!("cwbits:{v}"));
        cases.push(format!("tcwbits:{v}"));
    }
    for v in [-1i32, 0, 1, 8, 9, 255] {
        cases.push(format!("cmethod:{v}"));
        cases.push(format!("tcmethod:{v}"));
    }
    for v in [-1i32, 0, 1, 2, 3, 4, 5, 100] {
        cases.push(format!("cstrategy:{v}"));
        cases.push(format!("tcstrategy:{v}"));
    }
    for v in [0usize, 1, 2, 100, 1024, 65536] {
        cases.push(format!("cbuf:{v}"));
    }
    run_all(&cases);
}

#[test]
fn filter_rejections() {
    let mut cases = Vec::new();
    for method in [-1i32, 0, 1, 64, 65, 255] {
        cases.push(format!("filter:{method},248,0"));
    }
    for filters in [-1i32, 0, 1, 7, 8, 0xf8, 0xff, 0x100] {
        cases.push(format!("filter:0,{filters},0"));
    }
    // setting filters after the write has started
    cases.push("filter:0,8,1".into());
    cases.push("filter:0,248,1".into());
    for h in [-1i32, 0, 1, 2, 3, 4] {
        cases.push(format!("filter-heur:{h},0"));
    }
    run_all(&cases);
}

#[test]
fn write_transform_rejections() {
    let mut cases = Vec::new();
    for &(bd, ct) in &[(1i32, 0i32), (2, 0), (4, 0), (8, 0), (16, 0), (8, 2), (8, 3), (8, 4), (8, 6)] {
        for flags in [-1i32, 0, 1, 2, 255] {
            cases.push(format!("filler:{bd},{ct},{flags}"));
        }
        for v in [0u8, 1, 8, 9, 16, 17, 255] {
            cases.push(format!("shift:{bd},{ct},{v}"));
        }
    }
    cases.push("swap-after-start".into());
    cases.push("invert-mono-after-start".into());
    run_all(&cases);
}

#[test]
fn raw_chunk_api_rejections() {
    run_all(&[
        "chunk-start-bad-name".into(),
        "chunk-start-huge-length".into(),
        "chunk-data-more-than-declared".into(),
        "chunk-data-null".into(),
        "chunk-end-without-start".into(),
    ]);
}

#[test]
fn error_dispatcher_paths() {
    run_all(&[
        "png_error-direct".into(),
        "png_error-null-message".into(),
        "png_error-long-message".into(),
        "png_warning-direct".into(),
        "png_warning-null".into(),
        "png_warning-long".into(),
        "png_benign_error-write".into(),
        "png_benign_error-write-allowed".into(),
        "png_benign_error-read".into(),
        "png_chunk_error-write".into(),
        "png_chunk_warning-write".into(),
        "png_chunk_benign_error-read".into(),
        "png_warning-null-struct".into(),
        "set_longjmp-null-fn".into(),
        "set_longjmp-zero-size".into(),
        "set_longjmp-null-struct".into(),
    ]);
}

#[test]
fn memory_rejections() {
    let mut cases = Vec::new();
    for n in [
        0usize,
        1,
        16,
        1024,
        usize::MAX,
        usize::MAX / 2,
        usize::MAX - 1,
        0x7fff_ffff_ffff_ffff,
        1usize << 62,
    ] {
        cases.push(format!("malloc:{n}"));
    }
    cases.push("free-null".into());
    run_all(&cases);
}

#[test]
fn option_and_mng_rejections() {
    let mut cases = Vec::new();
    for v in [0u32, 1, 2, 3, 4, 5, 6, 0xff, 0xffff_ffff] {
        cases.push(format!("mng:{v}"));
    }
    for o in [-2i32, -1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 100] {
        for on in [-1i32, 0, 1, 2, 3, 4, 255] {
            cases.push(format!("option:{o},{on}"));
        }
    }
    run_all(&cases);
}

/// `png_longjmp` with no `jmp_buf` installed reaches `PNG_ABORT()` (pngerror.c),
/// which raises SIGABRT.  That IS the documented rejection, so the two libraries
/// must abort identically -- compared here rather than through `diff_case`,
/// which requires a transcript.
#[test]
fn png_longjmp_without_jmpbuf_aborts_identically() {
    // Two rows whose documented outcome is `PNG_ABORT()`:
    //   * png_longjmp with png_ptr->longjmp_fn == NULL   (pngerror.c)
    //   * png_error with png_ptr == NULL                 (pngerror.c png_err)
    for case in ["png_longjmp-no-jmpbuf", "png_error-null-struct"] {
        let c = run_child(case, "c");
        let r = run_child(case, "rs");
        assert_eq!(
            c, r,
            "{case} must behave identically\n C={c:?}\n R={r:?}"
        );
        assert_eq!(c.signal, Some(6), "expected PNG_ABORT() -> SIGABRT for {case}, got {c:?}");
    }
}

/// Self-check: prove the sub-process mechanism really observes fatal errors and
/// distinct messages, so the differential comparison cannot pass vacuously.
#[test]
fn subprocess_self_check() {
    let t = run_child("png_error-direct", "c");
    assert_eq!(
        t.exit,
        Some(70),
        "png_error must reach the error handler and exit(70); got {t:?}"
    );
    assert!(
        t.lines.iter().any(|l| l == "ERROR:deliberate error"),
        "expected the exact message in the transcript, got {:?}",
        t.lines
    );
    let r = run_child("png_error-direct", "rs");
    assert_eq!(t, r, "C and Rust transcripts differ for png_error");

    let t = run_child("png_warning-direct", "c");
    assert_eq!(t.exit, Some(0), "png_warning must not be fatal: {t:?}");
    assert!(
        t.lines.iter().any(|l| l == "WARN:deliberate warning"),
        "expected the warning in the transcript, got {:?}",
        t.lines
    );

    // and a genuinely different message must be distinguishable
    let a = run_child("ihdr:0,2,8,2,0,0,0", "c");
    let b = run_child("ihdr:4,2,3,0,0,0,0", "c");
    assert_ne!(
        a.lines, b.lines,
        "two different IHDR rejections produced identical transcripts"
    );
    assert!(!a.lines.is_empty() && !b.lines.is_empty());
    eprintln!("zero-width IHDR transcript: {:?}", a.lines);
    eprintln!("bad-depth IHDR transcript:  {:?}", b.lines);
}
