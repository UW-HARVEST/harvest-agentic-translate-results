use libloading::Library;
use std::ffi::{CStr, c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::Mutex;

type IntFn = unsafe extern "C" fn(c_int) -> c_int;
type ApplyFn = unsafe extern "C" fn(Option<IntFn>, c_int) -> c_int;
type CharinbufFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
type IsEmptyFn = unsafe extern "C" fn(*const c_char) -> c_int;
type FindFn = unsafe extern "C" fn(*const c_char, usize, c_char) -> *mut c_char;
type CreateFn = unsafe extern "C" fn(*const c_char) -> *mut c_char;

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn free(pointer: *mut c_void);
    fn pipe(fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
}

static PROCESS_LOCK: Mutex<()> = Mutex::new(());

struct Api {
    _library: Library,
    increment_counter: IntFn,
    decrement_counter: IntFn,
    multiply_counter: IntFn,
    reset_counter: IntFn,
    is_string_empty: IsEmptyFn,
    find_char_in_buffer: FindFn,
    create_buffer: CreateFn,
    validate_uint16_range: IntFn,
    apply_operation: ApplyFn,
    charinbuf: CharinbufFn,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));

        macro_rules! symbol {
            ($name:literal, $ty:ty) => {
                *unsafe { library.get::<$ty>(concat!($name, "\0").as_bytes()) }
                    .unwrap_or_else(|error| panic!("failed to load {}: {error}", $name))
            };
        }

        Self {
            increment_counter: symbol!("increment_counter", IntFn),
            decrement_counter: symbol!("decrement_counter", IntFn),
            multiply_counter: symbol!("multiply_counter", IntFn),
            reset_counter: symbol!("reset_counter", IntFn),
            is_string_empty: symbol!("is_string_empty", IsEmptyFn),
            find_char_in_buffer: symbol!("find_char_in_buffer", FindFn),
            create_buffer: symbol!("create_buffer", CreateFn),
            validate_uint16_range: symbol!("validate_uint16_range", IntFn),
            apply_operation: symbol!("apply_operation", ApplyFn),
            charinbuf: symbol!("charinbuf", CharinbufFn),
            _library: library,
        }
    }
}

struct Rng(u64);

impl Rng {
    fn new() -> Self {
        Self(0x4d59_5df4_d0f3_3173)
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

    fn range_i32(&mut self, minimum: i32, maximum: i32) -> i32 {
        let width = i64::from(maximum) - i64::from(minimum) + 1;
        minimum + (self.next_u64() % width as u64) as i32
    }

    fn byte(&mut self) -> u8 {
        self.next_u64() as u8
    }
}

fn library_paths() -> (PathBuf, PathBuf) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_library = manifest.join("c_src/build/libtranslated_rust.so");
    let test_executable = std::env::current_exe().expect("current test executable");
    let profile_dir = test_executable
        .parent()
        .and_then(Path::parent)
        .expect("Cargo profile directory");
    let profile_library = profile_dir.join("libcharinbuf_lib.so");
    let rust_library = if profile_library.is_file() {
        profile_library
    } else {
        manifest.join("target/release/libcharinbuf_lib.so")
    };
    assert!(c_library.is_file(), "missing {}", c_library.display());
    assert!(rust_library.is_file(), "missing {}", rust_library.display());
    (c_library, rust_library)
}

unsafe fn load_apis() -> (Api, Api) {
    let (c_path, rust_path) = library_paths();
    (unsafe { Api::load(&c_path) }, unsafe {
        Api::load(&rust_path)
    })
}

unsafe fn capture_stdout(call: impl FnOnce() -> c_int) -> (c_int, Vec<u8>) {
    assert_eq!(unsafe { fflush(ptr::null_mut()) }, 0);
    let mut fds = [-1; 2];
    assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0);
    let saved_stdout = unsafe { dup(1) };
    assert!(saved_stdout >= 0);
    assert_eq!(unsafe { dup2(fds[1], 1) }, 1);
    assert_eq!(unsafe { close(fds[1]) }, 0);

    let result = call();
    assert_eq!(unsafe { fflush(ptr::null_mut()) }, 0);
    assert_eq!(unsafe { dup2(saved_stdout, 1) }, 1);
    assert_eq!(unsafe { close(saved_stdout) }, 0);

    let mut output = Vec::new();
    let mut chunk = [0_u8; 1024];
    loop {
        let count = unsafe { read(fds[0], chunk.as_mut_ptr().cast(), chunk.len()) };
        assert!(count >= 0);
        if count == 0 {
            break;
        }
        output.extend_from_slice(&chunk[..count as usize]);
    }
    assert_eq!(unsafe { close(fds[0]) }, 0);
    (result, output)
}

