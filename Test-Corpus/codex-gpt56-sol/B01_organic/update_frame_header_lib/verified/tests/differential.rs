use libloading::Library;
use std::env;
use std::ffi::OsStr;
use std::mem::{align_of, offset_of, size_of};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

#[repr(C)]
struct Tflac {
    samplerate: u32,
    channels: u32,
    bitdepth: u32,
    channel_mode: u8,
    frame_header: u32,
    cur_blocksize: u32,
}

#[repr(C, align(8))]
#[derive(Clone)]
struct Storage([u8; size_of::<Tflac>()]);

type UpdateFrameHeader = unsafe extern "C" fn(*mut Tflac);

struct LoadedLibrary {
    _library: Library,
    update_frame_header: UpdateFrameHeader,
}

impl LoadedLibrary {
    unsafe fn open(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let update_frame_header = unsafe {
            *library
                .get::<UpdateFrameHeader>(b"update_frame_header\0")
                .unwrap_or_else(|error| {
                    panic!(
                        "failed to resolve update_frame_header from {}: {error}",
                        path.display()
                    )
                })
        };
        Self {
            _library: library,
            update_frame_header,
        }
    }
}

#[derive(Clone, Copy)]
struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.0;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("c_src/build/libtranslated_rust.so")
}

fn rust_library_path() -> PathBuf {
    let executable = env::current_exe().expect("current test executable path");
    let deps = executable.parent().expect("test executable parent");
    let direct_candidates = [
        deps.join("libupdate_frame_header_lib.so"),
        deps.parent()
            .expect("target profile directory")
            .join("libupdate_frame_header_lib.so"),
    ];

    if let Some(path) = direct_candidates.into_iter().find(|path| path.is_file()) {
        return path;
    }

    let mut candidates = std::fs::read_dir(deps)
        .expect("read target deps directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(OsStr::to_str)
                .is_some_and(|name| {
                    name.starts_with("libupdate_frame_header_lib") && name.ends_with(".so")
                })
        })
        .collect::<Vec<_>>();
    candidates.sort();
    candidates
        .pop()
        .unwrap_or_else(|| panic!("Rust cdylib not found beside {}", executable.display()))
}

fn assert_abi_layout() {
    assert_eq!(size_of::<Tflac>(), 24);
    assert_eq!(align_of::<Tflac>(), 4);
    assert_eq!(offset_of!(Tflac, samplerate), 0);
    assert_eq!(offset_of!(Tflac, channels), 4);
    assert_eq!(offset_of!(Tflac, bitdepth), 8);
    assert_eq!(offset_of!(Tflac, channel_mode), 12);
    assert_eq!(offset_of!(Tflac, frame_header), 16);
    assert_eq!(offset_of!(Tflac, cur_blocksize), 20);
}

fn initialized_storage(
    rng: &mut SplitMix64,
    samplerate: u32,
    channels: u32,
    bitdepth: u32,
    channel_mode: u8,
    cur_blocksize: u32,
) -> Storage {
    let mut storage = Storage([0; size_of::<Tflac>()]);
    for chunk in storage.0.chunks_mut(8) {
        let random = rng.next_u64().to_ne_bytes();
        chunk.copy_from_slice(&random[..chunk.len()]);
    }

    let pointer = storage.0.as_mut_ptr();
    unsafe {
        pointer.cast::<u32>().write(samplerate);
        pointer.add(4).cast::<u32>().write(channels);
        pointer.add(8).cast::<u32>().write(bitdepth);
        pointer.add(12).write(channel_mode);
        pointer.add(16).cast::<u32>().write(rng.next_u32());
        pointer.add(20).cast::<u32>().write(cur_blocksize);
    }
    storage
}

fn compare_one(c: &LoadedLibrary, rust: &LoadedLibrary, input: Storage, row: usize, sample: usize) {
    let mut c_value = input.clone();
    let mut rust_value = input;
    unsafe {
        (c.update_frame_header)(c_value.0.as_mut_ptr().cast());
        (rust.update_frame_header)(rust_value.0.as_mut_ptr().cast());
    }
    assert_eq!(
        c_value.0, rust_value.0,
        "byte mismatch in CONFIGS.md row C{row}, sample {sample}"
    );
}

