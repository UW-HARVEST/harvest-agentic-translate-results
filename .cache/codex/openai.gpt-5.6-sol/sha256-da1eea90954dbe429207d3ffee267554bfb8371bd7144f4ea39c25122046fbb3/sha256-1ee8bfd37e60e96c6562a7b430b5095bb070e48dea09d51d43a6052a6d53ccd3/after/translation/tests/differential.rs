use libloading::Library;
use std::env;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::Mutex;

type IntFn = unsafe extern "C" fn(c_int) -> c_int;
type StringEmptyFn = unsafe extern "C" fn(*const c_char) -> c_int;
type FindCharFn = unsafe extern "C" fn(*const c_char, usize, c_char) -> *mut c_char;
type CreateBufferFn = unsafe extern "C" fn(*const c_char) -> *mut c_char;
type OperationFn = unsafe extern "C" fn(c_int) -> c_int;
type ApplyOperationFn = unsafe extern "C" fn(Option<OperationFn>, c_int) -> c_int;
type CharinbufFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
type FailMallocFn = unsafe extern "C" fn(usize);

static TEST_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn free(ptr: *mut c_void);
    fn pipe(pipe_fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
}

struct Api {
    _library: Library,
    increment_counter: IntFn,
    decrement_counter: IntFn,
    multiply_counter: IntFn,
    reset_counter: IntFn,
    is_string_empty: StringEmptyFn,
    find_char_in_buffer: FindCharFn,
    create_buffer: CreateBufferFn,
    validate_uint16_range: IntFn,
    apply_operation: ApplyOperationFn,
    charinbuf: CharinbufFn,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));

        unsafe fn symbol<T: Copy>(library: &Library, name: &[u8]) -> T {
            *unsafe { library.get::<T>(name) }
                .unwrap_or_else(|error| panic!("missing symbol {:?}: {error}", name))
        }

        let increment_counter = unsafe { symbol(&library, b"increment_counter\0") };
        let decrement_counter = unsafe { symbol(&library, b"decrement_counter\0") };
        let multiply_counter = unsafe { symbol(&library, b"multiply_counter\0") };
        let reset_counter = unsafe { symbol(&library, b"reset_counter\0") };
        let is_string_empty = unsafe { symbol(&library, b"is_string_empty\0") };
        let find_char_in_buffer = unsafe { symbol(&library, b"find_char_in_buffer\0") };
        let create_buffer = unsafe { symbol(&library, b"create_buffer\0") };
        let validate_uint16_range = unsafe { symbol(&library, b"validate_uint16_range\0") };
        let apply_operation = unsafe { symbol(&library, b"apply_operation\0") };
        let charinbuf = unsafe { symbol(&library, b"charinbuf\0") };

        Self {
            _library: library,
            increment_counter,
            decrement_counter,
            multiply_counter,
            reset_counter,
            is_string_empty,
            find_char_in_buffer,
            create_buffer,
            validate_uint16_range,
            apply_operation,
            charinbuf,
        }
    }
}

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
        value as u32
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    fn range_i32(&mut self, low: i32, high: i32) -> i32 {
        assert!(low <= high);
        let width = (high as i64 - low as i64 + 1) as u64;
        low + (u64::from(self.next_u32()) % width) as i32
    }

    fn range_usize(&mut self, low: usize, high: usize) -> usize {
        assert!(low <= high);
        low + self.next_u32() as usize % (high - low + 1)
    }
}

fn library_paths() -> (PathBuf, PathBuf) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_library = manifest
        .parent()
        .unwrap()
        .join("c_src/build/libharvest-work-hklH5o.so");
    let rust_library = manifest.join("target/release/libcharinbuf_lib.so");
    assert!(c_library.is_file(), "missing {}", c_library.display());
    assert!(rust_library.is_file(), "missing {}", rust_library.display());
    (c_library, rust_library)
}

fn load_apis() -> (Api, Api) {
    let (c_path, rust_path) = library_paths();
    unsafe { (Api::load(&c_path), Api::load(&rust_path)) }
}

