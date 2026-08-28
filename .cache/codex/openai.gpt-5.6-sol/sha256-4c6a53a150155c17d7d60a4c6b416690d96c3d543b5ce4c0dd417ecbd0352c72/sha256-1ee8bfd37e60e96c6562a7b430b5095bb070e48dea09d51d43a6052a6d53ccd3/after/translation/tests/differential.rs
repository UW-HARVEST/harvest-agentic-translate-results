use libloading::Library;
use std::ffi::{CString, c_char, c_double, c_int, c_long, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;

type ClassifyMode = unsafe extern "C" fn(*const c_char) -> c_int;
type ApplyMultiplier = unsafe extern "C" fn(c_int, c_int) -> c_int;
type ConvertDouble = unsafe extern "C" fn(c_double) -> c_int;
type GetModifiedTime = unsafe extern "C" fn(c_int, c_int) -> c_long;
type HashTimeValue = unsafe extern "C" fn(c_long) -> c_int;
type Modeselect = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
}

struct Api {
    _library: Library,
    classify_mode: ClassifyMode,
    apply_multiplier: ApplyMultiplier,
    convert_time_factor: ConvertDouble,
    convert_negative_overflow: ConvertDouble,
    get_modified_time: GetModifiedTime,
    hash_time_value: HashTimeValue,
    modeselect: Modeselect,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }.unwrap();
        let classify_mode = unsafe { *library.get(b"classify_mode\0").unwrap() };
        let apply_multiplier = unsafe { *library.get(b"apply_multiplier\0").unwrap() };
        let convert_time_factor = unsafe { *library.get(b"convert_time_factor\0").unwrap() };
        let convert_negative_overflow =
            unsafe { *library.get(b"convert_negative_overflow\0").unwrap() };
        let get_modified_time = unsafe { *library.get(b"get_modified_time\0").unwrap() };
        let hash_time_value = unsafe { *library.get(b"hash_time_value\0").unwrap() };
        let modeselect = unsafe { *library.get(b"modeselect\0").unwrap() };

        Self {
            _library: library,
            classify_mode,
            apply_multiplier,
            convert_time_factor,
            convert_negative_overflow,
            get_modified_time,
            hash_time_value,
            modeselect,
        }
    }
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u64() as i32
    }

    fn range(&mut self, upper_exclusive: u32) -> i32 {
        (self.next_u64() % u64::from(upper_exclusive)) as i32
    }
}

fn paths() -> (PathBuf, PathBuf) {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    (
        crate_dir.join("../c_src/build/libharvest-work-pC0BvO.so"),
        crate_dir.join("target/release/libmodeselect_lib.so"),
    )
}

fn load_apis() -> (Api, Api) {
    let (c_path, rust_path) = paths();
    assert!(
        c_path.is_file(),
        "missing C shared object: {}",
        c_path.display()
    );
    assert!(
        rust_path.is_file(),
        "missing Rust shared object: {}; run cargo build --release first",
        rust_path.display()
    );
    unsafe { (Api::load(&c_path), Api::load(&rust_path)) }
}

fn compare_unary_double(
    row: usize,
    c_function: ConvertDouble,
    rust_function: ConvertDouble,
    values: impl IntoIterator<Item = f64>,
) {
    for value in values {
        let c_result = unsafe { c_function(value) };
        let rust_result = unsafe { rust_function(value) };
        assert_eq!(
            rust_result, c_result,
            "CONFIGS.md row {row}: input {value:?}"
        );
    }
}

fn capture_stdout<T>(function: impl FnOnce() -> T) -> (T, Vec<u8>) {
    let mut fds = [-1; 2];
    unsafe {
        assert_eq!(pipe(fds.as_mut_ptr()), 0);
        assert_eq!(fflush(std::ptr::null_mut()), 0);
    }
    let saved_stdout = unsafe { dup(1) };
    assert!(saved_stdout >= 0);
    unsafe {
        assert_eq!(dup2(fds[1], 1), 1);
        assert_eq!(close(fds[1]), 0);
    }

    let result = function();

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, 1), 1);
        assert_eq!(close(saved_stdout), 0);
    }
    let mut output = Vec::new();
    unsafe { File::from_raw_fd(fds[0]) }
        .read_to_end(&mut output)
        .unwrap();
    (result, output)
}

