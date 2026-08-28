use libloading::Library;
use std::ffi::{c_char, c_int, c_uint, c_void};
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;

const FLAG_MASK: u32 = 0x07;
const COUNTER_MASK: u32 = 0x1f << 3;
const MODE_MASK: u32 = 0x07 << 8;
const INITIAL_FLAGS: u32 = 1 | 4 | (3 << 8) | (15 << 11);

#[repr(C)]
union TypeConfusion {
    int_val: c_int,
    float_val: f32,
    uint_val: c_uint,
    bytes: [c_char; 4],
}

#[repr(C)]
struct ProcessState {
    flags: c_uint,
    data: TypeConfusion,
    buffer: *mut c_char,
    capacity: c_int,
}

type CreateState = unsafe extern "C" fn(c_int, c_int) -> *mut ProcessState;
type DestroyState = unsafe extern "C" fn(*mut ProcessState);
type ProcessBuffer = unsafe extern "C" fn(*mut ProcessState, c_char) -> c_int;
type UpdateFlags = unsafe extern "C" fn(*mut ProcessState, c_int);
type ConfuseTypes = unsafe extern "C" fn(*mut ProcessState, c_int) -> c_int;
type Confusion = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

struct Api {
    _library: Library,
    create_state: CreateState,
    destroy_state: DestroyState,
    process_buffer: ProcessBuffer,
    update_flags: UpdateFlags,
    confuse_types: ConfuseTypes,
    confusion: Confusion,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        unsafe {
            Self {
                create_state: *library.get(b"create_state\0").unwrap(),
                destroy_state: *library.get(b"destroy_state\0").unwrap(),
                process_buffer: *library.get(b"process_buffer\0").unwrap(),
                update_flags: *library.get(b"update_flags\0").unwrap(),
                confuse_types: *library.get(b"confuse_types\0").unwrap(),
                confusion: *library.get(b"confusion\0").unwrap(),
                _library: library,
            }
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
struct StateSnapshot {
    flags: u32,
    data_bits: u32,
    capacity: i32,
    buffer: Vec<u8>,
}

unsafe fn snapshot(state: *mut ProcessState) -> StateSnapshot {
    assert!(!state.is_null());
    let capacity = unsafe { (*state).capacity };
    let buffer_ptr = unsafe { (*state).buffer };
    let mut buffer = Vec::new();
    if capacity > 0 && !buffer_ptr.is_null() {
        for index in 0..capacity as usize {
            let byte = unsafe { *buffer_ptr.add(index) as u8 };
            buffer.push(byte);
            if byte == 0 {
                break;
            }
        }
    }
    StateSnapshot {
        flags: unsafe { (*state).flags },
        data_bits: unsafe { (*state).data.uint_val },
        capacity,
        buffer,
    }
}

fn assert_same<T: Debug + PartialEq>(row: &str, c_value: T, rust_value: T) {
    assert_eq!(c_value, rust_value, "{row}");
}

unsafe fn create_pair(
    c_api: &Api,
    rust_api: &Api,
    initial: i32,
    capacity: i32,
    row: &str,
) -> (*mut ProcessState, *mut ProcessState) {
    let (c_state, c_output) = capture_stdout(|| unsafe { (c_api.create_state)(initial, capacity) });
    let (rust_state, rust_output) =
        capture_stdout(|| unsafe { (rust_api.create_state)(initial, capacity) });
    assert_same(row, c_output, rust_output);
    assert_same(row, c_state.is_null(), rust_state.is_null());
    if !c_state.is_null() {
        assert_same(row, unsafe { snapshot(c_state) }, unsafe {
            snapshot(rust_state)
        });
    }
    (c_state, rust_state)
}

unsafe fn destroy_pair(
    c_api: &Api,
    rust_api: &Api,
    c_state: *mut ProcessState,
    rust_state: *mut ProcessState,
) {
    unsafe {
        (c_api.destroy_state)(c_state);
        (rust_api.destroy_state)(rust_state);
    }
}

unsafe fn set_buffer(state: *mut ProcessState, bytes: &[u8]) {
    assert!(!state.is_null());
    assert!(bytes.len() < unsafe { (*state).capacity as usize });
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), (*state).buffer.cast::<u8>(), bytes.len());
        *(*state).buffer.add(bytes.len()) = 0;
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
}

