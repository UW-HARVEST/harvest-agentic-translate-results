// Differential test harness: loads BOTH the C `.so` and the Rust `.so` with
// `libloading` and compares their observable behaviour byte-for-byte through
// the FFI boundary. The Rust implementation is NEVER called directly — every
// invocation goes through `dlsym` on the built `cdylib`, so the
// `#[no_mangle] extern "C"` export wrapper is under test too.
//
// `driver`'s only output is what it writes to stdout via libc `printf`, so the
// harness redirects fd 1 to a temp file around each batch of calls and diffs
// the captured bytes.

use std::ffi::c_void;
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex, MutexGuard};

use libloading::os::unix::{Library as UnixLibrary, RTLD_LOCAL, RTLD_NOW};
use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// libc bits needed for stdout capture. Declared directly so the test suite
// needs no extra dependency beyond `libloading`.
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn dup(oldfd: i32) -> i32;
    fn dup2(oldfd: i32, newfd: i32) -> i32;
    fn close(fd: i32) -> i32;
    /// `fflush(NULL)` flushes *every* open output stream, including the
    /// `stdout` `FILE` that both `.so`s write through.
    fn fflush(stream: *mut c_void) -> i32;
}

/// Signature of the symbol under test: `void driver(int x)`.
type DriverFn = unsafe extern "C" fn(i32);
/// Deliberately mis-declared wider signature, used to push an argument bit
/// pattern outside `int` range across the ABI (ERRORS.md row E7).
type DriverFnWide = unsafe extern "C" fn(i64);

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

/// Guard against loading a STALE shared object.
///
/// This crate is `cdylib`-only, so the integration test has no *link*
/// dependency on the library and `cargo test` will happily run against an
/// out-of-date `.so` left over from a previous `cargo build`. That silently
/// turns the whole suite into a no-op: an injected bug would still "pass".
/// Compare mtimes and fail loudly instead.
fn assert_fresh(so: &Path, sources: &[PathBuf], rebuild_hint: &str) {
    let so_mtime = fs::metadata(so)
        .and_then(|m| m.modified())
        .unwrap_or_else(|e| panic!("stat {}: {e}", so.display()));
    for src in sources {
        let Ok(src_mtime) = fs::metadata(src).and_then(|m| m.modified()) else {
            continue;
        };
        assert!(
            so_mtime >= src_mtime,
            "STALE SHARED OBJECT: {} is older than its source {}.\n\
             The suite would be testing an out-of-date library. Rebuild with:\n  {rebuild_hint}",
            so.display(),
            src.display()
        );
    }
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    let p = crate_root().join("../c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {}. Build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// Derive the Rust `cdylib` location from the running test binary, which lives
/// at `<target>/<profile>/deps/<name>-<hash>`. This keeps the harness correct
/// for any profile or feature combination without hardcoding `debug`/`release`.
fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let profile_dir = deps.parent().expect("profile dir");

    let mut candidates = vec![
        profile_dir.join("libdriver.so"),
        deps.join("libdriver.so"),
        crate_root().join("target/release/libdriver.so"),
        crate_root().join("target/debug/libdriver.so"),
    ];
    candidates.retain(|p| p.exists());
    assert!(
        !candidates.is_empty(),
        "Rust cdylib libdriver.so not found near {}. Build it with `cargo build`.",
        profile_dir.display()
    );
    candidates.remove(0)
}

/// Open with `RTLD_NOW` so every undefined symbol in the object is resolved
/// eagerly at load time. A successful load is therefore positive evidence that
/// the Rust `.so` has no unresolvable (non-libc) imports — see SYMBOLS.md.
fn open_now(path: &Path) -> Library {
    let lib = unsafe { UnixLibrary::open(Some(path), RTLD_NOW | RTLD_LOCAL) }
        .unwrap_or_else(|e| panic!("dlopen({}) with RTLD_NOW failed: {e}", path.display()));
    Library::from(lib)
}

struct Libs {
    c: Library,
    r: Library,
}

impl Libs {
    fn c_driver(&self) -> Symbol<'_, DriverFn> {
        unsafe { self.c.get(b"driver\0") }.expect("`driver` missing from C .so")
    }
    fn r_driver(&self) -> Symbol<'_, DriverFn> {
        unsafe { self.r.get(b"driver\0") }.expect("`driver` missing from Rust .so")
    }
    fn c_driver_wide(&self) -> Symbol<'_, DriverFnWide> {
        unsafe { self.c.get(b"driver\0") }.expect("`driver` missing from C .so")
    }
    fn r_driver_wide(&self) -> Symbol<'_, DriverFnWide> {
        unsafe { self.r.get(b"driver\0") }.expect("`driver` missing from Rust .so")
    }
}

