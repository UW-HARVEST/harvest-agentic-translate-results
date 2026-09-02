//! Phase C: error-path differential tests.  Each family constructs an invalid
//! input or an illegal call sequence and asserts that BOTH implementations
//! reject it with the SAME error/warning text (or the same sentinel), not merely
//! that both failed.
mod common;
use common::*;
use std::ffi::{c_int, c_void, CString};
use std::ptr;

const SEED: u64 = 0xE770_0001_0002_0003;

const LEGAL: &[(c_int, c_int)] = &[
    (0, 1),
    (0, 2),
    (0, 4),
    (0, 8),
    (0, 16),
    (3, 1),
    (3, 2),
    (3, 4),
    (3, 8),
    (2, 8),
    (2, 16),
    (4, 8),
    (4, 16),
    (6, 8),
    (6, 16),
];

fn pal_for(bd: c_int) -> usize {
    match bd {
        1 => 2,
        2 => 4,
        4 => 16,
        _ => 256,
    }
}

fn gen(cl: &Lib, w: u32, h: u32, ct: c_int, bd: c_int, il: c_int) -> Vec<u8> {
    let pal = if ct == PNG_COLOR_TYPE_PALETTE {
        make_palette(pal_for(bd), SEED ^ 7)
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

/// A rich stream carrying every ancillary chunk this build understands.
fn rich_stream(cl: &Lib) -> Vec<u8> {
    let purpose = CString::new("purpose").unwrap();
    let units = CString::new("m").unwrap();
    let iccname = CString::new("prof").unwrap();
    let key = CString::new("Title").unwrap();
    let txt = CString::new("value").unwrap();
    let sw = CString::new("1.5").unwrap();
    let sh = CString::new("2.5").unwrap();
    let mut prof = vec![0u8; 132];
    prof[0..4].copy_from_slice(&132u32.to_be_bytes());
    prof[4..8].copy_from_slice(b"ADBE");
    prof[8..12].copy_from_slice(&0x0200_0000u32.to_be_bytes());
    prof[12..16].copy_from_slice(b"mntr");
    prof[16..20].copy_from_slice(b"RGB ");
    prof[20..24].copy_from_slice(b"XYZ ");
    prof[36..40].copy_from_slice(b"acsp");
    prof[68..72].copy_from_slice(&0x0000_f6d6u32.to_be_bytes());
    prof[72..76].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    prof[76..80].copy_from_slice(&0x0000_d32du32.to_be_bytes());
    let pal = make_palette(256, SEED ^ 11);
    write_full(
        cl,
        8,
        4,
        PNG_COLOR_TYPE_PALETTE,
        8,
        PNG_INTERLACE_NONE,
        PNG_FILTER_TYPE_BASE,
        &pal,
        8,
        SEED ^ 13,
        &mut |l, png, info| unsafe {
            (l.api.png_set_gAMA_fixed)(png, info, 45455);
            (l.api.png_set_cHRM_fixed)(
                png, info, 31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000,
            );
            let sig = PngColor8 { red: 8, green: 8, blue: 8, gray: 8, alpha: 8 };
            (l.api.png_set_sBIT)(png, info, &sig);
            let bg = PngColor16 { index: 1, red: 0, green: 0, blue: 0, gray: 0 };
            (l.api.png_set_bKGD)(png, info, &bg);
            let alpha: Vec<u8> = (0..256u32).map(|i| i as u8).collect();
            (l.api.png_set_tRNS)(png, info, alpha.as_ptr(), 256, ptr::null());
            let hist: Vec<u16> = (0..256u32).map(|i| i as u16).collect();
            (l.api.png_set_hIST)(png, info, hist.as_ptr());
            (l.api.png_set_pHYs)(png, info, 400, 500, 1);
            (l.api.png_set_oFFs)(png, info, -7, 9, 1);
            (l.api.png_set_pCAL)(
                png, info, purpose.as_ptr(), 0, 255, 0, 0, units.as_ptr(), ptr::null_mut(),
            );
            (l.api.png_set_sCAL_s)(png, info, 1, sw.as_ptr(), sh.as_ptr());
            let t = PngTime { year: 2021, month: 3, day: 4, hour: 5, minute: 6, second: 7 };
            (l.api.png_set_tIME)(png, info, &t);
            (l.api.png_set_iCCP)(png, info, iccname.as_ptr(), 0, prof.as_ptr(), 132);
            let tt = PngText {
                compression: -1,
                key: key.as_ptr() as *mut i8,
                text: txt.as_ptr() as *mut i8,
                text_length: 5,
                ..Default::default()
            };
            (l.api.png_set_text)(png, info, &tt, 1);
            (l.api.png_set_cICP)(png, info, 9, 16, 0, 1);
            (l.api.png_set_cLLI_fixed)(png, info, 10_000_000, 4_000_000);
            (l.api.png_set_mDCV_fixed)(
                png, info, 31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000, 10_000_000, 500,
            );
            let exif = b"II*\0\x08\0\0\0";
            (l.api.png_set_eXIf_1)(png, info, 8, exif.as_ptr() as *mut u8);
            let payload = [1u8, 2, 3];
            let unk = [PngUnknownChunk {
                name: *b"prVt\0",
                data: payload.as_ptr() as *mut u8,
                size: 3,
                location: PNG_HAVE_IHDR as u8,
            }];
            (l.api.png_set_keep_unknown_chunks)(png, PNG_HANDLE_CHUNK_ALWAYS, ptr::null(), 0);
            (l.api.png_set_unknown_chunks)(png, info, unk.as_ptr(), 1);
            (l.api.png_set_unknown_chunk_location)(png, info, 0, PNG_HAVE_IHDR);
        },
    )
    .out
}

/// Read a stream and record everything observable, tolerating errors.
fn try_read(l: &Lib, stream: Vec<u8>) -> Report {
    read_session(l, stream, &mut |l, png, info| unsafe {
        (l.api.png_read_info)(png, info);
        let h = (l.api.png_get_image_height)(png, info);
        let il = (l.api.png_get_interlace_type)(png, info);
        log(format!(
            "info {}x{} bd={} ct={} il={il}",
            (l.api.png_get_image_width)(png, info),
            h,
            (l.api.png_get_bit_depth)(png, info),
            (l.api.png_get_color_type)(png, info)
        ));
        let passes = if il == 1 {
            (l.api.png_set_interlace_handling)(png)
        } else {
            1
        };
        (l.api.png_read_update_info)(png, info);
        let rb = (l.api.png_get_rowbytes)(png, info);
        let mut buf = vec![0u8; rb + 16];
        for _ in 0..passes {
            for i in 0..h {
                (l.api.png_read_row)(png, buf.as_mut_ptr(), ptr::null_mut());
                log(format!("row{i}={:02x?}", &buf[..rb]));
            }
        }
        (l.api.png_read_end)(png, info);
        log("read_end ok".to_string());
    })
}

// ===========================================================================
// C-1  Signature / container level rejections
// ===========================================================================
#[test]
fn c1_bad_signature_and_truncation() {
    let (c, r) = libs();
    let good = gen(&c, 8, 4, PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE);

    // empty input, and every truncation length
    for n in 0..good.len() {
        let s = good[..n].to_vec();
        let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
        diff(&format!("C1 truncated at {n}"), &c, &r, &mut run);
    }
    // corrupt each signature byte
    for i in 0..8 {
        let mut s = good.clone();
        s[i] ^= 0xff;
        let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
        diff(&format!("C1 bad signature byte {i}"), &c, &r, &mut run);
    }
    // garbage of various lengths
    let mut rng = Rng::new(SEED ^ 0x1111);
    for n in [1usize, 4, 8, 16, 64, 200] {
        let s = rng.bytes(n);
        let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
        diff(&format!("C1 garbage len={n}"), &c, &r, &mut run);
    }
    // all zeroes
    for n in [8usize, 32, 100] {
        let s = vec![0u8; n];
        let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
        diff(&format!("C1 zeroes len={n}"), &c, &r, &mut run);
    }
}

// ===========================================================================
// C-2  Chunk-level rejections: CRC, length, ordering, duplication
// ===========================================================================
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

/// Split a datastream into (offset, length, type) triples.
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

#[test]
fn c2_crc_and_length_corruption() {
    let (c, r) = libs();
    let good = rich_stream(&c);
    let cs = chunks(&good);
    assert!(cs.len() > 10, "expected a rich stream, got {} chunks", cs.len());
    for &(off, len, ty) in &cs {
        let name = String::from_utf8_lossy(&ty).into_owned();
        // (a) corrupt the CRC
        let mut s = good.clone();
        s[off + 8 + len] ^= 0x01;
        let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
        diff(&format!("C2 bad CRC in {name}"), &c, &r, &mut run);

        // (b) corrupt one data byte (keeps the length, breaks the CRC and the
        // chunk semantics)
        if len > 0 {
            let mut s = good.clone();
            s[off + 8] ^= 0xff;
            let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
            diff(&format!("C2 corrupt first data byte of {name}"), &c, &r, &mut run);
            let mut s = good.clone();
            s[off + 8 + len - 1] ^= 0xff;
            let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
            diff(&format!("C2 corrupt last data byte of {name}"), &c, &r, &mut run);
        }

        // (c) declare an absurd length
        for &bad in &[
            0u32,
            1,
            (len as u32).wrapping_sub(1),
            (len as u32) + 1,
            0x7fff_ffff,
            0x8000_0000,
            0xffff_ffff,
        ] {
            let mut s = good.clone();
            s[off..off + 4].copy_from_slice(&bad.to_be_bytes());
            let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
            diff(&format!("C2 {name} length={bad:#x}"), &c, &r, &mut run);
        }

        // (d) drop the chunk entirely
        let mut s = good[..off].to_vec();
        s.extend_from_slice(&good[off + 12 + len..]);
        let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
        diff(&format!("C2 missing {name}"), &c, &r, &mut run);

        // (e) duplicate the chunk
        let mut s = good[..off + 12 + len].to_vec();
        s.extend_from_slice(&good[off..off + 12 + len]);
        s.extend_from_slice(&good[off + 12 + len..]);
        let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
        diff(&format!("C2 duplicate {name}"), &c, &r, &mut run);

        // (f) truncate the stream in the middle of the chunk
        let mut run = |l: &Lib| -> Report { try_read(l, good[..off + 8].to_vec()) };
        diff(&format!("C2 truncate before {name} data"), &c, &r, &mut run);
    }
}

#[test]
fn c2b_chunk_ordering_violations() {
    let (c, r) = libs();
    let good = rich_stream(&c);
    let cs = chunks(&good);
    // Move every chunk to the very end (after IEND) and to right after the
    // signature (before IHDR).
    for &(off, len, ty) in &cs {
        let name = String::from_utf8_lossy(&ty).into_owned();
        let chunk = good[off..off + 12 + len].to_vec();
        let mut without = good[..off].to_vec();
        without.extend_from_slice(&good[off + 12 + len..]);

        let mut s = without.clone();
        s.extend_from_slice(&chunk);
        let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
        diff(&format!("C2b {name} after IEND"), &c, &r, &mut run);

        let mut s = without[..8].to_vec();
        s.extend_from_slice(&chunk);
        s.extend_from_slice(&without[8..]);
        let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
        diff(&format!("C2b {name} before IHDR"), &c, &r, &mut run);
    }
    // A stream with no IHDR at all, and no IEND at all
    let ihdr = cs.iter().find(|c| &c.2 == b"IHDR").unwrap();
    let mut s = good[..8].to_vec();
    s.extend_from_slice(&good[ihdr.0 + 12 + ihdr.1..]);
    let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
    diff("C2b missing IHDR", &c, &r, &mut run);

    let iend = cs.iter().find(|c| &c.2 == b"IEND").unwrap();
    let s = good[..iend.0].to_vec();
    let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
    diff("C2b missing IEND", &c, &r, &mut run);
}

// ===========================================================================
// C-3  Invalid IHDR fields on read
// ===========================================================================
#[test]
fn c3_invalid_ihdr_on_read() {
    let (c, r) = libs();
    let good = gen(&c, 8, 4, PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE);
    let cs = chunks(&good);
    let ihdr = cs.iter().find(|c| &c.2 == b"IHDR").unwrap();
    let d = ihdr.0 + 8; // start of IHDR data
    // width, height
    for (field, off) in [("width", 0usize), ("height", 4)] {
        for &v in &[0u32, 1, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff] {
            let mut s = good.clone();
            s[d + off..d + off + 4].copy_from_slice(&v.to_be_bytes());
            fix_crc(&mut s, ihdr.0, ihdr.1);
            let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
            diff(&format!("C3 IHDR {field}={v:#x}"), &c, &r, &mut run);
        }
    }
    // bit depth, colour type, compression, filter, interlace
    for (field, off, vals) in [
        ("bit_depth", 8usize, vec![0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9, 16, 17, 32, 255]),
        ("color_type", 9, vec![0u8, 1, 2, 3, 4, 5, 6, 7, 8, 255]),
        ("compression", 10, vec![0u8, 1, 2, 255]),
        ("filter", 11, vec![0u8, 1, 64, 255]),
        ("interlace", 12, vec![0u8, 1, 2, 3, 255]),
    ] {
        for v in vals {
            let mut s = good.clone();
            s[d + off] = v;
            fix_crc(&mut s, ihdr.0, ihdr.1);
            let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
            diff(&format!("C3 IHDR {field}={v}"), &c, &r, &mut run);
        }
    }
    // IHDR with the wrong length
    for extra in [0usize, 1, 5] {
        let mut data = good[d..d + ihdr.1].to_vec();
        data.truncate(13 - extra.min(13));
        let mut s = good[..ihdr.0].to_vec();
        s.extend_from_slice(&make_chunk(b"IHDR", &data));
        s.extend_from_slice(&good[ihdr.0 + 12 + ihdr.1..]);
        let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
        diff(&format!("C3 IHDR short by {extra}"), &c, &r, &mut run);
    }
}

fn fix_crc(s: &mut [u8], off: usize, len: usize) {
    let ci = s[off + 4..off + 8 + len].to_vec();
    let v = crc32(&ci).to_be_bytes();
    s[off + 8 + len..off + 12 + len].copy_from_slice(&v);
}

// ===========================================================================
// C-4  Malformed ancillary chunk payloads (one crafted chunk per handler)
// ===========================================================================
#[test]
fn c4_malformed_ancillary_payloads() {
    let (c, r) = libs();
    let base = gen(&c, 8, 4, PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE);
    let cs = chunks(&base);
    let idat = cs.iter().find(|c| &c.2 == b"IDAT").unwrap().0;

    let mut cases: Vec<(&str, Vec<u8>)> = Vec::new();
    // gAMA: must be 4 bytes
    for n in [0usize, 1, 3, 4, 5, 8] {
        cases.push(("gAMA", vec![0u8; n]));
    }
    cases.push(("gAMA", 0u32.to_be_bytes().to_vec()));
    cases.push(("gAMA", 0x8000_0000u32.to_be_bytes().to_vec()));
    cases.push(("gAMA", 0xffff_ffffu32.to_be_bytes().to_vec()));
    // cHRM: 32 bytes
    for n in [0usize, 4, 31, 32, 33] {
        cases.push(("cHRM", vec![0u8; n]));
    }
    // sRGB: 1 byte, value 0..3
    for v in [0u8, 1, 2, 3, 4, 255] {
        cases.push(("sRGB", vec![v]));
    }
    cases.push(("sRGB", vec![]));
    cases.push(("sRGB", vec![0, 0]));
    // sBIT
    for n in [0usize, 1, 2, 3, 4, 5] {
        cases.push(("sBIT", vec![8u8; n]));
    }
    cases.push(("sBIT", vec![0u8, 8, 8]));
    cases.push(("sBIT", vec![9u8, 8, 8]));
    // bKGD
    for n in [0usize, 1, 2, 3, 6, 7] {
        cases.push(("bKGD", vec![0u8; n]));
    }
    // tRNS
    for n in [0usize, 1, 2, 5, 6, 7] {
        cases.push(("tRNS", vec![0u8; n]));
    }
    // pHYs: 9 bytes
    for n in [0usize, 8, 9, 10] {
        cases.push(("pHYs", vec![0u8; n]));
    }
    cases.push(("pHYs", {
        let mut v = 100u32.to_be_bytes().to_vec();
        v.extend_from_slice(&200u32.to_be_bytes());
        v.push(9);
        v
    }));
    // oFFs: 9 bytes
    for n in [0usize, 8, 9, 10] {
        cases.push(("oFFs", vec![0u8; n]));
    }
    // tIME: 7 bytes
    for n in [0usize, 6, 7, 8] {
        cases.push(("tIME", vec![1u8; n]));
    }
    cases.push(("tIME", vec![7, 199, 13, 32, 24, 60, 61]));
    // sCAL
    cases.push(("sCAL", vec![]));
    cases.push(("sCAL", vec![1]));
    cases.push(("sCAL", b"\x01".to_vec()));
    cases.push(("sCAL", b"\x011.0\x002.0\x00".to_vec()));
    cases.push(("sCAL", b"\x011.0\x002.0".to_vec()));
    cases.push(("sCAL", b"\x031.0\x002.0".to_vec()));
    cases.push(("sCAL", b"\x01-1.0\x002.0".to_vec()));
    cases.push(("sCAL", b"\x01abc\x00def".to_vec()));
    // pCAL
    cases.push(("pCAL", vec![]));
    cases.push(("pCAL", b"p\x00".to_vec()));
    cases.push(("pCAL", b"p\x00\x00\x00\x00\x00\x00\x00\x00\xff\x00u\x00".to_vec()));
    cases.push(("pCAL", b"p\x00\x00\x00\x00\x00\x00\x00\x00\xff\x00\x02u\x00".to_vec()));
    cases.push(("pCAL", b"p\x00\x00\x00\x00\x00\x00\x00\x00\xff\x00\x00\x63u\x00".to_vec()));
    // hIST (needs PLTE; on an RGB stream it is invalid)
    cases.push(("hIST", vec![0u8; 4]));
    cases.push(("hIST", vec![0u8; 3]));
    // PLTE on an RGB image
    cases.push(("PLTE", vec![0u8; 3]));
    cases.push(("PLTE", vec![0u8; 4]));
    cases.push(("PLTE", vec![]));
    cases.push(("PLTE", vec![0u8; 3 * 257]));
    // sPLT
    cases.push(("sPLT", vec![]));
    cases.push(("sPLT", b"n\x00\x08".to_vec()));
    cases.push(("sPLT", b"n\x00\x08\x00\x00\x00\x00\x00\x00".to_vec()));
    cases.push(("sPLT", b"n\x00\x10".to_vec()));
    cases.push(("sPLT", b"n\x00\x07".to_vec()));
    // tEXt / zTXt / iTXt
    cases.push(("tEXt", vec![]));
    cases.push(("tEXt", b"key".to_vec()));
    cases.push(("tEXt", b"key\x00text".to_vec()));
    cases.push(("tEXt", b"\x00text".to_vec()));
    cases.push(("tEXt", b" key \x00text".to_vec()));
    cases.push(("zTXt", vec![]));
    cases.push(("zTXt", b"key\x00".to_vec()));
    cases.push(("zTXt", b"key\x00\x00garbage".to_vec()));
    cases.push(("zTXt", b"key\x00\x01garbage".to_vec()));
    cases.push(("iTXt", vec![]));
    cases.push(("iTXt", b"key\x00\x00\x00en\x00tk\x00text".to_vec()));
    cases.push(("iTXt", b"key\x00\x01\x00en\x00tk\x00garbage".to_vec()));
    cases.push(("iTXt", b"key\x00\x00\x01en\x00tk\x00text".to_vec()));
    cases.push(("iTXt", b"key\x00\x02\x00en\x00tk\x00text".to_vec()));
    // iCCP
    cases.push(("iCCP", vec![]));
    cases.push(("iCCP", b"n\x00\x00".to_vec()));
    cases.push(("iCCP", b"n\x00\x01deadbeef".to_vec()));
    cases.push(("iCCP", b"n\x00\x00deadbeef".to_vec()));
    // eXIf
    cases.push(("eXIf", vec![]));
    cases.push(("eXIf", b"II".to_vec()));
    cases.push(("eXIf", b"II*\x00".to_vec()));
    cases.push(("eXIf", b"XX*\x00".to_vec()));
    // cICP: 4 bytes
    for n in [0usize, 3, 4, 5] {
        cases.push(("cICP", vec![1u8; n]));
    }
    cases.push(("cICP", vec![9, 16, 0, 2]));
    // cLLI: 8 bytes
    for n in [0usize, 7, 8, 9] {
        cases.push(("cLLI", vec![0u8; n]));
    }
    cases.push(("cLLI", vec![0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0]));
    // mDCV: 24 bytes
    for n in [0usize, 23, 24, 25] {
        cases.push(("mDCV", vec![0u8; n]));
    }
    // IEND with data
    cases.push(("IEND", vec![1u8, 2, 3]));
    // unknown critical / ancillary chunks
    cases.push(("PRVT", vec![1u8, 2, 3]));
    cases.push(("prVt", vec![1u8, 2, 3]));
    cases.push(("pRVt", vec![1u8, 2, 3]));
    cases.push(("PrVt", vec![1u8, 2, 3]));

    for (i, (ty, data)) in cases.iter().enumerate() {
        let mut s = base[..idat].to_vec();
        s.extend_from_slice(&make_chunk(ty.as_bytes(), data));
        s.extend_from_slice(&base[idat..]);
        let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
        diff(
            &format!("C4 #{i} {ty} len={} before IDAT", data.len()),
            &c,
            &r,
            &mut run,
        );
        // ...and again with benign errors disallowed, which promotes the
        // chunk_benign_error rows from warnings to fatal errors
        let mut run = |l: &Lib| -> Report {
            read_session(l, s.clone(), &mut |l, png, info| unsafe {
                (l.api.png_set_benign_errors)(png, 0);
                (l.api.png_read_info)(png, info);
                log("read_info ok".to_string());
                (l.api.png_read_end)(png, info);
            })
        };
        diff(
            &format!("C4 #{i} {ty} len={} benign=0", data.len()),
            &c,
            &r,
            &mut run,
        );
    }
}

// ===========================================================================
// C-5  IDAT / zlib stream corruption
// ===========================================================================
#[test]
fn c5_idat_corruption() {
    let (c, r) = libs();
    let good = gen(&c, 24, 8, PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE);
    let cs = chunks(&good);
    let (off, len, _) = *cs.iter().find(|c| &c.2 == b"IDAT").unwrap();
    for i in 0..len.min(24) {
        let mut s = good.clone();
        s[off + 8 + i] ^= 0xff;
        fix_crc(&mut s, off, len);
        let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
        diff(&format!("C5 IDAT byte {i} flipped"), &c, &r, &mut run);
    }
    // truncate the IDAT payload
    for keep in [0usize, 1, 2, len / 2, len - 1] {
        let mut s = good[..off].to_vec();
        s.extend_from_slice(&make_chunk(b"IDAT", &good[off + 8..off + 8 + keep]));
        s.extend_from_slice(&good[off + 12 + len..]);
        let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
        diff(&format!("C5 IDAT truncated to {keep}"), &c, &r, &mut run);
    }
    // corrupt the zlib header (first two bytes) with every value combination
    for hi in [0x00u8, 0x08, 0x18, 0x78, 0x88, 0xff] {
        for lo in [0x00u8, 0x01, 0x5e, 0x9c, 0xda, 0xff] {
            let mut s = good.clone();
            s[off + 8] = hi;
            s[off + 9] = lo;
            fix_crc(&mut s, off, len);
            let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
            diff(&format!("C5 zlib header {hi:#02x}{lo:02x}"), &c, &r, &mut run);
            // ...and with the adler32 check disabled
            let mut run = |l: &Lib| -> Report {
                read_session(l, s.clone(), &mut |l, png, info| unsafe {
                    (l.api.png_set_option)(png, PNG_IGNORE_ADLER32, PNG_OPTION_ON);
                    (l.api.png_read_info)(png, info);
                    let h = (l.api.png_get_image_height)(png, info);
                    let rb = (l.api.png_get_rowbytes)(png, info);
                    let mut buf = vec![0u8; rb + 8];
                    for _ in 0..h {
                        (l.api.png_read_row)(png, buf.as_mut_ptr(), ptr::null_mut());
                    }
                    (l.api.png_read_end)(png, info);
                })
            };
            diff(
                &format!("C5 zlib header {hi:#02x}{lo:02x} ignore_adler32"),
                &c,
                &r,
                &mut run,
            );
        }
    }
    // no IDAT at all
    let mut s = good[..off].to_vec();
    s.extend_from_slice(&good[off + 12 + len..]);
    let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
    diff("C5 no IDAT", &c, &r, &mut run);
    // too few / too many rows of image data: shrink or grow the declared height
    let ihdr = cs.iter().find(|c| &c.2 == b"IHDR").unwrap();
    for h in [1u32, 4, 7, 9, 16] {
        let mut s = good.clone();
        s[ihdr.0 + 8 + 4..ihdr.0 + 8 + 8].copy_from_slice(&h.to_be_bytes());
        fix_crc(&mut s, ihdr.0, ihdr.1);
        let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
        diff(&format!("C5 declared height={h}"), &c, &r, &mut run);
    }
    // corrupt the filter byte of the first row: values 0..255 through a
    // recompressed IDAT is not possible without deflate, so instead exercise
    // png_read_filter_row's out-of-range filter directly
    let mut run = |l: &Lib| -> Report {
        read_session(l, vec![], &mut |l, png, _info| unsafe {
            let mut ri = PngRowInfo {
                width: 4,
                rowbytes: 12,
                color_type: 2,
                bit_depth: 8,
                channels: 3,
                pixel_depth: 24,
            };
            let mut row = vec![0x5au8; 13];
            let prev = vec![0xa5u8; 13];
            for f in [-1i32, 0, 1, 2, 3, 4, 5, 6, 100] {
                let mut rr = row.clone();
                (l.pv.png_read_filter_row)(png, &mut ri, rr.as_mut_ptr(), prev.as_ptr(), f);
                log(format!("filter {f} -> {:02x?}", rr));
            }
            row.clear();
        })
    };
    diff("C5 png_read_filter_row out-of-range filter", &c, &r, &mut run);
}

// ===========================================================================
// C-6  Randomized stream mutation (property style, fixed seed)
// ===========================================================================
#[test]
fn c6_random_stream_mutation() {
    let (c, r) = libs();
    let bases: Vec<Vec<u8>> = vec![
        gen(&c, 8, 4, PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE),
        gen(&c, 9, 5, PNG_COLOR_TYPE_PALETTE, 4, PNG_INTERLACE_ADAM7),
        gen(&c, 7, 3, PNG_COLOR_TYPE_GRAY, 16, PNG_INTERLACE_NONE),
        gen(&c, 6, 6, PNG_COLOR_TYPE_RGB_ALPHA, 16, PNG_INTERLACE_ADAM7),
        rich_stream(&c),
    ];
    let mut rng = Rng::new(SEED ^ 0x6666);
    for (bi, base) in bases.iter().enumerate() {
        for k in 0..300u32 {
            let mut s = base.clone();
            let nmut = 1 + rng.below(4);
            for _ in 0..nmut {
                let i = (rng.below(s.len() as u32)) as usize;
                match rng.below(3) {
                    0 => s[i] ^= 1 << (rng.below(8)),
                    1 => s[i] = rng.u8(),
                    _ => s[i] = if rng.u32() & 1 == 0 { 0x00 } else { 0xff },
                }
            }
            let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
            diff(&format!("C6 mutate base={bi} #{k}"), &c, &r, &mut run);
        }
        // truncations at random offsets
        for k in 0..60u32 {
            let n = (rng.below(base.len() as u32 + 1)) as usize;
            let s = base[..n].to_vec();
            let mut run = |l: &Lib| -> Report { try_read(l, s.clone()) };
            diff(&format!("C6 truncate base={bi} #{k} n={n}"), &c, &r, &mut run);
        }
    }
}

// ===========================================================================
// C-7  Illegal call sequences on write
// ===========================================================================
#[test]
fn c7_write_sequence_violations() {
    let (c, r) = libs();
    let steps: &[(&str, u32)] = &[
        ("write_row before write_info", 0),
        ("write_end before write_info", 1),
        ("write_info twice", 2),
        ("write_info without IHDR", 3),
        ("too few rows then write_end", 4),
        ("too many rows", 5),
        ("set_IHDR twice", 6),
        ("PLTE missing for palette image", 7),
        ("PLTE on non-palette image", 8),
        ("set_rows NULL then write_png", 9),
        ("write_row after write_end", 10),
        ("write_image before write_info", 11),
        ("interlace handling without ADAM7", 12),
        ("write_chunk before sig", 13),
        ("set_filter after write_info", 14),
        ("set_gAMA after write_info", 15),
        ("write_end twice", 16),
        ("flush without WRITE_FLUSH state", 17),
    ];
    for &(name, which) in steps {
        let mut run = |l: &Lib| -> Report {
            write_session(l, &mut |l, png, info| unsafe {
                let (w, h) = (6u32, 3u32);
                let rb = rowbytes(w, 8, PNG_COLOR_TYPE_RGB);
                let rows = make_rows(h as usize, rb, SEED ^ which as u64);
                let ihdr = |ct: c_int, il: c_int| {
                    (l.api.png_set_IHDR)(
                        png, info, w, h, 8, ct, il, PNG_COMPRESSION_TYPE_BASE,
                        PNG_FILTER_TYPE_BASE,
                    )
                };
                match which {
                    0 => {
                        ihdr(PNG_COLOR_TYPE_RGB, PNG_INTERLACE_NONE);
                        (l.api.png_write_row)(png, rows[0].as_ptr());
                    }
                    1 => {
                        ihdr(PNG_COLOR_TYPE_RGB, PNG_INTERLACE_NONE);
                        (l.api.png_write_end)(png, info);
                    }
                    2 => {
                        ihdr(PNG_COLOR_TYPE_RGB, PNG_INTERLACE_NONE);
                        (l.api.png_write_info)(png, info);
                        (l.api.png_write_info)(png, info);
                    }
                    3 => {
                        (l.api.png_write_info)(png, info);
                    }
                    4 => {
                        ihdr(PNG_COLOR_TYPE_RGB, PNG_INTERLACE_NONE);
                        (l.api.png_write_info)(png, info);
                        (l.api.png_write_row)(png, rows[0].as_ptr());
                        (l.api.png_write_end)(png, info);
                    }
                    5 => {
                        ihdr(PNG_COLOR_TYPE_RGB, PNG_INTERLACE_NONE);
                        (l.api.png_write_info)(png, info);
                        for _ in 0..(h + 2) {
                            (l.api.png_write_row)(png, rows[0].as_ptr());
                        }
                        (l.api.png_write_end)(png, info);
                    }
                    6 => {
                        ihdr(PNG_COLOR_TYPE_RGB, PNG_INTERLACE_NONE);
                        ihdr(PNG_COLOR_TYPE_GRAY, PNG_INTERLACE_NONE);
                        (l.api.png_write_info)(png, info);
                    }
                    7 => {
                        ihdr(PNG_COLOR_TYPE_PALETTE, PNG_INTERLACE_NONE);
                        (l.api.png_write_info)(png, info);
                    }
                    8 => {
                        ihdr(PNG_COLOR_TYPE_RGB, PNG_INTERLACE_NONE);
                        let pal = make_palette(4, 1);
                        (l.api.png_set_PLTE)(png, info, pal.as_ptr(), 4);
                        (l.api.png_write_info)(png, info);
                    }
                    9 => {
                        ihdr(PNG_COLOR_TYPE_RGB, PNG_INTERLACE_NONE);
                        (l.api.png_set_rows)(png, info, ptr::null_mut());
                        (l.api.png_write_png)(png, info, PNG_TRANSFORM_IDENTITY, ptr::null_mut());
                    }
                    10 => {
                        // NOTE: png_write_row(png, NULL) is NOT tested: the C
                        // memcpy()s from the row pointer with no NULL check
                        // (verified: the reference C .so segfaults), so it is
                        // undefined behaviour rather than a rejection.
                        ihdr(PNG_COLOR_TYPE_RGB, PNG_INTERLACE_NONE);
                        (l.api.png_write_info)(png, info);
                        for row in &rows {
                            (l.api.png_write_row)(png, row.as_ptr());
                        }
                        (l.api.png_write_end)(png, info);
                        (l.api.png_write_row)(png, rows[0].as_ptr());
                    }
                    11 => {
                        // NOTE: png_write_image(png, NULL) is NOT tested: the C
                        // iterates `for (rp = image; ...)` with no NULL check
                        // (verified: the reference C .so segfaults).
                        ihdr(PNG_COLOR_TYPE_RGB, PNG_INTERLACE_NONE);
                        let mut ptrs: Vec<*mut u8> =
                            rows.iter().map(|v| v.as_ptr() as *mut u8).collect();
                        (l.api.png_write_image)(png, ptrs.as_mut_ptr());
                    }
                    12 => {
                        ihdr(PNG_COLOR_TYPE_RGB, PNG_INTERLACE_NONE);
                        (l.api.png_write_info)(png, info);
                        log(format!(
                            "passes={}",
                            (l.api.png_set_interlace_handling)(png)
                        ));
                    }
                    13 => {
                        (l.api.png_write_chunk)(png, b"prVt".as_ptr(), ptr::null(), 0);
                    }
                    14 => {
                        ihdr(PNG_COLOR_TYPE_RGB, PNG_INTERLACE_NONE);
                        (l.api.png_write_info)(png, info);
                        (l.api.png_set_filter)(png, PNG_FILTER_TYPE_BASE, PNG_ALL_FILTERS);
                    }
                    15 => {
                        ihdr(PNG_COLOR_TYPE_RGB, PNG_INTERLACE_NONE);
                        (l.api.png_write_info)(png, info);
                        (l.api.png_set_gAMA_fixed)(png, info, 45455);
                        for row in &rows {
                            (l.api.png_write_row)(png, row.as_ptr());
                        }
                        (l.api.png_write_end)(png, info);
                    }
                    16 => {
                        ihdr(PNG_COLOR_TYPE_RGB, PNG_INTERLACE_NONE);
                        (l.api.png_write_info)(png, info);
                        for row in &rows {
                            (l.api.png_write_row)(png, row.as_ptr());
                        }
                        (l.api.png_write_end)(png, info);
                        (l.api.png_write_end)(png, info);
                    }
                    _ => {
                        ihdr(PNG_COLOR_TYPE_RGB, PNG_INTERLACE_NONE);
                        (l.api.png_write_flush)(png);
                    }
                }
                log("sequence completed".to_string());
            })
        };
        diff(&format!("C7 {name}"), &c, &r, &mut run);
    }
}

// ===========================================================================
// C-8  Illegal call sequences on read
// ===========================================================================
#[test]
fn c8_read_sequence_violations() {
    let (c, r) = libs();
    let stream = gen(&c, 6, 3, PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE);
    for which in 0..12u32 {
        let mut run = |l: &Lib| -> Report {
            read_session(l, stream.clone(), &mut |l, png, info| unsafe {
                let rb = 18usize;
                let mut buf = vec![0u8; rb + 16];
                match which {
                    0 => {
                        (l.api.png_read_row)(png, buf.as_mut_ptr(), ptr::null_mut());
                    }
                    1 => {
                        (l.api.png_read_end)(png, info);
                    }
                    2 => {
                        (l.api.png_read_info)(png, info);
                        (l.api.png_read_info)(png, info);
                    }
                    3 => {
                        (l.api.png_read_info)(png, info);
                        (l.api.png_read_update_info)(png, info);
                        (l.api.png_read_update_info)(png, info);
                    }
                    4 => {
                        (l.api.png_read_info)(png, info);
                        for _ in 0..10 {
                            (l.api.png_read_row)(png, buf.as_mut_ptr(), ptr::null_mut());
                        }
                    }
                    5 => {
                        (l.api.png_read_info)(png, info);
                        (l.api.png_read_row)(png, ptr::null_mut(), ptr::null_mut());
                    }
                    6 => {
                        // NOTE: png_read_image(png, NULL) is NOT tested: the C
                        // dereferences the row-pointer array with no NULL check
                        // (verified: the reference C .so segfaults).  Instead
                        // call png_read_image BEFORE png_read_info.
                        let mut rows: Vec<Vec<u8>> =
                            (0..3).map(|_| vec![0u8; rb + 16]).collect();
                        let mut ptrs: Vec<*mut u8> =
                            rows.iter_mut().map(|v| v.as_mut_ptr()).collect();
                        (l.api.png_read_image)(png, ptrs.as_mut_ptr());
                    }
                    7 => {
                        (l.api.png_read_info)(png, info);
                        (l.api.png_start_read_image)(png);
                        (l.api.png_start_read_image)(png);
                    }
                    8 => {
                        (l.api.png_read_info)(png, info);
                        (l.api.png_set_expand)(png);
                    }
                    9 => {
                        (l.api.png_read_info)(png, info);
                        (l.api.png_read_update_info)(png, info);
                        (l.api.png_set_gray_to_rgb)(png);
                    }
                    10 => {
                        (l.api.png_set_sig_bytes)(png, 9);
                        (l.api.png_read_info)(png, info);
                    }
                    _ => {
                        log(format!("reset_zstream={}", (l.api.png_reset_zstream)(png)));
                    }
                }
                log("sequence completed".to_string());
            })
        };
        diff(&format!("C8 read sequence #{which}"), &c, &r, &mut run);
    }
    // png_set_sig_bytes out of range
    for n in [-1i32, 0, 8, 9, 100] {
        let mut run = |l: &Lib| -> Report {
            read_session(l, stream.clone(), &mut |l, png, info| unsafe {
                (l.api.png_set_sig_bytes)(png, n);
                (l.api.png_read_info)(png, info);
                log("ok".to_string());
            })
        };
        diff(&format!("C8 set_sig_bytes({n})"), &c, &r, &mut run);
    }
}

// ===========================================================================
// C-9  Out-of-range enum / parameter values across the FFI boundary
// ===========================================================================
#[test]
fn c9_out_of_range_enums() {
    let (c, r) = libs();
    #[allow(unused)]
    fn dbg(label: &str, c: &Lib, r: &Lib, run: &mut dyn FnMut(&Lib) -> Report) {
        diff(label, c, r, run);
    }
    // png_set_IHDR: bit depth × colour type, the whole 2-D grid including
    // illegal combinations
    for bd in [-1i32, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 15, 16, 17, 32, 255] {
        for ct in [-1i32, 0, 1, 2, 3, 4, 5, 6, 7, 8, 255] {
            let mut run = |l: &Lib| -> Report {
                write_session(l, &mut |l, png, info| unsafe {
                    (l.api.png_set_IHDR)(
                        png, info, 4, 4, bd, ct, PNG_INTERLACE_NONE, PNG_COMPRESSION_TYPE_BASE,
                        PNG_FILTER_TYPE_BASE,
                    );
                    log("set_IHDR ok".to_string());
                })
            };
            dbg(&format!("C9 set_IHDR bd={bd} ct={ct}"), &c, &r, &mut run);
        }
    }
    // interlace / compression / filter methods
    for il in [-1i32, 0, 1, 2, 3, 255] {
        for cm in [-1i32, 0, 1, 255] {
            for fm in [-1i32, 0, 1, 64, 65, 255] {
                let mut run = |l: &Lib| -> Report {
                    write_session(l, &mut |l, png, info| unsafe {
                        (l.api.png_set_IHDR)(
                            png, info, 4, 4, 8, PNG_COLOR_TYPE_RGB, il, cm, fm,
                        );
                        log("set_IHDR ok".to_string());
                    })
                };
                dbg(&format!("C9 set_IHDR il={il} cm={cm} fm={fm}"), &c, &r, &mut run);
            }
        }
    }
    // png_set_filter with an invalid method / mask
    for method in [-1i32, 0, 1, 64, 255] {
        for filters in [
            -1i32, 0, 1, 2, 4, 7, 0x08, 0x10, 0x20, 0x40, 0x80, 0xf8, 0xff, 0x100, 0x1000,
        ] {
            let mut run = |l: &Lib| -> Report {
                write_session(l, &mut |l, png, info| unsafe {
                    (l.api.png_set_IHDR)(
                        png, info, 4, 4, 8, PNG_COLOR_TYPE_RGB, PNG_INTERLACE_NONE,
                        PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
                    );
                    (l.api.png_set_filter)(png, method, filters);
                    log("set_filter ok".to_string());
                })
            };
            dbg(
                &format!("C9 set_filter method={method} filters={filters:#x}"),
                &c,
                &r,
                &mut run,
            );
        }
    }
    // compression parameters out of range
    for v in [-2i32, -1, 0, 1, 9, 10, 100] {
        let mut run = |l: &Lib| -> Report {
            write_session(l, &mut |l, png, _info| unsafe {
                (l.api.png_set_compression_level)(png, v);
                (l.api.png_set_compression_strategy)(png, v);
                (l.api.png_set_compression_mem_level)(png, v);
                (l.api.png_set_compression_window_bits)(png, v);
                (l.api.png_set_compression_method)(png, v);
                (l.api.png_set_text_compression_level)(png, v);
                (l.api.png_set_text_compression_strategy)(png, v);
                (l.api.png_set_text_compression_mem_level)(png, v);
                (l.api.png_set_text_compression_window_bits)(png, v);
                (l.api.png_set_text_compression_method)(png, v);
                log("compression params ok".to_string());
            })
        };
        dbg(&format!("C9 compression params={v}"), &c, &r, &mut run);
    }
    // png_set_compression_buffer_size boundaries
    for v in [0usize, 1, 2, 3, usize::MAX / 2, usize::MAX] {
        let mut run = |l: &Lib| -> Report {
            write_session(l, &mut |l, png, _info| unsafe {
                (l.api.png_set_compression_buffer_size)(png, v);
                log(format!(
                    "buffer_size={}",
                    (l.api.png_get_compression_buffer_size)(png)
                ));
            })
        };
        dbg(&format!("C9 compression_buffer_size={v}"), &c, &r, &mut run);
    }
    // png_set_crc_action out-of-range
    for crit in [-1i32, 0, 5, 6, 100] {
        for anc in [-1i32, 0, 5, 6, 100] {
            let stream = gen(&c, 6, 3, PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE);
            let mut run = |l: &Lib| -> Report {
                read_session(l, stream.clone(), &mut |l, png, info| unsafe {
                    (l.api.png_set_crc_action)(png, crit, anc);
                    (l.api.png_read_info)(png, info);
                    log("ok".to_string());
                })
            };
            dbg(&format!("C9 crc_action {crit}/{anc}"), &c, &r, &mut run);
        }
    }
    // png_set_keep_unknown_chunks out-of-range keep
    for keep in [-2i32, -1, 0, 4, 5, 100] {
        let mut run = |l: &Lib| -> Report {
            read_session(l, vec![], &mut |l, png, _info| unsafe {
                (l.api.png_set_keep_unknown_chunks)(png, keep, ptr::null(), 0);
                log("ok".to_string());
            })
        };
        dbg(&format!("C9 keep_unknown keep={keep}"), &c, &r, &mut run);
    }
    // png_set_unknown_chunk_location out-of-range chunk index / location
    for idx in [-1i32, 0, 1, 100] {
        for loc in [-1i32, 0, 1, 2, 8, 0xff] {
            let mut run = |l: &Lib| -> Report {
                write_session(l, &mut |l, png, info| unsafe {
                    (l.api.png_set_unknown_chunk_location)(png, info, idx, loc);
                    log("ok".to_string());
                })
            };
            dbg(
                &format!("C9 unknown_chunk_location idx={idx} loc={loc}"),
                &c,
                &r,
                &mut run,
            );
        }
    }
    // png_set_PLTE with out-of-range num_palette
    for n in [-1i32, 0, 1, 256, 257, 1000] {
        let pal = make_palette(256, 3);
        let mut run = |l: &Lib| -> Report {
            write_session(l, &mut |l, png, info| unsafe {
                (l.api.png_set_IHDR)(
                    png, info, 4, 4, 8, PNG_COLOR_TYPE_PALETTE, PNG_INTERLACE_NONE,
                    PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
                );
                (l.api.png_set_PLTE)(png, info, pal.as_ptr(), n);
                log("set_PLTE ok".to_string());
            })
        };
        dbg(&format!("C9 set_PLTE n={n}"), &c, &r, &mut run);
    }
    // png_set_tRNS with out-of-range num_trans
    for n in [-1i32, 0, 1, 256, 257, 1000] {
        let alpha = vec![0x80u8; 256];
        let pal = make_palette(256, 3);
        let mut run = |l: &Lib| -> Report {
            write_session(l, &mut |l, png, info| unsafe {
                (l.api.png_set_IHDR)(
                    png, info, 4, 4, 8, PNG_COLOR_TYPE_PALETTE, PNG_INTERLACE_NONE,
                    PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
                );
                (l.api.png_set_PLTE)(png, info, pal.as_ptr(), 256);
                (l.api.png_set_tRNS)(png, info, alpha.as_ptr(), n, ptr::null());
                log("set_tRNS ok".to_string());
            })
        };
        dbg(&format!("C9 set_tRNS n={n}"), &c, &r, &mut run);
    }
    // png_set_text with out-of-range num_text / compression
    for comp in [-3i32, -2, -1, 0, 1, 2, 3, 100] {
        for n in [-1i32, 0, 1, 2] {
            let key = CString::new("K").unwrap();
            let txt = CString::new("V").unwrap();
            let mut run = |l: &Lib| -> Report {
                write_session(l, &mut |l, png, info| unsafe {
                    (l.api.png_set_IHDR)(
                        png, info, 4, 4, 8, PNG_COLOR_TYPE_RGB, PNG_INTERLACE_NONE,
                        PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
                    );
                    let t = PngText {
                        compression: comp,
                        key: key.as_ptr() as *mut i8,
                        text: txt.as_ptr() as *mut i8,
                        text_length: 1,
                        ..Default::default()
                    };
                    log(format!(
                        "set_text_2={}",
                        (l.pv.png_set_text_2)(png, info, &t, n)
                    ));
                    (l.api.png_set_text)(png, info, &t, n);
                    log("set_text ok".to_string());
                })
            };
            dbg(&format!("C9 set_text comp={comp} n={n}"), &c, &r, &mut run);
        }
    }
    // png_set_sPLT with out-of-range nentries / depth
    for depth in [0u8, 1, 7, 8, 9, 16, 17, 255] {
        for nent in [-1i32, 0, 1, 2] {
            let name = CString::new("s").unwrap();
            let entries = vec![
                PngSpltEntry { red: 1, green: 2, blue: 3, alpha: 4, frequency: 5 };
                4
            ];
            let mut run = |l: &Lib| -> Report {
                write_session(l, &mut |l, png, info| unsafe {
                    let s = PngSpltT {
                        name: name.as_ptr() as *mut i8,
                        depth,
                        entries: entries.as_ptr() as *mut PngSpltEntry,
                        nentries: nent,
                    };
                    (l.api.png_set_sPLT)(png, info, &s as *const PngSpltT as *const c_void, 1);
                    log("set_sPLT ok".to_string());
                })
            };
            dbg(&format!("C9 set_sPLT depth={depth} nent={nent}"), &c, &r, &mut run);
        }
    }
    // png_set_iCCP with an invalid compression type or profile
    for comp in [-1i32, 0, 1, 2, 100] {
        for len in [0u32, 1, 131, 132, 133] {
            let name = CString::new("p").unwrap();
            let prof = vec![0u8; len.max(1) as usize];
            let mut run = |l: &Lib| -> Report {
                write_session(l, &mut |l, png, info| unsafe {
                    (l.api.png_set_IHDR)(
                        png, info, 4, 4, 8, PNG_COLOR_TYPE_RGB, PNG_INTERLACE_NONE,
                        PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
                    );
                    (l.api.png_set_iCCP)(png, info, name.as_ptr(), comp, prof.as_ptr(), len);
                    log("set_iCCP ok".to_string());
                })
            };
            dbg(&format!("C9 set_iCCP comp={comp} len={len}"), &c, &r, &mut run);
        }
    }
    // png_set_filler / png_set_add_alpha with an invalid flags value
    for flags in [-1i32, 0, 1, 2, 100] {
        for &(ct, bd) in &[(0i32, 8i32), (2, 8), (4, 8), (6, 8), (3, 8)] {
            let mut run = |l: &Lib| -> Report {
                write_session(l, &mut |l, png, info| unsafe {
                    (l.api.png_set_IHDR)(
                        png, info, 4, 4, bd, ct, PNG_INTERLACE_NONE, PNG_COMPRESSION_TYPE_BASE,
                        PNG_FILTER_TYPE_BASE,
                    );
                    (l.api.png_set_filler)(png, 0xff, flags);
                    (l.api.png_set_add_alpha)(png, 0xff, flags);
                    log("filler ok".to_string());
                })
            };
            dbg(&format!("C9 set_filler flags={flags} ct={ct}"), &c, &r, &mut run);
        }
    }
    // png_set_rgb_to_gray with an invalid error action
    for action in [-2i32, -1, 0, 1, 2, 3, 4, 100] {
        let stream = gen(&c, 6, 3, PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE);
        let mut run = |l: &Lib| -> Report {
            read_session(l, stream.clone(), &mut |l, png, info| unsafe {
                (l.api.png_set_rgb_to_gray_fixed)(png, action, -1, -1);
                (l.api.png_read_info)(png, info);
                log("ok".to_string());
            })
        };
        dbg(&format!("C9 rgb_to_gray action={action}"), &c, &r, &mut run);
    }
    // png_set_rgb_to_gray coefficients out of range
    for &(red, green) in &[
        (-2i32, -2i32),
        (0, 0),
        (100000, 0),
        (0, 100000),
        (60000, 60000),
        (100001, 0),
        (-1, 50000),
        (i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN),
    ] {
        let stream = gen(&c, 6, 3, PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE);
        let mut run = |l: &Lib| -> Report {
            read_session(l, stream.clone(), &mut |l, png, info| unsafe {
                (l.api.png_set_rgb_to_gray_fixed)(png, PNG_ERROR_ACTION_WARN, red, green);
                (l.api.png_read_info)(png, info);
                log("ok".to_string());
            })
        };
        dbg(&format!("C9 rgb_to_gray coeff {red}/{green}"), &c, &r, &mut run);
    }
    // png_set_background with an invalid gamma code
    for code in [-2i32, -1, 0, 1, 2, 3, 4, 100] {
        let stream = gen(&c, 6, 3, PNG_COLOR_TYPE_RGB_ALPHA, 8, PNG_INTERLACE_NONE);
        let bg = PngColor16 { index: 0, red: 1, green: 2, blue: 3, gray: 4 };
        let mut run = |l: &Lib| -> Report {
            read_session(l, stream.clone(), &mut |l, png, info| unsafe {
                (l.api.png_set_background_fixed)(png, &bg, code, 0, 100000);
                (l.api.png_read_info)(png, info);
                log("ok".to_string());
            })
        };
        dbg(&format!("C9 background code={code}"), &c, &r, &mut run);
    }
    // png_set_alpha_mode with an invalid mode
    for mode in [-2i32, -1, 0, 1, 2, 3, 4, 100] {
        let stream = gen(&c, 6, 3, PNG_COLOR_TYPE_RGB_ALPHA, 8, PNG_INTERLACE_NONE);
        let mut run = |l: &Lib| -> Report {
            read_session(l, stream.clone(), &mut |l, png, info| unsafe {
                (l.api.png_set_alpha_mode_fixed)(png, mode, 100000);
                (l.api.png_read_info)(png, info);
                log("ok".to_string());
            })
        };
        dbg(&format!("C9 alpha_mode mode={mode}"), &c, &r, &mut run);
    }
    // png_set_gamma with out-of-range values
    for &g in &[i32::MIN, -3, -2, -1, 0, 1, i32::MAX] {
        let stream = gen(&c, 6, 3, PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE);
        let mut run = |l: &Lib| -> Report {
            read_session(l, stream.clone(), &mut |l, png, info| unsafe {
                (l.api.png_set_gamma_fixed)(png, g, g);
                (l.api.png_read_info)(png, info);
                log("ok".to_string());
            })
        };
        dbg(&format!("C9 gamma={g}"), &c, &r, &mut run);
    }
    // png_set_quantize with out-of-range parameters.
    //
    // NOTE: negative `num_palette`/`maximum_colors`, and `num_palette > 256`,
    // are NOT tested.  png_set_quantize ends with
    //   memcpy(png_ptr->palette, palette, (unsigned int)num_palette * sizeof(png_color))
    // into a fixed 256-entry buffer, so a negative count becomes ~4G and a count
    // above 256 overflows the destination (verified: the reference C .so
    // segfaults on num_palette = -1).  Those are undefined behaviour, not
    // rejections.  `maximum_colors < 1` with `num_palette >= 1` is excluded for
    // the same reason: the reduction loop
    //   while (num_new_palette > maximum_colors) { ...; max_d += 96; }
    // can never bring the count below 1, so `max_d` grows without bound and
    // `hash[max_d]` walks off the end of the 769-entry table.
    for npal in [0i32, 1, 2, 16, 256] {
        for maxcol in [1i32, 2, 16, 256, 257, 1000] {
            let stream = gen(&c, 8, 3, PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE);
            let pal0 = make_palette(256, 5);
            let mut run = |l: &Lib| -> Report {
                let mut pal = pal0.clone();
                read_session(l, stream.clone(), &mut |l, png, info| unsafe {
                    (l.api.png_set_quantize)(
                        png,
                        pal.as_mut_ptr(),
                        npal,
                        maxcol,
                        ptr::null(),
                        1,
                    );
                    (l.api.png_read_info)(png, info);
                    log("ok".to_string());
                })
            };
            dbg(&format!("C9 quantize npal={npal} max={maxcol}"), &c, &r, &mut run);
        }
    }
    // png_set_quantize with a NULL palette
    let mut run = |l: &Lib| -> Report {
        read_session(l, vec![], &mut |l, png, _info| unsafe {
            (l.api.png_set_quantize)(png, ptr::null_mut(), 4, 4, ptr::null(), 1);
            log("ok".to_string());
        })
    };
    dbg("C9 quantize NULL palette", &c, &r, &mut run);
    // png_permit_mng_features with unsupported bits
    for f in [0u32, 1, 2, 4, 5, 6, 7, 0xffff_ffff] {
        let mut run = |l: &Lib| -> Report {
            write_session(l, &mut |l, png, _info| unsafe {
                log(format!(
                    "mng={}",
                    (l.api.png_permit_mng_features)(png, f)
                ));
            })
        };
        dbg(&format!("C9 permit_mng_features={f:#x}"), &c, &r, &mut run);
    }
    // png_set_option with every option number and onoff value
    for opt in -4..20i32 {
        for onoff in [-1i32, 0, 1, 2, 3, 100] {
            let mut run = |l: &Lib| -> Report {
                read_session(l, vec![], &mut |l, png, _info| unsafe {
                    log(format!(
                        "set_option({opt},{onoff})={}",
                        (l.api.png_set_option)(png, opt, onoff)
                    ));
                })
            };
            dbg(&format!("C9 set_option {opt}/{onoff}"), &c, &r, &mut run);
        }
    }
    // png_set_check_for_invalid_index / png_set_benign_errors with odd values
    for v in [-2i32, -1, 0, 1, 2, 100] {
        let mut run = |l: &Lib| -> Report {
            read_session(l, vec![], &mut |l, png, _info| unsafe {
                (l.api.png_set_check_for_invalid_index)(png, v);
                (l.api.png_set_benign_errors)(png, v);
                log("ok".to_string());
            })
        };
        dbg(&format!("C9 invalid_index/benign={v}"), &c, &r, &mut run);
    }
    // png_set_longjmp_fn with a mismatched jmp_buf size
    for size in [0usize, 1, 8, 100, 199, 200, 201, 1000] {
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
                let jb = (l.api.png_set_longjmp_fn)(png, longjmp_addr(), size);
                log(format!("size={size} jb_null={}", jb.is_null()));
                let jb2 = (l.api.png_set_longjmp_fn)(png, longjmp_addr(), JMP_BUF_SIZE);
                log(format!("second jb_null={}", jb2.is_null()));
                let mut pp = png;
                (l.api.png_destroy_write_struct)(&mut pp, ptr::null_mut());
            }
            let rep = ctxb.digest();
            set_ctx(ptr::null_mut());
            rep
        };
        dbg(&format!("C9 set_longjmp_fn size={size}"), &c, &r, &mut run);
    }
    // png_set_user_limits at and beyond the image size
    for &(uw, uh) in &[(0u32, 0u32), (1, 1), (5, 3), (6, 3), (6, 2), (0x7fff_ffff, 0x7fff_ffff)] {
        let stream = gen(&c, 6, 3, PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE);
        let mut run = |l: &Lib| -> Report {
            read_session(l, stream.clone(), &mut |l, png, info| unsafe {
                (l.api.png_set_user_limits)(png, uw, uh);
                (l.api.png_read_info)(png, info);
                log("ok".to_string());
            })
        };
        dbg(&format!("C9 user_limits {uw}x{uh}"), &c, &r, &mut run);
    }
    // chunk cache / malloc limits forced low
    for cache in [0u32, 1, 2, 3] {
        for mal in [0usize, 1, 8, 100] {
            let stream = rich_stream(&c);
            let mut run = |l: &Lib| -> Report {
                read_session(l, stream.clone(), &mut |l, png, info| unsafe {
                    (l.api.png_set_chunk_cache_max)(png, cache);
                    (l.api.png_set_chunk_malloc_max)(png, mal);
                    (l.api.png_read_info)(png, info);
                    log("ok".to_string());
                    (l.api.png_read_end)(png, info);
                })
            };
            dbg(&format!("C9 chunk limits cache={cache} mal={mal}"), &c, &r, &mut run);
        }
    }
    // png_write_chunk with an illegal chunk name
    for name in [
        b"    ", b"0000", b"IHDR", b"IEND", b"\x00\x00\x00\x00", b"ab\xffd", b"AbCd",
    ] {
        let mut run = |l: &Lib| -> Report {
            write_session(l, &mut |l, png, _info| unsafe {
                (l.api.png_write_sig)(png);
                (l.api.png_write_chunk)(png, name.as_ptr(), ptr::null(), 0);
                log("write_chunk ok".to_string());
            })
        };
        diff(
            &format!("C9 write_chunk name={:?}", String::from_utf8_lossy(name)),
            &c,
            &r,
            &mut run,
        );
    }
    // png_write_chunk_start with a length beyond PNG_UINT_31_MAX
    for len in [0u32, 1, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff] {
        let mut run = |l: &Lib| -> Report {
            write_session(l, &mut |l, png, _info| unsafe {
                (l.api.png_write_sig)(png);
                (l.api.png_write_chunk_start)(png, b"prVt".as_ptr(), len);
                log("chunk_start ok".to_string());
            })
        };
        dbg(&format!("C9 write_chunk_start len={len:#x}"), &c, &r, &mut run);
    }
}

