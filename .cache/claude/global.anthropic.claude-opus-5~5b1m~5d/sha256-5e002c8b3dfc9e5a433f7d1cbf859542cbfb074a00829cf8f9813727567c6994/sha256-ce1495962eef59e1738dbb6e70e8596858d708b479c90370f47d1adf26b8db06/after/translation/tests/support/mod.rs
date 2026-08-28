//! Shared harness for the differential tests.
//!
//! Both implementations are reached **only** through `dlopen`/`dlsym` on their
//! respective shared objects, so the `#[no_mangle] extern "C"` export wrapper
//! of the Rust crate is under test just like the C entry point.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub type Pow43 = unsafe extern "C" fn(std::ffi::c_int) -> f32;

/// The `g_pow43` table has `129 + 16` entries.
pub const TABLE_LEN: usize = 129 + 16;

/// Lowest `x` for which the C's `g_pow43[16 + x]` subscript is still `>= 0`.
pub const DOMAIN_LO: i32 = -16;
/// Highest `x` for which the C's computed subscript is still `<= TABLE_LEN - 1`.
///
/// `x = 8192..=8223` all yield index `144`; `x = 8224` flips `sign` to 64 and
/// yields index `145`, one past the end.
pub const DOMAIN_HI: i32 = 8223;

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // .../<root>/translation/Cargo.toml -> .../<root>
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn find_so_in(dir: &Path) -> Option<PathBuf> {
    let mut found: Option<PathBuf> = None;
    for entry in std::fs::read_dir(dir).ok()? {
        let path = entry.ok()?.path();
        let name = path.file_name()?.to_str()?.to_owned();
        if name.starts_with("lib") && name.ends_with(".so") && path.is_file() {
            // Prefer a deterministic pick if several exist.
            if found.as_ref().is_none_or(|f| {
                f.file_name().unwrap().to_str().unwrap() > name.as_str()
            }) {
                found = Some(path);
            }
        }
    }
    found
}

/// Path to the C shared object, built by
/// `cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`
///
/// The library name is derived by CMake from the *parent directory name*, so it
/// is discovered rather than hard-coded.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("POW43_C_SO") {
        return PathBuf::from(p);
    }
    let build = workspace_root().join("c_src").join("build");
    find_so_in(&build).unwrap_or_else(|| {
        panic!(
            "no C .so found in {}. Build it with:\n  cd c_src && mkdir -p build && cd build && \\\n    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

/// Path to the Rust `cdylib`.
///
/// `cargo test` does not emit the `cdylib` artifact (the crate only declares
/// `crate-type = ["cdylib"]`, and test harnesses link the rlib), so the object
/// is produced on demand with a nested `cargo build` into a **separate**
/// `--target-dir` to avoid contending with the outer invocation's lock.
///
/// `POW43_RUST_SO` overrides the whole mechanism; `POW43_CARGO_FEATURE_ARGS`
/// (whitespace separated) is forwarded to the nested build so that the feature
/// matrix driver can build each combination.
pub fn rust_so_path() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        if let Ok(p) = std::env::var("POW43_RUST_SO") {
            return PathBuf::from(p);
        }

        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let target_dir = manifest.join("target").join("ffi-so");
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

        let mut cmd = std::process::Command::new(cargo);
        cmd.arg("build")
            .arg("--release")
            .arg("--manifest-path")
            .arg(manifest.join("Cargo.toml"))
            .arg("--target-dir")
            .arg(&target_dir);
        if let Ok(extra) = std::env::var("POW43_CARGO_FEATURE_ARGS") {
            for a in extra.split_whitespace() {
                cmd.arg(a);
            }
        }
        // Do not inherit the test harness's rustflags/profile overrides.
        cmd.env_remove("RUSTFLAGS")
            .env_remove("CARGO_ENCODED_RUSTFLAGS");

        let status = cmd.status();
        let built = target_dir.join("release");
        if let Some(p) = find_so_in(&built) {
            if matches!(&status, Ok(s) if s.success()) || p.is_file() {
                return p;
            }
        }
        // Fall back to a pre-existing `cargo build --release` artifact.
        for profile in ["release", "debug"] {
            if let Some(p) = find_so_in(&manifest.join("target").join(profile)) {
                return p;
            }
        }
        panic!(
            "could not obtain the Rust cdylib; nested build status = {status:?}. \
             Run `cargo build --release` in translation/ or set POW43_RUST_SO."
        );
    })
    .clone()
}

