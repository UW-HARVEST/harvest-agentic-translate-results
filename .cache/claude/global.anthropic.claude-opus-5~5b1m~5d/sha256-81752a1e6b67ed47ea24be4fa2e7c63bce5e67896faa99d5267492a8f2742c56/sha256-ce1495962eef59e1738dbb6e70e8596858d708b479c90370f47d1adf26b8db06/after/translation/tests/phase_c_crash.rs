//! Crash-mode / side-effect-ordering differential tests.
//!
//! Some divergences are invisible to a return-value comparison because the call
//! never returns, or because the difference is only in the ORDER of two side
//! effects. Each case here runs the payload in a CHILD PROCESS (this test binary
//! re-invoked with `CRASH_LIB`/`CRASH_CASE` set) so that a `SIGSEGV`/`SIGABRT` in
//! one implementation does not take the test runner down, and then compares the
//! child's termination signal, exit code and printed observables between C and
//! Rust.
//!
//! Covered:
//!  * `add_element` writes `arr->size` BEFORE the element store (GCC `-O0`
//!    orders it that way; see `objdump` of the C `.so` at `add_element+0x59`),
//!    which is observable when the element store faults.
//!  * a caller-supplied `DynamicArray*` or `data` pointer that is not
//!    4/8-byte-aligned — legal for the C, which just emits an unaligned `mov`.
//!  * `data == NULL` with `size < capacity` — the C faults on the store.

mod common;

use common::DynamicArray;
use std::os::unix::process::ExitStatusExt;
use std::process::Command;

/// What the child observed / how it died.
#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    signal: Option<i32>,
    code: Option<i32>,
    stdout: String,
}

fn run_child(which: &str, case: &str) -> Outcome {
    let exe = std::env::current_exe().expect("current_exe");
    let mut cmd = Command::new(exe);
    cmd.env("CRASH_LIB", which)
        .env("CRASH_CASE", case)
        // run only the payload harness test, single threaded, in the child
        .args(["--exact", "z_payload_entrypoint", "--nocapture", "--test-threads=1"]);
    if let Ok(v) = std::env::var("C_SO") {
        cmd.env("C_SO", v);
    }
    if let Ok(v) = std::env::var("RUST_SO") {
        cmd.env("RUST_SO", v);
    }
    let out = cmd.output().expect("failed to spawn child");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    // keep only our own marker lines, not libtest chatter
    let marked: String = stdout
        .lines()
        .filter(|l| l.starts_with("PAYLOAD:"))
        .collect::<Vec<_>>()
        .join("\n");
    Outcome {
        signal: out.status.signal(),
        code: out.status.code(),
        stdout: marked,
    }
}

/// Compare the two implementations on one payload case.
fn assert_same(case: &str) {
    let c = run_child("c", case);
    let r = run_child("rs", case);
    assert_eq!(
        c, r,
        "case `{case}`: C and Rust behaved differently in the child process\n\
         C   : {c:?}\n\
         Rust: {r:?}"
    );
}

#[test]
fn z1_add_element_store_faults_size_already_incremented() {
    // `data` points at a PROT_NONE page: the element store faults, but the C has
    // already written `arr->size = 1` by then.
    assert_same("fault_store");
}

