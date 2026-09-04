//! Phase C — differential tests for `pcre2_serialize.c`.
//!
//! `pcre2_serialize_encode` produces a self-contained byte stream that consists
//! of a 16-byte `pcre2_serialized_data` header (magic / version / config /
//! number_of_codes, pcre2_internal.h:2169), `TABLES_LENGTH` bytes of character
//! tables, and then one verbatim copy of every `pcre2_real_code` block. The C
//! code explicitly zeroes the three pointer-bearing fields of each copied block
//! (`memctl`, `tables`, `executable_jit`; pcre2_serialize.c:140-145) so that the
//! stream is fully position-independent. That is what makes the CROSS-DECODE
//! tests below legitimate: the blob embeds no library-specific pointer, so a
//! blob produced by C can be handed to the Rust decoder and vice versa.
mod common;

use common::diff::*;
use common::*;
use std::alloc::{alloc, dealloc, Layout};
use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};

// ------------------------------------------------------------------ constants
/// `sizeof(pcre2_serialized_data)` — pcre2_internal.h:2169-2174 (4 x 4 bytes).
const HDR: usize = 16;
/// `TABLES_LENGTH` = `ctypes_offset + 256` = `cbits_offset(512) + cbit_length(320) + 256`
/// — pcre2_internal.h:596-598.
const TABLES_LENGTH: usize = 512 + 320 + 256;
/// `offsetof(pcre2_real_code, blocksize)` — pcre2_intmodedep.h:660-665:
/// memctl(3 pointers = 24) + tables(8) + executable_jit(8) + start_bitmap[32].
const OFF_BLOCKSIZE: usize = 24 + 8 + 8 + 32;
/// `SERIALIZED_DATA_MAGIC` — pcre2_serialize.c:52.
const MAGIC: u32 = 0x5052_3253;

/// `PCRE2_ERROR_MIXEDTABLES` — pcre2.h:394.
const ERR_MIXEDTABLES: i32 = -30;
/// `PCRE2_ERROR_BADMODE` — pcre2.h:396.
const ERR_BADMODE: i32 = -32;
/// `PCRE2_ERROR_BADSERIALIZEDDATA` — pcre2.h:427.
const ERR_BADSERIALIZEDDATA: i32 = -62;

const INFO_NAMECOUNT: u32 = 17;
const INFO_NAMEENTRYSIZE: u32 = 18;
const INFO_NAMETABLE: u32 = 19;
const INFO_SIZE: u32 = 22;

const SENT: usize = 0xAAAA_AAAA_AAAA_AAAAu64 as usize;

// ============================================================ custom memctl
// A malloc/free pair of our own, so that the `gcontext != NULL` memctl path of
// pcre2_serialize.c (lines 82-83, 107, 165-166, 191, 217) is exercised in both
// libraries. The size is stashed in a 16-byte header so that `dealloc` can be
// given the matching `Layout`; 16 bytes keeps the returned pointer 16-aligned.
static ALLOCS: AtomicUsize = AtomicUsize::new(0);
static FREES: AtomicUsize = AtomicUsize::new(0);
const HEADER: usize = 16;

unsafe extern "C" fn my_malloc(size: usize, _data: *mut c_void) -> *mut c_void {
    let total = size + HEADER;
    let layout = Layout::from_size_align(total, 16).unwrap();
    let p = alloc(layout);
    if p.is_null() {
        return std::ptr::null_mut();
    }
    *(p as *mut usize) = total;
    ALLOCS.fetch_add(1, Ordering::Relaxed);
    p.add(HEADER) as *mut c_void
}

unsafe extern "C" fn my_free(ptr: *mut c_void, _data: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    FREES.fetch_add(1, Ordering::Relaxed);
    let base = (ptr as *mut u8).sub(HEADER);
    let total = *(base as *mut usize);
    dealloc(base, Layout::from_size_align(total, 16).unwrap());
}

unsafe fn my_gcontext(api: &Api) -> *mut c_void {
    let cx = (api.general_context_create)(
        Some(my_malloc),
        Some(my_free),
        std::ptr::null_mut(),
    );
    assert!(!cx.is_null(), "{}: general_context_create failed", api.name);
    cx
}

// ============================================================ info snapshot
/// Every `pcre2_pattern_info` selector, captured as raw bytes so that a decoded
/// code can be compared against the original inside ONE library, and so that a
/// C-decoded code can be compared against a Rust-decoded one.
#[derive(PartialEq, Eq, Debug, Clone)]
struct InfoSnap {
    scalars: Vec<(u32, i32, [u8; 24])>,
    firstbitmap: (i32, bool, Vec<u8>),
    nametable: (i32, bool, Vec<u8>),
}

unsafe fn info_snap(api: &Api, code: *mut c_void) -> InfoSnap {
    let mut scalars = Vec::new();
    for what in 0u32..=26 {
        if what == 7 || what == INFO_NAMETABLE {
            continue; // pointer-valued, handled below
        }
        let mut buf = [0xAAu8; 24];
        let rc = (api.pattern_info)(code, what, buf.as_mut_ptr() as *mut c_void);
        scalars.push((what, rc, buf));
    }
    let firstbitmap = {
        let mut p: *const u8 = std::ptr::null();
        let rc = (api.pattern_info)(code, 7, &mut p as *mut _ as *mut c_void);
        if rc == 0 && !p.is_null() {
            (rc, true, std::slice::from_raw_parts(p, 32).to_vec())
        } else {
            (rc, false, Vec::new())
        }
    };
    let nametable = {
        let mut cnt: u32 = 0;
        let mut esz: u32 = 0;
        (api.pattern_info)(code, INFO_NAMECOUNT, &mut cnt as *mut _ as *mut c_void);
        (api.pattern_info)(code, INFO_NAMEENTRYSIZE, &mut esz as *mut _ as *mut c_void);
        let mut p: *const u8 = std::ptr::null();
        let rc = (api.pattern_info)(code, INFO_NAMETABLE, &mut p as *mut _ as *mut c_void);
        if rc == 0 && cnt > 0 && !p.is_null() {
            let n = (cnt * esz) as usize;
            (rc, true, std::slice::from_raw_parts(p, n).to_vec())
        } else {
            (rc, false, Vec::new())
        }
    };
    InfoSnap { scalars, firstbitmap, nametable }
}

