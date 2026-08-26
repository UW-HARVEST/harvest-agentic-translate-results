//! Differential tests: C `libdriver.so` vs Rust `libdriver.so`.
//!
//! Both libraries are loaded with `libloading` and driven **only** through
//! their exported `driver` symbol, exactly as an external consumer would, so
//! the `#[no_mangle]` export wrappers are under test too.
//!
//! The library's whole observable effect is what it writes to `stdout`
//! (`printf("%02x")` per byte of the `house_t` struct, then `printf("\n")`), so
//! the comparison is done by capturing file descriptor 1 around each call and
//! comparing the captured bytes.
//!
//! Phase A artifacts: `SYMBOLS.md`, `ERRORS.md`, `CONFIGS.md`.

use libloading::{Library, Symbol};
use std::fs::File;
use std::os::raw::{c_char, c_int, c_void};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// libc bits we need for fd-level capture (declared directly so that the only
// dev-dependency remains `libloading`).
// ---------------------------------------------------------------------------

extern "C" {
    /// glibc's `stdout` FILE* — the *same* object both `.so`s print through.
    static stdout: *mut c_void;
    fn fflush(stream: *mut c_void) -> c_int;
    fn setvbuf(stream: *mut c_void, buf: *mut c_char, mode: c_int, size: usize) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
}

const IOFBF: c_int = 0; // fully buffered
const IOLBF: c_int = 1; // line buffered
const IONBF: c_int = 2; // unbuffered

// ---------------------------------------------------------------------------
// Loading the two shared objects
// ---------------------------------------------------------------------------

type DriverFn = unsafe extern "C" fn(c_int);

fn c_so_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libdriver.so")
}

fn rust_so_path() -> PathBuf {
    // The test binary lives in <target>/<profile>/deps/, the cdylib in
    // <target>/<profile>/.
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("profile dir")
        .to_path_buf();
    let p = profile_dir.join("libdriver.so");

    // IMPORTANT: `cargo test` does NOT rebuild a `crate-type = ["cdylib"]`
    // library (it only builds the unit-test binary and the integration test
    // binaries), so a plain `cargo test` would happily load a *stale* `.so`
    // and report false passes. Build it explicitly, then verify freshness.
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = std::process::Command::new(cargo);
    cmd.current_dir(env!("CARGO_MANIFEST_DIR"))
        .arg("build")
        .arg("--offline")
        .arg("--lib");
    if profile_dir.file_name().map(|s| s == "release").unwrap_or(false) {
        cmd.arg("--release");
    }
    // Forward the feature selection this test binary was compiled with, so the
    // loaded `.so` matches the configuration under test.
    cmd.arg("--no-default-features");
    for f in enabled_features() {
        cmd.arg("--features").arg(f);
    }
    match cmd.output() {
        Ok(out) if out.status.success() => {}
        Ok(out) => panic!(
            "rebuilding the cdylib failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        ),
        Err(e) => panic!("could not run cargo to rebuild the cdylib: {e}"),
    }

    assert!(
        p.exists(),
        "Rust cdylib not found at {p:?} — run `cargo build` first"
    );

    // Freshness guard: the `.so` must not predate any Rust source file.
    let so_mtime = std::fs::metadata(&p)
        .and_then(|m| m.modified())
        .expect("cdylib mtime");
    let src_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    for entry in std::fs::read_dir(&src_dir).expect("read src/") {
        let entry = entry.expect("dir entry");
        let m = entry.metadata().expect("src metadata");
        if m.is_file() {
            let src_mtime = m.modified().expect("src mtime");
            assert!(
                so_mtime >= src_mtime,
                "STALE cdylib: {p:?} is older than {:?} — the differential test \
                 would be comparing against an outdated Rust library",
                entry.path()
            );
        }
    }
    p
}

/// Features this test binary was compiled with (kept in sync with Cargo.toml;
/// the crate currently declares none, so this is empty for every valid combo).
fn enabled_features() -> Vec<&'static str> {
    #[allow(unused_mut)]
    let mut v: Vec<&'static str> = Vec::new();
    v
}

struct Libs {
    c: Library,
    rust: Library,
}