// ===========================================================================
// C-10  Allocation-limit driven rejections
// ===========================================================================
#[test]
fn c10_allocation_limits() {
    let (c, r) = libs();
    for size in [
        0usize,
        1,
        usize::MAX,
        usize::MAX / 2,
        usize::MAX - 1,
        0x7fff_ffff_ffff_ffff,
        1 << 40,
    ] {
        let mut run = |l: &Lib| -> Report {
            write_session(l, &mut |l, png, _info| unsafe {
                let p = (l.api.png_malloc_warn)(png, size);
                log(format!("malloc_warn({size}) null={}", p.is_null()));
                if !p.is_null() {
                    (l.api.png_free)(png, p);
                }
                let q = (l.pv.png_malloc_base)(png, size);
                log(format!("malloc_base({size}) null={}", q.is_null()));
                if !q.is_null() {
                    (l.api.png_free)(png, q);
                }
            })
        };
        diff(&format!("C10 malloc_warn size={size}"), &c, &r, &mut run);
    }
    // png_malloc (fatal on failure)
    for size in [usize::MAX, usize::MAX / 2, 1 << 40] {
        let mut run = |l: &Lib| -> Report {
            write_session(l, &mut |l, png, _info| unsafe {
                let p = (l.api.png_malloc)(png, size);
                log(format!("malloc({size}) null={}", p.is_null()));
                if !p.is_null() {
                    (l.api.png_free)(png, p);
                }
            })
        };
        diff(&format!("C10 png_malloc size={size}"), &c, &r, &mut run);
    }
    // png_malloc_array / png_realloc_array overflow boundaries
    for &(ne, es) in &[
        (-1i32, 1usize),
        (0, 1),
        (i32::MAX, 1),
        (i32::MAX, 16),
        (1 << 20, 1 << 20),
        (1000, usize::MAX),
    ] {
        let mut run = |l: &Lib| -> Report {
            write_session(l, &mut |l, png, _info| unsafe {
                let p = (l.pv.png_malloc_array)(png, ne, es);
                log(format!("malloc_array({ne},{es}) null={}", p.is_null()));
                if !p.is_null() {
                    (l.api.png_free)(png, p);
                }
                let q = (l.pv.png_realloc_array)(png, ptr::null(), 0, ne, es);
                log(format!("realloc_array({ne},{es}) null={}", q.is_null()));
                if !q.is_null() {
                    (l.api.png_free)(png, q);
                }
            })
        };
        diff(&format!("C10 array alloc ne={ne} es={es}"), &c, &r, &mut run);
    }
    // Huge declared image so png_read_start_row's rowbytes computation overflows
    let good = gen(&c, 8, 4, PNG_COLOR_TYPE_RGB_ALPHA, 16, PNG_INTERLACE_NONE);
    let cs = chunks(&good);
    let ihdr = cs.iter().find(|c| &c.2 == b"IHDR").unwrap();
    for w in [0x1000_0000u32, 0x2000_0000, 0x7fff_ffff] {
        let mut s = good.clone();
        s[ihdr.0 + 8..ihdr.0 + 12].copy_from_slice(&w.to_be_bytes());
        fix_crc(&mut s, ihdr.0, ihdr.1);
        let mut run = |l: &Lib| -> Report {
            read_session(l, s.clone(), &mut |l, png, info| unsafe {
                (l.api.png_set_user_limits)(png, 0x7fff_ffff, 0x7fff_ffff);
                (l.api.png_read_info)(png, info);
                log(format!("rowbytes={}", (l.api.png_get_rowbytes)(png, info)));
                (l.api.png_start_read_image)(png);
                log("start_read_image ok".to_string());
            })
        };
        diff(&format!("C10 huge width={w:#x}"), &c, &r, &mut run);
    }
}