static LIBS: LazyLock<Libs> = LazyLock::new(|| {
    let c = c_so_path();
    let r = rust_so_path();
    let root = crate_root();
    assert_fresh(
        &c,
        &[
            root.join("../c_src/src/driver.c"),
            root.join("../c_src/include/driver.h"),
            root.join("../c_src/CMakeLists.txt"),
        ],
        "cd c_src/build && cmake --build .",
    );
    assert_fresh(
        &r,
        &[root.join("src/lib.rs"), root.join("Cargo.toml")],
        "cd translation && cargo build   (run BEFORE cargo test)",
    );
    Libs {
        c: open_now(&c),
        r: open_now(&r),
    }
});

/// fd 1 redirection is process-global, so captures must not overlap. Every
/// test acquires this lock for the duration of its captures.
static CAPTURE_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

fn capture_lock() -> MutexGuard<'static, ()> {
    CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

static SCRATCH_SEQ: LazyLock<Mutex<u64>> = LazyLock::new(|| Mutex::new(0));

fn scratch_file() -> fs::File {
    let mut seq = SCRATCH_SEQ.lock().unwrap_or_else(|e| e.into_inner());
    *seq += 1;
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "driver_difftest_{}_{}.bin",
        std::process::id(),
        *seq
    ));
    let f = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("open scratch file");
    // Unlink immediately; the open fd keeps it alive, so nothing is left behind.
    let _ = fs::remove_file(&path);
    f
}

/// Redirect fd 1 into a scratch file, run `body`, then restore fd 1 and return
/// everything that was written. Flushes libc's stdio *and* Rust's own stdout
/// buffer on both sides of the swap so no unrelated bytes leak into the
/// capture and none of the captured bytes escape it.
fn capture<F: FnOnce()>(body: F) -> Vec<u8> {
    let mut file = scratch_file();

    let _ = std::io::stdout().flush();
    unsafe { fflush(std::ptr::null_mut()) };

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");

    body();

    unsafe { fflush(std::ptr::null_mut()) };
    assert!(unsafe { dup2(saved, 1) } >= 0, "dup2 restore failed");
    unsafe { close(saved) };

    let mut out = Vec::new();
    file.seek(SeekFrom::Start(0)).expect("seek");
    file.read_to_end(&mut out).expect("read capture");
    out
}

// ---------------------------------------------------------------------------
// Differential drivers
// ---------------------------------------------------------------------------

/// Run the whole batch of inputs through the C `.so`, then through the Rust
/// `.so`, and assert the two captured byte streams are identical.
///
/// Batching keeps the fd juggling out of the inner loop, which is what makes
/// the exhaustive 64 Ki-input rows affordable.
#[track_caller]
fn assert_same(label: &str, inputs: &[i32]) -> Vec<u8> {
    let _guard = capture_lock();

    let c_out = {
        let f = LIBS.c_driver();
        capture(|| {
            for &x in inputs {
                unsafe { f(x) };
            }
        })
    };
    let r_out = {
        let f = LIBS.r_driver();
        capture(|| {
            for &x in inputs {
                unsafe { f(x) };
            }
        })
    };

    compare(label, inputs, &c_out, &r_out);
    c_out
}

#[track_caller]
fn compare(label: &str, inputs: &[i32], c_out: &[u8], r_out: &[u8]) {
    if c_out == r_out {
        return;
    }
    // Pinpoint the first differing record so the failure names the input.
    let c_lines: Vec<&[u8]> = c_out.split(|&b| b == b'\n').collect();
    let r_lines: Vec<&[u8]> = r_out.split(|&b| b == b'\n').collect();
    for (i, (cl, rl)) in c_lines.iter().zip(r_lines.iter()).enumerate() {
        if cl != rl {
            let x = inputs.get(i).copied();
            panic!(
                "[{label}] divergence at record {i} (input {x:?} / {:?}):\n  C   : {:?}\n  Rust: {:?}",
                x.map(|v| format!("{v:#010x}")),
                String::from_utf8_lossy(cl),
                String::from_utf8_lossy(rl),
            );
        }
    }
    panic!(
        "[{label}] output length differs: C {} bytes ({} records), Rust {} bytes ({} records)",
        c_out.len(),
        c_lines.len(),
        r_out.len(),
        r_lines.len()
    );
}

