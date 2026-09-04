use libloading::Library;
use std::collections::BTreeSet;
use std::env;
use std::ffi::{c_char, c_int, c_uint, c_void};
use std::fmt::Debug;
use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::mem::size_of;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard};

type BinaryOp = unsafe extern "C" fn(c_int, c_int) -> c_int;
type OperationFunc = Option<BinaryOp>;
type GetOperation = unsafe extern "C" fn(c_int) -> OperationFunc;
type ExecuteOperation = unsafe extern "C" fn(OperationFunc, c_int, c_int, *const c_char) -> c_int;
type ComputeChecksum = unsafe extern "C" fn(*mut c_int, c_int) -> c_uint;
type InitState = unsafe extern "C" fn(*mut ComputeState, c_int);
type ApplyOperation = unsafe extern "C" fn(*mut ComputeState, c_int, OperationFunc);
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
    multiply: BinaryOp,
    add: BinaryOp,
    xor: BinaryOp,
    shift: BinaryOp,
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
            *unsafe { library.get::<T>(name) }
                .unwrap_or_else(|error| panic!("failed to load symbol {:?}: {error}", name))
        }

        let multiply = unsafe { load_symbol(&library, b"multiply_with_static\0") };
        let add = unsafe { load_symbol(&library, b"add_with_static\0") };
        let xor = unsafe { load_symbol(&library, b"xor_operation\0") };
        let shift = unsafe { load_symbol(&library, b"shift_with_static\0") };
        let get_operation = unsafe { load_symbol(&library, b"get_operation\0") };
        let execute_operation = unsafe { load_symbol(&library, b"execute_operation\0") };
        let compute_checksum = unsafe { load_symbol(&library, b"compute_checksum\0") };
        let init_state = unsafe { load_symbol(&library, b"init_state\0") };
        let apply_operation = unsafe { load_symbol(&library, b"apply_operation\0") };
        let checkshift = unsafe { load_symbol(&library, b"checkshift\0") };

        Self {
            _library: library,
            multiply,
            add,
            xor,
            shift,
            get_operation,
            execute_operation,
            compute_checksum,
            init_state,
            apply_operation,
            checkshift,
        }
    }
}

struct Libraries {
    c: Api,
    rust: Api,
}

impl Libraries {
    fn load() -> Self {
        assert!(
            c_library_path().is_file(),
            "build the C library first at {}",
            c_library_path().display()
        );
        assert!(
            rust_library_path().is_file(),
            "Rust cdylib missing at {}",
            rust_library_path().display()
        );
        unsafe {
            Self {
                c: Api::load(&c_library_path()),
                rust: Api::load(&rust_library_path()),
            }
        }
    }
}

#[derive(Clone)]
struct FixedRng(u64);

impl FixedRng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value >> 12;
        value ^= value << 25;
        value ^= value >> 27;
        self.0 = value;
        (value.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 32) as u32
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    fn between(&mut self, min: i32, max: i32) -> i32 {
        debug_assert!(min <= max);
        let width = (i64::from(max) - i64::from(min) + 1) as u64;
        (i64::from(min) + i64::try_from(u64::from(self.next_u32()) % width).unwrap()) as i32
    }
}

static STDOUT_LOCK: Mutex<()> = Mutex::new(());
static CAPTURE_COUNTER: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(old_fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
}

