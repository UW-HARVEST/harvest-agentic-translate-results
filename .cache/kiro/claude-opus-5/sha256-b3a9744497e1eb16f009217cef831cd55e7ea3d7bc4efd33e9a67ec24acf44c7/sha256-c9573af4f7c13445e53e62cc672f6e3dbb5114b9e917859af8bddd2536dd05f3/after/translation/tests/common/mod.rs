//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and calls only their exported
//! `parse_number` symbol — the Rust implementation is never linked directly, so
//! the `#[no_mangle] extern "C"` wrapper is part of what is under test.

#![allow(dead_code)]
#![allow(non_camel_case_types)]

use std::ffi::{c_double, c_int, c_uchar};
use std::path::PathBuf;
use std::sync::OnceLock;

pub type cJSON_bool = c_int;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct parse_buffer {
    pub content: *const c_uchar,
    pub length: usize,
    pub offset: usize,
    pub depth: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct cJSON {
    pub type_: c_int,
    pub valueint: c_int,
    pub valuedouble: c_double,
}

pub type ParseNumberFn = unsafe extern "C" fn(*mut cJSON, *mut parse_buffer) -> cJSON_bool;

/// Full observable outcome of one `parse_number` call.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Outcome {
    pub ret: c_int,
    pub type_: c_int,
    pub valueint: c_int,
    /// Bit pattern, so that `-0.0` vs `+0.0` and NaN payloads are compared exactly.
    pub valuedouble_bits: u64,
    pub buf_content_is_same: bool,
    pub buf_length: usize,
    pub buf_offset: usize,
    pub buf_depth: usize,
    /// The C never writes through `content`; the Rust must not either.
    pub data_after: Vec<u8>,
}

struct Libs {
    c: libloading::Library,
    rust: libloading::Library,
}

// Safety: the loaded libraries are leaked for the whole test-process lifetime.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    manifest_dir()
        .parent()
        .unwrap()
        .join("c_src/build/libdriver.so")
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let release = manifest_dir().join("target/release/libdriver.so");
    if release.exists() {
        return release;
    }
    manifest_dir().join("target/debug/libdriver.so")
}

fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let cp = c_so_path();
        let rp = rust_so_path();
        assert!(cp.exists(), "C .so not found at {cp:?}; build c_src first");
        assert!(
            rp.exists(),
            "Rust .so not found at {rp:?}; run `cargo build --release` first"
        );
        unsafe {
            Libs {
                c: libloading::Library::new(&cp).expect("dlopen C .so"),
                rust: libloading::Library::new(&rp).expect("dlopen Rust .so"),
            }
        }
    })
}

pub fn c_parse_number() -> ParseNumberFn {
    static F: OnceLock<ParseNumberFn> = OnceLock::new();
    *F.get_or_init(|| unsafe {
        let s: libloading::Symbol<'static, ParseNumberFn> =
            libs().c.get(b"parse_number\0").expect("C parse_number");
        *s
    })
}

pub fn rust_parse_number() -> ParseNumberFn {
    static F: OnceLock<ParseNumberFn> = OnceLock::new();
    *F.get_or_init(|| unsafe {
        let s: libloading::Symbol<'static, ParseNumberFn> = libs()
            .rust
            .get(b"parse_number\0")
            .expect("Rust parse_number");
        *s
    })
}

/// Distinctive poison so we can tell "field not written" from "field written".
pub const POISON_TYPE: c_int = 0x5A5A_5A5A;
pub const POISON_VALUEINT: c_int = -0x3C3C_3C3C;
/// A signalling-NaN-ish bit pattern; must round-trip untouched on failure paths.
pub const POISON_DOUBLE_BITS: u64 = 0x7FF0_0000_DEAD_BEEF;

#[derive(Clone, Copy, Debug)]
pub struct ItemSeed {
    pub type_: c_int,
    pub valueint: c_int,
    pub valuedouble_bits: u64,
}

impl Default for ItemSeed {
    fn default() -> Self {
        ItemSeed {
            type_: POISON_TYPE,
            valueint: POISON_VALUEINT,
            valuedouble_bits: POISON_DOUBLE_BITS,
        }
    }
}

/// One differential scenario: a byte buffer plus the struct fields to pass in.
#[derive(Clone, Debug)]
pub struct Scenario {
    /// Bytes actually allocated (may be longer than `length`, to hold poison).
    pub data: Vec<u8>,
    pub length: usize,
    pub offset: usize,
    pub depth: usize,
    pub item: ItemSeed,
}

