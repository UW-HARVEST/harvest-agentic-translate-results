//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and calls every function
//! exclusively through its exported C symbol — the Rust crate is never linked
//! or called directly, so the `#[no_mangle] extern "C"` wrappers are part of
//! what is under test.
//!
//! Both implementations are run over byte-identical, deterministically
//! initialised *arenas*.  That matters because the C code performs
//! out-of-bounds reads past `tflac_md5::buffer` (see `ERRORS.md` E10/E14): by
//! giving each side its own copy of the same 4 KiB arena, those reads become
//! defined and directly comparable instead of reading unrelated stack garbage.

#![allow(dead_code)]

use libloading::Library;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// C ABI of the three exported symbols
// ---------------------------------------------------------------------------

/// `void tflac_pack_u64le(tflac_u8 *d, tflac_u64 n)`
pub type PackFn = unsafe extern "C" fn(*mut u8, u64);
/// `void tflac_md5_addsample(tflac_md5 *m, tflac_u32 bits, tflac_uint val)`
pub type AddFn = unsafe extern "C" fn(*mut u8, u32, u64);
/// `tflac_u32 update_md5(tflac *t, const tflac_s32 *samples)`
pub type UpdFn = unsafe extern "C" fn(*mut u8, *const i32) -> u32;

pub struct Side {
    pub name: &'static str,
    pub pack: PackFn,
    pub add: AddFn,
    pub upd: UpdFn,
}

pub struct Libs {
    pub c: Side,
    pub r: Side,
}

fn manifest_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> std::path::PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return p.into();
    }
    manifest_dir().join("c_src/build/libtranslated_rust.so")
}

pub fn rust_so_path() -> std::path::PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return p.into();
    }
    manifest_dir().join("target/debug/libupdate_md5_lib.so")
}

fn load(name: &'static str, path: &std::path::Path) -> Side {
    // Leaked on purpose: the resolved fn pointers must outlive any Symbol
    // borrow, and the library must stay mapped for the whole test process.
    let lib: &'static Library = Box::leak(Box::new(unsafe {
        Library::new(path).unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()))
    }));
    unsafe {
        let pack = *lib
            .get::<PackFn>(b"tflac_pack_u64le\0")
            .expect("missing symbol tflac_pack_u64le");
        let add = *lib
            .get::<AddFn>(b"tflac_md5_addsample\0")
            .expect("missing symbol tflac_md5_addsample");
        let upd = *lib
            .get::<UpdFn>(b"update_md5\0")
            .expect("missing symbol update_md5");
        Side { name, pack, add, upd }
    }
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        assert_not_stale();
        Libs {
            c: load("C", &c_so_path()),
            r: load("RUST", &rust_so_path()),
        }
    })
}

fn mtime(p: &std::path::Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(p).ok()?.modified().ok()
}

