//! Tier 6: malformed and hostile input.  The two libraries must agree on which
//! inputs are rejected, on the exact error and warning strings, and on how much
//! of the image survives a recoverable problem.

mod common;
use common::*;
use std::ffi::{c_char, c_int, CString};

/* ---------------------------------------------------------------- helpers */

fn crc32(data: &[u8]) -> u32 {
    static mut TABLE: [u32; 256] = [0; 256];
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| unsafe {
        for i in 0..256u32 {
            let mut c = i;
            for _ in 0..8 {
                c = if c & 1 != 0 { 0xedb88320 ^ (c >> 1) } else { c >> 1 };
            }
            TABLE[i as usize] = c;
        }
    });
    let mut c: u32 = 0xffff_ffff;
    for &b in data {
        c = unsafe { TABLE[((c ^ b as u32) & 0xff) as usize] } ^ (c >> 8);
    }
    c ^ 0xffff_ffff
}

#[derive(Clone, Debug)]
struct Chunk {
    name: [u8; 4],
    data: Vec<u8>,
    /// when false the stored CRC is deliberately wrong
    good_crc: bool,
}

impl Chunk {
    fn new(name: &[u8; 4], data: Vec<u8>) -> Chunk {
        Chunk { name: *name, data, good_crc: true }
    }
    fn serialize(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&(self.data.len() as u32).to_be_bytes());
        out.extend_from_slice(&self.name);
        out.extend_from_slice(&self.data);
        let mut crcbuf = self.name.to_vec();
        crcbuf.extend_from_slice(&self.data);
        let mut c = crc32(&crcbuf);
        if !self.good_crc {
            c ^= 0xffff_ffff;
        }
        out.extend_from_slice(&c.to_be_bytes());
    }
}

const SIG: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

fn split_chunks(png: &[u8]) -> Vec<Chunk> {
    let mut out = Vec::new();
    let mut i = 8usize;
    while i + 8 <= png.len() {
        let len = u32::from_be_bytes([png[i], png[i + 1], png[i + 2], png[i + 3]]) as usize;
        let mut name = [0u8; 4];
        name.copy_from_slice(&png[i + 4..i + 8]);
        let start = i + 8;
        let end = (start + len).min(png.len());
        out.push(Chunk::new(&name, png[start..end].to_vec()));
        i = end + 4;
    }
    out
}

fn assemble(chunks: &[Chunk]) -> Vec<u8> {
    let mut out = SIG.to_vec();
    for c in chunks {
        c.serialize(&mut out);
    }
    out
}

fn rowbytes(pixel_depth: u32, width: u32) -> usize {
    if pixel_depth >= 8 {
        (width as usize) * ((pixel_depth as usize) >> 3)
    } else {
        ((width as usize) * (pixel_depth as usize) + 7) >> 3
    }
}

fn channels(ct: u8) -> u32 {
    match ct {
        PNG_COLOR_TYPE_GRAY | PNG_COLOR_TYPE_PALETTE => 1,
        PNG_COLOR_TYPE_GRAY_ALPHA => 2,
        PNG_COLOR_TYPE_RGB => 3,
        PNG_COLOR_TYPE_RGB_ALPHA => 4,
        _ => 4,
    }
}

