use libloading::Library;
use std::ffi::{CString, c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

type DriverFn = unsafe extern "C" fn(*const c_char);
type RunFn = unsafe extern "C" fn(*mut House, c_int);

struct Api {
    _library: Library,
    driver: DriverFn,
    run: RunFn,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let driver = unsafe { *library.get::<DriverFn>(b"driver\0").unwrap() };
        let run = unsafe { *library.get::<RunFn>(b"run\0").unwrap() };
        Self {
            _library: library,
            driver,
            run,
        }
    }
}

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fork() -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(status: c_int) -> !;
}

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

fn library_paths() -> (PathBuf, PathBuf) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    (
        manifest.join("../c_src/build/libdriver.so"),
        manifest.join("target/release/libdriver.so"),
    )
}

fn load_apis() -> (Api, Api) {
    let (c_path, rust_path) = library_paths();
    assert!(
        c_path.is_file(),
        "missing C shared object: {}",
        c_path.display()
    );
    assert!(
        rust_path.is_file(),
        "missing Rust shared object: {}",
        rust_path.display()
    );
    unsafe { (Api::load(&c_path), Api::load(&rust_path)) }
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0);
        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0);

        let mut fds = [-1, -1];
        assert_eq!(pipe(fds.as_mut_ptr()), 0);
        assert_eq!(dup2(fds[1], 1), 1);
        assert_eq!(close(fds[1]), 0);

        call();

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
        output
    }
}

fn house_bytes(house: &House) -> [u8; std::mem::size_of::<House>()] {
    unsafe { std::ptr::read((house as *const House).cast()) }
}

fn compare_run(c_api: &Api, rust_api: &Api, initial: House, extra: c_int, label: &str) {
    let mut c_house = initial;
    let mut rust_house = initial;
    let c_output = capture_stdout(|| unsafe { (c_api.run)(&mut c_house, extra) });
    let rust_output = capture_stdout(|| unsafe { (rust_api.run)(&mut rust_house, extra) });

    assert_eq!(c_output, rust_output, "{label}: stdout differs");
    assert_eq!(
        house_bytes(&c_house),
        house_bytes(&rust_house),
        "{label}: mutated house bytes differ; C={c_house:?}, Rust={rust_house:?}"
    );
}

fn compare_driver(c_api: &Api, rust_api: &Api, input: &str, label: &str) {
    let input = CString::new(input).unwrap();
    let c_output = capture_stdout(|| unsafe { (c_api.driver)(input.as_ptr()) });
    let rust_output = capture_stdout(|| unsafe { (rust_api.driver)(input.as_ptr()) });
    assert_eq!(c_output, rust_output, "{label}: input={input:?}");
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

    fn bounded_i32(&mut self, magnitude: i32) -> i32 {
        (self.next_u64() % (2 * magnitude as u64 + 1)) as i32 - magnitude
    }
}

fn with_apis(test: impl FnOnce(&Api, &Api)) {
    let _guard = STDOUT_LOCK.lock().unwrap();
    let (c_api, rust_api) = load_apis();
    test(&c_api, &rust_api);
}

#[test]
fn config_c1_run_negative_extra() {
    with_apis(|c_api, rust_api| {
        let mut rng = Rng::new(0x42c1_0001);
        for index in 0..128 {
            let house = House {
                floors: rng.bounded_i32(100_000),
                bedrooms: rng.bounded_i32(100_000),
                bathrooms: rng.bounded_i32(100_000) as f64 / 8.0,
            };
            let extra = -((rng.next_u64() % 100_000) as i32 + 1);
            compare_run(c_api, rust_api, house, extra, &format!("C1 case {index}"));
        }
    });
}

#[test]
fn config_c2_run_zero_extra() {
    with_apis(|c_api, rust_api| {
        let mut rng = Rng::new(0x42c2_0002);
        for index in 0..128 {
            let house = House {
                floors: rng.bounded_i32(1_000_000),
                bedrooms: rng.next_i32(),
                bathrooms: rng.bounded_i32(1_000_000) as f64 / 16.0,
            };
            compare_run(c_api, rust_api, house, 0, &format!("C2 case {index}"));
        }
    });
}

#[test]
fn config_c3_run_positive_extra() {
    with_apis(|c_api, rust_api| {
        let mut rng = Rng::new(0x42c3_0003);
        for index in 0..128 {
            let house = House {
                floors: rng.bounded_i32(100_000),
                bedrooms: rng.bounded_i32(100_000),
                bathrooms: rng.bounded_i32(100_000) as f64 / 32.0,
            };
            let extra = (rng.next_u64() % 100_000) as i32 + 1;
            compare_run(c_api, rust_api, house, extra, &format!("C3 case {index}"));
        }
    });
}

