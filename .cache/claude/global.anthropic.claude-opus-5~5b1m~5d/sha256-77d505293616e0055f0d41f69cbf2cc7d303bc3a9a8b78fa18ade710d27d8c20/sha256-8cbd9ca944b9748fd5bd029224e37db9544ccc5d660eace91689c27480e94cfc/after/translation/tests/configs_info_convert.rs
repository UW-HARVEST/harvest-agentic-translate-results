//! Differential tests for CONFIGS.md rows C190-C202.
//!
//!   * C190-C191 — `pcre2_pattern_info_8`: every key, `where` NULL and non-NULL,
//!                 the NULL/BADMAGIC/BADMODE gate ordering.
//!   * C192      — `pcre2_callout_enumerate_8`.
//!   * C193-C199 — `pcre2_pattern_convert_8` (mode legality, UTF, POSIX basic,
//!                 POSIX extended, glob stars, glob classes, the composite bits)
//!                 and `pcre2_converted_pattern_free_8`.
//!   * C200      — the free-function family and the hidden `pcre2_memctl` prefix.
//!   * C201-C202 — the JIT stub family (public + PRIV), SUPPORT_JIT undefined.
//!
//! Everything goes through the two loaded `.so`s (see `tests/common/mod.rs`).

mod common;
use common::*;

use std::cell::RefCell;
use std::collections::HashMap;
use std::ptr;

// ===================================================================== consts

/// MAGIC_NUMBER (pcre2_internal.h:542)
const MAGIC_NUMBER: u32 = 0x5043_5245;

// offsetof(pcre2_real_code, ...) for the LP64 layout in pcre2_intmodedep.h:660
//   memctl 0..24, tables 24, executable_jit 32, start_bitmap 40..72,
//   blocksize 72, code_start 80, magic_number 88, compile_options 92,
//   overall_options 96, extra_options 100, flags 104, ...
const RC_START_BITMAP: usize = 40;
const RC_BLOCKSIZE: usize = 72;
const RC_MAGIC: usize = 88;
const RC_FLAGS: usize = 104;
/// sizeof(pcre2_real_code) — the name table starts here (pattern_info.c:230)
const RC_SIZEOF: usize = 152;

/// (PCRE2_CODE_UNIT_WIDTH/8) — the bit checked by the BADMODE gate
const MODE_BIT: u32 = 1;

/// Positive (compile-time) error codes returned by the converters.
const PCRE2_ERROR_END_BACKSLASH: i32 = 101;
const PCRE2_ERROR_MISSING_SQUARE_BRACKET: i32 = 106;

const POISON_SZ: usize = 0xDEAD_BEEF_CAFE_F00D;

// ============================================================ pattern_info

/// One `pcre2_pattern_info` observation for a single key.
#[derive(Debug, PartialEq, Eq, Clone)]
struct InfoRow {
    key: u32,
    /// rc of the call with a real (poison-filled) `where` buffer
    rc_buf: i32,
    /// rc of the call with `where == NULL` (the length query)
    rc_nullwhere: i32,
    /// Everything observable about what the call wrote.  For scalar keys these
    /// are the raw 16 buffer bytes (8 payload + 8 guard).  For the two
    /// pointer-valued keys this is guard-bytes + offset-from-code +
    /// pointed-at contents, so that absolute pointer values are never compared.
    obs: Vec<u8>,
}

/// Query every key in `0..=30` (26 is the last defined key, so 27..=30 are
/// out-of-range) with a poisoned `where` buffer and with `where == NULL`.
///
/// `origin` is the base used to turn pointer results into offsets; pass the
/// same pointer that was handed to the library as `code`.
unsafe fn info_all_keys(api: &Api, code: Ptr, origin: Ptr) -> Vec<InfoRow> {
    // How many bytes of name table there are (needed to compare its contents).
    let mut ncount: u32 = 0;
    let mut nsize: u32 = 0;
    if !code.is_null() {
        if (api.pcre2_pattern_info_8)(code, PCRE2_INFO_NAMECOUNT, &mut ncount as *mut u32 as Ptr)
            != 0
        {
            ncount = 0;
        }
        if (api.pcre2_pattern_info_8)(
            code,
            PCRE2_INFO_NAMEENTRYSIZE,
            &mut nsize as *mut u32 as Ptr,
        ) != 0
        {
            nsize = 0;
        }
    }

    let mut out = Vec::with_capacity(31);
    for key in 0u32..=30 {
        // 8-byte aligned, poison filled: payload word + guard word.
        let mut w: [u64; 2] = [0xA5A5_A5A5_A5A5_A5A5, 0x5A5A_5A5A_5A5A_5A5A];
        let rc_buf = (api.pcre2_pattern_info_8)(code, key, w.as_mut_ptr() as Ptr);

        let obs = if (key == PCRE2_INFO_NAMETABLE || key == PCRE2_INFO_FIRSTBITMAP) && rc_buf == 0 {
            let p = w[0] as usize as *const u8;
            let mut v = Vec::new();
            v.extend_from_slice(&w[1].to_ne_bytes()); // guard must be untouched
            if p.is_null() {
                v.extend_from_slice(b"<NULL>");
            } else {
                let off = (p as usize).wrapping_sub(origin as usize);
                v.extend_from_slice(&off.to_ne_bytes());
                let n = if key == PCRE2_INFO_FIRSTBITMAP {
                    32
                } else {
                    (ncount as usize) * (nsize as usize)
                };
                v.extend_from_slice(std::slice::from_raw_parts(p, n));
            }
            v
        } else {
            let mut v = Vec::with_capacity(16);
            v.extend_from_slice(&w[0].to_ne_bytes());
            v.extend_from_slice(&w[1].to_ne_bytes());
            v
        };

        let rc_nullwhere = (api.pcre2_pattern_info_8)(code, key, ptr::null_mut());
        out.push(InfoRow {
            key,
            rc_buf,
            rc_nullwhere,
            obs,
        });
    }
    out
}

/// Byte-for-byte, 8-byte-aligned copy of a compiled pattern block, so that the
/// magic number / mode flag can be corrupted without touching library memory.
unsafe fn clone_code(api: &Api, code: Ptr) -> Vec<u64> {
    let mut sz: usize = 0;
    let rc = (api.pcre2_pattern_info_8)(code, PCRE2_INFO_SIZE, &mut sz as *mut usize as Ptr);
    assert_eq!(rc, 0, "{}: PCRE2_INFO_SIZE failed", api.tag);
    let mut v = vec![0u64; sz / 8 + 2];
    ptr::copy_nonoverlapping(code as *const u8, v.as_mut_ptr() as *mut u8, sz);
    // sanity: our hard-coded offsets really do describe this build's layout
    let base = v.as_ptr() as *const u8;
    assert_eq!(
        (base.add(RC_MAGIC) as *const u32).read_unaligned(),
        MAGIC_NUMBER,
        "{}: magic_number not at offset {}",
        api.tag,
        RC_MAGIC
    );
    assert_eq!(
        (base.add(RC_BLOCKSIZE) as *const usize).read_unaligned(),
        sz,
        "{}: blocksize not at offset {}",
        api.tag,
        RC_BLOCKSIZE
    );
    v
}

unsafe fn set_u32(buf: &mut [u64], off: usize, val: u32) {
    ((buf.as_mut_ptr() as *mut u8).add(off) as *mut u32).write_unaligned(val);
}
unsafe fn get_u32(buf: &[u64], off: usize) -> u32 {
    ((buf.as_ptr() as *const u8).add(off) as *const u32).read_unaligned()
}

// ------------------------------------------------------------- pattern pool

/// Tokens used by the fixed-seed random pattern generator.
const PAT_TOKS: &[&[u8]] = &[
    b"a", b"b", b"c", b"Z", b"0", b"9", b".", b"*", b"+", b"?", b"|", b"^", b"$", b"(", b")",
    b"(?:", b"(?i)", b"(?<n1>", b"(?<n2>", b"(?P=n1)", b"\\1", b"\\2", b"[a-z]", b"[^a-z]",
    b"[[:alpha:]]", b"\\d", b"\\D", b"\\w", b"\\s", b"\\R", b"\\b", b"\\B", b"\\A", b"\\z", b"\\Z",
    b"\\G", b"\\K", b"\\C", b"\\X", b"\\p{L}", b"\\P{Nd}", b"{2}", b"{1,3}", b"{2,}", b"*?",
    b"++", b"?+", b"(?=", b"(?!", b"(?<=", b"(?<!", b"(?>", b"(?C1)", b"(?C{x})", b"(*MARK:m)",
    b"(*COMMIT)", b"(*SKIP)", b"(*PRUNE)", b"(*THEN)", b"(*ACCEPT)", b"(*FAIL)", b"(?R)", b"(?1)",
    b"(?&n1)", b"\\x{100}", b"\\xff", b"\\cA", b"\r", b"\n", b"\xc3\xa9", b"\xf0\x9f\x98\x80",
    b"-", b"]", b"[", b"}", b"{", b"\\Q.*\\E", b"(?|(a)|(b))", b"(?(1)a|b)", b"[[a-z]&&[^q]]",
];

const OPT_BITS: &[u32] = &[
    PCRE2_ANCHORED,
    PCRE2_NO_UTF_CHECK,
    PCRE2_ENDANCHORED,
    PCRE2_ALLOW_EMPTY_CLASS,
    PCRE2_ALT_BSUX,
    PCRE2_AUTO_CALLOUT,
    PCRE2_CASELESS,
    PCRE2_DOLLAR_ENDONLY,
    PCRE2_DOTALL,
    PCRE2_DUPNAMES,
    PCRE2_EXTENDED,
    PCRE2_FIRSTLINE,
    PCRE2_MATCH_UNSET_BACKREF,
    PCRE2_MULTILINE,
    PCRE2_NEVER_UCP,
    PCRE2_NEVER_UTF,
    PCRE2_NO_AUTO_CAPTURE,
    PCRE2_NO_AUTO_POSSESS,
    PCRE2_NO_DOTSTAR_ANCHOR,
    PCRE2_NO_START_OPTIMIZE,
    PCRE2_UCP,
    PCRE2_UNGREEDY,
    PCRE2_UTF,
    PCRE2_NEVER_BACKSLASH_C,
    PCRE2_ALT_CIRCUMFLEX,
    PCRE2_ALT_VERBNAMES,
    PCRE2_USE_OFFSET_LIMIT,
    PCRE2_EXTENDED_MORE,
    PCRE2_LITERAL,
    PCRE2_MATCH_INVALID_UTF,
    PCRE2_ALT_EXTENDED_CLASS,
];

const XOPT_BITS: &[u32] = &[
    PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES,
    PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL,
    PCRE2_EXTRA_MATCH_WORD,
    PCRE2_EXTRA_MATCH_LINE,
    PCRE2_EXTRA_ESCAPED_CR_IS_LF,
    PCRE2_EXTRA_ALT_BSUX,
    PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK,
    PCRE2_EXTRA_CASELESS_RESTRICT,
    PCRE2_EXTRA_ASCII_BSD,
    PCRE2_EXTRA_ASCII_BSS,
    PCRE2_EXTRA_ASCII_BSW,
    PCRE2_EXTRA_ASCII_POSIX,
    PCRE2_EXTRA_ASCII_DIGIT,
    PCRE2_EXTRA_PYTHON_OCTAL,
    PCRE2_EXTRA_NO_BS0,
    PCRE2_EXTRA_NEVER_CALLOUT,
    PCRE2_EXTRA_TURKISH_CASING,
];

