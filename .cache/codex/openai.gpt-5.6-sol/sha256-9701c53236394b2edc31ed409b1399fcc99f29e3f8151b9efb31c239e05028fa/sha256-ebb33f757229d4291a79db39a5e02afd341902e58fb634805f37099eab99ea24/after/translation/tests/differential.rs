use libloading::{Library, Symbol};
use std::ffi::{c_int, c_long, c_void};
use std::path::PathBuf;
use std::ptr;
use std::sync::{Mutex, OnceLock};

type FmaArray = unsafe extern "C" fn(*mut c_int, *const c_int, *const c_int, *const c_int, c_int);
type Driver = unsafe extern "C" fn(*const c_int, c_int);

unsafe extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn tmpfile() -> *mut c_void;
    fn fileno(stream: *mut c_void) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fseek(stream: *mut c_void, offset: c_long, whence: c_int) -> c_int;
    fn ftell(stream: *mut c_void) -> c_long;
    fn rewind(stream: *mut c_void);
    fn fread(ptr: *mut c_void, size: usize, count: usize, stream: *mut c_void) -> usize;
    fn fclose(stream: *mut c_void) -> c_int;
}

const STDOUT_FILENO: c_int = 1;
const SEEK_END: c_int = 2;

struct Api {
    library: Library,
}

impl Api {
    unsafe fn load(path: PathBuf) -> Self {
        Self {
            library: unsafe { Library::new(&path) }
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display())),
        }
    }

    unsafe fn fma_array(
        &self,
        out: *mut c_int,
        mul1: *const c_int,
        mul2: *const c_int,
        add: *const c_int,
        len: c_int,
    ) {
        let function: Symbol<'_, FmaArray> =
            unsafe { self.library.get(b"fma_array\0") }.expect("missing fma_array export");
        unsafe { function(out, mul1, mul2, add, len) };
    }

    unsafe fn driver(&self, data: *const c_int, len: c_int) {
        let function: Symbol<'_, Driver> =
            unsafe { self.library.get(b"driver\0") }.expect("missing driver export");
        unsafe { function(data, len) };
    }
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as i32
    }

    fn len(&mut self, min: usize, max_inclusive: usize) -> usize {
        min + self.next_u64() as usize % (max_inclusive - min + 1)
    }

    fn vec(&mut self, len: usize) -> Vec<i32> {
        (0..len).map(|_| self.next_i32()).collect()
    }
}

fn library_paths() -> (PathBuf, PathBuf) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c = manifest.join("../c_src/build/libdriver.so");
    let rust = manifest.join("target/release/libdriver.so");
    (c, rust)
}

fn apis() -> (Api, Api) {
    let (c_path, rust_path) = library_paths();
    assert!(c_path.is_file(), "missing C library: {}", c_path.display());
    assert!(
        rust_path.is_file(),
        "missing Rust library: {}",
        rust_path.display()
    );
    unsafe { (Api::load(c_path), Api::load(rust_path)) }
}

fn stdout_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

unsafe fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let _guard = stdout_lock();
    unsafe { fflush(ptr::null_mut()) };
    let stream = unsafe { tmpfile() };
    assert!(!stream.is_null(), "tmpfile failed");
    let temporary_fd = unsafe { fileno(stream) };
    assert!(temporary_fd >= 0, "fileno failed");
    let saved_stdout = unsafe { dup(STDOUT_FILENO) };
    assert!(saved_stdout >= 0, "dup failed");
    assert_eq!(
        unsafe { dup2(temporary_fd, STDOUT_FILENO) },
        STDOUT_FILENO,
        "dup2 redirect failed"
    );

    call();
    unsafe { fflush(ptr::null_mut()) };

    assert_eq!(
        unsafe { dup2(saved_stdout, STDOUT_FILENO) },
        STDOUT_FILENO,
        "dup2 restore failed"
    );
    assert_eq!(unsafe { close(saved_stdout) }, 0, "close failed");
    assert_eq!(unsafe { fseek(stream, 0, SEEK_END) }, 0, "fseek failed");
    let size = unsafe { ftell(stream) };
    assert!(size >= 0, "ftell failed");
    unsafe { rewind(stream) };
    let mut bytes = vec![0_u8; size as usize];
    let bytes_read = unsafe { fread(bytes.as_mut_ptr().cast(), 1, bytes.len(), stream) };
    assert_eq!(bytes_read, bytes.len(), "short read from captured stdout");
    assert_eq!(unsafe { fclose(stream) }, 0, "fclose failed");
    bytes
}

fn call_distinct(api: &Api, initial: &[i32], a: &[i32], b: &[i32], c: &[i32]) -> Vec<i32> {
    let mut out = initial.to_vec();
    unsafe {
        api.fma_array(
            out.as_mut_ptr(),
            a.as_ptr(),
            b.as_ptr(),
            c.as_ptr(),
            out.len() as c_int,
        )
    };
    out
}

fn assert_distinct_case(c_api: &Api, rust_api: &Api, rng: &mut Rng, len: usize) {
    let initial = rng.vec(len);
    let mul1 = rng.vec(len);
    let mul2 = rng.vec(len);
    let add = rng.vec(len);
    assert_eq!(
        call_distinct(c_api, &initial, &mul1, &mul2, &add),
        call_distinct(rust_api, &initial, &mul1, &mul2, &add)
    );
}

