//! Differential tests: load BOTH the C `.so` and the Rust `.so` through
//! `libloading` and compare their observable effects byte-for-byte.
//!
//! Nothing here calls the Rust implementation directly — every call goes
//! through the `.so`'s exported symbols, so the `#[no_mangle]` wrappers and the
//! `extern "C"` ABI are exercised exactly as an external C consumer would.
//!
//! Layout facts (probed from the C build, see `SYMBOLS.md`):
//!   `tflac_md5`  : size 88, align 8, pos@0, total@8, buffer@16 (72 bytes)
//!   `tflac`      : size 96, align 8, md5_ctx@0, cur_blocksize@88, channels@92
//!
//! Because `md5_ctx` sits at offset 0 of `tflac`, one arena serves as both a
//! `tflac *` and a `tflac_md5 *`.

#![allow(clippy::missing_safety_doc)]

use libloading::Library;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// struct layout
// ---------------------------------------------------------------------------

const OFF_POS: usize = 0;
const OFF_TOTAL: usize = 8;
const OFF_BUFFER: usize = 16;
const OFF_CBS: usize = 88;
const OFF_CH: usize = 92;
const SIZEOF_TFLAC: usize = 96;
const BUFFER_LEN: usize = 64 + 8;

/// The C carry-down loop can read `buffer[64 + 62]` = struct offset 142, i.e.
/// well past `sizeof(tflac) == 96`. The arena is generously oversized so that
/// every such read lands inside memory we control and initialise identically
/// for both implementations, making the out-of-bounds behaviour deterministic
/// and therefore comparable.
const ARENA: usize = 512;

#[repr(C, align(16))]
#[derive(Clone)]
struct Arena {
    b: [u8; ARENA],
}

impl Arena {
    fn zeroed() -> Self {
        Arena { b: [0u8; ARENA] }
    }

    /// Fill the whole arena (including the bytes past the struct that the OOB
    /// reads touch) with pseudo-random data, then overwrite the header fields.
    fn random(rng: &mut Rng) -> Self {
        let mut a = Arena::zeroed();
        for byte in a.b.iter_mut() {
            *byte = rng.next_u64() as u8;
        }
        a
    }

    fn set_pos(&mut self, v: u32) {
        self.b[OFF_POS..OFF_POS + 4].copy_from_slice(&v.to_le_bytes());
    }
    fn set_total(&mut self, v: u64) {
        self.b[OFF_TOTAL..OFF_TOTAL + 8].copy_from_slice(&v.to_le_bytes());
    }
    fn set_cbs(&mut self, v: u32) {
        self.b[OFF_CBS..OFF_CBS + 4].copy_from_slice(&v.to_le_bytes());
    }
    fn set_ch(&mut self, v: u32) {
        self.b[OFF_CH..OFF_CH + 4].copy_from_slice(&v.to_le_bytes());
    }
    fn get_pos(&self) -> u32 {
        u32::from_le_bytes(self.b[OFF_POS..OFF_POS + 4].try_into().unwrap())
    }
    fn get_total(&self) -> u64 {
        u64::from_le_bytes(self.b[OFF_TOTAL..OFF_TOTAL + 8].try_into().unwrap())
    }
    fn buffer(&self) -> &[u8] {
        &self.b[OFF_BUFFER..OFF_BUFFER + BUFFER_LEN]
    }
}

// ---------------------------------------------------------------------------
// deterministic PRNG (xorshift64*), fixed seed for reproducibility
// ---------------------------------------------------------------------------

const SEED: u64 = 0x2545_F491_4F6C_DD1D;

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 1 } else { seed })
    }
    fn next_u64(&mut self) -> u64 {
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
    /// Uniform-ish in `0..n` (n > 0).
    fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
}

// ---------------------------------------------------------------------------
// the two implementations, both loaded as shared objects
// ---------------------------------------------------------------------------

type FnPack = unsafe extern "C" fn(*mut u8, u64);
type FnAdd = unsafe extern "C" fn(*mut u8, u32, u64);
type FnUpdate = unsafe extern "C" fn(*mut u8, *const i32) -> u32;

struct Impl {
    #[allow(dead_code)]
    name: &'static str,
    pack: FnPack,
    add: FnAdd,
    update: FnUpdate,
    // Keep the mapping alive for as long as the function pointers are used.
    _lib: Library,
}

impl Impl {
    fn load(name: &'static str, path: &Path) -> Impl {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("{name}: cannot dlopen {}: {e}", path.display()));
        let pack: FnPack = unsafe { *lib.get(b"tflac_pack_u64le\0").expect("tflac_pack_u64le") };
        let add: FnAdd = unsafe { *lib.get(b"tflac_md5_addsample\0").expect("tflac_md5_addsample") };
        let update: FnUpdate = unsafe { *lib.get(b"update_md5\0").expect("update_md5") };
        Impl { name, pack, add, update, _lib: lib }
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let dir = workspace_root().join("c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "so").unwrap_or(false) {
                found.push(p);
            }
        }
    }
    found.sort();
    found.pop().unwrap_or_else(|| {
        panic!(
            "no C .so found in {}. Build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            dir.display()
        )
    })
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    // Prefer the profile the tests were built in, then fall back.
    let candidates = [
        base.join("debug/libupdate_md5_lib.so"),
        base.join("release/libupdate_md5_lib.so"),
    ];
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for c in candidates.iter() {
        if let Ok(md) = std::fs::metadata(c) {
            let t = md.modified().unwrap_or(std::time::UNIX_EPOCH);
            if best.as_ref().map(|(bt, _)| t > *bt).unwrap_or(true) {
                best = Some((t, c.clone()));
            }
        }
    }
    best.map(|(_, p)| p).unwrap_or_else(|| {
        panic!("no Rust cdylib found; run `cargo build` (and/or `cargo build --release`) first")
    })
}

/// The (C, Rust) pair, loaded once per test process.
fn impls() -> &'static (Impl, Impl) {
    use std::sync::OnceLock;
    static PAIR: OnceLock<(Impl, Impl)> = OnceLock::new();
    PAIR.get_or_init(|| {
        let c = Impl::load("C", &c_so_path());
        let r = Impl::load("Rust", &rust_so_path());
        (c, r)
    })
}

// ---------------------------------------------------------------------------
// comparison helpers
// ---------------------------------------------------------------------------

fn describe(a: &Arena) -> String {
    format!(
        "pos={} total={} buffer={:02x?} tail(96..160)={:02x?}",
        a.get_pos(),
        a.get_total(),
        a.buffer(),
        &a.b[SIZEOF_TFLAC..SIZEOF_TFLAC + 64]
    )
}

fn assert_arena_eq(ctx: &str, c: &Arena, r: &Arena) {
    if c.b != r.b {
        let first = (0..ARENA).find(|&i| c.b[i] != r.b[i]).unwrap();
        let ndiff = (0..ARENA).filter(|&i| c.b[i] != r.b[i]).count();
        panic!(
            "MEMORY DIVERGENCE in {ctx}\n  first differing byte at arena offset {first}: \
             C=0x{:02x} Rust=0x{:02x} ({ndiff} bytes differ in total)\n  C   : {}\n  Rust: {}",
            c.b[first],
            r.b[first],
            describe(c),
            describe(r)
        );
    }
}

fn assert_ret_eq(ctx: &str, c: u32, r: u32) {
    assert_eq!(c, r, "RETURN DIVERGENCE in {ctx}: C=0x{c:08x} Rust=0x{r:08x}");
}

/// `tflac_pack_u64le(&arena[off], n)` on one implementation.
fn run_pack(im: &Impl, base: &Arena, off: usize, n: u64) -> Arena {
    assert!(off + 8 <= ARENA);
    let mut a = base.clone();
    unsafe { (im.pack)(a.b.as_mut_ptr().add(off), n) };
    a
}

