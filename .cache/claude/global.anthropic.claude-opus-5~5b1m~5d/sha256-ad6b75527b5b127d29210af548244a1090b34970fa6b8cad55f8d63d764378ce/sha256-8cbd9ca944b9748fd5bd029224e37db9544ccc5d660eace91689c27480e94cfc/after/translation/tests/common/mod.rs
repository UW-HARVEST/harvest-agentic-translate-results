//! Shared differential-test infrastructure.
//!
//! Both the C library and the Rust library are loaded with `libloading` and
//! driven **only** through their exported symbols, so the `#[no_mangle]`
//! wrappers are part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

pub fn c_so_path() -> PathBuf {
    let dir = repo_root().join("c_src").join("build");
    let mut best: Option<PathBuf> = None;
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().unwrap().to_string_lossy().to_string();
            if name.starts_with("lib") && name.ends_with(".so") {
                best = Some(p);
            }
        }
    }
    best.unwrap_or_else(|| {
        panic!(
            "C shared library not found in {}.\n\
             Build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            dir.display()
        )
    })
}

pub fn rust_so_path() -> PathBuf {
    // `PINFLATE_RUST_SO` lets the suite be pointed at a differently-built copy
    // of the crate's cdylib -- notably the *debug* build, which enables Rust's
    // integer-overflow checks and `debug_assert!`s and is therefore a genuinely
    // different code path from the shipped release `.so`.
    if let Ok(p) = std::env::var("PINFLATE_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "PINFLATE_RUST_SO does not exist: {}", p.display());
        return p;
    }
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join("libpinflate_lib.so");
    if !p.exists() {
        panic!(
            "Rust shared library not found at {}.\n\
             Build it with: cargo build --release",
            p.display()
        );
    }
    p
}

// ---------------------------------------------------------------------------
// The loaded library surface (all 8 exported symbols)
// ---------------------------------------------------------------------------

pub type PinflateFn = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int;

pub struct Lib {
    _lib: Library,
    pub tag: &'static str,
    pub pinflate: PinflateFn,
    pub error_reason: *mut *const c_char,
    pub fixed_table: *mut u8,
    pub permutation_order: *mut u8,
    pub len_extra_bits: *mut u8,
    pub len_base: *mut u32,
    pub dist_extra_bits: *mut u8,
    pub dist_base: *mut u32,
}

unsafe fn data_sym(lib: &Library, name: &[u8]) -> *mut c_void {
    let s: Symbol<*mut c_void> = lib
        .get(name)
        .unwrap_or_else(|e| panic!("missing symbol {}: {e}", String::from_utf8_lossy(name)));
    s.try_as_raw_ptr().expect("no raw pointer for symbol")
}

impl Lib {
    pub unsafe fn open(path: &Path, tag: &'static str) -> Lib {
        let lib = Library::new(path).unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()));
        let pinflate: PinflateFn = {
            let s: Symbol<PinflateFn> = lib.get(b"pinflate\0").expect("missing `pinflate`");
            *s
        };
        let error_reason = data_sym(&lib, b"cp_error_reason\0") as *mut *const c_char;
        let fixed_table = data_sym(&lib, b"cp_fixed_table\0") as *mut u8;
        let permutation_order = data_sym(&lib, b"cp_permutation_order\0") as *mut u8;
        let len_extra_bits = data_sym(&lib, b"cp_len_extra_bits\0") as *mut u8;
        let len_base = data_sym(&lib, b"cp_len_base\0") as *mut u32;
        let dist_extra_bits = data_sym(&lib, b"cp_dist_extra_bits\0") as *mut u8;
        let dist_base = data_sym(&lib, b"cp_dist_base\0") as *mut u32;
        Lib {
            _lib: lib,
            tag,
            pinflate,
            error_reason,
            fixed_table,
            permutation_order,
            len_extra_bits,
            len_base,
            dist_extra_bits,
            dist_base,
        }
    }

    /// Snapshot of the seven exported tables, so a test can prove that both
    /// libraries expose the same initial data and that a call does not mutate
    /// them.
    pub unsafe fn tables(&self) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(std::slice::from_raw_parts(self.fixed_table, 288 + 32));
        v.extend_from_slice(std::slice::from_raw_parts(self.permutation_order, 19));
        v.extend_from_slice(std::slice::from_raw_parts(self.len_extra_bits, 29 + 2));
        for i in 0..(29 + 2) {
            v.extend_from_slice(&(*self.len_base.add(i)).to_le_bytes());
        }
        v.extend_from_slice(std::slice::from_raw_parts(self.dist_extra_bits, 30 + 2));
        for i in 0..(30 + 2) {
            v.extend_from_slice(&(*self.dist_base.add(i)).to_le_bytes());
        }
        v
    }

    pub unsafe fn reset_tables(&self) {
        let pristine = pristine_tables();
        std::ptr::copy_nonoverlapping(pristine.fixed_table.as_ptr(), self.fixed_table, 320);
        std::ptr::copy_nonoverlapping(
            pristine.permutation_order.as_ptr(),
            self.permutation_order,
            19,
        );
        std::ptr::copy_nonoverlapping(pristine.len_extra_bits.as_ptr(), self.len_extra_bits, 31);
        std::ptr::copy_nonoverlapping(pristine.len_base.as_ptr(), self.len_base, 31);
        std::ptr::copy_nonoverlapping(pristine.dist_extra_bits.as_ptr(), self.dist_extra_bits, 32);
        std::ptr::copy_nonoverlapping(pristine.dist_base.as_ptr(), self.dist_base, 32);
        *self.error_reason = std::ptr::null();
    }
}

pub struct Pristine {
    pub fixed_table: [u8; 320],
    pub permutation_order: [u8; 19],
    pub len_extra_bits: [u8; 31],
    pub len_base: [u32; 31],
    pub dist_extra_bits: [u8; 32],
    pub dist_base: [u32; 32],
}

pub fn pristine_tables() -> Pristine {
    let mut fixed_table = [0u8; 320];
    for (i, e) in fixed_table.iter_mut().enumerate() {
        *e = match i {
            0..=143 => 8,
            144..=255 => 9,
            256..=279 => 7,
            280..=287 => 8,
            _ => 5,
        };
    }
    Pristine {
        fixed_table,
        permutation_order: [
            16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
        ],
        len_extra_bits: [
            0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
            0, 0,
        ],
        len_base: [
            3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99,
            115, 131, 163, 195, 227, 258, 0, 0,
        ],
        dist_extra_bits: [
            0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12,
            12, 13, 13, 0, 0,
        ],
        dist_base: [
            1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025,
            1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
        ],
    }
}

