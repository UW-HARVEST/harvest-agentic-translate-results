//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and calls `bitwriter_add`
//! only through the exported C ABI symbol — never by calling the Rust crate
//! directly. This exercises the `#[no_mangle] extern "C"` wrapper too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::PathBuf;

/// Raw 32-byte image of `struct tflac_bitwriter`, kept as bytes so comparisons
/// are byte-for-byte over the whole object (including any padding).
///
/// Layout verified against the real C header with an `offsetof` probe:
/// `size=32 align=8 val=0 bits=8 pos=12 len=16 tot=20 buffer=24`.
#[repr(C, align(8))]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Bw(pub [u8; 32]);

pub const OFF_VAL: usize = 0;
pub const OFF_BITS: usize = 8;
pub const OFF_POS: usize = 12;
pub const OFF_LEN: usize = 16;
pub const OFF_TOT: usize = 20;
pub const OFF_BUFFER: usize = 24;

impl Bw {
    pub fn zeroed() -> Self {
        Bw([0u8; 32])
    }

    pub fn from_bytes(b: [u8; 32]) -> Self {
        Bw(b)
    }

    pub fn get_u64(&self, off: usize) -> u64 {
        u64::from_ne_bytes(self.0[off..off + 8].try_into().unwrap())
    }
    pub fn get_u32(&self, off: usize) -> u32 {
        u32::from_ne_bytes(self.0[off..off + 4].try_into().unwrap())
    }
    pub fn set_u64(&mut self, off: usize, v: u64) {
        self.0[off..off + 8].copy_from_slice(&v.to_ne_bytes());
    }
    pub fn set_u32(&mut self, off: usize, v: u32) {
        self.0[off..off + 4].copy_from_slice(&v.to_ne_bytes());
    }

    pub fn val(&self) -> u64 {
        self.get_u64(OFF_VAL)
    }
    pub fn bits(&self) -> u32 {
        self.get_u32(OFF_BITS)
    }
    pub fn pos(&self) -> u32 {
        self.get_u32(OFF_POS)
    }
    pub fn len(&self) -> u32 {
        self.get_u32(OFF_LEN)
    }
    pub fn tot(&self) -> u32 {
        self.get_u32(OFF_TOT)
    }
    pub fn buffer(&self) -> u64 {
        self.get_u64(OFF_BUFFER)
    }

    pub fn set_val(&mut self, v: u64) {
        self.set_u64(OFF_VAL, v)
    }
    pub fn set_bits(&mut self, v: u32) {
        self.set_u32(OFF_BITS, v)
    }
    pub fn set_pos(&mut self, v: u32) {
        self.set_u32(OFF_POS, v)
    }
    pub fn set_len(&mut self, v: u32) {
        self.set_u32(OFF_LEN, v)
    }
    pub fn set_tot(&mut self, v: u32) {
        self.set_u32(OFF_TOT, v)
    }
    pub fn set_buffer(&mut self, v: u64) {
        self.set_u64(OFF_BUFFER, v)
    }
}

impl std::fmt::Debug for Bw {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Bw {{ val=0x{:016x} bits={} pos={} len={} tot={} buffer=0x{:016x} raw={:02x?} }}",
            self.val(),
            self.bits(),
            self.pos(),
            self.len(),
            self.tot(),
            self.buffer(),
            self.0
        )
    }
}

type BitwriterAdd = unsafe extern "C" fn(*mut Bw, u32, u64) -> std::os::raw::c_int;

/// One loaded implementation, reached only through its `.so` export table.
pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: Library,
    add: BitwriterAdd,
}

impl Impl {
    /// Call `bitwriter_add` on a copy of `state`; returns `(retval, new_state)`.
    pub fn add(&self, state: Bw, bits: u32, val: u64) -> (i32, Bw) {
        let mut s = state;
        let r = unsafe { (self.add)(&mut s as *mut Bw, bits, val) };
        (r as i32, s)
    }

    /// Call `bitwriter_add` with a NULL `bw`, exactly as an external C caller
    /// could. Used only by the `ERRORS.md` E13 row, which runs in a child
    /// process because the C code has no null check and therefore faults.
    pub fn add_raw_null(&self, bits: u32, val: u64) -> i32 {
        unsafe { (self.add)(std::ptr::null_mut(), bits, val) as i32 }
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().unwrap().to_path_buf()
}

/// Locate the C `.so`. Its file name is derived by CMake from the parent
/// directory name, so glob for it instead of hard-coding.
fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let build = workspace_root().join("c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                found.push(p);
            }
        }
    }
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one .so in {}, found {:?}. Build the C library first:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build.display(),
        found
    );
    found.pop().unwrap()
}

/// Locate the Rust `.so` under test.
///
/// Defaults to the **release** cdylib, because that is the artifact the crate
/// ships (`cargo build --release`, and `[profile.release] panic = "abort"` in
/// `Cargo.toml`) and the one an external C caller would link against.
///
/// The debug cdylib is deliberately *not* the default: rustc enables
/// `ub_checks` when `debug_assertions` is on, which turns the C code's
/// unchecked `bw->tot` store through a NULL pointer into a Rust panic (and
/// hence `SIGABRT`) instead of the `SIGSEGV` the C produces. That is a
/// property of the debug profile's inserted checks, not of the translation.
/// Override with `RUST_SO=/path/to/lib.so` to test any other artifact.
fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "RUST_SO points at a missing file: {}", p.display());
        return p;
    }
    // target dir = <...>/target/<profile>/deps/<test-exe> -> up 3
    let exe = std::env::current_exe().expect("current_exe");
    let target_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .expect("test exe should live in target/<profile>/deps/")
        .to_path_buf();
    let p = target_dir.join("release").join("libbitwriter_add_lib.so");
    assert!(
        p.exists(),
        "Rust cdylib not found at {}.\n`cargo test` does NOT build a \
         crate-type=[\"cdylib\"] target, so build it first:\n  \
         cargo build --release [--no-default-features --features <combo>]\n\
         or set RUST_SO=/path/to/libbitwriter_add_lib.so",
        p.display()
    );
    p
}

