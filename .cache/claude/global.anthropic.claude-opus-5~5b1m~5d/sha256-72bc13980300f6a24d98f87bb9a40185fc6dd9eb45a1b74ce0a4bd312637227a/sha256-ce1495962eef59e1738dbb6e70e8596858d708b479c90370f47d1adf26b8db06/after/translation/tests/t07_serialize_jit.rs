//! Phase B — `pcre2_serialize_*` and every public/private `pcre2_jit_*` entry
//! point.
//!
//! CONFIGS.md rows 129-134, 159-165 · ERRORS.md rows 204-217, 261-268.
#![allow(non_snake_case)]

mod common;
use common::corpus::*;
use common::*;
use std::ffi::c_void;

// --------------------------------------------------------------- layout info

/// `sizeof(pcre2_serialized_data)` — magic, version, config, number_of_codes.
const HDR: usize = 16;
const OFF_MAGIC_H: usize = 0;
const OFF_VERSION_H: usize = 4;
const OFF_CONFIG_H: usize = 8;
const OFF_NCODES_H: usize = 12;

/// A byte-for-byte mirror of `pcre2_real_code` (pcre2_intmodedep.h), used only
/// to derive field offsets so that the serialized body can be corrupted field
/// by field without hard-coding magic numbers.
#[repr(C)]
struct RealCodeMirror {
    memctl_malloc: *mut c_void,
    memctl_free: *mut c_void,
    memctl_data: *mut c_void,
    tables: *const u8,
    executable_jit: *mut c_void,
    start_bitmap: [u8; 32],
    blocksize: usize,
    code_start: usize,
    magic_number: u32,
    compile_options: u32,
    overall_options: u32,
    extra_options: u32,
    flags: u32,
    limit_heap: u32,
    limit_match: u32,
    limit_depth: u32,
    first_codeunit: u32,
    last_codeunit: u32,
    bsr_convention: u16,
    newline_convention: u16,
    max_lookbehind: u16,
    minlength: u16,
    top_bracket: u16,
    top_backref: u16,
    name_entry_size: u16,
    name_count: u16,
    optimization_flags: u32,
}

const OFF_BLOCKSIZE: usize = std::mem::offset_of!(RealCodeMirror, blocksize);
const OFF_MAGIC: usize = std::mem::offset_of!(RealCodeMirror, magic_number);
const OFF_FLAGS: usize = std::mem::offset_of!(RealCodeMirror, flags);
const OFF_NAME_ENTRY_SIZE: usize = std::mem::offset_of!(RealCodeMirror, name_entry_size);
const OFF_NAME_COUNT: usize = std::mem::offset_of!(RealCodeMirror, name_count);
const SIZEOF_REALCODE: usize = std::mem::size_of::<RealCodeMirror>();

/// Offset of the first compiled-code block inside a serialized stream.
const BODY: usize = HDR + TABLES_LENGTH;

const MAGIC_NUMBER: u32 = 0x5043_5245;

// --------------------------------------------------------------- byte buffer

/// An 8-byte-aligned owned copy of a serialized stream. `pcre2_serialize_decode`
/// reads `magic`/`version`/`config` through a struct pointer, so the copy must
/// be at least as aligned as `PCRE2_SIZE`.
struct Aligned {
    w: Vec<u64>,
    n: usize,
}

impl Aligned {
    unsafe fn from_raw(p: *const u8, n: usize) -> Aligned {
        let mut w = vec![0u64; n / 8 + 2];
        std::ptr::copy_nonoverlapping(p, w.as_mut_ptr() as *mut u8, n);
        Aligned { w, n }
    }
    fn dup(&self) -> Aligned {
        Aligned {
            w: self.w.clone(),
            n: self.n,
        }
    }
    fn as_ptr(&self) -> *const u8 {
        self.w.as_ptr() as *const u8
    }
    fn as_mut_ptr(&mut self) -> *mut u8 {
        self.w.as_mut_ptr() as *mut u8
    }
    fn bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.as_ptr(), self.n) }
    }
    unsafe fn get_u32(&self, off: usize) -> u32 {
        std::ptr::read_unaligned(self.as_ptr().add(off) as *const u32)
    }
    unsafe fn put_u32(&mut self, off: usize, v: u32) {
        std::ptr::write_unaligned(self.as_mut_ptr().add(off) as *mut u32, v)
    }
    unsafe fn get_usize(&self, off: usize) -> usize {
        std::ptr::read_unaligned(self.as_ptr().add(off) as *const usize)
    }
    unsafe fn put_usize(&mut self, off: usize, v: usize) {
        std::ptr::write_unaligned(self.as_mut_ptr().add(off) as *mut usize, v)
    }
    unsafe fn put_u16(&mut self, off: usize, v: u16) {
        std::ptr::write_unaligned(self.as_mut_ptr().add(off) as *mut u16, v)
    }
}

// ------------------------------------------------------------ test allocator

static mut ALLOC_FAIL: bool = false;
static mut ALLOC_LIVE: i64 = 0;
/// Tests run in parallel, so the two tests that install the failing allocator
/// must not overlap: `ALLOC_FAIL` / `ALLOC_LIVE` are process-wide.
static ALLOC_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

unsafe extern "C" fn tmalloc(size: usize, _d: *mut c_void) -> *mut c_void {
    if ALLOC_FAIL {
        return std::ptr::null_mut();
    }
    let total = size + 16;
    let p = std::alloc::alloc(std::alloc::Layout::from_size_align(total, 16).unwrap());
    if p.is_null() {
        return std::ptr::null_mut();
    }
    *(p as *mut usize) = total;
    ALLOC_LIVE += 1;
    p.add(16) as *mut c_void
}

unsafe extern "C" fn tfree(p: *mut c_void, _d: *mut c_void) {
    if p.is_null() {
        return;
    }
    let base = (p as *mut u8).sub(16);
    let total = *(base as *mut usize);
    ALLOC_LIVE -= 1;
    std::alloc::dealloc(
        base,
        std::alloc::Layout::from_size_align(total, 16).unwrap(),
    );
}

// ----------------------------------------------------------------- utilities

unsafe fn clear_ovector(api: &Api, md: MatchData) {
    let n = (api.get_ovector_count)(md) as usize;
    let ov = (api.get_ovector_pointer)(md);
    if !ov.is_null() {
        for i in 0..(2 * n) {
            *ov.add(i) = PCRE2_UNSET;
        }
    }
}