unsafe extern "C" {
    fn free(pointer: *mut c_void);
    fn fflush(stream: *mut c_void) -> c_int;
    fn open(path: *const c_char, flags: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
}

struct StdoutSilencer {
    saved: c_int,
}

impl StdoutSilencer {
    fn new() -> Self {
        unsafe {
            fflush(ptr::null_mut());
            let saved = dup(1);
            let null_fd = open(c"/dev/null".as_ptr(), 1);
            assert!(saved >= 0 && null_fd >= 0);
            assert_eq!(dup2(null_fd, 1), 1);
            close(null_fd);
            Self { saved }
        }
    }
}

impl Drop for StdoutSilencer {
    fn drop(&mut self) {
        unsafe {
            fflush(ptr::null_mut());
            dup2(self.saved, 1);
            close(self.saved);
        }
    }
}

fn capture_stdout<T>(operation: impl FnOnce() -> T) -> (T, Vec<u8>) {
    unsafe {
        fflush(ptr::null_mut());
        let mut fds = [0; 2];
        assert_eq!(pipe(fds.as_mut_ptr()), 0);
        let saved = dup(1);
        assert!(saved >= 0);
        assert_eq!(dup2(fds[1], 1), 1);
        close(fds[1]);

        let result = operation();

        fflush(ptr::null_mut());
        assert_eq!(dup2(saved, 1), 1);
        close(saved);

        let mut output = Vec::new();
        let mut chunk = [0u8; 4096];
        loop {
            let count = read(fds[0], chunk.as_mut_ptr().cast(), chunk.len());
            assert!(count >= 0);
            if count == 0 {
                break;
            }
            output.extend_from_slice(&chunk[..count as usize]);
        }
        close(fds[0]);
        (result, output)
    }
}

fn library_paths() -> (PathBuf, PathBuf) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    (
        root.join("../c_src/build/libharvest-work-m1bNJW.so"),
        root.join("target/release/libconfusion_lib.so"),
    )
}

fn allocator_shim_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/fail_alloc.so")
}

fn build_allocator_shim() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output = allocator_shim_path();
    let status = Command::new("cc")
        .args(["-shared", "-fPIC", "-O2"])
        .arg(root.join("tests/fail_alloc.c"))
        .arg("-o")
        .arg(&output)
        .status()
        .expect("failed to run cc for allocator shim");
    assert!(status.success(), "failed to build allocator shim");
    output
}

unsafe fn verify_preloaded_allocation_errors(c_api: &Api, rust_api: &Api) {
    type SetFailSize = unsafe extern "C" fn(usize);

    let shim_path = std::env::var_os("CONFUSION_ALLOC_SHIM").unwrap();
    let shim = unsafe { Library::new(shim_path) }.unwrap();
    let set_fail_size: SetFailSize = unsafe { *shim.get(b"fail_alloc_set_size\0").unwrap() };

    unsafe { set_fail_size(24) };
    let (c_state_failed, c_output) =
        capture_stdout(|| unsafe { (c_api.create_state)(1, 128).is_null() });
    unsafe { set_fail_size(0) };
    unsafe { set_fail_size(24) };
    let (rust_state_failed, rust_output) =
        capture_stdout(|| unsafe { (rust_api.create_state)(1, 128).is_null() });
    unsafe { set_fail_size(0) };
    assert_same("ERRORS row 1", c_output, rust_output);
    assert_same("ERRORS row 1", c_state_failed, rust_state_failed);
    assert!(c_state_failed);

    unsafe { set_fail_size(1024) };
    let (c_buffer_failed, c_output) =
        capture_stdout(|| unsafe { (c_api.create_state)(1, 1024).is_null() });
    unsafe { set_fail_size(0) };
    unsafe { set_fail_size(1024) };
    let (rust_buffer_failed, rust_output) =
        capture_stdout(|| unsafe { (rust_api.create_state)(1, 1024).is_null() });
    unsafe { set_fail_size(0) };
    assert_same("ERRORS row 2", c_output, rust_output);
    assert_same("ERRORS row 2", c_buffer_failed, rust_buffer_failed);
    assert!(c_buffer_failed);

    unsafe { set_fail_size(i32::MAX as usize) };
    let (c_oversized_failed, c_output) =
        capture_stdout(|| unsafe { (c_api.create_state)(1, i32::MAX).is_null() });
    unsafe { set_fail_size(0) };
    unsafe { set_fail_size(i32::MAX as usize) };
    let (rust_oversized_failed, rust_output) =
        capture_stdout(|| unsafe { (rust_api.create_state)(1, i32::MAX).is_null() });
    unsafe { set_fail_size(0) };
    assert_same("ERRORS row 4", c_output, rust_output);
    assert_same("ERRORS row 4", c_oversized_failed, rust_oversized_failed);
    assert!(c_oversized_failed);

    unsafe { set_fail_size(24) };
    let (c_wrapper, c_output) = capture_stdout(|| unsafe { (c_api.confusion)(1, 2, 3, 0) });
    unsafe { set_fail_size(0) };
    unsafe { set_fail_size(24) };
    let (rust_wrapper, rust_output) =
        capture_stdout(|| unsafe { (rust_api.confusion)(1, 2, 3, 0) });
    unsafe { set_fail_size(0) };
    assert_same("ERRORS row 12", c_output, rust_output);
    assert_same("ERRORS row 12", c_wrapper, rust_wrapper);
    assert_eq!(c_wrapper, -1);
}

