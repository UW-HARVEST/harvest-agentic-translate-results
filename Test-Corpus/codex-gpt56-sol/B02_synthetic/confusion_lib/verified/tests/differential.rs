use libloading::Library;
use std::ffi::{CStr, c_char, c_int, c_long, c_void};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::Mutex;

#[repr(C)]
struct ProcessState {
    flags: u32,
    data: u32,
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

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fclose(stream: *mut c_void) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fileno(stream: *mut c_void) -> c_int;
    fn fread(ptr: *mut c_void, size: usize, count: usize, stream: *mut c_void) -> usize;
    fn free(ptr: *mut c_void);
    fn fseek(stream: *mut c_void, offset: c_long, whence: c_int) -> c_int;
    fn ftell(stream: *mut c_void) -> c_long;
    fn malloc(size: usize) -> *mut c_void;
    fn rewind(stream: *mut c_void);
    fn tmpfile() -> *mut c_void;
}

const STDOUT_FILENO: c_int = 1;
const SEEK_END: c_int = 2;
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let create_state = unsafe { *library.get(b"create_state\0").unwrap() };
        let destroy_state = unsafe { *library.get(b"destroy_state\0").unwrap() };
        let process_buffer = unsafe { *library.get(b"process_buffer\0").unwrap() };
        let update_flags = unsafe { *library.get(b"update_flags\0").unwrap() };
        let confuse_types = unsafe { *library.get(b"confuse_types\0").unwrap() };
        let confusion = unsafe { *library.get(b"confusion\0").unwrap() };
        Self {
            _library: library,
            create_state,
            destroy_state,
            process_buffer,
            update_flags,
            confuse_types,
            confusion,
        }
    }
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libtranslated_rust.so")
}

fn rust_library_path() -> PathBuf {
    let deps = std::env::current_exe().unwrap();
    let profile_dir = deps.parent().unwrap().parent().unwrap();
    profile_dir.join("libconfusion_lib.so")
}

fn load_apis() -> (Api, Api) {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(c_path.is_file(), "missing C library: {}", c_path.display());
    assert!(
        rust_path.is_file(),
        "missing Rust library: {}",
        rust_path.display()
    );
    unsafe { (Api::load(&c_path), Api::load(&rust_path)) }
}

fn capture_stdout<T>(call: impl FnOnce() -> T) -> (T, Vec<u8>) {
    let _guard = STDOUT_LOCK.lock().unwrap();
    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
        let saved_stdout = dup(STDOUT_FILENO);
        assert!(saved_stdout >= 0);
        let stream = tmpfile();
        assert!(!stream.is_null());
        let capture_fd = fileno(stream);
        assert!(capture_fd >= 0);
        assert_eq!(dup2(capture_fd, STDOUT_FILENO), STDOUT_FILENO);

        let result = call();

        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, STDOUT_FILENO), STDOUT_FILENO);
        assert_eq!(close(saved_stdout), 0);
        assert_eq!(fseek(stream, 0, SEEK_END), 0);
        let length = ftell(stream);
        assert!(length >= 0);
        rewind(stream);
        let mut output = vec![0_u8; length as usize];
        if !output.is_empty() {
            assert_eq!(
                fread(output.as_mut_ptr().cast(), 1, output.len(), stream),
                output.len()
            );
        }
        assert_eq!(fclose(stream), 0);
        (result, output)
    }
}

#[derive(Debug, Eq, PartialEq)]
struct StateSnapshot {
    flags: u32,
    data: u32,
    capacity: c_int,
    buffer_is_null: bool,
    initialized_buffer: Vec<u8>,
}

unsafe fn snapshot(state: *mut ProcessState) -> StateSnapshot {
    assert!(!state.is_null());
    let state = unsafe { &*state };
    let initialized_buffer = if state.buffer.is_null() || state.capacity <= 0 {
        Vec::new()
    } else {
        unsafe { CStr::from_ptr(state.buffer) }
            .to_bytes_with_nul()
            .to_vec()
    };
    StateSnapshot {
        flags: state.flags,
        data: state.data,
        capacity: state.capacity,
        buffer_is_null: state.buffer.is_null(),
        initialized_buffer,
    }
}

