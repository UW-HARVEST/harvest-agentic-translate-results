use libloading::{Library, Symbol};
use std::env;
use std::ffi::{c_char, c_int, c_uint};
use std::fs;
use std::mem::{MaybeUninit, size_of};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::Mutex;

type Operation = unsafe extern "C" fn(c_int, c_int) -> c_int;
type OptionalOperation = Option<Operation>;
type GetOperation = unsafe extern "C" fn(c_int) -> OptionalOperation;
type ExecuteOperation =
    unsafe extern "C" fn(OptionalOperation, c_int, c_int, *const c_char) -> c_int;
type ComputeChecksum = unsafe extern "C" fn(*mut c_int, c_int) -> c_uint;
type InitState = unsafe extern "C" fn(*mut ComputeState, c_int);
type ApplyOperation = unsafe extern "C" fn(*mut ComputeState, c_int, OptionalOperation);
type Checkshift = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

const RANDOM_CASES: usize = 96;
const OP_NAME: &[u8] = b"DIFFERENTIAL\0";
static TEST_LOCK: Mutex<()> = Mutex::new(());

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct ComputeState {
    accumulator: c_int,
    operation_count: c_int,
    checksum: c_uint,
}

struct Api {
    _library: Library,
    multiply: Operation,
    add: Operation,
    xor: Operation,
    shift: Operation,
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

        unsafe fn load_symbol<T: Copy>(library: &Library, name: &[u8]) -> T {
            let symbol: Symbol<'_, T> = unsafe { library.get(name) }
                .unwrap_or_else(|error| panic!("failed to load {:?}: {error}", name));
            *symbol
        }

        Self {
            multiply: unsafe { load_symbol(&library, b"multiply_with_static\0") },
            add: unsafe { load_symbol(&library, b"add_with_static\0") },
            xor: unsafe { load_symbol(&library, b"xor_operation\0") },
            shift: unsafe { load_symbol(&library, b"shift_with_static\0") },
            get_operation: unsafe { load_symbol(&library, b"get_operation\0") },
            execute_operation: unsafe { load_symbol(&library, b"execute_operation\0") },
            compute_checksum: unsafe { load_symbol(&library, b"compute_checksum\0") },
            init_state: unsafe { load_symbol(&library, b"init_state\0") },
            apply_operation: unsafe { load_symbol(&library, b"apply_operation\0") },
            checkshift: unsafe { load_symbol(&library, b"checkshift\0") },
            _library: library,
        }
    }
}

#[derive(Clone, Copy)]
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

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir()
        .join("../c_src/build")
        .join("libharvest-work-heRBy4.so")
}

fn rust_library_path() -> PathBuf {
    manifest_dir()
        .join("target/release")
        .join("libcheckshift_lib.so")
}

fn load_apis() -> (Api, Api) {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(c_path.is_file(), "missing C library: {}", c_path.display());
    assert!(
        rust_path.is_file(),
        "missing Rust library: {}; run cargo build --release first",
        rust_path.display()
    );
    unsafe { (Api::load(&c_path), Api::load(&rust_path)) }
}

