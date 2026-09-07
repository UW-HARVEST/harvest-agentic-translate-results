use libloading::Library;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::ptr;
use std::sync::Mutex;

type CounterFn = unsafe extern "C" fn(c_int) -> c_int;
type EmptyFn = unsafe extern "C" fn(*const c_char) -> c_int;
type FindFn = unsafe extern "C" fn(*const c_char, usize, c_char) -> *mut c_char;
type CreateFn = unsafe extern "C" fn(*const c_char) -> *mut c_char;
type ValidateFn = unsafe extern "C" fn(c_int) -> c_int;
type ApplyFn = unsafe extern "C" fn(Option<CounterFn>, c_int) -> c_int;
type CharinbufFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn free(ptr: *mut c_void);
    fn pipe(pipe_fds: *mut c_int) -> c_int;
}

static PROCESS_STATE: Mutex<()> = Mutex::new(());

struct Api {
    _library: Library,
    increment: CounterFn,
    decrement: CounterFn,
    multiply: CounterFn,
    reset: CounterFn,
    is_empty: EmptyFn,
    find: FindFn,
    create: CreateFn,
    validate: ValidateFn,
    apply: ApplyFn,
    charinbuf: CharinbufFn,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let increment = unsafe { *library.get(b"increment_counter\0").unwrap() };
        let decrement = unsafe { *library.get(b"decrement_counter\0").unwrap() };
        let multiply = unsafe { *library.get(b"multiply_counter\0").unwrap() };
        let reset = unsafe { *library.get(b"reset_counter\0").unwrap() };
        let is_empty = unsafe { *library.get(b"is_string_empty\0").unwrap() };
        let find = unsafe { *library.get(b"find_char_in_buffer\0").unwrap() };
        let create = unsafe { *library.get(b"create_buffer\0").unwrap() };
        let validate = unsafe { *library.get(b"validate_uint16_range\0").unwrap() };
        let apply = unsafe { *library.get(b"apply_operation\0").unwrap() };
        let charinbuf = unsafe { *library.get(b"charinbuf\0").unwrap() };
        Self {
            _library: library,
            increment,
            decrement,
            multiply,
            reset,
            is_empty,
            find,
            create,
            validate,
            apply,
            charinbuf,
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
        self.next_u64() as u32 as i32
    }

    fn range_i32(&mut self, minimum: i32, maximum: i32) -> i32 {
        let width = (maximum as i64 - minimum as i64 + 1) as u64;
        minimum + (self.next_u64() % width) as i32
    }

    fn range_usize(&mut self, minimum: usize, maximum: usize) -> usize {
        minimum + (self.next_u64() % (maximum - minimum + 1) as u64) as usize
    }
}

fn library_paths() -> (PathBuf, PathBuf) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    (
        root.join("../c_src/build/libharvest-work-caSExF.so"),
        root.join("target/release/libcharinbuf_lib.so"),
    )
}

unsafe fn load_pair() -> (Api, Api) {
    let (c_path, rust_path) = library_paths();
    assert!(c_path.is_file(), "missing C library: {}", c_path.display());
    assert!(
        rust_path.is_file(),
        "missing Rust library: {}",
        rust_path.display()
    );
    (unsafe { Api::load(&c_path) }, unsafe {
        Api::load(&rust_path)
    })
}

fn capture_stdout(function: impl FnOnce() -> c_int) -> (c_int, Vec<u8>) {
    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);

        let mut pipe_fds = [0; 2];
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0);
        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0);
        assert_eq!(dup2(pipe_fds[1], 1), 1);
        assert_eq!(close(pipe_fds[1]), 0);

        let result = function();
        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, 1), 1);
        assert_eq!(close(saved_stdout), 0);

        let mut bytes = Vec::new();
        let mut reader = File::from_raw_fd(pipe_fds[0]);
        reader.read_to_end(&mut bytes).unwrap();
        (result, bytes)
    }
}

fn assert_charinbuf_equal(c: &Api, rust: &Api, args: [c_int; 4]) -> (c_int, Vec<u8>) {
    let c_result = capture_stdout(|| unsafe { (c.charinbuf)(args[0], args[1], args[2], args[3]) });
    let rust_result =
        capture_stdout(|| unsafe { (rust.charinbuf)(args[0], args[1], args[2], args[3]) });
    assert_eq!(c_result, rust_result, "charinbuf mismatch for {args:?}");
    c_result
}

