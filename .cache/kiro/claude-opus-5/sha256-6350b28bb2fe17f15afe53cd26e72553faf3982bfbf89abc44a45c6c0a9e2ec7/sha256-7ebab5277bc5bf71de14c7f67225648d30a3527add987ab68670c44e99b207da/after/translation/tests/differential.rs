//! Differential test: loads BOTH the C `.so` and the Rust `cdylib` via
//! `libloading` and compares their behaviour through the FFI boundary.
//!
//! Nothing in this file calls the Rust crate directly — every call goes
//! through the dynamic symbol exported by `libflac_validate_lib.so`, so the
//! `#[no_mangle]` wrappers are exercised exactly as an external C caller
//! would exercise them.

use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

/// `struct tflac` from `c_src/include/lib.h`.
///
/// 4x u32, 5x u8, 1x u32 => align 4, size 28, with 3 padding bytes at
/// offset 21..24.
const TFLAC_SIZE: usize = 28;

/// Raw byte image of a `struct tflac`, so we can compare *every* byte
/// (including padding) after each call.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(C, align(4))]
struct Tflac([u8; TFLAC_SIZE]);

impl Tflac {
    fn new(
        blocksize: u32,
        samplerate: u32,
        channels: u32,
        bitdepth: u32,
        channel_mode: u8,
        max_rice_value: u8,
        min_partition_order: u8,
        max_partition_order: u8,
        partition_order: u8,
        cur_blocksize: u32,
    ) -> Self {
        let mut b = [0u8; TFLAC_SIZE];
        b[0..4].copy_from_slice(&blocksize.to_ne_bytes());
        b[4..8].copy_from_slice(&samplerate.to_ne_bytes());
        b[8..12].copy_from_slice(&channels.to_ne_bytes());
        b[12..16].copy_from_slice(&bitdepth.to_ne_bytes());
        b[16] = channel_mode;
        b[17] = max_rice_value;
        b[18] = min_partition_order;
        b[19] = max_partition_order;
        b[20] = partition_order;
        // b[21..24] is padding, deliberately left at 0 in both copies.
        b[24..28].copy_from_slice(&cur_blocksize.to_ne_bytes());
        Self(b)
    }

    fn field_u32(&self, off: usize) -> u32 {
        u32::from_ne_bytes(self.0[off..off + 4].try_into().unwrap())
    }

    fn describe(&self) -> String {
        format!(
            "blocksize={} samplerate={} channels={} bitdepth={} \
             channel_mode={} max_rice_value={} min_po={} max_po={} po={} \
             cur_blocksize={} pad={:?}",
            self.field_u32(0),
            self.field_u32(4),
            self.field_u32(8),
            self.field_u32(12),
            self.0[16],
            self.0[17],
            self.0[18],
            self.0[19],
            self.0[20],
            self.field_u32(24),
            &self.0[21..24],
        )
    }
}

type FlacValidateFn = unsafe extern "C" fn(*mut Tflac) -> std::os::raw::c_int;
type SizeMemoryFn = unsafe extern "C" fn(u32) -> u32;

struct Impl {
    name: &'static str,
    _lib: Library,
    flac_validate: FlacValidateFn,
    tflac_size_memory: SizeMemoryFn,
}

impl Impl {
    fn load(name: &'static str, path: &Path) -> Self {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to load {}: {e}", path.display()));
        let flac_validate: FlacValidateFn = unsafe {
            let s: Symbol<FlacValidateFn> = lib
                .get(b"flac_validate\0")
                .unwrap_or_else(|e| panic!("{name}: missing flac_validate: {e}"));
            *s
        };
        let tflac_size_memory: SizeMemoryFn = unsafe {
            let s: Symbol<SizeMemoryFn> = lib
                .get(b"tflac_size_memory\0")
                .unwrap_or_else(|e| panic!("{name}: missing tflac_size_memory: {e}"));
            *s
        };
        Impl {
            name,
            _lib: lib,
            flac_validate,
            tflac_size_memory,
        }
    }