unsafe fn blocksize_of(api: &Api, code: *mut c_void) -> usize {
    let mut v: usize = 0;
    let rc = (api.pattern_info)(code, INFO_SIZE, &mut v as *mut _ as *mut c_void);
    assert_eq!(rc, 0, "{}: pattern_info(SIZE) failed", api.name);
    v
}

// ==================================================================== blobs
/// An encoded byte stream, freed with the library that produced it (the memctl
/// used for the allocation is hidden in the bytes immediately BEFORE the
/// returned pointer — pcre2_serialize.c:110-112, 290-291).
struct Blob {
    api: &'static Api,
    ptr: *mut u8,
    len: usize,
}

impl Blob {
    fn bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl Drop for Blob {
    fn drop(&mut self) {
        unsafe {
            if !self.ptr.is_null() {
                (self.api.serialize_free)(self.ptr);
            }
        }
    }
}

unsafe fn encode(
    api: &'static Api,
    codes: &[*mut c_void],
    gcontext: *mut c_void,
) -> (i32, usize, Option<Blob>) {
    let v: Vec<*const c_void> = codes.iter().map(|&p| p as *const c_void).collect();
    let mut ptr: *mut u8 = std::ptr::null_mut();
    let mut len = SENT;
    let rc = (api.serialize_encode)(
        v.as_ptr(),
        codes.len() as i32,
        &mut ptr,
        &mut len,
        gcontext,
    );
    if rc > 0 {
        assert!(!ptr.is_null(), "{}: encode rc={} but NULL blob", api.name, rc);
        (rc, len, Some(Blob { api, ptr, len }))
    } else {
        assert!(ptr.is_null(), "{}: encode rc={} but non-NULL blob", api.name, rc);
        (rc, len, None)
    }
}

/// Decoded codes, freed with the library that decoded them. ALL of them must be
/// freed for the shared table block's reference count to reach zero.
struct Decoded {
    api: &'static Api,
    codes: Vec<*mut c_void>,
}

impl Drop for Decoded {
    fn drop(&mut self) {
        unsafe {
            for &c in &self.codes {
                if !c.is_null() {
                    (self.api.code_free)(c);
                }
            }
        }
    }
}

unsafe fn decode(
    api: &'static Api,
    blob: *const u8,
    want: i32,
    gcontext: *mut c_void,
) -> (i32, Decoded) {
    let cap = if want > 0 { want as usize } else { 1 };
    let mut codes: Vec<*mut c_void> = vec![std::ptr::null_mut(); cap];
    let rc = (api.serialize_decode)(codes.as_mut_ptr(), want, blob, gcontext);
    if rc > 0 {
        codes.truncate(rc as usize);
        for (i, &c) in codes.iter().enumerate() {
            assert!(!c.is_null(), "{}: decode rc={} left codes[{}] NULL", api.name, rc, i);
        }
    } else {
        // On failure the C code sets every already-written slot back to NULL
        // (pcre2_serialize.c:254-258), so nothing is owned.
        for (i, &c) in codes.iter().enumerate() {
            assert!(c.is_null(), "{}: decode rc={} left codes[{}] non-NULL", api.name, rc, i);
        }
        codes.clear();
    }
    (rc, Decoded { api, codes })
}

/// A 8-byte-aligned copy of a blob, so that the `uint32_t` header fields can be
/// read by the library without an unaligned access, and can be corrupted here.
struct Aligned {
    words: Vec<u64>,
    len: usize,
}

impl Aligned {
    fn from(bytes: &[u8]) -> Aligned {
        let mut words = vec![0u64; (bytes.len() + 7) / 8 + 1];
        unsafe {
            std::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                words.as_mut_ptr() as *mut u8,
                bytes.len(),
            );
        }
        Aligned { words, len: bytes.len() }
    }
    fn ptr(&self) -> *const u8 {
        self.words.as_ptr() as *const u8
    }
    fn as_mut(&mut self) -> &mut [u8] {
        unsafe {
            std::slice::from_raw_parts_mut(self.words.as_mut_ptr() as *mut u8, self.len)
        }
    }
}

// =================================================================== corpus
const PATTERNS: &[&str] = &[
    "abc",
    "a*",
    "(a)(b)(c)",
    "^(?<year>\\d{4})-(?<month>\\d{2})-(?<day>\\d{2})$",
    "(?i)hello\\s+world",
    "[a-z0-9_]+",
    "(?:foo|bar|baz)+",
    "\\bword\\b",
    "(?J)(?<n>a)|(?<n>b)",
    "((((((a))))))",
    "a{2,10}b{3,}c?",
    "(?m)^line$",
    "(?s).*",
    "(?x) a  b  c   # comment",
    "\\p{L}+\\p{N}*",
    "(?=lookahead)x",
    "(?<=behind)y",
    "(?>atomic)z",
    "(a)\\1\\1",
    "(?<name>x)\\k<name>",
    "(?(?=a)b|c)",
    "(?R)?",
    "a(?R)?b",
    "(*MARK:tag)abc",
    "(*ACCEPT)",
    "\\R\\X\\N",
    "[[:alpha:][:digit:]]+",
    "(?<a>1)(?<bb>2)(?<ccc>3)(?<dddd>4)",
    "^(?:[a-z0-9!#$%&'*+/=?^_`{|}~-]+)@(?:[a-z0-9-]+\\.)+[a-z]{2,}$",
    "((a+)*)+b",
    "[\\x{20}-\\x{7e}]+",
    "(?:(?:(?:(?:a))))",
    "x|y|z|w|v|u|t|s|r|q",
    "\\d{1,3}(?:\\.\\d{1,3}){3}",
    "(?<big>(?:[^\\\\\"]|\\\\.)*)",
    "z",
];

const SUBJECTS: &[&str] = &[
    "",
    "a",
    "abc",
    "aaa",
    "hello   world",
    "2024-01-31",
    "word here",
    "foobarbaz",
    "x",
    "lookaheadx",
    "behindy",
    "atomicz",
    "1234",
    "user@example.com",
    "127.0.0.1",
    "line",
    "\u{00e9}\u{20ac}",
    "abcdefgh",
];

/// Compile one pattern in each library (already fully diffed by `compile_both`).
struct Pair {
    cc: Compiled,
    rr: Compiled,
    label: String,
}