fn capture_stdout(call: impl FnOnce() -> c_int) -> (c_int, Vec<u8>) {
    let mut pipe_fds = [0; 2];
    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0);
    }
    let saved_stdout = unsafe { dup(1) };
    assert!(saved_stdout >= 0);
    unsafe {
        assert_eq!(dup2(pipe_fds[1], 1), 1);
        assert_eq!(close(pipe_fds[1]), 0);
    }

    let result = call();

    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, 1), 1);
        assert_eq!(close(saved_stdout), 0);
    }

    let mut output = Vec::new();
    let mut chunk = [0_u8; 1024];
    loop {
        let count = unsafe { read(pipe_fds[0], chunk.as_mut_ptr().cast(), chunk.len()) };
        assert!(count >= 0);
        if count == 0 {
            break;
        }
        output.extend_from_slice(&chunk[..count as usize]);
    }
    unsafe {
        assert_eq!(close(pipe_fds[0]), 0);
    }
    (result, output)
}

fn compare_charinbuf(
    c_api: &Api,
    rust_api: &Api,
    mode: i32,
    value: i32,
    opt1: i32,
    opt2: i32,
) -> i32 {
    let c_output = capture_stdout(|| unsafe { (c_api.charinbuf)(mode, value, opt1, opt2) });
    let rust_output = capture_stdout(|| unsafe { (rust_api.charinbuf)(mode, value, opt1, opt2) });
    assert_eq!(
        c_output, rust_output,
        "charinbuf mismatch for ({mode}, {value}, {opt1}, {opt2})"
    );
    c_output.0
}

fn find_offset(function: FindCharFn, bytes: &[u8], size: usize, target: u8) -> Option<usize> {
    let pointer = unsafe { function(bytes.as_ptr().cast(), size, target as i8 as c_char) };
    (!pointer.is_null()).then(|| pointer as usize - bytes.as_ptr() as usize)
}

fn compare_created_buffer(c_api: &Api, rust_api: &Api, input: &CString) {
    let c_buffer = unsafe { (c_api.create_buffer)(input.as_ptr()) };
    let rust_buffer = unsafe { (rust_api.create_buffer)(input.as_ptr()) };
    assert_eq!(c_buffer.is_null(), rust_buffer.is_null());
    assert!(!c_buffer.is_null());
    let c_bytes = unsafe { CStr::from_ptr(c_buffer) }.to_bytes_with_nul();
    let rust_bytes = unsafe { CStr::from_ptr(rust_buffer) }.to_bytes_with_nul();
    assert_eq!(c_bytes, rust_bytes);
    assert_eq!(c_bytes, input.as_bytes_with_nul());
    unsafe {
        free(c_buffer.cast());
        free(rust_buffer.cast());
    }
}

unsafe extern "C" fn external_operation(value: c_int) -> c_int {
    value.wrapping_mul(3).wrapping_add(7)
}

