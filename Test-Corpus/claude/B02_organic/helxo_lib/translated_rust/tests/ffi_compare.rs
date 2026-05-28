//! Integration tests that load both the C and Rust shared libraries via
//! libloading and compare the byte-for-byte output of their exported
//! functions.

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int};
use std::os::unix::io::FromRawFd;
use std::path::PathBuf;
use std::sync::Mutex;

// stdout fd-1 capture is process-wide, so any test that captures it must
// serialize against any other test that captures it.  This static mutex
// is acquired by `capture_stdout`.
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

const C_LIB: &str = "c_src/build/libtranslated_rust.so";
const RUST_LIB: &str = "target/debug/libhelxo_lib.so";

fn workspace_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn lib_path(rel: &str) -> PathBuf {
    workspace_dir().join(rel)
}

/// Run a closure in a forked child with stdout redirected to a pipe; return
/// the bytes the child wrote to stdout.  Forking isolates the redirection
/// so it can't bleed into other test threads.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::io::Read;

    unsafe {
        let mut fds = [0i32; 2];
        assert_eq!(libc::pipe(fds.as_mut_ptr()), 0, "pipe failed");
        let read_fd = fds[0];
        let write_fd = fds[1];

        let pid = libc::fork();
        if pid < 0 {
            panic!("fork failed");
        }
        if pid == 0 {
            // Child: redirect stdout, run callback, exit.
            libc::close(read_fd);
            libc::dup2(write_fd, 1);
            libc::close(write_fd);
            f();
            let _ = std::io::Write::flush(&mut std::io::stdout());
            libc::fflush(std::ptr::null_mut());
            libc::_exit(0);
        }
        // Parent: read from pipe.
        libc::close(write_fd);
        let mut file = std::fs::File::from_raw_fd(read_fd);
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).expect("read pipe");
        let mut status: i32 = 0;
        libc::waitpid(pid, &mut status, 0);
        buf
    }
}

unsafe fn load_lib(rel: &str) -> Library {
    unsafe { Library::new(lib_path(rel).as_os_str()).expect("library load") }
}

fn run_helxo(lib: &Library, letter: c_char) -> Vec<u8> {
    unsafe {
        let helxo: Symbol<unsafe extern "C" fn(c_char)> = lib.get(b"helxo").unwrap();
        capture_stdout(|| helxo(letter))
    }
}