/// Every call must emit exactly one 8-hex-digit record plus `\n`.
#[track_caller]
fn assert_record_shape(label: &str, out: &[u8], n_inputs: usize) {
    assert_eq!(
        out.len(),
        n_inputs * 9,
        "[{label}] expected {} bytes ({} records x 9), got {}",
        n_inputs * 9,
        n_inputs,
        out.len()
    );
    for (i, rec) in out.chunks(9).enumerate() {
        assert_eq!(rec[8], b'\n', "[{label}] record {i} not newline-terminated");
        assert!(
            rec[..8]
                .iter()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(c)),
            "[{label}] record {i} is not 8 lowercase hex digits: {:?}",
            String::from_utf8_lossy(&rec[..8])
        );
    }
}

/// Expected C output for one input, computed from the documented semantics
/// (native-endian object representation, each byte as `%02x`, then `\n`).
fn expected(x: i32) -> String {
    let mut s = String::new();
    for b in x.to_ne_bytes() {
        s.push_str(&format!("{b:02x}"));
    }
    s.push('\n');
    s
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

const SEED: u64 = 0x5EED_1234;

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed)
    }
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
    fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[(self.next_u64() % xs.len() as u64) as usize]
    }
}

/// Build an `i32` from four chosen bytes, byte 0 = least significant.
fn from_le_bytes(b: [u8; 4]) -> i32 {
    i32::from_le_bytes(b)
}

/// Place `v` at byte position `pos` (0 = least significant), zeros elsewhere.
fn byte_at(pos: usize, v: u8) -> i32 {
    let mut b = [0u8; 4];
    b[pos] = v;
    from_le_bytes(b)
}

// ===========================================================================
// PHASE B — valid-path differential tests, one per CONFIGS.md row
// ===========================================================================

/// Sanity: both `.so`s load with RTLD_NOW and both export `driver`.
/// (Also the structural evidence cited by SYMBOLS.md.)
fn cfg_c0_both_libraries_export_driver() {
    let _c = LIBS.c_driver();
    let _r = LIBS.r_driver();
}

/// CONFIGS.md C1 — `x = 0`, all bytes `0x00`.
fn cfg_c1_zero() {
    let inputs = [0i32];
    let out = assert_same("C1 zero", &inputs);
    assert_record_shape("C1 zero", &out, inputs.len());
    assert_eq!(String::from_utf8_lossy(&out), "00000000\n");
}

/// CONFIGS.md C2 — `x = 1`; makes the little-endian byte order observable.
fn cfg_c2_one() {
    let inputs = [1i32];
    let out = assert_same("C2 one", &inputs);
    assert_record_shape("C2 one", &out, inputs.len());
    assert_eq!(String::from_utf8_lossy(&out), expected(1));
}

/// CONFIGS.md C3 — `x = INT_MAX`.
fn cfg_c3_int_max() {
    let inputs = [i32::MAX];
    let out = assert_same("C3 INT_MAX", &inputs);
    assert_record_shape("C3 INT_MAX", &out, inputs.len());
    assert_eq!(String::from_utf8_lossy(&out), "ffffff7f\n");
}

/// CONFIGS.md C4 — `x = INT_MIN`.
fn cfg_c4_int_min() {
    let inputs = [i32::MIN];
    let out = assert_same("C4 INT_MIN", &inputs);
    assert_record_shape("C4 INT_MIN", &out, inputs.len());
    assert_eq!(String::from_utf8_lossy(&out), "00000080\n");
}

/// CONFIGS.md C5 — a `0x00` byte isolated at each of the 4 positions.
fn cfg_c5_zero_byte_each_position() {
    let mut inputs = Vec::new();
    for pos in 0..4usize {
        let mut b = [0xffu8; 4];
        b[pos] = 0x00;
        inputs.push(from_le_bytes(b));
    }
    let out = assert_same("C5 zero byte per position", &inputs);
    assert_record_shape("C5 zero byte per position", &out, inputs.len());
}

/// Shared body for the per-position byte-class sweeps (C6..C11).
fn sweep_class(label: &str, pos: usize, range: std::ops::RangeInclusive<u8>) {
    let inputs: Vec<i32> = range.map(|v| byte_at(pos, v)).collect();
    let out = assert_same(label, &inputs);
    assert_record_shape(label, &out, inputs.len());
    // Independently confirm each record against the documented semantics.
    for (rec, &x) in out.chunks(9).zip(inputs.iter()) {
        assert_eq!(String::from_utf8_lossy(rec), expected(x), "{label} x={x:#010x}");
    }
}