/// The ORDER of `arr->size = old + 1` versus the element store, observed after
/// the element store has faulted.
///
/// The `DynamicArray` lives in a `MAP_SHARED` file mapping, so whatever the child
/// managed to write before dying is still visible to this parent process through
/// the page cache. GCC commits the new `size` first, so the C leaves `size == 1`
/// behind; a translation that stored the element first leaves `size == 0`.
#[test]
fn z5_add_element_side_effect_order_after_fault() {
    let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
    let mut results = Vec::new();
    for which in ["c", "rs"] {
        let path = format!("{dir}/dynarray_{}_{}.bin", std::process::id(), which);
        // 4096 zeroed bytes for the child to place the struct in
        std::fs::write(&path, vec![0u8; 4096]).expect("create shared file");

        let exe = std::env::current_exe().unwrap();
        let mut cmd = Command::new(exe);
        cmd.env("CRASH_LIB", which)
            .env("CRASH_CASE", "fault_store_shared")
            .env("CRASH_FILE", &path)
            .args(["--exact", "z_payload_entrypoint", "--nocapture", "--test-threads=1"]);
        if let Ok(v) = std::env::var("C_SO") {
            cmd.env("C_SO", v);
        }
        if let Ok(v) = std::env::var("RUST_SO") {
            cmd.env("RUST_SO", v);
        }
        let out = cmd.output().expect("spawn");

        let bytes = std::fs::read(&path).expect("read back shared file");
        let size = usize::from_ne_bytes(bytes[8..16].try_into().unwrap());
        let capacity = usize::from_ne_bytes(bytes[16..24].try_into().unwrap());
        let _ = std::fs::remove_file(&path);
        results.push((which, out.status.signal(), size, capacity));
    }
    let (_, c_sig, c_size, c_cap) = results[0];
    let (_, r_sig, r_size, r_cap) = results[1];

    assert_eq!(
        c_sig, r_sig,
        "termination signal differs: C={c_sig:?} Rust={r_sig:?}"
    );
    assert_eq!(
        c_sig,
        Some(11),
        "expected the C to die with SIGSEGV on the faulting element store"
    );
    assert_eq!(c_cap, r_cap, "capacity differs: C={c_cap} Rust={r_cap}");
    assert_eq!(
        c_size, r_size,
        "`arr->size` after the faulting element store differs: C={c_size}, Rust={r_size}. \
         The C commits `arr->size = old + 1` BEFORE storing the element \
         (`arr->data[arr->size++] = value` compiled by GCC), so the Rust must too."
    );
    assert_eq!(c_size, 1, "expected the C to have committed size=1 before faulting");
}

#[test]
fn z2_add_element_null_data_within_capacity() {
    // data == NULL, size(0) < capacity(4): the C dereferences NULL on the store.
    assert_same("null_data");
}

#[test]
fn z3_misaligned_struct_pointer() {
    // A `DynamicArray*` at an odd address. C emits plain unaligned `mov`s.
    assert_same("misaligned_arr_add");
    assert_same("misaligned_arr_expand");
    assert_same("misaligned_arr_free");
}

#[test]
fn z4_misaligned_data_pointer() {
    // `data` 4-byte-misaligned. C stores through it unaligned without complaint.
    assert_same("misaligned_data");
}

// ---------------------------------------------------------------------------
// The payload, executed in the child process.
// ---------------------------------------------------------------------------

extern "C" {
    fn mmap(
        addr: *mut std::ffi::c_void,
        len: usize,
        prot: i32,
        flags: i32,
        fd: i32,
        off: i64,
    ) -> *mut std::ffi::c_void;
}
const PROT_NONE: i32 = 0;
const PROT_READ: i32 = 0x1;
const PROT_WRITE: i32 = 0x2;
const MAP_SHARED: i32 = 0x01;
const MAP_PRIVATE: i32 = 0x02;
const MAP_ANONYMOUS: i32 = 0x20;

fn prot_none_page() -> *mut std::ffi::c_void {
    let page = unsafe {
        mmap(
            std::ptr::null_mut(),
            4096,
            PROT_NONE,
            MAP_PRIVATE | MAP_ANONYMOUS,
            -1,
            0,
        )
    };
    assert!(!page.is_null() && page as isize != -1, "mmap PROT_NONE failed");
    page
}