fn pointer_offset(base: *const c_char, found: *mut c_char) -> Option<usize> {
    if found.is_null() {
        None
    } else {
        Some(unsafe { found.offset_from(base) as usize })
    }
}

#[test]
fn valid_counter_and_function_pointer_configurations() {
    let _guard = PROCESS_STATE.lock().unwrap();
    let (c, rust) = unsafe { load_pair() };
    let mut rng = Rng::new(0x4d59_5df4_d0f3_3173);

    for value in [i32::MIN, -1, 0, 1, i32::MAX]
        .into_iter()
        .chain((0..256).map(|_| rng.next_i32()))
    {
        assert_eq!(unsafe { (c.reset)(value) }, unsafe { (rust.reset)(value) });
    }

    for _ in 0..256 {
        let base = rng.next_i32();
        let delta = rng.next_i32();
        unsafe {
            (c.reset)(base);
            (rust.reset)(base);
            assert_eq!((c.increment)(delta), (rust.increment)(delta));

            (c.reset)(base);
            (rust.reset)(base);
            assert_eq!((c.decrement)(delta), (rust.decrement)(delta));
        }
    }

    for _ in 0..256 {
        let base = rng.next_i32();
        let factor = rng.next_i32();
        unsafe {
            (c.reset)(base);
            (rust.reset)(base);
            assert_eq!((c.multiply)(factor), (rust.multiply)(factor));
        }
    }

    for _ in 0..256 {
        let value = rng.next_i32();
        assert_eq!(unsafe { (c.apply)(Some(c.reset), value) }, unsafe {
            (rust.apply)(Some(rust.reset), value)
        });

        let base = rng.next_i32();
        let delta = rng.next_i32();
        unsafe {
            (c.reset)(base);
            (rust.reset)(base);
            assert_eq!(
                (c.apply)(Some(c.increment), delta),
                (rust.apply)(Some(rust.increment), delta)
            );

            (c.reset)(base);
            (rust.reset)(base);
            assert_eq!(
                (c.apply)(Some(c.decrement), delta),
                (rust.apply)(Some(rust.decrement), delta)
            );
        }

        let product_base = rng.next_i32();
        let factor = rng.next_i32();
        unsafe {
            (c.reset)(product_base);
            (rust.reset)(product_base);
            assert_eq!(
                (c.apply)(Some(c.multiply), factor),
                (rust.apply)(Some(rust.multiply), factor)
            );
        }
    }
}

