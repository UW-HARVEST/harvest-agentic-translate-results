use libloading::Library;
use std::ffi::{c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

type HashFn = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
type SiphashFn = unsafe extern "C" fn(c_int);

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
}

struct APIs {
    _c_library: Library,
    _rust_library: Library,
    c_hash: HashFn,
    rust_hash: HashFn,
    c_siphash: SiphashFn,
    rust_siphash: SiphashFn,
}

impl APIs {
    fn load() -> Self {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest_dir.join("c_src/build/libtranslated_rust.so");
        let rust_path = rust_library_path(&manifest_dir);

        assert!(
            c_path.is_file(),
            "missing C shared object: {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "missing Rust shared object: {}",
            rust_path.display()
        );

        unsafe {
            let c_library = Library::new(&c_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display()));
            let rust_library = Library::new(&rust_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display()));

            let c_hash = *c_library
                .get::<HashFn>(b"stbds_hash_bytes\0")
                .expect("C stbds_hash_bytes export");
            let rust_hash = *rust_library
                .get::<HashFn>(b"stbds_hash_bytes\0")
                .expect("Rust stbds_hash_bytes export");
            let c_siphash = *c_library
                .get::<SiphashFn>(b"siphash\0")
                .expect("C siphash export");
            let rust_siphash = *rust_library
                .get::<SiphashFn>(b"siphash\0")
                .expect("Rust siphash export");

            Self {
                _c_library: c_library,
                _rust_library: rust_library,
                c_hash,
                rust_hash,
                c_siphash,
                rust_siphash,
            }
        }
    }
}

fn rust_library_path(manifest_dir: &Path) -> PathBuf {
    let target_dir = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .map(|path| {
            if path.is_absolute() {
                path
            } else {
                manifest_dir.join(path)
            }
        })
        .unwrap_or_else(|| manifest_dir.join("target"));

    target_dir.join("debug/libsiphash_lib.so")
}

#[derive(Clone, Copy)]
struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn fill(&mut self, bytes: &mut [u8]) {
        for byte in bytes {
            *byte = self.next_u64() as u8;
        }
    }
}

fn compare_hash(apis: &APIs, bytes: &mut [u8], len: usize, seed: usize, context: &str) {
    let pointer = bytes.as_mut_ptr().cast();
    let c_result = unsafe { (apis.c_hash)(pointer, len, seed) };
    let rust_result = unsafe { (apis.rust_hash)(pointer, len, seed) };
    assert_eq!(
        rust_result, c_result,
        "{context}: len={len}, seed={seed:#x}"
    );
}

#[test]
fn all_hash_block_and_tail_configurations_match() {
    let apis = APIs::load();
    let mut rng = Rng(0x4d59_5df4_d0f3_3173);

    for complete_blocks in 0..=1 {
        for tail_len in 0..8 {
            let len = complete_blocks * size_of::<usize>() + tail_len;
            for case in 0..256 {
                let mut bytes = vec![0; len.max(1)];
                rng.fill(&mut bytes);
                let seed = rng.next_u64() as usize;
                compare_hash(
                    &apis,
                    &mut bytes,
                    len,
                    seed,
                    &format!(
                        "{complete_blocks} complete block(s), {tail_len}-byte tail, case {case}"
                    ),
                );
            }
        }
    }

    for tail_len in 0..8 {
        for case in 0..256 {
            let complete_blocks = 2 + (rng.next_u64() as usize % 127);
            let len = complete_blocks * size_of::<usize>() + tail_len;
            let mut bytes = vec![0; len];
            rng.fill(&mut bytes);
            let seed = rng.next_u64() as usize;
            compare_hash(
                &apis,
                &mut bytes,
                len,
                seed,
                &format!("multiple complete blocks, {tail_len}-byte tail, case {case}"),
            );
        }
    }
}

#[test]
fn hash_defined_ffi_boundaries_match() {
    let apis = APIs::load();

    let c_null = unsafe { (apis.c_hash)(std::ptr::null_mut(), 0, usize::MAX) };
    let rust_null = unsafe { (apis.rust_hash)(std::ptr::null_mut(), 0, usize::MAX) };
    assert_eq!(rust_null, c_null, "NULL with zero length");

    let mut empty_storage = [0u8; 1];
    for seed in [0, 1, usize::MAX, 0x0123_4567_89ab_cdef] {
        compare_hash(&apis, &mut empty_storage, 0, seed, "zero length");
    }

    let mut rng = Rng(0xa076_1d64_78bd_642f);
    for len in [
        1,
        size_of::<usize>() - 1,
        size_of::<usize>(),
        size_of::<usize>() + 1,
        4096,
        65_535,
        1_048_583,
    ] {
        let mut bytes = vec![0; len];
        rng.fill(&mut bytes);
        compare_hash(
            &apis,
            &mut bytes,
            len,
            rng.next_u64() as usize,
            "boundary length",
        );
    }
}

#[test]
fn siphash_formatted_output_matches() {
    let _stdout_guard = STDOUT_LOCK.lock().expect("stdout lock");
    let apis = APIs::load();
    let mut rng = Rng(0xe703_7ed1_a0b4_28db);
    let mut initializers = vec![c_int::MIN, c_int::MAX, -1, 0, 1];
    initializers.extend((0..32).map(|_| rng.next_u64() as c_int));

    for init in initializers {
        let c_output = capture_stdout(|| unsafe { (apis.c_siphash)(init) });
        let rust_output = capture_stdout(|| unsafe { (apis.rust_siphash)(init) });
        assert_eq!(rust_output, c_output, "formatted output for init={init}");
        assert_eq!(
            c_output.iter().filter(|&&byte| byte == b'\n').count(),
            64,
            "C output line count for init={init}"
        );
    }
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    const STDOUT_FILENO: c_int = 1;
    let mut pipe_fds = [-1; 2];

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0, "flush before redirect");
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0, "create stdout pipe");
        let saved_stdout = dup(STDOUT_FILENO);
        assert!(saved_stdout >= 0, "duplicate stdout");
        assert_eq!(dup2(pipe_fds[1], STDOUT_FILENO), STDOUT_FILENO);
        assert_eq!(close(pipe_fds[1]), 0, "close duplicate pipe writer");

        call();

        assert_eq!(fflush(std::ptr::null_mut()), 0, "flush captured output");
        assert_eq!(dup2(saved_stdout, STDOUT_FILENO), STDOUT_FILENO);
        assert_eq!(close(saved_stdout), 0, "close saved stdout");

        let mut output = Vec::new();
        let mut reader = File::from_raw_fd(pipe_fds[0]);
        reader.read_to_end(&mut output).expect("read stdout pipe");
        output
    }
}
