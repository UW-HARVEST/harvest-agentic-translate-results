use libloading::Library;
use std::collections::BTreeSet;
use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;

const PNG_IMAGE_VERSION: u32 = 1;
const FORMAT_ALPHA: u32 = 0x01;
const FORMAT_COLOR: u32 = 0x02;
const FORMAT_LINEAR: u32 = 0x04;
const FORMAT_BGR: u32 = 0x10;
const FORMAT_AFIRST: u32 = 0x20;

#[repr(C)]
#[derive(Clone)]
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
    fn new(width: u32, height: u32, format: u32, flags: u32) -> Self {
        Self {
            opaque: ptr::null_mut(),
            version: PNG_IMAGE_VERSION,
            width,
            height,
            format,
            flags,
            colormap_entries: 0,
            warning_or_error: 0,
            message: [0; 64],
        }
    }

    fn message_bytes(&self) -> Vec<u8> {
        self.message
            .iter()
            .map(|&byte| byte as u8)
            .take_while(|&byte| byte != 0)
            .collect()
    }

    fn observable(&self) -> (u32, u32, u32, u32, u32, u32, Vec<u8>) {
        (
            self.version,
            self.width,
            self.height,
            self.format,
            self.flags,
            self.colormap_entries,
            self.warning_or_error,
        )
            .pipe(|(version, width, height, format, flags, entries, status)| {
                (
                    version,
                    width,
                    height,
                    format,
                    flags,
                    entries,
                    [
                        status.to_le_bytes().as_slice(),
                        self.message_bytes().as_slice(),
                    ]
                    .concat(),
                )
            })
    }
}

trait Pipe: Sized {
    fn pipe<T>(self, f: impl FnOnce(Self) -> T) -> T {
        f(self)
    }
}
impl<T> Pipe for T {}

type AccessVersion = unsafe extern "C" fn() -> u32;
type SigCmp = unsafe extern "C" fn(*const u8, usize, usize) -> c_int;
type GetU32 = unsafe extern "C" fn(*const u8) -> u32;
type GetU16 = unsafe extern "C" fn(*const u8) -> u16;
type GetI32 = unsafe extern "C" fn(*const u8) -> i32;
type SaveU32 = unsafe extern "C" fn(*mut u8, u32);
type SaveI32 = unsafe extern "C" fn(*mut u8, i32);
type SaveU16 = unsafe extern "C" fn(*mut u8, u32);
type CheckFp = unsafe extern "C" fn(*const c_char, usize) -> c_int;
type CheckFpNumber = unsafe extern "C" fn(*const c_char, usize, *mut c_int, *mut usize) -> c_int;
type SetOption = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> c_int;
type ImageWrite = unsafe extern "C" fn(
    *mut PngImage,
    *mut c_void,
    *mut usize,
    c_int,
    *const c_void,
    i32,
    *const c_void,
) -> c_int;
type ImageBeginMemory = unsafe extern "C" fn(*mut PngImage, *const c_void, usize) -> c_int;
type ImageFinish =
    unsafe extern "C" fn(*mut PngImage, *const c_void, *mut c_void, i32, *mut c_void) -> c_int;
type ImageFree = unsafe extern "C" fn(*mut PngImage);

struct Api {
    _library: Library,
    access_version: AccessVersion,
    sig_cmp: SigCmp,
    get_u32: GetU32,
    get_u16: GetU16,
    get_i32: GetI32,
    save_u32: SaveU32,
    save_i32: SaveI32,
    save_u16: SaveU16,
    check_fp: CheckFp,
    check_fp_number: CheckFpNumber,
    set_option: SetOption,
    image_write: ImageWrite,
    image_begin_memory: ImageBeginMemory,
    image_finish: ImageFinish,
    image_free: ImageFree,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }.unwrap();
        macro_rules! symbol {
            ($name:literal, $ty:ty) => {
                *unsafe { library.get::<$ty>(concat!($name, "\0").as_bytes()) }.unwrap()
            };
        }
        Self {
            access_version: symbol!("png_access_version_number", AccessVersion),
            sig_cmp: symbol!("png_sig_cmp", SigCmp),
            get_u32: symbol!("png_get_uint_32", GetU32),
            get_u16: symbol!("png_get_uint_16", GetU16),
            get_i32: symbol!("png_get_int_32", GetI32),
            save_u32: symbol!("png_save_uint_32", SaveU32),
            save_i32: symbol!("png_save_int_32", SaveI32),
            save_u16: symbol!("png_save_uint_16", SaveU16),
            check_fp: symbol!("png_check_fp_string", CheckFp),
            check_fp_number: symbol!("png_check_fp_number", CheckFpNumber),
            set_option: symbol!("png_set_option", SetOption),
            image_write: symbol!("png_image_write_to_memory", ImageWrite),
            image_begin_memory: symbol!("png_image_begin_read_from_memory", ImageBeginMemory),
            image_finish: symbol!("png_image_finish_read", ImageFinish),
            image_free: symbol!("png_image_free", ImageFree),
            _library: library,
        }
    }
}