unsafe fn build_pairs(pats: &[&str], cfg: &CompileCfg, tag: &str) -> Vec<Pair> {
    let mut out = Vec::new();
    for pat in pats {
        let label = format!("{} pattern={:?}", tag, pat);
        let (cc, rr) = compile_both(pat.as_bytes(), pat.len(), cfg, &label);
        if cc.code.is_null() {
            continue;
        }
        out.push(Pair { cc, rr, label });
    }
    out
}

/// `offsetof(pcre2_real_code, flags)` — blocksize(72) + code_start(8) +
/// magic_number(4) + compile_options(4) + overall_options(4) + extra_options(4).
const OFF_FLAGS: usize = OFF_BLOCKSIZE + 8 + 8 + 4 + 4 + 4 + 4;
/// `PCRE2_DEREF_TABLES` — pcre2_internal.h:520.
const PCRE2_DEREF_TABLES: u32 = 0x0004_0000;

/// Assert that `after` equals `before` except that every code block's `flags`
/// field has gained exactly `PCRE2_DEREF_TABLES`.
unsafe fn assert_diff_is_only_deref_tables(
    api: &Api,
    codes: &[*mut c_void],
    before: &[u8],
    after: &[u8],
) {
    assert_eq!(before.len(), after.len(), "{}: blob length changed", api.name);
    let mut allowed = std::collections::HashSet::new();
    let mut off = HDR + TABLES_LENGTH;
    for &code in codes {
        let f = off + OFF_FLAGS;
        let fb = u32::from_ne_bytes(before[f..f + 4].try_into().unwrap());
        let fa = u32::from_ne_bytes(after[f..f + 4].try_into().unwrap());
        assert_eq!(
            fa,
            fb | PCRE2_DEREF_TABLES,
            "{}: decoded flags at blob offset {} should be original|DEREF_TABLES",
            api.name,
            f
        );
        for k in f..f + 4 {
            allowed.insert(k);
        }
        off += blocksize_of(api, code);
    }
    assert_eq!(off, before.len(), "{}: blob not fully walked", api.name);
    for i in 0..before.len() {
        if before[i] != after[i] {
            assert!(
                allowed.contains(&i),
                "{}: re-encoded blob differs at byte {} outside the flags fields \
                 (before={:#04x} after={:#04x})",
                api.name,
                i,
                before[i],
                after[i]
            );
        }
    }
}

/// Compare the observable behaviour of two codes (possibly in two different
/// libraries) — the full `pattern_info` snapshot plus real matches.
unsafe fn assert_codes_behave_same(
    a_api: &'static Api,
    a: *mut c_void,
    b_api: &'static Api,
    b: *mut c_void,
    label: &str,
) {
    let sa = info_snap(a_api, a);
    let sb = info_snap(b_api, b);
    if sa.scalars != sb.scalars {
        let bad: Vec<_> = sa
            .scalars
            .iter()
            .zip(sb.scalars.iter())
            .filter(|(x, y)| x != y)
            .map(|(x, y)| (x.0, x.1, y.1, x.2, y.2))
            .collect();
        panic!(
            "{}: pattern_info differs between {} and {}: {:?}",
            label, a_api.name, b_api.name, bad
        );
    }
    assert_eq!(
        sa.firstbitmap, sb.firstbitmap,
        "{}: FIRSTBITMAP differs ({} vs {})",
        label, a_api.name, b_api.name
    );
    assert_eq!(
        sa.nametable, sb.nametable,
        "{}: NAMETABLE differs ({} vs {})",
        label, a_api.name, b_api.name
    );

    for subj in SUBJECTS {
        let s = subj.as_bytes();
        for &engine in &[Engine::Interpreter, Engine::Dfa] {
            for &start in &[0usize, s.len()] {
                let mo = run_match(a_api, a, s, s.len(), start, &MatchCfg::new(0), engine);
                let mb = run_match(b_api, b, s, s.len(), start, &MatchCfg::new(0), engine);
                assert_eq!(
                    mo, mb,
                    "{}: {:?} match differs for subject={:?} start={} ({} vs {})",
                    label, engine, subj, start, a_api.name, b_api.name
                );
            }
        }
    }
}

// ==================================================================== tests

/// `serialize_encode` of 1, 2, 3, ... N codes (up to 30) must produce byte
/// identical blobs in both libraries, agreeing on `*serialized_size` and on the
/// return value, and matching the size formula of pcre2_serialize.c:91/103.
#[test]
fn serialize_encode_blobs_identical() {
    unsafe {
        let (c, r) = both();
        let pairs = build_pairs(PATTERNS, &CompileCfg::new(0), "encode");
        assert!(pairs.len() >= 30, "corpus too small: {}", pairs.len());

        for n in [1usize, 2, 3, 4, 5, 7, 11, 17, 30] {
            let ccodes: Vec<*mut c_void> = pairs[..n].iter().map(|p| p.cc.code).collect();
            let rcodes: Vec<*mut c_void> = pairs[..n].iter().map(|p| p.rr.code).collect();

            let (crc, clen, cb) = encode(c, &ccodes, std::ptr::null_mut());
            let (rrc, rlen, rb) = encode(r, &rcodes, std::ptr::null_mut());
            assert_eq!(crc, rrc, "encode({}) rc differs (C={} Rust={})", n, crc, rrc);
            assert_eq!(crc, n as i32, "encode({}) should return the code count", n);
            assert_eq!(clen, rlen, "encode({}) *serialized_size differs", n);

            let cb = cb.unwrap();
            let rb = rb.unwrap();
            assert_eq!(cb.len, clen, "encode({}) blob length vs reported size", n);

            // the C size formula: header + tables + sum of blocksizes
            let sum: usize = ccodes.iter().map(|&x| blocksize_of(c, x)).sum();
            assert_eq!(
                clen,
                HDR + TABLES_LENGTH + sum,
                "encode({}) size formula (pcre2_serialize.c:91,103)",
                n
            );

            let a = cb.bytes();
            let b = rb.bytes();
            if a != b {
                let i = a.iter().zip(b.iter()).position(|(x, y)| x != y).unwrap();
                panic!(
                    "encode({}) blobs differ at byte {} (C={:#04x} Rust={:#04x})\n\
                     C[{}..]={:02x?}\nR[{}..]={:02x?}",
                    n,
                    i,
                    a[i],
                    b[i],
                    i,
                    &a[i..(i + 32).min(a.len())],
                    i,
                    &b[i..(i + 32).min(b.len())],
                );
            }

            // header sanity: magic + number_of_codes (pcre2_serialize.c:115-118)
            assert_eq!(
                u32::from_ne_bytes(a[0..4].try_into().unwrap()),
                MAGIC,
                "encode({}) magic",
                n
            );
            assert_eq!(
                i32::from_ne_bytes(a[12..16].try_into().unwrap()),
                n as i32,
                "encode({}) header number_of_codes",
                n
            );
            // the three pointer fields of each code copy must be zeroed
            // (pcre2_serialize.c:140-145) and blocksize must agree with
            // pattern_info(SIZE) -- this also validates OFF_BLOCKSIZE.
            let mut off = HDR + TABLES_LENGTH;
            for (i, &code) in ccodes.iter().enumerate() {
                let bs = blocksize_of(c, code);
                assert!(
                    a[off..off + 40].iter().all(|&x| x == 0),
                    "encode({}) code {}: memctl/tables/executable_jit not zeroed",
                    n,
                    i
                );
                let stored = usize::from_ne_bytes(
                    a[off + OFF_BLOCKSIZE..off + OFF_BLOCKSIZE + 8].try_into().unwrap(),
                );
                assert_eq!(
                    stored, bs,
                    "encode({}) code {}: embedded blocksize vs pattern_info(SIZE)",
                    n, i
                );
                off += bs;
            }
            assert_eq!(off, clen, "encode({}) blob fully consumed", n);

            // ---- serialize_get_number_of_codes, each lib on each blob
            for (who, blob) in [(c, &cb), (r, &rb)] {
                for reader in [c, r] {
                    let got = (reader.serialize_get_number_of_codes)(blob.ptr);
                    assert_eq!(
                        got, n as i32,
                        "get_number_of_codes: {} blob read by {} gave {}",
                        who.name, reader.name, got
                    );
                }
            }
        }
    }
}