// ===========================================================================
// C-11  Simplified-API rejections
// ===========================================================================
#[test]
fn c11_simplified_api_errors() {
    let (c, r) = libs();
    let good = gen(&c, 8, 4, PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE);
    // wrong version, non-NULL opaque, zero size, truncated memory
    for which in 0..8u32 {
        let mut run = |l: &Lib| -> Report {
            let mut ctxb = Box::new(Ctx::default());
            set_ctx(&mut *ctxb as *mut Ctx);
            unsafe {
                let mut im = PngImage::default();
                // NOTE: a non-NULL but bogus `opaque` is NOT tested: the C
                // rejects it via png_image_error(), which itself calls
                // png_image_free() and dereferences the bogus pointer.  That is
                // undefined behaviour in the reference implementation, not a
                // comparable rejection (verified: the C .so segfaults).
                match which {
                    0 => im.version = 0,
                    1 => im.version = 2,
                    _ => {}
                }
                let (p, n) = match which {
                    3 => (ptr::null(), 0usize),
                    4 => (good.as_ptr(), 0usize),
                    5 => (good.as_ptr(), 1usize),
                    6 => (good.as_ptr(), 7usize),
                    7 => (good.as_ptr(), good.len() - 1),
                    _ => (good.as_ptr(), good.len()),
                };
                let ok = (l.api.png_image_begin_read_from_memory)(
                    &mut im,
                    p as *const c_void,
                    n,
                );
                log_img(&format!("begin({which})={ok}"), &im);
                (l.api.png_image_free)(&mut im);
            }
            let rep = ctxb.digest();
            set_ctx(ptr::null_mut());
            rep
        };
        diff(&format!("C11 begin_read_from_memory case {which}"), &c, &r, &mut run);
    }
    // finish_read without begin_read, and with a NULL buffer
    for which in 0..4u32 {
        let mut run = |l: &Lib| -> Report {
            let mut ctxb = Box::new(Ctx::default());
            set_ctx(&mut *ctxb as *mut Ctx);
            unsafe {
                let mut im = PngImage::default();
                if which >= 2 {
                    (l.api.png_image_begin_read_from_memory)(
                        &mut im,
                        good.as_ptr() as *const c_void,
                        good.len(),
                    );
                }
                let mut buf = vec![0u8; 4096];
                let b = if which % 2 == 0 {
                    ptr::null_mut()
                } else {
                    buf.as_mut_ptr() as *mut c_void
                };
                let ok = (l.api.png_image_finish_read)(&mut im, ptr::null(), b, 0, ptr::null_mut());
                log_img(&format!("finish({which})={ok}"), &im);
                (l.api.png_image_free)(&mut im);
            }
            let rep = ctxb.digest();
            set_ctx(ptr::null_mut());
            rep
        };
        diff(&format!("C11 finish_read case {which}"), &c, &r, &mut run);
    }
    // write with an invalid image description
    for which in 0..10u32 {
        let mut run = |l: &Lib| -> Report {
            let mut ctxb = Box::new(Ctx::default());
            set_ctx(&mut *ctxb as *mut Ctx);
            unsafe {
                let mut im = PngImage {
                    version: PNG_IMAGE_VERSION,
                    width: 4,
                    height: 4,
                    format: 0,
                    ..Default::default()
                };
                // NOTE: `width == 0` is NOT tested.  png_image_write_main
                // computes `row_stride = width * channels` and then evaluates
                // `image->height > PNG_UINT_31_MAX / row_stride`, which is an
                // integer division by zero (verified: the reference C .so dies
                // with SIGFPE).  That is undefined behaviour, not a rejection.
                match which {
                    0 => im.version = 0,
                    1 => im.version = 3,
                    2 => im.height = 0,
                    3 => im.format = 0xffff_ffff,
                    4 => im.format = 0x40,
                    5 => im.format = 0x80,
                    6 => {
                        im.format = PNG_FORMAT_FLAG_COLORMAP;
                        im.colormap_entries = 0;
                    }
                    7 => {
                        im.format = PNG_FORMAT_FLAG_COLORMAP;
                        im.colormap_entries = 257;
                    }
                    8 => im.width = 0x7fff_ffff,
                    _ => im.height = 0x7fff_ffff,
                }
                let src = vec![0x40u8; 4096];
                let mut out = vec![0u8; 1 << 16];
                let mut sz = out.len();
                let ok = (l.api.png_image_write_to_memory)(
                    &mut im,
                    out.as_mut_ptr() as *mut c_void,
                    &mut sz,
                    0,
                    src.as_ptr() as *mut c_void,
                    0,
                    src.as_ptr() as *mut c_void,
                );
                log_img(&format!("write({which})={ok} sz={sz}"), &im);
                (l.api.png_image_free)(&mut im);
            }
            let rep = ctxb.digest();
            set_ctx(ptr::null_mut());
            rep
        };
        diff(&format!("C11 write_to_memory case {which}"), &c, &r, &mut run);
    }
    // png_image_error directly
    for msg in ["", "boom", "a very long message that will be truncated by the 64 byte buffer in png_image"] {
        let mut run = |l: &Lib| -> Report {
            let mut ctxb = Box::new(Ctx::default());
            set_ctx(&mut *ctxb as *mut Ctx);
            unsafe {
                let mut im = PngImage::default();
                let cs = CString::new(msg).unwrap();
                let got = (l.pv.png_image_error)(&mut im, cs.as_ptr());
                log_img(&format!("png_image_error={got}"), &im);
            }
            let rep = ctxb.digest();
            set_ctx(ptr::null_mut());
            rep
        };
        diff(&format!("C11 png_image_error {:?}", &msg[..msg.len().min(12)]), &c, &r, &mut run);
    }
}