fn libraries() -> (Api, Api) {
    let math = unsafe {
        libloading::os::unix::Library::open(
            Some("libm.so.6"),
            libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_GLOBAL,
        )
    }
    .expect("load libm for the CMake reference, which does not link it");
    std::mem::forget(math);

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c = root.join("../c_src/build/libpng.so");
    let rust = root.join("target/release/liblibpng.so");
    assert!(
        c.is_file(),
        "missing {}; build the C reference first",
        c.display()
    );
    assert!(
        rust.is_file(),
        "missing {}; run cargo build --release first",
        rust.display()
    );
    unsafe { (Api::load(&c), Api::load(&rust)) }
}

fn next_random(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

fn aligned_random_bytes(length: usize, state: &mut u64) -> Vec<u64> {
    let mut words = vec![0_u64; length.div_ceil(8)];
    for word in &mut words {
        *word = next_random(state);
    }
    words
}

fn image_bytes(format: u32, width: u32, height: u32) -> usize {
    let channels = ((format & (FORMAT_COLOR | FORMAT_ALPHA)) + 1) as usize;
    let component = (((format & FORMAT_LINEAR) >> 2) + 1) as usize;
    width as usize * height as usize * channels * component
}

unsafe fn write_png(
    api: &Api,
    format: u32,
    flags: u32,
    width: u32,
    height: u32,
    input: *const c_void,
    row_stride: i32,
) -> (c_int, PngImage, Vec<u8>) {
    let mut size = 0_usize;
    let mut sizing = PngImage::new(width, height, format, flags);
    let first = unsafe {
        (api.image_write)(
            &mut sizing,
            ptr::null_mut(),
            &mut size,
            0,
            input,
            row_stride,
            ptr::null(),
        )
    };
    if first == 0 {
        return (first, sizing, Vec::new());
    }

    let mut output = vec![0_u8; size];
    let mut written = output.len();
    let mut image = PngImage::new(width, height, format, flags);
    let result = unsafe {
        (api.image_write)(
            &mut image,
            output.as_mut_ptr().cast(),
            &mut written,
            0,
            input,
            row_stride,
            ptr::null(),
        )
    };
    output.truncate(written.min(output.len()));
    (result, image, output)
}

#[test]
fn dynamic_symbol_surface_is_identical_and_loadable() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let paths = [
        root.join("../c_src/build/libpng.so"),
        root.join("target/release/liblibpng.so"),
    ];
    let mut sets = Vec::new();
    for path in &paths {
        let output = Command::new("nm")
            .args(["-D", "--defined-only", "--format=posix"])
            .arg(path)
            .output()
            .unwrap();
        assert!(output.status.success());
        let symbols: BTreeSet<String> = String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .filter_map(|line| line.split_whitespace().next())
            .filter(|name| name.starts_with("png_"))
            .map(str::to_owned)
            .collect();
        assert_eq!(symbols.len(), 384);
        let library = unsafe { Library::new(path) }.unwrap();
        for name in &symbols {
            let terminated = format!("{name}\0");
            let address = unsafe { library.get::<*mut c_void>(terminated.as_bytes()) }.unwrap();
            assert!(!(*address).is_null(), "{name} resolved to NULL");
        }
        sets.push(symbols);
    }
    assert_eq!(sets[0], sets[1]);
}