const EXACT_BLOCKS: [u32; 13] = [
    192, 576, 1152, 2304, 4608, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768,
];

fn block_for_class(class: usize, sample: usize, rng: &mut SplitMix64) -> u32 {
    if class < EXACT_BLOCKS.len() {
        return EXACT_BLOCKS[class];
    }
    if class == 13 {
        let boundary = [0, 1, 191, 193, 255];
        if sample < boundary.len() {
            return boundary[sample];
        }
        loop {
            let value = rng.next_u32() % 257;
            if value != 192 && value != 256 {
                return value;
            }
        }
    }

    let boundary = [u32::MAX, 257, 32769];
    if sample < boundary.len() {
        return boundary[sample];
    }
    loop {
        let value = rng.next_u32();
        if value > 256 && !EXACT_BLOCKS.contains(&value) {
            return value;
        }
    }
}

const EXACT_RATES: [u32; 11] = [
    882000, 176400, 192000, 8000, 16000, 22050, 24000, 32000, 44100, 48000, 96000,
];

fn rate_for_class(class: usize, sample: usize, rng: &mut SplitMix64) -> u32 {
    if class < EXACT_RATES.len() {
        return EXACT_RATES[class];
    }

    loop {
        let value = match class {
            11 => {
                let boundaries = [0, 1000, 255000];
                if sample < boundaries.len() {
                    boundaries[sample]
                } else {
                    (rng.next_u32() % 256) * 1000
                }
            }
            12 => {
                let boundaries = [256000, 65536000, 4_294_967_000];
                if sample < boundaries.len() {
                    boundaries[sample]
                } else {
                    (256 + rng.next_u32() % (4_294_967 - 256 + 1)) * 1000
                }
            }
            13 => {
                let boundaries = [1, 999, 65535];
                if sample < boundaries.len() {
                    boundaries[sample]
                } else {
                    rng.next_u32() % 65536
                }
            }
            14 => {
                let boundaries = [65540, 655350];
                if sample < boundaries.len() {
                    boundaries[sample]
                } else {
                    (6554 + rng.next_u32() % (65535 - 6554 + 1)) * 10
                }
            }
            15 => {
                let boundaries = [655360, 4_294_967_290];
                if sample < boundaries.len() {
                    boundaries[sample]
                } else {
                    (65536 + rng.next_u32() % (429_496_729 - 65536 + 1)) * 10
                }
            }
            16 => {
                let boundaries = [65536, 65537, u32::MAX];
                if sample < boundaries.len() {
                    boundaries[sample]
                } else {
                    rng.next_u32()
                }
            }
            _ => unreachable!("sample-rate class {class}"),
        };

        let belongs = match class {
            11 => value % 1000 == 0 && value / 1000 < 256,
            12 => value % 1000 == 0 && value / 1000 >= 256,
            13 => value % 1000 != 0 && value < 65536,
            14 => value % 1000 != 0 && value >= 65536 && value % 10 == 0 && value / 10 < 65536,
            15 => value % 1000 != 0 && value % 10 == 0 && value / 10 >= 65536,
            16 => value % 1000 != 0 && value >= 65536 && value % 10 != 0,
            _ => false,
        };
        if belongs && !EXACT_RATES.contains(&value) {
            return value;
        }
    }
}

fn mode_for_class(class: usize, sample: usize, rng: &mut SplitMix64) -> u8 {
    let maximum = 255 - ((255 - class) % 4);
    if sample == 0 {
        class as u8
    } else if sample == 1 {
        maximum as u8
    } else {
        (class + 4 * (rng.next_u32() as usize % ((maximum - class) / 4 + 1))) as u8
    }
}

const EXACT_DEPTHS: [u32; 6] = [8, 12, 16, 20, 24, 32];

