//! Phase C round 2: the rejection branches that are only reachable through the
//! LOW-LEVEL exported chunk writers, through crafted ancillary chunk payloads,
//! or through specific illegal state transitions.  Each `png_write_<chunk>`
//! validates its own arguments, and those validations are unreachable through
//! `png_set_<chunk>` + `png_write_info` because the setters reject first.
mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void, CString};
use std::ptr;

const SEED: u64 = 0xE772_0002_0003_0004;

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

fn gen(cl: &Lib, w: u32, h: u32, ct: c_int, bd: c_int, il: c_int) -> Vec<u8> {
    let pal = if ct == PNG_COLOR_TYPE_PALETTE {
        make_palette(
            match bd {
                1 => 2,
                2 => 4,
                4 => 16,
                _ => 256,
            },
            SEED ^ 7,
        )
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
        log(format!("last row={:02x?}", &buf[..rb]));
        (l.api.png_read_end)(png, info);
        log("read_end ok".to_string());
    })
}

/// A minimal write session that has already emitted the signature and IHDR, so
/// the low-level chunk writers operate in a plausible state.
fn low_write(l: &Lib, body: &mut dyn FnMut(&Lib, *mut c_void, *mut c_void)) -> Report {
    write_session(l, &mut |l, png, info| unsafe {
        (l.api.png_set_IHDR)(
            png,
            info,
            4,
            4,
            8,
            PNG_COLOR_TYPE_RGB,
            PNG_INTERLACE_NONE,
            PNG_COMPRESSION_TYPE_BASE,
            PNG_FILTER_TYPE_BASE,
        );
        (l.api.png_write_info)(png, info);
        body(l, png, info);
    })
}

// ===========================================================================
// D-1  png_write_IHDR argument validation (unreachable via png_set_IHDR)
// ===========================================================================
#[test]
fn d1_write_ihdr_validation() {
    let (c, r) = libs();
    let mut cases: Vec<(u32, u32, c_int, c_int, c_int, c_int, c_int)> = Vec::new();
    for bd in [0i32, 1, 2, 4, 8, 16, 3, 32] {
        for ct in [0i32, 2, 3, 4, 6, 1, 5, 7, 99] {
            cases.push((4, 4, bd, ct, 0, 0, 0));
        }
    }
    for cm in [0i32, 1, 99] {
        for fm in [0i32, 1, 64, 99] {
            for il in [0i32, 1, 2, 99] {
                cases.push((4, 4, 8, PNG_COLOR_TYPE_RGB, cm, fm, il));
            }
        }
    }
    cases.push((0, 4, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0));
    cases.push((4, 0, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0));
    cases.push((0x8000_0000, 4, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0));
    cases.push((0x7fff_ffff, 4, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0));
    for (i, &(w, h, bd, ct, cm, fm, il)) in cases.iter().enumerate() {
        let mut run = |l: &Lib| -> Report {
            write_session(l, &mut |l, png, _info| unsafe {
                (l.api.png_write_sig)(png);
                (l.pv.png_write_IHDR)(png, w, h, bd, ct, cm, fm, il);
                log("png_write_IHDR returned".to_string());
            })
        };
        diff(
            &format!("D1 png_write_IHDR #{i} {w}x{h} bd={bd} ct={ct} cm={cm} fm={fm} il={il}"),
            &c,
            &r,
            &mut run,
        );
    }
}

