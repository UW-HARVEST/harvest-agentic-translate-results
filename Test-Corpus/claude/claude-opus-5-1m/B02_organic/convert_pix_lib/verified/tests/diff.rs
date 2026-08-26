//! Differential test harness: loads BOTH the C `.so` and the Rust `.so` through
//! `libloading` and compares every observable byte across the FFI boundary.
//!
//! `harness = false` (see Cargo.toml) so this file owns `main()`.  That is
//! required because the C library is compiled with assertions ENABLED, so a
//! large part of its error surface is `abort()`.  To compare aborts we execute
//! every batch of cases in a *child process* (this same executable re-invoked
//! with `DIFF_CHILD_LIB` set), streaming one result record per case straight to
//! a file with unbuffered `write(2)`.  If the child dies we still have every
//! record it produced before dying, so "both aborted after exactly the same k
//! cases with byte-identical output" is checkable.
//!
//! Nothing here ever calls a Rust function of the crate directly: the Rust code
//! is only reached through `dlopen`/`dlsym` on `libconvert_pix_lib.so`, exactly
//! like an external C consumer, which also exercises the `#[no_mangle]` wrappers.

#![allow(dead_code)]

use std::ffi::c_int;
use std::ffi::c_void;
use std::fs;
use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;

// ===========================================================================
// deterministic RNG (xorshift64*) - fixed seeds everywhere, reproducible
// ===========================================================================

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
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
    /// uniform in `0..n` (n > 0)
    fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// inclusive range
    fn range(&mut self, lo: i64, hi: i64) -> i64 {
        lo + (self.next_u64() % ((hi - lo + 1) as u64)) as i64
    }
    fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
    fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

/// deterministic filler for the parts of the arenas the library is not
/// supposed to touch (and for the out-of-bounds regions that the C code's
/// unchecked `memcpy` in `cp_stored` *does* touch).
fn pat(i: usize) -> u8 {
    // periodic with period 2^16 so that a 64 KiB block can be precomputed once
    // and memcpy'd; identical in both children either way.
    let x = (i as u16 as u32).wrapping_mul(0x9E37_79B1) ^ 0x5BF0_3635;
    ((x >> 13) ^ x) as u8
}

const PAT_BLOCK: usize = 1 << 16;

fn pat_block() -> &'static [u8] {
    use std::sync::OnceLock;
    static B: OnceLock<Vec<u8>> = OnceLock::new();
    B.get_or_init(|| (0..PAT_BLOCK).map(pat).collect())
}

// ===========================================================================
// spec encoding (parent -> child)
// ===========================================================================

const OP_CONVERT: u8 = 1;
const OP_INFLATE: u8 = 2;
const OP_TABLES: u8 = 3;

/// table ids for mutation records
const T_FIXED: u8 = 0; // cp_fixed_table        u8 [320]
const T_PERM: u8 = 1; // cp_permutation_order  u8 [19]
const T_LEXTRA: u8 = 2; // cp_len_extra_bits     u8 [31]
const T_LBASE: u8 = 3; // cp_len_base           u32[31]
const T_DEXTRA: u8 = 4; // cp_dist_extra_bits    u8 [32]
const T_DBASE: u8 = 5; // cp_dist_base          u32[32]

struct W(Vec<u8>);