#[test]
fn valid_surface() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let (c_api, rust_api) = load_apis();
    let mut rng = Rng::new(0xd1ff_e2e0_5eed_1234);

    // CONFIGS 1-5: all low-level counter exports and stateful composition.
    for value in [i32::MIN, 0, i32::MAX]
        .into_iter()
        .chain((0..256).map(|_| rng.next_i32()))
    {
        assert_eq!(unsafe { (c_api.reset_counter)(value) }, unsafe {
            (rust_api.reset_counter)(value)
        });
    }
    for _ in 0..256 {
        let initial = rng.range_i32(-1_000_000, 1_000_000);
        let operand = rng.range_i32(-1_000_000, 1_000_000);

        unsafe {
            (c_api.reset_counter)(initial);
            (rust_api.reset_counter)(initial);
        }
        assert_eq!(unsafe { (c_api.increment_counter)(operand) }, unsafe {
            (rust_api.increment_counter)(operand)
        });

        unsafe {
            (c_api.reset_counter)(initial);
            (rust_api.reset_counter)(initial);
        }
        assert_eq!(unsafe { (c_api.decrement_counter)(operand) }, unsafe {
            (rust_api.decrement_counter)(operand)
        });

        let left = rng.range_i32(-30_000, 30_000);
        let right = rng.range_i32(-30_000, 30_000);
        unsafe {
            (c_api.reset_counter)(left);
            (rust_api.reset_counter)(left);
        }
        assert_eq!(unsafe { (c_api.multiply_counter)(right) }, unsafe {
            (rust_api.multiply_counter)(right)
        });

        let start = rng.range_i32(-10_000, 10_000);
        let add = rng.range_i32(-10_000, 10_000);
        let factor = rng.range_i32(-100, 100);
        let subtract = rng.range_i32(-10_000, 10_000);
        unsafe {
            (c_api.reset_counter)(start);
            (rust_api.reset_counter)(start);
        }
        assert_eq!(unsafe { (c_api.increment_counter)(add) }, unsafe {
            (rust_api.increment_counter)(add)
        });
        assert_eq!(unsafe { (c_api.multiply_counter)(factor) }, unsafe {
            (rust_api.multiply_counter)(factor)
        });
        assert_eq!(unsafe { (c_api.decrement_counter)(subtract) }, unsafe {
            (rust_api.decrement_counter)(subtract)
        });
    }

    // CONFIGS 6-7: empty and non-empty valid strings.
    for _ in 0..256 {
        let empty_with_tail = [0_u8, rng.next_u32() as u8, rng.next_u32() as u8];
        assert_eq!(
            unsafe { (c_api.is_string_empty)(empty_with_tail.as_ptr().cast()) },
            unsafe { (rust_api.is_string_empty)(empty_with_tail.as_ptr().cast()) }
        );
        let first = rng.range_i32(1, 255) as u8;
        let non_empty = [first, 0];
        assert_eq!(
            unsafe { (c_api.is_string_empty)(non_empty.as_ptr().cast()) },
            unsafe { (rust_api.is_string_empty)(non_empty.as_ptr().cast()) }
        );
    }

    // CONFIGS 8-11: zero length, present/absent targets, and all byte values.
    for _ in 0..512 {
        let length = rng.range_usize(1, 256);
        let mut bytes: Vec<u8> = (0..length).map(|_| rng.next_u32() as u8).collect();
        let target = rng.next_u32() as u8;
        assert_eq!(
            find_offset(c_api.find_char_in_buffer, &bytes, 0, target),
            find_offset(rust_api.find_char_in_buffer, &bytes, 0, target)
        );

        let position = rng.range_usize(0, length - 1);
        bytes[position] = target;
        assert_eq!(
            find_offset(c_api.find_char_in_buffer, &bytes, length, target),
            find_offset(rust_api.find_char_in_buffer, &bytes, length, target)
        );

        let absent_target = bytes.iter().copied().fold(0_u8, |candidate, byte| {
            if candidate == byte {
                candidate.wrapping_add(1)
            } else {
                candidate
            }
        });
        let prefix = bytes
            .iter()
            .position(|byte| *byte == absent_target)
            .unwrap_or(bytes.len());
        assert_eq!(
            find_offset(c_api.find_char_in_buffer, &bytes, prefix, absent_target),
            find_offset(rust_api.find_char_in_buffer, &bytes, prefix, absent_target)
        );
    }
    let large_buffer = vec![0x5a_u8; 65_537];
    for target in [0_u8, 0x5a, 0x80, 0xff] {
        assert_eq!(
            find_offset(
                c_api.find_char_in_buffer,
                &large_buffer,
                large_buffer.len(),
                target
            ),
            find_offset(
                rust_api.find_char_in_buffer,
                &large_buffer,
                large_buffer.len(),
                target
            )
        );
    }

    // CONFIGS 12-13: allocated copies of empty and varied strings.
    compare_created_buffer(&c_api, &rust_api, &CString::new("").unwrap());
    for _ in 0..256 {
        let length = rng.range_usize(1, 1024);
        let bytes: Vec<u8> = (0..length).map(|_| rng.range_i32(1, 255) as u8).collect();
        compare_created_buffer(&c_api, &rust_api, &CString::new(bytes).unwrap());
    }

    // CONFIGS 14-16: all valid uint16 boundary classes.
    for value in [0, u16::MAX as i32] {
        assert_eq!(unsafe { (c_api.validate_uint16_range)(value) }, unsafe {
            (rust_api.validate_uint16_range)(value)
        });
    }
    for _ in 0..512 {
        let value = rng.range_i32(1, u16::MAX as i32 - 1);
        assert_eq!(unsafe { (c_api.validate_uint16_range)(value) }, unsafe {
            (rust_api.validate_uint16_range)(value)
        });
    }

    // CONFIG 17: non-null callbacks with arbitrary integer arguments.
    for _ in 0..512 {
        let value = rng.next_i32();
        assert_eq!(
            unsafe { (c_api.apply_operation)(Some(external_operation), value) },
            unsafe { (rust_api.apply_operation)(Some(external_operation), value) }
        );
    }

    // CONFIGS 18-24: every successful charinbuf mode and boundary class.
    for _ in 0..64 {
        let opt1 = rng.next_i32();
        let opt2 = rng.next_i32();
        assert_eq!(compare_charinbuf(&c_api, &rust_api, 0, 0, opt1, opt2), 0);
        let value = rng.range_i32(1, u16::MAX as i32 - 1);
        assert_eq!(
            compare_charinbuf(&c_api, &rust_api, 0, value, opt1, opt2),
            value
        );
        assert_eq!(
            compare_charinbuf(&c_api, &rust_api, 0, u16::MAX as i32, opt1, opt2),
            u16::MAX as i32
        );
        assert_eq!(
            compare_charinbuf(
                &c_api,
                &rust_api,
                1,
                rng.next_i32(),
                rng.next_i32(),
                rng.next_i32()
            ),
            10
        );
        assert_eq!(
            compare_charinbuf(
                &c_api,
                &rust_api,
                2,
                rng.next_i32(),
                rng.next_i32(),
                rng.next_i32()
            ),
            23
        );

        let value = rng.range_i32(-10_000, 10_000);
        let add = rng.range_i32(-10_000, 10_000);
        let factor = rng.range_i32(-100, 100);
        let expected = (value + add) * factor - 5;
        assert_eq!(
            compare_charinbuf(&c_api, &rust_api, 3, value, add, factor),
            expected
        );
        assert_eq!(
            compare_charinbuf(
                &c_api,
                &rust_api,
                4,
                rng.next_i32(),
                rng.next_i32(),
                rng.next_i32()
            ),
            21
        );
    }
}

