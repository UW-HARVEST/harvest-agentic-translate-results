use libloading::Library;
use std::ffi::{c_int, c_void};
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};

type HashFn = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
type SiphashFn = unsafe extern "C" fn(c_int);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

struct Apis {
    _c_library: Library,
    _rust_library: Library,
    c_hash: HashFn,
    rust_hash: HashFn,
    c_siphash: SiphashFn,
    rust_siphash: SiphashFn,
}

impl Apis {
    fn load() -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest
            .parent()
            .expect("translation crate must have a parent")
            .join("c_src/build/libharvest-work-Wm9von.so");
        let rust_path = manifest.join("target/release/libsiphash_lib.so");

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

struct Prng(u64);

impl Prng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value >> 12;
        value ^= value << 25;
        value ^= value >> 27;
        self.0 = value;
        value.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn fill(&mut self, bytes: &mut [u8]) {
        for chunk in bytes.chunks_mut(8) {
            let random = self.next_u64().to_ne_bytes();
            chunk.copy_from_slice(&random[..chunk.len()]);
        }
    }
}

fn seed_for_iteration(rng: &mut Prng, iteration: usize) -> usize {
    match iteration % 5 {
        0 => 0,
        1 => usize::MAX,
        2 => 1,
        3 => 1usize << (usize::BITS - 1),
        _ => rng.next_u64() as usize,
    }
}

fn exercise_hash_row(apis: &Apis, rng: &mut Prng, row: usize, complete_words: usize, tail: usize) {
    const ITERATIONS: usize = 256;
    const MULTI_WORD_COUNTS: [usize; 8] = [2, 3, 7, 16, 64, 257, 1024, 8192];

    for iteration in 0..ITERATIONS {
        let words = match complete_words {
            0 | 1 => complete_words,
            _ => {
                if iteration < MULTI_WORD_COUNTS.len() {
                    MULTI_WORD_COUNTS[iteration]
                } else {
                    MULTI_WORD_COUNTS[rng.next_u64() as usize % MULTI_WORD_COUNTS.len()]
                }
            }
        };
        let len = words * size_of::<usize>() + tail;
        let offset = rng.next_u64() as usize % size_of::<usize>();
        let mut allocation = vec![0u8; offset + len.max(1)];

        match iteration {
            0 => allocation.fill(0),
            1 => allocation.fill(0xff),
            2 => {
                for (index, byte) in allocation.iter_mut().enumerate() {
                    *byte = index as u8;
                }
            }
            _ => rng.fill(&mut allocation),
        }

        let seed = seed_for_iteration(rng, iteration);
        let data = unsafe { allocation.as_mut_ptr().add(offset).cast::<c_void>() };
        let c_result = unsafe { (apis.c_hash)(data, len, seed) };
        let rust_result = unsafe { (apis.rust_hash)(data, len, seed) };

        assert_eq!(
            c_result, rust_result,
            "CONFIGS.md row {row}: iteration={iteration}, len={len}, seed={seed:#x}, offset={offset}"
        );
    }
}

#[test]
fn all_hash_configuration_rows_match() {
    let apis = Apis::load();
    let mut rng = Prng::new(0x6a09_e667_f3bc_c909);
    let mut row = 1;

    for complete_words in 0..=2 {
        for tail in 0..size_of::<usize>() {
            exercise_hash_row(&apis, &mut rng, row, complete_words, tail);
            row += 1;
        }
    }
    assert_eq!(row, 25, "all 24 hash rows must execute");

    for iteration in 0..256 {
        let seed = seed_for_iteration(&mut rng, iteration);
        let c_result = unsafe { (apis.c_hash)(ptr::null_mut(), 0, seed) };
        let rust_result = unsafe { (apis.rust_hash)(ptr::null_mut(), 0, seed) };
        assert_eq!(
            c_result, rust_result,
            "NULL/zero boundary mismatch for seed {seed:#x}"
        );
    }
}

static CAPTURE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn capture_stdout(function: SiphashFn, init: c_int) -> Vec<u8> {
    let sequence = CAPTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "siphash-differential-{}-{sequence}.out",
        std::process::id()
    ));
    let mut output = OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(&path)
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", path.display()));

    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0, "fflush before redirect");
        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0, "dup(stdout) failed");
        assert_eq!(dup2(output.as_raw_fd(), 1), 1, "redirect stdout failed");

        function(init);

        assert_eq!(fflush(ptr::null_mut()), 0, "fflush after call");
        assert_eq!(dup2(saved_stdout, 1), 1, "restore stdout failed");
        assert_eq!(close(saved_stdout), 0, "close saved stdout failed");
    }

    output.seek(SeekFrom::Start(0)).expect("rewind capture");
    let mut bytes = Vec::new();
    output.read_to_end(&mut bytes).expect("read capture");
    drop(output);
    std::fs::remove_file(&path)
        .unwrap_or_else(|error| panic!("failed to remove {}: {error}", path.display()));
    bytes
}

#[test]
fn siphash_full_operation_matches() {
    let apis = Apis::load();
    let mut rng = Prng::new(0xbb67_ae85_84ca_a73b);
    let maximum_defined_init = i32::MAX as i64 - 64;
    let defined_init_count = (maximum_defined_init - i32::MIN as i64 + 1) as u64;
    let mut initial_values = vec![i32::MIN, -1, 0, 1, maximum_defined_init as i32];

    for _ in 0..123 {
        let value = i32::MIN as i64 + (rng.next_u64() % defined_init_count) as i64;
        initial_values.push(value as i32);
    }

    for (iteration, init) in initial_values.into_iter().enumerate() {
        let c_output = capture_stdout(apis.c_siphash, init);
        let rust_output = capture_stdout(apis.rust_siphash, init);
        assert_eq!(
            c_output, rust_output,
            "CONFIGS.md row 25: iteration={iteration}, init={init}"
        );
        assert!(!c_output.is_empty(), "siphash must emit its 64 output rows");
    }
}
