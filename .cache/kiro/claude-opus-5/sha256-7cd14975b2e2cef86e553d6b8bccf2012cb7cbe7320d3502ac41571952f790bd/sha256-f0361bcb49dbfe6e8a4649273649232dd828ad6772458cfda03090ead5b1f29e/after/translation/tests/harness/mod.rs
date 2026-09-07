//! Shared harness: loads BOTH the C `.so` and the Rust `.so` via `libloading`
//! and calls `bitwriter_add` through the dynamic-symbol export in each, exactly
//! as an external C consumer would. The Rust implementation is NEVER called
//! directly — this also exercises the `#[no_mangle]` / `extern "C"` wrapper.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::process::Command;

/// Mirror of `struct tflac_bitwriter` from `c_src/include/lib.h`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bitwriter {
    pub val: u64,
    pub bits: u32,
    pub pos: u32,
    pub len: u32,
    pub tot: u32,
    pub buffer: *mut u8,
}

impl Default for Bitwriter {
    fn default() -> Self {
        Bitwriter {
            val: 0,
            bits: 0,
            pos: 0,
            len: 0,
            tot: 0,
            buffer: std::ptr::null_mut(),
        }
    }
}

impl Bitwriter {
    /// Raw byte image of the struct, including any tail padding, so the
    /// comparison is genuinely byte-for-byte across the FFI boundary.
    pub fn bytes(&self) -> [u8; std::mem::size_of::<Bitwriter>()] {
        unsafe { std::mem::transmute_copy(self) }
    }
}

pub type AddFn = unsafe extern "C" fn(*mut Bitwriter, u32, u64) -> std::ffi::c_int;

