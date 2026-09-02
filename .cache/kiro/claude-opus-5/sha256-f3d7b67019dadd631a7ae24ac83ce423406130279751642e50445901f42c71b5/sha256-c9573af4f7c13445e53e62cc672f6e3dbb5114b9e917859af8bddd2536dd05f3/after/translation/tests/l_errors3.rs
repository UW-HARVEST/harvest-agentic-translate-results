//! Phase C round 3: the last group of reachable rejection branches — the ones
//! that need raised user limits, real stdio streams, progressive decoding of
//! damaged IDAT data, ragged chunk payloads, or the deprecated
//! `png_malloc_default` entry point.
mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void, CString};
use std::ptr;

const SEED: u64 = 0xE773_0003_0004_0005;

extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fclose(f: *mut c_void) -> c_int;
}

fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (i, t) in table.iter_mut().enumerate() {
        let mut cc = i as u32;
        for _ in 0..8 {
            cc = if cc & 1 != 0 { 0xedb8_8320 ^ (cc >> 1) } else { cc >> 1 };
        }
        *t = cc;
    }
    let mut cc = 0xffff_ffffu32;
    for &b in data {
        cc = table[((cc ^ b as u32) & 0xff) as usize] ^ (cc >> 8);
    }
    cc ^ 0xffff_ffff
}

fn make_chunk(ty: &[u8], data: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&(data.len() as u32).to_be_bytes());
    v.extend_from_slice(&ty[..4]);
    v.extend_from_slice(data);
    let mut ci = ty[..4].to_vec();
    ci.extend_from_slice(data);
    v.extend_from_slice(&crc32(&ci).to_be_bytes());
    v
}

fn chunks(s: &[u8]) -> Vec<(usize, usize, [u8; 4])> {
    let mut out = Vec::new();
    let mut i = 8usize;
    while i + 12 <= s.len() {
        let len = u32::from_be_bytes([s[i], s[i + 1], s[i + 2], s[i + 3]]) as usize;
        if i + 12 + len > s.len() {
            break;
        }
        out.push((i, len, [s[i + 4], s[i + 5], s[i + 6], s[i + 7]]));
        i += 12 + len;
    }
    out
}

fn fix_crc(s: &mut [u8], off: usize, len: usize) {
    let ci = s[off + 4..off + 8 + len].to_vec();
    let v = crc32(&ci).to_be_bytes();
    s[off + 8 + len..off + 12 + len].copy_from_slice(&v);
}

fn gen(cl: &Lib, w: u32, h: u32, ct: c_int, bd: c_int, il: c_int, npal: usize) -> Vec<u8> {
    let pal = if ct == PNG_COLOR_TYPE_PALETTE {
        make_palette(npal, SEED ^ 7)
    } else {
        vec![]
    };
    write_full(
        cl,
        w,
        h,
        ct,
        bd,
        il,
        PNG_FILTER_TYPE_BASE,
        &pal,
        rowbytes(w, bd, ct),
        SEED ^ ((ct as u64) << 8) ^ bd as u64,
        &mut no_setup,
    )
    .out
}

fn try_read(l: &Lib, stream: Vec<u8>) -> Report {
    read_session(l, stream, &mut |l, png, info| unsafe {
        (l.api.png_read_info)(png, info);
        let h = (l.api.png_get_image_height)(png, info);
        let il = (l.api.png_get_interlace_type)(png, info);
        let passes = if il == 1 {
            (l.api.png_set_interlace_handling)(png)
        } else {
            1
        };
        (l.api.png_read_update_info)(png, info);
        let rb = (l.api.png_get_rowbytes)(png, info);
        let mut buf = vec![0u8; rb + 16];
        for _ in 0..passes {
            for _ in 0..h {
                (l.api.png_read_row)(png, buf.as_mut_ptr(), ptr::null_mut());
            }
        }
        (l.api.png_read_end)(png, info);
        log("read_end ok".to_string());
    })
}

unsafe extern "C" fn e_info_cb(_png: *mut c_void, _info: *mut c_void) {
    log("e info");
}
unsafe extern "C" fn e_row_cb(_png: *mut c_void, _row: *mut u8, n: u32, p: c_int) {
    log(format!("e row {n} {p}"));
}
unsafe extern "C" fn e_end_cb(_png: *mut c_void, _info: *mut c_void) {
    log("e end");
}

fn push_read(l: &Lib, stream: &[u8], gran: usize) -> Report {
    read_session(l, vec![], &mut |l, png, info| unsafe {
        (l.api.png_set_progressive_read_fn)(
            png,
            ptr::null_mut(),
            e_info_cb as *mut c_void,
            e_row_cb as *mut c_void,
            e_end_cb as *mut c_void,
        );
        let mut pos = 0usize;
        while pos < stream.len() {
            let n = if gran == 0 {
                stream.len() - pos
            } else {
                gran.min(stream.len() - pos)
            };
            (l.api.png_process_data)(png, info, stream[pos..].as_ptr() as *mut u8, n);
            pos += n;
        }
        log("push done".to_string());
    })
}