// ---------------------------------------------------------------------------
// The loaded pair
// ---------------------------------------------------------------------------

pub struct Libs {
    _c: libloading::Library,
    _rust: libloading::Library,
    pub c: Pow43,
    pub rust: Pow43,
    /// Run-time located `g_pow43` base pointer + readable-mapping bounds, per
    /// object. Used by the error-path tests to build a per-object oracle for
    /// the C's unchecked (out-of-bounds) subscript.
    pub c_table: Option<TableView>,
    pub rust_table: Option<TableView>,
}

/// Address of an object's private `g_pow43`, plus the bounds of the readable
/// mapping it lives in. Stored as `usize` so the enclosing `Libs` stays
/// `Send + Sync` and can live in a `OnceLock`.
#[derive(Clone, Copy, Debug)]
pub struct TableView {
    pub base: usize,
    pub region_start: usize,
    pub region_end: usize,
}

impl TableView {
    /// Address the C would load from for subscript `idx`, or `None` if that
    /// address is not inside *any* readable mapping of the process (in which
    /// case a real call would trap, exactly as the C program would, and must
    /// not be attempted).
    pub fn readable_at(&self, idx: i32) -> Option<*const f32> {
        // Byte offset exactly as C computes it: `sizeof(float) * idx`, signed.
        let byte_off = (idx as isize).wrapping_mul(4);
        let addr = self.base.wrapping_add(byte_off as usize);
        if is_readable(addr, 4) {
            Some(addr as *const f32)
        } else {
            None
        }
    }
}

/// Snapshot of every readable mapping of the process, taken once after both
/// objects are `dlopen`ed. Used as a fast pre-filter for deciding whether an
/// out-of-bounds subscript can be dereferenced without trapping.
///
/// A file-backed mapping is **truncated to the pages that are actually backed
/// by file bytes**: `mmap` rounds the mapping length up to a page, and touching
/// a page that lies entirely past EOF raises `SIGBUS` even though
/// `/proc/self/maps` reports the range as readable. The `.so` objects here are
/// exactly that shape, so ignoring this would crash the test process.
fn readable_mappings() -> &'static [(usize, usize)] {
    static MAPS: OnceLock<Vec<(usize, usize)>> = OnceLock::new();
    MAPS.get_or_init(|| {
        let page = 4096usize;
        let mut v = Vec::new();
        if let Ok(maps) = std::fs::read_to_string("/proc/self/maps") {
            for line in maps.lines() {
                let cols: Vec<&str> = line.split_whitespace().collect();
                if cols.len() < 5 || !cols[1].starts_with('r') {
                    continue;
                }
                let Some((s, e)) = cols[0].split_once('-') else {
                    continue;
                };
                let (Ok(a), Ok(mut b)) =
                    (usize::from_str_radix(s, 16), usize::from_str_radix(e, 16))
                else {
                    continue;
                };
                if b <= a {
                    continue;
                }
                // File-backed? (inode != 0 and a pathname that stats)
                let inode = cols[4];
                if inode != "0" && cols.len() >= 6 {
                    let path = cols[5..].join(" ");
                    let file_off = usize::from_str_radix(cols[2], 16).unwrap_or(0);
                    if let Ok(md) = std::fs::metadata(&path) {
                        let size = md.len() as usize;
                        let backed = size.saturating_sub(file_off);
                        // Whole pages plus the partial EOF page are readable.
                        let limit = a.saturating_add(backed.div_ceil(page) * page);
                        b = b.min(limit);
                    }
                }
                if b > a {
                    v.push((a, b));
                }
            }
        }
        v.sort_unstable();
        // Merge adjacent/overlapping ranges so a subscript that walks from one
        // mapping of the same file into the next is still recognised.
        let mut merged: Vec<(usize, usize)> = Vec::with_capacity(v.len());
        for (a, b) in v {
            match merged.last_mut() {
                Some(last) if a <= last.1 => last.1 = last.1.max(b),
                _ => merged.push((a, b)),
            }
        }
        merged
    })
}

