use libloading::Library;
use std::env;
use std::ffi::{c_char, c_int, c_uint, c_void};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;

type BinOp = unsafe extern "C" fn(c_int, c_int) -> c_int;
type NullableBinOp = Option<BinOp>;
type GetOperation = unsafe extern "C" fn(c_int) -> NullableBinOp;
type ExecuteOperation = unsafe extern "C" fn(NullableBinOp, c_int, c_int, *const c_char) -> c_int;
type ComputeChecksum = unsafe extern "C" fn(*mut c_int, c_int) -> c_uint;
type InitState = unsafe extern "C" fn(*mut ComputeState, c_int);
type ApplyOperation = unsafe extern "C" fn(*mut ComputeState, c_int, NullableBinOp);
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
        let multiply = unsafe { *library.get(b"multiply_with_static\0").unwrap() };
        let add = unsafe { *library.get(b"add_with_static\0").unwrap() };
        let xor = unsafe { *library.get(b"xor_operation\0").unwrap() };
        let shift = unsafe { *library.get(b"shift_with_static\0").unwrap() };
        let get_operation = unsafe { *library.get(b"get_operation\0").unwrap() };
        let execute_operation = unsafe { *library.get(b"execute_operation\0").unwrap() };
        let compute_checksum = unsafe { *library.get(b"compute_checksum\0").unwrap() };
        let init_state = unsafe { *library.get(b"init_state\0").unwrap() };
        let apply_operation = unsafe { *library.get(b"apply_operation\0").unwrap() };
        let checkshift = unsafe { *library.get(b"checkshift\0").unwrap() };
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

    fn operation(&self, opcode: usize) -> BinOp {
        [self.multiply, self.add, self.xor, self.shift][opcode]
    }
}

unsafe extern "C" {
    fn pipe(fds: *mut c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
    fn fflush(stream: *mut c_void) -> c_int;
}

fn library_paths() -> (PathBuf, PathBuf) {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_path = crate_dir
        .parent()
        .unwrap()
        .join("c_src/build/libharvest-work-6PFtVb.so");
    let rust_path = crate_dir.join("target/release/libcheckshift_lib.so");
    assert!(c_path.is_file(), "missing C library: {}", c_path.display());
    assert!(
        rust_path.is_file(),
        "missing Rust library: {}",
        rust_path.display()
    );
    (c_path, rust_path)
}

fn capture_stdout<T>(call: impl FnOnce() -> T) -> (T, Vec<u8>) {
    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
        let mut fds = [-1; 2];
        assert_eq!(pipe(fds.as_mut_ptr()), 0);
        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0);
        assert_eq!(dup2(fds[1], 1), 1);
        assert_eq!(close(fds[1]), 0);

        let result = call();

        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, 1), 1);
        assert_eq!(close(saved_stdout), 0);

        let mut output = Vec::new();
        let mut buffer = [0_u8; 4096];
        loop {
            let count = read(fds[0], buffer.as_mut_ptr().cast(), buffer.len());
            assert!(count >= 0);
            if count == 0 {
                break;
            }
            output.extend_from_slice(&buffer[..count as usize]);
        }
        assert_eq!(close(fds[0]), 0);
        (result, output)
    }
}

#[derive(Clone)]
struct FixedRng(u64);

impl FixedRng {
    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x as u32
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
}

fn operand_pairs() -> Vec<(i32, i32)> {
    let boundaries = [
        i32::MIN,
        i32::MIN + 1,
        -65536,
        -3,
        -1,
        0,
        1,
        2,
        3,
        65535,
        65536,
        i32::MAX - 1,
        i32::MAX,
    ];
    let mut pairs = Vec::new();
    for &a in &boundaries {
        for &b in &boundaries {
            pairs.push((a, b));
        }
    }
    let mut rng = FixedRng(0x7A11_C0DE_D1FF_2025);
    for _ in 0..512 {
        pairs.push((rng.next_i32(), rng.next_i32()));
    }
    pairs
}

fn assert_binop_rows(c: &Api, rust: &Api, pairs: &[(i32, i32)]) {
    for opcode in 0..4 {
        let c_op = c.operation(opcode);
        let rust_op = rust.operation(opcode);
        for &(a, b) in pairs {
            let c_result = unsafe { c_op(a, b) };
            let rust_result = unsafe { rust_op(a, b) };
            assert_eq!(
                rust_result, c_result,
                "direct operation opcode={opcode}, a={a}, b={b}"
            );
        }
    }
}