/// `tflac_md5_addsample((tflac_md5 *)arena, bits, val)` on one implementation.
fn run_add(im: &Impl, base: &Arena, bits: u32, val: u64) -> Arena {
    let mut a = base.clone();
    unsafe { (im.add)(a.b.as_mut_ptr(), bits, val) };
    a
}

/// `update_md5((tflac *)arena, samples)` on one implementation.
/// The samples buffer is cloned per call so that an accidental write into the
/// `const` input would also be caught.
fn run_update(im: &Impl, base: &Arena, samples: &[i32]) -> (u32, Arena, Vec<i32>) {
    let mut a = base.clone();
    let mut s = samples.to_vec();
    let ret = unsafe { (im.update)(a.b.as_mut_ptr(), s.as_ptr()) };
    let _ = &mut s;
    (ret, a, s)
}

fn diff_pack(ctx: &str, base: &Arena, off: usize, n: u64) {
    let (c, r) = impls();
    let ac = run_pack(c, base, off, n);
    let ar = run_pack(r, base, off, n);
    assert_arena_eq(&format!("{ctx} [pack off={off} n=0x{n:016x}]"), &ac, &ar);
}

fn diff_add(ctx: &str, base: &Arena, bits: u32, val: u64) {
    let (c, r) = impls();
    let ac = run_add(c, base, bits, val);
    let ar = run_add(r, base, bits, val);
    assert_arena_eq(
        &format!("{ctx} [addsample pos_in={} bits={bits} val=0x{val:016x}]", base.get_pos()),
        &ac,
        &ar,
    );
}

fn diff_update(ctx: &str, base: &Arena, samples: &[i32]) {
    let (c, r) = impls();
    let (rc, ac, sc) = run_update(c, base, samples);
    let (rr, ar, sr) = run_update(r, base, samples);
    let label = format!(
        "{ctx} [update_md5 pos_in={} total_in={} cbs={} ch={} nsamples={}]",
        base.get_pos(),
        base.get_total(),
        u32::from_le_bytes(base.b[OFF_CBS..OFF_CBS + 4].try_into().unwrap()),
        u32::from_le_bytes(base.b[OFF_CH..OFF_CH + 4].try_into().unwrap()),
        samples.len()
    );
    assert_ret_eq(&label, rc, rr);
    assert_arena_eq(&label, &ac, &ar);
    assert_eq!(sc, sr, "INPUT-BUFFER DIVERGENCE in {label}");
}

/// A samples buffer laid out so every element the C actually reads is
/// distinguishable: index `i` gets a value whose low byte is `i as u8` unless
/// randomised. `n` must be >= 136 to stay in bounds (see ERRORS.md row 21).
fn samples_random(rng: &mut Rng, n: usize) -> Vec<i32> {
    (0..n).map(|_| rng.next_i32()).collect()
}

/// Minimum length `update_md5` can read without going out of bounds:
/// last iteration reads `samples[128..=135]`.
const MIN_SAMPLES: usize = 136;

/// Two arenas stepped in lockstep — one per implementation — so that
/// *sequences* of calls (the state machine's history-dependent behaviour) can
/// be compared after every single step.
struct Pair {
    c: Arena,
    r: Arena,
    step: usize,
}

impl Pair {
    fn new(base: &Arena) -> Pair {
        Pair { c: base.clone(), r: base.clone(), step: 0 }
    }

    fn add(&mut self, ctx: &str, bits: u32, val: u64) {
        let (ci, ri) = impls();
        unsafe {
            (ci.add)(self.c.b.as_mut_ptr(), bits, val);
            (ri.add)(self.r.b.as_mut_ptr(), bits, val);
        }
        let label = format!("{ctx} step {} [addsample bits={bits} val=0x{val:016x}]", self.step);
        assert_arena_eq(&label, &self.c, &self.r);
        self.step += 1;
    }

    fn pack(&mut self, ctx: &str, off: usize, n: u64) {
        assert!(off + 8 <= ARENA);
        let (ci, ri) = impls();
        unsafe {
            (ci.pack)(self.c.b.as_mut_ptr().add(off), n);
            (ri.pack)(self.r.b.as_mut_ptr().add(off), n);
        }
        let label = format!("{ctx} step {} [pack off={off} n=0x{n:016x}]", self.step);
        assert_arena_eq(&label, &self.c, &self.r);
        self.step += 1;
    }

    fn update(&mut self, ctx: &str, samples: &[i32]) {
        let (ci, ri) = impls();
        let mut sc = samples.to_vec();
        let mut sr = samples.to_vec();
        let rc = unsafe { (ci.update)(self.c.b.as_mut_ptr(), sc.as_ptr()) };
        let rr = unsafe { (ri.update)(self.r.b.as_mut_ptr(), sr.as_ptr()) };
        let label = format!("{ctx} step {} [update_md5 n={}]", self.step, samples.len());
        assert_ret_eq(&label, rc, rr);
        assert_arena_eq(&label, &self.c, &self.r);
        assert_eq!(sc, sr, "INPUT-BUFFER DIVERGENCE in {label}");
        let _ = (&mut sc, &mut sr);
        self.step += 1;
    }
}

// ===========================================================================
// Phase B — valid-path differential tests, one test per CONFIGS.md row
// ===========================================================================

mod configs {
    use super::*;

    // --- tflac_pack_u64le -------------------------------------------------

    /// CONFIGS row 1: aligned destination, randomized values.
    #[test]
    fn row01_pack_aligned_random() {
        let mut rng = Rng::new(SEED ^ 1);
        for i in 0..512 {
            let base = Arena::random(&mut rng);
            diff_pack(&format!("row01 iter {i}"), &base, 0, rng.next_u64());
        }
    }

    /// CONFIGS row 2: every misalignment 1..=7.
    #[test]
    fn row02_pack_misaligned_random() {
        let mut rng = Rng::new(SEED ^ 2);
        for off in 1..=7usize {
            for i in 0..128 {
                let base = Arena::random(&mut rng);
                diff_pack(&format!("row02 off={off} iter {i}"), &base, off, rng.next_u64());
            }
            // also at a high, still-in-range offset with the same misalignment
            for i in 0..64 {
                let base = Arena::random(&mut rng);
                diff_pack(
                    &format!("row02 high off={} iter {i}", 400 + off),
                    &base,
                    400 + off,
                    rng.next_u64(),
                );
            }
        }
    }

    /// CONFIGS row 3: boundary values — 0, u64::MAX, every single bit, byte lanes.
    #[test]
    fn row03_pack_boundary_values() {
        let mut rng = Rng::new(SEED ^ 3);
        let mut vals: Vec<u64> = vec![0, u64::MAX, 1, u64::MAX - 1, 0x8000_0000_0000_0000];
        for k in 0..64 {
            vals.push(1u64 << k);
            vals.push(!(1u64 << k));
        }
        for lane in 0..8 {
            vals.push(0xFFu64 << (8 * lane));
            vals.push(0x80u64 << (8 * lane));
        }
        for (i, v) in vals.iter().enumerate() {
            let base = Arena::random(&mut rng);
            diff_pack(&format!("row03 val #{i}"), &base, 0, *v);
            let zero = Arena::zeroed();
            diff_pack(&format!("row03 val #{i} on zeroed"), &zero, 0, *v);
        }
    }

    /// CONFIGS row 4: store ends exactly at the end of the region.
    #[test]
    fn row04_pack_at_region_end() {
        let mut rng = Rng::new(SEED ^ 4);
        for i in 0..256 {
            let base = Arena::random(&mut rng);
            diff_pack(&format!("row04 iter {i}"), &base, ARENA - 8, rng.next_u64());
        }
    }

