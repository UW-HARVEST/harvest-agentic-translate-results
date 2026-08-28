use libloading::{Library, Symbol};
use std::env;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::ptr;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct TflacMd5 {
    a: u32,
    b: u32,
    c: u32,
    d: u32,
}

type Md5Digest = unsafe extern "C" fn(*const TflacMd5, *mut u8);

struct DigestLibrary {
    _library: Library,
    digest: Md5Digest,
}

impl DigestLibrary {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let digest = {
            let symbol: Symbol<Md5Digest> =
                unsafe { library.get(b"md5_digest\0") }.unwrap_or_else(|error| {
                    panic!("failed to load md5_digest from {}: {error}", path.display())
                });
            *symbol
        };
        Self {
            _library: library,
            digest,
        }
    }

    fn run(&self, state: &TflacMd5, fill: u8) -> [u8; 32] {
        let mut guarded = [fill; 32];
        unsafe {
            (self.digest)(state, guarded.as_mut_ptr().add(8));
        }
        guarded
    }

    fn run_overlapping(&self, state: &TflacMd5, output_offset: usize) -> [u32; 8] {
        let mut storage = [0xa5a5_a5a5; 8];
        storage[2] = state.a;
        storage[3] = state.b;
        storage[4] = state.c;
        storage[5] = state.d;
        unsafe {
            let state_pointer = storage.as_ptr().add(2).cast::<TflacMd5>();
            let output_pointer = storage.as_mut_ptr().cast::<u8>().add(output_offset);
            (self.digest)(state_pointer, output_pointer);
        }
        storage
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("../c_src/build/libharvest-work-Dl8FUR.so")
}

fn rust_library_path() -> PathBuf {
    manifest_dir()
        .join("target")
        .join("release")
        .join("libmd5_digest_lib.so")
}

fn next_u32(seed: &mut u64) -> u32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    (*seed >> 16) as u32
}

fn invalid_pointer_child_status(library: &str, case: &str) -> ExitStatus {
    Command::new(env::current_exe().expect("failed to locate differential test executable"))
        .args(["--exact", "invalid_pointer_child", "--nocapture"])
        .env("MD5_INVALID_POINTER_LIBRARY", library)
        .env("MD5_INVALID_POINTER_CASE", case)
        .status()
        .unwrap_or_else(|error| panic!("failed to run {library} {case} child: {error}"))
}

#[test]
fn invalid_pointer_child() {
    let Ok(library_name) = env::var("MD5_INVALID_POINTER_LIBRARY") else {
        return;
    };
    let case = env::var("MD5_INVALID_POINTER_CASE").expect("missing invalid pointer case");
    let path = match library_name.as_str() {
        "c" => c_library_path(),
        "rust" => rust_library_path(),
        other => panic!("unknown library selector: {other}"),
    };
    let library = unsafe { DigestLibrary::load(&path) };
    let state = TflacMd5 {
        a: 0x0123_4567,
        b: 0x89ab_cdef,
        c: 0x55aa_55aa,
        d: 0xaa55_aa55,
    };
    let mut output = [0u8; 16];

    unsafe {
        match case.as_str() {
            "null_state" => (library.digest)(ptr::null(), output.as_mut_ptr()),
            "null_output" => (library.digest)(&state, ptr::null_mut()),
            other => panic!("unknown invalid pointer case: {other}"),
        }
    }
    panic!("{library_name} {case} unexpectedly returned");
}

#[test]
fn null_pointer_process_rejection_matches() {
    for case in ["null_state", "null_output"] {
        let c_status = invalid_pointer_child_status("c", case);
        let rust_status = invalid_pointer_child_status("rust", case);

        assert!(
            !c_status.success() && !rust_status.success(),
            "{case}: expected both invalid calls to terminate; C={c_status}, Rust={rust_status}"
        );
        assert_eq!(
            (c_status.code(), c_status.signal()),
            (rust_status.code(), rust_status.signal()),
            "{case}: process rejection differs; C={c_status}, Rust={rust_status}"
        );
    }
}

#[test]
fn md5_digest_matches_for_fixed_layout_and_full_u32_domain() {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(c_path.is_file(), "missing C library: {}", c_path.display());
    assert!(
        rust_path.is_file(),
        "missing Rust library: {}",
        rust_path.display()
    );

    let c = unsafe { DigestLibrary::load(&c_path) };
    let rust = unsafe { DigestLibrary::load(&rust_path) };

    let boundary_states = [
        TflacMd5 {
            a: 0,
            b: 0,
            c: 0,
            d: 0,
        },
        TflacMd5 {
            a: 1,
            b: 1,
            c: 1,
            d: 1,
        },
        TflacMd5 {
            a: u32::MAX,
            b: u32::MAX,
            c: u32::MAX,
            d: u32::MAX,
        },
        TflacMd5 {
            a: 0x0000_00ff,
            b: 0x0000_ff00,
            c: 0x00ff_0000,
            d: 0xff00_0000,
        },
        TflacMd5 {
            a: 0x0123_4567,
            b: 0x89ab_cdef,
            c: 0x8000_0000,
            d: 0x7fff_ffff,
        },
    ];

    for (index, state) in boundary_states.iter().enumerate() {
        let fill = 0xa5 ^ index as u8;
        assert_eq!(
            c.run(state, fill),
            rust.run(state, fill),
            "boundary state {index} diverged: {state:?}"
        );
    }

    let mut seed = 0x6a09_e667_f3bc_c909;
    for case in 0..10_000 {
        let state = TflacMd5 {
            a: next_u32(&mut seed),
            b: next_u32(&mut seed),
            c: next_u32(&mut seed),
            d: next_u32(&mut seed),
        };
        let fill = next_u32(&mut seed) as u8;
        assert_eq!(
            c.run(&state, fill),
            rust.run(&state, fill),
            "random case {case} diverged with seed state {seed:#018x}: {state:?}"
        );
    }

    for case in 0..10_000 {
        let state = TflacMd5 {
            a: next_u32(&mut seed),
            b: next_u32(&mut seed),
            c: next_u32(&mut seed),
            d: next_u32(&mut seed),
        };
        let output_offset = next_u32(&mut seed) as usize % 17;
        assert_eq!(
            c.run_overlapping(&state, output_offset),
            rust.run_overlapping(&state, output_offset),
            "overlap case {case} diverged at byte offset {output_offset}: {state:?}"
        );
    }
}