// ===========================================================================
// E-1  png_check_IHDR limits that need the user limits raised first
// ===========================================================================
#[test]
fn e1_check_ihdr_architecture_limits() {
    let (c, r) = libs();
    let sizes: &[u32] = &[
        1,
        1_000_000,
        1_000_001,
        0x1fff_ffcf,
        0x1fff_ffd0,
        0x1fff_ffd1,
        0x2000_0000,
        0x7fff_ffff,
        0x8000_0000,
        0xffff_ffff,
    ];
    for &v in sizes {
        for which in 0..2u32 {
            let mut run = |l: &Lib| -> Report {
                read_session(l, vec![], &mut |l, png, _info| unsafe {
                    (l.api.png_set_user_limits)(png, 0x7fff_ffff, 0x7fff_ffff);
                    let (w, h) = if which == 0 { (v, 4) } else { (4, v) };
                    (l.pv.png_check_IHDR)(
                        png,
                        w,
                        h,
                        8,
                        PNG_COLOR_TYPE_RGB,
                        PNG_INTERLACE_NONE,
                        PNG_COMPRESSION_TYPE_BASE,
                        PNG_FILTER_TYPE_BASE,
                    );
                    log("check_IHDR returned".to_string());
                })
            };
            diff(
                &format!("E1 png_check_IHDR {} = {v:#x}", if which == 0 { "width" } else { "height" }),
                &c,
                &r,
                &mut run,
            );
        }
    }
}

// ===========================================================================
// E-2  Compression buffer size beyond the zlib maximum
// ===========================================================================
#[test]
fn e2_compression_buffer_limits() {
    let (c, r) = libs();
    for size in [
        1usize,
        0xffff_fffeusize,
        0xffff_ffff,
        0x1_0000_0000,
        0x1_0000_0001,
        usize::MAX,
        usize::MAX / 2,
    ] {
        let mut run = |l: &Lib| -> Report {
            write_session(l, &mut |l, png, _info| unsafe {
                (l.api.png_set_compression_buffer_size)(png, size);
                log(format!(
                    "buffer_size={}",
                    (l.api.png_get_compression_buffer_size)(png)
                ));
            })
        };
        diff(&format!("E2 compression_buffer_size={size:#x}"), &c, &r, &mut run);
    }
}

// ===========================================================================
// E-3  png_write_chunk with a length beyond the PNG maximum
// ===========================================================================
#[test]
fn e3_chunk_length_maximum() {
    let (c, r) = libs();
    // NOTE: 0x7fffffff is PNG_UINT_31_MAX and is ACCEPTED, so libpng then reads
    // 2 GiB from the data pointer.  Only lengths ABOVE the maximum are rejected
    // before the data is touched, so only those are comparable here.
    for len in [
        0x8000_0000usize,
        0x8000_0001,
        0xffff_ffff,
        0x1_0000_0000,
        usize::MAX,
    ] {
        // png_write_complete_chunk rejects the length BEFORE touching the data,
        // so a small buffer is safe here.
        let data = vec![0u8; 16];
        let mut run = |l: &Lib| -> Report {
            write_session(l, &mut |l, png, _info| unsafe {
                (l.api.png_write_sig)(png);
                (l.api.png_write_chunk)(png, b"prVt".as_ptr(), data.as_ptr(), len);
                log("write_chunk returned".to_string());
            })
        };
        diff(&format!("E3 png_write_chunk length={len:#x}"), &c, &r, &mut run);
    }
}

// ===========================================================================
// E-4  png_write_iCCP profile-length validation
// ===========================================================================
#[test]
fn e4_write_iccp_lengths() {
    let (c, r) = libs();
    let mut prof = vec![0u8; 200];
    prof[4..8].copy_from_slice(b"ADBE");
    prof[8..12].copy_from_slice(&0x0200_0000u32.to_be_bytes());
    prof[12..16].copy_from_slice(b"mntr");
    prof[16..20].copy_from_slice(b"RGB ");
    prof[20..24].copy_from_slice(b"XYZ ");
    prof[36..40].copy_from_slice(b"acsp");
    prof[68..72].copy_from_slice(&0x0000_f6d6u32.to_be_bytes());
    prof[72..76].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    prof[76..80].copy_from_slice(&0x0000_d32du32.to_be_bytes());
    let name = CString::new("prof").unwrap();
    // `embedded` is the length stored INSIDE the profile; `plen` is the length
    // passed to the API.  The C checks that plen is a multiple of 4, that it is
    // at least 132, and that it matches the embedded value.
    for embedded in [0u32, 131, 132, 133, 136, 140, 200] {
        for plen in [0u32, 131, 132, 133, 136, 140, 200] {
            let mut p = prof.clone();
            p[0..4].copy_from_slice(&embedded.to_be_bytes());
            let mut run = |l: &Lib| -> Report {
                write_session(l, &mut |l, png, info| unsafe {
                    (l.api.png_set_IHDR)(
                        png, info, 4, 4, 8, PNG_COLOR_TYPE_RGB, PNG_INTERLACE_NONE,
                        PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
                    );
                    (l.api.png_write_info)(png, info);
                    (l.pv.png_write_iCCP)(png, name.as_ptr(), p.as_ptr(), plen);
                    log("write_iCCP returned".to_string());
                })
            };
            diff(
                &format!("E4 png_write_iCCP embedded={embedded} plen={plen}"),
                &c,
                &r,
                &mut run,
            );
        }
    }
}