/// A small, valid, chunk-rich reference PNG produced by the C library.
fn base_png(ct: u8, bd: u8, interlace: c_int) -> Vec<u8> {
    let pd = channels(ct) * bd as u32;
    let (w, h) = (8u32, 4u32);
    let rb = rowbytes(pd, w);
    let mut s: u32 = 12345;
    let rows: Vec<Vec<u8>> = (0..h)
        .map(|_| {
            (0..rb)
                .map(|_| {
                    s = s.wrapping_mul(1103515245).wrapping_add(12345);
                    (s >> 16) as u8
                })
                .collect()
        })
        .collect();
    let out = write_with(&libs().c, |c, _| {
        let png = c.png;
        let info = c.info;
        let mut keep: Vec<CString> = Vec::new();
        type Fihdr = unsafe extern "C-unwind" fn(
            png_structp, png_infop, u32, u32, c_int, c_int, c_int, c_int, c_int,
        );
        let f: libloading::Symbol<Fihdr> = c.sym("png_set_IHDR");
        unsafe {
            f(
                png, info, w, h, bd as c_int, ct as c_int, interlace,
                PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
            )
        };
        if ct == PNG_COLOR_TYPE_PALETTE {
            let npal = 1usize << bd;
            let pal: Vec<png_color> = (0..npal)
                .map(|i| png_color { red: i as u8, green: (i * 3) as u8, blue: (i * 5) as u8 })
                .collect();
            let g: libloading::Symbol<
                unsafe extern "C-unwind" fn(png_structp, png_infop, *const png_color, c_int),
            > = c.sym("png_set_PLTE");
            unsafe { g(png, info, pal.as_ptr(), npal as c_int) };
        }
        let g: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop, i32)> =
            c.sym("png_set_gAMA_fixed");
        unsafe { g(png, info, 45455) };
        let key = CString::new("Title").unwrap();
        let txt = CString::new("t").unwrap();
        let t = png_text {
            compression: PNG_TEXT_COMPRESSION_NONE,
            key: key.as_ptr() as *mut c_char,
            text: txt.as_ptr() as *mut c_char,
            text_length: 0,
            itxt_length: 0,
            lang: std::ptr::null_mut(),
            lang_key: std::ptr::null_mut(),
        };
        let g: libloading::Symbol<
            unsafe extern "C-unwind" fn(png_structp, png_infop, *const png_text, c_int),
        > = c.sym("png_set_text");
        unsafe { g(png, info, &t, 1) };
        keep.push(key);
        keep.push(txt);
        c.call2("png_write_info");
        let mut ptrs: Vec<*mut u8> = rows.iter().map(|r| r.as_ptr() as *mut u8).collect();
        let g: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, *mut *mut u8, u32)> =
            c.sym("png_write_image");
        unsafe { g(png, ptrs.as_mut_ptr(), h) };
        c.call2("png_write_end");
        drop(keep);
    });
    assert!(!out.errored, "base_png failed: {:?}", out.diag);
    out.bytes
}

/* ------------------------------------------------------------- comparison */

#[derive(Clone, Copy, Debug, Default)]
struct Cfg {
    crc_action: Option<(c_int, c_int)>,
    benign: bool,
    keep_unknown: Option<c_int>,
    user_limits: Option<(u32, u32)>,
    chunk_cache_max: Option<u32>,
    chunk_malloc_max: Option<usize>,
    /// use the progressive reader instead of the sequential one
    progressive: bool,
}

fn read_attempt(lib: &Lib, data: &[u8], cfg: &Cfg) -> ReadOutcome {
    read_with(lib, data, |c, out| {
        let png = c.png;
        if let Some((a, b)) = cfg.crc_action {
            let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, c_int, c_int)> =
                c.sym("png_set_crc_action");
            unsafe { f(png, a, b) };
        }
        if cfg.benign {
            let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, c_int)> =
                c.sym("png_set_benign_errors");
            unsafe { f(png, 1) };
        }
        if let Some(k) = cfg.keep_unknown {
            let f: libloading::Symbol<
                unsafe extern "C-unwind" fn(png_structp, c_int, *const u8, c_int),
            > = c.sym("png_set_keep_unknown_chunks");
            unsafe { f(png, k, std::ptr::null(), 0) };
        }
        if let Some((w, h)) = cfg.user_limits {
            let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, u32, u32)> =
                c.sym("png_set_user_limits");
            unsafe { f(png, w, h) };
        }
        if let Some(m) = cfg.chunk_cache_max {
            let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, u32)> =
                c.sym("png_set_chunk_cache_max");
            unsafe { f(png, m) };
        }
        if let Some(m) = cfg.chunk_malloc_max {
            let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, usize)> =
                c.sym("png_set_chunk_malloc_max");
            unsafe { f(png, m) };
        }

        c.call2("png_read_info");
        out.notes.extend(snapshot_info(c));
        c.call2("png_read_update_info");
        out.notes.extend(snapshot_info(c));

        let rb: usize = {
            let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop) -> usize> =
                c.sym("png_get_rowbytes");
            unsafe { f(png, c.info) }
        };
        let height: u32 = {
            let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop) -> u32> =
                c.sym("png_get_image_height");
            unsafe { f(png, c.info) }
        };
        out.notes.push(format!("rb={rb} h={height}"));
        let mut bufs: Vec<Vec<u8>> = (0..height.min(4096)).map(|_| vec![0x5au8; rb + 64]).collect();
        let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, *mut u8, *mut u8)> =
            c.sym("png_read_row");
        for b in bufs.iter_mut() {
            unsafe { f(png, b.as_mut_ptr(), std::ptr::null_mut()) };
        }
        out.rows = bufs;
        c.call2("png_read_end");
        out.notes.extend(snapshot_info(c));
    })
}

