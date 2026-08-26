use libloading::Library;
use std::env;
use std::ffi::{c_int, c_void};
use std::fs::{self, File};
use std::io::Write;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

type NoiseInternal = unsafe extern "C" fn(f32, f32, f32, c_int, c_int, c_int, u8) -> f32;
type Noise = unsafe extern "C" fn(f32, f32, f32, c_int, c_int, c_int) -> f32;
type NoiseSeed = unsafe extern "C" fn(f32, f32, f32, c_int, c_int, c_int, c_int) -> f32;
type Ridge = unsafe extern "C" fn(f32, f32, f32, f32, f32, f32, c_int) -> f32;
type Fractal = unsafe extern "C" fn(f32, f32, f32, f32, f32, c_int) -> f32;
type Inner = unsafe extern "C" fn(
    c_int,
    f32,
    f32,
    f32,
    c_int,
    c_int,
    c_int,
    c_int,
    f32,
    f32,
    f32,
    c_int,
) -> f32;
type Main = unsafe extern "C" fn() -> c_int;

struct Apis {
    c: Library,
    rust: Library,
}

impl Apis {
    fn load() -> Self {
        let c_path = c_library_path();
        let rust_path = rust_library_path();
        assert!(c_path.is_file(), "missing C library: {}", c_path.display());
        assert!(
            rust_path.is_file(),
            "missing Rust library: {}",
            rust_path.display()
        );
        unsafe {
            Self {
                c: Library::new(c_path).expect("load C shared library"),
                rust: Library::new(rust_path).expect("load Rust shared library"),
            }
        }
    }

    unsafe fn functions<T: Copy>(&self, name: &[u8]) -> (T, T) {
        let c = *self.c.get::<T>(name).expect("resolve C symbol");
        let rust = *self.rust.get::<T>(name).expect("resolve Rust symbol");
        (c, rust)
    }
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    crate_root().join("c_src/build/libdriver.so")
}

fn rust_library_path() -> PathBuf {
    crate_root().join("target/release/libdriver.so")
}

fn assert_float_eq(c: f32, rust: f32, context: &str) {
    assert_eq!(
        c.to_bits(),
        rust.to_bits(),
        "{context}: C={c:?} ({:#010x}), Rust={rust:?} ({:#010x})",
        c.to_bits(),
        rust.to_bits()
    );
}

struct Rng(u64);

impl Rng {
    fn new(row: u64) -> Self {
        Self(0x9e37_79b9_7f4a_7c15 ^ row.wrapping_mul(0xd1b5_4a32_d192_ed03))
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x as u32
    }

    fn index(&mut self, len: usize) -> usize {
        self.next_u32() as usize % len
    }

    fn int(&mut self, low: i32, high: i32) -> i32 {
        low + (self.next_u32() % (high - low + 1) as u32) as i32
    }

    fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / 16_777_216.0
    }

    fn float(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.unit()
    }

    fn positive_fractional(&mut self) -> f32 {
        self.int(0, 30) as f32 + self.int(1, 1023) as f32 / 1024.0
    }

    fn negative_fractional(&mut self) -> f32 {
        -(self.int(0, 30) as f32 + self.int(1, 1023) as f32 / 1024.0)
    }

    fn signed_fractional(&mut self) -> f32 {
        if self.next_u32() & 1 == 0 {
            self.positive_fractional()
        } else {
            self.negative_fractional()
        }
    }
}

fn compare_internal(api: &Apis, xyz: [f32; 3], wraps: [i32; 3], seed: u8, context: &str) {
    unsafe {
        let (c, rust) = api.functions::<NoiseInternal>(b"stb_perlin_noise3_internal\0");
        assert_float_eq(
            c(xyz[0], xyz[1], xyz[2], wraps[0], wraps[1], wraps[2], seed),
            rust(xyz[0], xyz[1], xyz[2], wraps[0], wraps[1], wraps[2], seed),
            context,
        );
    }
}

fn compare_noise(api: &Apis, xyz: [f32; 3], wraps: [i32; 3], context: &str) {
    unsafe {
        let (c, rust) = api.functions::<Noise>(b"stb_perlin_noise3\0");
        assert_float_eq(
            c(xyz[0], xyz[1], xyz[2], wraps[0], wraps[1], wraps[2]),
            rust(xyz[0], xyz[1], xyz[2], wraps[0], wraps[1], wraps[2]),
            context,
        );
    }
}