/// `cargo test` does **not** rebuild a `crate-type = ["cdylib"]` target — the
/// integration tests do not link the library, they `dlopen` it.  Without this
/// guard the whole suite would silently validate a *stale* `.so` and pass no
/// matter what `src/lib.rs` says.  Fail loudly instead.
fn assert_not_stale() {
    let root = manifest_dir();
    let rs = rust_so_path();
    let cs = c_so_path();

    let so_t = match mtime(&rs) {
        Some(t) => t,
        None => panic!(
            "Rust .so not found at {}\n  run: cargo build   (and/or cargo build --release)",
            rs.display()
        ),
    };
    assert!(
        cs.exists(),
        "C .so not found at {}\n  run: cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        cs.display()
    );

    // newest Rust source / manifest
    let mut newest: Option<(std::path::PathBuf, std::time::SystemTime)> = None;
    let mut consider = |p: std::path::PathBuf| {
        if let Some(t) = mtime(&p) {
            if newest.as_ref().map(|(_, n)| t > *n).unwrap_or(true) {
                newest = Some((p, t));
            }
        }
    };
    consider(root.join("Cargo.toml"));
    if let Ok(rd) = std::fs::read_dir(root.join("src")) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "rs").unwrap_or(false) {
                consider(p);
            }
        }
    }
    if let Some((p, t)) = newest {
        assert!(
            so_t >= t,
            "STALE Rust .so: {} was built at {:?} but {} was modified at {:?}.\n\
             `cargo test` does NOT rebuild a cdylib — run `cargo build` (or \
             `./run_all.sh`) before testing, otherwise the differential tests \
             validate an out-of-date shared object.",
            rs.display(),
            so_t,
            p.display(),
            t
        );
    }

    // same guard for the C side
    if let Some(c_t) = mtime(&cs) {
        for f in ["c_src/src/lib.c", "c_src/include/lib.h"] {
            let p = root.join(f);
            if let Some(t) = mtime(&p) {
                assert!(
                    c_t >= t,
                    "STALE C .so: {} is older than {} — rebuild it with cmake.",
                    cs.display(),
                    p.display()
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Arena
// ---------------------------------------------------------------------------

/// Size of the shared byte arena.
///
/// Rationale for the size: `tflac_md5_addsample`'s copy loop reads
/// `buffer[64 + bytes]` with `bytes` up to 62, i.e. up to byte
/// `offsetof(buffer) + 126 == 142` past the record base, and `update_md5`
/// reads sample elements `0..=135` (544 bytes).  4 KiB comfortably contains
/// both plus slack, so every access the C makes stays inside the arena and is
/// therefore deterministic and comparable.
pub const ARENA: usize = 4096;

/// An 8-byte-aligned mutable byte arena (backed by `Vec<u64>` for alignment,
/// matching `_Alignof(tflac) == 8`).
pub struct Arena {
    words: Vec<u64>,
    len: usize,
}

impl Arena {
    pub fn from_template(tpl: &[u8]) -> Self {
        let mut words = vec![0u64; (tpl.len() + 7) / 8];
        unsafe {
            std::ptr::copy_nonoverlapping(tpl.as_ptr(), words.as_mut_ptr() as *mut u8, tpl.len());
        }
        Arena { words, len: tpl.len() }
    }
    pub fn ptr(&mut self) -> *mut u8 {
        self.words.as_mut_ptr() as *mut u8
    }
    pub fn at(&mut self, off: usize) -> *mut u8 {
        assert!(off <= self.len);
        unsafe { self.ptr().add(off) }
    }
    pub fn bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.words.as_ptr() as *const u8, self.len) }
    }
}

// ---------------------------------------------------------------------------
// Record builders (layout verified against the C compiler: tflac_md5 is
// {pos@0, total@8, buffer@16}, size 88 align 8; tflac is
// {md5_ctx@0, cur_blocksize@88, channels@92}, size 96 align 8)
// ---------------------------------------------------------------------------

pub const MD5_SIZE: usize = 88;
pub const TFLAC_SIZE: usize = 96;
pub const BUF_OFF: usize = 16;
pub const BUF_LEN: usize = 72;

/// Write a `tflac_md5` at `off`.  The 4 padding bytes at `off+4..off+8` are
/// deliberately left holding the arena's pattern: the C never touches them,
/// and they are identical on both sides, so any spurious write shows up.
pub fn put_md5(tpl: &mut [u8], off: usize, pos: u32, total: u64, buffer: &[u8; BUF_LEN]) {
    tpl[off..off + 4].copy_from_slice(&pos.to_le_bytes());
    tpl[off + 8..off + 16].copy_from_slice(&total.to_le_bytes());
    tpl[off + BUF_OFF..off + BUF_OFF + BUF_LEN].copy_from_slice(buffer);
}

pub fn put_tflac(
    tpl: &mut [u8],
    off: usize,
    pos: u32,
    total: u64,
    buffer: &[u8; BUF_LEN],
    cur_blocksize: u32,
    channels: u32,
) {
    put_md5(tpl, off, pos, total, buffer);
    tpl[off + 88..off + 92].copy_from_slice(&cur_blocksize.to_le_bytes());
    tpl[off + 92..off + 96].copy_from_slice(&channels.to_le_bytes());
}

pub fn get_pos(bytes: &[u8], off: usize) -> u32 {
    u32::from_le_bytes(bytes[off..off + 4].try_into().unwrap())
}
pub fn get_total(bytes: &[u8], off: usize) -> u64 {
    u64::from_le_bytes(bytes[off + 8..off + 16].try_into().unwrap())
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seeds ⇒ fully reproducible
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0xA5A5_5A5A_DEAD_BEEF)
    }
    pub fn u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn u32(&mut self) -> u32 {
        self.u64() as u32
    }
    pub fn i32(&mut self) -> i32 {
        self.u64() as i32
    }
    pub fn u8(&mut self) -> u8 {
        self.u64() as u8
    }
    /// Uniform-ish value in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        self.u64() % n
    }
    pub fn fill(&mut self, out: &mut [u8]) {
        for b in out.iter_mut() {
            *b = self.u8();
        }
    }
    pub fn arena(&mut self) -> Vec<u8> {
        let mut v = vec![0u8; ARENA];
        self.fill(&mut v);
        v
    }
    pub fn buf72(&mut self) -> [u8; BUF_LEN] {
        let mut b = [0u8; BUF_LEN];
        self.fill(&mut b);
        b
    }
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[self.below(xs.len() as u64) as usize]
    }
}