fn in_readable_mapping(addr: usize, end: usize) -> bool {
    let m = readable_mappings();
    m.binary_search_by(|&(a, b)| {
        if b <= addr {
            std::cmp::Ordering::Less
        } else if a > addr {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    })
    .is_ok_and(|i| end <= m[i].1)
}

unsafe extern "C" {
    fn pipe(fds: *mut i32) -> i32;
    fn write(fd: i32, buf: *const u8, n: usize) -> isize;
    fn read(fd: i32, buf: *mut u8, n: usize) -> isize;
    fn mmap(
        addr: *mut u8,
        len: usize,
        prot: i32,
        flags: i32,
        fd: i32,
        off: i64,
    ) -> *mut u8;
    fn munmap(addr: *mut u8, len: usize) -> i32;
    fn fork() -> i32;
    fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
    fn _exit(code: i32) -> !;
}

pub const PAGE: usize = 4096;
const PROT_READ: i32 = 1;
const PROT_WRITE: i32 = 2;
const MAP_PRIVATE: i32 = 0x02;
const MAP_ANONYMOUS: i32 = 0x20;
const MAP_FIXED_NOREPLACE: i32 = 0x100000;

/// A page mapped at an exact address, unmapped on drop.
pub struct FixedPage(*mut u8);

impl FixedPage {
    /// Map one writable page at exactly `page_addr`, or `None` if that address
    /// is already occupied (`MAP_FIXED_NOREPLACE` ⇒ `EEXIST`).
    pub fn at(page_addr: usize) -> Option<Self> {
        if page_addr == 0 || page_addr % PAGE != 0 {
            return None;
        }
        // SAFETY: `MAP_FIXED_NOREPLACE` guarantees an existing mapping is never
        // clobbered; on collision `mmap` fails instead.
        let p = unsafe {
            mmap(
                page_addr as *mut u8,
                PAGE,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE,
                -1,
                0,
            )
        };
        if p as isize == -1 || p as usize != page_addr {
            if p as isize != -1 {
                // SAFETY: the kernel gave us this mapping; hand it straight back.
                unsafe { munmap(p, PAGE) };
            }
            return None;
        }
        Some(FixedPage(p))
    }

    pub fn addr(&self) -> usize {
        self.0 as usize
    }
}

impl Drop for FixedPage {
    fn drop(&mut self) {
        // SAFETY: `self.0` is a page this object mapped and still owns.
        unsafe { munmap(self.0, PAGE) };
    }
}

/// Value written into a synthetic out-of-bounds slot, as a function of the slot's
/// index *relative to that object's own table base*.
///
/// Deliberately a hash rather than `rel as f32`: indices in the deep
/// out-of-bounds region exceed 2^24, where consecutive integers are not
/// distinguishable in `f32`. The result is always finite, normal and in
/// `[0.5, 1.0)`, so multiplying by `poly * mult` can neither overflow nor flush
/// to zero.
pub fn slot_value(rel: i64) -> f32 {
    let mut z = (rel as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 29)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z ^= z >> 32;
    f32::from_bits((z as u32 & 0x007F_FFFF) | 0x3F00_0000)
}

/// Outcome of calling `pow43` in a forked child.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CallOutcome {
    /// Returned normally; payload is the raw `f32` bits.
    Returned(u32),
    /// Killed by a signal (e.g. `SIGSEGV` = 11, `SIGBUS` = 7).
    Signal(i32),
    /// Could not be determined.
    Unknown,
}

/// Call `f(x)` in a forked child so that a fault does not take the test process
/// down, and report how the call ended.
///
/// This is what makes "both objects reject this input the same way" checkable
/// for the inputs where the C's unchecked subscript leaves every mapped page:
/// the rejection *is* the fatal signal.
pub fn call_in_child(f: Pow43, x: i32) -> CallOutcome {
    let mut fds = [-1i32; 2];
    // SAFETY: `fds` is a valid 2-element `int` array.
    if unsafe { pipe(fds.as_mut_ptr()) } != 0 {
        return CallOutcome::Unknown;
    }
    let (rd, wr) = (fds[0], fds[1]);
    // SAFETY: the child only performs async-signal-safe work (one FFI call, one
    // `write`, `_exit`) before terminating, which is legal after `fork` in a
    // multithreaded process.
    let pid = unsafe { fork() };
    if pid == 0 {
        let bits = unsafe { f(x) }.to_bits();
        unsafe {
            write(wr, bits.to_le_bytes().as_ptr(), 4);
            _exit(0)
        };
    }
    // SAFETY: closing our copy of the write end so `read` sees EOF.
    unsafe { libc_close(wr) };
    let mut buf = [0u8; 4];
    // SAFETY: `buf` is a valid 4-byte destination.
    let got = unsafe { read(rd, buf.as_mut_ptr(), 4) };
    let mut status = 0i32;
    // SAFETY: `status` is a valid `int` destination.
    unsafe { waitpid(pid, &mut status, 0) };
    // SAFETY: our own pipe read end.
    unsafe { libc_close(rd) };

    if status & 0x7f != 0 {
        return CallOutcome::Signal(status & 0x7f);
    }
    if got == 4 {
        return CallOutcome::Returned(u32::from_le_bytes(buf));
    }
    CallOutcome::Unknown
}

