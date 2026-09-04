//! The CONFIGURATION SURFACE: every combination of runtime options and input
//! shape that the C code actually branches on.  `CONFIGS.md` is generated from
//! this list, so the document and the tests cannot drift apart.

/// (group, description, scenario string)
pub type Row = (&'static str, String, String);

/// The 15 legal colour-type / bit-depth pairs of the PNG format.
pub const CT_BD: &[(u8, u8)] = &[
    (0, 1), (0, 2), (0, 4), (0, 8), (0, 16),
    (2, 8), (2, 16),
    (3, 1), (3, 2), (3, 4), (3, 8),
    (4, 8), (4, 16),
    (6, 8), (6, 16),
];

fn ctname(ct: u8) -> &'static str {
    match ct {
        0 => "GRAY",
        2 => "RGB",
        3 => "PALETTE",
        4 => "GRAY_ALPHA",
        6 => "RGBA",
        _ => "?",
    }
}

/* ------------------------------------------------------------------ */
/* B1: write — every colour type / depth / interlace / entry point      */
/* ------------------------------------------------------------------ */

pub fn rows_write_shapes() -> Vec<Row> {
    let mut v = Vec::new();
    for &(ct, bd) in CT_BD {
        for il in [0u8, 1] {
            for mode in ["rows", "image", "split", "png"] {
                v.push((
                    "B1",
                    format!(
                        "write {}/{}-bit interlace={} via png_write_{}",
                        ctname(ct), bd, il,
                        match mode {
                            "rows" => "rows",
                            "image" => "image",
                            "split" => "row (one row at a time)",
                            _ => "png",
                        }
                    ),
                    format!("wr|ct={ct}|bd={bd}|il={il}|mode={mode}|w=19|h=11|n=3|seed={}", 1000 + ct as u32 * 17 + bd as u32),
                ));
            }
        }
    }
    v
}

/* ------------------------------------------------------------------ */
/* B2: write — zlib option matrix                                      */
/* ------------------------------------------------------------------ */

pub fn rows_write_zlib() -> Vec<Row> {
    let mut v = Vec::new();
    for lvl in [0i32, 1, 3, 6, 9, -1] {
        v.push((
            "B2",
            format!("write RGB/8 with png_set_compression_level({lvl})"),
            format!("wr|ct=2|bd=8|w=23|h=17|lvl={lvl}|n=2|seed=2001"),
        ));
    }
    for strat in [0i32, 1, 2, 3, 4] {
        v.push((
            "B2",
            format!("write RGB/8 with png_set_compression_strategy({strat})"),
            format!("wr|ct=2|bd=8|w=23|h=17|strat={strat}|n=2|seed=2002"),
        ));
    }
    for wb in [8i32, 9, 11, 13, 15] {
        v.push((
            "B2",
            format!("write RGB/8 with png_set_compression_window_bits({wb})"),
            format!("wr|ct=2|bd=8|w=23|h=17|wb={wb}|n=2|seed=2003"),
        ));
    }
    for ml in [1i32, 4, 8, 9] {
        v.push((
            "B2",
            format!("write RGB/8 with png_set_compression_mem_level({ml})"),
            format!("wr|ct=2|bd=8|w=23|h=17|ml={ml}|n=2|seed=2004"),
        ));
    }
    for cbuf in [8u32, 64, 1024, 8192, 65536] {
        v.push((
            "B2",
            format!("write RGB/8 with png_set_compression_buffer_size({cbuf})"),
            format!("wr|ct=2|bd=8|w=61|h=41|cbuf={cbuf}|n=1|seed=2005"),
        ));
    }
    for tlvl in [0i32, 6, 9] {
        v.push((
            "B2",
            format!("write RGB/8 + zTXt with png_set_text_compression_level({tlvl})"),
            format!("wr|ct=2|bd=8|w=17|h=9|x=text|tlvl={tlvl}|n=1|seed=2006"),
        ));
    }
    v
}

/* ------------------------------------------------------------------ */
/* B3: write — row filter matrix                                       */
/* ------------------------------------------------------------------ */

pub fn rows_write_filters() -> Vec<Row> {
    let mut v = Vec::new();
    let filters: &[(&str, i32)] = &[
        ("PNG_NO_FILTERS", 0x00),
        ("PNG_FILTER_NONE", 0x08),
        ("PNG_FILTER_SUB", 0x10),
        ("PNG_FILTER_UP", 0x20),
        ("PNG_FILTER_AVG", 0x40),
        ("PNG_FILTER_PAETH", 0x80),
        ("PNG_FAST_FILTERS", 0x38),
        ("PNG_ALL_FILTERS", 0xf8),
        ("SUB|PAETH", 0x90),
        ("value 0 (fixed filter NONE)", 0),
        ("value 4 (fixed filter PAETH)", 4),
    ];
    for &(name, mask) in filters {
        for &(ct, bd) in &[(2u8, 8u8), (0, 1), (6, 16), (3, 4)] {
            v.push((
                "B3",
                format!("write {}/{}-bit with png_set_filter(0, {name})", ctname(ct), bd),
                format!("wr|ct={ct}|bd={bd}|w=29|h=13|filt={mask}|n=2|seed=3001"),
            ));
        }
    }
    v
}

/* ------------------------------------------------------------------ */
/* B4: write — transforms                                             */
/* ------------------------------------------------------------------ */