#[test]
fn z_payload_entrypoint() {
    let (which, case) = match (std::env::var("CRASH_LIB"), std::env::var("CRASH_CASE")) {
        (Ok(w), Ok(c)) => (w, c),
        _ => return, // normal run: this test does nothing
    };

    let p = common::load();
    let imp: &common::Impl = if which == "c" { &p.c } else { &p.rs };

    unsafe {
        match case.as_str() {
            "fault_store_shared" => {
                // Put the DynamicArray in a MAP_SHARED file mapping so that the
                // parent can inspect `size`/`capacity` after we die.
                use std::os::unix::io::AsRawFd;
                let path = std::env::var("CRASH_FILE").expect("CRASH_FILE");
                let f = std::fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(&path)
                    .expect("open shared file");
                let base = mmap(
                    std::ptr::null_mut(),
                    4096,
                    PROT_READ | PROT_WRITE,
                    MAP_SHARED,
                    f.as_raw_fd(),
                    0,
                );
                assert!(!base.is_null() && base as isize != -1, "mmap MAP_SHARED failed");
                let arr = base as *mut DynamicArray;
                std::ptr::write(
                    arr,
                    DynamicArray {
                        data: prot_none_page() as *mut std::ffi::c_int,
                        size: 0,
                        capacity: 4,
                    },
                );
                println!("PAYLOAD: shared struct at {arr:p}, storing into PROT_NONE page");
                flush();
                let rc = imp.add_element(arr, 0x1234);
                let h = std::ptr::read(arr);
                println!("PAYLOAD: survived rc={rc} size={} cap={}", h.size, h.capacity);
            }
            "fault_store" => {
                let mut a = DynamicArray {
                    data: prot_none_page() as *mut std::ffi::c_int,
                    size: 0,
                    capacity: 4,
                };
                println!("PAYLOAD: about to store into PROT_NONE page");
                flush();
                let rc = imp.add_element(&mut a as *mut DynamicArray, 0x1234);
                // Only reached if the store did NOT fault.
                println!("PAYLOAD: survived rc={rc} size={} cap={}", a.size, a.capacity);
            }
            "null_data" => {
                let mut a = DynamicArray {
                    data: std::ptr::null_mut(),
                    size: 0,
                    capacity: 4,
                };
                println!("PAYLOAD: about to store through NULL data");
                flush();
                let rc = imp.add_element(&mut a as *mut DynamicArray, 0x1234);
                println!("PAYLOAD: survived rc={rc} size={} cap={}", a.size, a.capacity);
            }
            "misaligned_arr_add" | "misaligned_arr_expand" | "misaligned_arr_free" => {
                // a DynamicArray at an ODD address
                let raw = common::libc_malloc(64) as *mut u8;
                let arr = raw.add(1) as *mut DynamicArray;
                let data = common::libc_malloc(4 * 4) as *mut std::ffi::c_int;
                std::ptr::write_unaligned(
                    arr,
                    DynamicArray { data, size: 1, capacity: 4 },
                );
                std::ptr::write(data, 0x55);
                println!("PAYLOAD: misaligned arr = {:p}", arr);
                flush();
                let rc = match case.as_str() {
                    "misaligned_arr_add" => imp.add_element(arr, 0x99),
                    "misaligned_arr_expand" => imp.expand_array(arr),
                    _ => {
                        imp.free_array(arr);
                        -1
                    }
                };
                let h = std::ptr::read_unaligned(arr);
                if case == "misaligned_arr_free" {
                    println!("PAYLOAD: survived free_array on misaligned arr");
                } else {
                    println!(
                        "PAYLOAD: survived rc={rc} size={} cap={} first={}",
                        h.size,
                        h.capacity,
                        if h.data.is_null() { -1 } else { *h.data }
                    );
                }
            }
            "misaligned_data" => {
                // `data` 4-byte-MISALIGNED, size < capacity so no realloc happens
                let raw = common::libc_malloc(64) as *mut u8;
                let data = raw.add(1) as *mut std::ffi::c_int;
                let mut a = DynamicArray { data, size: 0, capacity: 8 };
                println!("PAYLOAD: misaligned data = {:p}", data);
                flush();
                let rc = imp.add_element(&mut a as *mut DynamicArray, 0x11223344);
                let stored = std::ptr::read_unaligned(data);
                println!(
                    "PAYLOAD: survived rc={rc} size={} cap={} stored={stored:#x}",
                    a.size, a.capacity
                );
            }
            other => panic!("unknown payload case `{other}`"),
        }
    }
    flush();
}

fn flush() {
    use std::io::Write;
    let _ = std::io::stdout().flush();
}