/// Matches `subj` against `code` and logs the entire outcome.
unsafe fn match_probe(api: &Api, code: Code, subj: &[u8], l: &mut Log) {
    let md = (api.match_data_create_from_pattern)(code, std::ptr::null_mut());
    if md.is_null() {
        l.tag("nomd");
        return;
    }
    clear_ovector(api, md);
    let rc = (api.do_match)(
        code,
        subj.as_ptr(),
        subj.len(),
        0,
        0,
        md,
        std::ptr::null_mut(),
    );
    log_match_result_full(api, code, md, rc, l);
    // The whole ovector is defined because clear_ovector pre-filled it.
    let n = (api.get_ovector_count)(md) as usize;
    let ov = (api.get_ovector_pointer)(md);
    for i in 0..(2 * n) {
        l.u(*ov.add(i) as u64);
    }
    (api.match_data_free)(md);
}

/// Subjects used to prove that a decoded code behaves like the original.
const MATCH_SUBJECTS: &[&str] = &["", "a", "abc", "ABC", "123", "aéb", "a\r\nb", "xyz", "日本"];

unsafe fn behaviour_log(api: &Api, code: Code, l: &mut Log) {
    log_all_info(api, code, l);
    for s in MATCH_SUBJECTS {
        match_probe(api, code, s.as_bytes(), l);
    }
}

/// Encodes `n` codes, logging rc, size and the *entire* byte stream, and
/// returns an aligned owned copy of it.
unsafe fn encode_probe(
    api: &Api,
    codes: &[Code],
    n: i32,
    gc: GContext,
    l: &mut Log,
) -> Option<Aligned> {
    let mut bytes: *mut u8 = std::ptr::null_mut();
    let mut size: Sz = 0xDEAD_BEEF;
    let rc = (api.serialize_encode)(codes.as_ptr(), n, &mut bytes, &mut size, gc);
    l.tag("enc").i(n as i64).i(rc as i64);

    if rc > 0 && !bytes.is_null() {
        l.u(size as u64)
            .b(std::slice::from_raw_parts(bytes, size))
            .i((api.serialize_get_number_of_codes)(bytes) as i64);
        let a = Aligned::from_raw(bytes, size);
        (api.serialize_free)(bytes);
        Some(a)
    } else {
        l.i(bytes.is_null() as i64);
        None
    }
}

/// Decodes `want` codes from `stream` and logs everything observable, including
/// the full behaviour of each decoded code.
unsafe fn decode_probe(api: &Api, stream: &Aligned, want: i32, gc: GContext, l: &mut Log) {
    let mut codes: [Code; 32] = [std::ptr::null_mut(); 32];
    let rc = (api.serialize_decode)(codes.as_mut_ptr(), want, stream.as_ptr(), gc);
    l.tag("dec").i(want as i64).i(rc as i64);
    if rc > 0 {
        for i in 0..(rc as usize).min(32) {
            l.u(i as u64).i(codes[i].is_null() as i64);
            if !codes[i].is_null() {
                behaviour_log(api, codes[i], l);
            }
        }
        for i in 0..(rc as usize).min(32) {
            if !codes[i].is_null() {
                (api.code_free)(codes[i]);
            }
        }
    } else {
        // Every slot must be left alone / NULLed out on failure.
        for i in 0..4 {
            l.i(codes[i].is_null() as i64);
        }
    }
    l.i((api.serialize_get_number_of_codes)(stream.as_ptr()) as i64);
}

/// Compiles one pattern, returning NULL on failure (logged).
unsafe fn compile1(api: &Api, pat: &str, opts: u32, l: &mut Log) -> Code {
    compile_logged(api, pat.as_bytes(), pat.len(), opts, std::ptr::null_mut(), l)
}

// ------------------------------------------------------ rows 159/160/162/164

/// A broad set of (pattern, options) pairs so the serialized bytecode covers
/// UTF, UCP, CASELESS, named groups, classes and callouts.
const SER_CASES: &[(&str, u32)] = &[
    ("a", 0),
    ("", 0),
    ("abc", 0),
    ("a|b|c", 0),
    ("(a)(b)(c)", 0),
    ("(?<n>a)(?<m>b)", 0),
    ("(?<dup>a)|(?<dup>b)", PCRE2_DUPNAMES),
    ("[a-z]+", 0),
    ("[[:alpha:]]{2,4}", 0),
    (r"[\p{L}\p{Nd}]", PCRE2_UTF | PCRE2_UCP),
    (r"\p{Greek}+", PCRE2_UTF),
    ("é", PCRE2_UTF),
    ("日本語", PCRE2_UTF),
    ("😀", PCRE2_UTF),
    ("abc", PCRE2_CASELESS),
    ("ÉÜ", PCRE2_UTF | PCRE2_CASELESS | PCRE2_UCP),
    ("a(?C1)b", 0),
    ("a(?C{txt})b", 0),
    ("a.b", PCRE2_AUTO_CALLOUT),
    (r"(?<year>\d{4})-(?<mon>\d{2})-(?<day>\d{2})", 0),
    ("(a(?R)?b)", 0),
    ("(?(DEFINE)(?<x>a))(?&x)", 0),
    ("a{2,4}+", 0),
    (r"\X\R\C", 0),
    ("(*UTF)(*UCP)a", 0),
    ("(*LIMIT_MATCH=100)(*LIMIT_DEPTH=50)a", 0),
    ("((((((((((a))))))))))", 0),
    ("a" , PCRE2_ANCHORED | PCRE2_ENDANCHORED),
    (r"^[\w.]+@[\w.]+$", PCRE2_UTF),
    ("[[a-z]--[aeiou]]", PCRE2_ALT_EXTENDED_CLASS),
    ("(?i)(?s)(?m)(?x) a b ", 0),
    (r"\d+\.\d+", PCRE2_UCP),
];

#[test]
fn serialize_encode_single_code() {
    for (i, (pat, opts)) in SER_CASES.iter().enumerate() {
        diff(&format!("ser1[{i}]={pat:?} opts={opts:#x}"), |api| {
            let mut l = Log::new();
            unsafe {
                let code = compile1(api, pat, *opts, &mut l);
                if code.is_null() {
                    return l;
                }
                behaviour_log(api, code, &mut l);
                let codes = [code];
                if let Some(st) = encode_probe(api, &codes, 1, std::ptr::null_mut(), &mut l) {
                    for want in [1i32, 2, 100, i32::MAX] {
                        decode_probe(api, &st, want, std::ptr::null_mut(), &mut l);
                    }
                }
                (api.code_free)(code);
            }
            l
        });
    }
}