// ---------------------------------------------------------------------------
// Byte-for-byte comparison / diagnostics
// ---------------------------------------------------------------------------

fn hexdump(b: &[u8], from: usize, to: usize) -> String {
    let from = from.min(b.len());
    let to = to.min(b.len());
    let mut s = String::new();
    for (i, x) in b[from..to].iter().enumerate() {
        if i % 16 == 0 {
            s.push_str(&format!("\n  {:04x}:", from + i));
        }
        s.push_str(&format!(" {x:02x}"));
    }
    s
}

pub fn assert_arenas_eq(label: &str, ctx: &str, c: &Arena, r: &Arena) {
    let cb = c.bytes();
    let rb = r.bytes();
    assert_eq!(cb.len(), rb.len());
    if cb == rb {
        return;
    }
    let first = (0..cb.len()).find(|&i| cb[i] != rb[i]).unwrap();
    let ndiff = (0..cb.len()).filter(|&i| cb[i] != rb[i]).count();
    let lo = first.saturating_sub(16);
    let hi = (first + 64).min(cb.len());
    panic!(
        "{label} arena mismatch [{ctx}]\n  \
         first differing byte @ 0x{first:04x} ({first}): C=0x{:02x} RUST=0x{:02x}\n  \
         {ndiff} differing byte(s) total\n\
         C   :{}\nRUST:{}",
        cb[first],
        rb[first],
        hexdump(cb, lo, hi),
        hexdump(rb, lo, hi),
    );
}

// ---------------------------------------------------------------------------
// The three differential drivers
// ---------------------------------------------------------------------------

/// `tflac_pack_u64le(arena + off, n)` on both sides.
pub fn diff_pack(tpl: &[u8], off: usize, n: u64, ctx: &str) {
    assert!(off + 8 <= tpl.len(), "pack would leave the arena");
    let l = libs();
    let mut a = Arena::from_template(tpl);
    let mut b = Arena::from_template(tpl);
    unsafe {
        (l.c.pack)(a.at(off), n);
        (l.r.pack)(b.at(off), n);
    }
    assert_arenas_eq("tflac_pack_u64le", ctx, &a, &b);
}

/// A sequence of `tflac_pack_u64le` writes on the same arena.
pub fn diff_pack_seq(tpl: &[u8], writes: &[(usize, u64)], ctx: &str) {
    let l = libs();
    let mut a = Arena::from_template(tpl);
    let mut b = Arena::from_template(tpl);
    unsafe {
        for &(off, n) in writes {
            (l.c.pack)(a.at(off), n);
        }
        for &(off, n) in writes {
            (l.r.pack)(b.at(off), n);
        }
    }
    assert_arenas_eq("tflac_pack_u64le(seq)", ctx, &a, &b);
}

/// `tflac_md5_addsample(arena + off, bits, val)` on both sides.
pub fn diff_add(tpl: &[u8], off: usize, bits: u32, val: u64, ctx: &str) {
    let l = libs();
    let mut a = Arena::from_template(tpl);
    let mut b = Arena::from_template(tpl);
    unsafe {
        (l.c.add)(a.at(off), bits, val);
        (l.r.add)(b.at(off), bits, val);
    }
    assert_arenas_eq("tflac_md5_addsample", ctx, &a, &b);
}

