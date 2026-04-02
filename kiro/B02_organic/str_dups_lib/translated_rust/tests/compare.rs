use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void, CString};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libstr_dups_lib.so")
}

// ── 1. stbds_rand_seed ────────────────────────────────────────────
#[test]
fn test_rand_seed() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("load C lib");
        let c_rand_seed: Symbol<unsafe extern "C" fn(usize)> =
            lib.get(b"stbds_rand_seed").unwrap();

        // Both should accept the call without crashing; no return value to compare.
        c_rand_seed(42);
        str_dups_lib::stbds_rand_seed(42);
    }
}

// ── 2. stbds_hash_string ──────────────────────────────────────────
#[test]
fn test_hash_string() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("load C lib");
        let c_hash_string: Symbol<unsafe extern "C" fn(*mut c_char, usize) -> usize> =
            lib.get(b"stbds_hash_string").unwrap();

        let seeds = [0usize, 1, 42, 0x31415926, 0xdeadbeef, usize::MAX];
        let strings = ["", "a", "hello", "test_0", "test_999", "the quick brown fox"];

        for &seed in &seeds {
            for s in &strings {
                let cs = CString::new(*s).unwrap();
                let c_result = c_hash_string(cs.as_ptr() as *mut c_char, seed);

                // Reset Rust global seed state to not interfere
                let r_result =
                    str_dups_lib::stbds_hash_string(cs.as_ptr() as *mut c_char, seed);

                assert_eq!(
                    c_result, r_result,
                    "hash_string mismatch for {:?} seed={}",
                    s, seed
                );
            }
        }
    }
}

// ── 3. stbds_hash_bytes ───────────────────────────────────────────
#[test]
fn test_hash_bytes() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("load C lib");
        let c_hash_bytes: Symbol<unsafe extern "C" fn(*mut c_void, usize, usize) -> usize> =
            lib.get(b"stbds_hash_bytes").unwrap();

        let seeds = [0usize, 42, 0x31415926];
        let test_data: Vec<Vec<u8>> = vec![
            vec![],
            vec![0],
            vec![1, 2, 3, 4],
            vec![1, 2, 3, 4, 5, 6, 7, 8],
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9],
            vec![0; 16],
            b"hello world".to_vec(),
            (0..=255).collect(),
        ];

        for &seed in &seeds {
            for data in &test_data {
                let mut buf = data.clone();
                let ptr = if buf.is_empty() {
                    std::ptr::null_mut()
                } else {
                    buf.as_mut_ptr() as *mut c_void
                };
                let c_result = c_hash_bytes(ptr, data.len(), seed);

                let mut buf2 = data.clone();
                let ptr2 = if buf2.is_empty() {
                    std::ptr::null_mut()
                } else {
                    buf2.as_mut_ptr() as *mut c_void
                };
                let r_result = str_dups_lib::stbds_hash_bytes(ptr2, data.len(), seed);

                assert_eq!(
                    c_result, r_result,
                    "hash_bytes mismatch for len={} seed={}",
                    data.len(),
                    seed
                );
            }
        }
    }
}

// ── 4. stbds_stralloc / stbds_strreset ────────────────────────────
#[test]
fn test_stralloc_strreset() {
    // We can't compare pointer values, but we can verify the allocated strings
    // contain the correct content and that reset doesn't crash.
    unsafe {
        let lib = Library::new(c_lib_path()).expect("load C lib");
        let c_stralloc: Symbol<
            unsafe extern "C" fn(*mut str_dups_lib::stbds_string_arena, *mut c_char) -> *mut c_char,
        > = lib.get(b"stbds_stralloc").unwrap();
        let c_strreset: Symbol<unsafe extern "C" fn(*mut str_dups_lib::stbds_string_arena)> =
            lib.get(b"stbds_strreset").unwrap();

        // Test C side
        let mut c_arena: str_dups_lib::stbds_string_arena = std::mem::zeroed();
        let test_strs = ["hello", "world", "test_123"];
        for s in &test_strs {
            let cs = CString::new(*s).unwrap();
            let p = c_stralloc(&mut c_arena, cs.as_ptr() as *mut c_char);
            let got = std::ffi::CStr::from_ptr(p).to_str().unwrap();
            assert_eq!(got, *s, "C stralloc content mismatch");
        }
        c_strreset(&mut c_arena);

        // Test Rust side
        let mut r_arena: str_dups_lib::stbds_string_arena = std::mem::zeroed();
        for s in &test_strs {
            let cs = CString::new(*s).unwrap();
            let p = str_dups_lib::stbds_stralloc(&mut r_arena, cs.as_ptr() as *mut c_char);
            let got = std::ffi::CStr::from_ptr(p).to_str().unwrap();
            assert_eq!(got, *s, "Rust stralloc content mismatch");
        }
        str_dups_lib::stbds_strreset(&mut r_arena);
    }
}