#[test]
fn valid_path_differential() {
    const CASES: usize = 128;
    let (c, rust) = load_apis();
    let mut rng = Rng::new(0x6D6F_6465_7365_6C65);
    let mut covered = [false; 64];

    // CONFIGS.md rows 1-5: every strcmp branch and randomized other strings.
    for (row, mode) in ["standard", "enhanced", "turbo", "extreme"]
        .into_iter()
        .enumerate()
    {
        for _ in 0..CASES {
            let value = CString::new(mode).unwrap();
            assert_eq!(
                unsafe { (rust.classify_mode)(value.as_ptr()) },
                unsafe { (c.classify_mode)(value.as_ptr()) },
                "CONFIGS.md row {}",
                row + 1
            );
        }
        covered[row + 1] = true;
    }
    for _ in 0..CASES {
        let length = rng.range(31) as usize;
        let bytes: Vec<u8> = (0..length).map(|_| b'a' + rng.range(26) as u8).collect();
        let value = CString::new(bytes).unwrap();
        assert_eq!(
            unsafe { (rust.classify_mode)(value.as_ptr()) },
            unsafe { (c.classify_mode)(value.as_ptr()) },
            "CONFIGS.md row 5: {value:?}"
        );
    }
    covered[5] = true;

    // CONFIGS.md rows 6-11: all switch outcomes.
    for level in 0..=4 {
        let row = 6 + level as usize;
        for _ in 0..CASES {
            let base = rng.range(2_000_000_000) - 1_000_000_000;
            assert_eq!(
                unsafe { (rust.apply_multiplier)(base, level) },
                unsafe { (c.apply_multiplier)(base, level) },
                "CONFIGS.md row {row}: base={base}, level={level}"
            );
        }
        covered[row] = true;
    }
    for case in 0..CASES {
        let level = match case {
            0 => -1,
            1 => 5,
            2 => c_int::MIN,
            3 => c_int::MAX,
            _ if rng.next_u64() & 1 == 0 => -(rng.range(100_000) + 1),
            _ => rng.range(100_000) + 5,
        };
        let base = rng.next_i32();
        assert_eq!(
            unsafe { (rust.apply_multiplier)(base, level) },
            unsafe { (c.apply_multiplier)(base, level) },
            "CONFIGS.md row 11: base={base}, level={level}"
        );
    }
    covered[11] = true;

    // CONFIGS.md rows 12-18: factor * 1e12 conversion classes.
    compare_unary_double(
        12,
        c.convert_time_factor,
        rust.convert_time_factor,
        (0..CASES).map(|i| if i % 2 == 0 { 0.0 } else { -0.0 }),
    );
    covered[12] = true;
    compare_unary_double(
        13,
        c.convert_time_factor,
        rust.convert_time_factor,
        (0..CASES).map(|_| (f64::from(rng.range(2_000_000_000)) + 0.75) / 1e12),
    );
    covered[13] = true;
    compare_unary_double(
        14,
        c.convert_time_factor,
        rust.convert_time_factor,
        (0..CASES).map(|_| -(f64::from(rng.range(2_000_000_000)) + 0.75) / 1e12),
    );
    covered[14] = true;
    compare_unary_double(
        15,
        c.convert_time_factor,
        rust.convert_time_factor,
        (0..CASES).map(|_| -2147483648.0 / 1e12),
    );
    covered[15] = true;
    compare_unary_double(
        16,
        c.convert_time_factor,
        rust.convert_time_factor,
        (0..CASES).map(|i| (2147483647.0 + (i as f64 % 10.0) / 10.0) / 1e12),
    );
    covered[16] = true;
    compare_unary_double(
        17,
        c.convert_time_factor,
        rust.convert_time_factor,
        (0..CASES).map(|_| {
            let magnitude = 2147483649.0 + f64::from(rng.range(1_000_000_000));
            magnitude.copysign(if rng.next_u64() & 1 == 0 { 1.0 } else { -1.0 }) / 1e12
        }),
    );
    covered[17] = true;
    let non_finite = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY];
    compare_unary_double(
        18,
        c.convert_time_factor,
        rust.convert_time_factor,
        (0..CASES).map(|_| non_finite[rng.range(3) as usize]),
    );
    covered[18] = true;

    // CONFIGS.md rows 19-25: value * -1e15 conversion classes.
    compare_unary_double(
        19,
        c.convert_negative_overflow,
        rust.convert_negative_overflow,
        (0..CASES).map(|i| if i % 2 == 0 { 0.0 } else { -0.0 }),
    );
    covered[19] = true;
    compare_unary_double(
        20,
        c.convert_negative_overflow,
        rust.convert_negative_overflow,
        (0..CASES).map(|_| (f64::from(rng.range(2_000_000_000)) + 0.75) / 1e15),
    );
    covered[20] = true;
    compare_unary_double(
        21,
        c.convert_negative_overflow,
        rust.convert_negative_overflow,
        (0..CASES).map(|_| -(f64::from(rng.range(2_000_000_000)) + 0.75) / 1e15),
    );
    covered[21] = true;
    compare_unary_double(
        22,
        c.convert_negative_overflow,
        rust.convert_negative_overflow,
        (0..CASES).map(|_| 2147483648.0 / 1e15),
    );
    covered[22] = true;
    compare_unary_double(
        23,
        c.convert_negative_overflow,
        rust.convert_negative_overflow,
        (0..CASES).map(|i| -(2147483647.0 + (i as f64 % 10.0) / 10.0) / 1e15),
    );
    covered[23] = true;
    compare_unary_double(
        24,
        c.convert_negative_overflow,
        rust.convert_negative_overflow,
        (0..CASES).map(|_| {
            let magnitude = 2147483649.0 + f64::from(rng.range(1_000_000_000));
            magnitude.copysign(if rng.next_u64() & 1 == 0 { 1.0 } else { -1.0 }) / 1e15
        }),
    );
    covered[24] = true;
    compare_unary_double(
        25,
        c.convert_negative_overflow,
        rust.convert_negative_overflow,
        (0..CASES).map(|_| non_finite[rng.range(3) as usize]),
    );
    covered[25] = true;

    // CONFIGS.md rows 26-34: cross-product of day/hour signs.
    for day_shape in 0..3 {
        for hour_shape in 0..3 {
            let row = 26 + day_shape * 3 + hour_shape;
            for _ in 0..CASES {
                let days = match day_shape {
                    0 => 0,
                    1 => rng.range(20_000) + 1,
                    _ => -(rng.range(20_000) + 1),
                };
                let hours = match hour_shape {
                    0 => 0,
                    1 => rng.range(100) + 1,
                    _ => -(rng.range(100) + 1),
                };
                assert_eq!(
                    unsafe { (rust.get_modified_time)(days, hours) },
                    unsafe { (c.get_modified_time)(days, hours) },
                    "CONFIGS.md row {row}: days={days}, hours={hours}"
                );
            }
            covered[row] = true;
        }
    }

    // CONFIGS.md rows 35-39: time_t sign, extremes, and byte patterns.
    for _ in 0..CASES {
        assert_eq!(
            unsafe { (rust.hash_time_value)(0) },
            unsafe { (c.hash_time_value)(0) },
            "CONFIGS.md row 35"
        );
    }
    covered[35] = true;
    for row in [36, 37] {
        for _ in 0..CASES {
            let raw = rng.next_u64() as i64;
            let value = if row == 36 {
                raw & i64::MAX
            } else {
                raw | i64::MIN
            };
            assert_eq!(
                unsafe { (rust.hash_time_value)(value as c_long) },
                unsafe { (c.hash_time_value)(value as c_long) },
                "CONFIGS.md row {row}: value={value}"
            );
        }
        covered[row] = true;
    }
    for value in [c_long::MIN, c_long::MAX].into_iter().cycle().take(CASES) {
        assert_eq!(
            unsafe { (rust.hash_time_value)(value) },
            unsafe { (c.hash_time_value)(value) },
            "CONFIGS.md row 38: value={value}"
        );
    }
    covered[38] = true;
    let patterns = [
        0x0101_0101_0101_0101_u64,
        0x7F7F_7F7F_7F7F_7F7F,
        0x8080_8080_8080_8080,
        0xFFFF_FFFF_FFFF_FFFF,
        0x00FF_00FF_00FF_00FF,
        0xAA55_AA55_AA55_AA55,
    ];
    for value in patterns.into_iter().cycle().take(CASES) {
        assert_eq!(
            unsafe { (rust.hash_time_value)(value as c_long) },
            unsafe { (c.hash_time_value)(value as c_long) },
            "CONFIGS.md row 39: value={value:#x}"
        );
    }
    covered[39] = true;

    // CONFIGS.md rows 40-63: mode x complexity branches, with all seed remainders.
    for mode_remainder in 0..4 {
        for complexity_shape in 0..6 {
            let row = 40 + mode_remainder * 6 + complexity_shape;
            for case in 0..CASES {
                let mode_selector = if case == 0 {
                    mode_remainder as i32
                } else if case == 1 {
                    4 + mode_remainder as i32
                } else {
                    rng.range(100_000) * 4 + mode_remainder as i32
                };
                let complexity = if complexity_shape < 5 {
                    if case == 0 {
                        complexity_shape as i32
                    } else {
                        rng.range(100_000) * 5 + complexity_shape as i32
                    }
                } else if case == 0 {
                    -1
                } else {
                    -(rng.range(100_000) * 5 + (rng.range(4) + 1))
                };
                let hour_remainder = (case as i32 % 47) - 23;
                let seed = if case < 47 {
                    hour_remainder
                } else if hour_remainder >= 0 {
                    hour_remainder + 24 * rng.range(100_000)
                } else {
                    hour_remainder - 24 * rng.range(100_000)
                };
                let time_offset = match case % 3 {
                    0 => 0,
                    1 => rng.range(20_000) + 1,
                    _ => -(rng.range(20_000) + 1),
                };

                let (c_result, c_output) = capture_stdout(|| unsafe {
                    (c.modeselect)(mode_selector, time_offset, complexity, seed)
                });
                let (rust_result, rust_output) = capture_stdout(|| unsafe {
                    (rust.modeselect)(mode_selector, time_offset, complexity, seed)
                });
                assert_eq!(
                    rust_result, c_result,
                    "CONFIGS.md row {row}: selector={mode_selector}, offset={time_offset}, \
                     complexity={complexity}, seed={seed}"
                );
                assert_eq!(
                    rust_output, c_output,
                    "CONFIGS.md row {row} stdout: selector={mode_selector}, \
                     offset={time_offset}, complexity={complexity}, seed={seed}"
                );
            }
            covered[row] = true;
        }
    }

    for (row, is_covered) in covered.into_iter().enumerate().skip(1) {
        assert!(is_covered, "CONFIGS.md row {row} was not exercised");
    }
}

#[test]
fn classify_mode_null_child() {
    let Ok(path) = std::env::var("MODESELECT_NULL_TEST_LIBRARY") else {
        return;
    };
    let library = unsafe { Library::new(path) }.unwrap();
    let function: libloading::Symbol<ClassifyMode> =
        unsafe { library.get(b"classify_mode\0") }.unwrap();
    unsafe {
        function(std::ptr::null());
    }
}

#[test]
fn null_pointer_boundary_matches() {
    let executable = std::env::current_exe().unwrap();
    let (c_path, rust_path) = paths();
    let run = |path: &Path| {
        Command::new(&executable)
            .args(["--exact", "classify_mode_null_child", "--nocapture"])
            .env("MODESELECT_NULL_TEST_LIBRARY", path)
            .status()
            .unwrap()
    };

    let c_status = run(&c_path);
    let rust_status = run(&rust_path);
    assert_eq!(
        c_status.signal(),
        Some(11),
        "unexpected C status: {c_status}"
    );
    assert_eq!(
        rust_status.signal(),
        c_status.signal(),
        "ERRORS.md row G1 differs: C={c_status}, Rust={rust_status}"
    );
}