#[test]
fn config_c4_run_floor_boundaries() {
    with_apis(|c_api, rust_api| {
        let fixed = [i32::MIN, i32::MIN + 1, -1, 0, i32::MAX - 1, i32::MAX];
        for (index, floors) in fixed.into_iter().enumerate() {
            compare_run(
                c_api,
                rust_api,
                House {
                    floors,
                    bedrooms: 7,
                    bathrooms: 2.5,
                },
                0,
                &format!("C4 fixed {index}"),
            );
        }

        let mut rng = Rng::new(0x42c4_0004);
        for index in 0..128 {
            let distance = (rng.next_u64() % 1024) as i32;
            let floors = if index % 2 == 0 {
                i32::MAX - distance
            } else {
                i32::MIN + distance
            };
            compare_run(
                c_api,
                rust_api,
                House {
                    floors,
                    bedrooms: rng.bounded_i32(1000),
                    bathrooms: 1.25,
                },
                0,
                &format!("C4 random {index}"),
            );
        }
    });
}

#[test]
fn config_c5_run_bedroom_boundaries() {
    with_apis(|c_api, rust_api| {
        let fixed = [
            (i32::MAX, 1),
            (i32::MAX - 5, 10),
            (i32::MIN, -1),
            (i32::MIN + 5, -10),
            (i32::MAX, i32::MAX),
            (i32::MIN, i32::MIN),
        ];
        for (index, (bedrooms, extra)) in fixed.into_iter().enumerate() {
            compare_run(
                c_api,
                rust_api,
                House {
                    floors: 2,
                    bedrooms,
                    bathrooms: 2.5,
                },
                extra,
                &format!("C5 fixed {index}"),
            );
        }

        let mut rng = Rng::new(0x42c5_0005);
        for index in 0..128 {
            compare_run(
                c_api,
                rust_api,
                House {
                    floors: rng.bounded_i32(1000),
                    bedrooms: rng.next_i32(),
                    bathrooms: 3.5,
                },
                rng.next_i32(),
                &format!("C5 random {index}"),
            );
        }
    });
}

#[test]
fn config_c6_run_finite_bathroom_shapes() {
    with_apis(|c_api, rust_api| {
        let fixed = [
            -0.0,
            0.0,
            f64::from_bits(1),
            -f64::from_bits(1),
            0.05,
            0.15,
            0.25,
            0.5,
            1.0,
            -1.0,
            f64::MAX,
            -f64::MAX,
            9_007_199_254_740_992.0,
        ];
        for (index, bathrooms) in fixed.into_iter().enumerate() {
            compare_run(
                c_api,
                rust_api,
                House {
                    floors: 2,
                    bedrooms: 5,
                    bathrooms,
                },
                3,
                &format!("C6 fixed {index}"),
            );
        }

        let mut rng = Rng::new(0x42c6_0006);
        let mut tested = 0;
        while tested < 128 {
            let bathrooms = f64::from_bits(rng.next_u64());
            if bathrooms.is_finite() {
                compare_run(
                    c_api,
                    rust_api,
                    House {
                        floors: rng.bounded_i32(1000),
                        bedrooms: rng.bounded_i32(1000),
                        bathrooms,
                    },
                    rng.bounded_i32(1000),
                    &format!("C6 random {tested}"),
                );
                tested += 1;
            }
        }
    });
}

#[test]
fn config_c7_run_non_finite_bathroom_shapes() {
    with_apis(|c_api, rust_api| {
        let mut values = vec![f64::INFINITY, f64::NEG_INFINITY, f64::NAN, -f64::NAN];
        let mut rng = Rng::new(0x42c7_0007);
        for _ in 0..128 {
            let sign = (rng.next_u64() & (1_u64 << 63)) | 0x7ff0_0000_0000_0000;
            let payload = (rng.next_u64() & 0x000f_ffff_ffff_ffff).max(1);
            values.push(f64::from_bits(sign | payload));
        }
        for (index, bathrooms) in values.into_iter().enumerate() {
            compare_run(
                c_api,
                rust_api,
                House {
                    floors: 2,
                    bedrooms: 5,
                    bathrooms,
                },
                -3,
                &format!("C7 case {index}"),
            );
        }
    });
}

#[test]
fn config_c8_driver_canonical_decimals() {
    with_apis(|c_api, rust_api| {
        let mut rng = Rng::new(0x42c8_0008);
        let mut values = vec![-1, 0, 1];
        values.extend((0..128).map(|_| rng.next_i32()));
        for (index, value) in values.into_iter().enumerate() {
            compare_driver(
                c_api,
                rust_api,
                &value.to_string(),
                &format!("C8 case {index}"),
            );
        }
    });
}