/// (pattern, compile options, extra compile options)
fn info_pattern_pool() -> Vec<(Vec<u8>, u32, u32)> {
    let mut v: Vec<(Vec<u8>, u32, u32)> = Vec::new();
    let mut push = |p: &[u8], o: u32, x: u32| v.push((p.to_vec(), o, x));

    // --- ALLOPTIONS vs ARGOPTIONS (start-of-pattern directives, auto-anchor)
    push(b"(*UTF)abc", 0, 0);
    push(b"(*UCP)abc", 0, 0);
    push(b"(*ANY)a.b", 0, 0);
    push(b"(*ANYCRLF)a", 0, 0);
    push(b"(*LIMIT_MATCH=99)a", 0, 0);
    push(b"(*LIMIT_DEPTH=7)a", 0, 0);
    push(b"(*LIMIT_HEAP=1234)a", 0, 0);
    push(b"(*NO_AUTO_POSSESS)a+b", 0, 0);
    push(b"(*NO_DOTSTAR_ANCHOR).*b", 0, 0);
    push(b"(*NO_START_OPT)abc", 0, 0);
    push(b"(*NOTEMPTY)a*", 0, 0);
    push(b"(*NOTEMPTY_ATSTART)a*", 0, 0);
    push(b"(*NO_JIT)abc", 0, 0);
    push(b".*abc", 0, 0);
    push(b".*abc", PCRE2_NO_DOTSTAR_ANCHOR, 0);
    push(b"(?s).*abc", 0, 0);

    // --- BACKREFMAX
    push(b"abc", 0, 0);
    push(b"(a)(b)\\2", 0, 0);
    push(b"(a)(b)(c)\\3\\1", 0, 0);
    push(b"(a)\\g{-1}", 0, 0);
    push(b"(?P<n>a)(?P=n)", 0, 0);
    push(b"(?<n>a)\\k<n>", 0, 0);
    push(b"(a)(?:b)\\1", 0, 0);

    // --- BSR
    push(b"(*BSR_ANYCRLF)\\R", 0, 0);
    push(b"(*BSR_UNICODE)\\R", 0, 0);
    push(b"\\R", 0, 0);

    // --- CAPTURECOUNT 0/1/many
    push(b"a", 0, 0);
    push(b"(a)", 0, 0);
    push(b"(a)(b)(c)(d)(e)(f)(g)(h)", 0, 0);
    {
        let mut p = Vec::new();
        for _ in 0..64 {
            p.extend_from_slice(b"(a)");
        }
        push(&p, 0, 0);
        let mut p2 = Vec::new();
        for _ in 0..300 {
            p2.extend_from_slice(b"(x)");
        }
        push(&p2, 0, 0);
    }

    // --- FIRSTCODETYPE / FIRSTCODEUNIT / FIRSTBITMAP / STARTLINE
    push(b"\\d", 0, 0);
    push(b"a|b", 0, 0);
    push(b"abc", 0, 0);
    push(b"^abc", 0, 0);
    push(b"^abc", PCRE2_MULTILINE, 0);
    push(b".*abc", PCRE2_MULTILINE, 0);
    push(b"[ab]x", 0, 0);
    push(b"\\d+", 0, 0);
    push(b"[^a]z", 0, 0);
    push(b"(?i)abc", 0, 0);
    push(b"\\x{100}bc", PCRE2_UTF, 0);
    push(b"[\\x{100}-\\x{200}]z", PCRE2_UTF, 0);
    push(b"a", PCRE2_ANCHORED, 0);
    push(b"\\Aabc", 0, 0);
    push(b"\\Gabc", 0, 0);

    // --- HASCRORLF
    push(b"a\rb", 0, 0);
    push(b"a\nb", 0, 0);
    push(b"a\r\nb", 0, 0);
    push(b"a\\rb", 0, 0);
    push(b"[\r]", 0, 0);

    // --- JCHANGED
    push(b"(?J)(?<n>a)(?<n>b)", 0, 0);
    push(b"(?<n>a)(?<m>b)", PCRE2_DUPNAMES, 0);
    push(b"(?J:(?<n>a)(?<n>b))", 0, 0);

    // --- LASTCODETYPE / LASTCODEUNIT
    push(b"ab", 0, 0);
    push(b"a.*b", 0, 0);
    push(b"a.*bZ", 0, 0);
    push(b"a[bc]*d", 0, 0);

    // --- MATCHEMPTY
    push(b"a*", 0, 0);
    push(b"(?:)", 0, 0);
    push(b"\\b", 0, 0);
    push(b"()", 0, 0);
    push(b"a+", 0, 0);
    push(b"a{0,3}", 0, 0);
    push(b"(?=a)", 0, 0);

    // --- MAXLOOKBEHIND
    push(b"a", 0, 0);
    push(b"(?<=abc)x", 0, 0);
    push(b"(?<!ab)x", 0, 0);
    push(b"(?<=a|bcd)x", 0, 0);
    push(b"(?<=abcdefghij)x", 0, 0);
    push(b"\\Kabc", 0, 0);
    push(b"a\\Kb", 0, 0);
    push(b"(?<=\\x{100})x", PCRE2_UTF, 0);

    // --- MINLENGTH
    push(b"a*", 0, 0);
    push(b"abc", 0, 0);
    push(b"abc|de", 0, 0);
    push(b"a{5,7}", 0, 0);

    // --- NAMECOUNT / NAMEENTRYSIZE / NAMETABLE
    push(b"(?<a>x)", 0, 0);
    push(b"(?<a>x)(?<bb>y)(?<ccc>z)", 0, 0);
    push(b"(?<averyveryverylongname>x)(?<b>y)", 0, 0);
    push(b"(?J)(?<n>a)(?<n>b)(?<n>c)", 0, 0);
    push(b"(?<z>1)(?<y>2)(?<x>3)(?<w>4)(?<v>5)", 0, 0);

    // --- NEWLINE conventions
    push(b"(*CR)a", 0, 0);
    push(b"(*LF)a", 0, 0);
    push(b"(*CRLF)a", 0, 0);
    push(b"(*ANY)a", 0, 0);
    push(b"(*ANYCRLF)a", 0, 0);
    push(b"(*NUL)a", 0, 0);

    // --- HASBACKSLASHC
    push(b"\\cA", 0, 0);
    push(b"a\\cZb", 0, 0);
    push(b"\\C", 0, 0);
    push(b"a\\Cb", 0, 0);

    // --- recursion / callouts / verbs
    push(b"(a(?R)?)", 0, 0);
    push(b"(a)(?1)", 0, 0);
    push(b"(?<n>a)(?&n)", 0, 0);
    push(b"a(?C)b", 0, 0);
    push(b"a(?C1)b", 0, 0);
    push(b"a(?C255)b", 0, 0);
    push(b"a(?C{some text})b", 0, 0);
    push(b"a(?C\"\")b", 0, 0);
    push(b"ab", PCRE2_AUTO_CALLOUT, 0);
    push(b"a(?C1)[b-z]+", PCRE2_AUTO_CALLOUT, 0);
    push(b"(*MARK:X)a", 0, 0);
    push(b"(*COMMIT:Y)a", 0, 0);
    push(b"(*PRUNE:Z)a", 0, 0);
    push(b"(*SKIP:W)a", 0, 0);
    push(b"(*THEN:V)a", 0, 0);
    push(b"[[a-z]&&[^q]]", PCRE2_ALT_EXTENDED_CLASS, 0);
    push(b"[\\x{100}-\\x{200}]", PCRE2_UTF, 0);
    push(b"\\p{L}*", PCRE2_UTF | PCRE2_UCP, 0);
    push(b"(?(?=a)b|c)", 0, 0);
    push(b"(?>ab)+", 0, 0);
    push(b"a++b*?c{2,3}+", 0, 0);

    // --- SIZE / FRAMESIZE spread
    {
        let mut p = Vec::new();
        for i in 0..40u32 {
            p.extend_from_slice(format!("(x{}y)", i % 10).as_bytes());
        }
        push(&p, 0, 0);
        let mut q = Vec::new();
        for _ in 0..500 {
            q.push(b'k');
        }
        push(&q, 0, 0);
    }

    // --- every compile option bit against two bases
    for &o in OPT_BITS {
        push(b"a(b)[c-e]\\1x*", o, 0);
        push(b"(?<nm>\\d+)\\R.", o, 0);
    }
    // --- every extra compile option bit
    for &x in XOPT_BITS {
        push(b"a(b)[c-e]\\1x*\\d\\s\\w", 0, x);
    }
    push(b"abc", 0, PCRE2_EXTRA_MATCH_WORD);
    push(b"abc", 0, PCRE2_EXTRA_MATCH_LINE);

    // --- fixed-seed random patterns
    let mut rng = Rng::new(0x1907_5190_C190_0001);
    while v.len() < 820 {
        let n = 1 + rng.below(9);
        let mut p: Vec<u8> = Vec::new();
        for _ in 0..n {
            p.extend_from_slice(rng.pick(PAT_TOKS));
        }
        let nb = rng.below(4);
        let mut o = 0u32;
        for _ in 0..nb {
            o |= *rng.pick(OPT_BITS);
        }
        let x = if rng.below(4) == 0 {
            *rng.pick(XOPT_BITS)
        } else {
            0
        };
        v.push((p, o, x));
    }
    v
}

// ==================================================================== C190

#[test]
fn c190_pattern_info_gates() {
    println!("C190 pcre2_pattern_info: NULL / BADMAGIC / BADMODE gate ordering");
    let (c, r) = both();
    // Harness sanity: the two Api structs must really resolve to two different
    // shared objects, otherwise every differential assertion below is vacuous.
    assert_ne!(
        c.pcre2_pattern_info_8 as usize, r.pcre2_pattern_info_8 as usize,
        "C and RUST resolved pcre2_pattern_info_8 to the same address"
    );
    assert_ne!(
        c.pcre2_pattern_convert_8 as usize, r.pcre2_pattern_convert_8 as usize,
        "C and RUST resolved pcre2_pattern_convert_8 to the same address"
    );
    assert_ne!(
        c._pcre2_jit_get_target_8 as usize, r._pcre2_jit_get_target_8 as usize,
        "C and RUST resolved _pcre2_jit_get_target_8 to the same address"
    );
    println!("  C    = {:?}\n  RUST = {:?}", c.path, r.path);
    unsafe {
        // ---- code == NULL, for every key, both `where` shapes
        assert_eq!(
            info_all_keys(c, ptr::null_mut(), ptr::null_mut()),
            info_all_keys(r, ptr::null_mut(), ptr::null_mut()),
            "C190 pattern_info(NULL, key, ..) mismatch"
        );
        // spot-assert the documented behaviour so a "both wrong" case is visible
        let rows = info_all_keys(c, ptr::null_mut(), ptr::null_mut());
        for row in &rows {
            if row.key <= 26 {
                assert!(
                    row.rc_nullwhere > 0,
                    "C190 key {} NULL-where length query should succeed, got {}",
                    row.key,
                    row.rc_nullwhere
                );
            } else {
                assert_eq!(
                    row.rc_nullwhere, PCRE2_ERROR_NULL,
                    "C190 key {} out of range with NULL where",
                    row.key
                );
            }
            assert_eq!(
                row.rc_buf, PCRE2_ERROR_NULL,
                "C190 key {} with real where and NULL code",
                row.key
            );
        }

        // ---- valid code, corrupted magic, cleared mode flag, both corrupted
        let pats: &[&[u8]] = &[
            b"abc",
            b"(?<n>a)(?<m>b)\\1",
            b"[ab]x*(?C1)",
            b"a",
            b"(*LIMIT_MATCH=5)\\d+",
        ];
        for pat in pats {
            let mut ec = 0i32;
            let mut eo = 0usize;
            let cc = (c.pcre2_compile_8)(
                pat.as_ptr(),
                pat.len(),
                0,
                &mut ec,
                &mut eo,
                ptr::null_mut(),
            );
            let mut ec2 = 0i32;
            let mut eo2 = 0usize;
            let rr = (r.pcre2_compile_8)(
                pat.as_ptr(),
                pat.len(),
                0,
                &mut ec2,
                &mut eo2,
                ptr::null_mut(),
            );
            assert_eq!((ec, eo, cc.is_null()), (ec2, eo2, rr.is_null()));
            assert!(!cc.is_null(), "C190 setup pattern {:?} failed", pat);

            // intact copies
            let mut bc = clone_code(c, cc);
            let mut br = clone_code(r, rr);
            let pc = bc.as_mut_ptr() as Ptr;
            let pr = br.as_mut_ptr() as Ptr;
            assert_eq!(
                info_all_keys(c, pc, pc),
                info_all_keys(r, pr, pr),
                "C190 intact copy mismatch for {:?}",
                pat
            );

            // BADMAGIC
            set_u32(&mut bc, RC_MAGIC, 0xDEAD_0000);
            set_u32(&mut br, RC_MAGIC, 0xDEAD_0000);
            let a = info_all_keys(c, pc, pc);
            let b = info_all_keys(r, pr, pr);
            assert_eq!(a, b, "C190 BADMAGIC mismatch for {:?}", pat);
            for row in &a {
                assert_eq!(
                    row.rc_buf, PCRE2_ERROR_BADMAGIC,
                    "C190 BADMAGIC key {}",
                    row.key
                );
                if row.key <= 26 {
                    assert!(row.rc_nullwhere > 0, "C190 BADMAGIC NULL-where key {}", row.key);
                } else {
                    assert_eq!(row.rc_nullwhere, PCRE2_ERROR_BADMAGIC);
                }
            }

            // BADMAGIC wins over BADMODE (gate order)
            let f = get_u32(&bc, RC_FLAGS);
            set_u32(&mut bc, RC_FLAGS, f & !MODE_BIT);
            let fr = get_u32(&br, RC_FLAGS);
            set_u32(&mut br, RC_FLAGS, fr & !MODE_BIT);
            let a = info_all_keys(c, pc, pc);
            assert_eq!(
                a,
                info_all_keys(r, pr, pr),
                "C190 BADMAGIC+BADMODE mismatch for {:?}",
                pat
            );
            for row in &a {
                assert_eq!(
                    row.rc_buf, PCRE2_ERROR_BADMAGIC,
                    "C190 magic gate must precede mode gate (key {})",
                    row.key
                );
            }

            // BADMODE only
            set_u32(&mut bc, RC_MAGIC, MAGIC_NUMBER);
            set_u32(&mut br, RC_MAGIC, MAGIC_NUMBER);
            let a = info_all_keys(c, pc, pc);
            assert_eq!(
                a,
                info_all_keys(r, pr, pr),
                "C190 BADMODE mismatch for {:?}",
                pat
            );
            for row in &a {
                assert_eq!(row.rc_buf, PCRE2_ERROR_BADMODE, "C190 BADMODE key {}", row.key);
                if row.key <= 26 {
                    assert!(row.rc_nullwhere > 0);
                } else {
                    assert_eq!(row.rc_nullwhere, PCRE2_ERROR_BADMODE);
                }
            }

            // restore the mode bit -> valid again
            set_u32(&mut bc, RC_FLAGS, f);
            let fr2 = get_u32(&br, RC_FLAGS);
            set_u32(&mut br, RC_FLAGS, fr2 | MODE_BIT);
            assert_eq!(
                info_all_keys(c, pc, pc),
                info_all_keys(r, pr, pr),
                "C190 restored copy mismatch for {:?}",
                pat
            );

            (c.pcre2_code_free_8)(cc);
            (r.pcre2_code_free_8)(rr);
        }
    }
}

// ==================================================================== C191