fn verify_allocation_errors_in_child() {
    let shim = build_allocator_shim();
    let current_exe = std::env::current_exe().unwrap();
    let status = Command::new(current_exe)
        .args([
            "--exact",
            "all_c_and_rust_ffi_surfaces_match",
            "--nocapture",
        ])
        .env("LD_PRELOAD", &shim)
        .env("CONFUSION_ALLOC_SHIM", &shim)
        .status()
        .expect("failed to run allocation-error child");
    assert!(status.success(), "allocation-error child failed");
}

unsafe fn verify_create_configs(c_api: &Api, rust_api: &Api, rng: &mut Rng) {
    let mut values = vec![0, 1, -1, i32::MIN, i32::MAX, 10, -10];
    values.extend((0..64).map(|_| rng.next_i32()));

    for initial in values {
        let formatted_len = format!("State:{initial}:Mode:3").len() as i32;
        let truncating = 2 + (rng.next_u32() % (formatted_len - 1) as u32) as i32;
        for (row, capacity) in [
            ("CONFIGS row 1", 0),
            ("CONFIGS row 2", 1),
            ("CONFIGS row 3", truncating),
            ("CONFIGS row 4", formatted_len + 1),
            (
                "CONFIGS row 5",
                formatted_len + 2 + (rng.next_u32() % 32) as i32,
            ),
        ] {
            let (c_state, rust_state) =
                unsafe { create_pair(c_api, rust_api, initial, capacity, row) };
            unsafe { destroy_pair(c_api, rust_api, c_state, rust_state) };
        }
    }

    let (c_state, rust_state) = unsafe { create_pair(c_api, rust_api, 42, 128, "CONFIGS row 6") };
    unsafe { destroy_pair(c_api, rust_api, c_state, rust_state) };
}

unsafe fn process_case(c_api: &Api, rust_api: &Api, bytes: &[u8], target: u8, row: &str) {
    let (c_state, rust_state) = unsafe { create_pair(c_api, rust_api, 0, 512, row) };
    unsafe {
        set_buffer(c_state, bytes);
        set_buffer(rust_state, bytes);
    }
    let (c_result, c_output) =
        capture_stdout(|| unsafe { (c_api.process_buffer)(c_state, target as c_char) });
    let (rust_result, rust_output) =
        capture_stdout(|| unsafe { (rust_api.process_buffer)(rust_state, target as c_char) });
    assert_same(row, c_output, rust_output);
    assert_same(row, c_result, rust_result);
    assert_same(row, unsafe { snapshot(c_state) }, unsafe {
        snapshot(rust_state)
    });
    unsafe { destroy_pair(c_api, rust_api, c_state, rust_state) };
}

unsafe fn verify_process_configs(c_api: &Api, rust_api: &Api, rng: &mut Rng) {
    for _ in 0..64 {
        unsafe { process_case(c_api, rust_api, b"", b'x', "CONFIGS row 7") };

        let mut no_match = vec![b'a'; 1 + (rng.next_u32() % 128) as usize];
        for byte in &mut no_match {
            *byte = 1 + (rng.next_u32() % 126) as u8;
            if *byte == b'Z' {
                *byte = b'Y';
            }
        }
        unsafe { process_case(c_api, rust_api, &no_match, b'Z', "CONFIGS row 8") };

        let mut one = no_match.clone();
        let one_index = rng.next_u32() as usize % one.len();
        one[one_index] = b'Z';
        unsafe { process_case(c_api, rust_api, &one, b'Z', "CONFIGS row 9") };

        let mut many = no_match;
        let occurrences = 2 + (rng.next_u32() % 8) as usize;
        let many_len = many.len();
        for index in 0..occurrences {
            let position = (rng.next_u32() as usize + index) % many_len;
            many[position] = b'Z';
        }
        unsafe { process_case(c_api, rust_api, &many, b'Z', "CONFIGS row 10") };

        unsafe { process_case(c_api, rust_api, &many, 0, "CONFIGS row 11") };
        let mut high = vec![0x81; 1 + (rng.next_u32() % 128) as usize];
        let high_index = rng.next_u32() as usize % high.len();
        high[high_index] = 0x80;
        unsafe { process_case(c_api, rust_api, &high, 0x80, "CONFIGS row 11") };
    }
}