pub fn rows_write_transforms() -> Vec<Row> {
    let mut v = Vec::new();
    let cases: &[(u8, u8, &str)] = &[
        (2, 8, "bgr"),
        (2, 16, "bgr"),
        (6, 8, "bgr"),
        (6, 8, "invalpha"),
        (6, 8, "swapalpha"),
        (6, 8, "bgr+invalpha+swapalpha"),
        (6, 16, "invalpha"),
        (4, 8, "invalpha"),
        (4, 8, "swapalpha"),
        (2, 16, "swap16"),
        (6, 16, "swap16"),
        (0, 16, "swap16"),
        (0, 1, "packing"),
        (0, 2, "packing"),
        (0, 4, "packing"),
        (3, 1, "packing"),
        (3, 2, "packing"),
        (3, 4, "packing"),
        (0, 1, "packswap"),
        (0, 2, "packswap"),
        (0, 4, "packswap"),
        (3, 4, "packswap"),
        (0, 1, "packing+packswap"),
        (0, 1, "invmono"),
        (0, 8, "invmono"),
        (4, 8, "invmono"),
        (0, 8, "shift"),
        (0, 16, "shift"),
        (2, 16, "shift"),
        (6, 16, "shift"),
        (2, 8, "filler_after"),
        (2, 8, "filler_before"),
        (0, 8, "filler_after"),
        (2, 16, "filler_after"),
    ];
    for &(ct, bd, tr) in cases {
        v.push((
            "B4",
            format!("write {}/{}-bit with transform(s) {tr}", ctname(ct), bd),
            format!("wr|ct={ct}|bd={bd}|w=21|h=9|tr={tr}|mode=split|n=2|seed=4001"),
        ));
    }
    // png_write_png transform mask
    for (name, mask) in [
        ("IDENTITY", 0x0000),
        ("PACKING", 0x0004),
        ("PACKSWAP", 0x0008),
        ("INVERT_MONO", 0x0020),
        ("SHIFT", 0x0040),
        ("BGR", 0x0080),
        ("SWAP_ALPHA", 0x0100),
        ("SWAP_ENDIAN", 0x0200),
        ("INVERT_ALPHA", 0x0400),
        ("BGR|INVERT_ALPHA", 0x0480),
    ] {
        let (ct, bd) = match mask {
            0x0004 | 0x0008 | 0x0020 => (0u8, 4u8),
            0x0040 => (2, 8),
            0x0100 | 0x0400 | 0x0480 => (6, 8),
            0x0200 => (2, 16),
            _ => (6, 8),
        };
        v.push((
            "B4",
            format!("png_write_png({}) on {}/{}-bit", name, ctname(ct), bd),
            format!("wr|ct={ct}|bd={bd}|w=15|h=7|mode=png|wt={mask}|n=2|seed=4002"),
        ));
    }
    v
}

/* ------------------------------------------------------------------ */
/* B5: write — ancillary chunk sets                                    */
/* ------------------------------------------------------------------ */

pub fn rows_write_chunks() -> Vec<Row> {
    let mut v = Vec::new();
    let sets = [
        "none", "gama", "chrm", "gamachrm", "srgb", "text", "time", "physoffs", "sbit", "trns",
        "bkgd", "iccp", "unk", "gamachrmtextphysoffstimesbit",
    ];
    for s in sets {
        for &(ct, bd) in &[(2u8, 8u8), (3, 8), (0, 16), (6, 8), (4, 8)] {
            v.push((
                "B5",
                format!("write {}/{}-bit with ancillary set [{s}]", ctname(ct), bd),
                format!("wr|ct={ct}|bd={bd}|w=13|h=7|x={s}|n=1|seed=5001"),
            ));
        }
    }
    v
}

/* ------------------------------------------------------------------ */
/* B6: write — output plumbing (flush, status callback, raw chunks)     */
/* ------------------------------------------------------------------ */

pub fn rows_write_io() -> Vec<Row> {
    let mut v = Vec::new();
    for flush in [0u32, 1, 3, 100] {
        v.push((
            "B6",
            format!("write with png_set_flush({flush}) + png_write_flush"),
            format!("wr|ct=2|bd=8|w=17|h=11|flush={flush}|mode=split|n=1|seed=6001"),
        ));
    }
    v.push((
        "B6",
        "write with png_set_write_status_fn callback".to_string(),
        "wr|ct=2|bd=8|w=17|h=11|wstat=1|mode=split|n=1|seed=6002".to_string(),
    ));
    v.push((
        "B6",
        "png_write_sig + png_write_chunk / _start / _data / _end directly".to_string(),
        "wr|ct=2|bd=8|w=8|h=4|mode=chunks|n=1|seed=6003".to_string(),
    ));
    for &(w, h) in &[(1u32, 1u32), (1, 33), (33, 1), (2, 2), (7, 1), (1, 7), (64, 64)] {
        v.push((
            "B6",
            format!("write extreme shape {w}x{h}"),
            format!("wr|ct=6|bd=8|w={w}|h={h}|n=1|seed=6004"),
        ));
    }
    v
}

/* ------------------------------------------------------------------ */
/* B7: read — every colour type / depth / interlace / entry point       */
/* ------------------------------------------------------------------ */

pub fn rows_read_shapes() -> Vec<Row> {
    let mut v = Vec::new();
    for &(ct, bd) in CT_BD {
        for il in [0u8, 1] {
            for mode in ["image", "rows", "row", "rowonly", "disponly", "png", "startimage"] {
                v.push((
                    "B7",
                    format!(
                        "read {}/{}-bit interlace={} via {}",
                        ctname(ct), bd, il,
                        match mode {
                            "image" => "png_read_image",
                            "rows" => "png_read_rows",
                            "row" => "png_read_row (row+display)",
                            "rowonly" => "png_read_row (row only)",
                            "disponly" => "png_read_row (display only)",
                            "startimage" => "png_start_read_image + png_read_image (no png_read_update_info)",
                            _ => "png_read_png",
                        }
                    ),
                    format!("rd|ct={ct}|bd={bd}|il={il}|mode={mode}|w=19|h=11|n=3|seed={}", 7000 + ct as u32 * 31 + bd as u32),
                ));
            }
        }
    }
    v
}

/* ------------------------------------------------------------------ */
/* B8: read — transform matrix                                         */
/* ------------------------------------------------------------------ */

