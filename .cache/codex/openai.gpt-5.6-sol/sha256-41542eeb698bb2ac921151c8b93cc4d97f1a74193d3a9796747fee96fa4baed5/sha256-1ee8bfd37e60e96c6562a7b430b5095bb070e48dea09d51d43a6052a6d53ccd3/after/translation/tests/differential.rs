use libloading::{Library, Symbol};
use std::ffi::{c_char, c_double, c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

type ConvertDoubleToInt = unsafe extern "C" fn(c_double) -> c_int;
type FindValueInBuffer = unsafe extern "C" fn(*const c_char, usize, c_int) -> c_int;
type ProcessNegation = unsafe extern "C" fn(c_int) -> c_int;
type CreateNumericBuffer = unsafe extern "C" fn(*mut c_char, c_int, c_int);
type CalculateWithDoubles = unsafe extern "C" fn(c_int, c_int, c_int) -> c_double;
type Doubleneg = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipefd: *mut c_int) -> c_int;
}

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x as u32
    }

    fn i32_between(&mut self, min: i32, max: i32) -> i32 {
        let width = max as i64 - min as i64 + 1;
        (min as i64 + (self.next_u32() as i64 % width)) as i32
    }
}

fn shared_object(dir: &Path, exact_name: &str) -> PathBuf {
    let exact = dir.join(exact_name);
    assert!(
        exact.is_file(),
        "missing shared object {}; run the required builds first",
        exact.display()
    );
    exact
}

fn libraries() -> (Library, Library) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let c_path = shared_object(&root.join("../c_src/build"), "libharvest-work-fHe7tI.so");
    let rust_path = shared_object(&root.join("target/release"), "libdoubleneg_lib.so");

    unsafe {
        (
            Library::new(c_path).expect("load C shared object"),
            Library::new(rust_path).expect("load Rust shared object"),
        )
    }
}

unsafe fn symbols<T: Copy>(c: &Library, rust: &Library, name: &[u8]) -> (T, T) {
    let c_symbol: Symbol<'_, T> = unsafe { c.get(name) }.expect("C symbol");
    let rust_symbol: Symbol<'_, T> = unsafe { rust.get(name) }.expect("Rust symbol");
    (*c_symbol, *rust_symbol)
}

fn assert_double_bits_eq(c: f64, rust: f64, context: &str) {
    assert_eq!(
        c.to_bits(),
        rust.to_bits(),
        "{context}: C={c:?} Rust={rust:?}"
    );
}

fn capture_stdout(call: impl FnOnce() -> c_int) -> (c_int, Vec<u8>) {
    let mut fds = [-1; 2];
    unsafe {
        assert_eq!(pipe(fds.as_mut_ptr()), 0, "pipe");
        assert_eq!(fflush(std::ptr::null_mut()), 0, "flush before capture");
    }

    let saved_stdout = unsafe { dup(1) };
    assert!(saved_stdout >= 0, "dup stdout");
    unsafe {
        assert_eq!(dup2(fds[1], 1), 1, "redirect stdout");
    }

    let result = call();

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0, "flush captured stdout");
        assert_eq!(dup2(saved_stdout, 1), 1, "restore stdout");
        assert_eq!(close(saved_stdout), 0, "close saved stdout");
        assert_eq!(close(fds[1]), 0, "close pipe writer");
    }

    let mut bytes = Vec::new();
    unsafe {
        File::from_raw_fd(fds[0])
            .read_to_end(&mut bytes)
            .expect("read captured stdout");
    }
    (result, bytes)
}

