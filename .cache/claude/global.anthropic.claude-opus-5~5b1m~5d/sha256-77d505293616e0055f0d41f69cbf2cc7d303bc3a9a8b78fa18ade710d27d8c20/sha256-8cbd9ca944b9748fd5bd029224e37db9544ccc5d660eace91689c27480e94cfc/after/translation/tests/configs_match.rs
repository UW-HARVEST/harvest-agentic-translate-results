//! Differential tests for CONFIGS.md rows **C126 - C157**: the whole
//! `pcre2_match_8` configuration surface.
//!
//! For every case the pattern is compiled SEPARATELY in each library (the C code
//! object is only ever used with the C matcher and vice versa), the match data is
//! created by the same library, the match is run and the result is captured with
//! `read_match` (rc + the FULL ovector + startchar + the `(*MARK)` name) and
//! compared with `assert_eq!`-style equality.  Where the row is about limits the
//! `pcre2_get_match_data_heapframes_size_8` value is compared too.
//!
//! Everything goes through the two loaded `.so`s (see `tests/common/mod.rs`).

mod common;
use common::*;

use std::cell::RefCell;
use std::ptr;

// ===================================================================== constants
// pcre2_match.c:71-75
const PUBLIC_MATCH_OPTIONS: u32 = PCRE2_ANCHORED
    | PCRE2_ENDANCHORED
    | PCRE2_NOTBOL
    | PCRE2_NOTEOL
    | PCRE2_NOTEMPTY
    | PCRE2_NOTEMPTY_ATSTART
    | PCRE2_NO_UTF_CHECK
    | PCRE2_PARTIAL_HARD
    | PCRE2_PARTIAL_SOFT
    | PCRE2_NO_JIT
    | PCRE2_COPY_MATCHED_SUBJECT
    | PCRE2_DISABLE_RECURSELOOP_CHECK;

// pcre2.h:499-507
const PCRE2_OPTIMIZATION_NONE: u32 = 0;
const PCRE2_OPTIMIZATION_FULL: u32 = 1;
const PCRE2_AUTO_POSSESS_OFF: u32 = 65;
const PCRE2_DOTSTAR_ANCHOR_OFF: u32 = 67;
const PCRE2_START_OPTIMIZE_OFF: u32 = 69;

// pcre2.h:577-578
const PCRE2_CALLOUT_STARTMATCH: u32 = 0x0000_0001;
const PCRE2_CALLOUT_BACKTRACK: u32 = 0x0000_0002;

// pcre2_internal.h:550
const REQ_CU_MAX: usize = 5000;

/// Sentinel written into both ovectors before every match so that any slot the
/// implementation does NOT write compares equal deterministically (a freshly
/// malloc'd ovector is otherwise uninitialised and could differ between the two
/// allocators).
const OV_SENTINEL: usize = 0xA5A5_A5A5_A5A5_A5A5;

// ================================================================ compile spec
#[derive(Clone, Copy, Debug)]
struct Spec {
    copts: u32,
    xopts: u32,
    /// 0 = leave the context default
    newline: u32,
    /// 0 = leave the context default
    bsr: u32,
    /// u32::MAX = never call pcre2_set_optimize
    optimize: u32,
    /// 0 = pcre2_match_data_create_from_pattern
    oveccount: u32,
}

impl Spec {
    const fn z() -> Spec {
        Spec {
            copts: 0,
            xopts: 0,
            newline: 0,
            bsr: 0,
            optimize: u32::MAX,
            oveccount: 0,
        }
    }
    const fn c(copts: u32) -> Spec {
        let mut s = Spec::z();
        s.copts = copts;
        s
    }
    const fn cx(copts: u32, xopts: u32) -> Spec {
        let mut s = Spec::z();
        s.copts = copts;
        s.xopts = xopts;
        s
    }
    const fn nl(mut self, n: u32) -> Spec {
        self.newline = n;
        self
    }
    const fn bsr(mut self, n: u32) -> Spec {
        self.bsr = n;
        self
    }
    const fn ovec(mut self, n: u32) -> Spec {
        self.oveccount = n;
        self
    }
}

fn show(b: &[u8]) -> String {
    let mut s = String::from("\"");
    for &c in b {
        if c == b'\\' {
            s.push_str("\\\\");
        } else if c == b'"' {
            s.push_str("\\\"");
        } else if (0x20..0x7f).contains(&c) {
            s.push(c as char);
        } else {
            s.push_str(&format!("\\x{:02x}", c));
        }
    }
    s.push('"');
    s
}

unsafe fn mk_ccontext(api: &Api, sp: &Spec) -> Ptr {
    if sp.newline == 0 && sp.bsr == 0 && sp.optimize == u32::MAX && sp.xopts == 0 {
        return ptr::null_mut();
    }
    let cc = (api.pcre2_compile_context_create_8)(ptr::null_mut());
    assert!(!cc.is_null(), "{}: compile_context_create failed", api.tag);
    if sp.newline != 0 {
        assert_eq!((api.pcre2_set_newline_8)(cc, sp.newline), 0);
    }
    if sp.bsr != 0 {
        assert_eq!((api.pcre2_set_bsr_8)(cc, sp.bsr), 0);
    }
    if sp.xopts != 0 {
        assert_eq!((api.pcre2_set_compile_extra_options_8)(cc, sp.xopts), 0);
    }
    if sp.optimize != u32::MAX {
        assert_eq!((api.pcre2_set_optimize_8)(cc, sp.optimize), 0);
    }
    cc
}

unsafe fn fill_ov(api: &Api, md: Ptr) {
    let n = (api.pcre2_get_ovector_count_8)(md) as usize * 2;
    let p = (api.pcre2_get_ovector_pointer_8)(md);
    for i in 0..n {
        *p.add(i) = OV_SENTINEL;
    }
}

/// A readable buffer used for zero-length subjects (see `Duo::go`).
static EMPTY_SUBJ: [u8; 4] = [0, 0, 0, 0];

/// The pointer actually handed to `pcre2_match` for `s` (see `Duo::go`).
unsafe fn subj_ptr(s: &[u8]) -> Sptr {
    if s.is_empty() {
        EMPTY_SUBJ.as_ptr().add(1)
    } else {
        s.as_ptr()
    }
}

// ======================================================================== Duo
/// One pattern compiled in BOTH libraries, with its own match data and match
/// context in each.  `go()` runs the same match through both and compares.
struct Duo {
    row: &'static str,
    c: &'static Api,
    r: &'static Api,
    pat: Vec<u8>,
    sp: Spec,
    code_c: Ptr,
    code_r: Ptr,
    md_c: Ptr,
    md_r: Ptr,
    mc_c: Ptr,
    mc_r: Ptr,
    /// also compare pcre2_get_match_data_heapframes_size_8
    cmp_frames: bool,
    /// extra text appended to failure messages (current limit settings etc.)
    note: String,
    count: u64,
}

impl Duo {
    unsafe fn new(row: &'static str, pat: &[u8], sp: Spec) -> Option<Duo> {
        let (c, r) = (&both().0, &both().1);

        let mut ec1: i32 = 0;
        let mut eo1: usize = 0;
        let cc = mk_ccontext(c, &sp);
        let code_c = (c.pcre2_compile_8)(pat.as_ptr(), pat.len(), sp.copts, &mut ec1, &mut eo1, cc);
        if !cc.is_null() {
            (c.pcre2_compile_context_free_8)(cc);
        }

        let mut ec2: i32 = 0;
        let mut eo2: usize = 0;
        let cr = mk_ccontext(r, &sp);
        let code_r = (r.pcre2_compile_8)(pat.as_ptr(), pat.len(), sp.copts, &mut ec2, &mut eo2, cr);
        if !cr.is_null() {
            (r.pcre2_compile_context_free_8)(cr);
        }

        assert_eq!(
            code_c.is_null(),
            code_r.is_null(),
            "[{}] compile disagreement pat={} spec={:?}: C ec={} eo={}, RUST ec={} eo={}",
            row,
            show(pat),
            sp,
            ec1,
            eo1,
            ec2,
            eo2
        );
        if code_c.is_null() {
            assert_eq!(
                (ec1, eo1),
                (ec2, eo2),
                "[{}] compile error mismatch pat={} spec={:?}",
                row,
                show(pat),
                sp
            );
            eprintln!(
                "[{}] NOTE pattern {} spec={:?} does not compile (ec={} eo={}) - case skipped",
                row,
                show(pat),
                sp,
                ec1,
                eo1
            );
            return None;
        }

        let (md_c, md_r) = if sp.oveccount == 0 {
            (
                (c.pcre2_match_data_create_from_pattern_8)(code_c, ptr::null_mut()),
                (r.pcre2_match_data_create_from_pattern_8)(code_r, ptr::null_mut()),
            )
        } else {
            (
                (c.pcre2_match_data_create_8)(sp.oveccount, ptr::null_mut()),
                (r.pcre2_match_data_create_8)(sp.oveccount, ptr::null_mut()),
            )
        };
        assert!(!md_c.is_null() && !md_r.is_null());
        assert_eq!(
            (c.pcre2_get_ovector_count_8)(md_c),
            (r.pcre2_get_ovector_count_8)(md_r),
            "[{}] oveccount mismatch pat={}",
            row,
            show(pat)
        );
        assert_eq!(
            (c.pcre2_get_match_data_size_8)(md_c),
            (r.pcre2_get_match_data_size_8)(md_r),
            "[{}] match_data size mismatch pat={}",
            row,
            show(pat)
        );

        let mc_c = (c.pcre2_match_context_create_8)(ptr::null_mut());
        let mc_r = (r.pcre2_match_context_create_8)(ptr::null_mut());
        assert!(!mc_c.is_null() && !mc_r.is_null());

        // The `mark` and `startchar` fields of a freshly created match data block are
        // NOT initialised (pcre2_match_data.c:56-70 sets only oveccount/flags/
        // heapframes).  Prime both blocks with one ordinary match on an empty subject
        // so that a later early-error return (which never reaches
        // `match_data->mark = ...` at pcre2_match.c:8163) compares a deterministic
        // value in both libraries instead of malloc garbage.
        let empty = [0u8; 1];
        (c.pcre2_match_8)(code_c, empty.as_ptr(), 0, 0, 0, md_c, ptr::null_mut());
        (r.pcre2_match_8)(code_r, empty.as_ptr(), 0, 0, 0, md_r, ptr::null_mut());

        Some(Duo {
            row,
            c,
            r,
            pat: pat.to_vec(),
            sp,
            code_c,
            code_r,
            md_c,
            md_r,
            mc_c,
            mc_r,
            cmp_frames: false,
            note: String::new(),
        	count: 0,
        })
    }

    fn set_note(&mut self, s: String) {
        self.note = s;
    }

    unsafe fn set_limits(&self, m: Option<u32>, d: Option<u32>, h: Option<u32>) {
        if let Some(v) = m {
            assert_eq!((self.c.pcre2_set_match_limit_8)(self.mc_c, v), 0);
            assert_eq!((self.r.pcre2_set_match_limit_8)(self.mc_r, v), 0);
        }
        if let Some(v) = d {
            assert_eq!((self.c.pcre2_set_depth_limit_8)(self.mc_c, v), 0);
            assert_eq!((self.r.pcre2_set_depth_limit_8)(self.mc_r, v), 0);
        }
        if let Some(v) = h {
            assert_eq!((self.c.pcre2_set_heap_limit_8)(self.mc_c, v), 0);
            assert_eq!((self.r.pcre2_set_heap_limit_8)(self.mc_r, v), 0);
        }
    }

    unsafe fn set_offset_limit(&self, v: usize) {
        assert_eq!(
            (self.c.pcre2_set_offset_limit_8)(self.mc_c, v),
            (self.r.pcre2_set_offset_limit_8)(self.mc_r, v)
        );
    }

    /// Run the same match through both libraries and compare everything.
    unsafe fn go(&mut self, subj: &[u8], so: usize, mopts: u32) -> MatchOut {
        // An empty Rust slice carries a DANGLING pointer (`NonNull::dangling()` == 1
        // for `u8`), and pcre2 legitimately forms/reads `subject[-1]` (req_cu_ptr,
        // pcre2_match.c:7049 and the partial-match paths).  Hand it a real address.
        self.raw(subj_ptr(subj), subj.len(), so, mopts, false)
    }

    /// As `go` but with an explicit length (for PCRE2_ZERO_TERMINATED / NULL).
    unsafe fn raw(
        &mut self,
        subj: Sptr,
        len: usize,
        so: usize,
        mopts: u32,
        null_mcontext: bool,
    ) -> MatchOut {
        let (mcc, mcr) = if null_mcontext {
            (ptr::null_mut(), ptr::null_mut())
        } else {
            (self.mc_c, self.mc_r)
        };
        fill_ov(self.c, self.md_c);
        fill_ov(self.r, self.md_r);
        let rc_c = (self.c.pcre2_match_8)(self.code_c, subj, len, so, mopts, self.md_c, mcc);
        let a = read_match(self.c, self.md_c, rc_c);
        let hf_c = (self.c.pcre2_get_match_data_heapframes_size_8)(self.md_c);
        let rc_r = (self.r.pcre2_match_8)(self.code_r, subj, len, so, mopts, self.md_r, mcr);
        let b = read_match(self.r, self.md_r, rc_r);
        let hf_r = (self.r.pcre2_get_match_data_heapframes_size_8)(self.md_r);
        self.count += 1;
        if a != b || (self.cmp_frames && hf_c != hf_r) {
            let sbytes = if subj.is_null() {
                "<NULL>".to_string()
            } else {
                show(std::slice::from_raw_parts(
                    subj,
                    if len == PCRE2_ZERO_TERMINATED { 0 } else { len },
                ))
            };
            panic!(
                "[{}] DIVERGENCE\n  pattern       = {}\n  spec          = {:?}\n  note          = {}\n  subject       = {}\n  length        = {}\n  start_offset  = {}\n  match_options = {:#x}\n  C    = {:?} heapframes={}\n  RUST = {:?} heapframes={}",
                self.row,
                show(&self.pat),
                self.sp,
                self.note,
                sbytes,
                len as i64,
                so,
                mopts,
                a,
                hf_c,
                b,
                hf_r
            );
        }
        a
    }
}