pub fn rows_read_transforms() -> Vec<Row> {
    let mut v = Vec::new();
    let cases: &[(u8, u8, &str)] = &[
        (0, 1, "expand"),
        (0, 2, "expand"),
        (0, 4, "expand"),
        (0, 1, "expandgray"),
        (0, 4, "expandgray"),
        (3, 8, "expand"),
        (3, 4, "expand"),
        (3, 8, "pal2rgb"),
        (0, 8, "expand16"),
        (2, 8, "expand16"),
        (3, 8, "expand16"),
        (6, 8, "expand16"),
        (0, 1, "expand+expand16"),
        (2, 8, "bgr"),
        (6, 8, "bgr"),
        (2, 16, "bgr"),
        (0, 8, "gray2rgb"),
        (4, 8, "gray2rgb"),
        (0, 1, "expand+gray2rgb"),
        (2, 8, "rgb2gray"),
        (6, 8, "rgb2gray"),
        (2, 16, "rgb2gray"),
        (2, 8, "rgb2graywarn"),
        (6, 8, "stripalpha"),
        (4, 8, "stripalpha"),
        (6, 16, "stripalpha"),
        (6, 8, "swapalpha"),
        (4, 8, "swapalpha"),
        (6, 8, "invalpha"),
        (4, 16, "invalpha"),
        (6, 8, "swapalpha+invalpha"),
        (0, 16, "swap16"),
        (2, 16, "swap16"),
        (6, 16, "swap16"),
        (0, 1, "packing"),
        (0, 2, "packing"),
        (0, 4, "packing"),
        (3, 1, "packing"),
        (3, 2, "packing"),
        (3, 4, "packing"),
        (0, 1, "packswap"),
        (0, 2, "packswap"),
        (0, 4, "packswap"),
        (0, 1, "packing+packswap"),
        (0, 1, "invmono"),
        (0, 8, "invmono"),
        (4, 8, "invmono"),
        (0, 16, "strip16"),
        (2, 16, "strip16"),
        (6, 16, "strip16"),
        (0, 16, "scale16"),
        (2, 16, "scale16"),
        (6, 16, "scale16"),
        (2, 8, "filler_after"),
        (2, 8, "filler_before"),
        (0, 8, "filler_after"),
        (2, 16, "filler_after"),
        (2, 8, "addalpha_after"),
        (2, 8, "addalpha_before"),
        (0, 8, "addalpha_after"),
        (0, 8, "shift"),
        (2, 8, "shift"),
        (2, 16, "shift"),
        (6, 8, "shift"),
        (2, 8, "gamma"),
        (0, 8, "gamma"),
        (6, 8, "gamma"),
        (2, 16, "gamma"),
        (3, 8, "expand+gamma"),
        (2, 8, "gammahigh"),
        (6, 8, "alphapng"),
        (6, 8, "alphastd"),
        (6, 8, "alphaopt"),
        (6, 8, "alphabroken"),
        (4, 16, "alphastd"),
        (6, 8, "background"),
        (4, 8, "background"),
        (6, 16, "background"),
        (3, 8, "expand+backgroundexp"),
        (6, 8, "backgroundunique"),
        (6, 8, "background+gamma"),
        (2, 8, "quantize"),
        (6, 8, "quantize"),
        (2, 8, "expand+bgr+invalpha+addalpha_after"),
        (0, 4, "expand+gray2rgb+addalpha_after+swap16"),
        (6, 16, "strip16+bgr+swapalpha"),
        (6, 16, "scale16+stripalpha"),
        (3, 8, "expand+rgb2gray"),
        (3, 8, "expand+gamma+background"),
    ];
    for &(ct, bd, tr) in cases {
        for il in [0u8, 1] {
            v.push((
                "B8",
                format!("read {}/{}-bit interlace={il} with transform(s) {tr}", ctname(ct), bd),
                format!("rd|ct={ct}|bd={bd}|il={il}|w=19|h=11|tr={tr}|mode=image|n=2|seed=8001"),
            ));
        }
    }
    v
}

/* ------------------------------------------------------------------ */
/* B9: read — png_read_png transform mask                              */
/* ------------------------------------------------------------------ */

pub fn rows_read_png_masks() -> Vec<Row> {
    let mut v = Vec::new();
    let masks: &[(&str, i32, u8, u8)] = &[
        ("IDENTITY", 0x0000, 2, 8),
        ("STRIP_16", 0x0001, 2, 16),
        ("STRIP_ALPHA", 0x0002, 6, 8),
        ("PACKING", 0x0004, 0, 4),
        ("PACKSWAP", 0x0008, 0, 2),
        ("EXPAND", 0x0010, 3, 8),
        ("INVERT_MONO", 0x0020, 0, 1),
        ("SHIFT", 0x0040, 2, 8),
        ("BGR", 0x0080, 2, 8),
        ("SWAP_ALPHA", 0x0100, 6, 8),
        ("SWAP_ENDIAN", 0x0200, 2, 16),
        ("INVERT_ALPHA", 0x0400, 6, 8),
        ("GRAY_TO_RGB", 0x2000, 0, 8),
        ("EXPAND_16", 0x4000, 2, 8),
        ("SCALE_16", 0x8000, 2, 16),
        ("EXPAND|GRAY_TO_RGB", 0x2010, 3, 4),
        ("STRIP_16|BGR|INVERT_ALPHA", 0x0481, 6, 16),
    ];
    for &(name, mask, ct, bd) in masks {
        for il in [0u8, 1] {
            v.push((
                "B9",
                format!("png_read_png({name}) on {}/{}-bit interlace={il}", ctname(ct), bd),
                format!("rd|ct={ct}|bd={bd}|il={il}|w=17|h=9|mode=png|rt={mask}|n=2|seed=9001"),
            ));
        }
    }
    v
}

/* ------------------------------------------------------------------ */
/* B10: read — ancillary chunk sets and stream layout                  */
/* ------------------------------------------------------------------ */