unsafe fn compare_charinbuf(c: &Api, rust: &Api, mode: i32, value: i32, opt1: i32, opt2: i32) {
    let c_output = unsafe { capture_stdout(|| (c.charinbuf)(mode, value, opt1, opt2)) };
    let rust_output = unsafe { capture_stdout(|| (rust.charinbuf)(mode, value, opt1, opt2)) };
    assert_eq!(
        c_output, rust_output,
        "charinbuf divergence for ({mode}, {value}, {opt1}, {opt2})"
    );
}

unsafe fn find_offset(function: FindFn, bytes: &[u8], size: usize, target: u8) -> Option<usize> {
    let base = bytes.as_ptr().cast::<c_char>();
    let found = unsafe { function(base, size, target as c_char) };
    if found.is_null() {
        None
    } else {
        Some(unsafe { found.offset_from(base) as usize })
    }
}

unsafe fn copied_bytes(function: CreateFn, input: &[u8]) -> Option<Vec<u8>> {
    let output = unsafe { function(input.as_ptr().cast()) };
    if output.is_null() {
        return None;
    }
    let bytes = unsafe { CStr::from_ptr(output) }
        .to_bytes_with_nul()
        .to_vec();
    unsafe { free(output.cast()) };
    Some(bytes)
}

unsafe extern "C" fn external_callback(value: c_int) -> c_int {
    value.wrapping_mul(3).wrapping_add(1)
}