#[test]
fn exported_tables_match_byte_for_byte() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c = unsafe { Library::new(root.join("../c_src/build/libpng.so")) }.unwrap();
    let rust = unsafe { Library::new(root.join("target/release/liblibpng.so")) }.unwrap();
    unsafe {
        let c_table = c.get::<*const u16>(b"png_sRGB_table\0").unwrap();
        let r_table = rust.get::<*const u16>(b"png_sRGB_table\0").unwrap();
        assert_eq!(
            std::slice::from_raw_parts(*c_table, 256),
            std::slice::from_raw_parts(*r_table, 256)
        );
        let c_base = c.get::<*const u16>(b"png_sRGB_base\0").unwrap();
        let r_base = rust.get::<*const u16>(b"png_sRGB_base\0").unwrap();
        assert_eq!(
            std::slice::from_raw_parts(*c_base, 512),
            std::slice::from_raw_parts(*r_base, 512)
        );
        let c_delta = c.get::<*const u8>(b"png_sRGB_delta\0").unwrap();
        let r_delta = rust.get::<*const u8>(b"png_sRGB_delta\0").unwrap();
        assert_eq!(
            std::slice::from_raw_parts(*c_delta, 512),
            std::slice::from_raw_parts(*r_delta, 512)
        );
    }
}

#[test]
fn randomized_low_level_integer_signature_and_number_apis_match() {
    let (c, rust) = libraries();
    assert_eq!(unsafe { (c.access_version)() }, unsafe {
        (rust.access_version)()
    });
    let mut state = 0x9e37_79b9_7f4a_7c15;
    for _ in 0..10_000 {
        let mut input = [0_u8; 32];
        for byte in &mut input {
            *byte = next_random(&mut state) as u8;
        }
        let start = (next_random(&mut state) % 10) as usize;
        let count = (next_random(&mut state) % 12) as usize;
        assert_eq!(
            unsafe { (c.sig_cmp)(input.as_ptr(), start, count) },
            unsafe { (rust.sig_cmp)(input.as_ptr(), start, count) }
        );
        assert_eq!(unsafe { (c.get_u32)(input.as_ptr()) }, unsafe {
            (rust.get_u32)(input.as_ptr())
        });
        assert_eq!(unsafe { (c.get_u16)(input.as_ptr()) }, unsafe {
            (rust.get_u16)(input.as_ptr())
        });
        assert_eq!(unsafe { (c.get_i32)(input.as_ptr()) }, unsafe {
            (rust.get_i32)(input.as_ptr())
        });

        let value = next_random(&mut state) as u32;
        let mut c_bytes = [0_u8; 4];
        let mut rust_bytes = [0_u8; 4];
        unsafe {
            (c.save_u32)(c_bytes.as_mut_ptr(), value);
            (rust.save_u32)(rust_bytes.as_mut_ptr(), value);
        }
        assert_eq!(c_bytes, rust_bytes);
        unsafe {
            (c.save_i32)(c_bytes.as_mut_ptr(), value as i32);
            (rust.save_i32)(rust_bytes.as_mut_ptr(), value as i32);
        }
        assert_eq!(c_bytes, rust_bytes);
        unsafe {
            (c.save_u16)(c_bytes.as_mut_ptr(), value);
            (rust.save_u16)(rust_bytes.as_mut_ptr(), value);
        }
        assert_eq!(&c_bytes[..2], &rust_bytes[..2]);

        let length = (next_random(&mut state) % input.len() as u64) as usize;
        assert_eq!(
            unsafe { (c.check_fp)(input.as_ptr().cast(), length) },
            unsafe { (rust.check_fp)(input.as_ptr().cast(), length) }
        );
        let mut c_state = (next_random(&mut state) & 0x7ff) as c_int;
        let mut rust_state = c_state;
        let mut c_offset = (next_random(&mut state) % (length as u64 + 1)) as usize;
        let mut rust_offset = c_offset;
        assert_eq!(
            unsafe {
                (c.check_fp_number)(input.as_ptr().cast(), length, &mut c_state, &mut c_offset)
            },
            unsafe {
                (rust.check_fp_number)(
                    input.as_ptr().cast(),
                    length,
                    &mut rust_state,
                    &mut rust_offset,
                )
            }
        );
        assert_eq!((c_state, c_offset), (rust_state, rust_offset));
    }
}

