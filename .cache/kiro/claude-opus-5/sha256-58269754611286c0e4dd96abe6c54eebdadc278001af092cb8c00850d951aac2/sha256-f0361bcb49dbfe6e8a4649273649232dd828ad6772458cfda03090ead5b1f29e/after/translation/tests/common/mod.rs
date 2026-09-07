//! Shared differential-test scaffolding.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading`; every call
//! goes through the dynamic symbols, so the `#[no_mangle]` export wrappers are
//! part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void, CStr};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Types mirroring the public ABI
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct CpPixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

type FnInflate = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int;
type FnConvertPix = unsafe extern "C" fn(c_int, c_int, c_int, *mut u8, *mut CpPixel);

/// One loaded implementation (C or Rust) reached only through its dyn symbols.
pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    pub inflate: FnInflate,
    pub convert_pix: FnConvertPix,
    pub error_reason: *mut *const c_char,
    pub fixed_table: *mut u8,
    pub permutation_order: *mut u8,
    pub len_extra_bits: *mut u8,
    pub len_base: *mut u32,
    pub dist_extra_bits: *mut u8,
    pub dist_base: *mut u32,
}

impl Impl {
    unsafe fn load(name: &'static str, path: &PathBuf) -> Impl {
        let lib = Library::new(path)
            .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", path.display(), name));

        let inflate: Symbol<FnInflate> = lib.get(b"cp_inflate\0").expect("cp_inflate missing");
        let convert_pix: Symbol<FnConvertPix> =
            lib.get(b"convert_pix\0").expect("convert_pix missing");
        let error_reason: Symbol<*mut *const c_char> =
            lib.get(b"cp_error_reason\0").expect("cp_error_reason missing");
        let fixed_table: Symbol<*mut u8> =
            lib.get(b"cp_fixed_table\0").expect("cp_fixed_table missing");
        let permutation_order: Symbol<*mut u8> = lib
            .get(b"cp_permutation_order\0")
            .expect("cp_permutation_order missing");
        let len_extra_bits: Symbol<*mut u8> = lib
            .get(b"cp_len_extra_bits\0")
            .expect("cp_len_extra_bits missing");
        let len_base: Symbol<*mut u32> = lib.get(b"cp_len_base\0").expect("cp_len_base missing");
        let dist_extra_bits: Symbol<*mut u8> = lib
            .get(b"cp_dist_extra_bits\0")
            .expect("cp_dist_extra_bits missing");
        let dist_base: Symbol<*mut u32> =
            lib.get(b"cp_dist_base\0").expect("cp_dist_base missing");

        let out = Impl {
            name,
            inflate: *inflate,
            convert_pix: *convert_pix,
            error_reason: *error_reason,
            fixed_table: *fixed_table,
            permutation_order: *permutation_order,
            len_extra_bits: *len_extra_bits,
            len_base: *len_base,
            dist_extra_bits: *dist_extra_bits,
            dist_base: *dist_base,
            _lib: lib,
        };
        out
    }

    pub fn error_reason_str(&self) -> Option<String> {
        unsafe {
            let p = std::ptr::read(self.error_reason);
            if p.is_null() {
                None
            } else {
                Some(CStr::from_ptr(p).to_string_lossy().into_owned())
            }
        }
    }

    pub fn clear_error(&self) {
        unsafe { std::ptr::write(self.error_reason, std::ptr::null()) }
    }

    /// Whether the `.so` exports `name` (NUL-terminated) as a dynamic symbol.
    pub fn has_symbol(&self, name: &[u8]) -> bool {
        unsafe { self._lib.get::<*mut c_void>(name).is_ok() }
    }

    /// Snapshot of every exported table, so mutation tests can restore state.
    pub fn snapshot_tables(&self) -> TableSnapshot {
        unsafe {
            TableSnapshot {
                fixed_table: std::slice::from_raw_parts(self.fixed_table, 320).to_vec(),
                permutation_order: std::slice::from_raw_parts(self.permutation_order, 19).to_vec(),
                len_extra_bits: std::slice::from_raw_parts(self.len_extra_bits, 31).to_vec(),
                len_base: std::slice::from_raw_parts(self.len_base, 31).to_vec(),
                dist_extra_bits: std::slice::from_raw_parts(self.dist_extra_bits, 32).to_vec(),
                dist_base: std::slice::from_raw_parts(self.dist_base, 32).to_vec(),
            }
        }
    }