    // --- tflac_md5_addsample ---------------------------------------------

    /// CONFIGS row 5: pos=0, bits=64, zeroed state — the branch is not taken.
    #[test]
    fn row05_add_pos0_bits64() {
        let mut rng = Rng::new(SEED ^ 5);
        for i in 0..512 {
            let mut base = Arena::zeroed();
            base.set_pos(0);
            base.set_total(0);
            diff_add(&format!("row05 iter {i}"), &base, 64, rng.next_u64());
        }
    }

    /// CONFIGS row 6: every pos in 1..=55 (branch not taken), ramp buffer.
    #[test]
    fn row06_add_pos_1_to_55() {
        let mut rng = Rng::new(SEED ^ 6);
        for pos in 1..=55u32 {
            for i in 0..16 {
                let mut base = Arena::zeroed();
                for (k, byte) in base.b[OFF_BUFFER..OFF_BUFFER + BUFFER_LEN].iter_mut().enumerate() {
                    *byte = k as u8;
                }
                base.set_pos(pos);
                base.set_total(rng.next_u64() >> 1);
                diff_add(&format!("row06 pos={pos} iter {i}"), &base, 64, rng.next_u64());
            }
        }
    }

    /// CONFIGS row 7: pos=56, bits=64 → pos reaches exactly 64; branch taken,
    /// copy loop empty (`while (bytes--)` with bytes==0).
    #[test]
    fn row07_add_pos56_exact_64() {
        let mut rng = Rng::new(SEED ^ 7);
        for i in 0..256 {
            let mut base = Arena::random(&mut rng);
            base.set_pos(56);
            diff_add(&format!("row07 iter {i}"), &base, 64, rng.next_u64());
        }
    }

    /// CONFIGS row 8: pos 57..=63 with bits=64 → branch taken with a non-empty,
    /// fully in-bounds carry-down copy.
    #[test]
    fn row08_add_pos_57_to_63() {
        let mut rng = Rng::new(SEED ^ 8);
        for pos in 57..=63u32 {
            for i in 0..64 {
                let mut base = Arena::random(&mut rng);
                base.set_pos(pos);
                diff_add(&format!("row08 pos={pos} iter {i}"), &base, 64, rng.next_u64());
            }
        }
    }

    /// CONFIGS row 9: full pruned cross-product pos 0..=63 × bits in
    /// {0,8,16,24,32,40,48,56,64}.
    #[test]
    fn row09_add_pos_x_bits_multiples_of_8() {
        let mut rng = Rng::new(SEED ^ 9);
        for pos in 0..64u32 {
            for step in 0..=8u32 {
                let bits = step * 8;
                for i in 0..4 {
                    let mut base = Arena::random(&mut rng);
                    base.set_pos(pos);
                    diff_add(
                        &format!("row09 pos={pos} bits={bits} iter {i}"),
                        &base,
                        bits,
                        rng.next_u64(),
                    );
                }
            }
        }
    }

    /// CONFIGS row 10: bits that are *not* multiples of 8 — `bytes` truncates
    /// while `total` takes the exact value.
    #[test]
    fn row10_add_bits_non_multiples_of_8() {
        let mut rng = Rng::new(SEED ^ 10);
        for i in 0..1024 {
            let mut bits = rng.next_u32() % 200;
            if bits % 8 == 0 {
                bits += 1;
            }
            let mut base = Arena::random(&mut rng);
            base.set_pos(rng.below(64));
            diff_add(&format!("row10 iter {i} bits={bits}"), &base, bits, rng.next_u64());
        }
    }

    /// CONFIGS row 11: `total` near u64::MAX so `total += bits` wraps.
    #[test]
    fn row11_add_total_near_u64_max() {
        let mut rng = Rng::new(SEED ^ 11);
        for i in 0..512 {
            let mut base = Arena::random(&mut rng);
            base.set_pos(rng.below(64));
            base.set_total(u64::MAX - (rng.next_u64() % 128));
            diff_add(&format!("row11 iter {i}"), &base, rng.next_u32() % 256, rng.next_u64());
        }
    }

    /// CONFIGS row 12: pos >= 64 on entry — drives the out-of-bounds source
    /// region of the carry-down copy. The whole arena is randomized so the
    /// bytes past `buffer` are identical (and non-trivial) for both sides.
    #[test]
    fn row12_add_pos_out_of_array_range() {
        let mut rng = Rng::new(SEED ^ 12);
        let mut positions: Vec<u32> = vec![64, 65, 66, 71, 72, 73, 100, 127, 128, 1000, 0xFFFF];
        for _ in 0..32 {
            positions.push(rng.next_u32());
        }
        for pos in positions {
            for i in 0..8 {
                let mut base = Arena::random(&mut rng);
                base.set_pos(pos);
                diff_add(&format!("row12 pos={pos} iter {i}"), &base, 64, rng.next_u64());
                // also with bits varied, since bytes = bits/8 shifts the result
                let bits = (rng.next_u32() % 33) * 8;
                diff_add(&format!("row12 pos={pos} bits={bits} iter {i}"), &base, bits, rng.next_u64());
            }
        }
    }

    /// CONFIGS row 13: many calls against one struct — history-dependent state.
    #[test]
    fn row13_add_repeated_calls() {
        let mut rng = Rng::new(SEED ^ 13);
        for trial in 0..64 {
            let mut base = Arena::random(&mut rng);
            base.set_pos(rng.below(64));
            base.set_total(rng.next_u64() >> 2);
            let mut pair = Pair::new(&base);
            for _ in 0..64 {
                let bits = match rng.below(4) {
                    0 => 64,
                    1 => (rng.below(9)) * 8,
                    2 => rng.next_u32() % 200,
                    _ => rng.next_u32(),
                };
                pair.add(&format!("row13 trial {trial}"), bits, rng.next_u64());
            }
        }
    }

    /// CONFIGS row 14: pos=63 — the largest write offset; the 8-byte store
    /// covers `buffer[63..=70]`, one byte short of the end.
    #[test]
    fn row14_add_pos63_write_boundary() {
        let mut rng = Rng::new(SEED ^ 14);
        let mut vals: Vec<u64> = vec![0, u64::MAX, 1, 0x8000_0000_0000_0000];
        for k in 0..64 {
            vals.push(1u64 << k);
        }
        for _ in 0..64 {
            vals.push(rng.next_u64());
        }
        for (i, v) in vals.iter().enumerate() {
            let mut base = Arena::random(&mut rng);
            base.set_pos(63);
            diff_add(&format!("row14 val #{i}"), &base, 64, *v);
            let mut z = Arena::zeroed();
            z.set_pos(63);
            diff_add(&format!("row14 val #{i} zeroed"), &z, 64, *v);
        }
    }

    // --- update_md5 -------------------------------------------------------

    /// CONFIGS row 15: the baseline valid path — fresh context, random
    /// blocksize/channels, random full-range samples.
    #[test]
    fn row15_update_baseline_random() {
        let mut rng = Rng::new(SEED ^ 15);
        for i in 0..512 {
            let mut base = Arena::zeroed();
            base.set_pos(0);
            base.set_total(0);
            base.set_cbs(rng.next_u32() % 65536);
            base.set_ch(rng.below(9) + 1);
            let s = samples_random(&mut rng, MIN_SAMPLES);
            diff_update(&format!("row15 iter {i}"), &base, &s);
        }
    }