// ---------------------------------------------------------------------------
// The pair of libraries (loaded once per process)
// ---------------------------------------------------------------------------

pub struct Pair {
    pub c: Lib,
    pub rs: Lib,
}

pub fn pair() -> &'static Pair {
    use std::sync::OnceLock;
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(|| unsafe {
        Pair {
            c: Lib::open(&c_so_path(), "C"),
            rs: Lib::open(&rust_so_path(), "Rust"),
        }
    })
}

// `Lib` holds raw pointers; the tests are single-threaded per case but the test
// harness runs cases on several threads, so the `OnceLock` needs these.
unsafe impl Send for Lib {}
unsafe impl Sync for Lib {}

/// Everything the tests compare.  Because the two libraries are separate
/// `.so`s their globals live at different addresses, so `reason` is compared as
/// the *string contents*, never as a pointer.
#[derive(PartialEq, Eq, Clone)]
pub struct Outcome {
    pub ret: c_int,
    pub out: Vec<u8>,
    pub reason: Option<Vec<u8>>,
    pub tables: Vec<u8>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Outcome {{ ret: {}, out: {} bytes {:02x?}{}, reason: {:?} }}",
            self.ret,
            self.out.len(),
            &self.out[..self.out.len().min(48)],
            if self.out.len() > 48 { " …" } else { "" },
            self.reason.as_ref().map(|r| String::from_utf8_lossy(r).to_string())
        )
    }
}

impl Outcome {
    pub fn digest(&self) -> String {
        format!(
            "ret={} out={:016x}/{} reason={:?}",
            self.ret,
            fnv(&self.out),
            self.out.len(),
            self.reason.as_ref().map(|r| String::from_utf8_lossy(r).to_string())
        )
    }
}

pub fn fnv(b: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &x in b {
        h ^= x as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}

// ---------------------------------------------------------------------------
// Buffers with controlled alignment
// ---------------------------------------------------------------------------

/// Slack after the input.  `cp_stored` does `memcpy(out, p, LEN)` where `LEN`
/// may legally exceed the bytes actually remaining in `in` (the C only checks
/// `bits_left / 8 <= LEN`), so both libraries read past the end of the input.
/// The tests give them a large, deterministically zeroed run-off area so the
/// read is well defined and, crucially, *identical* for both.
pub const IN_SLACK: usize = 70_000;

pub struct AlignedIn {
    buf: Vec<u32>,
    off: usize,
}

impl AlignedIn {
    /// `data` placed at `align` bytes past a 4-aligned address, followed by
    /// `IN_SLACK` zero bytes.
    pub fn new(data: &[u8], align: usize) -> AlignedIn {
        assert!(align < 4);
        let total = align + data.len() + IN_SLACK;
        let mut buf = vec![0u32; total / 4 + 2];
        unsafe {
            let base = buf.as_mut_ptr() as *mut u8;
            std::ptr::copy_nonoverlapping(data.as_ptr(), base.add(align), data.len());
        }
        AlignedIn { buf, off: align }
    }
    pub fn ptr(&mut self) -> *mut c_void {
        unsafe { (self.buf.as_mut_ptr() as *mut u8).add(self.off) as *mut c_void }
    }
}

// ---------------------------------------------------------------------------
// Calling one library
// ---------------------------------------------------------------------------

pub const OUT_FILL: u8 = 0xA5;

/// Slack **after** the output buffer.
///
/// `cp_stored` does `memcpy(s->out, p, LEN)` with **no** `out_end` check at all
/// (the only guard is `bits_left / 8 <= LEN`), so a stored block whose `LEN` is
/// larger than `out_bytes` writes up to 65535 bytes past the end of the caller's
/// buffer.  That is unchecked out-of-bounds writing in the C, and letting it
/// land on the test process's heap only produces allocator corruption at a
/// random later point (`munmap_chunk(): invalid pointer`, `realloc(): invalid
/// old size`) whose timing differs between the two runs and says nothing about
/// the translation.  Giving the buffer 64 KiB + change of slack keeps the
/// overrun inside our own allocation, so the bytes it writes are *compared*
/// instead of corrupting the harness.
pub const OUT_SLACK: usize = 65_536 + 64;

pub unsafe fn call(l: &Lib, data: &[u8], align: usize, in_bytes: c_int, out_len: usize) -> Outcome {
    let mut inbuf = AlignedIn::new(data, align);
    let mut out = vec![OUT_FILL; out_len + OUT_SLACK];
    *l.error_reason = std::ptr::null();
    let ret = (l.pinflate)(
        inbuf.ptr(),
        in_bytes,
        out.as_mut_ptr() as *mut c_void,
        out_len as c_int,
    );
    let reason = read_cstr(*l.error_reason);
    Outcome { ret, out, reason, tables: l.tables() }
}

/// Same as [`call`] but lets the caller pass an arbitrary `out_bytes` (possibly
/// negative or larger than the allocation) while still allocating `alloc` bytes.
pub unsafe fn call_raw(
    l: &Lib,
    data: &[u8],
    align: usize,
    in_bytes: c_int,
    out_ptr_null: bool,
    alloc: usize,
    out_bytes: c_int,
) -> Outcome {
    let mut inbuf = AlignedIn::new(data, align);
    let mut out = vec![OUT_FILL; alloc + OUT_SLACK];
    *l.error_reason = std::ptr::null();
    let op = if out_ptr_null {
        std::ptr::null_mut()
    } else {
        out.as_mut_ptr() as *mut c_void
    };
    let ret = (l.pinflate)(inbuf.ptr(), in_bytes, op, out_bytes);
    let reason = read_cstr(*l.error_reason);
    Outcome { ret, out, reason, tables: l.tables() }
}

pub unsafe fn read_cstr(p: *const c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        return None;
    }
    let mut v = Vec::new();
    let mut i = 0isize;
    while i < 4096 {
        let b = *(p.offset(i) as *const u8);
        if b == 0 {
            break;
        }
        v.push(b);
        i += 1;
    }
    Some(v)
}

