use libloading::Library;
use std::env;
use std::ffi::{CString, c_char, c_double, c_int, c_long, c_void};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;

type ClassifyMode = unsafe extern "C" fn(*const c_char) -> c_int;
type ApplyMultiplier = unsafe extern "C" fn(c_int, c_int) -> c_int;
type Convert = unsafe extern "C" fn(c_double) -> c_int;
type GetModifiedTime = unsafe extern "C" fn(c_int, c_int) -> c_long;
type HashTimeValue = unsafe extern "C" fn(c_long) -> c_int;
type Modeselect = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

unsafe extern "C" {
    fn pipe(fds: *mut c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
    fn fflush(stream: *mut c_void) -> c_int;
}

struct Api {
    _library: Library,
    classify_mode: ClassifyMode,
    apply_multiplier: ApplyMultiplier,
    convert_time_factor: Convert,
    convert_negative_overflow: Convert,
    get_modified_time: GetModifiedTime,
    hash_time_value: HashTimeValue,
    modeselect: Modeselect,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
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

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    root().join("../c_src/build/libharvest-work-NhGLyw.so")
}

fn rust_library_path() -> PathBuf {
    env::var_os("RUST_DIFF_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|| root().join("target/release/libmodeselect_lib.so"))
}

fn load_apis() -> (Api, Api) {
    unsafe {
        (
            Api::load(&c_library_path()),
            Api::load(&rust_library_path()),
        )
    }
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u64() as i32
    }

    fn positive_bounded(&mut self, bound: i32) -> i32 {
        ((self.next_u64() % bound as u64) + 1) as i32
    }
}

unsafe fn capture_stdout<T>(operation: impl FnOnce() -> T) -> (T, Vec<u8>) {
    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0);

        let mut fds = [-1, -1];
        assert_eq!(pipe(fds.as_mut_ptr()), 0);
        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0);
        assert_eq!(dup2(fds[1], 1), 1);
        assert_eq!(close(fds[1]), 0);

        let result = operation();
        assert_eq!(fflush(std::ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, 1), 1);
        assert_eq!(close(saved_stdout), 0);

        let mut output = Vec::new();
        let mut chunk = [0_u8; 4096];
        loop {
            let count = read(fds[0], chunk.as_mut_ptr().cast(), chunk.len());
            assert!(count >= 0);
            if count == 0 {
                break;
            }
            output.extend_from_slice(&chunk[..count as usize]);
        }
        assert_eq!(close(fds[0]), 0);
        (result, output)
    }
}

fn compare_modeselect(
    c_api: &Api,
    rust_api: &Api,
    mode: i32,
    offset: i32,
    complexity: i32,
    seed: i32,
) {
    let (c_result, c_stdout) =
        unsafe { capture_stdout(|| (c_api.modeselect)(mode, offset, complexity, seed)) };
    let (rust_result, rust_stdout) =
        unsafe { capture_stdout(|| (rust_api.modeselect)(mode, offset, complexity, seed)) };
    assert_eq!(
        rust_result, c_result,
        "return mismatch for modeselect({mode}, {offset}, {complexity}, {seed})"
    );
    assert_eq!(
        rust_stdout, c_stdout,
        "stdout mismatch for modeselect({mode}, {offset}, {complexity}, {seed})"
    );
}

fn verify_classify_mode(c_api: &Api, rust_api: &Api, rng: &mut Rng) {
    for (mode, expected) in [
        ("standard", 0x10),
        ("enhanced", 0x20),
        ("turbo", 0x30),
        ("extreme", 0x40),
    ] {
        for _ in 0..64 {
            let value = CString::new(mode).unwrap();
            let c_result = unsafe { (c_api.classify_mode)(value.as_ptr()) };
            let rust_result = unsafe { (rust_api.classify_mode)(value.as_ptr()) };
            assert_eq!(c_result, expected);
            assert_eq!(rust_result, c_result);
        }
    }

    let mut invalid = vec![
        "".to_owned(),
        "Standard".to_owned(),
        "standard ".to_owned(),
        "enhance".to_owned(),
        "turbo\tx".to_owned(),
        "extrem".to_owned(),
    ];
    for _ in 0..256 {
        invalid.push(format!("invalid-{:016x}", rng.next_u64()));
    }
    for mode in invalid {
        let value = CString::new(mode).unwrap();
        let c_result = unsafe { (c_api.classify_mode)(value.as_ptr()) };
        let rust_result = unsafe { (rust_api.classify_mode)(value.as_ptr()) };
        assert_eq!(c_result, 0);
        assert_eq!(rust_result, c_result);
    }
}