fn assert_get_operation_rows(
    c_path: &Path,
    rust_path: &Path,
    c: &Api,
    rust: &Api,
    pairs: &[(i32, i32)],
) {
    let unique = format!(
        "checkshift-differential-{}-{}",
        std::process::id(),
        0x51A7_u32
    );
    let temp_dir = env::temp_dir().join(unique);
    fs::create_dir_all(&temp_dir).unwrap();

    for opcode in 0..4 {
        let fresh_c_path = temp_dir.join(format!("c-{opcode}.so"));
        let fresh_rust_path = temp_dir.join(format!("rust-{opcode}.so"));
        fs::copy(c_path, &fresh_c_path).unwrap();
        fs::copy(rust_path, &fresh_rust_path).unwrap();
        let fresh_c = unsafe { Api::load(&fresh_c_path) };
        let fresh_rust = unsafe { Api::load(&fresh_rust_path) };
        let c_op = unsafe { (fresh_c.get_operation)(opcode as i32) }.unwrap();
        let rust_op = unsafe { (fresh_rust.get_operation)(opcode as i32) }.unwrap();
        for &(a, b) in pairs.iter().take(128) {
            assert_eq!(
                unsafe { rust_op(a, b) },
                unsafe { c_op(a, b) },
                "fresh get_operation opcode={opcode}, a={a}, b={b}"
            );
        }
    }

    assert!(unsafe { (c.get_operation)(0) }.is_some());
    assert!(unsafe { (rust.get_operation)(0) }.is_some());
    for opcode in 0..4 {
        let c_op = unsafe { (c.get_operation)(opcode as i32) }.unwrap();
        let rust_op = unsafe { (rust.get_operation)(opcode as i32) }.unwrap();
        for &(a, b) in pairs.iter().skip(64).take(128) {
            assert_eq!(
                unsafe { rust_op(a, b) },
                unsafe { c_op(a, b) },
                "warmed get_operation opcode={opcode}, a={a}, b={b}"
            );
        }
    }

    fs::remove_dir_all(&temp_dir).unwrap();
}

fn assert_execute_rows(c: &Api, rust: &Api, pairs: &[(i32, i32)]) {
    let name = b"DIFF\0";
    for opcode in 0..4 {
        let c_op = unsafe { (c.get_operation)(opcode as i32) };
        let rust_op = unsafe { (rust.get_operation)(opcode as i32) };
        for &(a, b) in pairs.iter().take(128) {
            let (c_result, c_output) = capture_stdout(|| unsafe {
                (c.execute_operation)(c_op, a, b, name.as_ptr().cast())
            });
            let (rust_result, rust_output) = capture_stdout(|| unsafe {
                (rust.execute_operation)(rust_op, a, b, name.as_ptr().cast())
            });
            assert_eq!(rust_result, c_result);
            assert_eq!(
                rust_output, c_output,
                "execute opcode={opcode}, a={a}, b={b}"
            );
        }
    }

    let c_op = unsafe { (c.get_operation)(2) };
    let rust_op = unsafe { (rust.get_operation)(2) };
    let (c_result, c_output) =
        capture_stdout(|| unsafe { (c.execute_operation)(c_op, 11, -7, ptr::null()) });
    let (rust_result, rust_output) =
        capture_stdout(|| unsafe { (rust.execute_operation)(rust_op, 11, -7, ptr::null()) });
    assert_eq!(rust_result, c_result);
    assert_eq!(rust_output, c_output);
}

fn assert_checksum_rows(c: &Api, rust: &Api) {
    let mut rng = FixedRng(0xC5EC_5A55_1234_5678);
    for &count in &[1, 2, 3, 4, 5, 17, i32::MAX] {
        for _ in 0..256 {
            let mut c_values = [
                rng.next_i32(),
                rng.next_i32(),
                rng.next_i32(),
                rng.next_i32(),
            ];
            let mut rust_values = c_values;
            let c_result = unsafe { (c.compute_checksum)(c_values.as_mut_ptr(), count) };
            let rust_result = unsafe { (rust.compute_checksum)(rust_values.as_mut_ptr(), count) };
            assert_eq!(rust_result, c_result, "checksum count={count}");
        }
    }
}