/// row 159: N codes at once (2, 3, 10) — the whole stream is compared.
#[test]
fn serialize_encode_multiple_codes() {
    let mut rng = Rng::new(0x5E21_0001);
    for n in [2usize, 3, 10] {
        for round in 0..30 {
            let picks: Vec<(&str, u32)> = (0..n).map(|_| *rng.pick(SER_CASES)).collect();
            diff(&format!("serN n={n} round={round}"), |api| {
                let mut l = Log::new();
                unsafe {
                    let mut codes: Vec<Code> = Vec::new();
                    for (pat, opts) in &picks {
                        let c = compile1(api, pat, *opts, &mut l);
                        if !c.is_null() {
                            codes.push(c);
                        }
                    }
                    if codes.is_empty() {
                        return l;
                    }
                    let k = codes.len() as i32;
                    if let Some(st) = encode_probe(api, &codes, k, std::ptr::null_mut(), &mut l) {
                        // row 162: decode fewer codes than the stream holds.
                        for want in [1i32, 2, k - 1, k, k + 1, 1000, i32::MAX] {
                            if want <= 0 {
                                continue;
                            }
                            decode_probe(api, &st, want, std::ptr::null_mut(), &mut l);
                        }
                    }
                    // Encoding a prefix of the array must also work.
                    for k2 in 1..=k {
                        if let Some(st2) =
                            encode_probe(api, &codes, k2, std::ptr::null_mut(), &mut l)
                        {
                            decode_probe(api, &st2, k2, std::ptr::null_mut(), &mut l);
                        }
                    }
                    for c in codes {
                        (api.code_free)(c);
                    }
                }
                l
            });
        }
    }
}

/// row 161: NULL gcontext vs a custom malloc/free general context, on both
/// `encode` and `decode`.
#[test]
fn serialize_with_custom_gcontext() {
    let _guard = ALLOC_LOCK.lock().unwrap();
    for (i, (pat, opts)) in SER_CASES.iter().enumerate() {
        diff(&format!("sergc[{i}]={pat:?}"), |api| {
            let mut l = Log::new();
            unsafe {
                let gc =
                    (api.general_context_create)(Some(tmalloc), Some(tfree), std::ptr::null_mut());
                assert!(!gc.is_null());
                let code = compile1(api, pat, *opts, &mut l);
                if code.is_null() {
                    (api.general_context_free)(gc);
                    return l;
                }
                let codes = [code];
                // custom encode context
                if let Some(st) = encode_probe(api, &codes, 1, gc, &mut l) {
                    decode_probe(api, &st, 1, std::ptr::null_mut(), &mut l);
                    decode_probe(api, &st, 1, gc, &mut l);
                }
                // default encode context, custom decode context
                if let Some(st) = encode_probe(api, &codes, 1, std::ptr::null_mut(), &mut l) {
                    decode_probe(api, &st, 1, gc, &mut l);
                }
                // ERRORS 217 style: a failing allocator on encode and decode.
                if let Some(st) = encode_probe(api, &codes, 1, std::ptr::null_mut(), &mut l) {
                    ALLOC_FAIL = true;
                    let mut bytes: *mut u8 = std::ptr::null_mut();
                    let mut size: Sz = 0;
                    l.tag("encnomem").i((api.serialize_encode)(
                        codes.as_ptr(),
                        1,
                        &mut bytes,
                        &mut size,
                        gc,
                    ) as i64)
                    .i(bytes.is_null() as i64);
                    let mut dc: [Code; 4] = [std::ptr::null_mut(); 4];
                    l.tag("decnomem")
                        .i((api.serialize_decode)(dc.as_mut_ptr(), 1, st.as_ptr(), gc) as i64)
                        .i(dc[0].is_null() as i64);
                    ALLOC_FAIL = false;
                }
                (api.code_free)(code);
                (api.general_context_free)(gc);
            }
            l
        });
    }
}

/// row 165: `pcre2_serialize_free` on a valid stream and on NULL.
#[test]
fn serialize_free_paths() {
    diff("ser_free", |api| {
        let mut l = Log::new();
        unsafe {
            (api.serialize_free)(std::ptr::null_mut());
            let code = compile1(api, "abc", 0, &mut l);
            let codes = [code];
            let mut bytes: *mut u8 = std::ptr::null_mut();
            let mut size: Sz = 0;
            let rc = (api.serialize_encode)(
                codes.as_ptr(),
                1,
                &mut bytes,
                &mut size,
                std::ptr::null_mut(),
            );
            l.i(rc as i64).u(size as u64);
            (api.serialize_free)(bytes);
            (api.serialize_free)(std::ptr::null_mut());
            (api.code_free)(code);
            l.tag("ok").i(1);
        }
        l
    });
}

// ------------------------------------------------------ ERRORS rows 204-217

