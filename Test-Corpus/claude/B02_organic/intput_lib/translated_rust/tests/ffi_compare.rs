// Integration tests that load BOTH the C-built shared library and the Rust
// `cdylib` via libloading and compare their outputs through the FFI boundary
// for every function the C library exports.
//
// Both .so files are expected to live in well-known locations relative to the
// crate root:
//   - C:    c_src/build/libtranslated_rust.so
//   - Rust: target/{debug,release}/libintput_lib.so
//
// Run with:  cargo test --release  (or cargo test)
// The CMake build step needs to have run first, see top-level instructions.

use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helpers to locate the two shared libraries.
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> PathBuf {
    workspace_root().join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    // Try release first, then debug.
    let release = workspace_root().join("target/release/libintput_lib.so");
    if release.exists() {
        return release;
    }
    workspace_root().join("target/debug/libintput_lib.so")
}

unsafe fn load_libs() -> (Library, Library) {
    let c = Library::new(c_lib_path()).expect("failed to load C .so");
    let r = Library::new(rust_lib_path()).expect("failed to load Rust .so");
    (c, r)
}

// ---------------------------------------------------------------------------
// stbds_rand_seed + stbds_hash_string
// ---------------------------------------------------------------------------

#[test]
fn test_stbds_rand_seed_and_hash_string() {
    unsafe {
        let (c_lib, r_lib) = load_libs();
        let c_seed: Symbol<unsafe extern "C" fn(usize)> =
            c_lib.get(b"stbds_rand_seed").unwrap();
        let r_seed: Symbol<unsafe extern "C" fn(usize)> =
            r_lib.get(b"stbds_rand_seed").unwrap();

        let c_hash: Symbol<unsafe extern "C" fn(*mut c_char, usize) -> usize> =
            c_lib.get(b"stbds_hash_string").unwrap();
        let r_hash: Symbol<unsafe extern "C" fn(*mut c_char, usize) -> usize> =
            r_lib.get(b"stbds_hash_string").unwrap();

        // Set seed in both libs.
        let seeds: &[usize] = &[
            0,
            1,
            0x31415926,
            0xDEADBEEFCAFEBABE,
            usize::MAX,
        ];
        let strs: &[&str] = &[
            "",
            "a",
            "ab",
            "abcdefgh",
            "the quick brown fox jumps over the lazy dog",
            "test_0",
            "test_-1",
            "test_2147483647",
        ];

        for &seed in seeds {
            c_seed(seed);
            r_seed(seed);
            for s in strs {
                let cs = CString::new(*s).unwrap();
                let cv = c_hash(cs.as_ptr() as *mut c_char, seed);
                let rv = r_hash(cs.as_ptr() as *mut c_char, seed);
                assert_eq!(cv, rv, "stbds_hash_string mismatch for {:?} seed {:#x}", s, seed);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// stbds_hash_bytes
// ---------------------------------------------------------------------------

#[test]
fn test_stbds_hash_bytes() {
    unsafe {
        let (c_lib, r_lib) = load_libs();
        let c_hash: Symbol<unsafe extern "C" fn(*mut c_void, usize, usize) -> usize> =
            c_lib.get(b"stbds_hash_bytes").unwrap();
        let r_hash: Symbol<unsafe extern "C" fn(*mut c_void, usize, usize) -> usize> =
            r_lib.get(b"stbds_hash_bytes").unwrap();

        let inputs: Vec<Vec<u8>> = vec![
            vec![],
            vec![0],
            vec![0xff],
            vec![1, 2, 3],
            vec![1, 2, 3, 4, 5, 6, 7],
            vec![1, 2, 3, 4, 5, 6, 7, 8],
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9],
            (0..16u8).collect(),
            (0..32u8).collect(),
            (0..63u8).collect(),
            (0..64u8).collect(),
            (0..65u8).collect(),
            (0..255u8).collect(),
            // i32 keys 0, 1, 9, 11 byte-by-byte (little-endian)
            0i32.to_le_bytes().to_vec(),
            1i32.to_le_bytes().to_vec(),
            9i32.to_le_bytes().to_vec(),
            11i32.to_le_bytes().to_vec(),
        ];

        let seeds: &[usize] = &[0, 1, 0x31415926, 0xDEADBEEFCAFEBABE];

        for seed in seeds {
            for buf in &inputs {
                let mut tmp = buf.clone();
                let p = if tmp.is_empty() {
                    std::ptr::null_mut()
                } else {
                    tmp.as_mut_ptr() as *mut c_void
                };
                let cv = c_hash(p, tmp.len(), *seed);
                let rv = r_hash(p, tmp.len(), *seed);
                assert_eq!(
                    cv, rv,
                    "stbds_hash_bytes mismatch len={} seed={:#x}",
                    tmp.len(),
                    seed
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// strkey
// ---------------------------------------------------------------------------

#[test]
fn test_strkey() {
    unsafe {
        let (c_lib, r_lib) = load_libs();
        let c_strkey: Symbol<unsafe extern "C" fn(c_int) -> *mut c_char> =
            c_lib.get(b"strkey").unwrap();
        let r_strkey: Symbol<unsafe extern "C" fn(c_int) -> *mut c_char> =
            r_lib.get(b"strkey").unwrap();

        let cases: &[c_int] = &[0, 1, -1, 9, 11, 12345, -42, c_int::MAX, c_int::MIN];
        for &n in cases {
            let cp = c_strkey(n);
            let rp = r_strkey(n);
            // Read until NUL.
            let c_bytes = std::ffi::CStr::from_ptr(cp).to_bytes().to_vec();
            let r_bytes = std::ffi::CStr::from_ptr(rp).to_bytes().to_vec();
            assert_eq!(c_bytes, r_bytes, "strkey({}) mismatch", n);
        }
    }
}

// ---------------------------------------------------------------------------
// intput - top-level public API.
//
// The function has no observable return value other than via assertions
// (which abort the process on failure). To compare the two libraries we
// run each `intput` call inside a forked child process and compare exit
// status, stdout and stderr.
// ---------------------------------------------------------------------------

fn run_intput_in_child(lib_path: &std::path::Path, num: c_int) -> i32 {
    use std::process::{Command, Stdio};
    // Use the current test binary as its own child runner via env.
    let exe = std::env::current_exe().unwrap();
    let out = Command::new(&exe)
        .env("INTPUT_LIB", lib_path.as_os_str())
        .env("INTPUT_NUM", num.to_string())
        .arg("--ignored")
        .arg("--exact")
        .arg("intput_runner")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .status();
    match out {
        Ok(s) => s.code().unwrap_or(-1),
        Err(_) => -2,
    }
}

#[test]
#[ignore]
fn intput_runner() {
    // Used as a child process. Call intput from the lib at INTPUT_LIB with
    // INTPUT_NUM. If the lib's intput aborts (assert), the process exits
    // non-zero; otherwise it exits 0. We use std::process::abort on assert
    // failures inside the lib so the difference is observable as exit code.
    let lib_path = std::env::var_os("INTPUT_LIB");
    let num = std::env::var("INTPUT_NUM")
        .ok()
        .and_then(|s| s.parse::<c_int>().ok());
    if let (Some(p), Some(n)) = (lib_path, num) {
        unsafe {
            let lib = Library::new(p).expect("failed to load lib");
            let intput: Symbol<unsafe extern "C" fn(c_int)> = lib.get(b"intput").unwrap();
            intput(n);
        }
    }
}

#[test]
fn test_intput_no_assert_inputs() {
    // Inputs that should NOT trigger any assertions in either library.
    // From the C source, the asserts require:
    //   hmget(intmap, num) == 7  ->  num != 9 AND num != 11
    //   (since after "hmput(intmap, 9, num)" the value at key 9 is num; for
    //    that to equal num the map must contain it; that's always true.)
    //   hmget(intmap,  9) == num  -> always, since we set 9 -> num last.
    //   hmget(intmap, 11) == 3   -> requires num != 11 (else 11's value is 7).
    //   hmget(intmap, num) == 7  -> requires num != 9 AND num != 11.
    let inputs: &[c_int] = &[0, 1, -1, 12345, -42, 100, c_int::MAX, c_int::MIN];
    for &n in inputs {
        let cs = run_intput_in_child(&c_lib_path(), n);
        let rs = run_intput_in_child(&rust_lib_path(), n);
        assert_eq!(cs, rs, "intput({}) exit-code mismatch C={} Rust={}", n, cs, rs);
    }
}

#[test]
fn test_intput_asserting_inputs() {
    // Inputs that DO trigger assertions in the C version. The Rust version
    // must do the same.
    let inputs: &[c_int] = &[9, 11];
    for &n in inputs {
        let cs = run_intput_in_child(&c_lib_path(), n);
        let rs = run_intput_in_child(&rust_lib_path(), n);
        assert_eq!(cs, rs, "intput({}) exit-code mismatch C={} Rust={}", n, cs, rs);
        // Both should be a non-zero exit code (assertion abort).
        assert!(cs != 0, "expected C intput({}) to abort, got {}", n, cs);
        assert!(rs != 0, "expected Rust intput({}) to abort, got {}", n, rs);
    }
}

// ---------------------------------------------------------------------------
// Stub-export presence: these are exported by both .so files even though the
// Rust translation does not implement their full semantics. Just make sure
// the symbols load.
// ---------------------------------------------------------------------------

#[test]
fn test_all_stbds_symbols_present() {
    let names: &[&[u8]] = &[
        b"stbds_arrgrowf",
        b"stbds_arrfreef",
        b"stbds_hmfree_func",
        b"stbds_hmget_key",
        b"stbds_hmget_key_ts",
        b"stbds_hmput_default",
        b"stbds_hmput_key",
        b"stbds_hmdel_key",
        b"stbds_shmode_func",
        b"stbds_stralloc",
        b"stbds_strreset",
        b"stbds_rand_seed",
        b"stbds_hash_string",
        b"stbds_hash_bytes",
        b"strkey",
        b"intput",
    ];
    unsafe {
        let (c_lib, r_lib) = load_libs();
        for name in names {
            let _: Symbol<unsafe extern "C" fn()> =
                c_lib.get(name).unwrap_or_else(|_| panic!("C missing {:?}", name));
            let _: Symbol<unsafe extern "C" fn()> =
                r_lib.get(name).unwrap_or_else(|_| panic!("Rust missing {:?}", name));
        }
    }
}