/// A chain of `tflac_md5_addsample` calls carrying `pos`/`total`/`buffer`
/// forward (the library keeps all state in the caller's record, so this is how
/// a real consumer streams samples).
pub fn diff_add_stream(tpl: &[u8], off: usize, calls: &[(u32, u64)], ctx: &str) {
    let l = libs();
    let mut a = Arena::from_template(tpl);
    let mut b = Arena::from_template(tpl);
    unsafe {
        for (i, &(bits, val)) in calls.iter().enumerate() {
            (l.c.add)(a.at(off), bits, val);
            (l.r.add)(b.at(off), bits, val);
            // Compare after every step so a divergence is pinned to one call.
            assert_arenas_eq(
                "tflac_md5_addsample(stream)",
                &format!("{ctx} step={i} bits={bits} val={val:#x}"),
                &a,
                &b,
            );
        }
    }
}

/// `update_md5(record_arena + off, (const i32*)(samples_arena + soff))`.
/// Compares the `tflac_u32` return value *and* both arenas.
pub fn diff_upd(tpl: &[u8], off: usize, stpl: &[u8], soff: usize, ctx: &str) -> u32 {
    let l = libs();
    let mut a = Arena::from_template(tpl);
    let mut b = Arena::from_template(tpl);
    let mut sa = Arena::from_template(stpl);
    let mut sb = Arena::from_template(stpl);
    let (rc, rr) = unsafe {
        (
            (l.c.upd)(a.at(off), sa.at(soff) as *const i32),
            (l.r.upd)(b.at(off), sb.at(soff) as *const i32),
        )
    };
    assert_eq!(
        rc, rr,
        "update_md5 return value mismatch [{ctx}]: C={rc} (0x{rc:08x}) RUST={rr} (0x{rr:08x})"
    );
    assert_arenas_eq("update_md5(record)", ctx, &a, &b);
    assert_arenas_eq("update_md5(samples)", ctx, &sa, &sb);
    rc
}

/// A chain of `update_md5` calls on the same record, advancing the sample
/// window; every intermediate return value and state is compared.
pub fn diff_upd_stream(tpl: &[u8], off: usize, stpl: &[u8], soffs: &[usize], ctx: &str) {
    let l = libs();
    let mut a = Arena::from_template(tpl);
    let mut b = Arena::from_template(tpl);
    let mut sa = Arena::from_template(stpl);
    let mut sb = Arena::from_template(stpl);
    for (i, &soff) in soffs.iter().enumerate() {
        let (rc, rr) = unsafe {
            (
                (l.c.upd)(a.at(off), sa.at(soff) as *const i32),
                (l.r.upd)(b.at(off), sb.at(soff) as *const i32),
            )
        };
        assert_eq!(rc, rr, "update_md5 return mismatch [{ctx}] step={i}: C={rc} RUST={rr}");
        assert_arenas_eq("update_md5(stream,record)", &format!("{ctx} step={i}"), &a, &b);
        assert_arenas_eq("update_md5(stream,samples)", &format!("{ctx} step={i}"), &sa, &sb);
    }
}

// ---------------------------------------------------------------------------
// Misc helpers
// ---------------------------------------------------------------------------

/// Number of `tflac_s32` elements `update_md5` actually touches:
/// iterations start at elements 0, 32, 64, 96, 128 and each reads 8 ⇒ 0..=135.
pub const UPD_SAMPLE_SPAN_ELEMS: usize = 136;
pub const UPD_SAMPLE_SPAN_BYTES: usize = UPD_SAMPLE_SPAN_ELEMS * 4;

/// Highest arena byte `tflac_md5_addsample` can touch relative to the record
/// base: `buffer[64 + 62]` ⇒ `16 + 126 == 142`.
pub const ADD_MAX_TOUCH: usize = BUF_OFF + 64 + 62;

/// Build a samples template where element `i` is a distinct, easily-recognised
/// value, so an incorrect stride or index shows up immediately.
pub fn ramp_samples(seed: u32) -> Vec<u8> {
    let mut v = vec![0u8; ARENA];
    for i in 0..ARENA / 4 {
        let val = (i as u32).wrapping_mul(0x0101_0101).wrapping_add(seed);
        v[i * 4..i * 4 + 4].copy_from_slice(&val.to_le_bytes());
    }
    v
}

/// `bits` that makes `tflac_md5_addsample` land on a reduced `pos == r` with
/// the `pos >= 64` branch taken, starting from `pos == 0`.
pub fn bits_for_reduced_pos(r: u32) -> u32 {
    assert!(r < 64);
    8 * (64 + r)
}