// ===========================================================================
// D-2  Low-level ancillary chunk writers with invalid arguments
// ===========================================================================
#[test]
fn d2_low_level_chunk_writers() {
    let (c, r) = libs();

    // PLTE with an illegal colour count, and on a grayscale image
    for n in [0u32, 1, 256, 257, 1000, 0xffff_ffff] {
        let pal = make_palette(256, 1);
        let mut run = |l: &Lib| -> Report {
            low_write(l, &mut |l, png, _info| unsafe {
                (l.pv.png_write_PLTE)(png, pal.as_ptr(), n);
                log("write_PLTE returned".to_string());
            })
        };
        diff(&format!("D2 png_write_PLTE n={n}"), &c, &r, &mut run);
    }
    // grayscale image + PLTE
    for n in [1u32, 4] {
        let pal = make_palette(4, 1);
        let mut run = |l: &Lib| -> Report {
            write_session(l, &mut |l, png, info| unsafe {
                (l.api.png_set_IHDR)(
                    png, info, 4, 4, 8, PNG_COLOR_TYPE_GRAY, PNG_INTERLACE_NONE,
                    PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
                );
                (l.api.png_write_info)(png, info);
                (l.pv.png_write_PLTE)(png, pal.as_ptr(), n);
                log("write_PLTE gray returned".to_string());
            })
        };
        diff(&format!("D2 png_write_PLTE gray n={n}"), &c, &r, &mut run);
    }

    // tRNS: invalid counts, and on an image that already has an alpha channel
    let alpha = vec![0x80u8; 300];
    let tc = PngColor16 { index: 0, red: 1, green: 2, blue: 3, gray: 4 };
    for ct in [0i32, 2, 3, 4, 6] {
        for n in [-1i32, 0, 1, 256, 257, 1000] {
            let mut run = |l: &Lib| -> Report {
                low_write(l, &mut |l, png, _info| unsafe {
                    (l.pv.png_write_tRNS)(png, alpha.as_ptr(), &tc, n, ct);
                    log("write_tRNS returned".to_string());
                })
            };
            diff(&format!("D2 png_write_tRNS ct={ct} n={n}"), &c, &r, &mut run);
        }
    }

    // bKGD with an out-of-range palette index or an unknown colour type
    for ct in [0i32, 2, 3, 4, 6, 99] {
        for idx in [0u8, 1, 200, 255] {
            let bg = PngColor16 {
                index: idx,
                red: 0xffff,
                green: 0xffff,
                blue: 0xffff,
                gray: 0xffff,
            };
            let mut run = |l: &Lib| -> Report {
                low_write(l, &mut |l, png, _info| unsafe {
                    (l.pv.png_write_bKGD)(png, &bg, ct);
                    log("write_bKGD returned".to_string());
                })
            };
            diff(&format!("D2 png_write_bKGD ct={ct} idx={idx}"), &c, &r, &mut run);
        }
    }

    // hIST with a count that does not match the palette
    let hist = vec![0x1234u16; 300];
    for n in [-1i32, 0, 1, 256, 257, 1000] {
        let mut run = |l: &Lib| -> Report {
            low_write(l, &mut |l, png, _info| unsafe {
                (l.pv.png_write_hIST)(png, hist.as_ptr(), n);
                log("write_hIST returned".to_string());
            })
        };
        diff(&format!("D2 png_write_hIST n={n}"), &c, &r, &mut run);
    }

    // sBIT with out-of-range significant bits
    for ct in [0i32, 2, 3, 4, 6, 99] {
        for v in [0u8, 1, 8, 9, 16, 17, 255] {
            let sig = PngColor8 { red: v, green: v, blue: v, gray: v, alpha: v };
            let mut run = |l: &Lib| -> Report {
                low_write(l, &mut |l, png, _info| unsafe {
                    (l.pv.png_write_sBIT)(png, &sig, ct);
                    log("write_sBIT returned".to_string());
                })
            };
            diff(&format!("D2 png_write_sBIT ct={ct} v={v}"), &c, &r, &mut run);
        }
    }

    // sRGB with an out-of-range intent
    for intent in [-1i32, 0, 3, 4, 100] {
        let mut run = |l: &Lib| -> Report {
            low_write(l, &mut |l, png, _info| unsafe {
                (l.pv.png_write_sRGB)(png, intent);
                log("write_sRGB returned".to_string());
            })
        };
        diff(&format!("D2 png_write_sRGB intent={intent}"), &c, &r, &mut run);
    }

    // tEXt / zTXt / iTXt keyword and payload validation
    let bad_keys: &[&str] = &[
        "",
        " ",
        " lead",
        "trail ",
        "mid  dle",
        "ctrl\u{1}x",
        "0123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890",
    ];
    let long_text: String = "x".repeat(100_000);
    for k in bad_keys {
        let key = CString::new(*k).unwrap();
        let txt = CString::new("value").unwrap();
        let mut run = |l: &Lib| -> Report {
            low_write(l, &mut |l, png, _info| unsafe {
                (l.pv.png_write_tEXt)(png, key.as_ptr(), txt.as_ptr(), 5);
                log("write_tEXt returned".to_string());
            })
        };
        diff(&format!("D2 png_write_tEXt key={k:?}"), &c, &r, &mut run);
        let mut run = |l: &Lib| -> Report {
            low_write(l, &mut |l, png, _info| unsafe {
                (l.pv.png_write_zTXt)(png, key.as_ptr(), txt.as_ptr(), 0);
                log("write_zTXt returned".to_string());
            })
        };
        diff(&format!("D2 png_write_zTXt key={k:?}"), &c, &r, &mut run);
        let mut run = |l: &Lib| -> Report {
            low_write(l, &mut |l, png, _info| unsafe {
                (l.pv.png_write_iTXt)(
                    png,
                    0,
                    key.as_ptr(),
                    ptr::null(),
                    ptr::null(),
                    txt.as_ptr(),
                );
                log("write_iTXt returned".to_string());
            })
        };
        diff(&format!("D2 png_write_iTXt key={k:?}"), &c, &r, &mut run);
    }
    // invalid compression selectors
    for comp in [-3i32, -2, -1, 0, 1, 2, 3, 99] {
        let key = CString::new("Key").unwrap();
        let txt = CString::new("value").unwrap();
        let mut run = |l: &Lib| -> Report {
            low_write(l, &mut |l, png, _info| unsafe {
                (l.pv.png_write_zTXt)(png, key.as_ptr(), txt.as_ptr(), comp);
                log("zTXt returned".to_string());
            })
        };
        diff(&format!("D2 png_write_zTXt comp={comp}"), &c, &r, &mut run);
        let mut run = |l: &Lib| -> Report {
            low_write(l, &mut |l, png, _info| unsafe {
                (l.pv.png_write_iTXt)(
                    png,
                    comp,
                    key.as_ptr(),
                    ptr::null(),
                    ptr::null(),
                    txt.as_ptr(),
                );
                log("iTXt returned".to_string());
            })
        };
        diff(&format!("D2 png_write_iTXt comp={comp}"), &c, &r, &mut run);
    }
    // text longer than a chunk can hold
    {
        let key = CString::new("Key").unwrap();
        let txt = CString::new(long_text.as_str()).unwrap();
        let mut run = |l: &Lib| -> Report {
            low_write(l, &mut |l, png, _info| unsafe {
                (l.pv.png_write_tEXt)(png, key.as_ptr(), txt.as_ptr(), long_text.len());
                log("long tEXt returned".to_string());
            })
        };
        diff("D2 png_write_tEXt very long", &c, &r, &mut run);
        let mut run = |l: &Lib| -> Report {
            low_write(l, &mut |l, png, _info| unsafe {
                (l.pv.png_write_iTXt)(
                    png,
                    1,
                    key.as_ptr(),
                    ptr::null(),
                    ptr::null(),
                    txt.as_ptr(),
                );
                log("long iTXt returned".to_string());
            })
        };
        diff("D2 png_write_iTXt very long uncompressed", &c, &r, &mut run);
    }

    // iCCP validation: NULL profile, short profile, bad length field, bad keyword
    let mut good = vec![0u8; 132];
    good[0..4].copy_from_slice(&132u32.to_be_bytes());
    good[4..8].copy_from_slice(b"ADBE");
    good[8..12].copy_from_slice(&0x0200_0000u32.to_be_bytes());
    good[12..16].copy_from_slice(b"mntr");
    good[16..20].copy_from_slice(b"RGB ");
    good[20..24].copy_from_slice(b"XYZ ");
    good[36..40].copy_from_slice(b"acsp");
    good[68..72].copy_from_slice(&0x0000_f6d6u32.to_be_bytes());
    good[72..76].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    good[76..80].copy_from_slice(&0x0000_d32du32.to_be_bytes());
    for (i, (name, prof, plen)) in [
        ("null profile", None, 132u32),
        ("zero length", Some(good.clone()), 0),
        ("short", Some(good[..64].to_vec()), 64),
        ("not multiple of 4", Some(good.clone()), 133),
        ("length mismatch", Some(good.clone()), 200),
        ("length 131", Some(good[..131].to_vec()), 131),
    ]
    .iter()
    .enumerate()
    {
        let iccname = CString::new("prof").unwrap();
        let mut run = |l: &Lib| -> Report {
            low_write(l, &mut |l, png, _info| unsafe {
                let p = prof.as_ref().map_or(ptr::null(), |v| v.as_ptr());
                (l.pv.png_write_iCCP)(png, iccname.as_ptr(), p, *plen);
                log("write_iCCP returned".to_string());
            })
        };
        diff(&format!("D2 png_write_iCCP #{i} {name}"), &c, &r, &mut run);
    }
    for k in bad_keys {
        let iccname = CString::new(*k).unwrap();
        let mut run = |l: &Lib| -> Report {
            low_write(l, &mut |l, png, _info| unsafe {
                (l.pv.png_write_iCCP)(png, iccname.as_ptr(), good.as_ptr(), 132);
                log("write_iCCP returned".to_string());
            })
        };
        diff(&format!("D2 png_write_iCCP key={k:?}"), &c, &r, &mut run);
    }

    // sPLT validation
    for k in bad_keys {
        let name = CString::new(*k).unwrap();
        let entries = vec![PngSpltEntry { red: 1, green: 2, blue: 3, alpha: 4, frequency: 5 }; 4];
        // NOTE: a NEGATIVE `nentries` is not tested here.  The C loop is
        //   for (ep = spalette->entries; ep < spalette->entries + nentries; ep++)
        // which forms `entries - 1`, a pointer before the start of the object,
        // and computes `entry_size * (size_t)nentries` from a negative value.
        // Both are undefined.  The public `png_set_sPLT` rejects `nentries <= 0`
        // with "png_set_sPLT: invalid sPLT", and that IS tested (D4).
        for depth in [0u8, 8, 16, 7, 255] {
            for nent in [0i32, 1, 4] {
                let s = PngSpltT {
                    name: name.as_ptr() as *mut c_char,
                    depth,
                    entries: entries.as_ptr() as *mut PngSpltEntry,
                    nentries: nent,
                };
                let mut run = |l: &Lib| -> Report {
                    low_write(l, &mut |l, png, _info| unsafe {
                        (l.pv.png_write_sPLT)(png, &s);
                        log("write_sPLT returned".to_string());
                    })
                };
                diff(
                    &format!("D2 png_write_sPLT key={k:?} depth={depth} nent={nent}"),
                    &c,
                    &r,
                    &mut run,
                );
            }
        }
    }

    // pCAL validation
    for k in bad_keys {
        let purpose = CString::new(*k).unwrap();
        let units = CString::new("u").unwrap();
        for eq in [-1i32, 0, 1, 2, 3, 4, 99] {
            let mut run = |l: &Lib| -> Report {
                low_write(l, &mut |l, png, _info| unsafe {
                    (l.pv.png_write_pCAL)(
                        png,
                        purpose.as_ptr() as *mut c_char,
                        0,
                        255,
                        eq,
                        0,
                        units.as_ptr(),
                        ptr::null_mut(),
                    );
                    log("write_pCAL returned".to_string());
                })
            };
            diff(&format!("D2 png_write_pCAL key={k:?} eq={eq}"), &c, &r, &mut run);
        }
    }

    // sCAL_s with values that do not fit the internal buffer
    let huge: String = "9".repeat(300);
    for (i, (wv, hv)) in [
        ("1", "1"),
        ("0", "1"),
        ("1", "0"),
        ("-1", "1"),
        ("1", "-1"),
        (huge.as_str(), "1"),
        ("1", huge.as_str()),
        ("", "1"),
        ("1", ""),
    ]
    .iter()
    .enumerate()
    {
        let cw = CString::new(*wv).unwrap();
        let ch = CString::new(*hv).unwrap();
        for unit in [-1i32, 0, 1, 2, 3, 99] {
            let mut run = |l: &Lib| -> Report {
                low_write(l, &mut |l, png, _info| unsafe {
                    (l.pv.png_write_sCAL_s)(png, unit, cw.as_ptr(), ch.as_ptr());
                    log("write_sCAL_s returned".to_string());
                })
            };
            diff(
                &format!("D2 png_write_sCAL_s #{i} unit={unit} len={}/{}", wv.len(), hv.len()),
                &c,
                &r,
                &mut run,
            );
        }
    }

    // tIME with out-of-range fields
    for t in [
        PngTime { year: 2020, month: 1, day: 1, hour: 0, minute: 0, second: 0 },
        PngTime { year: 2020, month: 0, day: 1, hour: 0, minute: 0, second: 0 },
        PngTime { year: 2020, month: 13, day: 1, hour: 0, minute: 0, second: 0 },
        PngTime { year: 2020, month: 1, day: 0, hour: 0, minute: 0, second: 0 },
        PngTime { year: 2020, month: 1, day: 32, hour: 0, minute: 0, second: 0 },
        PngTime { year: 2020, month: 1, day: 1, hour: 24, minute: 0, second: 0 },
        PngTime { year: 2020, month: 1, day: 1, hour: 0, minute: 60, second: 0 },
        PngTime { year: 2020, month: 1, day: 1, hour: 0, minute: 0, second: 61 },
    ] {
        let mut run = |l: &Lib| -> Report {
            low_write(l, &mut |l, png, _info| unsafe {
                (l.pv.png_write_tIME)(png, &t);
                log("write_tIME returned".to_string());
            })
        };
        diff(&format!("D2 png_write_tIME {t:?}"), &c, &r, &mut run);
    }

    // oFFs / pHYs with an out-of-range unit
    for unit in [-1i32, 0, 1, 2, 99] {
        let mut run = |l: &Lib| -> Report {
            low_write(l, &mut |l, png, _info| unsafe {
                (l.pv.png_write_oFFs)(png, -5, 7, unit);
                (l.pv.png_write_pHYs)(png, 100, 200, unit);
                log("oFFs/pHYs returned".to_string());
            })
        };
        diff(&format!("D2 png_write_oFFs/pHYs unit={unit}"), &c, &r, &mut run);
    }

    // gAMA / cHRM / cICP / cLLI / mDCV / eXIf low-level writers
    for g in [0i32, 1, 45455, 100000, i32::MAX, -1] {
        let mut run = |l: &Lib| -> Report {
            low_write(l, &mut |l, png, _info| unsafe {
                (l.pv.png_write_gAMA_fixed)(png, g);
                log("gAMA returned".to_string());
            })
        };
        diff(&format!("D2 png_write_gAMA_fixed g={g}"), &c, &r, &mut run);
    }
    for xy in [
        PngXy { redx: 64000, redy: 33000, greenx: 30000, greeny: 60000, bluex: 15000, bluey: 6000, whitex: 31270, whitey: 32900 },
        PngXy::default(),
        PngXy { redx: -1, redy: -1, greenx: -1, greeny: -1, bluex: -1, bluey: -1, whitex: -1, whitey: -1 },
    ] {
        let mut run = |l: &Lib| -> Report {
            low_write(l, &mut |l, png, _info| unsafe {
                (l.pv.png_write_cHRM_fixed)(png, &xy);
                log("cHRM returned".to_string());
            })
        };
        diff(&format!("D2 png_write_cHRM_fixed {xy:?}"), &c, &r, &mut run);
    }
    for (a, b) in [(0u32, 0u32), (0xffff_ffff, 0xffff_ffff), (10_000, 4_000_000)] {
        let mut run = |l: &Lib| -> Report {
            low_write(l, &mut |l, png, _info| unsafe {
                (l.pv.png_write_cLLI_fixed)(png, a, b);
                (l.pv.png_write_cICP)(png, 9, 16, 0, 1);
                (l.pv.png_write_mDCV_fixed)(png, 1, 2, 3, 4, 5, 6, 7, 8, a, b);
                log("pngv3 chunks returned".to_string());
            })
        };
        diff(&format!("D2 png_write_cLLI/cICP/mDCV {a}/{b}"), &c, &r, &mut run);
    }
    for n in [-1i32, 0, 1, 4, 8] {
        let mut exif = b"II*\0\x08\0\0\0".to_vec();
        let mut run = |l: &Lib| -> Report {
            low_write(l, &mut |l, png, _info| unsafe {
                (l.pv.png_write_eXIf)(png, exif.as_mut_ptr(), n);
                log("eXIf returned".to_string());
            })
        };
        diff(&format!("D2 png_write_eXIf n={n}"), &c, &r, &mut run);
    }
}

