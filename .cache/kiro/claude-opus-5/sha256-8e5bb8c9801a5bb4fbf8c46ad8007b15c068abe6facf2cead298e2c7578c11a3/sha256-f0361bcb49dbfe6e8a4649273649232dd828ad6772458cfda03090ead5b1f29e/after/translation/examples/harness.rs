//! Differential test harness.
//!
//! Runs in its own process so that (a) stdout can be captured as a raw byte
//! stream, (b) a `SIGSEGV` from the library's unchecked dereference can be
//! observed as a wait status instead of killing the test runner, and (c) the
//! stack state seen by `bad()`'s uninitialized read is controlled.
//!
//! Usage: `harness <path-to-libdriver.so> <op> [<op> ...]`
//!
//! The library is always loaded through `libloading` and every function is
//! called through a `dlsym`-resolved symbol, exactly as an external consumer
//! would. Rust functions are never called directly.
//!
//! Ops:
//!   p:<i32>            printIntPtrLine(&stack_int)
//!   ph:<i32>           printIntPtrLine(&heap_int)
//!   ps:<i32>           printIntPtrLine(&static_int)
//!   pm:<off>:<u32hex>  printIntPtrLine((int*)(bytes + off))  -- misaligned
//!   pnull              printIntPtrLine(NULL)
//!   praw:<u64hex>      printIntPtrLine((int*)addr)
//!   ppast              printIntPtrLine(one-past-end of a heap int array)
//!   good               good()
//!   bad                bad()
//!   pbad               poison stack with an index table, then bad()
//!   d:<i32>            driver(v)
//!   pd:<i32>           poison stack with an index table, then driver(v)
//!   draw:<u64hex>      driver called with a full 64-bit value in the arg reg
//!   linebuf            switch stdout to line buffering
//!   flush              fflush(stdout)

use std::ffi::{OsStr, c_int};

/// Number of 8-byte slots the stack poisoner fills (4 KiB).
const POISON_SLOTS: usize = 512;

type FnPrint = unsafe extern "C" fn(*const c_int);
type FnVoid = unsafe extern "C" fn();
type FnDriver = unsafe extern "C" fn(c_int);
type FnDriverRaw = unsafe extern "C" fn(u64);

static mut STATIC_INT: c_int = 0;

/// Fill `POISON_SLOTS` 8-byte slots of *dead* stack immediately below the
/// current `rsp` with pointers into `table`, where `table[j] == j`: the slot at
/// `rsp - 8*(j+1)` receives `&table[j]`.
///
/// Any call made right afterwards overlays its frame on this region, so an
/// uninitialized 8-byte read inside that frame yields a valid, readable pointer
/// whose pointee reveals *which slot* was read. That converts `bad()`'s
/// undefined read into a deterministic, comparable observation: if the C and
/// Rust `bad()` read the same address they print the same index, and if they
/// read different addresses they print different indices.
///
/// Writing below `rsp` is deliberate — that is precisely the memory under test.
/// Nothing runs in between, and the harness installs no signal handlers.
#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn poison_stack(table: &'static [c_int]) {
    let base = table.as_ptr();
    unsafe {
        std::arch::asm!(
            "mov r9, rsp",
            "xor r10, r10",
            "2:",
            "lea r8, [{tbl} + r10*4]",
            "sub r9, 8",
            "mov [r9], r8",
            "inc r10",
            "cmp r10, {n}",
            "jb 2b",
            tbl = in(reg) base,
            n = const POISON_SLOTS,
            out("r8") _,
            out("r9") _,
            out("r10") _,
            options(nostack),
        );
    }
}

#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
fn poison_stack(table: &'static [c_int]) {
    let _ = table;
}

/// Poison the dead stack, then immediately invoke `f` so that `f`'s frame lands
/// on top of the poison.
#[inline(never)]
fn poisoned_call(table: &'static [c_int], f: FnVoid) {
    poison_stack(table);
    unsafe { f() };
}

#[inline(never)]
fn poisoned_call_driver(table: &'static [c_int], f: FnDriver, v: c_int) {
    poison_stack(table);
    unsafe { f(v) };
}