    pub fn restore_tables(&self, s: &TableSnapshot) {
        unsafe {
            std::ptr::copy_nonoverlapping(s.fixed_table.as_ptr(), self.fixed_table, 320);
            std::ptr::copy_nonoverlapping(
                s.permutation_order.as_ptr(),
                self.permutation_order,
                19,
            );
            std::ptr::copy_nonoverlapping(s.len_extra_bits.as_ptr(), self.len_extra_bits, 31);
            std::ptr::copy_nonoverlapping(s.len_base.as_ptr(), self.len_base, 31);
            std::ptr::copy_nonoverlapping(s.dist_extra_bits.as_ptr(), self.dist_extra_bits, 32);
            std::ptr::copy_nonoverlapping(s.dist_base.as_ptr(), self.dist_base, 32);
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TableSnapshot {
    pub fixed_table: Vec<u8>,
    pub permutation_order: Vec<u8>,
    pub len_extra_bits: Vec<u8>,
    pub len_base: Vec<u32>,
    pub dist_extra_bits: Vec<u8>,
    pub dist_base: Vec<u32>,
}

pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let dir = repo_root().join("c_src").join("build");
    let mut cands: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("c_src/build not built ({}): {e}", dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    cands.sort();
    cands
        .pop()
        .unwrap_or_else(|| panic!("no lib*.so in {}", dir.display()))
}

fn find_rust_so() -> PathBuf {
    let base = repo_root().join("translation").join("target");
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libconvert_pix_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("libconvert_pix_lib.so not found; run `cargo build --release` in translation/");
}

pub fn load_pair() -> Pair {
    unsafe {
        Pair {
            c: Impl::load("C", &find_c_so()),
            rs: Impl::load("Rust", &find_rust_so()),
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    /// Uniform in `lo..=hi`.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        assert!(hi >= lo);
        lo + (self.next_u64() % ((hi - lo + 1) as u64)) as i64
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.byte()).collect()
    }
    pub fn bool_pct(&mut self, pct: u32) -> bool {
        (self.next_u32() % 100) < pct
    }
}

/// A heap buffer whose payload starts at a chosen `addr % 4`, surrounded by
/// zeroed slack on both sides.
///
/// The slack matters: `cp_stored` trusts `LEN` (up to 65535) and `memcpy`s that
/// many bytes from `cp_ptr(s)`, which can land before the payload start or far
/// past its end. Zero padding makes those reads land in deterministic memory so
/// the C and Rust runs (separate processes with different heap contents) stay
/// comparable instead of hashing whatever happened to be adjacent.
pub struct AlignedBuf {
    store: Vec<u8>,
    off: usize,
    len: usize,
}

pub const HEAD_PAD: usize = 1024;

impl AlignedBuf {
    pub fn new(data: &[u8], want_mod4: usize, tail_pad: usize) -> AlignedBuf {
        AlignedBuf::new_padded(data, want_mod4, HEAD_PAD, tail_pad + 64)
    }

    pub fn new_padded(
        data: &[u8],
        want_mod4: usize,
        head_pad: usize,
        tail_pad: usize,
    ) -> AlignedBuf {
        let mut store = vec![0u8; head_pad + data.len() + tail_pad + 8];
        let base = store.as_ptr() as usize;
        let mut off = head_pad;
        while (base + off) % 4 != want_mod4 % 4 {
            off += 1;
        }
        store[off..off + data.len()].copy_from_slice(data);
        AlignedBuf {
            store,
            off,
            len: data.len(),
        }
    }
    pub fn ptr(&mut self) -> *mut c_void {
        unsafe { self.store.as_mut_ptr().add(self.off) as *mut c_void }
    }
    pub fn len(&self) -> c_int {
        self.len as c_int
    }
    pub fn actual_mod4(&self) -> usize {
        (unsafe { self.store.as_ptr().add(self.off) } as usize) % 4
    }
}

// ---------------------------------------------------------------------------
// Differential runners
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
pub struct InflateOutcome {
    pub ret: c_int,
    pub out: Vec<u8>,
    pub reason: Option<String>,
}

/// Run `cp_inflate` on one implementation. `in_align` selects `first_bytes`.
pub fn run_inflate(
    imp: &Impl,
    input: &[u8],
    in_align: usize,
    out_bytes: c_int,
    out_pad: usize,
) -> InflateOutcome {
    run_inflate_ex(imp, input, in_align, out_bytes, out_pad, 8)
}

/// `in_tail` = extra zero bytes readable past the logical end of the input.
pub fn run_inflate_ex(
    imp: &Impl,
    input: &[u8],
    in_align: usize,
    out_bytes: c_int,
    out_pad: usize,
    in_tail: usize,
) -> InflateOutcome {
    imp.clear_error();
    let mut inbuf = AlignedBuf::new(input, in_align, in_tail);
    let cap = out_bytes.max(0) as usize + out_pad;
    let mut out = vec![0u8; cap + 1];
    let ret = unsafe {
        (imp.inflate)(
            inbuf.ptr(),
            inbuf.len(),
            out.as_mut_ptr() as *mut c_void,
            out_bytes,
        )
    };
    out.truncate(cap);
    InflateOutcome {
        ret,
        out,
        reason: imp.error_reason_str(),
    }
}

/// Differential `cp_inflate`: identical inputs to both `.so`s, byte-exact compare.
pub fn diff_inflate(
    p: &Pair,
    label: &str,
    input: &[u8],
    in_align: usize,
    out_bytes: c_int,
) -> InflateOutcome {
    diff_inflate_pad(p, label, input, in_align, out_bytes, 0)
}

pub fn diff_inflate_pad(
    p: &Pair,
    label: &str,
    input: &[u8],
    in_align: usize,
    out_bytes: c_int,
    out_pad: usize,
) -> InflateOutcome {
    let a = run_inflate(&p.c, input, in_align, out_bytes, out_pad);
    let b = run_inflate(&p.rs, input, in_align, out_bytes, out_pad);
    assert_eq!(
        a.ret, b.ret,
        "[{label}] cp_inflate return mismatch: C={} Rust={} (input {} bytes, align {}, out_bytes {})\n  C reason: {:?}\n  Rust reason: {:?}",
        a.ret, b.ret, input.len(), in_align, out_bytes, a.reason, b.reason
    );
    assert_eq!(
        a.reason, b.reason,
        "[{label}] cp_error_reason mismatch (ret={})",
        a.ret
    );
    if a.out != b.out {
        let at = a
            .out
            .iter()
            .zip(b.out.iter())
            .position(|(x, y)| x != y)
            .unwrap_or(a.out.len().min(b.out.len()));
        panic!(
            "[{label}] output mismatch at byte {at}: C=0x{:02x?} Rust=0x{:02x?} (len {} vs {})",
            a.out.get(at),
            b.out.get(at),
            a.out.len(),
            b.out.len()
        );
    }
    a
}

/// Differential `convert_pix`.
/// Write a `deflate::Tables` value into the library's exported globals.
pub fn write_tables(imp: &Impl, t: &deflate::Tables) {
    unsafe {
        std::ptr::copy_nonoverlapping(t.len_base.as_ptr(), imp.len_base, 31);
        std::ptr::copy_nonoverlapping(t.len_extra.as_ptr(), imp.len_extra_bits, 31);
        std::ptr::copy_nonoverlapping(t.dist_base.as_ptr(), imp.dist_base, 32);
        std::ptr::copy_nonoverlapping(t.dist_extra.as_ptr(), imp.dist_extra_bits, 32);
    }
}

/// Write 288 literal lengths + 32 distance lengths into `cp_fixed_table`.
pub fn write_fixed_table(imp: &Impl, lit288: &[u8], dist32: &[u8]) {
    assert_eq!(lit288.len(), 288);
    assert_eq!(dist32.len(), 32);
    unsafe {
        std::ptr::copy_nonoverlapping(lit288.as_ptr(), imp.fixed_table, 288);
        std::ptr::copy_nonoverlapping(dist32.as_ptr(), imp.fixed_table.add(288), 32);
    }
}

pub fn write_perm(imp: &Impl, perm: &[usize; 19]) {
    let bytes: Vec<u8> = perm.iter().map(|&x| x as u8).collect();
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), imp.permutation_order, 19) }
}

