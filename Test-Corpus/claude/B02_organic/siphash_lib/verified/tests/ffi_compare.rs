// Integration test: compares the C .so to the Rust .so via libloading.
// We never call Rust functions directly — we always go through the cdylib's
// exported `extern "C"` symbols, exactly like a foreign caller would.

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::sync::Once;

type StbdsHashBytesFn = unsafe extern "C" fn(*const u8, usize, usize) -> usize;
type SiphashFn = unsafe extern "C" fn(i32);

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    project_root().join("c_src/build/libtranslated_rust.so")
}

fn rust_so_path() -> PathBuf {
    let target_dir = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| project_root().join("target"));
    let candidates = [
        target_dir.join("debug/libsiphash_lib.so"),
        target_dir.join("release/libsiphash_lib.so"),
    ];
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    candidates[0].clone()
}

fn build_artifacts() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        if !c_so_path().exists() {
            let build_dir = project_root().join("c_src/build");
            std::fs::create_dir_all(&build_dir).unwrap();
            let _ = std::process::Command::new("cmake")
                .args([".."])
                .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
                .current_dir(&build_dir)
                .status()
                .unwrap();
            let _ = std::process::Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build_dir)
                .status()
                .unwrap();
        }
        if !rust_so_path().exists() {
            let _ = std::process::Command::new("cargo")
                .args(["build"])
                .current_dir(project_root())
                .status()
                .unwrap();
        }
    });
}

unsafe fn load_libs() -> (Library, Library) {
    build_artifacts();
    let c = Library::new(c_so_path()).expect("load C .so");
    let r = Library::new(rust_so_path()).expect("load Rust .so");
    (c, r)
}

#[test]
fn compare_stbds_hash_bytes_empty() {
    unsafe {
        let (c, r) = load_libs();
        let c_fn: Symbol<StbdsHashBytesFn> = c.get(b"stbds_hash_bytes").unwrap();
        let r_fn: Symbol<StbdsHashBytesFn> = r.get(b"stbds_hash_bytes").unwrap();

        for &seed in &[0usize, 1, 0x12345, usize::MAX, usize::MAX / 2] {
            let cv = c_fn(std::ptr::null(), 0, seed);
            let rv = r_fn(std::ptr::null(), 0, seed);
            assert_eq!(cv, rv, "len=0 seed={:#x}", seed);
        }
    }
}

#[test]
fn compare_stbds_hash_bytes_short_lengths() {
    unsafe {
        let (c, r) = load_libs();
        let c_fn: Symbol<StbdsHashBytesFn> = c.get(b"stbds_hash_bytes").unwrap();
        let r_fn: Symbol<StbdsHashBytesFn> = r.get(b"stbds_hash_bytes").unwrap();

        let mut buffer = [0u8; 128];
        for (seed_idx, &seed) in [0usize, 1, 0xDEAD_BEEF, usize::MAX].iter().enumerate() {
            let start = (seed_idx as u8).wrapping_mul(7);
            for (i, b) in buffer.iter_mut().enumerate() {
                *b = (start as usize).wrapping_add(i) as u8;
            }
            for len in 0..=64usize {
                let cv = c_fn(buffer.as_ptr(), len, seed);
                let rv = r_fn(buffer.as_ptr(), len, seed);
                assert_eq!(cv, rv, "len={} seed={:#x}", len, seed);
            }
        }
    }
}

#[test]
fn compare_stbds_hash_bytes_high_bit_bytes() {
    // The C implementation has implementation-defined sign-extension when
    // a byte ≥ 0x80 is shifted into the sign bit of an int.
    unsafe {
        let (c, r) = load_libs();
        let c_fn: Symbol<StbdsHashBytesFn> = c.get(b"stbds_hash_bytes").unwrap();
        let r_fn: Symbol<StbdsHashBytesFn> = r.get(b"stbds_hash_bytes").unwrap();

        let buffer = [0xFFu8; 64];
        for len in 0..=64usize {
            let cv = c_fn(buffer.as_ptr(), len, 0);
            let rv = r_fn(buffer.as_ptr(), len, 0);
            assert_eq!(cv, rv, "all-FF len={}", len);
        }

        let mut buffer = [0u8; 64];
        for i in 0..buffer.len() {
            buffer[i] = if i % 4 == 3 { 0x80 } else { (i as u8) | 0x80 };
        }
        for len in 0..=64usize {
            let cv = c_fn(buffer.as_ptr(), len, 0);
            let rv = r_fn(buffer.as_ptr(), len, 0);
            assert_eq!(cv, rv, "high-bit-pattern len={}", len);
        }
    }
}

#[test]
fn compare_stbds_hash_bytes_random_inputs() {
    unsafe {
        let (c, r) = load_libs();
        let c_fn: Symbol<StbdsHashBytesFn> = c.get(b"stbds_hash_bytes").unwrap();
        let r_fn: Symbol<StbdsHashBytesFn> = r.get(b"stbds_hash_bytes").unwrap();

        let mut state: u64 = 0xC0FFEE_BABE;
        let mut buffer = vec![0u8; 256];
        for trial in 0..100 {
            for b in buffer.iter_mut() {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                *b = (state >> 33) as u8;
            }
            for &len in &[0usize, 1, 2, 3, 7, 8, 9, 15, 16, 17, 31, 32, 63, 64, 100, 200, 256] {
                state = state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let seed = state as usize;
                let cv = c_fn(buffer.as_ptr(), len, seed);
                let rv = r_fn(buffer.as_ptr(), len, seed);
                assert_eq!(cv, rv, "trial={} len={} seed={:#x}", trial, len, seed);
            }
        }
    }
}