fn compare_seed(api: &Apis, xyz: [f32; 3], wraps: [i32; 3], seed: i32, context: &str) {
    unsafe {
        let (c, rust) = api.functions::<NoiseSeed>(b"stb_perlin_noise3_seed\0");
        assert_float_eq(
            c(xyz[0], xyz[1], xyz[2], wraps[0], wraps[1], wraps[2], seed),
            rust(xyz[0], xyz[1], xyz[2], wraps[0], wraps[1], wraps[2], seed),
            context,
        );
    }
}

fn compare_ridge(
    api: &Apis,
    xyz: [f32; 3],
    lacunarity: f32,
    gain: f32,
    offset: f32,
    octaves: i32,
    context: &str,
) {
    unsafe {
        let (c, rust) = api.functions::<Ridge>(b"stb_perlin_ridge_noise3\0");
        assert_float_eq(
            c(xyz[0], xyz[1], xyz[2], lacunarity, gain, offset, octaves),
            rust(xyz[0], xyz[1], xyz[2], lacunarity, gain, offset, octaves),
            context,
        );
    }
}

fn compare_fractal(
    api: &Apis,
    symbol: &[u8],
    xyz: [f32; 3],
    lacunarity: f32,
    gain: f32,
    octaves: i32,
    context: &str,
) {
    unsafe {
        let (c, rust) = api.functions::<Fractal>(symbol);
        assert_float_eq(
            c(xyz[0], xyz[1], xyz[2], lacunarity, gain, octaves),
            rust(xyz[0], xyz[1], xyz[2], lacunarity, gain, octaves),
            context,
        );
    }
}