impl Drop for Duo {
    fn drop(&mut self) {
        unsafe {
            (self.c.pcre2_match_data_free_8)(self.md_c);
            (self.r.pcre2_match_data_free_8)(self.md_r);
            (self.c.pcre2_match_context_free_8)(self.mc_c);
            (self.r.pcre2_match_context_free_8)(self.mc_r);
            (self.c.pcre2_code_free_8)(self.code_c);
            (self.r.pcre2_code_free_8)(self.code_r);
        }
    }
}

// ============================================================ subject builders
const LETTERS: &[u8] = b"aAbBcCxXqQ";
const MIXED: &[u8] = b"abcxyz012_ .-()[]{}|^$*+?\\/:;<>=!~@#%&'\"`,\t";
const NL_SHAPES: &[&[u8]] = &[
    b"\n",
    b"\r",
    b"\r\n",
    b"\x85",
    b"\xe2\x80\xa8", // U+2028
    b"\xe2\x80\xa9", // U+2029
    b"\x0b",
    b"\x0c",
    b"\x00",
];
const BAD_UTF: &[&[u8]] = &[
    b"\x80",
    b"\xbf",
    b"\xc0\x80",
    b"\xc1\xbf",
    b"\xc2",
    b"\xe0\x80\x80",
    b"\xe2\x28\xa1",
    b"\xe2\x82",
    b"\xed\xa0\x80", // surrogate
    b"\xf0\x80\x80\x80",
    b"\xf4\x90\x80\x80", // > U+10FFFF
    b"\xf5\x80\x80\x80",
    b"\xfe",
    b"\xff",
    b"\xf0\x9f",
];

fn push_utf8(v: &mut Vec<u8>, cp: u32) {
    if cp < 0x80 {
        v.push(cp as u8);
    } else if cp < 0x800 {
        v.push(0xc0 | (cp >> 6) as u8);
        v.push(0x80 | (cp & 0x3f) as u8);
    } else if cp < 0x10000 {
        v.push(0xe0 | (cp >> 12) as u8);
        v.push(0x80 | ((cp >> 6) & 0x3f) as u8);
        v.push(0x80 | (cp & 0x3f) as u8);
    } else {
        v.push(0xf0 | (cp >> 18) as u8);
        v.push(0x80 | ((cp >> 12) & 0x3f) as u8);
        v.push(0x80 | ((cp >> 6) & 0x3f) as u8);
        v.push(0x80 | (cp & 0x3f) as u8);
    }
}

/// Alphabet kinds for the subject generator.
const K_LETTERS: u8 = 0;
const K_MIXED: u8 = 1;
const K_HIGH: u8 = 2;
const K_UTF8: u8 = 3;
const K_BADUTF: u8 = 4;
const K_NEWLINE: u8 = 5;
const K_NUL: u8 = 6;
const K_RANDOM: u8 = 7;
const K_A: u8 = 8;
const K_PARENS: u8 = 9;
const K_WORDS: u8 = 10;

fn gen_subject(rng: &mut Rng, kind: u8, maxlen: usize) -> Vec<u8> {
    let n = rng.below(maxlen as u32 + 1) as usize;
    let mut v: Vec<u8> = Vec::with_capacity(n + 4);
    match kind {
        K_LETTERS => {
            for _ in 0..n {
                v.push(*rng.pick(LETTERS));
            }
        }
        K_MIXED => {
            for _ in 0..n {
                v.push(*rng.pick(MIXED));
            }
        }
        K_HIGH => {
            for _ in 0..n {
                v.push(0x80 | (rng.byte() & 0x7f));
            }
        }
        K_UTF8 => {
            while v.len() < n {
                let cp = match rng.below(4) {
                    0 => rng.below(0x80),
                    1 => 0x80 + rng.below(0x780),
                    2 => {
                        let c = 0x800 + rng.below(0xf800);
                        if (0xd800..=0xdfff).contains(&c) {
                            c + 0x800
                        } else {
                            c
                        }
                    }
                    _ => 0x10000 + rng.below(0x100000),
                };
                push_utf8(&mut v, cp);
            }
        }
        K_BADUTF => {
            while v.len() < n {
                match rng.below(3) {
                    0 => v.extend_from_slice(rng.pick(BAD_UTF)),
                    1 => v.push(*rng.pick(LETTERS)),
                    _ => push_utf8(&mut v, 0x80 + rng.below(0x2000)),
                }
            }
        }
        K_NEWLINE => {
            while v.len() < n {
                if rng.below(2) == 0 {
                    v.extend_from_slice(rng.pick(NL_SHAPES));
                } else {
                    v.push(*rng.pick(b"abx"));
                }
            }
        }
        K_NUL => {
            for _ in 0..n {
                if rng.below(3) == 0 {
                    v.push(0);
                } else {
                    v.push(*rng.pick(LETTERS));
                }
            }
        }
        K_A => {
            for _ in 0..n {
                v.push(*rng.pick(b"aab"));
            }
        }
        K_PARENS => {
            for _ in 0..n {
                v.push(*rng.pick(b"()()ab"));
            }
        }
        K_WORDS => {
            for _ in 0..n {
                v.push(*rng.pick(b"aA_1 \t-\xc3\xa9\xcf\x89\xe4\xb8\x80"));
            }
        }
        _ => {
            for _ in 0..n {
                v.push(rng.byte());
            }
        }
    }
    v
}

fn subjects(seed: u64, fixed: &[&[u8]], kinds: &[u8], per: usize, maxlen: usize) -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = fixed.iter().map(|s| s.to_vec()).collect();
    let mut rng = Rng::new(seed);
    for &k in kinds {
        for _ in 0..per {
            v.push(gen_subject(&mut rng, k, maxlen));
        }
    }
    v
}

fn utf_ok(s: &[u8]) -> bool {
    std::str::from_utf8(s).is_ok()
}

/// True when the combination would be *documented undefined behaviour*:
/// `PCRE2_NO_UTF_CHECK` together with a subject that is not valid UTF-8 and a
/// pattern in plain UTF mode (without `PCRE2_MATCH_INVALID_UTF`, which forces the
/// check anyway - pcre2_match.c:7274).  In that case the C itself indexes outside
/// `PRIV(ucd_stage1)` (see e.g. `GET_UCD` from `_pcre2_extuni`), so there is no
/// defined behaviour to compare against.  Every other invalid-UTF combination IS
/// tested.
fn ub_combo(sp: &Spec, mo: u32, s: &[u8]) -> bool {
    mo & PCRE2_NO_UTF_CHECK != 0
        && sp.copts & (PCRE2_UTF | PCRE2_MATCH_INVALID_UTF) == PCRE2_UTF
        && !utf_ok(s)
}

/// Cross every pattern with every subject, every start offset (step `step`) and
/// every match-option value.  Returns the number of comparisons.
unsafe fn drive(
    row: &'static str,
    pats: &[(&str, Spec)],
    subs: &[Vec<u8>],
    mopts: &[u32],
    step: usize,
) -> u64 {
    let mut total = 0u64;
    for (p, s) in pats {
        if let Some(mut d) = Duo::new(row, p.as_bytes(), *s) {
            for sub in subs {
                let mut so = 0usize;
                loop {
                    for &mo in mopts {
                        if ub_combo(s, mo, sub) {
                            continue;
                        }
                        d.go(sub, so, mo);
                    }
                    if so >= sub.len() {
                        break;
                    }
                    so = (so + step).min(sub.len());
                }
            }
            total += d.count;
        }
    }
    total
}

fn note(row: &str, n: u64) {
    println!("[{}] {} match comparisons", row, n);
}

// ============================================================ common option sets
const M_BOL_EOL: &[u32] = &[
    0,
    PCRE2_NOTBOL,
    PCRE2_NOTEOL,
    PCRE2_NOTBOL | PCRE2_NOTEOL,
];
const M_EMPTY: &[u32] = &[
    0,
    PCRE2_NOTEMPTY,
    PCRE2_NOTEMPTY_ATSTART,
    PCRE2_NOTEMPTY | PCRE2_NOTEMPTY_ATSTART,
];
const M_PARTIAL: &[u32] = &[0, PCRE2_PARTIAL_SOFT, PCRE2_PARTIAL_HARD];
const M_PLAIN: &[u32] = &[0];

// =============================================================================
// C126 - argument / length / option validation
// =============================================================================
#[test]
fn c126_null_length_and_option_checks() {
    println!("C126 pcre2_match argument, length and option validation");
    let (c, r) = (&both().0, &both().1);
    let mut total = 0u64;
    unsafe {
        // ---- match_data == NULL: PCRE2_ERROR_NULL, nothing else observable.
        {
            let d = Duo::new("C126", b"a", Spec::z()).unwrap();
            let s = b"abc";
            let rc_c = (c.pcre2_match_8)(
                d.code_c,
                s.as_ptr(),
                3,
                0,
                0,
                ptr::null_mut(),
                ptr::null_mut(),
            );
            let rc_r = (r.pcre2_match_8)(
                d.code_r,
                s.as_ptr(),
                3,
                0,
                0,
                ptr::null_mut(),
                ptr::null_mut(),
            );
            assert_eq!(rc_c, PCRE2_ERROR_NULL, "C126 C match_data NULL");
            assert_eq!(rc_r, rc_c, "C126 match_data NULL rc mismatch");
            total += 1;
        }

        // ---- code == NULL: PCRE2_ERROR_NULL, recorded in match_data->rc.
        {
            let mut d = Duo::new("C126", b"a", Spec::z()).unwrap();
            let s = b"abc";
            fill_ov(c, d.md_c);
            fill_ov(r, d.md_r);
            let rc_c = (c.pcre2_match_8)(ptr::null_mut(), s.as_ptr(), 3, 0, 0, d.md_c, d.mc_c);
            let a = read_match(c, d.md_c, rc_c);
            let rc_r = (r.pcre2_match_8)(ptr::null_mut(), s.as_ptr(), 3, 0, 0, d.md_r, d.mc_r);
            let b = read_match(r, d.md_r, rc_r);
            assert_eq!(rc_c, PCRE2_ERROR_NULL, "C126 C code NULL");
            assert_eq!(a, b, "C126 code NULL");
            d.count += 1;
            total += d.count;
        }

        // ---- subject NULL with length 0 (legal empty subject) and length 1 (NULL).
        for (pat, sp) in [
            ("a", Spec::z()),
            ("a*", Spec::z()),
            ("", Spec::z()),
            ("^$", Spec::z()),
            ("\\A", Spec::z()),
        ] {
            let mut d = Duo::new("C126", pat.as_bytes(), sp).unwrap();
            let o = d.raw(ptr::null(), 0, 0, 0, false);
            assert!(
                o.rc >= 0 || o.rc == PCRE2_ERROR_NOMATCH,
                "C126 NULL/0 should be a legal empty subject, got {}",
                o.rc
            );
            let o = d.raw(ptr::null(), 1, 0, 0, false);
            assert_eq!(o.rc, PCRE2_ERROR_NULL, "C126 NULL/1 must be PCRE2_ERROR_NULL");
            let o = d.raw(ptr::null(), 0, 0, 0, true); // NULL mcontext too
            assert!(o.rc >= 0 || o.rc == PCRE2_ERROR_NOMATCH);
            let o = d.raw(ptr::null(), PCRE2_ZERO_TERMINATED, 0, 0, false);
            assert_eq!(o.rc, PCRE2_ERROR_NULL);
            total += d.count;
        }

        // ---- PCRE2_ZERO_TERMINATED vs explicit length, embedded NULs.
        let zt_pats: &[&str] = &["a", "abc", "c\\z", "\\0", "a.c", "^abc$", "[\\x00]", ".*"];
        let zt_subs: &[&[u8]] = &[
            b"abc\0",
            b"\0abc\0",
            b"abc\0def\0",
            b"\0",
            b"abcabc\0abc\0",
            b"a\0c\0",
        ];
        for p in zt_pats {
            let mut d = Duo::new("C126", p.as_bytes(), Spec::z()).unwrap();
            for s in zt_subs {
                // zero-terminated form
                d.raw(s.as_ptr(), PCRE2_ZERO_TERMINATED, 0, 0, false);
                // every explicit length up to the full buffer
                for len in 0..=s.len() {
                    d.raw(s.as_ptr(), len, 0, 0, false);
                    d.raw(s.as_ptr(), len, 0, 0, true);
                }
            }
            total += d.count;
        }

        // ---- undefined option bits -> PCRE2_ERROR_BADOPTION.
        {
            let mut d = Duo::new("C126", b"(a)(b)?", Spec::z()).unwrap();
            let s = b"ab";
            for bit in 0..32u32 {
                let o = 1u32 << bit;
                let out = d.raw(s.as_ptr(), 2, 0, o, false);
                if o & PUBLIC_MATCH_OPTIONS == 0 {
                    assert_eq!(
                        out.rc,
                        PCRE2_ERROR_BADOPTION,
                        "C126 bit {:#x} should be rejected",
                        o
                    );
                } else {
                    assert_ne!(
                        out.rc,
                        PCRE2_ERROR_BADOPTION,
                        "C126 bit {:#x} is in PUBLIC_MATCH_OPTIONS and must be accepted",
                        o
                    );
                }
                // and combined with a legal bit
                d.raw(s.as_ptr(), 2, 0, o | PCRE2_NOTBOL, false);
                d.raw(s.as_ptr(), 2, 0, o | PCRE2_ANCHORED, false);
            }
            // random combinations
            let mut rng = Rng::new(0x126_0001);
            for _ in 0..3000 {
                let o = (rng.next_u64() as u32) & rng.next_u64() as u32;
                d.raw(s.as_ptr(), 2, 0, o, false);
            }
            total += d.count;
        }

        // ---- the whole legal option space on a small pattern set.
        let legal_bits = [
            PCRE2_NOTBOL,
            PCRE2_NOTEOL,
            PCRE2_NOTEMPTY,
            PCRE2_NOTEMPTY_ATSTART,
            PCRE2_ANCHORED,
            PCRE2_ENDANCHORED,
            PCRE2_NO_JIT,
            PCRE2_NO_UTF_CHECK,
            PCRE2_DISABLE_RECURSELOOP_CHECK,
        ];
        for p in ["a*", "(a)(b)?", "^a.c$"] {
            let mut d = Duo::new("C126", p.as_bytes(), Spec::z()).unwrap();
            for m in 0..(1u32 << legal_bits.len()) {
                let mut o = 0u32;
                for (i, b) in legal_bits.iter().enumerate() {
                    if m >> i & 1 == 1 {
                        o |= *b;
                    }
                }
                for s in [&b""[..], b"abc", b"aab"] {
                    d.go(s, 0, o);
                }
            }
            total += d.count;
        }
    }
    note("C126", total);
}