    fn validate(&self, t: &mut Tflac) -> i32 {
        unsafe { (self.flac_validate)(t as *mut Tflac) }
    }

    fn size_memory(&self, blocksize: u32) -> u32 {
        unsafe { (self.tflac_size_memory)(blocksize) }
    }
}

/// Locate the C `.so`. CMake names the library after the parent directory of
/// `c_src`, so we glob rather than hard-coding the name.
fn c_so_path() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf();
    let build = root.join("c_src/build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {} ({e}); build the C library first: \
                 cd c_src && mkdir -p build && cd build && \
                 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                build.display()
            )
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("lib") && n.ends_with(".so"))
        })
        .collect();
    candidates.sort();
    candidates
        .pop()
        .unwrap_or_else(|| panic!("no lib*.so found in {}", build.display()))
}

/// Locate the Rust `cdylib` next to the integration-test executable.
///
/// `cargo test` does not build the `cdylib` artifact on its own (integration
/// tests do not link against it), so if it is missing we shell out to
/// `cargo build --lib` for the matching profile. Cargo's package lock is
/// released while tests execute, so the nested invocation is safe; `OnceLock`
/// makes sure it happens exactly once even though tests run in parallel
/// threads of the same process.
fn rust_so_path() -> PathBuf {
    static PATH: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    PATH.get_or_init(|| {
        let exe = std::env::current_exe().expect("current_exe");
        // .../target/<profile>/deps/differential-<hash>
        let deps = exe.parent().expect("deps dir");
        let profile_dir = deps.parent().expect("profile dir");
        let name = format!(
            "{}flac_validate_lib{}",
            std::env::consts::DLL_PREFIX,
            std::env::consts::DLL_SUFFIX
        );

        let candidates = [profile_dir.join(&name), deps.join(&name)];

        // Always rebuild: `cargo test` leaves a previously-built `.so` in
        // place, so a conditional build would happily load a stale artifact
        // and make every comparison vacuous.
        let profile = profile_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("debug");
        let mut cmd =
            std::process::Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
        cmd.arg("build")
            .arg("--lib")
            .current_dir(env!("CARGO_MANIFEST_DIR"));
        if profile != "debug" {
            cmd.arg("--profile").arg(profile);
        }
        let status = cmd.status().expect("spawn cargo build --lib");
        assert!(status.success(), "nested `cargo build --lib` failed");

        candidates
            .iter()
            .find(|p| p.exists())
            .cloned()
            .unwrap_or_else(|| {
                panic!(
                    "could not find {name} in {} or {} after `cargo build --lib`",
                    profile_dir.display(),
                    deps.display()
                )
            })
    })
    .clone()
}

fn impls() -> (Impl, Impl) {
    (
        Impl::load("C", &c_so_path()),
        Impl::load("Rust", &rust_so_path()),
    )
}

/// Run one `flac_validate` case against both libraries and require the return
/// value *and* the full 28-byte struct image to match byte-for-byte.
fn check_validate(c: &Impl, r: &Impl, input: Tflac) {
    let mut ct = input;
    let mut rt = input;
    let cr = c.validate(&mut ct);
    let rr = r.validate(&mut rt);
    assert_eq!(
        cr, rr,
        "flac_validate return mismatch\n  input: {}\n  {} -> {cr}\n  {} -> {rr}",
        input.describe(),
        c.name,
        r.name,
    );
    assert!(
        ct == rt,
        "flac_validate struct mismatch (ret {cr})\n  input: {}\n  C   : {}\n  Rust: {}\n  C bytes   : {:?}\n  Rust bytes: {:?}",
        input.describe(),
        ct.describe(),
        rt.describe(),
        ct.0,
        rt.0,
    );
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) so failures are reproducible.
// ---------------------------------------------------------------------------
struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    fn u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// Uniform in `[0, n)`.
    fn below(&mut self, n: u32) -> u32 {
        self.u32() % n
    }
}

