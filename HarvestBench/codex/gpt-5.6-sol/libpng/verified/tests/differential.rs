use libloading::os::unix::{Library as UnixLibrary, RTLD_GLOBAL, RTLD_NOW};
use libloading::Library;
use std::collections::HashMap;
use std::ffi::{c_char, c_void, CStr};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::ptr;
use std::sync::{Mutex, OnceLock};

const PNG_IMAGE_VERSION: u32 = 1;
const PNG_FORMAT_FLAG_ALPHA: u32 = 0x01;
const PNG_FORMAT_FLAG_COLOR: u32 = 0x02;
const PNG_FORMAT_FLAG_LINEAR: u32 = 0x04;
const PNG_FORMAT_FLAG_COLORMAP: u32 = 0x08;
const PNG_FORMAT_FLAG_BGR: u32 = 0x10;
const PNG_FORMAT_FLAG_AFIRST: u32 = 0x20;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RowInfo {
    width: u32,
    rowbytes: usize,
    color_type: u8,
    bit_depth: u8,
    channels: u8,
    pixel_depth: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Color {
    red: u8,
    green: u8,
    blue: u8,
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

impl PngImage {
    fn new(width: u32, height: u32, format: u32) -> Self {
        Self {
            opaque: ptr::null_mut(),
            version: PNG_IMAGE_VERSION,
            width,
            height,
            format,
            flags: 0,
            colormap_entries: 0,
            warning_or_error: 0,
            message: [0; 64],
        }
    }

    fn message_bytes(&self) -> &[u8] {
        let bytes = unsafe {
            std::slice::from_raw_parts(self.message.as_ptr().cast::<u8>(), self.message.len())
        };
        let end = bytes
            .iter()
            .position(|&byte| byte == 0)
            .unwrap_or(bytes.len());
        &bytes[..end]
    }
}

struct Libraries {
    _libm: Library,
    c: Library,
    rust: Library,
}

impl Libraries {
    unsafe fn open() -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest.join("c_src/build/libpng.so");
        let rust_path = rust_library_path(&manifest);
        assert!(
            c_path.is_file(),
            "missing C reference: {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "missing Rust cdylib: {}",
            rust_path.display()
        );
        let libm = unsafe {
            UnixLibrary::open(Some("libm.so.6"), RTLD_NOW | RTLD_GLOBAL)
                .unwrap()
                .into()
        };
        Self {
            _libm: libm,
            c: unsafe { Library::new(c_path).unwrap() },
            rust: unsafe { Library::new(rust_path).unwrap() },
        }
    }

    unsafe fn pair<T: Copy>(&self, name: &[u8]) -> (T, T) {
        let c = unsafe { self.c.get::<T>(name).unwrap() };
        let rust = unsafe { self.rust.get::<T>(name).unwrap() };
        (*c, *rust)
    }
}

fn rust_library_path(manifest: &Path) -> PathBuf {
    let debug = manifest.join("target/debug/liblibpng.so");
    if debug.is_file() {
        debug
    } else {
        manifest.join("target/release/liblibpng.so")
    }
}

#[derive(Clone, Copy)]
struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    fn fill(&mut self, bytes: &mut [u8]) {
        for byte in bytes {
            *byte = self.next_u32() as u8;
        }
    }
}

type SigCmp = unsafe extern "C" fn(*const u8, usize, usize) -> i32;
type GetU32 = unsafe extern "C" fn(*const u8) -> u32;
type GetI32 = unsafe extern "C" fn(*const u8) -> i32;
type GetU16 = unsafe extern "C" fn(*const u8) -> u16;
type SaveU32 = unsafe extern "C" fn(*mut u8, u32);
type SaveI32 = unsafe extern "C" fn(*mut u8, i32);
type SaveU16 = unsafe extern "C" fn(*mut u8, u32);
type MulDiv = unsafe extern "C" fn(*mut i32, i32, i32, i32) -> i32;
type UnaryFixed = unsafe extern "C" fn(i32) -> i32;
type BinaryFixed = unsafe extern "C" fn(i32, i32) -> i32;
type Gamma8 = unsafe extern "C" fn(u32, i32) -> u8;
type Gamma16 = unsafe extern "C" fn(u32, i32) -> u16;
type CheckFpString = unsafe extern "C" fn(*const c_char, usize) -> i32;
type CheckFpNumber = unsafe extern "C" fn(*const c_char, usize, *mut i32, *mut usize) -> i32;
type SafeCat = unsafe extern "C" fn(*mut c_char, usize, usize, *const c_char) -> usize;
type FormatNumber = unsafe extern "C" fn(*const c_char, *mut c_char, i32, usize) -> *mut c_char;
type BuildPalette = unsafe extern "C" fn(i32, *mut Color);
type RowTransform = unsafe extern "C" fn(*mut RowInfo, *mut u8);
type StripChannel = unsafe extern "C" fn(*mut RowInfo, *mut u8, i32);
type WriteMemory = unsafe extern "C" fn(
    *mut PngImage,
    *mut c_void,
    *mut usize,
    i32,
    *const c_void,
    i32,
    *const c_void,
) -> i32;
type BeginReadMemory = unsafe extern "C" fn(*mut PngImage, *const c_void, usize) -> i32;
type FinishRead =
    unsafe extern "C" fn(*mut PngImage, *const Color, *mut c_void, i32, *mut c_void) -> i32;
type ImageFree = unsafe extern "C" fn(*mut PngImage);
type ErrorCallback = Option<unsafe extern "C" fn(*mut c_void, *const c_char)>;
type IoCallback = Option<unsafe extern "C" fn(*mut c_void, *mut u8, usize)>;
type FlushCallback = Option<unsafe extern "C" fn(*mut c_void)>;
type CreateStruct =
    unsafe extern "C" fn(*const c_char, *mut c_void, ErrorCallback, ErrorCallback) -> *mut c_void;
