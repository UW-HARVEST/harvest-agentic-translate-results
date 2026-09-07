use libloading::Library;
use std::ffi::{c_double, c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::mem::{MaybeUninit, size_of};
use std::os::fd::FromRawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::ptr;
use std::sync::Mutex;

type SafeDoubleToInt = unsafe extern "C" fn(c_double) -> c_int;
type ProcessWithFallthrough = unsafe extern "C" fn(c_int, c_int) -> c_int;
type CopyDataBlock = unsafe extern "C" fn(*mut DataBlock, *const DataBlock);
type HandlePointerOperations = unsafe extern "C" fn(c_int) -> c_int;
type Overunder = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipefd: *mut c_int) -> c_int;
}

#[repr(C)]
#[derive(Clone, Copy)]
struct DataBlock {
    id: c_int,
    value: c_double,
    label: [i8; 20],
}

struct Api {
    _library: Library,
    safe_double_to_int: SafeDoubleToInt,
    process_with_fallthrough: ProcessWithFallthrough,
    copy_data_block: CopyDataBlock,
    handle_pointer_operations: HandlePointerOperations,
    overunder: Overunder,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let safe_double_to_int =
            unsafe { *library.get::<SafeDoubleToInt>(b"safe_double_to_int\0").unwrap() };
        let process_with_fallthrough = unsafe {
            *library
                .get::<ProcessWithFallthrough>(b"process_with_fallthrough\0")
                .unwrap()
        };
        let copy_data_block =
            unsafe { *library.get::<CopyDataBlock>(b"copy_data_block\0").unwrap() };
        let handle_pointer_operations = unsafe {
            *library
                .get::<HandlePointerOperations>(b"handle_pointer_operations\0")
                .unwrap()
        };
        let overunder = unsafe { *library.get::<Overunder>(b"overunder\0").unwrap() };

        Self {
            _library: library,
            safe_double_to_int,
            process_with_fallthrough,
            copy_data_block,
            handle_pointer_operations,
            overunder,
        }
    }
}

#[derive(Clone, Copy)]
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

#[derive(Clone, Copy, Debug)]
enum AClass {
    Low,
    Normal,
    High,
}

#[derive(Clone, Copy, Debug)]
enum BClass {
    Low,
    Normal,
    High,
}

#[derive(Clone, Copy, Debug)]
enum Residue {
    Exact(i32),
    Default,
}

#[derive(Clone, Copy, Debug)]
struct OverCase {
    row: usize,
    a_class: AClass,
    residue: Residue,
    b_class: BClass,
    q_negative: bool,
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../c_src/build/libharvest-work-qzbIr9.so")
}

fn rust_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/liboverunder_lib.so")
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

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

fn capture_stdout(call: impl FnOnce() -> i32) -> (i32, Vec<u8>) {
    let _guard = STDOUT_LOCK.lock().unwrap();
    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);

        let mut fds = [-1, -1];
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
        let mut reader = File::from_raw_fd(fds[0]);
        reader.read_to_end(&mut output).unwrap();
        (result, output)
    }
}

fn block_bytes(block: &MaybeUninit<DataBlock>) -> &[u8] {
    unsafe { std::slice::from_raw_parts(block.as_ptr().cast::<u8>(), size_of::<DataBlock>()) }
}

fn random_block(rng: &mut Rng) -> MaybeUninit<DataBlock> {
    let mut block = MaybeUninit::<DataBlock>::uninit();
    let bytes = unsafe {
        std::slice::from_raw_parts_mut(
            block.as_mut_ptr().cast::<u8>(),
            size_of::<DataBlock>(),
        )
    };
    for byte in bytes {
        *byte = rng.next_u64() as u8;
    }
    block
}

fn a_matches(value: i32, class: AClass, residue: Residue) -> bool {
    let class_matches = match class {
        AClass::Low => value <= -1_431_655_766,
        AClass::Normal => (-1_431_655_765..=1_431_655_764).contains(&value),
        AClass::High => value >= 1_431_655_765,
    };
    let remainder = value % 6;
    let residue_matches = match residue {
        Residue::Exact(expected) => remainder == expected,
        Residue::Default => remainder < 0,
    };
    class_matches && residue_matches
}