fn assert_state_rows(c: &Api, rust: &Api) {
    let mut rng = FixedRng(0x57A7_E123_AAAA_0001);
    for _ in 0..256 {
        let initial = rng.next_i32();
        let mut c_state = ComputeState {
            accumulator: 17,
            operation_count: -9,
            checksum: 0xFFFF_FFFF,
        };
        let mut rust_state = c_state;
        let ((), c_output) = capture_stdout(|| unsafe { (c.init_state)(&mut c_state, initial) });
        let ((), rust_output) =
            capture_stdout(|| unsafe { (rust.init_state)(&mut rust_state, initial) });
        assert_eq!(rust_state, c_state);
        assert_eq!(rust_output, c_output);
    }

    for opcode in 0..4 {
        let c_op = unsafe { (c.get_operation)(opcode as i32) };
        let rust_op = unsafe { (rust.get_operation)(opcode as i32) };
        for _ in 0..256 {
            let state = ComputeState {
                accumulator: rng.next_i32(),
                operation_count: rng.next_i32(),
                checksum: rng.next_u32(),
            };
            let value = rng.next_i32();
            let mut c_state = state;
            let mut rust_state = state;
            unsafe {
                (c.apply_operation)(&mut c_state, value, c_op);
                (rust.apply_operation)(&mut rust_state, value, rust_op);
            }
            assert_eq!(
                rust_state, c_state,
                "apply opcode={opcode}, initial={state:?}, value={value}"
            );
        }
    }
}

fn assert_checkshift_row(c: &Api, rust: &Api) {
    let boundaries = [
        (0, 0, 0, 0),
        (1, 2, 3, 4),
        (-1, -2, -3, -4),
        (i32::MIN, i32::MAX, 0, -1),
        (i32::MAX, i32::MAX, i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN, i32::MIN, i32::MIN),
    ];
    let mut cases = boundaries.to_vec();
    let mut rng = FixedRng(0xF011_CAFE_0BAD_5EED);
    for _ in 0..256 {
        cases.push((
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
        ));
    }
    for (a, b, c_arg, d) in cases {
        let (c_result, c_output) = capture_stdout(|| unsafe { (c.checkshift)(a, b, c_arg, d) });
        let (rust_result, rust_output) =
            capture_stdout(|| unsafe { (rust.checkshift)(a, b, c_arg, d) });
        assert_eq!(
            rust_result, c_result,
            "checkshift return ({a}, {b}, {c_arg}, {d})"
        );
        assert_eq!(
            rust_output, c_output,
            "checkshift stdout ({a}, {b}, {c_arg}, {d})"
        );
    }
}

fn assert_error_rows(c: &Api, rust: &Api) {
    for &opcode in &[i32::MIN, -1, 4, i32::MAX] {
        assert!(unsafe { (c.get_operation)(opcode) }.is_none());
        assert!(unsafe { (rust.get_operation)(opcode) }.is_none());
    }

    let name = b"NULL_TEST\0";
    let (c_result, c_output) =
        capture_stdout(|| unsafe { (c.execute_operation)(None, 1, 2, name.as_ptr().cast()) });
    let (rust_result, rust_output) =
        capture_stdout(|| unsafe { (rust.execute_operation)(None, 1, 2, name.as_ptr().cast()) });
    assert_eq!(c_result, 0);
    assert_eq!(rust_result, c_result);
    assert_eq!(rust_output, c_output);

    for &count in &[0, 1, i32::MAX] {
        assert_eq!(
            unsafe { (rust.compute_checksum)(ptr::null_mut(), count) },
            unsafe { (c.compute_checksum)(ptr::null_mut(), count) }
        );
    }
    for &count in &[i32::MIN, -1, 0] {
        let invalid_nonnull = ptr::without_provenance_mut::<i32>(1);
        assert_eq!(
            unsafe { (rust.compute_checksum)(invalid_nonnull, count) },
            unsafe { (c.compute_checksum)(invalid_nonnull, count) }
        );
    }

    let ((), c_output) = capture_stdout(|| unsafe { (c.init_state)(ptr::null_mut(), 9) });
    let ((), rust_output) = capture_stdout(|| unsafe { (rust.init_state)(ptr::null_mut(), 9) });
    assert_eq!(rust_output, c_output);

    let c_op = unsafe { (c.get_operation)(0) };
    let rust_op = unsafe { (rust.get_operation)(0) };
    let ((), c_output) =
        capture_stdout(|| unsafe { (c.apply_operation)(ptr::null_mut(), 9, c_op) });
    let ((), rust_output) =
        capture_stdout(|| unsafe { (rust.apply_operation)(ptr::null_mut(), 9, rust_op) });
    assert_eq!(rust_output, c_output);

    let original = ComputeState {
        accumulator: -77,
        operation_count: 23,
        checksum: 0xA5A5_5A5A,
    };
    let mut c_state = original;
    let mut rust_state = original;
    let ((), c_output) = capture_stdout(|| unsafe { (c.apply_operation)(&mut c_state, 9, None) });
    let ((), rust_output) =
        capture_stdout(|| unsafe { (rust.apply_operation)(&mut rust_state, 9, None) });
    assert_eq!(c_state, original);
    assert_eq!(rust_state, c_state);
    assert_eq!(rust_output, c_output);
}