type CreateInfo = unsafe extern "C" fn(*const c_void) -> *mut c_void;
type SetWriteFn = unsafe extern "C" fn(*mut c_void, *mut c_void, IoCallback, FlushCallback);
type SetReadFn = unsafe extern "C" fn(*mut c_void, *mut c_void, IoCallback);
type SetIHDR = unsafe extern "C" fn(*const c_void, *mut c_void, u32, u32, i32, i32, i32, i32, i32);
type GetIHDR = unsafe extern "C" fn(
    *const c_void,
    *const c_void,
    *mut u32,
    *mut u32,
    *mut i32,
    *mut i32,
    *mut i32,
    *mut i32,
    *mut i32,
) -> u32;
type SetInt = unsafe extern "C" fn(*mut c_void, i32);
type SetFilter = unsafe extern "C" fn(*mut c_void, i32, i32);
type WriteInfo = unsafe extern "C" fn(*mut c_void, *const c_void);
type WriteRow = unsafe extern "C" fn(*mut c_void, *const u8);
type WriteEnd = unsafe extern "C" fn(*mut c_void, *mut c_void);
type DestroyWrite = unsafe extern "C" fn(*mut *mut c_void, *mut *mut c_void);
type ReadInfo = unsafe extern "C" fn(*mut c_void, *mut c_void);
type ReadUpdateInfo = unsafe extern "C" fn(*mut c_void, *mut c_void);
type ReadImage = unsafe extern "C" fn(*mut c_void, *mut *mut u8);
type ReadEnd = unsafe extern "C" fn(*mut c_void, *mut c_void);
type DestroyRead = unsafe extern "C" fn(*mut *mut c_void, *mut *mut c_void, *mut *mut c_void);
type GetRowbytes = unsafe extern "C" fn(*const c_void, *const c_void) -> usize;

#[derive(Clone, Copy)]
struct FullApi {
    create_write: CreateStruct,
    create_read: CreateStruct,
    create_info: CreateInfo,
    set_write_fn: SetWriteFn,
    set_read_fn: SetReadFn,
    set_ihdr: SetIHDR,
    get_ihdr: GetIHDR,
    set_filter: SetFilter,
    set_compression_level: SetInt,
    set_interlace_handling: unsafe extern "C" fn(*mut c_void) -> i32,
    write_info: WriteInfo,
    write_row: WriteRow,
    write_end: WriteEnd,
    destroy_write: DestroyWrite,
    read_info: ReadInfo,
    read_update_info: ReadUpdateInfo,
    read_image: ReadImage,
    read_end: ReadEnd,
    destroy_read: DestroyRead,
    get_rowbytes: GetRowbytes,
}

static IO_STATES: OnceLock<Mutex<HashMap<usize, usize>>> = OnceLock::new();

fn io_states() -> &'static Mutex<HashMap<usize, usize>> {
    IO_STATES.get_or_init(|| Mutex::new(HashMap::new()))
}

unsafe extern "C" fn write_callback(png: *mut c_void, data: *mut u8, length: usize) {
    let state = *io_states().lock().unwrap().get(&(png as usize)).unwrap();
    let output = unsafe { &mut *(state as *mut Vec<u8>) };
    output.extend_from_slice(unsafe { std::slice::from_raw_parts(data, length) });
}

struct ReadState {
    bytes: *const u8,
    length: usize,
    offset: usize,
}

unsafe extern "C" fn read_callback(png: *mut c_void, data: *mut u8, length: usize) {
    let state = *io_states().lock().unwrap().get(&(png as usize)).unwrap();
    let input = unsafe { &mut *(state as *mut ReadState) };
    let available = input.length.saturating_sub(input.offset);
    let copy = available.min(length);
    unsafe {
        ptr::copy_nonoverlapping(input.bytes.add(input.offset), data, copy);
        if copy < length {
            ptr::write_bytes(data.add(copy), 0, length - copy);
        }
    }
    input.offset += copy;
}

#[test]
fn every_c_dynamic_symbol_loads_from_both_libraries() {
    unsafe {
        let libraries = Libraries::open();
        let symbols = include_str!("../SYMBOLS.md")
            .lines()
            .filter_map(|line| {
                let mut fields = line.split('|');
                fields.next()?;
                let number = fields.next()?.trim();
                if number.parse::<usize>().is_err() {
                    return None;
                }
                Some(fields.next()?.trim().trim_matches('`').to_owned())
            })
            .collect::<Vec<_>>();
        assert_eq!(symbols.len(), 384);
        for symbol in symbols {
            let name = format!("{symbol}\0");
            libraries
                .c
                .get::<*const c_void>(name.as_bytes())
                .unwrap_or_else(|error| panic!("C missing {symbol}: {error}"));
            libraries
                .rust
                .get::<*const c_void>(name.as_bytes())
                .unwrap_or_else(|error| panic!("Rust missing {symbol}: {error}"));
        }
    }
}