// ===========================================================================
// C-12  png_error / png_warning family called directly
// ===========================================================================
#[test]
fn c12_error_family_direct() {
    let (c, r) = libs();
    let msgs: &[&str] = &[
        "",
        "plain message",
        "#123 numbered message",
        "#000 zero",
        "#99999999999 overflow",
        "message with a very long tail that exceeds the internal buffer used by png_format_number and friends for sure",
    ];
    for m in msgs {
        for which in 0..7u32 {
            let mut run = |l: &Lib| -> Report {
                read_session(l, vec![], &mut |l, png, _info| unsafe {
                    let cs = CString::new(*m).unwrap();
                    match which {
                        0 => (l.api.png_warning)(png, cs.as_ptr()),
                        1 => (l.api.png_chunk_warning)(png, cs.as_ptr()),
                        2 => (l.pv.png_app_warning)(png, cs.as_ptr()),
                        3 => (l.api.png_benign_error)(png, cs.as_ptr()),
                        4 => (l.api.png_chunk_benign_error)(png, cs.as_ptr()),
                        5 => (l.pv.png_app_error)(png, cs.as_ptr()),
                        _ => (l.api.png_error)(png, cs.as_ptr()),
                    }
                    log("returned".to_string());
                })
            };
            diff(&format!("C12 error fn {which} msg={m:?}"), &c, &r, &mut run);
        }
        // and the chunk variants
        let mut run = |l: &Lib| -> Report {
            read_session(l, vec![], &mut |l, png, _info| unsafe {
                let cs = CString::new(*m).unwrap();
                (l.api.png_chunk_error)(png, cs.as_ptr());
                log("returned".to_string());
            })
        };
        diff(&format!("C12 png_chunk_error msg={m:?}"), &c, &r, &mut run);
    }
    // benign errors allowed vs not, for both read and write structs
    for allowed in [0i32, 1] {
        for is_read in [false, true] {
            let mut run = |l: &Lib| -> Report {
                let body = &mut |l: &Lib, png: *mut c_void, _i: *mut c_void| unsafe {
                    (l.api.png_set_benign_errors)(png, allowed);
                    let m = CString::new("benign test").unwrap();
                    (l.api.png_benign_error)(png, m.as_ptr());
                    log("after benign".to_string());
                    (l.pv.png_app_error)(png, m.as_ptr());
                    log("after app_error".to_string());
                };
                if is_read {
                    read_session(l, vec![], body)
                } else {
                    write_session(l, body)
                }
            };
            diff(
                &format!("C12 benign allowed={allowed} read={is_read}"),
                &c,
                &r,
                &mut run,
            );
        }
    }
    // png_longjmp without a jmp_buf installed is PNG_ABORT() in the C, which is
    // not a comparable result; instead check png_longjmp WITH a buffer.
    let mut run = |l: &Lib| -> Report {
        read_session(l, vec![], &mut |l, png, _info| unsafe {
            (l.api.png_longjmp)(png, 7);
        })
    };
    diff("C12 png_longjmp(7)", &c, &r, &mut run);
}