#[test]
fn valid_configuration_surface_matches() {
    let _guard = PROCESS_LOCK.lock().unwrap();
    let (c, rust) = unsafe { load_apis() };
    let mut rng = Rng::new();

    // CONFIGS 1: reset over the complete int shape.
    for value in [i32::MIN, -1, 0, 1, i32::MAX]
        .into_iter()
        .chain((0..256).map(|_| rng.next_i32()))
    {
        assert_eq!(unsafe { (c.reset_counter)(value) }, unsafe {
            (rust.reset_counter)(value)
        });
    }

    // CONFIGS 2-4: individual arithmetic operations with controlled state.
    for _ in 0..256 {
        let base = rng.range_i32(-1_000_000, 1_000_000);
        let operand = rng.range_i32(-1_000_000, 1_000_000);
        unsafe {
            (c.reset_counter)(base);
            (rust.reset_counter)(base);
        }
        assert_eq!(unsafe { (c.increment_counter)(operand) }, unsafe {
            (rust.increment_counter)(operand)
        });

        unsafe {
            (c.reset_counter)(base);
            (rust.reset_counter)(base);
        }
        assert_eq!(unsafe { (c.decrement_counter)(operand) }, unsafe {
            (rust.decrement_counter)(operand)
        });

        let small_base = rng.range_i32(-10_000, 10_000);
        let multiplier = rng.range_i32(-100, 100);
        unsafe {
            (c.reset_counter)(small_base);
            (rust.reset_counter)(small_base);
        }
        assert_eq!(unsafe { (c.multiply_counter)(multiplier) }, unsafe {
            (rust.multiply_counter)(multiplier)
        });
    }

    // CONFIGS 5: stateful and wrapping sequences.
    unsafe {
        assert_eq!((c.reset_counter)(i32::MAX), (rust.reset_counter)(i32::MAX));
        assert_eq!((c.increment_counter)(1), (rust.increment_counter)(1));
        assert_eq!((c.multiply_counter)(-1), (rust.multiply_counter)(-1));
        assert_eq!(
            (c.decrement_counter)(i32::MAX),
            (rust.decrement_counter)(i32::MAX)
        );
    }
    for _ in 0..256 {
        let operand = rng.next_i32();
        let operation = rng.next_u64() % 3;
        let (c_result, rust_result) = unsafe {
            match operation {
                0 => (
                    (c.increment_counter)(operand),
                    (rust.increment_counter)(operand),
                ),
                1 => (
                    (c.decrement_counter)(operand),
                    (rust.decrement_counter)(operand),
                ),
                _ => (
                    (c.multiply_counter)(operand),
                    (rust.multiply_counter)(operand),
                ),
            }
        };
        assert_eq!(c_result, rust_result);
    }

    // CONFIGS 6-7: valid empty and non-empty strings.
    let empty = [0_u8];
    assert_eq!(
        unsafe { (c.is_string_empty)(empty.as_ptr().cast()) },
        unsafe { (rust.is_string_empty)(empty.as_ptr().cast()) }
    );
    for _ in 0..256 {
        let first = loop {
            let byte = rng.byte();
            if byte != 0 {
                break byte;
            }
        };
        let input = [first, rng.byte(), 0];
        assert_eq!(
            unsafe { (c.is_string_empty)(input.as_ptr().cast()) },
            unsafe { (rust.is_string_empty)(input.as_ptr().cast()) }
        );
    }

    // CONFIGS 8-14: all buffer length/target shapes.
    let sample = [b'a', 0, b'b', b'X', b'z'];
    assert_eq!(
        unsafe { find_offset(c.find_char_in_buffer, &sample, 0, b'a') },
        unsafe { find_offset(rust.find_char_in_buffer, &sample, 0, b'a') }
    );
    assert_eq!(
        unsafe { find_offset(c.find_char_in_buffer, &[b'Q'], 1, b'Q') },
        unsafe { find_offset(rust.find_char_in_buffer, &[b'Q'], 1, b'Q') }
    );
    assert_eq!(
        unsafe { find_offset(c.find_char_in_buffer, &[b'Q'], 1, b'R') },
        unsafe { find_offset(rust.find_char_in_buffer, &[b'Q'], 1, b'R') }
    );
    for _ in 0..256 {
        let length = (rng.next_u64() % 256 + 3) as usize;
        let target = rng.byte();
        let mut bytes = (0..length).map(|_| rng.byte()).collect::<Vec<_>>();
        for byte in &mut bytes {
            if *byte == target {
                *byte = target.wrapping_add(1);
            }
        }
        let position = match rng.next_u64() % 3 {
            0 => 0,
            1 => length / 2,
            _ => length - 1,
        };
        bytes[position] = target;
        assert_eq!(
            unsafe { find_offset(c.find_char_in_buffer, &bytes, bytes.len(), target) },
            unsafe { find_offset(rust.find_char_in_buffer, &bytes, bytes.len(), target) }
        );
        bytes[position] = target.wrapping_add(1);
        assert_eq!(
            unsafe { find_offset(c.find_char_in_buffer, &bytes, bytes.len(), target) },
            unsafe { find_offset(rust.find_char_in_buffer, &bytes, bytes.len(), target) }
        );
    }
    assert_eq!(
        unsafe { find_offset(c.find_char_in_buffer, &sample, sample.len(), 0) },
        unsafe { find_offset(rust.find_char_in_buffer, &sample, sample.len(), 0) }
    );
    let oversized = vec![b'X'; 64];
    assert_eq!(
        unsafe { find_offset(c.find_char_in_buffer, &oversized, usize::MAX, b'X') },
        unsafe { find_offset(rust.find_char_in_buffer, &oversized, usize::MAX, b'X') }
    );

    // CONFIGS 15-17: C-string allocation and copying.
    assert_eq!(unsafe { copied_bytes(c.create_buffer, &empty) }, unsafe {
        copied_bytes(rust.create_buffer, &empty)
    });
    for _ in 0..256 {
        let length = (rng.next_u64() % 512 + 1) as usize;
        let mut input = (0..length)
            .map(|_| {
                let byte = rng.byte();
                if byte == 0 { 1 } else { byte }
            })
            .collect::<Vec<_>>();
        input.push(0);
        assert_eq!(unsafe { copied_bytes(c.create_buffer, &input) }, unsafe {
            copied_bytes(rust.create_buffer, &input)
        });
    }
    let embedded_nul = b"prefix\0ignored suffix\0";
    assert_eq!(
        unsafe { copied_bytes(c.create_buffer, embedded_nul) },
        unsafe { copied_bytes(rust.create_buffer, embedded_nul) }
    );

    // CONFIGS 18-20: complete valid uint16 range shapes.
    for value in [0, 65_535] {
        assert_eq!(unsafe { (c.validate_uint16_range)(value) }, unsafe {
            (rust.validate_uint16_range)(value)
        });
    }
    for _ in 0..512 {
        let value = rng.range_i32(1, 65_534);
        assert_eq!(unsafe { (c.validate_uint16_range)(value) }, unsafe {
            (rust.validate_uint16_range)(value)
        });
    }

    // CONFIGS 21-22: external and every exported counter callback.
    for _ in 0..256 {
        let value = rng.next_i32();
        assert_eq!(
            unsafe { (c.apply_operation)(Some(external_callback), value) },
            unsafe { (rust.apply_operation)(Some(external_callback), value) }
        );
    }
    let callback_pairs = [
        (c.increment_counter, rust.increment_counter),
        (c.decrement_counter, rust.decrement_counter),
        (c.multiply_counter, rust.multiply_counter),
        (c.reset_counter, rust.reset_counter),
    ];
    for (c_callback, rust_callback) in callback_pairs {
        for _ in 0..128 {
            let initial = rng.range_i32(-1_000, 1_000);
            let value = rng.range_i32(-1_000, 1_000);
            unsafe {
                (c.reset_counter)(initial);
                (rust.reset_counter)(initial);
            }
            assert_eq!(
                unsafe { (c.apply_operation)(Some(c_callback), value) },
                unsafe { (rust.apply_operation)(Some(rust_callback), value) }
            );
        }
    }

    // CONFIGS 23-28: every valid top-level mode and its argument axes.
    let mut mode_zero_values = vec![0, 65_535];
    mode_zero_values.extend((0..128).map(|_| rng.range_i32(1, 65_534)));
    for value in mode_zero_values {
        unsafe {
            compare_charinbuf(&c, &rust, 0, value, rng.next_i32(), rng.next_i32());
        }
    }
    for _ in 0..64 {
        unsafe {
            compare_charinbuf(&c, &rust, 1, rng.next_i32(), rng.next_i32(), rng.next_i32());
            compare_charinbuf(&c, &rust, 2, rng.next_i32(), rng.next_i32(), rng.next_i32());
            compare_charinbuf(&c, &rust, 4, rng.next_i32(), rng.next_i32(), rng.next_i32());
            compare_charinbuf(
                &c,
                &rust,
                3,
                rng.range_i32(-1_000, 1_000),
                rng.range_i32(-1_000, 1_000),
                rng.range_i32(-100, 100),
            );
        }
    }
    for arguments in [
        (i32::MAX, 1, -1),
        (i32::MIN, -1, -1),
        (i32::MAX, i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN, i32::MIN),
    ] {
        unsafe {
            compare_charinbuf(&c, &rust, 3, arguments.0, arguments.1, arguments.2);
        }
    }
}