#[test]
fn randomized_simplified_write_and_read_formats_match_byte_for_byte() {
    let (c, rust) = libraries();
    let formats = [
        0,
        FORMAT_ALPHA,
        FORMAT_COLOR,
        FORMAT_COLOR | FORMAT_ALPHA,
        FORMAT_COLOR | FORMAT_BGR,
        FORMAT_COLOR | FORMAT_ALPHA | FORMAT_BGR,
        FORMAT_ALPHA | FORMAT_AFIRST,
        FORMAT_COLOR | FORMAT_ALPHA | FORMAT_AFIRST,
        FORMAT_LINEAR,
        FORMAT_LINEAR | FORMAT_ALPHA,
        FORMAT_LINEAR | FORMAT_COLOR,
        FORMAT_LINEAR | FORMAT_COLOR | FORMAT_ALPHA,
    ];
    let mut state = 0xd1b5_4a32_d192_ed03;
    for &format in &formats {
        for case in 0..32 {
            let width = 1 + (next_random(&mut state) % 17) as u32;
            let height = 1 + (next_random(&mut state) % 13) as u32;
            let byte_count = image_bytes(format, width, height);
            let input = aligned_random_bytes(byte_count, &mut state);
            let input_ptr = input.as_ptr().cast();
            let flags = if case & 1 == 0 { 0 } else { 0x02 };
            let channels = ((format & (FORMAT_COLOR | FORMAT_ALPHA)) + 1) as i32;
            let stride = if case % 3 == 0 {
                -(width as i32 * channels)
            } else {
                0
            };
            let (c_result, c_image, c_png) =
                unsafe { write_png(&c, format, flags, width, height, input_ptr, stride) };
            let (r_result, r_image, r_png) =
                unsafe { write_png(&rust, format, flags, width, height, input_ptr, stride) };
            assert_eq!(c_result, r_result, "format={format:#x} case={case}");
            assert_eq!(c_image.observable(), r_image.observable());
            assert_eq!(c_png, r_png, "format={format:#x} case={case}");
            assert_eq!(c_result, 1);

            for &read_format in &formats {
                let mut ci = PngImage::new(0, 0, 0, 0);
                let mut ri = PngImage::new(0, 0, 0, 0);
                let cb =
                    unsafe { (c.image_begin_memory)(&mut ci, c_png.as_ptr().cast(), c_png.len()) };
                let rb = unsafe {
                    (rust.image_begin_memory)(&mut ri, r_png.as_ptr().cast(), r_png.len())
                };
                assert_eq!(cb, rb);
                assert_eq!(ci.observable(), ri.observable());
                assert_eq!(cb, 1);
                ci.format = read_format;
                ri.format = read_format;
                let size = image_bytes(read_format, width, height);
                let mut c_out = vec![0_u8; size];
                let mut r_out = vec![0_u8; size];
                let cf = unsafe {
                    (c.image_finish)(
                        &mut ci,
                        ptr::null(),
                        c_out.as_mut_ptr().cast(),
                        0,
                        ptr::null_mut(),
                    )
                };
                let rf = unsafe {
                    (rust.image_finish)(
                        &mut ri,
                        ptr::null(),
                        r_out.as_mut_ptr().cast(),
                        0,
                        ptr::null_mut(),
                    )
                };
                assert_eq!(cf, rf, "write={format:#x} read={read_format:#x}");
                assert_eq!(ci.observable(), ri.observable());
                assert_eq!(c_out, r_out);
            }
        }
    }
}