/// CONFIGS.md C6 — pad-flag class `0x01..0x0f` at byte position 0.
fn cfg_c6_pad_class_position0() {
    sweep_class("C6 pad class pos0", 0, 0x01..=0x0f);
}

/// CONFIGS.md C7 — pad-flag class `0x01..0x0f` at byte position 1.
fn cfg_c7_pad_class_position1() {
    sweep_class("C7 pad class pos1", 1, 0x01..=0x0f);
}

/// CONFIGS.md C8 — pad-flag class `0x01..0x0f` at byte position 2.
fn cfg_c8_pad_class_position2() {
    sweep_class("C8 pad class pos2", 2, 0x01..=0x0f);
}

/// CONFIGS.md C9 — pad-flag class `0x01..0x0f` at byte position 3 (high byte).
fn cfg_c9_pad_class_position3() {
    sweep_class("C9 pad class pos3", 3, 0x01..=0x0f);
}

/// CONFIGS.md C10 — class `0x10..0x7f` swept at all 4 positions.
fn cfg_c10_mid_class_all_positions() {
    for pos in 0..4usize {
        sweep_class(&format!("C10 mid class pos{pos}"), pos, 0x10..=0x7f);
    }
}

/// CONFIGS.md C11 — class `0x80..0xff` swept at all 4 positions. This is the
/// `signed char` vs `unsigned char` divergence point: the C stores into
/// `char raw[]` (signed here) but reads through `unsigned char *`, so each byte
/// must print as 2 digits, never sign-extended to 8.
fn cfg_c11_high_bit_class_all_positions() {
    for pos in 0..4usize {
        sweep_class(&format!("C11 high-bit class pos{pos}"), pos, 0x80..=0xff);
    }
}

/// CONFIGS.md C12 — exhaustive {byte position} x {byte value} cross-product:
/// all 256 values at each of the 4 positions (1024 calls).
fn cfg_c12_exhaustive_position_value_crossproduct() {
    let mut inputs = Vec::with_capacity(1024);
    for pos in 0..4usize {
        for v in 0..=255u8 {
            inputs.push(byte_at(pos, v));
        }
    }
    let out = assert_same("C12 position x value", &inputs);
    assert_record_shape("C12 position x value", &out, inputs.len());
    for (rec, &x) in out.chunks(9).zip(inputs.iter()) {
        assert_eq!(String::from_utf8_lossy(rec), expected(x), "C12 x={x:#010x}");
    }
}

/// CONFIGS.md C13 — 20 000 seeded uniform random `i32` values.
fn cfg_c13_randomized_uniform() {
    let mut rng = Rng::new(SEED);
    let inputs: Vec<i32> = (0..20_000).map(|_| rng.next_i32()).collect();
    let out = assert_same("C13 uniform random", &inputs);
    assert_record_shape("C13 uniform random", &out, inputs.len());
    for (rec, &x) in out.chunks(9).zip(inputs.iter()) {
        assert_eq!(String::from_utf8_lossy(rec), expected(x), "C13 x={x:#010x}");
    }
}

/// CONFIGS.md C14 — randomized but biased to the boundary byte classes, so all
/// four positions carry an "interesting" byte simultaneously.
fn cfg_c14_randomized_boundary_bytes() {
    const CLASSES: [u8; 9] = [0x00, 0x01, 0x0f, 0x10, 0x7f, 0x80, 0x81, 0xfe, 0xff];
    let mut rng = Rng::new(SEED ^ 0xA5A5_A5A5);
    let inputs: Vec<i32> = (0..4_000)
        .map(|_| {
            from_le_bytes([
                rng.pick(&CLASSES),
                rng.pick(&CLASSES),
                rng.pick(&CLASSES),
                rng.pick(&CLASSES),
            ])
        })
        .collect();
    let out = assert_same("C14 boundary-byte random", &inputs);
    assert_record_shape("C14 boundary-byte random", &out, inputs.len());
    for (rec, &x) in out.chunks(9).zip(inputs.iter()) {
        assert_eq!(String::from_utf8_lossy(rec), expected(x), "C14 x={x:#010x}");
    }
}