#[test]
fn c01_c02_convert_double_to_int() {
    let (c, rust) = libraries();
    let (c_fn, rust_fn) =
        unsafe { symbols::<ConvertDoubleToInt>(&c, &rust, b"convert_double_to_int\0") };

    let boundaries = [
        i32::MIN as f64,
        -2_147_483_647.75,
        -1.999,
        -1.0,
        -0.999,
        -0.0,
        0.0,
        0.999,
        1.0,
        1.999,
        2_147_483_646.999,
        i32::MAX as f64,
    ];
    for value in boundaries {
        assert_eq!(
            unsafe { c_fn(value) },
            unsafe { rust_fn(value) },
            "{value:?}"
        );
    }

    let mut rng = Rng::new(0xC01C_02AA_55FF_1020);
    for _ in 0..1_024 {
        let integer = rng.i32_between(-2_000_000_000, 2_000_000_000);
        let fraction = (rng.next_u32() % 1_024) as f64 / 1_024.0;
        let value = integer as f64 + if integer < 0 { -fraction } else { fraction };
        assert_eq!(
            unsafe { c_fn(value) },
            unsafe { rust_fn(value) },
            "{value:?}"
        );
    }

    for value in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        i32::MAX as f64 + 1.0,
        i32::MIN as f64 - 1.0,
        f64::MAX,
        f64::MIN,
    ] {
        assert_eq!(
            unsafe { c_fn(value) },
            unsafe { rust_fn(value) },
            "{value:?}"
        );
    }
}

#[test]
fn c03_to_c07_find_value_in_buffer() {
    let (c, rust) = libraries();
    let (c_fn, rust_fn) =
        unsafe { symbols::<FindValueInBuffer>(&c, &rust, b"find_value_in_buffer\0") };
    let mut rng = Rng::new(0xC03C_0700_1234_5678);

    for search in [-1025, -256, -1, 0, 1, 255, 256, 1025] {
        assert_eq!(unsafe { c_fn(std::ptr::null(), 0, search) }, -1);
        assert_eq!(unsafe { c_fn(std::ptr::null(), 0, search) }, unsafe {
            rust_fn(std::ptr::null(), 0, search)
        });
    }

    for _ in 0..256 {
        let byte = rng.next_u32() as u8;
        let one = [byte as c_char];
        let congruent_search = byte as i32 + 256 * rng.i32_between(-100, 100);
        assert_eq!(unsafe { c_fn(one.as_ptr(), 1, congruent_search) }, 0);
        assert_eq!(unsafe { c_fn(one.as_ptr(), 1, congruent_search) }, unsafe {
            rust_fn(one.as_ptr(), 1, congruent_search)
        });

        let absent = byte.wrapping_add(1) as i32;
        assert_eq!(unsafe { c_fn(one.as_ptr(), 1, absent) }, -1);
        assert_eq!(unsafe { c_fn(one.as_ptr(), 1, absent) }, unsafe {
            rust_fn(one.as_ptr(), 1, absent)
        });
    }

    for _ in 0..512 {
        let len = rng.i32_between(2, 512) as usize;
        let target = rng.next_u32() as u8;
        let first = rng.i32_between(0, (len - 1) as i32) as usize;
        let mut buffer = vec![target.wrapping_add(1) as c_char; len];
        buffer[first] = target as c_char;
        if first + 1 < len {
            buffer[len - 1] = target as c_char;
        }
        let search = target as i32 + 256 * rng.i32_between(-10, 10);
        let c_result = unsafe { c_fn(buffer.as_ptr(), len, search) };
        let rust_result = unsafe { rust_fn(buffer.as_ptr(), len, search) };
        assert_eq!(c_result, first as i32);
        assert_eq!(c_result, rust_result);
    }

    for len in [2_usize, 255, 256, 257, 511] {
        for _ in 0..64 {
            let target = rng.next_u32() as u8;
            let buffer = vec![target.wrapping_add(1) as c_char; len];
            assert_eq!(unsafe { c_fn(buffer.as_ptr(), len, target as i32) }, -1);
            assert_eq!(
                unsafe { c_fn(buffer.as_ptr(), len, target as i32) },
                unsafe { rust_fn(buffer.as_ptr(), len, target as i32) }
            );
        }
    }
}

#[test]
fn e01_find_value_rejection() {
    let (c, rust) = libraries();
    let (c_fn, rust_fn) =
        unsafe { symbols::<FindValueInBuffer>(&c, &rust, b"find_value_in_buffer\0") };
    let mut rng = Rng::new(0xE010_E010_E010_E010);

    for len in [0_usize, 1, 2, 255, 256, 257, 1_024] {
        for _ in 0..128 {
            let target = rng.next_u32() as u8;
            let buffer = vec![target.wrapping_add(127) as c_char; len.max(1)];
            let c_result = unsafe { c_fn(buffer.as_ptr(), len, target as i32) };
            let rust_result = unsafe { rust_fn(buffer.as_ptr(), len, target as i32) };
            assert_eq!(c_result, -1);
            assert_eq!(rust_result, c_result);
        }
    }
}