#[test]
fn c191_pattern_info_every_key() {
    println!("C191 pcre2_pattern_info: every key over the whole pattern pool");
    let (c, r) = both();
    let pool = info_pattern_pool();
    assert!(pool.len() >= 300, "pool too small: {}", pool.len());
    let mut compiled = 0usize;
    unsafe {
        for (i, (pat, opts, xopts)) in pool.iter().enumerate() {
            // compile context only when extra options are wanted
            let (ctx_c, ctx_r) = if *xopts != 0 {
                let a = (c.pcre2_compile_context_create_8)(ptr::null_mut());
                let b = (r.pcre2_compile_context_create_8)(ptr::null_mut());
                assert!(!a.is_null() && !b.is_null());
                assert_eq!(
                    (c.pcre2_set_compile_extra_options_8)(a, *xopts),
                    (r.pcre2_set_compile_extra_options_8)(b, *xopts)
                );
                (a, b)
            } else {
                (ptr::null_mut(), ptr::null_mut())
            };

            let mut ec = 0i32;
            let mut eo = POISON_SZ;
            let cc = (c.pcre2_compile_8)(pat.as_ptr(), pat.len(), *opts, &mut ec, &mut eo, ctx_c);
            let mut ec2 = 0i32;
            let mut eo2 = POISON_SZ;
            let rr = (r.pcre2_compile_8)(pat.as_ptr(), pat.len(), *opts, &mut ec2, &mut eo2, ctx_r);
            assert_eq!(
                (ec, eo, cc.is_null()),
                (ec2, eo2, rr.is_null()),
                "C191 compile mismatch #{} pat={:?} opts={:#x} xopts={:#x}",
                i,
                String::from_utf8_lossy(pat),
                opts,
                xopts
            );

            if !cc.is_null() {
                compiled += 1;
                let a = info_all_keys(c, cc, cc);
                let b = info_all_keys(r, rr, rr);
                assert_eq!(
                    a, b,
                    "C191 pattern_info mismatch #{} pat={:?} opts={:#x} xopts={:#x}",
                    i,
                    String::from_utf8_lossy(pat),
                    opts,
                    xopts
                );
                // every defined key must have answered the NULL-where query
                for row in &a {
                    if row.key <= 26 {
                        assert!(
                            row.rc_nullwhere == 4 || row.rc_nullwhere == 8,
                            "C191 key {} NULL-where returned {}",
                            row.key,
                            row.rc_nullwhere
                        );
                    } else {
                        assert_eq!(row.rc_nullwhere, PCRE2_ERROR_BADOPTION);
                        assert_eq!(row.rc_buf, PCRE2_ERROR_BADOPTION);
                    }
                }
                // JIT is absent in this build: JITSIZE must be 0 in both
                let js = &a[PCRE2_INFO_JITSIZE as usize];
                assert_eq!(js.rc_buf, 0);
                assert_eq!(
                    u64::from_ne_bytes(js.obs[0..8].try_into().unwrap()),
                    0,
                    "C191 PCRE2_INFO_JITSIZE must be 0 without SUPPORT_JIT"
                );
                // pointer keys land at the documented offsets
                let nt = &a[PCRE2_INFO_NAMETABLE as usize];
                assert_eq!(nt.rc_buf, 0);
                assert_eq!(
                    usize::from_ne_bytes(nt.obs[8..16].try_into().unwrap()),
                    RC_SIZEOF,
                    "C191 NAMETABLE offset"
                );
                let fb = &a[PCRE2_INFO_FIRSTBITMAP as usize];
                assert_eq!(fb.rc_buf, 0);
                if &fb.obs[8..14] != b"<NULL>" {
                    assert_eq!(
                        usize::from_ne_bytes(fb.obs[8..16].try_into().unwrap()),
                        RC_START_BITMAP,
                        "C191 FIRSTBITMAP offset"
                    );
                }
                (c.pcre2_code_free_8)(cc);
                (r.pcre2_code_free_8)(rr);
            }
            if !ctx_c.is_null() {
                (c.pcre2_compile_context_free_8)(ctx_c);
                (r.pcre2_compile_context_free_8)(ctx_r);
            }
        }
    }
    println!(
        "C191: {} patterns in pool, {} compiled, 31 keys x 2 where-shapes each",
        pool.len(),
        compiled
    );
    assert!(compiled >= 300, "too few patterns compiled: {}", compiled);
}

// ==================================================================== C192