// ===========================================================================
// Level 1: tflac_size_memory (lowest-level leaf function)
// ===========================================================================

#[test]
fn size_memory_exhaustive_low_and_edges() {
    let (c, r) = impls();

    // Every blocksize in 0..=100_000 covers all the interesting alignment
    // behaviour of `(15 + blocksize*4) & 0xFFFFFFF0`.
    for bs in 0u32..=100_000 {
        let cv = c.size_memory(bs);
        let rv = r.size_memory(bs);
        assert_eq!(cv, rv, "tflac_size_memory({bs}): C={cv} Rust={rv}");
    }

    // Edges, including values where `blocksize * 4` and the `* 5` overflow.
    let mut edges = vec![
        0,
        1,
        3,
        4,
        15,
        16,
        17,
        65535,
        65536,
        0x0FFF_FFFF,
        0x1000_0000,
        0x3FFF_FFFF,
        0x4000_0000,
        0x7FFF_FFFF,
        0x8000_0000,
        0xFFFF_FFF0,
        0xFFFF_FFFB,
        0xFFFF_FFFC,
        0xFFFF_FFFD,
        0xFFFF_FFFE,
        u32::MAX,
    ];
    for base in [
        0u32,
        1 << 28,
        1 << 29,
        1 << 30,
        1 << 31,
        0xFFFF_0000,
        0x4000_0000,
    ] {
        for d in 0u32..64 {
            edges.push(base.wrapping_add(d));
            edges.push(base.wrapping_sub(d));
        }
    }
    for bs in edges {
        let cv = c.size_memory(bs);
        let rv = r.size_memory(bs);
        assert_eq!(cv, rv, "tflac_size_memory({bs}): C={cv} Rust={rv}");
    }
}

#[test]
fn size_memory_random_full_range() {
    let (c, r) = impls();
    let mut rng = Rng(0x1234_5678_9ABC_DEF1);
    for _ in 0..300_000 {
        let bs = rng.u32();
        let cv = c.size_memory(bs);
        let rv = r.size_memory(bs);
        assert_eq!(cv, rv, "tflac_size_memory({bs}): C={cv} Rust={rv}");
    }
}

// ===========================================================================
// Level 2: flac_validate
// ===========================================================================

/// Reject/accept boundaries for each scalar guard, one field at a time.
#[test]
fn validate_field_boundaries() {
    let (c, r) = impls();

    let blocksizes = [0u32, 1, 15, 16, 17, 4096, 65534, 65535, 65536, u32::MAX];
    let samplerates = [0u32, 1, 44100, 655349, 655350, 655351, u32::MAX];
    let channels = [0u32, 1, 2, 3, 7, 8, 9, u32::MAX];
    let bitdepths = [0u32, 1, 8, 15, 16, 17, 24, 31, 32, 33, u32::MAX];

    for &blocksize in &blocksizes {
        check_validate(&c, &r, Tflac::new(blocksize, 44100, 2, 16, 0, 0, 0, 0, 0, 0));
    }
    for &samplerate in &samplerates {
        check_validate(&c, &r, Tflac::new(4096, samplerate, 2, 16, 0, 0, 0, 0, 0, 0));
    }
    for &ch in &channels {
        check_validate(&c, &r, Tflac::new(4096, 44100, ch, 16, 0, 0, 0, 0, 0, 0));
    }
    for &bd in &bitdepths {
        check_validate(&c, &r, Tflac::new(4096, 44100, 2, bd, 0, 0, 0, 0, 0, 0));
    }
    // max_rice_value: 0 triggers the defaulting branch, >30 rejects.
    for rice in 0u8..=255 {
        check_validate(&c, &r, Tflac::new(4096, 44100, 2, 16, 0, rice, 0, 0, 0, 0));
        check_validate(&c, &r, Tflac::new(4096, 44100, 2, 24, 0, rice, 0, 0, 0, 0));
    }
    // max_partition_order: >15 rejects.
    for mpo in 0u8..=255 {
        check_validate(&c, &r, Tflac::new(4096, 44100, 2, 16, 0, 0, 0, mpo, 0, 0));
    }
    // channel_mode: every u8 value, including out-of-enum-range ones.
    for cm in 0u8..=255 {
        check_validate(&c, &r, Tflac::new(4096, 44100, 2, 16, cm, 0, 0, 0, 0, 0));
        check_validate(&c, &r, Tflac::new(4096, 44100, 2, 32, cm, 0, 0, 0, 0, 0));
        check_validate(&c, &r, Tflac::new(4096, 44100, 1, 16, cm, 0, 0, 0, 0, 0));
        check_validate(&c, &r, Tflac::new(4096, 44100, 3, 16, cm, 0, 0, 0, 0, 0));
    }
}