    /// CONFIGS row 16: every entry position 0..=63 of the composed pipeline.
    #[test]
    fn row16_update_pos_sweep_0_63() {
        let mut rng = Rng::new(SEED ^ 16);
        for pos in 0..64u32 {
            for i in 0..8 {
                let mut base = Arena::random(&mut rng);
                base.set_pos(pos);
                base.set_cbs(rng.next_u32());
                base.set_ch(rng.next_u32() % 17);
                let s = samples_random(&mut rng, MIN_SAMPLES + (i as usize) * 8);
                diff_update(&format!("row16 pos={pos} iter {i}"), &base, &s);
            }
        }
    }

    /// CONFIGS row 17: pos >= 64 on entry, propagated five times through the
    /// pipeline, over randomized trailing padding.
    #[test]
    fn row17_update_pos_out_of_range() {
        let mut rng = Rng::new(SEED ^ 17);
        let mut positions: Vec<u32> = vec![64, 65, 66, 71, 72, 100, 1000, 0xFFFF, u32::MAX, u32::MAX - 8];
        for _ in 0..24 {
            positions.push(rng.next_u32());
        }
        for pos in positions {
            for i in 0..8 {
                let mut base = Arena::random(&mut rng);
                base.set_pos(pos);
                base.set_cbs(rng.next_u32());
                base.set_ch(rng.next_u32());
                let s = samples_random(&mut rng, MIN_SAMPLES);
                diff_update(&format!("row17 pos={pos} iter {i}"), &base, &s);
            }
        }
    }

    /// CONFIGS row 18: degenerate / extreme sample shapes. The C casts
    /// `tflac_s32` -> `tflac_u64` (sign-extending) before masking with 0xFF, so
    /// negative values matter.
    #[test]
    fn row18_update_degenerate_sample_shapes() {
        let mut rng = Rng::new(SEED ^ 18);
        let shapes: Vec<(&str, Box<dyn Fn(usize, &mut Rng) -> i32>)> = vec![
            ("all zero", Box::new(|_, _| 0)),
            ("all -1", Box::new(|_, _| -1)),
            ("i32::MIN", Box::new(|_, _| i32::MIN)),
            ("i32::MAX", Box::new(|_, _| i32::MAX)),
            ("low byte 0x00", Box::new(|i, _| (i as i32) << 8)),
            ("low byte 0x80", Box::new(|i, _| ((i as i32) << 8) | 0x80)),
            ("low byte 0xFF", Box::new(|i, _| ((i as i32) << 8) | 0xFF)),
            ("index", Box::new(|i, _| i as i32)),
            ("neg index", Box::new(|i, _| -(i as i32))),
            ("alternating sign", Box::new(|i, _| if i % 2 == 0 { i32::MIN } else { i32::MAX })),
            ("random small", Box::new(|_, r| (r.next_u32() % 256) as i32)),
            ("random negative", Box::new(|_, r| -((r.next_u32() % 256) as i32))),
        ];
        for (name, f) in shapes.iter() {
            for pos in [0u32, 1, 31, 56, 57, 63] {
                let mut base = Arena::random(&mut rng);
                base.set_pos(pos);
                base.set_total(0);
                base.set_cbs(1024);
                base.set_ch(2);
                let s: Vec<i32> = (0..MIN_SAMPLES).map(|i| f(i, &mut rng)).collect();
                diff_update(&format!("row18 shape='{name}' pos={pos}"), &base, &s);
            }
        }
    }

    /// CONFIGS row 19: product of 0 (either factor zero) → return value wraps.
    #[test]
    fn row19_update_product_zero() {
        let mut rng = Rng::new(SEED ^ 19);
        let pairs: Vec<(u32, u32)> = vec![
            (0, 0),
            (0, 1),
            (1, 0),
            (0, 0xFFFF_FFFF),
            (0xFFFF_FFFF, 0),
            (0, 2),
            (12345, 0),
        ];
        for (cbs, ch) in pairs {
            for i in 0..8 {
                let mut base = Arena::random(&mut rng);
                base.set_pos(rng.below(64));
                base.set_cbs(cbs);
                base.set_ch(ch);
                let s = samples_random(&mut rng, MIN_SAMPLES);
                diff_update(&format!("row19 cbs={cbs} ch={ch} iter {i}"), &base, &s);
            }
        }
    }

    /// CONFIGS row 20: the underflow boundary of `b -= 8` five times — products
    /// below, at and just above 40.
    #[test]
    fn row20_update_product_underflow_boundary() {
        let mut rng = Rng::new(SEED ^ 20);
        // (cbs, ch) pairs whose products sweep 1..=48 plus the exact boundary.
        let mut pairs: Vec<(u32, u32)> = Vec::new();
        for p in 1..=48u32 {
            pairs.push((p, 1));
            pairs.push((1, p));
        }
        pairs.push((5, 8)); // == 40
        pairs.push((8, 5)); // == 40
        pairs.push((4, 2)); // == 8
        pairs.push((41, 1));
        pairs.push((39, 1));
        for (cbs, ch) in pairs {
            let mut base = Arena::random(&mut rng);
            base.set_pos(rng.below(64));
            base.set_cbs(cbs);
            base.set_ch(ch);
            let s = samples_random(&mut rng, MIN_SAMPLES);
            diff_update(&format!("row20 cbs={cbs} ch={ch}"), &base, &s);
        }
    }

    /// CONFIGS row 21: `cur_blocksize * channels` overflowing u32.
    #[test]
    fn row21_update_product_overflow() {
        let mut rng = Rng::new(SEED ^ 21);
        let mut pairs: Vec<(u32, u32)> = vec![
            (0x1_0000, 0x1_0000),
            (0xFFFF_FFFF, 3),
            (3, 0xFFFF_FFFF),
            (0xFFFF_FFFF, 0xFFFF_FFFF),
            (0x8000_0000, 2),
            (2, 0x8000_0000),
            (0x1_0001, 0x1_0001),
        ];
        for _ in 0..32 {
            pairs.push((rng.next_u32(), rng.next_u32()));
        }
        for (cbs, ch) in pairs {
            for i in 0..4 {
                let mut base = Arena::random(&mut rng);
                base.set_pos(rng.below(64));
                base.set_cbs(cbs);
                base.set_ch(ch);
                let s = samples_random(&mut rng, MIN_SAMPLES);
                diff_update(&format!("row21 cbs={cbs} ch={ch} iter {i}"), &base, &s);
            }
        }
    }

    /// CONFIGS row 22: `total` near u64::MAX — the five `+= 64` overflow it.
    #[test]
    fn row22_update_total_near_u64_max() {
        let mut rng = Rng::new(SEED ^ 22);
        for i in 0..256 {
            let mut base = Arena::random(&mut rng);
            base.set_pos(rng.below(64));
            base.set_total(u64::MAX - (rng.next_u64() % 400));
            base.set_cbs(rng.next_u32());
            base.set_ch(rng.next_u32() % 9);
            let s = samples_random(&mut rng, MIN_SAMPLES);
            diff_update(&format!("row22 iter {i}"), &base, &s);
        }
    }

    /// CONFIGS row 23: stride verification. `samples` advances by
    /// `8 * sizeof(tflac_s32) == 32` elements per iteration, so only indices
    /// 0..7, 32..39, 64..71, 96..103 and 128..135 are read. Every element gets a
    /// distinct low byte, so any wrong stride diverges immediately. Also runs at
    /// exactly the minimum safe length.
    #[test]
    fn row23_update_stride_and_min_length() {
        let mut rng = Rng::new(SEED ^ 23);
        for i in 0..64 {
            let mut base = Arena::random(&mut rng);
            base.set_pos(rng.below(64));
            base.set_cbs(rng.next_u32());
            base.set_ch(rng.next_u32() % 9);
            // distinct-valued long buffer
            let long: Vec<i32> = (0..4096).map(|k| (k as i32).wrapping_mul(2_654_435_761u32 as i32)).collect();
            diff_update(&format!("row23 long iter {i}"), &base, &long);
            // exactly the minimum non-OOB length
            let exact: Vec<i32> = (0..MIN_SAMPLES).map(|k| (k as i32) | ((k as i32) << 16)).collect();
            diff_update(&format!("row23 exact iter {i}"), &base, &exact);
            // ramp of every low-byte value
            let ramp: Vec<i32> = (0..MIN_SAMPLES).map(|k| (k as i32) - 68).collect();
            diff_update(&format!("row23 ramp iter {i}"), &base, &ramp);
        }
    }