#[test]
fn valid_string_buffer_and_range_configurations() {
    let _guard = PROCESS_STATE.lock().unwrap();
    let (c, rust) = unsafe { load_pair() };
    let mut rng = Rng::new(0xd1b5_4a32_d192_ed03);

    let empty = CString::new("").unwrap();
    assert_eq!(unsafe { (c.is_empty)(empty.as_ptr()) }, unsafe {
        (rust.is_empty)(empty.as_ptr())
    });

    for _ in 0..256 {
        let length = rng.range_usize(1, 128);
        let bytes: Vec<u8> = (0..length).map(|_| rng.range_i32(1, 127) as u8).collect();
        let string = CString::new(bytes).unwrap();
        assert_eq!(unsafe { (c.is_empty)(string.as_ptr()) }, unsafe {
            (rust.is_empty)(string.as_ptr())
        });
    }
    let high_bit_string = CString::new(vec![0x80, 0xff]).unwrap();
    assert_eq!(unsafe { (c.is_empty)(high_bit_string.as_ptr()) }, unsafe {
        (rust.is_empty)(high_bit_string.as_ptr())
    });

    for _ in 0..256 {
        let length = rng.range_usize(1, 128);
        let mut bytes: Vec<u8> = (0..length).map(|_| rng.next_u64() as u8).collect();
        let base = bytes.as_ptr().cast::<c_char>();

        let zero_size_target = rng.next_u64() as u8 as c_char;
        let c_found = unsafe { (c.find)(base, 0, zero_size_target) };
        let rust_found = unsafe { (rust.find)(base, 0, zero_size_target) };
        assert!(c_found.is_null());
        assert!(rust_found.is_null());

        let target = rng.next_u64() as u8;
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
        let base = bytes.as_ptr().cast::<c_char>();
        assert_eq!(
            pointer_offset(base, unsafe { (c.find)(base, length, target as c_char) }),
            pointer_offset(base, unsafe { (rust.find)(base, length, target as c_char) })
        );

        if length > 1 {
            for byte in &mut bytes {
                if *byte == target {
                    *byte = target.wrapping_add(1);
                }
            }
            bytes[length - 1] = target;
            let base = bytes.as_ptr().cast::<c_char>();
            assert_eq!(
                pointer_offset(base, unsafe {
                    (c.find)(base, length - 1, target as c_char)
                }),
                pointer_offset(base, unsafe {
                    (rust.find)(base, length - 1, target as c_char)
                })
            );
        }

        for byte in &mut bytes {
            if *byte == target {
                *byte = target.wrapping_add(1);
            }
        }
        let base = bytes.as_ptr().cast::<c_char>();
        assert_eq!(
            pointer_offset(base, unsafe { (c.find)(base, length, target as c_char) }),
            pointer_offset(base, unsafe { (rust.find)(base, length, target as c_char) })
        );
    }

    for target in [0_u8, 0x7f, 0x80, 0xff] {
        for position in [0_usize, 3, 7] {
            let mut bytes = [target.wrapping_add(1); 8];
            bytes[position] = target;
            let base = bytes.as_ptr().cast::<c_char>();
            assert_eq!(
                pointer_offset(base, unsafe {
                    (c.find)(base, bytes.len(), target as c_char)
                }),
                pointer_offset(base, unsafe {
                    (rust.find)(base, bytes.len(), target as c_char)
                })
            );
        }
    }

    let mut string_lengths = vec![0];
    string_lengths.extend((0..256).map(|_| rng.range_usize(1, 128)));
    for length in string_lengths {
        let bytes: Vec<u8> = (0..length).map(|_| rng.range_i32(1, 127) as u8).collect();
        let string = CString::new(bytes).unwrap();
        let c_buffer = unsafe { (c.create)(string.as_ptr()) };
        let rust_buffer = unsafe { (rust.create)(string.as_ptr()) };
        assert!(!c_buffer.is_null());
        assert!(!rust_buffer.is_null());
        assert_eq!(
            unsafe { CStr::from_ptr(c_buffer).to_bytes_with_nul() },
            unsafe { CStr::from_ptr(rust_buffer).to_bytes_with_nul() }
        );
        unsafe {
            free(c_buffer.cast());
            free(rust_buffer.cast());
        }
    }
    let high_bit_input = CString::new(vec![0x80, 0xff]).unwrap();
    let c_buffer = unsafe { (c.create)(high_bit_input.as_ptr()) };
    let rust_buffer = unsafe { (rust.create)(high_bit_input.as_ptr()) };
    assert_eq!(
        unsafe { CStr::from_ptr(c_buffer).to_bytes_with_nul() },
        unsafe { CStr::from_ptr(rust_buffer).to_bytes_with_nul() }
    );
    unsafe {
        free(c_buffer.cast());
        free(rust_buffer.cast());
    }

    for value in [0, 1, 65_534, 65_535]
        .into_iter()
        .chain((0..512).map(|_| rng.range_i32(0, 65_535)))
    {
        assert_eq!(unsafe { (c.validate)(value) }, unsafe {
            (rust.validate)(value)
        });
    }
}

#[test]
fn valid_charinbuf_composed_configurations_and_stdout() {
    let _guard = PROCESS_STATE.lock().unwrap();
    let (c, rust) = unsafe { load_pair() };
    let mut rng = Rng::new(0x94d0_49bb_1331_11eb);

    for _ in 0..128 {
        assert_charinbuf_equal(
            &c,
            &rust,
            [0, rng.range_i32(0, 65_535), rng.next_i32(), rng.next_i32()],
        );
    }

    for mode in [1, 2, 4] {
        for _ in 0..64 {
            let _ = assert_charinbuf_equal(
                &c,
                &rust,
                [mode, rng.next_i32(), rng.next_i32(), rng.next_i32()],
            );
        }
    }

    for _ in 0..256 {
        let _ = assert_charinbuf_equal(
            &c,
            &rust,
            [3, rng.next_i32(), rng.next_i32(), rng.next_i32()],
        );
    }
}