#[test]
fn integer_fixed_point_and_string_primitives_match() {
    unsafe {
        let libraries = Libraries::open();
        let (c_sig, r_sig) = libraries.pair::<SigCmp>(b"png_sig_cmp\0");
        let (c_get32, r_get32) = libraries.pair::<GetU32>(b"png_get_uint_32\0");
        let (c_geti32, r_geti32) = libraries.pair::<GetI32>(b"png_get_int_32\0");
        let (c_get16, r_get16) = libraries.pair::<GetU16>(b"png_get_uint_16\0");
        let (c_save32, r_save32) = libraries.pair::<SaveU32>(b"png_save_uint_32\0");
        let (c_savei32, r_savei32) = libraries.pair::<SaveI32>(b"png_save_int_32\0");
        let (c_save16, r_save16) = libraries.pair::<SaveU16>(b"png_save_uint_16\0");
        let (c_muldiv, r_muldiv) = libraries.pair::<MulDiv>(b"png_muldiv\0");
        let (c_recip, r_recip) = libraries.pair::<UnaryFixed>(b"png_reciprocal\0");
        let (c_recip2, r_recip2) = libraries.pair::<BinaryFixed>(b"png_reciprocal2\0");
        let (c_sig_gamma, r_sig_gamma) = libraries.pair::<UnaryFixed>(b"png_gamma_significant\0");
        let (c_gamma8, r_gamma8) = libraries.pair::<Gamma8>(b"png_gamma_8bit_correct\0");
        let (c_gamma16, r_gamma16) = libraries.pair::<Gamma16>(b"png_gamma_16bit_correct\0");
        let (c_fp, r_fp) = libraries.pair::<CheckFpString>(b"png_check_fp_string\0");
        let (c_fp_number, r_fp_number) = libraries.pair::<CheckFpNumber>(b"png_check_fp_number\0");
        let (c_cat, r_cat) = libraries.pair::<SafeCat>(b"png_safecat\0");
        let (c_format, r_format) = libraries.pair::<FormatNumber>(b"png_format_number\0");

        let signature = [137, 80, 78, 71, 13, 10, 26, 10];
        for start in 0..=9 {
            for count in 0..=10 {
                assert_eq!(
                    c_sig(signature.as_ptr(), start, count),
                    r_sig(signature.as_ptr(), start, count),
                    "signature start={start} count={count}"
                );
            }
        }

        let mut rng = Rng(0x5eed_5eed_d15c_a11e);
        for _ in 0..20_000 {
            let bytes = rng.next_u32().to_be_bytes();
            assert_eq!(c_get32(bytes.as_ptr()), r_get32(bytes.as_ptr()));
            assert_eq!(c_geti32(bytes.as_ptr()), r_geti32(bytes.as_ptr()));
            assert_eq!(c_get16(bytes.as_ptr()), r_get16(bytes.as_ptr()));

            let value = rng.next_u32();
            let mut c_bytes = [0xa5; 8];
            let mut r_bytes = [0xa5; 8];
            c_save32(c_bytes.as_mut_ptr().add(2), value);
            r_save32(r_bytes.as_mut_ptr().add(2), value);
            assert_eq!(c_bytes, r_bytes);
            c_savei32(c_bytes.as_mut_ptr().add(2), value as i32);
            r_savei32(r_bytes.as_mut_ptr().add(2), value as i32);
            assert_eq!(c_bytes, r_bytes);
            c_save16(c_bytes.as_mut_ptr().add(2), value);
            r_save16(r_bytes.as_mut_ptr().add(2), value);
            assert_eq!(c_bytes, r_bytes);

            let a = rng.next_u32() as i32;
            let times = rng.next_u32() as i32;
            let divisor = rng.next_u32() as i32;
            let mut c_result = 0x1234_5678;
            let mut r_result = 0x1234_5678;
            assert_eq!(
                c_muldiv(&mut c_result, a, times, divisor),
                r_muldiv(&mut r_result, a, times, divisor)
            );
            assert_eq!(c_result, r_result);
        }

        let fixed_values = [
            i32::MIN,
            -2_000_000_000,
            -100_000,
            -1,
            0,
            1,
            5_000,
            95_000,
            100_000,
            105_000,
            2_000_000_000,
            i32::MAX,
        ];
        for &a in &fixed_values {
            assert_eq!(c_recip(a), r_recip(a));
            assert_eq!(c_sig_gamma(a), r_sig_gamma(a));
            for &b in &fixed_values {
                assert_eq!(c_recip2(a, b), r_recip2(a, b));
            }
        }
        for gamma in [-500_000, -1, 0, 45_455, 100_000, 220_000, 1_000_000] {
            for value in 0..=255 {
                assert_eq!(c_gamma8(value, gamma), r_gamma8(value, gamma));
            }
            for value in (0..=65_535).step_by(37) {
                assert_eq!(c_gamma16(value, gamma), r_gamma16(value, gamma));
            }
        }

        let fp_inputs = [
            "", "0", "-0", "+1", ".5", "1.", "1.25", "1e3", "-2.5E-12", "e1", ".", "1e", "1x",
            " 1", "nan", "inf",
        ];
        for input in fp_inputs {
            let bytes = input.as_bytes();
            assert_eq!(
                c_fp(bytes.as_ptr().cast(), bytes.len()),
                r_fp(bytes.as_ptr().cast(), bytes.len()),
                "{input:?}"
            );
            for split in 0..=bytes.len() {
                let mut c_state = 0;
                let mut r_state = 0;
                let mut c_at = 0;
                let mut r_at = 0;
                let c_first = c_fp_number(bytes.as_ptr().cast(), split, &mut c_state, &mut c_at);
                let r_first = r_fp_number(bytes.as_ptr().cast(), split, &mut r_state, &mut r_at);
                assert_eq!((c_first, c_state, c_at), (r_first, r_state, r_at));
                let c_second =
                    c_fp_number(bytes.as_ptr().cast(), bytes.len(), &mut c_state, &mut c_at);
                let r_second =
                    r_fp_number(bytes.as_ptr().cast(), bytes.len(), &mut r_state, &mut r_at);
                assert_eq!((c_second, c_state, c_at), (r_second, r_state, r_at));
            }
        }

        for size in 0..=12 {
            for position in 0..=14 {
                let source = b"abcdef\0";
                let mut c_buffer = [b'X' as c_char; 16];
                let mut r_buffer = [b'X' as c_char; 16];
                let c_pos = c_cat(
                    c_buffer.as_mut_ptr(),
                    size,
                    position,
                    source.as_ptr().cast(),
                );
                let r_pos = r_cat(
                    r_buffer.as_mut_ptr(),
                    size,
                    position,
                    source.as_ptr().cast(),
                );
                assert_eq!((c_pos, c_buffer), (r_pos, r_buffer));
            }
        }

        for format in 1..=5 {
            for number in [0, 1, 9, 10, 15, 16, 99, 100, 65_535, usize::MAX] {
                let mut c_buffer = [b'?' as c_char; 24];
                let mut r_buffer = [b'?' as c_char; 24];
                let c_start = c_format(
                    c_buffer.as_ptr(),
                    c_buffer.as_mut_ptr().add(c_buffer.len()),
                    format,
                    number,
                );
                let r_start = r_format(
                    r_buffer.as_ptr(),
                    r_buffer.as_mut_ptr().add(r_buffer.len()),
                    format,
                    number,
                );
                let c_offset = c_start.offset_from(c_buffer.as_ptr());
                let r_offset = r_start.offset_from(r_buffer.as_ptr());
                assert_eq!((c_offset, c_buffer), (r_offset, r_buffer));
            }
        }
    }
}

