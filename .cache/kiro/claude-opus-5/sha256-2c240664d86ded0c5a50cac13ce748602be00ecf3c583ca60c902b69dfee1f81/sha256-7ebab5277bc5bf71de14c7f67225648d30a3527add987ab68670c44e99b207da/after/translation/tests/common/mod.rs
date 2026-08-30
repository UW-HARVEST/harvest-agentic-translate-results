//! Shared helpers: load both the C reference `.so` and the Rust `.so` and call
//! them exclusively through their exported C symbols.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void, CStr};
use std::path::{Path, PathBuf};

unsafe extern "C" {
    fn free(p: *mut c_void);
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CpPixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CpImage {
    pub w: c_int,
    pub h: c_int,
    pub pix: *mut CpPixel,
}

pub type InflateFn = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int;
pub type LoadPngFn = unsafe extern "C" fn(*const u8, c_int) -> CpImage;

pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    pub inflate: InflateFn,
    pub load_png: LoadPngFn,
    error_reason: *mut *const c_char,
}

impl Impl {
    fn open(name: &'static str, path: &Path) -> Impl {
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("failed to load {}: {e}", path.display()));
            let inflate: Symbol<InflateFn> = lib
                .get(b"cp_inflate\0")
                .expect("cp_inflate missing from library");
            let load_png: Symbol<LoadPngFn> = lib
                .get(b"load_png_mem\0")
                .expect("load_png_mem missing from library");
            let err: Symbol<*mut *const c_char> = lib
                .get(b"cp_error_reason\0")
                .expect("cp_error_reason missing from library");
            let inflate = *inflate;
            let load_png = *load_png;
            let error_reason = *err;
            Impl {
                name,
                _lib: lib,
                inflate,
                load_png,
                error_reason,
            }
        }
    }

    /// Raw pointer to an exported byte array symbol.
    pub fn sym_ptr(&self, sym: &[u8]) -> *const u8 {
        unsafe {
            let s: Symbol<*const u8> = self
                ._lib
                .get(sym)
                .unwrap_or_else(|e| panic!("{}: missing symbol {:?}: {e}", self.name, sym));
            *s as *const u8
        }
    }

    pub fn set_error_reason_null(&self) {
        unsafe { *self.error_reason = std::ptr::null() }
    }

    pub fn error_reason(&self) -> Option<Vec<u8>> {
        unsafe {
            let p = *self.error_reason;
            if p.is_null() {
                None
            } else {
                Some(CStr::from_ptr(p).to_bytes().to_vec())
            }
        }
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let build = workspace_root().join("c_src").join("build");
    let mut found = None;
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "so").unwrap_or(false) {
                found = Some(p);
                break;
            }
        }
    }
    found.unwrap_or_else(|| {
        panic!(
            "no .so found in {}; build the C library first",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    // Prefer the release artifact: `panic = "abort"` and no overflow checks
    // there, which is what an external consumer links against.
    for profile in ["release", "debug"] {
        let p = target.join(profile).join("libload_png_mem_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("libload_png_mem_lib.so not found; run `cargo build --release` first");
}

pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

// The libraries are only ever driven through the serialising `GLOBAL_LOCK`
// below, so sharing the handles across the test harness threads is fine.
unsafe impl Send for Impl {}
unsafe impl Sync for Impl {}

/// `cp_error_reason` is a process-global in both libraries, so every
/// call/inspect sequence has to be serialised.
pub static GLOBAL_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn locked<R>(f: impl FnOnce() -> R) -> R {
    let g = GLOBAL_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let r = f();
    drop(g);
    r
}

pub fn pair() -> &'static Pair {
    use std::sync::OnceLock;
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(|| Pair {
        c: Impl::open("C", &find_c_so()),
        rs: Impl::open("Rust", &find_rust_so()),
    })
}

/* ----------------------------------------------------------------------- */
/* cp_inflate driver                                                       */
/* ----------------------------------------------------------------------- */

pub const OUT_SLACK: usize = 64;
pub const OUT_FILL: u8 = 0xCD;

pub struct InflateResult {
    pub ret: c_int,
    pub out: Vec<u8>,
    pub err: Option<Vec<u8>>,
}

/// An input buffer whose payload starts at a controlled 4-byte alignment.
pub struct AlignedInput {
    backing: Vec<u8>,
    offset: usize,
    len: usize,
}

impl AlignedInput {
    /// `align` in 0..4 is the desired `ptr % 4`.
    pub fn new(data: &[u8], align: usize) -> AlignedInput {
        let mut backing = vec![0u8; data.len() + 128];
        let base = backing.as_ptr() as usize;
        // first slot >= base+8 with the requested residue
        let mut off = 8usize;
        while (base + off) % 4 != align % 4 {
            off += 1;
        }
        backing[off..off + data.len()].copy_from_slice(data);
        AlignedInput {
            backing,
            offset: off,
            len: data.len(),
        }
    }

    pub fn ptr(&self) -> *mut c_void {
        unsafe { self.backing.as_ptr().add(self.offset) as *mut c_void }
    }

    pub fn len(&self) -> c_int {
        self.len as c_int
    }
}

/// Runs `cp_inflate` with `out_bytes` reported to the library, but a backing
/// buffer of `backing` bytes. `cp_stored` performs no output bounds check at
/// all (the C `memcpy` is unguarded), so the backing buffer has to be able to
/// absorb whatever the stream asks for; the whole buffer is then compared.
pub fn run_inflate_backed(
    imp: &Impl,
    input: &AlignedInput,
    out_bytes: usize,
    backing: usize,
) -> InflateResult {
    let backing = backing.max(out_bytes + OUT_SLACK);
    let mut out = vec![OUT_FILL; backing];
    locked(|| {
        imp.set_error_reason_null();
        let ret = unsafe {
            (imp.inflate)(
                input.ptr(),
                input.len(),
                out.as_mut_ptr() as *mut c_void,
                out_bytes as c_int,
            )
        };
        InflateResult {
            ret,
            out,
            err: imp.error_reason(),
        }
    })
}

pub fn run_inflate(imp: &Impl, input: &AlignedInput, out_bytes: usize) -> InflateResult {
    run_inflate_backed(imp, input, out_bytes, out_bytes + OUT_SLACK)
}

/* ----------------------------------------------------------------------- */
/* load_png_mem driver                                                     */
/* ----------------------------------------------------------------------- */

pub struct PngResult {
    pub w: c_int,
    pub h: c_int,
    pub null: bool,
    pub pix: Vec<CpPixel>,
    pub err: Option<Vec<u8>>,
}

/// A PNG byte buffer with trailing slack. The C code reads a chunk length with
/// an unguarded 4-byte load (`cp_make32`) while `p < end`, so it can read up to
/// 3 bytes past `png_length`; the slack keeps that inside an allocation.
/// Both libraries get the *same* pointer, so whatever is in the slack is seen
/// identically by both.
pub struct PngInput {
    backing: Vec<u8>,
    len: usize,
}

impl PngInput {
    pub fn new(data: &[u8]) -> PngInput {
        let mut backing = vec![0u8; data.len() + 64];
        backing[..data.len()].copy_from_slice(data);
        PngInput {
            backing,
            len: data.len(),
        }
    }

    pub fn with_len(data: &[u8], len: usize) -> PngInput {
        let mut backing = vec![0u8; data.len() + 64];
        backing[..data.len()].copy_from_slice(data);
        PngInput { backing, len }
    }

    pub fn ptr(&self) -> *const u8 {
        self.backing.as_ptr()
    }

    pub fn bytes(&self) -> &[u8] {
        &self.backing
    }

    pub fn len(&self) -> c_int {
        self.len as c_int
    }
}

/// Poisons a freed heap block of `nbytes` with `pattern` so that the
/// *uninitialised* part of the `malloc(pix_bytes)` buffer that `load_png_mem`
/// hands back becomes deterministic. `load_png_mem` only fills the region the
/// DEFLATE stream actually produced, so for truncated/partial streams the tail
/// of the returned image is whatever was in the heap. Poisoning with two
/// different patterns lets the comparison identify and exclude exactly those
/// bytes instead of reporting a spurious mismatch.
fn poison_heap(nbytes: usize, pattern: u8) {
    // Stay below glibc's mmap threshold so the block really is recycled by the
    // following mallocs instead of being handed back to the kernel.
    let n = (nbytes + (64 << 10)).min(120 << 10);
    let v = vec![pattern; n];
    std::hint::black_box(v.as_ptr());
    drop(v);
    let _ = nbytes;
}

/// `(w+1) * h * 4`, truncated to `int`, exactly as `load_png_mem` computes it.
pub fn pix_bytes_of(data: &[u8], len: usize) -> usize {
    if len < 24 || data.len() < 24 {
        return 0;
    }
    let g = |o: usize| u32::from_be_bytes([data[o], data[o + 1], data[o + 2], data[o + 3]]);
    let w = g(16).wrapping_add(1) as i32;
    let h = g(20) as i32;
    let n = (w as i64).wrapping_mul(h as i64).wrapping_mul(4);
    if n <= 0 || n > (64 << 20) {
        0
    } else {
        n as usize
    }
}

pub fn run_load_png_poisoned(imp: &Impl, input: &PngInput, poison: usize, pattern: u8) -> PngResult {
    let (img, err) = locked(|| {
        poison_heap(poison, pattern);
        imp.set_error_reason_null();
        let img = unsafe { (imp.load_png)(input.ptr(), input.len()) };
        let err = imp.error_reason();
        (img, err)
    });
    collect_png(img, err)
}

pub fn run_load_png(imp: &Impl, input: &PngInput) -> PngResult {
    let (img, err) = locked(|| {
        imp.set_error_reason_null();
        let img = unsafe { (imp.load_png)(input.ptr(), input.len()) };
        let err = imp.error_reason();
        (img, err)
    });
    collect_png(img, err)
}

fn collect_png(img: CpImage, err: Option<Vec<u8>>) -> PngResult {
    let null = img.pix.is_null();
    let mut pix = Vec::new();
    if !null {
        // The C code allocates (w+1) * h * sizeof(cp_pixel_t) bytes but only the
        // first w*h pixels are meaningful output; compare exactly those.
        let n = (img.w as i64).max(0) * (img.h as i64).max(0);
        let n = n.max(0) as usize;
        pix.reserve(n);
        unsafe {
            for i in 0..n {
                pix.push(*img.pix.add(i));
            }
            free(img.pix as *mut c_void);
        }
    }
    PngResult {
        w: img.w,
        h: img.h,
        null,
        pix,
        err,
    }
}

pub fn assert_png_eq(label: &str, c: &PngResult, r: &PngResult) {
    assert_eq!(c.w, r.w, "{label}: img.w mismatch");
    assert_eq!(c.h, r.h, "{label}: img.h mismatch");
    assert_eq!(c.null, r.null, "{label}: pix-null mismatch");
    assert_eq!(
        c.err.as_deref().map(String::from_utf8_lossy),
        r.err.as_deref().map(String::from_utf8_lossy),
        "{label}: cp_error_reason mismatch"
    );
    if c.pix != r.pix {
        let mut first = None;
        for (i, (a, b)) in c.pix.iter().zip(r.pix.iter()).enumerate() {
            if a != b {
                first = Some((i, *a, *b));
                break;
            }
        }
        match first {
            Some((i, a, b)) => panic!(
                "{label}: pixel {i} differs: C=({},{},{},{}) Rust=({},{},{},{})",
                a.r, a.g, a.b, a.a, b.r, b.g, b.b, b.a
            ),
            None => panic!(
                "{label}: pixel count differs: C={} Rust={}",
                c.pix.len(),
                r.pix.len()
            ),
        }
    }
}

/// Full comparison of one PNG input.
///
/// Each library is run twice with a differently poisoned heap. Scalar results
/// (`w`, `h`, whether `pix` is NULL, `cp_error_reason`) must match exactly.
/// Pixels are compared wherever both libraries produced a value that is
/// independent of the heap poison; indices where *both* libraries returned
/// poison-dependent bytes are uninitialised memory in the C original (a
/// truncated DEFLATE stream never fills the whole buffer) and are skipped.
///
/// Returns the number of skipped pixels.
pub fn compare_png_input(label: &str, input: &PngInput) -> usize {
    let p = pair();
    let poison = pix_bytes_of(input.bytes(), input.len() as usize);
    let c1 = run_load_png_poisoned(&p.c, input, poison, 0xA5);
    let r1 = run_load_png_poisoned(&p.rs, input, poison, 0xA5);
    let c2 = run_load_png_poisoned(&p.c, input, poison, 0x5C);
    let r2 = run_load_png_poisoned(&p.rs, input, poison, 0x5C);

    for (tag, x) in [("C#2", &c2), ("Rust#1", &r1), ("Rust#2", &r2)] {
        assert_eq!(c1.w, x.w, "{label}: img.w mismatch ({tag})");
        assert_eq!(c1.h, x.h, "{label}: img.h mismatch ({tag})");
        assert_eq!(c1.null, x.null, "{label}: pix-NULL mismatch ({tag})");
        assert_eq!(
            c1.err.as_deref().map(String::from_utf8_lossy),
            x.err.as_deref().map(String::from_utf8_lossy),
            "{label}: cp_error_reason mismatch ({tag})"
        );
    }
    assert_eq!(c1.pix.len(), r1.pix.len(), "{label}: pixel count mismatch");

    let mut skipped = 0usize;
    for i in 0..c1.pix.len() {
        let c_det = c1.pix[i] == c2.pix[i];
        let r_det = r1.pix[i] == r2.pix[i];
        if !c_det && !r_det {
            skipped += 1;
            continue;
        }
        assert!(
            c_det,
            "{label}: pixel {i} is heap-poison dependent in C but stable in Rust"
        );
        assert!(
            r_det,
            "{label}: pixel {i} is heap-poison dependent in Rust but stable in C"
        );
        let (a, b) = (c1.pix[i], r1.pix[i]);
        assert!(
            a == b,
            "{label}: pixel {i} differs: C=({},{},{},{}) Rust=({},{},{},{})",
            a.r,
            a.g,
            a.b,
            a.a,
            b.r,
            b.g,
            b.b,
            b.a
        );
    }
    skipped
}

fn png_scalars(x: &PngResult) -> (c_int, c_int, bool, Option<String>) {
    (
        x.w,
        x.h,
        x.null,
        x.err.as_deref().map(|e| String::from_utf8_lossy(e).to_string()),
    )
}

/// Like [`compare_png_input`], but first checks whether the *C* implementation
/// is even deterministic for this input.
///
/// `cp_stored` copies `LEN` bytes out of the input buffer with an unguarded
/// `memcpy`, so a PNG whose zlib stream is a stored block that has been cut
/// short makes the C code read past the end of its `malloc(datalen)` buffer.
/// The result then depends on unrelated heap contents and there is nothing to
/// compare against. Such inputs are detected (the two poison patterns give
/// different answers) and reported as skipped.
///
/// Returns `false` when the case was skipped.
pub fn compare_png_input_if_deterministic(label: &str, input: &PngInput) -> bool {
    let p = pair();
    let poison = pix_bytes_of(input.bytes(), input.len() as usize);
    let c1 = run_load_png_poisoned(&p.c, input, poison, 0xA5);
    let c2 = run_load_png_poisoned(&p.c, input, poison, 0x5C);
    let r1 = run_load_png_poisoned(&p.rs, input, poison, 0xA5);
    let r2 = run_load_png_poisoned(&p.rs, input, poison, 0x5C);

    if png_scalars(&c1) != png_scalars(&c2) || c1.pix != c2.pix {
        // C is not self-consistent: heap-content dependent, nothing to verify.
        return false;
    }
    if png_scalars(&r1) != png_scalars(&r2) || r1.pix != r2.pix {
        // Rust is heap dependent where C is not: that *is* a divergence unless
        // C simply never allocated (error path). Report it.
        panic!("{label}: Rust result depends on heap contents but C does not");
    }
    assert_eq!(
        png_scalars(&c1),
        png_scalars(&r1),
        "{label}: scalar result mismatch"
    );
    assert_eq!(c1.pix.len(), r1.pix.len(), "{label}: pixel count mismatch");
    for i in 0..c1.pix.len() {
        let (a, b) = (c1.pix[i], r1.pix[i]);
        assert!(
            a == b,
            "{label}: pixel {i} differs: C=({},{},{},{}) Rust=({},{},{},{})",
            a.r,
            a.g,
            a.b,
            a.a,
            b.r,
            b.g,
            b.b,
            b.a
        );
    }
    true
}

pub fn assert_inflate_eq(label: &str, c: &InflateResult, r: &InflateResult) {
    assert_eq!(c.ret, r.ret, "{label}: return value mismatch");
    assert_eq!(
        c.err.as_deref().map(String::from_utf8_lossy),
        r.err.as_deref().map(String::from_utf8_lossy),
        "{label}: cp_error_reason mismatch"
    );
    if c.out != r.out {
        for (i, (a, b)) in c.out.iter().zip(r.out.iter()).enumerate() {
            if a != b {
                panic!("{label}: out byte {i} differs: C={a:#04x} Rust={b:#04x}");
            }
        }
        panic!("{label}: out length differs");
    }
}

/* ----------------------------------------------------------------------- */
/* PNG builders (shared by the png/png_structure test binaries)            */
/* ----------------------------------------------------------------------- */

pub fn bpp_of(ct: u8) -> u32 {
    match ct {
        0 => 1,
        2 => 3,
        3 => 1,
        4 => 2,
        6 => 4,
        _ => 1,
    }
}

pub fn filter0(row: &[u8]) -> Vec<u8> {
    let mut v = vec![0u8];
    v.extend_from_slice(row);
    v
}

pub fn crc32(data: &[u8]) -> u32 {
    use std::sync::OnceLock;
    static TABLE: OnceLock<[u32; 256]> = OnceLock::new();
    let t = TABLE.get_or_init(|| {
        let mut t = [0u32; 256];
        for n in 0..256u32 {
            let mut c = n;
            for _ in 0..8 {
                c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
            }
            t[n as usize] = c;
        }
        t
    });
    let mut c = 0xFFFF_FFFFu32;
    for &b in data {
        c = t[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    c ^ 0xFFFF_FFFF
}

pub fn chunk(tag: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(data.len() + 12);
    v.extend_from_slice(&(data.len() as u32).to_be_bytes());
    v.extend_from_slice(tag);
    v.extend_from_slice(data);
    let mut crc_in = tag.to_vec();
    crc_in.extend_from_slice(data);
    v.extend_from_slice(&crc32(&crc_in).to_be_bytes());
    v
}

/// A zlib stream wrapping `raw` in a single *stored* DEFLATE block. Keeping the
/// stream compression-free means a mutated byte can never turn into a corrupt
/// Huffman table (which the reference C library answers with `assert()`).
pub fn zlib_stored(raw: &[u8]) -> Vec<u8> {
    assert!(raw.len() <= u16::MAX as usize);
    let mut z = vec![0x78u8, 0x01];
    z.push(0x01); // BFINAL = 1, BTYPE = 00
    z.extend_from_slice(&(raw.len() as u16).to_le_bytes());
    z.extend_from_slice(&(!(raw.len() as u16)).to_le_bytes());
    z.extend_from_slice(raw);
    let (mut a, mut b) = (1u32, 0u32);
    for &x in raw {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    z.extend_from_slice(&(((b << 16) | a) as u32).to_be_bytes());
    z
}

pub const PNG_SIG: [u8; 8] = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];

pub fn build_png_raw(
    w: u32,
    h: u32,
    color_type: u8,
    raw_rows: &[Vec<u8>],
    plte: Option<&[u8]>,
    trns: Option<&[u8]>,
) -> Vec<u8> {
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[8, color_type, 0, 0, 0]);
    let mut out = PNG_SIG.to_vec();
    out.extend_from_slice(&chunk(b"IHDR", &ihdr));
    if let Some(p) = plte {
        out.extend_from_slice(&chunk(b"PLTE", p));
    }
    if let Some(t) = trns {
        out.extend_from_slice(&chunk(b"tRNS", t));
    }
    let raw: Vec<u8> = raw_rows.iter().flatten().copied().collect();
    out.extend_from_slice(&chunk(b"IDAT", &zlib_stored(&raw)));
    out.extend_from_slice(&chunk(b"IEND", b""));
    out
}

/// Deterministic pseudo-random image with filter byte `y % 5` per scanline.
pub fn synth_png(w: u32, h: u32, color_type: u8, seed: u64) -> Vec<u8> {
    let bpp = bpp_of(color_type) as usize;
    let mut state = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    let mut next = || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (state >> 33) as u8
    };
    let mut raw: Vec<Vec<u8>> = Vec::new();
    for y in 0..h {
        let mut line = vec![(y % 5) as u8];
        for _ in 0..(w as usize * bpp) {
            line.push(next());
        }
        raw.push(line);
    }
    let plte: Option<Vec<u8>> = if color_type == 3 {
        Some((0..768).map(|i| ((i * 7 + 13) & 0xFF) as u8).collect())
    } else {
        None
    };
    build_png_raw(w, h, color_type, &raw, plte.as_deref(), None)
}

/* ----------------------------------------------------------------------- */
/* fixtures                                                                */
/* ----------------------------------------------------------------------- */

pub fn deflate_fixtures() -> Vec<(String, Vec<u8>, usize)> {
    let dir = fixtures_dir().join("deflate");
    let manifest = std::fs::read_to_string(dir.join("manifest.txt"))
        .expect("deflate manifest missing; run gen_fixtures.py");
    manifest
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let mut it = l.split_whitespace();
            let name = it.next().unwrap().to_string();
            let ulen: usize = it.next().unwrap().parse().unwrap();
            let data = std::fs::read(dir.join(format!("{name}.bin"))).unwrap();
            (name, data, ulen)
        })
        .collect()
}

pub fn png_fixtures() -> Vec<(String, Vec<u8>)> {
    let dir = fixtures_dir().join("png");
    let manifest = std::fs::read_to_string(dir.join("manifest.txt"))
        .expect("png manifest missing; run gen_fixtures.py");
    manifest
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let name = l.trim().to_string();
            let data = std::fs::read(dir.join(format!("{name}.png"))).unwrap();
            (name, data)
        })
        .collect()
}