// =============================================================================
// C127 - start_offset validation (BADOFFSET / BADUTFOFFSET)
// =============================================================================
#[test]
fn c127_start_offset() {
    println!("C127 pcre2_match start_offset handling");
    let mut total = 0u64;
    unsafe {
        // non-UTF: every offset 0..=len+3
        let subs = subjects(
            0x127_0001,
            &[b"", b"a", b"abc", b"aaa", b"xabcx"],
            &[K_LETTERS, K_MIXED, K_NUL, K_HIGH],
            48,
            40,
        );
        for p in ["a", "^a", "a$", "abc", "\\w", ".", "\\Ga", "a*"] {
            let mut d = Duo::new("C127", p.as_bytes(), Spec::z()).unwrap();
            for s in &subs {
                for so in 0..=s.len() + 3 {
                    let o = d.raw(s.as_ptr(), s.len(), so, 0, false);
                    if so > s.len() {
                        assert_eq!(
                            o.rc,
                            PCRE2_ERROR_BADOFFSET,
                            "C127 start_offset {} > length {} must be BADOFFSET",
                            so,
                            s.len()
                        );
                    }
                }
            }
            total += d.count;
        }

        // UTF: mid-character offsets are BADUTFOFFSET (unless MATCH_INVALID_UTF)
        let utf_subs = subjects(
            0x127_0002,
            &[
                b"",
                b"\xc3\xa9",
                b"a\xc3\xa9b",
                b"\xe4\xb8\x80\xe4\xb8\x81",
                b"\xf0\x9f\x98\x80x",
                b"a\xc3\xa9\xe4\xb8\x80\xf0\x9f\x98\x80z",
            ],
            &[K_UTF8],
            10,
            16,
        );
        for (p, sp) in [
            ("a", Spec::c(PCRE2_UTF)),
            (".", Spec::c(PCRE2_UTF)),
            ("\\X", Spec::c(PCRE2_UTF)),
            ("(?<=.)b", Spec::c(PCRE2_UTF)),
            ("a", Spec::c(PCRE2_UTF | PCRE2_MATCH_INVALID_UTF)),
            (".", Spec::c(PCRE2_UTF | PCRE2_MATCH_INVALID_UTF)),
            ("(?<=.)b", Spec::c(PCRE2_UTF | PCRE2_MATCH_INVALID_UTF)),
        ] {
            let mut d = Duo::new("C127", p.as_bytes(), sp).unwrap();
            for s in &utf_subs {
                for so in 0..=s.len() + 1 {
                    for mo in [0, PCRE2_NO_UTF_CHECK] {
                        if ub_combo(&sp, mo, s) {
                            continue;
                        }
                        d.raw(s.as_ptr(), s.len(), so, mo, false);
                    }
                }
            }
            total += d.count;
        }
    }
    note("C127", total);
}

// =============================================================================
// C128 - PCRE2_NOTBOL / PCRE2_NOTEOL
// =============================================================================
#[test]
fn c128_notbol_noteol() {
    println!("C128 PCRE2_NOTBOL / PCRE2_NOTEOL");
    let base = [
        "^a", "a$", "^a$", "^", "$", "a\\Z", "a\\z", "\\Aa", "^a|b$", "(?m)^a",
    ];
    let copts = [
        0u32,
        PCRE2_MULTILINE,
        PCRE2_ALT_CIRCUMFLEX,
        PCRE2_DOLLAR_ENDONLY,
        PCRE2_MULTILINE | PCRE2_ALT_CIRCUMFLEX,
        PCRE2_MULTILINE | PCRE2_DOLLAR_ENDONLY,
    ];
    let mut pats: Vec<(&str, Spec)> = vec![];
    for p in base.iter() {
        for co in copts.iter() {
            pats.push((p, Spec::c(*co)));
        }
    }
    let subs = subjects(
        0x128_0001,
        &[
            b"", b"a", b"a\n", b"\na", b"a\r\n", b"aa", b"b", b"a\nb\na", b"\n", b"\r\n",
        ],
        &[K_NEWLINE, K_LETTERS],
        60,
        40,
    );
    let total = unsafe { drive("C128", &pats, &subs, M_BOL_EOL, 1) };
    note("C128", total);
}

// =============================================================================
// C129 - PCRE2_NOTEMPTY / PCRE2_NOTEMPTY_ATSTART (+ (*NOTEMPTY) etc.)
// =============================================================================
#[test]
fn c129_notempty() {
    println!("C129 PCRE2_NOTEMPTY / PCRE2_NOTEMPTY_ATSTART");
    let pats: Vec<(&str, Spec)> = vec![
        ("a*", Spec::z()),
        ("", Spec::z()),
        ("(?:)", Spec::z()),
        ("\\b", Spec::z()),
        ("(?=a)", Spec::z()),
        ("()", Spec::z()),
        ("a?", Spec::z()),
        ("a??", Spec::z()),
        ("b*", Spec::z()),
        ("(a*)(b*)", Spec::z()),
        ("(*NOTEMPTY)a*", Spec::z()),
        ("(*NOTEMPTY_ATSTART)a*", Spec::z()),
        ("(*NOTEMPTY)", Spec::z()),
        ("(*NOTEMPTY_ATSTART)(?:)", Spec::z()),
        ("(*NOTEMPTY)\\b", Spec::z()),
        ("(*NOTEMPTY_ATSTART)a?", Spec::z()),
        ("x|a*", Spec::z()),
        ("(?:a|)", Spec::z()),
    ];
    let subs = subjects(
        0x129_0001,
        &[b"", b"bbb", b"aaa", b"ab", b"ba", b" a ", b"a"],
        &[K_LETTERS, K_A, K_MIXED],
        72,
        40,
    );
    let total = unsafe { drive("C129", &pats, &subs, M_EMPTY, 1) };
    note("C129", total);
}

// =============================================================================
// C130 - PCRE2_ANCHORED / PCRE2_ENDANCHORED
// =============================================================================
#[test]
fn c130_anchored_endanchored() {
    println!("C130 PCRE2_ANCHORED / PCRE2_ENDANCHORED");
    let base = ["abc", "^abc", "abc$", "a", "a*", "(a)(b)?", "b|bc"];
    let copts = [
        0u32,
        PCRE2_ANCHORED,
        PCRE2_ENDANCHORED,
        PCRE2_ANCHORED | PCRE2_ENDANCHORED,
    ];
    let mut pats: Vec<(&str, Spec)> = vec![];
    for p in base.iter() {
        for co in copts.iter() {
            pats.push((p, Spec::c(*co)));
        }
    }
    let mopts: Vec<u32> = {
        let mut v = vec![];
        for a in [0, PCRE2_ANCHORED] {
            for b in [0, PCRE2_ENDANCHORED] {
                for c in [0, PCRE2_PARTIAL_SOFT] {
                    v.push(a | b | c);
                }
            }
        }
        v
    };
    let subs = subjects(
        0x130_0001,
        &[b"abc", b"xabc", b"abcx", b"xabcx", b"", b"a", b"aaa"],
        &[K_LETTERS, K_A],
        72,
        40,
    );
    let mut total = unsafe { drive("C130", &pats, &subs, &mopts, 1) };
    // PARTIAL + ENDANCHORED must be rejected with BADOPTION (both sources of the flag)
    unsafe {
        for (p, sp) in [
            ("abc", Spec::c(PCRE2_ENDANCHORED)),
            ("abc", Spec::z()),
            ("abc$", Spec::c(PCRE2_ENDANCHORED)),
        ] {
            let mut d = Duo::new("C130", p.as_bytes(), sp).unwrap();
            for mo in [
                PCRE2_PARTIAL_SOFT,
                PCRE2_PARTIAL_HARD,
                PCRE2_PARTIAL_SOFT | PCRE2_ENDANCHORED,
                PCRE2_PARTIAL_HARD | PCRE2_ENDANCHORED,
                PCRE2_PARTIAL_SOFT | PCRE2_PARTIAL_HARD | PCRE2_ENDANCHORED,
            ] {
                let o = d.go(b"ab", 0, mo);
                if sp.copts & PCRE2_ENDANCHORED != 0 || mo & PCRE2_ENDANCHORED != 0 {
                    assert_eq!(
                        o.rc,
                        PCRE2_ERROR_BADOPTION,
                        "C130 partial+ENDANCHORED must be BADOPTION"
                    );
                }
            }
            total += d.count;
        }
    }
    note("C130", total);
}

// =============================================================================
// C131 - pcre2_set_offset_limit + PCRE2_USE_OFFSET_LIMIT
// =============================================================================
#[test]
fn c131_offset_limit() {
    println!("C131 pcre2_set_offset_limit / PCRE2_USE_OFFSET_LIMIT");
    let mut total = 0u64;
    unsafe {
        let pats: &[(&str, Spec)] = &[
            ("b", Spec::z()),
            ("b", Spec::c(PCRE2_USE_OFFSET_LIMIT)),
            ("^b", Spec::c(PCRE2_MULTILINE)),
            ("^b", Spec::c(PCRE2_MULTILINE | PCRE2_USE_OFFSET_LIMIT)),
            ("a*", Spec::z()),
            ("a*", Spec::c(PCRE2_USE_OFFSET_LIMIT)),
            ("(a)b", Spec::c(PCRE2_USE_OFFSET_LIMIT)),
            (".", Spec::c(PCRE2_USE_OFFSET_LIMIT)),
            ("b$", Spec::c(PCRE2_USE_OFFSET_LIMIT)),
            (
                "b",
                Spec::c(PCRE2_USE_OFFSET_LIMIT | PCRE2_NO_START_OPTIMIZE),
            ),
        ];
        let subs = subjects(
            0x131_0001,
            &[b"aaab", b"b", b"", b"a\nb\nb", b"bbbb", b"aaaa"],
            &[K_A, K_NEWLINE],
            36,
            40,
        );
        for (p, sp) in pats {
            let mut d = Duo::new("C131", p.as_bytes(), *sp).unwrap();
            for s in &subs {
                // PCRE2_UNSET first (the default)
                d.set_offset_limit(PCRE2_UNSET);
                d.set_note("offset_limit=UNSET".into());
                for so in 0..=s.len() {
                    d.go(s, so, 0);
                }
                for lim in 0..=s.len() + 1 {
                    d.set_offset_limit(lim);
                    d.set_note(format!("offset_limit={}", lim));
                    for so in 0..=s.len() {
                        let o = d.go(s, so, 0);
                        if sp.copts & PCRE2_USE_OFFSET_LIMIT == 0 {
                            assert_eq!(
                                o.rc,
                                PCRE2_ERROR_BADOFFSETLIMIT,
                                "C131 offset limit without USE_OFFSET_LIMIT"
                            );
                        }
                    }
                }
                // restore so the next subject starts from UNSET
                d.set_offset_limit(PCRE2_UNSET);
            }
            total += d.count;
        }
    }
    note("C131", total);
}

// =============================================================================
// C132 - the start-of-match optimizations
// =============================================================================
#[test]
fn c132_start_optimizations() {
    println!("C132 start-of-match optimizations");
    let base: &[(&str, u32)] = &[
        ("abc", 0),
        ("(?i)abc", 0),
        ("[ab]c", 0),
        ("\\d+", 0),
        ("^abc", PCRE2_MULTILINE),
        ("a.*b", 0),
        ("^a.*b", 0),
        ("abcdef", 0),
        ("(?=a)ab", 0),
        ("a*a", 0),
        ("(?i)[ab]c", 0),
        ("\\babc", 0),
        (".*abc", 0),
        ("(?:abc|abd)", 0),
        ("(?i)éx", PCRE2_UTF),
    ];
    let variants: &[(u32, u32, u32)] = &[
        // (extra compile options, optimize directive, marker)
        (0, u32::MAX, 0),
        (PCRE2_NO_START_OPTIMIZE, u32::MAX, 1),
        (0, PCRE2_START_OPTIMIZE_OFF, 2),
        (0, PCRE2_OPTIMIZATION_NONE, 3),
        (0, PCRE2_OPTIMIZATION_FULL, 4),
        (0, PCRE2_AUTO_POSSESS_OFF, 5),
        (0, PCRE2_DOTSTAR_ANCHOR_OFF, 6),
    ];
    let mut pats: Vec<(String, Spec)> = vec![];
    for (p, co) in base {
        for (xco, opt, _) in variants {
            let mut sp = Spec::c(co | xco);
            sp.optimize = *opt;
            pats.push((p.to_string(), sp));
        }
        // the in-pattern form
        pats.push((format!("(*NO_START_OPT){}", p), Spec::c(*co)));
    }
    let pats_ref: Vec<(&str, Spec)> = pats.iter().map(|(a, b)| (a.as_str(), *b)).collect();

    let subs = subjects(
        0x132_0001,
        &[
            b"",
            b"a",
            b"abc",
            b"xxabc",
            b"ABC",
            b"aBc",
            b"zzz",
            b"a1b",
            b"123",
            b"abcdef",
            b"abcde",
            b"aaab",
            b"\xc3\xa9x",
            b"\xc3\x89X",
        ],
        &[K_LETTERS, K_MIXED, K_UTF8],
        48,
        40,
    );
    let mut total = unsafe { drive("C132", &pats_ref, &subs, M_PARTIAL, 1) };

    // the required-code-unit search is gated on the subject length: exercise both
    // sides of `check_length < REQ_CU_MAX` (anchored) and of
    // `check_length < REQ_CU_MAX*1000` (unanchored).
    unsafe {
        for (p, sp) in [
            ("a.*b", Spec::z()),
            ("^a.*b", Spec::z()),
            ("a.*b", Spec::c(PCRE2_ANCHORED)),
            ("ab", Spec::z()),
        ] {
            let mut d = Duo::new("C132", p.as_bytes(), sp).unwrap();
            for n in [
                REQ_CU_MAX - 1,
                REQ_CU_MAX,
                REQ_CU_MAX + 1,
                2 * REQ_CU_MAX,
            ] {
                let mut s = vec![b'a'; n];
                d.set_note(format!("len={} no b", n));
                d.go(&s, 0, 0);
                *s.last_mut().unwrap() = b'b';
                d.set_note(format!("len={} trailing b", n));
                d.go(&s, 0, 0);
            }
            total += d.count;
        }
        // the REQ_CU_MAX*1000 boundary (5 000 000) - "ab" makes both sides cheap.
        let mut d = Duo::new("C132", b"ab", Spec::z()).unwrap();
        for n in [REQ_CU_MAX * 1000 - 1, REQ_CU_MAX * 1000] {
            let mut s = vec![b'a'; n];
            d.set_note(format!("huge len={} match at 0", n));
            s[1] = b'b';
            d.go(&s, 0, 0);
            s[1] = b'a';
            d.set_note(format!("huge len={} no b", n));
            d.go(&s, 0, 0);
        }
        total += d.count;
    }
    note("C132", total);
}