fn depth_for_class(class: usize, sample: usize, rng: &mut SplitMix64) -> u32 {
    if class < EXACT_DEPTHS.len() {
        return EXACT_DEPTHS[class];
    }
    let boundaries = [0, 1, u32::MAX];
    if sample < boundaries.len() {
        return boundaries[sample];
    }
    loop {
        let value = rng.next_u32();
        if !EXACT_DEPTHS.contains(&value) {
            return value;
        }
    }
}

fn channels_for_sample(sample: usize, rng: &mut SplitMix64) -> u32 {
    match sample {
        0 => 0,
        1 => u32::MAX,
        2 => 1,
        3 => 8,
        _ => rng.next_u32(),
    }
}

#[test]
fn every_configuration_row_matches() {
    assert_abi_layout();
    let documented_rows = include_str!("../CONFIGS.md")
        .lines()
        .filter(|line| line.starts_with("| C"))
        .count();
    assert_eq!(documented_rows, 7_140);

    let c = unsafe { LoadedLibrary::open(&c_library_path()) };
    let rust = unsafe { LoadedLibrary::open(&rust_library_path()) };
    let mut row = 0;

    for block_class in 0..15 {
        for rate_class in 0..17 {
            for mode_class in 0..4 {
                for depth_class in 0..7 {
                    row += 1;
                    for sample in 0..16 {
                        let seed = 0x5eed_c0de_d15c_a11u64
                            ^ (row as u64).wrapping_mul(0xd6e8_feb8_6659_fd93)
                            ^ sample as u64;
                        let mut rng = SplitMix64(seed);
                        let block = block_for_class(block_class, sample, &mut rng);
                        let rate = rate_for_class(rate_class, sample, &mut rng);
                        let mode = mode_for_class(mode_class, sample, &mut rng);
                        let depth = depth_for_class(depth_class, sample, &mut rng);
                        let channels = channels_for_sample(sample, &mut rng);
                        let input =
                            initialized_storage(&mut rng, rate, channels, depth, mode, block);
                        compare_one(&c, &rust, input, row, sample);
                    }
                }
            }
        }
    }
    assert_eq!(row, documented_rows);
}

#[test]
fn generic_scalar_boundaries_and_all_mode_bytes_match() {
    assert_abi_layout();
    let c = unsafe { LoadedLibrary::open(&c_library_path()) };
    let rust = unsafe { LoadedLibrary::open(&rust_library_path()) };

    for (index, mode) in (u8::MIN..=u8::MAX).enumerate() {
        let mut rng = SplitMix64(0xb0ad_a7e5 ^ mode as u64);
        let scalar = if index % 2 == 0 { u32::MIN } else { u32::MAX };
        let input = initialized_storage(&mut rng, scalar, scalar, scalar, mode, scalar);
        compare_one(&c, &rust, input, 0, index);
    }
}

fn run_null_child(library: &Path) -> ExitStatus {
    Command::new(env::current_exe().expect("current test executable"))
        .args(["--exact", "null_pointer_child", "--nocapture"])
        .env("DIFFERENTIAL_NULL_LIBRARY", library)
        .status()
        .unwrap_or_else(|error| panic!("failed to run null-pointer child: {error}"))
}

#[cfg(unix)]
#[test]
fn null_pointer_observed_behavior_matches() {
    use std::os::unix::process::ExitStatusExt;

    let c_status = run_null_child(&c_library_path());
    let rust_status = run_null_child(&rust_library_path());
    assert_eq!(
        c_status.signal(),
        rust_status.signal(),
        "C status {c_status:?}, Rust status {rust_status:?}"
    );
    assert!(
        c_status.signal().is_some(),
        "the reference C call unexpectedly survived: {c_status:?}"
    );
}

#[test]
fn null_pointer_child() {
    let Some(path) = env::var_os("DIFFERENTIAL_NULL_LIBRARY") else {
        return;
    };
    let library = unsafe { LoadedLibrary::open(Path::new(&path)) };
    unsafe {
        (library.update_frame_header)(std::ptr::null_mut());
    }
}