// ===========================================================================
// E-5  Ragged / oversized sPLT payloads
// ===========================================================================
#[test]
fn e5_splt_payload_lengths() {
    let (c, r) = libs();
    let base = gen(&c, 8, 4, PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE, 0);
    let idat = chunks(&base).iter().find(|c| &c.2 == b"IDAT").unwrap().0;
    for depth in [8u8, 16] {
        let esz = if depth == 8 { 6usize } else { 10 };
        // lengths that are NOT a multiple of the entry size
        for extra in [1usize, 2, 3, 5, 7, 9] {
            let mut p = b"name\0".to_vec();
            p.push(depth);
            p.extend_from_slice(&vec![0x5au8; esz * 2 + extra]);
            let mut s = base[..idat].to_vec();
            s.extend_from_slice(&make_chunk(b"sPLT", &p));
            s.extend_from_slice(&base[idat..]);
            let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
            diff(&format!("E5 sPLT depth={depth} ragged +{extra}"), &c, &r, &mut run);
        }
        // no name terminator at all
        let mut p = b"noterminator".to_vec();
        p.push(depth);
        let mut s = base[..idat].to_vec();
        s.extend_from_slice(&make_chunk(b"sPLT", &p));
        s.extend_from_slice(&base[idat..]);
        let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
        diff(&format!("E5 sPLT depth={depth} no NUL"), &c, &r, &mut run);
        // an entry count above PNG_MAX_PALETTE_LENGTH, and a very long chunk
        for nent in [257usize, 1000, 20000] {
            let mut p = b"n\0".to_vec();
            p.push(depth);
            p.extend_from_slice(&vec![0x33u8; esz * nent]);
            let mut s = base[..idat].to_vec();
            s.extend_from_slice(&make_chunk(b"sPLT", &p));
            s.extend_from_slice(&base[idat..]);
            for mal in [0usize, 1000, 1 << 20] {
                let mut run = |l: &Lib| -> Report {
                    read_session(l, s.clone(), &mut |l, png, info| unsafe {
                        (l.api.png_set_chunk_malloc_max)(png, mal);
                        (l.api.png_read_info)(png, info);
                        let mut e: *mut c_void = ptr::null_mut();
                        log(format!("sPLT={}", (l.api.png_get_sPLT)(png, info, &mut e)));
                        (l.api.png_read_end)(png, info);
                    })
                };
                diff(
                    &format!("E5 sPLT depth={depth} nent={nent} mal={mal}"),
                    &c,
                    &r,
                    &mut run,
                );
            }
        }
    }
}

// ===========================================================================
// E-6  bKGD palette index out of range (needs a SMALL palette)
// ===========================================================================
#[test]
fn e6_bkgd_invalid_index() {
    let (c, r) = libs();
    for (bd, npal) in [(1i32, 2usize), (2, 4), (4, 16), (8, 32)] {
        let base = gen(&c, 8, 4, PNG_COLOR_TYPE_PALETTE, bd, PNG_INTERLACE_NONE, npal);
        let idat = chunks(&base).iter().find(|c| &c.2 == b"IDAT").unwrap().0;
        for idx in [0u8, 1, 2, 15, 16, 31, 32, 200, 255] {
            let mut s = base[..idat].to_vec();
            s.extend_from_slice(&make_chunk(b"bKGD", &[idx]));
            s.extend_from_slice(&base[idat..]);
            let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
            diff(&format!("E6 bKGD idx={idx} bd={bd} npal={npal}"), &c, &r, &mut run);
        }
    }
}