// =============================================================================
// C133 - PCRE2_FIRSTLINE
// =============================================================================
#[test]
fn c133_firstline() {
    println!("C133 PCRE2_FIRSTLINE");
    let base = ["abc", "^abc", "b", "a.c", "^b", "c$"];
    let mut pats: Vec<(&str, Spec)> = vec![];
    for p in base.iter() {
        for fl in [0u32, PCRE2_FIRSTLINE] {
            for ml in [0u32, PCRE2_MULTILINE] {
                for nl in [
                    0,
                    PCRE2_NEWLINE_CR,
                    PCRE2_NEWLINE_LF,
                    PCRE2_NEWLINE_CRLF,
                    PCRE2_NEWLINE_ANY,
                    PCRE2_NEWLINE_ANYCRLF,
                    PCRE2_NEWLINE_NUL,
                ] {
                    pats.push((p, Spec::c(fl | ml).nl(nl)));
                }
            }
        }
    }
    let subs = subjects(
        0x133_0001,
        &[
            b"abc\nxbc",
            b"x\nabc",
            b"abc",
            b"\nabc",
            b"\r\nabc",
            b"abc\r\nabc",
            b"a\0bc\0abc",
            b"",
            b"\n",
            b"abc\x85xbc",
        ],
        &[K_NEWLINE, K_RANDOM],
        48,
        40,
    );
    let total = unsafe { drive("C133", &pats, &subs, M_PLAIN, 2) };
    note("C133", total);
}

// =============================================================================
// C134 - pcre2_set_match_limit / (*LIMIT_MATCH=)
// =============================================================================
#[test]
fn c134_match_limit() {
    println!("C134 pcre2_set_match_limit and (*LIMIT_MATCH=n)");
    let mut total = 0u64;
    unsafe {
        // ---- the fine-grained sweep: the exact limit at which the two
        // implementations start returning PCRE2_ERROR_MATCHLIMIT must coincide.
        for (p, subj) in [
            ("(a+)+b", "aaaaaaaaaaaaaaaaaaaa"),
            ("(a+)+b", "aaaaaaaaaaaaaaaaaaaac"),
            ("(a|aa)+c", "aaaaaaaaaaaaaaaaaaaa"),
            ("(?:a|a?)+b", "aaaaaaaaaaaaaaaa"),
            ("\\((?:[^()]|(?R))*\\)", "((((((((((x))))))))))"),
            ("\\((?:[^()]|(?R))*\\)", "((((((((((x)))))))))"),
        ] {
            let mut d = Duo::new("C134", p.as_bytes(), Spec::z()).unwrap();
            d.cmp_frames = true;
            let mut lim = 1u32;
            while lim <= 3000 {
                d.set_limits(Some(lim), None, None);
                d.set_note(format!("match_limit={}", lim));
                d.go(subj.as_bytes(), 0, 0);
                lim += if lim < 64 {
                    1
                } else if lim < 512 {
                    7
                } else {
                    23
                };
            }
            // limit 0 always fails
            d.set_limits(Some(0), None, None);
            d.set_note("match_limit=0".into());
            let o = d.go(subj.as_bytes(), 0, 0);
            assert!(
                o.rc == PCRE2_ERROR_MATCHLIMIT || o.rc == PCRE2_ERROR_NOMATCH,
                "C134 match_limit 0 gave {}",
                o.rc
            );
            total += d.count;
        }

        // ---- fully contiguous 0..=3000 sweep on one pattern: EVERY single value,
        // so the exact limit at which MATCHLIMIT starts must coincide.
        {
            let mut d = Duo::new("C134", b"(a+)+b", Spec::z()).unwrap();
            d.cmp_frames = true;
            for lim in 0..=3000u32 {
                d.set_limits(Some(lim), None, None);
                d.set_note(format!("match_limit={}", lim));
                for s in ["aaaaaaaaaa", "aaaaaaaaaab", "ab", ""] {
                    d.go(s.as_bytes(), 0, 0);
                }
            }
            total += d.count;
        }

        // ---- coarse values incl. the default and UINT32_MAX on cheap patterns.
        let cheap: &[(&str, Spec)] = &[
            ("a", Spec::z()),
            ("(a)(b)(c)", Spec::z()),
            ("a*b", Spec::z()),
            ("(*LIMIT_MATCH=5)(a+)+b", Spec::z()),
            ("(*LIMIT_MATCH=50)(a+)+b", Spec::z()),
            ("(*LIMIT_MATCH=500)(a+)+b", Spec::z()),
            ("(*LIMIT_MATCH=1)a", Spec::z()),
            ("(*LIMIT_MATCH=10000000)(a+)+b", Spec::z()),
        ];
        let subs = subjects(
            0x134_0001,
            &[b"", b"a", b"abc", b"aaaaaaaaaa", b"aaaaaaaaab"],
            &[K_A],
            36,
            40,
        );
        for (p, sp) in cheap {
            let mut d = Duo::new("C134", p.as_bytes(), *sp).unwrap();
            d.cmp_frames = true;
            for lim in [0u32, 1, 2, 3, 10, 100, 1000, 10_000, 10_000_000, u32::MAX] {
                d.set_limits(Some(lim), None, None);
                d.set_note(format!("match_limit={}", lim));
                for s in &subs {
                    for so in 0..=s.len() {
                        d.go(s, so, 0);
                    }
                }
            }
            total += d.count;
        }
    }
    note("C134", total);
}

// =============================================================================
// C135 - pcre2_set_depth_limit / (*LIMIT_DEPTH=) / (*LIMIT_RECURSION=)
// =============================================================================
#[test]
fn c135_depth_limit() {
    println!("C135 pcre2_set_depth_limit and (*LIMIT_DEPTH=n)");
    let mut total = 0u64;
    unsafe {
        let nested: String = "(?:a)".repeat(100);
        let deep_group: String = format!("{}b{}", "(".repeat(80), ")".repeat(80));
        let pats: Vec<(String, Spec)> = vec![
            (nested.clone(), Spec::z()),
            (deep_group.clone(), Spec::z()),
            ("\\((?:[^()]|(?R))*\\)".into(), Spec::z()),
            ("(a)(?1)".into(), Spec::z()),
            ("a".into(), Spec::z()),
            ("(a+)+b".into(), Spec::z()),
            ("(*LIMIT_DEPTH=5)(a+)+b".into(), Spec::z()),
            ("(*LIMIT_DEPTH=50)\\((?:[^()]|(?R))*\\)".into(), Spec::z()),
            ("(*LIMIT_RECURSION=7)(a+)+b".into(), Spec::z()),
            ("(*LIMIT_RECURSION=200)(a)(?1)".into(), Spec::z()),
        ];
        let subs: Vec<Vec<u8>> = vec![
            b"".to_vec(),
            b"a".to_vec(),
            b"aa".to_vec(),
            b"b".to_vec(),
            b"aaaaaaaaaa".to_vec(),
            b"aaaaaaaaaab".to_vec(),
            b"((((x))))".to_vec(),
            b"((((((((((x))))))))))".to_vec(),
            b"(((((((((((((((x)))))))))))))))".to_vec(),
            "(".repeat(30).into_bytes(),
        ];
        for (p, sp) in &pats {
            let mut d = Duo::new("C135", p.as_bytes(), *sp).unwrap();
            d.cmp_frames = true;
            let mut lim = 0u32;
            while lim <= 220 {
                d.set_limits(None, Some(lim), None);
                d.set_note(format!("depth_limit={}", lim));
                for s in &subs {
                    d.go(s, 0, 0);
                }
                lim += 1;
            }
            for lim in [500u32, 1000, 10_000, 10_000_000, u32::MAX] {
                d.set_limits(None, Some(lim), None);
                d.set_note(format!("depth_limit={}", lim));
                for s in &subs {
                    d.go(s, 0, 0);
                }
            }
            total += d.count;
        }

        // pcre2_set_recursion_limit is a synonym for the depth limit
        {
            let mut d = Duo::new("C135", b"\\((?:[^()]|(?R))*\\)", Spec::z()).unwrap();
            d.cmp_frames = true;
            for lim in 0..=120u32 {
                assert_eq!((d.c.pcre2_set_recursion_limit_8)(d.mc_c, lim), 0);
                assert_eq!((d.r.pcre2_set_recursion_limit_8)(d.mc_r, lim), 0);
                d.set_note(format!("recursion_limit={}", lim));
                d.go(b"((((((x))))))", 0, 0);
                d.go(b"(((((x))))", 0, 0);
            }
            total += d.count;
        }
    }
    note("C135", total);
}

// =============================================================================
// C136 - pcre2_set_heap_limit / (*LIMIT_HEAP=)
// =============================================================================
#[test]
fn c136_heap_limit() {
    println!("C136 pcre2_set_heap_limit and (*LIMIT_HEAP=n)");
    let mut total = 0u64;
    unsafe {
        let many = format!("{}a", "()".repeat(5000)); // huge frame -> first frame may not fit
        let pats: Vec<(String, Spec)> = vec![
            ("(a+)*b".into(), Spec::z()),
            ("a".into(), Spec::z()),
            ("(a)(b)(c)".into(), Spec::z()),
            (many.clone(), Spec::z()),
            ("\\((?:[^()]|(?R))*\\)".into(), Spec::z()),
            ("(*LIMIT_HEAP=0)(a+)*b".into(), Spec::z()),
            ("(*LIMIT_HEAP=1)(a+)*b".into(), Spec::z()),
            ("(*LIMIT_HEAP=20)(a+)*b".into(), Spec::z()),
            ("(*LIMIT_HEAP=1024)(a+)*b".into(), Spec::z()),
        ];
        let subs: Vec<Vec<u8>> = vec![
            b"".to_vec(),
            b"a".to_vec(),
            b"abc".to_vec(),
            b"aaaaaaaaaaaaaaaaaaaa".to_vec(),
            b"aaaaaaaaaaaaaaaaaaab".to_vec(),
            b"((((((x))))))".to_vec(),
        ];
        for (p, sp) in &pats {
            let mut d = Duo::new("C136", p.as_bytes(), *sp).unwrap();
            d.cmp_frames = true;
            for lim in [
                0u32, 1, 2, 5, 10, 19, 20, 21, 40, 79, 80, 81, 100, 500, 1024, 20480, 20_000_000,
                u32::MAX,
            ] {
                d.set_limits(None, None, Some(lim));
                d.set_note(format!("heap_limit={}", lim));
                for s in &subs {
                    d.go(s, 0, 0);
                }
            }
            total += d.count;
        }

        // contiguous heap-limit sweep so the exact HEAPLIMIT threshold must coincide
        {
            let mut d = Duo::new("C136", b"(a+)*b", Spec::z()).unwrap();
            d.cmp_frames = true;
            for lim in 0..=300u32 {
                d.set_limits(None, None, Some(lim));
                d.set_note(format!("heap_limit={}", lim));
                for s in ["", "a", "aaaaaaaaaa", "aaaaaaaaaaaaaaaaaaaa", "aaaaaaaaaab"] {
                    d.go(s.as_bytes(), 0, 0);
                }
            }
            total += d.count;
        }

        // A match_data reused after a match that already grew heapframes: the
        // realloc path must only fire when the existing block is smaller.
        {
            let mut d = Duo::new("C136", b"(a+)*b", Spec::z()).unwrap();
            d.cmp_frames = true;
            d.set_limits(None, None, Some(u32::MAX));
            d.set_note("grow then shrink".into());
            d.go(b"aaaaaaaaaaaaaaaaaaaa", 0, 0);
            d.go(b"a", 0, 0);
            for lim in [1u32, 20, 100, 20480, u32::MAX] {
                d.set_limits(None, None, Some(lim));
                d.set_note(format!("reuse heap_limit={}", lim));
                d.go(b"aaaaaaaaaaaa", 0, 0);
                d.go(b"a", 0, 0);
            }
            total += d.count;
        }

        // Combination of all three limits at once.
        {
            let mut d = Duo::new("C136", b"(a+)+(b+)+c", Spec::z()).unwrap();
            d.cmp_frames = true;
            for m in [1u32, 10, 100, 1000] {
                for dl in [1u32, 5, 50, 500] {
                    for h in [0u32, 1, 20, 1024] {
                        d.set_limits(Some(m), Some(dl), Some(h));
                        d.set_note(format!("m={} d={} h={}", m, dl, h));
                        d.go(b"aaaabbbb", 0, 0);
                        d.go(b"aaaabbbbc", 0, 0);
                    }
                }
            }
            total += d.count;
        }
    }
    note("C136", total);
}

