// Integration tests that load BOTH the C and Rust shared libraries
// via libloading and compare results function-by-function across the
// FFI boundary.

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::os::raw::c_uchar;
use std::path::PathBuf;
use std::ptr;

const C_SO: &str = "c_src/build/libtranslated_rust.so";
const RUST_SO_BASE: &str = "target";
const RUST_SO_NAME: &str = "libstr_dups_lib.so";

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> PathBuf {
    project_root().join(C_SO)
}

fn rust_lib_path() -> PathBuf {
    // Prefer release; fall back to debug.
    let release = project_root().join(RUST_SO_BASE).join("release").join(RUST_SO_NAME);
    if release.exists() {
        return release;
    }
    project_root().join(RUST_SO_BASE).join("debug").join(RUST_SO_NAME)
}

fn load_libs() -> (Library, Library) {
    unsafe {
        let c = Library::new(c_lib_path()).expect("load C .so");
        let r = Library::new(rust_lib_path()).expect("load Rust .so");
        (c, r)
    }
}

// ------------------------------------------------------------------
// stbds_hash_string
// ------------------------------------------------------------------
type HashStringFn = unsafe extern "C" fn(*mut c_char, usize) -> usize;

#[test]
fn hash_string_matches() {
    let (c, r) = load_libs();
    unsafe {
        let cf: Symbol<HashStringFn> = c.get(b"stbds_hash_string").unwrap();
        let rf: Symbol<HashStringFn> = r.get(b"stbds_hash_string").unwrap();

        let inputs: &[&[u8]] = &[
            b"\0",
            b"a\0",
            b"abcdef\0",
            b"This is a longer test string\0",
            b"!@#$%^&*()_+-=[]{}|;:'\",.<>/?\0",
            b"\xff\xfe\xfd\xfc\0",
        ];
        let seeds: &[usize] = &[0, 1, 0x31415926, 0xdeadbeef, 0x123456789abcdef0];

        for input in inputs {
            for &seed in seeds {
                let mut buf = input.to_vec();
                let cv = cf(buf.as_mut_ptr() as *mut c_char, seed);
                let rv = rf(buf.as_mut_ptr() as *mut c_char, seed);
                assert_eq!(cv, rv, "hash_string(input={:?}, seed={:#x})", input, seed);
            }
        }
    }
}

// ------------------------------------------------------------------
// stbds_hash_bytes
// ------------------------------------------------------------------
type HashBytesFn = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;

#[test]
fn hash_bytes_matches() {
    let (c, r) = load_libs();
    unsafe {
        let cf: Symbol<HashBytesFn> = c.get(b"stbds_hash_bytes").unwrap();
        let rf: Symbol<HashBytesFn> = r.get(b"stbds_hash_bytes").unwrap();

        let inputs: &[Vec<u8>] = &[
            vec![],
            vec![0],
            vec![1, 2, 3, 4],
            vec![0xff; 8],
            (0..16u8).collect(),
            (0..32u8).collect(),
            (0..64u8).collect(),
            vec![0xaa; 100],
        ];
        let seeds: &[usize] = &[0, 1, 0x31415926, 0xdeadbeef, 0x123456789abcdef0];

        for buf in inputs {
            for &seed in seeds {
                let mut b = buf.clone();
                let p = if b.is_empty() {
                    ptr::null_mut()
                } else {
                    b.as_mut_ptr() as *mut c_void
                };
                let cv = cf(p, b.len(), seed);
                let rv = rf(p, b.len(), seed);
                assert_eq!(
                    cv, rv,
                    "hash_bytes(len={}, seed={:#x}, bytes={:?})",
                    b.len(),
                    seed,
                    &b[..b.len().min(8)]
                );
            }
        }
    }
}

// ------------------------------------------------------------------
// stbds_arrgrowf / stbds_arrfreef — basic round-trip
// ------------------------------------------------------------------
type ArrGrowFn = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
type ArrFreeFn = unsafe extern "C" fn(*mut c_void);

#[repr(C)]
struct ArrayHeader {
    length: usize,
    capacity: usize,
    hash_table: *mut c_void,
    temp: isize,
}

unsafe fn header(a: *mut c_void) -> *mut ArrayHeader {
    (a as *mut ArrayHeader).offset(-1)
}