#[test]
fn direct_error_paths_and_generic_ffi_boundaries() {
    let _guard = PROCESS_STATE.lock().unwrap();
    let (c, rust) = unsafe { load_pair() };
    let mut rng = Rng::new(0x853c_49e6_748f_ea9b);

    let c_empty_null = unsafe { (c.is_empty)(ptr::null()) };
    let rust_empty_null = unsafe { (rust.is_empty)(ptr::null()) };
    assert_eq!(c_empty_null, rust_empty_null);
    assert_eq!(c_empty_null, 1);

    for size in [0, 1, 1024, usize::MAX] {
        let c_found = unsafe { (c.find)(ptr::null(), size, b'X' as c_char) };
        let rust_found = unsafe { (rust.find)(ptr::null(), size, b'X' as c_char) };
        assert_eq!(c_found, rust_found);
        assert!(c_found.is_null());
    }

    let c_created = unsafe { (c.create)(ptr::null()) };
    let rust_created = unsafe { (rust.create)(ptr::null()) };
    assert_eq!(c_created, rust_created);
    assert!(c_created.is_null());

    for value in [i32::MIN, -65_536, -2, -1]
        .into_iter()
        .chain((0..256).map(|_| rng.range_i32(i32::MIN, -1)))
    {
        assert_eq!(unsafe { (c.validate)(value) }, unsafe {
            (rust.validate)(value)
        });
        assert_eq!(unsafe { (c.validate)(value) }, 0);
    }

    for value in [65_536, 65_537, i32::MAX]
        .into_iter()
        .chain((0..256).map(|_| rng.range_i32(65_536, i32::MAX)))
    {
        assert_eq!(unsafe { (c.validate)(value) }, unsafe {
            (rust.validate)(value)
        });
        assert_eq!(unsafe { (c.validate)(value) }, 0);
    }

    for value in [i32::MIN, -1, 0, 1, i32::MAX]
        .into_iter()
        .chain((0..256).map(|_| rng.next_i32()))
    {
        assert_eq!(unsafe { (c.apply)(None, value) }, unsafe {
            (rust.apply)(None, value)
        });
        assert_eq!(unsafe { (c.apply)(None, value) }, -1);
    }

    let logical = [b'A', b'B', b'X'];
    let base = logical.as_ptr().cast::<c_char>();
    assert_eq!(
        pointer_offset(base, unsafe {
            (c.find)(base, logical.len(), b'X' as c_char)
        }),
        pointer_offset(base, unsafe {
            (rust.find)(base, logical.len(), b'X' as c_char)
        })
    );

    let one_past_logical = [b'A', b'B', b'C', b'X'];
    let base = one_past_logical.as_ptr().cast::<c_char>();
    assert_eq!(
        pointer_offset(base, unsafe {
            (c.find)(base, one_past_logical.len(), b'X' as c_char)
        }),
        pointer_offset(base, unsafe {
            (rust.find)(base, one_past_logical.len(), b'X' as c_char)
        })
    );
}

#[test]
fn charinbuf_range_and_invalid_mode_error_paths() {
    let _guard = PROCESS_STATE.lock().unwrap();
    let (c, rust) = unsafe { load_pair() };
    let mut rng = Rng::new(0xda3e_39cb_94b9_5bdb);

    let mut negative_values = vec![i32::MIN, -2, -1];
    negative_values.extend((0..128).map(|_| rng.range_i32(i32::MIN, -1)));
    for value in negative_values {
        let (result, _) =
            assert_charinbuf_equal(&c, &rust, [0, value, rng.next_i32(), rng.next_i32()]);
        assert_eq!(result, -1);
    }

    let mut oversized_values = vec![65_536, 65_537, i32::MAX];
    oversized_values.extend((0..128).map(|_| rng.range_i32(65_536, i32::MAX)));
    for value in oversized_values {
        let (result, _) =
            assert_charinbuf_equal(&c, &rust, [0, value, rng.next_i32(), rng.next_i32()]);
        assert_eq!(result, -1);
    }

    let mut invalid_modes = vec![i32::MIN, -2, -1, 5, 6, i32::MAX];
    invalid_modes.extend((0..128).map(|_| {
        if rng.next_u64() & 1 == 0 {
            rng.range_i32(i32::MIN, -1)
        } else {
            rng.range_i32(5, i32::MAX)
        }
    }));
    for mode in invalid_modes {
        let (result, _) = assert_charinbuf_equal(
            &c,
            &rust,
            [mode, rng.next_i32(), rng.next_i32(), rng.next_i32()],
        );
        assert_eq!(result, -1);
    }
}