unsafe fn verify_flag_configs(c_api: &Api, rust_api: &Api, rng: &mut Rng) {
    for mode in 0..8u32 {
        for flags in 0..8u32 {
            let row = 12 + (mode * 8 + flags) as usize;
            let label = format!("CONFIGS row {row}");
            let (c_state, rust_state) =
                unsafe { create_pair(c_api, rust_api, rng.next_i32(), 128, &label) };
            let mut counters = vec![0, 31];
            counters.extend((0..32).map(|_| rng.next_u32() & 31));
            for counter in counters {
                let start_flags = (INITIAL_FLAGS & !COUNTER_MASK) | (counter << 3);
                unsafe {
                    (*c_state).flags = start_flags;
                    (*rust_state).flags = start_flags;
                }
                let low_bits = flags | (mode << 3);
                let param = ((rng.next_u32() & !0x3f) | low_bits) as i32;
                let (_, c_output) =
                    capture_stdout(|| unsafe { (c_api.update_flags)(c_state, param) });
                let (_, rust_output) =
                    capture_stdout(|| unsafe { (rust_api.update_flags)(rust_state, param) });
                assert_same(&label, c_output, rust_output);
                assert_same(&label, unsafe { snapshot(c_state) }, unsafe {
                    snapshot(rust_state)
                });
                let packed = unsafe { (*c_state).flags };
                assert_eq!(packed & FLAG_MASK, flags, "{label}");
                assert_eq!((packed & MODE_MASK) >> 8, mode, "{label}");
                assert_eq!((packed & COUNTER_MASK) >> 3, (counter + 1) & 31, "{label}");
            }
            unsafe { destroy_pair(c_api, rust_api, c_state, rust_state) };
        }
    }
}

unsafe fn confuse_case(c_api: &Api, rust_api: &Api, initial: i32, operation: i32, row: &str) {
    let (c_state, rust_state) = unsafe { create_pair(c_api, rust_api, initial, 128, row) };
    let (c_result, c_output) =
        capture_stdout(|| unsafe { (c_api.confuse_types)(c_state, operation) });
    let (rust_result, rust_output) =
        capture_stdout(|| unsafe { (rust_api.confuse_types)(rust_state, operation) });
    assert_same(row, c_output, rust_output);
    assert_same(row, c_result, rust_result);
    assert_same(row, unsafe { snapshot(c_state) }, unsafe {
        snapshot(rust_state)
    });
    unsafe { destroy_pair(c_api, rust_api, c_state, rust_state) };
}