// ===========================================================================
// D-3  NULL / mismatched I/O functions
// ===========================================================================
#[test]
fn d3_null_io_functions() {
    let (c, r) = libs();
    // NOTE: neither `png_create_read_struct` nor `png_create_write_struct` leaves
    // its I/O function NULL: both end with `png_set_{read,write}_fn(p, NULL, NULL)`
    // and, because PNG_STDIO_SUPPORTED is enabled in `pnglibconf.h`, that installs
    // `png_default_{read,write}_data`.  Using those with the default NULL `io_ptr`
    // fread()s/fwrite()s a NULL FILE* — undefined behaviour, and the reference C
    // `.so` segfaults.
    //
    // The one DEFINED way to get a NULL I/O function is the cross-setter guard:
    // `png_set_write_fn` on a struct that already has a read function clears
    // `read_data_fn` (pngwio.c) and warns, and `png_set_read_fn` does the mirror
    // for `write_data_fn` (pngrio.c).  That is what is exercised here.

    let stream = gen(&c, 4, 2, PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE);

    // png_set_write_fn on a read struct: clears read_data_fn (+ warning), then
    // reading hits "Call to NULL read function".
    let mut run = |l: &Lib| -> Report {
        read_session(l, stream.clone(), &mut |l, png, info| unsafe {
            (l.api.png_set_write_fn)(
                png,
                ptr::null_mut(),
                cb_write as *mut c_void,
                cb_flush as *mut c_void,
            );
            log("set_write_fn on read struct returned".to_string());
            (l.api.png_read_info)(png, info);
            log("read_info returned".to_string());
        })
    };
    diff("D3 set_write_fn on read struct then read", &c, &r, &mut run);

    let mut run = |l: &Lib| -> Report {
        read_session(l, stream.clone(), &mut |l, png, _info| unsafe {
            (l.api.png_set_write_fn)(
                png,
                ptr::null_mut(),
                cb_write as *mut c_void,
                cb_flush as *mut c_void,
            );
            let mut b = [0u8; 4];
            (l.pv.png_read_data)(png, b.as_mut_ptr(), 4);
            log("png_read_data returned".to_string());
        })
    };
    diff("D3 png_read_data with NULL read function", &c, &r, &mut run);

    // png_set_read_fn on a write struct: clears write_data_fn (+ warning), then
    // writing hits "Call to NULL write function".
    let mut run = |l: &Lib| -> Report {
        write_session(l, &mut |l, png, _info| unsafe {
            (l.api.png_set_read_fn)(png, ptr::null_mut(), cb_read as *mut c_void);
            log("set_read_fn on write struct returned".to_string());
            (l.api.png_write_sig)(png);
            log("write_sig returned".to_string());
        })
    };
    diff("D3 set_read_fn on write struct then write", &c, &r, &mut run);

    let mut run = |l: &Lib| -> Report {
        write_session(l, &mut |l, png, _info| unsafe {
            (l.api.png_set_read_fn)(png, ptr::null_mut(), cb_read as *mut c_void);
            let b = [1u8, 2, 3];
            (l.pv.png_write_data)(png, b.as_ptr(), 3);
            log("png_write_data returned".to_string());
        })
    };
    diff("D3 png_write_data with NULL write function", &c, &r, &mut run);

    // With our callbacks properly installed, the same low-level entry points
    // must behave identically too.
    let mut run = |l: &Lib| -> Report {
        write_session(l, &mut |l, png, _info| unsafe {
            let b = [1u8, 2, 3];
            (l.pv.png_write_data)(png, b.as_ptr(), 3);
            (l.pv.png_flush)(png);
            log("write_data returned".to_string());
        })
    };
    diff("D3 png_write_data", &c, &r, &mut run);
    let mut run = |l: &Lib| -> Report {
        read_session(l, vec![1, 2, 3, 4], &mut |l, png, _info| unsafe {
            let mut b = [0u8; 4];
            (l.pv.png_read_data)(png, b.as_mut_ptr(), 4);
            log(format!("read_data got {b:02x?}"));
        })
    };
    diff("D3 png_read_data", &c, &r, &mut run);
}