fn compare_bad(label: &str, data: &[u8], cfg: &Cfg) {
    let l = libs();
    let a = read_attempt(&l.c, data, cfg);
    let b = read_attempt(&l.r, data, cfg);
    assert_eq!(
        a.diag, b.diag,
        "{label} {cfg:?}: diagnostics differ\n C {:?}\n R {:?}",
        a.diag, b.diag
    );
    assert_eq!(a.errored, b.errored, "{label} {cfg:?}: error flag differs");
    // the notes are only meaningful up to the point where one side gave up
    let n = a.notes.len().min(b.notes.len());
    assert_snapshots_eq(&format!("{label} {cfg:?}"), &a.notes[..n], &b.notes[..n]);
    assert_eq!(a.notes.len(), b.notes.len(), "{label} {cfg:?}: note count differs");
    assert_eq!(a.rows, b.rows, "{label} {cfg:?}: rows differ");
}

fn cfgs() -> Vec<Cfg> {
    vec![
        Cfg::default(),
        Cfg { benign: true, ..Default::default() },
        Cfg { crc_action: Some((PNG_CRC_WARN_USE, PNG_CRC_WARN_DISCARD)), ..Default::default() },
        Cfg { crc_action: Some((PNG_CRC_QUIET_USE, PNG_CRC_QUIET_USE)), ..Default::default() },
        Cfg { crc_action: Some((PNG_CRC_ERROR_QUIT, PNG_CRC_ERROR_QUIT)), ..Default::default() },
        Cfg { keep_unknown: Some(PNG_HANDLE_CHUNK_ALWAYS), ..Default::default() },
        Cfg { keep_unknown: Some(PNG_HANDLE_CHUNK_NEVER), ..Default::default() },
    ]
}

/* ------------------------------------------------------------------ tests */

#[test]
fn bad_signatures_and_truncation() {
    let good = base_png(PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE);
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    cases.push(("empty".into(), Vec::new()));
    for n in [1usize, 4, 7, 8, 9, 16, 24, 32, 40, 50, 60] {
        if n <= good.len() {
            cases.push((format!("truncated to {n}"), good[..n].to_vec()));
        }
    }
    cases.push(("no signature".into(), good[8..].to_vec()));
    for i in 0..8 {
        let mut d = good.clone();
        d[i] ^= 0xff;
        cases.push((format!("signature byte {i} flipped"), d));
    }
    cases.push(("garbage".into(), vec![0u8; 64]));
    for (name, data) in cases {
        compare_bad(&name, &data, &Cfg::default());
    }
}

#[test]
fn bad_ihdr() {
    let good = base_png(PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE);
    let chunks = split_chunks(&good);
    let mutate = |f: &dyn Fn(&mut Vec<u8>)| -> Vec<u8> {
        let mut cs = chunks.clone();
        f(&mut cs[0].data);
        assemble(&cs)
    };
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    cases.push(("zero width".into(), mutate(&|d| d[0..4].copy_from_slice(&0u32.to_be_bytes()))));
    cases.push(("zero height".into(), mutate(&|d| d[4..8].copy_from_slice(&0u32.to_be_bytes()))));
    cases.push((
        "huge width".into(),
        mutate(&|d| d[0..4].copy_from_slice(&0x7fff_ffffu32.to_be_bytes())),
    ));
    cases.push((
        "negative width".into(),
        mutate(&|d| d[0..4].copy_from_slice(&0x8000_0000u32.to_be_bytes())),
    ));
    for bd in [0u8, 3, 5, 6, 7, 9, 15, 17, 32, 255] {
        cases.push((format!("bit depth {bd}"), mutate(&|d| d[8] = bd)));
    }
    for ct in [1u8, 5, 7, 8, 9, 255] {
        cases.push((format!("colour type {ct}"), mutate(&|d| d[9] = ct)));
    }
    for cm in [1u8, 2, 255] {
        cases.push((format!("compression {cm}"), mutate(&|d| d[10] = cm)));
    }
    for fm in [1u8, 2, 255] {
        cases.push((format!("filter {fm}"), mutate(&|d| d[11] = fm)));
    }
    for il in [2u8, 3, 255] {
        cases.push((format!("interlace {il}"), mutate(&|d| d[12] = il)));
    }
    // wrong IHDR length
    {
        let mut cs = chunks.clone();
        cs[0].data.truncate(12);
        cases.push(("short IHDR".into(), assemble(&cs)));
        let mut cs = chunks.clone();
        cs[0].data.push(0);
        cases.push(("long IHDR".into(), assemble(&cs)));
    }
    // bad CRC on IHDR
    {
        let mut cs = chunks.clone();
        cs[0].good_crc = false;
        cases.push(("IHDR bad CRC".into(), assemble(&cs)));
    }
    // missing IHDR
    {
        let cs = chunks[1..].to_vec();
        cases.push(("no IHDR".into(), assemble(&cs)));
    }
    for (name, data) in cases {
        for cfg in cfgs() {
            compare_bad(&name, &data, &cfg);
        }
    }
}

