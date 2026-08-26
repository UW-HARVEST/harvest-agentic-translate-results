use libloading::Library;
use std::ffi::{c_char, c_int, c_uint};
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom};
use std::mem::size_of;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

type BinOp = unsafe extern "C" fn(c_int, c_int) -> c_int;
type Operation = Option<BinOp>;
type GetOperation = unsafe extern "C" fn(c_int) -> Operation;
type ExecuteOperation = unsafe extern "C" fn(Operation, c_int, c_int, *const c_char) -> c_int;
type ComputeChecksum = unsafe extern "C" fn(*mut c_int, c_int) -> c_uint;
type InitState = unsafe extern "C" fn(*mut ComputeState, c_int);
type ApplyOperation = unsafe extern "C" fn(*mut ComputeState, c_int, Operation);
type Checkshift = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ComputeState {
    accumulator: c_int,
    operation_count: c_int,
    checksum: c_uint,
}

struct Api {
    _library: Library,
    multiply: BinOp,
    add: BinOp,
    xor: BinOp,
    shift: BinOp,
    get_operation: GetOperation,
    execute_operation: ExecuteOperation,
    compute_checksum: ComputeChecksum,
    init_state: InitState,
    apply_operation: ApplyOperation,
    checkshift: Checkshift,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));

        unsafe {
            Self {
                multiply: *library.get(b"multiply_with_static\0").unwrap(),
                add: *library.get(b"add_with_static\0").unwrap(),
                xor: *library.get(b"xor_operation\0").unwrap(),
                shift: *library.get(b"shift_with_static\0").unwrap(),
                get_operation: *library.get(b"get_operation\0").unwrap(),
                execute_operation: *library.get(b"execute_operation\0").unwrap(),
                compute_checksum: *library.get(b"compute_checksum\0").unwrap(),
                init_state: *library.get(b"init_state\0").unwrap(),
                apply_operation: *library.get(b"apply_operation\0").unwrap(),
                checkshift: *library.get(b"checkshift\0").unwrap(),
                _library: library,
            }
        }
    }

    fn operations(&self) -> [BinOp; 4] {
        [self.multiply, self.add, self.xor, self.shift]
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
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("c_src/build/libtranslated_rust.so")
}

fn rust_library_path() -> PathBuf {
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir().join("target"));
    let candidates = [
        target.join("debug/libcheckshift_lib.so"),
        target.join("debug/deps/libcheckshift_lib.so"),
    ];

    candidates
        .into_iter()
        .find(|path| path.is_file())
        .unwrap_or_else(|| target.join("debug/libcheckshift_lib.so"))
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

fn integer_pairs(seed: u64, random_count: usize) -> Vec<(i32, i32)> {
    let mut pairs = vec![
        (0, 0),
        (1, -1),
        (-1, 1),
        (i32::MIN, i32::MIN),
        (i32::MIN, i32::MAX),
        (i32::MAX, i32::MIN),
        (i32::MAX, i32::MAX),
        (0x4000_0000, 4),
        (-0x4000_0000, -4),
    ];
    let mut rng = Rng::new(seed);
    pairs.extend((0..random_count).map(|_| (rng.next_i32(), rng.next_i32())));
    pairs
}

fn states(seed: u64, random_count: usize) -> Vec<(ComputeState, i32)> {
    let mut cases = vec![
        (
            ComputeState {
                accumulator: 0,
                operation_count: 0,
                checksum: 0,
            },
            0,
        ),
        (
            ComputeState {
                accumulator: i32::MIN,
                operation_count: i32::MAX,
                checksum: u32::MAX,
            },
            i32::MAX,
        ),
        (
            ComputeState {
                accumulator: i32::MAX,
                operation_count: i32::MIN,
                checksum: 0xDEAD_BEEF,
            },
            i32::MIN,
        ),
    ];
    let mut rng = Rng::new(seed);
    cases.extend((0..random_count).map(|_| {
        (
            ComputeState {
                accumulator: rng.next_i32(),
                operation_count: rng.next_i32(),
                checksum: rng.next_u64() as u32,
            },
            rng.next_i32(),
        )
    }));
    cases
}

fn state_bytes(state: &ComputeState) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(ptr::from_ref(state).cast::<u8>(), size_of::<ComputeState>())
    }
}

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
}

static CAPTURE_ID: AtomicU64 = AtomicU64::new(0);