/// The exported globals are process-wide mutable state, and the libtest
/// harness runs test functions on several threads, so every call into either
/// library is serialised.
pub fn lock() -> std::sync::MutexGuard<'static, ()> {
    use std::sync::{Mutex, OnceLock};
    static M: OnceLock<Mutex<()>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Run one case against both libraries and assert byte-identical results.
pub fn diff(label: &str, data: &[u8], align: usize, out_len: usize) {
    let p = pair();
    let _g = lock();
    unsafe {
        p.c.reset_tables();
        p.rs.reset_tables();
        let a = call(&p.c, data, align, data.len() as c_int, out_len);
        let b = call(&p.rs, data, align, data.len() as c_int, out_len);
        assert_eq(label, data, align, out_len, &a, &b);
    }
}

pub fn assert_eq(
    label: &str,
    data: &[u8],
    align: usize,
    out_len: usize,
    a: &Outcome,
    b: &Outcome,
) {
    if a.ret != b.ret || a.out != b.out || a.reason != b.reason {
        let first_diff = a
            .out
            .iter()
            .zip(b.out.iter())
            .position(|(x, y)| x != y)
            .map(|i| i.to_string())
            .unwrap_or_else(|| "none".into());
        panic!(
            "DIVERGENCE [{label}] align={align} in_bytes={} out_bytes={out_len}\n\
             first differing out byte: {first_diff}\n\
             C   : {:?}\nRust: {:?}\n\
             input: {:02x?}",
            data.len(),
            a,
            b,
            &data[..data.len().min(96)]
        );
    }
    assert!(
        a.tables == b.tables,
        "DIVERGENCE [{label}]: exported tables differ after the call"
    );
}

/// Cross a case with all 4 input alignments and 4 trailing-length classes
/// (`in_bytes % 4`).  Trailing bytes are appended *after* the stream; the
/// decoder must not care, because `BFINAL` already stopped it.
pub fn diff_all_alignments(label: &str, data: &[u8], out_len: usize) {
    for align in 0..4usize {
        diff(&format!("{label}/align{align}"), data, align, out_len);
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*), so every "randomized" run is reproducible
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
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
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn range(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below(hi_inclusive - lo + 1)
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 40) as u8
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.byte()).collect()
    }
    /// Highly repetitive data, so that a real encoder emits long matches.
    pub fn repetitive(&mut self, n: usize) -> Vec<u8> {
        let period = self.range(1, 32);
        let seed: Vec<u8> = (0..period).map(|_| self.byte()).collect();
        let mut v = Vec::with_capacity(n);
        while v.len() < n {
            v.push(seed[v.len() % period]);
        }
        v
    }
}

// ---------------------------------------------------------------------------
// A minimal DEFLATE *writer*, so the tests can aim at individual C branches
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct BitWriter {
    pub bytes: Vec<u8>,
    acc: u32,
    nbits: u32,
}

impl BitWriter {
    pub fn new() -> BitWriter {
        BitWriter::default()
    }
    /// DEFLATE packs plain integer fields starting from the least significant
    /// bit.
    pub fn bits(&mut self, value: u32, n: u32) {
        assert!(n <= 24);
        if n == 0 {
            return;
        }
        let v = value & ((1u32 << n) - 1);
        self.acc |= v << self.nbits;
        self.nbits += n;
        while self.nbits >= 8 {
            self.bytes.push((self.acc & 0xFF) as u8);
            self.acc >>= 8;
            self.nbits -= 8;
        }
    }
    /// Huffman codes are packed starting from the *most* significant bit.
    pub fn huff(&mut self, code: u32, len: u32) {
        assert!(len >= 1 && len <= 15);
        for i in (0..len).rev() {
            self.bits((code >> i) & 1, 1);
        }
    }
    pub fn align(&mut self) {
        if self.nbits > 0 {
            self.bits(0, 8 - self.nbits);
        }
    }
    pub fn raw_bytes(&mut self, b: &[u8]) {
        assert_eq!(self.nbits, 0, "raw_bytes needs a byte-aligned writer");
        self.bytes.extend_from_slice(b);
    }
    pub fn finish(mut self) -> Vec<u8> {
        self.align();
        self.bytes
    }
    pub fn bit_len(&self) -> usize {
        self.bytes.len() * 8 + self.nbits as usize
    }
}

/// Canonical Huffman code assignment, exactly the one `cp_build` reconstructs.
/// Returns `(code, len)` per symbol; `len == 0` means "unused".
pub fn canonical(lens: &[u8]) -> Vec<(u32, u8)> {
    let mut counts = [0u32; 16];
    for &l in lens {
        counts[l as usize] += 1;
    }
    let mut codes = [0u32; 16];
    counts[0] = 0;
    for n in 1..16usize {
        codes[n] = (codes[n - 1] + counts[n - 1]) << 1;
    }
    let mut next = codes;
    lens.iter()
        .map(|&l| {
            if l == 0 {
                (0, 0)
            } else {
                let c = next[l as usize];
                next[l as usize] += 1;
                (c, l)
            }
        })
        .collect()
}

/// A *complete* (Kraft sum == 1) code over `symbols`, with every code the same
/// length.  `alphabet` is the size of the length array to produce.
pub fn uniform_lengths(alphabet: usize, symbols: &[usize]) -> Vec<u8> {
    let mut used: Vec<usize> = symbols.to_vec();
    used.sort_unstable();
    used.dedup();
    assert!(!used.is_empty());
    // Pad the symbol set up to a power of two with unused symbols so that the
    // resulting code is complete.
    let mut n = 1usize;
    let mut bits = 0u8;
    while n < used.len() {
        n <<= 1;
        bits += 1;
    }
    if used.len() == 1 {
        n = 2;
        bits = 1;
    }
    let mut lens = vec![0u8; alphabet];
    for &s in &used {
        lens[s] = bits;
    }
    let mut extra = 0usize;
    let mut have = used.len();
    while have < n {
        while extra < alphabet && lens[extra] != 0 {
            extra += 1;
        }
        assert!(extra < alphabet, "alphabet too small to complete the code");
        lens[extra] = bits;
        have += 1;
    }
    lens
}