/// The `partition_order` loop is the trickiest part: it tests
/// `blocksize % (1 << (po + 1)) == 0` *before* `po < max_partition_order`,
/// so the shift can reach `1 << 16`. Sweep every valid (min, max) pair
/// against a wide set of blocksizes.
#[test]
fn validate_partition_order_loop() {
    let (c, r) = impls();

    let mut blocksizes: Vec<u32> = Vec::new();
    // Powers of two and their neighbours, plus multiples with many trailing
    // zero bits -- these drive the loop to its upper bound.
    for shift in 4..=16u32 {
        let p = 1u32 << shift;
        for d in [0i64, -1, 1, -2, 2] {
            let v = p as i64 + d;
            if (16..=65535).contains(&v) {
                blocksizes.push(v as u32);
            }
        }
    }
    for bs in [
        16u32, 24, 32, 48, 64, 96, 128, 192, 256, 384, 512, 576, 768, 1024, 1152, 1536, 2048, 2304,
        3072, 4096, 4608, 6144, 8192, 9216, 12288, 16384, 18432, 24576, 32768, 32896, 36864, 49152,
        65024, 65280, 65520, 65535, 4095, 4097, 12345, 33333, 60000,
    ] {
        blocksizes.push(bs);
    }
    blocksizes.sort_unstable();
    blocksizes.dedup();

    for &bs in &blocksizes {
        for max_po in 0u8..=15 {
            for min_po in 0u8..=15 {
                // Includes the min>max rejection path.
                for &seed_po in &[0u8, 7, 255] {
                    check_validate(
                        &c,
                        &r,
                        Tflac::new(bs, 44100, 2, 16, 0, 0, min_po, max_po, seed_po, 0xDEAD_BEEF),
                    );
                }
            }
        }
    }
}

/// Full 16-bit blocksize sweep with the partition-order bounds pinned wide
/// open, so every blocksize exercises the loop to its natural stopping point.
#[test]
fn validate_all_blocksizes_widest_partition_range() {
    let (c, r) = impls();
    for bs in 0u32..=65_600 {
        check_validate(&c, &r, Tflac::new(bs, 44100, 2, 16, 0, 0, 0, 15, 0, 0));
        check_validate(&c, &r, Tflac::new(bs, 44100, 2, 16, 0, 0, 15, 15, 0, 0));
    }
}