// ===========================================================================
// E-7  Progressive decoding of damaged / truncated IDAT data
// ===========================================================================
#[test]
fn e7_progressive_idat_damage() {
    let (c, r) = libs();
    let good = gen(&c, 24, 8, PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE, 0);
    let cs = chunks(&good);
    let (off, len, _) = *cs.iter().find(|c| &c.2 == b"IDAT").unwrap();
    // flip bytes in the compressed data
    for i in 0..len.min(16) {
        let mut s = good.clone();
        s[off + 8 + i] ^= 0xff;
        fix_crc(&mut s, off, len);
        for gran in [1usize, 0] {
            let mut run = |l: &Lib| -> Report { push_read(l, &s, gran) };
            diff(&format!("E7 progressive IDAT byte {i} gran={gran}"), &c, &r, &mut run);
        }
    }
    // truncate the IDAT payload (the stream then ends early)
    for keep in [0usize, 1, 2, len / 2, len - 1] {
        let mut s = good[..off].to_vec();
        s.extend_from_slice(&make_chunk(b"IDAT", &good[off + 8..off + 8 + keep]));
        s.extend_from_slice(&good[off + 12 + len..]);
        for gran in [1usize, 0] {
            let mut run = |l: &Lib| -> Report { push_read(l, &s, gran) };
            diff(
                &format!("E7 progressive IDAT truncated to {keep} gran={gran}"),
                &c,
                &r,
                &mut run,
            );
        }
    }
    // extra compressed data after the image is complete
    for extra in [1usize, 8, 64] {
        let mut payload = good[off + 8..off + 8 + len].to_vec();
        payload.extend_from_slice(&vec![0x5au8; extra]);
        let mut s = good[..off].to_vec();
        s.extend_from_slice(&make_chunk(b"IDAT", &payload));
        s.extend_from_slice(&good[off + 12 + len..]);
        for gran in [1usize, 0] {
            let mut run = |l: &Lib| -> Report { push_read(l, &s, gran) };
            diff(&format!("E7 extra IDAT data +{extra} gran={gran}"), &c, &r, &mut run);
        }
        let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
        diff(&format!("E7 extra IDAT data +{extra} sequential"), &c, &r, &mut run);
    }
    // the whole stream truncated at every chunk boundary, progressively
    for &(o, l2, ty) in &cs {
        let s = good[..o + 8].to_vec();
        let mut run = |l: &Lib| -> Report { push_read(l, &s, 1) };
        diff(
            &format!("E7 progressive truncated before {:?} data", String::from_utf8_lossy(&ty)),
            &c,
            &r,
            &mut run,
        );
        let s = good[..o + 12 + l2 - 1].to_vec();
        let mut run = |l: &Lib| -> Report { push_read(l, &s, 1) };
        diff(
            &format!("E7 progressive truncated inside {:?} crc", String::from_utf8_lossy(&ty)),
            &c,
            &r,
            &mut run,
        );
    }
    // interlaced variants
    let inter = gen(&c, 17, 9, PNG_COLOR_TYPE_RGB_ALPHA, 8, PNG_INTERLACE_ADAM7, 0);
    let ics = chunks(&inter);
    let (ioff, ilen, _) = *ics.iter().find(|c| &c.2 == b"IDAT").unwrap();
    for i in 0..ilen.min(8) {
        let mut s = inter.clone();
        s[ioff + 8 + i] ^= 0xff;
        fix_crc(&mut s, ioff, ilen);
        let mut run = |l: &Lib| -> Report { push_read(l, &s, 3) };
        diff(&format!("E7 interlaced progressive IDAT byte {i}"), &c, &r, &mut run);
    }
}

// ===========================================================================
// E-8  Palette image whose PLTE is diverted to the unknown-chunk list
// ===========================================================================
#[test]
fn e8_palette_without_plte() {
    let (c, r) = libs();
    for bd in [1i32, 2, 4, 8] {
        let npal = match bd {
            1 => 2,
            2 => 4,
            4 => 16,
            _ => 256,
        };
        let stream = gen(&c, 8, 4, PNG_COLOR_TYPE_PALETTE, bd, PNG_INTERLACE_NONE, npal);
        for keep in [0i32, 1, 2, 3] {
            let mut run = |l: &Lib| -> Report {
                read_session(l, stream.clone(), &mut |l, png, info| unsafe {
                    let list = b"PLTE\0".to_vec();
                    (l.api.png_set_keep_unknown_chunks)(png, keep, list.as_ptr(), 1);
                    (l.api.png_read_info)(png, info);
                    log(format!(
                        "PLTE valid={}",
                        (l.api.png_get_valid)(png, info, PNG_INFO_PLTE)
                    ));
                    (l.api.png_read_update_info)(png, info);
                    let h = (l.api.png_get_image_height)(png, info);
                    let rb = (l.api.png_get_rowbytes)(png, info);
                    let mut buf = vec![0u8; rb + 16];
                    for _ in 0..h {
                        (l.api.png_read_row)(png, buf.as_mut_ptr(), ptr::null_mut());
                    }
                    (l.api.png_read_end)(png, info);
                    log("done".to_string());
                })
            };
            diff(&format!("E8 palette PLTE-as-unknown keep={keep} bd={bd}"), &c, &r, &mut run);
        }
        // and with the expansion transforms requested
        let mut run = |l: &Lib| -> Report {
            read_session(l, stream.clone(), &mut |l, png, info| unsafe {
                let list = b"PLTE\0".to_vec();
                (l.api.png_set_keep_unknown_chunks)(png, PNG_HANDLE_CHUNK_ALWAYS, list.as_ptr(), 1);
                (l.api.png_read_info)(png, info);
                (l.api.png_set_palette_to_rgb)(png);
                (l.api.png_read_update_info)(png, info);
                log("update_info done".to_string());
            })
        };
        diff(&format!("E8 palette_to_rgb without PLTE bd={bd}"), &c, &r, &mut run);
    }
}