#[test]
fn c08_c09_process_negation() {
    let (c, rust) = libraries();
    let (c_fn, rust_fn) = unsafe { symbols::<ProcessNegation>(&c, &rust, b"process_negation\0") };

    assert_eq!(unsafe { c_fn(0) }, unsafe { rust_fn(0) });
    let mut rng = Rng::new(0xC08C_0900_9090_8080);
    for value in [i32::MIN, -1, 1, i32::MAX] {
        assert_eq!(unsafe { c_fn(value) }, 1);
        assert_eq!(unsafe { c_fn(value) }, unsafe { rust_fn(value) });
    }
    for _ in 0..1_024 {
        let mut value = rng.next_u32() as i32;
        if value == 0 {
            value = 1;
        }
        assert_eq!(unsafe { c_fn(value) }, unsafe { rust_fn(value) });
    }
}

#[test]
fn c10_to_c12_create_numeric_buffer() {
    let (c, rust) = libraries();
    let (c_fn, rust_fn) =
        unsafe { symbols::<CreateNumericBuffer>(&c, &rust, b"create_numeric_buffer\0") };

    unsafe {
        c_fn(std::ptr::null_mut(), 0, 123);
        rust_fn(std::ptr::null_mut(), 0, 123);
        c_fn(std::ptr::null_mut(), -1, -123);
        rust_fn(std::ptr::null_mut(), -1, -123);
    }

    let mut rng = Rng::new(0xC10C_1200_0102_0304);
    for len in [1_i32, 2, 7, 255, 256, 257, 512] {
        for _ in 0..128 {
            let seed = rng.i32_between(-1_000_000, 1_000_000);
            let mut c_buffer = vec![0x55_u8 as c_char; len as usize];
            let mut rust_buffer = c_buffer.clone();
            unsafe {
                c_fn(c_buffer.as_mut_ptr(), len, seed);
                rust_fn(rust_buffer.as_mut_ptr(), len, seed);
            }
            assert_eq!(c_buffer, rust_buffer, "len={len}, seed={seed}");
        }
    }
}

#[test]
fn c13_to_c16_calculate_with_doubles() {
    let (c, rust) = libraries();
    let (c_fn, rust_fn) =
        unsafe { symbols::<CalculateWithDoubles>(&c, &rust, b"calculate_with_doubles\0") };
    let mut rng = Rng::new(0xC13C_1600_AAAA_5555);

    for c_exp in -29..=29 {
        for a in [i32::MIN, -1, 0, 1, i32::MAX] {
            assert_double_bits_eq(
                unsafe { c_fn(a, 0, c_exp) },
                unsafe { rust_fn(a, 0, c_exp) },
                "b=0",
            );
        }
    }

    for remainder_sign in [-1_i32, 0, 1] {
        for _ in 0..1_024 {
            let a = rng.i32_between(-1_000_000, 1_000_000);
            let mut b = rng.i32_between(-1_000_000, 1_000_000);
            if b == 0 {
                b = 1;
            }
            let magnitude = if remainder_sign == 0 {
                0
            } else {
                rng.i32_between(1, 9)
            };
            let c_exp = remainder_sign * magnitude + 10 * rng.i32_between(-100, 100);
            assert_double_bits_eq(
                unsafe { c_fn(a, b, c_exp) },
                unsafe { rust_fn(a, b, c_exp) },
                &format!("a={a}, b={b}, c={c_exp}"),
            );
        }
    }
}

fn assert_doubleneg_eq(c_fn: Doubleneg, rust_fn: Doubleneg, args: [i32; 4]) -> Vec<u8> {
    let (c_result, c_stdout) =
        capture_stdout(|| unsafe { c_fn(args[0], args[1], args[2], args[3]) });
    let (rust_result, rust_stdout) =
        capture_stdout(|| unsafe { rust_fn(args[0], args[1], args[2], args[3]) });
    assert_eq!(c_result, rust_result, "return value for {args:?}");
    assert_eq!(c_stdout, rust_stdout, "stdout for {args:?}");
    c_stdout
}