#[test]
fn config_c9_driver_leading_space_and_sign() {
    with_apis(|c_api, rust_api| {
        let mut rng = Rng::new(0x42c9_0009);
        for index in 0..128 {
            let value = rng.next_i32();
            let input = match index % 4 {
                0 => format!("   {value}"),
                1 if value >= 0 => format!("+{value}"),
                1 => format!(" {value}"),
                2 if value >= 0 => format!("\t +{value}"),
                2 => format!("\t {value}"),
                _ => format!("\n\r {value}"),
            };
            compare_driver(c_api, rust_api, &input, &format!("C9 case {index}"));
        }
    });
}

#[test]
fn config_c10_driver_trailing_suffix() {
    with_apis(|c_api, rust_api| {
        let mut rng = Rng::new(0x42ca_0010);
        for index in 0..128 {
            let value = rng.next_i32();
            let input = match index % 4 {
                0 => format!("{value}xyz"),
                1 => format!("{value} "),
                2 => format!("{value}\nnot-a-number"),
                _ => format!("{value}.75"),
            };
            compare_driver(c_api, rust_api, &input, &format!("C10 case {index}"));
        }
    });
}

#[test]
fn config_c11_driver_integer_boundaries() {
    with_apis(|c_api, rust_api| {
        for (index, value) in [i32::MIN, i32::MAX].into_iter().enumerate() {
            compare_driver(
                c_api,
                rust_api,
                &value.to_string(),
                &format!("C11 case {index}"),
            );
        }
    });
}

#[test]
fn error_e1_no_characters_consumed() {
    with_apis(|c_api, rust_api| {
        let cases = ["", " ", "\t\r\n", "x", "+", "-", " +x", "--1", "++1"];
        for (index, input) in cases.into_iter().enumerate() {
            compare_driver(c_api, rust_api, input, &format!("E1 case {index}"));
        }
    });
}

#[test]
fn error_e2_long_range_error() {
    with_apis(|c_api, rust_api| {
        let cases = [
            "999999999999999999999999999999999999999999999999999999999999",
            "-999999999999999999999999999999999999999999999999999999999999",
            "184467440737095516160000000000000000000",
            "-184467440737095516160000000000000000000",
        ];
        for (index, input) in cases.into_iter().enumerate() {
            compare_driver(c_api, rust_api, input, &format!("E2 case {index}"));
        }
    });
}

#[test]
fn error_e3_below_int_min() {
    with_apis(|c_api, rust_api| {
        let mut rng = Rng::new(0x42e3_0003);
        let mut values = vec![i32::MIN as i64 - 1, i64::MIN];
        values.extend((0..128).map(|_| i32::MIN as i64 - 1 - (rng.next_u64() % 1_000_000) as i64));
        for (index, value) in values.into_iter().enumerate() {
            compare_driver(
                c_api,
                rust_api,
                &value.to_string(),
                &format!("E3 case {index}"),
            );
        }
    });
}

#[test]
fn error_e4_above_int_max() {
    with_apis(|c_api, rust_api| {
        let mut rng = Rng::new(0x42e4_0004);
        let mut values = vec![i32::MAX as i64 + 1, i64::MAX];
        values.extend((0..128).map(|_| i32::MAX as i64 + 1 + (rng.next_u64() % 1_000_000) as i64));
        for (index, value) in values.into_iter().enumerate() {
            compare_driver(
                c_api,
                rust_api,
                &value.to_string(),
                &format!("E4 case {index}"),
            );
        }
    });
}

fn child_signal(call: impl FnOnce()) -> c_int {
    unsafe {
        let pid = fork();
        assert!(pid >= 0);
        if pid == 0 {
            call();
            _exit(0);
        }
        let mut status = 0;
        assert_eq!(waitpid(pid, &mut status, 0), pid);
        status & 0x7f
    }
}

#[test]
fn generic_null_pointer_boundaries() {
    with_apis(|c_api, rust_api| {
        let c_driver_signal = child_signal(|| unsafe { (c_api.driver)(std::ptr::null()) });
        let rust_driver_signal = child_signal(|| unsafe { (rust_api.driver)(std::ptr::null()) });
        assert_ne!(c_driver_signal, 0, "C driver(NULL) unexpectedly returned");
        assert_eq!(
            c_driver_signal, rust_driver_signal,
            "driver(NULL) termination signal differs"
        );

        let c_run_signal = child_signal(|| unsafe { (c_api.run)(std::ptr::null_mut(), 0) });
        let rust_run_signal = child_signal(|| unsafe { (rust_api.run)(std::ptr::null_mut(), 0) });
        assert_ne!(c_run_signal, 0, "C run(NULL, 0) unexpectedly returned");
        assert_eq!(
            c_run_signal, rust_run_signal,
            "run(NULL, 0) termination signal differs"
        );
    });
}