#[test]
fn bad_crcs_and_chunk_order() {
    for (ct, bd) in [
        (PNG_COLOR_TYPE_RGB, 8u8),
        (PNG_COLOR_TYPE_PALETTE, 4),
        (PNG_COLOR_TYPE_GRAY_ALPHA, 16),
    ] {
        let good = base_png(ct, bd, PNG_INTERLACE_NONE);
        let chunks = split_chunks(&good);
        let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
        // corrupt each chunk's CRC in turn
        for i in 0..chunks.len() {
            let mut cs = chunks.clone();
            cs[i].good_crc = false;
            cases.push((
                format!("bad CRC on {}", String::from_utf8_lossy(&chunks[i].name)),
                assemble(&cs),
            ));
        }
        // duplicate each chunk
        for i in 0..chunks.len() {
            let mut cs = chunks.clone();
            let dup = cs[i].clone();
            cs.insert(i + 1, dup);
            cases.push((
                format!("duplicate {}", String::from_utf8_lossy(&chunks[i].name)),
                assemble(&cs),
            ));
        }
        // drop each non-IHDR chunk
        for i in 1..chunks.len() {
            let mut cs = chunks.clone();
            let name = String::from_utf8_lossy(&cs[i].name).into_owned();
            cs.remove(i);
            cases.push((format!("missing {name}"), assemble(&cs)));
        }
        // reorder: move the last ancillary chunk before IHDR
        if chunks.len() > 2 {
            let mut cs = chunks.clone();
            let last = cs.pop().unwrap();
            cs.insert(0, last);
            cases.push(("chunk before IHDR".into(), assemble(&cs)));
        }
        // unknown critical and ancillary chunks
        {
            let mut cs = chunks.clone();
            cs.insert(1, Chunk::new(b"crIT", vec![1, 2, 3]));
            cases.push(("unknown critical chunk".into(), assemble(&cs)));
            let mut cs = chunks.clone();
            cs.insert(1, Chunk::new(b"anCl", vec![1, 2, 3]));
            cases.push(("unknown ancillary chunk".into(), assemble(&cs)));
            let mut cs = chunks.clone();
            cs.insert(1, Chunk::new(b"IHDR", vec![0; 13]));
            cases.push(("second IHDR".into(), assemble(&cs)));
        }
        // IDAT damage
        for i in 0..chunks.len() {
            if &chunks[i].name == b"IDAT" {
                let mut cs = chunks.clone();
                if !cs[i].data.is_empty() {
                    cs[i].data[0] ^= 0xff;
                }
                cases.push(("corrupt zlib header".into(), assemble(&cs)));
                let mut cs = chunks.clone();
                let n = cs[i].data.len();
                if n > 2 {
                    cs[i].data[n - 2] ^= 0xff;
                }
                cases.push(("corrupt adler".into(), assemble(&cs)));
                let mut cs = chunks.clone();
                cs[i].data.clear();
                cases.push(("empty IDAT".into(), assemble(&cs)));
                let mut cs = chunks.clone();
                cs[i].data.truncate(1);
                cases.push(("short IDAT".into(), assemble(&cs)));
                break;
            }
        }
        for (name, data) in cases {
            for cfg in cfgs() {
                compare_bad(&format!("{name} ct={ct} bd={bd}"), &data, &cfg);
            }
        }
    }
}