/// CONFIGS.md C15 — the aggregate-sign axis, separated.
fn cfg_c15_randomized_by_sign() {
    let mut rng = Rng::new(SEED ^ 0x1234_5678);

    let neg: Vec<i32> = (0..2_000)
        .map(|_| {
            let v = rng.next_i32();
            if v == i32::MIN {
                v
            } else {
                -(v.abs())
            }
        })
        .collect();
    assert!(neg.iter().all(|&v| v < 0));
    let out = assert_same("C15 negative random", &neg);
    assert_record_shape("C15 negative random", &out, neg.len());

    let pos: Vec<i32> = (0..2_000).map(|_| rng.next_i32().abs().max(0)).collect();
    assert!(pos.iter().all(|&v| v >= 0));
    let out = assert_same("C15 non-negative random", &pos);
    assert_record_shape("C15 non-negative random", &out, pos.len());
}

/// CONFIGS.md C16 — exhaustive over two full 16-bit windows: all 65 536 low
/// halves, and all 65 536 high halves.
fn cfg_c16_exhaustive_16bit_windows() {
    let low: Vec<i32> = (0..=0xffffu32).map(|v| v as i32).collect();
    let out = assert_same("C16 low 16-bit window", &low);
    assert_record_shape("C16 low 16-bit window", &out, low.len());

    let high: Vec<i32> = (0..=0xffffu32).map(|v| (v << 16) as i32).collect();
    let out = assert_same("C16 high 16-bit window", &high);
    assert_record_shape("C16 high 16-bit window", &out, high.len());
}

/// CONFIGS.md C17 — many consecutive calls into the *same* library; catches
/// retained state or buffer drift that a single call cannot reveal.
fn cfg_c17_repeated_same_library_calls() {
    let inputs: Vec<i32> = std::iter::repeat(0x0f_80_01_feu32 as i32).take(5_000).collect();
    let out = assert_same("C17 repeated identical calls", &inputs);
    assert_record_shape("C17 repeated identical calls", &out, inputs.len());
    let one = expected(inputs[0]);
    for rec in out.chunks(9) {
        assert_eq!(String::from_utf8_lossy(rec), one, "C17 drift across calls");
    }
}

/// CONFIGS.md C18 / ERRORS.md E11 — C and Rust calls interleaved in one
/// process, both writing the same libc `stdout` `FILE`. A translation using
/// Rust's own buffered stdout would reorder or lose bytes here even though each
/// library looks correct in isolation.
fn cfg_c18_interleaved_c_and_rust_calls() {
    let _guard = capture_lock();
    let mut rng = Rng::new(SEED ^ 0xDEAD_BEEF);
    let inputs: Vec<i32> = (0..1_000).map(|_| rng.next_i32()).collect();

    let cf = LIBS.c_driver();
    let rf = LIBS.r_driver();

    let out = capture(|| {
        for &x in &inputs {
            unsafe { cf(x) };
            unsafe { rf(x) };
        }
    });

    // 2 records per input, and each adjacent pair must be identical (that pair
    // is the C result next to the Rust result for the same input).
    assert_record_shape("C18 interleaved", &out, inputs.len() * 2);
    for (i, pair) in out.chunks(18).enumerate() {
        let (c_rec, r_rec) = pair.split_at(9);
        assert_eq!(
            c_rec,
            r_rec,
            "C18 interleaved divergence at input {i} ({:#010x}): C {:?} vs Rust {:?}",
            inputs[i],
            String::from_utf8_lossy(c_rec),
            String::from_utf8_lossy(r_rec)
        );
        assert_eq!(String::from_utf8_lossy(c_rec), expected(inputs[i]));
    }
}

/// CONFIGS.md C19 / ERRORS.md E7 — invoke the symbol through a wider
/// `extern "C" fn(i64)` so the argument register carries bits no valid `int`
/// could produce. Both sides must observe only the low 32 bits, identically.
fn cfg_c19_wide_argument_abi_shape() {
    let _guard = capture_lock();
    let mut rng = Rng::new(SEED ^ 0x0FF1_CE00);
    let mut wide: Vec<i64> = vec![
        0x7fff_ffff_dead_beef_u64 as i64,
        -1i64,
        i64::MIN,
        i64::MAX,
        0x0000_0001_0000_0000_u64 as i64, // low 32 bits zero, high bits set
        0xffff_ffff_0000_0000_u64 as i64,
    ];
    wide.extend((0..500).map(|_| rng.next_u64() as i64));

    let cf = LIBS.c_driver_wide();
    let rf = LIBS.r_driver_wide();

    let c_out = capture(|| {
        for &x in &wide {
            unsafe { cf(x) };
        }
    });
    let r_out = capture(|| {
        for &x in &wide {
            unsafe { rf(x) };
        }
    });

    let truncated: Vec<i32> = wide.iter().map(|&v| v as i32).collect();
    compare("C19 wide argument", &truncated, &c_out, &r_out);
    assert_record_shape("C19 wide argument", &c_out, wide.len());
    // And confirm the observed value really is the low 32 bits.
    for (rec, &x) in c_out.chunks(9).zip(truncated.iter()) {
        assert_eq!(String::from_utf8_lossy(rec), expected(x), "C19 x={x:#010x}");
    }
}