fn capture_stdout<T>(action: impl FnOnce() -> T) -> (T, Vec<u8>) {
    let id = CAPTURE_ID.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "checkshift-differential-{}-{id}.out",
        std::process::id()
    ));
    let mut file = OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();

    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0);
        assert_eq!(dup2(file.as_raw_fd(), 1), 1);

        let result = action();

        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, 1), 1);
        assert_eq!(close(saved_stdout), 0);

        file.seek(SeekFrom::Start(0)).unwrap();
        let mut output = Vec::new();
        file.read_to_end(&mut output).unwrap();
        drop(file);
        std::fs::remove_file(path).unwrap();
        (result, output)
    }
}

fn compare_direct_operations(c: &Api, rust: &Api) {
    let pairs = integer_pairs(0xC01C_02C0_3C04, 512);
    for (row, (c_op, rust_op)) in c
        .operations()
        .into_iter()
        .zip(rust.operations())
        .enumerate()
    {
        for &(a, b) in &pairs {
            assert_eq!(
                unsafe { c_op(a, b) },
                unsafe { rust_op(a, b) },
                "C{:02}: a={a}, b={b}",
                row + 1
            );
        }
    }
}

fn compare_operation_lookup(c: &Api, rust: &Api) {
    let first_c = unsafe { (c.get_operation)(0) }.expect("C05 C callback");
    let first_rust = unsafe { (rust.get_operation)(0) }.expect("C05 Rust callback");
    let pairs = integer_pairs(0xC05C_06C0_7C09, 128);
    for &(a, b) in &pairs {
        assert_eq!(
            unsafe { first_c(a, b) },
            unsafe { first_rust(a, b) },
            "C05: a={a}, b={b}"
        );
    }

    for opcode in 0..4 {
        let c_op = unsafe { (c.get_operation)(opcode) }.expect("C callback");
        let rust_op = unsafe { (rust.get_operation)(opcode) }.expect("Rust callback");
        for &(a, b) in &pairs {
            assert_eq!(
                unsafe { c_op(a, b) },
                unsafe { rust_op(a, b) },
                "C{:02}: opcode={opcode}, a={a}, b={b}",
                opcode + 6
            );
        }
    }
}

fn run_execute_cases(api: &Api) -> Vec<i32> {
    let pairs = integer_pairs(0xC10C_11C1_2C13, 64);
    let names = [c"MULTIPLY", c"ADD", c"XOR", c"SHIFT"];
    let mut results = Vec::new();

    for (operation, name) in api.operations().into_iter().zip(names) {
        for &(a, b) in &pairs {
            results.push(unsafe { (api.execute_operation)(Some(operation), a, b, name.as_ptr()) });
        }
    }

    // Generic pointer boundary: C's target libc accepts a null `%s` pointer.
    results.push(unsafe { (api.execute_operation)(Some(api.xor), 0x1234, 0x5678, ptr::null()) });
    results
}

fn compare_execute_operation(c: &Api, rust: &Api) {
    let (c_results, c_output) = capture_stdout(|| run_execute_cases(c));
    let (rust_results, rust_output) = capture_stdout(|| run_execute_cases(rust));
    assert_eq!(c_results, rust_results, "C10-C13 return values");
    assert_eq!(c_output, rust_output, "C10-C13 stdout bytes");
}

fn compare_checksums(c: &Api, rust: &Api) {
    let mut rng = Rng::new(0xC14C_15C1_6C18);
    for count in [1, 2, 3, 4, 5, 17, i32::MAX] {
        for _ in 0..128 {
            let mut c_values = [0i32; 8];
            for value in &mut c_values {
                *value = rng.next_i32();
            }
            let mut rust_values = c_values;
            let c_result = unsafe { (c.compute_checksum)(c_values.as_mut_ptr(), count) };
            let rust_result = unsafe { (rust.compute_checksum)(rust_values.as_mut_ptr(), count) };
            assert_eq!(c_result, rust_result, "C14-C18 count={count}");
            assert_eq!(c_values, rust_values, "C14-C18 input bytes count={count}");
        }
    }
}

fn run_init_cases(api: &Api, values: &[i32]) -> Vec<ComputeState> {
    values
        .iter()
        .map(|&value| {
            let mut state = ComputeState {
                accumulator: 0x5A5A_5A5A,
                operation_count: 0x5A5A_5A5A,
                checksum: 0xA5A5_A5A5,
            };
            unsafe { (api.init_state)(&mut state, value) };
            state
        })
        .collect()
}

