// Differential test runner.
//
// Loads a shared library (either the C `libdriver.so` or the Rust
// `libdriver.so`) with `libloading` and invokes its exported C symbols. Every
// call goes through the dynamic symbol table, exactly as an external consumer
// would do it -- Rust functions are never called directly, so the
// `#[no_mangle] extern "C"` wrappers are what gets exercised.
//
// This lives in a separate *process* on purpose: several rows of ERRORS.md make
// the library dereference an invalid pointer, which kills the process with
// SIGSEGV. The parent test compares (stdout bytes, exit code, signal), so a
// crash is a comparable observation instead of a lost test run.
//
// usage: runner <libpath> <op> [args...]

use std::ffi::{c_int, c_void};

type FnVoid = unsafe extern "C" fn();
type FnInt = unsafe extern "C" fn(c_int);
// Deliberately wrong-width view of `driver` used to put a "dirty" 64-bit value
// in the argument register: the C ABI says an `int` parameter is the low 32
// bits, and the callee must ignore the upper half.
type FnI64 = unsafe extern "C" fn(i64);
type FnPtr = unsafe extern "C" fn(*const c_int);

/// Fixed-seed SplitMix64 so the C run and the Rust run see identical inputs and
/// results are reproducible across machines.
struct Rng(u64);
impl Rng {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
}

// Writable static storage (.data) and read-only static storage (.rodata),
// used to prove `printIntPtrLine` reads 4 bytes at whatever address it is
// given, independent of the pointer's provenance.
static mut STATIC_SLOT: c_int = 0;
static RODATA_SLOT: c_int = 1_234_567;

