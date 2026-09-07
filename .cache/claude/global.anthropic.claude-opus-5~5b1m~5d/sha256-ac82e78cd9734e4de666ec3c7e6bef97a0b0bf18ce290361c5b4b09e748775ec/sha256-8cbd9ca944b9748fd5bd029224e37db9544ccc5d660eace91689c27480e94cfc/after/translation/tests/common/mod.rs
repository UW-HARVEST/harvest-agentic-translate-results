//! Shared harness: loads BOTH the C `.so` and the Rust `.so` via `libloading`
//! and calls `synth_pair` through the FFI boundary on each.
//!
//! No Rust function is ever called directly — everything goes through the
//! exported `synth_pair` symbol of the cdylib, exactly like an external caller.

use std::path::{Path, PathBuf};

pub type SynthPairFn = unsafe extern "C" fn(*mut i16, std::ffi::c_int, *const f32);

/// The highest `z` index the C reads is `2 + 14*64 == 898`, so 899 floats.
pub const Z_MIN_LEN: usize = 899;

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

/// Find the single shared object CMake produced in `c_src/build`.
fn c_so_path() -> PathBuf {
    let build = repo_root().join("c_src/build");
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
        "expected exactly one .so in {}, found {:?}. Build it with:\n  \
         cd c_src && mkdir -p build && cd build && cmake .. \
         -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build.display(),
        found
    );
    found.pop().unwrap()
}

/// Find the Rust cdylib that `cargo test` just built.
fn rust_so_path() -> PathBuf {
    let target = repo_root().join("translation/target");
    let mut cands: Vec<PathBuf> = Vec::new();
    for profile in ["debug", "release"] {
        let p = target.join(profile).join("libsynth_pair_lib.so");
        if p.is_file() {
            cands.push(p);
        }
    }
    assert!(
        !cands.is_empty(),
        "no libsynth_pair_lib.so under {} — run `cargo build` first",
        target.display()
    );
    // Prefer the freshest build.
    cands.sort_by_key(|p| {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    });
    let so = cands.pop().unwrap();

    // STALENESS GUARD.
    //
    // `cargo test` recompiles the crate but does NOT refresh the `cdylib`
    // artifact in `target/<profile>/`. Without this check the whole suite
    // silently loads a `.so` from an earlier `cargo build` and passes no matter
    // what `src/lib.rs` says — every test becomes vacuous. Always drive the
    // suite through `./run_tests.sh`, which builds before testing.
    let src = repo_root().join("translation/src/lib.rs");
    let mtime = |p: &Path| {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    };
    let so_t = mtime(&so);
    let src_t = mtime(&src);
    assert!(
        so_t >= src_t,
        "STALE RUST .so — refusing to run vacuous tests.\n  \
         {} is OLDER than {}.\n  \
         `cargo test` does not rebuild the cdylib; run `./run_tests.sh` \
         (or `cargo build --release` first).",
        so.display(),
        src.display()
    );
    so
}

pub struct Pair {
    // Keep the libraries alive for the lifetime of the function pointers.
    _c_lib: libloading::Library,
    _rust_lib: libloading::Library,
    pub c: SynthPairFn,
    pub rust: SynthPairFn,
}

impl Pair {
    pub fn load() -> Self {
        unsafe {
            let c_lib = libloading::Library::new(c_so_path()).expect("load C .so");
            let rust_lib = libloading::Library::new(rust_so_path()).expect("load Rust .so");
            let c: libloading::Symbol<SynthPairFn> =
                c_lib.get(b"synth_pair\0").expect("C synth_pair symbol");
            let rust: libloading::Symbol<SynthPairFn> = rust_lib
                .get(b"synth_pair\0")
                .expect("Rust synth_pair symbol (missing #[no_mangle] export?)");
            let c = *c;
            let rust = *rust;
            Pair {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c,
                rust,
            }
        }
    }
}