pub fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = repo_root().join("c_src").join("build");
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
    found.pop().unwrap_or_else(|| {
        panic!(
            "no C .so found in {}; build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

/// Locate (and always rebuild) the Rust `cdylib`.
///
/// The **release** artifact is used deliberately: it is the shipped `.so` an
/// external C consumer would `dlopen`, and it is what
/// `cd translation && cargo build --release` produces. The `dev` profile turns
/// on `debug_assertions`, which makes rustc inject UB checks (e.g. a
/// "null pointer dereference occurred" panic on `(*bw).tot`) that convert a
/// fault the C resolves as `SIGSEGV` into a non-unwinding panic / `SIGABRT`.
/// Those checks are a property of the debug build, not of the translated logic.
fn find_rust_so() -> PathBuf {
    // current_exe = <target>/<profile>/deps/<test>-<hash>
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let profile_dir = deps.parent().expect("profile dir");
    let target_dir = profile_dir.parent().expect("target dir");

    let name = format!(
        "{}bitwriter_add_lib{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    );

    // ALWAYS rebuild: `cargo test` does not refresh the release cdylib, and
    // silently loading a stale `.so` would invalidate the whole verification.
    let status = Command::new(env!("CARGO"))
        .args(["build", "--release", "--lib"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("spawn cargo build --release --lib");
    assert!(status.success(), "cargo build --release --lib failed");

    let cand = target_dir.join("release").join(&name);
    if cand.exists() {
        return cand;
    }
    panic!("could not locate {}", cand.display())
}

/// The two loaded libraries plus their resolved `bitwriter_add` symbols.
pub struct Pair {
    _c_lib: Library,
    _rs_lib: Library,
    pub c_add: AddFn,
    pub rs_add: AddFn,
}

impl Pair {
    pub fn load() -> Pair {
        unsafe {
            let c_path = find_c_so();
            let rs_path = find_rust_so();

            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
            let rs_lib = Library::new(&rs_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", rs_path.display()));

            let c_sym: Symbol<AddFn> = c_lib
                .get(b"bitwriter_add\0")
                .expect("C .so does not export bitwriter_add");
            let rs_sym: Symbol<AddFn> = rs_lib
                .get(b"bitwriter_add\0")
                .expect("Rust .so does not export bitwriter_add");

            let c_add = *c_sym;
            let rs_add = *rs_sym;

            Pair {
                _c_lib: c_lib,
                _rs_lib: rs_lib,
                c_add,
                rs_add,
            }
        }
    }

    /// Run one call on identical copies of `init` through both `.so`s and
    /// assert byte-identical results.
    #[track_caller]
    pub fn check(&self, row: &str, iter: u64, init: &Bitwriter, bits: u32, val: u64) {
        let mut c_bw = *init;
        let mut rs_bw = *init;

        let c_ret = unsafe { (self.c_add)(&mut c_bw, bits, val) };
        let rs_ret = unsafe { (self.rs_add)(&mut rs_bw, bits, val) };

        if c_ret != rs_ret || c_bw.bytes() != rs_bw.bytes() {
            panic!(
                "DIVERGENCE in {row} (iteration {iter})\n\
                 input : bw = {{ val: {:#018x}, bits: {}, pos: {}, len: {}, tot: {}, buffer: {:p} }}\n\
                 \x20        bits = {bits} ({bits:#x}), val = {val:#018x}\n\
                 C     : ret = {c_ret}, bw = {{ val: {:#018x}, bits: {}, pos: {}, len: {}, tot: {}, buffer: {:p} }}\n\
                 Rust  : ret = {rs_ret}, bw = {{ val: {:#018x}, bits: {}, pos: {}, len: {}, tot: {}, buffer: {:p} }}\n\
                 C bytes  : {:02x?}\n\
                 Rust bytes: {:02x?}",
                init.val,
                init.bits,
                init.pos,
                init.len,
                init.tot,
                init.buffer,
                c_bw.val,
                c_bw.bits,
                c_bw.pos,
                c_bw.len,
                c_bw.tot,
                c_bw.buffer,
                rs_bw.val,
                rs_bw.bits,
                rs_bw.pos,
                rs_bw.len,
                rs_bw.tot,
                rs_bw.buffer,
                c_bw.bytes(),
                rs_bw.bytes(),
            );
        }
    }

    /// Drive a composed sequence of calls against the same `bw`, comparing the
    /// full state after every single call (a per-call check cannot see state
    /// that only diverges after several accumulating calls).
    #[track_caller]
    pub fn check_sequence(&self, row: &str, seq_id: u64, init: &Bitwriter, ops: &[(u32, u64)]) {
        let mut c_bw = *init;
        let mut rs_bw = *init;

        for (step, &(bits, val)) in ops.iter().enumerate() {
            let c_ret = unsafe { (self.c_add)(&mut c_bw, bits, val) };
            let rs_ret = unsafe { (self.rs_add)(&mut rs_bw, bits, val) };

            if c_ret != rs_ret || c_bw.bytes() != rs_bw.bytes() {
                panic!(
                    "DIVERGENCE in {row} (sequence {seq_id}, step {step})\n\
                     seq start: bw = {{ val: {:#018x}, bits: {}, pos: {}, len: {}, tot: {} }}\n\
                     ops so far: {:?}\n\
                     this op  : bits = {bits} ({bits:#x}), val = {val:#018x}\n\
                     C   : ret = {c_ret}, bw = {{ val: {:#018x}, bits: {}, pos: {}, len: {}, tot: {} }}\n\
                     Rust: ret = {rs_ret}, bw = {{ val: {:#018x}, bits: {}, pos: {}, len: {}, tot: {} }}",
                    init.val,
                    init.bits,
                    init.pos,
                    init.len,
                    init.tot,
                    &ops[..=step],
                    c_bw.val,
                    c_bw.bits,
                    c_bw.pos,
                    c_bw.len,
                    c_bw.tot,
                    rs_bw.val,
                    rs_bw.bits,
                    rs_bw.pos,
                    rs_bw.len,
                    rs_bw.tot,
                );
            }
        }
    }
}

/* ---------------------------------------------------------------- */
/* Deterministic PRNG (SplitMix64) — fixed seed for reproducibility */
/* ---------------------------------------------------------------- */

pub struct Rng(u64);

pub const SEED: u64 = 0x5EED_1234_ABCD_F00D;

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

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform-ish in `lo..=hi`.
    pub fn range_u32(&mut self, lo: u32, hi: u32) -> u32 {
        debug_assert!(lo <= hi);
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as u32
    }

    /// A random `bw` with a caller-chosen `bits` field.
    pub fn bw_with_bits(&mut self, bits: u32) -> Bitwriter {
        Bitwriter {
            val: self.next_u64(),
            bits,
            pos: self.next_u32(),
            len: self.next_u32(),
            tot: self.next_u32(),
            buffer: self.next_u64() as *mut u8,
        }
    }
}

/// Number of randomized inputs per `CONFIGS.md` row.
pub const ITERS: u64 = 3000;