/// `serialize_get_number_of_codes` error paths.
#[test]
fn serialize_get_number_of_codes_errors() {
    unsafe {
        let (c, r) = both();
        let pairs = build_pairs(&PATTERNS[..3], &CompileCfg::new(0), "gnoc");
        let ccodes: Vec<*mut c_void> = pairs.iter().map(|p| p.cc.code).collect();
        let (_, _, cb) = encode(c, &ccodes, std::ptr::null_mut());
        let cb = cb.unwrap();

        // NULL bytes -> PCRE2_ERROR_NULL (pcre2_serialize.c:272)
        assert_eq!(
            (c.serialize_get_number_of_codes)(std::ptr::null()),
            (r.serialize_get_number_of_codes)(std::ptr::null()),
            "get_number_of_codes(NULL)"
        );
        assert_eq!(
            (c.serialize_get_number_of_codes)(std::ptr::null()),
            ERR_NULL,
            "get_number_of_codes(NULL) should be PCRE2_ERROR_NULL"
        );

        for (what, off, len) in [("magic", 0usize, 4usize), ("version", 4, 4), ("config", 8, 4)] {
            let mut a = Aligned::from(cb.bytes());
            for b in &mut a.as_mut()[off..off + len] {
                *b ^= 0xFF;
            }
            let cr = (c.serialize_get_number_of_codes)(a.ptr());
            let rr2 = (r.serialize_get_number_of_codes)(a.ptr());
            assert_eq!(cr, rr2, "get_number_of_codes with corrupt {}", what);
            let expect = if what == "magic" { ERR_BADMAGIC } else { ERR_BADMODE };
            assert_eq!(cr, expect, "get_number_of_codes with corrupt {}", what);
        }
    }
}

/// Round-trip inside a single library: the decoded code must be
/// indistinguishable from the original. Then compare the C-decoded code against
/// the Rust-decoded one.
#[test]
fn serialize_decode_roundtrip() {
    unsafe {
        let (c, r) = both();
        for &opts in &[0u32, PCRE2_UTF, PCRE2_CASELESS | PCRE2_MULTILINE, PCRE2_DUPNAMES]
        {
            let pairs = build_pairs(
                PATTERNS,
                &CompileCfg::new(opts),
                &format!("roundtrip opts={:#x}", opts),
            );
            for chunk in pairs.chunks(6) {
                let ccodes: Vec<*mut c_void> = chunk.iter().map(|p| p.cc.code).collect();
                let rcodes: Vec<*mut c_void> = chunk.iter().map(|p| p.rr.code).collect();
                let n = chunk.len() as i32;
                let (_, _, cb) = encode(c, &ccodes, std::ptr::null_mut());
                let (_, _, rb) = encode(r, &rcodes, std::ptr::null_mut());
                let cb = cb.unwrap();
                let rb = rb.unwrap();
                assert_eq!(cb.bytes(), rb.bytes(), "roundtrip: blobs differ");

                let (cdrc, cdec) = decode(c, cb.ptr, n, std::ptr::null_mut());
                let (rdrc, rdec) = decode(r, rb.ptr, n, std::ptr::null_mut());
                assert_eq!(cdrc, rdrc, "roundtrip: decode rc differs");
                assert_eq!(cdrc, n, "roundtrip: decode should return {}", n);

                for (i, p) in chunk.iter().enumerate() {
                    // decoded == original, inside each library
                    assert_codes_behave_same(
                        c,
                        p.cc.code,
                        c,
                        cdec.codes[i],
                        &format!("{} C original-vs-decoded", p.label),
                    );
                    assert_codes_behave_same(
                        r,
                        p.rr.code,
                        r,
                        rdec.codes[i],
                        &format!("{} Rust original-vs-decoded", p.label),
                    );
                    // C-decoded == Rust-decoded
                    assert_codes_behave_same(
                        c,
                        cdec.codes[i],
                        r,
                        rdec.codes[i],
                        &format!("{} C-decoded-vs-Rust-decoded", p.label),
                    );
                }

                // re-encoding the decoded codes must reproduce the same blob
                let (crc2, _, cb2) = encode(c, &cdec.codes, std::ptr::null_mut());
                let (rrc2, _, rb2) = encode(r, &rdec.codes, std::ptr::null_mut());
                assert_eq!(crc2, rrc2, "roundtrip: re-encode rc differs");
                let cb2 = cb2.unwrap();
                let rb2 = rb2.unwrap();
                assert_eq!(cb2.bytes(), rb2.bytes(), "roundtrip: re-encoded blobs differ");
                // The re-encoded blob is identical to the original EXCEPT that
                // `pcre2_serialize_decode` ORs PCRE2_DEREF_TABLES (0x00040000,
                // pcre2_internal.h:520) into every decoded code's `flags`
                // (pcre2_serialize.c:241). Verify that this is the ONLY
                // difference, in both libraries.
                assert_diff_is_only_deref_tables(c, &ccodes, cb.bytes(), cb2.bytes());
                assert_diff_is_only_deref_tables(r, &rcodes, rb.bytes(), rb2.bytes());
            }
        }
    }
}