// ===========================================================================
// D-4  Deprecated / state-dependent setter rejections
// ===========================================================================
#[test]
fn d4_state_dependent_setters() {
    let (c, r) = libs();
    // png_set_eXIf / png_get_eXIf: the deprecated forms warn
    let mut run = |l: &Lib| -> Report {
        write_session(l, &mut |l, png, info| unsafe {
            let mut exif = b"II*\0".to_vec();
            (l.api.png_set_eXIf)(png, info, exif.as_mut_ptr());
            let mut p: *mut u8 = ptr::null_mut();
            log(format!(
                "get_eXIf={}",
                (l.api.png_get_eXIf)(png, info, &mut p)
            ));
        })
    };
    diff("D4 deprecated eXIf accessors", &c, &r, &mut run);

    // png_set_compression_buffer_size after writing has begun, and a huge size
    for (name, when) in [("before", 0u32), ("after write_info", 1), ("after rows", 2)] {
        for size in [0usize, 1, 8, 1 << 20, usize::MAX, usize::MAX / 2] {
            let mut run = |l: &Lib| -> Report {
                write_session(l, &mut |l, png, info| unsafe {
                    if when == 0 {
                        (l.api.png_set_compression_buffer_size)(png, size);
                    }
                    (l.api.png_set_IHDR)(
                        png, info, 4, 4, 8, PNG_COLOR_TYPE_RGB, PNG_INTERLACE_NONE,
                        PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
                    );
                    (l.api.png_write_info)(png, info);
                    if when == 1 {
                        (l.api.png_set_compression_buffer_size)(png, size);
                    }
                    let rows = make_rows(4, 12, SEED);
                    (l.api.png_write_row)(png, rows[0].as_ptr());
                    if when == 2 {
                        (l.api.png_set_compression_buffer_size)(png, size);
                    }
                    log(format!(
                        "buffer_size={}",
                        (l.api.png_get_compression_buffer_size)(png)
                    ));
                })
            };
            diff(
                &format!("D4 compression_buffer_size {name} size={size}"),
                &c,
                &r,
                &mut run,
            );
        }
    }

    // png_set_filter after row writing has started
    for mask in [
        PNG_NO_FILTERS,
        PNG_FILTER_NONE,
        PNG_FILTER_SUB,
        PNG_FILTER_UP,
        PNG_FILTER_AVG,
        PNG_FILTER_PAETH,
        PNG_ALL_FILTERS,
    ] {
        let mut run = |l: &Lib| -> Report {
            write_session(l, &mut |l, png, info| unsafe {
                (l.api.png_set_IHDR)(
                    png, info, 4, 4, 8, PNG_COLOR_TYPE_RGB, PNG_INTERLACE_NONE,
                    PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
                );
                (l.api.png_set_filter)(png, PNG_FILTER_TYPE_BASE, PNG_FILTER_NONE);
                (l.api.png_write_info)(png, info);
                let rows = make_rows(4, 12, SEED);
                (l.api.png_write_row)(png, rows[0].as_ptr());
                (l.api.png_set_filter)(png, PNG_FILTER_TYPE_BASE, mask);
                for row in rows.iter().skip(1) {
                    (l.api.png_write_row)(png, row.as_ptr());
                }
                (l.api.png_write_end)(png, info);
            })
        };
        diff(&format!("D4 set_filter after start mask={mask:#x}"), &c, &r, &mut run);
    }

    // png_set_user_transform_info after png_start_read_image
    let stream = gen(&c, 4, 2, PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE);
    for when in 0..3u32 {
        let mut run = |l: &Lib| -> Report {
            read_session(l, stream.clone(), &mut |l, png, info| unsafe {
                (l.api.png_read_info)(png, info);
                if when == 0 {
                    (l.api.png_set_user_transform_info)(png, ptr::null_mut(), 8, 3);
                }
                (l.api.png_start_read_image)(png);
                if when == 1 {
                    (l.api.png_set_user_transform_info)(png, ptr::null_mut(), 8, 3);
                }
                (l.api.png_read_update_info)(png, info);
                if when == 2 {
                    (l.api.png_set_user_transform_info)(png, ptr::null_mut(), 8, 3);
                }
                log("done".to_string());
            })
        };
        diff(&format!("D4 set_user_transform_info when={when}"), &c, &r, &mut run);
    }

    // png_set_longjmp_fn: request a heap jmp_buf then a smaller one
    for (big, small) in [(400usize, 200usize), (256, 8), (1000, 200), (201, 200)] {
        let mut run = |l: &Lib| -> Report {
            let mut ctxb = Box::new(Ctx::default());
            set_ctx(&mut *ctxb as *mut Ctx);
            unsafe {
                let png = (l.api.png_create_write_struct)(
                    ver(),
                    ptr::null_mut(),
                    cb_error as *mut c_void,
                    cb_warn as *mut c_void,
                );
                let a = (l.api.png_set_longjmp_fn)(png, longjmp_addr(), big);
                log(format!("big={big} null={}", a.is_null()));
                let b = (l.api.png_set_longjmp_fn)(png, longjmp_addr(), small);
                log(format!("small={small} null={}", b.is_null()));
                let mut pp = png;
                (l.api.png_destroy_write_struct)(&mut pp, ptr::null_mut());
            }
            let rep = ctxb.digest();
            set_ctx(ptr::null_mut());
            rep
        };
        diff(&format!("D4 longjmp_fn {big}->{small}"), &c, &r, &mut run);
    }

    // png_set_keep_unknown_chunks: NULL list with num_chunks > 0, and a huge count
    for (name, num, use_list) in [
        ("null list, num>0", 3i32, false),
        ("huge count", 100000, true),
        ("num = INT_MAX", i32::MAX, true),
        ("num = INT_MIN", i32::MIN, true),
    ] {
        let list = b"prVt\0PRVt\0pRVt\0".to_vec();
        let mut run = |l: &Lib| -> Report {
            read_session(l, vec![], &mut |l, png, _info| unsafe {
                let p = if use_list { list.as_ptr() } else { ptr::null() };
                (l.api.png_set_keep_unknown_chunks)(png, PNG_HANDLE_CHUNK_ALWAYS, p, num);
                log("keep_unknown returned".to_string());
            })
        };
        diff(&format!("D4 keep_unknown {name}"), &c, &r, &mut run);
    }
    // IHDR / IEND in the keep list
    for nm in [b"IHDR\0", b"IEND\0"] {
        let list = nm.to_vec();
        let mut run = |l: &Lib| -> Report {
            read_session(l, vec![], &mut |l, png, _info| unsafe {
                (l.api.png_set_keep_unknown_chunks)(
                    png,
                    PNG_HANDLE_CHUNK_ALWAYS,
                    list.as_ptr(),
                    1,
                );
                log("keep_unknown returned".to_string());
            })
        };
        diff(
            &format!("D4 keep_unknown list={:?}", String::from_utf8_lossy(&nm[..4])),
            &c,
            &r,
            &mut run,
        );
    }

    // png_set_sPLT with an invalid descriptor
    for (name, nent, null_entries) in [
        ("nentries 0", 0i32, false),
        ("nentries -1", -1, false),
        ("NULL entries", 4, true),
    ] {
        let nm = CString::new("s").unwrap();
        let entries = vec![PngSpltEntry { red: 0, green: 0, blue: 0, alpha: 0, frequency: 0 }; 4];
        let mut run = |l: &Lib| -> Report {
            write_session(l, &mut |l, png, info| unsafe {
                let s = PngSpltT {
                    name: nm.as_ptr() as *mut c_char,
                    depth: 8,
                    entries: if null_entries {
                        ptr::null_mut()
                    } else {
                        entries.as_ptr() as *mut PngSpltEntry
                    },
                    nentries: nent,
                };
                (l.api.png_set_sPLT)(png, info, &s as *const PngSpltT as *const c_void, 1);
                log("set_sPLT returned".to_string());
            })
        };
        diff(&format!("D4 set_sPLT {name}"), &c, &r, &mut run);
    }

    // png_set_cHRM_XYZ_fixed with a degenerate matrix
    for a in [
        [0i32; 9],
        [1, 1, 1, 1, 1, 1, 1, 1, 1],
        [i32::MAX; 9],
        [i32::MIN; 9],
        [-100000, 0, 0, 0, -100000, 0, 0, 0, -100000],
    ] {
        let mut run = |l: &Lib| -> Report {
            write_session(l, &mut |l, png, info| unsafe {
                (l.api.png_set_cHRM_XYZ_fixed)(
                    png, info, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7], a[8],
                );
                log("set_cHRM_XYZ returned".to_string());
            })
        };
        diff(&format!("D4 set_cHRM_XYZ_fixed {a:?}"), &c, &r, &mut run);
    }

    // png_set_hIST without a palette
    for n in [1usize, 4, 256] {
        let hist = vec![1u16; n];
        let mut run = |l: &Lib| -> Report {
            write_session(l, &mut |l, png, info| unsafe {
                (l.api.png_set_IHDR)(
                    png, info, 4, 4, 8, PNG_COLOR_TYPE_RGB, PNG_INTERLACE_NONE,
                    PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
                );
                (l.api.png_set_hIST)(png, info, hist.as_ptr());
                log("set_hIST returned".to_string());
            })
        };
        diff(&format!("D4 set_hIST no palette n={n}"), &c, &r, &mut run);
    }

    // png_set_sCAL / _fixed / _s with a non-positive height
    for (wv, hv) in [(1.0f64, 0.0f64), (1.0, -1.0), (0.0, 0.0), (-1.0, -1.0)] {
        let mut run = |l: &Lib| -> Report {
            write_session(l, &mut |l, png, info| unsafe {
                (l.api.png_set_sCAL)(png, info, 1, wv, hv);
                log("set_sCAL returned".to_string());
            })
        };
        diff(&format!("D4 set_sCAL {wv}x{hv}"), &c, &r, &mut run);
    }
    for (wv, hv) in [(100000i32, 0i32), (100000, -1), (0, 0), (-1, -1)] {
        let mut run = |l: &Lib| -> Report {
            write_session(l, &mut |l, png, info| unsafe {
                (l.api.png_set_sCAL_fixed)(png, info, 1, wv, hv);
                log("set_sCAL_fixed returned".to_string());
            })
        };
        diff(&format!("D4 set_sCAL_fixed {wv}x{hv}"), &c, &r, &mut run);
    }
    for (wv, hv) in [
        ("1", "0"),
        ("1", "-1"),
        ("0", "0"),
        ("-1", "-1"),
        ("1", "abc"),
        ("abc", "1"),
        ("1", ""),
        ("", "1"),
    ] {
        let cw = CString::new(wv).unwrap();
        let ch = CString::new(hv).unwrap();
        let mut run = |l: &Lib| -> Report {
            write_session(l, &mut |l, png, info| unsafe {
                (l.api.png_set_sCAL_s)(png, info, 1, cw.as_ptr(), ch.as_ptr());
                log("set_sCAL_s returned".to_string());
            })
        };
        diff(&format!("D4 set_sCAL_s {wv:?}x{hv:?}"), &c, &r, &mut run);
    }

    // png_write_png with both STRIP_FILLER flags
    let mut run = |l: &Lib| -> Report {
        write_session(l, &mut |l, png, info| unsafe {
            (l.api.png_set_IHDR)(
                png, info, 4, 4, 8, PNG_COLOR_TYPE_RGB, PNG_INTERLACE_NONE,
                PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
            );
            let rows = make_rows(4, 16, SEED);
            let mut ptrs: Vec<*mut u8> = rows.iter().map(|v| v.as_ptr() as *mut u8).collect();
            (l.api.png_set_rows)(png, info, ptrs.as_mut_ptr());
            (l.api.png_write_png)(
                png,
                info,
                PNG_TRANSFORM_STRIP_FILLER_BEFORE | PNG_TRANSFORM_STRIP_FILLER_AFTER,
                ptr::null_mut(),
            );
        })
    };
    diff("D4 png_write_png STRIP_FILLER BEFORE+AFTER", &c, &r, &mut run);

    // png_set_unknown_chunk_location with every location value
    for loc in [-1i32, 0, 1, 2, 3, 4, 8, 9, 0x10, 0xff] {
        let payload = [1u8, 2, 3];
        let mut run = |l: &Lib| -> Report {
            write_session(l, &mut |l, png, info| unsafe {
                let unk = [PngUnknownChunk {
                    name: *b"prVt\0",
                    data: payload.as_ptr() as *mut u8,
                    size: 3,
                    location: 0,
                }];
                (l.api.png_set_unknown_chunks)(png, info, unk.as_ptr(), 1);
                (l.api.png_set_unknown_chunk_location)(png, info, 0, loc);
                log("location set".to_string());
            })
        };
        diff(&format!("D4 unknown_chunk_location loc={loc}"), &c, &r, &mut run);
    }
    // png_set_unknown_chunks with an invalid location byte in the descriptor
    for loc in [0u8, 1, 2, 3, 4, 8, 9, 0x10, 0xff] {
        let payload = [1u8, 2, 3];
        let mut run = |l: &Lib| -> Report {
            write_session(l, &mut |l, png, info| unsafe {
                let unk = [PngUnknownChunk {
                    name: *b"prVt\0",
                    data: payload.as_ptr() as *mut u8,
                    size: 3,
                    location: loc,
                }];
                (l.api.png_set_unknown_chunks)(png, info, unk.as_ptr(), 1);
                log("unknown chunks set".to_string());
            })
        };
        diff(&format!("D4 set_unknown_chunks loc={loc}"), &c, &r, &mut run);
    }
}