fn compare_init_state(c: &Api, rust: &Api) {
    let mut rng = Rng::new(0xC19C_19C1_9C19);
    let mut values = vec![0, 1, -1, i32::MIN, i32::MAX];
    values.extend((0..128).map(|_| rng.next_i32()));

    let (c_states, c_output) = capture_stdout(|| run_init_cases(c, &values));
    let (rust_states, rust_output) = capture_stdout(|| run_init_cases(rust, &values));
    assert_eq!(c_output, rust_output, "C19 stdout bytes");
    for (c_state, rust_state) in c_states.iter().zip(&rust_states) {
        assert_eq!(state_bytes(c_state), state_bytes(rust_state), "C19");
    }
}

fn run_apply_cases(api: &Api, cases: &[(ComputeState, i32)]) -> Vec<ComputeState> {
    let mut results = Vec::new();
    for operation in api.operations() {
        for &(initial, value) in cases {
            let mut state = initial;
            unsafe { (api.apply_operation)(&mut state, value, Some(operation)) };
            results.push(state);
        }
    }
    results
}

fn compare_apply_operation(c: &Api, rust: &Api) {
    let cases = states(0xC20C_21C2_2C23, 256);
    let c_states = run_apply_cases(c, &cases);
    let rust_states = run_apply_cases(rust, &cases);
    for (index, (c_state, rust_state)) in c_states.iter().zip(&rust_states).enumerate() {
        assert_eq!(
            state_bytes(c_state),
            state_bytes(rust_state),
            "C20-C23 case {index}"
        );
    }
}

fn run_checkshift_cases(api: &Api, cases: &[[i32; 4]]) -> Vec<i32> {
    cases
        .iter()
        .map(|values| unsafe { (api.checkshift)(values[0], values[1], values[2], values[3]) })
        .collect()
}

fn compare_checkshift(c: &Api, rust: &Api) {
    let mut cases = vec![
        [0, 0, 0, 0],
        [1, 2, 3, 4],
        [-1, -2, -3, -4],
        [i32::MIN, i32::MAX, i32::MIN, i32::MAX],
        [i32::MAX, i32::MIN, i32::MAX, i32::MIN],
    ];
    let mut rng = Rng::new(0xC24C_24C2_4C24);
    cases.extend((0..128).map(|_| {
        [
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
        ]
    }));

    let (c_results, c_output) = capture_stdout(|| run_checkshift_cases(c, &cases));
    let (rust_results, rust_output) = capture_stdout(|| run_checkshift_cases(rust, &cases));
    assert_eq!(c_results, rust_results, "C24 return values");
    assert_eq!(c_output, rust_output, "C24 stdout bytes");
}

fn compare_error_surface(c: &Api, rust: &Api) {
    for opcode in [-1, i32::MIN, 4, 5, i32::MAX] {
        assert_eq!(
            unsafe { (c.get_operation)(opcode) }.is_none(),
            unsafe { (rust.get_operation)(opcode) }.is_none(),
            "E01-E02 opcode={opcode}"
        );
    }

    let (c_execute, c_output) =
        capture_stdout(|| unsafe { (c.execute_operation)(None, 17, 23, c"NULL".as_ptr()) });
    let (rust_execute, rust_output) =
        capture_stdout(|| unsafe { (rust.execute_operation)(None, 17, 23, c"NULL".as_ptr()) });
    assert_eq!(c_execute, rust_execute, "E03 result");
    assert_eq!(c_execute, 0, "E03 exact sentinel");
    assert_eq!(c_output, rust_output, "E03 stdout bytes");

    let mut c_values = [1, 2, 3, 4];
    let mut rust_values = c_values;
    for count in [1, i32::MAX] {
        let c_result = unsafe { (c.compute_checksum)(ptr::null_mut(), count) };
        let rust_result = unsafe { (rust.compute_checksum)(ptr::null_mut(), count) };
        assert_eq!(c_result, rust_result, "E04 count={count}");
        assert_eq!(c_result, 0, "E04 exact sentinel count={count}");
    }
    for count in [0, -1, i32::MIN] {
        let c_result = unsafe { (c.compute_checksum)(c_values.as_mut_ptr(), count) };
        let rust_result = unsafe { (rust.compute_checksum)(rust_values.as_mut_ptr(), count) };
        assert_eq!(c_result, rust_result, "E05-E06 count={count}");
        assert_eq!(c_result, 0, "E05-E06 exact sentinel count={count}");
    }

    let ((), c_output) = capture_stdout(|| unsafe { (c.init_state)(ptr::null_mut(), 17) });
    let ((), rust_output) = capture_stdout(|| unsafe { (rust.init_state)(ptr::null_mut(), 17) });
    assert_eq!(c_output, rust_output, "E07 stdout bytes");

    CALLBACK_CALLS.store(0, Ordering::SeqCst);
    let ((), c_output) = capture_stdout(|| unsafe {
        (c.apply_operation)(ptr::null_mut(), 17, Some(counting_callback))
    });
    assert_eq!(CALLBACK_CALLS.load(Ordering::SeqCst), 0, "E08 C callback");
    let ((), rust_output) = capture_stdout(|| unsafe {
        (rust.apply_operation)(ptr::null_mut(), 17, Some(counting_callback))
    });
    assert_eq!(
        CALLBACK_CALLS.load(Ordering::SeqCst),
        0,
        "E08 Rust callback"
    );
    assert_eq!(c_output, rust_output, "E08 stdout bytes");

    let initial = ComputeState {
        accumulator: 0x1234_5678,
        operation_count: -123,
        checksum: 0xDEAD_BEEF,
    };
    let mut c_state = initial;
    let mut rust_state = initial;
    let ((), c_output) = capture_stdout(|| unsafe { (c.apply_operation)(&mut c_state, 17, None) });
    let ((), rust_output) =
        capture_stdout(|| unsafe { (rust.apply_operation)(&mut rust_state, 17, None) });
    assert_eq!(state_bytes(&c_state), state_bytes(&initial), "E09 C state");
    assert_eq!(
        state_bytes(&rust_state),
        state_bytes(&initial),
        "E09 Rust state"
    );
    assert_eq!(c_output, rust_output, "E09 stdout bytes");
}