/// CROSS-DECODE: the blob has no library-specific pointers in it
/// (pcre2_serialize.c:140-145), so the C blob can be decoded by Rust and the
/// Rust blob by C. The decoded codes must behave identically.
#[test]
fn serialize_cross_decode() {
    unsafe {
        let (c, r) = both();
        for &opts in &[0u32, PCRE2_UTF | PCRE2_UCP, PCRE2_DUPNAMES] {
            let pairs = build_pairs(
                PATTERNS,
                &CompileCfg::new(opts),
                &format!("cross opts={:#x}", opts),
            );
            for chunk in pairs.chunks(5) {
                let ccodes: Vec<*mut c_void> = chunk.iter().map(|p| p.cc.code).collect();
                let rcodes: Vec<*mut c_void> = chunk.iter().map(|p| p.rr.code).collect();
                let n = chunk.len() as i32;
                let (_, _, cb) = encode(c, &ccodes, std::ptr::null_mut());
                let (_, _, rb) = encode(r, &rcodes, std::ptr::null_mut());
                let cb = cb.unwrap();
                let rb = rb.unwrap();
                assert_eq!(cb.bytes(), rb.bytes(), "cross: blobs differ");

                // C blob -> RUST decoder
                let (rc1, rdec) = decode(r, cb.ptr, n, std::ptr::null_mut());
                // Rust blob -> C decoder
                let (rc2, cdec) = decode(c, rb.ptr, n, std::ptr::null_mut());
                assert_eq!(rc1, n, "cross: Rust decoding the C blob returned {}", rc1);
                assert_eq!(rc2, n, "cross: C decoding the Rust blob returned {}", rc2);

                for (i, p) in chunk.iter().enumerate() {
                    assert_codes_behave_same(
                        r,
                        rdec.codes[i],
                        r,
                        p.rr.code,
                        &format!("{} Rust-decoded-C-blob vs Rust original", p.label),
                    );
                    assert_codes_behave_same(
                        c,
                        cdec.codes[i],
                        c,
                        p.cc.code,
                        &format!("{} C-decoded-Rust-blob vs C original", p.label),
                    );
                    assert_codes_behave_same(
                        c,
                        cdec.codes[i],
                        r,
                        rdec.codes[i],
                        &format!("{} cross-decoded pair", p.label),
                    );
                }
            }
        }
    }
}

/// `number_of_codes` smaller than, equal to and larger than the blob's actual
/// count (pcre2_serialize.c:177, 183-184).
#[test]
fn serialize_decode_count_variants() {
    unsafe {
        let (c, r) = both();
        let pairs = build_pairs(&PATTERNS[..8], &CompileCfg::new(0), "count");
        let n = pairs.len();
        let ccodes: Vec<*mut c_void> = pairs.iter().map(|p| p.cc.code).collect();
        let rcodes: Vec<*mut c_void> = pairs.iter().map(|p| p.rr.code).collect();
        let (_, _, cb) = encode(c, &ccodes, std::ptr::null_mut());
        let (_, _, rb) = encode(r, &rcodes, std::ptr::null_mut());
        let cb = cb.unwrap();
        let rb = rb.unwrap();

        for want in [-5i32, -1, 0, 1, 2, 3, n as i32 - 1, n as i32, n as i32 + 1, n as i32 + 40]
        {
            let (crc, cdec) = decode(c, cb.ptr, want, std::ptr::null_mut());
            let (rrc, rdec) = decode(r, rb.ptr, want, std::ptr::null_mut());
            assert_eq!(
                crc, rrc,
                "decode(want={}) rc differs (C={} Rust={})",
                want, crc, rrc
            );
            let expect = if want <= 0 {
                ERR_BADDATA // pcre2_serialize.c:177
            } else {
                want.min(n as i32) // clamped at pcre2_serialize.c:183-184
            };
            assert_eq!(crc, expect, "decode(want={}) unexpected rc", want);
            for i in 0..cdec.codes.len() {
                assert_codes_behave_same(
                    c,
                    cdec.codes[i],
                    r,
                    rdec.codes[i],
                    &format!("count want={} index {}", want, i),
                );
                assert_codes_behave_same(
                    c,
                    cdec.codes[i],
                    c,
                    pairs[i].cc.code,
                    &format!("count want={} index {} vs original", want, i),
                );
            }
        }
    }
}