fn run_arrgrow(
    growf: &Symbol<ArrGrowFn>,
    freef: &Symbol<ArrFreeFn>,
) -> (usize, usize, isize) {
    unsafe {
        // Start with NULL, grow several times, push values, return the
        // final {length, capacity, temp} so we can compare across libs.
        let mut a: *mut c_void = ptr::null_mut();
        let elemsize = 4usize;
        for n in &[1usize, 5, 10, 100, 1000] {
            a = growf(a, elemsize, *n, 0);
            // Manually bump length, mimicking arrput.
            let h = header(a);
            for k in 0..*n {
                let p = (a as *mut u32).add((*h).length + k);
                *p = (k as u32) ^ 0xbeef;
            }
            (*h).length += *n;
        }
        let h = header(a);
        let result = ((*h).length, (*h).capacity, (*h).temp);
        freef(a);
        result
    }
}

#[test]
fn arrgrow_arrfree_matches() {
    let (c, r) = load_libs();
    unsafe {
        let cgrow: Symbol<ArrGrowFn> = c.get(b"stbds_arrgrowf").unwrap();
        let cfree: Symbol<ArrFreeFn> = c.get(b"stbds_arrfreef").unwrap();
        let rgrow: Symbol<ArrGrowFn> = r.get(b"stbds_arrgrowf").unwrap();
        let rfree: Symbol<ArrFreeFn> = r.get(b"stbds_arrfreef").unwrap();
        let cv = run_arrgrow(&cgrow, &cfree);
        let rv = run_arrgrow(&rgrow, &rfree);
        assert_eq!(cv, rv, "arrgrowf result mismatch");
    }
}

// ------------------------------------------------------------------
// String arena: stbds_stralloc / stbds_strreset.
// ------------------------------------------------------------------
#[repr(C)]
#[derive(Clone, Copy)]
struct StringArena {
    storage: *mut c_void,
    remaining: usize,
    block: u8,
    mode: u8,
    _pad: [u8; 6],
}

type StrallocFn = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
type StrresetFn = unsafe extern "C" fn(*mut StringArena);

fn drive_arena(
    salloc: &Symbol<StrallocFn>,
    sreset: &Symbol<StrresetFn>,
) -> Vec<String> {
    unsafe {
        let mut arena = StringArena {
            storage: ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
            _pad: [0; 6],
        };
        let mut results: Vec<String> = Vec::new();
        for i in 0..50 {
            let s = format!("entry_{:03}\0", i);
            let mut bytes = s.into_bytes();
            let p = salloc(&mut arena, bytes.as_mut_ptr() as *mut c_char);
            let len = my_strlen(p);
            let slice = std::slice::from_raw_parts(p as *const u8, len);
            results.push(String::from_utf8_lossy(slice).into_owned());
        }
        sreset(&mut arena);
        results
    }
}

unsafe fn my_strlen(p: *const c_char) -> usize {
    let mut n = 0;
    while *p.add(n) != 0 {
        n += 1;
    }
    n
}

#[test]
fn stralloc_strreset_matches() {
    let (c, r) = load_libs();
    unsafe {
        let csa: Symbol<StrallocFn> = c.get(b"stbds_stralloc").unwrap();
        let csr: Symbol<StrresetFn> = c.get(b"stbds_strreset").unwrap();
        let rsa: Symbol<StrallocFn> = r.get(b"stbds_stralloc").unwrap();
        let rsr: Symbol<StrresetFn> = r.get(b"stbds_strreset").unwrap();
        let cv = drive_arena(&csa, &csr);
        let rv = drive_arena(&rsa, &rsr);
        assert_eq!(cv, rv);
    }
}

// ------------------------------------------------------------------
// strkey: produces "test_<n>" in a static buffer.
// ------------------------------------------------------------------
type StrkeyFn = unsafe extern "C" fn(c_int) -> *mut c_char;

#[test]
fn strkey_matches() {
    let (c, r) = load_libs();
    unsafe {
        let cf: Symbol<StrkeyFn> = c.get(b"strkey").unwrap();
        let rf: Symbol<StrkeyFn> = r.get(b"strkey").unwrap();
        for n in &[0i32, 1, 5, 42, 99, -1, 1234567] {
            let cp = cf(*n);
            let cs = std::ffi::CStr::from_ptr(cp).to_owned();
            let rp = rf(*n);
            let rs = std::ffi::CStr::from_ptr(rp).to_owned();
            assert_eq!(cs, rs, "strkey({})", n);
        }
    }
}