// ===========================================================================
// E-9  Real stdio streams: png_default_read_data / png_default_write_data
// ===========================================================================
#[test]
fn e9_stdio_read_write_errors() {
    let (c, r) = libs();
    // Reading from /dev/null: fread returns 0 -> "Read Error"
    let mut run = |l: &Lib| -> Report {
        let mut ctxb = Box::new(Ctx::default());
        set_ctx(&mut *ctxb as *mut Ctx);
        unsafe {
            let path = CString::new("/dev/null").unwrap();
            let mode = CString::new("rb").unwrap();
            let f = fopen(path.as_ptr(), mode.as_ptr());
            assert!(!f.is_null(), "cannot open /dev/null");
            let png = (l.api.png_create_read_struct)(
                ver(),
                ptr::null_mut(),
                cb_error as *mut c_void,
                cb_warn as *mut c_void,
            );
            let info = (l.api.png_create_info_struct)(png);
            (l.api.png_init_io)(png, f);
            protect(&l.api, png, &mut || {
                (l.api.png_read_info)(png, info);
                log("read_info from /dev/null returned".to_string());
            });
            let mut pp = png;
            let mut ip = info;
            (l.api.png_destroy_read_struct)(&mut pp, &mut ip, ptr::null_mut());
            fclose(f);
        }
        let rep = ctxb.digest();
        set_ctx(ptr::null_mut());
        rep
    };
    diff("E9 png_default_read_data at EOF", &c, &r, &mut run);

    // Writing to /dev/full: fwrite fails -> "Write Error"
    if std::path::Path::new("/dev/full").exists() {
        let mut run = |l: &Lib| -> Report {
            let mut ctxb = Box::new(Ctx::default());
            set_ctx(&mut *ctxb as *mut Ctx);
            unsafe {
                let path = CString::new("/dev/full").unwrap();
                let mode = CString::new("wb").unwrap();
                let f = fopen(path.as_ptr(), mode.as_ptr());
                assert!(!f.is_null(), "cannot open /dev/full");
                let png = (l.api.png_create_write_struct)(
                    ver(),
                    ptr::null_mut(),
                    cb_error as *mut c_void,
                    cb_warn as *mut c_void,
                );
                let info = (l.api.png_create_info_struct)(png);
                (l.api.png_init_io)(png, f);
                protect(&l.api, png, &mut || {
                    (l.api.png_set_IHDR)(
                        png,
                        info,
                        64,
                        64,
                        8,
                        PNG_COLOR_TYPE_RGB,
                        PNG_INTERLACE_NONE,
                        PNG_COMPRESSION_TYPE_BASE,
                        PNG_FILTER_TYPE_BASE,
                    );
                    (l.api.png_write_info)(png, info);
                    let rows = make_rows(64, 64 * 3, SEED);
                    for row in &rows {
                        (l.api.png_write_row)(png, row.as_ptr());
                    }
                    (l.api.png_write_end)(png, info);
                    log("write to /dev/full returned".to_string());
                });
                let mut pp = png;
                let mut ip = info;
                (l.api.png_destroy_write_struct)(&mut pp, &mut ip);
                fclose(f);
            }
            let rep = ctxb.digest();
            set_ctx(ptr::null_mut());
            rep
        };
        diff("E9 png_default_write_data on a full device", &c, &r, &mut run);
    }

    // png_init_io with a valid file, plus png_write_flush through stdio
    let mut run = |l: &Lib| -> Report {
        let mut ctxb = Box::new(Ctx::default());
        set_ctx(&mut *ctxb as *mut Ctx);
        unsafe {
            let path = CString::new("/dev/null").unwrap();
            let mode = CString::new("wb").unwrap();
            let f = fopen(path.as_ptr(), mode.as_ptr());
            let png = (l.api.png_create_write_struct)(
                ver(),
                ptr::null_mut(),
                cb_error as *mut c_void,
                cb_warn as *mut c_void,
            );
            let info = (l.api.png_create_info_struct)(png);
            (l.api.png_init_io)(png, f);
            protect(&l.api, png, &mut || {
                (l.api.png_set_IHDR)(
                    png, info, 4, 4, 8, PNG_COLOR_TYPE_RGB, PNG_INTERLACE_NONE,
                    PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
                );
                (l.api.png_set_flush)(png, 1);
                (l.api.png_write_info)(png, info);
                let rows = make_rows(4, 12, SEED);
                for row in &rows {
                    (l.api.png_write_row)(png, row.as_ptr());
                }
                (l.api.png_write_flush)(png);
                (l.api.png_write_end)(png, info);
                log("stdio write ok".to_string());
            });
            let mut pp = png;
            let mut ip = info;
            (l.api.png_destroy_write_struct)(&mut pp, &mut ip);
            fclose(f);
        }
        let rep = ctxb.digest();
        set_ctx(ptr::null_mut());
        rep
    };
    diff("E9 png_init_io write path", &c, &r, &mut run);

    // The simplified stdio entry points on a non-PNG file
    let mut run = |l: &Lib| -> Report {
        let mut ctxb = Box::new(Ctx::default());
        set_ctx(&mut *ctxb as *mut Ctx);
        unsafe {
            let mut im = PngImage::default();
            let path = CString::new("/dev/null").unwrap();
            let ok = (l.api.png_image_begin_read_from_file)(&mut im, path.as_ptr() as *mut c_char);
            log_img(&format!("begin_read_from_file={ok}"), &im);
            (l.api.png_image_free)(&mut im);
            let missing = CString::new("/nonexistent/path/xyz.png").unwrap();
            let mut im2 = PngImage::default();
            let ok2 =
                (l.api.png_image_begin_read_from_file)(&mut im2, missing.as_ptr() as *mut c_char);
            log_img(&format!("begin_read_missing={ok2}"), &im2);
            (l.api.png_image_free)(&mut im2);
        }
        let rep = ctxb.digest();
        set_ctx(ptr::null_mut());
        rep
    };
    diff("E9 png_image_begin_read_from_file", &c, &r, &mut run);
}