fn compare_nonpow2(api: &Apis, xyz: [f32; 3], wraps: [i32; 3], seed: u8, context: &str) {
    unsafe {
        let (c, rust) = api.functions::<NoiseInternal>(b"stb_perlin_noise3_wrap_nonpow2\0");
        assert_float_eq(
            c(xyz[0], xyz[1], xyz[2], wraps[0], wraps[1], wraps[2], seed),
            rust(xyz[0], xyz[1], xyz[2], wraps[0], wraps[1], wraps[2], seed),
            context,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn compare_inner(
    api: &Apis,
    which: i32,
    xyz: [f32; 3],
    wraps: [i32; 3],
    seed: i32,
    lacunarity: f32,
    gain: f32,
    offset: f32,
    octaves: i32,
    context: &str,
) {
    unsafe {
        let (c, rust) = api.functions::<Inner>(b"inner\0");
        assert_float_eq(
            c(
                which, xyz[0], xyz[1], xyz[2], wraps[0], wraps[1], wraps[2], seed, lacunarity,
                gain, offset, octaves,
            ),
            rust(
                which, xyz[0], xyz[1], xyz[2], wraps[0], wraps[1], wraps[2], seed, lacunarity,
                gain, offset, octaves,
            ),
            context,
        );
    }
}

fn random_xyz(rng: &mut Rng) -> [f32; 3] {
    [
        rng.signed_fractional(),
        rng.signed_fractional(),
        rng.signed_fractional(),
    ]
}

fn run_config(row: u32) {
    let api = Apis::load();
    let mut rng = Rng::new(row as u64);
    let powers = [1, 2, 4, 8, 16, 32, 64, 128, 256];
    let nonpowers = [3, 5, 6, 7, 9, 10, 11, 12, 15, 17, 31, 63];

    match row {
        1 => {
            for i in 0..64 {
                let xyz = [
                    rng.int(-30, 30) as f32,
                    rng.int(-30, 30) as f32,
                    rng.int(-30, 30) as f32,
                ];
                compare_internal(&api, xyz, [0; 3], 0, &format!("row 1 case {i}"));
            }
        }
        2 => {
            for i in 0..64 {
                let xyz = [
                    rng.positive_fractional(),
                    rng.positive_fractional(),
                    rng.positive_fractional(),
                ];
                let seed = rng.int(1, 255) as u8;
                compare_internal(&api, xyz, [0; 3], seed, &format!("row 2 case {i}"));
            }
        }
        3 => {
            for i in 0..64 {
                let xyz = [
                    rng.negative_fractional(),
                    rng.int(-30, 30) as f32,
                    rng.positive_fractional(),
                ];
                let wraps = [1, powers[rng.index(powers.len())], 256];
                compare_internal(
                    &api,
                    xyz,
                    wraps,
                    rng.int(1, 255) as u8,
                    &format!("row 3 case {i}"),
                );
            }
        }
        4 => {
            for i in 0..64 {
                let wraps = [
                    powers[rng.index(powers.len())],
                    powers[rng.index(powers.len())],
                    powers[rng.index(powers.len())],
                ];
                compare_internal(
                    &api,
                    random_xyz(&mut rng),
                    wraps,
                    255,
                    &format!("row 4 case {i}"),
                );
            }
        }
        5 => {
            for i in 0..64 {
                let xyz = [
                    rng.positive_fractional(),
                    rng.positive_fractional(),
                    rng.positive_fractional(),
                ];
                compare_noise(&api, xyz, [0; 3], &format!("row 5 case {i}"));
            }
        }
        6 => {
            for i in 0..64 {
                let wraps = [
                    powers[rng.index(powers.len())],
                    powers[rng.index(powers.len())],
                    powers[rng.index(powers.len())],
                ];
                let xyz = [
                    rng.negative_fractional(),
                    rng.negative_fractional(),
                    rng.negative_fractional(),
                ];
                compare_noise(&api, xyz, wraps, &format!("row 6 case {i}"));
            }
        }
        7 => {
            for i in 0..64 {
                let xyz = [
                    rng.int(-30, 30) as f32,
                    rng.int(-30, 30) as f32,
                    rng.int(-30, 30) as f32,
                ];
                let wraps = if i & 1 == 0 {
                    [1, 256, 1]
                } else {
                    [256, 1, 256]
                };
                compare_noise(&api, xyz, wraps, &format!("row 7 case {i}"));
            }
        }
        8 => {
            for i in 0..64 {
                compare_seed(
                    &api,
                    random_xyz(&mut rng),
                    [0; 3],
                    0,
                    &format!("row 8 case {i}"),
                );
            }
        }
        9 => {
            for i in 0..64 {
                let wraps = [
                    powers[rng.index(powers.len())],
                    powers[rng.index(powers.len())],
                    powers[rng.index(powers.len())],
                ];
                compare_seed(
                    &api,
                    random_xyz(&mut rng),
                    wraps,
                    rng.int(1, 255),
                    &format!("row 9 case {i}"),
                );
            }
        }
        10 => {
            let boundaries = [i32::MIN, -65_537, -257, -1, 256, 257, 65_535, i32::MAX];
            for i in 0..64 {
                let seed = boundaries[rng.index(boundaries.len())];
                compare_seed(
                    &api,
                    random_xyz(&mut rng),
                    [0; 3],
                    seed,
                    &format!("row 10 case {i}"),
                );
            }
        }
        11 | 15 | 19 => {
            for i in 0..64 {
                let octaves = if i & 1 == 0 { 0 } else { -rng.int(1, 1000) };
                let xyz = random_xyz(&mut rng);
                if row == 11 {
                    compare_ridge(
                        &api,
                        xyz,
                        rng.float(0.5, 3.0),
                        rng.float(-1.0, 1.0),
                        rng.float(-2.0, 2.0),
                        octaves,
                        &format!("row 11 case {i}"),
                    );
                } else {
                    let symbol = if row == 15 {
                        b"stb_perlin_fbm_noise3\0".as_slice()
                    } else {
                        b"stb_perlin_turbulence_noise3\0".as_slice()
                    };
                    compare_fractal(
                        &api,
                        symbol,
                        xyz,
                        rng.float(0.5, 3.0),
                        rng.float(-1.0, 1.0),
                        octaves,
                        &format!("row {row} case {i}"),
                    );
                }
            }
        }
        12 | 16 | 20 => {
            for i in 0..64 {
                let xyz = random_xyz(&mut rng);
                if row == 12 {
                    compare_ridge(
                        &api,
                        xyz,
                        rng.float(0.5, 3.0),
                        rng.float(-1.0, 1.0),
                        rng.float(-2.0, 2.0),
                        1,
                        &format!("row 12 case {i}"),
                    );
                } else {
                    let symbol = if row == 16 {
                        b"stb_perlin_fbm_noise3\0".as_slice()
                    } else {
                        b"stb_perlin_turbulence_noise3\0".as_slice()
                    };
                    compare_fractal(
                        &api,
                        symbol,
                        xyz,
                        rng.float(0.5, 3.0),
                        rng.float(-1.0, 1.0),
                        1,
                        &format!("row {row} case {i}"),
                    );
                }
            }
        }
        13 | 17 | 21 => {
            for i in 0..64 {
                let xyz = random_xyz(&mut rng);
                let octaves = rng.int(2, 12);
                if row == 13 {
                    compare_ridge(
                        &api,
                        xyz,
                        rng.float(1.1, 2.2),
                        rng.float(0.2, 0.9),
                        rng.float(-1.0, 2.0),
                        octaves,
                        &format!("row 13 case {i}"),
                    );
                } else {
                    let symbol = if row == 17 {
                        b"stb_perlin_fbm_noise3\0".as_slice()
                    } else {
                        b"stb_perlin_turbulence_noise3\0".as_slice()
                    };
                    compare_fractal(
                        &api,
                        symbol,
                        xyz,
                        rng.float(1.1, 2.2),
                        rng.float(0.2, 0.9),
                        octaves,
                        &format!("row {row} case {i}"),
                    );
                }
            }
        }
        14 | 18 | 22 => {
            for i in 0..24 {
                let xyz = random_xyz(&mut rng);
                let octaves = rng.int(257, 272);
                if row == 14 {
                    compare_ridge(
                        &api,
                        xyz,
                        1.0,
                        rng.float(0.95, 0.995),
                        rng.float(0.5, 1.25),
                        octaves,
                        &format!("row 14 case {i}"),
                    );
                } else {
                    let symbol = if row == 18 {
                        b"stb_perlin_fbm_noise3\0".as_slice()
                    } else {
                        b"stb_perlin_turbulence_noise3\0".as_slice()
                    };
                    compare_fractal(
                        &api,
                        symbol,
                        xyz,
                        1.0,
                        rng.float(0.95, 0.995),
                        octaves,
                        &format!("row {row} case {i}"),
                    );
                }
            }
        }
        23 => {
            for i in 0..64 {
                compare_nonpow2(
                    &api,
                    random_xyz(&mut rng),
                    [0; 3],
                    0,
                    &format!("row 23 case {i}"),
                );
            }
        }
        24 => {
            for i in 0..64 {
                let wraps = match i % 3 {
                    0 => [1, 1, 1],
                    1 => [1, 7, 11],
                    _ => [5, 1, 1],
                };
                compare_nonpow2(
                    &api,
                    random_xyz(&mut rng),
                    wraps,
                    rng.int(0, 255) as u8,
                    &format!("row 24 case {i}"),
                );
            }
        }
        25 => {
            for i in 0..64 {
                let wraps = [
                    nonpowers[rng.index(nonpowers.len())],
                    nonpowers[rng.index(nonpowers.len())],
                    nonpowers[rng.index(nonpowers.len())],
                ];
                compare_nonpow2(
                    &api,
                    random_xyz(&mut rng),
                    wraps,
                    rng.int(1, 255) as u8,
                    &format!("row 25 case {i}"),
                );
            }
        }
        26 => {
            for i in 0..64 {
                let xyz = [
                    rng.negative_fractional(),
                    rng.negative_fractional(),
                    rng.negative_fractional(),
                ];
                compare_nonpow2(
                    &api,
                    xyz,
                    [256; 3],
                    rng.int(0, 255) as u8,
                    &format!("row 26 case {i}"),
                );
            }
        }
        27..=32 => {
            let which = (row - 27) as i32;
            for i in 0..64 {
                let wraps = if which == 5 {
                    [
                        nonpowers[rng.index(nonpowers.len())],
                        nonpowers[rng.index(nonpowers.len())],
                        nonpowers[rng.index(nonpowers.len())],
                    ]
                } else {
                    [
                        powers[rng.index(powers.len())],
                        powers[rng.index(powers.len())],
                        powers[rng.index(powers.len())],
                    ]
                };
                compare_inner(
                    &api,
                    which,
                    random_xyz(&mut rng),
                    wraps,
                    rng.int(-1024, 1024),
                    rng.float(1.1, 2.2),
                    rng.float(0.2, 0.9),
                    rng.float(-1.0, 2.0),
                    rng.int(1, 10),
                    &format!("row {row} case {i}"),
                );
            }
        }
        33..=35 => run_main_config(row, &mut rng),
        _ => panic!("unknown CONFIGS.md row {row}"),
    }
}

fn main_tokens(rng: &mut Rng, which: i32) -> Vec<String> {
    vec![
        which.to_string(),
        format!("{:?}", rng.signed_fractional()),
        format!("{:?}", rng.signed_fractional()),
        format!("{:?}", rng.signed_fractional()),
        [0, 1, 2, 4, 8, 16, 32, 64, 128, 256][rng.index(10)].to_string(),
        [0, 1, 2, 4, 8, 16, 32, 64, 128, 256][rng.index(10)].to_string(),
        [0, 1, 2, 4, 8, 16, 32, 64, 128, 256][rng.index(10)].to_string(),
        rng.int(-1024, 1024).to_string(),
        format!("{:?}", rng.float(1.1, 2.2)),
        format!("{:?}", rng.float(0.2, 0.9)),
        format!("{:?}", rng.float(-1.0, 2.0)),
        rng.int(1, 10).to_string(),
    ]
}

fn run_main_config(row: u32, rng: &mut Rng) {
    let cases = if row == 35 { 24 } else { 16 };
    for i in 0..cases {
        let tokens = main_tokens(rng, i as i32 % 6);
        let input = match row {
            33 => format!("{}\n", tokens.join(" ")),
            34 => {
                let separators = [" ", "\n", "\t", " \n\t "];
                let mut text = String::new();
                for (index, token) in tokens.iter().enumerate() {
                    if index != 0 {
                        text.push_str(separators[rng.index(separators.len())]);
                    }
                    text.push_str(token);
                }
                text.push('\n');
                text
            }
            35 => {
                let count = i as usize % tokens.len();
                if count == 0 {
                    String::new()
                } else {
                    format!("{}\n", tokens[..count].join(" "))
                }
            }
            _ => unreachable!(),
        };
        let c = invoke_main(&c_library_path(), input.as_bytes());
        let rust = invoke_main(&rust_library_path(), input.as_bytes());
        assert_eq!(
            c, rust,
            "CONFIGS.md row {row}, main case {i}, input {input:?}"
        );
    }
}

static CHILD_COUNTER: AtomicU64 = AtomicU64::new(0);

fn invoke_main(library: &Path, input: &[u8]) -> (i32, Vec<u8>) {
    let id = CHILD_COUNTER.fetch_add(1, Ordering::Relaxed);
    let stem = format!("driver-diff-{}-{id}", std::process::id());
    let output_path = env::temp_dir().join(format!("{stem}.stdout"));
    let result_path = env::temp_dir().join(format!("{stem}.result"));
    let mut child = Command::new(env::current_exe().expect("current test executable"))
        .arg("--exact")
        .arg("main_child_entry")
        .arg("--nocapture")
        .env("DRIVER_DIFF_CHILD_LIBRARY", library)
        .env("DRIVER_DIFF_CHILD_OUTPUT", &output_path)
        .env("DRIVER_DIFF_CHILD_RESULT", &result_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn main child");
    child
        .stdin
        .as_mut()
        .expect("child stdin")
        .write_all(input)
        .expect("write child input");
    drop(child.stdin.take());
    let status = child.wait_with_output().expect("wait for main child");
    assert!(
        status.status.success(),
        "main child failed for {}: {}",
        library.display(),
        String::from_utf8_lossy(&status.stderr)
    );
    let output = fs::read(&output_path).expect("read captured main stdout");
    let result = fs::read_to_string(&result_path)
        .expect("read main result")
        .parse()
        .expect("parse main result");
    let _ = fs::remove_file(output_path);
    let _ = fs::remove_file(result_path);
    (result, output)
}

unsafe extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

#[test]
fn main_child_entry() {
    let Ok(library_path) = env::var("DRIVER_DIFF_CHILD_LIBRARY") else {
        return;
    };
    let output_path = env::var("DRIVER_DIFF_CHILD_OUTPUT").expect("child output path");
    let result_path = env::var("DRIVER_DIFF_CHILD_RESULT").expect("child result path");
    let output = File::create(output_path).expect("create child output");

    unsafe {
        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0, "dup stdout");
        assert!(dup2(output.as_raw_fd(), 1) >= 0, "redirect stdout");
        let library = Library::new(library_path).expect("load child library");
        let main = *library.get::<Main>(b"main\0").expect("resolve main");
        let result = main();
        fflush(std::ptr::null_mut());
        assert!(dup2(saved_stdout, 1) >= 0, "restore stdout");
        close(saved_stdout);
        fs::write(result_path, result.to_string()).expect("write child result");
    }
}

macro_rules! config_tests {
    ($($name:ident => $row:literal),+ $(,)?) => {
        $(
            #[test]
            fn $name() {
                run_config($row);
            }
        )+
    };
}

config_tests!(
    config_01_internal_integer_unwrapped => 1,
    config_02_internal_fractional_seeded => 2,
    config_03_internal_mixed_floor_and_wraps => 3,
    config_04_internal_wrap_and_seed_boundaries => 4,
    config_05_noise_unwrapped => 5,
    config_06_noise_negative_fractional_wrapped => 6,
    config_07_noise_integer_wrap_boundaries => 7,
    config_08_seeded_noise_seed_zero => 8,
    config_09_seeded_noise_nonzero => 9,
    config_10_seeded_noise_low_byte_truncation => 10,
    config_11_ridge_nonpositive_octaves => 11,
    config_12_ridge_one_octave => 12,
    config_13_ridge_many_octaves => 13,
    config_14_ridge_seed_rollover => 14,
    config_15_fbm_nonpositive_octaves => 15,
    config_16_fbm_one_octave => 16,
    config_17_fbm_many_octaves => 17,
    config_18_fbm_seed_rollover => 18,
    config_19_turbulence_nonpositive_octaves => 19,
    config_20_turbulence_one_octave => 20,
    config_21_turbulence_many_octaves => 21,
    config_22_turbulence_seed_rollover => 22,
    config_23_nonpow2_zero_wrap_fallback => 23,
    config_24_nonpow2_wrap_one => 24,
    config_25_nonpow2_positive_wraps => 25,
    config_26_nonpow2_negative_remainder_correction => 26,
    config_27_inner_noise_dispatch => 27,
    config_28_inner_seeded_dispatch => 28,
    config_29_inner_ridge_dispatch => 29,
    config_30_inner_fbm_dispatch => 30,
    config_31_inner_turbulence_dispatch => 31,
    config_32_inner_nonpow2_dispatch => 32,
    config_33_main_complete_input => 33,
    config_34_main_multiline_whitespace => 34,
    config_35_main_partial_input => 35,
);

#[test]
fn error_01_inner_rejects_unknown_selector_with_identical_nan() {
    let api = Apis::load();
    let mut rng = Rng::new(0xe001);
    let boundary_selectors = [i32::MIN, -1024, -2, -1, 6, 7, 1024, i32::MAX];
    for i in 0..128 {
        let which = boundary_selectors[rng.index(boundary_selectors.len())];
        compare_inner(
            &api,
            which,
            random_xyz(&mut rng),
            [0; 3],
            rng.next_u32() as i32,
            rng.float(0.5, 3.0),
            rng.float(-1.0, 1.0),
            rng.float(-2.0, 2.0),
            rng.int(-1000, 1000),
            &format!("ERRORS.md row 1 case {i}"),
        );
    }
}

#[test]
fn documented_invalid_standard_wrap_is_equally_permissive() {
    let api = Apis::load();
    let mut rng = Rng::new(0xe002);
    for i in 0..64 {
        compare_noise(
            &api,
            random_xyz(&mut rng),
            [3, 6, 12],
            &format!("non-power-of-two standard wrap case {i}"),
        );
    }
}