#[test]
fn direct_error_surface_matches() {
    let _guard = PROCESS_LOCK.lock().unwrap();
    let (c, rust) = unsafe { load_apis() };
    let mut rng = Rng::new();

    // ERRORS 1-3 and generic null/zero/oversized pointer boundaries.
    assert_eq!(unsafe { (c.is_string_empty)(ptr::null()) }, unsafe {
        (rust.is_string_empty)(ptr::null())
    });
    for size in [0, 1, usize::MAX] {
        assert_eq!(
            unsafe { (c.find_char_in_buffer)(ptr::null(), size, b'X' as c_char) },
            unsafe { (rust.find_char_in_buffer)(ptr::null(), size, b'X' as c_char) }
        );
    }
    assert_eq!(unsafe { (c.create_buffer)(ptr::null()) }, unsafe {
        (rust.create_buffer)(ptr::null())
    });

    // ERRORS 5-7: both sides must return the exact C sentinel.
    for value in [i32::MIN, -65_536, -2, -1]
        .into_iter()
        .chain((0..256).map(|_| rng.range_i32(i32::MIN, -1)))
    {
        assert_eq!(unsafe { (c.validate_uint16_range)(value) }, 0);
        assert_eq!(unsafe { (c.validate_uint16_range)(value) }, unsafe {
            (rust.validate_uint16_range)(value)
        });
    }
    for value in [65_536, 65_537, i32::MAX]
        .into_iter()
        .chain((0..256).map(|_| rng.range_i32(65_536, i32::MAX)))
    {
        assert_eq!(unsafe { (c.validate_uint16_range)(value) }, 0);
        assert_eq!(unsafe { (c.validate_uint16_range)(value) }, unsafe {
            (rust.validate_uint16_range)(value)
        });
    }
    for value in [i32::MIN, -1, 0, 1, i32::MAX] {
        assert_eq!(unsafe { (c.apply_operation)(None, value) }, -1);
        assert_eq!(unsafe { (c.apply_operation)(None, value) }, unsafe {
            (rust.apply_operation)(None, value)
        });
    }

    // ERRORS 8-9 and 13: mode-specific and out-of-range discriminants.
    for _ in 0..128 {
        unsafe {
            compare_charinbuf(
                &c,
                &rust,
                0,
                rng.range_i32(i32::MIN, -1),
                rng.next_i32(),
                rng.next_i32(),
            );
            compare_charinbuf(
                &c,
                &rust,
                0,
                rng.range_i32(65_536, i32::MAX),
                rng.next_i32(),
                rng.next_i32(),
            );
        }
    }
    for mode in [i32::MIN, -100, -1, 5, 6, 100, i32::MAX] {
        unsafe {
            let (result, _) = capture_stdout(|| (c.charinbuf)(mode, rng.next_i32(), 0, 0));
            assert_eq!(result, -1);
            compare_charinbuf(&c, &rust, mode, rng.next_i32(), rng.next_i32(), 0);
        }
    }
}