// ===========================================================================
// E-10  png_malloc_default / png_free_default
// ===========================================================================
#[test]
fn e10_malloc_default() {
    let (c, r) = libs();
    for size in [0usize, 1, 1000, usize::MAX, usize::MAX / 2, 1 << 46] {
        let mut run = |l: &Lib| -> Report {
            write_session(l, &mut |l, png, _info| unsafe {
                let p = (l.api.png_malloc_default)(png, size);
                log(format!("malloc_default({size}) null={}", p.is_null()));
                if !p.is_null() {
                    (l.api.png_free_default)(png, p);
                }
            })
        };
        diff(&format!("E10 png_malloc_default size={size}"), &c, &r, &mut run);
    }
    let mut run = |l: &Lib| -> Report {
        write_session(l, &mut |l, png, _info| unsafe {
            (l.api.png_free_default)(png, ptr::null_mut());
            log(format!("mem_ptr={:?}", (l.api.png_get_mem_ptr)(png)));
        })
    };
    diff("E10 png_free_default(NULL)", &c, &r, &mut run);
}

// ===========================================================================
// E-11  png_set_unknown_chunk_location with a VALID descriptor
// ===========================================================================
#[test]
fn e11_unknown_chunk_location() {
    let (c, r) = libs();
    for loc in [-1i32, 0, 1, 2, 3, 4, 7, 8, 9, 0x0b, 0x10, 0xff] {
        for idx in [-1i32, 0, 1, 2] {
            let payload = [1u8, 2, 3];
            let mut run = |l: &Lib| -> Report {
                write_session(l, &mut |l, png, info| unsafe {
                    let unk = [PngUnknownChunk {
                        name: *b"prVt\0",
                        data: payload.as_ptr() as *mut u8,
                        size: 3,
                        // a VALID location so png_set_unknown_chunks succeeds
                        location: PNG_HAVE_IHDR as u8,
                    }];
                    (l.api.png_set_unknown_chunks)(png, info, unk.as_ptr(), 1);
                    (l.api.png_set_unknown_chunk_location)(png, info, idx, loc);
                    log("location set".to_string());
                })
            };
            diff(
                &format!("E11 set_unknown_chunk_location idx={idx} loc={loc}"),
                &c,
                &r,
                &mut run,
            );
        }
    }
}

// ===========================================================================
// E-12  Simplified read from memory truncated INSIDE the image data
// ===========================================================================
#[test]
fn e12_simplified_memory_truncation() {
    let (c, r) = libs();
    let good = gen(&c, 32, 16, PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE, 0);
    let cs = chunks(&good);
    let (off, len, _) = *cs.iter().find(|c| &c.2 == b"IDAT").unwrap();
    let mut cut_points: Vec<usize> = vec![
        off + 8,
        off + 9,
        off + 8 + len / 4,
        off + 8 + len / 2,
        off + 8 + len - 1,
        off + 12 + len,
        good.len() - 1,
        good.len(),
    ];
    cut_points.sort_unstable();
    cut_points.dedup();
    for n in cut_points {
        let mut run = |l: &Lib| -> Report {
            let mut ctxb = Box::new(Ctx::default());
            set_ctx(&mut *ctxb as *mut Ctx);
            unsafe {
                let mut im = PngImage::default();
                let ok = (l.api.png_image_begin_read_from_memory)(
                    &mut im,
                    good.as_ptr() as *const c_void,
                    n,
                );
                log_img(&format!("begin({n})={ok}"), &im);
                if ok != 0 {
                    let sz = (im.width as usize) * (im.height as usize) * 4 + 64;
                    let mut buf = vec![0u8; sz];
                    let ok2 = (l.api.png_image_finish_read)(
                        &mut im,
                        ptr::null(),
                        buf.as_mut_ptr() as *mut c_void,
                        0,
                        ptr::null_mut(),
                    );
                    log_img(&format!("finish={ok2}"), &im);
                }
                (l.api.png_image_free)(&mut im);
            }
            let rep = ctxb.digest();
            set_ctx(ptr::null_mut());
            rep
        };
        diff(&format!("E12 simplified read truncated at {n}"), &c, &r, &mut run);
    }
}

