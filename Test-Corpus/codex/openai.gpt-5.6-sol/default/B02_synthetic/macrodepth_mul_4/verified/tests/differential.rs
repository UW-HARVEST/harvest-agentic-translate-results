use libloading::Library;
use std::ffi::{CStr, c_char, c_int, c_void};
use std::fs::{OpenOptions, remove_file};
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

type BinaryFn = unsafe extern "C" fn(c_int, c_int) -> c_int;
type GeneratedFn = unsafe extern "C" fn(c_int) -> c_int;

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

struct Api {
    op_add: BinaryFn,
    op_sub: BinaryFn,
    op_mul: BinaryFn,
    helper_call: BinaryFn,
    helper_ptr: BinaryFn,
    use_generated: GeneratedFn,
    global_op: BinaryFn,
    global_name: String,
}

impl Api {
    unsafe fn load(library: &Library) -> Self {
        let global_op = unsafe {
            **library
                .get::<*const BinaryFn>(b"G_OP\0")
                .expect("missing G_OP")
        };
        let global_name_ptr = unsafe {
            **library
                .get::<*const *const c_char>(b"G_OP_NAME\0")
                .expect("missing G_OP_NAME")
        };
        let global_name = unsafe { CStr::from_ptr(global_name_ptr) }
            .to_str()
            .expect("G_OP_NAME is not UTF-8")
            .to_owned();

        Self {
            op_add: unsafe { *library.get(b"op_add\0").expect("missing op_add") },
            op_sub: unsafe { *library.get(b"op_sub\0").expect("missing op_sub") },
            op_mul: unsafe { *library.get(b"op_mul\0").expect("missing op_mul") },
            helper_call: unsafe { *library.get(b"helper_call\0").expect("missing helper_call") },
            helper_ptr: unsafe { *library.get(b"helper_ptr\0").expect("missing helper_ptr") },
            use_generated: unsafe {
                *library
                    .get(b"use_generated\0")
                    .expect("missing use_generated")
            },
            global_op,
            global_name,
        }
    }
}

fn selected_operation() -> &'static str {
    if cfg!(feature = "mul") {
        "mul"
    } else if cfg!(feature = "sub") {
        "sub"
    } else {
        "add"
    }
}

fn selected_repeat() -> c_int {
    if cfg!(feature = "7") {
        7
    } else if cfg!(feature = "6") {
        6
    } else if cfg!(feature = "5") {
        5
    } else if cfg!(feature = "4") {
        4
    } else if cfg!(feature = "3") {
        3
    } else if cfg!(feature = "2") {
        2
    } else if cfg!(feature = "1") {
        1
    } else if cfg!(feature = "0") {
        0
    } else {
        5
    }
}

fn rust_library_path() -> PathBuf {
    if let Some(path) = std::env::var_os("MD_RUST_SO") {
        return PathBuf::from(path);
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("debug")
        .join("libmd_driver.so")
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("c_src")
        .join("build")
        .join("ffi")
        .join(format!(
            "libmdcore_{}_{}.so",
            selected_operation(),
            selected_repeat()
        ))
}

static CAPTURE_ID: AtomicU64 = AtomicU64::new(0);

fn capture_stdout<T>(call: impl FnOnce() -> T) -> (T, Vec<u8>) {
    let id = CAPTURE_ID.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("md-driver-ffi-{}-{id}.out", std::process::id()));
    let mut file = OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(&path)
        .expect("create stdout capture");

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0);
    }
    let saved_stdout = unsafe { dup(1) };
    assert!(saved_stdout >= 0, "dup(stdout) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "redirect failed");

    let result = call();

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0);
    }
    assert!(unsafe { dup2(saved_stdout, 1) } >= 0, "restore failed");
    assert_eq!(unsafe { close(saved_stdout) }, 0);

    file.seek(SeekFrom::Start(0)).expect("rewind capture");
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read capture");
    drop(file);
    remove_file(path).expect("remove capture");
    (result, bytes)
}

fn randomized_pairs() -> Vec<(c_int, c_int)> {
    let mut pairs = vec![
        (0, 0),
        (1, -1),
        (-1, 1),
        (c_int::MAX, 0),
        (c_int::MIN, 0),
        (c_int::MAX, 1),
        (c_int::MIN, -1),
        (c_int::MAX, c_int::MAX),
        (c_int::MIN, c_int::MIN),
        (46_340, 46_340),
        (-46_341, 46_341),
    ];
    let mut state = 0x4d59_5df4_d0f3_3173_u64;
    for _ in 0..256 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let a = state as u32 as c_int;
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let b = state as u32 as c_int;
        pairs.push((a, b));
    }
    pairs
}

fn compare_printing_binary(
    c_function: BinaryFn,
    rust_function: BinaryFn,
    a: c_int,
    b: c_int,
    label: &str,
) {
    let (c_result, c_stdout) = capture_stdout(|| unsafe { c_function(a, b) });
    let (rust_result, rust_stdout) = capture_stdout(|| unsafe { rust_function(a, b) });
    assert_eq!(
        rust_result, c_result,
        "{label} return mismatch for ({a}, {b})"
    );
    assert_eq!(
        rust_stdout, c_stdout,
        "{label} stdout mismatch for ({a}, {b})"
    );
}

fn compare_generated(c_function: GeneratedFn, rust_function: GeneratedFn, n: c_int) {
    let (c_result, c_stdout) = capture_stdout(|| unsafe { c_function(n) });
    let (rust_result, rust_stdout) = capture_stdout(|| unsafe { rust_function(n) });
    assert_eq!(
        rust_result, c_result,
        "use_generated return mismatch for {n}"
    );
    assert_eq!(
        rust_stdout, c_stdout,
        "use_generated stdout mismatch for {n}"
    );
}

#[test]
fn ffi_surface_matches_c() {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(c_path.is_file(), "missing C library: {}", c_path.display());
    assert!(
        rust_path.is_file(),
        "missing Rust library: {}",
        rust_path.display()
    );

    let c_library = unsafe { Library::new(&c_path) }.expect("load C library");
    let rust_library = unsafe { Library::new(&rust_path) }.expect("load Rust library");
    let c = unsafe { Api::load(&c_library) };
    let rust = unsafe { Api::load(&rust_library) };

    assert_eq!(rust.global_name, c.global_name);
    assert_eq!(rust.global_name, selected_operation());

    for (a, b) in randomized_pairs() {
        for (name, c_function, rust_function) in [
            ("op_add", c.op_add, rust.op_add),
            ("op_sub", c.op_sub, rust.op_sub),
            ("op_mul", c.op_mul, rust.op_mul),
            ("G_OP", c.global_op, rust.global_op),
        ] {
            let c_result = unsafe { c_function(a, b) };
            let rust_result = unsafe { rust_function(a, b) };
            assert_eq!(rust_result, c_result, "{name} mismatch for ({a}, {b})");
        }

        compare_printing_binary(c.helper_ptr, rust.helper_ptr, a, b, "helper_ptr");
        compare_printing_binary(c.helper_call, rust.helper_call, a, b, "helper_call");
    }

    for n in c_int::MIN..=c_int::MIN + 31 {
        compare_generated(c.use_generated, rust.use_generated, n);
    }
    for n in -32..=38 {
        compare_generated(c.use_generated, rust.use_generated, n);
    }
    for n in c_int::MAX - 31..=c_int::MAX {
        compare_generated(c.use_generated, rust.use_generated, n);
    }

    let mut state = 0xa076_1d64_78bd_642f_u64;
    for _ in 0..256 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let n = state as u32 as c_int;
        compare_generated(c.use_generated, rust.use_generated, n);
    }
}
