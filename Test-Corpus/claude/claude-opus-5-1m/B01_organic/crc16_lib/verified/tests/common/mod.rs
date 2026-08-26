//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and calls `crc16` only via its
//! exported dynamic symbol — never by linking the Rust crate directly. That way
//! the `#[unsafe(no_mangle)] extern "C"` wrapper is part of what is under test,
//! exactly as an external C consumer would see it.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// ABI of the one and only exported symbol:
/// `tflac_u16 crc16(const tflac_u8 *d, tflac_u32 len, tflac_u16 crc16);`
pub type Crc16Fn = unsafe extern "C" fn(*const u8, u32, u16) -> u16;

pub struct Libs {
    // Kept alive for as long as the process runs so the fn pointers stay valid.
    _c_lib: Library,
    _rust_lib: Library,
    c_fn: Crc16Fn,
    rust_fn: Crc16Fn,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn first_existing(candidates: &[PathBuf], what: &str, hint: &str) -> PathBuf {
    for c in candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "could not locate the {what} shared object.\nLooked in:\n{}\n\n{hint}",
        candidates
            .iter()
            .map(|p| format!("  {}", p.display()))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// Allow pointing the harness at a specific artifact (e.g. the release cdylib).
fn env_override(var: &str) -> Option<PathBuf> {
    let p = PathBuf::from(std::env::var_os(var)?);
    assert!(
        p.is_file(),
        "{var} is set to {} but that file does not exist",
        p.display()
    );
    Some(p)
}

fn c_so_path() -> PathBuf {
    if let Some(p) = env_override("CRC16_C_SO") {
        return p;
    }
    let root = manifest_dir();
    let names = [
        "libtranslated_rust.so",
        "libc_src.so",
        "libtranslated_rust.dylib",
    ];
    let dirs = [
        root.join("c_src/build"),
        root.join("c_src/build/Debug"),
        root.join("c_src/build/Release"),
    ];
    let mut cands = Vec::new();
    for d in &dirs {
        for n in &names {
            cands.push(d.join(n));
        }
    }
    first_existing(
        &cands,
        "C",
        "Build it with:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
    )
}

fn rust_so_path() -> PathBuf {
    if let Some(p) = env_override("CRC16_RUST_SO") {
        return p;
    }
    let root = manifest_dir();
    // `cargo test` places the cdylib next to the test binaries' profile dir.
    // Derive the profile dir from the running test executable when possible so
    // that --release / custom target dirs keep working.
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        // .../target/<profile>/deps/<test-bin>
        if let Some(deps) = exe.parent() {
            dirs.push(deps.to_path_buf());
            if let Some(profile) = deps.parent() {
                dirs.push(profile.to_path_buf());
            }
        }
    }
    dirs.push(root.join("target/debug"));
    dirs.push(root.join("target/release"));

    let names = ["libcrc16_lib.so", "libcrc16_lib.dylib", "crc16_lib.dll"];
    let mut cands = Vec::new();
    for d in &dirs {
        for n in &names {
            cands.push(d.join(n));
        }
    }
    first_existing(
        &cands,
        "Rust cdylib",
        "Build it with:\n  cargo build",
    )
}

fn load(path: &Path) -> (Library, Crc16Fn) {
    // SAFETY: loading a trusted, locally built shared object.
    let lib = unsafe { Library::new(path) }
        .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
    let f = {
        // SAFETY: the symbol's real type matches `Crc16Fn` (see lib.h).
        let sym: Symbol<Crc16Fn> = unsafe { lib.get(b"crc16\0") }.unwrap_or_else(|e| {
            panic!("symbol `crc16` missing from {}: {e}", path.display())
        });
        *sym
    };
    (lib, f)
}

/// Newest mtime among the files matching `ext` under `dir` (recursively).
fn newest_source_mtime(dir: &Path, exts: &[&str]) -> Option<std::time::SystemTime> {
    let mut newest: Option<std::time::SystemTime> = None;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                // Never descend into build output.
                let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
                if name != "build" && name != "target" {
                    stack.push(p);
                }
            } else if p
                .extension()
                .map(|x| exts.iter().any(|e| *e == x))
                .unwrap_or(false)
            {
                if let Ok(m) = p.metadata().and_then(|m| m.modified()) {
                    if newest.is_none() || Some(m) > newest {
                        newest = Some(m);
                    }
                }
            }
        }
    }
    newest
}