#[test]
fn error_surface() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let (c_api, rust_api) = load_apis();
    let mut rng = Rng::new(0xbad0_5eed_ffee_1234);

    // ERRORS 1-3 and generic null/zero/oversized pointer-length boundaries.
    assert_eq!(unsafe { (c_api.is_string_empty)(ptr::null()) }, unsafe {
        (rust_api.is_string_empty)(ptr::null())
    });
    for size in [0, 1, 65_537, usize::MAX] {
        assert_eq!(
            unsafe { (c_api.find_char_in_buffer)(ptr::null(), size, 0) },
            unsafe { (rust_api.find_char_in_buffer)(ptr::null(), size, 0) }
        );
    }
    assert_eq!(unsafe { (c_api.create_buffer)(ptr::null()) }, unsafe {
        (rust_api.create_buffer)(ptr::null())
    });

    // ERRORS 5-6: both rejected uint16 ranges, including one-step-past.
    for value in [-1, i32::MIN] {
        assert_eq!(unsafe { (c_api.validate_uint16_range)(value) }, unsafe {
            (rust_api.validate_uint16_range)(value)
        });
    }
    for value in [u16::MAX as i32 + 1, i32::MAX] {
        assert_eq!(unsafe { (c_api.validate_uint16_range)(value) }, unsafe {
            (rust_api.validate_uint16_range)(value)
        });
    }
    for _ in 0..256 {
        let negative = rng.range_i32(i32::MIN, -1);
        let oversized = rng.range_i32(u16::MAX as i32 + 1, i32::MAX);
        assert_eq!(unsafe { (c_api.validate_uint16_range)(negative) }, unsafe {
            (rust_api.validate_uint16_range)(negative)
        });
        assert_eq!(
            unsafe { (c_api.validate_uint16_range)(oversized) },
            unsafe { (rust_api.validate_uint16_range)(oversized) }
        );
    }

    // ERROR 7: nullable function pointer.
    for _ in 0..256 {
        let value = rng.next_i32();
        assert_eq!(unsafe { (c_api.apply_operation)(None, value) }, unsafe {
            (rust_api.apply_operation)(None, value)
        });
    }

    // ERRORS 8-9: mode 0 forwards both invalid range classes.
    for value in [-1, i32::MIN, u16::MAX as i32 + 1, i32::MAX] {
        assert_eq!(
            compare_charinbuf(&c_api, &rust_api, 0, value, rng.next_i32(), rng.next_i32()),
            -1
        );
    }
    for _ in 0..128 {
        let value = if rng.next_u32() & 1 == 0 {
            rng.range_i32(i32::MIN, -1)
        } else {
            rng.range_i32(u16::MAX as i32 + 1, i32::MAX)
        };
        assert_eq!(
            compare_charinbuf(&c_api, &rust_api, 0, value, rng.next_i32(), rng.next_i32()),
            -1
        );
    }

    // ERROR 11: out-of-range mode/enum values at and beyond both boundaries.
    for mode in [i32::MIN, -2, -1, 5, 6, i32::MAX] {
        assert_eq!(
            compare_charinbuf(
                &c_api,
                &rust_api,
                mode,
                rng.next_i32(),
                rng.next_i32(),
                rng.next_i32()
            ),
            -1
        );
    }
    for _ in 0..256 {
        let mode = if rng.next_u32() & 1 == 0 {
            rng.range_i32(i32::MIN, -1)
        } else {
            rng.range_i32(5, i32::MAX)
        };
        assert_eq!(
            compare_charinbuf(
                &c_api,
                &rust_api,
                mode,
                rng.next_i32(),
                rng.next_i32(),
                rng.next_i32()
            ),
            -1
        );
    }
}