/// Randomised sweep over the whole input space, biased towards values that
/// pass the early guards so the deeper logic actually runs.
#[test]
fn validate_random_structured() {
    let (c, r) = impls();
    let mut rng = Rng(0x0BAD_C0FF_EE0D_DF00);

    for _ in 0..400_000 {
        // Mostly-valid draws.
        let blocksize = match rng.below(10) {
            0 => rng.u32(),
            1 => rng.below(32),
            2 => 1u32 << (4 + rng.below(13)),
            _ => 16 + rng.below(65520),
        };
        let samplerate = match rng.below(8) {
            0 => rng.u32(),
            1 => rng.below(4),
            _ => 1 + rng.below(655_350),
        };
        let channels = match rng.below(8) {
            0 => rng.u32(),
            _ => rng.below(11),
        };
        let bitdepth = match rng.below(8) {
            0 => rng.u32(),
            _ => rng.below(35),
        };
        let channel_mode = if rng.below(2) == 0 { rng.u8() } else { rng.below(5) as u8 };
        let max_rice_value = if rng.below(2) == 0 { rng.u8() } else { rng.below(33) as u8 };
        let min_partition_order = if rng.below(4) == 0 { rng.u8() } else { rng.below(17) as u8 };
        let max_partition_order = if rng.below(4) == 0 { rng.u8() } else { rng.below(17) as u8 };
        let partition_order = rng.u8();
        let cur_blocksize = rng.u32();

        check_validate(
            &c,
            &r,
            Tflac::new(
                blocksize,
                samplerate,
                channels,
                bitdepth,
                channel_mode,
                max_rice_value,
                min_partition_order,
                max_partition_order,
                partition_order,
                cur_blocksize,
            ),
        );
    }
}

/// Fully unstructured fuzz: fill all 28 bytes (padding excluded) with random
/// data. Most cases bail out early, but this catches anything the structured
/// generators miss.
#[test]
fn validate_random_unstructured() {
    let (c, r) = impls();
    let mut rng = Rng(0xFEED_FACE_CAFE_1234);
    for _ in 0..200_000 {
        let t = Tflac::new(
            rng.u32(),
            rng.u32(),
            rng.u32(),
            rng.u32(),
            rng.u8(),
            rng.u8(),
            rng.u8(),
            rng.u8(),
            rng.u8(),
            rng.u32(),
        );
        check_validate(&c, &r, t);
    }
}

/// Calling `flac_validate` repeatedly on the same struct must converge
/// identically in both implementations (it mutates its input).
#[test]
fn validate_idempotence_after_mutation() {
    let (c, r) = impls();
    let mut rng = Rng(0x5EED_0000_1111_2222);
    for _ in 0..20_000 {
        let mut ct = Tflac::new(
            16 + rng.below(65520),
            1 + rng.below(655_350),
            rng.below(11),
            rng.below(35),
            rng.below(5) as u8,
            rng.below(33) as u8,
            rng.below(17) as u8,
            rng.below(17) as u8,
            rng.u8(),
            rng.u32(),
        );
        let mut rt = ct;
        for round in 0..4 {
            let cr = c.validate(&mut ct);
            let rr = r.validate(&mut rt);
            assert_eq!(cr, rr, "round {round}: ret C={cr} Rust={rr}");
            assert!(
                ct == rt,
                "round {round}: struct mismatch\n  C   : {}\n  Rust: {}",
                ct.describe(),
                rt.describe(),
            );
        }
    }
}

/// The exported symbol set of the Rust `cdylib` must be a superset of the
/// C `.so`'s exported symbol set (macro-generated symbols included).
#[test]
fn exported_symbols_match() {
    fn dynamic_symbols(path: &Path) -> Vec<String> {
        let out = std::process::Command::new("nm")
            .arg("-D")
            .arg("--defined-only")
            .arg(path)
            .output()
            .expect("run nm");
        assert!(
            out.status.success(),
            "nm failed on {}: {}",
            path.display(),
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| {
                let mut it = l.split_whitespace();
                let _addr = it.next()?;
                let kind = it.next()?;
                let name = it.next()?;
                // Only strong, defined symbols in the text/data segments.
                matches!(kind, "T" | "D" | "B" | "R").then(|| name.to_string())
            })
            .collect()
    }

    let c_syms = dynamic_symbols(&c_so_path());
    let r_syms = dynamic_symbols(&rust_so_path());
    assert!(!c_syms.is_empty(), "no symbols found in the C .so");

    let missing: Vec<&String> = c_syms.iter().filter(|s| !r_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}\n  C: {c_syms:?}\n  Rust: {r_syms:?}"
    );
}