fn compile_malloc_interposer() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = crate_dir.join("tests/malloc_fail.c");
    let output = env::temp_dir().join(format!(
        "libcheckshift-malloc-fail-{}.so",
        std::process::id()
    ));
    let status = Command::new("timeout")
        .args(["600", "cc", "-shared", "-fPIC", "-std=c11", "-o"])
        .arg(&output)
        .arg(&source)
        .status()
        .expect("failed to run cc for malloc interposer");
    assert!(status.success());
    assert!(output.is_file());
    output
}

fn malloc_child_result(target_library: &Path, interposer: &Path) -> (String, String) {
    let existing_preload = env::var("LD_PRELOAD").unwrap_or_default();
    let preload = if existing_preload.is_empty() {
        interposer.display().to_string()
    } else {
        format!("{}:{existing_preload}", interposer.display())
    };
    let output = Command::new("timeout")
        .arg("600")
        .arg(env::current_exe().unwrap())
        .args(["malloc_failure_child", "--exact", "--nocapture"])
        .env("CHECKSHIFT_MALLOC_CHILD", "1")
        .env("CHECKSHIFT_TARGET_LIBRARY", target_library)
        .env("CHECKSHIFT_INTERPOSER", interposer)
        .env("LD_PRELOAD", preload)
        .output()
        .expect("failed to run malloc-failure child");
    assert!(
        output.status.success(),
        "malloc child failed:\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let capture = stdout
        .lines()
        .find_map(|line| line.strip_prefix("CAPTURE_HEX="))
        .expect("missing CAPTURE_HEX")
        .to_owned();
    let result = stdout
        .lines()
        .find_map(|line| line.strip_prefix("RESULT="))
        .expect("missing RESULT")
        .to_owned();
    (capture, result)
}

fn assert_malloc_failure_row(c_path: &Path, rust_path: &Path) {
    let interposer = compile_malloc_interposer();
    let c = malloc_child_result(c_path, &interposer);
    let rust = malloc_child_result(rust_path, &interposer);
    assert_eq!(c.1, "-1");
    assert_eq!(rust.1, c.1);
    assert_eq!(rust.0, c.0);
    fs::remove_file(interposer).unwrap();
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        write!(&mut output, "{byte:02x}").unwrap();
    }
    output
}

#[test]
fn differential_surface() {
    let (c_path, rust_path) = library_paths();
    let c = unsafe { Api::load(&c_path) };
    let rust = unsafe { Api::load(&rust_path) };
    let pairs = operand_pairs();

    assert_binop_rows(&c, &rust, &pairs);
    assert_get_operation_rows(&c_path, &rust_path, &c, &rust, &pairs);
    assert_execute_rows(&c, &rust, &pairs);
    assert_checksum_rows(&c, &rust);
    assert_state_rows(&c, &rust);
    assert_checkshift_row(&c, &rust);
    assert_error_rows(&c, &rust);
    assert_malloc_failure_row(&c_path, &rust_path);
}

#[test]
fn malloc_failure_child() {
    if env::var_os("CHECKSHIFT_MALLOC_CHILD").is_none() {
        return;
    }

    let target = PathBuf::from(env::var_os("CHECKSHIFT_TARGET_LIBRARY").unwrap());
    let interposer = PathBuf::from(env::var_os("CHECKSHIFT_INTERPOSER").unwrap());
    let interposer_library = unsafe { Library::new(interposer) }.unwrap();
    let arm: libloading::Symbol<unsafe extern "C" fn(usize, *const c_void)> =
        unsafe { interposer_library.get(b"arm_fail_size\0") }.unwrap();
    let target_library = unsafe { Library::new(target) }.unwrap();
    let checkshift: libloading::Symbol<Checkshift> =
        unsafe { target_library.get(b"checkshift\0") }.unwrap();
    let (result, output) = capture_stdout(|| unsafe {
        arm(
            std::mem::size_of::<ComputeState>(),
            (*checkshift as *const ()).cast(),
        );
        checkshift(1, 2, 3, 4)
    });
    println!("CAPTURE_HEX={}", hex(&output));
    println!("RESULT={result}");
    assert_eq!(result, -1);
}