// ===========================================================================
// C-13  Interlaced / row-count mismatches and transform conflicts
// ===========================================================================
#[test]
fn c13_transform_conflicts() {
    let (c, r) = libs();
    for &(ct, bd) in LEGAL {
        let stream = gen(&c, 9, 5, ct, bd, PNG_INTERLACE_NONE);
        // Conflicting transform pairs the C explicitly rejects or warns about
        let combos: &[(&str, u32)] = &[
            ("background+alpha_mode", 0),
            ("alpha_mode twice", 1),
            ("background twice", 2),
            ("strip_alpha+background", 3),
            ("scale_16+strip_16", 4),
            ("expand_16+strip_16", 5),
            ("rgb_to_gray+gray_to_rgb", 6),
            ("quantize+expand", 7),
            ("packing+packswap+expand", 8),
            ("filler+add_alpha", 9),
        ];
        for &(name, k) in combos {
            let mut run = |l: &Lib| -> Report {
                read_session(l, stream.clone(), &mut |l, png, info| unsafe {
                    (l.api.png_read_info)(png, info);
                    let bg = PngColor16 { index: 0, red: 1, green: 2, blue: 3, gray: 4 };
                    match k {
                        0 => {
                            (l.api.png_set_background_fixed)(
                                png,
                                &bg,
                                PNG_BACKGROUND_GAMMA_SCREEN,
                                0,
                                100000,
                            );
                            (l.api.png_set_alpha_mode_fixed)(png, PNG_ALPHA_STANDARD, 100000);
                        }
                        1 => {
                            (l.api.png_set_alpha_mode_fixed)(png, PNG_ALPHA_STANDARD, 100000);
                            (l.api.png_set_alpha_mode_fixed)(png, PNG_ALPHA_OPTIMIZED, 220000);
                        }
                        2 => {
                            (l.api.png_set_background_fixed)(
                                png,
                                &bg,
                                PNG_BACKGROUND_GAMMA_SCREEN,
                                0,
                                100000,
                            );
                            (l.api.png_set_background_fixed)(
                                png,
                                &bg,
                                PNG_BACKGROUND_GAMMA_FILE,
                                1,
                                220000,
                            );
                        }
                        3 => {
                            (l.api.png_set_strip_alpha)(png);
                            (l.api.png_set_background_fixed)(
                                png,
                                &bg,
                                PNG_BACKGROUND_GAMMA_SCREEN,
                                0,
                                100000,
                            );
                        }
                        4 => {
                            (l.api.png_set_scale_16)(png);
                            (l.api.png_set_strip_16)(png);
                        }
                        5 => {
                            (l.api.png_set_expand_16)(png);
                            (l.api.png_set_strip_16)(png);
                        }
                        6 => {
                            (l.api.png_set_rgb_to_gray_fixed)(png, PNG_ERROR_ACTION_WARN, -1, -1);
                            (l.api.png_set_gray_to_rgb)(png);
                        }
                        7 => {
                            let mut pal = make_palette(256, 9);
                            (l.api.png_set_quantize)(
                                png,
                                pal.as_mut_ptr(),
                                256,
                                16,
                                ptr::null(),
                                1,
                            );
                            (l.api.png_set_expand)(png);
                            std::mem::forget(pal);
                        }
                        8 => {
                            (l.api.png_set_packing)(png);
                            (l.api.png_set_packswap)(png);
                            (l.api.png_set_expand)(png);
                        }
                        _ => {
                            (l.api.png_set_filler)(png, 0xff, PNG_FILLER_BEFORE);
                            (l.api.png_set_add_alpha)(png, 0xff, PNG_FILLER_AFTER);
                        }
                    }
                    (l.api.png_read_update_info)(png, info);
                    let h = (l.api.png_get_image_height)(png, info);
                    let rb = (l.api.png_get_rowbytes)(png, info);
                    log(format!("rb={rb}"));
                    let mut buf = vec![0u8; rb + 16];
                    for _ in 0..h {
                        (l.api.png_read_row)(png, buf.as_mut_ptr(), ptr::null_mut());
                    }
                    log(format!("last row={:02x?}", &buf[..rb]));
                    (l.api.png_read_end)(png, info);
                })
            };
            diff(&format!("C13 {name} ct={ct} bd={bd}"), &c, &r, &mut run);
        }
    }
}