fn b_matches(value: i32, class: BClass) -> bool {
    match class {
        BClass::Low => value <= -795_364_315,
        BClass::Normal => (-795_364_314..=795_364_314).contains(&value),
        BClass::High => value >= 795_364_315,
    }
}

fn random_a(rng: &mut Rng, class: AClass, residue: Residue) -> i32 {
    for _ in 0..1_000_000 {
        let value = rng.next_i32();
        if a_matches(value, class, residue) {
            return value;
        }
    }
    panic!("could not generate a for {class:?}/{residue:?}");
}

fn random_b(rng: &mut Rng, class: BClass) -> i32 {
    for _ in 0..1_000_000 {
        let value = rng.next_i32();
        if b_matches(value, class) {
            return value;
        }
    }
    panic!("could not generate b for {class:?}");
}

fn random_d(rng: &mut Rng, a: i32, q_negative: bool) -> Option<i32> {
    for _ in 0..10_000 {
        let d = rng.next_i32();
        let q = d
            .wrapping_mul(d)
            .wrapping_add(a.wrapping_mul(a));
        if (q < 0) == q_negative {
            return Some(d);
        }
    }
    None
}

fn over_input(rng: &mut Rng, case: OverCase) -> (i32, i32, i32, i32) {
    for _ in 0..10_000 {
        let a = random_a(rng, case.a_class, case.residue);
        if let Some(d) = random_d(rng, a, case.q_negative) {
            return (a, random_b(rng, case.b_class), rng.next_i32(), d);
        }
    }
    panic!("could not generate input for CONFIGS.md row {}", case.row);
}

fn over_cases() -> Vec<OverCase> {
    let mut cases = Vec::new();
    let b_classes = [BClass::Low, BClass::Normal, BClass::High];
    let signs = [false, true];
    let mut row = 11;

    for residue in [
        Residue::Exact(0),
        Residue::Exact(1),
        Residue::Exact(2),
        Residue::Exact(3),
        Residue::Exact(4),
        Residue::Exact(5),
        Residue::Default,
    ] {
        for b_class in b_classes {
            for q_negative in signs {
                cases.push(OverCase {
                    row,
                    a_class: AClass::Normal,
                    residue,
                    b_class,
                    q_negative,
                });
                row += 1;
            }
        }
    }

    for residue in [Residue::Exact(0), Residue::Default] {
        for b_class in b_classes {
            for q_negative in signs {
                cases.push(OverCase {
                    row,
                    a_class: AClass::Low,
                    residue,
                    b_class,
                    q_negative,
                });
                row += 1;
            }
        }
    }

    for residue in [
        Residue::Exact(0),
        Residue::Exact(1),
        Residue::Exact(2),
        Residue::Exact(3),
        Residue::Exact(4),
        Residue::Exact(5),
    ] {
        for b_class in b_classes {
            for q_negative in signs {
                cases.push(OverCase {
                    row,
                    a_class: AClass::High,
                    residue,
                    b_class,
                    q_negative,
                });
                row += 1;
            }
        }
    }

    assert_eq!(cases.len(), 90);
    assert_eq!(row, 101);
    cases
}

#[test]
fn phase_a_and_d_symbols_are_loadable() {
    let (c, rust) = load_apis();
    let _ = (
        c.safe_double_to_int,
        c.process_with_fallthrough,
        c.copy_data_block,
        c.handle_pointer_operations,
        c.overunder,
        rust.safe_double_to_int,
        rust.process_with_fallthrough,
        rust.copy_data_block,
        rust.handle_pointer_operations,
        rust.overunder,
    );
}