// ===========================================================================
// E-13  Simplified read into every colour-mapped output format, from every
//       source shape, with and without a background
// ===========================================================================
#[test]
fn e13_simplified_colormap_matrix() {
    let (c, r) = libs();
    let shapes: &[(c_int, c_int)] = &[
        (0, 1),
        (0, 2),
        (0, 4),
        (0, 8),
        (0, 16),
        (3, 1),
        (3, 4),
        (3, 8),
        (2, 8),
        (2, 16),
        (4, 8),
        (4, 16),
        (6, 8),
        (6, 16),
    ];
    for &(ct, bd) in shapes {
        for with_trns in [false, true] {
            let npal = match bd {
                1 => 2,
                2 => 4,
                4 => 16,
                _ => 256,
            };
            let pal = if ct == PNG_COLOR_TYPE_PALETTE {
                make_palette(npal, SEED ^ 5)
            } else {
                vec![]
            };
            let stream = write_full(
                &c,
                8,
                4,
                ct,
                bd,
                PNG_INTERLACE_NONE,
                PNG_FILTER_TYPE_BASE,
                &pal,
                rowbytes(8, bd, ct),
                SEED ^ 0x13,
                &mut |l, png, info| unsafe {
                    if with_trns {
                        if ct == PNG_COLOR_TYPE_PALETTE {
                            let alpha: Vec<u8> = (0..npal).map(|i| (i as u8) ^ 0x0f).collect();
                            (l.api.png_set_tRNS)(png, info, alpha.as_ptr(), npal as c_int, ptr::null());
                        } else if ct == PNG_COLOR_TYPE_GRAY || ct == PNG_COLOR_TYPE_RGB {
                            let maxv = ((1u32 << bd) - 1) as u16;
                            let tc = PngColor16 {
                                index: 0,
                                red: maxv / 2,
                                green: maxv / 3,
                                blue: maxv / 4,
                                gray: maxv / 2,
                            };
                            (l.api.png_set_tRNS)(png, info, ptr::null(), 0, &tc);
                        }
                    }
                },
            )
            .out;
            for fmt in 0u32..0x40 {
                if fmt & PNG_FORMAT_FLAG_COLORMAP == 0 {
                    continue;
                }
                for bg in [false, true] {
                    let mut run = |l: &Lib| -> Report {
                        let mut ctxb = Box::new(Ctx::default());
                        set_ctx(&mut *ctxb as *mut Ctx);
                        unsafe {
                            let mut im = PngImage::default();
                            let ok = (l.api.png_image_begin_read_from_memory)(
                                &mut im,
                                stream.as_ptr() as *const c_void,
                                stream.len(),
                            );
                            log_img(&format!("begin={ok}"), &im);
                            if ok != 0 {
                                im.format = fmt;
                                let mut buf = vec![0u8; 1 << 14];
                                let mut cmap = vec![0u8; 1 << 13];
                                let bgc = PngColor { red: 9, green: 8, blue: 7 };
                                let ok2 = (l.api.png_image_finish_read)(
                                    &mut im,
                                    if bg { &bgc } else { ptr::null() },
                                    buf.as_mut_ptr() as *mut c_void,
                                    0,
                                    cmap.as_mut_ptr() as *mut c_void,
                                );
                                log_img(&format!("finish={ok2}"), &im);
                                if ok2 != 0 {
                                    let n = (im.colormap_entries as usize)
                                        * (((fmt & 3) + 1) as usize)
                                        * if fmt & PNG_FORMAT_FLAG_LINEAR != 0 { 2 } else { 1 };
                                    log(format!("cmap={:02x?}", &cmap[..n.min(cmap.len())]));
                                    log(format!("pixels={:02x?}", &buf[..32]));
                                }
                            }
                            (l.api.png_image_free)(&mut im);
                        }
                        let rep = ctxb.digest();
                        set_ctx(ptr::null_mut());
                        rep
                    };
                    diff(
                        &format!("E13 cmap read ct={ct} bd={bd} trns={with_trns} fmt={fmt:#x} bg={bg}"),
                        &c,
                        &r,
                        &mut run,
                    );
                }
            }
        }
    }
}

// ===========================================================================
// E-14  png_write_iCCP: the "not a multiple of 4" branch additionally requires
//       profile[8] > 3 (the ICC major version), which the earlier
//       "Incorrect data in iCCP" check does not shadow.
// ===========================================================================
#[test]
fn e14_write_iccp_version_and_alignment() {
    let (c, r) = libs();
    let name = CString::new("prof").unwrap();
    for ver8 in [0u8, 1, 2, 3, 4, 5, 255] {
        for plen in [132u32, 133, 134, 135, 136, 139, 140, 200, 201] {
            let mut p = vec![0u8; plen.max(132) as usize];
            p[0..4].copy_from_slice(&plen.to_be_bytes());
            p[4..8].copy_from_slice(b"ADBE");
            p[8] = ver8;
            p[12..16].copy_from_slice(b"mntr");
            p[16..20].copy_from_slice(b"RGB ");
            p[20..24].copy_from_slice(b"XYZ ");
            p[36..40].copy_from_slice(b"acsp");
            p[68..72].copy_from_slice(&0x0000_f6d6u32.to_be_bytes());
            p[72..76].copy_from_slice(&0x0001_0000u32.to_be_bytes());
            p[76..80].copy_from_slice(&0x0000_d32du32.to_be_bytes());
            let mut run = |l: &Lib| -> Report {
                write_session(l, &mut |l, png, info| unsafe {
                    (l.api.png_set_IHDR)(
                        png, info, 4, 4, 8, PNG_COLOR_TYPE_RGB, PNG_INTERLACE_NONE,
                        PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
                    );
                    (l.api.png_write_info)(png, info);
                    (l.pv.png_write_iCCP)(png, name.as_ptr(), p.as_ptr(), plen);
                    log("write_iCCP returned".to_string());
                })
            };
            diff(
                &format!("E14 png_write_iCCP ver8={ver8} plen={plen}"),
                &c,
                &r,
                &mut run,
            );
        }
    }
}

