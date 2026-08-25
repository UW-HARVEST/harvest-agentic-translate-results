use libloading::Library;
use std::ffi::{c_char, c_double, c_int, c_void};
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

type ConvertDoubleToInt = unsafe extern "C" fn(c_double) -> c_int;
type FindValueInBuffer = unsafe extern "C" fn(*const c_char, usize, c_int) -> c_int;
type ProcessNegation = unsafe extern "C" fn(c_int) -> c_int;
type CreateNumericBuffer = unsafe extern "C" fn(*mut c_char, c_int, c_int);
type CalculateWithDoubles = unsafe extern "C" fn(c_int, c_int, c_int) -> c_double;
type Doubleneg = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

struct Api {
    _library: Library,
    convert_double_to_int: ConvertDoubleToInt,
    find_value_in_buffer: FindValueInBuffer,
    process_negation: ProcessNegation,
    create_numeric_buffer: CreateNumericBuffer,
    calculate_with_doubles: CalculateWithDoubles,
    doubleneg: Doubleneg,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));

        let convert_double_to_int = unsafe {
            *library
                .get::<ConvertDoubleToInt>(b"convert_double_to_int\0")
                .unwrap()
        };
        let find_value_in_buffer = unsafe {
            *library
                .get::<FindValueInBuffer>(b"find_value_in_buffer\0")
                .unwrap()
        };
        let process_negation = unsafe {
            *library
                .get::<ProcessNegation>(b"process_negation\0")
                .unwrap()
        };
        let create_numeric_buffer = unsafe {
            *library
                .get::<CreateNumericBuffer>(b"create_numeric_buffer\0")
                .unwrap()
        };
        let calculate_with_doubles = unsafe {
            *library
                .get::<CalculateWithDoubles>(b"calculate_with_doubles\0")
                .unwrap()
        };
        let doubleneg = unsafe { *library.get::<Doubleneg>(b"doubleneg\0").unwrap() };

        Self {
            _library: library,
            convert_double_to_int,
            find_value_in_buffer,
            process_negation,
            create_numeric_buffer,
            calculate_with_doubles,
            doubleneg,
        }
    }
}

fn library_paths() -> (PathBuf, PathBuf) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_library = manifest.join("c_src/build/libtranslated_rust.so");
    let test_executable = std::env::current_exe().expect("test executable path");
    let profile_dir = test_executable
        .parent()
        .and_then(Path::parent)
        .expect("target profile directory");
    let rust_library = profile_dir.join("libdoubleneg_lib.so");

    assert!(c_library.is_file(), "missing {}", c_library.display());
    assert!(rust_library.is_file(), "missing {}", rust_library.display());
    (c_library, rust_library)
}

fn load_apis() -> (Api, Api) {
    let (c_library, rust_library) = library_paths();
    unsafe { (Api::load(&c_library), Api::load(&rust_library)) }
}

#[derive(Clone, Copy)]
struct Prng(u64);

impl Prng {
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

    fn range_i32(&mut self, low: i32, high: i32) -> i32 {
        assert!(low <= high);
        let width = (i64::from(high) - i64::from(low) + 1) as u64;
        (i64::from(low) + (self.next_u64() % width) as i64) as i32
    }

    fn nonzero_i32(&mut self, low: i32, high: i32) -> i32 {
        loop {
            let value = self.range_i32(low, high);
            if value != 0 {
                return value;
            }
        }
    }
}

fn assert_double_bits_equal(c_value: f64, rust_value: f64, context: &str) {
    assert_eq!(
        c_value.to_bits(),
        rust_value.to_bits(),
        "{context}: C={c_value:?}, Rust={rust_value:?}"
    );
}