// ===========================================================================
// PHASE C — error/boundary-path differential tests, one per ERRORS.md row
//
// `driver` returns `void` and has no error channel (see the mechanical
// derivation in ERRORS.md: the C source contains zero `return`s, zero asserts,
// zero null checks, zero range checks, and exactly one conditional, the
// `i < len` loop guard). So "same error/rejection" is asserted here as: the
// same observable result for the same boundary input — identical stdout bytes,
// identical (void) return, and neither library aborting/trapping. Where a row
// is structurally not constructible (no pointer param, no length param) the
// test asserts that non-constructibility against the actual dynamic symbol
// tables rather than silently skipping it.
// ===========================================================================

/// ERRORS.md E1 — zero/empty-value boundary.
fn err_e1_zero() {
    let out = assert_same("E1 zero", &[0]);
    assert_eq!(String::from_utf8_lossy(&out), "00000000\n");
}

/// ERRORS.md E2 — largest valid `int`.
fn err_e2_int_max() {
    let out = assert_same("E2 INT_MAX", &[i32::MAX]);
    assert_eq!(String::from_utf8_lossy(&out), "ffffff7f\n");
}

/// ERRORS.md E3 — smallest valid `int` (one step past `INT_MAX` when wrapping).
fn err_e3_int_min() {
    let out = assert_same("E3 INT_MIN", &[i32::MIN]);
    assert_eq!(String::from_utf8_lossy(&out), "00000080\n");
    // Also the arithmetic neighbours of both extremes.
    let out = assert_same(
        "E3 extremes neighbourhood",
        &[i32::MIN, i32::MIN + 1, i32::MAX - 1, i32::MAX, -1, 0, 1],
    );
    assert_record_shape("E3 extremes neighbourhood", &out, 7);
}

/// ERRORS.md E4 — `x = -1`: all four bytes are `0xff`, i.e. negative as
/// `signed char`. Must print 8 hex digits, not 32 (sign-extension bug).
fn err_e4_all_high_bytes() {
    let out = assert_same("E4 all-0xff", &[-1]);
    assert_eq!(
        String::from_utf8_lossy(&out),
        "ffffffff\n",
        "sign-extension divergence: each byte must format as exactly 2 hex digits"
    );
    assert_eq!(out.len(), 9, "record must be 8 hex digits + newline");
}

/// ERRORS.md E5 — high-bit-set byte swept through every position, plus the
/// all-high-bytes combinations.
fn err_e5_high_bit_per_byte_position() {
    let mut inputs = vec![
        0x8080_8080_u32 as i32,
        0xff00_0000_u32 as i32,
        0x0080_00ff_u32 as i32,
        0xffff_ffff_u32 as i32,
        0x80ff_80ff_u32 as i32,
    ];
    for pos in 0..4usize {
        for v in [0x80u8, 0x81, 0xc0, 0xfe, 0xff] {
            inputs.push(byte_at(pos, v));
        }
    }
    let out = assert_same("E5 high-bit per position", &inputs);
    assert_record_shape("E5 high-bit per position", &out, inputs.len());
    for (rec, &x) in out.chunks(9).zip(inputs.iter()) {
        assert_eq!(String::from_utf8_lossy(rec), expected(x), "E5 x={x:#010x}");
    }
}

/// ERRORS.md E6 — sub-`0x10` byte in every position; a missing `0`/width flag
/// would emit one digit and desynchronise the record.
fn err_e6_zero_padding_per_byte_position() {
    let mut inputs = vec![0x0102_0304, 0x0f00_0000, 0x0000_000f, 0x0101_0101];
    for pos in 0..4usize {
        for v in 0x00u8..=0x0f {
            inputs.push(byte_at(pos, v));
        }
    }
    let out = assert_same("E6 zero-pad per position", &inputs);
    // The shape assertion IS the padding assertion: 9 bytes per record only
    // holds if every byte produced exactly 2 digits.
    assert_record_shape("E6 zero-pad per position", &out, inputs.len());
    for (rec, &x) in out.chunks(9).zip(inputs.iter()) {
        assert_eq!(String::from_utf8_lossy(rec), expected(x), "E6 x={x:#010x}");
    }
}