// ===========================================================================
// D-5  Crafted chunks reaching the remaining read-side handlers
// ===========================================================================
#[test]
fn d5_crafted_chunk_handlers() {
    let (c, r) = libs();

    // bKGD with out-of-range values, per colour type
    for &(ct, bd) in &[(0i32, 8i32), (0, 16), (2, 8), (2, 16), (3, 8), (4, 8), (6, 8)] {
        let base = gen(&c, 8, 4, ct, bd, PNG_INTERLACE_NONE);
        let cs = chunks(&base);
        let idat = cs.iter().find(|c| &c.2 == b"IDAT").unwrap().0;
        let mut payloads: Vec<Vec<u8>> = Vec::new();
        // palette form (1 byte index)
        for idx in [0u8, 1, 255] {
            payloads.push(vec![idx]);
        }
        // gray form (2 bytes)
        for v in [0u16, 1, 0xff, 0x100, 0xffff] {
            payloads.push(v.to_be_bytes().to_vec());
        }
        // rgb form (6 bytes)
        for v in [0u16, 0xff, 0x100, 0xffff] {
            let mut p = Vec::new();
            p.extend_from_slice(&v.to_be_bytes());
            p.extend_from_slice(&v.to_be_bytes());
            p.extend_from_slice(&v.to_be_bytes());
            payloads.push(p);
        }
        for (i, p) in payloads.iter().enumerate() {
            let mut s = base[..idat].to_vec();
            s.extend_from_slice(&make_chunk(b"bKGD", p));
            s.extend_from_slice(&base[idat..]);
            let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
            diff(
                &format!("D5 bKGD #{i} len={} ct={ct} bd={bd}", p.len()),
                &c,
                &r,
                &mut run,
            );
        }
    }

    // tRNS on an image that already has an alpha channel
    for &(ct, bd) in &[(4i32, 8i32), (6, 8), (4, 16), (6, 16)] {
        let base = gen(&c, 8, 4, ct, bd, PNG_INTERLACE_NONE);
        let cs = chunks(&base);
        let idat = cs.iter().find(|c| &c.2 == b"IDAT").unwrap().0;
        for p in [vec![0u8, 0], vec![0u8; 6], vec![0u8; 1]] {
            let mut s = base[..idat].to_vec();
            s.extend_from_slice(&make_chunk(b"tRNS", &p));
            s.extend_from_slice(&base[idat..]);
            let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
            diff(
                &format!("D5 tRNS with alpha ct={ct} bd={bd} len={}", p.len()),
                &c,
                &r,
                &mut run,
            );
        }
    }

    // pCAL with every equation type, including unrecognised ones
    let base = gen(&c, 8, 4, PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE);
    let cs = chunks(&base);
    let idat = cs.iter().find(|c| &c.2 == b"IDAT").unwrap().0;
    for eq in 0u8..8 {
        let mut p = b"purpose\0".to_vec();
        p.extend_from_slice(&0i32.to_be_bytes());
        p.extend_from_slice(&255i32.to_be_bytes());
        p.push(eq);
        p.push(0); // nparams
        p.extend_from_slice(b"unit\0");
        let mut s = base[..idat].to_vec();
        s.extend_from_slice(&make_chunk(b"pCAL", &p));
        s.extend_from_slice(&base[idat..]);
        let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
        diff(&format!("D5 pCAL equation={eq}"), &c, &r, &mut run);
    }

    // sCAL with non-positive / malformed width and height
    for (i, body) in [
        b"\x011\x000".to_vec(),
        b"\x011\x00-1".to_vec(),
        b"\x010\x001".to_vec(),
        b"\x01-1\x001".to_vec(),
        b"\x011\x00".to_vec(),
        b"\x011".to_vec(),
        b"\x02 \x00 ".to_vec(),
        b"\x011.0e10\x001.0e-10".to_vec(),
    ]
    .iter()
    .enumerate()
    {
        let mut s = base[..idat].to_vec();
        s.extend_from_slice(&make_chunk(b"sCAL", body));
        s.extend_from_slice(&base[idat..]);
        let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
        diff(&format!("D5 sCAL #{i}"), &c, &r, &mut run);
    }

    // sPLT: bad length, too many entries, and with a tiny chunk cache
    for (i, (depth, nent)) in [(8u8, 1usize), (8, 100), (16, 1), (16, 100), (7, 1), (0, 1)]
        .iter()
        .enumerate()
    {
        let esz = if *depth == 8 { 6 } else { 10 };
        let mut p = b"name\0".to_vec();
        p.push(*depth);
        p.extend_from_slice(&vec![0x5au8; esz * nent]);
        let mut s = base[..idat].to_vec();
        s.extend_from_slice(&make_chunk(b"sPLT", &p));
        s.extend_from_slice(&base[idat..]);
        let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
        diff(&format!("D5 sPLT #{i} depth={depth} nent={nent}"), &c, &r, &mut run);
        // ...and with the chunk cache / malloc limits forced low
        for (cache, mal) in [(1u32, 0usize), (0, 8), (2, 16)] {
            let mut run = |l: &Lib| -> Report {
                read_session(l, s.clone(), &mut |l, png, info| unsafe {
                    (l.api.png_set_chunk_cache_max)(png, cache);
                    (l.api.png_set_chunk_malloc_max)(png, mal);
                    (l.api.png_read_info)(png, info);
                    let mut e: *mut c_void = ptr::null_mut();
                    log(format!("sPLT count={}", (l.api.png_get_sPLT)(png, info, &mut e)));
                    (l.api.png_read_end)(png, info);
                })
            };
            diff(
                &format!("D5 sPLT #{i} cache={cache} mal={mal}"),
                &c,
                &r,
                &mut run,
            );
        }
    }
    // an sPLT chunk longer than the cache allows
    {
        let mut p = b"name\0".to_vec();
        p.push(8);
        p.extend_from_slice(&vec![0x11u8; 6 * 3000]);
        let mut s = base[..idat].to_vec();
        s.extend_from_slice(&make_chunk(b"sPLT", &p));
        s.extend_from_slice(&base[idat..]);
        for mal in [0usize, 100, 1000, 1 << 20] {
            let mut run = |l: &Lib| -> Report {
                read_session(l, s.clone(), &mut |l, png, info| unsafe {
                    (l.api.png_set_chunk_malloc_max)(png, mal);
                    (l.api.png_read_info)(png, info);
                    (l.api.png_read_end)(png, info);
                    log("ok".to_string());
                })
            };
            diff(&format!("D5 huge sPLT mal={mal}"), &c, &r, &mut run);
        }
    }

    // A second IDAT run after other chunks, and IDAT after IEND
    {
        let cs = chunks(&base);
        let (ioff, ilen, _) = *cs.iter().find(|c| &c.2 == b"IDAT").unwrap();
        let iend = cs.iter().find(|c| &c.2 == b"IEND").unwrap();
        // extra IDAT after IEND
        let mut s = base.clone();
        s.extend_from_slice(&base[ioff..ioff + 12 + ilen]);
        let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
        diff("D5 IDAT after IEND", &c, &r, &mut run);
        // IDAT ... tEXt ... IDAT (non-consecutive IDATs)
        let mut s = base[..ioff + 12 + ilen].to_vec();
        s.extend_from_slice(&make_chunk(b"tEXt", b"K\0v"));
        s.extend_from_slice(&base[ioff..ioff + 12 + ilen]);
        s.extend_from_slice(&base[iend.0..]);
        let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
        diff("D5 non-consecutive IDATs", &c, &r, &mut run);
        // and progressively
        let mut run = |l: &Lib| -> Report {
            read_session(l, vec![], &mut |l, png, info| unsafe {
                (l.api.png_set_progressive_read_fn)(
                    png,
                    ptr::null_mut(),
                    d5_info_cb as *mut c_void,
                    d5_row_cb as *mut c_void,
                    d5_end_cb as *mut c_void,
                );
                (l.api.png_process_data)(png, info, s.as_ptr() as *mut u8, s.len());
                log("processed".to_string());
            })
        };
        diff("D5 non-consecutive IDATs (progressive)", &c, &r, &mut run);
    }

    // chunk header with an out-of-range length, and an invalid chunk type
    for len in [0x8000_0000u32, 0xffff_ffff, 0x7fff_ffff] {
        let mut s = base[..8].to_vec();
        s.extend_from_slice(&len.to_be_bytes());
        s.extend_from_slice(b"IHDR");
        s.extend_from_slice(&base[8 + 8..]);
        let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
        diff(&format!("D5 chunk length {len:#x}"), &c, &r, &mut run);
    }
    for ty in [
        b"\x00\x00\x00\x00", b"0123", b"    ", b"ab\x00d", b"\xff\xff\xff\xff", b"IhDr",
    ] {
        let mut s = base[..8].to_vec();
        s.extend_from_slice(&make_chunk(ty, &base[8 + 8..8 + 8 + 13]));
        s.extend_from_slice(&base[8 + 25..]);
        let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
        diff(
            &format!("D5 chunk type {:?}", String::from_utf8_lossy(ty)),
            &c,
            &r,
            &mut run,
        );
    }
    // IHDR with a wrong declared length
    for n in [0usize, 12, 13, 14, 20] {
        let mut data = vec![0u8; n];
        let real = &base[8 + 8..8 + 8 + 13];
        for (i, b) in data.iter_mut().enumerate() {
            *b = if i < real.len() { real[i] } else { 0 };
        }
        let mut s = base[..8].to_vec();
        s.extend_from_slice(&make_chunk(b"IHDR", &data));
        s.extend_from_slice(&base[8 + 25..]);
        let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
        diff(&format!("D5 IHDR length={n}"), &c, &r, &mut run);
        // and progressively, where png_push_read_chunk has its own check
        let mut run = |l: &Lib| -> Report {
            read_session(l, vec![], &mut |l, png, info| unsafe {
                (l.api.png_set_progressive_read_fn)(
                    png,
                    ptr::null_mut(),
                    d5_info_cb as *mut c_void,
                    d5_row_cb as *mut c_void,
                    d5_end_cb as *mut c_void,
                );
                (l.api.png_process_data)(png, info, s.as_ptr() as *mut u8, s.len());
                log("processed".to_string());
            })
        };
        diff(&format!("D5 IHDR length={n} (progressive)"), &c, &r, &mut run);
    }

    // unknown chunk handling: user callback returning <0, and keep modes that
    // force a save of an unhandled chunk
    let payload = [1u8, 2, 3, 4];
    for keep in [0i32, 1, 2, 3] {
        for cbret in [-1i32, 0, 1] {
            for critical in [false, true] {
                let ty: &[u8; 4] = if critical { b"PRVT" } else { b"prVt" };
                let mut s = base[..idat].to_vec();
                s.extend_from_slice(&make_chunk(ty, &payload));
                s.extend_from_slice(&base[idat..]);
                let mut run = |l: &Lib| -> Report {
                    unsafe { D5_CB_RET = cbret };
                    read_session(l, s.clone(), &mut |l, png, info| unsafe {
                        (l.api.png_set_keep_unknown_chunks)(png, keep, ptr::null(), 0);
                        (l.api.png_set_read_user_chunk_fn)(
                            png,
                            ptr::null_mut(),
                            d5_chunk_cb as *mut c_void,
                        );
                        (l.api.png_read_info)(png, info);
                        let mut e: *mut PngUnknownChunk = ptr::null_mut();
                        log(format!(
                            "unknown={}",
                            (l.api.png_get_unknown_chunks)(png, info, &mut e)
                        ));
                        (l.api.png_read_end)(png, info);
                    })
                };
                diff(
                    &format!("D5 unknown keep={keep} cbret={cbret} critical={critical}"),
                    &c,
                    &r,
                    &mut run,
                );
            }
        }
    }
    // ...and with the chunk cache exhausted so the save is refused
    for cache in [0u32, 1, 2] {
        for mal in [0usize, 1, 2, 3, 4, 100] {
            let mut s = base[..idat].to_vec();
            s.extend_from_slice(&make_chunk(b"prVt", &payload));
            s.extend_from_slice(&base[idat..]);
            let mut run = |l: &Lib| -> Report {
                read_session(l, s.clone(), &mut |l, png, info| unsafe {
                    (l.api.png_set_keep_unknown_chunks)(
                        png,
                        PNG_HANDLE_CHUNK_ALWAYS,
                        ptr::null(),
                        0,
                    );
                    (l.api.png_set_chunk_cache_max)(png, cache);
                    (l.api.png_set_chunk_malloc_max)(png, mal);
                    (l.api.png_read_info)(png, info);
                    let mut e: *mut PngUnknownChunk = ptr::null_mut();
                    log(format!(
                        "unknown={}",
                        (l.api.png_get_unknown_chunks)(png, info, &mut e)
                    ));
                    (l.api.png_read_end)(png, info);
                })
            };
            diff(
                &format!("D5 unknown cache={cache} mal={mal}"),
                &c,
                &r,
                &mut run,
            );
        }
    }
}