/// One differential invocation.
///
/// `pcm_len` cells are allocated and pre-filled with `canary`; the pointer handed
/// to the library is `base + head`, so a negative `16*nch` offset still lands
/// inside the allocation. The *entire* buffer is compared byte-for-byte, which
/// also proves neither implementation writes any cell the other leaves alone.
pub struct Case<'a> {
    pub pcm_len: usize,
    pub head: usize,
    pub canary: i16,
    pub nch: i32,
    pub z: &'a [f32],
    /// Extra floats to skip at the start of `z` (unaligned-view testing).
    pub z_offset: usize,
}

impl<'a> Case<'a> {
    pub fn new(nch: i32, z: &'a [f32]) -> Self {
        let head = if nch < 0 {
            (16i64 * (-(nch as i64))) as usize
        } else {
            0
        };
        let span = head + 16 * nch.unsigned_abs() as usize + 1;
        Case {
            pcm_len: span.max(head + 1),
            head,
            canary: 0x5A5A_u16 as i16,
            nch,
            z,
            z_offset: 0,
        }
    }
}

/// Runs the case against both libraries; returns `(c_buffer, rust_buffer)`.
pub fn run(p: &Pair, case: &Case<'_>) -> (Vec<i16>, Vec<i16>) {
    assert!(
        case.z.len() >= case.z_offset + Z_MIN_LEN,
        "z too short: need {} floats past offset {}, got {}",
        Z_MIN_LEN,
        case.z_offset,
        case.z.len()
    );
    let mut out = Vec::with_capacity(2);
    for f in [p.c, p.rust] {
        let mut pcm = vec![case.canary; case.pcm_len];
        unsafe {
            let ptr = pcm.as_mut_ptr().add(case.head);
            let z = case.z.as_ptr().add(case.z_offset);
            f(ptr, case.nch, z);
        }
        out.push(pcm);
    }
    let rust = out.pop().unwrap();
    let c = out.pop().unwrap();
    (c, rust)
}

/// Assert C and Rust agree bit-for-bit, with a diagnostic naming the row.
pub fn assert_same(p: &Pair, case: &Case<'_>, ctx: &str) {
    let (c, r) = run(p, case);
    if c != r {
        let first = c
            .iter()
            .zip(r.iter())
            .enumerate()
            .find(|(_, (a, b))| a != b)
            .map(|(i, (a, b))| (i, *a, *b));
        panic!(
            "DIVERGENCE [{ctx}]\n  nch={} head={} pcm_len={} z_offset={}\n  \
             first differing pcm index (absolute in buffer): {:?}\n  \
             C   = {:?}\n  Rust= {:?}\n  z taps = {:?}",
            case.nch,
            case.head,
            case.pcm_len,
            case.z_offset,
            first,
            &c[..c.len().min(80)],
            &r[..r.len().min(80)],
            tap_dump(case)
        );
    }
}

/// The exact set of `z` indices the C source reads, for diagnostics.
pub fn tap_offsets() -> Vec<usize> {
    let mut v: Vec<usize> = (0..=14).map(|k| k * 64).collect();
    v.extend((0..=14).step_by(2).map(|k| 2 + k * 64));
    v
}

fn tap_dump(case: &Case<'_>) -> Vec<(usize, f32)> {
    tap_offsets()
        .into_iter()
        .map(|o| (o, case.z[case.z_offset + o]))
        .collect()
}

/// splitmix64 — deterministic, seeded, portable.
pub struct Rng(pub u64);

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
    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform in [-scale, scale).
    pub fn sym(&mut self, scale: f32) -> f32 {
        (self.unit() * 2.0 - 1.0) * scale
    }
    /// Any bit pattern reinterpreted as f32 (normals, subnormals, +-0, +-inf, NaN).
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
}

/// Fill a fresh `z` buffer of `Z_MIN_LEN + pad` floats using `f`.
pub fn make_z(pad: usize, mut f: impl FnMut(usize) -> f32) -> Vec<f32> {
    (0..Z_MIN_LEN + pad).map(&mut f).collect()
}