fn capture_stdout<T>(operation: impl FnOnce() -> T) -> (T, Vec<u8>) {
    let _lock: MutexGuard<'_, ()> = STDOUT_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let capture_id = CAPTURE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = env::temp_dir().join(format!(
        "checkshift-capture-{}-{capture_id}",
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
    }
    let saved_stdout = unsafe { dup(1) };
    assert!(saved_stdout >= 0);
    assert_eq!(unsafe { dup2(file.as_raw_fd(), 1) }, 1);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(operation));

    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
    }
    assert_eq!(unsafe { dup2(saved_stdout, 1) }, 1);
    assert_eq!(unsafe { close(saved_stdout) }, 0);

    file.seek(SeekFrom::Start(0)).unwrap();
    let mut output = Vec::new();
    file.read_to_end(&mut output).unwrap();
    drop(file);
    fs::remove_file(path).unwrap();

    match result {
        Ok(value) => (value, output),
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

fn compare_ffi<T: Debug + Eq>(
    label: &str,
    c_operation: impl FnOnce() -> T,
    rust_operation: impl FnOnce() -> T,
) -> T {
    let (c_result, c_stdout) = capture_stdout(c_operation);
    let (rust_result, rust_stdout) = capture_stdout(rust_operation);
    assert_eq!(c_stdout, rust_stdout, "{label}: stdout differs");
    assert_eq!(c_result, rust_result, "{label}: result differs");
    rust_result
}

fn state_bytes(state: &ComputeState) -> [u8; size_of::<ComputeState>()] {
    let mut bytes = [0_u8; size_of::<ComputeState>()];
    unsafe {
        ptr::copy_nonoverlapping(
            ptr::from_ref(state).cast::<u8>(),
            bytes.as_mut_ptr(),
            bytes.len(),
        );
    }
    bytes
}

fn binary_cases(kind: usize, count: usize, seed: u64) -> Vec<(i32, i32)> {
    let mut rng = FixedRng::new(seed);
    let mut cases = match kind {
        0 => vec![(-1000, -1000), (-1, 0), (0, 0), (1, 1), (1000, 1000)],
        1 => vec![
            (i32::MIN, 0),
            (-100, 0),
            (0, 0),
            (1, -1),
            (i32::MAX - 100, 0),
        ],
        2 => vec![
            (i32::MIN, i32::MAX),
            (-1, 0),
            (0, 0),
            (1, -1),
            (i32::MAX, i32::MIN),
        ],
        3 => vec![(0, i32::MIN), (1, -1), (2, 0), (i32::MAX >> 2, i32::MAX)],
        _ => unreachable!(),
    };
    while cases.len() < count {
        cases.push(match kind {
            0 => (rng.between(-10_000, 10_000), rng.between(-10_000, 10_000)),
            1 => (
                rng.between(-1_000_000, 1_000_000),
                rng.between(-1_000_000, 1_000_000),
            ),
            2 => (rng.next_i32(), rng.next_i32()),
            3 => (rng.between(0, i32::MAX >> 2), rng.next_i32()),
            _ => unreachable!(),
        });
    }
    cases
}

fn run_binary(operation: BinaryOp, cases: &[(i32, i32)]) -> Vec<i32> {
    cases
        .iter()
        .map(|&(a, b)| unsafe { operation(a, b) })
        .collect()
}

fn profile_directory() -> PathBuf {
    env::current_exe()
        .unwrap()
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf()
}

fn manifest_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_directory()
        .parent()
        .unwrap()
        .join("c_src/build/libharvest-work-kocQYC.so")
}