static mut D5_CB_RET: c_int = 0;

unsafe extern "C" fn d5_chunk_cb(_png: *mut c_void, chunk: *mut PngUnknownChunk) -> c_int {
    if !chunk.is_null() {
        let ch = &*chunk;
        log(format!(
            "d5 chunk {:?} size={}",
            String::from_utf8_lossy(&ch.name[..4]),
            ch.size
        ));
    }
    D5_CB_RET
}

unsafe extern "C" fn d5_info_cb(_png: *mut c_void, _info: *mut c_void) {
    log("d5 info");
}
unsafe extern "C" fn d5_row_cb(_png: *mut c_void, _row: *mut u8, n: u32, p: c_int) {
    log(format!("d5 row {n} {p}"));
}
unsafe extern "C" fn d5_end_cb(_png: *mut c_void, _info: *mut c_void) {
    log("d5 end");
}

// ===========================================================================
// D-6  Simplified-API rejections that need specific format combinations
// ===========================================================================
#[test]
fn d6_simplified_format_rejections() {
    let (c, r) = libs();
    // Sources with an alpha channel or tRNS, read into a colour-mapped output
    // without a background: "background color must be supplied ..."
    for &(ct, bd) in &[(4i32, 8i32), (6, 8), (3, 8), (0, 8), (2, 8), (6, 16)] {
        let with_trns = ct == PNG_COLOR_TYPE_PALETTE;
        let stream = {
            let pal = if ct == PNG_COLOR_TYPE_PALETTE {
                make_palette(256, SEED)
            } else {
                vec![]
            };
            write_full(
                &c,
                8,
                4,
                ct,
                bd,
                PNG_INTERLACE_NONE,
                PNG_FILTER_TYPE_BASE,
                &pal,
                rowbytes(8, bd, ct),
                SEED ^ 3,
                &mut |l, png, info| unsafe {
                    if with_trns {
                        let alpha: Vec<u8> = (0..256u32).map(|i| i as u8).collect();
                        (l.api.png_set_tRNS)(png, info, alpha.as_ptr(), 256, ptr::null());
                    }
                },
            )
            .out
        };
        for fmt in [
            PNG_FORMAT_FLAG_COLORMAP,
            PNG_FORMAT_FLAG_COLORMAP | PNG_FORMAT_FLAG_COLOR,
            PNG_FORMAT_FLAG_COLORMAP | PNG_FORMAT_FLAG_ALPHA,
            PNG_FORMAT_FLAG_COLORMAP | PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA,
            PNG_FORMAT_FLAG_COLORMAP | PNG_FORMAT_FLAG_LINEAR,
            PNG_FORMAT_FLAG_COLORMAP | PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_LINEAR,
            0,
            PNG_FORMAT_FLAG_COLOR,
            PNG_FORMAT_FLAG_ALPHA,
            PNG_FORMAT_FLAG_LINEAR,
        ] {
            for bg in [false, true] {
                for cmap_null in [false, true] {
                    let mut run = |l: &Lib| -> Report {
                        scratch(&mut || unsafe {
                            let mut im = PngImage::default();
                            let ok = (l.api.png_image_begin_read_from_memory)(
                                &mut im,
                                stream.as_ptr() as *const c_void,
                                stream.len(),
                            );
                            log(format!("begin={ok}"));
                            if ok != 0 {
                                im.format = fmt;
                                let mut buf = vec![0u8; 1 << 14];
                                let mut cmap = vec![0u8; 1 << 12];
                                let bgc = PngColor { red: 1, green: 2, blue: 3 };
                                let ok2 = (l.api.png_image_finish_read)(
                                    &mut im,
                                    if bg { &bgc } else { ptr::null() },
                                    buf.as_mut_ptr() as *mut c_void,
                                    0,
                                    if cmap_null {
                                        ptr::null_mut()
                                    } else {
                                        cmap.as_mut_ptr() as *mut c_void
                                    },
                                );
                                log_img(&format!("finish={ok2}"), &im);
                            }
                            (l.api.png_image_free)(&mut im);
                        })
                    };
                    diff(
                        &format!("D6 read ct={ct} bd={bd} fmt={fmt:#x} bg={bg} cmapnull={cmap_null}"),
                        &c,
                        &r,
                        &mut run,
                    );
                }
            }
        }
    }

    // Write side: colour-mapped format with no colormap, oversize strides and
    // oversize images.
    for fmt in [
        PNG_FORMAT_FLAG_COLORMAP,
        PNG_FORMAT_FLAG_COLORMAP | PNG_FORMAT_FLAG_COLOR,
        PNG_FORMAT_FLAG_COLORMAP | PNG_FORMAT_FLAG_ALPHA,
    ] {
        for entries in [0u32, 1, 256, 257] {
            for cmap_null in [false, true] {
                let mut run = |l: &Lib| -> Report {
                    scratch(&mut || unsafe {
                        let mut im = PngImage {
                            version: PNG_IMAGE_VERSION,
                            width: 4,
                            height: 4,
                            format: fmt,
                            colormap_entries: entries,
                            ..Default::default()
                        };
                        let src = vec![0x20u8; 4096];
                        let cmap = vec![0x40u8; 4096];
                        let mut out = vec![0u8; 1 << 16];
                        let mut sz = out.len();
                        let ok = (l.api.png_image_write_to_memory)(
                            &mut im,
                            out.as_mut_ptr() as *mut c_void,
                            &mut sz,
                            0,
                            src.as_ptr() as *mut c_void,
                            0,
                            if cmap_null {
                                ptr::null_mut()
                            } else {
                                cmap.as_ptr() as *mut c_void
                            },
                        );
                        log_img(&format!("write={ok} sz={sz}"), &im);
                        (l.api.png_image_free)(&mut im);
                    })
                };
                diff(
                    &format!("D6 write fmt={fmt:#x} entries={entries} cmapnull={cmap_null}"),
                    &c,
                    &r,
                    &mut run,
                );
            }
        }
    }
    // strides that are too small / far too large, and huge images
    // NOTE: an over-large but ACCEPTED row stride is not tested: libpng only
    // rejects strides SMALLER than the minimum, so a huge stride is a promise by
    // the caller that the buffer really is that big and libpng reads accordingly.
    // Passing a huge stride with a small buffer is a caller bug, not a library
    // rejection (verified: the reference C .so reads out of bounds and faults).
    for (w, h, stride) in [
        (4u32, 4u32, 1i32),
        (4, 4, 2),
        (4, 4, 3),
        (4, 4, -1),
        (0x1000_0000, 4, 0),
        (4, 0x1000_0000, 0),
        (0x4000_0000, 0x4000_0000, 0),
        (0x7fff_ffff, 1, 0),
    ] {
        for fmt in [0u32, PNG_FORMAT_FLAG_COLOR, PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA] {
            let mut run = |l: &Lib| -> Report {
                scratch(&mut || unsafe {
                    let mut im = PngImage {
                        version: PNG_IMAGE_VERSION,
                        width: w,
                        height: h,
                        format: fmt,
                        ..Default::default()
                    };
                    let src = vec![0x20u8; 1 << 16];
                    let mut out = vec![0u8; 1 << 16];
                    let mut sz = out.len();
                    let ok = (l.api.png_image_write_to_memory)(
                        &mut im,
                        out.as_mut_ptr() as *mut c_void,
                        &mut sz,
                        0,
                        src.as_ptr() as *mut c_void,
                        stride,
                        ptr::null_mut(),
                    );
                    log_img(&format!("write={ok} sz={sz}"), &im);
                    (l.api.png_image_free)(&mut im);
                })
            };
            diff(
                &format!("D6 write {w}x{h} stride={stride} fmt={fmt:#x}"),
                &c,
                &r,
                &mut run,
            );
        }
    }
    // A PNG that will not fit in the supplied memory at all
    for cap in [0usize, 1, 2, 8, 40, 60] {
        let mut run = |l: &Lib| -> Report {
            scratch(&mut || unsafe {
                let mut im = PngImage {
                    version: PNG_IMAGE_VERSION,
                    width: 64,
                    height: 64,
                    format: PNG_FORMAT_FLAG_COLOR,
                    ..Default::default()
                };
                let src = vec![0x33u8; 64 * 64 * 3];
                let mut out = vec![0u8; cap.max(1)];
                let mut sz = cap;
                let ok = (l.api.png_image_write_to_memory)(
                    &mut im,
                    out.as_mut_ptr() as *mut c_void,
                    &mut sz,
                    0,
                    src.as_ptr() as *mut c_void,
                    0,
                    ptr::null_mut(),
                );
                log_img(&format!("write={ok} sz={sz}"), &im);
                (l.api.png_image_free)(&mut im);
            })
        };
        diff(&format!("D6 PNG too big for buffer cap={cap}"), &c, &r, &mut run);
    }
    // truncated / corrupted memory sources for png_image_begin_read_from_memory
    let good = gen(&c, 8, 4, PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE);
    for n in [0usize, 1, 7, 8, 20, 33, good.len() / 2, good.len() - 1, good.len()] {
        let mut run = |l: &Lib| -> Report {
            scratch(&mut || unsafe {
                let mut im = PngImage::default();
                let ok = (l.api.png_image_begin_read_from_memory)(
                    &mut im,
                    good.as_ptr() as *const c_void,
                    n,
                );
                log_img(&format!("begin={ok}"), &im);
                if ok != 0 {
                    let mut buf = vec![0u8; 1 << 14];
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
            })
        };
        diff(&format!("D6 begin_read_from_memory n={n}"), &c, &r, &mut run);
    }
}

fn scratch(f: &mut dyn FnMut()) -> Report {
    let mut ctxb = Box::new(Ctx::default());
    set_ctx(&mut *ctxb as *mut Ctx);
    f();
    let rep = ctxb.digest();
    set_ctx(ptr::null_mut());
    rep
}

// ===========================================================================
// D-7  png_read_png / png_read_start_row size limits, gamma+background+gray
// ===========================================================================
#[test]
fn d7_size_limits_and_transform_combinations() {
    let (c, r) = libs();
    let good = gen(&c, 8, 4, PNG_COLOR_TYPE_RGB_ALPHA, 16, PNG_INTERLACE_NONE);
    let cs = chunks(&good);
    let ihdr = cs.iter().find(|c| &c.2 == b"IHDR").unwrap();
    // huge declared height -> png_read_png row-pointer array overflow
    for h in [0x1000_0000u32, 0x2000_0000, 0x4000_0000, 0x7fff_ffff] {
        let mut s = good.clone();
        s[ihdr.0 + 12..ihdr.0 + 16].copy_from_slice(&h.to_be_bytes());
        let ci = s[ihdr.0 + 4..ihdr.0 + 8 + ihdr.1].to_vec();
        let v = crc32(&ci).to_be_bytes();
        s[ihdr.0 + 8 + ihdr.1..ihdr.0 + 12 + ihdr.1].copy_from_slice(&v);
        let mut run = |l: &Lib| -> Report {
            read_session(l, s.clone(), &mut |l, png, info| unsafe {
                (l.api.png_set_user_limits)(png, 0x7fff_ffff, 0x7fff_ffff);
                (l.api.png_read_png)(png, info, PNG_TRANSFORM_IDENTITY, ptr::null_mut());
                log("read_png returned".to_string());
            })
        };
        diff(&format!("D7 png_read_png height={h:#x}"), &c, &r, &mut run);
    }
    // huge declared width -> png_read_start_row allocation failure
    for w in [0x0400_0000u32, 0x1000_0000, 0x4000_0000, 0x7fff_ffff] {
        let mut s = good.clone();
        s[ihdr.0 + 8..ihdr.0 + 12].copy_from_slice(&w.to_be_bytes());
        let ci = s[ihdr.0 + 4..ihdr.0 + 8 + ihdr.1].to_vec();
        let v = crc32(&ci).to_be_bytes();
        s[ihdr.0 + 8 + ihdr.1..ihdr.0 + 12 + ihdr.1].copy_from_slice(&v);
        for extra in 0..2u32 {
            let mut run = |l: &Lib| -> Report {
                read_session(l, s.clone(), &mut |l, png, info| unsafe {
                    (l.api.png_set_user_limits)(png, 0x7fff_ffff, 0x7fff_ffff);
                    (l.api.png_read_info)(png, info);
                    if extra == 1 {
                        (l.api.png_set_expand_16)(png);
                        (l.api.png_set_gray_to_rgb)(png);
                    }
                    (l.api.png_start_read_image)(png);
                    log("start_read_image returned".to_string());
                })
            };
            diff(&format!("D7 start_read_image width={w:#x} extra={extra}"), &c, &r, &mut run);
        }
    }
    // gamma + background + rgb_to_gray together
    let rgb = gen(&c, 8, 4, PNG_COLOR_TYPE_RGB_ALPHA, 8, PNG_INTERLACE_NONE);
    for order in 0..6u32 {
        let mut run = |l: &Lib| -> Report {
            read_session(l, rgb.clone(), &mut |l, png, info| unsafe {
                (l.api.png_read_info)(png, info);
                let bg = PngColor16 { index: 0, red: 1, green: 2, blue: 3, gray: 4 };
                let steps: [u32; 3] = match order {
                    0 => [0, 1, 2],
                    1 => [0, 2, 1],
                    2 => [1, 0, 2],
                    3 => [1, 2, 0],
                    4 => [2, 0, 1],
                    _ => [2, 1, 0],
                };
                for s in steps {
                    match s {
                        0 => (l.api.png_set_gamma_fixed)(png, 100000, 45455),
                        1 => (l.api.png_set_background_fixed)(
                            png,
                            &bg,
                            PNG_BACKGROUND_GAMMA_SCREEN,
                            0,
                            100000,
                        ),
                        _ => (l.api.png_set_rgb_to_gray_fixed)(png, PNG_ERROR_ACTION_WARN, -1, -1),
                    }
                }
                (l.api.png_read_update_info)(png, info);
                let h = (l.api.png_get_image_height)(png, info);
                let rb = (l.api.png_get_rowbytes)(png, info);
                let mut buf = vec![0u8; rb + 16];
                for _ in 0..h {
                    (l.api.png_read_row)(png, buf.as_mut_ptr(), ptr::null_mut());
                }
                log(format!("rb={rb} last={:02x?}", &buf[..rb]));
                (l.api.png_read_end)(png, info);
            })
        };
        diff(&format!("D7 gamma+background+rgb_to_gray order={order}"), &c, &r, &mut run);
    }
    // repeated png_build_gamma_table
    let mut run = |l: &Lib| -> Report {
        read_session(l, rgb.clone(), &mut |l, png, info| unsafe {
            (l.api.png_read_info)(png, info);
            (l.api.png_set_gamma_fixed)(png, 100000, 45455);
            (l.pv.png_build_gamma_table)(png, 8);
            (l.pv.png_build_gamma_table)(png, 8);
            (l.pv.png_build_gamma_table)(png, 16);
            (l.pv.png_destroy_gamma_table)(png);
            (l.pv.png_destroy_gamma_table)(png);
            log(format!(
                "resolve_file_gamma={}",
                (l.pv.png_resolve_file_gamma)(png)
            ));
        })
    };
    diff("D7 repeated build_gamma_table", &c, &r, &mut run);
    // png_set_rgb_coefficients directly
    let mut run = |l: &Lib| -> Report {
        read_session(l, rgb.clone(), &mut |l, png, info| unsafe {
            (l.api.png_read_info)(png, info);
            (l.pv.png_set_rgb_coefficients)(png);
            (l.api.png_set_cHRM_fixed)(png, info, 0, 0, 0, 0, 0, 0, 0, 0);
            (l.pv.png_set_rgb_coefficients)(png);
            log("rgb coefficients set".to_string());
        })
    };
    diff("D7 png_set_rgb_coefficients", &c, &r, &mut run);
    // png_ascii_from_fp / _fixed with a buffer that is too small
    let mut run = |l: &Lib| -> Report {
        write_session(l, &mut |l, png, _info| unsafe {
            for size in [0usize, 1, 2, 3, 4, 5, 8, 12] {
                let mut buf = vec![0u8; 64];
                (l.pv.png_ascii_from_fixed)(png, buf.as_mut_ptr() as *mut c_char, size, 123456789);
                log(format!("fixed size={size} -> {:02x?}", &buf[..16]));
            }
        })
    };
    diff("D7 ascii_from_fixed small buffer", &c, &r, &mut run);
    let mut run = |l: &Lib| -> Report {
        write_session(l, &mut |l, png, _info| unsafe {
            for size in [0usize, 1, 2, 3, 4, 5, 8, 12] {
                let mut buf = vec![0u8; 64];
                (l.pv.png_ascii_from_fp)(
                    png,
                    buf.as_mut_ptr() as *mut c_char,
                    size,
                    1.23456789e10,
                    15,
                );
                log(format!("fp size={size} -> {:02x?}", &buf[..16]));
            }
        })
    };
    diff("D7 ascii_from_fp small buffer", &c, &r, &mut run);
    // png_zalloc overflow guard and png_zfree
    let mut run = |l: &Lib| -> Report {
        write_session(l, &mut |l, png, _info| unsafe {
            for (items, size) in [
                (0u32, 0u32),
                (1, 1),
                (1, 65535),
                (65535, 1),
                (65535, 65535),
                (0xffff, 0x10000),
            ] {
                let p = (l.pv.png_zalloc)(png, items, size);
                log(format!("zalloc({items},{size}) null={}", p.is_null()));
                if !p.is_null() {
                    (l.pv.png_zfree)(png, p);
                }
            }
            (l.pv.png_zfree)(png, ptr::null_mut());
        })
    };
    diff("D7 png_zalloc/png_zfree", &c, &r, &mut run);
    // png_realloc_array with a bogus old_elements count
    let mut run = |l: &Lib| -> Report {
        write_session(l, &mut |l, png, _info| unsafe {
            for (old, add) in [
                (0i32, 1i32),
                (1, 0),
                (1, -1),
                (-1, 1),
                (i32::MAX, 1),
                (1, i32::MAX),
            ] {
                let p = (l.pv.png_realloc_array)(png, ptr::null(), old, add, 8);
                log(format!("realloc_array({old},{add}) null={}", p.is_null()));
                if !p.is_null() {
                    (l.api.png_free)(png, p);
                }
            }
        })
    };
    diff("D7 png_realloc_array bogus counts", &c, &r, &mut run);
    // png_write_start_row / png_write_finish_row misuse
    let mut run = |l: &Lib| -> Report {
        write_session(l, &mut |l, png, info| unsafe {
            (l.api.png_set_IHDR)(
                png, info, 4, 4, 8, PNG_COLOR_TYPE_RGB, PNG_INTERLACE_NONE,
                PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
            );
            (l.api.png_write_info)(png, info);
            (l.pv.png_write_start_row)(png);
            (l.pv.png_write_finish_row)(png);
            log("start/finish row returned".to_string());
        })
    };
    diff("D7 png_write_start_row/finish_row", &c, &r, &mut run);
    // png_read_IDAT_data with no IDAT
    let mut run = |l: &Lib| -> Report {
        read_session(l, vec![], &mut |l, png, _info| unsafe {
            let mut buf = vec![0u8; 32];
            (l.pv.png_read_IDAT_data)(png, buf.as_mut_ptr(), 32);
            log("read_IDAT_data returned".to_string());
        })
    };
    diff("D7 png_read_IDAT_data with no IDAT", &c, &r, &mut run);
    // png_compress_IDAT without a write struct set up
    let mut run = |l: &Lib| -> Report {
        write_session(l, &mut |l, png, _info| unsafe {
            let d = [1u8, 2, 3, 4];
            (l.pv.png_compress_IDAT)(png, d.as_ptr(), 4, 0);
            (l.pv.png_compress_IDAT)(png, ptr::null(), 0, 4);
            log("compress_IDAT returned".to_string());
        })
    };
    diff("D7 png_compress_IDAT", &c, &r, &mut run);
    // png_handle_unknown called directly with every keep value
    for keep in [-1i32, 0, 1, 2, 3, 4, 99] {
        for len in [0u32, 4, 100] {
            let mut run = |l: &Lib| -> Report {
                read_session(l, vec![0u8; 200], &mut |l, png, info| unsafe {
                    log(format!(
                        "handle_unknown={}",
                        (l.pv.png_handle_unknown)(png, info, len, keep)
                    ));
                })
            };
            diff(&format!("D7 png_handle_unknown keep={keep} len={len}"), &c, &r, &mut run);
        }
    }
}
