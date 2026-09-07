//! Shared harness: dynamically loads BOTH the reference C shared object and the
//! Rust shared object and drives them through their exported C ABI only.
//!
//! Nothing in here ever calls a Rust function directly — every call goes through
//! `dlsym`, exactly like an external consumer, so the `#[no_mangle]` wrappers
//! are part of what is being tested.

#![allow(dead_code)]

pub mod deflate;

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// locating the two shared objects
// ---------------------------------------------------------------------------

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/lib<project>.so` — the project name is derived from the
/// directory name by `CMakeLists.txt`, so glob for it instead of hard-coding.
pub fn c_so_path() -> PathBuf {
    let build = manifest_dir().join("..").join("c_src").join("build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name.starts_with("lib") && name.ends_with(".so") {
                found.push(p);
            }
        }
    }
    found.sort();
    assert!(
        !found.is_empty(),
        "no C shared object in {}; build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build.display()
    );
    found.remove(0)
}

/// `target/<profile>/libconvert_pix_lib.so`, found relative to the running test
/// executable (`target/<profile>/deps/<test>-<hash>`).
pub fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let profile = deps.parent().expect("profile dir");
    for dir in [profile, deps] {
        let p = dir.join("libconvert_pix_lib.so");
        if p.is_file() {
            return p;
        }
    }
    // fall back to either profile directory under target/
    for prof in ["debug", "release"] {
        let p = manifest_dir().join("target").join(prof).join("libconvert_pix_lib.so");
        if p.is_file() {
            return p;
        }
    }
    panic!("libconvert_pix_lib.so not found near {}", exe.display());
}

// ---------------------------------------------------------------------------
// the loaded pair
// ---------------------------------------------------------------------------

pub type CpInflateFn = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int;
pub type ConvertPixFn = unsafe extern "C" fn(c_int, c_int, c_int, *mut u8, *mut CpPixel);

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct CpPixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

pub struct Lib {
    pub tag: &'static str,
    pub path: PathBuf,
    lib: Library,
}

impl Lib {
    pub fn open(tag: &'static str, path: &Path) -> Lib {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));
        Lib { tag, path: path.to_path_buf(), lib }
    }

    pub fn cp_inflate(&self) -> Symbol<'_, CpInflateFn> {
        unsafe { self.lib.get(b"cp_inflate\0") }.expect("cp_inflate not exported")
    }

    pub fn convert_pix(&self) -> Symbol<'_, ConvertPixFn> {
        unsafe { self.lib.get(b"convert_pix\0") }.expect("convert_pix not exported")
    }

    /// `const char *cp_error_reason;` — the *variable*, so a `char **`.
    pub fn error_reason_slot(&self) -> Symbol<'_, *mut *const c_char> {
        unsafe { self.lib.get(b"cp_error_reason\0") }.expect("cp_error_reason not exported")
    }

    /// Current value of `cp_error_reason` as bytes (`None` when NULL).
    pub fn error_reason(&self) -> Option<Vec<u8>> {
        let slot = self.error_reason_slot();
        unsafe {
            let p = **slot;
            if p.is_null() {
                return None;
            }
            let mut n = 0usize;
            while *p.add(n) != 0 {
                n += 1;
            }
            Some(std::slice::from_raw_parts(p as *const u8, n).to_vec())
        }
    }

    pub fn set_error_reason_null(&self) {
        let slot = self.error_reason_slot();
        unsafe {
            **slot = std::ptr::null();
        }
    }

    pub fn data_u8(&self, name: &[u8], len: usize) -> Vec<u8> {
        let mut n = name.to_vec();
        n.push(0);
        let sym: Symbol<'_, *mut u8> = unsafe { self.lib.get(&n) }
            .unwrap_or_else(|e| panic!("{:?} not exported: {e}", String::from_utf8_lossy(name)));
        unsafe { std::slice::from_raw_parts(*sym as *const u8, len).to_vec() }
    }

    pub fn data_u32(&self, name: &[u8], len: usize) -> Vec<u32> {
        let mut n = name.to_vec();
        n.push(0);
        let sym: Symbol<'_, *mut u32> = unsafe { self.lib.get(&n) }
            .unwrap_or_else(|e| panic!("{:?} not exported: {e}", String::from_utf8_lossy(name)));
        unsafe { std::slice::from_raw_parts(*sym as *const u32, len).to_vec() }
    }

    pub fn data_ptr_u8(&self, name: &[u8]) -> *mut u8 {
        let mut n = name.to_vec();
        n.push(0);
        let sym: Symbol<'_, *mut u8> = unsafe { self.lib.get(&n) }
            .unwrap_or_else(|e| panic!("{:?} not exported: {e}", String::from_utf8_lossy(name)));
        *sym
    }
}

pub struct Pair {
    pub c: Lib,
    pub rs: Lib,
}

static PAIR: OnceLock<Pair> = OnceLock::new();

/// `cp_error_reason` is a single mutable global *inside each shared object*, so
/// two tests running concurrently would clobber each other's error state. Every
/// call sequence that touches the shared pair must hold this lock.
static GLOBAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn global_lock() -> std::sync::MutexGuard<'static, ()> {
    GLOBAL.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| Pair {
        c: Lib::open("C", &c_so_path()),
        rs: Lib::open("RUST", &rust_so_path()),
    })
}

// ---------------------------------------------------------------------------
// 4-byte-aligned input buffer with a controllable address%4
// ---------------------------------------------------------------------------

/// Owns a 4-byte-aligned allocation and hands out `base + off`, so
/// `ptr() as usize % 4 == off % 4` deterministically for both libraries.
pub struct AlignedBuf {
    backing: Vec<u32>,
    off: usize,
    len: usize,
}

impl AlignedBuf {
    pub fn new(data: &[u8], off: usize) -> AlignedBuf {
        let words = (off + data.len() + 8 + 3) / 4 + 2;
        let mut backing = vec![0u32; words];
        assert_eq!(backing.as_ptr() as usize % 4, 0, "Vec<u32> must be 4-aligned");
        unsafe {
            let base = backing.as_mut_ptr() as *mut u8;
            std::ptr::copy_nonoverlapping(data.as_ptr(), base.add(off), data.len());
        }
        AlignedBuf { backing, off, len: data.len() }
    }

    pub fn ptr(&mut self) -> *mut u8 {
        unsafe { (self.backing.as_mut_ptr() as *mut u8).add(self.off) }
    }

    pub fn len(&self) -> usize {
        self.len
    }
}

// ---------------------------------------------------------------------------
// differential drivers
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
pub struct InflateOutcome {
    pub ret: c_int,
    pub out: Vec<u8>,
    pub err: Option<Vec<u8>>,
}

/// Runs `cp_inflate` in one library on a freshly built, identically aligned
/// input buffer and a freshly patterned output buffer.
pub fn run_inflate(
    lib: &Lib,
    input: &[u8],
    in_align: usize,
    in_bytes: c_int,
    out_bytes: c_int,
    out_capacity: usize,
    fill: u8,
) -> InflateOutcome {
    let mut inbuf = AlignedBuf::new(input, in_align);
    let mut out = vec![fill; out_capacity];
    lib.set_error_reason_null();
    let f = lib.cp_inflate();
    let ret = unsafe {
        f(
            inbuf.ptr() as *mut c_void,
            in_bytes,
            out.as_mut_ptr() as *mut c_void,
            out_bytes,
        )
    };
    InflateOutcome { ret, out, err: lib.error_reason() }
}

/// Full differential `cp_inflate` comparison. `out_capacity` is how much memory
/// is really available (>= `out_bytes` so an overrun does not corrupt the heap);
/// `out_bytes` is what the library is told.
#[track_caller]
pub fn diff_inflate_full(
    label: &str,
    input: &[u8],
    in_align: usize,
    in_bytes: c_int,
    out_bytes: c_int,
    out_capacity: usize,
) -> InflateOutcome {
    let _g = global_lock();
    let p = pair();
    let fill = 0xCD;
    let a = run_inflate(&p.c, input, in_align, in_bytes, out_bytes, out_capacity, fill);
    let b = run_inflate(&p.rs, input, in_align, in_bytes, out_bytes, out_capacity, fill);
    if a.ret != b.ret {
        panic!(
            "[{label}] return value differs: C={} RUST={}\n  input({} B, align {})={:02x?}",
            a.ret,
            b.ret,
            input.len(),
            in_align,
            &input[..input.len().min(64)]
        );
    }
    if a.err != b.err {
        panic!(
            "[{label}] cp_error_reason differs:\n  C   ={:?}\n  RUST={:?}",
            a.err.as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
            b.err.as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
        );
    }
    if a.out != b.out {
        let idx = a.out.iter().zip(b.out.iter()).position(|(x, y)| x != y).unwrap_or(0);
        let lo = idx.saturating_sub(8);
        let hi = (idx + 8).min(a.out.len());
        panic!(
            "[{label}] output differs at byte {idx} (len {}):\n  C   ={:02x?}\n  RUST={:02x?}",
            a.out.len(),
            &a.out[lo..hi],
            &b.out[lo..hi]
        );
    }
    a
}

/// Convenience: honest `out_bytes`, capacity == `out_bytes` rounded up a bit.
#[track_caller]
pub fn diff_inflate(label: &str, input: &[u8], in_align: usize, out_bytes: usize) -> InflateOutcome {
    diff_inflate_full(
        label,
        input,
        in_align,
        input.len() as c_int,
        out_bytes as c_int,
        out_bytes + 64,
    )
}

/// Happy-path helper: both libraries must agree AND the shared answer must be
/// the payload we encoded (so a row cannot silently pass by both failing).
#[track_caller]
pub fn diff_inflate_expect(label: &str, input: &[u8], in_align: usize, expected: &[u8]) {
    let o = diff_inflate(label, input, in_align, expected.len());
    assert_eq!(o.ret, 1, "[{label}] cp_inflate failed: err={:?}", o.err.as_ref().map(|v| String::from_utf8_lossy(v).into_owned()));
    assert_eq!(
        &o.out[..expected.len()],
        expected,
        "[{label}] decompressed payload is not the encoded payload"
    );
}

/// Same, but the caller controls `out_bytes` / capacity (row 25: slack buffer).
#[track_caller]
pub fn diff_inflate_expect_slack(
    label: &str,
    input: &[u8],
    in_align: usize,
    expected: &[u8],
    out_bytes: usize,
) {
    let o = diff_inflate_full(
        label,
        input,
        in_align,
        input.len() as c_int,
        out_bytes as c_int,
        out_bytes + 64,
    );
    assert_eq!(o.ret, 1, "[{label}] cp_inflate failed");
    assert_eq!(&o.out[..expected.len()], expected, "[{label}] payload mismatch");
}

#[track_caller]
pub fn diff_convert_pix(label: &str, bpp: c_int, w: c_int, h: c_int, src: &[u8], dst_len: usize) {
    let _g = global_lock();
    let p = pair();
    let mut sa = src.to_vec();
    let mut sb = src.to_vec();
    let sentinel = CpPixel { r: 0x11, g: 0x22, b: 0x33, a: 0x44 };
    let mut da = vec![sentinel; dst_len];
    let mut db = vec![sentinel; dst_len];
    unsafe {
        (p.c.convert_pix())(bpp, w, h, sa.as_mut_ptr(), da.as_mut_ptr());
        (p.rs.convert_pix())(bpp, w, h, sb.as_mut_ptr(), db.as_mut_ptr());
    }
    assert_eq!(sa, sb, "[{label}] convert_pix modified src differently (it must not)");
    if da != db {
        let i = da.iter().zip(db.iter()).position(|(x, y)| x != y).unwrap();
        panic!(
            "[{label}] convert_pix dst differs at pixel {i}: C={:?} RUST={:?} (bpp={bpp} w={w} h={h})",
            da[i], db[i]
        );
    }
}

// ---------------------------------------------------------------------------
// deterministic RNG (xoshiro256**), so every row uses many inputs reproducibly
// ---------------------------------------------------------------------------

/// Interior mutability so that nested calls such as
/// `rng.bytes(rng.range(1, 40))` are legal (they are still deterministic:
/// arguments are evaluated left-to-right, inner call first).
pub struct Rng {
    st: std::cell::Cell<[u64; 4]>,
}

impl Rng {
    pub fn new(seed: u64) -> Rng {
        // SplitMix64 seeding.
        let mut z = seed;
        let mut s = [0u64; 4];
        for slot in s.iter_mut() {
            z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut x = z;
            x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            *slot = x ^ (x >> 31);
        }
        Rng { st: std::cell::Cell::new(s) }
    }

    pub fn next_u64(&self) -> u64 {
        let mut s = self.st.get();
        let result = s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = s[1] << 17;
        s[2] ^= s[0];
        s[3] ^= s[1];
        s[1] ^= s[2];
        s[0] ^= s[3];
        s[2] ^= t;
        s[3] = s[3].rotate_left(45);
        self.st.set(s);
        result
    }

    pub fn next_u32(&self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform in `[0, n)`.
    pub fn below(&self, n: usize) -> usize {
        assert!(n > 0);
        (self.next_u64() % n as u64) as usize
    }

    pub fn range(&self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below(hi_inclusive - lo + 1)
    }

    pub fn byte(&self) -> u8 {
        (self.next_u64() >> 56) as u8
    }

    pub fn bytes(&self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.byte()).collect()
    }

    /// Bytes drawn from a small alphabet — produces matches, so encoders emit
    /// length/distance pairs and (for level >= 1) dynamic Huffman blocks.
    pub fn bytes_alphabet(&self, n: usize, alphabet: usize) -> Vec<u8> {
        (0..n).map(|_| (self.below(alphabet)) as u8).collect()
    }
}