#[test]
fn serialize_encode_error_paths() {
    diff("enc_errors", |api| {
        let mut l = Log::new();
        unsafe {
            let code = compile1(api, "abc", 0, &mut l);
            assert!(!code.is_null());
            let codes = [code];
            let mut bytes: *mut u8 = std::ptr::null_mut();
            let mut size: Sz = 0xDEAD;

            // ERRORS 204: NULL arguments.
            l.tag("e204")
                .i((api.serialize_encode)(
                    std::ptr::null(),
                    1,
                    &mut bytes,
                    &mut size,
                    std::ptr::null_mut(),
                ) as i64)
                .i((api.serialize_encode)(
                    codes.as_ptr(),
                    1,
                    std::ptr::null_mut(),
                    &mut size,
                    std::ptr::null_mut(),
                ) as i64)
                .i((api.serialize_encode)(
                    codes.as_ptr(),
                    1,
                    &mut bytes,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                ) as i64)
                .i((api.serialize_encode)(
                    std::ptr::null(),
                    0,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                ) as i64);

            // ERRORS 205: number_of_codes <= 0. The `<= 0` test happens before
            // the array is touched, so even i32::MIN is safe to pass. A *huge*
            // positive count is deliberately NOT tested here: encode would read
            // `codes[0..n]`, i.e. off the end of the caller's array.
            for n in [0i32, -1, -2, -1000, i32::MIN] {
                l.tag("e205").i(n as i64).i((api.serialize_encode)(
                    codes.as_ptr(),
                    n,
                    &mut bytes,
                    &mut size,
                    std::ptr::null_mut(),
                ) as i64);
            }

            // ERRORS 206: codes[i] == NULL, for i == 0 and i > 0.
            let with_null: [Code; 4] = [code, std::ptr::null_mut(), code, std::ptr::null_mut()];
            let only_null: [Code; 2] = [std::ptr::null_mut(), code];
            for (tag, arr, n) in [
                ("first", &only_null[..], 1i32),
                ("second", &with_null[..], 2),
                ("fourth", &with_null[..], 4),
            ] {
                l.tag("e206").tag(tag).i((api.serialize_encode)(
                    arr.as_ptr(),
                    n,
                    &mut bytes,
                    &mut size,
                    std::ptr::null_mut(),
                ) as i64);
            }

            // ERRORS 207: a corrupted magic number in a code block.
            let cp = (api.code_copy)(code);
            assert!(!cp.is_null());
            let magic = (cp as *mut u8).add(OFF_MAGIC) as *mut u32;
            l.tag("magicok").i((*magic == MAGIC_NUMBER) as i64);
            let saved = *magic;
            for bad in [0u32, 0xDEAD_BEEF, MAGIC_NUMBER ^ 1, MAGIC_NUMBER + 1] {
                *magic = bad;
                let bad_arr: [Code; 2] = [cp, code];
                let bad_arr2: [Code; 2] = [code, cp];
                l.tag("e207")
                    .i((api.serialize_encode)(
                        bad_arr.as_ptr(),
                        2,
                        &mut bytes,
                        &mut size,
                        std::ptr::null_mut(),
                    ) as i64)
                    .i((api.serialize_encode)(
                        bad_arr2.as_ptr(),
                        2,
                        &mut bytes,
                        &mut size,
                        std::ptr::null_mut(),
                    ) as i64);
            }
            *magic = saved;
            (api.code_free)(cp);

            // ERRORS 208: MIXEDTABLES — two codes with different `tables`.
            let tables = (api.maketables)(std::ptr::null_mut());
            assert!(!tables.is_null());
            let cc = (api.compile_context_create)(std::ptr::null_mut());
            l.i((api.set_character_tables)(cc, tables) as i64);
            let code_t = compile_logged(api, b"abc", 3, 0, cc, &mut l);
            assert!(!code_t.is_null());
            for (n, arr) in [
                (2i32, [code, code_t]),
                (2, [code_t, code]),
                (1, [code_t, code]),
            ] {
                l.tag("e208").i(n as i64).i((api.serialize_encode)(
                    arr.as_ptr(),
                    n,
                    &mut bytes,
                    &mut size,
                    std::ptr::null_mut(),
                ) as i64);
            }
            // Two codes sharing the *same* custom tables must succeed.
            let code_t2 = compile_logged(api, b"xyz", 3, 0, cc, &mut l);
            let both = [code_t, code_t2];
            if let Some(st) = encode_probe(api, &both, 2, std::ptr::null_mut(), &mut l) {
                decode_probe(api, &st, 2, std::ptr::null_mut(), &mut l);
            }
            (api.code_free)(code_t);
            (api.code_free)(code_t2);
            (api.compile_context_free)(cc);
            (api.maketables_free)(std::ptr::null_mut(), tables);
            (api.code_free)(code);
        }
        l
    });
}