unsafe extern "C" {
    #[link_name = "close"]
    fn libc_close(fd: i32) -> i32;
}

/// Authoritative, non-faulting readability probe.
///
/// The kernel copies from user space when servicing `write(2)`, so a bad source
/// address is reported as `-EFAULT` instead of delivering `SIGSEGV`/`SIGBUS` to
/// us. A per-thread pipe is used as the sink and immediately drained.
fn probe_readable(addr: usize, len: usize) -> bool {
    use std::cell::RefCell;
    thread_local! {
        static PIPE: RefCell<Option<(i32, i32)>> = const { RefCell::new(None) };
    }
    PIPE.with(|p| {
        let mut slot = p.borrow_mut();
        if slot.is_none() {
            let mut fds = [-1i32; 2];
            // SAFETY: `fds` is a valid 2-element array of `int`.
            if unsafe { pipe(fds.as_mut_ptr()) } != 0 {
                return false;
            }
            *slot = Some((fds[0], fds[1]));
        }
        let (rd, wr) = slot.unwrap();
        // SAFETY: the point of the call is to let the *kernel* validate `addr`;
        // an invalid address yields -1/EFAULT rather than a fault in this
        // process.
        let n = unsafe { write(wr, addr as *const u8, len) };
        if n > 0 {
            let mut sink = [0u8; 64];
            let mut left = n as usize;
            while left > 0 {
                // SAFETY: `sink` is a valid, writable buffer of 64 bytes.
                let got = unsafe { read(rd, sink.as_mut_ptr(), left.min(sink.len())) };
                if got <= 0 {
                    break;
                }
                left -= got as usize;
            }
        }
        n == len as isize
    })
}

/// True when `[addr, addr + len)` can be dereferenced without trapping.
pub fn is_readable(addr: usize, len: usize) -> bool {
    let Some(end) = addr.checked_add(len) else {
        return false;
    };
    in_readable_mapping(addr, end) && probe_readable(addr, len)
}