fn main() {
    let mut args = std::env::args_os();
    let _exe = args.next();
    let lib_path = args.next().expect("usage: harness <lib.so> <op>...");
    let ops: Vec<String> = args
        .map(|a| a.to_string_lossy().into_owned())
        .collect::<Vec<_>>();

    // `table[j] == j`, leaked so the pointers stay valid for the whole run.
    let table: &'static [c_int] = Box::leak(
        (0..POISON_SLOTS as c_int)
            .collect::<Vec<c_int>>()
            .into_boxed_slice(),
    );

    let lib = unsafe { libloading::Library::new(OsStr::new(&lib_path)) }
        .unwrap_or_else(|e| panic!("dlopen {lib_path:?}: {e}"));

    // Resolve every exported symbol up front through dlsym.
    let print_int_ptr_line: FnPrint =
        *unsafe { lib.get::<FnPrint>(b"printIntPtrLine\0") }.expect("printIntPtrLine");
    let bad: FnVoid = *unsafe { lib.get::<FnVoid>(b"bad\0") }.expect("bad");
    let good: FnVoid = *unsafe { lib.get::<FnVoid>(b"good\0") }.expect("good");
    let driver: FnDriver = *unsafe { lib.get::<FnDriver>(b"driver\0") }.expect("driver");
    let driver_raw: FnDriverRaw = unsafe { std::mem::transmute::<FnDriver, FnDriverRaw>(driver) };

    // A mapped byte buffer used for misaligned reads; 16 bytes of slack past the
    // largest offset used so the 4-byte read always stays in bounds.
    let mut bytes = [0u8; 32];
    let _ = &bytes;

    for op in &ops {
        let mut parts = op.split(':');
        let kind = parts.next().unwrap_or("");
        match kind {
            "p" => {
                let v: c_int = parts.next().unwrap().parse().unwrap();
                let stack_int: c_int = v;
                unsafe { print_int_ptr_line(&raw const stack_int) };
            }
            "ph" => {
                let v: c_int = parts.next().unwrap().parse().unwrap();
                let boxed = Box::new(v);
                unsafe { print_int_ptr_line(&raw const *boxed) };
            }
            "ps" => {
                let v: c_int = parts.next().unwrap().parse().unwrap();
                unsafe {
                    STATIC_INT = v;
                    print_int_ptr_line(&raw const STATIC_INT);
                }
            }
            "pm" => {
                let off: usize = parts.next().unwrap().parse().unwrap();
                let pat = u32::from_str_radix(parts.next().unwrap(), 16).unwrap();
                bytes = [0u8; 32];
                bytes[off..off + 4].copy_from_slice(&pat.to_le_bytes());
                let p = unsafe { bytes.as_ptr().add(off) } as *const c_int;
                unsafe { print_int_ptr_line(p) };
            }
            "pnull" => unsafe { print_int_ptr_line(std::ptr::null()) },
            "praw" => {
                let addr = u64::from_str_radix(parts.next().unwrap(), 16).unwrap();
                unsafe { print_int_ptr_line(addr as *const c_int) };
            }
            "ppast" => {
                // One-past-the-end of a 4-element heap array: dangling but, in
                // practice, still inside the same mapped malloc chunk.
                let v: Vec<c_int> = vec![11, 22, 33, 44];
                let p = unsafe { v.as_ptr().add(4) };
                unsafe { print_int_ptr_line(p) };
            }
            "good" => unsafe { good() },
            "bad" => unsafe { bad() },
            "pbad" => poisoned_call(table, bad),
            "d" => {
                let v: c_int = parts.next().unwrap().parse().unwrap();
                unsafe { driver(v) };
            }
            "pd" => {
                let v: c_int = parts.next().unwrap().parse().unwrap();
                poisoned_call_driver(table, driver, v);
            }
            "draw" => {
                let v = u64::from_str_radix(parts.next().unwrap(), 16).unwrap();
                unsafe { driver_raw(v) };
            }
            "linebuf" => unsafe {
                libc::setvbuf(
                    libc::fdopen(1, c"w".as_ptr()),
                    std::ptr::null_mut(),
                    libc::_IOLBF,
                    0,
                );
            },
            "flush" => unsafe {
                libc::fflush(std::ptr::null_mut());
            },
            other => panic!("unknown op {other:?}"),
        }
    }

    // Let libc's atexit flush stdout, exactly as the C library's own consumers
    // would; do not use Rust's stdout at all.
    unsafe { libc::fflush(std::ptr::null_mut()) };
}