/// Apply `setup` to both implementations' globals, run `cp_inflate`, compare,
/// then restore the pristine table state on both sides.
pub fn diff_inflate_globals<F: Fn(&Impl)>(
    p: &Pair,
    label: &str,
    input: &[u8],
    in_align: usize,
    out_bytes: c_int,
    setup: F,
) -> InflateOutcome {
    let snap_c = p.c.snapshot_tables();
    let snap_rs = p.rs.snapshot_tables();
    setup(&p.c);
    setup(&p.rs);
    let a = run_inflate(&p.c, input, in_align, out_bytes, 0);
    let b = run_inflate(&p.rs, input, in_align, out_bytes, 0);
    // Both sides must also observe identical global state after the call.
    let post_c = p.c.snapshot_tables();
    let post_rs = p.rs.snapshot_tables();
    p.c.restore_tables(&snap_c);
    p.rs.restore_tables(&snap_rs);
    assert_eq!(post_c, post_rs, "[{label}] global table state diverged");
    assert_eq!(
        a.ret, b.ret,
        "[{label}] cp_inflate return mismatch: C={} Rust={}\n  C reason: {:?}\n  Rust reason: {:?}",
        a.ret, b.ret, a.reason, b.reason
    );
    assert_eq!(a.reason, b.reason, "[{label}] cp_error_reason mismatch");
    assert!(a.out == b.out, "[{label}] output mismatch");
    a
}

pub fn diff_convert_pix(
    p: &Pair,
    label: &str,
    bpp: c_int,
    w: c_int,
    h: c_int,
    src: &[u8],
    dst_pixels: usize,
    src_offset: usize,
) {
    let run = |imp: &Impl| -> Vec<CpPixel> {
        let mut s = src.to_vec();
        let mut d = vec![
            CpPixel {
                r: 0xA5,
                g: 0x5A,
                b: 0x3C,
                a: 0xC3
            };
            dst_pixels + 1
        ];
        unsafe {
            (imp.convert_pix)(
                bpp,
                w,
                h,
                s.as_mut_ptr().add(src_offset),
                d.as_mut_ptr(),
            );
        }
        d
    };
    let a = run(&p.c);
    let b = run(&p.rs);
    assert_eq!(
        a, b,
        "[{label}] convert_pix mismatch (bpp={bpp} w={w} h={h})"
    );
}

pub mod deflate;