// `siphash` writes through libc's printf. We capture stdout using a subprocess
// helper so we don't fight with parallel test threads or the test framework's
// own stdout handling. The harness binary itself runs the helper.
#[test]
fn compare_siphash_stdout() {
    fn run_siphash(which: &str, init: i32) -> Vec<u8> {
        let exe = std::env::current_exe().expect("current_exe");
        let out = std::process::Command::new(&exe)
            .env("SIPHASH_HELPER_LIB", which)
            .env("SIPHASH_HELPER_INIT", init.to_string())
            .args(["--exact", "siphash_helper_entry", "--nocapture"])
            .output()
            .expect("spawn helper");
        // The helper exits with status 0 after writing the captured bytes
        // followed by a sentinel marker. Extract bytes between the markers.
        let stdout = out.stdout;
        let begin = b"<<<SIPHASH_HELPER_BEGIN>>>";
        let end = b"<<<SIPHASH_HELPER_END>>>";
        let pos_b = find_subslice(&stdout, begin)
            .unwrap_or_else(|| panic!("helper stdout missing BEGIN marker: {:?}", String::from_utf8_lossy(&stdout)));
        let pos_e = find_subslice(&stdout, end)
            .unwrap_or_else(|| panic!("helper stdout missing END marker"));
        let start = pos_b + begin.len();
        stdout[start..pos_e].to_vec()
    }

    fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        if needle.is_empty() || haystack.len() < needle.len() {
            return None;
        }
        (0..=haystack.len() - needle.len()).find(|&i| &haystack[i..i + needle.len()] == needle)
    }

    for &init in &[0i32, 1, 7, 42, -1, 100] {
        let c_out = run_siphash("c", init);
        let r_out = run_siphash("rust", init);
        if c_out != r_out {
            let cs = String::from_utf8_lossy(&c_out);
            let rs = String::from_utf8_lossy(&r_out);
            eprintln!("--- C OUTPUT ---\n{}", cs);
            eprintln!("--- R OUTPUT ---\n{}", rs);
            panic!("siphash output mismatch for init={}", init);
        }
    }
}

// Helper test that the parent test re-spawns. When env vars are set, it loads
// the requested .so and invokes `siphash`, with stdout captured between
// markers. Without env vars set it does nothing and passes.
#[test]
fn siphash_helper_entry() {
    let which = match std::env::var("SIPHASH_HELPER_LIB") {
        Ok(v) => v,
        Err(_) => return, // not running as helper
    };
    let init: i32 = std::env::var("SIPHASH_HELPER_INIT")
        .expect("SIPHASH_HELPER_INIT")
        .parse()
        .expect("parse init");

    use std::io::Write;
    print!("<<<SIPHASH_HELPER_BEGIN>>>");
    std::io::stdout().flush().unwrap();
    extern "C" {
        fn fflush(stream: *mut core::ffi::c_void) -> i32;
    }
    unsafe { fflush(core::ptr::null_mut()) };

    let path = match which.as_str() {
        "c" => c_so_path(),
        "rust" => rust_so_path(),
        other => panic!("unknown SIPHASH_HELPER_LIB={}", other),
    };
    unsafe {
        let lib = Library::new(&path).expect("load helper lib");
        let f: Symbol<SiphashFn> = lib.get(b"siphash").unwrap();
        f(init);
        fflush(core::ptr::null_mut());
    }

    print!("<<<SIPHASH_HELPER_END>>>");
    std::io::stdout().flush().unwrap();
    unsafe { fflush(core::ptr::null_mut()) };
}

#[test]
fn rust_so_exports_all_c_symbols() {
    // Sanity check: every public symbol that the C .so exports must also be
    // exported by the Rust .so. Use `nm -D` and diff the lists.
    build_artifacts();

    fn defined_symbols(path: &std::path::Path) -> std::collections::BTreeSet<String> {
        let out = std::process::Command::new("nm")
            .args(["-D", "--defined-only"])
            .arg(path)
            .output()
            .expect("nm -D");
        let s = String::from_utf8_lossy(&out.stdout);
        let mut set = std::collections::BTreeSet::new();
        for line in s.lines() {
            // Format: "<addr> <type> <name>"
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 3 {
                continue;
            }
            let kind = parts[1];
            let name = parts[2];
            // Only public code/data symbols (T/D/B/R), skip linker internals.
            if !matches!(kind, "T" | "D" | "B" | "R") {
                continue;
            }
            if matches!(
                name,
                "_init"
                    | "_fini"
                    | "__bss_start"
                    | "_end"
                    | "_edata"
                    | "__TMC_END__"
            ) {
                continue;
            }
            set.insert(name.to_string());
        }
        set
    }

    let c_syms = defined_symbols(&c_so_path());
    let r_syms = defined_symbols(&rust_so_path());
    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing C-exported symbols: {:?}",
        missing
    );
}
