//! Large-buffer differential tests for the code paths that are unreachable with
//! small inputs:
//!
//! * `find_value_in_buffer` returns `(int)((char*)result - buffer)`. When the
//!   match sits more than `INT_MAX` bytes into the buffer, that cast truncates.
//!   Reaching it needs a >2 GiB buffer.
//! * `create_numeric_buffer` computes `seed + i * 7`. `i * 7` itself only
//!   overflows `int` once `i > INT_MAX / 7 == 306783378`, i.e. a >292 MiB buffer.
//!
//! `harness = false`: these rows are memory-hungry and must not run
//! concurrently with each other. Each row degrades to a skip if the allocation
//! is refused, so the target is safe on small machines.

mod common;

use std::ffi::{c_char, c_int};

use common::apis;

const INT_MAX_USIZE: usize = i32::MAX as usize;

/// Anonymous, lazily-faulted mapping. Untouched pages stay backed by the shared
/// zero page, so a multi-gigabyte zeroed buffer costs almost no RSS.
struct Anon {
    ptr: *mut u8,
    len: usize,
}

unsafe extern "C" {
    fn mmap(
        addr: *mut core::ffi::c_void,
        len: usize,
        prot: c_int,
        flags: c_int,
        fd: c_int,
        off: i64,
    ) -> *mut core::ffi::c_void;
    fn munmap(addr: *mut core::ffi::c_void, len: usize) -> c_int;
}

const PROT_READ: c_int = 1;
const PROT_WRITE: c_int = 2;
const MAP_PRIVATE: c_int = 0x02;
const MAP_ANONYMOUS: c_int = 0x20;
const MAP_NORESERVE: c_int = 0x4000;

impl Anon {
    fn new(len: usize) -> Option<Anon> {
        let p = unsafe {
            mmap(
                std::ptr::null_mut(),
                len,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS | MAP_NORESERVE,
                -1,
                0,
            )
        };
        if p as isize == -1 {
            None
        } else {
            Some(Anon {
                ptr: p as *mut u8,
                len,
            })
        }
    }
}

impl Drop for Anon {
    fn drop(&mut self) {
        unsafe { munmap(self.ptr as *mut _, self.len) };
    }
}

fn main() {
    let mut failures: Vec<&'static str> = Vec::new();
    let report = |name: &'static str, ok: Result<(), String>, failures: &mut Vec<&'static str>| {
        match ok {
            Ok(()) => println!("test {name} ... ok"),
            Err(msg) => {
                println!("test {name} ... FAILED\n    {msg}");
                failures.push(name);
            }
        }
    };

    // -----------------------------------------------------------------------
    // find_value_in_buffer: offset > INT_MAX must truncate identically.
    // -----------------------------------------------------------------------
    let name = "large_row_01_e2_offset_past_int_max_truncates";
    let res = (|| -> Result<(), String> {
        let (c, r) = apis();
        // Just past INT_MAX so the `(int)` cast wraps to a negative value.
        let needle_at = INT_MAX_USIZE + 5; // 2147483652
        let len = needle_at + 64;
        let Some(buf) = Anon::new(len) else {
            println!("    (skipped: could not mmap {len} bytes)");
            return Ok(());
        };
        unsafe { *buf.ptr.add(needle_at) = 0x5A };
        let p = buf.ptr as *const c_char;

        let cv = unsafe { (c.find_value_in_buffer)(p, len, 0x5A) };
        let rv = unsafe { (r.find_value_in_buffer)(p, len, 0x5A) };
        if cv != rv {
            return Err(format!("offset {needle_at}: C = {cv} vs Rust = {rv}"));
        }
        let expected = needle_at as u32 as i32; // gcc truncates the ptrdiff
        if cv != expected {
            return Err(format!(
                "offset {needle_at}: expected truncated {expected}, C gave {cv}"
            ));
        }
        if cv >= 0 {
            return Err(format!("expected a negative truncated index, got {cv}"));
        }

        // Exactly INT_MAX still fits.
        unsafe { *buf.ptr.add(needle_at) = 0 };
        unsafe { *buf.ptr.add(INT_MAX_USIZE) = 0x5A };
        let cv = unsafe { (c.find_value_in_buffer)(p, len, 0x5A) };
        let rv = unsafe { (r.find_value_in_buffer)(p, len, 0x5A) };
        if cv != rv || cv != i32::MAX {
            return Err(format!("offset INT_MAX: C = {cv} vs Rust = {rv}"));
        }

        // One past INT_MAX wraps to INT_MIN.
        unsafe { *buf.ptr.add(INT_MAX_USIZE) = 0 };
        unsafe { *buf.ptr.add(INT_MAX_USIZE + 1) = 0x5A };
        let cv = unsafe { (c.find_value_in_buffer)(p, len, 0x5A) };
        let rv = unsafe { (r.find_value_in_buffer)(p, len, 0x5A) };
        if cv != rv || cv != i32::MIN {
            return Err(format!("offset INT_MAX+1: C = {cv} vs Rust = {rv}"));
        }

        // Absent needle over a >2 GiB range still yields the -1 sentinel.
        let cv = unsafe { (c.find_value_in_buffer)(p, len, 0x33) };
        let rv = unsafe { (r.find_value_in_buffer)(p, len, 0x33) };
        if cv != rv || cv != -1 {
            return Err(format!("absent over 2GiB: C = {cv} vs Rust = {rv}"));
        }
        Ok(())
    })();
    report(name, res, &mut failures);

    // -----------------------------------------------------------------------
    // create_numeric_buffer: `i * 7` overflows int for i > INT_MAX / 7.
    // -----------------------------------------------------------------------
    let name = "large_row_02_e4_index_times_seven_overflow";
    let res = (|| -> Result<(), String> {
        let (c, r) = apis();
        let overflow_at = (i32::MAX / 7) as usize + 1; // 306783379
        let size: c_int = (overflow_at + 4096) as c_int;
        let len = size as usize;
        let Some(cb) = Anon::new(len) else {
            println!("    (skipped: could not mmap {len} bytes)");
            return Ok(());
        };
        let Some(rb) = Anon::new(len) else {
            println!("    (skipped: could not mmap a second {len} bytes)");
            return Ok(());
        };
        for seed in [0i32, 1, -1, i32::MAX, i32::MIN, 12345] {
            unsafe { (c.create_numeric_buffer)(cb.ptr as *mut c_char, size, seed) };
            unsafe { (r.create_numeric_buffer)(rb.ptr as *mut c_char, size, seed) };
            let cs = unsafe { std::slice::from_raw_parts(cb.ptr, len) };
            let rs = unsafe { std::slice::from_raw_parts(rb.ptr, len) };
            if cs != rs {
                let at = cs.iter().zip(rs).position(|(a, b)| a != b).unwrap();
                return Err(format!(
                    "seed={seed}: first divergence at index {at}: C = {} vs Rust = {}",
                    cs[at], rs[at]
                ));
            }
            // Sanity: the overflowing region really is being written.
            let probe = overflow_at + 1;
            let _ = cs[probe];
        }
        Ok(())
    })();
    report(name, res, &mut failures);

    println!();
    if failures.is_empty() {
        println!("test result: ok. all large-buffer rows passed");
    } else {
        println!("failures:");
        for f in &failures {
            println!("    {f}");
        }
        println!("\ntest result: FAILED. {} failed", failures.len());
        std::process::exit(101);
    }
}