fn load_fresh_apis() -> (Api, Api, PathBuf) {
    let directory = manifest_dir()
        .join("target/test-support")
        .join(format!("fresh-libraries-{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).unwrap();
    let c_copy = directory.join("libreference.so");
    let rust_copy = directory.join("libtranslation.so");
    fs::copy(c_library_path(), &c_copy).unwrap();
    fs::copy(rust_library_path(), &rust_copy).unwrap();
    let apis = unsafe { (Api::load(&c_copy), Api::load(&rust_copy)) };
    (apis.0, apis.1, directory)
}

fn integer_pairs(seed: u64) -> Vec<(i32, i32)> {
    let mut cases = vec![
        (0, 0),
        (1, -1),
        (-1, 1),
        (i32::MIN, i32::MAX),
        (i32::MAX, i32::MIN),
        (i32::MIN, -1),
        (i32::MAX, 1),
        (0x1234_5678, -0x1234_567),
    ];
    let mut rng = Rng::new(seed);
    cases.extend((0..RANDOM_CASES).map(|_| (rng.next_i32(), rng.next_i32())));
    cases
}

fn integer_quads(seed: u64) -> Vec<[i32; 4]> {
    let mut cases = vec![
        [0, 0, 0, 0],
        [1, -1, i32::MIN, i32::MAX],
        [i32::MIN, i32::MAX, -1, 1],
        [i32::MAX, i32::MAX, i32::MAX, i32::MAX],
    ];
    let mut rng = Rng::new(seed);
    cases.extend((0..RANDOM_CASES).map(|_| {
        [
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
        ]
    }));
    cases
}

fn state_bytes(state: &ComputeState) -> Vec<u8> {
    unsafe {
        std::slice::from_raw_parts(ptr::from_ref(state).cast::<u8>(), size_of::<ComputeState>())
            .to_vec()
    }
}

unsafe fn initialized_state(api: &Api, initial: i32) -> ComputeState {
    let mut state = MaybeUninit::<ComputeState>::uninit();
    unsafe { (api.init_state)(state.as_mut_ptr(), initial) };
    unsafe { state.assume_init() }
}

#[test]
fn valid_configuration_rows_match() {
    let _guard = TEST_LOCK.lock().unwrap();
    let pairs = integer_pairs(0xC0FF_EE11_5EED_1234);

    // CONFIGS row 5: the first valid lookup takes the lazy initialization branch.
    let (fresh_c, fresh_rust, fresh_directory) = load_fresh_apis();
    {
        let c_first = unsafe { (fresh_c.get_operation)(3) }.expect("C first operation");
        let rust_first = unsafe { (fresh_rust.get_operation)(3) }.expect("Rust first operation");
        for &(a, b) in &pairs {
            assert_eq!(
                unsafe { c_first(a, b) },
                unsafe { rust_first(a, b) },
                "CONFIGS row 5: ({a}, {b})"
            );
        }
    }
    drop((fresh_c, fresh_rust));
    fs::remove_dir_all(fresh_directory).unwrap();

    let (c, rust) = load_apis();

    // CONFIGS rows 1-4: direct operation exports.
    let direct = [
        (1, c.multiply, rust.multiply),
        (2, c.add, rust.add),
        (3, c.xor, rust.xor),
        (4, c.shift, rust.shift),
    ];
    for &(row, c_operation, rust_operation) in &direct {
        for &(a, b) in &pairs {
            assert_eq!(
                unsafe { c_operation(a, b) },
                unsafe { rust_operation(a, b) },
                "CONFIGS row {row}: ({a}, {b})"
            );
        }
    }

    // CONFIGS rows 6-9: initialized dispatch table, one row per opcode.
    for opcode in 0..4 {
        let row = opcode + 6;
        let c_operation = unsafe { (c.get_operation)(opcode) }.expect("C valid operation pointer");
        let rust_operation =
            unsafe { (rust.get_operation)(opcode) }.expect("Rust valid operation pointer");
        for &(a, b) in &pairs {
            assert_eq!(
                unsafe { c_operation(a, b) },
                unsafe { rust_operation(a, b) },
                "CONFIGS row {row}, opcode {opcode}: ({a}, {b})"
            );
        }
    }

    // CONFIGS rows 10-13: execute each non-null operation through the wrapper.
    for (index, &(c_operation, rust_operation)) in [
        (c.multiply, rust.multiply),
        (c.add, rust.add),
        (c.xor, rust.xor),
        (c.shift, rust.shift),
    ]
    .iter()
    .enumerate()
    {
        let row = index + 10;
        for &(a, b) in &pairs {
            assert_eq!(
                unsafe { (c.execute_operation)(Some(c_operation), a, b, OP_NAME.as_ptr().cast()) },
                unsafe {
                    (rust.execute_operation)(Some(rust_operation), a, b, OP_NAME.as_ptr().cast())
                },
                "CONFIGS row {row}: ({a}, {b})"
            );
        }
    }

    // CONFIGS rows 14-18: every count shape the C implementation distinguishes.
    let mut rng = Rng::new(0x51A5_51A5_DEAD_BEEF);
    for _ in 0..RANDOM_CASES {
        let mut c_values = [0_i32; 8];
        for value in &mut c_values {
            *value = rng.next_i32();
        }
        let mut rust_values = c_values;
        for (row, count) in [(14, 1), (15, 2), (16, 3), (17, 4), (18, 8)] {
            assert_eq!(
                unsafe { (c.compute_checksum)(c_values.as_mut_ptr(), count) },
                unsafe { (rust.compute_checksum)(rust_values.as_mut_ptr(), count) },
                "CONFIGS row {row}, count {count}: {c_values:?}"
            );
        }
    }

    // CONFIGS row 19: init_state writes the complete C-layout state.
    let mut rng = Rng::new(0x1A17_57A7_E123_4567);
    for _ in 0..RANDOM_CASES {
        let initial = rng.next_i32();
        let c_state = unsafe { initialized_state(&c, initial) };
        let rust_state = unsafe { initialized_state(&rust, initial) };
        assert_eq!(
            state_bytes(&c_state),
            state_bytes(&rust_state),
            "CONFIGS row 19, initial {initial}"
        );
    }

    // CONFIGS rows 20-23: state mutation through each operation pointer.
    for (index, &(c_operation, rust_operation)) in [
        (c.multiply, rust.multiply),
        (c.add, rust.add),
        (c.xor, rust.xor),
        (c.shift, rust.shift),
    ]
    .iter()
    .enumerate()
    {
        let row = index + 20;
        let mut rng = Rng::new(0xA991_0000_0000_0000 | row as u64);
        for _ in 0..RANDOM_CASES {
            let mut c_state = ComputeState {
                accumulator: rng.next_i32(),
                operation_count: rng.next_i32(),
                checksum: rng.next_u32(),
            };
            let mut rust_state = c_state;
            let value = rng.next_i32();
            unsafe {
                (c.apply_operation)(&mut c_state, value, Some(c_operation));
                (rust.apply_operation)(&mut rust_state, value, Some(rust_operation));
            }
            assert_eq!(
                state_bytes(&c_state),
                state_bytes(&rust_state),
                "CONFIGS row {row}, value {value}"
            );
        }
    }

    // CONFIGS row 24: exercise the complete composition through low-level exports.
    for params in integer_quads(0x10A1_EA7E_CAFE_BABE) {
        let mut c_state = unsafe { initialized_state(&c, params[0]) };
        let mut rust_state = unsafe { initialized_state(&rust, params[0]) };
        let mut c_params = params;
        let mut rust_params = params;

        let c_operations: Vec<Operation> = (0..4)
            .map(|opcode| unsafe { (c.get_operation)(opcode) }.unwrap())
            .collect();
        let rust_operations: Vec<Operation> = (0..4)
            .map(|opcode| unsafe { (rust.get_operation)(opcode) }.unwrap())
            .collect();

        unsafe {
            (c.apply_operation)(&mut c_state, params[1], Some(c_operations[0]));
            (rust.apply_operation)(&mut rust_state, params[1], Some(rust_operations[0]));
            (c.apply_operation)(&mut c_state, params[2], Some(c_operations[1]));
            (rust.apply_operation)(&mut rust_state, params[2], Some(rust_operations[1]));
        }
        assert_eq!(state_bytes(&c_state), state_bytes(&rust_state));

        let c_xor = unsafe {
            (c.execute_operation)(
                Some(c_operations[2]),
                c_state.accumulator,
                params[3],
                OP_NAME.as_ptr().cast(),
            )
        };
        let rust_xor = unsafe {
            (rust.execute_operation)(
                Some(rust_operations[2]),
                rust_state.accumulator,
                params[3],
                OP_NAME.as_ptr().cast(),
            )
        };
        assert_eq!(c_xor, rust_xor, "CONFIGS row 24 XOR: {params:?}");

        let c_shift = unsafe {
            (c.execute_operation)(
                Some(c_operations[3]),
                c_xor,
                params[1],
                OP_NAME.as_ptr().cast(),
            )
        };
        let rust_shift = unsafe {
            (rust.execute_operation)(
                Some(rust_operations[3]),
                rust_xor,
                params[1],
                OP_NAME.as_ptr().cast(),
            )
        };
        assert_eq!(c_shift, rust_shift, "CONFIGS row 24 shift: {params:?}");

        let c_checksum =
            unsafe { (c.compute_checksum)(c_params.as_mut_ptr(), c_params.len() as i32) };
        let rust_checksum =
            unsafe { (rust.compute_checksum)(rust_params.as_mut_ptr(), rust_params.len() as i32) };
        assert_eq!(
            c_checksum, rust_checksum,
            "CONFIGS row 24 checksum: {params:?}"
        );
    }

    // CONFIGS row 25: one-shot public API.
    for [a, b, c_value, d] in integer_quads(0xC4EC_551F_700D_F00D) {
        assert_eq!(
            unsafe { (c.checkshift)(a, b, c_value, d) },
            unsafe { (rust.checkshift)(a, b, c_value, d) },
            "CONFIGS row 25: [{a}, {b}, {c_value}, {d}]"
        );
    }
}

#[test]
fn error_surface_rows_match() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (c, rust) = load_apis();

    // ERRORS row 1 and one-step/full-width range boundaries.
    for opcode in [i32::MIN, -1, 4, 5, i32::MAX] {
        assert!(
            unsafe { (c.get_operation)(opcode) }.is_none(),
            "C accepted invalid opcode {opcode}"
        );
        assert!(
            unsafe { (rust.get_operation)(opcode) }.is_none(),
            "Rust accepted invalid opcode {opcode}"
        );
    }

    // ERRORS row 2.
    for (a, b) in integer_pairs(0xE220_0002_1234_5678) {
        assert_eq!(
            unsafe { (c.execute_operation)(None, a, b, OP_NAME.as_ptr().cast()) },
            0
        );
        assert_eq!(
            unsafe { (rust.execute_operation)(None, a, b, OP_NAME.as_ptr().cast()) },
            0
        );
    }

    // ERRORS row 3.
    for count in [1, 4, 5, i32::MAX] {
        assert_eq!(unsafe { (c.compute_checksum)(ptr::null_mut(), count) }, 0);
        assert_eq!(
            unsafe { (rust.compute_checksum)(ptr::null_mut(), count) },
            0
        );
    }

    // ERRORS row 4 and zero/negative length boundaries.
    let mut c_value = 0x1234_5678;
    let mut rust_value = c_value;
    for count in [0, -1, i32::MIN] {
        assert_eq!(unsafe { (c.compute_checksum)(&mut c_value, count) }, 0);
        assert_eq!(
            unsafe { (rust.compute_checksum)(&mut rust_value, count) },
            0
        );
    }

    // ERRORS row 5.
    unsafe {
        (c.init_state)(ptr::null_mut(), 7);
        (rust.init_state)(ptr::null_mut(), 7);
    }

    // ERRORS row 6.
    unsafe {
        (c.apply_operation)(ptr::null_mut(), 9, Some(c.add));
        (rust.apply_operation)(ptr::null_mut(), 9, Some(rust.add));
    }

    // ERRORS row 7.
    for initial in [i32::MIN, -1, 0, 1, i32::MAX] {
        let mut c_state = ComputeState {
            accumulator: initial,
            operation_count: 99,
            checksum: 0xDEAD_BEEF,
        };
        let mut rust_state = c_state;
        let before = state_bytes(&c_state);
        unsafe {
            (c.apply_operation)(&mut c_state, 123, None);
            (rust.apply_operation)(&mut rust_state, 123, None);
        }
        assert_eq!(state_bytes(&c_state), before);
        assert_eq!(state_bytes(&rust_state), before);
    }

    // ERRORS row 9: unchecked null operation name on this glibc ABI.
    for (a, b) in integer_pairs(0xE220_0009_1234_5678) {
        assert_eq!(
            unsafe { (c.execute_operation)(Some(c.xor), a, b, ptr::null()) },
            unsafe { (rust.execute_operation)(Some(rust.xor), a, b, ptr::null()) }
        );
    }

    // The accepted oversized count boundary reads exactly four values.
    let mut c_values = [11, 22, 33, 44];
    let mut rust_values = c_values;
    assert_eq!(
        unsafe { (c.compute_checksum)(c_values.as_mut_ptr(), i32::MAX) },
        unsafe { (rust.compute_checksum)(rust_values.as_mut_ptr(), i32::MAX) }
    );
}

fn interposer_path() -> PathBuf {
    manifest_dir()
        .join("target/test-support")
        .join("libmalloc_fail.so")
}

fn build_malloc_interposer(path: &Path) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let status = Command::new("cc")
        .args(["-std=c11", "-shared", "-fPIC", "-O2"])
        .arg(manifest_dir().join("tests/malloc_fail.c"))
        .arg("-o")
        .arg(path)
        .status()
        .expect("failed to run cc for malloc interposer");
    assert!(status.success(), "malloc interposer build failed");
}

