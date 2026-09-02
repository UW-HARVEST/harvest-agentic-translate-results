//! Phase B, group S: the SIMPLIFIED API (`png_image_*`).  Every `PNG_FORMAT_*`
//! flag combination is enumerated, including combinations the C rejects, plus
//! `convert_to_8_bit`, row-stride sign, colormaps, flags and buffer-size edge
//! cases.
mod common;
use common::*;
use std::ffi::{c_int, c_void};
use std::ptr;

const SEED: u64 = 0x5117_9911_2244_8800;

// The PNG_IMAGE_* macros from png.h, transcribed.
fn sample_channels(fmt: u32) -> u32 {
    (fmt & (PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA)) + 1
}
fn sample_component_size(fmt: u32) -> u32 {
    ((fmt & PNG_FORMAT_FLAG_LINEAR) >> 2) + 1
}
fn pixel_channels(fmt: u32) -> u32 {
    if fmt & PNG_FORMAT_FLAG_COLORMAP != 0 {
        1
    } else {
        sample_channels(fmt)
    }
}
fn pixel_component_size(fmt: u32) -> u32 {
    if fmt & PNG_FORMAT_FLAG_COLORMAP != 0 {
        1
    } else {
        sample_component_size(fmt)
    }
}
fn image_row_stride(fmt: u32, w: u32) -> u32 {
    pixel_channels(fmt) * w
}
fn image_size(fmt: u32, w: u32, h: u32) -> usize {
    pixel_component_size(fmt) as usize * h as usize * image_row_stride(fmt, w) as usize
}
fn colormap_size(fmt: u32, entries: u32) -> usize {
    (sample_channels(fmt) * sample_component_size(fmt)) as usize * entries as usize
}

/// All 64 combinations of the six documented format flag bits.
fn all_formats() -> Vec<u32> {
    (0u32..0x40).collect()
}

fn new_image(w: u32, h: u32, fmt: u32, flags: u32, cmap_entries: u32) -> PngImage {
    PngImage {
        opaque: ptr::null_mut(),
        version: PNG_IMAGE_VERSION,
        width: w,
        height: h,
        format: fmt,
        flags,
        colormap_entries: cmap_entries,
        warning_or_error: 0,
        message: [0; 64],
    }
}

fn log_image(tag: &str, im: &PngImage) {
    log_img(tag, im);
}

// ---------------------------------------------------------------------------
// S1..S6 png_image_write_to_memory over every format
// ---------------------------------------------------------------------------
#[test]
fn s1_s6_write_to_memory_all_formats() {
    let (c, r) = libs();
    let (w, h) = (7u32, 5u32);
    for fmt in all_formats() {
        for conv in [0i32, 1] {
            for flags in [0u32, 1, 2, 4, 3] {
                let cmap_entries = if fmt & PNG_FORMAT_FLAG_COLORMAP != 0 {
                    16
                } else {
                    0
                };
                let bufsz = image_size(fmt, w, h).max(1);
                let cmapsz = colormap_size(fmt, cmap_entries.max(1)).max(1);
                let mut rng = Rng::new(SEED ^ (fmt as u64) ^ ((conv as u64) << 8));
                let buf = rng.bytes(bufsz + 64);
                let cmap = rng.bytes(cmapsz + 64);
                let mut run = |l: &Lib| -> Report {
                    let buf = buf.clone();
                    let cmap = cmap.clone();
                    diff_scratch(&mut || unsafe {
                        // size query: memory == NULL
                        let mut im = new_image(w, h, fmt, flags, cmap_entries);
                        let mut need: usize = 0;
                        let ok = (l.api.png_image_write_to_memory)(
                            &mut im,
                            ptr::null_mut(),
                            &mut need,
                            conv,
                            buf.as_ptr() as *mut c_void,
                            0,
                            if cmap_entries > 0 {
                                cmap.as_ptr() as *mut c_void
                            } else {
                                ptr::null_mut()
                            },
                        );
                        log(format!("query ok={ok} need={need}"));
                        log_image("query", &im);
                        (l.api.png_image_free)(&mut im);

                        // real write into a generous buffer
                        let mut im = new_image(w, h, fmt, flags, cmap_entries);
                        let mut out = vec![0u8; 1 << 18];
                        let mut sz = out.len();
                        let ok = (l.api.png_image_write_to_memory)(
                            &mut im,
                            out.as_mut_ptr() as *mut c_void,
                            &mut sz,
                            conv,
                            buf.as_ptr() as *mut c_void,
                            0,
                            if cmap_entries > 0 {
                                cmap.as_ptr() as *mut c_void
                            } else {
                                ptr::null_mut()
                            },
                        );
                        log(format!("write ok={ok} sz={sz}"));
                        log_image("write", &im);
                        if ok != 0 && sz <= out.len() {
                            log(format!("png={:02x?}", &out[..sz]));
                        }
                        (l.api.png_image_free)(&mut im);
                    })
                };
                diff(
                    &format!("S1-S6 write_to_memory fmt={fmt:#x} conv={conv} flags={flags:#x}"),
                    &c,
                    &r,
                    &mut run,
                );
            }
        }
    }
}

