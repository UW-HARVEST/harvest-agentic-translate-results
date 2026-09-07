// Differential tests: load BOTH the C `.so` and the Rust `.so` with `libloading`
// and compare their observable output (stdout) byte-for-byte.
//
// No Rust function is ever called directly — every call goes through the
// `.so`'s exported symbol, so the `#[no_mangle]` / `extern "C"` wrappers are
// under test too.
//
// Phase B rows live in `CONFIGS.md`, Phase C rows in `ERRORS.md`.

use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// libc bits needed to capture whatever the loaded libraries write to fd 1.
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
}

const O_RDWR: c_int = 0o2;
const O_CREAT: c_int = 0o100;
const O_TRUNC: c_int = 0o1000;

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

type DriverFn = unsafe extern "C" fn(u32, u32, u8, i32);
type PrintFooFn = unsafe extern "C" fn(*const u8);

struct Impl {
    #[allow(dead_code)]
    lib: Library,
    driver: DriverFn,
    print_foo: PrintFooFn,
}

impl Impl {
    fn load(path: &PathBuf) -> Impl {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
        // SAFETY: transmuting the raw symbol addresses to fn pointers detaches
        // them from the `Symbol` borrow; `lib` is kept alive in the same struct
        // (and the struct lives in a `'static` OnceLock), so they stay valid.
        let driver: DriverFn = unsafe {
            let s: Symbol<DriverFn> = lib
                .get(b"driver\0")
                .unwrap_or_else(|e| panic!("no `driver` in {}: {e}", path.display()));
            *s
        };
        let print_foo: PrintFooFn = unsafe {
            let s: Symbol<PrintFooFn> = lib
                .get(b"print_foo\0")
                .unwrap_or_else(|e| panic!("no `print_foo` in {}: {e}", path.display()));
            *s
        };
        Impl {
            lib,
            driver,
            print_foo,
        }
    }
}

struct Pair {
    c: Impl,
    r: Impl,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_C_SO") {
        return PathBuf::from(p);
    }
    let p = manifest_dir().join("../c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {}. Build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    let base = manifest_dir().join("target");
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libdriver.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "Rust shared library not found under {}. Build it with:\n  \
         cd translation && cargo build --release",
        base.display()
    );
}

fn pair() -> &'static Pair {
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| Pair {
        c: Impl::load(&c_so_path()),
        r: Impl::load(&rust_so_path()),
    })
}

// ---------------------------------------------------------------------------
// stdout capture
//
// The libraries print with libc `printf`, which writes to the process-wide
// glibc `stdout`. Redirect fd 1 to a scratch file, run the closure, flush all
// streams, restore fd 1, and read the bytes back. A global mutex serialises
// this because fd 1 is process-global while the test harness is threaded.
// ---------------------------------------------------------------------------

fn capture_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "driver_diff_{}_{:p}.out",
        std::process::id(),
        &dir as *const _
    ));
    let c_path = std::ffi::CString::new(path.to_str().unwrap()).unwrap();

    unsafe {
        // Drain anything already buffered so it is not misattributed. Both
        // libc's `stdout` and Rust's `std::io::stdout()` (which the test
        // harness itself writes progress through) must be flushed, or their
        // pending bytes would land in the capture file.
        let _ = std::io::Write::flush(&mut std::io::stdout());
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        let fd = open(c_path.as_ptr(), O_RDWR | O_CREAT | O_TRUNC, 0o600 as c_int);
        assert!(fd >= 0, "open({}) failed", path.display());
        assert!(dup2(fd, 1) >= 0, "dup2 failed");

        f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
        close(fd);
    }

    let bytes = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    bytes
}

/// Run the same closure against the C impl and the Rust impl and assert the
/// captured stdout is byte-identical.
fn assert_same<F>(label: &str, f: F)
where
    F: Fn(&Impl),
{
    let c_out = capture(|| f(&pair().c));
    let r_out = capture(|| f(&pair().r));
    if c_out != r_out {
        panic!(
            "{label}: stdout differs\n  C   ({} bytes): {:?}\n  Rust({} bytes): {:?}",
            c_out.len(),
            String::from_utf8_lossy(&c_out),
            r_out.len(),
            String::from_utf8_lossy(&r_out),
        );
    }
    assert!(!c_out.is_empty(), "{label}: expected some output");
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64, fixed seed) — property-style inputs that are
// reproducible run to run.
// ---------------------------------------------------------------------------