#[test]
fn serialize_decode_error_paths() {
    diff("dec_errors", |api| {
        let mut l = Log::new();
        unsafe {
            // Build a 3-code stream to corrupt.
            let mut codes: Vec<Code> = Vec::new();
            for p in ["(?<n>abc)", "[a-z]+", "x|y|z"] {
                let c = compile1(api, p, 0, &mut l);
                assert!(!c.is_null());
                codes.push(c);
            }
            let st = encode_probe(api, &codes, 3, std::ptr::null_mut(), &mut l);
            for c in &codes {
                (api.code_free)(*c);
            }
            let st = match st {
                Some(s) => s,
                None => return l,
            };
            let mut sink: [Code; 8] = [std::ptr::null_mut(); 8];

            // ERRORS 209: NULL data / NULL codes.
            l.tag("e209")
                .i((api.serialize_decode)(
                    sink.as_mut_ptr(),
                    1,
                    std::ptr::null(),
                    std::ptr::null_mut(),
                ) as i64)
                .i((api.serialize_decode)(
                    std::ptr::null_mut(),
                    1,
                    st.as_ptr(),
                    std::ptr::null_mut(),
                ) as i64)
                .i((api.serialize_decode)(
                    std::ptr::null_mut(),
                    1,
                    std::ptr::null(),
                    std::ptr::null_mut(),
                ) as i64)
                .i((api.serialize_get_number_of_codes)(std::ptr::null()) as i64);

            // ERRORS 210: number_of_codes <= 0.
            for n in [0i32, -1, -3, -1000, i32::MIN] {
                l.tag("e210").i(n as i64).i((api.serialize_decode)(
                    sink.as_mut_ptr(),
                    n,
                    st.as_ptr(),
                    std::ptr::null_mut(),
                ) as i64);
            }
            // Huge positive values are clamped to the stream's count.
            for n in [4i32, 8, 1000, i32::MAX, i32::MAX - 1] {
                decode_probe(api, &st, n, std::ptr::null_mut(), &mut l);
            }

            // ERRORS 211: header number_of_codes <= 0.
            for bad in [0i32, -1, -100, i32::MIN] {
                let mut s2 = st.dup();
                s2.put_u32(OFF_NCODES_H, bad as u32);
                l.tag("e211")
                    .i(bad as i64)
                    .i((api.serialize_decode)(
                        sink.as_mut_ptr(),
                        1,
                        s2.as_ptr(),
                        std::ptr::null_mut(),
                    ) as i64)
                    .i((api.serialize_get_number_of_codes)(s2.as_ptr()) as i64);
            }
            // A header count larger than what is present: decode would walk off
            // the end of the buffer, so only the *count query* is exercised.
            for big in [4i32, 99, i32::MAX] {
                let mut s2 = st.dup();
                s2.put_u32(OFF_NCODES_H, big as u32);
                l.tag("e211big")
                    .i(big as i64)
                    .i((api.serialize_get_number_of_codes)(s2.as_ptr()) as i64);
            }

            // ERRORS 212: bad magic.
            let good_magic = st.get_u32(OFF_MAGIC_H);
            l.tag("hdr")
                .u(good_magic as u64)
                .u(st.get_u32(OFF_VERSION_H) as u64)
                .u(st.get_u32(OFF_CONFIG_H) as u64)
                .u(st.get_u32(OFF_NCODES_H) as u64);
            for bad in [0u32, 0xFFFF_FFFF, good_magic ^ 1, good_magic ^ 0x8000_0000] {
                let mut s2 = st.dup();
                s2.put_u32(OFF_MAGIC_H, bad);
                l.tag("e212")
                    .u(bad as u64)
                    .i((api.serialize_decode)(
                        sink.as_mut_ptr(),
                        3,
                        s2.as_ptr(),
                        std::ptr::null_mut(),
                    ) as i64)
                    .i((api.serialize_get_number_of_codes)(s2.as_ptr()) as i64);
            }

            // ERRORS 213: bad version.
            let good_ver = st.get_u32(OFF_VERSION_H);
            for bad in [
                0u32,
                0xFFFF_FFFF,
                good_ver ^ 1,
                good_ver.wrapping_add(1 << 16),
                good_ver.wrapping_sub(1),
            ] {
                let mut s2 = st.dup();
                s2.put_u32(OFF_VERSION_H, bad);
                l.tag("e213")
                    .u(bad as u64)
                    .i((api.serialize_decode)(
                        sink.as_mut_ptr(),
                        3,
                        s2.as_ptr(),
                        std::ptr::null_mut(),
                    ) as i64)
                    .i((api.serialize_get_number_of_codes)(s2.as_ptr()) as i64);
            }

            // ERRORS 214: bad config (code-unit width / pointer / size width).
            let good_cfg = st.get_u32(OFF_CONFIG_H);
            for bad in [
                0u32,
                0xFFFF_FFFF,
                good_cfg ^ 1,
                good_cfg ^ 0x0000_0100,
                good_cfg ^ 0x0001_0000,
                2 | (8 << 8) | (8 << 16),
                4 | (4 << 8) | (4 << 16),
            ] {
                let mut s2 = st.dup();
                s2.put_u32(OFF_CONFIG_H, bad);
                l.tag("e214")
                    .u(bad as u64)
                    .i((api.serialize_decode)(
                        sink.as_mut_ptr(),
                        3,
                        s2.as_ptr(),
                        std::ptr::null_mut(),
                    ) as i64)
                    .i((api.serialize_get_number_of_codes)(s2.as_ptr()) as i64);
            }

            // ERRORS 215: truncated / corrupted body.
            let real_blocksize = st.get_usize(BODY + OFF_BLOCKSIZE);
            l.tag("bs")
                .u(real_blocksize as u64)
                .u(SIZEOF_REALCODE as u64)
                .i((real_blocksize > SIZEOF_REALCODE) as i64);

            // (a) blocksize too small (of the first and of the second block).
            for bad in [0usize, 1, 8, SIZEOF_REALCODE - 1, SIZEOF_REALCODE] {
                let mut s2 = st.dup();
                s2.put_usize(BODY + OFF_BLOCKSIZE, bad);
                l.tag("e215bs").u(bad as u64).i((api.serialize_decode)(
                    sink.as_mut_ptr(),
                    3,
                    s2.as_ptr(),
                    std::ptr::null_mut(),
                ) as i64);
                let mut s3 = st.dup();
                s3.put_usize(BODY + real_blocksize + OFF_BLOCKSIZE, bad);
                l.tag("e215bs2").u(bad as u64).i((api.serialize_decode)(
                    sink.as_mut_ptr(),
                    3,
                    s3.as_ptr(),
                    std::ptr::null_mut(),
                ) as i64);
                for i in 0..4 {
                    l.i(sink[i].is_null() as i64);
                }
            }

            // (b) the whole body zeroed => blocksize reads as 0.
            {
                let mut s2 = st.dup();
                for i in BODY..s2.n {
                    *s2.as_mut_ptr().add(i) = 0;
                }
                l.tag("e215zero").i((api.serialize_decode)(
                    sink.as_mut_ptr(),
                    3,
                    s2.as_ptr(),
                    std::ptr::null_mut(),
                ) as i64);
            }

            // (c) corrupted magic number inside a code block.
            for bad in [0u32, 0xDEAD_BEEF, MAGIC_NUMBER ^ 2] {
                let mut s2 = st.dup();
                s2.put_u32(BODY + OFF_MAGIC, bad);
                l.tag("e215magic").u(bad as u64).i((api.serialize_decode)(
                    sink.as_mut_ptr(),
                    3,
                    s2.as_ptr(),
                    std::ptr::null_mut(),
                ) as i64);
                let mut s3 = st.dup();
                s3.put_u32(BODY + real_blocksize + OFF_MAGIC, bad);
                l.tag("e215magic2").u(bad as u64).i((api.serialize_decode)(
                    sink.as_mut_ptr(),
                    3,
                    s3.as_ptr(),
                    std::ptr::null_mut(),
                ) as i64);
            }

            // (d) name_entry_size / name_count out of range.
            for bad in [0xFFFFu16, 200, 130, 1000] {
                let mut s2 = st.dup();
                s2.put_u16(BODY + OFF_NAME_ENTRY_SIZE, bad);
                l.tag("e215nes").u(bad as u64).i((api.serialize_decode)(
                    sink.as_mut_ptr(),
                    3,
                    s2.as_ptr(),
                    std::ptr::null_mut(),
                ) as i64);
                let mut s3 = st.dup();
                s3.put_u16(BODY + OFF_NAME_COUNT, bad);
                l.tag("e215nc").u(bad as u64).i((api.serialize_decode)(
                    sink.as_mut_ptr(),
                    3,
                    s3.as_ptr(),
                    std::ptr::null_mut(),
                ) as i64);
            }

            // (e) a flipped flag bit in the body — accepted, but the decoded
            // code must report it identically.
            {
                let mut s2 = st.dup();
                let f = s2.get_u32(BODY + OFF_FLAGS);
                s2.put_u32(BODY + OFF_FLAGS, f ^ 0x0000_0001);
                l.tag("e215flags").u(f as u64);
                decode_probe(api, &s2, 3, std::ptr::null_mut(), &mut l);
            }

            // (f) corrupted character tables — the stream stays valid.
            {
                let mut s2 = st.dup();
                for i in 0..32 {
                    *s2.as_mut_ptr().add(HDR + i) ^= 0xFF;
                }
                l.tag("e215tables");
                decode_probe(api, &s2, 3, std::ptr::null_mut(), &mut l);
            }

            // A pristine stream must still decode after all of that.
            decode_probe(api, &st, 3, std::ptr::null_mut(), &mut l);
        }
        l
    });
}

// ------------------------------------------------------------------- row 163