fn fault_library_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let test_executable = std::env::current_exe().expect("current test executable");
    test_executable
        .parent()
        .expect("test deps directory")
        .join("libcharinbuf_fault_inject.so")
        .tap(|path| {
            let status = Command::new("cc")
                .args(["-shared", "-fPIC", "-O2"])
                .arg(manifest.join("tests/fault_inject.c"))
                .arg("-o")
                .arg(path)
                .status()
                .expect("compile fault injection library");
            assert!(status.success());
        })
}

trait Tap: Sized {
    fn tap(self, function: impl FnOnce(&Self)) -> Self {
        function(&self);
        self
    }
}
impl<T> Tap for T {}

#[test]
fn allocator_and_memchr_failure_paths_match() {
    if std::env::var_os("CHARINBUF_FAULT_CHILD").is_none() {
        let fault_library = fault_library_path();
        let status = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "allocator_and_memchr_failure_paths_match",
                "--nocapture",
            ])
            .env("CHARINBUF_FAULT_CHILD", "1")
            .env("LD_PRELOAD", fault_library)
            .status()
            .expect("run preloaded fault-path child");
        assert!(status.success());
        return;
    }

    let _guard = PROCESS_LOCK.lock().unwrap();
    unsafe {
        type FailMallocFn = unsafe extern "C" fn(usize);
        type FailMemchrFn = unsafe extern "C" fn(c_int);
        let process = libloading::os::unix::Library::this();
        let fail_malloc = *process
            .get::<FailMallocFn>(b"fault_fail_next_malloc_size\0")
            .expect("preloaded malloc fault control");
        let fail_memchr = *process
            .get::<FailMemchrFn>(b"fault_fail_next_memchr_byte\0")
            .expect("preloaded memchr fault control");
        let (c, rust) = load_apis();

        // ERRORS 4: create_buffer propagates malloc failure as NULL.
        let input = b"allocation failure\0";
        fail_malloc(input.len());
        let c_result = (c.create_buffer)(input.as_ptr().cast());
        fail_malloc(input.len());
        let rust_result = (rust.create_buffer)(input.as_ptr().cast());
        assert!(c_result.is_null());
        assert_eq!(c_result.is_null(), rust_result.is_null());

        // ERRORS 10: mode 2 allocation failure.
        fail_malloc(b"Testing malloc and free\0".len());
        let c_result = capture_stdout(|| (c.charinbuf)(2, 0, 0, 0));
        fail_malloc(b"Testing malloc and free\0".len());
        let rust_result = capture_stdout(|| (rust.charinbuf)(2, 0, 0, 0));
        assert_eq!(c_result.0, -1);
        assert_eq!(c_result, rust_result);

        // ERRORS 11: mode 4 allocation failure preserves initialized result 0.
        fail_malloc(b"Search for character X in this buffer\0".len());
        let c_result = capture_stdout(|| (c.charinbuf)(4, 0, 0, 0));
        fail_malloc(b"Search for character X in this buffer\0".len());
        let rust_result = capture_stdout(|| (rust.charinbuf)(4, 0, 0, 0));
        assert_eq!(c_result.0, 0);
        assert_eq!(c_result, rust_result);

        // ERRORS 12: mode 4 search failure.
        fail_memchr(c_int::from(b'X'));
        let c_result = capture_stdout(|| (c.charinbuf)(4, 0, 0, 0));
        fail_memchr(c_int::from(b'X'));
        let rust_result = capture_stdout(|| (rust.charinbuf)(4, 0, 0, 0));
        assert_eq!(c_result.0, -1);
        assert_eq!(c_result, rust_result);
    }
}