fn compare_create(c: &Api, rust: &Api, initial: c_int, capacity: c_int) {
    let (c_state, c_output) = capture_stdout(|| unsafe { (c.create_state)(initial, capacity) });
    let (rust_state, rust_output) =
        capture_stdout(|| unsafe { (rust.create_state)(initial, capacity) });
    assert_eq!(
        c_output, rust_output,
        "create output ({initial}, {capacity})"
    );
    assert_eq!(
        c_state.is_null(),
        rust_state.is_null(),
        "create null result ({initial}, {capacity})"
    );
    if !c_state.is_null() {
        assert_eq!(
            unsafe { snapshot(c_state) },
            unsafe { snapshot(rust_state) },
            "create state ({initial}, {capacity})"
        );
        let (_, c_destroy_output) = capture_stdout(|| unsafe { (c.destroy_state)(c_state) });
        let (_, rust_destroy_output) =
            capture_stdout(|| unsafe { (rust.destroy_state)(rust_state) });
        assert_eq!(c_destroy_output, rust_destroy_output);
    }
}

fn install_buffer(state: *mut ProcessState, bytes: &[u8]) {
    unsafe {
        assert!(!state.is_null());
        assert!(!(*state).buffer.is_null());
        assert!(((*state).capacity as usize) > bytes.len());
        ptr::copy_nonoverlapping(bytes.as_ptr(), (*state).buffer.cast(), bytes.len());
        *(*state).buffer.add(bytes.len()) = 0;
    }
}

fn compare_process(c: &Api, rust: &Api, bytes: &[u8], target: c_char) {
    let capacity = (bytes.len() + 2).max(32) as c_int;
    let c_state = unsafe { (c.create_state)(0, capacity) };
    let rust_state = unsafe { (rust.create_state)(0, capacity) };
    install_buffer(c_state, bytes);
    install_buffer(rust_state, bytes);

    let (c_result, c_output) = capture_stdout(|| unsafe { (c.process_buffer)(c_state, target) });
    let (rust_result, rust_output) =
        capture_stdout(|| unsafe { (rust.process_buffer)(rust_state, target) });
    assert_eq!(
        c_result, rust_result,
        "process result for {bytes:?}/{target}"
    );
    assert_eq!(
        c_output, rust_output,
        "process output for {bytes:?}/{target}"
    );
    assert_eq!(unsafe { snapshot(c_state) }, unsafe {
        snapshot(rust_state)
    });
    unsafe {
        (c.destroy_state)(c_state);
        (rust.destroy_state)(rust_state);
    }
}

fn set_counter(state: *mut ProcessState, counter: u32) {
    unsafe {
        (*state).flags = ((*state).flags & !(0x1f << 3)) | ((counter & 0x1f) << 3);
    }
}

fn compare_update(c: &Api, rust: &Api, param: c_int, counter: u32) {
    let c_state = unsafe { (c.create_state)(-73, 64) };
    let rust_state = unsafe { (rust.create_state)(-73, 64) };
    set_counter(c_state, counter);
    set_counter(rust_state, counter);
    let (_, c_output) = capture_stdout(|| unsafe { (c.update_flags)(c_state, param) });
    let (_, rust_output) = capture_stdout(|| unsafe { (rust.update_flags)(rust_state, param) });
    assert_eq!(c_output, rust_output, "update output ({param}, {counter})");
    assert_eq!(
        unsafe { snapshot(c_state) },
        unsafe { snapshot(rust_state) },
        "update state ({param}, {counter})"
    );
    unsafe {
        (c.destroy_state)(c_state);
        (rust.destroy_state)(rust_state);
    }
}

fn compare_confuse(c: &Api, rust: &Api, bits: u32, operation: c_int) {
    let c_state = unsafe { (c.create_state)(bits as i32, 64) };
    let rust_state = unsafe { (rust.create_state)(bits as i32, 64) };
    let (c_result, c_output) = capture_stdout(|| unsafe { (c.confuse_types)(c_state, operation) });
    let (rust_result, rust_output) =
        capture_stdout(|| unsafe { (rust.confuse_types)(rust_state, operation) });
    assert_eq!(
        c_result, rust_result,
        "confuse result ({bits:#010x}, {operation})"
    );
    assert_eq!(
        c_output, rust_output,
        "confuse output ({bits:#010x}, {operation})"
    );
    assert_eq!(
        unsafe { snapshot(c_state) },
        unsafe { snapshot(rust_state) },
        "confuse state ({bits:#010x}, {operation})"
    );
    unsafe {
        (c.destroy_state)(c_state);
        (rust.destroy_state)(rust_state);
    }
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        (self.0.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 32) as u32
    }
}