/// Ordinary Huffman code-length construction from frequencies.  Always yields
/// a *complete* code (except for the degenerate one-symbol case, where DEFLATE
/// itself emits a single 1-bit code and the tree is incomplete — which is also
/// what real encoders do for `HDIST == 1`).
pub fn huffman_lengths(alphabet: usize, freqs: &[u64]) -> Vec<u8> {
    assert_eq!(freqs.len(), alphabet);
    let used: Vec<usize> = (0..alphabet).filter(|&i| freqs[i] > 0).collect();
    let mut lens = vec![0u8; alphabet];
    if used.is_empty() {
        return lens;
    }
    if used.len() == 1 {
        lens[used[0]] = 1;
        return lens;
    }
    let mut nodes: Vec<(u64, Vec<usize>)> = used.iter().map(|&s| (freqs[s], vec![s])).collect();
    while nodes.len() > 1 {
        // Deterministic tie-breaking so the produced stream is reproducible.
        nodes.sort_by(|a, b| a.0.cmp(&b.0).then(a.1[0].cmp(&b.1[0])));
        let a = nodes.remove(0);
        let b = nodes.remove(0);
        for &s in a.1.iter().chain(b.1.iter()) {
            lens[s] += 1;
        }
        let mut merged = a.1;
        merged.extend(b.1);
        nodes.push((a.0.saturating_add(b.0), merged));
    }
    assert!(lens.iter().all(|&l| l <= 15), "code length > 15");
    lens
}

/// Equal-frequency Huffman code over `symbols` (complete, near-uniform depth).
pub fn balanced_lengths(alphabet: usize, symbols: &[usize]) -> Vec<u8> {
    let mut f = vec![0u64; alphabet];
    for &s in symbols {
        f[s] = 1;
    }
    huffman_lengths(alphabet, &f)
}

/// A complete code whose longest code is `symbols.len() - 1` bits: the classic
/// "unary" shape 1,2,3,…,k,k.  With ≥ 11 symbols this produces code lengths
/// above 9, which is what makes `cp_build` skip its `lookup` fast path.
pub fn skewed_lengths(alphabet: usize, symbols: &[usize]) -> Vec<u8> {
    let mut used: Vec<usize> = symbols.to_vec();
    used.sort_unstable();
    used.dedup();
    assert!(used.len() >= 2);
    assert!(used.len() <= 16, "unary code would exceed 15 bits");
    let mut lens = vec![0u8; alphabet];
    let m = used.len();
    for (i, &s) in used.iter().enumerate() {
        lens[s] = if i + 1 == m { (m - 1) as u8 } else { (i + 1) as u8 };
    }
    lens
}

/// Encoded literal/length/distance token.
#[derive(Clone, Copy, Debug)]
pub enum Tok {
    Lit(u8),
    /// (length 3..=258, distance 1..=32768)
    Match(u32, u32),
}

pub fn len_symbol(length: u32) -> (usize, u32, u32) {
    let base = pristine_tables();
    for s in (0..29usize).rev() {
        let b = base.len_base[s];
        let e = base.len_extra_bits[s] as u32;
        if length >= b && length < b + (1 << e) {
            return (257 + s, length - b, e);
        }
    }
    panic!("bad length {length}");
}

pub fn dist_symbol(dist: u32) -> (usize, u32, u32) {
    let base = pristine_tables();
    for s in (0..30usize).rev() {
        let b = base.dist_base[s];
        let e = base.dist_extra_bits[s] as u32;
        if dist >= b && dist < b + (1 << e) {
            return (s, dist - b, e);
        }
    }
    panic!("bad distance {dist}");
}

pub const FIXED_LIT_LENS: fn() -> Vec<u8> = || pristine_tables().fixed_table[..288].to_vec();
pub const FIXED_DST_LENS: fn() -> Vec<u8> = || pristine_tables().fixed_table[288..].to_vec();

/// Append a `BTYPE == 1` (fixed Huffman) block.
pub fn write_fixed_block(w: &mut BitWriter, bfinal: bool, toks: &[Tok]) {
    write_fixed_block_with(w, bfinal, toks, &FIXED_LIT_LENS(), &FIXED_DST_LENS());
}

pub fn write_fixed_block_with(
    w: &mut BitWriter,
    bfinal: bool,
    toks: &[Tok],
    lit_lens: &[u8],
    dst_lens: &[u8],
) {
    w.bits(bfinal as u32, 1);
    w.bits(1, 2);
    write_tokens(w, toks, lit_lens, dst_lens);
}

pub fn write_tokens(w: &mut BitWriter, toks: &[Tok], lit_lens: &[u8], dst_lens: &[u8]) {
    let lit = canonical(lit_lens);
    let dst = canonical(dst_lens);
    for t in toks {
        match *t {
            Tok::Lit(b) => {
                let (c, l) = lit[b as usize];
                assert!(l > 0, "literal {b} has no code");
                w.huff(c, l as u32);
            }
            Tok::Match(length, distance) => {
                let (ls, lextra, lbits) = len_symbol(length);
                let (c, l) = lit[ls];
                assert!(l > 0, "length symbol {ls} has no code");
                w.huff(c, l as u32);
                if lbits > 0 {
                    w.bits(lextra, lbits);
                }
                let (ds, dextra, dbits) = dist_symbol(distance);
                let (c, l) = dst[ds];
                assert!(l > 0, "distance symbol {ds} has no code");
                w.huff(c, l as u32);
                if dbits > 0 {
                    w.bits(dextra, dbits);
                }
            }
        }
    }
    // end of block
    let (c, l) = lit[256];
    assert!(l > 0, "EOB has no code");
    w.huff(c, l as u32);
}

/// Code-length-alphabet item for a dynamic header.
#[derive(Clone, Copy, Debug)]
pub enum Cl {
    /// a literal code length 0..=15
    Len(u8),
    /// symbol 16: copy the previous length `3 + extra` times (extra 0..=3)
    Rep(u8),
    /// symbol 17: `3 + extra` zeros (extra 0..=7)
    Z3(u8),
    /// symbol 18: `11 + extra` zeros (extra 0..=127)
    Z11(u8),
}