#[test]
fn exact_error_sentinels_and_out_of_range_ffi_values_match() {
    let (c, rust) = libraries();
    let bytes = [137_u8, 80, 78, 71, 13, 10, 26, 10, 0];
    for &(start, count) in &[(0, 0), (8, 1), (9, 9), (usize::MAX, usize::MAX)] {
        assert_eq!(
            unsafe { (c.sig_cmp)(bytes.as_ptr(), start, count) },
            unsafe { (rust.sig_cmp)(bytes.as_ptr(), start, count) }
        );
    }
    for option in [-1, 0, 1, 15, 16, 17, c_int::MAX, c_int::MIN] {
        for onoff in [-1, 0, 1, 2, c_int::MAX] {
            assert_eq!(
                unsafe { (c.set_option)(ptr::null_mut(), option, onoff) },
                unsafe { (rust.set_option)(ptr::null_mut(), option, onoff) }
            );
        }
    }

    let mut size = 99_usize;
    assert_eq!(
        unsafe {
            (c.image_write)(
                ptr::null_mut(),
                ptr::null_mut(),
                &mut size,
                0,
                ptr::null(),
                0,
                ptr::null(),
            )
        },
        unsafe {
            (rust.image_write)(
                ptr::null_mut(),
                ptr::null_mut(),
                &mut size,
                0,
                ptr::null(),
                0,
                ptr::null(),
            )
        }
    );
    assert_eq!(
        unsafe {
            (c.image_finish)(
                ptr::null_mut(),
                ptr::null(),
                ptr::null_mut(),
                0,
                ptr::null_mut(),
            )
        },
        unsafe {
            (rust.image_finish)(
                ptr::null_mut(),
                ptr::null(),
                ptr::null_mut(),
                0,
                ptr::null_mut(),
            )
        }
    );

    for version in [0, 2, u32::MAX] {
        let mut ci = PngImage::new(1, 1, 0, 0);
        let mut ri = ci.clone();
        ci.version = version;
        ri.version = version;
        let input = [0_u8; 8];
        let mut c_size = 0;
        let mut r_size = 0;
        let cw = unsafe {
            (c.image_write)(
                &mut ci,
                ptr::null_mut(),
                &mut c_size,
                0,
                input.as_ptr().cast(),
                0,
                ptr::null(),
            )
        };
        let rw = unsafe {
            (rust.image_write)(
                &mut ri,
                ptr::null_mut(),
                &mut r_size,
                0,
                input.as_ptr().cast(),
                0,
                ptr::null(),
            )
        };
        assert_eq!((cw, c_size, ci.observable()), (rw, r_size, ri.observable()));
    }
}

#[test]
fn randomized_malformed_png_rejections_match() {
    let (c, rust) = libraries();
    let mut state = 0x243f_6a88_85a3_08d3;
    let source = aligned_random_bytes(8 * 7 * 4, &mut state);
    let (_, _, valid) = unsafe {
        write_png(
            &c,
            FORMAT_COLOR | FORMAT_ALPHA,
            0,
            8,
            7,
            source.as_ptr().cast(),
            0,
        )
    };
    assert!(!valid.is_empty());

    for case in 0..2_000 {
        let mut data = if case < 500 {
            let length = (next_random(&mut state) % 160) as usize;
            aligned_random_bytes(length, &mut state)
                .iter()
                .flat_map(|word| word.to_ne_bytes())
                .take(length)
                .collect()
        } else {
            valid.clone()
        };
        if case >= 500 {
            let changes = 1 + (next_random(&mut state) % 4) as usize;
            for _ in 0..changes {
                let index = (next_random(&mut state) % data.len() as u64) as usize;
                data[index] ^= (next_random(&mut state) as u8) | 1;
            }
            if case % 5 == 0 {
                let new_len = (next_random(&mut state) % data.len() as u64) as usize;
                data.truncate(new_len);
            }
        }

        let mut ci = PngImage::new(0, 0, 0, 0);
        let mut ri = PngImage::new(0, 0, 0, 0);
        let cb = unsafe { (c.image_begin_memory)(&mut ci, data.as_ptr().cast(), data.len()) };
        let rb = unsafe { (rust.image_begin_memory)(&mut ri, data.as_ptr().cast(), data.len()) };
        assert_eq!(cb, rb, "case={case}");
        assert_eq!(ci.observable(), ri.observable(), "case={case}");

        if cb != 0 && ci.width.saturating_mul(ci.height) <= 1_000_000 {
            ci.format = FORMAT_COLOR | FORMAT_ALPHA;
            ri.format = FORMAT_COLOR | FORMAT_ALPHA;
            let length = image_bytes(ci.format, ci.width, ci.height);
            let mut c_out = vec![0_u8; length];
            let mut r_out = vec![0_u8; length];
            let cf = unsafe {
                (c.image_finish)(
                    &mut ci,
                    ptr::null(),
                    c_out.as_mut_ptr().cast(),
                    0,
                    ptr::null_mut(),
                )
            };
            let rf = unsafe {
                (rust.image_finish)(
                    &mut ri,
                    ptr::null(),
                    r_out.as_mut_ptr().cast(),
                    0,
                    ptr::null_mut(),
                )
            };
            assert_eq!(cf, rf, "case={case}");
            assert_eq!(ci.observable(), ri.observable(), "case={case}");
            assert_eq!(c_out, r_out, "case={case}");
        } else {
            unsafe {
                (c.image_free)(&mut ci);
                (rust.image_free)(&mut ri);
            }
        }
    }
}