#[test]
fn phase_b_low_level_valid_paths() {
    let (c, rust) = load_apis();
    let mut random = Prng::new(0x8a5c_31d2_4e77_b901);

    // C01: finite exact integers.
    for _ in 0..128 {
        let input = f64::from(random.range_i32(-2_000_000_000, 2_000_000_000));
        let c_result = unsafe { (c.convert_double_to_int)(input) };
        let rust_result = unsafe { (rust.convert_double_to_int)(input) };
        assert_eq!(c_result, rust_result, "C01 input={input:?}");
    }

    // C02: finite fractions, including both signs.
    for sign in [-1.0, 1.0] {
        for _ in 0..64 {
            let integer = f64::from(random.range_i32(0, 1_000_000));
            let fraction = f64::from(random.range_i32(1, 999)) / 1000.0;
            let input = sign * (integer + fraction);
            let c_result = unsafe { (c.convert_double_to_int)(input) };
            let rust_result = unsafe { (rust.convert_double_to_int)(input) };
            assert_eq!(c_result, rust_result, "C02 input={input:?}");
        }
    }

    // C03: empty prefixes.
    for _ in 0..64 {
        let buffer = [random.next_u64() as c_char; 8];
        let search = random.range_i32(-1024, 1024);
        let c_result = unsafe { (c.find_value_in_buffer)(buffer.as_ptr(), 0, search) };
        let rust_result = unsafe { (rust.find_value_in_buffer)(buffer.as_ptr(), 0, search) };
        assert_eq!(c_result, rust_result, "C03 search={search}");
    }

    // C04-C06: found first, found later, and absent.
    for _ in 0..64 {
        let length = random.range_i32(2, 128) as usize;
        let target = random.range_i32(0, 255) as u8;
        let other = target.wrapping_add(1);

        let mut first = vec![other as c_char; length];
        first[0] = target as c_char;
        let c_first =
            unsafe { (c.find_value_in_buffer)(first.as_ptr(), first.len(), target.into()) };
        let rust_first =
            unsafe { (rust.find_value_in_buffer)(first.as_ptr(), first.len(), target.into()) };
        assert_eq!(c_first, rust_first, "C04 target={target}");

        let mut later = vec![other as c_char; length];
        let offset = if random.next_u64() & 1 == 0 {
            length / 2
        } else {
            length - 1
        };
        later[offset] = target as c_char;
        let c_later =
            unsafe { (c.find_value_in_buffer)(later.as_ptr(), later.len(), target.into()) };
        let rust_later =
            unsafe { (rust.find_value_in_buffer)(later.as_ptr(), later.len(), target.into()) };
        assert_eq!(c_later, rust_later, "C05 target={target}, offset={offset}");

        let absent = vec![other as c_char; length];
        let c_absent =
            unsafe { (c.find_value_in_buffer)(absent.as_ptr(), absent.len(), target.into()) };
        let rust_absent =
            unsafe { (rust.find_value_in_buffer)(absent.as_ptr(), absent.len(), target.into()) };
        assert_eq!(c_absent, rust_absent, "C06 target={target}");
    }

    // C07: search integers alias through the C char cast.
    for _ in 0..64 {
        let target = random.range_i32(0, 255);
        let alias = target + 256 * random.nonzero_i32(-1000, 1000);
        let buffer = [target as c_char, 17, 23, 42];
        let c_result = unsafe { (c.find_value_in_buffer)(buffer.as_ptr(), buffer.len(), alias) };
        let rust_result =
            unsafe { (rust.find_value_in_buffer)(buffer.as_ptr(), buffer.len(), alias) };
        assert_eq!(c_result, rust_result, "C07 target={target}, alias={alias}");
    }

    // C08-C09: boolean normalization of zero and randomized nonzero values.
    assert_eq!(
        unsafe { (c.process_negation)(0) },
        unsafe { (rust.process_negation)(0) },
        "C08"
    );
    for _ in 0..128 {
        let input = random.nonzero_i32(i32::MIN, i32::MAX);
        let c_result = unsafe { (c.process_negation)(input) };
        let rust_result = unsafe { (rust.process_negation)(input) };
        assert_eq!(c_result, rust_result, "C09 input={input}");
    }

    // C10-C12: negative, zero, and unit loop counts.
    for _ in 0..64 {
        let seed = random.range_i32(-1_000_000, 1_000_000);
        let size = random.range_i32(-1000, -1);
        let mut c_buffer = [0x5a as c_char; 4];
        let mut rust_buffer = c_buffer;
        unsafe {
            (c.create_numeric_buffer)(c_buffer.as_mut_ptr(), size, seed);
            (rust.create_numeric_buffer)(rust_buffer.as_mut_ptr(), size, seed);
        }
        assert_eq!(c_buffer, rust_buffer, "C10 size={size}, seed={seed}");

        unsafe {
            (c.create_numeric_buffer)(c_buffer.as_mut_ptr(), 0, seed);
            (rust.create_numeric_buffer)(rust_buffer.as_mut_ptr(), 0, seed);
        }
        assert_eq!(c_buffer, rust_buffer, "C11 seed={seed}");

        unsafe {
            (c.create_numeric_buffer)(c_buffer.as_mut_ptr(), 1, seed);
            (rust.create_numeric_buffer)(rust_buffer.as_mut_ptr(), 1, seed);
        }
        assert_eq!(c_buffer, rust_buffer, "C12 seed={seed}");
    }

    // C13-C14: multi-byte generation with positive and negative seeds.
    for negative in [false, true] {
        for _ in 0..96 {
            let size = random.range_i32(2, 256);
            let magnitude = random.range_i32(1, 1_000_000);
            let seed = if negative { -magnitude } else { magnitude };
            let mut c_buffer = vec![0x5a as c_char; size as usize];
            let mut rust_buffer = c_buffer.clone();
            unsafe {
                (c.create_numeric_buffer)(c_buffer.as_mut_ptr(), size, seed);
                (rust.create_numeric_buffer)(rust_buffer.as_mut_ptr(), size, seed);
            }
            let row = if negative { "C14" } else { "C13" };
            assert_eq!(c_buffer, rust_buffer, "{row} size={size}, seed={seed}");
        }
    }

    // C15: division skipped.
    for _ in 0..96 {
        let a = random.range_i32(-1_000_000, 1_000_000);
        let exponent = random.range_i32(-1000, 1000);
        let c_result = unsafe { (c.calculate_with_doubles)(a, 0, exponent) };
        let rust_result = unsafe { (rust.calculate_with_doubles)(a, 0, exponent) };
        assert_double_bits_equal(c_result, rust_result, "C15");
    }

    // C16-C18: nonzero denominator and zero, positive, or negative c % 10.
    for row in 0..3 {
        for _ in 0..128 {
            let a = random.range_i32(-1_000_000, 1_000_000);
            let b = random.nonzero_i32(-10_000, 10_000);
            let c_value = match row {
                0 => random.range_i32(-100, 100) * 10,
                1 => random.range_i32(-100, 100) * 10 + random.range_i32(1, 9),
                _ => random.range_i32(-100, 100) * 10 - random.range_i32(1, 9),
            };
            let c_result = unsafe { (c.calculate_with_doubles)(a, b, c_value) };
            let rust_result = unsafe { (rust.calculate_with_doubles)(a, b, c_value) };
            assert_double_bits_equal(c_result, rust_result, &format!("C{}", 16 + row));
        }
    }
}