/// ERRORS.md E7 — out-of-range argument bit pattern across the FFI boundary
/// (the `int`-parameter analog of passing an out-of-range enum value). Covered
/// by the C19 test body; re-asserted here against the ERRORS.md row with a
/// distinct, adversarial input set.
fn err_e7_oversized_argument_truncation() {
    let _guard = capture_lock();
    let wide: Vec<i64> = vec![
        i64::MIN,
        i64::MAX,
        -1,
        0,
        1,
        0x7fff_ffff_ffff_ffff,
        0x8000_0000_8000_0000_u64 as i64,
        0xdead_beef_dead_beef_u64 as i64,
        0x0000_0000_ffff_ffff_u64 as i64,
        0xffff_ffff_0000_0000_u64 as i64,
        (i32::MAX as i64) + 1,
        (i32::MIN as i64) - 1,
        (u32::MAX as i64),
        (u32::MAX as i64) + 1,
    ];

    let cf = LIBS.c_driver_wide();
    let rf = LIBS.r_driver_wide();
    let c_out = capture(|| {
        for &x in &wide {
            unsafe { cf(x) };
        }
    });
    let r_out = capture(|| {
        for &x in &wide {
            unsafe { rf(x) };
        }
    });

    let truncated: Vec<i32> = wide.iter().map(|&v| v as i32).collect();
    compare("E7 oversized argument", &truncated, &c_out, &r_out);
    assert_record_shape("E7 oversized argument", &c_out, wide.len());
}

/// ERRORS.md E8 — no `int` value is rejected. Swept over the boundary values
/// and a seeded random sample: every call must succeed (no abort, no trap) and
/// produce one well-formed record, identically on both sides.
fn err_e8_no_value_is_rejected() {
    let mut inputs: Vec<i32> = vec![
        i32::MIN,
        i32::MIN + 1,
        -1,
        0,
        1,
        i32::MAX - 1,
        i32::MAX,
        0x8000_0000_u32 as i32,
        0x7fff_ffff,
        0x5555_5555,
        0xAAAA_AAAA_u32 as i32,
    ];
    let mut rng = Rng::new(SEED ^ 0xC0FF_EE00);
    inputs.extend((0..10_000).map(|_| rng.next_i32()));

    let out = assert_same("E8 nothing rejected", &inputs);
    // No input produced zero output, an extra record, or a malformed record:
    // i.e. there is no rejection path.
    assert_record_shape("E8 nothing rejected", &out, inputs.len());
}

/// ERRORS.md E9 — a null-pointer row is not constructible through the public
/// API: `driver` takes no pointer, and `print_hex` (the only function with a
/// pointer parameter) has internal linkage. Assert that structurally against
/// both dynamic symbol tables, so neither library can be made to null-deref.
fn err_e9_no_null_pointer_surface() {
    for (name, lib) in [("C", &LIBS.c), ("Rust", &LIBS.r)] {
        let sym: Result<Symbol<'_, *const c_void>, _> = unsafe { lib.get(b"print_hex\0") };
        assert!(
            sym.is_err(),
            "{name} .so unexpectedly exports `print_hex`; it is `static` in the C \
             source and must not be reachable via dlsym from either library"
        );
    }
    // And `driver` itself is present in both, with no pointer parameter to null.
    let _ = LIBS.c_driver();
    let _ = LIBS.r_driver();
}

/// ERRORS.md E10 — zero-length / oversized-length rows are not constructible:
/// `driver` takes no length, and `print_hex`'s `len` is always
/// `sizeof(int)` == 4. Assert the target really has `sizeof(int) == 4` (so the
/// `len <= 0` branch of the source's only conditional is genuinely dead code),
/// and that the unconditional trailing newline is always emitted.
fn err_e10_no_length_surface() {
    assert_eq!(
        std::mem::size_of::<std::ffi::c_int>(),
        4,
        "harness assumes sizeof(int) == 4 on this target"
    );
    let inputs = [0i32, -1, i32::MIN, i32::MAX, 0x1234_5678];
    let out = assert_same("E10 fixed length", &inputs);
    assert_record_shape("E10 fixed length", &out, inputs.len());
    // Exactly 4 bytes formatted (8 digits) per call, and the newline is always
    // present even though it sits after the loop.
    for rec in out.chunks(9) {
        assert_eq!(rec.len(), 9);
        assert_eq!(rec[8], b'\n');
    }
    // A single call must produce a newline and nothing more.
    let single = assert_same("E10 single call newline", &[0]);
    assert_eq!(single, b"00000000\n");
}