/// Run a closure with a fresh recording Ctx (no png_struct involved).
fn diff_scratch(f: &mut dyn FnMut()) -> Report {
    let mut ctxb = Box::new(Ctx::default());
    set_ctx(&mut *ctxb as *mut Ctx);
    f();
    let rep = ctxb.digest();
    set_ctx(ptr::null_mut());
    rep
}

// ---------------------------------------------------------------------------
// S5 row_stride variants
// ---------------------------------------------------------------------------
#[test]
fn s5_row_stride_variants() {
    let (c, r) = libs();
    let (w, h) = (6u32, 4u32);
    for fmt in [0u32, 1, 2, 3, 4, 6, 7, 0x12, 0x23, 0x0a, 0x0b] {
        let min_stride = image_row_stride(fmt, w) as i32;
        for stride in [
            0i32,
            min_stride,
            min_stride + 3,
            -min_stride,
            -(min_stride + 3),
            min_stride - 1,
            1,
            -1,
        ] {
            let cmap_entries = if fmt & PNG_FORMAT_FLAG_COLORMAP != 0 { 8 } else { 0 };
            let mut rng = Rng::new(SEED ^ (fmt as u64) ^ ((stride as i64 as u64) << 16));
            let bufsz = image_size(fmt, w, h).max(1) + (min_stride.unsigned_abs() as usize + 8) * 4;
            let buf = rng.bytes(bufsz);
            let cmap = rng.bytes(colormap_size(fmt, cmap_entries.max(1)).max(1) + 32);
            let mut run = |l: &Lib| -> Report {
                let buf = buf.clone();
                let cmap = cmap.clone();
                diff_scratch(&mut || unsafe {
                    let mut im = new_image(w, h, fmt, 0, cmap_entries);
                    let mut out = vec![0u8; 1 << 17];
                    let mut sz = out.len();
                    // Verified against the reference C: the pointer handed to
                    // png_image_write_to_memory is the LOWEST address of the
                    // block for both stride signs; the sign only selects the
                    // row order within it.
                    let base = buf.as_ptr();
                    let ok = (l.api.png_image_write_to_memory)(
                        &mut im,
                        out.as_mut_ptr() as *mut c_void,
                        &mut sz,
                        0,
                        base as *mut c_void,
                        stride,
                        if cmap_entries > 0 {
                            cmap.as_ptr() as *mut c_void
                        } else {
                            ptr::null_mut()
                        },
                    );
                    log(format!("ok={ok} sz={sz}"));
                    log_image("img", &im);
                    if ok != 0 && sz <= out.len() {
                        log(format!("png={:02x?}", &out[..sz]));
                    }
                    (l.api.png_image_free)(&mut im);
                })
            };
            diff(
                &format!("S5 row_stride fmt={fmt:#x} stride={stride}"),
                &c,
                &r,
                &mut run,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// S7 exact / undersized output buffers
// ---------------------------------------------------------------------------
#[test]
fn s7_output_buffer_sizes() {
    let (c, r) = libs();
    let (w, h) = (9u32, 6u32);
    for fmt in [0u32, 2, 3, 6, 7, 0x0a] {
        let cmap_entries = if fmt & PNG_FORMAT_FLAG_COLORMAP != 0 { 32 } else { 0 };
        let mut rng = Rng::new(SEED ^ 0x777 ^ fmt as u64);
        let buf = rng.bytes(image_size(fmt, w, h).max(1) + 64);
        let cmap = rng.bytes(colormap_size(fmt, cmap_entries.max(1)).max(1) + 32);
        // Determine the exact size with the C implementation first (both must
        // agree; that is checked by S1).
        let need = unsafe {
            let mut im = new_image(w, h, fmt, 0, cmap_entries);
            let mut need: usize = 0;
            (c.api.png_image_write_to_memory)(
                &mut im,
                ptr::null_mut(),
                &mut need,
                0,
                buf.as_ptr() as *mut c_void,
                0,
                if cmap_entries > 0 {
                    cmap.as_ptr() as *mut c_void
                } else {
                    ptr::null_mut()
                },
            );
            (c.api.png_image_free)(&mut im);
            need
        };
        for delta in [0isize, -1, -2, 1, -(need as isize)] {
            let cap = (need as isize + delta).max(0) as usize;
            let mut run = |l: &Lib| -> Report {
                let buf = buf.clone();
                let cmap = cmap.clone();
                diff_scratch(&mut || unsafe {
                    let mut im = new_image(w, h, fmt, 0, cmap_entries);
                    let mut out = vec![0u8; cap.max(1)];
                    let mut sz = cap;
                    let ok = (l.api.png_image_write_to_memory)(
                        &mut im,
                        out.as_mut_ptr() as *mut c_void,
                        &mut sz,
                        0,
                        buf.as_ptr() as *mut c_void,
                        0,
                        if cmap_entries > 0 {
                            cmap.as_ptr() as *mut c_void
                        } else {
                            ptr::null_mut()
                        },
                    );
                    log(format!("cap={cap} ok={ok} sz={sz}"));
                    log_image("img", &im);
                    if ok != 0 && sz <= out.len() {
                        log(format!("png={:02x?}", &out[..sz]));
                    }
                    (l.api.png_image_free)(&mut im);
                })
            };
            diff(
                &format!("S7 buffer size fmt={fmt:#x} need={need} cap={cap}"),
                &c,
                &r,
                &mut run,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// S8..S13 simplified read
// ---------------------------------------------------------------------------
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

fn gen(cl: &Lib, w: u32, h: u32, ct: c_int, bd: c_int, il: c_int, gama: bool) -> Vec<u8> {
    let pal = if ct == PNG_COLOR_TYPE_PALETTE {
        make_palette(
            match bd {
                1 => 2,
                2 => 4,
                4 => 16,
                _ => 256,
            },
            SEED ^ 0x99,
        )
    } else {
        vec![]
    };
    let rep = write_full(
        cl,
        w,
        h,
        ct,
        bd,
        il,
        PNG_FILTER_TYPE_BASE,
        &pal,
        rowbytes(w, bd, ct),
        SEED ^ ((ct as u64) << 20) ^ (bd as u64) ^ ((il as u64) << 12),
        &mut |l, png, info| unsafe {
            if gama {
                (l.api.png_set_gAMA_fixed)(png, info, 45455);
            }
        },
    );
    assert!(rep.error.is_none());
    rep.out
}

#[test]
fn s8_read_native_format() {
    let (c, r) = libs();
    for &(ct, bd) in LEGAL {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            for gama in [false, true] {
                let stream = gen(&c, 11, 5, ct, bd, il, gama);
                let mut run = |l: &Lib| -> Report {
                    diff_scratch(&mut || unsafe {
                        let mut im = PngImage::default();
                        let ok = (l.api.png_image_begin_read_from_memory)(
                            &mut im,
                            stream.as_ptr() as *const c_void,
                            stream.len(),
                        );
                        log(format!("begin ok={ok}"));
                        log_image("begin", &im);
                        if ok != 0 {
                            let sz = image_size(im.format, im.width, im.height).max(1);
                            let mut buf = vec![0u8; sz + 64];
                            let cm = colormap_size(im.format, 256).max(1);
                            let mut cmap = vec![0u8; cm + 64];
                            let ok2 = (l.api.png_image_finish_read)(
                                &mut im,
                                ptr::null(),
                                buf.as_mut_ptr() as *mut c_void,
                                0,
                                cmap.as_mut_ptr() as *mut c_void,
                            );
                            log(format!("finish ok={ok2}"));
                            log_image("finish", &im);
                            log(format!("pixels={:02x?}", &buf[..sz]));
                            if im.format & PNG_FORMAT_FLAG_COLORMAP != 0 {
                                let n = colormap_size(im.format, im.colormap_entries);
                                log(format!("cmap={:02x?}", &cmap[..n]));
                            }
                        }
                        (l.api.png_image_free)(&mut im);
                        // free twice must be safe and identical
                        (l.api.png_image_free)(&mut im);
                        log_image("after free", &im);
                    })
                };
                diff(
                    &format!("S8/S13 simplified read ct={ct} bd={bd} il={il} gama={gama}"),
                    &c,
                    &r,
                    &mut run,
                );
            }
        }
    }
}

#[test]
fn s9_s12_read_with_format_override() {
    let (c, r) = libs();
    for &(ct, bd) in LEGAL {
        let stream = gen(&c, 9, 5, ct, bd, PNG_INTERLACE_NONE, true);
        for fmt in all_formats() {
            for bg in [false, true] {
                for neg_stride in [false, true] {
                    let mut run = |l: &Lib| -> Report {
                        diff_scratch(&mut || unsafe {
                            let mut im = PngImage::default();
                            let ok = (l.api.png_image_begin_read_from_memory)(
                                &mut im,
                                stream.as_ptr() as *const c_void,
                                stream.len(),
                            );
                            if ok == 0 {
                                log(format!("begin failed ok={ok}"));
                                log_image("begin", &im);
                                (l.api.png_image_free)(&mut im);
                                return;
                            }
                            im.format = fmt;
                            let stride = image_row_stride(fmt, im.width) as i32;
                            let sz = image_size(fmt, im.width, im.height).max(1);
                            let mut buf = vec![0u8; sz + 128];
                            let cm = colormap_size(fmt, 256).max(1);
                            let mut cmap = vec![0u8; cm + 128];
                            let background = PngColor {
                                red: 0x11,
                                green: 0x22,
                                blue: 0x33,
                            };
                            // Verified against the reference C .so: for a negative
                            // row_stride libpng still writes forward from the
                            // supplied pointer (it is the lowest address of the
                            // block); the sign only reverses the row order.
                            let base = buf.as_mut_ptr();
                            let ok2 = (l.api.png_image_finish_read)(
                                &mut im,
                                if bg { &background } else { ptr::null() },
                                base as *mut c_void,
                                if neg_stride { -stride } else { stride },
                                cmap.as_mut_ptr() as *mut c_void,
                            );
                            log(format!("finish ok={ok2}"));
                            log_image("finish", &im);
                            if ok2 != 0 {
                                log(format!("pixels={:02x?}", &buf[..sz]));
                                if fmt & PNG_FORMAT_FLAG_COLORMAP != 0 {
                                    let n = colormap_size(fmt, im.colormap_entries);
                                    log(format!("cmap={:02x?}", &cmap[..n]));
                                }
                            }
                            (l.api.png_image_free)(&mut im);
                        })
                    };
                    diff(
                        &format!("S9-S12 read fmt={fmt:#x} bg={bg} neg={neg_stride} ct={ct} bd={bd}"),
                        &c,
                        &r,
                        &mut run,
                    );
                }
            }
        }
    }
}

#[test]
fn s11_16bit_srgb_flag() {
    let (c, r) = libs();
    for &(ct, bd) in &[(0i32, 16i32), (2, 16), (4, 16), (6, 16)] {
        // No gAMA / sRGB in the stream so the flag is meaningful.
        let stream = gen(&c, 9, 4, ct, bd, PNG_INTERLACE_NONE, false);
        for flag in [0u32, 4] {
            for fmt in [0u32, 1, 2, 3, 4, 5, 6, 7] {
                let mut run = |l: &Lib| -> Report {
                    diff_scratch(&mut || unsafe {
                        let mut im = PngImage::default();
                        let ok = (l.api.png_image_begin_read_from_memory)(
                            &mut im,
                            stream.as_ptr() as *const c_void,
                            stream.len(),
                        );
                        log(format!("begin ok={ok}"));
                        if ok != 0 {
                            im.flags |= flag;
                            im.format = fmt;
                            let sz = image_size(fmt, im.width, im.height).max(1);
                            let mut buf = vec![0u8; sz + 64];
                            let ok2 = (l.api.png_image_finish_read)(
                                &mut im,
                                ptr::null(),
                                buf.as_mut_ptr() as *mut c_void,
                                0,
                                ptr::null_mut(),
                            );
                            log(format!("finish ok={ok2}"));
                            log_image("finish", &im);
                            if ok2 != 0 {
                                log(format!("pixels={:02x?}", &buf[..sz]));
                            }
                        }
                        (l.api.png_image_free)(&mut im);
                    })
                };
                diff(
                    &format!("S11 16bit_sRGB flag={flag} fmt={fmt:#x} ct={ct} bd={bd}"),
                    &c,
                    &r,
                    &mut run,
                );
            }
        }
    }
}