// The two handles are only ever used to read function pointers.
unsafe impl Sync for Libs {}
unsafe impl Send for Libs {}

fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| unsafe {
        let c = Library::new(c_so_path()).expect("dlopen C libdriver.so");
        let rust = Library::new(rust_so_path()).expect("dlopen Rust libdriver.so");
        Libs { c, rust }
    })
}

fn c_driver() -> DriverFn {
    unsafe {
        let sym: Symbol<DriverFn> = libs().c.get(b"driver\0").expect("C `driver` symbol");
        *sym
    }
}

fn rust_driver() -> DriverFn {
    unsafe {
        let sym: Symbol<DriverFn> = libs().rust.get(b"driver\0").expect("Rust `driver` symbol");
        *sym
    }
}

// ---------------------------------------------------------------------------
// stdout capture (process-global ⇒ serialised behind a mutex)
// ---------------------------------------------------------------------------

fn capture_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let m = LOCK.get_or_init(|| Mutex::new(()));
    m.lock().unwrap_or_else(|e| e.into_inner())
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Buf {
    Full,
    Line,
    Unbuffered,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Sink {
    File,
    Pipe,
}

fn tmp_path() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "driver_diff_{}_{}.out",
        std::process::id(),
        n
    ))
}

/// Run `body` with fd 1 redirected to `sink` and `stdout` in buffering mode
/// `buf`; return everything that was written. fd 1 and the buffering mode are
/// always restored before returning, so assertions can be made by the caller.
fn capture(sink: Sink, buf: Buf, mut body: impl FnMut()) -> Vec<u8> {
    // fd 1 is a process-global resource, so captures are serialised. The test
    // binary uses `harness = false` (see Cargo.toml) and runs strictly
    // sequentially, so nothing else can write to fd 1 inside the window.
    let _guard = capture_lock();
    // Push out anything the runner has buffered in Rust's own stdout so it
    // cannot be flushed into our capture window later.
    {
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }
    unsafe {
        fflush(stdout);
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");

        let (path, read_fd, write_fd, file) = match sink {
            Sink::File => {
                let path = tmp_path();
                let f = File::create(&path).expect("create temp file");
                let fd = f.as_raw_fd();
                (Some(path), -1, fd, Some(f))
            }
            Sink::Pipe => {
                let mut fds = [-1 as c_int; 2];
                assert_eq!(pipe(fds.as_mut_ptr()), 0, "pipe() failed");
                (None, fds[0], fds[1], None)
            }
        };

        assert!(dup2(write_fd, 1) >= 0, "dup2 onto fd 1 failed");

        let mode = match buf {
            Buf::Full => IOFBF,
            Buf::Line => IOLBF,
            Buf::Unbuffered => IONBF,
        };
        // Both implementations observe the exact same stream configuration.
        // (The return code is checked *after* fd 1 has been restored so a
        // failure can never leave the process without a usable stdout.)
        let setvbuf_rc = setvbuf(stdout, std::ptr::null_mut(), mode, 0);

        body();

        fflush(stdout);
        // Restore default (fully buffered) before handing fd 1 back.
        setvbuf(stdout, std::ptr::null_mut(), IOFBF, 0);
        assert!(dup2(saved, 1) >= 0, "restore fd 1 failed");
        close(saved);
        assert_eq!(
            setvbuf_rc, 0,
            "setvbuf(mode={mode}) failed — buffering axis F was not actually applied"
        );

        match sink {
            Sink::File => {
                drop(file);
                let path = path.unwrap();
                let data = std::fs::read(&path).expect("read temp file");
                let _ = std::fs::remove_file(&path);
                data
            }
            Sink::Pipe => {
                close(write_fd);
                let mut out = Vec::new();
                let mut chunk = [0u8; 8192];
                loop {
                    let n = read(read_fd, chunk.as_mut_ptr() as *mut c_void, chunk.len());
                    if n <= 0 {
                        break;
                    }
                    out.extend_from_slice(&chunk[..n as usize]);
                }
                close(read_fd);
                out
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (fixed seed ⇒ reproducible)
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    fn next_u64(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `0..n`.
    fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
}

const SEED: u64 = 0x2026_0818;

// ---------------------------------------------------------------------------
// Independent expectation of the C behaviour (structure checks C12/C13)
// ---------------------------------------------------------------------------

/// `house_t { floors = x, bedrooms = 3, bathrooms = 2.0 }` hex-dumped LE.
fn expected_line(x: i32) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(16);
    bytes.extend_from_slice(&x.to_le_bytes());
    bytes.extend_from_slice(&3i32.to_le_bytes());
    bytes.extend_from_slice(&2.0f64.to_le_bytes());
    let mut s = String::with_capacity(33);
    for b in &bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s.push('\n');
    s.into_bytes()
}

fn expected_stream(inputs: &[i32]) -> Vec<u8> {
    let mut v = Vec::new();
    for &x in inputs {
        v.extend_from_slice(&expected_line(x));
    }
    v
}

/// Structural validation applied to real captured output (C12/C13).
fn validate_structure(out: &[u8], inputs: &[i32], what: &str) {
    assert_eq!(
        out.len(),
        33 * inputs.len(),
        "{what}: expected {} bytes (33 per call), got {}",
        33 * inputs.len(),
        out.len()
    );
    for (i, (line, &x)) in out
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .zip(inputs)
        .enumerate()
    {
        assert_eq!(line.len(), 32, "{what}: line {i} is not 32 hex digits");
        assert!(
            line.iter().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(c)),
            "{what}: line {i} has non-lowercase-hex chars: {:?}",
            String::from_utf8_lossy(line)
        );
        let exp = expected_line(x);
        assert_eq!(
            line,
            &exp[..32],
            "{what}: line {i} for input {x}: got {}, want {}",
            String::from_utf8_lossy(line),
            String::from_utf8_lossy(&exp[..32])
        );
        // C13: constant fields, independent of `floors`.
        assert_eq!(&line[8..16], b"03000000", "{what}: bedrooms field changed");
        assert_eq!(
            &line[16..32],
            b"0000000000000040",
            "{what}: bathrooms field changed"
        );
    }
}

// ---------------------------------------------------------------------------
// The two core differential drivers
// ---------------------------------------------------------------------------

/// One capture window per input (fine-grained: catches per-call divergence).
fn diff_each(what: &str, inputs: &[i32]) {
    let cd = c_driver();
    let rd = rust_driver();
    for &x in inputs {
        let c_out = capture(Sink::File, Buf::Full, || unsafe { cd(x) });
        let r_out = capture(Sink::File, Buf::Full, || unsafe { rd(x) });
        assert_eq!(
            c_out,
            r_out,
            "{what}: divergence for input {x}\n  C   : {}\n  Rust: {}",
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
        validate_structure(&c_out, &[x], &format!("{what}/C"));
        validate_structure(&r_out, &[x], &format!("{what}/Rust"));
        assert_eq!(c_out, expected_line(x), "{what}: C output vs model, x={x}");
    }
}

/// All inputs inside a single capture window under a given stream config.
fn diff_all(what: &str, sink: Sink, buf: Buf, inputs: &[i32]) {
    let cd = c_driver();
    let rd = rust_driver();
    let c_out = capture(sink, buf, || {
        for &x in inputs {
            unsafe { cd(x) }
        }
    });
    let r_out = capture(sink, buf, || {
        for &x in inputs {
            unsafe { rd(x) }
        }
    });
    assert_eq!(
        c_out.len(),
        r_out.len(),
        "{what} ({sink:?}/{buf:?}): length differs ({} vs {})",
        c_out.len(),
        r_out.len()
    );
    assert_eq!(c_out, r_out, "{what} ({sink:?}/{buf:?}): byte divergence");
    validate_structure(&c_out, inputs, &format!("{what}/C"));
    assert_eq!(
        c_out,
        expected_stream(inputs),
        "{what}: C stream vs model"
    );
}

// ===========================================================================
// Phase B — CONFIGS.md rows
// ===========================================================================

fn c1_zero() {
    diff_each("C1 floors=0", &[0]);
}

fn c2_minus_one() {
    diff_each("C2 floors=-1", &[-1]);
}

fn c3_small_positive_randomized() {
    let mut rng = Rng::new(SEED ^ 3);
    let inputs: Vec<i32> = (0..64).map(|_| 1 + rng.below(255) as i32).collect();
    diff_each("C3 small positive", &inputs);
    diff_all("C3 small positive", Sink::File, Buf::Full, &inputs);
}

fn c4_small_negative_randomized() {
    let mut rng = Rng::new(SEED ^ 4);
    let inputs: Vec<i32> = (0..64).map(|_| -1 - rng.below(256) as i32).collect();
    diff_each("C4 small negative", &inputs);
    diff_all("C4 small negative", Sink::File, Buf::Full, &inputs);
}

fn c5_all_bytes_low_nibble_randomized() {
    // Every byte in 0x00..=0x0f ⇒ `%02x` zero-pad path on all four bytes.
    let mut rng = Rng::new(SEED ^ 5);
    let inputs: Vec<i32> = (0..64)
        .map(|_| {
            let b = [
                rng.below(16) as u8,
                rng.below(16) as u8,
                rng.below(16) as u8,
                rng.below(16) as u8,
            ];
            i32::from_le_bytes(b)
        })
        .collect();
    diff_each("C5 low-nibble bytes", &inputs);
}

fn c6_all_bytes_high_bit_randomized() {
    // Every byte in 0x80..=0xff ⇒ catches signed-char promotion bugs.
    let mut rng = Rng::new(SEED ^ 6);
    let inputs: Vec<i32> = (0..64)
        .map(|_| {
            let b = [
                0x80 | rng.below(128) as u8,
                0x80 | rng.below(128) as u8,
                0x80 | rng.below(128) as u8,
                0x80 | rng.below(128) as u8,
            ];
            i32::from_le_bytes(b)
        })
        .collect();
    diff_each("C6 high-bit bytes", &inputs);
}

fn c7_range_extremes() {
    let inputs = [
        i32::MIN,
        i32::MIN + 1,
        -1,
        0,
        1,
        i32::MAX - 1,
        i32::MAX,
    ];
    diff_each("C7 extremes", &inputs);
    diff_all("C7 extremes", Sink::File, Buf::Full, &inputs);
}

fn c8_special_bytes_randomized() {
    // Patterns embedding 0x0a (\n), 0x0d (\r), 0x00 (NUL) and 0x25 ('%').
    let specials = [0x0au8, 0x0d, 0x00, 0x25];
    let mut rng = Rng::new(SEED ^ 8);
    let mut inputs = Vec::new();
    for _ in 0..64 {
        let mut b = [
            rng.next_u32() as u8,
            rng.next_u32() as u8,
            rng.next_u32() as u8,
            rng.next_u32() as u8,
        ];
        // Force at least one special byte at a random position.
        let pos = rng.below(4) as usize;
        b[pos] = specials[rng.below(specials.len() as u32) as usize];
        inputs.push(i32::from_le_bytes(b));
    }
    // Fully special patterns too.
    for s in specials {
        inputs.push(i32::from_le_bytes([s; 4]));
    }
    diff_each("C8 special bytes", &inputs);
    diff_all("C8 special bytes", Sink::File, Buf::Full, &inputs);
}

fn c9_uniform_random_full_domain() {
    let mut rng = Rng::new(SEED ^ 9);
    let inputs: Vec<i32> = (0..512).map(|_| rng.next_i32()).collect();
    diff_each("C9 uniform random", &inputs);
    diff_all("C9 uniform random", Sink::File, Buf::Full, &inputs);
}

fn c10_byte_position_sweep_ff() {
    let inputs: Vec<i32> = (0..4)
        .map(|i| {
            let mut b = [0u8; 4];
            b[i] = 0xff;
            i32::from_le_bytes(b)
        })
        .collect();
    diff_each("C10 0xff sweep", &inputs);
}

fn c11_byte_position_sweep_01() {
    let inputs: Vec<i32> = (0..4)
        .map(|i| {
            let mut b = [0xffu8; 4];
            b[i] = 0x01;
            i32::from_le_bytes(b)
        })
        .collect();
    diff_each("C11 0x01 sweep", &inputs);
}

fn c12_c13_output_shape_and_constant_fields() {
    // 33 bytes per call, lowercase hex, constant bedrooms/bathrooms fields.
    // `validate_structure` enforces all of it on real captured output.
    let mut rng = Rng::new(SEED ^ 12);
    let inputs: Vec<i32> = (0..128).map(|_| rng.next_i32()).collect();
    let cd = c_driver();
    let rd = rust_driver();
    for &x in &inputs {
        let c_out = capture(Sink::File, Buf::Full, || unsafe { cd(x) });
        let r_out = capture(Sink::File, Buf::Full, || unsafe { rd(x) });
        assert_eq!(c_out.len(), 33, "C12: C wrote {} bytes", c_out.len());
        assert_eq!(r_out.len(), 33, "C12: Rust wrote {} bytes", r_out.len());
        assert_eq!(*c_out.last().unwrap(), b'\n', "C12: C missing trailing \\n");
        assert_eq!(*r_out.last().unwrap(), b'\n', "C12: Rust missing trailing \\n");
        validate_structure(&c_out, &[x], "C12/C");
        validate_structure(&r_out, &[x], "C12/Rust");
        assert_eq!(c_out, r_out, "C12/C13: divergence for {x}");
    }
}

fn c14_stdout_file_fully_buffered() {
    let mut rng = Rng::new(SEED ^ 14);
    let inputs: Vec<i32> = (0..64).map(|_| rng.next_i32()).collect();
    diff_all("C14 file/full", Sink::File, Buf::Full, &inputs);
}

fn c15_stdout_pipe_fully_buffered() {
    let mut rng = Rng::new(SEED ^ 15);
    let inputs: Vec<i32> = (0..64).map(|_| rng.next_i32()).collect();
    diff_all("C15 pipe/full", Sink::Pipe, Buf::Full, &inputs);
}

fn c16_stdout_unbuffered() {
    let mut rng = Rng::new(SEED ^ 16);
    let inputs: Vec<i32> = (0..64).map(|_| rng.next_i32()).collect();
    diff_all("C16 file/unbuffered", Sink::File, Buf::Unbuffered, &inputs);
    diff_all("C16 pipe/unbuffered", Sink::Pipe, Buf::Unbuffered, &inputs);
}

fn c17_stdout_line_buffered() {
    let mut rng = Rng::new(SEED ^ 17);
    let inputs: Vec<i32> = (0..64).map(|_| rng.next_i32()).collect();
    diff_all("C17 file/line", Sink::File, Buf::Line, &inputs);
    diff_all("C17 pipe/line", Sink::Pipe, Buf::Line, &inputs);
}

fn c18_repeated_same_input() {
    let inputs = [7i32; 8];
    diff_all("C18 repeated same", Sink::File, Buf::Full, &inputs);
}

fn c19_repeated_differing_inputs() {
    let mut rng = Rng::new(SEED ^ 19);
    let inputs: Vec<i32> = (0..64).map(|_| rng.next_i32()).collect();
    diff_all("C19 repeated differing", Sink::File, Buf::Full, &inputs);
}

fn c20_interleaved_c_and_rust() {
    // C and Rust calls alternating inside ONE buffer window: ordering and
    // buffering through the shared glibc `stdout` must be indistinguishable.
    let mut rng = Rng::new(SEED ^ 20);
    let inputs: Vec<i32> = (0..64).map(|_| rng.next_i32()).collect();
    let cd = c_driver();
    let rd = rust_driver();

    let interleaved = capture(Sink::File, Buf::Full, || {
        for &x in &inputs {
            unsafe {
                cd(x);
                rd(x);
            }
        }
    });
    // Reference: the same sequence produced by C alone (each x twice).
    let c_only = capture(Sink::File, Buf::Full, || {
        for &x in &inputs {
            unsafe {
                cd(x);
                cd(x);
            }
        }
    });
    let rust_only = capture(Sink::File, Buf::Full, || {
        for &x in &inputs {
            unsafe {
                rd(x);
                rd(x);
            }
        }
    });
    assert_eq!(interleaved, c_only, "C20: interleaved C/Rust != C-only");
    assert_eq!(interleaved, rust_only, "C20: interleaved C/Rust != Rust-only");
}

fn c21_buffer_overflow_many_calls() {
    // 400 calls ≈ 13.2 KiB > the 4 KiB stdio buffer ⇒ real write(2) syscalls
    // happen mid-stream in both implementations.
    let mut rng = Rng::new(SEED ^ 21);
    let inputs: Vec<i32> = (0..400).map(|_| rng.next_i32()).collect();
    diff_all("C21 buffer overflow/file", Sink::File, Buf::Full, &inputs);
    diff_all("C21 buffer overflow/pipe", Sink::Pipe, Buf::Full, &inputs);
}

fn c22_unbuffered_interleaved() {
    let mut rng = Rng::new(SEED ^ 22);
    let inputs: Vec<i32> = (0..64).map(|_| rng.next_i32()).collect();
    let cd = c_driver();
    let rd = rust_driver();
    let interleaved = capture(Sink::File, Buf::Unbuffered, || {
        for &x in &inputs {
            unsafe {
                rd(x);
                cd(x);
            }
        }
    });
    let c_only = capture(Sink::File, Buf::Unbuffered, || {
        for &x in &inputs {
            unsafe {
                cd(x);
                cd(x);
            }
        }
    });
    assert_eq!(interleaved, c_only, "C22: unbuffered interleave divergence");
}

// ===========================================================================
// Phase C — ERRORS.md rows
//
// The C error surface is empty (no return value, no asserts, no range checks),
// so "same error" == "same total absence of rejection": both must accept the
// input and emit the same 33 bytes.
// ===========================================================================

/// Assert C and Rust behave identically for a boundary/invalid input.
fn err_same(row: &str, x: i32) {
    let cd = c_driver();
    let rd = rust_driver();
    let c_out = capture(Sink::File, Buf::Full, || unsafe { cd(x) });
    let r_out = capture(Sink::File, Buf::Full, || unsafe { rd(x) });
    assert_eq!(
        c_out,
        r_out,
        "{row}: divergence for {x} (0x{:08x})\n  C   : {}\n  Rust: {}",
        x as u32,
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out)
    );
    // Neither side rejected the input: both produced a full 33-byte record.
    assert_eq!(c_out.len(), 33, "{row}: C rejected/short output");
    assert_eq!(r_out.len(), 33, "{row}: Rust rejected/short output");
    assert_eq!(c_out, expected_line(x), "{row}: C vs model");
}

fn err_e1_int_min() {
    err_same("E1 INT_MIN", i32::MIN);
}

fn err_e2_int_max() {
    err_same("E2 INT_MAX", i32::MAX);
}

fn err_e3_minus_one() {
    err_same("E3 -1", -1);
}

fn err_e4_zero() {
    err_same("E4 0", 0);
}

fn err_e5_one_step_inside_range() {
    for x in [i32::MIN + 1, i32::MAX - 1, -2147483647] {
        err_same("E5 one step inside range", x);
    }
}

fn err_e6_ffi_arg_width_truncation() {
    // Out-of-range values pushed across the FFI boundary: the argument register
    // holds 64 bits whose upper half is non-zero. Both sides must see only the
    // low 32 bits, and must agree.
    type Driver64 = unsafe extern "C" fn(i64);
    let cd: Driver64 = unsafe { std::mem::transmute(c_driver()) };
    let rd: Driver64 = unsafe { std::mem::transmute(rust_driver()) };

    let wide: [i64; 8] = [
        0x1_0000_0000,
        -1,
        0x7fff_ffff_ffff_ffff,
        i64::MIN,
        0xDEAD_BEEF_0000_0007u64 as i64,
        0xFFFF_FFFF_0000_0000u64 as i64,
        0x0000_0001_8000_0000u64 as i64,
        0xCCCC_CCCC_CCCC_CCCCu64 as i64,
    ];
    for w in wide {
        let c_out = capture(Sink::File, Buf::Full, || unsafe { cd(w) });
        let r_out = capture(Sink::File, Buf::Full, || unsafe { rd(w) });
        assert_eq!(
            c_out,
            r_out,
            "E6: divergence for wide arg 0x{:016x}\n  C   : {}\n  Rust: {}",
            w as u64,
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
        // And both must equal the truncated-to-int behaviour.
        let truncated = w as u64 as u32 as i32;
        assert_eq!(
            c_out,
            expected_line(truncated),
            "E6: C did not truncate 0x{:016x} to {truncated}",
            w as u64
        );
    }
}

fn err_e7_out_of_range_enum_values() {
    // C enums accept any int: values with "no valid variant" must be handled
    // identically by both sides rather than rejected.
    for x in [
        0x7fff_ffffu32 as i32,
        0x8000_0000u32 as i32,
        -99999,
        0xcccc_ccccu32 as i32,
        0xdead_beefu32 as i32,
        i32::MIN,
        123456789,
        -123456789,
    ] {
        err_same("E7 out-of-range enum-style value", x);
    }
}

fn err_e8_print_hex_not_reachable_via_abi() {
    // `print_hex` is `static` in C ⇒ not in the ABI, so its `p == NULL` /
    // `len <= 0` / oversized-`len` paths are unreachable for any caller.
    // The Rust `.so` must not widen the ABI by exporting it either.
    unsafe {
        let c_has: Result<Symbol<*const c_void>, _> = libs().c.get(b"print_hex\0");
        let r_has: Result<Symbol<*const c_void>, _> = libs().rust.get(b"print_hex\0");
        assert!(c_has.is_err(), "C .so unexpectedly exports print_hex");
        assert!(
            r_has.is_err(),
            "Rust .so exports print_hex but the C .so does not (ABI widened)"
        );
    }
    // Same for the struct/typedef and anything else: only `driver` is public.
    for name in [b"house_t\0".as_ref(), b"HouseT\0".as_ref(), b"main\0".as_ref()] {
        unsafe {
            let c_has: Result<Symbol<*const c_void>, _> = libs().c.get(name);
            let r_has: Result<Symbol<*const c_void>, _> = libs().rust.get(name);
            assert_eq!(
                c_has.is_ok(),
                r_has.is_ok(),
                "symbol visibility mismatch for {:?}",
                String::from_utf8_lossy(name)
            );
        }
    }
}

fn err_e9_no_latched_error_state() {
    // After every boundary/extreme input, a subsequent ordinary call must be
    // unaffected: no hidden error state is latched on either side.
    let cd = c_driver();
    let rd = rust_driver();
    let probes = [i32::MIN, i32::MAX, -1, 0, 0xcccc_ccccu32 as i32];
    for p in probes {
        let seq = [p, 42, p, -42, 0];
        let c_out = capture(Sink::File, Buf::Full, || {
            for &x in &seq {
                unsafe { cd(x) }
            }
        });
        let r_out = capture(Sink::File, Buf::Full, || {
            for &x in &seq {
                unsafe { rd(x) }
            }
        });
        assert_eq!(c_out, r_out, "E9: divergence after probe {p}");
        assert_eq!(c_out, expected_stream(&seq), "E9: C vs model after {p}");
    }
}

// ===========================================================================
// Phase D — symbol parity through the loaded objects
// ===========================================================================

fn d1_exported_symbol_parity() {
    // Every symbol the C `.so` exports must resolve in the Rust `.so` too.
    // (`nm -D --defined-only` on the C object yields exactly `driver`; see
    // SYMBOLS.md.)
    for name in [b"driver\0".as_ref()] {
        unsafe {
            let c_sym: Symbol<*const c_void> = libs()
                .c
                .get(name)
                .unwrap_or_else(|e| panic!("C .so missing {:?}: {e}", String::from_utf8_lossy(name)));
            let r_sym: Symbol<*const c_void> = libs()
                .rust
                .get(name)
                .unwrap_or_else(|e| panic!("Rust .so missing {:?}: {e}", String::from_utf8_lossy(name)));
            assert!(!c_sym.into_raw().is_null());
            assert!(!r_sym.into_raw().is_null());
        }
    }
}

// ===========================================================================
// Sequential runner (`harness = false`)
//
// The library's observable behaviour is what it writes to fd 1, and capturing
// fd 1 is process-global, so tests MUST NOT run concurrently. Owning `main`
// guarantees that regardless of any `--test-threads` argument.
// ===========================================================================

fn main() {
    let tests: Vec<(&str, fn())> = vec![
        // Phase B — CONFIGS.md rows C1..C22
        ("c1_zero", c1_zero),
        ("c2_minus_one", c2_minus_one),
        ("c3_small_positive_randomized", c3_small_positive_randomized),
        ("c4_small_negative_randomized", c4_small_negative_randomized),
        ("c5_all_bytes_low_nibble_randomized", c5_all_bytes_low_nibble_randomized),
        ("c6_all_bytes_high_bit_randomized", c6_all_bytes_high_bit_randomized),
        ("c7_range_extremes", c7_range_extremes),
        ("c8_special_bytes_randomized", c8_special_bytes_randomized),
        ("c9_uniform_random_full_domain", c9_uniform_random_full_domain),
        ("c10_byte_position_sweep_ff", c10_byte_position_sweep_ff),
        ("c11_byte_position_sweep_01", c11_byte_position_sweep_01),
        ("c12_c13_output_shape_and_constant_fields", c12_c13_output_shape_and_constant_fields),
        ("c14_stdout_file_fully_buffered", c14_stdout_file_fully_buffered),
        ("c15_stdout_pipe_fully_buffered", c15_stdout_pipe_fully_buffered),
        ("c16_stdout_unbuffered", c16_stdout_unbuffered),
        ("c17_stdout_line_buffered", c17_stdout_line_buffered),
        ("c18_repeated_same_input", c18_repeated_same_input),
        ("c19_repeated_differing_inputs", c19_repeated_differing_inputs),
        ("c20_interleaved_c_and_rust", c20_interleaved_c_and_rust),
        ("c21_buffer_overflow_many_calls", c21_buffer_overflow_many_calls),
        ("c22_unbuffered_interleaved", c22_unbuffered_interleaved),
        // Phase C — ERRORS.md rows E1..E9
        ("err_e1_int_min", err_e1_int_min),
        ("err_e2_int_max", err_e2_int_max),
        ("err_e3_minus_one", err_e3_minus_one),
        ("err_e4_zero", err_e4_zero),
        ("err_e5_one_step_inside_range", err_e5_one_step_inside_range),
        ("err_e6_ffi_arg_width_truncation", err_e6_ffi_arg_width_truncation),
        ("err_e7_out_of_range_enum_values", err_e7_out_of_range_enum_values),
        ("err_e8_print_hex_not_reachable_via_abi", err_e8_print_hex_not_reachable_via_abi),
        ("err_e9_no_latched_error_state", err_e9_no_latched_error_state),
        // Phase D — symbol parity
        ("d1_exported_symbol_parity", d1_exported_symbol_parity),
    ];

    // Accept an optional substring filter, like libtest does.
    let filters: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| !a.starts_with('-'))
        .collect();

    println!("\nrunning {} tests (sequential harness)", tests.len());
    let mut passed = 0usize;
    let mut skipped = 0usize;
    let mut failed: Vec<&str> = Vec::new();

    for (name, f) in tests {
        if !filters.is_empty() && !filters.iter().any(|q| name.contains(q.as_str())) {
            skipped += 1;
            continue;
        }
        print!("test {name} ... ");
        {
            use std::io::Write;
            let _ = std::io::stdout().flush();
        }
        match std::panic::catch_unwind(f) {
            Ok(()) => {
                println!("ok");
                passed += 1;
            }
            Err(_) => {
                println!("FAILED");
                failed.push(name);
            }
        }
        {
            use std::io::Write;
            let _ = std::io::stdout().flush();
        }
    }

    println!();
    if failed.is_empty() {
        println!(
            "test result: ok. {passed} passed; 0 failed; {skipped} ignored; 0 measured; 0 filtered out"
        );
    } else {
        println!(
            "test result: FAILED. {passed} passed; {} failed; {skipped} ignored; 0 measured; 0 filtered out",
            failed.len()
        );
        for name in &failed {
            println!("    failed: {name}");
        }
        std::process::exit(101);
    }
}