fn load(name: &'static str, path: PathBuf) -> Impl {
    unsafe {
        let lib = Library::new(&path)
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
        let sym: Symbol<BitwriterAdd> = lib
            .get(b"bitwriter_add\0")
            .unwrap_or_else(|e| panic!("{} does not export bitwriter_add: {e}", path.display()));
        let add = *sym;
        Impl {
            name,
            path,
            _lib: lib,
            add,
        }
    }
}

/// Both implementations, loaded once per test binary.
pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

pub fn pair() -> &'static Pair {
    use std::sync::OnceLock;
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(|| Pair {
        c: load("C", c_so_path()),
        rust: load("Rust", rust_so_path()),
    })
}

/// Deterministic PRNG (splitmix64) so every property-style run is reproducible.
pub struct Rng(std::cell::Cell<u64>);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(std::cell::Cell::new(seed))
    }
    pub fn next_u64(&self) -> u64 {
        self.0.set(self.0.get().wrapping_add(0x9E37_79B9_7F4A_7C15));
        let mut z = self.0.get();
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in `[lo, hi]` inclusive.
    pub fn range_u32(&self, lo: u32, hi: u32) -> u32 {
        debug_assert!(lo <= hi);
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as u32
    }
    pub fn bytes32(&self) -> [u8; 32] {
        let mut out = [0u8; 32];
        for c in out.chunks_mut(8) {
            c.copy_from_slice(&self.next_u64().to_ne_bytes());
        }
        out
    }
    /// A `val` drawn from a distribution that favours interesting bit patterns.
    pub fn interesting_u64(&self) -> u64 {
        const SPECIAL: [u64; 10] = [
            0,
            1,
            2,
            3,
            u64::MAX,
            u64::MAX - 1,
            0x8000_0000_0000_0000,
            0xAAAA_AAAA_AAAA_AAAA,
            0x5555_5555_5555_5555,
            0x0000_0000_FFFF_FFFF,
        ];
        let r = self.next_u64();
        if r % 4 == 0 {
            SPECIAL[(r >> 8) as usize % SPECIAL.len()]
        } else {
            self.next_u64()
        }
    }
    /// A `bits` value from a distribution that favours boundaries.
    pub fn interesting_bits(&self) -> u32 {
        const SPECIAL: [u32; 16] = [
            0,
            1,
            2,
            31,
            32,
            33,
            62,
            63,
            64,
            65,
            100,
            127,
            128,
            255,
            0x8000_0000,
            0xFFFF_FFFF,
        ];
        let r = self.next_u64();
        match r % 3 {
            0 => SPECIAL[(r >> 8) as usize % SPECIAL.len()],
            1 => self.range_u32(0, 70),
            _ => self.next_u32(),
        }
    }
}

/// The core differential assertion: identical inputs in, byte-identical
/// struct + identical `int` return out.
#[track_caller]
pub fn assert_same(p: &Pair, ctx: &str, state: Bw, bits: u32, val: u64) {
    let (rc, sc) = p.c.add(state, bits, val);
    let (rr, sr) = p.rust.add(state, bits, val);
    if rc != rr || sc != sr {
        panic!(
            "DIVERGENCE [{ctx}]\n  input : {state:?}\n          bits={bits} (0x{bits:08x}) \
             val=0x{val:016x}\n  C     : ret={rc} {sc:?}\n  Rust  : ret={rr} {sr:?}\n  \
             C .so   = {}\n  Rust .so= {}",
            p.c.path.display(),
            p.rust.path.display()
        );
    }
}

/// Differential assertion over a *sequence* of calls sharing accumulated state
/// (the composed pipeline a real consumer drives).
#[track_caller]
pub fn assert_same_chain(p: &Pair, ctx: &str, start: Bw, ops: &[(u32, u64)]) {
    let mut sc = start;
    let mut sr = start;
    for (i, &(bits, val)) in ops.iter().enumerate() {
        let (rc, nc) = p.c.add(sc, bits, val);
        let (rr, nr) = p.rust.add(sr, bits, val);
        if rc != rr || nc != nr {
            panic!(
                "DIVERGENCE [{ctx}] at chain step {i}\n  state-in C   : {sc:?}\n  \
                 state-in Rust: {sr:?}\n  bits={bits} (0x{bits:08x}) val=0x{val:016x}\n  \
                 C     : ret={rc} {nc:?}\n  Rust  : ret={rr} {nr:?}",
            );
        }
        sc = nc;
        sr = nr;
    }
}

/// Build a state with the given `bits`/`val` and randomized untouched fields.
pub fn state_with(rng: &Rng, bw_bits: u32, bw_val: u64) -> Bw {
    let mut s = Bw::from_bytes(rng.bytes32());
    s.set_val(bw_val);
    s.set_bits(bw_bits);
    s
}