impl Cl {
    pub fn symbol(self) -> usize {
        match self {
            Cl::Len(l) => l as usize,
            Cl::Rep(_) => 16,
            Cl::Z3(_) => 17,
            Cl::Z11(_) => 18,
        }
    }
    pub fn extra(self) -> Option<(u32, u32)> {
        match self {
            Cl::Len(_) => None,
            Cl::Rep(e) => Some((e as u32, 2)),
            Cl::Z3(e) => Some((e as u32, 3)),
            Cl::Z11(e) => Some((e as u32, 7)),
        }
    }
    pub fn count(self) -> usize {
        match self {
            Cl::Len(_) => 1,
            Cl::Rep(e) => 3 + e as usize,
            Cl::Z3(e) => 3 + e as usize,
            Cl::Z11(e) => 11 + e as usize,
        }
    }
}

/// Encode a flat length vector as `Cl` items with no repeat codes at all.
pub fn cl_flat(all: &[u8]) -> Vec<Cl> {
    all.iter().map(|&l| Cl::Len(l)).collect()
}

/// Append a `BTYPE == 2` (dynamic Huffman) block built from explicit pieces.
///
/// * `nlit` / `ndst`  — the `HLIT + 257` / `HDIST + 1` values in the header
/// * `cl_items`       — the code-length stream (must expand to exactly
///                      `nlit + ndst` lengths)
/// * `hclen`          — how many of the 19 code-length-code lengths to emit
/// * `cl_lens`        — the 19 code lengths of the code-length alphabet
pub fn write_dynamic_block_raw(
    w: &mut BitWriter,
    bfinal: bool,
    nlit: usize,
    ndst: usize,
    hclen: usize,
    cl_lens: &[u8; 19],
    cl_items: &[Cl],
    toks: &[Tok],
    lit_lens: &[u8],
    dst_lens: &[u8],
) {
    assert!((257..=288).contains(&nlit));
    assert!((1..=32).contains(&ndst));
    assert!((4..=19).contains(&hclen));
    w.bits(bfinal as u32, 1);
    w.bits(2, 2);
    w.bits((nlit - 257) as u32, 5);
    w.bits((ndst - 1) as u32, 5);
    w.bits((hclen - 4) as u32, 4);
    let perm = pristine_tables().permutation_order;
    for i in 0..hclen {
        w.bits(cl_lens[perm[i] as usize] as u32, 3);
    }
    let clcode = canonical(cl_lens);
    for it in cl_items {
        let (c, l) = clcode[it.symbol()];
        assert!(l > 0, "code-length symbol {} has no code", it.symbol());
        w.huff(c, l as u32);
        if let Some((e, n)) = it.extra() {
            w.bits(e, n);
        }
    }
    write_tokens(w, toks, lit_lens, dst_lens);
}

/// Choose a complete code for the code-length alphabet covering `used`.
pub fn cl_lengths_for(used: &[usize]) -> [u8; 19] {
    let v = balanced_lengths(19, used);
    let mut a = [0u8; 19];
    a.copy_from_slice(&v);
    a
}

/// Code-length-alphabet code derived from the real frequencies of `items`.
pub fn cl_lengths_from_items(items: &[Cl]) -> [u8; 19] {
    let mut f = vec![0u64; 19];
    for it in items {
        f[it.symbol()] += 1;
    }
    let v = huffman_lengths(19, &f);
    let mut a = [0u8; 19];
    a.copy_from_slice(&v);
    a
}

/// Convenience: a dynamic block with no repeat codes, minimal HCLEN coverage.
pub fn write_dynamic_block(
    w: &mut BitWriter,
    bfinal: bool,
    lit_lens: &[u8],
    dst_lens: &[u8],
    toks: &[Tok],
) {
    let mut all: Vec<u8> = lit_lens.to_vec();
    all.extend_from_slice(dst_lens);
    let items = cl_flat(&all);
    let cl_lens = cl_lengths_from_items(&items);
    write_dynamic_block_raw(
        w,
        bfinal,
        lit_lens.len(),
        dst_lens.len(),
        19,
        &cl_lens,
        &items,
        toks,
        lit_lens,
        dst_lens,
    );
}

/// Append a `BTYPE == 0` (stored) block.
pub fn write_stored_block(w: &mut BitWriter, bfinal: bool, payload: &[u8]) {
    write_stored_block_lens(w, bfinal, payload.len() as u16, !(payload.len() as u16), payload);
}

pub fn write_stored_block_lens(
    w: &mut BitWriter,
    bfinal: bool,
    len: u16,
    nlen: u16,
    payload: &[u8],
) {
    w.bits(bfinal as u32, 1);
    w.bits(0, 2);
    w.align();
    w.bits(len as u32, 16);
    w.bits(nlen as u32, 16);
    w.raw_bytes(payload);
}

/// Reference decompressor used to predict the plaintext (only for building
/// test vectors — the *assertions* are always C-vs-Rust, never against this).
pub fn inflate_ref(compressed: &[u8]) -> Option<Vec<u8>> {
    use flate2::write::DeflateDecoder;
    use std::io::Write;
    let mut d = DeflateDecoder::new(Vec::new());
    d.write_all(compressed).ok()?;
    d.finish().ok()
}

pub fn deflate_ref(data: &[u8], level: u32) -> Vec<u8> {
    use flate2::write::DeflateEncoder;
    use flate2::Compression;
    use std::io::Write;
    let mut e = DeflateEncoder::new(Vec::new(), Compression::new(level));
    e.write_all(data).unwrap();
    e.finish().unwrap()
}

// ---------------------------------------------------------------------------
// Child-process differential runner
// ---------------------------------------------------------------------------
//
// Ten of the C library's rejection paths are live `assert()`s, i.e. they call
// `__assert_fail()` and raise `SIGABRT`.  Those cannot be observed in-process,
// so every risky case is executed in a re-executed copy of this very test
// binary: one child per library, each running the same list of cases and
// appending a digest line per case to a result file.  The parent then compares
//
//   * the two result transcripts, byte for byte,
//   * the child exit statuses (including "killed by signal 6"),
//   * the children's stderr, byte for byte (the `assert()` message).
//
// Each test binary that wants this must expose the runner as a test:
//     define_child_runner!();

pub fn hex_encode(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for &x in b {
        s.push(char::from_digit((x >> 4) as u32, 16).unwrap());
        s.push(char::from_digit((x & 15) as u32, 16).unwrap());
    }
    s
}