#[test]
fn error_malloc_failure_matches() {
    let _guard = TEST_LOCK.lock().unwrap();
    let interposer = interposer_path();

    if env::var_os("CHECKSHIFT_MALLOC_FAILURE_CHILD").is_none() {
        build_malloc_interposer(&interposer);
        let existing_preload = env::var_os("LD_PRELOAD").unwrap_or_default();
        let mut preload = interposer.as_os_str().to_owned();
        if !existing_preload.is_empty() {
            preload.push(":");
            preload.push(existing_preload);
        }

        let status = Command::new(env::current_exe().unwrap())
            .args(["--exact", "error_malloc_failure_matches", "--nocapture"])
            .env("CHECKSHIFT_MALLOC_FAILURE_CHILD", "1")
            .env("LD_PRELOAD", preload)
            .status()
            .expect("failed to start malloc-failure child test");
        assert!(status.success(), "malloc-failure child test failed");
        return;
    }

    assert!(
        interposer.is_file(),
        "preloaded interposer is missing: {}",
        interposer.display()
    );
    let interposer_library = unsafe { Library::new(&interposer) }.unwrap();
    let arm: Symbol<'_, unsafe extern "C" fn(usize)> =
        unsafe { interposer_library.get(b"fail_next_malloc_of_size\0") }.unwrap();
    let (c, rust) = load_apis();

    unsafe { arm(size_of::<ComputeState>()) };
    let c_result = unsafe { (c.checkshift)(1, 2, 3, 4) };
    unsafe { arm(size_of::<ComputeState>()) };
    let rust_result = unsafe { (rust.checkshift)(1, 2, 3, 4) };

    assert_eq!(c_result, -1, "ERRORS row 8 C result");
    assert_eq!(rust_result, c_result, "ERRORS row 8 Rust result");
}