fn compile_interposer() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output_dir = root.join("target/test-support");
    std::fs::create_dir_all(&output_dir).unwrap();
    let output = output_dir.join("libcharinbuf_interpose.so");
    let status = Command::new("cc")
        .args(["-shared", "-fPIC", "-std=c11"])
        .arg(root.join("tests/interpose.c"))
        .arg("-o")
        .arg(&output)
        .status()
        .unwrap();
    assert!(status.success(), "failed to compile test interposer");
    output
}

fn child_output(interposer: &Path, library: &str, scenario: &str) -> Output {
    Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "interposed_failure_child",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("LD_PRELOAD", interposer)
        .env("CHARINBUF_CHILD_LIBRARY", library)
        .env("CHARINBUF_CHILD_SCENARIO", scenario)
        .env("CHARINBUF_INTERPOSER", interposer)
        .output()
        .unwrap()
}

fn result_marker(output: &Output) -> String {
    assert!(
        output.status.success(),
        "child failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let marker_start = stdout
        .find("CHARINBUF_RESULT=")
        .unwrap_or_else(|| panic!("missing child result marker in:\n{stdout}"));
    stdout[marker_start..].lines().next().unwrap().to_owned()
}

#[test]
fn injected_allocator_and_search_failure_paths() {
    let interposer = compile_interposer();
    for scenario in [
        "create_malloc",
        "charinbuf_mode2_malloc",
        "charinbuf_mode4_malloc",
        "charinbuf_mode4_memchr",
    ] {
        let c_output = child_output(&interposer, "c", scenario);
        let rust_output = child_output(&interposer, "rust", scenario);
        assert_eq!(
            result_marker(&c_output),
            result_marker(&rust_output),
            "interposed scenario mismatch: {scenario}"
        );
    }
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        output.push(DIGITS[(byte >> 4) as usize] as char);
        output.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    output
}

#[test]
fn interposed_failure_child() {
    let Ok(library_kind) = std::env::var("CHARINBUF_CHILD_LIBRARY") else {
        return;
    };
    let scenario = std::env::var("CHARINBUF_CHILD_SCENARIO").unwrap();
    let interposer_path = PathBuf::from(std::env::var("CHARINBUF_INTERPOSER").unwrap());
    let (c_path, rust_path) = library_paths();
    let library_path = if library_kind == "c" {
        c_path
    } else {
        rust_path
    };

    let api = unsafe { Api::load(&library_path) };
    let interposer = unsafe { Library::new(&interposer_path).unwrap() };
    let fail_malloc: unsafe extern "C" fn(usize) = unsafe {
        *interposer
            .get(b"interpose_fail_next_malloc_of_size\0")
            .unwrap()
    };
    let fail_memchr: unsafe extern "C" fn() =
        unsafe { *interposer.get(b"interpose_fail_next_memchr\0").unwrap() };

    match scenario.as_str() {
        "create_malloc" => {
            let input = CString::new("abcdefg").unwrap();
            unsafe { fail_malloc(8) };
            let result = unsafe { (api.create)(input.as_ptr()) };
            assert!(result.is_null());
            println!("CHARINBUF_RESULT=NULL");
        }
        "charinbuf_mode2_malloc" => {
            unsafe { fail_malloc(24) };
            let (result, stdout) = capture_stdout(|| unsafe { (api.charinbuf)(2, 11, 22, 33) });
            assert_eq!(result, -1);
            println!("CHARINBUF_RESULT={result}:{}", hex(&stdout));
        }
        "charinbuf_mode4_malloc" => {
            unsafe { fail_malloc(38) };
            let (result, stdout) = capture_stdout(|| unsafe { (api.charinbuf)(4, 11, 22, 33) });
            assert_eq!(result, 0);
            println!("CHARINBUF_RESULT={result}:{}", hex(&stdout));
        }
        "charinbuf_mode4_memchr" => {
            unsafe { fail_memchr() };
            let (result, stdout) = capture_stdout(|| unsafe { (api.charinbuf)(4, 11, 22, 33) });
            assert_eq!(result, -1);
            println!("CHARINBUF_RESULT={result}:{}", hex(&stdout));
        }
        _ => panic!("unknown child scenario: {scenario}"),
    }
}