/// Encode AND decode through a general context carrying our own malloc/free, so
/// that the `gcontext != NULL` memctl path is taken in both libraries. The blob
/// must still be byte identical, and every allocation must be released.
#[test]
fn serialize_custom_general_context() {
    let _guard = global_lock();
    unsafe {
        let (c, r) = both();
        let a0 = ALLOCS.load(Ordering::Relaxed);
        let f0 = FREES.load(Ordering::Relaxed);

        let pairs = build_pairs(PATTERNS, &CompileCfg::new(0), "gctx");
        for chunk in pairs.chunks(9) {
            let ccodes: Vec<*mut c_void> = chunk.iter().map(|p| p.cc.code).collect();
            let rcodes: Vec<*mut c_void> = chunk.iter().map(|p| p.rr.code).collect();
            let n = chunk.len() as i32;

            let cgx = my_gcontext(c);
            let rgx = my_gcontext(r);

            let (crc, clen, cb) = encode(c, &ccodes, cgx);
            let (rrc, rlen, rb) = encode(r, &rcodes, rgx);
            assert_eq!(crc, rrc, "gctx: encode rc differs");
            assert_eq!(clen, rlen, "gctx: encode size differs");
            let cb = cb.unwrap();
            let rb = rb.unwrap();
            assert_eq!(
                cb.bytes(),
                rb.bytes(),
                "gctx: blobs differ with a custom general context"
            );

            // The blob content must not depend on which allocator produced it.
            let (_, _, cb_def) = encode(c, &ccodes, std::ptr::null_mut());
            let cb_def = cb_def.unwrap();
            assert_eq!(
                cb.bytes(),
                cb_def.bytes(),
                "gctx: blob differs from the default-allocator blob"
            );
            drop(cb_def);

            // decode through the custom context too (both same-library and cross)
            {
                let (crc2, cdec) = decode(c, cb.ptr, n, cgx);
                let (rrc2, rdec) = decode(r, rb.ptr, n, rgx);
                assert_eq!(crc2, rrc2, "gctx: decode rc differs");
                assert_eq!(crc2, n, "gctx: decode returned {}", crc2);
                for i in 0..chunk.len() {
                    assert_codes_behave_same(
                        c,
                        cdec.codes[i],
                        r,
                        rdec.codes[i],
                        &format!("gctx decoded {}", chunk[i].label),
                    );
                    assert_codes_behave_same(
                        c,
                        cdec.codes[i],
                        c,
                        chunk[i].cc.code,
                        &format!("gctx decoded vs original {}", chunk[i].label),
                    );
                }
                // cross-decode with the custom allocator as well
                let (xrc, xdec) = decode(r, cb.ptr, n, rgx);
                assert_eq!(xrc, n, "gctx: Rust cross-decode returned {}", xrc);
                for i in 0..chunk.len() {
                    assert_codes_behave_same(
                        r,
                        xdec.codes[i],
                        c,
                        cdec.codes[i],
                        &format!("gctx cross-decoded {}", chunk[i].label),
                    );
                }
            }

            drop(cb);
            drop(rb);
            (c.general_context_free)(cgx);
            (r.general_context_free)(rgx);
        }

        let da = ALLOCS.load(Ordering::Relaxed) - a0;
        let df = FREES.load(Ordering::Relaxed) - f0;
        assert!(da > 0, "gctx: the custom allocator was never used");
        assert_eq!(da, df, "gctx: {} allocations but {} frees (leak)", da, df);
    }
}

/// Codes compiled against the library's OWN `pcre2_maketables()` output mix
/// badly with default-tables codes: pcre2_serialize.c:99-102 returns
/// `PCRE2_ERROR_MIXEDTABLES`. Two codes sharing one `maketables()` block encode
/// fine, and the resulting blob must still be identical between the libraries
/// because `pcre2_maketables()` reproduces the built-in tables byte for byte in
/// the C locale.
#[test]
fn serialize_mixed_tables() {
    unsafe {
        let (c, r) = both();

        // (a) default-tables code + own-tables code -> MIXEDTABLES
        let def = build_pairs(&["abc"], &CompileCfg::new(0), "mixed-def");
        let own = build_pairs(&["(a)(b)"], &CompileCfg::new(0).own_tables(), "mixed-own");
        assert_eq!(def.len(), 1);
        assert_eq!(own.len(), 1);

        for (api, get) in [
            (c, (def[0].cc.code, own[0].cc.code)),
            (r, (def[0].rr.code, own[0].rr.code)),
        ] {
            for order in [[get.0, get.1], [get.1, get.0]] {
                let (rc, len, blob) = encode(api, &order, std::ptr::null_mut());
                assert_eq!(
                    rc, ERR_MIXEDTABLES,
                    "{}: mixing default and maketables() codes should give MIXEDTABLES, got {}",
                    api.name, rc
                );
                assert_eq!(len, SENT, "{}: MIXEDTABLES must not write the size", api.name);
                assert!(blob.is_none());
            }
            // a single own-tables code on its own is fine
            let (rc, _, blob) = encode(api, &[get.1], std::ptr::null_mut());
            assert_eq!(rc, 1, "{}: single own-tables code should encode", api.name);
            drop(blob);
        }

        // (b) two codes compiled with two SEPARATE maketables() blocks also
        // mismatch, because line 101 compares the POINTERS.
        let own2 = build_pairs(&["xyz"], &CompileCfg::new(0).own_tables(), "mixed-own2");
        for (api, codes) in [
            (c, [own[0].cc.code, own2[0].cc.code]),
            (r, [own[0].rr.code, own2[0].rr.code]),
        ] {
            let (rc, _, blob) = encode(api, &codes, std::ptr::null_mut());
            assert_eq!(
                rc, ERR_MIXEDTABLES,
                "{}: two distinct maketables() blocks should give MIXEDTABLES",
                api.name
            );
            assert!(blob.is_none());
        }

        // (c) two codes SHARING one maketables() block encode successfully and
        // the two libraries agree byte for byte.
        let mut blobs: Vec<Vec<u8>> = Vec::new();
        for api in [c, r] {
            let tables = (api.maketables)(std::ptr::null_mut());
            assert!(!tables.is_null(), "{}: maketables failed", api.name);
            let cx = (api.compile_context_create)(std::ptr::null_mut());
            assert_eq!((api.set_character_tables)(cx, tables), 0);
            let mut codes = Vec::new();
            for pat in ["abc", "(a)(b)", "(?<n>x)+"] {
                let mut ec = 0i32;
                let mut eo = 0usize;
                let code = (api.compile)(
                    pat.as_ptr(),
                    pat.len(),
                    0,
                    &mut ec,
                    &mut eo,
                    cx,
                );
                assert!(!code.is_null(), "{}: compile({:?}) failed ec={}", api.name, pat, ec);
                codes.push(code);
            }
            let (rc, len, blob) = encode(api, &codes, std::ptr::null_mut());
            assert_eq!(rc, 3, "{}: shared-tables encode rc", api.name);
            let blob = blob.unwrap();
            assert_eq!(blob.len, len);
            blobs.push(blob.bytes().to_vec());

            // decoding it must still work
            let (drc, dec) = decode(api, blob.ptr, 3, std::ptr::null_mut());
            assert_eq!(drc, 3, "{}: shared-tables decode rc", api.name);
            for (i, &orig) in codes.iter().enumerate() {
                assert_codes_behave_same(
                    api,
                    orig,
                    api,
                    dec.codes[i],
                    &format!("{} shared-tables code {}", api.name, i),
                );
            }
            drop(dec);
            drop(blob);
            for code in codes {
                (api.code_free)(code);
            }
            (api.compile_context_free)(cx);
            (api.maketables_free)(std::ptr::null_mut(), tables);
        }
        assert_eq!(
            blobs[0], blobs[1],
            "shared-maketables blobs differ between C and Rust"
        );
    }
}