impl Scenario {
    pub fn new(data: Vec<u8>) -> Self {
        let length = data.len();
        Scenario {
            data,
            length,
            offset: 0,
            depth: 0,
            item: ItemSeed::default(),
        }
    }

    /// `s` plus a NUL terminator inside the allocation, `length` covering both.
    pub fn from_str_nul(s: &str) -> Self {
        let mut data = s.as_bytes().to_vec();
        data.push(0);
        Scenario::new(data)
    }

    /// `s` with no terminator at all: `length == s.len()`, poison bytes after.
    pub fn from_str_no_term(s: &str) -> Self {
        let mut data = s.as_bytes().to_vec();
        let length = data.len();
        data.extend_from_slice(b"999999999999999999");
        let mut sc = Scenario::new(data);
        sc.length = length;
        sc
    }

    pub fn length(mut self, l: usize) -> Self {
        self.length = l;
        self
    }
    pub fn offset(mut self, o: usize) -> Self {
        self.offset = o;
        self
    }
    pub fn depth(mut self, d: usize) -> Self {
        self.depth = d;
        self
    }
    pub fn item(mut self, i: ItemSeed) -> Self {
        self.item = i;
        self
    }
}

/// Run one scenario through `f`, returning the full observable outcome.
pub fn run(f: ParseNumberFn, sc: &Scenario) -> Outcome {
    // Private copy of the bytes so the two implementations cannot influence
    // each other, and so any write by the callee is detected.
    let mut data = sc.data.clone();
    let base = data.as_mut_ptr();
    let mut buf = parse_buffer {
        content: base,
        length: sc.length,
        offset: sc.offset,
        depth: sc.depth,
    };
    let mut item = cJSON {
        type_: sc.item.type_,
        valueint: sc.item.valueint,
        valuedouble: f64::from_bits(sc.item.valuedouble_bits),
    };
    let ret = unsafe { f(&mut item, &mut buf) };
    Outcome {
        ret,
        type_: item.type_,
        valueint: item.valueint,
        valuedouble_bits: item.valuedouble.to_bits(),
        buf_content_is_same: buf.content == base,
        buf_length: buf.length,
        buf_offset: buf.offset,
        buf_depth: buf.depth,
        data_after: data,
    }
}

/// Assert C and Rust agree on `sc`. `label` identifies the CONFIGS/ERRORS row.
pub fn assert_same(label: &str, sc: &Scenario) {
    let c = run(c_parse_number(), sc);
    let r = run(rust_parse_number(), sc);
    assert_eq!(
        c.data_after, sc.data,
        "[{label}] the C wrote through `content` (test-harness assumption broken)"
    );
    if c != r {
        panic!(
            "[{label}] DIVERGENCE\n  scenario: bytes={:?} (as text {:?})\n            \
             length={} offset={} depth={} item={:?}\n  C   : {:?}\n  Rust: {:?}",
            sc.data,
            String::from_utf8_lossy(&sc.data),
            sc.length,
            sc.offset,
            sc.depth,
            sc.item,
            c,
            r
        );
    }
    // The return value must be exactly 0 or 1 (cJSON true/false), not merely truthy.
    assert!(
        c.ret == 0 || c.ret == 1,
        "[{label}] C returned non-boolean {}",
        c.ret
    );
}

/// Deterministic PRNG (SplitMix64) — fixed seed keeps failures reproducible.
pub struct Rng(u64);

pub const SEED: u64 = 0x5EED_1234_ABCD_EF01;

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
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    pub fn range_incl(&mut self, lo: u64, hi: u64) -> u64 {
        lo + self.below(hi - lo + 1)
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u64) as usize]
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

/// The 15 bytes the C scan loop accepts (`switch` cases before `default`).
pub const ACCEPTED: [u8; 15] = [
    b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'+', b'-', b'e', b'E', b'.',
];

pub fn is_accepted(b: u8) -> bool {
    ACCEPTED.contains(&b)
}

pub fn random_item_seed(rng: &mut Rng) -> ItemSeed {
    ItemSeed {
        type_: rng.next_u64() as u32 as c_int,
        valueint: rng.next_u64() as u32 as c_int,
        valuedouble_bits: rng.next_u64(),
    }
}