pub fn rows_read_chunks() -> Vec<Row> {
    let mut v = Vec::new();
    let sets = [
        "none", "gama", "chrm", "gamachrm", "srgb", "sbit", "trns", "bkgd", "hist", "phys",
        "offs", "scal", "pcal", "splt", "text", "time", "exif", "cicp", "clli", "mdcv", "iccp",
        "unk", "tail", "plte",
        "gamachrmsbittrnsbkgdphysoffsscaltextexiftail",
    ];
    for s in sets {
        for &(ct, bd) in &[(2u8, 8u8), (3, 8), (0, 8), (6, 8), (4, 16)] {
            v.push((
                "B10",
                format!("read {}/{}-bit with ancillary set [{s}]", ctname(ct), bd),
                format!("rd|ct={ct}|bd={bd}|w=13|h=7|x={s}|mode=image|n=1|seed=10001"),
            ));
        }
    }
    for split in [1u32, 2, 3, 7, 64, 0] {
        v.push((
            "B10",
            format!("read with IDAT split into {} byte pieces", if split == 0 { "single".to_string() } else { split.to_string() }),
            format!("rd|ct=2|bd=8|w=31|h=17|split={split}|mode=image|n=1|seed=10002"),
        ));
    }
    for &(crit, anc) in &[(0i32, 0i32), (1, 1), (2, 2), (3, 3), (4, 4), (5, 5)] {
        v.push((
            "B10",
            format!("read with png_set_crc_action({crit}, {anc})"),
            format!("rd|ct=2|bd=8|w=13|h=7|x=gamatext|crc={crit}|crca={anc}|mode=image|n=1|seed=10003"),
        ));
    }
    for opt in [2i32, 4, 8] {
        for onoff in [2i32, 3] {
            v.push((
                "B10",
                format!("read with png_set_option({opt}, {onoff})"),
                format!("rd|ct=2|bd=8|w=13|h=7|x=iccp|opt={opt}|optv={onoff}|mode=image|n=1|seed=10004"),
            ));
        }
    }
    for idx in [0i32, 1] {
        v.push((
            "B10",
            format!("read palette image with png_set_check_for_invalid_index({idx})"),
            format!("rd|ct=3|bd=8|w=13|h=7|idx={idx}|mode=image|n=1|seed=10005"),
        ));
    }
    v.push((
        "B10",
        "read with png_set_read_status_fn callback".to_string(),
        "rd|ct=2|bd=8|w=13|h=7|rstat=1|mode=image|n=1|seed=10006".to_string(),
    ));
    v.push((
        "B10",
        "read with png_permit_mng_features(PNG_ALL_MNG_FEATURES)".to_string(),
        "rd|ct=2|bd=8|w=13|h=7|mng=5|mode=image|n=1|seed=10007".to_string(),
    ));
    v.push((
        "B10",
        "read with png_set_benign_errors(1)".to_string(),
        "rd|ct=2|bd=8|w=13|h=7|benign=1|mode=image|n=1|seed=10008".to_string(),
    ));
    for &(w, h) in &[(1u32, 1u32), (1, 33), (33, 1), (2, 2), (7, 1), (1, 7), (64, 64), (8, 8)] {
        for il in [0u8, 1] {
            v.push((
                "B10",
                format!("read extreme shape {w}x{h} interlace={il}"),
                format!("rd|ct=6|bd=8|il={il}|w={w}|h={h}|mode=image|n=1|seed=10009"),
            ));
        }
    }
    v
}

/* ------------------------------------------------------------------ */
/* B11: unknown-chunk handling matrix                                  */
/* ------------------------------------------------------------------ */

pub fn rows_unknown() -> Vec<Row> {
    let mut v = Vec::new();
    for keep in [0i32, 1, 2, 3] {
        for cb in [0i32, 1] {
            v.push((
                "B11",
                format!("png_set_keep_unknown_chunks(default={keep}), user callback={cb}"),
                format!("unk|keep={keep}|cb={cb}|seed=11001"),
            ));
        }
    }
    for keep2 in [0i32, 1, 2, 3] {
        v.push((
            "B11",
            format!("per-chunk keep={keep2} for prVt,prIv,tEXt,gAMA"),
            format!("unk|keep=1|keep2={keep2}|list=prVt,prIv,tEXt,gAMA|seed=11002"),
        ));
    }
    v
}

/* ------------------------------------------------------------------ */
/* B12: progressive (push) reader                                      */
/* ------------------------------------------------------------------ */

pub fn rows_progressive() -> Vec<Row> {
    let mut v = Vec::new();
    for &(ct, bd) in CT_BD {
        for il in [0u8, 1] {
            v.push((
                "B12",
                format!("progressive read {}/{}-bit interlace={il}", ctname(ct), bd),
                format!("prog|ct={ct}|bd={bd}|il={il}|w=19|h=11|feed=7|seed=12001"),
            ));
        }
    }
    for feed in [1u32, 2, 3, 5, 13, 100, 100000] {
        v.push((
            "B12",
            format!("progressive read fed {feed} bytes at a time"),
            format!("prog|ct=6|bd=8|w=23|h=13|feed={feed}|seed=12002"),
        ));
    }
    for pause in [1u32, 2, 5] {
        v.push((
            "B12",
            format!("progressive read with png_process_data_pause every {pause} feeds"),
            format!("prog|ct=2|bd=8|w=23|h=13|feed=11|pause={pause}|seed=12003"),
        ));
    }
    for x in ["gamachrmtext", "trnsbkgd", "unktail", "iccp", "splthist"] {
        v.push((
            "B12",
            format!("progressive read with ancillary set [{x}]"),
            format!("prog|ct=3|bd=8|w=17|h=9|x={x}|feed=9|seed=12004"),
        ));
    }
    for split in [1u32, 3, 20] {
        v.push((
            "B12",
            format!("progressive read of stream with IDAT split into {split} byte chunks"),
            format!("prog|ct=2|bd=8|w=29|h=13|split={split}|feed=7|seed=12005"),
        ));
    }
    v
}

/* ------------------------------------------------------------------ */
/* B13: simplified read API                                            */
/* ------------------------------------------------------------------ */

pub const SIMPLE_FORMATS: &[(&str, u32)] = &[
    ("PNG_FORMAT_GRAY", 0),
    ("PNG_FORMAT_GA", 1),
    ("PNG_FORMAT_AG", 0x21),
    ("PNG_FORMAT_RGB", 2),
    ("PNG_FORMAT_BGR", 0x12),
    ("PNG_FORMAT_RGBA", 3),
    ("PNG_FORMAT_ARGB", 0x23),
    ("PNG_FORMAT_BGRA", 0x13),
    ("PNG_FORMAT_ABGR", 0x33),
    ("PNG_FORMAT_LINEAR_Y", 4),
    ("PNG_FORMAT_LINEAR_Y_ALPHA", 5),
    ("PNG_FORMAT_LINEAR_RGB", 6),
    ("PNG_FORMAT_LINEAR_RGB_ALPHA", 7),
    ("PNG_FORMAT_RGB_COLORMAP", 0x0a),
    ("PNG_FORMAT_BGR_COLORMAP", 0x1a),
    ("PNG_FORMAT_RGBA_COLORMAP", 0x0b),
    ("PNG_FORMAT_ARGB_COLORMAP", 0x2b),
    ("PNG_FORMAT_GRAY_COLORMAP", 0x08),
    ("PNG_FORMAT_GA_COLORMAP", 0x09),
];