/// Bytes produced by the C library must decode in the Rust library and vice
/// versa, and the resulting codes must behave identically. This is the one
/// check that deliberately steps outside `diff`, because it mixes libraries.
#[test]
fn serialize_cross_compatibility() {
    let (c, r) = apis();
    for (i, (pat, opts)) in SER_CASES.iter().enumerate() {
        unsafe {
            let mut lc = Log::new();
            let mut lr = Log::new();
            let cc = compile1(c, pat, *opts, &mut lc);
            let rc_ = compile1(r, pat, *opts, &mut lr);
            assert_eq!(
                cc.is_null(),
                rc_.is_null(),
                "compile disagreement for case {i} {pat:?}"
            );
            if cc.is_null() {
                continue;
            }

            // --- Streams must be byte-identical.
            let sc = raw_encode(c, cc).expect("C encode failed");
            let sr = raw_encode(r, rc_).expect("Rust encode failed");
            assert_eq!(
                sc.bytes(),
                sr.bytes(),
                "serialized streams differ for case {i} {pat:?}"
            );
            assert_eq!(
                (c.serialize_get_number_of_codes)(sc.as_ptr()),
                (r.serialize_get_number_of_codes)(sr.as_ptr()),
                "code count differs for case {i}"
            );

            // --- Cross-decode: Rust reads C's bytes, C reads Rust's bytes.
            let mut cross_r = Log::new();
            decode_probe(r, &sc, 1, std::ptr::null_mut(), &mut cross_r);
            let mut cross_c = Log::new();
            decode_probe(c, &sr, 1, std::ptr::null_mut(), &mut cross_c);
            assert_eq!(
                cross_c, cross_r,
                "cross-decoded behaviour differs for case {i} {pat:?}"
            );

            // --- Same-library round trips must match the cross results.
            let mut own_c = Log::new();
            decode_probe(c, &sc, 1, std::ptr::null_mut(), &mut own_c);
            let mut own_r = Log::new();
            decode_probe(r, &sr, 1, std::ptr::null_mut(), &mut own_r);
            assert_eq!(own_c, cross_c, "C round trip differs for case {i} {pat:?}");
            assert_eq!(
                own_r, cross_r,
                "Rust round trip differs for case {i} {pat:?}"
            );

            // Each stream is freed by the library that produced it.
            (c.code_free)(cc);
            (r.code_free)(rc_);
        }
    }

    // Multi-code streams, too.
    unsafe {
        let pats = ["(?<a>x)", "[0-9]{3}", "a|bb|ccc", "(?i)Zz", "\\p{L}+"];
        let mut cc: Vec<Code> = Vec::new();
        let mut rr: Vec<Code> = Vec::new();
        let mut junk = Log::new();
        for p in pats {
            let a = compile1(c, p, PCRE2_UTF, &mut junk);
            let b = compile1(r, p, PCRE2_UTF, &mut junk);
            assert_eq!(a.is_null(), b.is_null());
            if !a.is_null() {
                cc.push(a);
                rr.push(b);
            }
        }
        let n = cc.len() as i32;
        let sc = raw_encode_n(c, &cc, n).expect("C multi-encode failed");
        let sr = raw_encode_n(r, &rr, n).expect("Rust multi-encode failed");
        assert_eq!(sc.bytes(), sr.bytes(), "multi-code streams differ");
        for want in [1i32, 2, n, n + 5] {
            let mut a = Log::new();
            let mut b = Log::new();
            decode_probe(r, &sc, want, std::ptr::null_mut(), &mut a);
            decode_probe(c, &sr, want, std::ptr::null_mut(), &mut b);
            assert_eq!(a, b, "multi-code cross decode differs (want={want})");
        }
        for x in cc {
            (c.code_free)(x);
        }
        for x in rr {
            (r.code_free)(x);
        }
    }
}

unsafe fn raw_encode(api: &Api, code: Code) -> Option<Aligned> {
    raw_encode_n(api, &[code], 1)
}

unsafe fn raw_encode_n(api: &Api, codes: &[Code], n: i32) -> Option<Aligned> {
    let mut bytes: *mut u8 = std::ptr::null_mut();
    let mut size: Sz = 0;
    let rc = (api.serialize_encode)(codes.as_ptr(), n, &mut bytes, &mut size, std::ptr::null_mut());
    if rc != n || bytes.is_null() {
        return None;
    }
    let a = Aligned::from_raw(bytes, size);
    (api.serialize_free)(bytes);
    Some(a)
}

// ------------------------------------------------------ rows 129/134, 261-263

const JIT_OPTION_SETS: &[u32] = &[
    0,
    PCRE2_JIT_COMPLETE,
    PCRE2_JIT_PARTIAL_SOFT,
    PCRE2_JIT_PARTIAL_HARD,
    PCRE2_JIT_COMPLETE | PCRE2_JIT_PARTIAL_SOFT,
    PCRE2_JIT_COMPLETE | PCRE2_JIT_PARTIAL_HARD,
    PCRE2_JIT_PARTIAL_SOFT | PCRE2_JIT_PARTIAL_HARD,
    PCRE2_JIT_COMPLETE | PCRE2_JIT_PARTIAL_SOFT | PCRE2_JIT_PARTIAL_HARD,
    PCRE2_JIT_INVALID_UTF,
    PCRE2_JIT_INVALID_UTF | PCRE2_JIT_COMPLETE,
    PCRE2_JIT_INVALID_UTF | PCRE2_JIT_PARTIAL_SOFT | PCRE2_JIT_PARTIAL_HARD,
    PCRE2_JIT_TEST_ALLOC,
    PCRE2_JIT_TEST_ALLOC | PCRE2_JIT_COMPLETE,
    PCRE2_JIT_TEST_ALLOC | PCRE2_JIT_INVALID_UTF,
    // undefined bits
    0x0000_0008,
    0x0000_0010,
    0x0000_0020,
    0x0000_0040,
    0x0000_0080,
    0x0000_0400,
    0x8000_0000,
    0xFFFF_FFFF,
    PCRE2_JIT_COMPLETE | 0x0000_0008,
    PCRE2_JIT_COMPLETE | 0x8000_0000,
];

const JIT_PATTERNS: &[(&str, u32)] = &[
    ("a", 0),
    ("abc", 0),
    ("(a)(b)", 0),
    ("[a-z]+", 0),
    ("é", PCRE2_UTF),
    ("é", PCRE2_UTF | PCRE2_MATCH_INVALID_UTF),
    ("(?<n>a)", 0),
    ("(*NO_JIT)a", 0),
    ("a.b", PCRE2_UTF | PCRE2_UCP),
    ("", 0),
    ("(a(?R)?b)", 0),
];