// =============================================================================
// C137 - PCRE2_DISABLE_RECURSELOOP_CHECK
// =============================================================================
#[test]
fn c137_recurse_loop_check() {
    println!("C137 PCRE2_DISABLE_RECURSELOOP_CHECK");
    let mut total = 0u64;
    unsafe {
        let pats: &[&str] = &[
            "(?1)()",
            "(a(?2))((?1))",
            "\\((?:[^()]|(?R))*\\)",
            "(?R)|a",
            "()(?1)*",
            "(?:(?1)|a)()",
            "(a*)(?1)",
        ];
        let subs: &[&[u8]] = &[
            b"",
            b"a",
            b"aa",
            b"()",
            b"(a)",
            b"((a))",
            b"(((",
            b"(()",
            b"x",
        ];
        for p in pats {
            let mut d = Duo::new("C137", p.as_bytes(), Spec::z()).unwrap();
            d.cmp_frames = true;
            for (m, dl, h) in [
                (200u32, 100u32, 20u32),
                (1000, 1000, 100),
                (50, 1000, 1024),
                (10_000, 200, 20480),
                (2000, 2000, 1),
            ] {
                d.set_limits(Some(m), Some(dl), Some(h));
                for mo in [0, PCRE2_DISABLE_RECURSELOOP_CHECK] {
                    d.set_note(format!("m={} d={} h={} mo={:#x}", m, dl, h, mo));
                    for s in subs {
                        for so in 0..=s.len() {
                            d.go(s, so, mo);
                        }
                    }
                }
            }
            total += d.count;
        }
    }
    note("C137", total);
}

// =============================================================================
// C138 - PCRE2_COPY_MATCHED_SUBJECT
// =============================================================================
#[test]
fn c138_copy_matched_subject() {
    println!("C138 PCRE2_COPY_MATCHED_SUBJECT");
    let (c, r) = (&both().0, &both().1);
    let mut total = 0u64;
    unsafe {
        let pats: &[(&str, Spec)] = &[
            ("a(b)(c)?", Spec::z()),
            ("a*", Spec::z()),
            ("", Spec::z()),
            ("x", Spec::z()),
            ("(a)(b)(c)", Spec::z()),
            ("abcd", Spec::z()),
            ("(?<n>a)b", Spec::z()),
        ];
        let subs: &[&[u8]] = &[b"", b"abc", b"ab", b"abcd", b"zzz", b"a", b"abcabc"];
        for (p, sp) in pats {
            let mut d = Duo::new("C138", p.as_bytes(), *sp).unwrap();
            for s in subs {
                for mo in [0u32, PCRE2_COPY_MATCHED_SUBJECT] {
                    for extra in [0u32, PCRE2_PARTIAL_SOFT, PCRE2_NOTEMPTY] {
                        for so in 0..=s.len() {
                            // fresh mutable copy of the subject for each run
                            let mut buf = s.to_vec();
                            buf.push(0); // avoid a zero-length allocation
                            let o = d.raw(buf.as_ptr(), s.len(), so, mo | extra, false);
                            // Trash the ORIGINAL buffer, then re-read the substrings.
                            for b in buf.iter_mut() {
                                *b = 0x5a;
                            }
                            let mut got_c: Vec<Option<Vec<u8>>> = vec![];
                            let mut got_r: Vec<Option<Vec<u8>>> = vec![];
                            let ngroups = if o.rc > 0 { o.rc as u32 } else { 1 };
                            for g in 0..ngroups {
                                got_c.push(substr(c, d.md_c, g));
                                got_r.push(substr(r, d.md_r, g));
                            }
                            assert_eq!(
                                got_c, got_r,
                                "[C138] substring bytes differ after the original \
                                 subject buffer was overwritten: pat={} subj={} so={} mo={:#x}",
                                show(p.as_bytes()),
                                show(s),
                                so,
                                mo | extra
                            );
                            if mo & PCRE2_COPY_MATCHED_SUBJECT != 0 && o.rc > 0 && !s.is_empty() {
                                // the copy must still hold the ORIGINAL bytes
                                let want = &s[o.ovector[0]..o.ovector[1]];
                                assert_eq!(
                                    got_c[0].as_deref(),
                                    Some(want),
                                    "[C138] C copy corrupted"
                                );
                                assert_eq!(
                                    got_r[0].as_deref(),
                                    Some(want),
                                    "[C138] Rust copy corrupted"
                                );
                            }
                        }
                    }
                }
                // reuse the same match data for a second match after a successful
                // COPY_MATCHED_SUBJECT match (the previous copy must be freed)
                let mut buf = s.to_vec();
                buf.push(0);
                d.raw(buf.as_ptr(), s.len(), 0, PCRE2_COPY_MATCHED_SUBJECT, false);
                d.raw(buf.as_ptr(), s.len(), 0, PCRE2_COPY_MATCHED_SUBJECT, false);
                d.raw(buf.as_ptr(), s.len(), 0, 0, false);
                d.raw(ptr::null(), 0, 0, PCRE2_COPY_MATCHED_SUBJECT, false);
            }
            total += d.count;
        }
    }
    note("C138", total);
}

unsafe fn substr(api: &Api, md: Ptr, n: u32) -> Option<Vec<u8>> {
    let mut p: *mut u8 = ptr::null_mut();
    let mut l: usize = 0;
    let rc = (api.pcre2_substring_get_bynumber_8)(md, n, &mut p, &mut l);
    if rc != 0 {
        return None;
    }
    let v = std::slice::from_raw_parts(p, l).to_vec();
    (api.pcre2_substring_free_8)(p);
    Some(v)
}

// =============================================================================
// C139 - PCRE2_NO_JIT is accepted and has no effect
// =============================================================================
#[test]
fn c139_no_jit() {
    println!("C139 PCRE2_NO_JIT accepted with no effect");
    let mut total = 0u64;
    unsafe {
        let pats: &[(&str, Spec)] = &[
            ("a", Spec::z()),
            ("(a)(b)?", Spec::z()),
            ("^a.*b$", Spec::c(PCRE2_MULTILINE)),
            ("\\X", Spec::c(PCRE2_UTF)),
            ("(?R)|a", Spec::z()),
        ];
        let subs = subjects(
            0x139_0001,
            &[b"", b"a", b"ab", b"a\nb", b"\xc3\xa9"],
            &[K_LETTERS, K_UTF8],
            36,
            40,
        );
        for (p, sp) in pats {
            let mut d = Duo::new("C139", p.as_bytes(), *sp).unwrap();
            for s in &subs {
                for so in 0..=s.len() {
                    let a = d.go(s, so, 0);
                    let b = d.go(s, so, PCRE2_NO_JIT);
                    assert_eq!(a, b, "C139 PCRE2_NO_JIT changed the result");
                    assert_ne!(a.rc, PCRE2_ERROR_BADOPTION);
                    d.go(s, so, PCRE2_NO_JIT | PCRE2_ANCHORED);
                    d.go(s, so, PCRE2_NO_JIT | PCRE2_PARTIAL_HARD);
                }
            }
            total += d.count;
        }
        // jit_compile / jit_match stubs must behave identically too
        let d = Duo::new("C139", b"abc", Spec::z()).unwrap();
        assert_eq!(
            (d.c.pcre2_jit_compile_8)(d.code_c, PCRE2_JIT_COMPLETE),
            (d.r.pcre2_jit_compile_8)(d.code_r, PCRE2_JIT_COMPLETE)
        );
        let s = b"xabcx";
        let rc_c = (d.c.pcre2_jit_match_8)(d.code_c, s.as_ptr(), 5, 0, 0, d.md_c, d.mc_c);
        let rc_r = (d.r.pcre2_jit_match_8)(d.code_r, s.as_ptr(), 5, 0, 0, d.md_r, d.mc_r);
        assert_eq!(rc_c, rc_r, "C139 pcre2_jit_match stub");
        total += d.count;
    }
    note("C139", total);
}

// =============================================================================
// C140 - PCRE2_UTF / PCRE2_NO_UTF_CHECK subject validation
// =============================================================================
#[test]
fn c140_utf_check() {
    println!("C140 PCRE2_UTF / PCRE2_NO_UTF_CHECK subject validation");
    let mut pats: Vec<(&str, Spec)> = vec![];
    for p in [
        "a",
        ".",
        "\\w+",
        "(?<=abc)def",
        "(?<=.{1,3})x",
        "\\X",
        "[\\x{100}-\\x{200}]",
        "[\\x{80}-\\xff]",
        "(?i)é",
        "def",
    ] {
        for co in [0u32, PCRE2_UTF, PCRE2_UTF | PCRE2_UCP] {
            pats.push((p, Spec::c(co)));
        }
    }
    let subs = subjects(
        0x140_0001,
        &[
            b"",
            b"abcdef",
            b"\xc3\xa9",
            b"abc\x80def",
            b"\x80abcdef",
            b"\xff",
            b"\xc2",
            b"\xe0\x80\x80",
            b"\xed\xa0\x80",
            b"\xf5\x80\x80\x80",
            b"\xf4\x90\x80\x80",
            b"a\xc3",
            b"\xe4\xb8\x80def",
        ],
        &[K_BADUTF, K_UTF8, K_HIGH, K_RANDOM],
        60,
        40,
    );
    let mopts: &[u32] = &[
        0,
        PCRE2_NO_UTF_CHECK,
        PCRE2_PARTIAL_SOFT,
        PCRE2_NO_UTF_CHECK | PCRE2_PARTIAL_HARD,
        PCRE2_ANCHORED,
    ];
    let total = unsafe { drive("C140", &pats, &subs, mopts, 1) };
    note("C140", total);
}

// =============================================================================
// C141 - PCRE2_MATCH_INVALID_UTF fragment machinery
// =============================================================================
#[test]
fn c141_match_invalid_utf() {
    println!("C141 PCRE2_MATCH_INVALID_UTF");
    let mut pats: Vec<(&str, Spec)> = vec![];
    for p in [
        "def",
        "(?<=abc)def",
        "^x",
        "x$",
        "x\\z",
        "x\\Z",
        ".",
        "\\X",
        "a+",
        "\\bx\\b",
        "(?<=x{1,3})y",
        "\\w+",
        "^",
        "$",
    ] {
        for co in [
            PCRE2_MATCH_INVALID_UTF,
            PCRE2_MATCH_INVALID_UTF | PCRE2_UTF,
            PCRE2_MATCH_INVALID_UTF | PCRE2_UTF | PCRE2_MULTILINE,
            PCRE2_UTF,
        ] {
            pats.push((p, Spec::c(co)));
        }
    }
    let subs = subjects(
        0x141_0001,
        &[
            b"abc\x80def",
            b"\x80abcdef",
            b"\xffx\xff",
            b"x\xff",
            b"abcdef\xc3",
            b"\xff\xff\xff\xff",
            b"\x80",
            b"",
            b"abcxdef",
            b"a\xffbc\xffdef",
            b"\xc3\xa9\xffx",
            b"xy\xffxy",
        ],
        &[K_BADUTF, K_UTF8],
        72,
        40,
    );
    let mopts: &[u32] = &[
        0,
        PCRE2_NO_UTF_CHECK,
        PCRE2_PARTIAL_SOFT,
        PCRE2_PARTIAL_HARD,
        PCRE2_NO_UTF_CHECK | PCRE2_PARTIAL_SOFT,
        PCRE2_NOTBOL,
        PCRE2_NOTEOL,
        PCRE2_ANCHORED,
    ];
    let total = unsafe { drive("C141", &pats, &subs, mopts, 1) };
    note("C141", total);
}

// =============================================================================
// C142 - partial matching
// =============================================================================
#[test]
fn c142_partial_matching() {
    println!("C142 PCRE2_PARTIAL_SOFT / PCRE2_PARTIAL_HARD");
    let mut pats: Vec<(&str, Spec)> = vec![];
    for p in [
        "abcd",
        "abcd|abcx",
        "ab|abcd",
        "a+",
        "abc",
        "abc\\z",
        "abc\\Z",
        "(?<=abc)def",
        "a*",
        "a$",
        ".",
        "$",
        "\\Z",
        "\\R",
        "\\R+",
        "\\R*",
        ".+",
        "\\d{2,4}",
        "[abc]{2,}",
        "(a)(b)(c)(d)",
        "\\X",
        "\\X+",
    ] {
        pats.push((p, Spec::z()));
    }
    for p in [".", "$", "\\Z", "\\R", "\\R+", "a\\R?"] {
        for nl in [
            PCRE2_NEWLINE_CR,
            PCRE2_NEWLINE_CRLF,
            PCRE2_NEWLINE_ANY,
            PCRE2_NEWLINE_ANYCRLF,
        ] {
            pats.push((p, Spec::z().nl(nl)));
        }
    }
    let subs = subjects(
        0x142_0001,
        &[
            b"", b"a", b"ab", b"abc", b"abcd", b"xyz", b"aaa", b"12", b"123", b"\r", b"\r\n",
            b"a\r", b"a\n", b"abc\r",
        ],
        &[K_LETTERS, K_NEWLINE, K_MIXED],
        48,
        40,
    );
    let mopts: &[u32] = &[
        0,
        PCRE2_PARTIAL_SOFT,
        PCRE2_PARTIAL_HARD,
        PCRE2_PARTIAL_SOFT | PCRE2_PARTIAL_HARD,
        PCRE2_PARTIAL_SOFT | PCRE2_NOTEOL,
        PCRE2_PARTIAL_HARD | PCRE2_NOTEOL,
        PCRE2_PARTIAL_SOFT | PCRE2_NOTEMPTY,
        PCRE2_PARTIAL_HARD | PCRE2_ANCHORED,
    ];
    let mut total = unsafe { drive("C142", &pats, &subs, mopts, 1) };

    // partial + ENDANCHORED must be rejected
    unsafe {
        let mut d = Duo::new("C142", b"abcd", Spec::z()).unwrap();
        for mo in [
            PCRE2_PARTIAL_SOFT | PCRE2_ENDANCHORED,
            PCRE2_PARTIAL_HARD | PCRE2_ENDANCHORED,
        ] {
            let o = d.go(b"abc", 0, mo);
            assert_eq!(o.rc, PCRE2_ERROR_BADOPTION);
        }
        // allowemptypartial via PCRE2_MATCH_EMPTY
        for p in ["a*", "(?:)", "\\b", "x*"] {
            let mut e = Duo::new("C142", p.as_bytes(), Spec::z()).unwrap();
            for s in [&b""[..], b"a", b"b"] {
                for mo in [PCRE2_PARTIAL_SOFT, PCRE2_PARTIAL_HARD] {
                    e.go(s, 0, mo);
                }
            }
            total += e.count;
        }
        total += d.count;
    }
    note("C142", total);
}