#[test]
fn c17_doubleneg_zero_divisor_and_negation_states() {
    let _lock = STDOUT_LOCK.lock().expect("stdout lock");
    let (c, rust) = libraries();
    let (c_fn, rust_fn) = unsafe { symbols::<Doubleneg>(&c, &rust, b"doubleneg\0") };
    let mut rng = Rng::new(0xC170_C170_C170_C170);

    for mask in 0_u8..8 {
        for _ in 0..16 {
            let nonzero = |rng: &mut Rng| {
                let value = rng.i32_between(-50_000, 50_000);
                if value == 0 { 1 } else { value }
            };
            let args = [
                if mask & 1 == 0 { 0 } else { nonzero(&mut rng) },
                0,
                if mask & 2 == 0 { 0 } else { nonzero(&mut rng) },
                if mask & 4 == 0 { 0 } else { nonzero(&mut rng) },
            ];
            assert_doubleneg_eq(c_fn, rust_fn, args);
        }
    }
}

#[test]
fn c18_doubleneg_nonzero_divisor_and_exponent_shapes() {
    let _lock = STDOUT_LOCK.lock().expect("stdout lock");
    let (c, rust) = libraries();
    let (c_fn, rust_fn) = unsafe { symbols::<Doubleneg>(&c, &rust, b"doubleneg\0") };
    let mut rng = Rng::new(0xC180_C180_C180_C180);

    for remainder_sign in [-1_i32, 0, 1] {
        for _ in 0..64 {
            let mut param2 = rng.i32_between(-50_000, 50_000);
            if param2 == 0 {
                param2 = 1;
            }
            let magnitude = if remainder_sign == 0 {
                0
            } else {
                rng.i32_between(1, 9)
            };
            let param3 = remainder_sign * magnitude + 10 * rng.i32_between(-100, 100);
            let args = [
                rng.i32_between(-50_000, 50_000),
                param2,
                param3,
                rng.i32_between(-50_000, 50_000),
            ];
            assert_doubleneg_eq(c_fn, rust_fn, args);
        }
    }
}

#[test]
fn c19_doubleneg_full_buffer_permutation_searches() {
    let _lock = STDOUT_LOCK.lock().expect("stdout lock");
    let (c, rust) = libraries();
    let (c_fn, rust_fn) = unsafe { symbols::<Doubleneg>(&c, &rust, b"doubleneg\0") };
    let mut rng = Rng::new(0xC190_C190_C190_C190);

    for _ in 0..128 {
        let args = [
            rng.i32_between(-1_000_000, 1_000_000),
            rng.i32_between(-10_000, 10_000),
            rng.i32_between(-10_000, 10_000),
            rng.i32_between(-10_000, 10_000),
        ];
        let stdout = assert_doubleneg_eq(c_fn, rust_fn, args);
        let text = String::from_utf8(stdout).expect("ASCII output");
        assert_eq!(text.matches("Found value ").count(), 4, "{args:?}");
        assert!(text.contains("Direct memchr found byte 100"), "{args:?}");
        assert_eq!(text.matches("found=1").count(), 10, "{args:?}");
        assert!(!text.contains("not found"), "{args:?}");
    }
}

#[test]
fn c20_doubleneg_defined_integer_boundaries() {
    let _lock = STDOUT_LOCK.lock().expect("stdout lock");
    let (c, rust) = libraries();
    let (c_fn, rust_fn) = unsafe { symbols::<Doubleneg>(&c, &rust, b"doubleneg\0") };

    let cases = [
        [i32::MIN, 0, -9, i32::MAX],
        [i32::MIN, 1, 0, i32::MAX],
        [i32::MIN + 20, -1, 9, i32::MAX],
        [i32::MAX - 1_785, 0, -9, i32::MIN],
        [i32::MAX - 1_805, 2, 9, i32::MIN],
    ];
    for args in cases {
        assert_doubleneg_eq(c_fn, rust_fn, args);
    }
}