#[test]
fn jit_compile_options() {
    for (pi, (pat, copts)) in JIT_PATTERNS.iter().enumerate() {
        diff(&format!("jitc pat[{pi}]={pat:?} c={copts:#x}"), |api| {
            let mut l = Log::new();
            unsafe {
                // Fresh code for each option set: no cross-contamination from
                // the PCRE2_MATCH_INVALID_UTF side effect.
                for o in JIT_OPTION_SETS {
                    let code = compile1(api, pat, *copts, &mut l);
                    if code.is_null() {
                        continue;
                    }
                    let before = info_u32(api, code, INFO_ALLOPTIONS);
                    let rc = (api.jit_compile)(code, *o);
                    let after = info_u32(api, code, INFO_ALLOPTIONS);
                    let mut js: Sz = 0xDEAD;
                    let jrc = (api.pattern_info)(
                        code,
                        INFO_JITSIZE,
                        &mut js as *mut Sz as *mut c_void,
                    );
                    l.tag("jc")
                        .u(*o as u64)
                        .i(rc as i64)
                        .u(before as u64)
                        .u(after as u64)
                        .i(jrc as i64)
                        .u(js as u64)
                        .u(info_u32(api, code, INFO_ARGOPTIONS) as u64)
                        .u(info_u32(api, code, INFO_EXTRAOPTIONS) as u64);
                    // A second call with the same options must be stable.
                    l.i((api.jit_compile)(code, *o) as i64)
                        .u(info_u32(api, code, INFO_ALLOPTIONS) as u64);
                    // Row 130/264: jit_match after the jit_compile attempt.
                    jit_match_probe(api, code, b"abc", &mut l);
                    (api.code_free)(code);
                }

                // Sequential application of every option set to one code, so
                // the accumulating side effects are compared too.
                let code = compile1(api, pat, *copts, &mut l);
                if !code.is_null() {
                    for o in JIT_OPTION_SETS {
                        l.tag("seq")
                            .u(*o as u64)
                            .i((api.jit_compile)(code, *o) as i64)
                            .u(info_u32(api, code, INFO_ALLOPTIONS) as u64);
                    }
                    log_all_info(api, code, &mut l);
                    (api.code_free)(code);
                }

                // ERRORS 262: code == NULL. Note that PCRE2_JIT_TEST_ALLOC is
                // handled *before* the NULL check.
                for o in JIT_OPTION_SETS {
                    l.tag("jcnull")
                        .u(*o as u64)
                        .i((api.jit_compile)(std::ptr::null_mut(), *o) as i64);
                }
            }
            l
        });
    }
}

/// row 130 · ERRORS 264: `pcre2_jit_match`.
unsafe fn jit_match_probe(api: &Api, code: Code, subj: &[u8], l: &mut Log) {
    let md = (api.match_data_create_from_pattern)(code, std::ptr::null_mut());
    if md.is_null() {
        return;
    }
    clear_ovector(api, md);
    // A real match first, so that every field of the block is initialised
    // before jit_match (which only assigns match_data->rc) is called.
    let rc0 = (api.do_match)(
        code,
        subj.as_ptr(),
        subj.len(),
        0,
        0,
        md,
        std::ptr::null_mut(),
    );
    log_match_result_full(api, code, md, rc0, l);
    for mopts in [
        0u32,
        PCRE2_PARTIAL_SOFT,
        PCRE2_PARTIAL_HARD,
        PCRE2_NOTEMPTY,
        PCRE2_ANCHORED,
        0xFFFF_FFFF,
    ] {
        let rc = (api.jit_match)(
            code,
            subj.as_ptr(),
            subj.len(),
            0,
            mopts,
            md,
            std::ptr::null_mut(),
        );
        l.tag("jm").u(mopts as u64).i(rc as i64);
        // jit_match writes match_data->rc; observe it through next_match.
        let mut so: Sz = 0xDEAD_BEEF;
        let mut op: u32 = 0xDEAD_BEEF;
        l.i((api.next_match)(md, &mut so, &mut op) as i64)
            .u(so as u64)
            .u(op as u64);
    }
    // With a real match context, too.
    let mc = (api.match_context_create)(std::ptr::null_mut());
    l.tag("jmctx").i((api.jit_match)(
        code,
        subj.as_ptr(),
        subj.len(),
        0,
        0,
        md,
        mc,
    ) as i64);
    (api.match_context_free)(mc);
    (api.match_data_free)(md);
}

#[test]
fn jit_match_variants() {
    for (pi, (pat, copts)) in JIT_PATTERNS.iter().enumerate() {
        for (si, s) in ["", "a", "abc", "abcabc", "é"].iter().enumerate() {
            diff(&format!("jitm pat[{pi}]={pat:?} subj[{si}]={s:?}"), |api| {
                let mut l = Log::new();
                unsafe {
                    let code = compile1(api, pat, *copts, &mut l);
                    if code.is_null() {
                        return l;
                    }
                    // Before any jit_compile …
                    jit_match_probe(api, code, s.as_bytes(), &mut l);
                    // … after a failed jit_compile …
                    l.i((api.jit_compile)(code, PCRE2_JIT_COMPLETE) as i64);
                    jit_match_probe(api, code, s.as_bytes(), &mut l);
                    // … and after a bad-option jit_compile.
                    l.i((api.jit_compile)(code, 0xFFFF_FFFF) as i64);
                    jit_match_probe(api, code, s.as_bytes(), &mut l);
                    (api.code_free)(code);
                }
                l
            });
        }
    }
}

// -------------------------------------------------- rows 131-133, 265-267

const JIT_STACK_SIZES: &[(usize, usize)] = &[
    (0, 0),
    (0, 1),
    (1, 0),
    (1, 1),
    (1, 1024),
    (1024, 1),
    (32 * 1024, 512 * 1024),
    (512 * 1024, 32 * 1024),
    (1, usize::MAX),
    (usize::MAX, 1),
    (usize::MAX, usize::MAX),
    (usize::MAX - 1, usize::MAX - 1),
    (4096, 4096),
    (1 << 20, 1 << 30),
];

unsafe extern "C" fn jit_cb(_d: *mut c_void) -> JitStack {
    std::ptr::null_mut()
}

static mut CB_CALLS: u32 = 0;

unsafe extern "C" fn jit_cb_counting(_d: *mut c_void) -> JitStack {
    CB_CALLS += 1;
    std::ptr::null_mut()
}

