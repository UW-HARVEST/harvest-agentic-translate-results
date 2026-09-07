// Shared test support: loads BOTH the C `.so` and the Rust `.so` via
// `libloading` and calls every function through its exported symbol, exactly
// as an external C consumer would. No Rust function is ever called directly.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_double, c_int, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// libc bits we need for byte-exact stdout capture of the libraries' printf().
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

const O_WRONLY: c_int = 0o1;
const O_CREAT: c_int = 0o100;
const O_TRUNC: c_int = 0o1000;

/// C: `typedef struct { int id; double value; char label[20]; } DataBlock;`
/// Verified against the C compiler: size=40 align=8, off_id=0, off_value=8,
/// off_label=16 (4 bytes of padding after `id`, 4 trailing pad bytes).
pub const DATABLOCK_SIZE: usize = 40;
/// Extra guard bytes after the destination struct: `copy_data_block` must
/// write EXACTLY `DATABLOCK_SIZE` bytes, so these must stay untouched and
/// identical between the two implementations.
pub const GUARD: usize = 24;
pub const BUF_SIZE: usize = DATABLOCK_SIZE + GUARD;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DataBlock {
    pub id: c_int,
    pub value: c_double,
    pub label: [c_char; 20],
}

pub type FnOverunder = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
pub type FnSafeDoubleToInt = unsafe extern "C" fn(c_double) -> c_int;
pub type FnProcessWithFallthrough = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type FnCopyDataBlock = unsafe extern "C" fn(*mut c_void, *const c_void);
pub type FnHandlePointerOperations = unsafe extern "C" fn(c_int) -> c_int;

/// One loaded implementation (either the C `.so` or the Rust `.so`).
pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    pub overunder: FnOverunder,
    pub safe_double_to_int: FnSafeDoubleToInt,
    pub process_with_fallthrough: FnProcessWithFallthrough,
    pub copy_data_block: FnCopyDataBlock,
    pub handle_pointer_operations: FnHandlePointerOperations,
}

impl Impl {
    fn load(name: &'static str, path: &PathBuf) -> Impl {
        let lib = unsafe {
            Library::new(path).unwrap_or_else(|e| panic!("dlopen {:?} failed: {e}", path))
        };
        macro_rules! sym {
            ($t:ty, $n:literal) => {{
                let s: Symbol<$t> = unsafe {
                    lib.get($n)
                        .unwrap_or_else(|e| panic!("{} missing symbol {}: {e}", name, unsafe {
                            std::str::from_utf8_unchecked($n)
                        }))
                };
                unsafe { *s.into_raw() }
            }};
        }
        let overunder = sym!(FnOverunder, b"overunder\0");
        let safe_double_to_int = sym!(FnSafeDoubleToInt, b"safe_double_to_int\0");
        let process_with_fallthrough =
            sym!(FnProcessWithFallthrough, b"process_with_fallthrough\0");
        let copy_data_block = sym!(FnCopyDataBlock, b"copy_data_block\0");
        let handle_pointer_operations =
            sym!(FnHandlePointerOperations, b"handle_pointer_operations\0");
        Impl {
            name,
            _lib: lib,
            overunder,
            safe_double_to_int,
            process_with_fallthrough,
            copy_data_block,
            handle_pointer_operations,
        }
    }
}

pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

static PAIR: OnceLock<Pair> = OnceLock::new();

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let build = workspace_root().join("c_src/build");
    let mut found: Option<PathBuf> = None;
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            let n = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            if n.starts_with("lib") && n.ends_with(".so") {
                found = Some(p);
                break;
            }
        }
    }
    found.unwrap_or_else(|| {
        panic!(
            "no C .so found in {:?} — build it with:\n  cd c_src && mkdir -p build && cd build \
             && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build
        )
    })
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    // Prefer the release artifact: that is what an external caller links
    // against, and it is built with the crate's real `[profile.release]`
    // (panic = "abort", overflow-checks off) just like the C build.
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    for prof in ["release", "debug"] {
        let p = base.join(prof).join("liboverunder_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "liboverunder_lib.so not found under {:?} — build it with `cargo build --release`",
        base
    )
}

pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| Pair {
        c: Impl::load("C", &find_c_so()),
        rs: Impl::load("Rust", &find_rust_so()),
    })
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// Runs `f`, capturing everything the loaded library writes to fd 1 via
/// libc `printf`. Both implementations share the same glibc `stdout`, so
/// `fflush(NULL)` before and after gives an exact, self-contained capture.
///
/// fd 1 is process-global, so all captures are serialized behind a mutex
/// (poison is ignored: a failed assertion inside one capture must not make
/// every later test fail for the wrong reason).
pub fn capture_stdout<R, F: FnOnce() -> R>(f: F) -> (R, Vec<u8>) {
    static FD1: OnceLock<std::sync::Mutex<()>> = OnceLock::new();
    let m = FD1.get_or_init(|| std::sync::Mutex::new(()));
    let _guard = match m.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    unsafe {
        // libtest writes its own progress lines to fd 1 through Rust's
        // `std::io::Stdout` (a buffer independent of glibc's `stdout`). Drain
        // both so nothing already pending lands inside the capture.
        use std::io::Write;
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();
        fflush(std::ptr::null_mut()); // drain anything already buffered
        let tmp = std::env::temp_dir().join(format!(
            "cdiff-{}-{:?}.out",
            std::process::id(),
            std::thread::current().id()
        ));
        let mut cpath: Vec<u8> = tmp.to_string_lossy().as_bytes().to_vec();
        cpath.push(0);
        let tmpfd = open(cpath.as_ptr() as *const c_char, O_WRONLY | O_CREAT | O_TRUNC, 0o644 as c_int);
        assert!(tmpfd >= 0, "open temp capture file failed");
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(tmpfd, 1) >= 0, "dup2 failed");
        close(tmpfd);

        let r = f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);

        let bytes = std::fs::read(&tmp).unwrap_or_default();
        let _ = std::fs::remove_file(&tmp);
        (r, bytes)
    }
}

