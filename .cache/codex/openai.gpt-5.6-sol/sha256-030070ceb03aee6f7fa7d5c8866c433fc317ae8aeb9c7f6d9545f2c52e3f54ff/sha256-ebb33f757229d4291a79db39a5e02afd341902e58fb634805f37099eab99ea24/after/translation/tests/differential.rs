use libloading::Library;
use std::mem::{offset_of, size_of};
use std::path::{Path, PathBuf};
use std::process::Command;

#[repr(C)]
struct Tflac {
    samplerate: u32,
    channels: u32,
    bitdepth: u32,
    channel_mode: u8,
    frame_header: u32,
    cur_blocksize: u32,
}

const STRUCT_SIZE: usize = size_of::<Tflac>();

#[repr(C, align(4))]
#[derive(Clone)]
struct RawTflac {
    bytes: [u8; STRUCT_SIZE],
}

type UpdateFrameHeader = unsafe extern "C" fn(*mut Tflac);

struct Api {
    _library: Library,
    update_frame_header: UpdateFrameHeader,
}

impl Api {
    fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let update_frame_header = unsafe {
            *library
                .get::<UpdateFrameHeader>(b"update_frame_header\0")
                .unwrap_or_else(|error| {
                    panic!(
                        "failed to load update_frame_header from {}: {error}",
                        path.display()
                    )
                })
        };
        Self {
            _library: library,
            update_frame_header,
        }
    }

    fn call(&self, value: &mut RawTflac) {
        unsafe {
            (self.update_frame_header)(value.bytes.as_mut_ptr().cast::<Tflac>());
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Input {
    samplerate: u32,
    channels: u32,
    bitdepth: u32,
    channel_mode: u8,
    frame_header: u32,
    cur_blocksize: u32,
}

impl RawTflac {
    fn from_input(input: Input) -> Self {
        let mut value = Self {
            bytes: [0xa5; STRUCT_SIZE],
        };
        put_u32(
            &mut value.bytes,
            offset_of!(Tflac, samplerate),
            input.samplerate,
        );
        put_u32(
            &mut value.bytes,
            offset_of!(Tflac, channels),
            input.channels,
        );
        put_u32(
            &mut value.bytes,
            offset_of!(Tflac, bitdepth),
            input.bitdepth,
        );
        value.bytes[offset_of!(Tflac, channel_mode)] = input.channel_mode;
        put_u32(
            &mut value.bytes,
            offset_of!(Tflac, frame_header),
            input.frame_header,
        );
        put_u32(
            &mut value.bytes,
            offset_of!(Tflac, cur_blocksize),
            input.cur_blocksize,
        );
        value
    }
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_ne_bytes());
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    std::env::var_os("C_REFERENCE_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let build_dir = manifest_dir().join("../c_src/build");
            let mut libraries: Vec<_> = std::fs::read_dir(&build_dir)
                .unwrap_or_else(|error| {
                    panic!(
                        "failed to read C build directory {}: {error}",
                        build_dir.display()
                    )
                })
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|extension| extension == "so"))
                .collect();
            libraries.sort();
            assert_eq!(
                libraries.len(),
                1,
                "expected exactly one C shared library in {}, found {libraries:?}",
                build_dir.display()
            );
            libraries.pop().unwrap()
        })
}

fn rust_library_path() -> PathBuf {
    std::env::var_os("RUST_TRANSLATION_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            manifest_dir()
                .join("target/release")
                .join("libupdate_frame_header_lib.so")
        })
}

fn assert_layout() {
    assert_eq!(STRUCT_SIZE, 24);
    assert_eq!(offset_of!(Tflac, samplerate), 0);
    assert_eq!(offset_of!(Tflac, channels), 4);
    assert_eq!(offset_of!(Tflac, bitdepth), 8);
    assert_eq!(offset_of!(Tflac, channel_mode), 12);
    assert_eq!(offset_of!(Tflac, frame_header), 16);
    assert_eq!(offset_of!(Tflac, cur_blocksize), 20);
}