/// `cargo test` does NOT build the `cdylib` artifact — integration tests cannot
/// link a cdylib, so Cargo skips producing it and the test would silently load a
/// stale `.so` from an earlier `cargo build`. That would make every differential
/// assertion meaningless. Fail loudly instead.
fn assert_fresh(so: &Path, src_dir: &Path, exts: &[&str], rebuild_cmd: &str) {
    let so_m = match so.metadata().and_then(|m| m.modified()) {
        Ok(m) => m,
        Err(_) => return,
    };
    let Some(src_m) = newest_source_mtime(src_dir, exts) else {
        return;
    };
    if src_m > so_m {
        panic!(
            "STALE ARTIFACT: {}\n  was built at {:?}\n  but {} contains a newer source \
             ({:?}).\n\nThe differential test would compare against out-of-date code.\n\
             Rebuild it first:\n  {rebuild_cmd}\n",
            so.display(),
            so_m,
            src_dir.display(),
            src_m
        );
    }
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let root = manifest_dir();
        let c_path = c_so_path();
        let rust_path = rust_so_path();

        assert_fresh(
            &rust_path,
            &root.join("src"),
            &["rs"],
            "cargo build   # then re-run: cargo test",
        );
        assert_fresh(
            &c_path,
            &root.join("c_src"),
            &["c", "h"],
            "cd c_src/build && cmake --build .",
        );

        let (c_lib, c_fn) = load(&c_path);
        let (rust_lib, rust_fn) = load(&rust_path);
        Libs {
            _c_lib: c_lib,
            _rust_lib: rust_lib,
            c_fn,
            rust_fn,
            c_path,
            rust_path,
        }
    })
}

impl Libs {
    /// Call the C `crc16` over a slice.
    pub fn c(&self, data: &[u8], seed: u16) -> u16 {
        // SAFETY: `data.len()` bytes are readable at `data.as_ptr()`.
        unsafe { (self.c_fn)(data.as_ptr(), data.len() as u32, seed) }
    }

    /// Call the Rust `crc16` over a slice, via its exported symbol.
    pub fn rust(&self, data: &[u8], seed: u16) -> u16 {
        // SAFETY: as above.
        unsafe { (self.rust_fn)(data.as_ptr(), data.len() as u32, seed) }
    }

    /// Raw call, for deliberately odd pointer/length combinations.
    ///
    /// # Safety
    /// `ptr`/`len` must describe a region both implementations may legally read
    /// (or `len == 0`, in which case `ptr` is never dereferenced).
    pub unsafe fn c_raw(&self, ptr: *const u8, len: u32, seed: u16) -> u16 {
        unsafe { (self.c_fn)(ptr, len, seed) }
    }

    /// # Safety
    /// See [`Libs::c_raw`].
    pub unsafe fn rust_raw(&self, ptr: *const u8, len: u32, seed: u16) -> u16 {
        unsafe { (self.rust_fn)(ptr, len, seed) }
    }

    /// Assert both libraries agree, with a diagnostic that identifies the case.
    #[track_caller]
    pub fn assert_same(&self, data: &[u8], seed: u16, ctx: &str) -> u16 {
        let c = self.c(data, seed);
        let r = self.rust(data, seed);
        assert_eq!(
            c, r,
            "divergence [{ctx}]: len={} seed=0x{seed:04x} C=0x{c:04x} Rust=0x{r:04x}\n  data={}",
            data.len(),
            preview(data)
        );
        c
    }

    /// Feed `chunks` sequentially, threading the result in as the next seed.
    /// Returns `(c_result, rust_result)`.
    pub fn chained(&self, chunks: &[&[u8]], seed: u16) -> (u16, u16) {
        let mut c = seed;
        let mut r = seed;
        for ch in chunks {
            c = self.c(ch, c);
            r = self.rust(ch, r);
        }
        (c, r)
    }
}

pub fn preview(data: &[u8]) -> String {
    const N: usize = 48;
    let head: String = data.iter().take(N).map(|b| format!("{b:02x}")).collect();
    if data.len() > N {
        format!("{head}... ({} bytes)", data.len())
    } else {
        head
    }
}

/// Deterministic splitmix64 PRNG — fixed seed, reproducible across runs.
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
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_u16(&mut self) -> u16 {
        (self.next_u64() >> 48) as u16
    }
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// Uniform-ish value in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.next_u8()).collect()
    }
}

/// The canonical fixed seed used by every randomized row.
pub const SEED: u64 = 0x2545_F491_4F6C_DD1D;