fn compare_confusion(
    c: &Api,
    rust: &Api,
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) {
    let (c_result, c_output) =
        capture_stdout(|| unsafe { (c.confusion)(param1, param2, param3, param4) });
    let (rust_result, rust_output) =
        capture_stdout(|| unsafe { (rust.confusion)(param1, param2, param3, param4) });
    assert_eq!(
        c_result, rust_result,
        "confusion result ({param1}, {param2}, {param3}, {param4})"
    );
    assert_eq!(
        c_output, rust_output,
        "confusion output ({param1}, {param2}, {param3}, {param4})"
    );
}

fn initial_values(rng: &mut Rng) -> Vec<c_int> {
    let mut values = vec![
        c_int::MIN,
        c_int::MIN + 1,
        -1_000_000_000,
        -101,
        -10,
        -1,
        0,
        1,
        9,
        10,
        101,
        1_000_000_000,
        c_int::MAX - 1,
        c_int::MAX,
    ];
    values.extend((0..32).map(|_| rng.next_u32() as c_int));
    values
}

fn test_create_and_destroy(c: &Api, rust: &Api, rng: &mut Rng) {
    for initial in initial_values(rng) {
        let formatted_length = format!("State:{initial}:Mode:3").len() as c_int;
        compare_create(c, rust, initial, 0);
        compare_create(c, rust, initial, 1);
        let truncating_capacity =
            2 + (rng.next_u32() % (formatted_length.saturating_sub(1) as u32)) as c_int;
        compare_create(c, rust, initial, truncating_capacity);
        compare_create(c, rust, initial, formatted_length + 1);
        compare_create(
            c,
            rust,
            initial,
            formatted_length + 2 + (rng.next_u32() % 128) as c_int,
        );
    }

    // Generic zero/oversized boundaries and a deterministic invalid size.
    compare_create(c, rust, 0, 4096);
    compare_create(c, rust, -1, -1);
}

fn test_process_buffer(c: &Api, rust: &Api, rng: &mut Rng) {
    compare_process(c, rust, b"", b'a' as c_char);
    for _ in 0..64 {
        let length = 1 + (rng.next_u32() % 96) as usize;
        let mut absent = Vec::with_capacity(length);
        for _ in 0..length {
            absent.push(b'a' + (rng.next_u32() % 23) as u8);
        }
        compare_process(c, rust, &absent, b'x' as c_char);

        let mut one = absent.clone();
        one[(rng.next_u32() as usize) % length] = b'x';
        compare_process(c, rust, &one, b'x' as c_char);

        let mut many = absent;
        let occurrences = 2 + (rng.next_u32() % 8) as usize;
        for _ in 0..occurrences {
            let index = (rng.next_u32() as usize) % length;
            many[index] = b'x';
        }
        compare_process(c, rust, &many, b'x' as c_char);
    }

    compare_process(c, rust, b"abc", 0);
    compare_process(c, rust, &[0x80, 1, 0x80, 2], c_char::MIN);
    compare_process(c, rust, &[0x7f, 1, 0x7f, 2], c_char::MAX);
    compare_process(c, rust, b"a\0aaa", b'a' as c_char);
}

fn test_update_flags(c: &Api, rust: &Api, rng: &mut Rng) {
    for combination in 0..64_i32 {
        let counter = rng.next_u32() % 31;
        let high_bits = (rng.next_u32() as i32) & !0x3f;
        compare_update(c, rust, high_bits | combination, counter);
        compare_update(c, rust, combination, 31);
    }
}

fn test_confuse_types(c: &Api, rust: &Api, rng: &mut Rng) {
    for _ in 0..64 {
        compare_confuse(c, rust, rng.next_u32(), 0);
    }

    let mut finite_count = 0;
    while finite_count < 128 {
        let bits = rng.next_u32();
        if f32::from_bits(bits).is_finite() {
            compare_confuse(c, rust, bits, 1);
            finite_count += 1;
        }
    }

    let special_float_bits = [
        f32::INFINITY.to_bits(),
        f32::NEG_INFINITY.to_bits(),
        f32::NAN.to_bits(),
        0x7f80_0001,
        0xffc0_0001,
        (c_int::MAX as f32).to_bits(),
        (c_int::MIN as f32).to_bits(),
        1.0e20_f32.to_bits(),
        (-1.0e20_f32).to_bits(),
    ];
    for bits in special_float_bits {
        for _ in 0..8 {
            compare_confuse(c, rust, bits, 1);
        }
    }

    for _ in 0..128 {
        compare_confuse(c, rust, rng.next_u32(), 2);
        compare_confuse(c, rust, rng.next_u32(), 3);
    }
    for operation in [c_int::MIN, -100, -1, 4, 5, 100, c_int::MAX] {
        for _ in 0..16 {
            compare_confuse(c, rust, rng.next_u32(), operation);
        }
    }
}