unsafe fn verify_confuse_configs(c_api: &Api, rust_api: &Api, rng: &mut Rng) {
    let mut values = vec![
        0,
        1,
        -1,
        i32::MIN,
        i32::MAX,
        0x7f80_0000,
        0xff80_0000u32 as i32,
        0x7fc0_0000,
        0x0000_0001,
        0x8000_0001u32 as i32,
    ];
    values.extend((0..256).map(|_| rng.next_i32()));

    for initial in values {
        for operation in 0..=3 {
            let row = 76 + operation as usize;
            unsafe {
                confuse_case(
                    c_api,
                    rust_api,
                    initial,
                    operation,
                    &format!("CONFIGS row {row}"),
                )
            };
        }
        for operation in [-1001, -4, -1, 4, 17, i32::MAX] {
            unsafe {
                confuse_case(
                    c_api,
                    rust_api,
                    initial,
                    operation,
                    "CONFIGS row 80 / ERRORS row 11",
                )
            };
        }
    }

    for second_operation in 1..=3 {
        let row = 80 + second_operation as usize;
        for _ in 0..64 {
            let (c_state, rust_state) = unsafe {
                create_pair(
                    c_api,
                    rust_api,
                    rng.next_i32(),
                    128,
                    &format!("CONFIGS row {row}"),
                )
            };
            let (c_first, c_first_output) =
                capture_stdout(|| unsafe { (c_api.confuse_types)(c_state, 0) });
            let (rust_first, rust_first_output) =
                capture_stdout(|| unsafe { (rust_api.confuse_types)(rust_state, 0) });
            assert_same(
                &format!("CONFIGS row {row}"),
                c_first_output,
                rust_first_output,
            );
            assert_same(&format!("CONFIGS row {row}"), c_first, rust_first);
            let (c_second, c_second_output) =
                capture_stdout(|| unsafe { (c_api.confuse_types)(c_state, second_operation) });
            let (rust_second, rust_second_output) = capture_stdout(|| unsafe {
                (rust_api.confuse_types)(rust_state, second_operation)
            });
            assert_same(
                &format!("CONFIGS row {row}"),
                c_second_output,
                rust_second_output,
            );
            assert_same(&format!("CONFIGS row {row}"), c_second, rust_second);
            assert_same(
                &format!("CONFIGS row {row}"),
                unsafe { snapshot(c_state) },
                unsafe { snapshot(rust_state) },
            );
            unsafe { destroy_pair(c_api, rust_api, c_state, rust_state) };
        }
    }
}

fn rendered_match_count(initial: i32, remainder: i32) -> usize {
    let target = (b'0' as i32 + remainder) as u8;
    format!("State:{initial}:Mode:3")
        .bytes()
        .filter(|byte| *byte == target)
        .count()
}

fn wrapper_input(rng: &mut Rng, cardinality: usize, iteration: usize) -> (i32, i32) {
    const NO_MATCH_REMAINDERS: [i32; 18] = [
        -9, -8, -7, -6, -5, -4, -3, -2, -1, 0, 1, 2, 4, 5, 6, 7, 8, 9,
    ];
    for _ in 0..1_000_000 {
        let remainder = if cardinality == 0 {
            NO_MATCH_REMAINDERS[iteration % NO_MATCH_REMAINDERS.len()]
        } else {
            (iteration % 10) as i32
        };
        let initial = rng.next_i32();
        let count = rendered_match_count(initial, remainder);
        let matches = match cardinality {
            0 => count == 0,
            1 => count == 1,
            _ => count >= 2,
        };
        if matches {
            let quotient = 1 + (rng.next_u32() % 100_000) as i32;
            let param3 = if remainder < 0 {
                remainder - quotient * 10
            } else {
                remainder + quotient * 10
            };
            return (initial, param3);
        }
    }
    panic!("could not generate wrapper input for cardinality {cardinality}");
}

unsafe fn verify_wrapper_configs(c_api: &Api, rust_api: &Api, rng: &mut Rng) {
    for operation_class in 0..5usize {
        for cardinality in 0..3usize {
            let row = 84 + operation_class * 3 + cardinality;
            for iteration in 0..128usize {
                let (param1, param3) = wrapper_input(rng, cardinality, iteration);
                let low_six = (iteration % 64) as u32;
                let param2 = ((rng.next_u32() & !0x3f) | low_six) as i32;
                let param4 = if operation_class < 4 {
                    operation_class as i32 + 4 * (rng.next_u32() % 100_000) as i32
                } else {
                    -1 - (rng.next_u32() % 100_000) as i32 * 4
                };
                let (c_result, c_output) =
                    capture_stdout(|| unsafe { (c_api.confusion)(param1, param2, param3, param4) });
                let (rust_result, rust_output) = capture_stdout(|| unsafe {
                    (rust_api.confusion)(param1, param2, param3, param4)
                });
                assert_same(&format!("CONFIGS row {row}"), c_output, rust_output);
                assert_same(&format!("CONFIGS row {row}"), c_result, rust_result);
            }
        }
    }
}

