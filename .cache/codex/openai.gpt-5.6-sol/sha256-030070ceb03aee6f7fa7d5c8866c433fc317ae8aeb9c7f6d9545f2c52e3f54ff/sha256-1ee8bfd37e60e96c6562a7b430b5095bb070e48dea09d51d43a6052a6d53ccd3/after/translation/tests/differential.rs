use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Tflac {
    samplerate: u32,
    channels: u32,
    bitdepth: u32,
    channel_mode: u8,
    frame_header: u32,
    cur_blocksize: u32,
}

type UpdateFrameHeader = unsafe extern "C" fn(*mut Tflac);

const BLOCK_CASES: [u32; 13] = [
    192, 576, 1152, 2304, 4608, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768,
];
const RATE_CASES: [u32; 11] = [
    882000, 176400, 192000, 8000, 16000, 22050, 24000, 32000, 44100, 48000, 96000,
];
const DEPTH_CASES: [u32; 6] = [8, 12, 16, 20, 24, 32];

const BLOCK_CLASS_COUNT: usize = 15;
const RATE_CLASS_COUNT: usize = 17;
const MODE_CLASS_COUNT: usize = 4;
const DEPTH_CLASS_COUNT: usize = 7;
const INPUTS_PER_ROW: usize = 32;
const CONFIG_ROW_COUNT: usize =
    BLOCK_CLASS_COUNT * RATE_CLASS_COUNT * MODE_CLASS_COUNT * DEPTH_CLASS_COUNT;

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        (value >> 16) as u32
    }
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libharvest-work-7NfxTl.so")
}

fn rust_library_path() -> PathBuf {
    let executable = std::env::current_exe().expect("get current test executable");
    let profile_dir = executable
        .parent()
        .and_then(Path::parent)
        .expect("test executable is under target/<profile>/deps");
    let direct = profile_dir.join("libupdate_frame_header_lib.so");
    if direct.is_file() {
        return direct;
    }

    let deps = profile_dir
        .join("deps")
        .join("libupdate_frame_header_lib.so");
    if deps.is_file() {
        return deps;
    }

    let release =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libupdate_frame_header_lib.so");
    assert!(
        release.is_file(),
        "Rust cdylib not found at {}, {}, or {}",
        direct.display(),
        deps.display(),
        release.display()
    );
    release
}

fn is_block_case(value: u32) -> bool {
    BLOCK_CASES.contains(&value)
}

fn is_rate_case(value: u32) -> bool {
    RATE_CASES.contains(&value)
}

fn block_value(class: usize, iteration: usize, rng: &mut Rng) -> u32 {
    if class < BLOCK_CASES.len() {
        return BLOCK_CASES[class];
    }

    if class == 13 {
        let boundaries = [0, 1, 191, 193, 255];
        if iteration < boundaries.len() {
            return boundaries[iteration];
        }
        loop {
            let value = rng.next_u32() % 257;
            if !is_block_case(value) {
                return value;
            }
        }
    }

    let boundaries = [257, u32::MAX];
    if iteration < boundaries.len() {
        return boundaries[iteration];
    }
    loop {
        let value = rng.next_u32();
        if value > 256 && !is_block_case(value) {
            return value;
        }
    }
}

fn rate_value(class: usize, iteration: usize, rng: &mut Rng) -> u32 {
    if class < RATE_CASES.len() {
        return RATE_CASES[class];
    }

    loop {
        let value = match class {
            11 => {
                if iteration == 0 {
                    0
                } else {
                    (rng.next_u32() % 256) * 1000
                }
            }
            12 => {
                let quotient = if iteration == 0 {
                    256
                } else {
                    256 + rng.next_u32() % (4_294_967 - 256 + 1)
                };
                quotient * 1000
            }
            13 => match iteration {
                0 => 1,
                1 => 65_535,
                _ => rng.next_u32() % 65_536,
            },
            14 => {
                let quotient = match iteration {
                    0 => 6_554,
                    1 => 65_535,
                    _ => 6_554 + rng.next_u32() % (65_535 - 6_554 + 1),
                };
                quotient * 10
            }
            15 => match iteration {
                0 => 65_537,
                1 => u32::MAX,
                _ => rng.next_u32().max(65_536),
            },
            16 => {
                let quotient = match iteration {
                    0 => 65_536,
                    1 => 429_496_729,
                    _ => 65_536 + rng.next_u32() % (429_496_729 - 65_536 + 1),
                };
                quotient * 10
            }
            _ => unreachable!("unknown sample-rate class"),
        };

        let matches = match class {
            11 => value % 1000 == 0 && value / 1000 < 256 && !is_rate_case(value),
            12 => value % 1000 == 0 && value / 1000 >= 256 && !is_rate_case(value),
            13 => value % 1000 != 0 && value < 65_536 && !is_rate_case(value),
            14 => {
                value >= 65_536
                    && value % 1000 != 0
                    && value % 10 == 0
                    && value / 10 < 65_536
                    && !is_rate_case(value)
            }
            15 => value >= 65_536 && value % 1000 != 0 && value % 10 != 0,
            16 => value % 1000 != 0 && value % 10 == 0 && value / 10 >= 65_536,
            _ => unreachable!(),
        };
        if matches {
            return value;
        }
    }
}