pub fn rows_simple_read() -> Vec<Row> {
    let mut v = Vec::new();
    for &(fname, fmt) in SIMPLE_FORMATS {
        for &(ct, bd) in &[(0u8, 8u8), (2, 8), (3, 8), (4, 8), (6, 8), (0, 16), (6, 16), (0, 1), (3, 4)] {
            v.push((
                "B13",
                format!("simplified read of {}/{}-bit into {fname}", ctname(ct), bd),
                format!("sr|ct={ct}|bd={bd}|w=17|h=11|fmt={fmt}|n=1|seed=13001"),
            ));
        }
    }
    for il in [0u8, 1] {
        for &(fname, fmt) in &[("PNG_FORMAT_RGBA", 3u32), ("PNG_FORMAT_LINEAR_RGB_ALPHA", 7)] {
            v.push((
                "B13",
                format!("simplified read interlace={il} into {fname}"),
                format!("sr|ct=6|bd=8|il={il}|w=17|h=11|fmt={fmt}|n=1|seed=13002"),
            ));
        }
    }
    v.push((
        "B13",
        "simplified read with negative row_stride (bottom-up buffer)".to_string(),
        "sr|ct=6|bd=8|w=17|h=11|fmt=3|neg=1|n=1|seed=13003".to_string(),
    ));
    for bgfmt in [0u32, 2, 0x0a] {
        v.push((
            "B13",
            format!("simplified read of RGBA source into format {bgfmt:#x} with a background colour"),
            format!("sr|ct=6|bd=8|w=17|h=11|fmt={bgfmt}|bg=1|n=1|seed=13004"),
        ));
    }
    for flags in [1u32, 2, 4] {
        v.push((
            "B13",
            format!("simplified read with png_image flags {flags:#x}"),
            format!("sr|ct=6|bd=16|w=17|h=11|fmt=7|flags={flags}|n=1|seed=13005"),
        ));
    }
    for x in ["gama", "srgb", "chrm", "trns", "bkgd", "iccp", "gamachrm"] {
        v.push((
            "B13",
            format!("simplified read of source with ancillary set [{x}]"),
            format!("sr|ct=2|bd=8|w=17|h=11|x={x}|fmt=3|n=1|seed=13006"),
        ));
    }
    v
}

/* ------------------------------------------------------------------ */
/* B14: simplified write API                                           */
/* ------------------------------------------------------------------ */

pub fn rows_simple_write() -> Vec<Row> {
    let mut v = Vec::new();
    for &(fname, fmt) in SIMPLE_FORMATS {
        let cme = if fmt & 0x08 != 0 { 64 } else { 0 };
        for c8 in [0i32, 1] {
            v.push((
                "B14",
                format!("simplified write {fname} convert_to_8bit={c8}"),
                format!("sw|fmt={fmt}|cme={cme}|c8={c8}|w=17|h=11|n=1|seed=14001"),
            ));
        }
    }
    for flags in [0u32, 1, 2] {
        v.push((
            "B14",
            format!("simplified write RGBA with flags {flags:#x}"),
            format!("sw|fmt=3|flags={flags}|w=23|h=13|n=1|seed=14002"),
        ));
    }
    v.push((
        "B14",
        "simplified write with negative row_stride".to_string(),
        "sw|fmt=3|neg=1|w=17|h=11|n=1|seed=14003".to_string(),
    ));
    for &(w, h) in &[(1u32, 1u32), (1, 40), (40, 1), (2, 3), (64, 64)] {
        v.push((
            "B14",
            format!("simplified write shape {w}x{h}"),
            format!("sw|fmt=3|w={w}|h={h}|n=1|seed=14004"),
        ));
    }
    for cme in [1u32, 2, 16, 255, 256] {
        v.push((
            "B14",
            format!("simplified write colour-mapped with {cme} entries"),
            format!("sw|fmt=10|cme={cme}|w=17|h=11|n=1|seed=14005"),
        ));
    }
    v
}

/* ------------------------------------------------------------------ */
/* B15: info setter/getter surface and library-wide state              */
/* ------------------------------------------------------------------ */

pub fn rows_setget() -> Vec<Row> {
    let mut v = Vec::new();
    for g in [
        "ihdr", "gama", "chrm", "plte", "trns", "misc", "scal", "newchunks", "text", "iccp",
        "splt", "pcal", "exif", "hist", "bkgd",
    ] {
        for seed in [1u32, 2] {
            v.push((
                "B15",
                format!("randomized png_set_* / png_get_* round trip: {g} (seed {seed})"),
                format!("sg|g={g}|seed={seed}"),
            ));
        }
    }
    for seed in [1u32, 2] {
        v.push((
            "B15",
            format!("user limits, png_set_option matrix, MNG features, allocator (seed {seed})"),
            format!("lim|seed={seed}"),
        ));
    }
    for f in ["version", "graypal"] {
        v.push((
            "B15",
            format!("library-wide accessor: {f}"),
            format!("util|f={f}"),
        ));
    }
    for (f, seed) in [("sigcmp", 1u32), ("sigcmp", 2), ("intfns", 1), ("intfns", 77), ("uint31", 3), ("rfc1123", 5), ("rfc1123", 6), ("timet", 7)] {
        v.push((
            "B15",
            format!("pure utility function {f} over randomized inputs (seed {seed})"),
            format!("util|f={f}|seed={seed}"),
        ));
    }
    v
}

/* ------------------------------------------------------------------ */
/* B16/B17: randomized cross-product sweeps                            */
/* ------------------------------------------------------------------ */