unsafe fn verify_error_configs(c_api: &Api, rust_api: &Api, rng: &mut Rng) {
    unsafe {
        (c_api.destroy_state)(ptr::null_mut());
        (rust_api.destroy_state)(ptr::null_mut());
    }

    for api in [c_api, rust_api] {
        let state = unsafe { (api.create_state)(1, 16) };
        assert!(!state.is_null());
        unsafe {
            free((*state).buffer.cast());
            (*state).buffer = ptr::null_mut();
            (api.destroy_state)(state);
        }
    }

    let (c_result, c_output) =
        capture_stdout(|| unsafe { (c_api.process_buffer)(ptr::null_mut(), b'x' as c_char) });
    let (rust_result, rust_output) =
        capture_stdout(|| unsafe { (rust_api.process_buffer)(ptr::null_mut(), b'x' as c_char) });
    assert_same("ERRORS row 7", c_output, rust_output);
    assert_same("ERRORS row 7", c_result, rust_result);

    let (c_state, rust_state) = unsafe { create_pair(c_api, rust_api, 1, 16, "ERRORS row 8") };
    unsafe {
        free((*c_state).buffer.cast());
        free((*rust_state).buffer.cast());
        (*c_state).buffer = ptr::null_mut();
        (*rust_state).buffer = ptr::null_mut();
    }
    let (c_result, c_output) =
        capture_stdout(|| unsafe { (c_api.process_buffer)(c_state, b'x' as c_char) });
    let (rust_result, rust_output) =
        capture_stdout(|| unsafe { (rust_api.process_buffer)(rust_state, b'x' as c_char) });
    assert_same("ERRORS row 8", c_output, rust_output);
    assert_same("ERRORS row 8", c_result, rust_result);
    unsafe { destroy_pair(c_api, rust_api, c_state, rust_state) };

    let param = rng.next_i32();
    let (_, c_output) = capture_stdout(|| unsafe { (c_api.update_flags)(ptr::null_mut(), param) });
    let (_, rust_output) =
        capture_stdout(|| unsafe { (rust_api.update_flags)(ptr::null_mut(), param) });
    assert_same("ERRORS row 9", c_output, rust_output);

    for operation in [i32::MIN, -5, -1, 0, 1, 2, 3, 4, i32::MAX] {
        let (c_result, c_output) =
            capture_stdout(|| unsafe { (c_api.confuse_types)(ptr::null_mut(), operation) });
        let (rust_result, rust_output) =
            capture_stdout(|| unsafe { (rust_api.confuse_types)(ptr::null_mut(), operation) });
        assert_same("ERRORS row 10", c_output, rust_output);
        assert_same("ERRORS row 10", c_result, rust_result);
    }

    for capacity in [-1, i32::MIN] {
        let (c_state, rust_state) =
            unsafe { create_pair(c_api, rust_api, 7, capacity, "ERRORS rows 2-3") };
        unsafe { destroy_pair(c_api, rust_api, c_state, rust_state) };
    }

    let (c_oversized, c_output) = capture_stdout(|| unsafe { (c_api.create_state)(7, i32::MAX) });
    let (rust_oversized, rust_output) =
        capture_stdout(|| unsafe { (rust_api.create_state)(7, i32::MAX) });
    assert_same("ERRORS row 4 boundary", c_output, rust_output);
    assert_same(
        "ERRORS row 4",
        c_oversized.is_null(),
        rust_oversized.is_null(),
    );
    unsafe { destroy_pair(c_api, rust_api, c_oversized, rust_oversized) };
}

#[test]
fn all_c_and_rust_ffi_surfaces_match() {
    let (c_path, rust_path) = library_paths();
    assert!(c_path.is_file(), "missing C library: {}", c_path.display());
    assert!(
        rust_path.is_file(),
        "missing Rust library: {}",
        rust_path.display()
    );

    let c_api = unsafe { Api::load(&c_path) };
    let rust_api = unsafe { Api::load(&rust_path) };
    let _silencer = StdoutSilencer::new();
    if std::env::var_os("CONFUSION_ALLOC_SHIM").is_some() {
        unsafe { verify_preloaded_allocation_errors(&c_api, &rust_api) };
        return;
    }
    let mut rng = Rng::new(0x8d26_4e91_7a5b_c3f1);

    unsafe {
        eprintln!("differential phase: create/destroy");
        verify_create_configs(&c_api, &rust_api, &mut rng);
        eprintln!("differential phase: process_buffer");
        verify_process_configs(&c_api, &rust_api, &mut rng);
        eprintln!("differential phase: update_flags");
        verify_flag_configs(&c_api, &rust_api, &mut rng);
        eprintln!("differential phase: confuse_types");
        verify_confuse_configs(&c_api, &rust_api, &mut rng);
        eprintln!("differential phase: confusion wrapper");
        verify_wrapper_configs(&c_api, &rust_api, &mut rng);
        eprintln!("differential phase: errors");
        verify_error_configs(&c_api, &rust_api, &mut rng);
    }
    eprintln!("differential phase: allocation-error child");
    verify_allocation_errors_in_child();
}