static STDOUT_LOCK: Mutex<()> = Mutex::new(());
static CAPTURE_ID: AtomicU64 = AtomicU64::new(0);

fn capture_stdout<T>(call: impl FnOnce() -> T) -> (T, Vec<u8>) {
    let _lock = STDOUT_LOCK.lock().expect("stdout lock");
    let path = std::env::temp_dir().join(format!(
        "doubleneg-differential-{}-{}",
        std::process::id(),
        CAPTURE_ID.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(&path)
        .expect("create stdout capture");

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0);
    }
    let saved_stdout = unsafe { dup(1) };
    assert!(saved_stdout >= 0, "dup stdout");
    assert_eq!(unsafe { dup2(file.as_raw_fd(), 1) }, 1, "redirect stdout");

    let result = call();

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0);
    }
    assert_eq!(unsafe { dup2(saved_stdout, 1) }, 1, "restore stdout");
    assert_eq!(unsafe { close(saved_stdout) }, 0, "close saved stdout");

    file.seek(SeekFrom::Start(0)).expect("rewind capture");
    let mut output = Vec::new();
    file.read_to_end(&mut output).expect("read capture");
    drop(file);
    std::fs::remove_file(path).expect("remove capture");
    (result, output)
}

#[test]
fn phase_b_doubleneg_all_zero_masks() {
    let (c, rust) = load_apis();
    let mut random = Prng::new(0x5be0_cd19_137e_2179);

    // C19-C34: all 16 independently observable zero/nonzero parameter masks.
    for mask in 0_u8..16 {
        for iteration in 0..16 {
            let mut values = [0_i32; 4];
            for (index, value) in values.iter_mut().enumerate() {
                if mask & (1 << (3 - index)) != 0 {
                    *value = if index == 2 {
                        random.nonzero_i32(-2, 2)
                    } else {
                        random.nonzero_i32(-1000, 1000)
                    };
                }
            }

            let (c_result, c_output) = capture_stdout(|| unsafe {
                (c.doubleneg)(values[0], values[1], values[2], values[3])
            });
            let (rust_result, rust_output) = capture_stdout(|| unsafe {
                (rust.doubleneg)(values[0], values[1], values[2], values[3])
            });

            assert_eq!(
                c_result,
                rust_result,
                "C{} mask={mask:04b}, iteration={iteration}, values={values:?}",
                19 + mask
            );
            assert_eq!(
                c_output,
                rust_output,
                "C{} stdout mask={mask:04b}, iteration={iteration}, values={values:?}",
                19 + mask
            );
        }
    }
}