#[test]
fn exported_tables_palettes_and_row_transforms_match() {
    unsafe {
        let libraries = Libraries::open();
        for name in [
            b"png_sRGB_table\0".as_slice(),
            b"png_sRGB_base\0".as_slice(),
        ] {
            let (c, rust) = libraries.pair::<*const u16>(name);
            let length = if name == b"png_sRGB_table\0" {
                256
            } else {
                512
            };
            assert_eq!(
                std::slice::from_raw_parts(c, length),
                std::slice::from_raw_parts(rust, length)
            );
        }
        let (c_delta, r_delta) = libraries.pair::<*const u8>(b"png_sRGB_delta\0");
        assert_eq!(
            std::slice::from_raw_parts(c_delta, 512),
            std::slice::from_raw_parts(r_delta, 512)
        );

        let (c_palette, r_palette) =
            libraries.pair::<BuildPalette>(b"png_build_grayscale_palette\0");
        for depth in [1, 2, 4, 8] {
            let mut c = [Color {
                red: 0xa5,
                green: 0xa5,
                blue: 0xa5,
            }; 256];
            let mut rust = c;
            c_palette(depth, c.as_mut_ptr());
            r_palette(depth, rust.as_mut_ptr());
            assert_eq!(c, rust, "palette depth={depth}");
        }

        let (c_invert, r_invert) = libraries.pair::<RowTransform>(b"png_do_invert\0");
        let (c_swap, r_swap) = libraries.pair::<RowTransform>(b"png_do_swap\0");
        let (c_packswap, r_packswap) = libraries.pair::<RowTransform>(b"png_do_packswap\0");
        let (c_strip, r_strip) = libraries.pair::<StripChannel>(b"png_do_strip_channel\0");
        let (c_bgr, r_bgr) = libraries.pair::<RowTransform>(b"png_do_bgr\0");
        let mut rng = Rng(0xd1ff_3e77_1a1b_1e55);

        for &(color_type, channels, depth) in &[
            (0, 1, 1),
            (0, 1, 2),
            (0, 1, 4),
            (0, 1, 8),
            (0, 1, 16),
            (4, 2, 8),
            (4, 2, 16),
            (2, 3, 8),
        ] {
            compare_row_transform(&mut rng, c_invert, r_invert, color_type, channels, depth);
        }
        for &(color_type, channels, depth) in &[(0, 1, 8), (0, 1, 16), (2, 3, 16), (6, 4, 16)] {
            compare_row_transform(&mut rng, c_swap, r_swap, color_type, channels, depth);
        }
        for depth in [1, 2, 4, 8] {
            compare_row_transform(&mut rng, c_packswap, r_packswap, 0, 1, depth);
        }
        for &(color_type, channels, depth) in &[(4, 2, 8), (4, 2, 16), (6, 4, 8), (6, 4, 16)] {
            for at_start in [0, 1] {
                compare_strip(
                    &mut rng, c_strip, r_strip, color_type, channels, depth, at_start,
                );
            }
        }
        for &(color_type, channels, depth) in
            &[(0, 1, 8), (2, 3, 8), (6, 4, 8), (2, 3, 16), (6, 4, 16)]
        {
            compare_row_transform(&mut rng, c_bgr, r_bgr, color_type, channels, depth);
        }
    }
}

unsafe fn compare_row_transform(
    rng: &mut Rng,
    c: RowTransform,
    rust: RowTransform,
    color_type: u8,
    channels: u8,
    depth: u8,
) {
    for width in [0, 1, 2, 3, 7, 8, 9, 31, 64] {
        let rowbytes = (width * channels as u32 * depth as u32).div_ceil(8) as usize;
        for _ in 0..40 {
            let info = RowInfo {
                width,
                rowbytes,
                color_type,
                bit_depth: depth,
                channels,
                pixel_depth: channels * depth,
            };
            let mut c_info = info;
            let mut r_info = info;
            let mut c_row = vec![0u8; rowbytes + 16];
            rng.fill(&mut c_row);
            let mut r_row = c_row.clone();
            unsafe { c(&mut c_info, c_row.as_mut_ptr()) };
            unsafe { rust(&mut r_info, r_row.as_mut_ptr()) };
            assert_eq!(c_info, r_info);
            assert_eq!(c_row, r_row);
        }
    }
}

unsafe fn compare_strip(
    rng: &mut Rng,
    c: StripChannel,
    rust: StripChannel,
    color_type: u8,
    channels: u8,
    depth: u8,
    at_start: i32,
) {
    for width in [1, 2, 3, 7, 31] {
        let rowbytes = (width * channels as u32 * depth as u32 / 8) as usize;
        for _ in 0..40 {
            let info = RowInfo {
                width,
                rowbytes,
                color_type,
                bit_depth: depth,
                channels,
                pixel_depth: channels * depth,
            };
            let mut c_info = info;
            let mut r_info = info;
            let mut c_row = vec![0u8; rowbytes + 16];
            rng.fill(&mut c_row);
            let mut r_row = c_row.clone();
            unsafe { c(&mut c_info, c_row.as_mut_ptr(), at_start) };
            unsafe { rust(&mut r_info, r_row.as_mut_ptr(), at_start) };
            assert_eq!(c_info, r_info);
            assert_eq!(c_row, r_row);
        }
    }
}