pub fn hex_decode(s: &str) -> Vec<u8> {
    let b = s.as_bytes();
    assert!(b.len() % 2 == 0);
    (0..b.len() / 2)
        .map(|i| {
            let hi = (b[2 * i] as char).to_digit(16).unwrap() as u8;
            let lo = (b[2 * i + 1] as char).to_digit(16).unwrap() as u8;
            (hi << 4) | lo
        })
        .collect()
}

/// One differential case: input bytes, input alignment, the `in_bytes` and
/// `out_bytes` arguments, and how much output to actually allocate.
#[derive(Clone, Debug)]
pub struct Case {
    pub label: String,
    pub data: Vec<u8>,
    pub align: usize,
    pub in_bytes: i32,
    pub out_bytes: i32,
    pub out_alloc: usize,
    pub null_out: bool,
    /// index/value pairs poked into `cp_fixed_table` before the call
    pub poke_fixed: Vec<(usize, u8)>,
    /// index/value pairs poked into `cp_len_extra_bits`
    pub poke_len_extra: Vec<(usize, u8)>,
    /// index/value pairs poked into `cp_dist_extra_bits`
    pub poke_dist_extra: Vec<(usize, u8)>,
}

impl Case {
    pub fn new(label: &str, data: &[u8]) -> Case {
        Case {
            label: label.to_string(),
            data: data.to_vec(),
            align: 0,
            in_bytes: data.len() as i32,
            out_bytes: 0,
            out_alloc: 0,
            null_out: false,
            poke_fixed: Vec::new(),
            poke_len_extra: Vec::new(),
            poke_dist_extra: Vec::new(),
        }
    }
    pub fn align(mut self, a: usize) -> Case {
        self.align = a;
        self
    }
    pub fn in_bytes(mut self, n: i32) -> Case {
        self.in_bytes = n;
        self
    }
    pub fn out(mut self, n: usize) -> Case {
        self.out_bytes = n as i32;
        self.out_alloc = n;
        self
    }
    pub fn out_raw(mut self, out_bytes: i32, alloc: usize) -> Case {
        self.out_bytes = out_bytes;
        self.out_alloc = alloc;
        self
    }
    pub fn null_out(mut self) -> Case {
        self.null_out = true;
        self
    }
    pub fn poke_fixed(mut self, i: usize, v: u8) -> Case {
        self.poke_fixed.push((i, v));
        self
    }
    pub fn poke_len_extra(mut self, i: usize, v: u8) -> Case {
        self.poke_len_extra.push((i, v));
        self
    }
    pub fn poke_dist_extra(mut self, i: usize, v: u8) -> Case {
        self.poke_dist_extra.push((i, v));
        self
    }

    fn encode(&self) -> String {
        let pk = |v: &Vec<(usize, u8)>| {
            if v.is_empty() {
                ".".to_string()
            } else {
                v.iter().map(|(i, x)| format!("{i}:{x}")).collect::<Vec<_>>().join(",")
            }
        };
        format!(
            "{} {} {} {} {} {} {} {} {} {}",
            self.align,
            self.in_bytes,
            self.out_bytes,
            self.out_alloc,
            self.null_out as u8,
            pk(&self.poke_fixed),
            pk(&self.poke_len_extra),
            pk(&self.poke_dist_extra),
            self.label.replace(' ', "_"),
            hex_encode(&self.data),
        )
    }

    fn decode(line: &str) -> Case {
        let f: Vec<&str> = line.split(' ').collect();
        let pk = |s: &str| -> Vec<(usize, u8)> {
            if s == "." {
                Vec::new()
            } else {
                s.split(',')
                    .map(|kv| {
                        let mut it = kv.split(':');
                        (
                            it.next().unwrap().parse().unwrap(),
                            it.next().unwrap().parse().unwrap(),
                        )
                    })
                    .collect()
            }
        };
        Case {
            align: f[0].parse().unwrap(),
            in_bytes: f[1].parse().unwrap(),
            out_bytes: f[2].parse().unwrap(),
            out_alloc: f[3].parse().unwrap(),
            null_out: f[4] == "1",
            poke_fixed: pk(f[5]),
            poke_len_extra: pk(f[6]),
            poke_dist_extra: pk(f[7]),
            label: f[8].to_string(),
            data: hex_decode(f[9]),
        }
    }
}

/// Exit codes at or above this value mean "the grandchild caught fatal signal
/// `code - FATAL_EXIT_BASE`".
const FATAL_EXIT_BASE: libc::c_int = 100;

/// Async-signal-safe handler installed in the grandchild for every fatal
/// signal, so that a dying case is *recorded* rather than costing a core dump.
extern "C" fn fatal_handler(sig: libc::c_int) {
    unsafe { libc::_exit(FATAL_EXIT_BASE + sig) }
}