const SEED: u64 = 0x243F_6A88_85A3_08D3;

struct Rng(u64);

impl Rng {
    fn new() -> Rng {
        Rng(SEED)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn u32(&mut self) -> u32 {
        self.next_u64() as u32
    }
    fn u8(&mut self) -> u8 {
        self.next_u64() as u8
    }
    fn i32(&mut self) -> i32 {
        self.next_u64() as i32
    }
}

/// `int` boundary values: zero, ±1, and one step inside/at each end.
const Z_BOUNDARIES: [i32; 7] = [
    0,
    1,
    -1,
    i32::MIN,
    i32::MAX,
    i32::MIN.wrapping_add(1),
    i32::MAX - 1,
];

/// Build the 8-byte `foo_t` image, matching the layout gcc produces on
/// x86-64 (verified: sizeof 8, alignof 4, `z` at offset 4; bit-fields packed
/// into byte 0 as x=bits0..1, y=bits2..4, b=bit5).
fn foo_image(byte0: u8, pad: [u8; 3], z: i32) -> [u8; 8] {
    let mut img = [0u8; 8];
    img[0] = byte0;
    img[1..4].copy_from_slice(&pad);
    img[4..8].copy_from_slice(&z.to_ne_bytes());
    img
}

fn packed_byte0(x: u32, y: u32, b: u8) -> u8 {
    ((x as u8) & 0x03) | (((y as u8) & 0x07) << 2) | ((b & 0x01) << 5)
}

// ===========================================================================
// Phase B — valid-path differential tests (CONFIGS.md rows C1..C12)
// ===========================================================================

/// C1: every in-range `(x, y, b)` combination, `z = 0`. Exhaustive (64).
fn cfg_c1_driver_inrange_exhaustive() {
    for x in 0u32..4 {
        for y in 0u32..8 {
            for b in 0u8..2 {
                assert_same(&format!("C1 x={x} y={y} b={b}"), |i| unsafe {
                    (i.driver)(x, y, b, 0)
                });
            }
        }
    }
}

/// C2: in-range `x`, `y`, `b` crossed with every `int` boundary `z` (448).
fn cfg_c2_driver_z_boundaries() {
    for &z in &Z_BOUNDARIES {
        for x in 0u32..4 {
            for y in 0u32..8 {
                for b in 0u8..2 {
                    assert_same(&format!("C2 x={x} y={y} b={b} z={z}"), |i| unsafe {
                        (i.driver)(x, y, b, z)
                    });
                }
            }
        }
    }
}

/// C3: in-range `x`, `y`, `b`; `z` randomized over the full `i32` range (2048).
fn cfg_c3_driver_z_random() {
    let mut rng = Rng::new();
    for n in 0..2048 {
        let x = rng.u32() % 4;
        let y = rng.u32() % 8;
        let b = (rng.u8() & 1) as u8;
        let z = rng.i32();
        assert_same(&format!("C3 #{n} x={x} y={y} b={b} z={z}"), |i| unsafe {
            (i.driver)(x, y, b, z)
        });
    }
}

/// C4: `x` randomized over the full `u32` range (mostly out of the 2-bit
/// range, exercising truncation) with the other arguments randomized (2048).
fn cfg_c4_driver_x_random_full_u32() {
    let mut rng = Rng::new();
    for n in 0..2048 {
        let x = rng.u32();
        let y = rng.u32() % 8;
        let b = (rng.u8() & 1) as u8;
        let z = rng.i32();
        assert_same(&format!("C4 #{n} x={x} y={y} b={b} z={z}"), |i| unsafe {
            (i.driver)(x, y, b, z)
        });
    }
}

/// C5: `y` randomized over the full `u32` range (truncation) (2048).
fn cfg_c5_driver_y_random_full_u32() {
    let mut rng = Rng::new();
    for n in 0..2048 {
        let x = rng.u32() % 4;
        let y = rng.u32();
        let b = (rng.u8() & 1) as u8;
        let z = rng.i32();
        assert_same(&format!("C5 #{n} x={x} y={y} b={b} z={z}"), |i| unsafe {
            (i.driver)(x, y, b, z)
        });
    }
}

/// C6: every one of the 256 `b` byte values, crossed with in-range `x`/`y`
/// and a boundary `z`. Exhaustive over the `_Bool` ABI byte.
fn cfg_c6_driver_b_all_bytes() {
    let mut rng = Rng::new();
    for b in 0u16..256 {
        let b = b as u8;
        for x in 0u32..4 {
            let y = rng.u32() % 8;
            let z = Z_BOUNDARIES[(b as usize) % Z_BOUNDARIES.len()];
            assert_same(&format!("C6 x={x} y={y} b={b} z={z}"), |i| unsafe {
                (i.driver)(x, y, b, z)
            });
        }
    }
}

/// C7: all four arguments randomized over their full ABI ranges at once (4096).
fn cfg_c7_driver_all_random() {
    let mut rng = Rng::new();
    for n in 0..4096 {
        let x = rng.u32();
        let y = rng.u32();
        let b = rng.u8();
        let z = rng.i32();
        assert_same(&format!("C7 #{n} x={x} y={y} b={b} z={z}"), |i| unsafe {
            (i.driver)(x, y, b, z)
        });
    }
}

/// C8: lowest-level entry point. Byte 0 of the struct image takes every value
/// `0..=255` — covering all `x`/`y`/`b` bit patterns and both padding bits.
fn cfg_c8_print_foo_byte0_exhaustive() {
    for byte0 in 0u16..256 {
        let img = foo_image(byte0 as u8, [0, 0, 0], 0);
        assert_same(&format!("C8 byte0={byte0:#04x}"), |i| unsafe {
            (i.print_foo)(img.as_ptr())
        });
    }
}

/// C9: lowest-level entry point driven with a fully arbitrary struct image:
/// random bit-field byte, random padding garbage, random `z` (4096).
fn cfg_c9_print_foo_image_random() {
    let mut rng = Rng::new();
    for n in 0..4096 {
        let byte0 = rng.u8();
        let pad = [rng.u8(), rng.u8(), rng.u8()];
        let z = rng.i32();
        let img = foo_image(byte0, pad, z);
        assert_same(
            &format!("C9 #{n} byte0={byte0:#04x} pad={pad:?} z={z}"),
            |i| unsafe { (i.print_foo)(img.as_ptr()) },
        );
    }
}

/// C10: `print_foo` with every `int` boundary `z`, crossed with 64 random
/// bit-field bytes.
fn cfg_c10_print_foo_z_boundaries() {
    let mut rng = Rng::new();
    for &z in &Z_BOUNDARIES {
        for _ in 0..64 {
            let byte0 = rng.u8();
            let pad = [rng.u8(), rng.u8(), rng.u8()];
            let img = foo_image(byte0, pad, z);
            assert_same(&format!("C10 byte0={byte0:#04x} z={z}"), |i| unsafe {
                (i.print_foo)(img.as_ptr())
            });
        }
    }
}

/// C11: the composed pipeline. `driver` encodes into a `foo_t` and calls
/// `print_foo`, which decodes it. Assert (a) C and Rust agree on `driver`,
/// (b) C and Rust agree on `print_foo` given the image `driver` builds, and
/// (c) the encode path and the decode path agree with each other — a bug in
/// either half alone would be invisible to a per-function test.
fn cfg_c11_encode_decode_roundtrip() {
    let mut rng = Rng::new();
    for n in 0..2048 {
        let x = rng.u32();
        let y = rng.u32();
        let b = rng.u8();
        let z = rng.i32();
        let img = foo_image(packed_byte0(x, y, b), [0, 0, 0], z);

        let via_driver_c = capture(|| unsafe { (pair().c.driver)(x, y, b, z) });
        let via_driver_r = capture(|| unsafe { (pair().r.driver)(x, y, b, z) });
        let via_print_c = capture(|| unsafe { (pair().c.print_foo)(img.as_ptr()) });
        let via_print_r = capture(|| unsafe { (pair().r.print_foo)(img.as_ptr()) });

        assert_eq!(
            via_driver_c, via_driver_r,
            "C11 #{n} driver mismatch for x={x} y={y} b={b} z={z}"
        );
        assert_eq!(
            via_print_c, via_print_r,
            "C11 #{n} print_foo mismatch for x={x} y={y} b={b} z={z}"
        );
        assert_eq!(
            via_driver_c, via_print_c,
            "C11 #{n} encode/decode round trip mismatch for x={x} y={y} b={b} z={z}"
        );
    }
}

/// C12: many interleaved calls to both entry points inside a single capture,
/// i.e. a single stdout flush covering 512 calls. Catches any divergence in
/// buffering or per-call state.
fn cfg_c12_interleaved_multicall_stream() {
    for round in 0..16u64 {
        let mut inputs = Vec::with_capacity(512);
        let mut rng = Rng(SEED ^ round.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        for _ in 0..512 {
            inputs.push((rng.u32(), rng.u32(), rng.u8(), rng.i32(), rng.u8()));
        }
        assert_same(&format!("C12 round={round}"), |i| {
            for &(x, y, b, z, sel) in &inputs {
                if sel & 1 == 0 {
                    unsafe { (i.driver)(x, y, b, z) };
                } else {
                    let img = foo_image(b, [0xAA, 0xBB, 0xCC], z);
                    unsafe { (i.print_foo)(img.as_ptr()) };
                }
            }
        });
    }
}

// ===========================================================================
// Phase C — error-path differential tests (ERRORS.md rows E1..E7)
// ===========================================================================

/// E1: `x` above the 2-bit range. The C silently truncates (`and $0x3`);
/// no error is reported. Includes `UINT_MAX` and one step past `3`.
fn err_e1_x_out_of_range() {
    let mut cases: Vec<u32> = vec![4, 5, 7, 8, 0x7FFF_FFFF, 0x8000_0000, u32::MAX, u32::MAX - 1];
    let mut rng = Rng::new();
    for _ in 0..256 {
        cases.push(4 + (rng.u32() % (u32::MAX - 4)));
    }
    for x in cases {
        assert_same(&format!("E1 x={x}"), |i| unsafe { (i.driver)(x, 5, 1, -7) });
    }
}

/// E2: `y` above the 3-bit range — silently truncated (`and $0x7`).
fn err_e2_y_out_of_range() {
    let mut cases: Vec<u32> = vec![8, 9, 15, 16, 0x7FFF_FFFF, 0x8000_0000, u32::MAX, u32::MAX - 1];
    let mut rng = Rng::new();
    for _ in 0..256 {
        cases.push(8 + (rng.u32() % (u32::MAX - 8)));
    }
    for y in cases {
        assert_same(&format!("E2 y={y}"), |i| unsafe { (i.driver)(2, y, 1, 42) });
    }
}

/// E3: out-of-range `_Bool` across the FFI boundary. C `_Bool` is a byte with
/// only two valid "variants"; the ABI lets any of the 256 values through.
/// gcc masks with `& 1`, so e.g. `0xFE` prints 0 and `0xFF` prints 1.
/// Exhaustive over all 256 byte values, at several `z`.
fn err_e3_bool_out_of_range() {
    for b in 0u16..256 {
        let b = b as u8;
        for &z in &[0i32, -1, i32::MIN, i32::MAX] {
            assert_same(&format!("E3 b={b} z={z}"), |i| unsafe {
                (i.driver)(1, 6, b, z)
            });
        }
    }
}

/// E4: `z` at and one step inside the `int` range ends, plus negatives.
fn err_e4_z_extremes() {
    let cases = [
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 2,
        -1,
        0,
        1,
        i32::MAX - 2,
        i32::MAX - 1,
        i32::MAX,
    ];
    for &z in &cases {
        assert_same(&format!("E4 driver z={z}"), |i| unsafe {
            (i.driver)(3, 7, 1, z)
        });
        let img = foo_image(packed_byte0(3, 7, 1), [0, 0, 0], z);
        assert_same(&format!("E4 print_foo z={z}"), |i| unsafe {
            (i.print_foo)(img.as_ptr())
        });
    }
}

/// E5: bits 6–7 of byte 0 are padding inside the bit-field allocation unit.
/// They are unreachable through `driver` but reachable through `print_foo`.
/// They must not affect the output: `byte0` and `byte0 | 0xC0` must print
/// identically, in both implementations.
fn err_e5_padding_bits_ignored() {
    for low in 0u8..64 {
        for extra in [0x00u8, 0x40, 0x80, 0xC0] {
            let byte0 = low | extra;
            let img = foo_image(byte0, [0, 0, 0], 12345);
            assert_same(&format!("E5 byte0={byte0:#04x}"), |i| unsafe {
                (i.print_foo)(img.as_ptr())
            });
        }
        // And the padding bits really are ignored by both.
        let base = capture(|| unsafe {
            (pair().c.print_foo)(foo_image(low, [0, 0, 0], 12345).as_ptr())
        });
        for extra in [0x40u8, 0x80, 0xC0] {
            let with = capture(|| unsafe {
                (pair().c.print_foo)(foo_image(low | extra, [0, 0, 0], 12345).as_ptr())
            });
            assert_eq!(base, with, "E5: C padding bits changed output (low={low:#04x})");
            let with_r = capture(|| unsafe {
                (pair().r.print_foo)(foo_image(low | extra, [0, 0, 0], 12345).as_ptr())
            });
            assert_eq!(
                base, with_r,
                "E5: Rust padding bits changed output (low={low:#04x})"
            );
        }
    }
}

/// E6: struct padding bytes 1–3 hold garbage (which is exactly what `driver`
/// leaves there — it never writes them). They must never be read.
fn err_e6_struct_padding_ignored() {
    let mut rng = Rng::new();
    for _ in 0..512 {
        let byte0 = rng.u8();
        let z = rng.i32();
        let pad = [rng.u8(), rng.u8(), rng.u8()];
        let clean = foo_image(byte0, [0, 0, 0], z);
        let dirty = foo_image(byte0, pad, z);

        let c_clean = capture(|| unsafe { (pair().c.print_foo)(clean.as_ptr()) });
        let c_dirty = capture(|| unsafe { (pair().c.print_foo)(dirty.as_ptr()) });
        let r_clean = capture(|| unsafe { (pair().r.print_foo)(clean.as_ptr()) });
        let r_dirty = capture(|| unsafe { (pair().r.print_foo)(dirty.as_ptr()) });

        assert_eq!(c_clean, c_dirty, "E6: C read struct padding");
        assert_eq!(r_clean, r_dirty, "E6: Rust read struct padding");
        assert_eq!(c_dirty, r_dirty, "E6: C/Rust mismatch with dirty padding");
    }
}

/// E7: `print_foo(NULL)`. The C dereferences unconditionally, so this is a
/// fatal signal rather than an error code. Assert both implementations die
/// the same way, by running each call in a forked child and comparing the
/// wait status.
fn err_e7_null_pointer_segv() {
    fn crash_status(f: PrintFooFn) -> c_int {
        unsafe {
            fflush(std::ptr::null_mut());
            let pid = fork();
            assert!(pid >= 0, "fork failed");
            if pid == 0 {
                // Child: make the fatal call. If it somehow returns, exit
                // with a distinctive code so the parent can tell.
                f(std::ptr::null());
                _exit(77);
            }
            let mut status: c_int = 0;
            let r = waitpid(pid, &mut status, 0);
            assert_eq!(r, pid, "waitpid failed");
            status
        }
    }

    // Force both libraries to be loaded (and any lazy PLT binding resolved
    // with a valid call) before forking.
    let warm = foo_image(0, [0, 0, 0], 0);
    let _ = capture(|| unsafe { (pair().c.print_foo)(warm.as_ptr()) });
    let _ = capture(|| unsafe { (pair().r.print_foo)(warm.as_ptr()) });

    let c_status = crash_status(pair().c.print_foo);
    let r_status = crash_status(pair().r.print_foo);

    // WIFSIGNALED / WTERMSIG
    let sig = |s: c_int| if s & 0x7f != 0 && s & 0x7f != 0x7f { s & 0x7f } else { -1 };
    let c_sig = sig(c_status);
    let r_sig = sig(r_status);

    assert!(
        c_sig > 0,
        "E7: expected the C print_foo(NULL) to die from a signal, status={c_status:#x}"
    );
    assert_eq!(
        c_sig, r_sig,
        "E7: C died from signal {c_sig} but Rust status was {r_status:#x} (signal {r_sig})"
    );
}

// ===========================================================================
// Phase D — symbol parity, asserted from inside the test suite as well.
// ===========================================================================

/// Every symbol the C `.so` exports must also be exported by the Rust `.so`,
/// under the exact same name.
fn phase_d_symbol_parity() {
    fn defined_symbols(path: &PathBuf) -> Vec<String> {
        let out = std::process::Command::new("nm")
            .args(["-D", "--defined-only", path.to_str().unwrap()])
            .output()
            .expect("run nm");
        assert!(
            out.status.success(),
            "nm failed on {}: {}",
            path.display(),
            String::from_utf8_lossy(&out.stderr)
        );
        let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().nth(2).map(str::to_string))
            .collect();
        v.sort();
        v.dedup();
        v
    }

    let c_syms = defined_symbols(&c_so_path());
    let r_syms = defined_symbols(&rust_so_path());

    assert!(
        c_syms.contains(&"driver".to_string()) && c_syms.contains(&"print_foo".to_string()),
        "sanity: C .so should export driver and print_foo, got {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.iter().filter(|s| !r_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}"
    );
}

// ===========================================================================
// Single entry point.
//
// Every row runs inside ONE `#[test]` on purpose. The capture mechanism
// redirects the process-wide fd 1, so nothing else may write to stdout while a
// capture is open — and libtest prints its own per-test progress ("test X ...
// ok") from the main thread, which would otherwise race into the capture file.
// With a single test, libtest's writes are strictly ordered before and after
// the whole run. Per-row progress is reported on stderr, which is untouched.
// ===========================================================================

#[test]
fn differential_all_phases() {
    let rows: &[(&str, fn())] = &[
        // Phase D — symbol parity first: if the surface is incomplete there is
        // no point comparing behaviour.
        ("D  symbol parity (SYMBOLS.md)", phase_d_symbol_parity),
        // Phase B — CONFIGS.md
        ("B  C1  driver in-range exhaustive", cfg_c1_driver_inrange_exhaustive),
        ("B  C2  driver z boundaries", cfg_c2_driver_z_boundaries),
        ("B  C3  driver z random", cfg_c3_driver_z_random),
        ("B  C4  driver x random full u32", cfg_c4_driver_x_random_full_u32),
        ("B  C5  driver y random full u32", cfg_c5_driver_y_random_full_u32),
        ("B  C6  driver b all 256 bytes", cfg_c6_driver_b_all_bytes),
        ("B  C7  driver all args random", cfg_c7_driver_all_random),
        ("B  C8  print_foo byte0 exhaustive", cfg_c8_print_foo_byte0_exhaustive),
        ("B  C9  print_foo image random", cfg_c9_print_foo_image_random),
        ("B  C10 print_foo z boundaries", cfg_c10_print_foo_z_boundaries),
        ("B  C11 encode/decode round trip", cfg_c11_encode_decode_roundtrip),
        ("B  C12 interleaved multicall stream", cfg_c12_interleaved_multicall_stream),
        // Phase C — ERRORS.md
        ("C  E1  x out of range", err_e1_x_out_of_range),
        ("C  E2  y out of range", err_e2_y_out_of_range),
        ("C  E3  bool out of range (all 256)", err_e3_bool_out_of_range),
        ("C  E4  z extremes", err_e4_z_extremes),
        ("C  E5  bit-field padding bits ignored", err_e5_padding_bits_ignored),
        ("C  E6  struct padding ignored", err_e6_struct_padding_ignored),
        ("C  E7  print_foo(NULL) signal parity", err_e7_null_pointer_segv),
    ];

    eprintln!("\nC   .so: {}", c_so_path().display());
    eprintln!("Rust .so: {}\n", rust_so_path().display());

    for (name, f) in rows {
        let t = std::time::Instant::now();
        f();
        eprintln!("  [ok] {name}  ({:.2}s)", t.elapsed().as_secs_f64());
    }
    eprintln!("\nall {} rows passed\n", rows.len());
}