fn verify_apply_multiplier(c_api: &Api, rust_api: &Api, rng: &mut Rng) {
    for level in 0..=4 {
        let mut bases = vec![0, 1, -1, i32::MIN, i32::MAX, 0xA0];
        bases.extend((0..512).map(|_| rng.next_i32()));
        for base in bases {
            let c_result = unsafe { (c_api.apply_multiplier)(base, level) };
            let rust_result = unsafe { (rust_api.apply_multiplier)(base, level) };
            assert_eq!(
                rust_result, c_result,
                "apply_multiplier mismatch for base={base}, level={level}"
            );
        }
    }

    let mut invalid_levels = vec![i32::MIN, -1000, -1, 5, 6, 1000, i32::MAX];
    invalid_levels.extend((0..512).map(|_| {
        let candidate = rng.next_i32();
        if (0..=4).contains(&candidate) {
            candidate.wrapping_add(10)
        } else {
            candidate
        }
    }));
    for level in invalid_levels {
        let base = rng.next_i32();
        let c_result = unsafe { (c_api.apply_multiplier)(base, level) };
        let rust_result = unsafe { (rust_api.apply_multiplier)(base, level) };
        assert_eq!(c_result, 0xDEAD);
        assert_eq!(rust_result, c_result);
    }
}

fn verify_convert(
    name: &str,
    c_convert: Convert,
    rust_convert: Convert,
    scale: f64,
    rng: &mut Rng,
) {
    let target_values = [
        0.0,
        1.0,
        -1.0,
        123.75,
        -123.75,
        i32::MIN as f64,
        i32::MAX as f64,
        2147483647.75,
        2147483648.0,
        -2147483649.0,
        1.0e30,
        -1.0e30,
    ];
    for target in target_values {
        let input = target / scale;
        let c_result = unsafe { c_convert(input) };
        let rust_result = unsafe { rust_convert(input) };
        assert_eq!(
            rust_result, c_result,
            "{name} mismatch for input={input:?}, scaled target={target:?}"
        );
    }

    for special in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let c_result = unsafe { c_convert(special) };
        let rust_result = unsafe { rust_convert(special) };
        assert_eq!(rust_result, c_result, "{name} mismatch for {special:?}");
    }

    for _ in 0..4096 {
        let target = if rng.next_u64() & 3 == 0 {
            let magnitude = (rng.next_u64() as f64) * 1.0e5;
            if rng.next_u64() & 1 == 0 {
                magnitude
            } else {
                -magnitude
            }
        } else {
            (rng.next_i32() as f64) + ((rng.next_u64() % 1000) as f64 / 1000.0)
        };
        let input = target / scale;
        let c_result = unsafe { c_convert(input) };
        let rust_result = unsafe { rust_convert(input) };
        assert_eq!(
            rust_result, c_result,
            "{name} randomized mismatch for input={input:?}, target={target:?}"
        );
    }
}

fn verify_get_modified_time(c_api: &Api, rust_api: &Api, rng: &mut Rng) {
    let cases = [
        (0, 0),
        (1, 0),
        (-1, 0),
        (0, 1),
        (0, -1),
        (1, -1),
        (-1, 1),
        (24_854, 0),
        (-24_854, 0),
        (24_000, 23),
        (-24_000, -23),
    ];
    for (days, hours) in cases {
        for _ in 0..32 {
            let c_result = unsafe { (c_api.get_modified_time)(days, hours) };
            let rust_result = unsafe { (rust_api.get_modified_time)(days, hours) };
            assert_eq!(
                rust_result, c_result,
                "get_modified_time mismatch for days={days}, hours={hours}"
            );
        }
    }

    for _ in 0..4096 {
        let days = (rng.next_u64() % 48_001) as i32 - 24_000;
        let hours = (rng.next_u64() % 47) as i32 - 23;
        let c_result = unsafe { (c_api.get_modified_time)(days, hours) };
        let rust_result = unsafe { (rust_api.get_modified_time)(days, hours) };
        assert_eq!(
            rust_result, c_result,
            "get_modified_time randomized mismatch for days={days}, hours={hours}"
        );
    }
}