#[test]
fn phase_c_error_and_generic_boundaries() {
    let (c, rust) = load_apis();
    let mut random = Prng::new(0xd6e8_feb8_6659_fd93);

    // E01: the sole C rejection branch must return the exact -1 sentinel.
    for _ in 0..128 {
        let target = random.range_i32(0, 255) as u8;
        let other = target.wrapping_add(1);
        let length = random.range_i32(0, 256) as usize;
        let buffer = vec![other as c_char; length.max(1)];
        let c_result = unsafe { (c.find_value_in_buffer)(buffer.as_ptr(), length, target.into()) };
        let rust_result =
            unsafe { (rust.find_value_in_buffer)(buffer.as_ptr(), length, target.into()) };
        assert_eq!(c_result, -1, "E01 C target={target}, length={length}");
        assert_eq!(
            rust_result, c_result,
            "E01 Rust target={target}, length={length}"
        );
    }

    // G01: null is not dereferenced for a zero-length memchr.
    let null_search = random.range_i32(-1024, 1024);
    let c_null_empty = unsafe { (c.find_value_in_buffer)(std::ptr::null(), 0, null_search) };
    let rust_null_empty = unsafe { (rust.find_value_in_buffer)(std::ptr::null(), 0, null_search) };
    assert_eq!(c_null_empty, -1, "G01 C");
    assert_eq!(rust_null_empty, c_null_empty, "G01 Rust");

    // G02: an oversized size_t is observable safely when byte zero matches.
    for _ in 0..64 {
        let target = random.range_i32(0, 255) as u8;
        let buffer = [target as c_char];
        let c_result =
            unsafe { (c.find_value_in_buffer)(buffer.as_ptr(), usize::MAX, target.into()) };
        let rust_result =
            unsafe { (rust.find_value_in_buffer)(buffer.as_ptr(), usize::MAX, target.into()) };
        assert_eq!(c_result, 0, "G02 C target={target}");
        assert_eq!(rust_result, c_result, "G02 Rust target={target}");
    }

    // G03: nonpositive loop counts do not touch a null output pointer.
    for size in [i32::MIN, -1, 0] {
        unsafe {
            (c.create_numeric_buffer)(std::ptr::null_mut(), size, 123);
            (rust.create_numeric_buffer)(std::ptr::null_mut(), size, 123);
        }
    }

    // G04: exact values around the create loop threshold.
    for _ in 0..64 {
        let seed = random.range_i32(-1_000_000, 1_000_000);
        for size in [-1, 0, 1] {
            let mut c_buffer = [0x5a as c_char, 0x33 as c_char];
            let mut rust_buffer = c_buffer;
            unsafe {
                (c.create_numeric_buffer)(c_buffer.as_mut_ptr(), size, seed);
                (rust.create_numeric_buffer)(rust_buffer.as_mut_ptr(), size, seed);
            }
            assert_eq!(c_buffer, rust_buffer, "G04 size={size}, seed={seed}");
        }
    }

    // G05: exact double encodings at and immediately around int boundaries.
    let lower = -2_147_483_648.0_f64;
    let upper = 2_147_483_648.0_f64;
    let values = [
        f64::from_bits(lower.to_bits() + 1),
        lower,
        f64::from_bits(lower.to_bits() - 1),
        f64::from_bits(upper.to_bits() - 1),
        upper,
        f64::from_bits(upper.to_bits() + 1),
        f64::NAN,
        f64::from_bits(0xfff8_0000_0000_0001),
        f64::INFINITY,
        f64::NEG_INFINITY,
    ];
    for input in values {
        let c_result = unsafe { (c.convert_double_to_int)(input) };
        let rust_result = unsafe { (rust.convert_double_to_int)(input) };
        assert_eq!(
            rust_result,
            c_result,
            "G05 input={input:?}, bits={:#018x}",
            input.to_bits()
        );
    }
}