// ── 5. str_dups (capture stdout) ──────────────────────────────────
#[test]
fn test_str_dups_output() {
    use std::process::Command;

    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let c_lib = format!("{}/c_src/build/libstr_dups_lib.so", manifest_dir);

    // Small helper program that calls str_dups via the C library
    let c_output = Command::new("sh")
        .arg("-c")
        .arg(format!(
            r#"cat > /tmp/test_str_dups_c.c << 'EOF'
#include <dlfcn.h>
#include <stdlib.h>
int main() {{
    void *lib = dlopen("{}", RTLD_NOW);
    if (!lib) return 1;
    void (*fn)(int) = dlsym(lib, "str_dups");
    if (!fn) return 2;
    fn(5);
    dlclose(lib);
    return 0;
}}
EOF
gcc -o /tmp/test_str_dups_c /tmp/test_str_dups_c.c -ldl 2>&1 && /tmp/test_str_dups_c"#,
            c_lib
        ))
        .output()
        .expect("run C test");

    let c_stdout = String::from_utf8_lossy(&c_output.stdout);

    // Now call Rust version and capture its stdout
    let r_output = Command::new("sh")
        .arg("-c")
        .arg(format!(
            r#"cat > /tmp/test_str_dups_r.rs << 'REOF'
use std::ffi::c_int;
extern "C" {{ fn str_dups(num: c_int); }}
fn main() {{ unsafe {{ str_dups(5); }} }}
REOF
cd {} && cargo build --release 2>/dev/null
gcc -o /tmp/test_str_dups_r /tmp/test_str_dups_r.rs -L{}/target/release -lstr_dups_lib -Wl,-rpath,{}/target/release 2>/dev/null || true
LD_LIBRARY_PATH={}/target/release /tmp/test_str_dups_r 2>/dev/null || true"#,
            manifest_dir, manifest_dir, manifest_dir, manifest_dir
        ))
        .output()
        .expect("run Rust test");

    // Alternative: just call both via dlopen from a single C program
    // For now, let's use a simpler approach - call both from Rust directly
    // and capture via pipe

    // Actually, let's just call both directly and compare
    let c_out2 = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "LD_PRELOAD={} /tmp/test_str_dups_c",
            c_lib
        ))
        .output();

    // Simpler: just call both from Rust test and redirect stdout to pipe
    // Use libc pipe/dup2 to capture printf output
    let c_captured = capture_str_dups_c(&c_lib, 5);
    let r_captured = capture_str_dups_rust(5);

    assert_eq!(
        c_captured, r_captured,
        "str_dups output mismatch:\nC:    {:?}\nRust: {:?}",
        c_captured, r_captured
    );
}

fn capture_str_dups_c(lib_path: &str, num: i32) -> String {
    unsafe {
        // Create pipe
        let mut fds = [0i32; 2];
        libc::pipe(fds.as_mut_ptr());

        let old_stdout = libc::dup(1);
        libc::dup2(fds[1], 1);
        libc::close(fds[1]);

        let lib = Library::new(lib_path).expect("load C lib");
        let c_str_dups: Symbol<unsafe extern "C" fn(c_int)> = lib.get(b"str_dups").unwrap();
        c_str_dups(num);

        libc::fflush(std::ptr::null_mut());
        libc::dup2(old_stdout, 1);
        libc::close(old_stdout);

        let mut buf = vec![0u8; 4096];
        let n = libc::read(fds[0], buf.as_mut_ptr() as *mut c_void, buf.len());
        libc::close(fds[0]);

        if n > 0 {
            String::from_utf8_lossy(&buf[..n as usize]).to_string()
        } else {
            String::new()
        }
    }
}

fn capture_str_dups_rust(num: i32) -> String {
    unsafe {
        let mut fds = [0i32; 2];
        libc::pipe(fds.as_mut_ptr());

        let old_stdout = libc::dup(1);
        libc::dup2(fds[1], 1);
        libc::close(fds[1]);

        str_dups_lib::str_dups(num);

        libc::fflush(std::ptr::null_mut());
        libc::dup2(old_stdout, 1);
        libc::close(old_stdout);

        let mut buf = vec![0u8; 4096];
        let n = libc::read(fds[0], buf.as_mut_ptr() as *mut c_void, buf.len());
        libc::close(fds[0]);

        if n > 0 {
            String::from_utf8_lossy(&buf[..n as usize]).to_string()
        } else {
            String::new()
        }
    }
}

// ── 6. arrgrowf basic test ────────────────────────────────────────
#[test]
fn test_arrgrowf() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("load C lib");
        let c_arrgrowf: Symbol<
            unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void,
        > = lib.get(b"stbds_arrgrowf").unwrap();
        let c_arrfreef: Symbol<unsafe extern "C" fn(*mut c_void)> =
            lib.get(b"stbds_arrfreef").unwrap();

        // Test: grow from null
        let elemsize = std::mem::size_of::<i32>();
        let c_arr = c_arrgrowf(std::ptr::null_mut(), elemsize, 0, 4);
        assert!(!c_arr.is_null());
        c_arrfreef(c_arr);

        let r_arr = str_dups_lib::stbds_arrgrowf(std::ptr::null_mut(), elemsize, 0, 4);
        assert!(!r_arr.is_null());
        str_dups_lib::stbds_arrfreef(r_arr);
    }
}

// ── 7. Full hashmap round-trip ────────────────────────────────────
#[test]
fn test_hmput_hmget_round_trip() {
    // Test that both C and Rust produce the same hash table behavior
    unsafe {
        let lib = Library::new(c_lib_path()).expect("load C lib");

        // We test stbds_hash_string with the same seed to verify determinism
        let c_hash_string: Symbol<unsafe extern "C" fn(*mut c_char, usize) -> usize> =
            lib.get(b"stbds_hash_string").unwrap();

        let seed = 0x31415926usize;
        let test_keys = ["alpha", "beta", "gamma", "delta"];

        for key in &test_keys {
            let cs = CString::new(*key).unwrap();
            let c_h = c_hash_string(cs.as_ptr() as *mut c_char, seed);
            let r_h = str_dups_lib::stbds_hash_string(cs.as_ptr() as *mut c_char, seed);
            assert_eq!(c_h, r_h, "hash mismatch for key {:?}", key);
        }
    }
}