fn verify_hash_time_value(c_api: &Api, rust_api: &Api, rng: &mut Rng) {
    let values = [
        0_i64,
        1,
        -1,
        0x0102_0304_0506_0708,
        -0x0102_0304_0506_0708,
        i64::MIN,
        i64::MAX,
    ];
    for value in values {
        let c_result = unsafe { (c_api.hash_time_value)(value as c_long) };
        let rust_result = unsafe { (rust_api.hash_time_value)(value as c_long) };
        assert_eq!(
            rust_result, c_result,
            "hash_time_value mismatch for {value:#018x}"
        );
    }

    for _ in 0..16_384 {
        let value = rng.next_u64() as i64;
        let c_result = unsafe { (c_api.hash_time_value)(value as c_long) };
        let rust_result = unsafe { (rust_api.hash_time_value)(value as c_long) };
        assert_eq!(
            rust_result, c_result,
            "hash_time_value randomized mismatch for {value:#018x}"
        );
    }
}

fn verify_modeselect(c_api: &Api, rust_api: &Api, rng: &mut Rng) {
    for mode_remainder in 0..4 {
        for complexity_remainder in 0..5 {
            for seed_remainder in 0..24 {
                for offset_sign in [-1, 0, 1] {
                    let mode = mode_remainder + 4 * (rng.positive_bounded(100_000) - 1);
                    let complexity = complexity_remainder + 5 * (rng.positive_bounded(100_000) - 1);
                    let offset = match offset_sign {
                        -1 => -rng.positive_bounded(20_000),
                        0 => 0,
                        1 => rng.positive_bounded(20_000),
                        _ => unreachable!(),
                    };
                    let seed = if seed_remainder == 0 && offset_sign == 0 {
                        0
                    } else {
                        seed_remainder + 24 * rng.positive_bounded(100_000)
                    };
                    compare_modeselect(c_api, rust_api, mode, offset, complexity, seed);
                }
            }
        }
    }

    compare_modeselect(c_api, rust_api, i32::MAX, i32::MAX, i32::MAX, i32::MAX);
    compare_modeselect(c_api, rust_api, i32::MIN, i32::MIN, i32::MIN, i32::MIN);
}

fn null_child_status(side: &str) -> std::process::ExitStatus {
    Command::new(env::current_exe().unwrap())
        .arg("--exact")
        .arg("null_pointer_child")
        .arg("--nocapture")
        .env("DIFF_NULL_SIDE", side)
        .status()
        .unwrap()
}

fn verify_null_pointer_boundary() {
    let c_status = null_child_status("c");
    let rust_status = null_child_status("rust");
    assert!(
        !c_status.success(),
        "C null-pointer call unexpectedly returned"
    );
    assert!(
        !rust_status.success(),
        "Rust null-pointer call unexpectedly returned"
    );
    assert_eq!(
        rust_status.signal(),
        c_status.signal(),
        "null-pointer calls terminated differently: C={c_status:?}, Rust={rust_status:?}"
    );
    assert!(
        c_status.signal().is_some(),
        "C null-pointer boundary did not terminate by signal: {c_status:?}"
    );
}

#[test]
fn differential_all_configurations_and_errors() {
    let (c_api, rust_api) = load_apis();
    let mut rng = Rng::new(0x4d4f_4445_5345_4c45);

    verify_classify_mode(&c_api, &rust_api, &mut rng);
    verify_apply_multiplier(&c_api, &rust_api, &mut rng);
    verify_convert(
        "convert_time_factor",
        c_api.convert_time_factor,
        rust_api.convert_time_factor,
        1.0e12,
        &mut rng,
    );
    verify_convert(
        "convert_negative_overflow",
        c_api.convert_negative_overflow,
        rust_api.convert_negative_overflow,
        -1.0e15,
        &mut rng,
    );
    verify_get_modified_time(&c_api, &rust_api, &mut rng);
    verify_hash_time_value(&c_api, &rust_api, &mut rng);
    verify_modeselect(&c_api, &rust_api, &mut rng);
    verify_null_pointer_boundary();
}

#[test]
fn null_pointer_child() {
    let Some(side) = env::var_os("DIFF_NULL_SIDE") else {
        return;
    };
    let path = if side == "c" {
        c_library_path()
    } else if side == "rust" {
        rust_library_path()
    } else {
        panic!("unknown DIFF_NULL_SIDE value: {side:?}");
    };
    let api = unsafe { Api::load(&path) };
    unsafe {
        (api.classify_mode)(std::ptr::null());
    }
    panic!("null-pointer call unexpectedly returned");
}