fn channel_mode_value(class: usize, iteration: usize, rng: &mut Rng) -> u8 {
    match iteration {
        0 => class as u8,
        1 => 252 + class as u8,
        _ => ((rng.next_u32() % 64) * 4 + class as u32) as u8,
    }
}

fn channels_value(iteration: usize, rng: &mut Rng) -> u32 {
    match iteration {
        0 => 0,
        1 => 1,
        2 => u32::MAX,
        _ => rng.next_u32(),
    }
}

fn depth_value(class: usize, iteration: usize, rng: &mut Rng) -> u32 {
    if class < DEPTH_CASES.len() {
        return DEPTH_CASES[class];
    }

    match iteration {
        0 => 0,
        1 => u32::MAX,
        _ => loop {
            let value = rng.next_u32();
            if !DEPTH_CASES.contains(&value) {
                return value;
            }
        },
    }
}

unsafe fn resolve_update(library: &Library) -> Symbol<'_, UpdateFrameHeader> {
    unsafe {
        library
            .get(b"update_frame_header\0")
            .expect("resolve update_frame_header")
    }
}

#[test]
fn all_configuration_rows_match_byte_for_byte() {
    assert_eq!(CONFIG_ROW_COUNT, 7_140);

    let c_library = unsafe { Library::new(c_library_path()) }.expect("load C shared library");
    let rust_library =
        unsafe { Library::new(rust_library_path()) }.expect("load Rust shared library");
    let c_update = unsafe { resolve_update(&c_library) };
    let rust_update = unsafe { resolve_update(&rust_library) };
    let mut rng = Rng::new(0x6d5a_56da_4b7c_2f91);
    let mut row = 0;

    for block_class in 0..BLOCK_CLASS_COUNT {
        for rate_class in 0..RATE_CLASS_COUNT {
            for mode_class in 0..MODE_CLASS_COUNT {
                for depth_class in 0..DEPTH_CLASS_COUNT {
                    row += 1;
                    for iteration in 0..INPUTS_PER_ROW {
                        let input = Tflac {
                            samplerate: rate_value(rate_class, iteration, &mut rng),
                            channels: channels_value(iteration, &mut rng),
                            bitdepth: depth_value(depth_class, iteration, &mut rng),
                            channel_mode: channel_mode_value(mode_class, iteration, &mut rng),
                            frame_header: rng.next_u32(),
                            cur_blocksize: block_value(block_class, iteration, &mut rng),
                        };
                        let mut c_output = input;
                        let mut rust_output = input;

                        unsafe {
                            c_update(&mut c_output);
                            rust_update(&mut rust_output);
                        }

                        assert_eq!(
                            c_output.frame_header.to_ne_bytes(),
                            rust_output.frame_header.to_ne_bytes(),
                            "frame-header bytes differ at CONFIGS.md row {row}, \
                             iteration {iteration}, input {input:?}"
                        );
                        assert_eq!(
                            c_output, rust_output,
                            "struct fields differ at CONFIGS.md row {row}, \
                             iteration {iteration}, input {input:?}"
                        );
                    }
                }
            }
        }
    }

    assert_eq!(row, CONFIG_ROW_COUNT);
}

#[test]
fn null_pointer_child() {
    let Some(path) = std::env::var_os("TFLAC_NULL_PROBE_LIBRARY") else {
        return;
    };

    let library = unsafe { Library::new(path) }.expect("load null-probe library");
    let update = unsafe { resolve_update(&library) };
    unsafe {
        update(std::ptr::null_mut());
    }
}

fn run_null_probe(path: &Path) -> ExitStatus {
    Command::new(std::env::current_exe().expect("get current test executable"))
        .args(["--exact", "null_pointer_child", "--nocapture"])
        .env("TFLAC_NULL_PROBE_LIBRARY", path)
        .status()
        .expect("run isolated null-pointer probe")
}

#[test]
#[cfg(unix)]
fn null_pointer_process_result_matches() {
    let c_status = run_null_probe(&c_library_path());
    let rust_status = run_null_probe(&rust_library_path());

    assert!(
        !c_status.success(),
        "C null-pointer probe unexpectedly returned"
    );
    assert!(
        !rust_status.success(),
        "Rust null-pointer probe unexpectedly returned"
    );
    assert_eq!(
        c_status.signal(),
        rust_status.signal(),
        "null-pointer probes terminated with different signals: \
         C={c_status:?}, Rust={rust_status:?}"
    );
}