fn choose_param1(rng: &mut Rng, index: usize) -> c_int {
    const EDGES: [c_int; 12] = [
        c_int::MIN,
        c_int::MIN + 1,
        -1_010_101_010,
        -100,
        -1,
        0,
        1,
        10,
        101,
        1_010_101_010,
        c_int::MAX - 1,
        c_int::MAX,
    ];
    if index % 4 == 0 {
        EDGES[(index / 4) % EDGES.len()]
    } else {
        rng.next_u32() as c_int
    }
}

fn test_composed_confusion(c: &Api, rust: &Api, rng: &mut Rng) {
    // These are the 5 operation classes x 3 search-target classes in rows 21-35.
    for operation_class in 0..5 {
        for target_class in 0..3 {
            for combination in 0..64_i32 {
                let param1 = choose_param1(rng, combination as usize);
                let param2 = if combination & 1 == 0 {
                    combination + 64 * (rng.next_u32() % 1024) as i32
                } else {
                    combination - 64 * (1 + (rng.next_u32() % 1024) as i32)
                };

                let target_remainder = match target_class {
                    0 => {
                        const ABSENT: [i32; 8] = [-9, -8, -7, -6, -5, -4, -2, -1];
                        ABSENT[combination as usize % ABSENT.len()]
                    }
                    1 => -3,
                    2 => combination % 10,
                    _ => unreachable!(),
                };
                let param3 = if target_remainder < 0 {
                    target_remainder - 10 * (rng.next_u32() % 1000) as i32
                } else {
                    target_remainder + 10 * (rng.next_u32() % 1000) as i32
                };

                let operation_remainder = match operation_class {
                    0..=3 => operation_class,
                    4 => -1 - (combination % 3),
                    _ => unreachable!(),
                };
                let param4 = if operation_remainder < 0 {
                    operation_remainder - 4 * (rng.next_u32() % 1000) as i32
                } else {
                    operation_remainder + 4 * (rng.next_u32() % 1000) as i32
                };
                compare_confusion(c, rust, param1, param2, param3, param4);
            }
        }
    }
}

fn allocate_state_with_null_buffer() -> *mut ProcessState {
    unsafe {
        let state = malloc(size_of::<ProcessState>()).cast::<ProcessState>();
        assert!(!state.is_null());
        state.write(ProcessState {
            flags: 0x7b05,
            data: 0,
            buffer: ptr::null_mut(),
            capacity: 0,
        });
        state
    }
}

fn test_null_and_invalid_boundaries(c: &Api, rust: &Api) {
    let (_, c_output) = capture_stdout(|| unsafe { (c.destroy_state)(ptr::null_mut()) });
    let (_, rust_output) = capture_stdout(|| unsafe { (rust.destroy_state)(ptr::null_mut()) });
    assert_eq!(c_output, rust_output);

    let c_state = allocate_state_with_null_buffer();
    let rust_state = allocate_state_with_null_buffer();
    let (_, c_output) = capture_stdout(|| unsafe { (c.destroy_state)(c_state) });
    let (_, rust_output) = capture_stdout(|| unsafe { (rust.destroy_state)(rust_state) });
    assert_eq!(c_output, rust_output);

    let (c_result, c_output) = capture_stdout(|| unsafe { (c.process_buffer)(ptr::null_mut(), 0) });
    let (rust_result, rust_output) =
        capture_stdout(|| unsafe { (rust.process_buffer)(ptr::null_mut(), 0) });
    assert_eq!((c_result, c_output), (rust_result, rust_output));
    assert_eq!(c_result, -1);

    let c_state = allocate_state_with_null_buffer();
    let rust_state = allocate_state_with_null_buffer();
    let (c_result, c_output) = capture_stdout(|| unsafe { (c.process_buffer)(c_state, 0) });
    let (rust_result, rust_output) =
        capture_stdout(|| unsafe { (rust.process_buffer)(rust_state, 0) });
    assert_eq!((c_result, c_output), (rust_result, rust_output));
    unsafe {
        free(c_state.cast());
        free(rust_state.cast());
    }

    let (_, c_output) = capture_stdout(|| unsafe { (c.update_flags)(ptr::null_mut(), c_int::MAX) });
    let (_, rust_output) =
        capture_stdout(|| unsafe { (rust.update_flags)(ptr::null_mut(), c_int::MAX) });
    assert_eq!(c_output, rust_output);

    let (c_result, c_output) = capture_stdout(|| unsafe { (c.confuse_types)(ptr::null_mut(), 0) });
    let (rust_result, rust_output) =
        capture_stdout(|| unsafe { (rust.confuse_types)(ptr::null_mut(), 0) });
    assert_eq!((c_result, c_output), (rust_result, rust_output));
    assert_eq!(c_result, 0);
}