#[test]
fn helxo_outputs_match_default_letter() {
    let c = unsafe { load_lib(C_LIB) };
    let r = unsafe { load_lib(RUST_LIB) };

    for letter in [b'x', b'A', b'!', 0u8, b'\n'] {
        let c_out = run_helxo(&c, letter as c_char);
        let r_out = run_helxo(&r, letter as c_char);
        assert_eq!(
            c_out, r_out,
            "helxo output mismatch for letter {letter:?}\nC:\n{}\nRust:\n{}",
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
    }
}

#[test]
fn helxo_outputs_match_negative_letter() {
    let c = unsafe { load_lib(C_LIB) };
    let r = unsafe { load_lib(RUST_LIB) };

    // Signed char values, including negatives, to exercise %c sign-extension.
    let letters: [c_char; 4] = [-1 as c_char, -128 as c_char, 127 as c_char, 1 as c_char];
    for &letter in letters.iter() {
        let c_out = run_helxo(&c, letter);
        let r_out = run_helxo(&r, letter);
        assert_eq!(
            c_out, r_out,
            "helxo output mismatch for letter {letter}\nC: {:?}\nRust: {:?}",
            c_out, r_out
        );
    }
}

// --------------------------------------------------------------------------
// Tests for stb_ds.h symbols re-exported from the Rust .so so external
// callers can link against either library interchangeably.
// --------------------------------------------------------------------------

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct StbdsArrayHeader {
    length: usize,
    capacity: usize,
    hash_table: *mut std::ffi::c_void,
    temp: isize,
}

unsafe fn header_of<T>(p: *mut T) -> *mut StbdsArrayHeader {
    unsafe { (p as *mut StbdsArrayHeader).offset(-1) }
}

#[test]
fn arrgrowf_grows_array_consistently() {
    let c = unsafe { load_lib(C_LIB) };
    let r = unsafe { load_lib(RUST_LIB) };

    type ArrGrowF = unsafe extern "C" fn(
        a: *mut std::ffi::c_void,
        elemsize: usize,
        addlen: usize,
        min_cap: usize,
    ) -> *mut std::ffi::c_void;
    type ArrFreeF = unsafe extern "C" fn(a: *mut std::ffi::c_void);

    unsafe {
        let c_grow: Symbol<ArrGrowF> = c.get(b"stbds_arrgrowf").unwrap();
        let r_grow: Symbol<ArrGrowF> = r.get(b"stbds_arrgrowf").unwrap();
        let c_free: Symbol<ArrFreeF> = c.get(b"stbds_arrfreef").unwrap();
        let r_free: Symbol<ArrFreeF> = r.get(b"stbds_arrfreef").unwrap();

        // Grow from null.
        for &(addlen, min_cap) in &[(0usize, 1usize), (3, 0), (0, 16), (1, 100)] {
            let cp = c_grow(std::ptr::null_mut(), 4, addlen, min_cap);
            let rp = r_grow(std::ptr::null_mut(), 4, addlen, min_cap);
            assert!(!cp.is_null());
            assert!(!rp.is_null());
            let ch = *header_of(cp as *mut u8);
            let rh = *header_of(rp as *mut u8);
            assert_eq!(ch.length, rh.length, "length mismatch addlen={addlen} min_cap={min_cap}");
            assert_eq!(ch.capacity, rh.capacity, "capacity mismatch addlen={addlen} min_cap={min_cap}");
            c_free(cp);
            r_free(rp);
        }
    }
}

#[test]
fn hash_string_matches() {
    let c = unsafe { load_lib(C_LIB) };
    let r = unsafe { load_lib(RUST_LIB) };

    type HashStr = unsafe extern "C" fn(*mut c_char, usize) -> usize;
    unsafe {
        let c_h: Symbol<HashStr> = c.get(b"stbds_hash_string").unwrap();
        let r_h: Symbol<HashStr> = r.get(b"stbds_hash_string").unwrap();
        for s in &["", "a", "hello world", "test_42"] {
            let mut bytes: Vec<u8> = s.bytes().chain(std::iter::once(0)).collect();
            for seed in [0u64, 1, 0xdeadbeef, 0x31415926] {
                let cv = c_h(bytes.as_mut_ptr() as *mut c_char, seed as usize);
                let rv = r_h(bytes.as_mut_ptr() as *mut c_char, seed as usize);
                assert_eq!(cv, rv, "hash_string mismatch s={s:?} seed={seed:#x}");
            }
        }
    }
}

#[test]
fn hash_bytes_matches() {
    let c = unsafe { load_lib(C_LIB) };
    let r = unsafe { load_lib(RUST_LIB) };

    type HashB = unsafe extern "C" fn(*mut std::ffi::c_void, usize, usize) -> usize;
    unsafe {
        let c_h: Symbol<HashB> = c.get(b"stbds_hash_bytes").unwrap();
        let r_h: Symbol<HashB> = r.get(b"stbds_hash_bytes").unwrap();
        for input in &[
            vec![],
            vec![0u8],
            (0u8..=15u8).collect::<Vec<u8>>(),
            (0u8..=63u8).collect::<Vec<u8>>(),
        ] {
            let mut buf = input.clone();
            for seed in [0u64, 1, 0xdeadbeef, 0x31415926] {
                let cv = c_h(
                    buf.as_mut_ptr() as *mut std::ffi::c_void,
                    buf.len(),
                    seed as usize,
                );
                let rv = r_h(
                    buf.as_mut_ptr() as *mut std::ffi::c_void,
                    buf.len(),
                    seed as usize,
                );
                assert_eq!(cv, rv, "hash_bytes mismatch len={} seed={:#x}", buf.len(), seed);
            }
        }
    }
}

#[test]
fn rand_seed_symbol_present() {
    // We don't assert behavior here (the seed is internal state), just that
    // the Rust .so exports the symbol the C .so exports.
    let r = unsafe { load_lib(RUST_LIB) };
    unsafe {
        let _: Symbol<unsafe extern "C" fn(usize)> = r.get(b"stbds_rand_seed").unwrap();
    }
}

#[test]
fn strkey_matches() {
    let c = unsafe { load_lib(C_LIB) };
    let r = unsafe { load_lib(RUST_LIB) };

    type StrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
    unsafe {
        let cs: Symbol<StrKey> = c.get(b"strkey").unwrap();
        let rs: Symbol<StrKey> = r.get(b"strkey").unwrap();
        for n in [0, 1, 42, -7, 100000] {
            let cp = cs(n);
            let rp = rs(n);
            let cstr = std::ffi::CStr::from_ptr(cp).to_bytes().to_vec();
            let rstr = std::ffi::CStr::from_ptr(rp).to_bytes().to_vec();
            assert_eq!(cstr, rstr, "strkey({n}) mismatch");
        }
    }
}

#[test]
fn hmput_get_basic() {
    let c = unsafe { load_lib(C_LIB) };
    let r = unsafe { load_lib(RUST_LIB) };

    // Mimic a string-keyed hash by directly calling stbds_hmput_key with
    // STBDS_HM_STRING (= 1) on a hash backed by `struct { char *key; int value; }`.
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Entry {
        key: *mut c_char,
        value: c_int,
    }

    type HmPutKey = unsafe extern "C" fn(
        a: *mut std::ffi::c_void,
        elemsize: usize,
        key: *mut std::ffi::c_void,
        keysize: usize,
        mode: c_int,
    ) -> *mut std::ffi::c_void;

    type HmFree = unsafe extern "C" fn(*mut std::ffi::c_void, usize);

    unsafe fn run(
        put: &Symbol<HmPutKey>,
        free: &Symbol<HmFree>,
    ) -> Vec<(Vec<u8>, c_int)> {
        unsafe {
            let mut hash: *mut std::ffi::c_void = std::ptr::null_mut();
            let elemsize = std::mem::size_of::<Entry>();
            let keys = ["alpha", "beta", "gamma", "delta", "alpha"];
            let values: [c_int; 5] = [1, 2, 3, 4, 99]; // alpha overwrites
            let mut cstrings: Vec<Vec<u8>> = keys
                .iter()
                .map(|k| {
                    let mut v = k.as_bytes().to_vec();
                    v.push(0);
                    v
                })
                .collect();
            // Helper: returns pointer to underlying array header for the
            // user-facing `hash` pointer.
            let header_of = |h: *mut std::ffi::c_void| -> *mut StbdsArrayHeader {
                let raw = (h as *mut u8).sub(elemsize);
                (raw as *mut StbdsArrayHeader).offset(-1)
            };

            for (i, k) in cstrings.iter_mut().enumerate() {
                let kptr = k.as_mut_ptr() as *mut std::ffi::c_void;
                hash = put(hash, elemsize, kptr, std::mem::size_of::<*mut c_char>(), 1);
                // After put, header.temp is the user-visible slot index.
                let header = header_of(hash);
                let temp = (*header).temp;
                let entries = hash as *mut Entry;
                (*entries.offset(temp)).value = values[i];
            }

            // Now read back: user entries are hash[0..length-1]; the
            // underlying array's first element is reserved (hence length-1
            // user entries).
            let header = header_of(hash);
            let length = (*header).length; // total underlying length
            let entries = hash as *mut Entry;
            let mut result = Vec::new();
            for i in 0..(length - 1) {
                let e = *entries.add(i);
                let key_bytes = std::ffi::CStr::from_ptr(e.key).to_bytes().to_vec();
                result.push((key_bytes, e.value));
            }
            // hmfree_func takes the underlying array pointer (`hash - 1`
            // in the C macro `stbds_hmfree`).
            let raw_arr = (hash as *mut u8).sub(elemsize) as *mut std::ffi::c_void;
            free(raw_arr, elemsize);
            result
        }
    }

    unsafe {
        let cput: Symbol<HmPutKey> = c.get(b"stbds_hmput_key").unwrap();
        let rput: Symbol<HmPutKey> = r.get(b"stbds_hmput_key").unwrap();
        let cfree: Symbol<HmFree> = c.get(b"stbds_hmfree_func").unwrap();
        let rfree: Symbol<HmFree> = r.get(b"stbds_hmfree_func").unwrap();

        let cres = run(&cput, &cfree);
        let rres = run(&rput, &rfree);
        assert_eq!(cres, rres, "hmput/hmget result mismatch");
    }
}

// Exercise the actual `helxo` flow which uses all of shput/shlen/shfree
// underneath.  This is the primary integration test.
#[test]
fn helxo_full_alphabet() {
    let c = unsafe { load_lib(C_LIB) };
    let r = unsafe { load_lib(RUST_LIB) };

    for letter in b'a'..=b'z' {
        let c_out = run_helxo(&c, letter as c_char);
        let r_out = run_helxo(&r, letter as c_char);
        assert_eq!(
            c_out, r_out,
            "helxo({letter:?}) mismatch:\nC:\n{}\nRust:\n{}",
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
    }
}

// Test stralloc + strreset by exercising the string-arena path on both
// libraries.  We allocate a private arena, copy a string in, and confirm
// the two libraries produce equal arena state at each step.
#[test]
fn stralloc_strreset_basic_smoke() {
    // We cannot easily compare arena byte-for-byte across libraries
    // because they store host pointers.  Instead, verify that each
    // library's stralloc returns a NUL-terminated copy of the input
    // and that strreset frees without crashing.
    type StrAlloc = unsafe extern "C" fn(*mut std::ffi::c_void, *mut c_char) -> *mut c_char;
    type StrReset = unsafe extern "C" fn(*mut std::ffi::c_void);

    let cs = [
        unsafe { load_lib(C_LIB) },
        unsafe { load_lib(RUST_LIB) },
    ];
    for lib in &cs {
        unsafe {
            let stralloc: Symbol<StrAlloc> = lib.get(b"stbds_stralloc").unwrap();
            let strreset: Symbol<StrReset> = lib.get(b"stbds_strreset").unwrap();

            // Layout of stbds_string_arena: { void*, size_t, u8 block, u8 mode }
            #[repr(C)]
            #[derive(Default)]
            struct Arena {
                storage: *mut std::ffi::c_void,
                remaining: usize,
                block: u8,
                mode: u8,
                _pad: [u8; 6],
            }
            let mut arena = Arena {
                storage: std::ptr::null_mut(),
                remaining: 0,
                block: 0,
                mode: 0,
                _pad: [0; 6],
            };
            for s in ["hi", "world", "another string", &"x".repeat(600)] {
                let mut buf: Vec<u8> = s.bytes().chain(std::iter::once(0)).collect();
                let p = stralloc(
                    &mut arena as *mut _ as *mut std::ffi::c_void,
                    buf.as_mut_ptr() as *mut c_char,
                );
                let returned = std::ffi::CStr::from_ptr(p).to_bytes();
                assert_eq!(returned, s.as_bytes(), "stralloc returned wrong bytes");
            }
            strreset(&mut arena as *mut _ as *mut std::ffi::c_void);
            assert!(arena.storage.is_null());
            assert_eq!(arena.remaining, 0);
        }
    }
}

#[test]
fn hmdel_key_basic() {
    let c = unsafe { load_lib(C_LIB) };
    let r = unsafe { load_lib(RUST_LIB) };

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Entry {
        key: *mut c_char,
        value: c_int,
    }

    type HmPutKey = unsafe extern "C" fn(
        a: *mut std::ffi::c_void,
        elemsize: usize,
        key: *mut std::ffi::c_void,
        keysize: usize,
        mode: c_int,
    ) -> *mut std::ffi::c_void;
    type HmDelKey = unsafe extern "C" fn(
        a: *mut std::ffi::c_void,
        elemsize: usize,
        key: *mut std::ffi::c_void,
        keysize: usize,
        keyoffset: usize,
        mode: c_int,
    ) -> *mut std::ffi::c_void;
    type HmFree = unsafe extern "C" fn(*mut std::ffi::c_void, usize);

    unsafe fn run(
        put: &Symbol<HmPutKey>,
        del: &Symbol<HmDelKey>,
        free: &Symbol<HmFree>,
    ) -> Vec<(Vec<u8>, c_int)> {
        unsafe {
            let mut hash: *mut std::ffi::c_void = std::ptr::null_mut();
            let elemsize = std::mem::size_of::<Entry>();
            let keys = ["a", "b", "c", "d", "e"];
            let mut cstrings: Vec<Vec<u8>> = keys
                .iter()
                .map(|k| {
                    let mut v = k.as_bytes().to_vec();
                    v.push(0);
                    v
                })
                .collect();
            for (i, k) in cstrings.iter_mut().enumerate() {
                hash = put(
                    hash,
                    elemsize,
                    k.as_mut_ptr() as *mut std::ffi::c_void,
                    std::mem::size_of::<*mut c_char>(),
                    1,
                );
                let raw = (hash as *mut u8).sub(elemsize) as *mut StbdsArrayHeader;
                let header = raw.offset(-1);
                let entries = hash as *mut Entry;
                (*entries.offset((*header).temp)).value = (i + 1) as c_int;
            }

            // Delete "c"
            let mut delkey = b"c\0".to_vec();
            hash = del(
                hash,
                elemsize,
                delkey.as_mut_ptr() as *mut std::ffi::c_void,
                std::mem::size_of::<*mut c_char>(),
                0,
                1,
            );

            // Read remaining
            let raw = (hash as *mut u8).sub(elemsize) as *mut StbdsArrayHeader;
            let header = raw.offset(-1);
            let length = (*header).length;
            let entries = hash as *mut Entry;
            let mut result = Vec::new();
            for i in 0..(length - 1) {
                let e = *entries.add(i);
                let key_bytes = std::ffi::CStr::from_ptr(e.key).to_bytes().to_vec();
                result.push((key_bytes, e.value));
            }
            let raw_arr = (hash as *mut u8).sub(elemsize) as *mut std::ffi::c_void;
            free(raw_arr, elemsize);
            result
        }
    }

    unsafe {
        let cput: Symbol<HmPutKey> = c.get(b"stbds_hmput_key").unwrap();
        let cdel: Symbol<HmDelKey> = c.get(b"stbds_hmdel_key").unwrap();
        let cfree: Symbol<HmFree> = c.get(b"stbds_hmfree_func").unwrap();
        let rput: Symbol<HmPutKey> = r.get(b"stbds_hmput_key").unwrap();
        let rdel: Symbol<HmDelKey> = r.get(b"stbds_hmdel_key").unwrap();
        let rfree: Symbol<HmFree> = r.get(b"stbds_hmfree_func").unwrap();

        let cres = run(&cput, &cdel, &cfree);
        let rres = run(&rput, &rdel, &rfree);
        assert_eq!(cres, rres, "hmdel result mismatch");
    }
}

// Verify every C-exported user symbol is also in the Rust .so.
#[test]
fn rust_exports_match_c_exports() {
    use std::process::Command;

    fn defined_symbols(path: &PathBuf) -> Vec<String> {
        let out = Command::new("nm")
            .args(["-D", "--defined-only"])
            .arg(path)
            .output()
            .expect("run nm");
        let stdout = String::from_utf8_lossy(&out.stdout);
        stdout
            .lines()
            .filter_map(|l| {
                let mut it = l.split_whitespace();
                let _addr = it.next()?;
                let kind = it.next()?;
                let name = it.next()?;
                if kind == "T" {
                    // Filter out linker-generated init/fini and rust mangled
                    // crate symbols (we only compare user-facing C symbols).
                    if name.starts_with('_') {
                        None
                    } else {
                        Some(name.to_string())
                    }
                } else {
                    None
                }
            })
            .collect()
    }

    let mut c_syms = defined_symbols(&lib_path(C_LIB));
    let mut r_syms = defined_symbols(&lib_path(RUST_LIB));
    c_syms.sort();
    r_syms.sort();
    for s in &c_syms {
        assert!(
            r_syms.contains(s),
            "Rust .so missing C-exported symbol: {s}\nC syms: {c_syms:?}\nRust syms: {r_syms:?}",
        );
    }
}