// ===========================================================================
// E-15  Declared height that disagrees with the amount of image data, read
//       both sequentially and progressively.
// ===========================================================================
#[test]
fn e15_row_count_mismatch() {
    let (c, r) = libs();
    for &(ct, bd, il) in &[
        (2i32, 8i32, PNG_INTERLACE_NONE),
        (0, 1, PNG_INTERLACE_NONE),
        (6, 16, PNG_INTERLACE_NONE),
        (2, 8, PNG_INTERLACE_ADAM7),
        (3, 4, PNG_INTERLACE_ADAM7),
    ] {
        let real_h = 8u32;
        let npal = match bd {
            1 => 2,
            2 => 4,
            4 => 16,
            _ => 256,
        };
        let good = gen(&c, 16, real_h, ct, bd, il, npal);
        let cs = chunks(&good);
        let ihdr = cs.iter().find(|c| &c.2 == b"IHDR").unwrap();
        for h in [1u32, 2, 4, 7, 8, 9, 12, 16] {
            let mut s = good.clone();
            s[ihdr.0 + 12..ihdr.0 + 16].copy_from_slice(&h.to_be_bytes());
            fix_crc(&mut s, ihdr.0, ihdr.1);
            let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
            diff(
                &format!("E15 declared height={h} (real {real_h}) ct={ct} bd={bd} il={il} sequential"),
                &c,
                &r,
                &mut run,
            );
            for gran in [1usize, 0] {
                let mut run = |l: &Lib| -> Report { push_read(l, &s, gran) };
                diff(
                    &format!("E15 declared height={h} ct={ct} bd={bd} il={il} progressive gran={gran}"),
                    &c,
                    &r,
                    &mut run,
                );
            }
        }
        // and a width mismatch, which changes the bytes-per-row
        for w in [1u32, 8, 15, 16, 17, 32] {
            let mut s = good.clone();
            s[ihdr.0 + 8..ihdr.0 + 12].copy_from_slice(&w.to_be_bytes());
            fix_crc(&mut s, ihdr.0, ihdr.1);
            let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
            diff(
                &format!("E15 declared width={w} ct={ct} bd={bd} il={il} sequential"),
                &c,
                &r,
                &mut run,
            );
            let mut run = |l: &Lib| -> Report { push_read(l, &s, 1) };
            diff(
                &format!("E15 declared width={w} ct={ct} bd={bd} il={il} progressive"),
                &c,
                &r,
                &mut run,
            );
        }
    }
}

// ===========================================================================
// E-16  png_combine_row's "invalid user transform pixel depth": an interlaced
//       read where the application declares a user-transform pixel depth that is
//       >= 8 but not a multiple of 8.
// ===========================================================================
unsafe extern "C" fn e16_transform(_png: *mut c_void, row_info: *mut PngRowInfo, _data: *mut u8) {
    if !row_info.is_null() {
        log(format!("e16 ri={:?}", *row_info));
    }
}

#[test]
fn e16_user_transform_pixel_depth() {
    let (c, r) = libs();
    for &(ct, bd) in &[(2i32, 8i32), (0, 8), (6, 8), (0, 1), (3, 4), (6, 16)] {
        let npal = match bd {
            1 => 2,
            2 => 4,
            4 => 16,
            _ => 256,
        };
        for il in [PNG_INTERLACE_ADAM7, PNG_INTERLACE_NONE] {
            let stream = gen(&c, 17, 9, ct, bd, il, npal);
            // NOTE: only declared pixel depths up to 64 bits are used.  The C's
            // png_do_read_interlace default branch is
            //     png_byte v[8]; /* SAFE; pixel_depth does not exceed 64 */
            //     memcpy(v, sp, pixel_bytes);
            // with `pixel_bytes = pixel_depth >> 3`, so a declared depth above 64
            // bits memcpy()s more than 8 bytes onto an 8-byte stack array.  That
            // invariant is broken by png_set_user_transform_info, which accepts any
            // depth*channels up to 255*255, and the resulting stack overflow is
            // undefined behaviour in the reference implementation (verified: the C
            // .so corrupts its stack; whether it faults depends on the frame
            // layout).  The target branch, "invalid user transform pixel depth", is
            // still reached by e.g. depth 9 x 1 channel.
            for (d, ch) in [
                (9i32, 1i32),
                (1, 9),
                (3, 3),
                (5, 5),
                (7, 1),
                (8, 1),
                (8, 3),
                (16, 4),
                (0, 0),
                (1, 1),
                (17, 1),
                (33, 1),
                (7, 9),
                (64, 1),
                (2, 32),
            ] {
                let mut run = |l: &Lib| -> Report {
                    read_session(l, stream.clone(), &mut |l, png, info| unsafe {
                        (l.api.png_read_info)(png, info);
                        (l.api.png_set_read_user_transform_fn)(
                            png,
                            e16_transform as *mut c_void,
                        );
                        (l.api.png_set_user_transform_info)(png, ptr::null_mut(), d, ch);
                        let passes = if (l.api.png_get_interlace_type)(png, info) == 1 {
                            (l.api.png_set_interlace_handling)(png)
                        } else {
                            1
                        };
                        (l.api.png_read_update_info)(png, info);
                        let h = (l.api.png_get_image_height)(png, info);
                        let rb = (l.api.png_get_rowbytes)(png, info);
                        log(format!("rb={rb} passes={passes}"));
                        let mut buf = vec![0u8; rb + 4096];
                        for _ in 0..passes {
                            for _ in 0..h {
                                (l.api.png_read_row)(png, buf.as_mut_ptr(), ptr::null_mut());
                            }
                        }
                        (l.api.png_read_end)(png, info);
                        log("done".to_string());
                    })
                };
                diff(
                    &format!("E16 user transform depth={d} channels={ch} ct={ct} bd={bd} il={il}"),
                    &c,
                    &r,
                    &mut run,
                );
            }
        }
    }
}