fn parse(s: &str) -> i64 {
    let s = s.trim();
    if let Some(h) = s.strip_prefix("0x") {
        u64::from_str_radix(h, 16).expect("bad hex") as i64
    } else {
        s.parse::<i64>().expect("bad int")
    }
}

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    if argv.len() < 3 {
        eprintln!("usage: runner <libpath> <op> [args...]");
        std::process::exit(2);
    }
    let libpath = &argv[1];
    let op = argv[2].as_str();
    let rest = &argv[3..];

    let lib = unsafe { libloading::Library::new(libpath) }
        .unwrap_or_else(|e| panic!("failed to dlopen {libpath}: {e}"));

    // Resolve all four exported symbols up front: this alone asserts that the
    // library under test really exports the full C surface.
    let print_int_ptr_line: libloading::Symbol<FnPtr> =
        unsafe { lib.get(b"printIntPtrLine\0") }.expect("printIntPtrLine missing");
    let good: libloading::Symbol<FnVoid> = unsafe { lib.get(b"good\0") }.expect("good missing");
    let bad: libloading::Symbol<FnVoid> = unsafe { lib.get(b"bad\0") }.expect("bad missing");
    let driver: libloading::Symbol<FnInt> =
        unsafe { lib.get(b"driver\0") }.expect("driver missing");

    match op {
        // ---- symbol resolution only ---------------------------------------
        "symbols" => {
            println!("printIntPtrLine good bad driver");
        }

        // ---- printIntPtrLine, stack local ---------------------------------
        // CONFIGS rows 1-8, 18
        "print" => {
            for a in rest {
                let v: c_int = parse(a) as i32;
                unsafe { print_int_ptr_line(&v as *const c_int) };
            }
        }

        // ---- printIntPtrLine, heap storage (CONFIGS row 9) ----------------
        "print_heap" => {
            for a in rest {
                let v: c_int = parse(a) as i32;
                let p = unsafe { libc::malloc(std::mem::size_of::<c_int>()) } as *mut c_int;
                assert!(!p.is_null());
                unsafe {
                    *p = v;
                    print_int_ptr_line(p as *const c_int);
                    libc::free(p as *mut c_void);
                }
            }
        }

        // ---- printIntPtrLine, writable static storage (CONFIGS row 10) ----
        "print_static" => {
            for a in rest {
                let v: c_int = parse(a) as i32;
                unsafe {
                    let p = &raw mut STATIC_SLOT;
                    *p = v;
                    print_int_ptr_line(p as *const c_int);
                }
            }
        }

        // ---- printIntPtrLine, read-only static storage (CONFIGS row 11) ---
        "print_rodata" => {
            unsafe { print_int_ptr_line(&raw const RODATA_SLOT) };
        }

        // ---- printIntPtrLine over an array (CONFIGS rows 12-15) -----------
        // args: <mode: first|mid|last|all> <values...>
        "print_array" => {
            let mode = rest[0].as_str();
            let arr: Vec<c_int> = rest[1..].iter().map(|a| parse(a) as i32).collect();
            assert!(!arr.is_empty());
            let idxs: Vec<usize> = match mode {
                "first" => vec![0],
                "mid" => vec![arr.len() / 2],
                "last" => vec![arr.len() - 1],
                "all" => (0..arr.len()).collect(),
                other => panic!("bad array mode {other}"),
            };
            for i in idxs {
                // pointer arithmetic on the array, not a copy
                unsafe { print_int_ptr_line(arr.as_ptr().add(i)) };
            }
        }

        // ---- printIntPtrLine at a misaligned address ----------------------
        // CONFIGS row 16 / ERRORS row 4. args: <byte values...> (>=5 bytes)
        "print_misaligned" => {
            let bytes: Vec<u8> = rest.iter().map(|a| parse(a) as u8).collect();
            assert!(bytes.len() >= 5);
            let p = unsafe { bytes.as_ptr().add(1) } as *const c_int;
            unsafe { print_int_ptr_line(p) };
        }

        // ---- printIntPtrLine at the last valid 4 bytes of a mapping -------
        // CONFIGS row 17 / ERRORS row 5. args: <value>
        "print_page_end" => {
            let v: c_int = parse(&rest[0]) as i32;
            let (base, page) = map_two_pages_unmap_second();
            let p = unsafe { base.add(page - std::mem::size_of::<c_int>()) } as *mut c_int;
            unsafe {
                *p = v;
                print_int_ptr_line(p as *const c_int);
            }
        }

        // ---- printIntPtrLine one byte past a mapping (ERRORS row 6) -------
        "print_past_end" => {
            let (base, page) = map_two_pages_unmap_second();
            // 4-byte read starting at the first unmapped byte
            let p = unsafe { base.add(page) } as *const c_int;
            unsafe { print_int_ptr_line(p) };
        }

        // ---- printIntPtrLine at a raw address (ERRORS rows 1-3) -----------
        // args: <address, e.g. 0 or 0x1 or 0xdeadbeefdeadbee0>
        "print_raw_ptr" => {
            let addr = parse(&rest[0]) as u64;
            unsafe { print_int_ptr_line(addr as usize as *const c_int) };
        }

        // ---- good (CONFIGS rows 19-20 / ERRORS row 11) --------------------
        // args: [repeat count, default 1]
        "good" => {
            let n = if rest.is_empty() { 1 } else { parse(&rest[0]) };
            for _ in 0..n {
                unsafe { good() };
            }
        }

        // ---- bad: the CWE-457 defect (CONFIGS row 31 / ERRORS row 7) ------
        "bad" => {
            unsafe { bad() };
        }

        // ---- driver (CONFIGS rows 21-30 / ERRORS rows 8-9) ----------------
        // args: <useGood values...>
        "driver" => {
            for a in rest {
                unsafe { driver(parse(a) as i32) };
            }
        }

        // ---- driver called through an i64-typed signature -----------------
        // ERRORS row 10: dirty high 32 bits in the argument register.
        "driver_dirty" => {
            let driver64: libloading::Symbol<FnI64> =
                unsafe { lib.get(b"driver\0") }.expect("driver missing");
            for a in rest {
                unsafe { driver64(parse(a)) };
            }
        }

        // ---- composed pipeline (CONFIGS row 32) ---------------------------
        // args: <seed> <steps>. Interleaves all three levels of the call
        // hierarchy in one process so buffering/ordering of the composed
        // sequence is observable.
        "pipeline" => {
            let seed = parse(&rest[0]) as u64;
            let steps = parse(&rest[1]);
            let mut rng = Rng(seed);
            for _ in 0..steps {
                let choice = rng.next_u64() % 4;
                let v = rng.next_i32();
                match choice {
                    0 => unsafe { driver(1) },
                    1 => unsafe { good() },
                    2 => {
                        let x: c_int = v;
                        unsafe { print_int_ptr_line(&x as *const c_int) };
                    }
                    _ => {
                        // non-zero useGood derived from the RNG: still the good path
                        let nz = if v == 0 { 1 } else { v };
                        unsafe { driver(nz) };
                    }
                }
            }
        }

        // ---- large randomized burst (CONFIGS row 18) ----------------------
        // args: <seed> <count>. Generates values in-process (identical seed =>
        // identical values for the C and the Rust run) and crosses the stdio
        // BUFSIZ boundary many times over.
        "print_burst" => {
            let seed = parse(&rest[0]) as u64;
            let count = parse(&rest[1]);
            let mut rng = Rng(seed);
            for _ in 0..count {
                let v: c_int = rng.next_i32();
                unsafe { print_int_ptr_line(&v as *const c_int) };
            }
        }

        other => {
            eprintln!("unknown op {other}");
            std::process::exit(2);
        }
    }

    // Flush libc's stdout buffer before exiting. The library under test uses
    // libc printf; Rust's process exit does not necessarily flush it, and
    // an unflushed buffer would hide output differences.
    unsafe { libc::fflush(std::ptr::null_mut()) };
}

/// Map two pages, then unmap the second one so the first is immediately
/// followed by an unmapped hole. Returns (base, page_size).
fn map_two_pages_unmap_second() -> (*mut u8, usize) {
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize;
    let base = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            page * 2,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        )
    };
    assert_ne!(base, libc::MAP_FAILED, "mmap failed");
    let base = base as *mut u8;
    let rc = unsafe { libc::munmap(base.add(page) as *mut c_void, page) };
    assert_eq!(rc, 0, "munmap failed");
    (base, page)
}