fn assert_differential(c_api: &Api, rust_api: &Api, input: Input, context: &str) {
    let mut c_value = RawTflac::from_input(input);
    let mut rust_value = c_value.clone();
    c_api.call(&mut c_value);
    rust_api.call(&mut rust_value);
    assert_eq!(
        c_value.bytes, rust_value.bytes,
        "{context}; input={input:?}; C={:02x?}; Rust={:02x?}",
        c_value.bytes, rust_value.bytes
    );
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 16) as u32
    }
}

const EXACT_BLOCKS: [u32; 13] = [
    192, 576, 1152, 2304, 4608, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768,
];
const EXACT_RATES: [u32; 11] = [
    882000, 176400, 192000, 8000, 16000, 22050, 24000, 32000, 44100, 48000, 96000,
];
const EXACT_DEPTHS: [u32; 6] = [8, 12, 16, 20, 24, 32];

fn block_for_class(class: usize, rng: &mut Rng) -> u32 {
    match class {
        0..=12 => EXACT_BLOCKS[class],
        13 => loop {
            let value = rng.next_u32() % 257;
            if !EXACT_BLOCKS.contains(&value) {
                break value;
            }
        },
        14 => loop {
            let value = rng.next_u32();
            if value > 256 && !EXACT_BLOCKS.contains(&value) {
                break value;
            }
        },
        _ => unreachable!(),
    }
}

fn rate_for_class(class: usize, rng: &mut Rng) -> u32 {
    match class {
        0..=10 => EXACT_RATES[class],
        11 => loop {
            let value = (rng.next_u32() % 256) * 1000;
            if !EXACT_RATES.contains(&value) {
                break value;
            }
        },
        12 => loop {
            const MAX_QUOTIENT: u32 = u32::MAX / 1000;
            let quotient = 256 + rng.next_u32() % (MAX_QUOTIENT - 256 + 1);
            let value = quotient * 1000;
            if !EXACT_RATES.contains(&value) {
                break value;
            }
        },
        13 => loop {
            let value = rng.next_u32() % 65536;
            if value % 1000 != 0 && !EXACT_RATES.contains(&value) {
                break value;
            }
        },
        14 => loop {
            let quotient = 6554 + rng.next_u32() % (65535 - 6554 + 1);
            let value = quotient * 10;
            if value >= 65536 && value % 1000 != 0 && !EXACT_RATES.contains(&value) {
                break value;
            }
        },
        15 => loop {
            const MAX_QUOTIENT: u32 = u32::MAX / 10;
            let quotient = 65536 + rng.next_u32() % (MAX_QUOTIENT - 65536 + 1);
            let value = quotient * 10;
            if value % 1000 != 0 && !EXACT_RATES.contains(&value) {
                break value;
            }
        },
        16 => loop {
            let value = rng.next_u32();
            if value >= 65536
                && value % 1000 != 0
                && value % 10 != 0
                && !EXACT_RATES.contains(&value)
            {
                break value;
            }
        },
        _ => unreachable!(),
    }
}

fn mode_for_class(class: usize, rng: &mut Rng) -> u8 {
    ((rng.next_u32() % 64) as u8) * 4 + class as u8
}

fn depth_for_class(class: usize, rng: &mut Rng) -> u32 {
    match class {
        0..=5 => EXACT_DEPTHS[class],
        6 => loop {
            let value = rng.next_u32();
            if !EXACT_DEPTHS.contains(&value) {
                break value;
            }
        },
        _ => unreachable!(),
    }
}