#[test]
fn jit_stack_and_misc() {
    let _guard = ALLOC_LOCK.lock().unwrap();
    diff("jit_misc", |api| {
        let mut l = Log::new();
        unsafe {
            // row 131 · ERRORS 265/266: jit_stack_create.
            let gc = (api.general_context_create)(Some(tmalloc), Some(tfree), std::ptr::null_mut());
            assert!(!gc.is_null());
            for (start, max) in JIT_STACK_SIZES {
                for g in [std::ptr::null_mut(), gc] {
                    let st = (api.jit_stack_create)(*start, *max, g);
                    l.tag("jsc")
                        .u(*start as u64)
                        .u(*max as u64)
                        .i(st.is_null() as i64);
                    (api.jit_stack_free)(st);
                }
            }
            (api.jit_stack_free)(std::ptr::null_mut());

            // row 131: jit_stack_assign with and without a callback, and with
            // a NULL match context.
            CB_CALLS = 0;
            let mc = (api.match_context_create)(std::ptr::null_mut());
            assert!(!mc.is_null());
            (api.jit_stack_assign)(mc, None, std::ptr::null_mut());
            (api.jit_stack_assign)(mc, Some(jit_cb), std::ptr::null_mut());
            (api.jit_stack_assign)(mc, Some(jit_cb_counting), &raw mut CB_CALLS as *mut c_void);
            (api.jit_stack_assign)(std::ptr::null_mut(), Some(jit_cb), std::ptr::null_mut());
            (api.jit_stack_assign)(std::ptr::null_mut(), None, std::ptr::null_mut());
            l.tag("jsa").u(CB_CALLS as u64);

            // A match through the mutated match context must be unaffected.
            let code = compile1(api, "(a)(b)?", 0, &mut l);
            assert!(!code.is_null());
            let md = (api.match_data_create_from_pattern)(code, std::ptr::null_mut());
            clear_ovector(api, md);
            let rc = (api.do_match)(code, b"ab".as_ptr(), 2, 0, 0, md, mc);
            log_match_result_full(api, code, md, rc, &mut l);
            l.u(CB_CALLS as u64);
            (api.match_data_free)(md);
            (api.match_context_free)(mc);

            // row 132 · ERRORS 267: jit_free_unused_memory.
            (api.jit_free_unused_memory)(std::ptr::null_mut());
            (api.jit_free_unused_memory)(gc);
            let gc2 = (api.general_context_create)(Some(tmalloc), Some(tfree), gc);
            (api.jit_free_unused_memory)(gc2);
            (api.general_context_free)(gc2);
            (api.jit_free_unused_memory)(std::ptr::null_mut());
            l.tag("jfum").i(0);

            // row 133: _pcre2_jit_get_size / _pcre2_jit_get_target.
            l.tag("jgs").u((api.p_jit_get_size)(std::ptr::null_mut()) as u64);
            let mut scratch = [0u8; 64];
            l.u((api.p_jit_get_size)(scratch.as_mut_ptr() as *mut c_void) as u64);
            let t = (api.p_jit_get_target)();
            l.tag("jgt").i(t.is_null() as i64);
            if !t.is_null() {
                l.b(&cstr(t as *const u8));
            }
            // Also compare it against what pcre2_config reports.
            let mut cbuf = [0u8; 128];
            l.tag("cfgjit")
                .i((api.config)(CONFIG_JIT, cbuf.as_mut_ptr() as *mut c_void) as i64)
                .u(u32::from_le_bytes([cbuf[0], cbuf[1], cbuf[2], cbuf[3]]) as u64)
                .i((api.config)(CONFIG_JITTARGET, cbuf.as_mut_ptr() as *mut c_void) as i64);

            // row 134 · ERRORS 268: JITSIZE after a failed jit_compile.
            for o in [0u32, PCRE2_JIT_COMPLETE, PCRE2_JIT_TEST_ALLOC, 0xFFFF_FFFF] {
                l.i((api.jit_compile)(code, o) as i64);
                let mut js: Sz = 0xDEAD;
                l.i((api.pattern_info)(code, INFO_JITSIZE, &mut js as *mut Sz as *mut c_void)
                    as i64)
                    .u(js as u64);
            }
            (api.code_free)(code);
            (api.general_context_free)(gc);
            l.tag("live").i((ALLOC_LIVE == 0) as i64);
        }
        l
    });
}

// --------------------------------------------------- randomized serialisation

#[test]
fn serialize_random_patterns() {
    let mut rng = Rng::new(0x5E21_0002);
    for iter in 0..1500 {
        let pat = PatternGen::gen(&mut rng);
        let opts = *rng.pick(&[
            0u32,
            PCRE2_UTF,
            PCRE2_UTF | PCRE2_UCP,
            PCRE2_CASELESS,
            PCRE2_DUPNAMES,
            PCRE2_AUTO_CALLOUT,
            PCRE2_MULTILINE | PCRE2_DOTALL,
        ]);
        diff(&format!("serrand iter={iter} pat={pat:?} opts={opts:#x}"), |api| {
            let mut l = Log::new();
            unsafe {
                let code = compile1(api, &pat, opts, &mut l);
                if code.is_null() {
                    return l;
                }
                let codes = [code];
                if let Some(st) = encode_probe(api, &codes, 1, std::ptr::null_mut(), &mut l) {
                    decode_probe(api, &st, 1, std::ptr::null_mut(), &mut l);
                }
                (api.code_free)(code);
            }
            l
        });
    }
}

#[test]
fn serialize_corpus_patterns() {
    for (i, p) in PATTERNS.iter().enumerate() {
        for opts in [0u32, PCRE2_UTF, PCRE2_CASELESS | PCRE2_DUPNAMES] {
            diff(&format!("sercorp[{i}]={p:?} opts={opts:#x}"), |api| {
                let mut l = Log::new();
                unsafe {
                    let code = compile1(api, p, opts, &mut l);
                    if code.is_null() {
                        return l;
                    }
                    let codes = [code, code];
                    if let Some(st) = encode_probe(api, &codes, 2, std::ptr::null_mut(), &mut l) {
                        decode_probe(api, &st, 2, std::ptr::null_mut(), &mut l);
                    }
                    (api.code_free)(code);
                }
                l
            });
        }
    }
}

// ----------------------------------------------------------------- utilities

unsafe fn info_u32(api: &Api, code: Code, what: u32) -> u32 {
    let mut v: u32 = 0;
    (api.pattern_info)(code, what, &mut v as *mut u32 as *mut c_void);
    v
}