fn compile_allocator_interposer() -> PathBuf {
    let output_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/test-support");
    std::fs::create_dir_all(&output_dir).unwrap();
    let output = output_dir.join("libfail_alloc.so");
    let status = Command::new("cc")
        .args(["-shared", "-fPIC", "-O2"])
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fail_alloc.c"))
        .arg("-o")
        .arg(&output)
        .status()
        .expect("failed to invoke cc for allocator interposer");
    assert!(status.success());
    assert!(output.is_file());
    output
}

fn run_allocator_failure_child() {
    let interposer = compile_allocator_interposer();
    let existing_preload = std::env::var_os("LD_PRELOAD").unwrap_or_default();
    let mut preload = interposer.as_os_str().to_os_string();
    if !existing_preload.is_empty() {
        preload.push(":");
        preload.push(existing_preload);
    }
    let status = Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "allocation_failure_child",
            "--nocapture",
        ])
        .env("LD_PRELOAD", preload)
        .env("ALLOCATOR_INTERPOSER", &interposer)
        .status()
        .expect("failed to run allocation-failure child");
    assert!(status.success(), "allocation-failure child failed");
}

#[test]
fn differential_surface() {
    assert_eq!(size_of::<ProcessState>(), 24);
    let (c, rust) = load_apis();
    let mut rng = Rng::new(0x5eed_c0de_1234_5678);
    test_create_and_destroy(&c, &rust, &mut rng);
    test_process_buffer(&c, &rust, &mut rng);
    test_update_flags(&c, &rust, &mut rng);
    test_confuse_types(&c, &rust, &mut rng);
    test_composed_confusion(&c, &rust, &mut rng);
    test_null_and_invalid_boundaries(&c, &rust);
    run_allocator_failure_child();
}

#[test]
#[ignore = "run in a child process with the malloc interposer preloaded"]
fn allocation_failure_child() {
    type FailNextMalloc = unsafe extern "C" fn(usize);

    let interposer_path = std::env::var_os("ALLOCATOR_INTERPOSER")
        .expect("allocator child must have ALLOCATOR_INTERPOSER");
    let interposer = unsafe { Library::new(interposer_path) }.unwrap();
    let fail_next: FailNextMalloc =
        unsafe { *interposer.get(b"fail_next_malloc_of_size\0").unwrap() };
    let (c, rust) = load_apis();

    let (c_state, c_output) = capture_stdout(|| unsafe {
        fail_next(size_of::<ProcessState>());
        (c.create_state)(7, 128)
    });
    let (rust_state, rust_output) = capture_stdout(|| unsafe {
        fail_next(size_of::<ProcessState>());
        (rust.create_state)(7, 128)
    });
    assert!(c_state.is_null() && rust_state.is_null());
    assert_eq!(c_output, rust_output);
    assert_eq!(c_output, b"Error: Failed to allocate memory for state\n");

    let (c_state, c_output) = capture_stdout(|| unsafe {
        fail_next(31_337);
        (c.create_state)(7, 31_337)
    });
    let (rust_state, rust_output) = capture_stdout(|| unsafe {
        fail_next(31_337);
        (rust.create_state)(7, 31_337)
    });
    assert!(c_state.is_null() && rust_state.is_null());
    assert_eq!(c_output, rust_output);
    assert_eq!(c_output, b"Error: Failed to allocate buffer\n");

    let (c_result, c_output) = capture_stdout(|| unsafe {
        fail_next(size_of::<ProcessState>());
        (c.confusion)(1, 2, 3, 0)
    });
    let (rust_result, rust_output) = capture_stdout(|| unsafe {
        fail_next(size_of::<ProcessState>());
        (rust.confusion)(1, 2, 3, 0)
    });
    assert_eq!(c_result, -1);
    assert_eq!(c_result, rust_result);
    assert_eq!(c_output, rust_output);
}