/// The argument-validation paths of `serialize_encode` / `serialize_decode`
/// (pcre2_serialize.c:85-88, 96-98, 176-181, 209-213, 229-235).
#[test]
fn serialize_error_paths() {
    unsafe {
        let (c, r) = both();
        let pairs = build_pairs(&PATTERNS[..4], &CompileCfg::new(0), "err");
        let ccodes: Vec<*mut c_void> = pairs.iter().map(|p| p.cc.code).collect();
        let rcodes: Vec<*mut c_void> = pairs.iter().map(|p| p.rr.code).collect();

        // ---- encode: NULL codes / NULL out-pointers
        for api in [c, r] {
            let mut p: *mut u8 = std::ptr::null_mut();
            let mut l = SENT;
            let rc = (api.serialize_encode)(
                std::ptr::null(),
                1,
                &mut p,
                &mut l,
                std::ptr::null_mut(),
            );
            assert_eq!(rc, ERR_NULL, "{}: encode(codes=NULL)", api.name);
            assert_eq!(l, SENT, "{}: encode(codes=NULL) wrote the size", api.name);
        }
        let cv: Vec<*const c_void> = ccodes.iter().map(|&x| x as *const c_void).collect();
        let rv: Vec<*const c_void> = rcodes.iter().map(|&x| x as *const c_void).collect();
        for (api, v) in [(c, &cv), (r, &rv)] {
            let mut l = SENT;
            let rc = (api.serialize_encode)(
                v.as_ptr(),
                1,
                std::ptr::null_mut(),
                &mut l,
                std::ptr::null_mut(),
            );
            assert_eq!(rc, ERR_NULL, "{}: encode(serialized_bytes=NULL)", api.name);

            let mut p: *mut u8 = std::ptr::null_mut();
            let rc = (api.serialize_encode)(
                v.as_ptr(),
                1,
                &mut p,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            assert_eq!(rc, ERR_NULL, "{}: encode(serialized_size=NULL)", api.name);

            // number_of_codes <= 0 -> PCRE2_ERROR_BADDATA (line 88)
            for n in [0i32, -1, -100, i32::MIN] {
                let mut p: *mut u8 = std::ptr::null_mut();
                let mut l = SENT;
                let rc = (api.serialize_encode)(
                    v.as_ptr(),
                    n,
                    &mut p,
                    &mut l,
                    std::ptr::null_mut(),
                );
                assert_eq!(rc, ERR_BADDATA, "{}: encode(n={})", api.name, n);
                assert_eq!(l, SENT, "{}: encode(n={}) wrote the size", api.name, n);
            }

            // a NULL entry in the middle of the array -> PCRE2_ERROR_NULL (line 96)
            let mut with_null = v.clone();
            with_null[2] = std::ptr::null();
            let mut p: *mut u8 = std::ptr::null_mut();
            let mut l = SENT;
            let rc = (api.serialize_encode)(
                with_null.as_ptr(),
                4,
                &mut p,
                &mut l,
                std::ptr::null_mut(),
            );
            assert_eq!(rc, ERR_NULL, "{}: encode with codes[2]=NULL", api.name);
        }

        // ---- decode: NULL arguments
        for api in [c, r] {
            let mut codes = [std::ptr::null_mut::<c_void>(); 4];
            let rc = (api.serialize_decode)(
                codes.as_mut_ptr(),
                1,
                std::ptr::null(),
                std::ptr::null_mut(),
            );
            assert_eq!(rc, ERR_NULL, "{}: decode(bytes=NULL)", api.name);
            let rc = (api.serialize_decode)(
                std::ptr::null_mut(),
                1,
                std::ptr::null(),
                std::ptr::null_mut(),
            );
            assert_eq!(rc, ERR_NULL, "{}: decode(codes=NULL, bytes=NULL)", api.name);
        }

        // ---- decode: corrupted headers and a corrupted blocksize
        let (_, _, cb) = encode(c, &ccodes, std::ptr::null_mut());
        let cb = cb.unwrap();

        let mut codes = [std::ptr::null_mut::<c_void>(); 8];
        for (what, off, expect) in [
            ("magic", 0usize, ERR_BADMAGIC),
            ("version", 4, ERR_BADMODE),
            ("config", 8, ERR_BADMODE),
        ] {
            let mut a = Aligned::from(cb.bytes());
            for b in &mut a.as_mut()[off..off + 4] {
                *b ^= 0xFF;
            }
            let crc = (c.serialize_decode)(codes.as_mut_ptr(), 4, a.ptr(), std::ptr::null_mut());
            let rrc = (r.serialize_decode)(codes.as_mut_ptr(), 4, a.ptr(), std::ptr::null_mut());
            assert_eq!(crc, rrc, "decode with corrupt {}: rc differs", what);
            assert_eq!(crc, expect, "decode with corrupt {}", what);
        }

        // number_of_codes in the header <= 0 -> BADSERIALIZEDDATA (line 178)
        for bad in [0i32, -1, -77] {
            let mut a = Aligned::from(cb.bytes());
            a.as_mut()[12..16].copy_from_slice(&bad.to_ne_bytes());
            let crc = (c.serialize_decode)(codes.as_mut_ptr(), 4, a.ptr(), std::ptr::null_mut());
            let rrc = (r.serialize_decode)(codes.as_mut_ptr(), 4, a.ptr(), std::ptr::null_mut());
            assert_eq!(crc, rrc, "decode with header count {}: rc differs", bad);
            assert_eq!(
                crc, ERR_BADSERIALIZEDDATA,
                "decode with header count {}",
                bad
            );
        }

        // blocksize <= sizeof(pcre2_real_code) -> BADSERIALIZEDDATA (line 209-213)
        for bad in [0usize, 1, 16, 96] {
            let mut a = Aligned::from(cb.bytes());
            let at = HDR + TABLES_LENGTH + OFF_BLOCKSIZE;
            a.as_mut()[at..at + 8].copy_from_slice(&bad.to_ne_bytes());
            let crc = (c.serialize_decode)(codes.as_mut_ptr(), 4, a.ptr(), std::ptr::null_mut());
            let rrc = (r.serialize_decode)(codes.as_mut_ptr(), 4, a.ptr(), std::ptr::null_mut());
            assert_eq!(crc, rrc, "decode with blocksize {}: rc differs", bad);
            assert_eq!(crc, ERR_BADSERIALIZEDDATA, "decode with blocksize {}", bad);
        }

        // a corrupted magic_number inside the SECOND code block must be detected
        // at line 229 and everything already decoded must be released (254-258).
        {
            let bs0 = blocksize_of(c, ccodes[0]);
            // magic_number sits right after blocksize + code_start
            let at = HDR + TABLES_LENGTH + bs0 + OFF_BLOCKSIZE + 16;
            let mut a = Aligned::from(cb.bytes());
            a.as_mut()[at] ^= 0xFF;
            let crc = (c.serialize_decode)(codes.as_mut_ptr(), 4, a.ptr(), std::ptr::null_mut());
            let rrc = (r.serialize_decode)(codes.as_mut_ptr(), 4, a.ptr(), std::ptr::null_mut());
            assert_eq!(crc, rrc, "decode with corrupt second magic_number: rc differs");
            assert_eq!(
                crc, ERR_BADSERIALIZEDDATA,
                "decode with corrupt second magic_number"
            );
            for (i, &p) in codes.iter().enumerate() {
                assert!(p.is_null(), "decode failure left codes[{}] non-NULL", i);
            }
        }
    }
}

/// `serialize_free` must tolerate NULL and must release the blob without
/// crashing, in both libraries and for blobs made with either allocator.
#[test]
fn serialize_free_paths() {
    let _guard = global_lock();
    unsafe {
        let (c, r) = both();
        (c.serialize_free)(std::ptr::null_mut());
        (r.serialize_free)(std::ptr::null_mut());

        let pairs = build_pairs(&PATTERNS[..6], &CompileCfg::new(0), "free");
        let ccodes: Vec<*mut c_void> = pairs.iter().map(|p| p.cc.code).collect();
        let rcodes: Vec<*mut c_void> = pairs.iter().map(|p| p.rr.code).collect();

        let a0 = ALLOCS.load(Ordering::Relaxed);
        let f0 = FREES.load(Ordering::Relaxed);
        for _ in 0..64 {
            // default allocator
            let (_, _, cb) = encode(c, &ccodes, std::ptr::null_mut());
            let (_, _, rb) = encode(r, &rcodes, std::ptr::null_mut());
            drop(cb); // -> serialize_free
            drop(rb);
            // custom allocator
            let cgx = my_gcontext(c);
            let rgx = my_gcontext(r);
            let (_, _, cb) = encode(c, &ccodes, cgx);
            let (_, _, rb) = encode(r, &rcodes, rgx);
            drop(cb);
            drop(rb);
            (c.general_context_free)(cgx);
            (r.general_context_free)(rgx);
        }
        let da = ALLOCS.load(Ordering::Relaxed) - a0;
        let df = FREES.load(Ordering::Relaxed) - f0;
        assert_eq!(da, df, "serialize_free: {} allocs vs {} frees", da, df);
        assert_eq!(da, 64 * 4, "serialize_free: unexpected allocation count {}", da);
    }
}

/// Randomized encode/decode: random groupings of the corpus, encoded in both
/// libraries, cross-decoded, and behaviour-compared.
#[test]
fn serialize_randomized() {
    let _guard = global_lock();
    unsafe {
        let (c, r) = both();
        let mut pairs = build_pairs(PATTERNS, &CompileCfg::new(0), "rnd");
        pairs.extend(build_pairs(
            &PATTERNS[..12],
            &CompileCfg::new(PCRE2_UTF),
            "rnd-utf",
        ));
        pairs.extend(build_pairs(
            &["(?<n>a)|(?<n>b)", "(?<n>x)(?<n>y)?"],
            &CompileCfg::new(PCRE2_DUPNAMES),
            "rnd-dup",
        ));
        let mut rng = Rng::new(0x5E71_A11E_D00Du64);
        for iter in 0..400 {
            let n = rng.range(1, 12) as usize;
            let mut idx = Vec::new();
            for _ in 0..n {
                idx.push(rng.below(pairs.len() as u32) as usize);
            }
            let ccodes: Vec<*mut c_void> = idx.iter().map(|&i| pairs[i].cc.code).collect();
            let rcodes: Vec<*mut c_void> = idx.iter().map(|&i| pairs[i].rr.code).collect();
            let use_gctx = rng.bool();
            let cgx = if use_gctx { my_gcontext(c) } else { std::ptr::null_mut() };
            let rgx = if use_gctx { my_gcontext(r) } else { std::ptr::null_mut() };

            let (crc, clen, cb) = encode(c, &ccodes, cgx);
            let (rrc, rlen, rb) = encode(r, &rcodes, rgx);
            assert_eq!(crc, rrc, "rnd {}: encode rc differs", iter);
            assert_eq!(clen, rlen, "rnd {}: encode size differs", iter);
            let cb = cb.unwrap();
            let rb = rb.unwrap();
            assert_eq!(cb.bytes(), rb.bytes(), "rnd {}: blobs differ", iter);
            assert_eq!(
                (c.serialize_get_number_of_codes)(cb.ptr),
                n as i32,
                "rnd {}: C get_number_of_codes",
                iter
            );
            assert_eq!(
                (r.serialize_get_number_of_codes)(cb.ptr),
                n as i32,
                "rnd {}: Rust reading the C blob",
                iter
            );

            let want = rng.range(1, n as u32 + 3) as i32;
            // cross-decode: C blob -> Rust, Rust blob -> C
            let (rc1, xr) = decode(r, cb.ptr, want, rgx);
            let (rc2, xc) = decode(c, rb.ptr, want, cgx);
            assert_eq!(rc1, rc2, "rnd {}: cross decode rc differs", iter);
            assert_eq!(rc1, want.min(n as i32), "rnd {}: cross decode rc", iter);
            for k in 0..xr.codes.len() {
                assert_codes_behave_same(
                    c,
                    xc.codes[k],
                    r,
                    xr.codes[k],
                    &format!("rnd {} cross-decoded {} ({})", iter, k, pairs[idx[k]].label),
                );
                assert_codes_behave_same(
                    r,
                    xr.codes[k],
                    r,
                    pairs[idx[k]].rr.code,
                    &format!("rnd {} decoded-vs-original {}", iter, k),
                );
            }
            drop(xr);
            drop(xc);
            drop(cb);
            drop(rb);
            if use_gctx {
                (c.general_context_free)(cgx);
                (r.general_context_free)(rgx);
            }
        }
    }
}