/// Runs inside the child.  Returns immediately when not driven by the parent.
///
/// Each case is executed in a `fork()`ed grandchild with `alarm(1)` armed and
/// stderr redirected to a scratch file, so a live `assert()` (SIGABRT), a
/// segfault, or an infinite loop is *recorded* instead of tearing down the
/// whole run.  The grandchild reports its digest through a shared page.
pub fn child_runner_main() {
    let Ok(libsel) = std::env::var("PINFLATE_CHILD_LIB") else {
        return;
    };
    let cases_path = std::env::var("PINFLATE_CHILD_CASES").unwrap();
    let result_path = std::env::var("PINFLATE_CHILD_RESULT").unwrap();
    let body = std::fs::read_to_string(&cases_path).unwrap();

    let lib = unsafe {
        match libsel.as_str() {
            "c" => Lib::open(&c_so_path(), "C"),
            "rust" => Lib::open(&rust_so_path(), "Rust"),
            other => panic!("bad PINFLATE_CHILD_LIB {other}"),
        }
    };

    const SH_LEN: usize = 8192;
    let sh = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            SH_LEN,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED | libc::MAP_ANONYMOUS,
            -1,
            0,
        ) as *mut u8
    };
    assert!(!sh.is_null() && sh as isize != -1);

    use std::io::{Read, Seek, Write};
    use std::os::unix::io::AsRawFd;
    let err_path = std::env::temp_dir().join(format!(
        "pinflate_stderr_{}_{}",
        std::process::id(),
        libsel
    ));
    let mut err_file = std::fs::File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&err_path)
        .unwrap();
    let err_fd = err_file.as_raw_fd();

    // Most cases in the sweeps die from `assert()` → `abort()`, and letting the
    // kernel produce a core dump for each one costs the better part of a second.
    // Disable core dumps for the runner and everything it forks.
    unsafe {
        let rl = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
        libc::setrlimit(libc::RLIMIT_CORE, &rl);
    }

    let mut f = std::fs::File::create(&result_path).unwrap();
    for (idx, line) in body.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        let c = Case::decode(line);
        unsafe {
            std::ptr::write_bytes(sh, 0, 8);
            err_file.set_len(0).unwrap();
            err_file.seek(std::io::SeekFrom::Start(0)).unwrap();
            let pid = libc::fork();
            if pid == 0 {
                // ---- grandchild ----
                // Sub-second watchdog.  `pinflate` can loop forever on some
                // corrupt dynamic-block headers (a code-length repeat code can
                // overwrite `cp_dynamic`'s own loop counter), so each case gets
                // a small time budget and a SIGALRM if it overruns.  The inputs
                // the sweeps use are a few dozen bytes, i.e. microseconds of
                // work, so the budget is orders of magnitude above what a
                // terminating case needs -- and it is identical for both
                // libraries, so a genuine divergence still shows up as
                // "one hung, the other returned".
                let ms: i64 = std::env::var("PINFLATE_CASE_MS")
                    .ok()
                    .and_then(|v| v.parse::<i64>().ok())
                    .unwrap_or(30);
                let us = ms * 1000;
                let it = libc::itimerval {
                    it_interval: libc::timeval { tv_sec: 0, tv_usec: 0 },
                    it_value: libc::timeval {
                        tv_sec: us / 1_000_000,
                        tv_usec: (us % 1_000_000) as libc::suseconds_t,
                    },
                };
                libc::setitimer(libc::ITIMER_REAL, &it, std::ptr::null_mut());
                // Turn the fatal signals into a plain exit.  Letting the
                // grandchild really die from SIGABRT costs ~200 ms per case on
                // this machine (the kernel hands the corpse to a core-dump
                // helper), which would make the sweeps unusable.  glibc's
                // `__assert_fail` has already written its message to stderr by
                // the time `abort()` raises SIGABRT, so nothing observable is
                // lost: the signal number is reported through the exit code and
                // the message through the redirected stderr.
                for sig in [
                    libc::SIGABRT,
                    libc::SIGSEGV,
                    libc::SIGBUS,
                    libc::SIGILL,
                    libc::SIGFPE,
                    libc::SIGTRAP,
                ] {
                    libc::signal(sig, fatal_handler as *const () as libc::sighandler_t);
                }
                libc::dup2(err_fd, 2);
                lib.reset_tables();
                for &(i, v) in &c.poke_fixed {
                    *lib.fixed_table.add(i) = v;
                }
                for &(i, v) in &c.poke_len_extra {
                    *lib.len_extra_bits.add(i) = v;
                }
                for &(i, v) in &c.poke_dist_extra {
                    *lib.dist_extra_bits.add(i) = v;
                }
                let o = call_raw(
                    &lib,
                    &c.data,
                    c.align,
                    c.in_bytes,
                    c.null_out,
                    c.out_alloc,
                    c.out_bytes,
                );
                let d = o.digest().into_bytes();
                let n = d.len().min(SH_LEN - 8);
                std::ptr::copy_nonoverlapping(d.as_ptr(), sh.add(8), n);
                *(sh as *mut u32) = n as u32;
                *(sh.add(4) as *mut u32) = 0xC0FFEE_u32 & 0xFFFF;
                libc::_exit(0);
            }
            let mut st: libc::c_int = 0;
            libc::waitpid(pid, &mut st, 0);
            let caught = if libc::WIFEXITED(st) {
                let code = libc::WEXITSTATUS(st);
                if code >= FATAL_EXIT_BASE {
                    Some(code - FATAL_EXIT_BASE)
                } else {
                    None
                }
            } else {
                None
            };
            let outcome = if let Some(sig) = caught {
                err_file.seek(std::io::SeekFrom::Start(0)).unwrap();
                let mut txt = String::new();
                err_file.read_to_string(&mut txt).ok();
                format!("SIG{sig} {}", txt.trim().replace('\n', " | "))
            } else if libc::WIFEXITED(st) && *(sh.add(4) as *const u32) != 0 {
                let n = *(sh as *const u32) as usize;
                let bytes = std::slice::from_raw_parts(sh.add(8), n.min(SH_LEN - 8));
                format!("OK {}", String::from_utf8_lossy(bytes))
            } else if libc::WIFSIGNALED(st) && libc::WTERMSIG(st) == libc::SIGALRM {
                "HANG".to_string()
            } else {
                err_file.seek(std::io::SeekFrom::Start(0)).unwrap();
                let mut txt = String::new();
                err_file.read_to_string(&mut txt).ok();
                let sig = if libc::WIFSIGNALED(st) {
                    libc::WTERMSIG(st)
                } else {
                    -libc::WEXITSTATUS(st)
                };
                format!("SIG{sig} {}", txt.trim().replace('\n', " | "))
            };
            writeln!(f, "{idx} {} {}", c.label, outcome).unwrap();
        }
        f.flush().unwrap();
    }
    drop(f);
    let _ = std::fs::remove_file(&err_path);
    // Bypass the harness's own summary so the parent sees a clean exit.
    std::process::exit(0);
}

#[macro_export]
macro_rules! define_child_runner {
    () => {
        /// Helper test re-executed by the parent process; a no-op otherwise.
        #[test]
        fn zz_child_runner() {
            $crate::common::child_runner_main();
        }
    };
}

pub struct ChildOutcome {
    pub status: String,
    pub stderr: Vec<u8>,
    pub lines: Vec<String>,
}