impl W {
    fn new() -> W {
        W(Vec::new())
    }
    fn u8(&mut self, v: u8) -> &mut W {
        self.0.push(v);
        self
    }
    fn u16(&mut self, v: u16) -> &mut W {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    fn u32(&mut self, v: u32) -> &mut W {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    fn i32(&mut self, v: i32) -> &mut W {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    fn blob(&mut self, v: &[u8]) -> &mut W {
        self.u32(v.len() as u32);
        self.0.extend_from_slice(v);
        self
    }
}

struct R<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> R<'a> {
    fn new(b: &'a [u8]) -> R<'a> {
        R { b, i: 0 }
    }
    fn u8(&mut self) -> u8 {
        let v = self.b[self.i];
        self.i += 1;
        v
    }
    fn u16(&mut self) -> u16 {
        let v = u16::from_le_bytes(self.b[self.i..self.i + 2].try_into().unwrap());
        self.i += 2;
        v
    }
    fn u32(&mut self) -> u32 {
        let v = u32::from_le_bytes(self.b[self.i..self.i + 4].try_into().unwrap());
        self.i += 4;
        v
    }
    fn i32(&mut self) -> i32 {
        self.u32() as i32
    }
    fn blob(&mut self) -> &'a [u8] {
        let n = self.u32() as usize;
        let v = &self.b[self.i..self.i + n];
        self.i += n;
        v
    }
}

type Case = Vec<u8>;

fn case_tables() -> Case {
    let mut w = W::new();
    w.u8(OP_TABLES);
    w.0
}

/// `convert_pix(bpp, w, h, src, dst)`
///
/// `src` is copied into a page-aligned arena at `src_off` (so the caller can
/// also drive misalignment); `dst` is a page-aligned arena of `dst_len` bytes
/// pre-filled with `pat()` and returned in full afterwards.
fn case_convert(
    bpp: i32,
    w: i32,
    h: i32,
    src: &[u8],
    dst_len: u32,
    src_null: bool,
    dst_null: bool,
) -> Case {
    let mut o = W::new();
    o.u8(OP_CONVERT);
    o.i32(bpp);
    o.i32(w);
    o.i32(h);
    o.blob(src);
    o.u32(dst_len);
    o.u8(src_null as u8);
    o.u8(dst_null as u8);
    o.0
}

#[derive(Clone)]
struct InflateCase {
    /// `(in as usize) & 3` is forced to this value
    align: u8,
    /// value passed as `in_bytes` (`i32::MIN` sentinel means "use data.len()")
    in_bytes: Option<i32>,
    data: Vec<u8>,
    out_bytes: i32,
    out_arena: u32,
    in_null: bool,
    out_null: bool,
    muts: Vec<(u8, u16, u32)>,
}

impl InflateCase {
    fn new(data: Vec<u8>) -> InflateCase {
        let n = data.len() as i32;
        InflateCase {
            align: 0,
            in_bytes: Some(n),
            data,
            out_bytes: 4096,
            out_arena: 8192,
            in_null: false,
            out_null: false,
            muts: Vec::new(),
        }
    }
    fn align(mut self, a: u8) -> Self {
        self.align = a;
        self
    }
    fn in_bytes(mut self, n: i32) -> Self {
        self.in_bytes = Some(n);
        self
    }
    fn out(mut self, out_bytes: i32, arena: u32) -> Self {
        self.out_bytes = out_bytes;
        self.out_arena = arena;
        self
    }
    fn nulls(mut self, i: bool, o: bool) -> Self {
        self.in_null = i;
        self.out_null = o;
        self
    }
    fn mutate(mut self, t: u8, idx: u16, val: u32) -> Self {
        self.muts.push((t, idx, val));
        self
    }
    fn encode(&self) -> Case {
        let mut o = W::new();
        o.u8(OP_INFLATE);
        o.u8(self.align);
        o.i32(self.in_bytes.unwrap_or(self.data.len() as i32));
        o.blob(&self.data);
        o.i32(self.out_bytes);
        o.u32(self.out_arena);
        o.u8(self.in_null as u8);
        o.u8(self.out_null as u8);
        o.u32(self.muts.len() as u32);
        for &(t, i, v) in &self.muts {
            o.u8(t);
            o.u16(i);
            o.u32(v);
        }
        o.0
    }
}

fn encode_spec(cases: &[Case]) -> Vec<u8> {
    let mut o = W::new();
    o.u32(cases.len() as u32);
    for c in cases {
        o.blob(c);
    }
    o.0
}

// ===========================================================================
// child process: load ONE .so, run every case, stream results to a file
// ===========================================================================

#[repr(C)]
struct RLimit {
    cur: u64,
    max: u64,
}

const RLIMIT_CORE: i32 = 4;

extern "C" {
    fn setrlimit(resource: i32, rlim: *const RLimit) -> i32;
    fn fork() -> i32;
    fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
    fn alarm(seconds: u32) -> u32;
    fn _exit(code: i32) -> !;
}

/// normalise a `wait(2)` status: `>= 0` is an exit code, `< 0` is `-signal`
fn norm_status(raw: i32) -> i32 {
    if raw & 0x7f == 0 {
        (raw >> 8) & 0xff
    } else {
        -(raw & 0x7f)
    }
}

fn status_str(v: i32) -> String {
    if v >= 0 {
        format!("exit {v}")
    } else {
        format!("signal {}", -v)
    }
}

const IN_PRE: usize = 16384; // bytes of deterministic filler before `in`
const IN_POST: usize = 70000; // deterministic filler after the data
                              // (cp_stored's unchecked memcpy reads up to 65535 bytes)

struct Arena {
    ptr: *mut u8,
    len: usize,
}

impl Arena {
    fn new(len: usize) -> Arena {
        let len = if len == 0 { 1 } else { len };
        let layout = std::alloc::Layout::from_size_align(len, 4096).unwrap();
        let ptr = unsafe { std::alloc::alloc(layout) };
        assert!(!ptr.is_null());
        let blk = pat_block();
        let mut off = 0usize;
        while off < len {
            let n = (len - off).min(PAT_BLOCK - (off % PAT_BLOCK));
            unsafe {
                std::ptr::copy_nonoverlapping(blk.as_ptr().add(off % PAT_BLOCK), ptr.add(off), n)
            };
            off += n;
        }
        Arena { ptr, len }
    }
    fn bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
}

struct TableDesc {
    name: &'static [u8],
    elem: usize,
    count: usize,
}

const TABLES: [TableDesc; 6] = [
    TableDesc { name: b"cp_fixed_table\0", elem: 1, count: 288 + 32 },
    TableDesc { name: b"cp_permutation_order\0", elem: 1, count: 19 },
    TableDesc { name: b"cp_len_extra_bits\0", elem: 1, count: 29 + 2 },
    TableDesc { name: b"cp_len_base\0", elem: 4, count: 29 + 2 },
    TableDesc { name: b"cp_dist_extra_bits\0", elem: 1, count: 30 + 2 },
    TableDesc { name: b"cp_dist_base\0", elem: 4, count: 30 + 2 },
];

type ConvertFn = unsafe extern "C" fn(c_int, c_int, c_int, *mut u8, *mut u8);
type InflateFn = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int;

fn sym_addr(lib: &libloading::Library, name: &[u8]) -> *mut u8 {
    unsafe {
        let s: libloading::Symbol<*mut u8> = lib
            .get(name)
            .unwrap_or_else(|e| panic!("missing symbol {:?}: {e}", String::from_utf8_lossy(name)));
        s.into_raw().into_raw() as *mut u8
    }
}

fn child_main(lib_path: &str, spec_path: &str, out_path: &str) {
    let spec = fs::read(spec_path).expect("read spec");
    let lib = unsafe { libloading::Library::new(lib_path) }.expect("dlopen");

    let convert: libloading::Symbol<ConvertFn> =
        unsafe { lib.get(b"convert_pix\0") }.expect("convert_pix");
    let inflate: libloading::Symbol<InflateFn> =
        unsafe { lib.get(b"cp_inflate\0") }.expect("cp_inflate");
    let table_ptrs: Vec<*mut u8> = TABLES.iter().map(|t| sym_addr(&lib, t.name)).collect();
    let reason_ptr = sym_addr(&lib, b"cp_error_reason\0") as *mut *const u8;

    // pristine copies so every case starts from the library's initial tables
    let pristine: Vec<Vec<u8>> = TABLES
        .iter()
        .zip(&table_ptrs)
        .map(|(t, &p)| unsafe { std::slice::from_raw_parts(p, t.elem * t.count) }.to_vec())
        .collect();

    let case_timeout: u32 = std::env::var("DIFF_CASE_TIMEOUT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2);

    // every aborting case would otherwise be handed to systemd-coredump, which
    // costs ~0.2 s per abort and dwarfs the actual work
    unsafe { setrlimit(RLIMIT_CORE, &RLimit { cur: 0, max: 0 }) };

    let mut out = fs::File::create(out_path).expect("create out");
    let tmp_path = format!("{out_path}.tmp");

    let mut r = R::new(&spec);
    let ncases = r.u32();
    for idx in 0..ncases {
        let rec = r.blob().to_vec();
        // reset the mutable globals before every case
        for (t, (&p, orig)) in TABLES.iter().zip(table_ptrs.iter().zip(&pristine)) {
            unsafe { std::ptr::copy_nonoverlapping(orig.as_ptr(), p, t.elem * t.count) };
        }
        unsafe { *reason_ptr = std::ptr::null() };
        let _ = fs::remove_file(&tmp_path);

        // Run the case in a forked grandchild so that an abort / segfault / hang
        // is observable per case instead of killing the whole batch.  `alarm()`
        // bounds the C library's genuinely non-terminating inputs.
        let pid = unsafe { fork() };
        if pid == 0 {
            unsafe { alarm(case_timeout) };
            let payload = run_case(&rec, &convert, &inflate, &table_ptrs, reason_ptr);
            let mut f = fs::File::create(&tmp_path).expect("create tmp");
            f.write_all(&payload).unwrap();
            f.flush().unwrap();
            drop(f);
            unsafe { _exit(0) };
        }
        assert!(pid > 0, "fork failed");
        let mut raw = 0i32;
        unsafe { waitpid(pid, &mut raw, 0) };
        let payload = fs::read(&tmp_path).unwrap_or_default();
        let _ = fs::remove_file(&tmp_path);

        let mut hdr = W::new();
        hdr.u32(idx);
        hdr.i32(norm_status(raw));
        hdr.blob(&payload);
        out.write_all(&hdr.0).unwrap();
        out.flush().unwrap();
    }
    std::process::exit(0);
}

fn read_reason(reason_ptr: *mut *const u8) -> Option<Vec<u8>> {
    unsafe {
        let p = *reason_ptr;
        if p.is_null() {
            return None;
        }
        let mut v = Vec::new();
        let mut i = 0usize;
        loop {
            let b = *p.add(i);
            if b == 0 {
                break;
            }
            v.push(b);
            i += 1;
            assert!(i < 4096, "unterminated cp_error_reason");
        }
        Some(v)
    }
}

fn run_case(
    rec: &[u8],
    convert: &ConvertFn,
    inflate: &InflateFn,
    table_ptrs: &[*mut u8],
    reason_ptr: *mut *const u8,
) -> Vec<u8> {
    let mut r = R::new(rec);
    let op = r.u8();
    let mut o = W::new();
    match op {
        OP_TABLES => {
            for (t, &p) in TABLES.iter().zip(table_ptrs) {
                let b = unsafe { std::slice::from_raw_parts(p, t.elem * t.count) };
                o.blob(b);
            }
            match read_reason(reason_ptr) {
                None => o.u8(0),
                Some(s) => {
                    o.u8(1);
                    o.blob(&s)
                }
            };
        }
        OP_CONVERT => {
            let bpp = r.i32();
            let w = r.i32();
            let h = r.i32();
            let src = r.blob();
            let dst_len = r.u32() as usize;
            let src_null = r.u8() != 0;
            let dst_null = r.u8() != 0;

            let src_arena = Arena::new(src.len().max(1));
            unsafe { std::ptr::copy_nonoverlapping(src.as_ptr(), src_arena.ptr, src.len()) };
            let dst_arena = Arena::new(dst_len.max(1));

            let sp = if src_null { std::ptr::null_mut() } else { src_arena.ptr };
            let dp = if dst_null { std::ptr::null_mut() } else { dst_arena.ptr };
            unsafe { convert(bpp, w, h, sp, dp) };

            o.blob(&dst_arena.bytes()[..dst_len]);
            o.blob(src_arena.bytes());
        }
        OP_INFLATE => {
            let align = r.u8() as usize;
            let in_bytes = r.i32();
            let data = r.blob();
            let out_bytes = r.i32();
            let out_arena_len = r.u32() as usize;
            let in_null = r.u8() != 0;
            let out_null = r.u8() != 0;
            let nmuts = r.u32();
            for _ in 0..nmuts {
                let t = r.u8() as usize;
                let idx = r.u16() as usize;
                let val = r.u32();
                let d = &TABLES[t];
                assert!(idx < d.count);
                unsafe {
                    let p = table_ptrs[t].add(idx * d.elem);
                    if d.elem == 1 {
                        *p = val as u8;
                    } else {
                        std::ptr::copy_nonoverlapping(val.to_le_bytes().as_ptr(), p, 4);
                    }
                }
            }

            let in_arena = Arena::new(IN_PRE + data.len() + IN_POST);
            let inp = unsafe { in_arena.ptr.add(IN_PRE + align) };
            unsafe { std::ptr::copy_nonoverlapping(data.as_ptr(), inp, data.len()) };
            assert_eq!(inp as usize & 3, align);

            assert!(
                out_bytes <= 0 || (out_bytes as usize) <= out_arena_len,
                "out_arena too small for out_bytes"
            );
            let out_arena = Arena::new(out_arena_len);

            let ip = if in_null { std::ptr::null_mut() } else { inp as *mut c_void };
            let op2 = if out_null { std::ptr::null_mut() } else { out_arena.ptr as *mut c_void };

            let ret = unsafe { inflate(ip, in_bytes, op2, out_bytes) };

            o.i32(ret);
            let ob = &out_arena.bytes()[..out_arena_len];
            if out_arena_len <= 16384 {
                o.blob(ob);
            } else {
                o.blob(&ob[..4096]);
                let mut h: u64 = 1469598103934665603;
                for &b in ob {
                    h ^= b as u64;
                    h = h.wrapping_mul(1099511628211);
                }
                o.u32((h & 0xFFFF_FFFF) as u32);
                o.u32((h >> 32) as u32);
            }
            match read_reason(reason_ptr) {
                None => o.u8(0),
                Some(s) => {
                    o.u8(1);
                    o.blob(&s)
                }
            };
            // the input arena must be untouched by an inflate call
            let mut h: u64 = 1469598103934665603;
            for &b in in_arena.bytes() {
                h ^= b as u64;
                h = h.wrapping_mul(1099511628211);
            }
            o.u32((h & 0xFFFF_FFFF) as u32);
        }
        _ => panic!("unknown op {op}"),
    }
    o.0
}

// ===========================================================================
// deflate encoder used to produce VALID streams for the happy-path rows
// (mirrors exactly what cp_build / cp_decode expect: canonical codes, bits
// packed LSB-first, Huffman codes emitted MSB-first)
// ===========================================================================

const FIXED_LIT_LENS: [u8; 288] = {
    let mut t = [0u8; 288];
    let mut i = 0;
    while i < 144 {
        t[i] = 8;
        i += 1;
    }
    while i < 256 {
        t[i] = 9;
        i += 1;
    }
    while i < 280 {
        t[i] = 7;
        i += 1;
    }
    while i < 288 {
        t[i] = 8;
        i += 1;
    }
    t
};

const LEN_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const LEN_BASE: [u32; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const DIST_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];
const DIST_BASE: [u32; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const PERM: [usize; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];

struct BitW {
    bytes: Vec<u8>,
    nbits: usize,
}

impl BitW {
    fn new() -> BitW {
        BitW { bytes: Vec::new(), nbits: 0 }
    }
    fn bit(&mut self, b: u32) {
        if self.nbits % 8 == 0 {
            self.bytes.push(0);
        }
        if b & 1 != 0 {
            let i = self.nbits / 8;
            self.bytes[i] |= 1 << (self.nbits % 8);
        }
        self.nbits += 1;
    }
    /// n raw bits of `v`, least-significant bit first (deflate "bits" order)
    fn bits(&mut self, v: u32, n: u32) {
        for k in 0..n {
            self.bit(v >> k);
        }
    }
    /// a Huffman code: most-significant bit first
    fn code(&mut self, c: u32, n: u32) {
        for k in (0..n).rev() {
            self.bit(c >> k);
        }
    }
    fn align_byte(&mut self) {
        while self.nbits % 8 != 0 {
            self.bit(0);
        }
    }
    fn byte(&mut self, b: u8) {
        assert_eq!(self.nbits % 8, 0);
        self.bytes.push(b);
        self.nbits += 8;
    }
}

/// canonical codes exactly as `cp_build` derives them
fn canonical(lens: &[u8]) -> Vec<u32> {
    let mut counts = [0u32; 16];
    for &l in lens {
        assert!(l < 16);
        counts[l as usize] += 1;
    }
    let mut codes = [0u32; 16];
    counts[0] = 0;
    for n in 1..=15usize {
        codes[n] = (codes[n - 1] + counts[n - 1]) << 1;
    }
    let mut out = vec![0u32; lens.len()];
    for i in 0..lens.len() {
        let l = lens[i] as usize;
        if l != 0 {
            out[i] = codes[l];
            codes[l] += 1;
        }
    }
    out
}

/// depths of a *complete* (Kraft sum == 1) prefix code with `n` leaves,
/// random shape, never deeper than `max_depth`
fn complete_depths(n: usize, max_depth: u8, rng: &mut Rng) -> Vec<u8> {
    assert!(n >= 2);
    let mut leaves: Vec<u8> = vec![1, 1];
    while leaves.len() < n {
        // pick a leaf that can still be split
        let cands: Vec<usize> =
            (0..leaves.len()).filter(|&i| leaves[i] < max_depth).collect();
        assert!(!cands.is_empty(), "cannot build code of {n} leaves within depth {max_depth}");
        let pick = cands[rng.below(cands.len() as u32) as usize];
        let d = leaves[pick] + 1;
        leaves[pick] = d;
        leaves.push(d);
    }
    leaves
}

/// A deflate token stream
#[derive(Clone, Debug)]
enum Tok {
    Lit(u8),
    /// (length symbol 257..=287, extra-bit value, distance symbol 0..=31, extra value)
    Match { lsym: u32, lextra: u32, dsym: u32, dextra: u32 },
    /// raw literal/length symbol, no extra handling (for 286/287 probing)
    RawSym(u32),
}

fn lensym_for(len: u32) -> (u32, u32) {
    for s in (0..29).rev() {
        let base = LEN_BASE[s];
        let ex = LEN_EXTRA[s] as u32;
        if len >= base && len < base + (1 << ex) {
            return (257 + s as u32, len - base);
        }
    }
    panic!("bad length {len}");
}

fn distsym_for(d: u32) -> (u32, u32) {
    for s in (0..30).rev() {
        let base = DIST_BASE[s];
        let ex = DIST_EXTRA[s] as u32;
        if d >= base && d < base + (1 << ex) {
            return (s as u32, d - base);
        }
    }
    panic!("bad distance {d}");
}

/// Simulate what cp_inflate will produce for a token list (so the caller can
/// size `out_bytes` correctly).  Returns None when the token list is not
/// self-consistent (distance before start of output).
fn simulate(toks: &[Tok]) -> Option<Vec<u8>> {
    let mut out: Vec<u8> = Vec::new();
    for t in toks {
        match *t {
            Tok::Lit(b) => out.push(b),
            Tok::RawSym(s) => {
                if s < 256 {
                    out.push(s as u8)
                } else {
                    return None;
                }
            }
            Tok::Match { lsym, lextra, dsym, dextra } => {
                let len = LEN_BASE[(lsym - 257) as usize] + lextra;
                let dist = DIST_BASE[dsym as usize] + dextra;
                if (dist as usize) > out.len() {
                    return None;
                }
                for _ in 0..len {
                    let b = out[out.len() - dist as usize];
                    out.push(b);
                }
            }
        }
    }
    Some(out)
}

fn emit_fixed_block(w: &mut BitW, bfinal: bool, toks: &[Tok]) {
    w.bit(bfinal as u32);
    w.bits(1, 2); // btype = 01
    let lit_codes = canonical(&FIXED_LIT_LENS);
    let dist_lens = [5u8; 32];
    let dist_codes = canonical(&dist_lens);
    for t in toks {
        match *t {
            Tok::Lit(b) => w.code(lit_codes[b as usize], FIXED_LIT_LENS[b as usize] as u32),
            Tok::RawSym(s) => w.code(lit_codes[s as usize], FIXED_LIT_LENS[s as usize] as u32),
            Tok::Match { lsym, lextra, dsym, dextra } => {
                w.code(lit_codes[lsym as usize], FIXED_LIT_LENS[lsym as usize] as u32);
                let le = if lsym >= 257 && lsym <= 285 { LEN_EXTRA[(lsym - 257) as usize] } else { 0 };
                w.bits(lextra, le as u32);
                w.code(dist_codes[dsym as usize], 5);
                let de = if dsym < 30 { DIST_EXTRA[dsym as usize] } else { 0 };
                w.bits(dextra, de as u32);
            }
        }
    }
    // end of block
    w.code(lit_codes[256], FIXED_LIT_LENS[256] as u32);
}

struct DynTrees {
    lit_lens: Vec<u8>, // exactly nlit entries
    dst_lens: Vec<u8>, // exactly ndst entries
    hclen: usize,      // 4..=19
}

/// Emit a dynamic block.  `cl_encode` selects whether run-length code-length
/// symbols 16/17/18 are used.
fn emit_dynamic_block(w: &mut BitW, bfinal: bool, t: &DynTrees, toks: &[Tok], use_runs: bool) {
    let nlit = t.lit_lens.len();
    let ndst = t.dst_lens.len();
    assert!((257..=288).contains(&nlit));
    assert!((1..=32).contains(&ndst));
    assert!((4..=19).contains(&t.hclen));

    // the flat code-length sequence the decoder must reproduce
    let mut flat: Vec<u8> = Vec::new();
    flat.extend_from_slice(&t.lit_lens);
    flat.extend_from_slice(&t.dst_lens);

    // encode `flat` with code-length symbols
    let mut cl_syms: Vec<(u32, u32, u32)> = Vec::new(); // (sym, extra_bits, extra_val)
    let mut i = 0usize;
    while i < flat.len() {
        let v = flat[i];
        if use_runs && v == 0 {
            let mut run = 1usize;
            while i + run < flat.len() && flat[i + run] == 0 && run < 138 {
                run += 1;
            }
            if run >= 11 {
                cl_syms.push((18, 7, (run - 11) as u32));
                i += run;
                continue;
            } else if run >= 3 {
                cl_syms.push((17, 3, (run - 3) as u32));
                i += run;
                continue;
            }
        }
        if use_runs && i > 0 && flat[i - 1] == v {
            let mut run = 0usize;
            while i + run < flat.len() && flat[i + run] == v && run < 6 {
                run += 1;
            }
            if run >= 3 {
                cl_syms.push((16, 2, (run - 3) as u32));
                i += run;
                continue;
            }
        }
        cl_syms.push((v as u32, 0, 0));
        i += 1;
    }

    // code-length alphabet lengths (<= 7 bits, complete code)
    let mut used: Vec<u32> = Vec::new();
    for &(s, _, _) in &cl_syms {
        if !used.contains(&s) {
            used.push(s);
        }
    }
    used.sort();
    // ensure at least 2 symbols so the CL code is complete
    if used.len() < 2 {
        for s in 0..19u32 {
            if !used.contains(&s) {
                used.push(s);
                break;
            }
        }
        used.sort();
    }
    // all symbols must fit in the first `hclen` entries of the permutation order
    let allowed: Vec<usize> = PERM[..t.hclen].to_vec();
    for &s in &used {
        assert!(allowed.contains(&(s as usize)), "cl symbol {s} not covered by hclen {}", t.hclen);
    }
    let mut rng = Rng::new(0xC0DE_1234 ^ used.len() as u64);
    let depths = complete_depths(used.len(), 7, &mut rng);
    let mut cl_lens = [0u8; 19];
    for (k, &s) in used.iter().enumerate() {
        cl_lens[s as usize] = depths[k];
    }
    let cl_codes = canonical(&cl_lens);

    w.bit(bfinal as u32);
    w.bits(2, 2); // btype = 10
    w.bits((nlit - 257) as u32, 5);
    w.bits((ndst - 1) as u32, 5);
    w.bits((t.hclen - 4) as u32, 4);
    for k in 0..t.hclen {
        w.bits(cl_lens[PERM[k]] as u32, 3);
    }
    for &(s, eb, ev) in &cl_syms {
        w.code(cl_codes[s as usize], cl_lens[s as usize] as u32);
        if eb > 0 {
            w.bits(ev, eb);
        }
    }

    let lit_codes = canonical(&t.lit_lens);
    let dst_codes = canonical(&t.dst_lens);
    for tk in toks {
        match *tk {
            Tok::Lit(b) => {
                assert!(t.lit_lens[b as usize] != 0);
                w.code(lit_codes[b as usize], t.lit_lens[b as usize] as u32)
            }
            Tok::RawSym(s) => w.code(lit_codes[s as usize], t.lit_lens[s as usize] as u32),
            Tok::Match { lsym, lextra, dsym, dextra } => {
                assert!(t.lit_lens[lsym as usize] != 0);
                w.code(lit_codes[lsym as usize], t.lit_lens[lsym as usize] as u32);
                let le = if (257..=285).contains(&lsym) { LEN_EXTRA[(lsym - 257) as usize] } else { 0 };
                w.bits(lextra, le as u32);
                assert!(t.dst_lens[dsym as usize] != 0);
                w.code(dst_codes[dsym as usize], t.dst_lens[dsym as usize] as u32);
                let de = if dsym < 30 { DIST_EXTRA[dsym as usize] } else { 0 };
                w.bits(dextra, de as u32);
            }
        }
    }
    assert!(t.lit_lens[256] != 0);
    w.code(lit_codes[256], t.lit_lens[256] as u32);
}

/// Build a dynamic tree covering exactly the symbols used by `toks` (+ 256).
fn dyn_trees_for(toks: &[Tok], nlit: usize, ndst: usize, hclen: usize, rng: &mut Rng) -> DynTrees {
    let mut lit_used: Vec<usize> = vec![256];
    let mut dst_used: Vec<usize> = Vec::new();
    for t in toks {
        match *t {
            Tok::Lit(b) => {
                if !lit_used.contains(&(b as usize)) {
                    lit_used.push(b as usize)
                }
            }
            Tok::RawSym(s) => {
                if !lit_used.contains(&(s as usize)) {
                    lit_used.push(s as usize)
                }
            }
            Tok::Match { lsym, dsym, .. } => {
                if !lit_used.contains(&(lsym as usize)) {
                    lit_used.push(lsym as usize)
                }
                if !dst_used.contains(&(dsym as usize)) {
                    dst_used.push(dsym as usize)
                }
            }
        }
    }
    lit_used.retain(|&s| s < nlit);
    dst_used.retain(|&s| s < ndst);
    if lit_used.len() < 2 {
        for s in 0..nlit {
            if !lit_used.contains(&s) {
                lit_used.push(s);
                break;
            }
        }
    }
    while dst_used.len() < 2 && ndst >= 2 {
        for s in 0..ndst {
            if !dst_used.contains(&s) {
                dst_used.push(s);
                break;
            }
        }
    }
    lit_used.sort();
    dst_used.sort();

    let mut lit_lens = vec![0u8; nlit];
    let d = complete_depths(lit_used.len(), 15, rng);
    for (k, &s) in lit_used.iter().enumerate() {
        lit_lens[s] = d[k];
    }
    let mut dst_lens = vec![0u8; ndst];
    if dst_used.len() >= 2 {
        let d = complete_depths(dst_used.len(), 15, rng);
        for (k, &s) in dst_used.iter().enumerate() {
            dst_lens[s] = d[k];
        }
    }
    DynTrees { lit_lens, dst_lens, hclen }
}

/// pad the encoded stream so the trailing bit reads never run off the end
fn finish(mut w: BitW, pad: usize) -> Vec<u8> {
    w.align_byte();
    let mut v = w.bytes;
    for k in 0..pad {
        v.push(pat(0x1000 + k));
    }
    v
}

// ===========================================================================
// parent: locate both .so files, run every group in child processes, compare
// ===========================================================================

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> PathBuf {
    let build = manifest_dir().join("c_src/build");
    let p = build.join("libtranslated_rust.so");
    if p.exists() {
        return p;
    }
    // try to build it
    fs::create_dir_all(&build).unwrap();
    let ok = Command::new("cmake")
        .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
        .current_dir(&build)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
        && Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
    assert!(ok && p.exists(), "could not build the C shared library at {p:?}");
    p
}

fn rust_lib_path() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    let deps = exe.parent().unwrap().to_path_buf(); // target/<profile>/deps
    let profile = deps.parent().unwrap().to_path_buf(); // target/<profile>
    for cand in [
        profile.join("libconvert_pix_lib.so"),
        deps.join("libconvert_pix_lib.so"),
    ] {
        if cand.exists() {
            return cand;
        }
    }
    panic!("libconvert_pix_lib.so not found next to {exe:?}; run `cargo build` first");
}

struct RunOut {
    /// concatenated (idx,len,payload) records
    raw: Vec<u8>,
    /// number of complete records
    n: usize,
    status: String,
    /// the child's stderr (glibc prints the failed assertion there)
    stderr: String,
}

/// one record per case: `(case index, normalised status, payload)`
fn split_records(raw: &[u8]) -> Vec<(u32, i32, Vec<u8>)> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + 12 <= raw.len() {
        let idx = u32::from_le_bytes(raw[i..i + 4].try_into().unwrap());
        let st = i32::from_le_bytes(raw[i + 4..i + 8].try_into().unwrap());
        let len = u32::from_le_bytes(raw[i + 8..i + 12].try_into().unwrap()) as usize;
        if i + 12 + len > raw.len() {
            break;
        }
        out.push((idx, st, raw[i + 12..i + 12 + len].to_vec()));
        i += 12 + len;
    }
    out
}

fn run_child(lib: &Path, spec: &[u8], tag: &str) -> RunOut {
    let tmp = std::env::temp_dir();
    let pid = std::process::id();
    let spec_path = tmp.join(format!("diffspec-{pid}-{tag}.bin"));
    let out_path = tmp.join(format!("diffout-{pid}-{tag}.bin"));
    fs::write(&spec_path, spec).unwrap();
    let _ = fs::remove_file(&out_path);
    let exe = std::env::current_exe().unwrap();
    let o = Command::new("timeout")
        .arg(std::env::var("DIFF_CHILD_TIMEOUT").unwrap_or_else(|_| "540".into()))
        .arg(exe)
        .env("DIFF_CHILD_LIB", lib)
        .env("DIFF_CHILD_SPEC", &spec_path)
        .env("DIFF_CHILD_OUT", &out_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .output()
        .expect("spawn child");
    let st = o.status;
    let stderr = String::from_utf8_lossy(&o.stderr).to_string();
    let raw = fs::read(&out_path).unwrap_or_default();
    let n = split_records(&raw).len();
    let status = match st.code() {
        Some(c) => format!("exit {c}"),
        None => format!("signal {}", st.signal().unwrap_or(-1)),
    };
    let _ = fs::remove_file(&spec_path);
    let _ = fs::remove_file(&out_path);
    RunOut { raw, n, status, stderr }
}

struct Report {
    groups: usize,
    cases: usize,
    aborts: usize,
    failures: Vec<String>,
}

impl Report {
    fn new() -> Report {
        Report { groups: 0, cases: 0, aborts: 0, failures: Vec::new() }
    }
}

/// Run `cases` against both libraries and require byte-identical behaviour AND
/// identical per-case termination (exit code / fatal signal).
fn check_group(rep: &mut Report, name: &str, cases: &[Case]) {
    let _ = run_and_compare(rep, name, cases, None, None);
}

/// per-case results of one comparison run
struct GroupOut {
    /// (status, payload) for the C library, per case
    c: Vec<(i32, Vec<u8>)>,
    /// the C child's stderr for the whole batch
    stderr: String,
}

fn run_and_compare(
    rep: &mut Report,
    name: &str,
    cases: &[Case],
    want_status: Option<i32>,
    assert_text: Option<&str>,
) -> Option<GroupOut> {
    rep.groups += 1;
    let t0 = std::time::Instant::now();
    let cl = c_lib_path();
    let rl = rust_lib_path();
    let spec = encode_spec(cases);
    let c = run_child(&cl, &spec, "c");
    let r = run_child(&rl, &spec, "rust");
    let crecs = split_records(&c.raw);
    let rrecs = split_records(&r.raw);

    if crecs.len() != cases.len() || rrecs.len() != cases.len() {
        rep.failures.push(format!(
            "{name}: batch did not complete: C produced {} records ({}), RUST {} ({}) for {} cases",
            crecs.len(),
            c.status,
            rrecs.len(),
            r.status,
            cases.len()
        ));
        return None;
    }
    let mut aborts = 0usize;
    for k in 0..cases.len() {
        let (_, cst, ref cp) = crecs[k];
        let (_, rst, ref rp) = rrecs[k];
        if cst != rst {
            rep.failures.push(format!(
                "{name}: case {k} terminated differently: C={} RUST={}",
                status_str(cst),
                status_str(rst)
            ));
            return None;
        }
        if cp != rp {
            rep.failures.push(format!(
                "{name}: case {k} output differs\n    C   ({} bytes) {}\n    RUST({} bytes) {}",
                cp.len(),
                hexdump(cp),
                rp.len(),
                hexdump(rp)
            ));
            return None;
        }
        if cst != 0 {
            aborts += 1;
        }
        if let Some(w) = want_status {
            if cst != w {
                rep.failures.push(format!(
                    "{name}: case {k} expected {} but got {}",
                    status_str(w),
                    status_str(cst)
                ));
                return None;
            }
        }
    }
    if let Some(t) = assert_text {
        if !c.stderr.contains(t) {
            rep.failures.push(format!(
                "{name}: the C library's stderr never mentions {t:?}; got {:?}",
                c.stderr.trim()
            ));
            return None;
        }
    }
    if std::env::var("DIFF_SHOW_STDERR").is_ok() && !c.stderr.trim().is_empty() {
        println!("    [{name}] C stderr: {}", c.stderr.trim().lines().last().unwrap_or(""));
    }
    rep.cases += cases.len();
    rep.aborts += aborts;
    let hangs = crecs.iter().filter(|r| r.1 == -14).count();
    println!(
        "  ok {name}: {} cases, {} identical non-zero terminations{}{}",
        cases.len(),
        aborts,
        if hangs > 0 {
            format!(" ({hangs} of them identical SIGALRM timeouts: the C library does not terminate)")
        } else {
            String::new()
        },
        match assert_text {
            Some(t) => format!("  [C assert: {t}]"),
            None => String::new(),
        }
    );
    if t0.elapsed().as_secs_f64() > 5.0 {
        println!("       ({:.1}s)", t0.elapsed().as_secs_f64());
    }
    Some(GroupOut { c: crecs.into_iter().map(|(_, s, p)| (s, p)).collect(), stderr: c.stderr })
}

/// every case must terminate with `sig` (as a fatal signal) in both libraries;
/// `sig < 0` means "any fatal signal, as long as it is the same one".
fn check_group_expect_signal(rep: &mut Report, name: &str, cases: &[Case], sig: i32) {
    check_group_expect_signal_msg(rep, name, cases, sig, None)
}

/// `assert_text`: a substring that must appear in the C child's stderr, proving
/// that the *intended* `assert()` (and not some other one) fired.
fn check_group_expect_signal_msg(
    rep: &mut Report,
    name: &str,
    cases: &[Case],
    sig: i32,
    assert_text: Option<&str>,
) {
    let want = if sig < 0 { None } else { Some(-sig) };
    if let Some(g) = run_and_compare(rep, name, cases, want, assert_text) {
        if sig < 0 {
            for (k, (st, _)) in g.c.iter().enumerate() {
                if *st >= 0 {
                    rep.failures.push(format!(
                        "{name}: case {k} was expected to die from a signal, got {}",
                        status_str(*st)
                    ));
                    return;
                }
            }
        }
        let _ = g.stderr;
    }
}

fn decode_inflate(payload: &[u8]) -> (i32, Option<Vec<u8>>) {
    let mut r = R::new(payload);
    let ret = r.i32();
    let _out = r.blob();
    let has = r.u8();
    let reason = if has == 1 { Some(r.blob().to_vec()) } else { None };
    (ret, reason)
}

/// Phase C helper: both libraries must return 0 AND set cp_error_reason to the
/// very same C string literal (the "same error code / sentinel" requirement).
fn check_group_expect_error(rep: &mut Report, name: &str, cases: &[Case], expected: &str) {
    let g = match run_and_compare(rep, name, cases, Some(0), None) {
        Some(g) => g,
        None => return,
    };
    for (k, (_, payload)) in g.c.iter().enumerate() {
        let (ret, reason) = decode_inflate(payload);
        if ret != 0 {
            rep.failures.push(format!("{name}: case {k} returned {ret}, expected 0"));
            return;
        }
        let got = reason
            .as_ref()
            .map(|v| String::from_utf8_lossy(v).to_string())
            .unwrap_or_default();
        if got != expected {
            rep.failures.push(format!(
                "{name}: case {k} cp_error_reason = {got:?}, expected {expected:?}"
            ));
            return;
        }
    }
    println!("     .. and all {} cases return 0 with the expected reason", g.c.len());
}

fn hexdump(b: &[u8]) -> String {
    let n = b.len().min(96);
    let mut s = String::new();
    for &x in &b[..n] {
        s.push_str(&format!("{x:02x}"));
    }
    if b.len() > n {
        s.push_str("...");
    }
    s
}

// ===========================================================================
// stream builders
// ===========================================================================

fn len_base_of(lsym: u32) -> u32 {
    let i = (lsym - 257) as usize;
    if i < 29 { LEN_BASE[i] } else { 0 }
}
fn dist_base_of(dsym: u32) -> u32 {
    let i = dsym as usize;
    if i < 30 { DIST_BASE[i] } else { 0 }
}

fn fixed_stream(bfinal: bool, toks: &[Tok]) -> Vec<u8> {
    let mut w = BitW::new();
    emit_fixed_block(&mut w, bfinal, toks);
    finish(w, 12)
}

fn dyn_stream(bfinal: bool, t: &DynTrees, toks: &[Tok], use_runs: bool) -> Vec<u8> {
    let mut w = BitW::new();
    emit_dynamic_block(&mut w, bfinal, t, toks, use_runs);
    finish(w, 12)
}

fn stored_bytes(payload: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    v.push(0x01); // bfinal=1, btype=00, then 5 pad bits to the byte boundary
    let len = payload.len() as u16;
    v.extend_from_slice(&len.to_le_bytes());
    v.extend_from_slice(&(!len).to_le_bytes());
    v.extend_from_slice(payload);
    v
}

/// tokens that inflate to at least `target` bytes (uses the distance==1 memset
/// path so it is cheap to encode)
fn grow_tokens(target: usize) -> Vec<Tok> {
    let mut t = vec![Tok::Lit(0xA5)];
    let mut n = 1usize;
    while n < target {
        let want = (target - n).min(258).max(3);
        let (ls, le) = lensym_for(want as u32);
        let (ds, de) = distsym_for(1);
        t.push(Tok::Match { lsym: ls, lextra: le, dsym: ds, dextra: de });
        n += want;
    }
    t
}

fn inflate_case_for(stream: Vec<u8>, out_len: usize) -> InflateCase {
    let ob = out_len as i32;
    InflateCase::new(stream).out(ob, (out_len + 128) as u32)
}

/// build an inflate case for a token list encoded as a fixed block
fn fixed_case(toks: &[Tok]) -> Option<InflateCase> {
    let sim = simulate_ext(toks)?;
    Some(inflate_case_for(fixed_stream(true, toks), sim.len()))
}

/// like `simulate` but tolerates the reserved symbols (`*_base == 0`)
fn simulate_ext(toks: &[Tok]) -> Option<Vec<u8>> {
    let mut out: Vec<u8> = Vec::new();
    for t in toks {
        match *t {
            Tok::Lit(b) => out.push(b),
            Tok::RawSym(s) => {
                if s < 256 {
                    out.push(s as u8)
                } else {
                    return None;
                }
            }
            Tok::Match { lsym, lextra, dsym, dextra } => {
                let len = len_base_of(lsym).wrapping_add(lextra);
                let dist = dist_base_of(dsym).wrapping_add(dextra);
                if dist as usize > out.len() {
                    return None;
                }
                if dist == 0 {
                    if len != 0 {
                        return None;
                    }
                    continue;
                }
                for _ in 0..len {
                    let b = out[out.len() - dist as usize];
                    out.push(b);
                }
            }
        }
    }
    Some(out)
}

// ===========================================================================
// main
// ===========================================================================

fn main() {
    if let Ok(lib) = std::env::var("DIFF_CHILD_LIB") {
        let spec = std::env::var("DIFF_CHILD_SPEC").unwrap();
        let out = std::env::var("DIFF_CHILD_OUT").unwrap();
        child_main(&lib, &spec, &out);
        return;
    }

    if std::env::var("DIFF_PROBE").is_ok() {
        probe();
        return;
    }

    let mut rep = Report::new();
    let only = std::env::var("DIFF_ONLY").unwrap_or_default();
    let sel = |n: &str| only.is_empty() || n.contains(&only) || only.split(',').any(|s| n == s);

    // ---- Phase D: symbol parity -------------------------------------------
    if sel("symbol_parity") {
        symbol_parity(&mut rep);
        dead_static_not_exported(&mut rep);
    }

    // ---- Phase B ----------------------------------------------------------
    if sel("cfg") {
        phase_b(&mut rep);
        phase_b_dynamic(&mut rep);
        phase_b_tables(&mut rep);
    }
    // ---- Phase C ----------------------------------------------------------
    if sel("err") {
        phase_c_errors(&mut rep);
    }
    if sel("abort") {
        phase_c_aborts(&mut rep);
    }
    if sel("ovs") {
        phase_c_overshoot(&mut rep);
    }
    // ---- fuzz -------------------------------------------------------------
    if sel("fuzz37") {
        fuzz_valid(&mut rep);
    }
    if sel("fuzz38") {
        fuzz_mutated(&mut rep);
    }
    if sel("fuzz39") {
        fuzz_random(&mut rep);
    }

    println!(
        "\n=== groups: {}  cases compared: {}  identical aborts: {}  failures: {} ===",
        rep.groups,
        rep.cases,
        rep.aborts,
        rep.failures.len()
    );
    for f in &rep.failures {
        println!("FAIL {f}");
    }
    if !rep.failures.is_empty() {
        std::process::exit(1);
    }
    println!("ALL DIFFERENTIAL CHECKS PASSED");
}

fn nm_syms(p: &Path) -> Vec<String> {
    let out = Command::new("nm").args(["-D", "--defined-only"]).arg(p).output().expect("nm");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let _addr = it.next()?;
            let kind = it.next()?.to_string();
            let name = it.next()?.to_string();
            if name.starts_with("_ZN") {
                None
            } else {
                Some(format!("{kind} {name}"))
            }
        })
        .collect()
}

fn symbol_parity(rep: &mut Report) {
    rep.groups += 1;
    let mut c = nm_syms(&c_lib_path());
    let mut r = nm_syms(&rust_lib_path());
    c.sort();
    r.sort();
    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    if !missing.is_empty() {
        rep.failures.push(format!("symbol_parity: Rust .so is missing {missing:?}"));
    }
    let extra: Vec<&String> = r.iter().filter(|s| !c.contains(s)).collect();
    if !extra.is_empty() {
        rep.failures.push(format!("symbol_parity: Rust .so exports extra {extra:?}"));
    }
    println!("symbol_parity: {} C symbols, {} Rust symbols, diff empty = {}", c.len(), r.len(), missing.is_empty() && extra.is_empty());
    // CONFIGS row 1: the exported table contents must be identical
    check_group(rep, "cfg01_table_contents", &[case_tables()]);
}

fn dead_static_not_exported(rep: &mut Report) {
    rep.groups += 1;
    let names = [
        "cp_build", "cp_decode", "cp_unfilter", "cp_paeth", "cp_stored", "cp_fixed",
        "cp_dynamic", "cp_block", "cp_chunk", "cp_find", "cp_make32", "cp_read_bits",
        "cp_peak_bits", "cp_consume_bits", "cp_would_overflow", "cp_ptr", "cp_rev16",
        "cp_make_pixel", "cp_make_pixel_a",
    ];
    for lib in [c_lib_path(), rust_lib_path()] {
        let syms = nm_syms(&lib);
        for n in names {
            if syms.iter().any(|s| s.ends_with(&format!(" {n}"))) {
                rep.failures
                    .push(format!("err_dead_static_not_exported: {lib:?} exports static {n}"));
            }
        }
    }
    println!(
        "  ok err_dead_static_not_exported: {} `static` C functions are absent from both .so files \
         (so ERRORS.md rows 7-10 are unobservable through the API)",
        names.len()
    );
}

// ===========================================================================
// Phase B - one function per CONFIGS.md row
// ===========================================================================

const SHAPES: [(i32, i32); 6] = [(1, 1), (1, 7), (7, 1), (3, 5), (17, 13), (64, 3)];

fn convert_cases(bpp: i32, seed: u64, n: usize) -> Vec<Case> {
    let mut rng = Rng::new(seed);
    let mut v = Vec::new();
    for i in 0..n {
        let (w, h) = SHAPES[i % SHAPES.len()];
        let src_len = (h as usize) * (1 + (w as usize) * (bpp as usize)) + 8;
        let src: Vec<u8> = (0..src_len).map(|_| rng.byte()).collect();
        let dst_len = (w as usize * h as usize * 4 + 16) as u32;
        v.push(case_convert(bpp, w, h, &src, dst_len, false, false));
    }
    v
}

fn phase_b(rep: &mut Report) {
    // rows 2..5 : convert_pix, bpp = 1..4
    for bpp in 1..=4i32 {
        let cases = convert_cases(bpp, 0x5EED_0000 + bpp as u64, 256);
        check_group(rep, &format!("cfg{:02}_convert_pix_bpp{}", bpp + 1, bpp), &cases);
    }

    // row 6 : bpp outside the switch
    {
        let mut v = Vec::new();
        for bpp in [0i32, 5, 6, 255, -1, -4, i32::MIN, i32::MAX] {
            for &(w, h) in &SHAPES[..4] {
                let src: Vec<u8> = (0..512).map(|i| pat(i + 7)).collect();
                v.push(case_convert(bpp, w, h, &src, 256, false, false));
            }
        }
        check_group(rep, "cfg06_convert_pix_bpp_out_of_range", &v);
    }

    // row 7 : degenerate shapes + NULL pointers
    {
        let mut v = Vec::new();
        let src: Vec<u8> = (0..512).map(|i| pat(i + 11)).collect();
        for bpp in [1i32, 2, 3, 4, 0, -1] {
            for &(w, h) in &[
                (0i32, 0i32),
                (0, 5),
                (5, 0),
                (-1, 5),
                (5, -1),
                (i32::MIN, 3),
                (3, i32::MIN),
                (i32::MAX, 0),
                // NOTE: (0, i32::MAX) is deliberately *not* used here.  With
                // w <= 0 the inner loop never runs, so the row loop only does
                // `src++` h times; the C build (-O0) really executes all 2^31
                // iterations while an optimised Rust build deletes the dead
                // loop.  The observable result (dst untouched) is the same, only
                // the running time differs, so the row is exercised with a large
                // but tractable h instead.
                (0, 1_000_000),
                (0, 20_000_000),
                (-3, 1_000_000),
            ] {
                v.push(case_convert(bpp, w, h, &src, 256, false, false));
            }
            // NULL src/dst with h <= 0 (nothing is dereferenced)
            v.push(case_convert(bpp, 4, 0, &[], 0, true, true));
            v.push(case_convert(bpp, 4, -3, &[], 0, true, true));
            // NULL src/dst with w <= 0 (only pointer arithmetic runs)
            v.push(case_convert(bpp, 0, 4, &[], 0, true, true));
        }
        check_group(rep, "cfg07_convert_pix_degenerate", &v);
    }

    // row 8 : stored, LEN == 0
    check_group(
        rep,
        "cfg08_stored_len0",
        &[
            InflateCase::new(stored_bytes(&[])).out(0, 64).encode(),
            InflateCase::new(stored_bytes(&[])).out(16, 64).encode(),
        ],
    );

    // row 9 : stored, several LEN values
    {
        let mut rng = Rng::new(0x5709ED);
        let mut v = Vec::new();
        for len in [1usize, 2, 3, 4, 7, 8, 31, 255, 4096] {
            for _ in 0..4 {
                let p: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
                v.push(
                    InflateCase::new(stored_bytes(&p))
                        .out(len as i32, (len + 128) as u32)
                        .encode(),
                );
            }
        }
        check_group(rep, "cfg09_stored_lengths", &v);
    }

    // row 10 : stored x alignment sweep
    {
        let mut rng = Rng::new(0x5709ED_A1);
        let mut v = Vec::new();
        for align in 0..4u8 {
            for len in [0usize, 1, 2, 3, 4, 5, 6, 7, 8, 9, 100, 1000] {
                let p: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
                v.push(
                    InflateCase::new(stored_bytes(&p))
                        .align(align)
                        .out(len as i32, (len + 256) as u32)
                        .encode(),
                );
            }
        }
        check_group(rep, "cfg10_stored_alignment", &v);
    }

    // row 11 : stored with out slack
    {
        let mut rng = Rng::new(0x5709ED_B2);
        let mut v = Vec::new();
        for len in [0usize, 1, 5, 33, 700] {
            let p: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
            v.push(
                InflateCase::new(stored_bytes(&p))
                    .out((len + 100) as i32, (len + 300) as u32)
                    .encode(),
            );
        }
        check_group(rep, "cfg11_stored_out_slack", &v);
    }

    // row 12 : stored whose unchecked memcpy overruns out_end
    {
        let mut rng = Rng::new(0x5709ED_C3);
        let mut v = Vec::new();
        for (len, ob) in [(4096usize, 16i32), (4096, 0), (65535, 16), (300, 1)] {
            let p: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
            v.push(
                InflateCase::new(stored_bytes(&p))
                    .out(ob, (len + 4096) as u32)
                    .encode(),
            );
        }
        check_group(rep, "cfg12_stored_memcpy_overrun", &v);
    }

    // row 13 : fixed, empty block
    {
        let s = fixed_stream(true, &[]);
        let mut v = Vec::new();
        for ob in [0i32, 1, 64] {
            v.push(InflateCase::new(s.clone()).out(ob, 256).encode());
        }
        check_group(rep, "cfg13_fixed_empty", &v);
    }

    // row 14 : fixed, literals only (8-bit and 9-bit code classes)
    {
        let mut rng = Rng::new(0xF1_1EAD);
        let mut v = Vec::new();
        for i in 0..512 {
            let n = 1 + rng.below(300) as usize;
            let toks: Vec<Tok> = (0..n)
                .map(|_| {
                    // deliberately bias towards both fixed-code length classes
                    let b = if i % 3 == 0 {
                        rng.below(144) as u8
                    } else if i % 3 == 1 {
                        (144 + rng.below(112)) as u8
                    } else {
                        rng.byte()
                    };
                    Tok::Lit(b)
                })
                .collect();
            v.push(fixed_case(&toks).unwrap().encode());
        }
        check_group(rep, "cfg14_fixed_literals", &v);
    }

    // row 15 : length symbols 257..264 (0 extra bits) x distances 1..4
    {
        let mut v = Vec::new();
        for lsym in 257..=264u32 {
            for dist in 1..=4u32 {
                let mut toks: Vec<Tok> = (0..8).map(|i| Tok::Lit(0x40 + i as u8)).collect();
                let (ds, de) = distsym_for(dist);
                toks.push(Tok::Match { lsym, lextra: 0, dsym: ds, dextra: de });
                toks.push(Tok::Lit(0xFF));
                v.push(fixed_case(&toks).unwrap().encode());
            }
        }
        check_group(rep, "cfg15_fixed_len257_264", &v);
    }

    // row 16 : length symbols 265..284 (1..5 extra bits) x every distance class
    {
        let mut rng = Rng::new(0x1E16_5EED);
        let mut v = Vec::new();
        for lsym in 265..=284u32 {
            let le = LEN_EXTRA[(lsym - 257) as usize] as u32;
            for dsym in 0..30u32 {
                let de = DIST_EXTRA[dsym as usize] as u32;
                let lextra = rng.below(1 << le);
                let dextra = rng.below(1 << de);
                let dist = DIST_BASE[dsym as usize] + dextra;
                let mut toks = grow_tokens(dist as usize + 4);
                toks.push(Tok::Match { lsym, lextra, dsym, dextra });
                toks.push(Tok::Lit(0x5A));
                v.push(fixed_case(&toks).unwrap().encode());
            }
        }
        check_group(rep, "cfg16_fixed_len_dist_classes", &v);
    }

    // row 17 : length symbol 285 (258, no extra bits)
    {
        let mut v = Vec::new();
        for dist in [1u32, 2, 257, 258, 4096, 32768] {
            let mut toks = grow_tokens(dist as usize + 2);
            let (ds, de) = distsym_for(dist);
            toks.push(Tok::Match { lsym: 285, lextra: 0, dsym: ds, dextra: de });
            v.push(fixed_case(&toks).unwrap().encode());
        }
        check_group(rep, "cfg17_fixed_len285", &v);
    }

    // row 18 : distance == 1 (memset branch), all lengths
    {
        let mut v = Vec::new();
        for len in 3..=258u32 {
            let (ls, le) = lensym_for(len);
            let toks =
                vec![Tok::Lit(0x77), Tok::Match { lsym: ls, lextra: le, dsym: 0, dextra: 0 }];
            v.push(fixed_case(&toks).unwrap().encode());
        }
        check_group(rep, "cfg18_fixed_memset_dist1", &v);
    }

    // row 19 : overlapping matches (length > distance)
    {
        let mut v = Vec::new();
        for dist in [2u32, 3, 5] {
            for len in [3u32, 4, 7, 16, 100, 258] {
                let mut toks: Vec<Tok> =
                    (0..dist).map(|i| Tok::Lit(0x10 + i as u8)).collect();
                let (ls, le) = lensym_for(len);
                let (ds, de) = distsym_for(dist);
                toks.push(Tok::Match { lsym: ls, lextra: le, dsym: ds, dextra: de });
                v.push(fixed_case(&toks).unwrap().encode());
            }
        }
        check_group(rep, "cfg19_fixed_overlap", &v);
    }

    // row 20 : random literal/match mixtures
    {
        let mut v = Vec::new();
        for c in 0..512u64 {
            let mut rng = Rng::new(0x2000_0000 + c);
            let toks = random_tokens(&mut rng, 4096);
            v.push(fixed_case(&toks).unwrap().encode());
        }
        check_group(rep, "cfg20_fixed_random_tokens", &v);
    }

    // row 21 : reserved length symbols 286/287 and distance symbols 30/31
    {
        let mut v = Vec::new();
        for lsym in [286u32, 287] {
            for dsym in [0u32, 1, 29, 30, 31] {
                let toks = vec![
                    Tok::Lit(1),
                    Tok::Lit(2),
                    Tok::Lit(3),
                    Tok::Match { lsym, lextra: 0, dsym, dextra: 0 },
                    Tok::Lit(4),
                ];
                let s = fixed_stream(true, &toks);
                v.push(InflateCase::new(s).out(64, 256).encode());
            }
        }
        for dsym in [30u32, 31] {
            let toks = vec![
                Tok::Lit(1),
                Tok::Lit(2),
                Tok::Match { lsym: 257, lextra: 0, dsym, dextra: 0 },
            ];
            let s = fixed_stream(true, &toks);
            v.push(InflateCase::new(s).out(64, 256).encode());
        }
        check_group(rep, "cfg21_reserved_len_dist_symbols", &v);
    }
}

fn random_tokens(rng: &mut Rng, max_out: usize) -> Vec<Tok> {
    let mut toks = Vec::new();
    let mut produced = 0usize;
    let target = 1 + rng.below(max_out as u32) as usize;
    while produced < target {
        if produced >= 4 && rng.below(100) < 45 {
            let maxlen = (target - produced).min(258);
            if maxlen >= 3 {
                let len = 3 + rng.below((maxlen - 2) as u32);
                let dist = 1 + rng.below(produced.min(32768) as u32);
                let (ls, le) = lensym_for(len);
                let (ds, de) = distsym_for(dist);
                toks.push(Tok::Match { lsym: ls, lextra: le, dsym: ds, dextra: de });
                produced += len as usize;
                continue;
            }
        }
        toks.push(Tok::Lit(rng.byte()));
        produced += 1;
    }
    toks
}

// ===========================================================================
// Phase B - dynamic blocks, multi-block, sizes, table mutation
// ===========================================================================

/// build a dynamic tree whose non-zero code lengths are all `l`
fn single_length_trees(
    lit_used: &[usize],
    dst_used: &[usize],
    nlit: usize,
    ndst: usize,
    l: u8,
    hclen: usize,
) -> DynTrees {
    let mut lit_lens = vec![0u8; nlit];
    for &s in lit_used {
        lit_lens[s] = l;
    }
    let mut dst_lens = vec![0u8; ndst];
    for &s in dst_used {
        dst_lens[s] = l;
    }
    DynTrees { lit_lens, dst_lens, hclen }
}

fn phase_b_dynamic(rep: &mut Report) {
    // row 22 : dynamic, literals only, random complete trees, no run codes
    {
        let mut v = Vec::new();
        for c in 0..256u64 {
            let mut rng = Rng::new(0x3000_0000 + c);
            let n = 1 + rng.below(200) as usize;
            let toks: Vec<Tok> = (0..n).map(|_| Tok::Lit(rng.byte())).collect();
            let t = dyn_trees_for(&toks, 288, 32, 19, &mut rng);
            let sim = simulate_ext(&toks).unwrap();
            let s = dyn_stream(true, &t, &toks, false);
            v.push(inflate_case_for(s, sim.len()).encode());
        }
        check_group(rep, "cfg22_dynamic_literals", &v);
    }

    // rows 23/24/25 : code-length symbols 16 / 17 / 18
    {
        let mut v16 = Vec::new();
        let mut v17 = Vec::new();
        let mut v18 = Vec::new();
        for c in 0..64u64 {
            let mut rng = Rng::new(0x4000_0000 + c);
            let n = 1 + rng.below(80) as usize;
            let toks: Vec<Tok> = (0..n).map(|_| Tok::Lit(rng.byte())).collect();

            // 16 : four consecutive symbols with the same non-zero length
            let mut t = dyn_trees_for(&toks, 288, 32, 19, &mut rng);
            for k in 60..68 {
                t.lit_lens[k] = 9;
            }
            // keep the code a valid prefix code: rebuild with an all-9 tree
            let mut used: Vec<usize> = vec![256];
            for tk in &toks {
                if let Tok::Lit(b) = *tk {
                    if !used.contains(&(b as usize)) {
                        used.push(b as usize);
                    }
                }
            }
            for k in 60..68 {
                if !used.contains(&k) {
                    used.push(k);
                }
            }
            used.sort();
            let t16 = single_length_trees(&used, &[0, 1], 288, 32, 9, 19);
            let sim = simulate_ext(&toks).unwrap();
            v16.push(inflate_case_for(dyn_stream(true, &t16, &toks, true), sim.len()).encode());

            // 17 : a short (3..10) zero gap
            let mut used17: Vec<usize> = vec![0, 1, 2, 3, 12, 13, 256];
            for tk in &toks {
                if let Tok::Lit(b) = *tk {
                    if !used17.contains(&(b as usize)) {
                        used17.push(b as usize);
                    }
                }
            }
            used17.retain(|&s| s < 288);
            used17.sort();
            used17.dedup();
            let t17 = single_length_trees(&used17, &[0, 1], 257, 2, 9, 19);
            // nlit must be >= 257
            let t17 = DynTrees {
                lit_lens: {
                    let mut l = vec![0u8; 257];
                    for &s in &used17 {
                        if s < 257 {
                            l[s] = 9;
                        }
                    }
                    l[256] = 9;
                    l
                },
                dst_lens: t17.dst_lens,
                hclen: 19,
            };
            let toks17: Vec<Tok> = toks
                .iter()
                .cloned()
                .filter(|t| matches!(t, Tok::Lit(b) if (*b as usize) < 257))
                .collect();
            let toks17: Vec<Tok> = toks17
                .into_iter()
                .filter(|t| matches!(t, Tok::Lit(b) if t17.lit_lens[*b as usize] != 0))
                .collect();
            let sim17 = simulate_ext(&toks17).unwrap();
            v17.push(
                inflate_case_for(dyn_stream(true, &t17, &toks17, true), sim17.len()).encode(),
            );

            // 18 : long zero runs (nlit = 288 with few used symbols)
            let t18 = single_length_trees(&[0, 256], &[0, 1], 288, 32, 1, 19);
            let toks18: Vec<Tok> = (0..1 + rng.below(50) as usize).map(|_| Tok::Lit(0)).collect();
            let sim18 = simulate_ext(&toks18).unwrap();
            v18.push(
                inflate_case_for(dyn_stream(true, &t18, &toks18, true), sim18.len()).encode(),
            );
        }
        check_group(rep, "cfg23_dynamic_clsym16", &v16);
        check_group(rep, "cfg24_dynamic_clsym17", &v17);
        check_group(rep, "cfg25_dynamic_clsym18", &v18);
    }

    // row 26 : HCLEN sweep (nlen = 4..19)
    {
        let mut v = Vec::new();
        for hclen in 5..=19usize {
            let allowed: Vec<usize> = PERM[..hclen].to_vec();
            let l = *allowed.iter().filter(|&&x| (1..=15).contains(&x)).max().unwrap() as u8;
            let cap = 1usize << l;
            let mut rng = Rng::new(0x5000_0000 + hclen as u64);
            let nsym = 2 + rng.below((cap.min(60) - 1) as u32) as usize;
            let mut used: Vec<usize> = vec![256];
            let mut k = 0usize;
            while used.len() < nsym && k < 256 {
                used.push(k);
                k += 1;
            }
            used.sort();
            let t = single_length_trees(&used, &[0, 1], 257, 2, l, hclen);
            let toks: Vec<Tok> = (0..20)
                .map(|_| Tok::Lit(used[rng.below((used.len() - 1) as u32) as usize] as u8))
                .collect();
            let sim = simulate_ext(&toks).unwrap();
            v.push(inflate_case_for(dyn_stream(true, &t, &toks, false), sim.len()).encode());
        }
        check_group(rep, "cfg26_dynamic_hclen_sweep", &v);
    }

    // row 27 : HLIT x HDIST sweep
    {
        let mut v = Vec::new();
        for nlit in [257usize, 258, 270, 288] {
            for ndst in [1usize, 2, 15, 32] {
                let mut rng = Rng::new(0x6000_0000 + (nlit * 64 + ndst) as u64);
                let toks: Vec<Tok> = (0..24).map(|_| Tok::Lit(rng.byte() % 60)).collect();
                let mut used: Vec<usize> = vec![256];
                for t in &toks {
                    if let Tok::Lit(b) = *t {
                        if !used.contains(&(b as usize)) {
                            used.push(b as usize);
                        }
                    }
                }
                used.retain(|&s| s < nlit);
                used.sort();
                let dst_used: Vec<usize> = if ndst >= 2 { vec![0, 1] } else { vec![0] };
                let t = single_length_trees(&used, &dst_used, nlit, ndst, 9, 19);
                let sim = simulate_ext(&toks).unwrap();
                v.push(inflate_case_for(dyn_stream(true, &t, &toks, true), sim.len()).encode());
            }
        }
        check_group(rep, "cfg27_dynamic_hlit_hdist", &v);
    }

    // row 28 : dynamic with matches
    {
        let mut v = Vec::new();
        for c in 0..256u64 {
            let mut rng = Rng::new(0x7000_0000 + c);
            let toks = random_tokens(&mut rng, 2048);
            let t = dyn_trees_for(&toks, 288, 32, 19, &mut rng);
            let sim = simulate_ext(&toks).unwrap();
            v.push(inflate_case_for(dyn_stream(true, &t, &toks, true), sim.len()).encode());
        }
        check_group(rep, "cfg28_dynamic_matches", &v);
    }

    // row 29 : multi-block chains
    {
        let mut v = Vec::new();
        for c in 0..256u64 {
            let mut rng = Rng::new(0x8000_0000 + c);
            let nblocks = 2 + rng.below(3) as usize;
            let mut w = BitW::new();
            let mut total = 0usize;
            for b in 0..nblocks {
                let last = b + 1 == nblocks;
                let toks = random_tokens(&mut rng, 512);
                total += simulate_ext(&toks).unwrap().len();
                if rng.bool() {
                    emit_fixed_block(&mut w, last, &toks);
                } else {
                    let t = dyn_trees_for(&toks, 288, 32, 19, &mut rng);
                    emit_dynamic_block(&mut w, last, &t, &toks, rng.bool());
                }
            }
            let s = finish(w, 12);
            v.push(inflate_case_for(s, total).encode());
        }
        check_group(rep, "cfg29_multiblock", &v);
    }

    // row 30 : non-final fixed block followed by a final stored block
    {
        let mut v = Vec::new();
        for nlits in 0..12usize {
            for storedlen in [0usize, 1, 2, 3, 4, 8, 17] {
                let mut w = BitW::new();
                let toks: Vec<Tok> = (0..nlits).map(|i| Tok::Lit(0x30 + i as u8)).collect();
                emit_fixed_block(&mut w, false, &toks);
                // stored block header
                w.bit(1); // bfinal
                w.bits(0, 2); // btype = 00
                w.align_byte();
                let payload: Vec<u8> = (0..storedlen).map(|i| pat(i + 0x77)).collect();
                let len = storedlen as u16;
                w.byte((len & 0xFF) as u8);
                w.byte((len >> 8) as u8);
                w.byte((!len & 0xFF) as u8);
                w.byte(((!len) >> 8) as u8);
                for &b in &payload {
                    w.byte(b);
                }
                v.push(InflateCase::new(w.bytes).out(512, 1024).encode());
            }
        }
        check_group(rep, "cfg30_fixed_then_stored", &v);
    }

    // row 31 : in_bytes size classes x alignment
    {
        let mut v = Vec::new();
        let toks: Vec<Tok> = (0..40).map(|i| Tok::Lit(i as u8)).collect();
        let base = fixed_stream(true, &toks);
        for align in 0..4u8 {
            for extra in 0..8usize {
                let mut s = base.clone();
                for k in 0..extra {
                    s.push(pat(k + 0x321));
                }
                v.push(InflateCase::new(s).align(align).out(256, 512).encode());
            }
            // very short inputs (word_count == 0)
            for n in 1..=7usize {
                let s: Vec<u8> = base[..n.min(base.len())].to_vec();
                v.push(InflateCase::new(s).align(align).out(256, 512).encode());
            }
            // many words
            let big = fixed_stream(true, &(0..4096).map(|i| Tok::Lit(pat(i))).collect::<Vec<_>>());
            v.push(InflateCase::new(big).align(align).out(8192, 16384).encode());
        }
        check_group(rep, "cfg31_input_size_alignment", &v);
    }

    // row 32 : trailing garbage after the final block
    {
        let mut v = Vec::new();
        let toks: Vec<Tok> = (0..17).map(|i| Tok::Lit(i as u8 * 7)).collect();
        let mut w = BitW::new();
        emit_fixed_block(&mut w, true, &toks);
        let bits_used = w.nbits;
        for junkbits in 0..24usize {
            let mut w2 = BitW::new();
            emit_fixed_block(&mut w2, true, &toks);
            let mut rng = Rng::new(0x9000 + junkbits as u64);
            for _ in 0..junkbits {
                w2.bit(rng.next_u32());
            }
            let s = finish(w2, 8);
            v.push(InflateCase::new(s).out(64, 256).encode());
        }
        let _ = bits_used;
        check_group(rep, "cfg32_trailing_garbage", &v);
    }
}

// ===========================================================================
// Phase B - rows 33..36 : the exported tables must be read LIVE
// ===========================================================================

fn phase_b_tables(rep: &mut Report) {
    // row 33 : cp_fixed_table replaced by a different (still valid) assignment
    {
        // a complete code over the 288 literal/length symbols where the first
        // 32 get length 6 and the rest ... keep it simple and *legal*: swap the
        // 7-bit and 8-bit groups' lengths around by giving symbols 0..255 the
        // length 9 and 256..287 the length 5 -> Kraft = 256/512 + 32/32 > 1,
        // so instead use a plain "all 288 symbols would not fit" free layout:
        // 256 symbols of length 8 + 32 symbols of length 13 keeps Kraft <= 1.
        let mut muts = Vec::new();
        for i in 0..256u16 {
            muts.push((T_FIXED, i, 8u32));
        }
        for i in 256..288u16 {
            muts.push((T_FIXED, i, 13u32));
        }
        // distance table part (indices 288..320) : 32 codes of length 5
        let mut lit_lens = vec![0u8; 288];
        for i in 0..256 {
            lit_lens[i] = 8;
        }
        for i in 256..288 {
            lit_lens[i] = 13;
        }
        let lit_codes = canonical(&lit_lens);
        let dist_codes = canonical(&[5u8; 32]);

        let mut v = Vec::new();
        for c in 0..32u64 {
            let mut rng = Rng::new(0xA000 + c);
            let n = 1 + rng.below(60) as usize;
            let mut w = BitW::new();
            w.bit(1);
            w.bits(1, 2);
            let mut out = 0usize;
            for _ in 0..n {
                let b = rng.byte();
                w.code(lit_codes[b as usize], 8);
                out += 1;
            }
            // one match to exercise the relocated length codes
            if out >= 4 {
                w.code(lit_codes[257], 13);
                w.code(dist_codes[0], 5);
                out += 3;
            }
            w.code(lit_codes[256], 13);
            let s = finish(w, 12);
            let mut ic = InflateCase::new(s).out((out + 8) as i32, (out + 256) as u32);
            for &(t, i, val) in &muts {
                ic = ic.mutate(t, i, val);
            }
            v.push(ic.encode());
        }
        check_group(rep, "cfg33_mutate_fixed_table", &v);
    }

    // row 34 : cp_permutation_order rotated
    {
        // rotate the order; the encoder must use the SAME order, so emit the
        // 3-bit code-length fields in the mutated order.
        let mut perm = PERM;
        perm.rotate_left(5);
        let mut v = Vec::new();
        for c in 0..16u64 {
            let mut rng = Rng::new(0xB000 + c);
            let toks: Vec<Tok> = (0..1 + rng.below(40) as usize)
                .map(|_| Tok::Lit(rng.byte() % 40))
                .collect();
            let mut used: Vec<usize> = vec![256];
            for t in &toks {
                if let Tok::Lit(b) = *t {
                    if !used.contains(&(b as usize)) {
                        used.push(b as usize);
                    }
                }
            }
            used.sort();
            let t = single_length_trees(&used, &[0, 1], 257, 2, 9, 19);

            // hand-roll the dynamic header with the mutated permutation
            let mut flat = t.lit_lens.clone();
            flat.extend_from_slice(&t.dst_lens);
            let mut cl_lens = [0u8; 19];
            cl_lens[0] = 1;
            cl_lens[9] = 1;
            let cl_codes = canonical(&cl_lens);
            let mut w = BitW::new();
            w.bit(1);
            w.bits(2, 2);
            w.bits((t.lit_lens.len() - 257) as u32, 5);
            w.bits((t.dst_lens.len() - 1) as u32, 5);
            w.bits(15, 4); // hclen = 19
            for k in 0..19 {
                w.bits(cl_lens[perm[k]] as u32, 3);
            }
            for &f in &flat {
                w.code(cl_codes[f as usize], cl_lens[f as usize] as u32);
            }
            let lit_codes = canonical(&t.lit_lens);
            for tk in &toks {
                if let Tok::Lit(b) = *tk {
                    w.code(lit_codes[b as usize], 9);
                }
            }
            w.code(lit_codes[256], 9);
            let s = finish(w, 12);
            let mut ic = InflateCase::new(s).out(256, 512);
            for k in 0..19u16 {
                ic = ic.mutate(T_PERM, k, perm[k as usize] as u32);
            }
            v.push(ic.encode());
        }
        check_group(rep, "cfg34_mutate_permutation_order", &v);
    }

    // rows 35/36 : cp_len_extra_bits / cp_len_base / cp_dist_extra_bits /
    //              cp_dist_base mutated (still <= 32 extra bits)
    {
        let mut v = Vec::new();
        let mut rng = Rng::new(0xC0DE_0035);
        for c in 0..48u64 {
            let base_toks = grow_tokens(600);
            let mut toks = base_toks.clone();
            toks.push(Tok::Match { lsym: 260, lextra: 0, dsym: 5, dextra: 1 });
            toks.push(Tok::Lit(0x11));
            let s = fixed_stream(true, &toks);
            let mut ic = InflateCase::new(s).out(4096, 8192);
            match c % 4 {
                0 => {
                    ic = ic.mutate(T_LEXTRA, 3, 1 + rng.below(6));
                    ic = ic.mutate(T_LBASE, 3, 1 + rng.below(200));
                }
                1 => {
                    ic = ic.mutate(T_DEXTRA, 5, rng.below(5));
                    ic = ic.mutate(T_DBASE, 5, 1 + rng.below(400));
                }
                2 => {
                    ic = ic.mutate(T_LEXTRA, 3, 0);
                    ic = ic.mutate(T_LBASE, 3, 3);
                    ic = ic.mutate(T_DEXTRA, 5, 0);
                    ic = ic.mutate(T_DBASE, 5, 1);
                }
                _ => {
                    ic = ic.mutate(T_LBASE, 3, rng.next_u32());
                    ic = ic.mutate(T_DBASE, 5, 1 + rng.below(500));
                }
            }
            v.push(ic.encode());
        }
        check_group(rep, "cfg35_36_mutate_len_dist_tables", &v);
    }
}

// ===========================================================================
// Phase C - error rows 1..6 (cp_error_reason + return 0)
// ===========================================================================

fn phase_c_errors(rep: &mut Report) {
    // ERRORS row 1 : LEN != ~NLEN
    {
        let mut v = Vec::new();
        for (len, nlen) in [(4u16, 0u16), (0, 0), (5, !6u16), (0xFFFF, 0xFFFF), (3, !2u16)] {
            let mut s = vec![0x01u8];
            s.extend_from_slice(&len.to_le_bytes());
            s.extend_from_slice(&nlen.to_le_bytes());
            s.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
            v.push(InflateCase::new(s).out(64, 256).encode());
        }
        check_group_expect_error(rep, "err_len_nlen_mismatch", &v, "Failed to find LEN and NLEN as complements within stored (uncompressed) stream.");
    }

    // ERRORS row 2 : stored block shorter than the remaining input
    {
        let mut v = Vec::new();
        for (len, trail) in [(1usize, 1usize), (2, 5), (0, 1), (4, 16), (8, 3)] {
            let payload: Vec<u8> = (0..len).map(|i| pat(i + 3)).collect();
            let mut s = stored_bytes(&payload);
            for k in 0..trail {
                s.push(pat(k + 91));
            }
            v.push(InflateCase::new(s).out(256, 512).encode());
        }
        check_group_expect_error(rep, "err_stored_beyond_end", &v, "Stored block extends beyond end of input stream.");
    }

    // ERRORS row 3 : out buffer full while writing a literal
    {
        let mut v = Vec::new();
        for (nlit, ob) in [(4usize, 3i32), (1, 0), (10, 9), (300, 128)] {
            let toks: Vec<Tok> = (0..nlit).map(|i| Tok::Lit(i as u8)).collect();
            let s = fixed_stream(true, &toks);
            v.push(InflateCase::new(s).out(ob, 512).encode());
        }
        check_group_expect_error(rep, "err_out_full_symbol", &v, "Attempted to overwrite out buffer while outputting a symbol.");
    }
    // out_bytes == 0 / negative
    {
        let toks: Vec<Tok> = (0..4).map(|i| Tok::Lit(i as u8)).collect();
        let s = fixed_stream(true, &toks);
        check_group_expect_error(
            rep,
            "err_out_bytes_zero",
            &[InflateCase::new(s.clone()).out(0, 256).encode()],
            "Attempted to overwrite out buffer while outputting a symbol.",
        );
        check_group_expect_error(
            rep,
            "err_out_bytes_negative",
            &[
                InflateCase::new(s.clone()).out(-1, 256).encode(),
                InflateCase::new(s.clone()).out(-1000, 256).encode(),
                InflateCase::new(s).out(i32::MIN, 256).encode(),
            ],
            "Attempted to overwrite out buffer while outputting a symbol.",
        );
    }

    // ERRORS row 4 : backwards distance reaches before the start of `out`
    {
        let mut v = Vec::new();
        for (nlit, dist) in [(1usize, 2u32), (3, 4), (3, 5), (10, 11), (1, 32768)] {
            let mut toks: Vec<Tok> = (0..nlit).map(|i| Tok::Lit(i as u8)).collect();
            let (ds, de) = distsym_for(dist);
            toks.push(Tok::Match { lsym: 257, lextra: 0, dsym: ds, dextra: de });
            let s = fixed_stream(true, &toks);
            v.push(InflateCase::new(s).out(4096, 8192).encode());
        }
        check_group_expect_error(rep, "err_backdist_before_begin", &v, "Attempted to write before out buffer (invalid backwards distance).");
    }

    // ERRORS row 5 : match longer than the room left in `out`
    {
        let mut v = Vec::new();
        for (nlit, len, ob) in [(4usize, 258u32, 8i32), (4, 3, 5), (8, 100, 20), (4, 4, 7)] {
            let mut toks: Vec<Tok> = (0..nlit).map(|i| Tok::Lit(i as u8)).collect();
            let (ls, le) = lensym_for(len);
            toks.push(Tok::Match { lsym: ls, lextra: le, dsym: 0, dextra: 0 });
            let s = fixed_stream(true, &toks);
            v.push(InflateCase::new(s).out(ob, 4096).encode());
        }
        check_group_expect_error(rep, "err_match_past_out_end", &v, "Attempted to overwrite out buffer while outputting a string.");
    }

    // ERRORS row 6 : btype == 3
    {
        let mut v = Vec::new();
        for bfinal in [0u32, 1] {
            for align in 0..4u8 {
                let mut w = BitW::new();
                w.bit(bfinal);
                w.bits(3, 2);
                let s = finish(w, 12);
                v.push(InflateCase::new(s).align(align).out(64, 256).encode());
            }
        }
        // btype 3 as the *second* block
        {
            let toks: Vec<Tok> = (0..5).map(|i| Tok::Lit(i as u8)).collect();
            let mut w = BitW::new();
            emit_fixed_block(&mut w, false, &toks);
            w.bit(1);
            w.bits(3, 2);
            let s = finish(w, 12);
            v.push(InflateCase::new(s).out(64, 256).encode());
        }
        check_group_expect_error(rep, "err_btype3", &v, "Detected unknown block type within input stream.");
    }
}

// ===========================================================================
// Phase C - assertion rows (both libraries must die with SIGABRT)
// ===========================================================================

const SIGABRT: i32 = 6;

fn phase_c_aborts(rep: &mut Report) {
    // ERRORS row 16 / G1 / G2 : bits_left <= 0
    check_group_expect_signal(
        rep,
        "abort_null_in_zero_len",
        &[InflateCase::new(vec![]).in_bytes(0).out(0, 64).nulls(true, true).encode()],
        SIGABRT,
    );
    check_group_expect_signal(
        rep,
        "abort_in_bytes_zero",
        &[InflateCase::new(vec![1, 2, 3, 4, 5, 6, 7, 8]).in_bytes(0).out(64, 256).encode()],
        SIGABRT,
    );
    for (name, n, sig) in [
        ("abort_in_bytes_negative_1", -1i32, SIGABRT),
        ("abort_in_bytes_negative_2", -2i32, SIGABRT),
        ("abort_in_bytes_negative_3", -3i32, SIGABRT),
        ("abort_in_bytes_negative_4", -4i32, SIGABRT),
        // in_bytes = -12345 makes the final-word fold-in loop read
        // in[-12348 .. -12346]; both libraries walk off the front of the arena
        // and are killed by the same signal.
        ("abort_in_bytes_negative_big", -12345, -1),
        ("abort_in_bytes_min", i32::MIN, SIGABRT),
    ] {
        check_group_expect_signal(
            rep,
            name,
            &[InflateCase::new(vec![1, 2, 3, 4, 5, 6, 7, 8]).in_bytes(n).out(64, 256).encode()],
            sig,
        );
    }
    // G6 : in_bytes * 8 overflows to exactly 0
    check_group_expect_signal(
        rep,
        "abort_in_bytes_overflow",
        &[InflateCase::new(vec![1, 2, 3, 4, 5, 6, 7, 8])
            .in_bytes(0x2000_0000)
            .out(64, 256)
            .encode()],
        SIGABRT,
    );

    // ERRORS row 16 : the stream simply runs dry (truncated literal run)
    {
        let toks: Vec<Tok> = (0..40).map(|i| Tok::Lit(i as u8)).collect();
        let full = fixed_stream(true, &toks);
        // cut it short: at some length the decoder must abort in BOTH libs.
        let mut cases = Vec::new();
        for n in 1..=full.len() {
            // cp_stored's memcpy is unchecked, so a truncated prefix that happens
            // to look like a stored block can write up to 65535 bytes at `out`;
            // the arena has to be big enough for BOTH libraries to do that
            // without corrupting their own heap.
            cases.push(InflateCase::new(full[..n].to_vec()).out(256, 70000).encode());
        }
        check_group(rep, "abort_stream_exhausted", &cases);
    }

    // ERRORS row 18 : !cp_would_overflow(s, num_bits_to_read)
    //
    // 9 bytes, aligned.  Both 32-bit words are folded in (F = 64 bits) and
    // exactly 64 bits are consumed by the time cp_stored performs its *second*
    // 16-bit read, leaving bits_left = 8 and count = 0:
    //   (8 + 0) - 16 = -8 < 0  ->  assert `!cp_would_overflow(...)` fails.
    // stream: bfinal=0 btype=fixed, lit(sym 0, 8b), lit(sym 144, 9b),
    //         match(len sym 257, dist sym 0), EOB, bfinal=1 btype=stored
    check_group_expect_signal_msg(
        rep,
        "abort_would_overflow",
        &[InflateCase::new(vec![0x62, 0x98, 0x00, 0x04, 0x80, 0x00, 0x00, 0x00, 0x00])
            .out(64, 256)
            .encode()],
        SIGABRT,
        Some("cp_would_overflow"),
    );

    // extreme in_bytes values crossed with every input alignment: the
    // `in_bytes - first_bytes` / `in_bytes * 8` arithmetic overflows in C (which
    // wraps), so the Rust translation must wrap too instead of trapping.
    {
        let mut v = Vec::new();
        for align in 0..4u8 {
            for n in [0i32, 1, 2, 3, 0x2000_0000, 0x4000_0000, 0x6000_0000] {
                v.push(
                    InflateCase::new(vec![0x62, 0x98, 0x00, 0x04, 0x80, 0, 0, 0, 0])
                        .align(align)
                        .in_bytes(n)
                        .out(64, 70000)
                        .encode(),
                );
            }
        }
        check_group(rep, "abort_in_bytes_extremes", &v);
    }
    {
        // i32::MIN with every alignment.  `in_bytes - first_bytes` underflows in
        // C (wrapping to a huge positive), so for align != 0 the final-word
        // fold-in loop indexes `in[in_bytes - last_bytes + i]` miles outside the
        // arena and both libraries die from the *same* signal (SIGSEGV).  For
        // align == 0 the assert `s->bits_left > 0` fires first (SIGABRT).
        // Rust must therefore wrap exactly like C rather than trapping on the
        // overflow, which a debug-profile `-` would do.
        for align in 0..4u8 {
            check_group_expect_signal(
                rep,
                &format!("abort_in_bytes_min_align{align}"),
                &[InflateCase::new(vec![0x62, 0x98, 0x00, 0x04, 0x80, 0, 0, 0, 0])
                    .align(align)
                    .in_bytes(i32::MIN)
                    .out(64, 70000)
                    .encode()],
                if align == 0 { SIGABRT } else { -1 },
            );
        }
    }

    // ERRORS row 11 : cp_ptr assert (bits_left not byte aligned)
    check_group_expect_signal(
        rep,
        "abort_cp_ptr_misaligned",
        &[InflateCase::new(vec![0x62, 0x60, 0x00, 0xE4, 0xFF, 0x1F, 0x00])
            .out(64, 256)
            .encode()],
        SIGABRT,
    );

    // ERRORS row 14 : num_bits_to_read > 32 via the exported tables
    check_group_expect_signal(
        rep,
        "abort_read_bits_gt32_len",
        &[InflateCase::new(vec![0x03, 0x02, 0, 0, 0, 0, 0, 0])
            .out(64, 256)
            .mutate(T_LEXTRA, 0, 33)
            .encode()],
        SIGABRT,
    );
    check_group_expect_signal(
        rep,
        "abort_read_bits_gt32_len_255",
        &[InflateCase::new(vec![0x03, 0x02, 0, 0, 0, 0, 0, 0])
            .out(64, 256)
            .mutate(T_LEXTRA, 0, 255)
            .encode()],
        SIGABRT,
    );
    check_group_expect_signal(
        rep,
        "abort_read_bits_gt32_dist",
        &[InflateCase::new(vec![0x03, 0x02, 0, 0, 0, 0, 0, 0])
            .out(64, 256)
            .mutate(T_DEXTRA, 0, 40)
            .encode()],
        SIGABRT,
    );

    // ERRORS row 19 : cp_build assert(len < 16) via cp_fixed_table
    for val in [16u32, 17, 20, 31, 32, 40, 47, 63, 100, 255] {
        check_group_expect_signal(
            rep,
            &format!("abort_build_len_ge16_{val}"),
            &[InflateCase::new(vec![0x03, 0x02, 0, 0, 0, 0, 0, 0])
                .out(64, 256)
                .mutate(T_FIXED, 0, val)
                .encode()],
            SIGABRT,
        );
    }

    // ERRORS row 20 : cp_decode found no matching code
    check_group_expect_signal(
        rep,
        "abort_decode_no_match",
        &[InflateCase::new(vec![0x05, 0x00, 0x02, 0x20, 0, 0, 0, 0])
            .out(64, 256)
            .encode()],
        SIGABRT,
    );
    // hclen == 4 -> the code-length tree is empty -> tree[-1] / 32-bit shift
    {
        let mut w = BitW::new();
        w.bit(1);
        w.bits(2, 2);
        w.bits(0, 5);
        w.bits(0, 5);
        w.bits(0, 4); // hclen = 4
        for _ in 0..4 {
            w.bits(0, 3);
        }
        let s = finish(w, 12);
        check_group_expect_signal(
            rep,
            "abort_dynamic_empty_cl_tree",
            &[InflateCase::new(s).out(64, 256).encode()],
            SIGABRT,
        );
    }
}

// ===========================================================================
// rows 37..39 : property fuzzing
// ===========================================================================

fn valid_stream(rng: &mut Rng) -> (Vec<u8>, usize) {
    let nblocks = 1 + rng.below(3) as usize;
    let mut w = BitW::new();
    let mut total = 0usize;
    for b in 0..nblocks {
        let last = b + 1 == nblocks;
        let toks = random_tokens(rng, 400);
        total += simulate_ext(&toks).unwrap().len();
        if rng.bool() {
            emit_fixed_block(&mut w, last, &toks);
        } else {
            let t = dyn_trees_for(&toks, 288, 32, 19, rng);
            emit_dynamic_block(&mut w, last, &t, &toks, rng.bool());
        }
    }
    (finish(w, 12), total)
}

fn fuzz_valid(rep: &mut Report) {
    // row 37 : valid streams, random block splitting / btype / token mix
    {
        let n: usize = std::env::var("DIFF_FUZZ_VALID")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(2000);
        let mut v = Vec::new();
        for c in 0..n as u64 {
            let mut rng = Rng::new(0xF000_0000 + c);
            let (s, total) = valid_stream(&mut rng);
            let align = rng.below(4) as u8;
            let slack = rng.below(8) as usize;
            v.push(
                InflateCase::new(s)
                    .align(align)
                    .out((total + slack) as i32, (total + slack + 256) as u32)
                    .encode(),
            );
        }
        check_group(rep, "fuzz37_valid_streams", &v);
    }
}

fn fuzz_mutated(rep: &mut Report) {
    // row 38 : bit-level mutations of valid streams
    {
        let n: usize = std::env::var("DIFF_FUZZ_MUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(600);
        let mut v = Vec::new();
        for c in 0..n as u64 {
            let mut rng = Rng::new(0xF100_0000 + c);
            let (mut s, total) = valid_stream(&mut rng);
            let flips = 1 + rng.below(3);
            for _ in 0..flips {
                let bit = rng.below((s.len() * 8) as u32) as usize;
                s[bit / 8] ^= 1 << (bit % 8);
            }
            v.push(
                InflateCase::new(s)
                    .align(rng.below(4) as u8)
                    .out((total + 64) as i32, (total + 4096 + 65536) as u32)
                    .encode(),
            );
        }
        check_group(rep, "fuzz38_mutated_streams", &v);
    }
}

fn fuzz_random(rep: &mut Report) {
    // row 39 : completely random input bytes
    {
        let n: usize = std::env::var("DIFF_FUZZ_RAND")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(800);
        let mut v = Vec::new();
        for c in 0..n as u64 {
            let mut rng = Rng::new(0xF200_0000 + c);
            let len = 1 + rng.below(64) as usize;
            let s: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
            v.push(
                InflateCase::new(s)
                    .align(rng.below(4) as u8)
                    .out(4096, 4096 + 65536 + 4096)
                    .encode(),
            );
        }
        check_group(rep, "fuzz39_random_bytes", &v);
    }
}

// ===========================================================================
// diagnostic probe: which C assertion does each input trip?  (DIFF_PROBE=1)
// Used to *find* inputs for the rarer ERRORS.md assertion rows.
// ===========================================================================

fn probe() {
    let cl = c_lib_path();
    let mut tally: std::collections::BTreeMap<String, (usize, String)> =
        std::collections::BTreeMap::new();
    let n: usize = std::env::var("DIFF_PROBE").ok().and_then(|s| s.parse().ok()).unwrap_or(400);

    let mut inputs: Vec<(Vec<u8>, u8)> = Vec::new();
    // structured family: fixed block with k literals, then a stored block
    for nlit in 0..14usize {
        for pad in 0..24usize {
            for align in 0..4u8 {
                let mut w = BitW::new();
                let toks: Vec<Tok> = (0..nlit).map(|i| Tok::Lit(i as u8)).collect();
                emit_fixed_block(&mut w, false, &toks);
                w.bit(1);
                w.bits(0, 2);
                let mut s = w.bytes.clone();
                for k in 0..pad {
                    s.push(pat(k + 5));
                }
                inputs.push((s, align));
            }
        }
    }
    // random family
    for c in 0..n as u64 {
        let mut rng = Rng::new(0xABCD_0000 + c);
        let len = 1 + rng.below(24) as usize;
        inputs.push(((0..len).map(|_| rng.byte()).collect(), rng.below(4) as u8));
    }

    for (data, align) in inputs {
        let case = InflateCase::new(data.clone()).align(align).out(4096, 4096 + 65536).encode();
        let spec = encode_spec(&[case]);
        let c = run_child(&cl, &spec, "probe");
        let recs = split_records(&c.raw);
        let key = if recs.len() == 1 && recs[0].1 == 0 {
            let (ret, reason) = decode_inflate(&recs[0].2);
            format!(
                "ret {ret} / {}",
                reason.map(|v| String::from_utf8_lossy(&v).to_string()).unwrap_or("-".into())
            )
        } else {
            let line = c.stderr.trim().rsplit('/').next().unwrap_or("").to_string();
            let st = recs.first().map(|r| status_str(r.1)).unwrap_or(c.status.clone());
            format!("{st} :: {line}")
        };
        let e = tally.entry(key).or_insert((0, String::new()));
        e.0 += 1;
        if e.1.is_empty() {
            e.1 = format!("align={align} data={}", hexdump(&data));
        }
    }
    for (k, (cnt, ex)) in tally {
        println!("{cnt:6}  {k}\n           e.g. {ex}");
    }
}

// ===========================================================================
// cp_dynamic stack-frame overshoot (deterministic regressions)
//
// A code-18 run can add up to 138 entries while the loop bound is at most 320,
// so the C code writes past the end of `uint8_t lens[288+32]` and over its own
// locals.  These three cases pin down the three regimes; see the frame map in
// src/lib.rs.
// ===========================================================================

/// dynamic block with 319 explicit code lengths followed by one code-18 run of
/// `11 + run_extra` zeros, so `n` ends at `319 + 11 + run_extra`
fn dynamic_overshoot_stream(run_extra: u32) -> Vec<u8> {
    // code-length alphabet: symbol 0 -> 1 bit, symbols 1 and 18 -> 2 bits
    let mut cl_lens = [0u8; 19];
    cl_lens[0] = 1;
    cl_lens[1] = 2;
    cl_lens[18] = 2;
    let cl = canonical(&cl_lens);

    let mut w = BitW::new();
    w.bit(1); // bfinal
    w.bits(2, 2); // btype = dynamic
    w.bits((288 - 257) as u32, 5); // HLIT  -> nlit = 288
    w.bits((32 - 1) as u32, 5); // HDIST -> ndst = 32
    w.bits((19 - 4) as u32, 4); // HCLEN -> nlen = 19
    for k in 0..19 {
        w.bits(cl_lens[PERM[k]] as u32, 3);
    }
    // lens[0] = 1, lens[1..256] = 0, lens[256] = 1, lens[257..319] = 0
    w.code(cl[1], 2);
    for _ in 1..256 {
        w.code(cl[0], 1);
    }
    w.code(cl[1], 2);
    for _ in 257..319 {
        w.code(cl[0], 1);
    }
    // the overshooting run
    w.code(cl[18], 2);
    w.bits(run_extra, 7);
    // payload for the (2-symbol, 1-bit) literal tree: 0,0,0 then EOB
    let lit_lens = {
        let mut l = vec![0u8; 288];
        l[0] = 1;
        l[256] = 1;
        l
    };
    let lc = canonical(&lit_lens);
    for _ in 0..3 {
        w.code(lc[0], 1);
    }
    w.code(lc[256], 1);
    finish(w, 16)
}

fn phase_c_overshoot(rep: &mut Report) {
    // (a) run = 11  -> n = 330: only lens[320..329] == lenlens[0..9] is hit, and
    //     lenlens has already been consumed, so the block decodes normally.
    check_group(
        rep,
        "ovs_a_into_lenlens",
        &[InflateCase::new(dynamic_overshoot_stream(0)).out(64, 70000).encode()],
    );
    // runs that stay below lens[364] still terminate but zero `sym`, `nlen`,
    // `ndst` and `nlit`, so the outer loop bound collapses to 0.
    {
        let mut v = Vec::new();
        for run_extra in [1u32, 5, 10, 17, 18, 19, 20, 25, 30, 33, 34] {
            v.push(InflateCase::new(dynamic_overshoot_stream(run_extra)).out(64, 70000).encode());
        }
        check_group(rep, "ovs_b_into_locals", &v);
    }
    // (c) long runs reach lens[376..379], which IS the storage of `n`; writing a
    //     zero there snaps n back to 256 and the inner run loop never finishes.
    //     Both libraries must therefore hit the per-case SIGALRM.
    {
        let mut v = Vec::new();
        for run_extra in [60u32, 100, 127] {
            v.push(InflateCase::new(dynamic_overshoot_stream(run_extra)).out(64, 70000).encode());
        }
        check_group_expect_signal(rep, "ovs_c_nonterminating", &v, 14);
    }
    // the whole transition band, compared case by case
    {
        let mut v = Vec::new();
        for run_extra in [35u32, 39, 44, 50, 55, 59] {
            v.push(InflateCase::new(dynamic_overshoot_stream(run_extra)).out(64, 70000).encode());
        }
        check_group(rep, "ovs_d_transition_band", &v);
    }
}