#[test]
fn simplified_memory_write_and_read_match_byte_for_byte() {
    unsafe {
        let libraries = Libraries::open();
        let (c_write, r_write) = libraries.pair::<WriteMemory>(b"png_image_write_to_memory\0");
        let (c_begin, r_begin) =
            libraries.pair::<BeginReadMemory>(b"png_image_begin_read_from_memory\0");
        let (c_finish, r_finish) = libraries.pair::<FinishRead>(b"png_image_finish_read\0");
        let (c_free, r_free) = libraries.pair::<ImageFree>(b"png_image_free\0");
        let formats = [
            0,
            PNG_FORMAT_FLAG_ALPHA,
            PNG_FORMAT_FLAG_ALPHA | PNG_FORMAT_FLAG_AFIRST,
            PNG_FORMAT_FLAG_COLOR,
            PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_BGR,
            PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA,
            PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA | PNG_FORMAT_FLAG_AFIRST,
            PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA | PNG_FORMAT_FLAG_BGR,
            PNG_FORMAT_FLAG_COLOR
                | PNG_FORMAT_FLAG_ALPHA
                | PNG_FORMAT_FLAG_BGR
                | PNG_FORMAT_FLAG_AFIRST,
            PNG_FORMAT_FLAG_LINEAR,
            PNG_FORMAT_FLAG_LINEAR | PNG_FORMAT_FLAG_ALPHA,
            PNG_FORMAT_FLAG_LINEAR | PNG_FORMAT_FLAG_COLOR,
            PNG_FORMAT_FLAG_LINEAR | PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA,
        ];
        let mut rng = Rng(0x1a2b_3c4d_5566_7788);

        for &(width, height) in &[(1, 1), (1, 9), (9, 1), (2, 3), (7, 5), (31, 17)] {
            for &format in &formats {
                let channels = channels(format);
                let component_size = component_size(format);
                let mut pixels =
                    vec![0u8; width as usize * height as usize * channels * component_size];
                rng.fill(&mut pixels);
                for convert_to_8bit in if component_size == 2 {
                    [0, 1].as_slice()
                } else {
                    [0].as_slice()
                } {
                    let c_png =
                        write_png(c_write, width, height, format, *convert_to_8bit, &pixels);
                    let r_png =
                        write_png(r_write, width, height, format, *convert_to_8bit, &pixels);
                    assert_eq!(
                        c_png, r_png,
                        "write width={width} height={height} format={format:#x} convert={convert_to_8bit}"
                    );

                    for &output_format in &formats {
                        let c_pixels = read_png(c_begin, c_finish, c_free, &c_png, output_format);
                        let r_pixels = read_png(r_begin, r_finish, r_free, &c_png, output_format);
                        assert_eq!(
                            c_pixels, r_pixels,
                            "read source={format:#x} output={output_format:#x} {width}x{height}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn classic_callback_row_pipeline_matches() {
    unsafe {
        let libraries = Libraries::open();
        let (c_api, r_api) = full_api_pair(&libraries);
        let mut rng = Rng(0x6c6f_772d_6c65_7665);
        let configurations = [
            (1, 1, 0, 8, 0, 8, 0),
            (9, 3, 0, 1, 0, 8, 6),
            (17, 5, 0, 2, 1, 248, 9),
            (31, 7, 0, 4, 0, 248, 1),
            (7, 9, 0, 16, 1, 248, 6),
            (13, 4, 2, 8, 0, 8, 0),
            (13, 4, 2, 16, 1, 248, 9),
            (11, 6, 4, 8, 0, 248, 6),
            (11, 6, 4, 16, 1, 248, 6),
            (19, 3, 6, 8, 1, 248, 9),
            (19, 3, 6, 16, 0, 248, 1),
        ];

        for &(width, height, color_type, depth, interlace, filters, level) in &configurations {
            let channels = match color_type {
                0 => 1,
                2 => 3,
                4 => 2,
                6 => 4,
                _ => unreachable!(),
            };
            let rowbytes = (width * channels * depth as u32).div_ceil(8) as usize;
            let mut rows = vec![0u8; rowbytes * height as usize];
            rng.fill(&mut rows);
            let used_tail_bits = (width * channels * depth as u32) % 8;
            if used_tail_bits != 0 {
                let mask = 0xff << (8 - used_tail_bits);
                for row in rows.chunks_exact_mut(rowbytes) {
                    row[rowbytes - 1] &= mask as u8;
                }
            }

            let c_png = classic_write(
                c_api, width, height, color_type, depth, interlace, filters, level, &rows,
            );
            let r_png = classic_write(
                r_api, width, height, color_type, depth, interlace, filters, level, &rows,
            );
            assert_eq!(
                c_png, r_png,
                "classic write {width}x{height} color={color_type} depth={depth} interlace={interlace} filters={filters:#x} level={level}"
            );

            let c_rows = classic_read(c_api, &c_png);
            let r_rows = classic_read(r_api, &c_png);
            assert_eq!(c_rows, r_rows);
            assert_eq!(c_rows, rows);
        }
    }
}

unsafe fn full_api_pair(libraries: &Libraries) -> (FullApi, FullApi) {
    macro_rules! pair {
        ($ty:ty, $name:literal) => {
            unsafe { libraries.pair::<$ty>(concat!($name, "\0").as_bytes()) }
        };
    }
    let (c_create_write, r_create_write) = pair!(CreateStruct, "png_create_write_struct");
    let (c_create_read, r_create_read) = pair!(CreateStruct, "png_create_read_struct");
    let (c_create_info, r_create_info) = pair!(CreateInfo, "png_create_info_struct");
    let (c_set_write_fn, r_set_write_fn) = pair!(SetWriteFn, "png_set_write_fn");
    let (c_set_read_fn, r_set_read_fn) = pair!(SetReadFn, "png_set_read_fn");
    let (c_set_ihdr, r_set_ihdr) = pair!(SetIHDR, "png_set_IHDR");
    let (c_get_ihdr, r_get_ihdr) = pair!(GetIHDR, "png_get_IHDR");
    let (c_set_filter, r_set_filter) = pair!(SetFilter, "png_set_filter");
    let (c_set_compression, r_set_compression) = pair!(SetInt, "png_set_compression_level");
    let (c_set_interlace, r_set_interlace) = pair!(
        unsafe extern "C" fn(*mut c_void) -> i32,
        "png_set_interlace_handling"
    );
    let (c_write_info, r_write_info) = pair!(WriteInfo, "png_write_info");
    let (c_write_row, r_write_row) = pair!(WriteRow, "png_write_row");
    let (c_write_end, r_write_end) = pair!(WriteEnd, "png_write_end");
    let (c_destroy_write, r_destroy_write) = pair!(DestroyWrite, "png_destroy_write_struct");
    let (c_read_info, r_read_info) = pair!(ReadInfo, "png_read_info");
    let (c_read_update, r_read_update) = pair!(ReadUpdateInfo, "png_read_update_info");
    let (c_read_image, r_read_image) = pair!(ReadImage, "png_read_image");
    let (c_read_end, r_read_end) = pair!(ReadEnd, "png_read_end");
    let (c_destroy_read, r_destroy_read) = pair!(DestroyRead, "png_destroy_read_struct");
    let (c_get_rowbytes, r_get_rowbytes) = pair!(GetRowbytes, "png_get_rowbytes");

    (
        FullApi {
            create_write: c_create_write,
            create_read: c_create_read,
            create_info: c_create_info,
            set_write_fn: c_set_write_fn,
            set_read_fn: c_set_read_fn,
            set_ihdr: c_set_ihdr,
            get_ihdr: c_get_ihdr,
            set_filter: c_set_filter,
            set_compression_level: c_set_compression,
            set_interlace_handling: c_set_interlace,
            write_info: c_write_info,
            write_row: c_write_row,
            write_end: c_write_end,
            destroy_write: c_destroy_write,
            read_info: c_read_info,
            read_update_info: c_read_update,
            read_image: c_read_image,
            read_end: c_read_end,
            destroy_read: c_destroy_read,
            get_rowbytes: c_get_rowbytes,
        },
        FullApi {
            create_write: r_create_write,
            create_read: r_create_read,
            create_info: r_create_info,
            set_write_fn: r_set_write_fn,
            set_read_fn: r_set_read_fn,
            set_ihdr: r_set_ihdr,
            get_ihdr: r_get_ihdr,
            set_filter: r_set_filter,
            set_compression_level: r_set_compression,
            set_interlace_handling: r_set_interlace,
            write_info: r_write_info,
            write_row: r_write_row,
            write_end: r_write_end,
            destroy_write: r_destroy_write,
            read_info: r_read_info,
            read_update_info: r_read_update,
            read_image: r_read_image,
            read_end: r_read_end,
            destroy_read: r_destroy_read,
            get_rowbytes: r_get_rowbytes,
        },
    )
}

unsafe fn classic_write(
    api: FullApi,
    width: u32,
    height: u32,
    color_type: i32,
    depth: i32,
    interlace: i32,
    filters: i32,
    compression_level: i32,
    rows: &[u8],
) -> Vec<u8> {
    let version = b"1.6.59.git\0";
    let mut png =
        unsafe { (api.create_write)(version.as_ptr().cast(), ptr::null_mut(), None, None) };
    assert!(!png.is_null());
    let mut info = unsafe { (api.create_info)(png) };
    assert!(!info.is_null());
    let mut output = Vec::new();
    io_states()
        .lock()
        .unwrap()
        .insert(png as usize, (&mut output as *mut Vec<u8>) as usize);
    unsafe {
        (api.set_write_fn)(
            png,
            (&mut output as *mut Vec<u8>).cast(),
            Some(write_callback),
            None,
        );
        (api.set_ihdr)(png, info, width, height, depth, color_type, interlace, 0, 0);
        (api.set_filter)(png, 0, filters);
        (api.set_compression_level)(png, compression_level);
        (api.write_info)(png, info);
        let passes = (api.set_interlace_handling)(png);
        let rowbytes = rows.len() / height as usize;
        for _ in 0..passes {
            for row in rows.chunks_exact(rowbytes) {
                (api.write_row)(png, row.as_ptr());
            }
        }
        (api.write_end)(png, info);
    }
    io_states().lock().unwrap().remove(&(png as usize));
    unsafe { (api.destroy_write)(&mut png, &mut info) };
    assert!(png.is_null());
    output
}

unsafe fn classic_read(api: FullApi, png_bytes: &[u8]) -> Vec<u8> {
    let version = b"1.6.59.git\0";
    let mut png =
        unsafe { (api.create_read)(version.as_ptr().cast(), ptr::null_mut(), None, None) };
    assert!(!png.is_null());
    let mut info = unsafe { (api.create_info)(png) };
    assert!(!info.is_null());
    let mut input = ReadState {
        bytes: png_bytes.as_ptr(),
        length: png_bytes.len(),
        offset: 0,
    };
    io_states()
        .lock()
        .unwrap()
        .insert(png as usize, (&mut input as *mut ReadState) as usize);
    unsafe {
        (api.set_read_fn)(
            png,
            (&mut input as *mut ReadState).cast(),
            Some(read_callback),
        );
        (api.read_info)(png, info);
    }
    let mut width = 0;
    let mut height = 0;
    let mut depth = 0;
    let mut color_type = 0;
    let mut interlace = 0;
    let mut compression = 0;
    let mut filter = 0;
    assert_ne!(
        unsafe {
            (api.get_ihdr)(
                png,
                info,
                &mut width,
                &mut height,
                &mut depth,
                &mut color_type,
                &mut interlace,
                &mut compression,
                &mut filter,
            )
        },
        0
    );
    unsafe { (api.set_interlace_handling)(png) };
    unsafe { (api.read_update_info)(png, info) };
    let rowbytes = unsafe { (api.get_rowbytes)(png, info) };
    let mut output = vec![0u8; rowbytes * height as usize];
    let mut row_pointers = output
        .chunks_exact_mut(rowbytes)
        .map(|row| row.as_mut_ptr())
        .collect::<Vec<_>>();
    unsafe {
        (api.read_image)(png, row_pointers.as_mut_ptr());
        (api.read_end)(png, info);
    }
    io_states().lock().unwrap().remove(&(png as usize));
    unsafe { (api.destroy_read)(&mut png, &mut info, ptr::null_mut()) };
    assert!(png.is_null());
    output
}

unsafe fn write_png(
    function: WriteMemory,
    width: u32,
    height: u32,
    format: u32,
    convert_to_8bit: i32,
    pixels: &[u8],
) -> Vec<u8> {
    let mut query = PngImage::new(width, height, format);
    let mut size = 0usize;
    let result = unsafe {
        function(
            &mut query,
            ptr::null_mut(),
            &mut size,
            convert_to_8bit,
            pixels.as_ptr().cast(),
            0,
            ptr::null(),
        )
    };
    assert_eq!(
        result,
        1,
        "write size query failed: {:?}",
        query.message_bytes()
    );
    assert!(size > 8);

    let mut image = PngImage::new(width, height, format);
    let mut output = vec![0u8; size];
    let result = unsafe {
        function(
            &mut image,
            output.as_mut_ptr().cast(),
            &mut size,
            convert_to_8bit,
            pixels.as_ptr().cast(),
            0,
            ptr::null(),
        )
    };
    assert_eq!(result, 1, "write failed: {:?}", image.message_bytes());
    output.truncate(size);
    output
}

unsafe fn read_png(
    begin: BeginReadMemory,
    finish: FinishRead,
    free: ImageFree,
    png: &[u8],
    output_format: u32,
) -> Vec<u8> {
    let mut image = PngImage::new(0, 0, 0);
    let result = unsafe { begin(&mut image, png.as_ptr().cast(), png.len()) };
    assert_eq!(result, 1, "begin read failed: {:?}", image.message_bytes());
    image.format = output_format;
    let size = image.width as usize
        * image.height as usize
        * channels(output_format)
        * component_size(output_format);
    let mut pixels = vec![0xa5u8; size];
    let result = unsafe {
        finish(
            &mut image,
            ptr::null(),
            pixels.as_mut_ptr().cast(),
            0,
            ptr::null_mut(),
        )
    };
    if result != 1 {
        let message = image.message_bytes().to_vec();
        unsafe { free(&mut image) };
        panic!("finish read failed: {message:?}");
    }
    pixels
}

fn channels(format: u32) -> usize {
    1 + usize::from(format & PNG_FORMAT_FLAG_COLOR != 0) * 2
        + usize::from(format & PNG_FORMAT_FLAG_ALPHA != 0)
}

fn component_size(format: u32) -> usize {
    if format & PNG_FORMAT_FLAG_LINEAR != 0 {
        2
    } else {
        1
    }
}

#[test]
fn sentinel_and_simplified_api_errors_match_exactly() {
    type NullUnary = unsafe extern "C" fn(*const c_void) -> *mut c_void;
    type ZAlloc = unsafe extern "C" fn(*mut c_void, u32, u32) -> *mut c_void;
    type ResetZ = unsafe extern "C" fn(*mut c_void) -> i32;
    type SetOption = unsafe extern "C" fn(*mut c_void, i32, i32) -> i32;

    unsafe {
        let libraries = Libraries::open();
        let (c_sig, r_sig) = libraries.pair::<SigCmp>(b"png_sig_cmp\0");
        let bytes = [0u8; 8];
        assert_eq!(c_sig(bytes.as_ptr(), 0, 0), r_sig(bytes.as_ptr(), 0, 0));
        assert_eq!(c_sig(bytes.as_ptr(), 8, 1), r_sig(bytes.as_ptr(), 8, 1));

        let (c_zalloc, r_zalloc) = libraries.pair::<ZAlloc>(b"png_zalloc\0");
        assert_eq!(
            c_zalloc(ptr::null_mut(), 1, 1),
            r_zalloc(ptr::null_mut(), 1, 1)
        );
        let (c_info, r_info) = libraries.pair::<NullUnary>(b"png_create_info_struct\0");
        assert_eq!(c_info(ptr::null()), r_info(ptr::null()));
        let (c_io, r_io) = libraries.pair::<NullUnary>(b"png_get_io_ptr\0");
        assert_eq!(c_io(ptr::null()), r_io(ptr::null()));
        let (c_reset, r_reset) = libraries.pair::<ResetZ>(b"png_reset_zstream\0");
        assert_eq!(c_reset(ptr::null_mut()), r_reset(ptr::null_mut()));
        let (c_option, r_option) = libraries.pair::<SetOption>(b"png_set_option\0");
        for option in [-1, 0, 1, 2, 15, 16, i32::MAX] {
            for onoff in [-1, 0, 1, 2, 3, i32::MAX] {
                assert_eq!(
                    c_option(ptr::null_mut(), option, onoff),
                    r_option(ptr::null_mut(), option, onoff)
                );
            }
        }

        for case in 0..22 {
            let c = run_error_child("c", case);
            let rust = run_error_child("rust", case);
            assert_child_results_match(case, &c, &rust);
        }

        let version_functions = [
            b"png_get_copyright\0".as_slice(),
            b"png_get_header_ver\0".as_slice(),
            b"png_get_header_version\0".as_slice(),
            b"png_get_libpng_ver\0".as_slice(),
        ];
        type VersionString = unsafe extern "C" fn(*const c_void) -> *const c_char;
        for name in version_functions {
            let (c, rust) = libraries.pair::<VersionString>(name);
            assert_eq!(
                CStr::from_ptr(c(ptr::null())).to_bytes(),
                CStr::from_ptr(rust(ptr::null())).to_bytes()
            );
        }
    }
}

#[test]
fn ffi_error_child() {
    let Ok(kind) = std::env::var("PNG_DIFF_CHILD_LIBRARY") else {
        return;
    };
    let case = std::env::var("PNG_DIFF_CHILD_CASE")
        .unwrap()
        .parse::<usize>()
        .unwrap();
    unsafe {
        let libraries = Libraries::open();
        if case < 5 {
            let (c, rust) = libraries.pair::<WriteMemory>(b"png_image_write_to_memory\0");
            let function = if kind == "c" { c } else { rust };
            let cases = [
                (0, 1, 1, 0, false),
                (PNG_IMAGE_VERSION, 0, 1, 0, false),
                (PNG_IMAGE_VERSION, 1, 0, 0, false),
                (PNG_IMAGE_VERSION, 1, 1, 0xffff_ffff, false),
                (PNG_IMAGE_VERSION, 1, 1, 0, true),
            ];
            let (version, width, height, format, null_pixels) = cases[case];
            let mut image = PngImage::new(width, height, format);
            image.version = version;
            let pixels = [0u8; 8];
            let pixel_ptr = if null_pixels {
                ptr::null()
            } else {
                pixels.as_ptr().cast()
            };
            let mut size = 0;
            let result = function(
                &mut image,
                ptr::null_mut(),
                &mut size,
                0,
                pixel_ptr,
                0,
                ptr::null(),
            );
            print_child_result(result, size, &image);
        } else if case < 17 {
            let (c, rust) =
                libraries.pair::<BeginReadMemory>(b"png_image_begin_read_from_memory\0");
            let function = if kind == "c" { c } else { rust };
            let malformed = [
                Vec::new(),
                vec![0],
                vec![137, 80, 78, 71, 13, 10, 26, 10],
                vec![0xff; 64],
            ];
            let read_case = case - 5;
            let input = &malformed[read_case / 3];
            let version = [0, PNG_IMAGE_VERSION, 2][read_case % 3];
            let mut image = PngImage::new(0, 0, 0);
            image.version = version;
            let pointer = if input.is_empty() {
                ptr::null()
            } else {
                input.as_ptr().cast()
            };
            let result = function(&mut image, pointer, input.len());
            print_child_result(result, input.len(), &image);
        } else {
            let (c_write, r_write) = libraries.pair::<WriteMemory>(b"png_image_write_to_memory\0");
            let (c_begin, r_begin) =
                libraries.pair::<BeginReadMemory>(b"png_image_begin_read_from_memory\0");
            let (c_finish, r_finish) = libraries.pair::<FinishRead>(b"png_image_finish_read\0");
            let write = if kind == "c" { c_write } else { r_write };
            let begin = if kind == "c" { c_begin } else { r_begin };
            let finish = if kind == "c" { c_finish } else { r_finish };
            let png_bytes = write_png(write, 1, 1, 0, 0, &[0x7f]);
            let mut image = PngImage::new(0, 0, 0);
            assert_eq!(
                begin(&mut image, png_bytes.as_ptr().cast(), png_bytes.len()),
                1
            );
            let mut output = [0u8; 8];
            let mut buffer = output.as_mut_ptr().cast();
            let mut row_stride = 0;
            match case {
                17 => image.format |= PNG_FORMAT_FLAG_COLORMAP,
                18 => image.height = u32::MAX,
                19 => buffer = ptr::null_mut(),
                20 => image.width = u32::MAX,
                21 => image.version = 2,
                _ => unreachable!(),
            }
            if case == 18 {
                row_stride = 1;
            }
            let result = finish(&mut image, ptr::null(), buffer, row_stride, ptr::null_mut());
            print_child_result(result, png_bytes.len(), &image);
        }
    }
}

fn run_error_child(kind: &str, case: usize) -> Output {
    Command::new(std::env::current_exe().unwrap())
        .args(["ffi_error_child", "--exact", "--nocapture"])
        .env("PNG_DIFF_CHILD_LIBRARY", kind)
        .env("PNG_DIFF_CHILD_CASE", case.to_string())
        .output()
        .unwrap()
}

fn assert_child_results_match(case: usize, c: &Output, rust: &Output) {
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(
        (c.status.code(), c.status.signal()),
        (rust.status.code(), rust.status.signal()),
        "case {case} termination differs\nC stderr: {}\nRust stderr: {}",
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&rust.stderr)
    );
    if c.status.success() {
        assert_eq!(
            child_result(&c.stdout),
            child_result(&rust.stdout),
            "case {case}"
        );
    }
}

fn child_result(stdout: &[u8]) -> &[u8] {
    stdout
        .split(|&byte| byte == b'\n')
        .find(|line| line.starts_with(b"FFI_RESULT "))
        .expect("child result marker")
}

fn print_child_result(result: i32, size: usize, image: &PngImage) {
    print!("FFI_RESULT {result} {size} {} ", image.warning_or_error);
    for byte in image.message_bytes() {
        print!("{byte:02x}");
    }
    println!();
}