const READ_TRANSFORMS: &[&str] = &[
    "expand", "expandgray", "pal2rgb", "trns2alpha", "expand16", "bgr", "gray2rgb", "rgb2gray",
    "rgb2graywarn", "stripalpha", "swapalpha", "invalpha", "swap16", "packing", "packswap",
    "invmono", "strip16", "scale16", "filler_before", "filler_after", "addalpha_before",
    "addalpha_after", "shift", "gamma", "gammahigh", "alphapng", "alphastd", "alphaopt",
    "alphabroken", "background", "backgroundexp", "backgroundunique", "quantize", "interlace",
];

const EXTRA_SETS: &[&str] = &[
    "none", "gama", "srgb", "chrm", "gamachrm", "sbit", "trns", "bkgd", "trnsbkgd", "hist",
    "physoffs", "scal", "pcal", "splt", "text", "time", "exif", "cicp", "clli", "mdcv", "iccp",
    "unk", "tail", "plte", "gamachrmsbittrnsbkgdtexttail",
];

/// A deterministic sweep over (colour type, depth, interlace, size, transform
/// subset, entry point, ancillary chunk set) for the *read* pipeline.
fn fuzz_scale() -> u32 {
    // `PNGDIFF_FUZZ=<n>` multiplies the size of the randomized sweeps; the
    // default keeps `cargo test` fast while still covering the cross-product.
    std::env::var("PNGDIFF_FUZZ")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1u32)
        .clamp(1, 200)
}

pub fn rows_read_fuzz() -> Vec<Row> {
    let mut rng = super::rng::Rng::new(0xF00D_1234);
    let modes = ["image", "rows", "row", "rowonly", "disponly", "startimage"];
    let mut v = Vec::new();
    for i in 0..420u32 * fuzz_scale() {
        let (ct, bd) = CT_BD[(i as usize) % CT_BD.len()];
        let il = (rng.below(2)) as u8;
        let w = rng.range(1, 40);
        let h = rng.range(1, 24);
        let ntr = rng.below(4);
        let mut trs: Vec<&str> = Vec::new();
        for _ in 0..ntr {
            let t = rng.pick(READ_TRANSFORMS);
            if !trs.contains(&t) {
                trs.push(t);
            }
        }
        let tr = if trs.is_empty() { "none".to_string() } else { trs.join("+") };
        let mode = rng.pick(&modes);
        let x = rng.pick(EXTRA_SETS);
        let split = rng.pick(&[0u32, 1, 3, 17]);
        let seed = 16000 + i;
        v.push((
            "B16",
            format!(
                "fuzz read {}/{}-bit il={il} {w}x{h} tr=[{tr}] via {mode} chunks=[{x}] idat_split={split}",
                ctname(ct), bd
            ),
            format!("rd|ct={ct}|bd={bd}|il={il}|w={w}|h={h}|tr={tr}|mode={mode}|x={x}|split={split}|n=2|seed={seed}"),
        ));
    }
    v
}

const WRITE_TRANSFORMS: &[&str] = &[
    "bgr", "swap16", "packing", "packswap", "invmono", "invalpha", "swapalpha", "shift",
    "filler_before", "filler_after",
];

/// The same idea for the *write* pipeline, including the zlib option axes.
pub fn rows_write_fuzz() -> Vec<Row> {
    let mut rng = super::rng::Rng::new(0xBEEF_5678);
    let modes = ["rows", "image", "split", "png"];
    let mut v = Vec::new();
    for i in 0..320u32 * fuzz_scale() {
        let (ct, bd) = CT_BD[(i as usize) % CT_BD.len()];
        let il = (rng.below(2)) as u8;
        let w = rng.range(1, 40);
        let h = rng.range(1, 24);
        // Only transforms whose input layout we can construct for this shape.
        let mut trs: Vec<&str> = Vec::new();
        for _ in 0..rng.below(3) {
            let t = rng.pick(WRITE_TRANSFORMS);
            let legal = match t {
                "packing" | "packswap" => bd < 8,
                "swap16" => bd == 16,
                "invalpha" | "swapalpha" => ct == 4 || ct == 6,
                "bgr" => ct == 2 || ct == 6,
                "invmono" => ct == 0 || ct == 4,
                "filler_before" | "filler_after" => ct == 0 || ct == 2,
                _ => true,
            };
            if legal && !trs.contains(&t) {
                trs.push(t);
            }
        }
        if trs.iter().any(|t| t.starts_with("filler")) && trs.len() > 1 {
            trs.retain(|t| t.starts_with("filler"));
        }
        let tr = if trs.is_empty() { "none".to_string() } else { trs.join("+") };
        let mode = rng.pick(&modes);
        let x = rng.pick(&[
            "none", "gama", "chrm", "srgb", "text", "time", "physoffs", "sbit", "trns", "bkgd",
            "iccp", "unk", "gamachrmtext",
        ]);
        let lvl = rng.pick(&[-1i32, 0, 1, 5, 9]);
        let strat = rng.pick(&[0i32, 1, 2, 3, 4]);
        let filt = rng.pick(&[0x00i32, 0x08, 0x10, 0x20, 0x40, 0x80, 0x38, 0xf8]);
        let seed = 17000 + i;
        v.push((
            "B17",
            format!(
                "fuzz write {}/{}-bit il={il} {w}x{h} tr=[{tr}] via {mode} chunks=[{x}] level={lvl} strategy={strat} filters={filt:#04x}",
                ctname(ct), bd
            ),
            format!("wr|ct={ct}|bd={bd}|il={il}|w={w}|h={h}|tr={tr}|mode={mode}|x={x}|lvl={lvl}|strat={strat}|filt={filt}|n=2|seed={seed}"),
        ));
    }
    v
}