// ------------------------------------------------------------------
// stbds_shmode_func + stbds_hmput_key + stbds_hmget_key + stbds_hmfree_func
// ------------------------------------------------------------------
type ShmodeFn = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
type HmputKeyFn = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type HmgetKeyFn = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type HmfreeFn = unsafe extern "C" fn(*mut c_void, usize);
type HmdelKeyFn = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;

#[repr(C)]
struct StrEntry {
    key: *mut c_char,
    value: c_int,
}

const STBDS_HM_STRING: c_int = 1;
const STBDS_SH_STRDUP: c_int = 2;

fn drive_strdup_map(lib: &Library) -> Vec<(String, c_int, isize)> {
    unsafe {
        let shmode: Symbol<ShmodeFn> = lib.get(b"stbds_shmode_func").unwrap();
        let hmput: Symbol<HmputKeyFn> = lib.get(b"stbds_hmput_key").unwrap();
        let hmget: Symbol<HmgetKeyFn> = lib.get(b"stbds_hmget_key").unwrap();
        let hmfree: Symbol<HmfreeFn> = lib.get(b"stbds_hmfree_func").unwrap();

        let elemsize = std::mem::size_of::<StrEntry>();
        let mut t = shmode(elemsize, STBDS_SH_STRDUP);

        let keys: &[&[u8]] = &[
            b"alpha\0",
            b"beta\0",
            b"gamma\0",
            b"delta\0",
            b"epsilon\0",
            b"zeta\0",
            b"eta\0",
            b"theta\0",
            b"iota\0",
            b"kappa\0",
        ];
        for (i, k) in keys.iter().enumerate() {
            let mut kbuf = k.to_vec();
            t = hmput(
                t,
                elemsize,
                kbuf.as_mut_ptr() as *mut c_void,
                std::mem::size_of::<*mut c_char>(),
                STBDS_HM_STRING,
            );
            // Assign the value following the temp index, replicating
            // shput / shputs:
            let raw = ((t as *mut u8).sub(elemsize) as *mut ArrayHeader).offset(-1);
            let temp_idx = (*raw).temp;
            let entry = (t as *mut u8).add(elemsize * temp_idx as usize) as *mut StrEntry;
            // Read back the duplicated key pointer that hmput stashed in
            // the table's temp_key slot (first field of hash_table).
            let temp_key = *((*raw).hash_table as *mut *mut c_char);
            (*entry).key = temp_key;
            (*entry).value = (i as c_int) * 7 + 1;
        }

        // Now do reads.
        let mut results = Vec::new();
        let mut all_keys: Vec<&[u8]> = keys.to_vec();
        all_keys.push(b"missing\0");
        for k in all_keys.iter() {
            let mut kbuf = k.to_vec();
            t = hmget(
                t,
                elemsize,
                kbuf.as_mut_ptr() as *mut c_void,
                std::mem::size_of::<*mut c_char>(),
                STBDS_HM_STRING,
            );
            let raw = ((t as *mut u8).sub(elemsize) as *mut ArrayHeader).offset(-1);
            let idx = (*raw).temp;
            if idx < 0 {
                results.push((String::from_utf8_lossy(k).trim_end_matches('\0').to_string(), -1, idx));
            } else {
                let entry = (t as *mut u8).add(elemsize * idx as usize) as *mut StrEntry;
                let kstr = std::ffi::CStr::from_ptr((*entry).key).to_string_lossy().into_owned();
                results.push((kstr, (*entry).value, idx));
            }
        }

        let raw = (t as *mut u8).sub(elemsize) as *mut c_void;
        hmfree(raw, elemsize);
        results
    }
}

#[test]
fn strdup_hashmap_matches() {
    let (c, r) = load_libs();
    let cv = drive_strdup_map(&c);
    let rv = drive_strdup_map(&r);
    assert_eq!(cv, rv);
}

// ------------------------------------------------------------------
// Binary-keyed hash map (mode==STBDS_HM_BINARY)
// ------------------------------------------------------------------
#[repr(C)]
struct IntEntry {
    key: c_int,
    value: c_int,
}