fn run_child(libsel: &str, cases_file: &std::path::Path, tag: &str) -> ChildOutcome {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().unwrap();
    let result = std::env::temp_dir().join(format!(
        "pinflate_res_{}_{}_{}",
        std::process::id(),
        tag,
        libsel
    ));
    let _ = std::fs::remove_file(&result);
    // Belt and braces: the runner already bounds each case with `alarm(1)`,
    // but the whole batch is additionally bounded by `timeout`.
    let secs = std::env::var("PINFLATE_CHILD_TIMEOUT").unwrap_or_else(|_| "550".to_string());
    let out = std::process::Command::new("timeout")
        .arg("-k")
        .arg("1")
        .arg(&secs)
        .arg(&exe)
        .args(["zz_child_runner", "--exact", "--nocapture", "--test-threads=1"])
        .env("PINFLATE_CHILD_LIB", libsel)
        .env("PINFLATE_CHILD_CASES", cases_file)
        .env("PINFLATE_CHILD_RESULT", &result)
        .env("RUST_BACKTRACE", "0")
        .output()
        .expect("failed to re-exec the test binary");
    let status = match out.status.code() {
        Some(124) | Some(137) => "timeout".to_string(),
        Some(c) if c > 128 && c < 160 => format!("signal:{}", c - 128),
        Some(c) => format!("exit:{c}"),
        None => format!("signal:{}", out.status.signal().unwrap_or(-1)),
    };
    let lines = std::fs::read_to_string(&result)
        .unwrap_or_default()
        .lines()
        .map(|s| s.to_string())
        .collect();
    let _ = std::fs::remove_file(&result);
    ChildOutcome { status, stderr: out.stderr, lines }
}

/// Run `cases` through both libraries in separate child processes and require
/// identical per-case transcripts.  Returns the (identical) transcript.
pub fn diff_cases_in_children(tag: &str, cases: &[Case]) -> Vec<String> {
    let cases_file =
        std::env::temp_dir().join(format!("pinflate_cases_{}_{}", std::process::id(), tag));
    let body: String = cases.iter().map(|c| format!("{}\n", c.encode())).collect();
    std::fs::write(&cases_file, &body).unwrap();

    let c = run_child("c", &cases_file, tag);
    let r = run_child("rust", &cases_file, tag);
    let _ = std::fs::remove_file(&cases_file);

    assert_eq!(
        c.status, "exit:0",
        "[{tag}] C runner did not finish: {}\nstderr: {}",
        c.status,
        String::from_utf8_lossy(&c.stderr)
    );
    assert_eq!(
        r.status, "exit:0",
        "[{tag}] Rust runner did not finish: {}\nstderr: {}",
        r.status,
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(
        c.lines.len(),
        cases.len(),
        "[{tag}] C transcript is short ({} of {})",
        c.lines.len(),
        cases.len()
    );
    assert_eq!(
        r.lines.len(),
        cases.len(),
        "[{tag}] Rust transcript is short ({} of {})",
        r.lines.len(),
        cases.len()
    );
    for (i, (a, b)) in c.lines.iter().zip(r.lines.iter()).enumerate() {
        if a != b {
            panic!(
                "[{tag}] DIVERGENCE at case {i}\nC   : {a}\nRust: {b}\ncase: {}",
                cases[i].encode()
            );
        }
    }
    c.lines
}

/// Run a single case in children and return the (identical) transcript line.
pub fn diff_one_in_child(tag: &str, case: Case) -> String {
    diff_cases_in_children(tag, &[case]).pop().unwrap()
}

/// Like [`diff`] but also returns the (now known-identical) outcome so the
/// caller can additionally assert *which* rejection happened.
pub fn diff_get(label: &str, data: &[u8], align: usize, out_len: usize) -> Outcome {
    let p = pair();
    let _g = lock();
    unsafe {
        p.c.reset_tables();
        p.rs.reset_tables();
        let a = call(&p.c, data, align, data.len() as c_int, out_len);
        let b = call(&p.rs, data, align, data.len() as c_int, out_len);
        assert_eq(label, data, align, out_len, &a, &b);
        a
    }
}

/// Same, for the raw-argument form.
pub fn diff_get_raw(
    label: &str,
    data: &[u8],
    align: usize,
    in_bytes: c_int,
    null_out: bool,
    alloc: usize,
    out_bytes: c_int,
) -> Outcome {
    let p = pair();
    let _g = lock();
    unsafe {
        p.c.reset_tables();
        p.rs.reset_tables();
        let a = call_raw(&p.c, data, align, in_bytes, null_out, alloc, out_bytes);
        let b = call_raw(&p.rs, data, align, in_bytes, null_out, alloc, out_bytes);
        assert_eq(label, data, align, alloc, &a, &b);
        a
    }
}

pub fn reason_str(o: &Outcome) -> String {
    o.reason
        .as_ref()
        .map(|r| String::from_utf8_lossy(r).to_string())
        .unwrap_or_else(|| "<null>".to_string())
}

/// Whether `out == NULL` cases may be included in a sweep.
///
/// Passing `out == NULL` to a stream that emits a literal makes *both*
/// libraries dereference a null pointer.  The release `.so` and the C `.so`
/// both take `SIGSEGV`; a **debug**-profile Rust cdylib instead traps the
/// dereference with its own "null pointer dereference occurred" check and
/// aborts, so the two die from different signals.  That is a property of the
/// Rust *debug profile*, not of the translation, so `check_features.sh` sets
/// `PINFLATE_SKIP_NULL_OUT=1` when it exercises the debug cdylib.  The
/// null-`out` cases that do **not** dereference (`g03_null_out_empty_block`)
/// still run in every configuration.
pub fn null_out_allowed() -> bool {
    std::env::var("PINFLATE_SKIP_NULL_OUT").is_err()
}

/// Transcript from ONE library only — used to *search* for an input that
/// reaches a particular `assert()` before asserting C/Rust equivalence on it.
pub fn one_library_transcript(libsel: &str, tag: &str, cases: &[Case]) -> Vec<String> {
    let cases_file =
        std::env::temp_dir().join(format!("pinflate_scan_{}_{}", std::process::id(), tag));
    let body: String = cases.iter().map(|c| format!("{}\n", c.encode())).collect();
    std::fs::write(&cases_file, &body).unwrap();
    let o = run_child(libsel, &cases_file, &format!("{tag}_scan"));
    let _ = std::fs::remove_file(&cases_file);
    assert_eq!(
        o.status, "exit:0",
        "[{tag}] scan runner did not finish: {}\nstderr: {}",
        o.status,
        String::from_utf8_lossy(&o.stderr)
    );
    o.lines
}