static CALLBACK_CALLS: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C" fn counting_callback(_a: c_int, _b: c_int) -> c_int {
    CALLBACK_CALLS.fetch_add(1, Ordering::SeqCst);
    0
}

#[test]
fn differential_surface() {
    assert_eq!(size_of::<ComputeState>(), 12);
    let (c, rust) = load_apis();
    compare_direct_operations(&c, &rust);
    compare_operation_lookup(&c, &rust);
    compare_execute_operation(&c, &rust);
    compare_checksums(&c, &rust);
    compare_init_state(&c, &rust);
    compare_apply_operation(&c, &rust);
    compare_checkshift(&c, &rust);
    compare_error_surface(&c, &rust);
}

fn malloc_shim_path() -> PathBuf {
    manifest_dir().join("target/test-support/libfail_malloc.so")
}

fn build_malloc_shim() -> PathBuf {
    let output = malloc_shim_path();
    std::fs::create_dir_all(output.parent().unwrap()).unwrap();
    let status = Command::new("cc")
        .args(["-shared", "-fPIC", "-o"])
        .arg(&output)
        .arg(manifest_dir().join("tests/fail_malloc.c"))
        .status()
        .expect("failed to run cc for malloc shim");
    assert!(status.success(), "failed to build malloc shim");
    output
}

#[test]
fn allocation_failure_child() {
    let Some(library_path) = std::env::var_os("CHECKSHIFT_ALLOC_FAILURE_LIBRARY") else {
        return;
    };

    unsafe {
        let api = Api::load(Path::new(&library_path));
        let process = libloading::os::unix::Library::this();
        let arm: libloading::os::unix::Symbol<unsafe extern "C" fn()> =
            process.get(b"arm_state_malloc_failure\0").unwrap();
        arm();
        let (result, output) = capture_stdout(|| (api.checkshift)(1, 2, 3, 4));
        assert_eq!(result, -1, "E10 exact sentinel");
        assert_eq!(
            output,
            b"\n=== Starting foo function ===\nParameters: 1, 2, 3, 4\nError: Failed to allocate memory for state\n"
        );
    }
}

fn run_allocation_failure_child(library: &Path, shim: &Path) {
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "allocation_failure_child", "--nocapture"])
        .env("LD_PRELOAD", shim)
        .env("CHECKSHIFT_ALLOC_FAILURE_LIBRARY", library)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "E10 child failed for {}:\nstdout:\n{}\nstderr:\n{}",
        library.display(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn allocation_failure_matches() {
    let shim = build_malloc_shim();
    run_allocation_failure_child(&c_library_path(), &shim);
    run_allocation_failure_child(&rust_library_path(), &shim);
}