/// Calls `overunder` on both `.so`s and returns
/// `((c_ret, c_stdout), (rs_ret, rs_stdout))`.
pub fn overunder_both(
    a: c_int,
    b: c_int,
    c: c_int,
    d: c_int,
) -> ((c_int, Vec<u8>), (c_int, Vec<u8>)) {
    let p = pair();
    let cres = capture_stdout(|| unsafe { (p.c.overunder)(a, b, c, d) });
    let rres = capture_stdout(|| unsafe { (p.rs.overunder)(a, b, c, d) });
    (cres, rres)
}

/// Asserts return value AND stdout are byte-identical for `overunder`.
#[track_caller]
pub fn assert_overunder_eq(row: &str, a: c_int, b: c_int, c: c_int, d: c_int) {
    let ((cr, cout), (rr, rout)) = overunder_both(a, b, c, d);
    assert_eq!(
        cr, rr,
        "[{row}] overunder({a},{b},{c},{d}) return mismatch: C={cr} Rust={rr}"
    );
    if cout != rout {
        panic!(
            "[{row}] overunder({a},{b},{c},{d}) stdout mismatch:\n--- C ---\n{}\n--- Rust ---\n{}\n\
             (C bytes: {:?})\n(R bytes: {:?})",
            String::from_utf8_lossy(&cout),
            String::from_utf8_lossy(&rout),
            cout,
            rout
        );
    }
}

#[track_caller]
pub fn assert_sdti_eq(row: &str, d: c_double) {
    let p = pair();
    let cv = unsafe { (p.c.safe_double_to_int)(d) };
    let rv = unsafe { (p.rs.safe_double_to_int)(d) };
    assert_eq!(
        cv, rv,
        "[{row}] safe_double_to_int({d:?} bits={:#018x}) C={cv} Rust={rv}",
        d.to_bits()
    );
}

#[track_caller]
pub fn assert_pwf_eq(row: &str, code: c_int, base: c_int) {
    let p = pair();
    let cv = unsafe { (p.c.process_with_fallthrough)(code, base) };
    let rv = unsafe { (p.rs.process_with_fallthrough)(code, base) };
    assert_eq!(
        cv, rv,
        "[{row}] process_with_fallthrough({code},{base}) C={cv} Rust={rv}"
    );
}

#[track_caller]
pub fn assert_hpo_eq(row: &str, v: c_int) {
    let p = pair();
    let cv = unsafe { (p.c.handle_pointer_operations)(v) };
    let rv = unsafe { (p.rs.handle_pointer_operations)(v) };
    assert_eq!(
        cv, rv,
        "[{row}] handle_pointer_operations({v}) C={cv} Rust={rv}"
    );
}

/// Copies a raw `DataBlock` payload through both `.so`s into destinations
/// pre-filled with `fill`, and compares the full struct (padding included)
/// plus `GUARD` trailing bytes, so a wrong copy length is also caught.
#[track_caller]
pub fn assert_copy_eq(row: &str, src: &[u8; DATABLOCK_SIZE], fill: u8) {
    let p = pair();
    // 8-byte aligned backing storage so the `double` field is properly aligned.
    let mut src_al = [0u64; BUF_SIZE / 8];
    let fillw = u64::from_le_bytes([fill; 8]);
    let mut cdst = [fillw; BUF_SIZE / 8];
    let mut rdst = cdst;
    unsafe {
        std::ptr::copy_nonoverlapping(src.as_ptr(), src_al.as_mut_ptr() as *mut u8, DATABLOCK_SIZE);
        (p.c.copy_data_block)(cdst.as_mut_ptr() as *mut c_void, src_al.as_ptr() as *const c_void);
        (p.rs.copy_data_block)(rdst.as_mut_ptr() as *mut c_void, src_al.as_ptr() as *const c_void);
    }
    let cb: &[u8; BUF_SIZE] = unsafe { &*(cdst.as_ptr() as *const [u8; BUF_SIZE]) };
    let rb: &[u8; BUF_SIZE] = unsafe { &*(rdst.as_ptr() as *const [u8; BUF_SIZE]) };
    assert_eq!(
        &cb[..], &rb[..],
        "[{row}] copy_data_block byte mismatch (fill={fill:#04x}), src={src:?}"
    );
    assert_eq!(
        &cb[..DATABLOCK_SIZE], &src[..],
        "[{row}] copy_data_block did not copy all {DATABLOCK_SIZE} bytes"
    );
    assert_eq!(
        &cb[DATABLOCK_SIZE..], &[fill; GUARD][..],
        "[{row}] copy_data_block wrote past sizeof(DataBlock)"
    );
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64) — fixed seed for reproducibility.
// ---------------------------------------------------------------------------
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Uniform in `[lo, hi]` inclusive.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    pub fn next_f64_bits(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }
    /// A finite double with a randomized exponent, spanning tiny to huge.
    pub fn next_f64_scaled(&mut self) -> f64 {
        let m = (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64; // [0,1)
        let e = self.range_i32(-320, 300);
        let sign = if self.next_u64() & 1 == 0 { 1.0 } else { -1.0 };
        sign * m * 10f64.powi(e)
    }
}
