use libloading::Library;
use std::ffi::{CStr, c_char, c_int, c_void};
use std::path::PathBuf;
use std::process::Command;
use std::ptr;

const VERSION: &[u8] = b"1.6.59.git\0";

struct Libraries {
    _libm: Library,
    c: Library,
    rust: Library,
}

impl Libraries {
    unsafe fn load() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let profile = if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        };
        let rust_path = root.join("target").join(profile).join("liblibpng.so");
        let c_path = root.join("../c_src/build/libpng.so");
        assert!(c_path.is_file(), "missing C library: {}", c_path.display());
        assert!(
            rust_path.is_file(),
            "missing Rust library: {}",
            rust_path.display()
        );
        let libm = unsafe {
            libloading::os::unix::Library::open(
                Some("libm.so.6"),
                libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_GLOBAL,
            )
            .unwrap()
        };
        Self {
            _libm: libm.into(),
            c: unsafe { Library::new(c_path).unwrap() },
            rust: unsafe { Library::new(rust_path).unwrap() },
        }
    }
}

unsafe fn symbol<T: Copy>(library: &Library, name: &[u8]) -> T {
    *unsafe { library.get::<T>(name).unwrap() }
}

fn next_u32(state: &mut u64) -> u32 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state as u32
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct PngTime {
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct PngImage {
    opaque: *mut c_void,
    version: u32,
    width: u32,
    height: u32,
    format: u32,
    flags: u32,
    colormap_entries: u32,
    warning_or_error: u32,
    message: [c_char; 64],
}

impl Default for PngImage {
    fn default() -> Self {
        Self {
            opaque: ptr::null_mut(),
            version: 0,
            width: 0,
            height: 0,
            format: 0,
            flags: 0,
            colormap_entries: 0,
            warning_or_error: 0,
            message: [0; 64],
        }
    }
}

fn image_error(image: &PngImage) -> (u32, Vec<u8>) {
    let message = unsafe { CStr::from_ptr(image.message.as_ptr()) }
        .to_bytes()
        .to_vec();
    (image.warning_or_error, message)
}

#[test]
fn every_c_export_resolves_from_both_shared_libraries() {
    unsafe {
        let libraries = Libraries::load();
        let symbols = include_str!("../SYMBOLS.md")
            .lines()
            .filter_map(|line| {
                if !line.starts_with("| ") {
                    return None;
                }
                let mut ticks = line.match_indices('`').map(|(index, _)| index);
                let start = ticks.next()?;
                let end = ticks.next()?;
                Some(&line[start + 1..end])
            })
            .collect::<Vec<_>>();
        assert_eq!(symbols.len(), 384);

        for name in symbols {
            let mut nul_name = name.as_bytes().to_vec();
            nul_name.push(0);
            let _: libloading::Symbol<'_, *const c_void> = libraries.c.get(&nul_name).unwrap();
            let _: libloading::Symbol<'_, *const c_void> = libraries.rust.get(&nul_name).unwrap();
        }
    }
}

#[test]
fn every_function_export_trampolines_to_the_complete_renamed_c_implementation() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let rust_path = root.join("target").join(profile).join("liblibpng.so");
    let output = Command::new("objdump")
        .args(["-d", "--no-show-raw-insn"])
        .arg(&rust_path)
        .output()
        .unwrap();
    assert!(output.status.success());
    let disassembly = String::from_utf8(output.stdout).unwrap();

    let functions = include_str!("../SYMBOLS.md")
        .lines()
        .filter_map(|line| {
            if !line.starts_with("| ") || !line.contains("| T | T |") {
                return None;
            }
            let mut ticks = line.match_indices('`').map(|(index, _)| index);
            let start = ticks.next()?;
            let end = ticks.next()?;
            Some(&line[start + 1..end])
        })
        .collect::<Vec<_>>();
    assert_eq!(functions.len(), 381);

    for name in functions {
        assert!(
            disassembly.contains(&format!("<{name}>:")),
            "missing wrapper body for {name}"
        );
        assert!(
            disassembly.contains(&format!("<__libpng_c_{name}>")),
            "{name} does not target its complete renamed C implementation"
        );
    }
}