/// Large images: exercises multi-buffer zlib output, filter selection over long
/// rows and the >64KB row guard.
pub fn rows_large() -> Vec<Row> {
    let mut v = Vec::new();
    for &(ct, bd) in &[(2u8, 8u8), (6, 16), (0, 1), (3, 8)] {
        for il in [0u8, 1] {
            v.push((
                "B18",
                format!("write then read back a 257x131 {}/{}-bit il={il} image", ctname(ct), bd),
                format!("wr|ct={ct}|bd={bd}|il={il}|w=257|h=131|filt=248|n=1|seed=18001"),
            ));
            v.push((
                "B18",
                format!("read a 257x131 {}/{}-bit il={il} image", ctname(ct), bd),
                format!("rd|ct={ct}|bd={bd}|il={il}|w=257|h=131|mode=image|n=1|seed=18002"),
            ));
        }
    }
    v.push((
        "B18",
        "read a 1024x8 RGBA image with a 1-byte IDAT split".to_string(),
        "rd|ct=6|bd=8|w=1024|h=8|split=1|mode=image|n=1|seed=18003".to_string(),
    ));
    v.push((
        "B18",
        "progressive read of a 512x16 RGBA image fed 1 byte at a time".to_string(),
        "prog|ct=6|bd=8|w=512|h=16|feed=1|seed=18004".to_string(),
    ));
    v.push((
        "B18",
        "write a 512x64 16-bit RGBA image at compression level 9".to_string(),
        "wr|ct=6|bd=16|w=512|h=64|lvl=9|filt=248|n=1|seed=18005".to_string(),
    ));
    v.push((
        "B18",
        "simplified round trip through a 300x200 RGBA image".to_string(),
        "sw|fmt=3|w=300|h=200|n=1|seed=18006".to_string(),
    ));
    v
}

/* ------------------------------------------------------------------ */
/* B19..B24: user transforms, MNG, CRC actions, FP getters, stdio       */
/* ------------------------------------------------------------------ */

pub fn rows_user_transforms() -> Vec<Row> {
    let mut v = Vec::new();
    for &(ct, bd) in CT_BD {
        for il in [0u8, 1] {
            v.push((
                "B19",
                format!("read {}/{}-bit il={il} through a png_set_read_user_transform_fn callback", ctname(ct), bd),
                format!("ut|side=read|ct={ct}|bd={bd}|il={il}|w=19|h=9|seed=19001"),
            ));
            v.push((
                "B19",
                format!("write {}/{}-bit il={il} through a png_set_write_user_transform_fn callback", ctname(ct), bd),
                format!("ut|side=write|ct={ct}|bd={bd}|il={il}|w=19|h=9|seed=19002"),
            ));
        }
    }
    for tr in ["expand", "gray2rgb", "strip16", "bgr", "gamma", "expand16"] {
        v.push((
            "B19",
            format!("read user transform combined with {tr}"),
            format!("ut|side=read|ct=6|bd=16|w=17|h=7|tr={tr}|seed=19003"),
        ));
    }
    for (d, c) in [(8i32, 3i32), (16, 4), (1, 1), (0, 0)] {
        v.push((
            "B19",
            format!("read user transform with png_set_user_transform_info(depth={d}, channels={c})"),
            format!("ut|side=read|ct=2|bd=8|w=17|h=7|uti=1|utd={d}|utc={c}|seed=19004"),
        ));
    }
    v
}

pub fn rows_mng() -> Vec<Row> {
    let mut v = Vec::new();
    for &(ct, bd) in &[(2u8, 8u8), (2, 16), (6, 8), (6, 16)] {
        for permit in [0u32, 1, 4, 5] {
            v.push((
                "B20",
                format!(
                    "read MNG intrapixel (IHDR filter method 64) {}/{}-bit with png_permit_mng_features({permit})",
                    ctname(ct), bd
                ),
                format!("mng|f=filter64|ct={ct}|bd={bd}|permit={permit}|w=13|h=7|seed=20001"),
            ));
            v.push((
                "B20",
                format!(
                    "write MNG intrapixel {}/{}-bit with png_permit_mng_features({permit})",
                    ctname(ct), bd
                ),
                format!("mng|f=write64|ct={ct}|bd={bd}|permit={permit}|w=13|h=7|seed=20002"),
            ));
        }
    }
    for permit in [0u32, 1, 4, 5] {
        v.push((
            "B20",
            format!("read a palette image with an empty PLTE, png_permit_mng_features({permit})"),
            format!("mng|f=emptyplte|permit={permit}|seed=20003"),
        ));
    }
    // The intrapixel filter is only *accepted* when libpng did not validate the
    // first three signature bytes itself, so hand the stream over mid-signature.
    for &(ct, bd) in &[(2u8, 8u8), (2, 16), (6, 8), (6, 16), (0, 8), (3, 8)] {
        for permit in [0u32, 4, 5] {
            for skip in [3u32, 4, 8] {
                v.push((
                    "B20",
                    format!(
                        "MNG intrapixel read of {}/{}-bit with png_set_sig_bytes({skip}) and png_permit_mng_features({permit})",
                        ctname(ct), bd
                    ),
                    format!("mng|f=filter64sig|ct={ct}|bd={bd}|permit={permit}|skip={skip}|w=13|h=7|seed=20004"),
                ));
            }
        }
    }
    for skip in [0u32, 1, 2, 3, 4, 5, 6, 7, 8] {
        v.push((
            "B20",
            format!("read a normal stream handed over with png_set_sig_bytes({skip})"),
            format!("mng|f=sigbytes|ct=2|bd=8|skip={skip}|w=11|h=5|seed=20005"),
        ));
    }
    v
}

pub fn rows_crc_actions() -> Vec<Row> {
    let mut v = Vec::new();
    for chunk in ["IHDR", "gAMA", "IDAT", "tEXt", "IEND", "none"] {
        for crit in [0i32, 1, 2, 3, 4, 5] {
            for anc in [0i32, 2, 3, 4, 5] {
                v.push((
                    "B21",
                    format!("CRC error in {chunk} with png_set_crc_action(crit={crit}, ancil={anc})"),
                    format!("crc|chunk={chunk}|crit={crit}|anc={anc}"),
                ));
            }
        }
    }
    v
}

pub fn rows_fp_getters() -> Vec<Row> {
    (1..=3u32)
        .map(|seed| {
            (
                "B22",
                format!("floating-point getters (cHRM/cHRM_XYZ/cLLI/mDCV/sCAL/aspect/offset/gAMA) over randomized fixed-point inputs, seed {seed}"),
                format!("fpget|seed={seed}"),
            )
        })
        .collect()
}