fn drive_int_map(lib: &Library) -> Vec<(c_int, isize, c_int)> {
    unsafe {
        let hmput: Symbol<HmputKeyFn> = lib.get(b"stbds_hmput_key").unwrap();
        let hmget: Symbol<HmgetKeyFn> = lib.get(b"stbds_hmget_key").unwrap();
        let hmdel: Symbol<HmdelKeyFn> = lib.get(b"stbds_hmdel_key").unwrap();
        let hmfree: Symbol<HmfreeFn> = lib.get(b"stbds_hmfree_func").unwrap();

        let elemsize = std::mem::size_of::<IntEntry>();
        let mut t: *mut c_void = ptr::null_mut();
        let n = 50i32;
        for i in 0..n {
            let mut k = i;
            t = hmput(
                t,
                elemsize,
                &mut k as *mut c_int as *mut c_void,
                std::mem::size_of::<c_int>(),
                0,
            );
            let raw = ((t as *mut u8).sub(elemsize) as *mut ArrayHeader).offset(-1);
            let idx = (*raw).temp;
            let entry = (t as *mut u8).add(elemsize * idx as usize) as *mut IntEntry;
            (*entry).key = k;
            (*entry).value = k * 3 + 7;
        }

        // Delete a few.
        for i in &[3i32, 17, 42, 99] {
            let mut k = *i;
            t = hmdel(
                t,
                elemsize,
                &mut k as *mut c_int as *mut c_void,
                std::mem::size_of::<c_int>(),
                0,
                0,
            );
        }

        // Read back.
        let mut results = Vec::new();
        for i in -2..n + 5 {
            let mut k = i;
            t = hmget(
                t,
                elemsize,
                &mut k as *mut c_int as *mut c_void,
                std::mem::size_of::<c_int>(),
                0,
            );
            let raw = ((t as *mut u8).sub(elemsize) as *mut ArrayHeader).offset(-1);
            let idx = (*raw).temp;
            if idx < 0 {
                results.push((i, idx, -1));
            } else {
                let entry = (t as *mut u8).add(elemsize * idx as usize) as *mut IntEntry;
                results.push((i, idx, (*entry).value));
            }
        }

        let raw = (t as *mut u8).sub(elemsize) as *mut c_void;
        hmfree(raw, elemsize);
        results
    }
}

#[test]
fn int_hashmap_matches() {
    let (c, r) = load_libs();
    let cv = drive_int_map(&c);
    let rv = drive_int_map(&r);
    assert_eq!(cv, rv);
}

// ------------------------------------------------------------------
// str_dups: full top-level test.
// ------------------------------------------------------------------
type StrDupsFn = unsafe extern "C" fn(c_int);

fn run_in_subprocess_capture(lib: &str, num: c_int) -> Vec<u8> {
    use std::process::Command;
    // Build a tiny C harness once.
    let runner = std::env::temp_dir().join("str_dups_runner");
    if !runner.exists() {
        let src = std::env::temp_dir().join("str_dups_runner.c");
        std::fs::write(
            &src,
            br#"
#include <stdio.h>
#include <stdlib.h>
#include <dlfcn.h>
int main(int argc, char**argv){
  void *h=dlopen(argv[1],2);
  if(!h){fprintf(stderr,"%s\n",dlerror());return 1;}
  void(*f)(int)=dlsym(h,"str_dups");
  if(!f){fprintf(stderr,"%s\n",dlerror());return 2;}
  f(atoi(argv[2]));
  fflush(stdout);
  return 0;
}
"#,
        )
        .unwrap();
        let out = Command::new("gcc")
            .arg(&src)
            .arg("-ldl")
            .arg("-o")
            .arg(&runner)
            .output()
            .expect("compile runner");
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    }
    let out = Command::new(&runner)
        .arg(lib)
        .arg(num.to_string())
        .output()
        .expect("run");
    assert!(out.status.success());
    out.stdout
}

#[test]
fn str_dups_output_matches() {
    let c = c_lib_path();
    let r = rust_lib_path();
    for n in &[0, 1, 2, 5, 10, 100] {
        let cv = run_in_subprocess_capture(c.to_str().unwrap(), *n);
        let rv = run_in_subprocess_capture(r.to_str().unwrap(), *n);
        assert_eq!(cv, rv, "str_dups({}) output mismatch", n);
    }
    // Ensure the helper variable types are referenced.
    let _ = std::mem::size_of::<c_uchar>();
}

// ------------------------------------------------------------------
// stbds_rand_seed — does not produce output but lets us confirm the
// symbol is exported and callable.
// ------------------------------------------------------------------
type RandSeedFn = unsafe extern "C" fn(usize);

#[test]
fn rand_seed_callable() {
    let (c, r) = load_libs();
    unsafe {
        let cf: Symbol<RandSeedFn> = c.get(b"stbds_rand_seed").unwrap();
        let rf: Symbol<RandSeedFn> = r.get(b"stbds_rand_seed").unwrap();
        cf(0xdeadbeef);
        rf(0xdeadbeef);
    }
}