/// `dlopen` both objects exactly once per test process.
pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        unsafe {
            let c_lib = libloading::Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", c_path.display()));
            let rust_lib = libloading::Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", rust_path.display()));

            let c_sym: libloading::Symbol<Pow43> = c_lib
                .get(b"pow43\0")
                .unwrap_or_else(|e| panic!("dlsym pow43 in C .so failed: {e}"));
            let rust_sym: libloading::Symbol<Pow43> = rust_lib
                .get(b"pow43\0")
                .unwrap_or_else(|e| panic!("dlsym pow43 in Rust .so failed: {e}"));

            let c = *c_sym;
            let rust = *rust_sym;

            let c_table = locate_table(&c_path);
            let rust_table = locate_table(&rust_path);
            // Snapshot the mappings *after* both objects are loaded, so the
            // readability oracle sees their segments.
            let _ = readable_mappings();

            Libs {
                c: c,
                rust: rust,
                c_table,
                rust_table,
                _c: c_lib,
                _rust: rust_lib,
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Locating the private `g_pow43` inside a loaded object
// ---------------------------------------------------------------------------

/// The expected contents of `g_pow43`, transcribed from `c_src/src/lib.c`.
/// Used purely as a search needle / oracle input; it is *not* the value under
/// test (both objects carry their own copy).
pub const G_POW43: [f32; TABLE_LEN] = [
    0.0, -1.0, -2.519842, -4.326749, -6.349604, -8.549880, -10.902724, -13.390518, -16.000000,
    -18.720754, -21.544347, -24.463781, -27.473142, -30.567351, -33.741992, -36.993181, 0.0, 1.0,
    2.519842, 4.326749, 6.349604, 8.549880, 10.902724, 13.390518, 16.000000, 18.720754, 21.544347,
    24.463781, 27.473142, 30.567351, 33.741992, 36.993181, 40.317474, 43.711787, 47.173345,
    50.699631, 54.288352, 57.937408, 61.644865, 65.408941, 69.227979, 73.100443, 77.024898,
    81.000000, 85.024491, 89.097188, 93.216975, 97.382800, 101.593667, 105.848633, 110.146801,
    114.487321, 118.869381, 123.292209, 127.755065, 132.257246, 136.798076, 141.376907, 145.993119,
    150.646117, 155.335327, 160.060199, 164.820202, 169.614826, 174.443577, 179.305980, 184.201575,
    189.129918, 194.090580, 199.083145, 204.107210, 209.162385, 214.248292, 219.364564, 224.510845,
    229.686789, 234.892058, 240.126328, 245.389280, 250.680604, 256.000000, 261.347174, 266.721841,
    272.123723, 277.552547, 283.008049, 288.489971, 293.998060, 299.532071, 305.091761, 310.676898,
    316.287249, 321.922592, 327.582707, 333.267377, 338.976394, 344.709550, 350.466646, 356.247482,
    362.051866, 367.879608, 373.730522, 379.604427, 385.501143, 391.420496, 397.362314, 403.326427,
    409.312672, 415.320884, 421.350905, 427.402579, 433.475750, 439.570269, 445.685987, 451.822757,
    457.980436, 464.158883, 470.357960, 476.577530, 482.817459, 489.077615, 495.357868, 501.658090,
    507.978156, 514.317941, 520.677324, 527.056184, 533.454404, 539.871867, 546.308458, 552.764065,
    559.238575, 565.731879, 572.243870, 578.774440, 585.323483, 591.890898, 598.476581, 605.080431,
    611.702349, 618.342238, 625.000000, 631.675540, 638.368763, 645.079578,
];

/// Parse `/proc/self/maps` and scan the object's readable mappings for the
/// 145-`f32` byte pattern of `g_pow43`.
fn locate_table(so_path: &Path) -> Option<TableView> {
    let canon = std::fs::canonicalize(so_path).ok()?;
    let canon = canon.to_str()?.to_owned();
    let maps = std::fs::read_to_string("/proc/self/maps").ok()?;

    let needle: Vec<u8> = G_POW43
        .iter()
        .take(12)
        .flat_map(|f| f.to_bits().to_le_bytes())
        .collect();
    let full: Vec<u8> = G_POW43
        .iter()
        .flat_map(|f| f.to_bits().to_le_bytes())
        .collect();

    for line in maps.lines() {
        let mut it = line.split_whitespace();
        let range = it.next()?;
        let perms = it.next()?;
        let path = line.split_whitespace().nth(5).unwrap_or("");
        if path != canon || !perms.starts_with('r') {
            continue;
        }
        let (s, e) = range.split_once('-')?;
        let start = usize::from_str_radix(s, 16).ok()?;
        let end = usize::from_str_radix(e, 16).ok()?;
        if end <= start || end - start < full.len() {
            continue;
        }
        // SAFETY: the kernel reports this range as mapped and readable.
        let hay: &[u8] = unsafe { std::slice::from_raw_parts(start as *const u8, end - start) };
        let mut off = 0usize;
        while off + full.len() <= hay.len() {
            match find_sub(&hay[off..], &needle) {
                None => break,
                Some(rel) => {
                    let at = off + rel;
                    if (start + at) % 4 == 0
                        && at + full.len() <= hay.len()
                        && &hay[at..at + full.len()] == full.as_slice()
                    {
                        return Some(TableView {
                            base: start + at,
                            region_start: start,
                            region_end: end,
                        });
                    }
                    off = at + 1;
                }
            }
        }
    }
    None
}

fn find_sub(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || hay.len() < needle.len() {
        return None;
    }
    hay.windows(needle.len()).position(|w| w == needle)
}

// ---------------------------------------------------------------------------
// The C algorithm, re-derived in the test as an oracle
// ---------------------------------------------------------------------------

/// Everything `pow43` computes *except* the table load, so that the load can be
/// aimed at a specific object's own table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Decoded {
    /// The C subscript: `16 + x`, or `16 + ((x + sign) >> 6)`.
    pub idx: i32,
    /// `sign = 2 * x & 64` (only meaningful on the computed path).
    pub sign: i32,
    /// `mult` (only meaningful on the computed path).
    pub mult: i32,
    /// `false` for `x < 129`, where the C returns the raw table entry.
    pub computed: bool,
    /// Raw bits of `frac` (only meaningful on the computed path).
    pub frac_bits: u32,
    /// Raw bits of `poly = 1 + frac*(4/3 + frac*(2/9))`.
    pub poly_bits: u32,
}

/// Faithful re-derivation of `c_src/src/lib.c`, using wrapping arithmetic where
/// the C relies on two's-complement wrap-around.
pub fn decode(x_in: i32) -> Decoded {
    let mut x = x_in;
    let mut mult: i32 = 256;
    if x < 129 {
        return Decoded {
            idx: 16i32.wrapping_add(x),
            sign: 0,
            mult: 0,
            computed: false,
            frac_bits: 0,
            poly_bits: 0,
        };
    }
    if x < 1024 {
        mult = 16;
        x = x.wrapping_shl(3);
    }
    let sign = x.wrapping_mul(2) & 64;
    let frac = ((x & 63).wrapping_sub(sign) as f32) / ((x & !63).wrapping_add(sign) as f32);
    let poly = 1.0f32 + frac * ((4.0f32 / 3.0f32) + frac * (2.0f32 / 9.0f32));
    Decoded {
        idx: 16i32.wrapping_add(x.wrapping_add(sign) >> 6),
        sign,
        mult,
        computed: true,
        frac_bits: frac.to_bits(),
        poly_bits: poly.to_bits(),
    }
}

/// The value `pow43(x)` must return in an object whose `g_pow43` lives at
/// `view.base`, including the out-of-bounds region.
///
/// Returns `None` when the C's load address is not inside the object's readable
/// mapping — such an input must not be called at all.
pub fn oracle(x: i32, view: &TableView) -> Option<u32> {
    let d = decode(x);
    let p = view.readable_at(d.idx)?;
    // SAFETY: `readable_at` confirmed the address lies inside a mapped,
    // readable region of the object.
    let entry = unsafe { std::ptr::read_unaligned(p) };
    if !d.computed {
        return Some(entry.to_bits());
    }
    let poly = f32::from_bits(d.poly_bits);
    Some(((entry * poly) * (d.mult as f32)).to_bits())
}

/// True when `x` is inside the domain for which the C's subscript is in bounds
/// (i.e. the region where the C program has no undefined behaviour).
pub fn in_defined_domain(x: i32) -> bool {
    let d = decode(x);
    (0..TABLE_LEN as i32).contains(&d.idx)
}

// ---------------------------------------------------------------------------
// Differential comparison
// ---------------------------------------------------------------------------

/// Call both objects and compare the raw IEEE-754 bits.
#[track_caller]
pub fn assert_same(x: i32) {
    let l = libs();
    let c = unsafe { (l.c)(x) };
    let r = unsafe { (l.rust)(x) };
    if c.to_bits() != r.to_bits() {
        let d = decode(x);
        panic!(
            "pow43({x}) diverges: C = {:#010x} ({c:?}), Rust = {:#010x} ({r:?}); decoded = {d:?}",
            c.to_bits(),
            r.to_bits()
        );
    }
}

#[track_caller]
pub fn assert_same_all<I: IntoIterator<Item = i32>>(xs: I) -> usize {
    let mut n = 0;
    for x in xs {
        assert_same(x);
        n += 1;
    }
    assert!(n > 0, "row exercised zero inputs");
    n
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed, reproducible
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
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
    /// Uniform in `lo..=hi`.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    /// `n` draws from `lo..=hi` kept only when `pred` holds.
    pub fn take_where<F: Fn(i32) -> bool>(
        &mut self,
        n: usize,
        lo: i32,
        hi: i32,
        pred: F,
    ) -> Vec<i32> {
        let mut out = Vec::with_capacity(n);
        let mut guard = 0usize;
        while out.len() < n && guard < n * 4096 {
            let x = self.range(lo, hi);
            if pred(x) {
                out.push(x);
            }
            guard += 1;
        }
        assert_eq!(out.len(), n, "could not draw {n} inputs matching predicate");
        out
    }
}