#[test]
fn bad_ancillary_chunks() {
    let good = base_png(PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE);
    let chunks = split_chunks(&good);
    let insert = |c: Chunk| -> Vec<u8> {
        let mut cs = chunks.clone();
        // after IHDR
        cs.insert(1, c);
        assemble(&cs)
    };
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();

    // malformed fixed-size chunks
    for (name, tag) in [
        ("gAMA", &b"gAMA"[..]),
        ("cHRM", b"cHRM"),
        ("sRGB", b"sRGB"),
        ("pHYs", b"pHYs"),
        ("oFFs", b"oFFs"),
        ("tIME", b"tIME"),
        ("bKGD", b"bKGD"),
        ("tRNS", b"tRNS"),
        ("sBIT", b"sBIT"),
        ("cICP", b"cICP"),
        ("cLLI", b"cLLI"),
        ("mDCV", b"mDCV"),
        ("PLTE", b"PLTE"),
        ("hIST", b"hIST"),
        ("sCAL", b"sCAL"),
        ("pCAL", b"pCAL"),
        ("iCCP", b"iCCP"),
        ("sPLT", b"sPLT"),
        ("eXIf", b"eXIf"),
        ("tEXt", b"tEXt"),
        ("zTXt", b"zTXt"),
        ("iTXt", b"iTXt"),
    ] {
        let mut t = [0u8; 4];
        t.copy_from_slice(tag);
        for len in [0usize, 1, 2, 3, 4, 5, 7, 9, 13, 33] {
            cases.push((
                format!("{name} length {len}"),
                insert(Chunk::new(&t, vec![0x41; len])),
            ));
        }
        cases.push((
            format!("{name} 0xff filled"),
            insert(Chunk::new(&t, vec![0xff; 32])),
        ));
    }
    // text chunks with dodgy keywords and payloads
    for (name, payload) in [
        ("empty key", vec![0u8]),
        ("no separator", b"Titlexxxx".to_vec()),
        ("leading space key", b" Title\0text".to_vec()),
        ("trailing space key", b"Title \0text".to_vec()),
        ("control char key", b"Ti\x01tle\0text".to_vec()),
        ("long key", {
            let mut v = vec![b'k'; 100];
            v.push(0);
            v.extend_from_slice(b"text");
            v
        }),
        ("consecutive spaces", b"Ti  tle\0text".to_vec()),
        ("latin1 high", b"Ti\xe9tle\0te\xe9xt".to_vec()),
    ] {
        cases.push((format!("tEXt {name}"), insert(Chunk::new(b"tEXt", payload.clone()))));
        let mut z = payload.clone();
        z.push(0); // compression method byte tacked on
        cases.push((format!("zTXt {name}"), insert(Chunk::new(b"zTXt", z))));
        cases.push((format!("iTXt {name}"), insert(Chunk::new(b"iTXt", payload))));
    }
    // a zTXt with a valid keyword but broken deflate stream
    cases.push((
        "zTXt bad deflate".into(),
        insert(Chunk::new(b"zTXt", b"Title\0\0\xff\xff\xff\xff".to_vec())),
    ));
    // an iTXt with all the fields present
    cases.push((
        "iTXt well formed".into(),
        insert(Chunk::new(b"iTXt", b"Title\0\0\0en\0Titel\0hello".to_vec())),
    ));
    cases.push((
        "iTXt compressed flag without method".into(),
        insert(Chunk::new(b"iTXt", b"Title\0\x01".to_vec())),
    ));

    for (name, data) in cases {
        for cfg in [
            Cfg::default(),
            Cfg { benign: true, ..Default::default() },
            Cfg { keep_unknown: Some(PNG_HANDLE_CHUNK_ALWAYS), ..Default::default() },
        ] {
            compare_bad(&name, &data, &cfg);
        }
    }
}

#[test]
fn limits() {
    let good = base_png(PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE);
    for cfg in [
        Cfg { user_limits: Some((1, 1)), ..Default::default() },
        Cfg { user_limits: Some((8, 4)), ..Default::default() },
        Cfg { user_limits: Some((7, 4)), ..Default::default() },
        Cfg { user_limits: Some((0, 0)), ..Default::default() },
        Cfg { chunk_cache_max: Some(1), ..Default::default() },
        Cfg { chunk_cache_max: Some(2), ..Default::default() },
        Cfg { chunk_malloc_max: Some(1), ..Default::default() },
        Cfg { chunk_malloc_max: Some(16), ..Default::default() },
        Cfg { chunk_malloc_max: Some(1), benign: true, ..Default::default() },
    ] {
        compare_bad("limits", &good, &cfg);
    }
}

#[test]
fn many_ancillary_chunks() {
    // exercise the chunk cache and the "too many" paths
    let good = base_png(PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE);
    let chunks = split_chunks(&good);
    let mut cs = chunks.clone();
    for i in 0..40u32 {
        let mut payload = format!("K{i}").into_bytes();
        payload.push(0);
        payload.extend_from_slice(b"value");
        cs.insert(1, Chunk::new(b"tEXt", payload));
        cs.insert(1, Chunk::new(b"unKn", vec![i as u8; 3]));
    }
    let data = assemble(&cs);
    for cfg in [
        Cfg::default(),
        Cfg { keep_unknown: Some(PNG_HANDLE_CHUNK_ALWAYS), ..Default::default() },
        Cfg { chunk_cache_max: Some(5), keep_unknown: Some(PNG_HANDLE_CHUNK_ALWAYS), ..Default::default() },
        Cfg { chunk_cache_max: Some(5), ..Default::default() },
    ] {
        compare_bad("many chunks", &data, &cfg);
    }
}