// =============================================================================
// C143 - newline conventions and \R
// =============================================================================
#[test]
fn c143_newline_conventions() {
    println!("C143 newline conventions and \\R");
    let mut pats: Vec<(&str, Spec)> = vec![];
    let ps = [
        "^a", "a$", "a\\Z", "a\\z", ".", "\\R", "\\R+", "\\n", "\\r\\n", "\\N", "a.b", "^", "$",
        "\\R{2}", "a\r", "a\n", "(?m)^a", "(?m)a$",
    ];
    for p in ps.iter() {
        for nl in [
            0,
            PCRE2_NEWLINE_CR,
            PCRE2_NEWLINE_LF,
            PCRE2_NEWLINE_CRLF,
            PCRE2_NEWLINE_ANY,
            PCRE2_NEWLINE_ANYCRLF,
            PCRE2_NEWLINE_NUL,
        ] {
            for bsr in [0, PCRE2_BSR_UNICODE, PCRE2_BSR_ANYCRLF] {
                pats.push((p, Spec::c(PCRE2_MULTILINE).nl(nl).bsr(bsr)));
                pats.push((p, Spec::z().nl(nl).bsr(bsr)));
            }
        }
    }
    // UTF variants so U+2028 / U+2029 are single characters
    for p in [".", "\\R", "^a", "a$"] {
        for nl in [PCRE2_NEWLINE_ANY, PCRE2_NEWLINE_LF] {
            pats.push((p, Spec::c(PCRE2_UTF | PCRE2_MULTILINE).nl(nl)));
        }
    }
    let subs = subjects(
        0x143_0001,
        &[
            b"",
            b"a",
            b"a\n",
            b"a\r",
            b"a\r\n",
            b"\r\na",
            b"a\x85b",
            b"a\x0bb",
            b"a\x0cb",
            b"a\0b",
            b"a\xe2\x80\xa8b",
            b"a\xe2\x80\xa9b",
            b"aa\r\nbb",
            b"\r\n\r\n",
            b"\n\r",
        ],
        &[K_NEWLINE, K_RANDOM],
        60,
        40,
    );
    let total = unsafe { drive("C143", &pats, &subs, M_PLAIN, 1) };
    note("C143", total);
}

// =============================================================================
// C144 - backreferences
// =============================================================================
#[test]
fn c144_backreferences() {
    println!("C144 backreference opcodes");
    let pats: Vec<(&str, Spec)> = vec![
        ("(a)\\1", Spec::z()),
        ("(?i)(a)\\1", Spec::z()),
        ("(a)\\1{2,3}", Spec::z()),
        ("(a)\\1*", Spec::z()),
        ("(a)\\1++", Spec::z()),
        ("(a)\\1?", Spec::z()),
        ("(a)\\1{0,2}+", Spec::z()),
        ("(a)?\\1b", Spec::z()),
        ("(a)?\\1b", Spec::c(PCRE2_MATCH_UNSET_BACKREF)),
        ("(?<n>a)\\k<n>", Spec::z()),
        ("(?J)(?<n>a)|(?<n>b)\\k<n>", Spec::z()),
        ("(?i)(?<n>a)\\k<n>", Spec::z()),
        ("(?J)(?i)(?<n>a)|(?<n>b)\\k<n>", Spec::z()),
        ("()\\1", Spec::z()),
        ("(a*)\\1", Spec::z()),
        ("(a*)\\1*", Spec::z()),
        ("(?i)(é)\\1", Spec::c(PCRE2_UTF)),
        ("(?i)(é)\\1", Spec::c(PCRE2_UTF | PCRE2_UCP)),
        (
            "(?i)(é)\\1",
            Spec::cx(PCRE2_UTF, PCRE2_EXTRA_CASELESS_RESTRICT),
        ),
        ("(?i)(i)\\1", Spec::cx(PCRE2_UTF, PCRE2_EXTRA_TURKISH_CASING)),
        ("(?i)(I)\\1", Spec::cx(PCRE2_UTF, PCRE2_EXTRA_TURKISH_CASING)),
        ("(?i)(İ)\\1", Spec::cx(PCRE2_UTF, PCRE2_EXTRA_TURKISH_CASING)),
        ("(?i)(s)\\1", Spec::c(PCRE2_UTF | PCRE2_UCP)),
        ("(?i)(k)\\1", Spec::c(PCRE2_UTF | PCRE2_UCP)),
        ("(a|b)\\1{1,4}", Spec::z()),
        ("(a)(b)\\2\\1", Spec::z()),
    ];
    let subs = subjects(
        0x144_0001,
        &[
            b"",
            b"aa",
            b"aaa",
            b"aaaa",
            b"aA",
            b"AA",
            b"ab",
            b"b",
            b"bb",
            b"abba",
            b"\xc3\xa9\xc3\xa9",
            b"\xc3\xa9\xc3\x89",
            b"\xc3\x89\xc3\xa9",
            b"ii",
            b"iI",
            b"Ii",
            b"II",
            b"\xc4\xb0i",
            b"ss",
            b"s\xc5\xbf",
            b"k\xe2\x84\xaa",
        ],
        &[K_LETTERS, K_UTF8, K_A],
        96,
        40,
    );
    let mopts: &[u32] = &[0, PCRE2_PARTIAL_SOFT, PCRE2_PARTIAL_HARD, PCRE2_ANCHORED];
    let total = unsafe { drive("C144", &pats, &subs, mopts, 1) };
    note("C144", total);
}

// =============================================================================
// C145 - recursion
// =============================================================================
#[test]
fn c145_recursion() {
    println!("C145 OP_RECURSE");
    let pats: Vec<(&str, Spec)> = vec![
        ("\\((?:[^()]++|(?R))*\\)", Spec::z()),
        ("\\((?:[^()]|(?R))*\\)", Spec::z()),
        ("(a)(?1)", Spec::z()),
        ("(?1)(?<x>a)", Spec::z()),
        ("(?&w)(?<w>a)", Spec::z()),
        ("(?R)|a", Spec::z()),
        ("(a)(?1)(?(1)b)", Spec::z()),
        ("(a)(?1)\\1", Spec::z()),
        ("(?>(a)(?1))", Spec::z()),
        ("(a(*FAIL))?(?1)|b", Spec::z()),
        ("((*ACCEPT)a)(?1)b", Spec::z()),
        ("(a|b(?1))c", Spec::z()),
        ("(?P<x>a(?P>x)?)", Spec::z()),
        ("(?:(a)|b)(?1)?", Spec::z()),
        ("(a)(?-1)", Spec::z()),
        ("(?+1)(b)", Spec::z()),
    ];
    let subs = subjects(
        0x145_0001,
        &[
            b"",
            b"a",
            b"aa",
            b"aaa",
            b"ab",
            b"b",
            b"bc",
            b"bbc",
            b"()",
            b"(a)",
            b"((a))",
            b"(((x)))",
            b"((",
            b"(()",
            b"(a(b)c)",
        ],
        &[K_PARENS, K_LETTERS],
        60,
        40,
    );
    let mut total = 0u64;
    unsafe {
        for (p, sp) in &pats {
            if let Some(mut d) = Duo::new("C145", p.as_bytes(), *sp) {
                // bound the runaway recursions
                d.set_limits(Some(20000), Some(1000), Some(2048));
                d.cmp_frames = true;
                for s in &subs {
                    for so in 0..=s.len() {
                        for mo in [0, PCRE2_PARTIAL_SOFT, PCRE2_ANCHORED] {
                            d.go(s, so, mo);
                        }
                    }
                }
                total += d.count;
            }
        }
    }
    note("C145", total);
}

// =============================================================================
// C146 - atomic groups and possessive quantifiers
// =============================================================================
#[test]
fn c146_atomic_and_possessive() {
    println!("C146 atomic groups and possessive quantifiers");
    let pats: Vec<(&str, Spec)> = vec![
        ("(?>a+)b", Spec::z()),
        ("(*atomic:a+)b", Spec::z()),
        ("a++b", Spec::z()),
        ("a*+b", Spec::z()),
        ("a?+b", Spec::z()),
        ("a{2,5}+b", Spec::z()),
        ("[a-z]++b", Spec::z()),
        ("[^a]*+b", Spec::z()),
        ("\\d?+b", Spec::z()),
        ("(a|b)++c", Spec::z()),
        ("(?:ab)*+", Spec::z()),
        ("(a)++", Spec::z()),
        ("(?>(a))", Spec::z()),
        ("(a?)++b", Spec::z()),
        ("(a*)*+b", Spec::z()),
        ("(?>a|ab)c", Spec::z()),
        ("(?>)a", Spec::z()),
        ("(a){2,4}+b", Spec::z()),
        ("(?:a|)*+b", Spec::z()),
        ("\\X++b", Spec::c(PCRE2_UTF)),
        ("[\\x{100}]*+b", Spec::c(PCRE2_UTF)),
        ("(?>a(?>b+)c)d", Spec::z()),
        ("(?i)(?>A+)b", Spec::z()),
    ];
    let subs = subjects(
        0x146_0001,
        &[
            b"",
            b"a",
            b"ab",
            b"aab",
            b"aaab",
            b"b",
            b"abc",
            b"ababc",
            b"aaaaab",
            b"1b",
            b"xb",
            b"abcd",
            b"AAb",
            b"\xc4\x80b",
        ],
        &[K_A, K_LETTERS, K_MIXED],
        96,
        40,
    );
    let mopts: &[u32] = &[0, PCRE2_PARTIAL_SOFT, PCRE2_PARTIAL_HARD, PCRE2_NOTEMPTY];
    let total = unsafe { drive("C146", &pats, &subs, mopts, 1) };
    note("C146", total);
}

// =============================================================================
// C147 - assertions at match time
// =============================================================================
#[test]
fn c147_assertions() {
    println!("C147 look-around assertions at match time");
    let pats: Vec<(&str, Spec)> = vec![
        ("(?=ab)a", Spec::z()),
        ("(?!ab)a.", Spec::z()),
        ("(?<=ab)c", Spec::z()),
        ("(?<!ab)c", Spec::z()),
        ("(?<=a{2,5})b", Spec::z()),
        ("(*pla:ab)a", Spec::z()),
        ("(*plb:ab)c", Spec::z()),
        ("(*nla:ab)a", Spec::z()),
        ("(*nlb:ab)c", Spec::z()),
        ("(?=a(*ACCEPT))", Spec::z()),
        ("(?=(a))\\1", Spec::z()),
        ("(?!a(*COMMIT))b", Spec::z()),
        ("(?=a(*THEN)b|c)", Spec::z()),
        ("(?<=^a)b", Spec::z()),
        ("(?<=\\A.)b", Spec::z()),
        ("(?<=a)", Spec::z()),
        ("(?<!)a", Spec::z()),
        ("(?=)a", Spec::z()),
        ("a(?=b)(?=.c)", Spec::z()),
        ("(?<=ab|cd)e", Spec::z()),
        ("(?<=é)x", Spec::c(PCRE2_UTF)),
        ("(?<=.)x", Spec::c(PCRE2_UTF)),
        ("(?<=.{2})x", Spec::c(PCRE2_UTF)),
        ("(?*ab)c", Spec::z()),
        ("(?<*ab)c", Spec::z()),
    ];
    let subs = subjects(
        0x147_0001,
        &[
            b"",
            b"a",
            b"ab",
            b"abc",
            b"aab",
            b"aaab",
            b"c",
            b"xc",
            b"abe",
            b"cde",
            b"\xc3\xa9x",
            b"a\xc3\xa9x",
            b"ax",
            b"b",
        ],
        &[K_LETTERS, K_UTF8, K_A],
        96,
        40,
    );
    let mopts: &[u32] = &[
        0,
        PCRE2_PARTIAL_SOFT,
        PCRE2_PARTIAL_HARD,
        PCRE2_NOTBOL,
        PCRE2_ANCHORED,
    ];
    let total = unsafe { drive("C147", &pats, &subs, mopts, 1) };
    note("C147", total);
}

// =============================================================================
// C148 - (*scs:) / (*scan_substring:)
// =============================================================================
#[test]
fn c148_scan_substring() {
    println!("C148 (*scs:) / (*scan_substring:)");
    let pats: Vec<(&str, Spec)> = vec![
        ("(a)(*scs:(1)b)", Spec::z()),
        ("(ab)(*scs:(1)b)", Spec::z()),
        ("(ab)(*scan_substring:(1)b)", Spec::z()),
        ("(?<n>ab)(*scs:(<n>)b)", Spec::z()),
        ("(a)?(*scs:(1)b)", Spec::z()),
        ("(a)?(b)(*scs:(1,2)b)", Spec::z()),
        ("(a)(b)(*scs:(2,1)a)", Spec::z()),
        ("(ab)(*scs:(1)b$)", Spec::z()),
        ("(ab)(*scs:(1)^a)", Spec::z()),
        ("(abc)(*scs:(1)(*scs:(1)c))", Spec::z()),
        ("(abc)(*scs:(1)b(*scs:(1)c))", Spec::z()),
        ("(a+)(*scs:(1)a\\z)", Spec::z()),
        ("(a+)(*scs:(1)\\Aa)", Spec::z()),
        ("(a+)x(*scs:(1)aa)", Spec::z()),
        ("(\\w+)(*scs:(1)\\bx)", Spec::z()),
    ];
    let subs = subjects(
        0x148_0001,
        &[
            b"", b"a", b"ab", b"abc", b"abcd", b"aab", b"aaa", b"aaax", b"b", b"x", b"axb",
        ],
        &[K_LETTERS, K_A, K_MIXED],
        96,
        40,
    );
    let mopts: &[u32] = &[
        0,
        PCRE2_NOTEOL,
        PCRE2_NOTBOL,
        PCRE2_PARTIAL_SOFT,
        PCRE2_ANCHORED,
    ];
    let total = unsafe { drive("C148", &pats, &subs, mopts, 1) };
    note("C148", total);
}