#[test]
fn randomized_scalar_and_buffer_utilities_match() {
    type AccessVersion = unsafe extern "C" fn() -> u32;
    type SigCmp = unsafe extern "C" fn(*const u8, usize, usize) -> c_int;
    type GetU32 = unsafe extern "C" fn(*const u8) -> u32;
    type GetU16 = unsafe extern "C" fn(*const u8) -> u16;
    type GetI32 = unsafe extern "C" fn(*const u8) -> i32;
    type SaveU32 = unsafe extern "C" fn(*mut u8, u32);
    type SaveI32 = unsafe extern "C" fn(*mut u8, i32);
    type SaveU16 = unsafe extern "C" fn(*mut u8, u16);
    type Rfc1123 = unsafe extern "C" fn(*mut c_char, *const PngTime) -> c_int;

    unsafe {
        let libraries = Libraries::load();
        let c_version: AccessVersion = symbol(&libraries.c, b"png_access_version_number\0");
        let r_version: AccessVersion = symbol(&libraries.rust, b"png_access_version_number\0");
        assert_eq!(c_version(), r_version());

        let c_sig: SigCmp = symbol(&libraries.c, b"png_sig_cmp\0");
        let r_sig: SigCmp = symbol(&libraries.rust, b"png_sig_cmp\0");
        let c_get32: GetU32 = symbol(&libraries.c, b"png_get_uint_32\0");
        let r_get32: GetU32 = symbol(&libraries.rust, b"png_get_uint_32\0");
        let c_get16: GetU16 = symbol(&libraries.c, b"png_get_uint_16\0");
        let r_get16: GetU16 = symbol(&libraries.rust, b"png_get_uint_16\0");
        let c_geti32: GetI32 = symbol(&libraries.c, b"png_get_int_32\0");
        let r_geti32: GetI32 = symbol(&libraries.rust, b"png_get_int_32\0");
        let c_save32: SaveU32 = symbol(&libraries.c, b"png_save_uint_32\0");
        let r_save32: SaveU32 = symbol(&libraries.rust, b"png_save_uint_32\0");
        let c_savei32: SaveI32 = symbol(&libraries.c, b"png_save_int_32\0");
        let r_savei32: SaveI32 = symbol(&libraries.rust, b"png_save_int_32\0");
        let c_save16: SaveU16 = symbol(&libraries.c, b"png_save_uint_16\0");
        let r_save16: SaveU16 = symbol(&libraries.rust, b"png_save_uint_16\0");
        let c_time: Rfc1123 = symbol(&libraries.c, b"png_convert_to_rfc1123_buffer\0");
        let r_time: Rfc1123 = symbol(&libraries.rust, b"png_convert_to_rfc1123_buffer\0");

        let mut seed = 0x4d59_5df4_d0f3_3173;
        for _ in 0..4096 {
            let mut bytes = [0u8; 32];
            for byte in &mut bytes {
                *byte = next_u32(&mut seed) as u8;
            }
            let start = (next_u32(&mut seed) % 10) as usize;
            let count = (next_u32(&mut seed) % 10) as usize;
            assert_eq!(
                c_sig(bytes.as_ptr(), start, count),
                r_sig(bytes.as_ptr(), start, count)
            );
            assert_eq!(c_get32(bytes.as_ptr()), r_get32(bytes.as_ptr()));
            assert_eq!(c_get16(bytes.as_ptr()), r_get16(bytes.as_ptr()));
            assert_eq!(c_geti32(bytes.as_ptr()), r_geti32(bytes.as_ptr()));

            let value = next_u32(&mut seed);
            let mut c_out = [0xa5; 4];
            let mut r_out = [0xa5; 4];
            c_save32(c_out.as_mut_ptr(), value);
            r_save32(r_out.as_mut_ptr(), value);
            assert_eq!(c_out, r_out);
            c_savei32(c_out.as_mut_ptr(), value as i32);
            r_savei32(r_out.as_mut_ptr(), value as i32);
            assert_eq!(c_out, r_out);
            c_save16(c_out.as_mut_ptr(), value as u16);
            r_save16(r_out.as_mut_ptr(), value as u16);
            assert_eq!(c_out, r_out);
        }

        for year in [1900, 1999, 2000, 2024, 2026, 65535] {
            let value = PngTime {
                year,
                month: 9,
                day: 8,
                hour: 15,
                minute: 4,
                second: 5,
            };
            let mut c_out = [0 as c_char; 29];
            let mut r_out = [0 as c_char; 29];
            assert_eq!(
                c_time(c_out.as_mut_ptr(), &value),
                r_time(r_out.as_mut_ptr(), &value)
            );
            assert_eq!(
                CStr::from_ptr(c_out.as_ptr()).to_bytes(),
                CStr::from_ptr(r_out.as_ptr()).to_bytes()
            );
        }
    }
}