pub fn rows_stdio() -> Vec<Row> {
    let mut v = Vec::new();
    for &(ct, bd) in &[(0u8, 1u8), (0, 8), (2, 8), (3, 8), (4, 16), (6, 8), (6, 16)] {
        v.push((
            "B23",
            format!("png_init_io round trip through a real FILE* for {}/{}-bit", ctname(ct), bd),
            format!("fileio|m=lowlevel|ct={ct}|bd={bd}|w=15|h=9|seed=23001"),
        ));
    }
    for &(fname, fmt) in &[
        ("PNG_FORMAT_GRAY", 0u32),
        ("PNG_FORMAT_RGB", 2),
        ("PNG_FORMAT_RGBA", 3),
        ("PNG_FORMAT_LINEAR_RGB_ALPHA", 7),
    ] {
        for c8 in [0i32, 1] {
            v.push((
                "B23",
                format!("png_image_write_to_file / begin_read_from_file with {fname}, convert_to_8bit={c8}"),
                format!("fileio|m=simple|fmt={fmt}|c8={c8}|w=15|h=9|seed=23002"),
            ));
        }
        v.push((
            "B23",
            format!("png_image_write_to_stdio / begin_read_from_stdio with {fname}"),
            format!("fileio|m=stdio|fmt={fmt}|w=15|h=9|seed=23003"),
        ));
    }
    v
}

pub fn rows_freedata() -> Vec<Row> {
    let masks: &[(u32, &str)] = &[
        (0xffff, "PNG_FREE_ALL"),
        (0x0008, "PNG_FREE_HIST"),
        (0x0010, "PNG_FREE_ICCP"),
        (0x0020, "PNG_FREE_SPLT"),
        (0x0040, "PNG_FREE_ROWS"),
        (0x0080, "PNG_FREE_PCAL"),
        (0x0100, "PNG_FREE_SCAL"),
        (0x0200, "PNG_FREE_UNKN"),
        (0x1000, "PNG_FREE_PLTE"),
        (0x2000, "PNG_FREE_TRNS"),
        (0x4000, "PNG_FREE_TEXT"),
        (0x8000, "PNG_FREE_EXIF"),
        (0x4220, "PNG_FREE_MUL"),
        (0x0000, "nothing"),
    ];
    masks
        .iter()
        .map(|(m, name)| {
            (
                "B24",
                format!("png_free_data({name}) then png_data_freer / png_set_invalid / png_destroy_info_struct"),
                format!("freedata|mask={m}"),
            )
        })
        .collect()
}

pub fn rows_heuristics() -> Vec<Row> {
    let mut v = Vec::new();
    for hm in [0i32, 1, 2, 3] {
        for nw in [0i32, 1, 3, 5] {
            v.push((
                "B25",
                format!("png_set_filter_heuristics(method={hm}, num_weights={nw}) then a filtered write"),
                format!("heur|hm={hm}|nw={nw}|seed=25001"),
            ));
        }
    }
    v
}

/* ------------------------------------------------------------------ */
/* C5/B26: mutation fuzzing and simplified-API fuzzing                  */
/* ------------------------------------------------------------------ */

/// Single-bit corruptions of a rich, valid datastream.  Every row is a distinct
/// (base image, corruption count, CRC handling, benign-error setting) combination
/// and internally tries several independently seeded mutations.
pub fn rows_mutation_fuzz() -> Vec<Row> {
    let mut v = Vec::new();
    let scale = fuzz_scale();
    let bases: &[(u8, u8, u8)] = &[(3, 8, 0), (2, 8, 0), (6, 16, 1), (0, 4, 0), (4, 8, 1)];
    for &(ct, bd, il) in bases {
        for k in [1u32, 2, 4, 8] {
            for fixcrc in [0i32, 1] {
                for benign in [-1i32, 0, 1] {
                    for rep in 0..scale {
                        v.push((
                            "C5",
                            format!(
                                "mutation fuzz: {} bit flip(s) in a valid {}/{}-bit il={il} stream (CRCs {}, benign_errors={})",
                                k,
                                ctname(ct),
                                bd,
                                if fixcrc != 0 { "recomputed" } else { "left broken" },
                                match benign { -1 => "default", 0 => "0", _ => "1" }
                            ),
                            format!(
                                "mut|ct={ct}|bd={bd}|il={il}|k={k}|fixcrc={fixcrc}|benign={benign}|n=8|seed={}",
                                26000 + rep * 977 + k * 13 + (fixcrc as u32) * 7 + (benign + 1) as u32
                            ),
                        ));
                    }
                }
            }
        }
    }
    v
}

/// Randomized sweep of the simplified read API over every source shape and
/// every output format, including negative strides, backgrounds and flags.
pub fn rows_simple_fuzz() -> Vec<Row> {
    let scale = fuzz_scale();
    (0..24u32 * scale)
        .map(|i| {
            (
                "B26",
                format!("simplified-API fuzz batch {i} (8 randomized source/format combinations)"),
                format!("sfuzz|n=8|seed={}", 27000 + i),
            )
        })
        .collect()
}

/* ------------------------------------------------------------------ */

pub fn all_rows() -> Vec<Row> {
    let mut v = Vec::new();
    v.extend(rows_write_shapes());
    v.extend(rows_write_zlib());
    v.extend(rows_write_filters());
    v.extend(rows_write_transforms());
    v.extend(rows_write_chunks());
    v.extend(rows_write_io());
    v.extend(rows_read_shapes());
    v.extend(rows_read_transforms());
    v.extend(rows_read_png_masks());
    v.extend(rows_read_chunks());
    v.extend(rows_unknown());
    v.extend(rows_progressive());
    v.extend(rows_simple_read());
    v.extend(rows_simple_write());
    v.extend(rows_setget());
    v.extend(rows_read_fuzz());
    v.extend(rows_write_fuzz());
    v.extend(rows_large());
    v.extend(rows_user_transforms());
    v.extend(rows_mng());
    v.extend(rows_crc_actions());
    v.extend(rows_fp_getters());
    v.extend(rows_stdio());
    v.extend(rows_freedata());
    v.extend(rows_heuristics());
    v.extend(rows_mutation_fuzz());
    v.extend(rows_simple_fuzz());
    v
}