// =============================================================================
// C149 - \K (OP_SET_SOM)
// =============================================================================
#[test]
fn c149_backslash_k() {
    println!("C149 \\K / OP_SET_SOM");
    let pats: Vec<(&str, Spec)> = vec![
        ("a\\Kb", Spec::z()),
        ("(a\\K)b", Spec::z()),
        ("a\\Kb\\Kc", Spec::z()),
        ("\\Ka", Spec::z()),
        ("a\\K", Spec::z()),
        ("(?:a\\K|b)c", Spec::z()),
        ("a\\K(?=b)", Spec::z()),
        ("(a)\\K(b)", Spec::z()),
        ("a*\\Kb", Spec::z()),
        (
            "(?=(a\\K))(?1)",
            Spec::cx(0, PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK),
        ),
        (
            "(?<=a\\K)b",
            Spec::cx(0, PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK),
        ),
        (
            "(?=a\\K)ab",
            Spec::cx(0, PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK),
        ),
        (
            "a(?<=\\Kb)c",
            Spec::cx(0, PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK),
        ),
        (
            "(?<=ab\\K)c",
            Spec::cx(0, PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK),
        ),
        ("(?<=a\\Kb)c", Spec::cx(0, PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK)),
    ];
    let subs = subjects(
        0x149_0001,
        &[
            b"", b"a", b"ab", b"abc", b"aab", b"b", b"bc", b"xabc", b"aaab", b"abcabc",
        ],
        &[K_LETTERS, K_A],
        96,
        40,
    );
    let mopts: &[u32] = &[0, PCRE2_PARTIAL_SOFT, PCRE2_NOTEMPTY, PCRE2_ANCHORED];
    let total = unsafe { drive("C149", &pats, &subs, mopts, 1) };
    note("C149", total);
}

// =============================================================================
// C150 - backtracking control verbs
// =============================================================================
#[test]
fn c150_verbs() {
    println!("C150 backtracking control verbs");
    let pats: Vec<(&str, Spec)> = vec![
        ("a(*MARK:X)b", Spec::z()),
        ("a(*MARK:X)b|(*SKIP:X)c", Spec::z()),
        ("A(*SKIP:A)B|AC", Spec::z()),
        ("a(*SKIP)b", Spec::z()),
        ("a(*PRUNE)b", Spec::z()),
        ("a(*COMMIT)b", Spec::z()),
        ("a(*THEN)b|ac", Spec::z()),
        ("(a(*THEN)b|c)", Spec::z()),
        ("a(*ACCEPT)b", Spec::z()),
        ("(a(*ACCEPT))(?1)", Spec::z()),
        ("(*FAIL)", Spec::z()),
        ("(*F)|a", Spec::z()),
        ("(?>a(*THEN)b|ac)", Spec::z()),
        ("(?>a(*PRUNE)b)|ac", Spec::z()),
        ("(?:a(*COMMIT)b)++c", Spec::z()),
        ("(?=a(*SKIP)b)", Spec::z()),
        ("(?!a(*PRUNE)b)c", Spec::z()),
        ("(?!a(*THEN)b)", Spec::z()),
        ("\\((?:(*COMMIT)[^()]|(?R))*\\)", Spec::z()),
        ("(a(*MARK:M)b|c)(?1)", Spec::z()),
        ("(*MARK:m1)a|(*MARK:m2)b", Spec::z()),
        ("a(*:tag)b", Spec::z()),
        ("(*ACCEPT:X)", Spec::z()),
        ("x(*MARK:one)y|x(*MARK:two)z", Spec::z()),
        ("(*PRUNE:P)a", Spec::z()),
        ("(*SKIP:S)a|b", Spec::z()),
        ("(*THEN:T)a|b", Spec::z()),
        ("a(*MARK:X)(*SKIP:X)b|ac", Spec::z()),
        ("(?:(*MARK:A)a|(*MARK:B)b)(*SKIP:A)c", Spec::z()),
    ];
    let subs = subjects(
        0x150_0001,
        &[
            b"", b"a", b"ab", b"ac", b"AC", b"AB", b"abc", b"c", b"b", b"xy", b"xz", b"aab",
            b"(a)", b"((a))", b"aaab",
        ],
        &[K_LETTERS, K_A, K_PARENS],
        84,
        40,
    );
    let mopts: &[u32] = &[
        0,
        PCRE2_NOTEMPTY,
        PCRE2_PARTIAL_SOFT,
        PCRE2_ANCHORED,
        PCRE2_NOTBOL,
    ];
    let total = unsafe { drive("C150", &pats, &subs, mopts, 1) };
    note("C150", total);
}

// =============================================================================
// C151 - script runs
// =============================================================================
#[test]
fn c151_script_runs() {
    println!("C151 script runs");
    let mut pats: Vec<(&str, Spec)> = vec![];
    for p in [
        "(*sr:\\w+)",
        "(*script_run:\\w+)",
        "(*asr:\\w+)",
        "(*atomic_script_run:\\w+)",
        "(*sr:)",
        "(*sr:.)",
        "(*sr:.{0,3})",
        "(*sr:\\w*)x",
        "(*sr:[\\p{L}\\p{N}]+)",
        "(*sr:\\w+)\\z",
    ] {
        for co in [PCRE2_UTF | PCRE2_UCP, PCRE2_UCP, 0] {
            pats.push((p, Spec::c(co)));
        }
    }
    let subs = subjects(
        0x151_0001,
        &[
            b"",
            b"a",
            b"abc",
            b"abc123",
            b"\xd0\xb0\xd0\xb1",             // Cyrillic
            b"a\xd0\xb0",                    // Latin + Cyrillic (fails)
            b"\xe3\x81\x82\xe3\x82\xa2",     // Hiragana + Katakana
            b"\xe4\xb8\x80\xe3\x81\x82",     // Han + Hiragana
            b"\xd8\xa7\xd9\xa1",             // Arabic + Arabic-Indic digit
            b"\xd8\xa7\xdb\xb1",             // Arabic + Ext Arabic-Indic digit
            b"\xd8\xa7\xd9\xa1\xdb\xb1",     // mixed digit sets (fails)
            b"a1_",
            b"\xc3\xa9a",
            b"\xe1\xbf\xb6\xce\xb1",         // Greek
        ],
        &[K_WORDS, K_UTF8],
        84,
        40,
    );
    let mopts: &[u32] = &[0, PCRE2_PARTIAL_SOFT, PCRE2_ANCHORED];
    let total = unsafe { drive("C151", &pats, &subs, mopts, 1) };
    note("C151", total);
}

// =============================================================================
// C152 - \X (extended grapheme clusters)
// =============================================================================
#[test]
fn c152_extuni() {
    println!("C152 \\X / OP_EXTUNI");
    let mut pats: Vec<(&str, Spec)> = vec![];
    for p in [
        "\\X", "\\X+", "\\X{2,3}", "\\X*+", "(?:\\X)*", "\\X?", "\\X{2}", "\\X+?", "\\X*",
        "\\X\\X", "a\\X", "\\X\\z",
    ] {
        for co in [PCRE2_UTF, 0, PCRE2_UTF | PCRE2_UCP] {
            pats.push((p, Spec::c(co)));
        }
    }
    let subs = subjects(
        0x152_0001,
        &[
            b"",
            b"a",
            b"ab",
            b"\r\n",
            b"\r",
            b"\n",
            b"a\r\nb",
            b"e\xcc\x81",                         // e + combining acute
            b"e\xcc\x81\xcc\x82",                 // multiple marks
            b"\xe0\xa4\x95\xe0\xa4\xbc",          // Devanagari + nukta
            b"\xf0\x9f\x91\xa8\xe2\x80\x8d\xf0\x9f\x91\xa9", // ZWJ sequence
            b"\xe1\x84\x80\xe1\x85\xa1",          // Hangul L + V
            b"\xe1\x84\x80\xe1\x85\xa1\xe1\x86\xa8", // L V T
            b"\xf0\x9f\x87\xa6\xf0\x9f\x87\xa7",  // regional indicators
            b"a\xcc\x81",
            b"\xcc\x81",
            b"e\xcc",                             // truncated mark
            b"\xf0\x9f\x91\xa8\xe2\x80\x8d",      // ends mid-cluster
        ],
        &[K_UTF8, K_BADUTF],
        84,
        40,
    );
    let mopts: &[u32] = &[
        0,
        PCRE2_PARTIAL_SOFT,
        PCRE2_PARTIAL_HARD,
        PCRE2_NO_UTF_CHECK,
        PCRE2_ANCHORED,
    ];
    let total = unsafe { drive("C152", &pats, &subs, mopts, 1) };
    note("C152", total);
}

// =============================================================================
// C153 - character classes at match time, with every quantifier
// =============================================================================
#[test]
fn c153_classes() {
    println!("C153 class opcodes with every OP_CR* quantifier");
    let classes: &[(&str, u32, u32)] = &[
        ("[abc]", 0, 0),
        ("[^abc]", 0, 0),
        ("[\\x{100}]", PCRE2_UTF, 0),
        ("[^\\x{100}]", PCRE2_UTF, 0),
        ("[\\p{L}]", PCRE2_UTF | PCRE2_UCP, 0),
        ("[[a-z]&&[^q]]", PCRE2_ALT_EXTENDED_CLASS, 0),
        ("[\\d\\s]", 0, 0),
        ("[a-y]", 0, 0),
        ("[^\\x{100}-\\x{200}]", PCRE2_UTF, 0),
        ("[\\p{Nd}\\p{L}]", PCRE2_UTF | PCRE2_UCP, 0),
        ("[[\\p{L}]&&[^\\x{100}-\\x{ffff}]]", PCRE2_UTF | PCRE2_UCP | PCRE2_ALT_EXTENDED_CLASS, 0),
    ];
    let quants: &[&str] = &[
        "", "*", "*?", "+", "+?", "?", "??", "{2,4}", "{2,4}?", "*+", "++", "?+", "{2,4}+",
        "{0,2}", "{3}", "{1,}",
    ];
    let mut pats: Vec<(String, Spec)> = vec![];
    for (cl, co, xo) in classes {
        for q in quants {
            pats.push((format!("{}{}x", cl, q), Spec::cx(*co, *xo)));
        }
    }
    let pats_ref: Vec<(&str, Spec)> = pats.iter().map(|(a, b)| (a.as_str(), *b)).collect();
    let subs = subjects(
        0x153_0001,
        &[
            b"",
            b"a",
            b"ax",
            b"aax",
            b"aaax",
            b"aaaax",
            b"aaaaax",
            b"qx",
            b"x",
            b"1 x",
            b"\xc4\x80x",
            b"\xc4\x80\xc4\x80x",
            b"\xe4\xb8\x80x",
            b"aaa",
        ],
        &[K_LETTERS, K_UTF8, K_MIXED],
        60,
        40,
    );
    let mopts: &[u32] = &[0, PCRE2_PARTIAL_SOFT, PCRE2_PARTIAL_HARD];
    let total = unsafe { drive("C153", &pats_ref, &subs, mopts, 1) };
    note("C153", total);
}

// =============================================================================
// C154 - callouts
// =============================================================================
#[derive(Debug, PartialEq, Eq, Clone)]
struct CalloutRec {
    version: u32,
    number: u32,
    capture_top: u32,
    capture_last: u32,
    subject_len: usize,
    start_match: usize,
    current_position: usize,
    pattern_position: usize,
    next_item_length: usize,
    cstr_offset: usize,
    cstr_len: usize,
    flags: u32,
    ov: Vec<usize>,
    mark: Option<Vec<u8>>,
    cstr: Option<Vec<u8>>,
    subject_ok: bool,
    data_ok: bool,
}

thread_local! {
    static LOG_C: RefCell<Vec<CalloutRec>> = RefCell::new(Vec::new());
    static LOG_R: RefCell<Vec<CalloutRec>> = RefCell::new(Vec::new());
    static CB_RET: RefCell<i32> = RefCell::new(0);
    static CB_SUBJ: RefCell<usize> = RefCell::new(0);
    static CB_DATA: RefCell<usize> = RefCell::new(0);
    static CB_STRLEN: RefCell<unsafe extern "C" fn(Sptr) -> usize> = RefCell::new(dummy_strlen);
}

unsafe extern "C" fn dummy_strlen(_: Sptr) -> usize {
    0
}

// pcre2.h:581-602, LP64 layout
const CB_VERSION: usize = 0;
const CB_NUMBER: usize = 4;
const CB_CAPTURE_TOP: usize = 8;
const CB_CAPTURE_LAST: usize = 12;
const CB_OFFSET_VECTOR: usize = 16;
const CB_MARK: usize = 24;
const CB_SUBJECT: usize = 32;
const CB_SUBJECT_LENGTH: usize = 40;
const CB_START_MATCH: usize = 48;
const CB_CURRENT_POSITION: usize = 56;
const CB_PATTERN_POSITION: usize = 64;
const CB_NEXT_ITEM_LENGTH: usize = 72;
const CB_CSTR_OFFSET: usize = 80;
const CB_CSTR_LENGTH: usize = 88;
const CB_CSTR: usize = 96;
const CB_FLAGS: usize = 104;

unsafe fn rd_u32(p: *const u8, off: usize) -> u32 {
    (p.add(off) as *const u32).read_unaligned()
}
unsafe fn rd_sz(p: *const u8, off: usize) -> usize {
    (p.add(off) as *const usize).read_unaligned()
}
unsafe fn rd_ptr(p: *const u8, off: usize) -> *const u8 {
    (p.add(off) as *const *const u8).read_unaligned()
}