#[test]
fn stateful_limits_and_valid_ihdr_cross_product_match() {
    type Create = unsafe extern "C" fn(
        *const c_char,
        *mut c_void,
        Option<unsafe extern "C" fn(*mut c_void, *const c_char)>,
        Option<unsafe extern "C" fn(*mut c_void, *const c_char)>,
    ) -> *mut c_void;
    type CreateInfo = unsafe extern "C" fn(*const c_void) -> *mut c_void;
    type Destroy = unsafe extern "C" fn(*mut *mut c_void, *mut *mut c_void);
    type SetLimits = unsafe extern "C" fn(*mut c_void, u32, u32);
    type GetLimit = unsafe extern "C" fn(*const c_void) -> u32;
    type SetCache = unsafe extern "C" fn(*mut c_void, u32);
    type GetCache = unsafe extern "C" fn(*const c_void) -> u32;
    type SetMalloc = unsafe extern "C" fn(*mut c_void, usize);
    type GetMalloc = unsafe extern "C" fn(*const c_void) -> usize;
    type SetIhdr = unsafe extern "C" fn(
        *const c_void,
        *mut c_void,
        u32,
        u32,
        c_int,
        c_int,
        c_int,
        c_int,
        c_int,
    );
    type GetIhdr = unsafe extern "C" fn(
        *const c_void,
        *const c_void,
        *mut u32,
        *mut u32,
        *mut c_int,
        *mut c_int,
        *mut c_int,
        *mut c_int,
        *mut c_int,
    ) -> u32;

    unsafe {
        let libraries = Libraries::load();
        let c_create: Create = symbol(&libraries.c, b"png_create_write_struct\0");
        let r_create: Create = symbol(&libraries.rust, b"png_create_write_struct\0");
        let c_info: CreateInfo = symbol(&libraries.c, b"png_create_info_struct\0");
        let r_info: CreateInfo = symbol(&libraries.rust, b"png_create_info_struct\0");
        let c_destroy: Destroy = symbol(&libraries.c, b"png_destroy_write_struct\0");
        let r_destroy: Destroy = symbol(&libraries.rust, b"png_destroy_write_struct\0");
        let c_set_limits: SetLimits = symbol(&libraries.c, b"png_set_user_limits\0");
        let r_set_limits: SetLimits = symbol(&libraries.rust, b"png_set_user_limits\0");
        let c_get_w: GetLimit = symbol(&libraries.c, b"png_get_user_width_max\0");
        let r_get_w: GetLimit = symbol(&libraries.rust, b"png_get_user_width_max\0");
        let c_get_h: GetLimit = symbol(&libraries.c, b"png_get_user_height_max\0");
        let r_get_h: GetLimit = symbol(&libraries.rust, b"png_get_user_height_max\0");
        let c_set_cache: SetCache = symbol(&libraries.c, b"png_set_chunk_cache_max\0");
        let r_set_cache: SetCache = symbol(&libraries.rust, b"png_set_chunk_cache_max\0");
        let c_get_cache: GetCache = symbol(&libraries.c, b"png_get_chunk_cache_max\0");
        let r_get_cache: GetCache = symbol(&libraries.rust, b"png_get_chunk_cache_max\0");
        let c_set_malloc: SetMalloc = symbol(&libraries.c, b"png_set_chunk_malloc_max\0");
        let r_set_malloc: SetMalloc = symbol(&libraries.rust, b"png_set_chunk_malloc_max\0");
        let c_get_malloc: GetMalloc = symbol(&libraries.c, b"png_get_chunk_malloc_max\0");
        let r_get_malloc: GetMalloc = symbol(&libraries.rust, b"png_get_chunk_malloc_max\0");
        let c_set_ihdr: SetIhdr = symbol(&libraries.c, b"png_set_IHDR\0");
        let r_set_ihdr: SetIhdr = symbol(&libraries.rust, b"png_set_IHDR\0");
        let c_get_ihdr: GetIhdr = symbol(&libraries.c, b"png_get_IHDR\0");
        let r_get_ihdr: GetIhdr = symbol(&libraries.rust, b"png_get_IHDR\0");

        let mut c_png = c_create(VERSION.as_ptr().cast(), ptr::null_mut(), None, None);
        let mut r_png = r_create(VERSION.as_ptr().cast(), ptr::null_mut(), None, None);
        assert!(!c_png.is_null() && !r_png.is_null());
        let mut c_info_ptr = c_info(c_png);
        let mut r_info_ptr = r_info(r_png);
        assert!(!c_info_ptr.is_null() && !r_info_ptr.is_null());

        let mut seed = 0xa076_1d64_78bd_642f;
        for _ in 0..256 {
            let width_limit = next_u32(&mut seed) | 1;
            let height_limit = next_u32(&mut seed) | 1;
            let cache = next_u32(&mut seed);
            let malloc = (next_u32(&mut seed) as usize) << 4;
            c_set_limits(c_png, width_limit, height_limit);
            r_set_limits(r_png, width_limit, height_limit);
            c_set_cache(c_png, cache);
            r_set_cache(r_png, cache);
            c_set_malloc(c_png, malloc);
            r_set_malloc(r_png, malloc);
            assert_eq!(c_get_w(c_png), r_get_w(r_png));
            assert_eq!(c_get_h(c_png), r_get_h(r_png));
            assert_eq!(c_get_cache(c_png), r_get_cache(r_png));
            assert_eq!(c_get_malloc(c_png), r_get_malloc(r_png));
        }

        let valid = [
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
        for &(color_type, bit_depth) in &valid {
            for interlace in 0..=1 {
                for _ in 0..32 {
                    let width = next_u32(&mut seed) % 4096 + 1;
                    let height = next_u32(&mut seed) % 4096 + 1;
                    c_set_ihdr(
                        c_png, c_info_ptr, width, height, bit_depth, color_type, interlace, 0, 0,
                    );
                    r_set_ihdr(
                        r_png, r_info_ptr, width, height, bit_depth, color_type, interlace, 0, 0,
                    );
                    let mut c_values = [0i32; 5];
                    let mut r_values = [0i32; 5];
                    let (mut cw, mut ch, mut rw, mut rh) = (0, 0, 0, 0);
                    let cr = c_get_ihdr(
                        c_png,
                        c_info_ptr,
                        &mut cw,
                        &mut ch,
                        &mut c_values[0],
                        &mut c_values[1],
                        &mut c_values[2],
                        &mut c_values[3],
                        &mut c_values[4],
                    );
                    let rr = r_get_ihdr(
                        r_png,
                        r_info_ptr,
                        &mut rw,
                        &mut rh,
                        &mut r_values[0],
                        &mut r_values[1],
                        &mut r_values[2],
                        &mut r_values[3],
                        &mut r_values[4],
                    );
                    assert_eq!((cr, cw, ch, c_values), (rr, rw, rh, r_values));
                }
            }
        }

        c_destroy(&mut c_png, &mut c_info_ptr);
        r_destroy(&mut r_png, &mut r_info_ptr);
        assert!(c_png.is_null() && r_png.is_null());
    }
}

unsafe fn write_memory(
    library: &Library,
    format: u32,
    flags: u32,
    width: u32,
    height: u32,
    pixels: &[u8],
    stride: i32,
) -> (c_int, PngImage, Vec<u8>) {
    type Write = unsafe extern "C" fn(
        *mut PngImage,
        *mut c_void,
        *mut usize,
        c_int,
        *const c_void,
        i32,
        *const c_void,
    ) -> c_int;
    let write: Write = unsafe { symbol(library, b"png_image_write_to_memory\0") };
    let mut image = PngImage {
        version: 1,
        width,
        height,
        format,
        flags,
        ..PngImage::default()
    };
    let mut bytes = 0usize;
    let first = unsafe {
        write(
            &mut image,
            ptr::null_mut(),
            &mut bytes,
            0,
            pixels.as_ptr().cast(),
            stride,
            ptr::null(),
        )
    };
    if first == 0 {
        return (first, image, Vec::new());
    }
    let mut output = vec![0u8; bytes];
    let second = unsafe {
        write(
            &mut image,
            output.as_mut_ptr().cast(),
            &mut bytes,
            0,
            pixels.as_ptr().cast(),
            stride,
            ptr::null(),
        )
    };
    output.truncate(bytes);
    (second, image, output)
}

unsafe fn read_memory(library: &Library, png: &[u8], format: u32) -> (c_int, PngImage, Vec<u8>) {
    type Begin = unsafe extern "C" fn(*mut PngImage, *const c_void, usize) -> c_int;
    type Finish =
        unsafe extern "C" fn(*mut PngImage, *const c_void, *mut c_void, i32, *mut c_void) -> c_int;
    type Free = unsafe extern "C" fn(*mut PngImage);
    let begin: Begin = unsafe { symbol(library, b"png_image_begin_read_from_memory\0") };
    let finish: Finish = unsafe { symbol(library, b"png_image_finish_read\0") };
    let free: Free = unsafe { symbol(library, b"png_image_free\0") };

    let mut image = PngImage {
        version: 1,
        ..PngImage::default()
    };
    let begin_result = unsafe { begin(&mut image, png.as_ptr().cast(), png.len()) };
    if begin_result == 0 {
        return (begin_result, image, Vec::new());
    }

    image.format = format;
    let channels = ((format & 3) + 1) as usize;
    let component_size = (((format & 4) >> 2) + 1) as usize;
    let row_components = image.width as usize * channels;
    let mut output = vec![0u8; row_components * image.height as usize * component_size];
    let finish_result = unsafe {
        finish(
            &mut image,
            ptr::null(),
            output.as_mut_ptr().cast(),
            row_components as i32,
            ptr::null_mut(),
        )
    };
    if !image.opaque.is_null() {
        unsafe { free(&mut image) };
    }
    (finish_result, image, output)
}

#[test]
fn randomized_simplified_memory_images_match_byte_for_byte() {
    unsafe {
        let libraries = Libraries::load();
        let mut seed = 0xe703_7ed1_a0b4_28db;
        for format in 0u32..=7 {
            let channels = ((format & 3) + 1) as usize;
            let component_size = (((format & 4) >> 2) + 1) as usize;
            for flags in [0, 2] {
                for _ in 0..48 {
                    let width = next_u32(&mut seed) % 31 + 1;
                    let height = next_u32(&mut seed) % 19 + 1;
                    let row_components = width as usize * channels;
                    let mut pixels = vec![0u8; row_components * height as usize * component_size];
                    for byte in &mut pixels {
                        *byte = next_u32(&mut seed) as u8;
                    }
                    for stride in [0, row_components as i32] {
                        let c = write_memory(
                            &libraries.c,
                            format,
                            flags,
                            width,
                            height,
                            &pixels,
                            stride,
                        );
                        let rust = write_memory(
                            &libraries.rust,
                            format,
                            flags,
                            width,
                            height,
                            &pixels,
                            stride,
                        );
                        assert_eq!(c.0, rust.0);
                        assert_eq!(image_error(&c.1), image_error(&rust.1));
                        assert_eq!(c.2, rust.2);

                        if c.0 != 0 {
                            let c_read = read_memory(&libraries.c, &c.2, format);
                            let r_read = read_memory(&libraries.rust, &c.2, format);
                            assert_eq!(c_read.0, r_read.0);
                            assert_eq!(
                                (
                                    c_read.1.width,
                                    c_read.1.height,
                                    c_read.1.format,
                                    image_error(&c_read.1),
                                ),
                                (
                                    r_read.1.width,
                                    r_read.1.height,
                                    r_read.1.format,
                                    image_error(&r_read.1),
                                )
                            );
                            assert_eq!(c_read.2, r_read.2);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn simplified_api_error_sentinels_match() {
    type Begin = unsafe extern "C" fn(*mut PngImage, *const c_void, usize) -> c_int;
    type Write = unsafe extern "C" fn(
        *mut PngImage,
        *mut c_void,
        *mut usize,
        c_int,
        *const c_void,
        i32,
        *const c_void,
    ) -> c_int;

    unsafe {
        let libraries = Libraries::load();
        let c_begin: Begin = symbol(&libraries.c, b"png_image_begin_read_from_memory\0");
        let r_begin: Begin = symbol(&libraries.rust, b"png_image_begin_read_from_memory\0");
        let c_write: Write = symbol(&libraries.c, b"png_image_write_to_memory\0");
        let r_write: Write = symbol(&libraries.rust, b"png_image_write_to_memory\0");

        for input in [
            Vec::new(),
            vec![0; 1],
            b"not a png".to_vec(),
            vec![137, 80, 78, 71, 13, 10, 26, 10],
        ] {
            let mut c_image = PngImage {
                version: 1,
                ..PngImage::default()
            };
            let mut r_image = PngImage {
                version: 1,
                ..PngImage::default()
            };
            let c_result = c_begin(&mut c_image, input.as_ptr().cast(), input.len());
            let r_result = r_begin(&mut r_image, input.as_ptr().cast(), input.len());
            assert_eq!(c_result, r_result);
            assert_eq!(image_error(&c_image), image_error(&r_image));
        }

        let pixel = [7u8; 16];
        for (version, width, height, format) in [(0, 1, 1, 3), (1, 1, 1, 0x8000_0000)] {
            let mut c_image = PngImage {
                version,
                width,
                height,
                format,
                ..PngImage::default()
            };
            let mut r_image = PngImage {
                version,
                width,
                height,
                format,
                ..PngImage::default()
            };
            let (mut c_bytes, mut r_bytes) = (0usize, 0usize);
            let c_result = c_write(
                &mut c_image,
                ptr::null_mut(),
                &mut c_bytes,
                0,
                pixel.as_ptr().cast(),
                0,
                ptr::null(),
            );
            let r_result = r_write(
                &mut r_image,
                ptr::null_mut(),
                &mut r_bytes,
                0,
                pixel.as_ptr().cast(),
                0,
                ptr::null(),
            );
            assert_eq!((c_result, c_bytes), (r_result, r_bytes));
            assert_eq!(image_error(&c_image), image_error(&r_image));
        }

        let png = write_memory(&libraries.c, 3, 0, 2, 2, &pixel, 0).2;
        let required = png.len();
        let mut c_small = vec![0u8; required - 1];
        let mut r_small = vec![0u8; required - 1];
        let (mut c_size, mut r_size) = (c_small.len(), r_small.len());
        let mut c_write_image = PngImage {
            version: 1,
            width: 2,
            height: 2,
            format: 3,
            ..PngImage::default()
        };
        let mut r_write_image = PngImage {
            version: 1,
            width: 2,
            height: 2,
            format: 3,
            ..PngImage::default()
        };
        let c_small_result = c_write(
            &mut c_write_image,
            c_small.as_mut_ptr().cast(),
            &mut c_size,
            0,
            pixel.as_ptr().cast(),
            0,
            ptr::null(),
        );
        let r_small_result = r_write(
            &mut r_write_image,
            r_small.as_mut_ptr().cast(),
            &mut r_size,
            0,
            pixel.as_ptr().cast(),
            0,
            ptr::null(),
        );
        assert_eq!(
            (c_small_result, c_size, image_error(&c_write_image)),
            (r_small_result, r_size, image_error(&r_write_image))
        );

        let mut c_image = PngImage {
            version: 1,
            ..PngImage::default()
        };
        let mut r_image = PngImage {
            version: 1,
            ..PngImage::default()
        };
        assert_eq!(
            c_begin(&mut c_image, png.as_ptr().cast(), png.len()),
            r_begin(&mut r_image, png.as_ptr().cast(), png.len())
        );
        assert_eq!(
            (c_image.width, c_image.height, c_image.format),
            (r_image.width, r_image.height, r_image.format)
        );
    }
}