/// Mirrors `pcre2_callout_enumerate_block` (pcre2.h:604-614) on LP64.
#[repr(C)]
struct CalloutEnumBlock {
    version: u32,
    pattern_position: usize,
    next_item_length: usize,
    callout_number: u32,
    callout_string_offset: usize,
    callout_string_length: usize,
    callout_string: *const u8,
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct CbRec {
    version: u32,
    pattern_position: usize,
    next_item_length: usize,
    callout_number: u32,
    callout_string_offset: usize,
    callout_string_length: usize,
    string_null: bool,
    /// offset of `callout_string` from the code block base (never the raw pointer)
    string_off: usize,
    string: Vec<u8>,
    /// the `callout_data` argument, echoed back
    data: usize,
}

#[derive(Default)]
struct CbState {
    code_base: usize,
    stop_at: i32,
    stop_rc: i32,
    recs: Vec<CbRec>,
}

thread_local! {
    static CB: RefCell<CbState> = RefCell::new(CbState::default());
    /// (conv_case invocations, individual pcre2_pattern_convert calls per library)
    static CONV_COUNT: RefCell<(u64, u64)> = const { RefCell::new((0, 0)) };
}

fn conv_bump(calls: u64) {
    CONV_COUNT.with(|c| {
        let mut c = c.borrow_mut();
        c.0 += 1;
        c.1 += calls;
    });
}
fn conv_report(tag: &str) {
    CONV_COUNT.with(|c| {
        let c = c.borrow();
        println!(
            "{}: {} conversion cases, {} pcre2_pattern_convert calls per library",
            tag, c.0, c.1
        );
    });
}

unsafe extern "C" fn enum_cb(blk: Ptr, data: Ptr) -> i32 {
    let b = &*(blk as *const CalloutEnumBlock);
    CB.with(|st| {
        let mut s = st.borrow_mut();
        let (snull, soff, sbytes) = if b.callout_string.is_null() {
            (true, 0usize, Vec::new())
        } else {
            (
                false,
                (b.callout_string as usize).wrapping_sub(s.code_base),
                std::slice::from_raw_parts(b.callout_string, b.callout_string_length).to_vec(),
            )
        };
        s.recs.push(CbRec {
            version: b.version,
            pattern_position: b.pattern_position,
            next_item_length: b.next_item_length,
            callout_number: b.callout_number,
            callout_string_offset: b.callout_string_offset,
            callout_string_length: b.callout_string_length,
            string_null: snull,
            string_off: soff,
            string: sbytes,
            data: data as usize,
        });
        let idx = s.recs.len() as i32 - 1;
        if s.stop_at >= 0 && idx == s.stop_at {
            s.stop_rc
        } else {
            0
        }
    })
}

unsafe fn run_enum(
    api: &Api,
    code: Ptr,
    origin: Ptr,
    stop_at: i32,
    stop_rc: i32,
    data: Ptr,
) -> (i32, Vec<CbRec>) {
    CB.with(|st| {
        let mut s = st.borrow_mut();
        s.code_base = origin as usize;
        s.stop_at = stop_at;
        s.stop_rc = stop_rc;
        s.recs.clear();
    });
    let rc = (api.pcre2_callout_enumerate_8)(code, Some(enum_cb), data);
    let recs = CB.with(|st| st.borrow().recs.clone());
    (rc, recs)
}

#[test]
fn c192_callout_enumerate() {
    println!("C192 pcre2_callout_enumerate: every block field of every step");
    let (c, r) = both();

    // (pattern, options)
    let cases: &[(&[u8], u32)] = &[
        (b"abc", 0),
        (b"", 0),
        (b"a(?C)b", 0),
        (b"a(?C1)b", 0),
        (b"a(?C255)b", 0),
        (b"a(?C0)b", 0),
        (b"a(?C{txt})b", 0),
        (b"a(?C\"\")b", 0),
        (b"a(?C{})b", 0),
        (b"a(?C{a}}b)c", 0),
        (b"(?C1)(?C2)(?C3)", 0),
        (b"a(?C1)b(?C{s})c(?C2)", 0),
        (b"abc", PCRE2_AUTO_CALLOUT),
        (b"a(b|c)*[d-f]\\d+", PCRE2_AUTO_CALLOUT),
        (b"(?<n>a)(?&n)", PCRE2_AUTO_CALLOUT),
        (b"a(?C1)b", PCRE2_AUTO_CALLOUT),
        (b"[abc](?C1)", 0),
        (b"[^abc](?C1)", 0),
        (b"(?=a(?C1))b", 0),
        (b"(?!a(?C1))b", 0),
        (b"(?<=a(?C1))b", 0),
        (b"(?<!ab(?C1))c", 0),
        (b"(?>a(?C1))b", 0),
        (b"(*MARK:X)(?C1)", 0),
        (b"(*COMMIT:X)(?C1)", 0),
        (b"(*PRUNE:X)(?C1)", 0),
        (b"(*SKIP:X)(?C1)", 0),
        (b"(*THEN:X)(?C1)", 0),
        (b"(*MARK:AVeryLongMarkName)(?C7)", 0),
        (b"\\p{L}*(?C1)", PCRE2_UTF | PCRE2_UCP),
        (b"\\p{L}(?C1)", PCRE2_UCP),
        (b"\\P{Nd}+(?C1)", PCRE2_UCP),
        (b"[\\x{100}-\\x{200}](?C1)", PCRE2_UTF),
        (b"[[a-z]&&[^q]](?C1)", PCRE2_ALT_EXTENDED_CLASS),
        (b"\\x{1234}(?C1)", PCRE2_UTF),
        (b"\\x{1234}*(?C1)", PCRE2_UTF),
        (b"\\x{1234}{2,5}(?C1)", PCRE2_UTF),
        (b"\\x{10000}+(?C1)", PCRE2_UTF),
        (b"(?i)\\x{100}(?C1)", PCRE2_UTF),
        (b"aaaaaaaaaaaaaaaaaaaa(?C1)", 0),
        (b"a{10}(?C1)", 0),
        (b"[^a]{3,7}(?C1)", 0),
        (b".*(?C1)", 0),
        (b"\\d{2,}(?C1)", 0),
        (b"\\X+(?C1)", PCRE2_UTF),
        (b"(?C1)(a(?C2)(b(?C3)))", 0),
        (b"a(?C{multi\nline})b", 0),
        (b"(?C{\xc3\xa9})a", PCRE2_UTF),
        (b"a*+(?C1)b?+(?C2)c{1,2}+(?C3)", 0),
        (b"(?(?C1)a|b)", 0),
        (b"(?J)(?<n>a(?C1))(?<n>b(?C2))", 0),
    ];

    let data_sentinels: [usize; 2] = [0, 0xABCD_1234];

    unsafe {
        for (pat, opts) in cases {
            let mut ec = 0i32;
            let mut eo = 0usize;
            let cc = (c.pcre2_compile_8)(
                pat.as_ptr(),
                pat.len(),
                *opts,
                &mut ec,
                &mut eo,
                ptr::null_mut(),
            );
            let mut ec2 = 0i32;
            let mut eo2 = 0usize;
            let rr = (r.pcre2_compile_8)(
                pat.as_ptr(),
                pat.len(),
                *opts,
                &mut ec2,
                &mut eo2,
                ptr::null_mut(),
            );
            assert_eq!(
                (ec, eo, cc.is_null()),
                (ec2, eo2, rr.is_null()),
                "C192 compile mismatch pat={:?}",
                String::from_utf8_lossy(pat)
            );
            if cc.is_null() {
                continue;
            }

            for &d in &data_sentinels {
                // full enumeration
                let (rc_c, recs_c) = run_enum(c, cc, cc, -1, 0, d as Ptr);
                let (rc_r, recs_r) = run_enum(r, rr, rr, -1, 0, d as Ptr);
                assert_eq!(
                    (rc_c, &recs_c),
                    (rc_r, &recs_r),
                    "C192 enumeration mismatch pat={:?} opts={:#x}",
                    String::from_utf8_lossy(pat),
                    opts
                );
                assert_eq!(rc_c, 0, "C192 full enumeration should return 0");
                for rec in &recs_c {
                    assert_eq!(rec.version, 0);
                    assert_eq!(rec.data, d);
                }

                // stop the enumeration at every step, with a positive and a
                // negative callback return
                for stop in 0..(recs_c.len() as i32) {
                    for &srr in &[1i32, -1, 42, PCRE2_ERROR_NOMEMORY, i32::MIN, i32::MAX] {
                        let (rc_c, a) = run_enum(c, cc, cc, stop, srr, d as Ptr);
                        let (rc_r, b) = run_enum(r, rr, rr, stop, srr, d as Ptr);
                        assert_eq!(
                            (rc_c, &a),
                            (rc_r, &b),
                            "C192 stop@{} rc={} mismatch pat={:?}",
                            stop,
                            srr,
                            String::from_utf8_lossy(pat)
                        );
                        assert_eq!(rc_c, srr, "C192 callback return must propagate");
                        assert_eq!(a.len(), (stop + 1) as usize);
                    }
                }
            }

            // ---- the three gates
            let mut bc = clone_code(c, cc);
            let mut br = clone_code(r, rr);
            let pc = bc.as_mut_ptr() as Ptr;
            let pr = br.as_mut_ptr() as Ptr;
            assert_eq!(
                run_enum(c, pc, pc, -1, 0, ptr::null_mut()),
                run_enum(r, pr, pr, -1, 0, ptr::null_mut()),
                "C192 enumeration over a copied block"
            );
            set_u32(&mut bc, RC_MAGIC, 0);
            set_u32(&mut br, RC_MAGIC, 0);
            let (a, ra) = run_enum(c, pc, pc, -1, 0, ptr::null_mut());
            let (b, rb) = run_enum(r, pr, pr, -1, 0, ptr::null_mut());
            assert_eq!((a, ra), (b, rb));
            assert_eq!(a, PCRE2_ERROR_BADMAGIC);
            set_u32(&mut bc, RC_MAGIC, MAGIC_NUMBER);
            set_u32(&mut br, RC_MAGIC, MAGIC_NUMBER);
            let f = get_u32(&bc, RC_FLAGS);
            set_u32(&mut bc, RC_FLAGS, f & !MODE_BIT);
            let fr = get_u32(&br, RC_FLAGS);
            set_u32(&mut br, RC_FLAGS, fr & !MODE_BIT);
            let (a, ra) = run_enum(c, pc, pc, -1, 0, ptr::null_mut());
            let (b, rb) = run_enum(r, pr, pr, -1, 0, ptr::null_mut());
            assert_eq!((a, ra), (b, rb));
            assert_eq!(a, PCRE2_ERROR_BADMODE);

            (c.pcre2_code_free_8)(cc);
            (r.pcre2_code_free_8)(rr);
        }

        // code == NULL
        let (a, ra) = run_enum(c, ptr::null_mut(), ptr::null_mut(), -1, 0, ptr::null_mut());
        let (b, rb) = run_enum(r, ptr::null_mut(), ptr::null_mut(), -1, 0, ptr::null_mut());
        assert_eq!((a, ra), (b, rb));
        assert_eq!(a, PCRE2_ERROR_NULL);
    }
}

// ================================================================ convert

/// Result of the "library allocates the buffer" shape.
#[derive(Debug, PartialEq, Eq)]
struct ConvAlloc {
    rc: i32,
    blen: usize,
    /// converted bytes including the terminating zero, when one was produced
    bytes: Option<Vec<u8>>,
}

unsafe fn conv_alloc(api: &Api, pat: *const u8, plen: usize, opts: u32, cctx: Ptr) -> ConvAlloc {
    let mut buf: *mut u8 = ptr::null_mut();
    let mut blen: usize = POISON_SZ;
    let rc = (api.pcre2_pattern_convert_8)(pat, plen, opts, &mut buf, &mut blen, cctx);
    let bytes = if buf.is_null() {
        None
    } else {
        let v = if rc == 0 && blen < (1 << 22) {
            std::slice::from_raw_parts(buf, blen + 1).to_vec()
        } else {
            Vec::new()
        };
        (api.pcre2_converted_pattern_free_8)(buf);
        Some(v)
    };
    ConvAlloc { rc, blen, bytes }
}

/// The "length only" shape: `buffptr == NULL`.
unsafe fn conv_len(api: &Api, pat: *const u8, plen: usize, opts: u32, cctx: Ptr) -> (i32, usize) {
    let mut blen: usize = POISON_SZ;
    let rc = (api.pcre2_pattern_convert_8)(pat, plen, opts, ptr::null_mut(), &mut blen, cctx);
    (rc, blen)
}

/// The "caller supplied buffer" shape: `*buffptr != NULL`, `*bufflenptr == cap`.
/// The returned Vec is the whole poison-filled arena (cap bytes + 16 guard
/// bytes) so overwrites past the end are caught too.
unsafe fn conv_user(
    api: &Api,
    pat: *const u8,
    plen: usize,
    opts: u32,
    cctx: Ptr,
    cap: usize,
) -> (i32, usize, Vec<u8>) {
    let mut mem = vec![0x7Eu8; cap + 16];
    let base = mem.as_mut_ptr();
    let mut buf: *mut u8 = base;
    let mut blen: usize = cap;
    let rc = (api.pcre2_pattern_convert_8)(pat, plen, opts, &mut buf, &mut blen, cctx);
    assert_eq!(
        buf, base,
        "{}: *buffptr must not be reassigned when a buffer is supplied",
        api.tag
    );
    (rc, blen, mem)
}

/// Compare one conversion case in every buffer shape.
#[allow(clippy::too_many_arguments)]
unsafe fn conv_case(
    c: &Api,
    r: &Api,
    tag: &str,
    pat: &[u8],
    zero_terminated: bool,
    opts: u32,
    cctx: (Ptr, Ptr),
    sweep: bool,
    do_compile: bool,
) {
    // For the zero-terminated shape we need a real NUL after the bytes.
    let mut buf = pat.to_vec();
    buf.push(0);
    let (p, plen) = if zero_terminated {
        (buf.as_ptr(), PCRE2_ZERO_TERMINATED)
    } else {
        (buf.as_ptr(), pat.len())
    };

    let desc = || {
        format!(
            "{} pat={:?} (len {}) zt={} opts={:#x}",
            tag,
            String::from_utf8_lossy(pat),
            pat.len(),
            zero_terminated,
            opts
        )
    };

    let mut ncalls = 2u64; // length-only + allocate

    // 1. length only
    let lc = conv_len(c, p, plen, opts, cctx.0);
    let lr = conv_len(r, p, plen, opts, cctx.1);
    assert_eq!(lc, lr, "length-only mismatch: {}", desc());

    // 2. library allocates
    let ac = conv_alloc(c, p, plen, opts, cctx.0);
    let ar = conv_alloc(r, p, plen, opts, cctx.1);
    assert_eq!(ac, ar, "allocate mismatch: {}", desc());
    // the two shapes must agree with each other
    assert_eq!((lc.0, lc.1), (ac.rc, ac.blen), "shape disagreement: {}", desc());

    // 3. caller supplied buffer, sizes 0..=needed+4
    if sweep {
        let need = if ac.rc == 0 { ac.blen } else { 8 };
        let hi = need.min(4096) + 4;
        ncalls += (hi + 1) as u64;
        for cap in 0..=hi {
            let uc = conv_user(c, p, plen, opts, cctx.0, cap);
            let ur = conv_user(r, p, plen, opts, cctx.1, cap);
            assert_eq!(
                (uc.0, uc.1),
                (ur.0, ur.1),
                "user-buffer(cap={}) rc/blen mismatch: {}",
                cap,
                desc()
            );
            assert_eq!(
                uc.2, ur.2,
                "user-buffer(cap={}) content mismatch: {}",
                cap,
                desc()
            );
            // guard bytes must be intact
            assert_eq!(
                &uc.2[cap..],
                &[0x7Eu8; 16][..],
                "user-buffer(cap={}) wrote past the end: {}",
                cap,
                desc()
            );
        }
    }

    conv_bump(ncalls);

    // 4. the converted pattern must compile identically in both libraries
    if do_compile && ac.rc == 0 {
        if let Some(bytes) = &ac.bytes {
            let n = ac.blen;
            let oc = c.compile_probe(&bytes[..n], n, 0, ptr::null_mut());
            let or_ = r.compile_probe(&bytes[..n], n, 0, ptr::null_mut());
            assert_eq!(
                oc, or_,
                "converted pattern {:?} compiles differently: {}",
                String::from_utf8_lossy(&bytes[..n]),
                desc()
            );
        }
    }
}

/// The alphabet for the random glob / POSIX corpus.
const CONV_ALPHA: &[&[u8]] = &[
    b"*", b"?", b"[", b"]", b"^", b"-", b"!", b"\\", b"/", b".", b"{", b"}", b"(", b")", b"|",
    b"+", b"$", b",", b":", b"a", b"b", b"q", b"z", b"Z", b"0", b"9", b"_", b"`", b"~", b"=",
    b"[:", b":]", b"[:alpha:]", b"[:digit:]", b"[:punct:]", b"[!", b"[^", b"**", b"\xc3\xa9",
    b"\xe2\x82\xac", b"\xf0\x9f\x98\x80", b"\x80", b"\xff", b"\xc3", b"\xed\xa0\x80",
];

/// Append 8 ASCII bytes so that an unchecked UTF-8 over-read (at most 5 extra
/// code units, see `PRIV(utf8_table4)` in pcre2_tables.c:128) stays inside the
/// buffer and `plength -= clength` cannot underflow.
fn pad_utf(p: &[u8]) -> Vec<u8> {
    let mut v = p.to_vec();
    v.extend_from_slice(b"WXYZwxyz");
    v
}

fn gen_conv(rng: &mut Rng) -> Vec<u8> {
    let n = rng.below(31);
    let mut v = Vec::new();
    for _ in 0..n {
        v.extend_from_slice(rng.pick(CONV_ALPHA));
    }
    v
}

/// Create a convert context in each library and apply separator/escape.
unsafe fn conv_ctx(c: &Api, r: &Api, sep: u32, esc: u32) -> (Ptr, Ptr) {
    let a = (c.pcre2_convert_context_create_8)(ptr::null_mut());
    let b = (r.pcre2_convert_context_create_8)(ptr::null_mut());
    assert!(!a.is_null() && !b.is_null());
    assert_eq!(
        (c.pcre2_set_glob_separator_8)(a, sep),
        (r.pcre2_set_glob_separator_8)(b, sep),
        "set_glob_separator({}) rc mismatch",
        sep
    );
    assert_eq!(
        (c.pcre2_set_glob_escape_8)(a, esc),
        (r.pcre2_set_glob_escape_8)(b, esc),
        "set_glob_escape({}) rc mismatch",
        esc
    );
    (a, b)
}

unsafe fn free_ctx(c: &Api, r: &Api, ctx: (Ptr, Ptr)) {
    (c.pcre2_convert_context_free_8)(ctx.0);
    (r.pcre2_convert_context_free_8)(ctx.1);
}

// ==================================================================== C193

#[test]
fn c193_convert_mode_legality() {
    println!("C193 pcre2_pattern_convert: option legality and the three buffer shapes");
    let (c, r) = both();
    let pats: &[&[u8]] = &[b"", b"a", b"a*b?[c-e]", b"\\(a\\)", b"**/x", b"[[:alpha:]]"];
    unsafe {
        // every option value in 0..=0xFF plus a spread of high/undefined bits
        let mut extra: Vec<u32> = vec![
            0x100,
            0x200,
            0x1000,
            0x8000,
            0x1_0000,
            0x8000_0000,
            0xFFFF_FFFF,
            0x30 | 0x50,
            0x30 | 0x4,
            0x50 | 0x8,
            0x70 | 0x3,
        ];
        for o in 0u32..=0xFF {
            extra.push(o);
        }
        for pat in pats {
            let mut buf = pat.to_vec();
            buf.push(0);
            for &o in &extra {
                let lc = conv_len(c, buf.as_ptr(), pat.len(), o, ptr::null_mut());
                let lr = conv_len(r, buf.as_ptr(), pat.len(), o, ptr::null_mut());
                assert_eq!(lc, lr, "C193 opts={:#x} pat={:?}", o, pat);
                let ac = conv_alloc(c, buf.as_ptr(), pat.len(), o, ptr::null_mut());
                let ar = conv_alloc(r, buf.as_ptr(), pat.len(), o, ptr::null_mut());
                assert_eq!(ac, ar, "C193 alloc opts={:#x} pat={:?}", o, pat);

                // legality follows the documented rule
                let pattype = o & 0x1C;
                let legal = (o & !0x7F_u32) == 0
                    && (o & 0x80) == 0
                    && pattype != 0
                    && pattype.count_ones() == 1;
                if !legal {
                    assert_eq!(
                        ac.rc, PCRE2_ERROR_BADOPTION,
                        "C193 opts={:#x} should be illegal",
                        o
                    );
                    assert_eq!(ac.blen, 0, "C193 error offset for opts={:#x}", o);
                }
            }
        }

        // ---- pattern == NULL
        for &plen in &[0usize, 1, 5, PCRE2_ZERO_TERMINATED] {
            for &o in &[
                0u32,
                PCRE2_CONVERT_GLOB,
                PCRE2_CONVERT_POSIX_BASIC,
                PCRE2_CONVERT_POSIX_EXTENDED,
                0xFFFF_FFFF,
            ] {
                let ac = conv_alloc(c, ptr::null(), plen, o, ptr::null_mut());
                let ar = conv_alloc(r, ptr::null(), plen, o, ptr::null_mut());
                assert_eq!(ac, ar, "C193 NULL pattern plen={} opts={:#x}", plen, o);
                let lc = conv_len(c, ptr::null(), plen, o, ptr::null_mut());
                let lr = conv_len(r, ptr::null(), plen, o, ptr::null_mut());
                assert_eq!(lc, lr, "C193 NULL pattern len-only plen={}", plen);
            }
        }

        // ---- bufflenptr == NULL (and buffptr NULL / non-NULL)
        for &o in &[0u32, PCRE2_CONVERT_GLOB, PCRE2_CONVERT_POSIX_BASIC] {
            let a = (c.pcre2_pattern_convert_8)(
                b"a*".as_ptr(),
                2,
                o,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            );
            let b = (r.pcre2_pattern_convert_8)(
                b"a*".as_ptr(),
                2,
                o,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            );
            assert_eq!(a, b, "C193 bufflenptr NULL opts={:#x}", o);
            assert_eq!(a, PCRE2_ERROR_NULL);

            let mut bp: *mut u8 = ptr::null_mut();
            let a = (c.pcre2_pattern_convert_8)(
                b"a*".as_ptr(),
                2,
                o,
                &mut bp,
                ptr::null_mut(),
                ptr::null_mut(),
            );
            let mut bp2: *mut u8 = ptr::null_mut();
            let b = (r.pcre2_pattern_convert_8)(
                b"a*".as_ptr(),
                2,
                o,
                &mut bp2,
                ptr::null_mut(),
                ptr::null_mut(),
            );
            assert_eq!((a, bp.is_null()), (b, bp2.is_null()));
            assert_eq!(a, PCRE2_ERROR_NULL);
            // pattern NULL *and* bufflenptr NULL
            let a = (c.pcre2_pattern_convert_8)(
                ptr::null(),
                0,
                o,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            );
            let b = (r.pcre2_pattern_convert_8)(
                ptr::null(),
                0,
                o,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            );
            assert_eq!(a, b);
        }

        // ---- ccontext NULL vs non-NULL, all three buffer shapes, zero-terminated
        let ctx = conv_ctx(c, r, '/' as u32, '\\' as u32);
        for mode in [
            PCRE2_CONVERT_GLOB,
            PCRE2_CONVERT_POSIX_BASIC,
            PCRE2_CONVERT_POSIX_EXTENDED,
        ] {
            for pat in pats {
                for zt in [false, true] {
                    conv_case(c, r, "C193", pat, zt, mode, (ptr::null_mut(), ptr::null_mut()), true, true);
                    conv_case(c, r, "C193", pat, zt, mode, ctx, true, true);
                }
            }
        }
        free_ctx(c, r, ctx);
        conv_report("C193");
    }
}

// ==================================================================== C194

#[test]
fn c194_convert_utf() {
    println!("C194 pcre2_pattern_convert: PCRE2_CONVERT_UTF / NO_UTF_CHECK");
    let (c, r) = both();
    // Ill-formed UTF-8 shapes plus valid ones.
    let pats: &[&[u8]] = &[
        b"abc",
        b"\xc3\xa9",
        b"\xe2\x82\xac",
        b"\xf0\x9f\x98\x80",
        b"a\xc3\xa9*[\xe2\x82\xac-\xf0\x9f\x98\x80]",
        b"\x80",                 // isolated continuation byte
        b"\xc3",                 // truncated 2-byte
        b"\xe2\x82",             // truncated 3-byte
        b"\xf0\x9f\x98",         // truncated 4-byte
        b"\xc0\x80",             // overlong
        b"\xe0\x80\x80",         // overlong
        b"\xf0\x80\x80\x80",     // overlong
        b"\xed\xa0\x80",         // surrogate
        b"\xf4\x90\x80\x80",     // > 0x10FFFF
        b"\xf8\x88\x80\x80\x80", // 5-byte
        b"\xfe",
        b"\xff",
        b"a*\x80b",
        b"[\xff]",
        b"[a-\xc3]",
        b"\\\xc3",
        b"*\xed\xa0\x80*",
        b"\xc3\xa9\xff\xc3\xa9",
    ];
    unsafe {
        let seps = ['/' as u32, '\\' as u32, '.' as u32];
        for mode in [
            PCRE2_CONVERT_GLOB,
            PCRE2_CONVERT_POSIX_BASIC,
            PCRE2_CONVERT_POSIX_EXTENDED,
        ] {
            for &sep in &seps {
                let ctx = conv_ctx(c, r, sep, '\\' as u32);
                for pat in pats {
                    // utf == FALSE (GETCHARLENTEST/GETCHARINCTEST degrade to a
                    // single code unit), or utf == TRUE with the check on (the
                    // ill-formed bytes are rejected before any conversion): both
                    // are safe with the raw, possibly truncated sequences.
                    for extra in [0u32, PCRE2_CONVERT_NO_UTF_CHECK, PCRE2_CONVERT_UTF] {
                        conv_case(c, r, "C194", pat, false, mode | extra, ctx, true, true);
                        conv_case(c, r, "C194", pat, true, mode | extra, ctx, false, false);
                    }
                    // UTF|NO_UTF_CHECK lets the ill-formed bytes reach
                    // GETCHARLENTEST/GETCHARINCTEST.  Those macros trust
                    // utf8_table4 (max 5 extra bytes), so the input must carry
                    // enough trailing ASCII for the over-read to stay inside the
                    // buffer — otherwise the *C* is undefined too.
                    let padded = pad_utf(pat);
                    let both = mode | PCRE2_CONVERT_UTF | PCRE2_CONVERT_NO_UTF_CHECK;
                    conv_case(c, r, "C194", &padded, false, both, ctx, true, true);
                    conv_case(c, r, "C194", &padded, true, both, ctx, false, false);
                }
                free_ctx(c, r, ctx);
            }
        }

        // a non-ASCII separator/escape under UTF is rejected by convert_glob;
        // the setters only accept ASCII values, so drive that path through the
        // raw context bytes is impossible — instead confirm the setters agree.
        let ctx = conv_ctx(c, r, '.' as u32, '`' as u32);
        for pat in pats {
            conv_case(c, r, "C194", pat, false, PCRE2_CONVERT_GLOB | PCRE2_CONVERT_UTF, ctx, false, true);
        }
        free_ctx(c, r, ctx);

        // random corpus, UTF on and off
        let mut rng = Rng::new(0x1907_5194_C194_0001);
        for i in 0..3000 {
            let p0 = gen_conv(&mut rng);
            let mode = *rng.pick(&[
                PCRE2_CONVERT_GLOB,
                PCRE2_CONVERT_POSIX_BASIC,
                PCRE2_CONVERT_POSIX_EXTENDED,
            ]);
            let extra = *rng.pick(&[
                0u32,
                PCRE2_CONVERT_UTF,
                PCRE2_CONVERT_NO_UTF_CHECK,
                PCRE2_CONVERT_UTF | PCRE2_CONVERT_NO_UTF_CHECK,
            ]);
            // see the comment above: the unchecked-UTF path may over-read by up
            // to 5 code units, so pad the input for that combination only.
            let p = if extra == (PCRE2_CONVERT_UTF | PCRE2_CONVERT_NO_UTF_CHECK) {
                pad_utf(&p0)
            } else {
                p0
            };
            conv_case(
                c,
                r,
                "C194/rand",
                &p,
                i % 3 == 0,
                mode | extra,
                (ptr::null_mut(), ptr::null_mut()),
                i % 10 == 0,
                i % 5 == 0,
            );
        }
        conv_report("C194");
    }
}

// ------------------------------------------------ shared POSIX input corpus

const POSIX_SHAPES: &[&[u8]] = &[
    b"",
    b"a",
    b"a*b",
    b"*a",
    b"(a",
    b"\\(a\\)",
    b"\\{2\\}",
    b"\\1",
    b"\\2",
    b"\\9",
    b"\\0",
    b"\\z",
    b"^a",
    b"a^b",
    b"(^a",
    b"$a",
    b"a$",
    b"[abc]",
    b"[^abc]",
    b"[]abc]",
    b"[a-z]",
    b"[[:alpha:]]",
    b"[[:alpha:",
    b"[abc",
    b"a\\",
    b"**",
    b"(*",
    b"?a",
    b"+a",
    b"{2}",
    b"|a",
    b".a",
    b"\xc3\xa9",
    b"a\xe2\x82\xacb",
    // extended-specific extras (C196)
    b"(a)",
    b")a",
    b"a|b",
    b"a?",
    b"a+",
    b"a{2,3}",
    b"a\\1",
    b"\\(",
    b"^a^b",
    b"***",
    b"()",
    b"(a)(b)",
    b"((a)",
    b"a))",
    b"[[:alpha:]x]",
    b"[[:nosuch:]]",
    b"[]]",
    b"[^]]",
    b"[^]a]",
    b"[a\\]b]",
    b"[\\]",
    b"[-a]",
    b"[a-]",
    b"[/]",
    b"[.]",
    b"[\\\\]",
    b"\\[",
    b"\\]",
    b"\\.",
    b"\\*",
    b"\\\\",
    b"a**b",
    b"(*a)",
    b"(a*)",
    b"^*a",
    b"a\\{2,3\\}",
    b"[[:upper:][:lower:]]",
    b"[[.a.]]",
    b"[[=a=]]",
    b"\\<a\\>",
    b"a\nb",
    b"a\rb",
    b"a\0b",
];

#[test]
fn c195_convert_posix_basic() {
    println!("C195 pcre2_pattern_convert: PCRE2_CONVERT_POSIX_BASIC");
    let (c, r) = both();
    unsafe {
        // NO_WILD_SEPARATOR / NO_STARSTAR carry the GLOB bit, so they cannot be
        // combined with POSIX_BASIC (two type bits) — verify that, then run the
        // legal shapes.
        for &bad in &[
            PCRE2_CONVERT_POSIX_BASIC | PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR,
            PCRE2_CONVERT_POSIX_BASIC | PCRE2_CONVERT_GLOB_NO_STARSTAR,
        ] {
            let a = conv_alloc(c, b"a\0".as_ptr(), 1, bad, ptr::null_mut());
            let b = conv_alloc(r, b"a\0".as_ptr(), 1, bad, ptr::null_mut());
            assert_eq!(a, b);
            assert_eq!(a.rc, PCRE2_ERROR_BADOPTION, "C195 opts={:#x}", bad);
        }
        // The bit 0x20 / 0x40 halves on their own are "silently ignored" extras.
        for &opts in &[
            PCRE2_CONVERT_POSIX_BASIC,
            PCRE2_CONVERT_POSIX_BASIC | 0x20,
            PCRE2_CONVERT_POSIX_BASIC | 0x40,
            PCRE2_CONVERT_POSIX_BASIC | 0x60,
        ] {
            for pat in POSIX_SHAPES {
                conv_case(
                    c,
                    r,
                    "C195",
                    pat,
                    false,
                    opts,
                    (ptr::null_mut(), ptr::null_mut()),
                    true,
                    true,
                );
            }
        }
        // some documented outcomes, so a "both wrong" case is visible
        let e = conv_alloc(c, b"a\\\0".as_ptr(), 2, PCRE2_CONVERT_POSIX_BASIC, ptr::null_mut());
        assert_eq!(e.rc, PCRE2_ERROR_END_BACKSLASH, "C195 trailing backslash");
        let e = conv_alloc(c, b"[abc\0".as_ptr(), 4, PCRE2_CONVERT_POSIX_BASIC, ptr::null_mut());
        assert_eq!(
            e.rc, PCRE2_ERROR_MISSING_SQUARE_BRACKET,
            "C195 unterminated class"
        );

        // fixed-seed random corpus
        let mut rng = Rng::new(0x1907_5195_C195_0001);
        for i in 0..3000 {
            let p = gen_conv(&mut rng);
            conv_case(
                c,
                r,
                "C195/rand",
                &p,
                i % 4 == 0,
                PCRE2_CONVERT_POSIX_BASIC,
                (ptr::null_mut(), ptr::null_mut()),
                i % 10 == 0,
                i % 5 == 0,
            );
        }
        conv_report("C195");
    }
}

#[test]
fn c196_convert_posix_extended() {
    println!("C196 pcre2_pattern_convert: PCRE2_CONVERT_POSIX_EXTENDED");
    let (c, r) = both();
    unsafe {
        for pat in POSIX_SHAPES {
            conv_case(
                c,
                r,
                "C196",
                pat,
                false,
                PCRE2_CONVERT_POSIX_EXTENDED,
                (ptr::null_mut(), ptr::null_mut()),
                true,
                true,
            );
            conv_case(
                c,
                r,
                "C196",
                pat,
                true,
                PCRE2_CONVERT_POSIX_EXTENDED,
                (ptr::null_mut(), ptr::null_mut()),
                false,
                true,
            );
        }
        let mut rng = Rng::new(0x1907_5196_C196_0001);
        for i in 0..3000 {
            let p = gen_conv(&mut rng);
            conv_case(
                c,
                r,
                "C196/rand",
                &p,
                i % 4 == 1,
                PCRE2_CONVERT_POSIX_EXTENDED,
                (ptr::null_mut(), ptr::null_mut()),
                i % 10 == 0,
                i % 5 == 0,
            );
        }
        conv_report("C196");
    }
}

// ------------------------------------------------------- glob input corpora

const GLOB_STARS: &[&[u8]] = &[
    b"",
    b"a",
    b"*",
    b"**",
    b"***",
    b"****",
    b"?",
    b"a?b",
    b"*a",
    b"a*",
    b"a**b",
    b"**/x",
    b"/**",
    b"a/**/b",
    b"a/**\\/b",
    b"a/**\\b",
    b"x/**",
    b"x**",
    b"**a",
    b"**/",
    b"/**/",
    b"a**",
    b"a*",
    b"*/",
    b"/*",
    b"/*/",
    b"a/*/b",
    b"\\*",
    b"\\",
    b"a\\",
    b"\\\\",
    b"\\a",
    b"a\\*b",
    b"*\\*",
    b"**\\**",
    b"a/**/**/b",
    b"**/**",
    b"*.*",
    b"*.txt",
    b"a.b.c",
    b"a|b",
    b"a+b",
    b"a{b}",
    b"a(b)",
    b"a^b",
    b"a$b",
    b"...",
    b"a/b/c",
    b"//",
    b"a//b",
    b"*?*",
    b"?*?",
    b"**?",
    b"?**",
    b"\xc3\xa9*",
    b"*\xc3\xa9",
];

const GLOB_CLASSES: &[&[u8]] = &[
    b"[abc]",
    b"[!abc]",
    b"[^abc]",
    b"[]abc]",
    b"[a-z]",
    b"[z-a]",
    b"[9-0]",
    b"[a-[:digit:]]",
    b"[[:digit:]]",
    b"[[:punct:]]",
    b"[[:graph:]]",
    b"[[:print:]]",
    b"[[:ascii:]]",
    b"[[:alpha:]]",
    b"[[:lower:]]",
    b"[[:upper:]]",
    b"[[:alnum:]]",
    b"[[:blank:]]",
    b"[[:cntrl:]]",
    b"[[:space:]]",
    b"[[:word:]]",
    b"[[:xdigit:]]",
    b"[[:nosuch:]]",
    b"[a",
    b"[!",
    b"[",
    b"[]",
    b"[-a]",
    b"[a-]",
    b"[/]",
    b"[.]",
    b"[\\\\]",
    b"[a\\]b]",
    b"[!]a]",
    b"[^]a]",
    b"[!/]",
    b"[.-0]",
    b"[+-0]",
    b"[,-1]",
    b"[!.-0]",
    b"[[:punct:]x]",
    b"[x[:punct:]]",
    b"[[:alpha:][:digit:]]",
    b"[a-z0-9_]",
    b"[!a-z]",
    b"[\\-]",
    b"[a\\-z]",
    b"[[]",
    b"[]]",
    b"[^]]",
    b"[a-\\z]",
    b"[\xc3\xa9]",
    b"[a-\xc3\xa9]",
    b"[:]",
    b"[::]",
    b"[:a:]",
    b"[[:]",
    b"a[b]c",
    b"*[a-z]*",
    b"[a][b]",
];

#[test]
fn c197_convert_glob_stars() {
    println!("C197 pcre2_pattern_convert: PCRE2_CONVERT_GLOB star/question branches");
    let (c, r) = both();
    unsafe {
        for &sep in &['/' as u32, '\\' as u32, '.' as u32] {
            for &esc in &[0u32, '\\' as u32, '`' as u32, '*' as u32] {
                let ctx = conv_ctx(c, r, sep, esc);
                for pat in GLOB_STARS {
                    conv_case(c, r, "C197", pat, false, PCRE2_CONVERT_GLOB, ctx, true, true);
                    conv_case(c, r, "C197", pat, true, PCRE2_CONVERT_GLOB, ctx, false, true);
                }
                free_ctx(c, r, ctx);
            }
        }
        // random corpus with rotating separator/escape
        let seps = ['/' as u32, '\\' as u32, '.' as u32];
        let escs = [0u32, '\\' as u32, '`' as u32, '*' as u32, '/' as u32, '.' as u32];
        let mut ctxs = Vec::new();
        for &s in &seps {
            for &e in &escs {
                ctxs.push((s, e, conv_ctx(c, r, s, e)));
            }
        }
        let mut rng = Rng::new(0x1907_5197_C197_0001);
        for i in 0..3000 {
            let p = gen_conv(&mut rng);
            let k = rng.below(ctxs.len() as u32) as usize;
            conv_case(
                c,
                r,
                "C197/rand",
                &p,
                i % 4 == 2,
                PCRE2_CONVERT_GLOB,
                ctxs[k].2,
                i % 10 == 0,
                i % 5 == 0,
            );
        }
        for (_, _, cx) in ctxs {
            free_ctx(c, r, cx);
        }
        conv_report("C197");
    }
}

#[test]
fn c198_convert_glob_classes() {
    println!("C198 pcre2_pattern_convert: PCRE2_CONVERT_GLOB class parser");
    let (c, r) = both();
    unsafe {
        // every escape value the setter accepts, against every separator
        let mut accepted: Vec<u32> = Vec::new();
        for e in 0u32..=300 {
            let a = (c.pcre2_convert_context_create_8)(ptr::null_mut());
            let b = (r.pcre2_convert_context_create_8)(ptr::null_mut());
            let ra = (c.pcre2_set_glob_escape_8)(a, e);
            let rb = (r.pcre2_set_glob_escape_8)(b, e);
            assert_eq!(ra, rb, "C198 set_glob_escape({}) rc mismatch", e);
            if ra == 0 {
                accepted.push(e);
            }
            (c.pcre2_convert_context_free_8)(a);
            (r.pcre2_convert_context_free_8)(b);
        }
        assert_eq!(accepted.len(), 33, "C198 accepted escapes: {:?}", accepted);
        let mut accepted_sep: Vec<u32> = Vec::new();
        for s in 0u32..=300 {
            let a = (c.pcre2_convert_context_create_8)(ptr::null_mut());
            let b = (r.pcre2_convert_context_create_8)(ptr::null_mut());
            let ra = (c.pcre2_set_glob_separator_8)(a, s);
            let rb = (r.pcre2_set_glob_separator_8)(b, s);
            assert_eq!(ra, rb, "C198 set_glob_separator({}) rc mismatch", s);
            if ra == 0 {
                accepted_sep.push(s);
            }
            (c.pcre2_convert_context_free_8)(a);
            (r.pcre2_convert_context_free_8)(b);
        }
        assert_eq!(
            accepted_sep,
            vec!['.' as u32, '/' as u32, '\\' as u32],
            "C198 accepted separators"
        );

        // every accepted escape x every accepted separator over the class corpus
        for &sep in &accepted_sep {
            for &esc in &accepted {
                let ctx = conv_ctx(c, r, sep, esc);
                for pat in GLOB_CLASSES {
                    conv_case(c, r, "C198", pat, false, PCRE2_CONVERT_GLOB, ctx, false, false);
                }
                free_ctx(c, r, ctx);
            }
        }
        // full sweep (buffer shapes + compile) for the default-ish contexts
        for &sep in &accepted_sep {
            for &esc in &[0u32, '\\' as u32, '`' as u32, '*' as u32] {
                let ctx = conv_ctx(c, r, sep, esc);
                for pat in GLOB_CLASSES {
                    conv_case(c, r, "C198", pat, false, PCRE2_CONVERT_GLOB, ctx, true, true);
                    conv_case(c, r, "C198", pat, true, PCRE2_CONVERT_GLOB, ctx, false, true);
                }
                free_ctx(c, r, ctx);
            }
        }
        // random, class-heavy corpus
        let ctx = conv_ctx(c, r, '/' as u32, '\\' as u32);
        let mut rng = Rng::new(0x1907_5198_C198_0001);
        for i in 0..3000 {
            let mut p = gen_conv(&mut rng);
            if i % 2 == 0 {
                p.insert(0, b'[');
            }
            conv_case(
                c,
                r,
                "C198/rand",
                &p,
                i % 4 == 3,
                PCRE2_CONVERT_GLOB,
                ctx,
                i % 10 == 0,
                i % 5 == 0,
            );
        }
        free_ctx(c, r, ctx);
        conv_report("C198");
    }
}

#[test]
fn c199_convert_glob_composites() {
    println!("C199 pcre2_pattern_convert: GLOB vs NO_WILD_SEPARATOR vs NO_STARSTAR");
    let (c, r) = both();
    let modes = [
        PCRE2_CONVERT_GLOB,
        PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR,
        PCRE2_CONVERT_GLOB_NO_STARSTAR,
        PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR | PCRE2_CONVERT_GLOB_NO_STARSTAR,
    ];
    unsafe {
        for &m in &modes {
            for &sep in &['/' as u32, '\\' as u32, '.' as u32] {
                for &esc in &[0u32, '\\' as u32, '`' as u32, '*' as u32] {
                    let ctx = conv_ctx(c, r, sep, esc);
                    for pat in GLOB_STARS.iter().chain(GLOB_CLASSES.iter()) {
                        conv_case(c, r, "C199", pat, false, m, ctx, false, true);
                    }
                    free_ctx(c, r, ctx);
                }
            }
        }
        // full buffer sweep for the default context
        let ctx = conv_ctx(c, r, '/' as u32, '\\' as u32);
        for &m in &modes {
            for pat in GLOB_STARS.iter().chain(GLOB_CLASSES.iter()) {
                conv_case(c, r, "C199", pat, false, m, ctx, true, true);
            }
        }
        // random corpus across all four composite modes
        let mut rng = Rng::new(0x1907_5199_C199_0001);
        for i in 0..3000 {
            let p = gen_conv(&mut rng);
            let m = *rng.pick(&modes);
            conv_case(
                c,
                r,
                "C199/rand",
                &p,
                i % 5 == 0,
                m,
                ctx,
                i % 10 == 0,
                i % 5 == 0,
            );
        }
        free_ctx(c, r, ctx);
        conv_report("C199");
    }
}

// ==================================================================== C200

struct Ctr {
    n_alloc: usize,
    n_free: usize,
    n_free_null: usize,
    bad_free: usize,
    sizes: Vec<usize>,
    live: HashMap<usize, usize>,
}

impl Ctr {
    fn new() -> Box<Ctr> {
        Box::new(Ctr {
            n_alloc: 0,
            n_free: 0,
            n_free_null: 0,
            bad_free: 0,
            sizes: Vec::new(),
            live: HashMap::new(),
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
struct CtrSum {
    n_alloc: usize,
    n_free: usize,
    n_free_null: usize,
    bad_free: usize,
    live: usize,
    sizes: Vec<usize>,
}

fn sum(c: &Ctr) -> CtrSum {
    CtrSum {
        n_alloc: c.n_alloc,
        n_free: c.n_free,
        n_free_null: c.n_free_null,
        bad_free: c.bad_free,
        live: c.live.len(),
        sizes: c.sizes.clone(),
    }
}

/// A 16-byte header holds the allocation size so `dealloc` can be exact.
unsafe extern "C" fn ctr_malloc(size: usize, data: Ptr) -> Ptr {
    let c = &mut *(data as *mut Ctr);
    c.n_alloc += 1;
    c.sizes.push(size);
    let total = size + 16;
    let layout = std::alloc::Layout::from_size_align(total, 16).unwrap();
    let p = std::alloc::alloc(layout);
    if p.is_null() {
        return ptr::null_mut();
    }
    (p as *mut usize).write(total);
    let user = p.add(16);
    c.live.insert(user as usize, size);
    user as Ptr
}

unsafe extern "C" fn ctr_free(p: Ptr, data: Ptr) {
    let c = &mut *(data as *mut Ctr);
    if p.is_null() {
        c.n_free_null += 1;
        return;
    }
    c.n_free += 1;
    if c.live.remove(&(p as usize)).is_none() {
        c.bad_free += 1;
        return;
    }
    let base = (p as *mut u8).sub(16);
    let total = (base as *mut usize).read();
    std::alloc::dealloc(
        base,
        std::alloc::Layout::from_size_align(total, 16).unwrap(),
    );
}

/// Exercise every allocating API and its matching free function through a
/// custom allocator installed via a general context.
unsafe fn c200_scenario(api: &Api, ctr: *mut Ctr, pattern: &[u8], subject: &[u8]) {
    let g = (api.pcre2_general_context_create_8)(Some(ctr_malloc), Some(ctr_free), ctr as Ptr);
    assert!(!g.is_null(), "{}: general_context_create", api.tag);

    let cc = (api.pcre2_compile_context_create_8)(g);
    let mc = (api.pcre2_match_context_create_8)(g);
    let vc = (api.pcre2_convert_context_create_8)(g);
    assert!(!cc.is_null() && !mc.is_null() && !vc.is_null());
    let g2 = (api.pcre2_general_context_copy_8)(g);
    let cc2 = (api.pcre2_compile_context_copy_8)(cc);
    let mc2 = (api.pcre2_match_context_copy_8)(mc);
    let vc2 = (api.pcre2_convert_context_copy_8)(vc);
    assert!(!g2.is_null() && !cc2.is_null() && !mc2.is_null() && !vc2.is_null());

    // ---- maketables through the custom allocator
    let tab = (api.pcre2_maketables_8)(g);
    assert!(!tab.is_null());
    assert_eq!((api.pcre2_set_character_tables_8)(cc, tab), 0);

    // ---- compile
    let mut ec = 0i32;
    let mut eo = 0usize;
    let code = (api.pcre2_compile_8)(pattern.as_ptr(), pattern.len(), 0, &mut ec, &mut eo, cc);
    assert!(
        !code.is_null(),
        "{}: compile failed ec={} eo={}",
        api.tag,
        ec,
        eo
    );

    // ---- match data (with and without heapframes, and with a copied subject)
    let subj = subject;
    let md = (api.pcre2_match_data_create_from_pattern_8)(code, g);
    assert!(!md.is_null());
    let rc = (api.pcre2_match_8)(code, subj.as_ptr(), subj.len(), 0, 0, md, mc);
    assert!(
        rc > 0,
        "{}: match rc={} (the C200 scenario needs a match)",
        api.tag,
        rc
    );
    assert!((api.pcre2_get_match_data_heapframes_size_8)(md) > 0);

    // substring get / list get, freed with their own free functions
    let mut sp: *mut u8 = ptr::null_mut();
    let mut sl: usize = 0;
    let grc = (api.pcre2_substring_get_bynumber_8)(md, 0, &mut sp, &mut sl);
    assert_eq!(grc, 0);
    (api.pcre2_substring_free_8)(sp);
    let mut lst: *mut *mut u8 = ptr::null_mut();
    let mut lens: *mut usize = ptr::null_mut();
    let lrc = (api.pcre2_substring_list_get_8)(md, &mut lst, &mut lens);
    assert_eq!(lrc, 0);
    (api.pcre2_substring_list_free_8)(lst);
    // list without lengths
    let mut lst2: *mut *mut u8 = ptr::null_mut();
    assert_eq!(
        (api.pcre2_substring_list_get_8)(md, &mut lst2, ptr::null_mut()),
        0
    );
    (api.pcre2_substring_list_free_8)(lst2);

    // a match_data with PCRE2_MD_COPIED_SUBJECT
    let md2 = (api.pcre2_match_data_create_8)(4, g);
    let rc = (api.pcre2_match_8)(
        code,
        subj.as_ptr(),
        subj.len(),
        0,
        PCRE2_COPY_MATCHED_SUBJECT,
        md2,
        mc,
    );
    // rc == 0 means "matched, but the ovector was too small" - still a match,
    // so PCRE2_MD_COPIED_SUBJECT is set and the subject copy must be freed.
    assert!(rc >= 0, "{}: copied-subject match rc={}", api.tag, rc);
    (api.pcre2_match_data_free_8)(md2);
    // a match_data that never matched -> no heapframes
    let md3 = (api.pcre2_match_data_create_8)(1, g);
    assert_eq!((api.pcre2_get_match_data_heapframes_size_8)(md3), 0);
    (api.pcre2_match_data_free_8)(md3);

    // ---- serialize / deserialize (the decoded code gets PCRE2_DEREF_TABLES)
    let codes = [code];
    let mut sb: *mut u8 = ptr::null_mut();
    let mut ss: usize = 0;
    let src = (api.pcre2_serialize_encode_8)(codes.as_ptr() as *const Ptr, 1, &mut sb, &mut ss, g);
    assert_eq!(src, 1);
    let mut dec: [Ptr; 1] = [ptr::null_mut()];
    let drc = (api.pcre2_serialize_decode_8)(dec.as_mut_ptr(), 1, sb, g);
    assert_eq!(drc, 1);
    (api.pcre2_code_free_8)(dec[0]); // DEREF_TABLES path
    (api.pcre2_serialize_free_8)(sb);

    // ---- code_copy (no DEREF_TABLES) and code_copy_with_tables (DEREF_TABLES)
    let cpy = (api.pcre2_code_copy_8)(code);
    assert!(!cpy.is_null());
    (api.pcre2_code_free_8)(cpy);
    let cpyt = (api.pcre2_code_copy_with_tables_8)(code);
    assert!(!cpyt.is_null());
    (api.pcre2_code_free_8)(cpyt);

    // ---- converted pattern through the custom allocator
    let mut cb: *mut u8 = ptr::null_mut();
    let mut cl: usize = 0;
    let crc = (api.pcre2_pattern_convert_8)(
        b"a*b?[a-z]/**/x".as_ptr(),
        14,
        PCRE2_CONVERT_GLOB,
        &mut cb,
        &mut cl,
        vc,
    );
    assert_eq!(crc, 0);
    (api.pcre2_converted_pattern_free_8)(cb);

    // ---- release everything
    (api.pcre2_match_data_free_8)(md);
    (api.pcre2_code_free_8)(code);
    (api.pcre2_maketables_free_8)(g, tab);

    // ---- NULL is a documented no-op for every one of them
    (api.pcre2_code_free_8)(ptr::null_mut());
    (api.pcre2_match_data_free_8)(ptr::null_mut());
    (api.pcre2_substring_free_8)(ptr::null_mut());
    (api.pcre2_substring_list_free_8)(ptr::null_mut());
    (api.pcre2_serialize_free_8)(ptr::null_mut());
    (api.pcre2_converted_pattern_free_8)(ptr::null_mut());
    (api.pcre2_general_context_free_8)(ptr::null_mut());
    (api.pcre2_compile_context_free_8)(ptr::null_mut());
    (api.pcre2_match_context_free_8)(ptr::null_mut());
    (api.pcre2_convert_context_free_8)(ptr::null_mut());
    (api.pcre2_maketables_free_8)(ptr::null_mut(), ptr::null_mut());
    // gcontext non-NULL + tables NULL: the custom free() sees a NULL pointer
    (api.pcre2_maketables_free_8)(g, ptr::null_mut());

    // ---- the four context frees
    (api.pcre2_convert_context_free_8)(vc2);
    (api.pcre2_match_context_free_8)(mc2);
    (api.pcre2_compile_context_free_8)(cc2);
    (api.pcre2_general_context_free_8)(g2);
    (api.pcre2_convert_context_free_8)(vc);
    (api.pcre2_match_context_free_8)(mc);
    (api.pcre2_compile_context_free_8)(cc);
    (api.pcre2_general_context_free_8)(g);
}

#[test]
fn c200_free_family_memctl() {
    println!("C200 free functions: hidden pcre2_memctl prefix + custom allocator");
    let (c, r) = both();
    let long_name_pat = {
        // >20 named groups forces the heap-allocated named-group list, and a
        // long pattern forces the heap-allocated parsed-pattern workspace.
        let mut p = Vec::new();
        for i in 0..30 {
            p.extend_from_slice(format!("(?<n{}>a)", i).as_bytes());
        }
        for _ in 0..1500 {
            p.push(b'z');
        }
        p
    };
    let long_subj = {
        let mut s = vec![b'a'; 30];
        s.extend(std::iter::repeat(b'z').take(1500));
        s
    };
    let pats: Vec<(Vec<u8>, Vec<u8>)> = vec![
        (
            b"(?<n>a)(b)c\\1|x{2,4}[[:alpha:]]+(?C1)".to_vec(),
            b"zzabcaxxqq".to_vec(),
        ),
        (b"a".to_vec(), b"bbba".to_vec()),
        (long_name_pat, long_subj),
    ];
    unsafe {
        for (pat, subj) in &pats {
            let mut cc = Ctr::new();
            let mut rc_ = Ctr::new();
            c200_scenario(c, &mut *cc as *mut Ctr, pat, subj);
            c200_scenario(r, &mut *rc_ as *mut Ctr, pat, subj);
            let a = sum(&cc);
            let b = sum(&rc_);
            assert_eq!(a.bad_free, 0, "C200 C freed a pointer it never allocated");
            assert_eq!(b.bad_free, 0, "C200 Rust freed a pointer it never allocated");
            assert_eq!(a.live, 0, "C200 C leaked {} blocks", a.live);
            assert_eq!(b.live, 0, "C200 Rust leaked {} blocks", b.live);
            assert_eq!(
                a.n_alloc, a.n_free,
                "C200 C alloc/free counts differ ({} / {})",
                a.n_alloc, a.n_free
            );
            assert_eq!(
                b.n_alloc, b.n_free,
                "C200 Rust alloc/free counts differ ({} / {})",
                b.n_alloc, b.n_free
            );
            assert_eq!(
                a, b,
                "C200 allocator traffic differs for pattern {:?}",
                String::from_utf8_lossy(&pat[..pat.len().min(40)])
            );
            println!(
                "C200 pattern len {}: {} allocations, {} frees, {} free(NULL)",
                pat.len(),
                a.n_alloc,
                a.n_free,
                a.n_free_null
            );
        }

        // ---- the same free functions on default-allocator blocks
        for _ in 0..1 {
            let mut ec = 0i32;
            let mut eo = 0usize;
            let cco = (c.pcre2_compile_8)(
                b"(a)(b)".as_ptr(),
                6,
                0,
                &mut ec,
                &mut eo,
                ptr::null_mut(),
            );
            let rro = (r.pcre2_compile_8)(
                b"(a)(b)".as_ptr(),
                6,
                0,
                &mut ec,
                &mut eo,
                ptr::null_mut(),
            );
            assert!(!cco.is_null() && !rro.is_null());
            for (api, code) in [(c, cco), (r, rro)] {
                let md = (api.pcre2_match_data_create_from_pattern_8)(code, ptr::null_mut());
                let s = b"ab";
                assert!(
                    (api.pcre2_match_8)(
                        code,
                        s.as_ptr(),
                        2,
                        0,
                        0,
                        md,
                        ptr::null_mut()
                    ) >= 0
                );
                let mut sp: *mut u8 = ptr::null_mut();
                let mut sl = 0usize;
                assert_eq!(
                    (api.pcre2_substring_get_bynumber_8)(md, 1, &mut sp, &mut sl),
                    0
                );
                (api.pcre2_substring_free_8)(sp);
                let mut lst: *mut *mut u8 = ptr::null_mut();
                let mut lens: *mut usize = ptr::null_mut();
                assert_eq!(
                    (api.pcre2_substring_list_get_8)(md, &mut lst, &mut lens),
                    0
                );
                (api.pcre2_substring_list_free_8)(lst);
                let codes = [code];
                let mut sb: *mut u8 = ptr::null_mut();
                let mut ss = 0usize;
                assert_eq!(
                    (api.pcre2_serialize_encode_8)(
                        codes.as_ptr() as *const Ptr,
                        1,
                        &mut sb,
                        &mut ss,
                        ptr::null_mut()
                    ),
                    1
                );
                (api.pcre2_serialize_free_8)(sb);
                let mut cb: *mut u8 = ptr::null_mut();
                let mut cl = 0usize;
                assert_eq!(
                    (api.pcre2_pattern_convert_8)(
                        b"a*".as_ptr(),
                        2,
                        PCRE2_CONVERT_GLOB,
                        &mut cb,
                        &mut cl,
                        ptr::null_mut()
                    ),
                    0
                );
                (api.pcre2_converted_pattern_free_8)(cb);
                let tab = (api.pcre2_maketables_8)(ptr::null_mut());
                assert!(!tab.is_null());
                (api.pcre2_maketables_free_8)(ptr::null_mut(), tab);
                (api.pcre2_match_data_free_8)(md);
                (api.pcre2_code_free_8)(code);
            }
        }
    }
}

// ============================================================== C201 / C202

const JIT_OPTS: &[u32] = &[
    0,
    PCRE2_JIT_COMPLETE,
    PCRE2_JIT_PARTIAL_SOFT,
    PCRE2_JIT_PARTIAL_HARD,
    PCRE2_JIT_COMPLETE | PCRE2_JIT_PARTIAL_SOFT,
    PCRE2_JIT_COMPLETE | PCRE2_JIT_PARTIAL_SOFT | PCRE2_JIT_PARTIAL_HARD,
    PCRE2_JIT_INVALID_UTF,
    PCRE2_JIT_INVALID_UTF | PCRE2_JIT_COMPLETE,
    PCRE2_JIT_TEST_ALLOC,
    PCRE2_JIT_TEST_ALLOC | PCRE2_JIT_COMPLETE,
    PCRE2_JIT_TEST_ALLOC | PCRE2_JIT_INVALID_UTF,
    PCRE2_JIT_TEST_ALLOC | 0x8,
    0x8,
    0x10,
    0x20,
    0x40,
    0x80,
    0x400,
    0x1_0000,
    0x8000_0000,
    0xFFFF_FFFF,
    0x0000_0307,
];

#[test]
fn c201_jit_public_stubs() {
    println!("C201 JIT public stubs (SUPPORT_JIT undefined)");
    let (c, r) = both();
    let pats: &[(&[u8], u32)] = &[
        (b"abc", 0),
        (b"(a)(b)\\1", 0),
        (b"\\x{100}", PCRE2_UTF),
        (b"abc", PCRE2_MATCH_INVALID_UTF | PCRE2_UTF),
        (b"(*NO_JIT)abc", 0),
        (b"a*", PCRE2_UTF),
    ];
    unsafe {
        // PCRE2_CONFIG_JIT must be 0 and JITTARGET unavailable
        let mut vc: u32 = 0xFFFF_FFFF;
        let mut vr: u32 = 0xFFFF_FFFF;
        assert_eq!(
            (c.pcre2_config_8)(PCRE2_CONFIG_JIT, &mut vc as *mut u32 as Ptr),
            (r.pcre2_config_8)(PCRE2_CONFIG_JIT, &mut vr as *mut u32 as Ptr)
        );
        assert_eq!((vc, vr), (0, 0), "C201 PCRE2_CONFIG_JIT must be 0");
        let mut tbuf_c = [0x5Au8; 64];
        let mut tbuf_r = [0x5Au8; 64];
        assert_eq!(
            (c.pcre2_config_8)(PCRE2_CONFIG_JITTARGET, tbuf_c.as_mut_ptr() as Ptr),
            (r.pcre2_config_8)(PCRE2_CONFIG_JITTARGET, tbuf_r.as_mut_ptr() as Ptr)
        );
        assert_eq!(tbuf_c, tbuf_r, "C201 JITTARGET buffer");
        assert_eq!(
            (c.pcre2_config_8)(PCRE2_CONFIG_JITTARGET, tbuf_c.as_mut_ptr() as Ptr),
            PCRE2_ERROR_BADOPTION
        );

        // ---- pcre2_jit_compile
        for (pat, opts) in pats {
            for &o in JIT_OPTS {
                let mut ec = 0i32;
                let mut eo = 0usize;
                let cc = (c.pcre2_compile_8)(
                    pat.as_ptr(),
                    pat.len(),
                    *opts,
                    &mut ec,
                    &mut eo,
                    ptr::null_mut(),
                );
                let rr = (r.pcre2_compile_8)(
                    pat.as_ptr(),
                    pat.len(),
                    *opts,
                    &mut ec,
                    &mut eo,
                    ptr::null_mut(),
                );
                assert!(!cc.is_null() && !rr.is_null(), "C201 setup {:?}", pat);
                let before_c = info_all_keys(c, cc, cc);
                let before_r = info_all_keys(r, rr, rr);
                assert_eq!(before_c, before_r);

                let a = (c.pcre2_jit_compile_8)(cc, o);
                let b = (r.pcre2_jit_compile_8)(rr, o);
                assert_eq!(
                    a, b,
                    "C201 jit_compile({:?}, {:#x}) rc mismatch",
                    String::from_utf8_lossy(pat),
                    o
                );
                assert!(
                    a == PCRE2_ERROR_JIT_BADOPTION || a == PCRE2_ERROR_JIT_UNSUPPORTED,
                    "C201 jit_compile rc {} for opts {:#x}",
                    a,
                    o
                );
                // whatever it did to `re`, it must do identically in both
                let after_c = info_all_keys(c, cc, cc);
                let after_r = info_all_keys(r, rr, rr);
                assert_eq!(
                    after_c, after_r,
                    "C201 jit_compile side effects differ ({:?}, {:#x})",
                    String::from_utf8_lossy(pat),
                    o
                );
                // documented side effect: PCRE2_JIT_INVALID_UTF forces
                // PCRE2_MATCH_INVALID_UTF into overall_options
                let expect_change = (o & PCRE2_JIT_TEST_ALLOC) == 0
                    && (o & !0x107u32) == 0
                    && (o & PCRE2_JIT_INVALID_UTF) != 0;
                if expect_change {
                    let all_before =
                        u32::from_ne_bytes(before_c[0].obs[0..4].try_into().unwrap());
                    let all_after = u32::from_ne_bytes(after_c[0].obs[0..4].try_into().unwrap());
                    assert_eq!(
                        all_after,
                        all_before | PCRE2_MATCH_INVALID_UTF,
                        "C201 PCRE2_JIT_INVALID_UTF must set PCRE2_MATCH_INVALID_UTF"
                    );
                } else {
                    assert_eq!(
                        before_c[0].obs, after_c[0].obs,
                        "C201 ALLOPTIONS changed unexpectedly for opts {:#x}",
                        o
                    );
                }
                (c.pcre2_code_free_8)(cc);
                (r.pcre2_code_free_8)(rr);
            }
        }

        // ---- NULL code
        for &o in JIT_OPTS {
            let a = (c.pcre2_jit_compile_8)(ptr::null_mut(), o);
            let b = (r.pcre2_jit_compile_8)(ptr::null_mut(), o);
            assert_eq!(a, b, "C201 jit_compile(NULL, {:#x})", o);
        }

        // ---- pcre2_jit_match on a non-JIT-compiled code
        for (pat, opts) in pats {
            let mut ec = 0i32;
            let mut eo = 0usize;
            let cc = (c.pcre2_compile_8)(
                pat.as_ptr(),
                pat.len(),
                *opts,
                &mut ec,
                &mut eo,
                ptr::null_mut(),
            );
            let rr = (r.pcre2_compile_8)(
                pat.as_ptr(),
                pat.len(),
                *opts,
                &mut ec,
                &mut eo,
                ptr::null_mut(),
            );
            let mdc = (c.pcre2_match_data_create_from_pattern_8)(cc, ptr::null_mut());
            let mdr = (r.pcre2_match_data_create_from_pattern_8)(rr, ptr::null_mut());
            // run a real match first so every field of the match data is defined
            let subj = b"abcabc\x00abc";
            let m1 = (c.pcre2_match_8)(cc, subj.as_ptr(), subj.len(), 0, 0, mdc, ptr::null_mut());
            let m2 = (r.pcre2_match_8)(rr, subj.as_ptr(), subj.len(), 0, 0, mdr, ptr::null_mut());
            assert_eq!(m1, m2);
            let base_c = read_match(c, mdc, m1);
            let base_r = read_match(r, mdr, m2);
            assert_eq!(base_c, base_r);

            for &(off, o) in &[
                (0usize, 0u32),
                (1, 0),
                (0, PCRE2_NOTBOL),
                (0, PCRE2_PARTIAL_SOFT),
                (0, PCRE2_PARTIAL_HARD),
                (0, PCRE2_NO_JIT),
                (subj.len(), 0),
            ] {
                let a = (c.pcre2_jit_match_8)(cc, subj.as_ptr(), subj.len(), off, o, mdc, ptr::null_mut());
                let b = (r.pcre2_jit_match_8)(rr, subj.as_ptr(), subj.len(), off, o, mdr, ptr::null_mut());
                assert_eq!(a, b, "C201 jit_match rc mismatch off={} o={:#x}", off, o);
                assert_eq!(
                    a, PCRE2_ERROR_JIT_BADOPTION,
                    "C201 jit_match must return PCRE2_ERROR_JIT_BADOPTION"
                );
                // the stub must have stored rc in the match data and touched
                // nothing else
                let mut sz = 0usize;
                let ra = (c.pcre2_substring_length_bynumber_8)(mdc, 0, &mut sz);
                let rb = (r.pcre2_substring_length_bynumber_8)(mdr, 0, &mut sz);
                assert_eq!(ra, rb);
                assert_eq!(ra, PCRE2_ERROR_JIT_BADOPTION, "C201 match_data->rc");
                let now_c = read_match(c, mdc, a);
                let now_r = read_match(r, mdr, b);
                assert_eq!(now_c, now_r);
                assert_eq!(
                    now_c.ovector, base_c.ovector,
                    "C201 jit_match must not touch the ovector"
                );
                assert_eq!(now_c.startchar, base_c.startchar);
                assert_eq!(now_c.mark, base_c.mark);
            }
            (c.pcre2_match_data_free_8)(mdc);
            (r.pcre2_match_data_free_8)(mdr);
            (c.pcre2_code_free_8)(cc);
            (r.pcre2_code_free_8)(rr);
        }

        // ---- pcre2_jit_stack_create
        let sizes: &[(usize, usize)] = &[
            (0, 0),
            (0, 1024),
            (1, 1),
            (32 * 1024, 512 * 1024),
            (512 * 1024, 32 * 1024), // min > max
            (usize::MAX, 0),
            (0, usize::MAX),
            (usize::MAX, usize::MAX),
            (1024, 1024),
        ];
        let gc = (c.pcre2_general_context_create_8)(None, None, ptr::null_mut());
        let gr = (r.pcre2_general_context_create_8)(None, None, ptr::null_mut());
        assert!(!gc.is_null() && !gr.is_null());
        for &(lo, hi) in sizes {
            for (gcx, grx) in [(ptr::null_mut(), ptr::null_mut()), (gc, gr)] {
                let a = (c.pcre2_jit_stack_create_8)(lo, hi, gcx);
                let b = (r.pcre2_jit_stack_create_8)(lo, hi, grx);
                assert_eq!(
                    (a.is_null(), b.is_null()),
                    (true, true),
                    "C201 jit_stack_create({}, {}) must return NULL in both",
                    lo,
                    hi
                );
                (c.pcre2_jit_stack_free_8)(a);
                (r.pcre2_jit_stack_free_8)(b);
            }
        }

        // ---- pcre2_jit_stack_assign must not modify the match context
        let mcc = (c.pcre2_match_context_create_8)(ptr::null_mut());
        let mcr = (r.pcre2_match_context_create_8)(ptr::null_mut());
        // sizeof(pcre2_real_match_context) on LP64 without SUPPORT_JIT
        // (pcre2_intmodedep.h:618): memctl 24 + 6 pointers 48 + offset_limit 8
        // + 3 x uint32_t 12 = 92, padded to 96.
        const MCTX_BYTES: usize = 96;
        let snap = |p: Ptr| std::slice::from_raw_parts(p as *const u8, MCTX_BYTES).to_vec();
        let bc = snap(mcc);
        let br = snap(mcr);
        let dummy_cb: usize = 0xDEAD_BEEF;
        let dummy_data: usize = 0x1234_5678;
        for (cbp, dp) in [
            (ptr::null_mut::<u8>() as Ptr, ptr::null_mut::<u8>() as Ptr),
            (dummy_cb as Ptr, dummy_data as Ptr),
        ] {
            (c.pcre2_jit_stack_assign_8)(mcc, cbp, dp);
            (r.pcre2_jit_stack_assign_8)(mcr, cbp, dp);
            (c.pcre2_jit_stack_assign_8)(ptr::null_mut(), cbp, dp);
            (r.pcre2_jit_stack_assign_8)(ptr::null_mut(), cbp, dp);
        }
        assert_eq!(snap(mcc), bc, "C201 jit_stack_assign modified the C mcontext");
        assert_eq!(
            snap(mcr),
            br,
            "C201 jit_stack_assign modified the Rust mcontext"
        );

        // ---- jit_stack_free / jit_free_unused_memory with NULL and non-NULL
        (c.pcre2_jit_stack_free_8)(ptr::null_mut());
        (r.pcre2_jit_stack_free_8)(ptr::null_mut());
        let mut scratch = [0u8; 64];
        (c.pcre2_jit_stack_free_8)(scratch.as_mut_ptr() as Ptr);
        (r.pcre2_jit_stack_free_8)(scratch.as_mut_ptr() as Ptr);
        (c.pcre2_jit_free_unused_memory_8)(ptr::null_mut());
        (r.pcre2_jit_free_unused_memory_8)(ptr::null_mut());
        (c.pcre2_jit_free_unused_memory_8)(gc);
        (r.pcre2_jit_free_unused_memory_8)(gr);
        assert_eq!(scratch, [0u8; 64], "C201 jit_stack_free touched its argument");

        (c.pcre2_match_context_free_8)(mcc);
        (r.pcre2_match_context_free_8)(mcr);
        (c.pcre2_general_context_free_8)(gc);
        (r.pcre2_general_context_free_8)(gr);
    }
}

#[test]
fn c202_jit_priv_stubs() {
    println!("C202 PRIV JIT hooks (_pcre2_jit_free/_rodata/_get_size/_get_target)");
    let (c, r) = both();
    unsafe {
        // _pcre2_jit_get_target -> the fixed "JIT is not supported" string
        let tc = (c._pcre2_jit_get_target_8)();
        let tr = (r._pcre2_jit_get_target_8)();
        assert_eq!(
            tc.is_null(),
            tr.is_null(),
            "C202 jit_get_target NULL-ness differs"
        );
        if !tc.is_null() {
            let sc = std::ffi::CStr::from_ptr(tc).to_bytes().to_vec();
            let sr = std::ffi::CStr::from_ptr(tr).to_bytes().to_vec();
            assert_eq!(
                sc, sr,
                "C202 jit_get_target: C={:?} RUST={:?}",
                String::from_utf8_lossy(&sc),
                String::from_utf8_lossy(&sr)
            );
            assert_eq!(
                sc, b"JIT is not supported",
                "C202 unexpected jit_get_target string"
            );
            // calling twice must give the same string
            let sc2 = std::ffi::CStr::from_ptr((c._pcre2_jit_get_target_8)())
                .to_bytes()
                .to_vec();
            assert_eq!(sc, sc2);
        }

        // _pcre2_jit_get_size -> always 0
        let mut probes: Vec<Ptr> = vec![ptr::null_mut()];
        let mut blob = [0u8; 128];
        probes.push(blob.as_mut_ptr() as Ptr);
        probes.push(1usize as Ptr);
        probes.push(usize::MAX as Ptr);
        for &p in &probes {
            let a = (c._pcre2_jit_get_size_8)(p);
            let b = (r._pcre2_jit_get_size_8)(p);
            assert_eq!(a, b, "C202 jit_get_size({:?}) mismatch", p);
            assert_eq!(a, 0, "C202 jit_get_size must be 0 without SUPPORT_JIT");
        }

        // _pcre2_jit_free / _pcre2_jit_free_rodata must be callable no-ops,
        // including with a real code block and its memctl.
        let pats: &[&[u8]] = &[b"abc", b"(a)(b)\\1", b"[a-z]+(?<n>x)\\d"];
        for pat in pats {
            let mut ec = 0i32;
            let mut eo = 0usize;
            let cc = (c.pcre2_compile_8)(
                pat.as_ptr(),
                pat.len(),
                0,
                &mut ec,
                &mut eo,
                ptr::null_mut(),
            );
            let rr = (r.pcre2_compile_8)(
                pat.as_ptr(),
                pat.len(),
                0,
                &mut ec,
                &mut eo,
                ptr::null_mut(),
            );
            assert!(!cc.is_null() && !rr.is_null());
            let before_c = info_all_keys(c, cc, cc);
            let before_r = info_all_keys(r, rr, rr);
            assert_eq!(before_c, before_r);

            // &code->memctl is the head of the block
            for (api, code) in [(c, cc), (r, rr)] {
                (api._pcre2_jit_free_8)(ptr::null_mut(), ptr::null_mut());
                (api._pcre2_jit_free_8)(ptr::null_mut(), code);
                (api._pcre2_jit_free_8)(1usize as Ptr, code);
                (api._pcre2_jit_free_rodata_8)(ptr::null_mut(), ptr::null_mut());
                (api._pcre2_jit_free_rodata_8)(ptr::null_mut(), code);
                (api._pcre2_jit_free_rodata_8)(1usize as Ptr, code);
                assert_eq!((api._pcre2_jit_get_size_8)(code), 0);
            }
            // nothing observable may have changed, and the block must still be
            // usable / freeable
            assert_eq!(
                info_all_keys(c, cc, cc),
                before_c,
                "C202 PRIV JIT hooks changed the C code block"
            );
            assert_eq!(
                info_all_keys(r, rr, rr),
                before_r,
                "C202 PRIV JIT hooks changed the Rust code block"
            );
            assert_eq!(info_all_keys(c, cc, cc), info_all_keys(r, rr, rr));
            (c.pcre2_code_free_8)(cc);
            (r.pcre2_code_free_8)(rr);
        }
    }
}