fn rust_library_path() -> PathBuf {
    env::var_os("CHECKSHIFT_RUST_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_directory().join("target/release/libcheckshift_lib.so"))
}

fn defined_dynamic_symbols(path: &Path) -> BTreeSet<String> {
    let output = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "nm failed for {}: {}",
        path.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .map(str::to_owned)
        .collect()
}

#[test]
fn symbol_parity() {
    let c_symbols = defined_dynamic_symbols(&c_library_path());
    let rust_symbols = defined_dynamic_symbols(&rust_library_path());
    assert_eq!(c_symbols, rust_symbols);
    assert_eq!(c_symbols.len(), 10);
}

#[test]
fn differential_valid_and_error_surface() {
    let libraries = Libraries::load();

    // CONFIGS rows 5-8: cold opcode 0 lookup, then warm opcodes 1-3.
    for opcode in 0..4 {
        let c_operation = unsafe { (libraries.c.get_operation)(opcode) };
        let rust_operation = unsafe { (libraries.rust.get_operation)(opcode) };
        assert!(c_operation.is_some());
        assert!(rust_operation.is_some());
        let cases = binary_cases(opcode as usize, 128, 0x1000 + opcode as u64);
        compare_ffi(
            &format!("get_operation opcode {opcode}"),
            || run_binary(c_operation.unwrap(), &cases),
            || run_binary(rust_operation.unwrap(), &cases),
        );
    }

    // CONFIGS rows 1-4: every direct low-level arithmetic export.
    for (row, c_operation, rust_operation) in [
        (0, libraries.c.multiply, libraries.rust.multiply),
        (1, libraries.c.add, libraries.rust.add),
        (2, libraries.c.xor, libraries.rust.xor),
        (3, libraries.c.shift, libraries.rust.shift),
    ] {
        let cases = binary_cases(row, 256, 0x2000 + row as u64);
        compare_ffi(
            &format!("direct binary operation {row}"),
            || run_binary(c_operation, &cases),
            || run_binary(rust_operation, &cases),
        );
    }

    // CONFIGS rows 9-12: callback execution through execute_operation.
    let names: [&[u8]; 4] = [b"MULTIPLY\0", b"ADD\0", b"XOR\0", b"SHIFT\0"];
    for (row, c_callback, rust_callback) in [
        (0, libraries.c.multiply, libraries.rust.multiply),
        (1, libraries.c.add, libraries.rust.add),
        (2, libraries.c.xor, libraries.rust.xor),
        (3, libraries.c.shift, libraries.rust.shift),
    ] {
        let cases = binary_cases(row, 64, 0x3000 + row as u64);
        compare_ffi(
            &format!("execute_operation callback {row}"),
            || {
                cases
                    .iter()
                    .map(|&(a, b)| unsafe {
                        (libraries.c.execute_operation)(
                            Some(c_callback),
                            a,
                            b,
                            names[row].as_ptr().cast(),
                        )
                    })
                    .collect::<Vec<_>>()
            },
            || {
                cases
                    .iter()
                    .map(|&(a, b)| unsafe {
                        (libraries.rust.execute_operation)(
                            Some(rust_callback),
                            a,
                            b,
                            names[row].as_ptr().cast(),
                        )
                    })
                    .collect::<Vec<_>>()
            },
        );
    }

    // CONFIGS rows 13-17: each checksum length shape, including truncation.
    let mut checksum_rng = FixedRng::new(0x4000);
    let checksum_inputs: Vec<[i32; 8]> = (0..128)
        .map(|_| std::array::from_fn(|_| checksum_rng.next_i32()))
        .collect();
    for count in [1, 2, 3, 4, 5, 8] {
        compare_ffi(
            &format!("compute_checksum count {count}"),
            || {
                checksum_inputs
                    .iter()
                    .map(|values| unsafe {
                        (libraries.c.compute_checksum)(values.as_ptr() as *mut i32, count)
                    })
                    .collect::<Vec<_>>()
            },
            || {
                checksum_inputs
                    .iter()
                    .map(|values| unsafe {
                        (libraries.rust.compute_checksum)(values.as_ptr() as *mut i32, count)
                    })
                    .collect::<Vec<_>>()
            },
        );
    }

    // CONFIGS row 18: full-width initial state values and byte-exact state.
    let mut init_rng = FixedRng::new(0x5000);
    let initial_values: Vec<i32> = (0..128).map(|_| init_rng.next_i32()).collect();
    compare_ffi(
        "init_state",
        || {
            initial_values
                .iter()
                .map(|&initial| {
                    let mut state = ComputeState {
                        accumulator: 0x5555_5555,
                        operation_count: -1,
                        checksum: 0xAAAA_AAAA,
                    };
                    unsafe { (libraries.c.init_state)(&mut state, initial) };
                    state_bytes(&state)
                })
                .collect::<Vec<_>>()
        },
        || {
            initial_values
                .iter()
                .map(|&initial| {
                    let mut state = ComputeState {
                        accumulator: 0x5555_5555,
                        operation_count: -1,
                        checksum: 0xAAAA_AAAA,
                    };
                    unsafe { (libraries.rust.init_state)(&mut state, initial) };
                    state_bytes(&state)
                })
                .collect::<Vec<_>>()
        },
    );

    // CONFIGS rows 19-22: initialized state plus each callback.
    for opcode in 0..4 {
        let cases = binary_cases(opcode, 64, 0x6000 + opcode as u64);
        let c_callback = unsafe { (libraries.c.get_operation)(opcode as i32) }.unwrap();
        let rust_callback = unsafe { (libraries.rust.get_operation)(opcode as i32) }.unwrap();
        compare_ffi(
            &format!("apply_operation opcode {opcode}"),
            || {
                cases
                    .iter()
                    .enumerate()
                    .map(|(index, &(initial, value))| {
                        let mut state = ComputeState {
                            accumulator: 0,
                            operation_count: 0,
                            checksum: 0,
                        };
                        unsafe { (libraries.c.init_state)(&mut state, initial) };
                        state.operation_count = index as i32 - 32;
                        state.checksum = 0xA5A5_0000 | index as u32;
                        unsafe {
                            (libraries.c.apply_operation)(&mut state, value, Some(c_callback))
                        };
                        state_bytes(&state)
                    })
                    .collect::<Vec<_>>()
            },
            || {
                cases
                    .iter()
                    .enumerate()
                    .map(|(index, &(initial, value))| {
                        let mut state = ComputeState {
                            accumulator: 0,
                            operation_count: 0,
                            checksum: 0,
                        };
                        unsafe { (libraries.rust.init_state)(&mut state, initial) };
                        state.operation_count = index as i32 - 32;
                        state.checksum = 0xA5A5_0000 | index as u32;
                        unsafe {
                            (libraries.rust.apply_operation)(&mut state, value, Some(rust_callback))
                        };
                        state_bytes(&state)
                    })
                    .collect::<Vec<_>>()
            },
        );
    }

    // CONFIGS row 23: full composed operation using safe randomized inputs.
    let mut pipeline_rng = FixedRng::new(0x7000);
    let pipeline_cases: Vec<[i32; 4]> = (0..64)
        .map(|_| {
            [
                pipeline_rng.between(0, 1000),
                pipeline_rng.between(0, 1000),
                pipeline_rng.between(-1000, 1000),
                pipeline_rng.between(0, 1000),
            ]
        })
        .collect();
    compare_ffi(
        "checkshift valid pipeline",
        || {
            pipeline_cases
                .iter()
                .map(|values| unsafe {
                    (libraries.c.checkshift)(values[0], values[1], values[2], values[3])
                })
                .collect::<Vec<_>>()
        },
        || {
            pipeline_cases
                .iter()
                .map(|values| unsafe {
                    (libraries.rust.checkshift)(values[0], values[1], values[2], values[3])
                })
                .collect::<Vec<_>>()
        },
    );

    // ERRORS rows 1, 2, 13, and 14: invalid selector values.
    for opcode in [i32::MIN, -1, 4, i32::MAX] {
        assert!(unsafe { (libraries.c.get_operation)(opcode) }.is_none());
        assert!(unsafe { (libraries.rust.get_operation)(opcode) }.is_none());
    }

    // ERRORS row 3: null callback has the exact sentinel and diagnostic.
    compare_ffi(
        "execute_operation null callback",
        || unsafe { (libraries.c.execute_operation)(None, 11, 22, c"NULL CALLBACK".as_ptr()) },
        || unsafe { (libraries.rust.execute_operation)(None, 11, 22, c"NULL CALLBACK".as_ptr()) },
    );

    // ERRORS rows 4, 5, and 11: null values and non-positive counts.
    for count in [1, i32::MAX] {
        compare_ffi(
            &format!("compute_checksum null count {count}"),
            || unsafe { (libraries.c.compute_checksum)(ptr::null_mut(), count) },
            || unsafe { (libraries.rust.compute_checksum)(ptr::null_mut(), count) },
        );
    }
    let boundary_values = [1_i32, 2, 3, 4];
    for count in [0, -1, i32::MIN] {
        compare_ffi(
            &format!("compute_checksum non-positive count {count}"),
            || unsafe {
                (libraries.c.compute_checksum)(boundary_values.as_ptr() as *mut i32, count)
            },
            || unsafe {
                (libraries.rust.compute_checksum)(boundary_values.as_ptr() as *mut i32, count)
            },
        );
    }

    // ERRORS row 6: null state initialization.
    compare_ffi(
        "init_state null",
        || unsafe { (libraries.c.init_state)(ptr::null_mut(), 123) },
        || unsafe { (libraries.rust.init_state)(ptr::null_mut(), 123) },
    );

    // ERRORS row 7: null state application.
    compare_ffi(
        "apply_operation null state",
        || unsafe { (libraries.c.apply_operation)(ptr::null_mut(), 123, Some(libraries.c.add)) },
        || unsafe {
            (libraries.rust.apply_operation)(ptr::null_mut(), 123, Some(libraries.rust.add))
        },
    );

    // ERRORS row 8: null callback preserves every state byte.
    let initial_state = ComputeState {
        accumulator: -123_456,
        operation_count: 789,
        checksum: 0xDEAD_BEEF,
    };
    let unchanged = compare_ffi(
        "apply_operation null callback",
        || {
            let mut state = initial_state;
            unsafe { (libraries.c.apply_operation)(&mut state, 123, None) };
            state_bytes(&state)
        },
        || {
            let mut state = initial_state;
            unsafe { (libraries.rust.apply_operation)(&mut state, 123, None) };
            state_bytes(&state)
        },
    );
    assert_eq!(unchanged, state_bytes(&initial_state));

    // ERRORS row 10: glibc's null %s handling with a valid callback.
    compare_ffi(
        "execute_operation null name",
        || unsafe { (libraries.c.execute_operation)(Some(libraries.c.add), 10, 20, ptr::null()) },
        || unsafe {
            (libraries.rust.execute_operation)(Some(libraries.rust.add), 10, 20, ptr::null())
        },
    );

    // ERRORS row 12: oversized count is clamped to the four-element buffer.
    compare_ffi(
        "compute_checksum oversized count",
        || unsafe {
            (libraries.c.compute_checksum)(boundary_values.as_ptr() as *mut i32, i32::MAX)
        },
        || unsafe {
            (libraries.rust.compute_checksum)(boundary_values.as_ptr() as *mut i32, i32::MAX)
        },
    );
}

#[test]
fn malloc_failure_child() {
    if env::var_os("CHECKSHIFT_MALLOC_CHILD").is_none() {
        return;
    }
    let libraries = Libraries::load();
    let interposer_path = PathBuf::from(env::var_os("CHECKSHIFT_MALLOC_INTERPOSER").unwrap());
    let interposer = unsafe { Library::new(&interposer_path) }.unwrap();
    let arm = unsafe {
        *interposer
            .get::<unsafe extern "C" fn()>(b"arm_state_malloc_failure\0")
            .unwrap()
    };
    let result = compare_ffi(
        "checkshift malloc failure",
        || unsafe {
            arm();
            (libraries.c.checkshift)(1, 2, 3, 4)
        },
        || unsafe {
            arm();
            (libraries.rust.checkshift)(1, 2, 3, 4)
        },
    );
    assert_eq!(result, -1);
}

#[test]
fn malloc_failure_differential() {
    if env::var_os("CHECKSHIFT_MALLOC_CHILD").is_some() {
        return;
    }

    let interposer = profile_directory().join("libfail_state_malloc.so");
    let source = manifest_directory().join("tests/support/fail_state_malloc.c");
    let compile = Command::new("cc")
        .args(["-shared", "-fPIC"])
        .arg(&source)
        .args(["-o"])
        .arg(&interposer)
        .arg("-ldl")
        .output()
        .unwrap();
    assert!(
        compile.status.success(),
        "allocator interposer compilation failed: {}",
        String::from_utf8_lossy(&compile.stderr)
    );

    let child = Command::new(env::current_exe().unwrap())
        .args([
            "--exact",
            "malloc_failure_child",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("CHECKSHIFT_MALLOC_CHILD", "1")
        .env("CHECKSHIFT_MALLOC_INTERPOSER", &interposer)
        .env("LD_PRELOAD", &interposer)
        .output()
        .unwrap();
    assert!(
        child.status.success(),
        "malloc failure child failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&child.stdout),
        String::from_utf8_lossy(&child.stderr)
    );
}