/// ERRORS.md E11 — interleaved C/Rust invocation through the shared libc
/// `stdout` stream. (Same scenario as CONFIGS.md C18; kept as its own row-test
/// so the ERRORS.md row has a dedicated passing test.)
fn err_e11_interleaved_c_and_rust_calls() {
    let _guard = capture_lock();
    let inputs: [i32; 6] = [0, 1, -1, i32::MIN, i32::MAX, 0x0f80_01fe];

    let cf = LIBS.c_driver();
    let rf = LIBS.r_driver();

    // C, C, R, R, C, R, ... an irregular pattern, so a buffer that only flushed
    // on alternation would still be caught.
    let out = capture(|| unsafe {
        cf(inputs[0]);
        cf(inputs[1]);
        rf(inputs[2]);
        rf(inputs[3]);
        cf(inputs[4]);
        rf(inputs[5]);
    });

    let mut want = String::new();
    for &x in &inputs {
        // Call order above visits inputs 0..5 in order.
        want.push_str(&expected(x));
    }
    assert_eq!(
        String::from_utf8_lossy(&out),
        want,
        "interleaved output must be the exact concatenation of each call, in call order"
    );
}

// ===========================================================================
// Custom serial harness (`harness = false` in Cargo.toml)
//
// libtest is not usable here: capturing the library's output requires
// redirecting fd 1, which is process-global, while libtest writes its own
// progress lines to fd 1 from the main thread and runs tests in parallel. That
// interleaving corrupts the captured bytes and produces phantom "divergences".
// This runner executes every case sequentially and only ever writes to stdout
// while fd 1 is un-redirected. Panic messages go to stderr, which is never
// redirected, so failure output stays intact.
// ===========================================================================

macro_rules! cases {
    ($($f:ident),* $(,)?) => { &[ $((stringify!($f), $f as fn())),* ] };
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let filter = args.iter().find(|a| !a.starts_with('-')).cloned();

    let cases: &[(&str, fn())] = cases![
        // Phase B — CONFIGS.md rows
        cfg_c0_both_libraries_export_driver,
        cfg_c1_zero,
        cfg_c2_one,
        cfg_c3_int_max,
        cfg_c4_int_min,
        cfg_c5_zero_byte_each_position,
        cfg_c6_pad_class_position0,
        cfg_c7_pad_class_position1,
        cfg_c8_pad_class_position2,
        cfg_c9_pad_class_position3,
        cfg_c10_mid_class_all_positions,
        cfg_c11_high_bit_class_all_positions,
        cfg_c12_exhaustive_position_value_crossproduct,
        cfg_c13_randomized_uniform,
        cfg_c14_randomized_boundary_bytes,
        cfg_c15_randomized_by_sign,
        cfg_c16_exhaustive_16bit_windows,
        cfg_c17_repeated_same_library_calls,
        cfg_c18_interleaved_c_and_rust_calls,
        cfg_c19_wide_argument_abi_shape,
        // Phase C — ERRORS.md rows
        err_e1_zero,
        err_e2_int_max,
        err_e3_int_min,
        err_e4_all_high_bytes,
        err_e5_high_bit_per_byte_position,
        err_e6_zero_padding_per_byte_position,
        err_e7_oversized_argument_truncation,
        err_e8_no_value_is_rejected,
        err_e9_no_null_pointer_surface,
        err_e10_no_length_surface,
        err_e11_interleaved_c_and_rust_calls,
    ];

    println!("\nrunning {} differential cases (serial)", cases.len());
    println!("  C   .so: {}", c_so_path().display());
    println!("  Rust .so: {}", rust_so_path().display());
    println!();

    let mut passed = 0usize;
    let mut skipped = 0usize;
    let mut failed: Vec<&str> = Vec::new();

    for (name, f) in cases {
        if let Some(fl) = &filter {
            if !name.contains(fl.as_str()) {
                skipped += 1;
                continue;
            }
        }
        print!("test {name} ... ");
        let _ = std::io::stdout().flush();
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
            Ok(()) => {
                passed += 1;
                println!("ok");
            }
            Err(_) => {
                failed.push(name);
                println!("FAILED");
            }
        }
        let _ = std::io::stdout().flush();
    }

    println!();
    if failed.is_empty() {
        println!("test result: ok. {passed} passed; 0 failed; {skipped} filtered out");
    } else {
        println!("failures:");
        for name in &failed {
            println!("    {name}");
        }
        println!(
            "\ntest result: FAILED. {passed} passed; {} failed; {skipped} filtered out",
            failed.len()
        );
        std::process::exit(1);
    }
}