    /// CONFIGS row 24: consecutive `update_md5` calls accumulating state.
    #[test]
    fn row24_update_repeated_calls() {
        let mut rng = Rng::new(SEED ^ 24);
        for trial in 0..64 {
            let mut base = Arena::random(&mut rng);
            base.set_pos(rng.below(64));
            base.set_total(rng.next_u64() >> 2);
            base.set_cbs(rng.next_u32());
            base.set_ch(rng.next_u32() % 9);
            let mut pair = Pair::new(&base);
            for _ in 0..10 {
                let extra = rng.below(64) as usize;
                let s = samples_random(&mut rng, MIN_SAMPLES + extra);
                pair.update(&format!("row24 trial {trial}"), &s);
            }
        }
    }

    /// CONFIGS row 25: the composed pipeline — all three entry points
    /// interleaved against one shared struct, driven the way a real consumer
    /// would drive it, for many randomized steps.
    #[test]
    fn row25_composed_pipeline_all_entry_points() {
        let mut rng = Rng::new(SEED ^ 25);
        for trial in 0..16 {
            let mut base = Arena::random(&mut rng);
            base.set_pos(rng.below(64));
            base.set_total(rng.next_u64() >> 3);
            base.set_cbs(rng.next_u32());
            base.set_ch(rng.next_u32() % 9);
            let mut pair = Pair::new(&base);
            let ctx = format!("row25 trial {trial}");
            for _ in 0..200 {
                match rng.below(3) {
                    0 => {
                        let s = samples_random(&mut rng, MIN_SAMPLES);
                        pair.update(&ctx, &s);
                    }
                    1 => {
                        let bits = if rng.below(2) == 0 { 64 } else { rng.next_u32() % 300 };
                        pair.add(&ctx, bits, rng.next_u64());
                    }
                    _ => {
                        // write straight into the md5 buffer through the
                        // low-level packer, at a random in-buffer offset
                        let off = OFF_BUFFER + (rng.below(BUFFER_LEN as u32 - 8) as usize);
                        pair.pack(&ctx, off, rng.next_u64());
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Phase C — error / rejection differential tests, one test per ERRORS.md row
// ===========================================================================

// The C dereferences its pointer arguments unconditionally, so the only
// observable "rejection" for a null argument is the fatal signal. To compare
// that *identically* (same signal, not merely "both failed"), each side runs in
// its own forked child.
unsafe extern "C" {
    fn fork() -> i32;
    fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
    fn _exit(status: i32) -> !;
}

/// Result of running a call in a child process.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Outcome {
    /// Terminated by signal N (e.g. 11 = SIGSEGV).
    Signal(i32),
    /// Returned normally with this exit code.
    Exit(i32),
}

fn crash_probe<F: FnOnce()>(f: F) -> Outcome {
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork() failed");
        if pid == 0 {
            f();
            _exit(0);
        }
        let mut status: i32 = 0;
        let w = waitpid(pid, &mut status, 0);
        assert_eq!(w, pid, "waitpid failed");
        let sig = status & 0x7f;
        if sig != 0 && sig != 0x7f {
            Outcome::Signal(sig)
        } else {
            Outcome::Exit((status >> 8) & 0xff)
        }
    }
}

fn assert_same_outcome(ctx: &str, c: Outcome, r: Outcome) {
    assert_eq!(
        c, r,
        "REJECTION DIVERGENCE in {ctx}: C terminated with {c:?} but Rust terminated with {r:?}"
    );
}

mod errors {
    use super::*;

    // --- rows 1-4: null pointers (same fatal signal, compared exactly) -----

    /// ERRORS row 1: `tflac_pack_u64le(NULL, n)`.
    #[test]
    fn row01_pack_null_dst() {
        let (c, r) = impls();
        for n in [0u64, 1, u64::MAX, 0x0123_4567_89AB_CDEF] {
            let oc = crash_probe(|| unsafe { (c.pack)(std::ptr::null_mut(), n) });
            let or = crash_probe(|| unsafe { (r.pack)(std::ptr::null_mut(), n) });
            assert_same_outcome(&format!("row01 pack(NULL, 0x{n:016x})"), oc, or);
            assert!(
                matches!(oc, Outcome::Signal(_)),
                "row01: expected the C to die on a null store, got {oc:?}"
            );
        }
    }

    /// ERRORS row 2: `tflac_md5_addsample(NULL, bits, val)`.
    #[test]
    fn row02_addsample_null_ctx() {
        let (c, r) = impls();
        for bits in [0u32, 8, 64, u32::MAX] {
            let oc = crash_probe(|| unsafe { (c.add)(std::ptr::null_mut(), bits, 0xDEAD_BEEF) });
            let or = crash_probe(|| unsafe { (r.add)(std::ptr::null_mut(), bits, 0xDEAD_BEEF) });
            assert_same_outcome(&format!("row02 addsample(NULL, {bits}, ..)"), oc, or);
            assert!(matches!(oc, Outcome::Signal(_)), "row02: expected the C to die, got {oc:?}");
        }
    }

    /// ERRORS row 3: `update_md5(NULL, samples)`.
    #[test]
    fn row03_update_md5_null_t() {
        let (c, r) = impls();
        let s: Vec<i32> = (0..MIN_SAMPLES).map(|i| i as i32).collect();
        let p = s.as_ptr();
        let oc = crash_probe(|| unsafe {
            let _ = (c.update)(std::ptr::null_mut(), p);
        });
        let or = crash_probe(|| unsafe {
            let _ = (r.update)(std::ptr::null_mut(), p);
        });
        assert_same_outcome("row03 update_md5(NULL, samples)", oc, or);
        assert!(matches!(oc, Outcome::Signal(_)), "row03: expected the C to die, got {oc:?}");
    }

    /// ERRORS row 4: `update_md5(t, NULL)`.
    #[test]
    fn row04_update_md5_null_samples() {
        let (c, r) = impls();
        let mut rng = Rng::new(SEED ^ 104);
        let mut base = Arena::random(&mut rng);
        base.set_pos(0);
        base.set_cbs(1024);
        base.set_ch(2);
        let mut ac = base.clone();
        let mut ar = base.clone();
        let pc = ac.b.as_mut_ptr();
        let pr = ar.b.as_mut_ptr();
        let oc = crash_probe(|| unsafe {
            let _ = (c.update)(pc, std::ptr::null());
        });
        let or = crash_probe(|| unsafe {
            let _ = (r.update)(pr, std::ptr::null());
        });
        assert_same_outcome("row04 update_md5(t, NULL)", oc, or);
        assert!(matches!(oc, Outcome::Signal(_)), "row04: expected the C to die, got {oc:?}");
    }

    // --- rows 5-8: the `bits` parameter (the only length-like scalar) -------

    /// ERRORS row 5: `bits == 0` (zero length), with the branch both untaken
    /// (`pos < 64`) and taken (`pos >= 64`).
    #[test]
    fn row05_bits_zero() {
        let mut rng = Rng::new(SEED ^ 105);
        for pos in [0u32, 1, 8, 32, 63, 64, 65, 127, 1000, u32::MAX] {
            for i in 0..16 {
                let mut base = Arena::random(&mut rng);
                base.set_pos(pos);
                diff_add(&format!("row05 pos={pos} iter {i}"), &base, 0, rng.next_u64());
                let mut z = Arena::zeroed();
                z.set_pos(pos);
                diff_add(&format!("row05 zeroed pos={pos} iter {i}"), &z, 0, rng.next_u64());
            }
        }
    }

    /// ERRORS row 6: `bits` not a multiple of 8 — `bytes` truncates to 0 for
    /// 1..=7 while `total` still absorbs the full value.
    #[test]
    fn row06_bits_not_multiple_of_8() {
        let mut rng = Rng::new(SEED ^ 106);
        for bits in 1..=7u32 {
            for pos in [0u32, 1, 56, 63, 64, 100] {
                let mut base = Arena::random(&mut rng);
                base.set_pos(pos);
                base.set_total(rng.next_u64());
                diff_add(&format!("row06 bits={bits} pos={pos}"), &base, bits, rng.next_u64());
            }
        }
        // and a wide randomized sweep of non-multiples
        for i in 0..512 {
            let mut bits = rng.next_u32();
            if bits % 8 == 0 {
                bits ^= 3;
            }
            let mut base = Arena::random(&mut rng);
            base.set_pos(rng.next_u32());
            diff_add(&format!("row06 random iter {i} bits={bits}"), &base, bits, rng.next_u64());
        }
    }

    /// ERRORS row 7: `bits == 65`, one step past the 64 that `update_md5` uses.
    #[test]
    fn row07_bits_65_one_past() {
        let mut rng = Rng::new(SEED ^ 107);
        for bits in [63u32, 64, 65, 71, 72] {
            for pos in 0..64u32 {
                let mut base = Arena::random(&mut rng);
                base.set_pos(pos);
                diff_add(&format!("row07 bits={bits} pos={pos}"), &base, bits, rng.next_u64());
            }
        }
    }

    /// ERRORS row 8: oversized `bits` — up to u32::MAX, where `pos += bits/8`
    /// wraps mod 2^32.
    #[test]
    fn row08_bits_u32_max() {
        let mut rng = Rng::new(SEED ^ 108);
        let mut values: Vec<u32> = vec![
            u32::MAX,
            u32::MAX - 1,
            u32::MAX - 7,
            0x8000_0000,
            0x7FFF_FFFF,
            0xFFFF_FFF8,
            512,
            0x1_0000,
        ];
        for _ in 0..64 {
            values.push(rng.next_u32());
        }
        for bits in values {
            for pos in [0u32, 1, 63, 64, 1000, u32::MAX] {
                let mut base = Arena::random(&mut rng);
                base.set_pos(pos);
                base.set_total(rng.next_u64());
                diff_add(&format!("row08 bits={bits} pos={pos}"), &base, bits, rng.next_u64());
            }
        }
    }

    // --- rows 9-16: the `pos` state machine boundaries ---------------------

    /// ERRORS row 9: `pos + bits/8 == 64` exactly — branch taken, `bytes == 0`,
    /// so `while (bytes--)` must NOT execute its body.
    #[test]
    fn row09_pos_plus_bytes_exactly_64() {
        let mut rng = Rng::new(SEED ^ 109);
        // every (pos, bits) with pos + bits/8 == 64 for bits a multiple of 8
        for k in 0..=8u32 {
            let bits = k * 8;
            let pos = 64 - k;
            for i in 0..16 {
                let mut base = Arena::random(&mut rng);
                base.set_pos(pos);
                diff_add(&format!("row09 pos={pos} bits={bits} iter {i}"), &base, bits, rng.next_u64());
            }
        }
        // larger bits reaching exactly 64 from a low pos
        for (pos, bits) in [(0u32, 512u32), (32, 256), (48, 128), (56, 64), (60, 32), (62, 16)] {
            let mut base = Arena::random(&mut rng);
            base.set_pos(pos);
            diff_add(&format!("row09 wide pos={pos} bits={bits}"), &base, bits, rng.next_u64());
        }
    }

    /// ERRORS row 10: `pos == 63`, the largest documented position.
    #[test]
    fn row10_pos_63_max_valid() {
        let mut rng = Rng::new(SEED ^ 110);
        for i in 0..256 {
            let mut base = Arena::random(&mut rng);
            base.set_pos(63);
            diff_add(&format!("row10 iter {i}"), &base, 64, rng.next_u64());
        }
    }

    /// ERRORS row 11: `pos == 64`, one step past the valid range; the copy loop
    /// touches exactly the final in-bounds byte `buffer[71]`.
    #[test]
    fn row11_pos_64_one_past_valid() {
        let mut rng = Rng::new(SEED ^ 111);
        for i in 0..256 {
            let mut base = Arena::random(&mut rng);
            base.set_pos(64);
            diff_add(&format!("row11 iter {i}"), &base, 64, rng.next_u64());
        }
    }

    /// ERRORS row 12: `pos == 65`, the first input for which the C reads past
    /// the end of `buffer` (`buffer[72]`).
    #[test]
    fn row12_pos_65_first_oob() {
        let mut rng = Rng::new(SEED ^ 112);
        for pos in [65u32, 66, 67, 68] {
            for i in 0..64 {
                let mut base = Arena::random(&mut rng);
                base.set_pos(pos);
                diff_add(&format!("row12 pos={pos} iter {i}"), &base, 64, rng.next_u64());
            }
        }
    }

    /// ERRORS row 13: `pos == 1000` — a deep out-of-bounds read that runs past
    /// the end of `struct tflac` itself.
    #[test]
    fn row13_pos_1000_deep_oob() {
        let mut rng = Rng::new(SEED ^ 113);
        for pos in [1000u32, 1001, 4096, 0xDEAD, 0x7FFF_FFFF] {
            for i in 0..32 {
                let mut base = Arena::random(&mut rng);
                base.set_pos(pos);
                diff_add(&format!("row13 pos={pos} iter {i}"), &base, 64, rng.next_u64());
            }
        }
    }

    /// ERRORS row 14: `pos == u32::MAX` — `pos + 8` wraps to 7, so `>= 64` is
    /// false and the branch is NOT taken.
    #[test]
    fn row14_pos_u32_max_wraps_below_64() {
        let mut rng = Rng::new(SEED ^ 114);
        for pos in [u32::MAX, u32::MAX - 1, u32::MAX - 7, u32::MAX - 8, u32::MAX - 63] {
            for i in 0..32 {
                let mut base = Arena::random(&mut rng);
                base.set_pos(pos);
                diff_add(&format!("row14 pos={pos} iter {i}"), &base, 64, rng.next_u64());
                diff_add(&format!("row14 pos={pos} bits=8 iter {i}"), &base, 8, rng.next_u64());
            }
        }
    }

    /// ERRORS row 15: `total == u64::MAX` — the accumulator overflows.
    #[test]
    fn row15_total_u64_max_wraps() {
        let mut rng = Rng::new(SEED ^ 115);
        for total in [u64::MAX, u64::MAX - 1, u64::MAX - 63, u64::MAX - 64, 0x7FFF_FFFF_FFFF_FFFF] {
            for i in 0..32 {
                let mut base = Arena::random(&mut rng);
                base.set_total(total);
                base.set_pos(rng.below(64));
                diff_add(&format!("row15 total={total} iter {i}"), &base, 64, rng.next_u64());
                diff_add(
                    &format!("row15 total={total} bits=max iter {i}"),
                    &base,
                    u32::MAX,
                    rng.next_u64(),
                );
            }
        }
    }

    /// ERRORS row 16: write boundary — every `pos` with `pos % 64 == 63`, where
    /// the 8-byte store lands on `buffer[63..=70]`.
    #[test]
    fn row16_write_boundary_pos63() {
        let mut rng = Rng::new(SEED ^ 116);
        for pos in [63u32, 127, 191, 255, 65535, 0xFFFF_FFBF] {
            for i in 0..32 {
                let mut base = Arena::random(&mut rng);
                base.set_pos(pos);
                diff_add(&format!("row16 pos={pos} iter {i}"), &base, 64, u64::MAX);
                diff_add(&format!("row16 pos={pos} rnd iter {i}"), &base, 64, rng.next_u64());
            }
        }
    }

    // --- rows 17-23: update_md5 arithmetic and length boundaries -----------

    /// ERRORS row 17: `channels == 0` → `b == 0`, then five `b -= 8` wrap.
    #[test]
    fn row17_channels_zero() {
        let mut rng = Rng::new(SEED ^ 117);
        for (cbs, ch) in [(0u32, 0u32), (1024, 0), (0, 2), (0xFFFF_FFFF, 0), (0, 0xFFFF_FFFF)] {
            for i in 0..16 {
                let mut base = Arena::random(&mut rng);
                base.set_pos(rng.below(64));
                base.set_cbs(cbs);
                base.set_ch(ch);
                let s = samples_random(&mut rng, MIN_SAMPLES);
                diff_update(&format!("row17 cbs={cbs} ch={ch} iter {i}"), &base, &s);
            }
        }
    }

    /// ERRORS row 18: `0 < cur_blocksize * channels < 40` → the return value
    /// underflows.
    #[test]
    fn row18_product_below_40() {
        let mut rng = Rng::new(SEED ^ 118);
        for p in 1..40u32 {
            let mut base = Arena::random(&mut rng);
            base.set_pos(rng.below(64));
            base.set_cbs(p);
            base.set_ch(1);
            let s = samples_random(&mut rng, MIN_SAMPLES);
            diff_update(&format!("row18 product={p}"), &base, &s);
        }
        for (cbs, ch) in [(4u32, 2u32), (2, 4), (3, 13), (13, 3), (1, 39), (39, 1)] {
            let mut base = Arena::random(&mut rng);
            base.set_pos(rng.below(64));
            base.set_cbs(cbs);
            base.set_ch(ch);
            let s = samples_random(&mut rng, MIN_SAMPLES);
            diff_update(&format!("row18 cbs={cbs} ch={ch}"), &base, &s);
        }
    }

    /// ERRORS row 19: product exactly 40 — the boundary where the result stops
    /// underflowing and becomes 0.
    #[test]
    fn row19_product_exactly_40() {
        let mut rng = Rng::new(SEED ^ 119);
        for (cbs, ch) in [(40u32, 1u32), (1, 40), (5, 8), (8, 5), (4, 10), (10, 4), (20, 2), (2, 20)] {
            for i in 0..8 {
                let mut base = Arena::random(&mut rng);
                base.set_pos(rng.below(64));
                base.set_cbs(cbs);
                base.set_ch(ch);
                let s = samples_random(&mut rng, MIN_SAMPLES);
                diff_update(&format!("row19 cbs={cbs} ch={ch} iter {i}"), &base, &s);
            }
        }
        // one step either side of the boundary
        for p in [39u32, 40, 41] {
            let mut base = Arena::random(&mut rng);
            base.set_pos(0);
            base.set_cbs(p);
            base.set_ch(1);
            let s = samples_random(&mut rng, MIN_SAMPLES);
            diff_update(&format!("row19 boundary product={p}"), &base, &s);
        }
    }

    /// ERRORS row 20: `cur_blocksize * channels` overflows u32.
    #[test]
    fn row20_product_overflows_u32() {
        let mut rng = Rng::new(SEED ^ 120);
        let mut pairs: Vec<(u32, u32)> = vec![
            (0x1_0000, 0x1_0000),
            (0x1_0000, 0x1_0001),
            (0xFFFF_FFFF, 0xFFFF_FFFF),
            (0xFFFF_FFFF, 2),
            (0x8000_0000, 3),
            (0x4000_0000, 4),
        ];
        for _ in 0..48 {
            pairs.push((rng.next_u32() | 0x8000_0000, rng.next_u32() | 2));
        }
        for (cbs, ch) in pairs {
            let mut base = Arena::random(&mut rng);
            base.set_pos(rng.below(64));
            base.set_cbs(cbs);
            base.set_ch(ch);
            let s = samples_random(&mut rng, MIN_SAMPLES);
            diff_update(&format!("row20 cbs={cbs} ch={ch}"), &base, &s);
        }
    }

    /// ERRORS row 21: a `samples` array shorter than the 136 elements the C
    /// reads. The short array is embedded in a larger, deterministically filled
    /// backing region so the out-of-bounds reads hit memory both
    /// implementations see identically — otherwise any divergence would be an
    /// artefact of the allocator, not of the translation.
    #[test]
    fn row21_samples_shorter_than_136() {
        let mut rng = Rng::new(SEED ^ 121);
        for logical_len in [0usize, 1, 8, 9, 32, 33, 64, 100, 128, 129, 135] {
            for i in 0..8 {
                // backing region: the logical array, then a distinct filler
                // pattern standing in for "whatever follows a too-short array".
                let mut backing: Vec<i32> = Vec::with_capacity(4096);
                for k in 0..4096usize {
                    if k < logical_len {
                        backing.push(rng.next_i32());
                    } else {
                        backing.push(0x5A5A_0000u32 as i32 | (k as i32 & 0xFFFF));
                    }
                }
                let mut base = Arena::random(&mut rng);
                base.set_pos(rng.below(64));
                base.set_cbs(rng.next_u32());
                base.set_ch(rng.next_u32() % 9);
                diff_update(&format!("row21 logical_len={logical_len} iter {i}"), &base, &backing);
            }
        }
    }

    /// ERRORS row 22: `md5_ctx.pos` out of range on entry to `update_md5`, so
    /// the out-of-range/OOB behaviour is exercised five times in sequence.
    #[test]
    fn row22_ctx_pos_out_of_range() {
        let mut rng = Rng::new(SEED ^ 122);
        for pos in [64u32, 65, 66, 71, 72, 100, 1000, 0xFFFF, 0x7FFF_FFFF, u32::MAX, u32::MAX - 40] {
            for i in 0..16 {
                let mut base = Arena::random(&mut rng);
                base.set_pos(pos);
                base.set_cbs(rng.next_u32());
                base.set_ch(rng.next_u32() % 9);
                let s = samples_random(&mut rng, MIN_SAMPLES);
                diff_update(&format!("row22 pos={pos} iter {i}"), &base, &s);
            }
        }
    }

    /// ERRORS row 23: `md5_ctx.total` near u64::MAX so the five `+= 64` overflow.
    #[test]
    fn row23_total_near_u64_max() {
        let mut rng = Rng::new(SEED ^ 123);
        for total in [
            u64::MAX,
            u64::MAX - 1,
            u64::MAX - 63,
            u64::MAX - 64,
            u64::MAX - 319,
            u64::MAX - 320,
            u64::MAX - 321,
        ] {
            for i in 0..8 {
                let mut base = Arena::random(&mut rng);
                base.set_pos(rng.below(64));
                base.set_total(total);
                base.set_cbs(rng.next_u32());
                base.set_ch(rng.next_u32() % 9);
                let s = samples_random(&mut rng, MIN_SAMPLES);
                diff_update(&format!("row23 total={total} iter {i}"), &base, &s);
            }
        }
    }

    // --- rows 24-25: documented-empty surfaces, re-verified mechanically ----

    /// ERRORS rows 24 & 25: the C API has no `enum` parameter (so there is no
    /// out-of-range discriminant to pass across the FFI boundary) and no
    /// length/count/size parameter. Re-grep the C at test time so the claim
    /// cannot silently rot.
    #[test]
    fn enum_and_length_surface_is_empty() {
        let root = workspace_root();
        let mut sources = String::new();
        for rel in ["c_src/include/lib.h", "c_src/src/lib.c"] {
            let p = root.join(rel);
            sources.push_str(&std::fs::read_to_string(&p).unwrap_or_else(|e| {
                panic!("cannot read {}: {e}", p.display());
            }));
            sources.push('\n');
        }
        // Row 24: no enum types at all.
        assert!(
            !sources.contains("enum"),
            "ERRORS.md row 24 claims the C API has no enum; the source now contains one \
             and an out-of-range-discriminant differential test must be added"
        );
        // Row 25: no length/count/size parameters in the public signatures.
        for banned in ["size_t", "count", "nbytes", "nsamples"] {
            assert!(
                !sources.contains(banned),
                "ERRORS.md row 25 claims the C API has no length parameter, but the source \
                 now mentions `{banned}`; add a zero/oversized-length differential test"
            );
        }
        // And the error surface really is implicit: no explicit error machinery.
        for banned in ["assert", "RETURN_ERROR", "return -1", "return NULL", "errno"] {
            assert!(
                !sources.contains(banned),
                "ERRORS.md asserts the C has no explicit error machinery, but `{banned}` \
                 now appears; ERRORS.md must gain a row for it"
            );
        }
    }

    /// Generic FFI boundary sanity not tied to a single ERRORS.md row: every
    /// scalar argument driven over its extremes, in combination.
    #[test]
    fn generic_scalar_extremes_cross_product() {
        let mut rng = Rng::new(SEED ^ 200);
        let bits_set = [0u32, 1, 7, 8, 9, 63, 64, 65, 127, 128, 0x7FFF_FFFF, 0x8000_0000, u32::MAX];
        let pos_set = [0u32, 1, 55, 56, 57, 63, 64, 65, 71, 72, 127, 128, 1000, u32::MAX];
        let val_set = [0u64, 1, u64::MAX, 0x8000_0000_0000_0000, 0x00FF_00FF_00FF_00FF];
        for &bits in bits_set.iter() {
            for &pos in pos_set.iter() {
                for &val in val_set.iter() {
                    let mut base = Arena::random(&mut rng);
                    base.set_pos(pos);
                    base.set_total(rng.next_u64());
                    diff_add(&format!("generic bits={bits} pos={pos} val={val:#x}"), &base, bits, val);
                }
            }
        }
        // and the same for update_md5's two u32 fields
        let field_set = [0u32, 1, 5, 8, 39, 40, 41, 0xFFFF, 0x1_0000, 0x7FFF_FFFF, 0x8000_0000, u32::MAX];
        for &cbs in field_set.iter() {
            for &ch in field_set.iter() {
                let mut base = Arena::random(&mut rng);
                base.set_pos(rng.next_u32());
                base.set_cbs(cbs);
                base.set_ch(ch);
                let s = samples_random(&mut rng, MIN_SAMPLES);
                diff_update(&format!("generic cbs={cbs} ch={ch}"), &base, &s);
            }
        }
    }
}

// ===========================================================================
// Phase D — symbol parity, asserted from inside the test suite
// ===========================================================================

mod symbols {
    use super::*;
    use std::process::Command;

    fn defined_symbols(so: &Path) -> Vec<String> {
        let out = Command::new("nm")
            .args(["-D", "--defined-only", "--format=posix"])
            .arg(so)
            .output()
            .unwrap_or_else(|e| panic!("cannot run nm on {}: {e}", so.display()));
        assert!(out.status.success(), "nm failed on {}", so.display());
        let text = String::from_utf8_lossy(&out.stdout);
        let mut v: Vec<String> = text
            .lines()
            .filter_map(|l| {
                let mut it = l.split_whitespace();
                let name = it.next()?;
                let kind = it.next()?;
                // Exported code/data only; skip the Rust/toolchain-internal ones
                // that are not part of the C ABI surface.
                if matches!(kind, "T" | "t" | "D" | "B" | "R" | "W")
                    && !name.starts_with("__")
                    && !name.starts_with("_ITM_")
                    && !name.starts_with("_fini")
                    && !name.starts_with("_init")
                    && !name.starts_with("rust_")
                    && !name.starts_with("_Z")
                {
                    Some(name.to_string())
                } else {
                    None
                }
            })
            .collect();
        v.sort();
        v.dedup();
        v
    }

    /// Every symbol the C `.so` exports must also be exported by the Rust
    /// `.so`, with the exact same name. The diff must be empty.
    #[test]
    fn every_c_symbol_is_exported_by_rust() {
        let c = defined_symbols(&c_so_path());
        let r = defined_symbols(&rust_so_path());
        assert!(!c.is_empty(), "nm reported no symbols for the C .so — bad build?");
        let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
        assert!(
            missing.is_empty(),
            "SYMBOL PARITY FAILURE: the Rust .so is missing {} of the C .so's {} exported \
             symbols: {missing:?}\n  C   : {c:?}\n  Rust: {r:?}",
            missing.len(),
            c.len()
        );
        // Sanity: the three known entry points really are there.
        for want in ["tflac_pack_u64le", "tflac_md5_addsample", "update_md5"] {
            assert!(c.contains(&want.to_string()), "C .so lacks {want}");
            assert!(r.contains(&want.to_string()), "Rust .so lacks {want}");
        }
    }

    /// The Rust `.so` must not leave any non-libc symbol undefined.
    #[test]
    fn rust_so_has_no_unresolved_non_libc_symbols() {
        let out = Command::new("nm")
            .args(["-D", "--undefined-only", "--format=posix"])
            .arg(rust_so_path())
            .output()
            .expect("nm");
        assert!(out.status.success());
        let text = String::from_utf8_lossy(&out.stdout);
        // Everything the Rust cdylib imports must come from libc / libgcc's
        // unwinder / be a weak toolchain hook.
        let allowed_prefixes = ["_Unwind_", "_ITM_", "__"];
        let allowed_exact = [
            "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat64", "getcwd",
            "getenv", "gettid", "lseek64", "malloc", "memcmp", "memcpy", "memmove", "memset",
            "mmap64", "munmap", "open64", "posix_memalign", "pthread_key_create",
            "pthread_key_delete", "pthread_getspecific", "pthread_setspecific", "read", "readlink",
            "realloc", "realpath", "stat64", "statx", "strlen", "syscall", "write", "writev",
            "sysconf", "pthread_self", "pthread_mutex_lock", "pthread_mutex_unlock", "poll",
            "sigaction", "sigaltstack", "mprotect", "getrandom", "memrchr", "qsort", "exit",
        ];
        let mut bad = Vec::new();
        for line in text.lines() {
            let mut it = line.split_whitespace();
            let name = match it.next() {
                Some(n) => n,
                None => continue,
            };
            let base = name.split('@').next().unwrap_or(name);
            let ok = allowed_prefixes.iter().any(|p| base.starts_with(p))
                || allowed_exact.contains(&base)
                || base.starts_with("pthread_")
                || base.starts_with("_dl");
            if !ok {
                bad.push(base.to_string());
            }
        }
        assert!(
            bad.is_empty(),
            "Rust .so has unresolved non-libc symbols (a sign of a missing translation \
             unit or a stub): {bad:?}"
        );
    }
}