#[test]
fn phase_b_all_config_rows_match() {
    let (c, rust) = load_apis();
    let mut rng = Rng::new(0x6a09_e667_f3bc_c909);

    // CONFIGS.md row 1.
    let fixed_safe_values = [
        i32::MIN as f64,
        -2_000_000_000.75,
        -1.75,
        -0.0,
        0.0,
        1.75,
        2_000_000_000.75,
        i32::MAX as f64,
    ];
    for value in fixed_safe_values {
        let c_result = unsafe { (c.safe_double_to_int)(value) };
        let rust_result = unsafe { (rust.safe_double_to_int)(value) };
        assert_eq!(rust_result, c_result, "CONFIGS.md row 1, value={value:?}");
    }
    for _ in 0..256 {
        let integer = rng.next_i32().clamp(i32::MIN + 1, i32::MAX - 1);
        let fraction = if integer < 0 { -0.75 } else { 0.75 };
        let value = integer as f64 + fraction;
        let c_result = unsafe { (c.safe_double_to_int)(value) };
        let rust_result = unsafe { (rust.safe_double_to_int)(value) };
        assert_eq!(rust_result, c_result, "CONFIGS.md row 1, value={value:?}");
    }

    // CONFIGS.md rows 2-8.
    for (row, code) in (2usize..=7).zip(0i32..=5) {
        for _ in 0..256 {
            let base = rng.next_i32();
            let c_result = unsafe { (c.process_with_fallthrough)(code, base) };
            let rust_result = unsafe { (rust.process_with_fallthrough)(code, base) };
            assert_eq!(
                rust_result, c_result,
                "CONFIGS.md row {row}, code={code}, base={base}"
            );
        }
    }
    for _ in 0..256 {
        let mut code = rng.next_i32();
        while (0..=5).contains(&code) {
            code = rng.next_i32();
        }
        let base = rng.next_i32();
        let c_result = unsafe { (c.process_with_fallthrough)(code, base) };
        let rust_result = unsafe { (rust.process_with_fallthrough)(code, base) };
        assert_eq!(
            rust_result, c_result,
            "CONFIGS.md row 8, code={code}, base={base}"
        );
    }

    // CONFIGS.md row 9.
    assert_eq!(size_of::<DataBlock>(), 40);
    for iteration in 0..256 {
        let source = random_block(&mut rng);
        let mut c_dest = MaybeUninit::<DataBlock>::zeroed();
        let mut rust_dest = MaybeUninit::<DataBlock>::zeroed();
        unsafe {
            (c.copy_data_block)(c_dest.as_mut_ptr(), source.as_ptr());
            (rust.copy_data_block)(rust_dest.as_mut_ptr(), source.as_ptr());
        }
        assert_eq!(
            block_bytes(&rust_dest),
            block_bytes(&c_dest),
            "CONFIGS.md row 9, iteration={iteration}"
        );
        assert_eq!(
            block_bytes(&rust_dest),
            block_bytes(&source),
            "CONFIGS.md row 9 did not copy every byte, iteration={iteration}"
        );
    }

    // CONFIGS.md row 10.
    for _ in 0..256 {
        let value = rng.next_i32();
        let c_result = unsafe { (c.handle_pointer_operations)(value) };
        let rust_result = unsafe { (rust.handle_pointer_operations)(value) };
        assert_eq!(
            rust_result, c_result,
            "CONFIGS.md row 10, value={value}"
        );
    }

    // CONFIGS.md rows 11-100. Each row receives 32 fixed-seed random inputs.
    for case in over_cases() {
        for iteration in 0..32 {
            let (a, b, c_value, d) = over_input(&mut rng, case);
            let (c_result, c_stdout) =
                capture_stdout(|| unsafe { (c.overunder)(a, b, c_value, d) });
            let (rust_result, rust_stdout) =
                capture_stdout(|| unsafe { (rust.overunder)(a, b, c_value, d) });
            assert_eq!(
                rust_result, c_result,
                "CONFIGS.md row {}, iteration={iteration}, input=({a},{b},{c_value},{d})",
                case.row
            );
            assert_eq!(
                rust_stdout, c_stdout,
                "CONFIGS.md row {} stdout, iteration={iteration}, input=({a},{b},{c_value},{d})",
                case.row
            );
        }
    }
}

fn status_signature(status: ExitStatus) -> (Option<i32>, Option<i32>) {
    (status.code(), status.signal())
}

fn run_null_child(library: &str, case: &str) -> ExitStatus {
    Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("copy_data_block_null_boundary_child")
        .arg("--nocapture")
        .env("OVERUNDER_NULL_LIBRARY", library)
        .env("OVERUNDER_NULL_CASE", case)
        .status()
        .unwrap()
}