unsafe fn read_cb(block: Ptr, data: Ptr) -> CalloutRec {
    let p = block as *const u8;
    let capture_top = rd_u32(p, CB_CAPTURE_TOP);
    let ovp = rd_ptr(p, CB_OFFSET_VECTOR) as *const usize;
    let mut ov = vec![];
    if !ovp.is_null() {
        for i in 0..(2 * capture_top as usize) {
            ov.push(ovp.add(i).read_unaligned());
        }
    }
    let markp = rd_ptr(p, CB_MARK);
    let strlen = CB_STRLEN.with(|f| *f.borrow());
    let mark = if markp.is_null() {
        None
    } else {
        let n = strlen(markp);
        Some(std::slice::from_raw_parts(markp, n).to_vec())
    };
    let cstr_len = rd_sz(p, CB_CSTR_LENGTH);
    let cstrp = rd_ptr(p, CB_CSTR);
    let cstr = if cstrp.is_null() {
        None
    } else {
        Some(std::slice::from_raw_parts(cstrp, cstr_len).to_vec())
    };
    CalloutRec {
        version: rd_u32(p, CB_VERSION),
        number: rd_u32(p, CB_NUMBER),
        capture_top,
        capture_last: rd_u32(p, CB_CAPTURE_LAST),
        subject_len: rd_sz(p, CB_SUBJECT_LENGTH),
        start_match: rd_sz(p, CB_START_MATCH),
        current_position: rd_sz(p, CB_CURRENT_POSITION),
        pattern_position: rd_sz(p, CB_PATTERN_POSITION),
        next_item_length: rd_sz(p, CB_NEXT_ITEM_LENGTH),
        cstr_offset: rd_sz(p, CB_CSTR_OFFSET),
        cstr_len,
        flags: rd_u32(p, CB_FLAGS),
        ov,
        mark,
        cstr,
        subject_ok: rd_ptr(p, CB_SUBJECT) as usize == CB_SUBJ.with(|v| *v.borrow()),
        data_ok: data as usize == CB_DATA.with(|v| *v.borrow()),
    }
}

unsafe extern "C" fn cb_c(block: Ptr, data: Ptr) -> i32 {
    let rec = read_cb(block, data);
    let over = LOG_C.with(|l| {
        let mut b = l.borrow_mut();
        b.push(rec);
        b.len() > 4000
    });
    if over {
        return PCRE2_ERROR_NOMATCH;
    }
    CB_RET.with(|v| *v.borrow())
}

unsafe extern "C" fn cb_r(block: Ptr, data: Ptr) -> i32 {
    let rec = read_cb(block, data);
    let over = LOG_R.with(|l| {
        let mut b = l.borrow_mut();
        b.push(rec);
        b.len() > 4000
    });
    if over {
        return PCRE2_ERROR_NOMATCH;
    }
    CB_RET.with(|v| *v.borrow())
}

#[test]
fn c154_callouts() {
    println!("C154 pcre2_set_callout / the pcre2_callout_block");
    let (c, r) = (&both().0, &both().1);
    let mut total = 0u64;
    unsafe {
        CB_STRLEN.with(|f| *f.borrow_mut() = c._pcre2_strlen_8);
        let pats: &[(&str, Spec)] = &[
            ("a(?C1)b", Spec::z()),
            ("a(?C{str})b", Spec::z()),
            ("a(?C)b", Spec::z()),
            ("a(?C255)b", Spec::z()),
            ("(?(?C1)(?=a)b|c)", Spec::z()),
            ("(?:(?C1)a)*b", Spec::z()),
            ("(?:a(?C2))++b", Spec::z()),
            ("(?C1)a(?C2)b(?C3)", Spec::z()),
            ("(a)(?C1)(b)(?C2)", Spec::z()),
            ("(?C1)(a+)(?:x(?C2))+b", Spec::z()),
            ("a(?C1)|b(?C2)", Spec::z()),
            ("(?C{x}}y})a", Spec::z()),
            ("(?C{})a", Spec::z()),
            ("(?C\"str\")a", Spec::z()),
            ("abc", Spec::c(PCRE2_AUTO_CALLOUT)),
            ("a(b|c)*d", Spec::c(PCRE2_AUTO_CALLOUT)),
            ("(?<n>a)(?C1)\\k<n>", Spec::z()),
            ("a(*MARK:M)(?C7)b", Spec::z()),
            ("(?C1)\\((?:[^()]|(?R))*\\)", Spec::z()),
        ];
        let subs: &[&[u8]] = &[
            b"", b"a", b"ab", b"aab", b"abc", b"b", b"c", b"aaab", b"abd", b"acd", b"aa", b"(a)",
        ];
        for (p, sp) in pats {
            let mut d = match Duo::new("C154", p.as_bytes(), *sp) {
                Some(d) => d,
                None => continue,
            };
            d.set_limits(Some(20000), Some(1000), Some(2048));
            for (data_c, data_r) in [
                (ptr::null_mut::<std::ffi::c_void>(), ptr::null_mut::<std::ffi::c_void>()),
                (0x1234usize as Ptr, 0x1234usize as Ptr),
            ] {
                assert_eq!(
                    (c.pcre2_set_callout_8)(d.mc_c, Some(cb_c), data_c),
                    0
                );
                assert_eq!(
                    (r.pcre2_set_callout_8)(d.mc_r, Some(cb_r), data_r),
                    0
                );
                for ret in [0i32, 1, PCRE2_ERROR_NOMATCH, -1000, 99] {
                    CB_RET.with(|v| *v.borrow_mut() = ret);
                    CB_DATA.with(|v| *v.borrow_mut() = data_c as usize);
                    for s in subs {
                        for so in 0..=s.len() {
                            for mo in [0u32, PCRE2_PARTIAL_SOFT, PCRE2_NOTEMPTY] {
                                CB_SUBJ.with(|v| *v.borrow_mut() = subj_ptr(s) as usize);
                                LOG_C.with(|l| l.borrow_mut().clear());
                                LOG_R.with(|l| l.borrow_mut().clear());
                                let o = d.go(s, so, mo);
                                let lc = LOG_C.with(|l| l.borrow().clone());
                                let lr = LOG_R.with(|l| l.borrow().clone());
                                assert_eq!(
                                    lc.len(),
                                    lr.len(),
                                    "[C154] callout count differs: pat={} subj={} so={} \
                                     mo={:#x} ret={} rc={}\n  C   ={:#?}\n  RUST={:#?}",
                                    show(p.as_bytes()),
                                    show(s),
                                    so,
                                    mo,
                                    ret,
                                    o.rc,
                                    lc,
                                    lr
                                );
                                for (i, (a, b)) in lc.iter().zip(lr.iter()).enumerate() {
                                    assert_eq!(
                                        a, b,
                                        "[C154] callout #{} differs: pat={} subj={} so={} \
                                         mo={:#x} ret={}",
                                        i,
                                        show(p.as_bytes()),
                                        show(s),
                                        so,
                                        mo,
                                        ret
                                    );
                                    // flags must be one of the documented values
                                    assert_eq!(
                                        a.flags
                                            & !(PCRE2_CALLOUT_STARTMATCH | PCRE2_CALLOUT_BACKTRACK),
                                        0
                                    );
                                    assert!(a.subject_ok, "[C154] cb->subject wrong");
                                    assert!(a.data_ok, "[C154] callout_data wrong");
                                    assert_eq!(a.version, 2, "[C154] callout block version");
                                    // slots 0 and 1 are always unset during a callout
                                    if a.ov.len() >= 2 {
                                        assert_eq!(a.ov[0], PCRE2_UNSET);
                                        assert_eq!(a.ov[1], PCRE2_UNSET);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // uninstall and check the no-callout path once more
            assert_eq!((c.pcre2_set_callout_8)(d.mc_c, None, ptr::null_mut()), 0);
            assert_eq!((r.pcre2_set_callout_8)(d.mc_r, None, ptr::null_mut()), 0);
            for s in subs {
                d.go(s, 0, 0);
            }
            total += d.count;
        }
    }
    note("C154", total);
}

// =============================================================================
// C155 - anchors and word boundaries
// =============================================================================
#[test]
fn c155_anchors_and_boundaries() {
    println!("C155 anchor and boundary opcodes");
    let mut pats: Vec<(&str, Spec)> = vec![
        ("^a", Spec::z()),
        ("\\Aa", Spec::z()),
        ("a$", Spec::z()),
        ("a\\z", Spec::z()),
        ("a\\Z", Spec::z()),
        ("^a", Spec::c(PCRE2_MULTILINE)),
        ("a$", Spec::c(PCRE2_MULTILINE)),
        ("\\Ga", Spec::z()),
        ("a\\Kb", Spec::z()),
        ("^$", Spec::z()),
        ("\\A\\z", Spec::z()),
        ("^", Spec::c(PCRE2_MULTILINE)),
        ("$", Spec::c(PCRE2_MULTILINE)),
        ("a$", Spec::c(PCRE2_DOLLAR_ENDONLY)),
        ("^a", Spec::c(PCRE2_MULTILINE | PCRE2_ALT_CIRCUMFLEX)),
    ];
    for p in ["\\ba", "a\\b", "\\Ba", "a\\B", "\\b", "\\B", "\\b.\\b", "\\B.\\B"] {
        pats.push((p, Spec::z()));
        pats.push((p, Spec::c(PCRE2_UCP)));
        pats.push((p, Spec::c(PCRE2_UTF | PCRE2_UCP)));
        pats.push((p, Spec::cx(PCRE2_UCP, PCRE2_EXTRA_ASCII_BSW)));
        pats.push((
            p,
            Spec::cx(PCRE2_UTF | PCRE2_UCP, PCRE2_EXTRA_ASCII_BSW),
        ));
    }
    let subs = subjects(
        0x155_0001,
        &[
            b"",
            b"a",
            b"ab",
            b" a ",
            b"_a",
            b"a\n",
            b"\na",
            b"a\nb",
            b"a\r\n",
            b"\xc3\xa9a",
            b"a\xc3\xa9",
            b"\xd0\xb0a",
            b"1a",
            b"a1",
            b"-a-",
            b"\xe4\xb8\x80a",
        ],
        &[K_WORDS, K_NEWLINE, K_UTF8],
        72,
        40,
    );
    let mopts: &[u32] = &[
        0,
        PCRE2_NOTBOL,
        PCRE2_NOTEOL,
        PCRE2_NOTBOL | PCRE2_NOTEOL,
        PCRE2_ANCHORED,
    ];
    let total = unsafe { drive("C155", &pats, &subs, mopts, 1) };
    note("C155", total);
}

// =============================================================================
// C156 - '.' , \C , \N
// =============================================================================
#[test]
fn c156_any_and_anybyte() {
    println!("C156 OP_ANY / OP_ALLANY / OP_ANYBYTE");
    let mut pats: Vec<(&str, Spec)> = vec![];
    for p in [".", ".+", ".*?", "\\C", "\\C+", "\\N", ".{2}", "\\C{2}", "\\N+", ".*", "a.b", "\\C\\C"]
    {
        for co in [0u32, PCRE2_DOTALL, PCRE2_UTF, PCRE2_UTF | PCRE2_DOTALL] {
            for nl in [
                0,
                PCRE2_NEWLINE_CR,
                PCRE2_NEWLINE_LF,
                PCRE2_NEWLINE_CRLF,
                PCRE2_NEWLINE_ANY,
                PCRE2_NEWLINE_ANYCRLF,
                PCRE2_NEWLINE_NUL,
            ] {
                pats.push((p, Spec::c(co).nl(nl)));
            }
        }
    }
    let subs = subjects(
        0x156_0001,
        &[
            b"",
            b"a",
            b"a\nb",
            b"a\r",
            b"a\r\n",
            b"\n",
            b"a\0b",
            b"\xc3\xa9",
            b"a\xc3\xa9b",
            b"\xe4\xb8\x80",
            b"\xf0\x9f\x98\x80",
            b"a\x85b",
            b"a\xe2\x80\xa8b",
        ],
        &[K_NEWLINE, K_UTF8],
        48,
        40,
    );
    let mopts: &[u32] = &[0, PCRE2_PARTIAL_SOFT, PCRE2_PARTIAL_HARD, PCRE2_NO_UTF_CHECK];
    let total = unsafe { drive("C156", &pats, &subs, mopts, 1) };
    note("C156", total);
}

// =============================================================================
// C157 - ovector sizes, get_ovector_count/pointer, get_startchar, get_mark
// =============================================================================
#[test]
fn c157_ovector_sizes() {
    println!("C157 match data ovector sizes and the accessors");
    let (c, r) = (&both().0, &both().1);
    let mut total = 0u64;
    unsafe {
        let mut shapes: Vec<(String, usize)> = vec![
            ("abc".into(), 0),
            ("(a)bc".into(), 1),
            ("(a)(b)c".into(), 2),
            ("(a)(b)?(c)".into(), 3),
            ("a\\Kbc".into(), 0),
            ("(a)\\K(b)".into(), 2),
            ("a(*MARK:X)bc".into(), 0),
            ("a(*MARK:X)bX|zz".into(), 0),
            ("(a)(?:(x))?(b)".into(), 3),
        ];
        for n in [64usize, 65, 100] {
            let p: String = (0..n).map(|_| "(a)").collect::<Vec<_>>().join("");
            shapes.push((p, n));
        }
        let subs: Vec<Vec<u8>> = vec![
            b"".to_vec(),
            b"abc".to_vec(),
            b"ab".to_vec(),
            b"ac".to_vec(),
            b"xabc".to_vec(),
            b"zz".to_vec(),
            vec![b'a'; 64],
            vec![b'a'; 100],
            vec![b'a'; 120],
        ];
        for (p, ngroups) in &shapes {
            let mut counts: Vec<u32> = vec![1, 2, 3, *ngroups as u32 + 1, *ngroups as u32 + 5, 0];
            counts.dedup();
            for oc in counts {
                let sp = Spec::z().ovec(oc);
                let mut d = Duo::new("C157", p.as_bytes(), sp).unwrap();
                // check the accessors agree on the shape
                assert_eq!(
                    (c.pcre2_get_ovector_count_8)(d.md_c),
                    (r.pcre2_get_ovector_count_8)(d.md_r)
                );
                for s in &subs {
                    for so in 0..=s.len() {
                        for mo in [0u32, PCRE2_PARTIAL_SOFT, PCRE2_NOTEMPTY, PCRE2_ANCHORED] {
                            let o = d.go(s, so, mo);
                            if o.rc > 0 {
                                // rc == 0 exactly when the ovector was too small
                                let need = o.rc as usize;
                                assert!(need <= (c.pcre2_get_ovector_count_8)(d.md_c) as usize);
                            }
                        }
                    }
                }
                total += d.count;
            }
        }
    }
    note("C157", total);
}