#[test]
fn all_7140_valid_configuration_rows_match_byte_for_byte() {
    assert_layout();
    let c_api = Api::load(&c_library_path());
    let rust_api = Api::load(&rust_library_path());
    let mut rng = Rng::new(0x6a09_e667_f3bc_c909);
    let mut row = 0usize;

    for block_class in 0..15 {
        for rate_class in 0..17 {
            for mode_class in 0..4 {
                for depth_class in 0..7 {
                    row += 1;
                    for sample in 0..16 {
                        let input = Input {
                            samplerate: rate_for_class(rate_class, &mut rng),
                            channels: rng.next_u32(),
                            bitdepth: depth_for_class(depth_class, &mut rng),
                            channel_mode: mode_for_class(mode_class, &mut rng),
                            frame_header: rng.next_u32(),
                            cur_blocksize: block_for_class(block_class, &mut rng),
                        };
                        assert_differential(
                            &c_api,
                            &rust_api,
                            input,
                            &format!(
                                "CONFIGS.md row {row}, randomized sample {sample}; \
                                 classes=({block_class},{rate_class},{mode_class},{depth_class})"
                            ),
                        );
                    }
                }
            }
        }
    }

    assert_eq!(row, 7140);
}

fn run_boundary(input: Input, row: &str) {
    assert_layout();
    let c_api = Api::load(&c_library_path());
    let rust_api = Api::load(&rust_library_path());
    assert_differential(&c_api, &rust_api, input, row);
}

#[test]
fn error_boundary_g2_all_zero() {
    run_boundary(
        Input {
            samplerate: 0,
            channels: 0,
            bitdepth: 0,
            channel_mode: 0,
            frame_header: 0,
            cur_blocksize: 0,
        },
        "ERRORS.md G2",
    );
}

#[test]
fn error_boundary_g3_all_maximum_width_values() {
    run_boundary(
        Input {
            samplerate: u32::MAX,
            channels: u32::MAX,
            bitdepth: u32::MAX,
            channel_mode: u8::MAX,
            frame_header: u32::MAX,
            cur_blocksize: u32::MAX,
        },
        "ERRORS.md G3",
    );
}

#[test]
fn error_boundary_g4_enum_count_value() {
    run_boundary(
        Input {
            samplerate: 44100,
            channels: 2,
            bitdepth: 16,
            channel_mode: 4,
            frame_header: 0xdead_beef,
            cur_blocksize: 4096,
        },
        "ERRORS.md G4",
    );
}

#[test]
fn error_boundary_g5_maximum_unnamed_enum_value() {
    run_boundary(
        Input {
            samplerate: 96000,
            channels: 8,
            bitdepth: 24,
            channel_mode: u8::MAX,
            frame_header: 0x0123_4567,
            cur_blocksize: 16384,
        },
        "ERRORS.md G5",
    );
}

#[test]
fn error_boundary_g6_one_past_largest_listed_values() {
    run_boundary(
        Input {
            samplerate: 882001,
            channels: 9,
            bitdepth: 33,
            channel_mode: 3,
            frame_header: 0x89ab_cdef,
            cur_blocksize: 32769,
        },
        "ERRORS.md G6",
    );
}

#[test]
fn ffi_null_child() {
    let Some(path) = std::env::var_os("DIFFERENTIAL_NULL_CHILD_LIBRARY") else {
        return;
    };
    let api = Api::load(Path::new(&path));
    unsafe {
        (api.update_frame_header)(std::ptr::null_mut());
    }
    panic!("update_frame_header unexpectedly returned for a null pointer");
}

#[cfg(unix)]
#[test]
fn error_boundary_g1_null_pointer_termination_matches() {
    use std::os::unix::process::ExitStatusExt;

    let executable = std::env::current_exe().expect("current integration-test executable");
    let invoke = |library: PathBuf| {
        Command::new(&executable)
            .arg("--exact")
            .arg("ffi_null_child")
            .arg("--nocapture")
            .env("DIFFERENTIAL_NULL_CHILD_LIBRARY", library)
            .status()
            .expect("spawn null-pointer child")
    };

    let c_status = invoke(c_library_path());
    let rust_status = invoke(rust_library_path());
    assert!(
        !c_status.success() && !rust_status.success(),
        "both null calls must terminate abnormally: C={c_status:?}, Rust={rust_status:?}"
    );
    assert_eq!(
        c_status.signal(),
        rust_status.signal(),
        "ERRORS.md G1 signal mismatch: C={c_status:?}, Rust={rust_status:?}"
    );
    assert!(
        c_status.signal().is_some(),
        "ERRORS.md G1 expected signal termination: C={c_status:?}"
    );
}