#[test]
fn phase_c_all_error_rows_match() {
    let (c, rust) = load_apis();
    let mut rng = Rng::new(0xbb67_ae85_84ca_a73b);

    // ERRORS.md row 1.
    let mut high_values = vec![
        i32::MAX as f64 + 1.0,
        1e15,
        f64::MAX,
        f64::INFINITY,
    ];
    for _ in 0..128 {
        high_values.push(i32::MAX as f64 + 1.0 + (rng.next_u64() as f64));
    }
    for value in high_values {
        let c_result = unsafe { (c.safe_double_to_int)(value) };
        let rust_result = unsafe { (rust.safe_double_to_int)(value) };
        assert_eq!(c_result, i32::MAX, "ERRORS.md row 1, value={value:?}");
        assert_eq!(rust_result, c_result, "ERRORS.md row 1, value={value:?}");
    }

    // ERRORS.md row 2.
    let mut low_values = vec![
        i32::MIN as f64 - 1.0,
        -1e15,
        -f64::MAX,
        f64::NEG_INFINITY,
    ];
    for _ in 0..128 {
        low_values.push(i32::MIN as f64 - 1.0 - (rng.next_u64() as f64));
    }
    for value in low_values {
        let c_result = unsafe { (c.safe_double_to_int)(value) };
        let rust_result = unsafe { (rust.safe_double_to_int)(value) };
        assert_eq!(c_result, i32::MIN, "ERRORS.md row 2, value={value:?}");
        assert_eq!(rust_result, c_result, "ERRORS.md row 2, value={value:?}");
    }

    // ERRORS.md row 3.
    let mut nan_values = vec![f64::NAN, -f64::NAN];
    for _ in 0..128 {
        let payload = rng.next_u64() & 0x000f_ffff_ffff_ffff;
        nan_values.push(f64::from_bits(0x7ff8_0000_0000_0000 | payload));
    }
    for value in nan_values {
        let c_result = unsafe { (c.safe_double_to_int)(value) };
        let rust_result = unsafe { (rust.safe_double_to_int)(value) };
        assert_eq!(c_result, 0, "ERRORS.md row 3");
        assert_eq!(rust_result, c_result, "ERRORS.md row 3");
    }

    // ERRORS.md row 4, including both one-step-past values.
    for code in [-1, 6, i32::MIN, i32::MAX] {
        for _ in 0..64 {
            let base = rng.next_i32();
            let c_result = unsafe { (c.process_with_fallthrough)(code, base) };
            let rust_result = unsafe { (rust.process_with_fallthrough)(code, base) };
            assert_eq!(c_result, -1, "ERRORS.md row 4, code={code}");
            assert_eq!(
                rust_result, c_result,
                "ERRORS.md row 4, code={code}, base={base}"
            );
        }
    }

    // ERRORS.md rows 5-7: compare isolated process outcomes.
    for (row, case) in [(5, "dest"), (6, "src"), (7, "both")] {
        let c_status = run_null_child("c", case);
        let rust_status = run_null_child("rust", case);
        assert!(!c_status.success(), "ERRORS.md row {row}: C unexpectedly returned");
        assert!(
            !rust_status.success(),
            "ERRORS.md row {row}: Rust unexpectedly returned"
        );
        assert_eq!(
            status_signature(rust_status),
            status_signature(c_status),
            "ERRORS.md row {row}: process outcome differs"
        );
    }
}

#[test]
fn copy_data_block_null_boundary_child() {
    let Ok(library_name) = std::env::var("OVERUNDER_NULL_LIBRARY") else {
        return;
    };
    let case = std::env::var("OVERUNDER_NULL_CASE").unwrap();
    let path = match library_name.as_str() {
        "c" => c_library_path(),
        "rust" => rust_library_path(),
        other => panic!("unknown child library {other}"),
    };
    let api = unsafe { Api::load(&path) };
    let mut dest = MaybeUninit::<DataBlock>::zeroed();
    let source = MaybeUninit::<DataBlock>::zeroed();
    let (dest_ptr, source_ptr) = match case.as_str() {
        "dest" => (ptr::null_mut(), source.as_ptr()),
        "src" => (dest.as_mut_ptr(), ptr::null()),
        "both" => (ptr::null_mut(), ptr::null()),
        other => panic!("unknown child case {other}"),
    };
    unsafe {
        (api.copy_data_block)(dest_ptr, source_ptr);
    }
    panic!("null-pointer copy unexpectedly returned");
}