fn exported_symbols_are_loadable() {
    let (c_api, rust_api) = apis();
    for api in [&c_api, &rust_api] {
        unsafe {
            let _: Symbol<'_, FmaArray> = api.library.get(b"fma_array\0").unwrap();
            let _: Symbol<'_, Driver> = api.library.get(b"driver\0").unwrap();
        }
    }
}

fn fma_array_length_and_distinct_buffer_rows_match() {
    let (c_api, rust_api) = apis();
    unsafe {
        c_api.fma_array(ptr::null_mut(), ptr::null(), ptr::null(), ptr::null(), -1);
        rust_api.fma_array(ptr::null_mut(), ptr::null(), ptr::null(), ptr::null(), -1);
        c_api.fma_array(ptr::null_mut(), ptr::null(), ptr::null(), ptr::null(), 0);
        rust_api.fma_array(ptr::null_mut(), ptr::null(), ptr::null(), ptr::null(), 0);
    }

    let mut rng = Rng::new(0x9f4a_7c15_2d31_b803);
    for _ in 0..256 {
        assert_distinct_case(&c_api, &rust_api, &mut rng, 1);
    }
    for _ in 0..256 {
        let len = rng.len(2, 128);
        assert_distinct_case(&c_api, &rust_api, &mut rng, len);
    }
    assert_distinct_case(&c_api, &rust_api, &mut rng, 4096);
}

fn fma_array_alias_and_overlap_rows_match() {
    let (c_api, rust_api) = apis();
    let mut rng = Rng::new(0xd1b5_4a32_d192_ed03);

    for _ in 0..128 {
        let len = rng.len(2, 128);
        let original = rng.vec(len);
        let mul1 = rng.vec(len);
        let mul2 = rng.vec(len);
        let add = rng.vec(len);

        for alias in 0..4 {
            let run = |api: &Api| {
                let mut out = original.clone();
                unsafe {
                    match alias {
                        0 => api.fma_array(
                            out.as_mut_ptr(),
                            out.as_ptr(),
                            mul2.as_ptr(),
                            add.as_ptr(),
                            len as c_int,
                        ),
                        1 => api.fma_array(
                            out.as_mut_ptr(),
                            mul1.as_ptr(),
                            out.as_ptr(),
                            add.as_ptr(),
                            len as c_int,
                        ),
                        2 => api.fma_array(
                            out.as_mut_ptr(),
                            mul1.as_ptr(),
                            mul2.as_ptr(),
                            out.as_ptr(),
                            len as c_int,
                        ),
                        3 => api.fma_array(
                            out.as_mut_ptr(),
                            out.as_ptr(),
                            out.as_ptr(),
                            out.as_ptr(),
                            len as c_int,
                        ),
                        _ => unreachable!(),
                    }
                }
                out
            };
            assert_eq!(run(&c_api), run(&rust_api), "alias shape {alias}");
        }

        let shared = rng.vec(len);
        assert_eq!(
            call_distinct(&c_api, &original, &shared, &shared, &shared),
            call_distinct(&rust_api, &original, &shared, &shared, &shared)
        );

        for direction in 0..2 {
            let base = rng.vec(len + 1);
            let other = rng.vec(len);
            let addend = rng.vec(len);
            let run = |api: &Api| {
                let mut buffer = base.clone();
                unsafe {
                    if direction == 0 {
                        api.fma_array(
                            buffer.as_mut_ptr().add(1),
                            buffer.as_ptr(),
                            other.as_ptr(),
                            addend.as_ptr(),
                            len as c_int,
                        );
                    } else {
                        api.fma_array(
                            buffer.as_mut_ptr(),
                            buffer.as_ptr().add(1),
                            other.as_ptr(),
                            addend.as_ptr(),
                            len as c_int,
                        );
                    }
                }
                buffer
            };
            assert_eq!(
                run(&c_api),
                run(&rust_api),
                "shifted overlap direction {direction}"
            );
        }
    }
}

fn driver_stdout_rows_match() {
    let (c_api, rust_api) = apis();
    let c_null_output = unsafe { capture_stdout(|| c_api.driver(ptr::null(), 0)) };
    let rust_null_output = unsafe { capture_stdout(|| rust_api.driver(ptr::null(), 0)) };
    assert_eq!(
        c_null_output, rust_null_output,
        "null data with zero length"
    );

    let compare = |data: &[i32]| {
        let c_output =
            unsafe { capture_stdout(|| c_api.driver(data.as_ptr(), data.len() as c_int)) };
        let rust_output =
            unsafe { capture_stdout(|| rust_api.driver(data.as_ptr(), data.len() as c_int)) };
        assert_eq!(c_output, rust_output, "input length {}", data.len());
    };

    compare(&[]);
    let mut rng = Rng::new(0x94d0_49bb_1331_11eb);
    for _ in 0..256 {
        compare(&rng.vec(1));
    }
    for _ in 0..128 {
        let len = rng.len(2, 128);
        compare(&rng.vec(len));
    }
    compare(&rng.vec(4096));
}

#[test]
fn complete_differential_surface_matches() {
    exported_symbols_are_loadable();
    fma_array_length_and_distinct_buffer_rows_match();
    fma_array_alias_and_overlap_rows_match();
    driver_stdout_rows_match();
}