fn interposer_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/test-support/libfailmalloc.so")
}

fn compile_interposer(path: &Path) {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/support/failmalloc.c");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let status = Command::new("cc")
        .args(["-std=c11", "-shared", "-fPIC"])
        .arg(&source)
        .arg("-o")
        .arg(path)
        .status()
        .expect("failed to invoke C compiler for malloc interposer");
    assert!(status.success());
    let metadata = fs::metadata(path).expect("malloc interposer was not produced");
    assert!(metadata.len() > 0);
}

#[test]
fn allocation_failure_surface() {
    const CHILD_ENV: &str = "CHARINBUF_FAILMALLOC_CHILD";
    let interposer = interposer_path();

    if env::var_os(CHILD_ENV).is_none() {
        compile_interposer(&interposer);
        let status = Command::new(env::current_exe().unwrap())
            .args(["--exact", "allocation_failure_surface", "--nocapture"])
            .env(CHILD_ENV, "1")
            .env("LD_PRELOAD", &interposer)
            .status()
            .expect("failed to launch allocation-failure test subprocess");
        assert!(status.success());
        return;
    }

    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let (c_api, rust_api) = load_apis();
    let control_library = unsafe { Library::new(&interposer) }.unwrap();
    let fail_malloc: FailMallocFn =
        *unsafe { control_library.get(b"fail_next_malloc_of_size\0") }.unwrap();

    // ERROR 4: create_buffer propagates malloc failure.
    for length in [0_usize, 1, 17, 1024] {
        let input = CString::new(vec![b'x'; length]).unwrap();
        unsafe { fail_malloc(input.as_bytes_with_nul().len()) };
        let c_result = unsafe { (c_api.create_buffer)(input.as_ptr()) };
        unsafe { fail_malloc(input.as_bytes_with_nul().len()) };
        let rust_result = unsafe { (rust_api.create_buffer)(input.as_ptr()) };
        assert_eq!(c_result, ptr::null_mut());
        assert_eq!(rust_result, ptr::null_mut());
    }

    let mut rng = Rng::new(0xa110_ca7e_fa11_0001);

    // ERROR 10: charinbuf mode 2 maps fixed-string allocation failure to -1.
    for _ in 0..64 {
        let c_output = capture_stdout(|| unsafe {
            fail_malloc(b"Testing malloc and free\0".len());
            (c_api.charinbuf)(2, rng.next_i32(), rng.next_i32(), rng.next_i32())
        });
        let rust_args = (rng.next_i32(), rng.next_i32(), rng.next_i32());
        let rust_output = capture_stdout(|| unsafe {
            fail_malloc(b"Testing malloc and free\0".len());
            (rust_api.charinbuf)(2, rust_args.0, rust_args.1, rust_args.2)
        });
        assert_eq!(c_output.0, -1);
        assert_eq!(rust_output.0, -1);
        assert_eq!(c_output.1, rust_output.1);
    }

    // CONFIG 25: mode 4 allocation failure leaves the initialized result zero.
    for _ in 0..64 {
        let args = (rng.next_i32(), rng.next_i32(), rng.next_i32());
        let c_output = capture_stdout(|| unsafe {
            fail_malloc(b"Search for character X in this buffer\0".len());
            (c_api.charinbuf)(4, args.0, args.1, args.2)
        });
        let rust_output = capture_stdout(|| unsafe {
            fail_malloc(b"Search for character X in this buffer\0".len());
            (rust_api.charinbuf)(4, args.0, args.1, args.2)
        });
        assert_eq!(c_output.0, 0);
        assert_eq!(rust_output.0, 0);
        assert_eq!(c_output.1, rust_output.1);
    }

    drop(control_library);
}
