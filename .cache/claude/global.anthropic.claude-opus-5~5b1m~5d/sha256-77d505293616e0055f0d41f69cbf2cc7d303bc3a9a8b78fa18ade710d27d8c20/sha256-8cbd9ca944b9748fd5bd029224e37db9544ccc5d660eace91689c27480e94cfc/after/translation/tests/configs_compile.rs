//! Differential tests for CONFIGS.md rows **C078 - C125**: the whole
//! `pcre2_compile_8` configuration surface, plus `pcre2_code_copy_8`,
//! `pcre2_code_copy_with_tables_8`, `pcre2_match_data_create_8` and
//! `pcre2_match_data_create_from_pattern_8`.
//!
//! Every case drives the SAME pattern / options / compile-context through the C `.so`
//! and the Rust `.so` and compares `Api::compile_probe` output, which contains the
//! errorcode, the erroroffset, every scalar `pcre2_pattern_info` key, the name table,
//! the start bitmap and the byte-exact serialized image of the compiled pattern.
//!
//! Each row test also runs a fixed-seed RANDOM PATTERN GENERATOR (see `fuzz`) that
//! builds patterns out of a token pool covering literals (incl. bytes >= 0x80),
//! escapes, classes, every group form, quantifiers, back references, recursion,
//! conditionals, verbs, callouts and inline option settings, and compiles each of
//! them under a random selection of compile options, extra options, newline
//! convention and BSR setting — with the bits owned by that row forced on and off.
//! Patterns that fail to compile are compared too (errorcode AND erroroffset).
//!
//! Each row's body runs in a CHILD copy of this test binary (see `isolate!`), because a
//! Rust panic inside the cdylib aborts the process and would otherwise hide every other
//! row; that is why the output contains one `test result:` line per row plus the overall
//! one. `PCRE2_TRACE=1` makes every probe print its pattern first, which is how a
//! crashing input is bisected.
//!
//! Set `FUZZ_N` in the environment to change the per-row random-pattern count
//! (default 200000, which is ~40s for the whole file on one thread and ~2s across
//! 48 cores; `FUZZ_N=4000` reproduces the minimum the brief asks for).

mod common;
use common::*;

use std::collections::BTreeMap;
use std::ptr;

// ---------------------------------------------------------------- C constants
// pcre2.h:495-507 — pcre2_set_optimize() directives
const PCRE2_OPTIMIZATION_NONE: u32 = 0;
const PCRE2_OPTIMIZATION_FULL: u32 = 1;
const PCRE2_AUTO_POSSESS_D: u32 = 64;
const PCRE2_AUTO_POSSESS_OFF: u32 = 65;
const PCRE2_DOTSTAR_ANCHOR_D: u32 = 66;
const PCRE2_DOTSTAR_ANCHOR_OFF: u32 = 67;
const PCRE2_START_OPTIMIZE_D: u32 = 68;
const PCRE2_START_OPTIMIZE_OFF: u32 = 69;

/// MAX_NAME_SIZE (config.h) — a name of 128 units is legal, 129 is ERR48.
const MAX_NAME_SIZE: usize = 128;
/// MAX_MARK in the 8-bit build (pcre2_intmodedep.h:233)
const MAX_MARK: usize = 255;
/// PARSED_PATTERN_DEFAULT_SIZE (pcre2_compile.c) — parsed pattern goes on the heap above this
const PARSED_PATTERN_DEFAULT_SIZE: usize = 1024;

// ================================================================= reporting

fn info_name(k: u32) -> &'static str {
    match k {
        PCRE2_INFO_ALLOPTIONS => "ALLOPTIONS",
        PCRE2_INFO_ARGOPTIONS => "ARGOPTIONS",
        PCRE2_INFO_BACKREFMAX => "BACKREFMAX",
        PCRE2_INFO_BSR => "BSR",
        PCRE2_INFO_CAPTURECOUNT => "CAPTURECOUNT",
        PCRE2_INFO_FIRSTCODEUNIT => "FIRSTCODEUNIT",
        PCRE2_INFO_FIRSTCODETYPE => "FIRSTCODETYPE",
        PCRE2_INFO_HASCRORLF => "HASCRORLF",
        PCRE2_INFO_JCHANGED => "JCHANGED",
        PCRE2_INFO_JITSIZE => "JITSIZE",
        PCRE2_INFO_LASTCODEUNIT => "LASTCODEUNIT",
        PCRE2_INFO_LASTCODETYPE => "LASTCODETYPE",
        PCRE2_INFO_MATCHEMPTY => "MATCHEMPTY",
        PCRE2_INFO_MATCHLIMIT => "MATCHLIMIT",
        PCRE2_INFO_MAXLOOKBEHIND => "MAXLOOKBEHIND",
        PCRE2_INFO_MINLENGTH => "MINLENGTH",
        PCRE2_INFO_NAMECOUNT => "NAMECOUNT",
        PCRE2_INFO_NAMEENTRYSIZE => "NAMEENTRYSIZE",
        PCRE2_INFO_NEWLINE => "NEWLINE",
        PCRE2_INFO_DEPTHLIMIT => "DEPTHLIMIT",
        PCRE2_INFO_SIZE => "SIZE",
        PCRE2_INFO_HASBACKSLASHC => "HASBACKSLASHC",
        PCRE2_INFO_FRAMESIZE => "FRAMESIZE",
        PCRE2_INFO_HEAPLIMIT => "HEAPLIMIT",
        PCRE2_INFO_EXTRAOPTIONS => "EXTRAOPTIONS",
        _ => "?",
    }
}

fn esc(b: &[u8]) -> String {
    let mut s = String::new();
    let show = if b.len() > 200 { &b[..200] } else { b };
    for &c in show {
        match c {
            b'\\' => s.push_str("\\\\"),
            b'"' => s.push_str("\\\""),
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7e => s.push(c as char),
            _ => s.push_str(&format!("\\x{:02x}", c)),
        }
    }
    if b.len() > 200 {
        s.push_str(&format!("...  (total len {})", b.len()));
    }
    s
}

/// Find WHICH field of two `CompileOut`s differs. Returns (kind, detail).
fn describe(a: &CompileOut, b: &CompileOut) -> (String, String) {
    if a.ok != b.ok {
        return (
            "compile-success".to_string(),
            format!(
                "C ok={} (errorcode={} erroroffset={}) vs RUST ok={} (errorcode={} erroroffset={})",
                a.ok, a.errorcode, a.erroroffset, b.ok, b.errorcode, b.erroroffset
            ),
        );
    }
    if a.errorcode != b.errorcode {
        return (
            "errorcode".to_string(),
            format!(
                "errorcode C={} RUST={} (erroroffset C={} RUST={})",
                a.errorcode, b.errorcode, a.erroroffset, b.erroroffset
            ),
        );
    }
    if a.erroroffset != b.erroroffset {
        return (
            "erroroffset".to_string(),
            format!(
                "erroroffset C={} RUST={} (errorcode {})",
                a.erroroffset, b.erroroffset, a.errorcode
            ),
        );
    }
    let n = a.info.len().min(b.info.len());
    for i in 0..n {
        let (k, rc, v) = a.info[i];
        let (k2, rc2, v2) = b.info[i];
        if k != k2 || rc != rc2 || v != v2 {
            return (
                format!("info:{}", info_name(k)),
                format!(
                    "pattern_info {} ({}): C rc={} value={} (0x{:x}) ; RUST rc={} value={} (0x{:x})",
                    k,
                    info_name(k),
                    rc,
                    v,
                    v,
                    rc2,
                    v2,
                    v2
                ),
            );
        }
    }
    if a.info.len() != b.info.len() {
        return ("info-count".to_string(), "info vector lengths differ".to_string());
    }
    if a.image.len() != b.image.len() {
        return (
            "image-length".to_string(),
            format!(
                "compiled/serialized image length C={} RUST={}",
                a.image.len(),
                b.image.len()
            ),
        );
    }
    for i in 0..a.image.len() {
        if a.image[i] != b.image[i] {
            let lo = i.saturating_sub(12);
            let hi = (i + 13).min(a.image.len());
            return (
                "image-bytes".to_string(),
                format!(
                    "first differing image byte at {} of {}: C=0x{:02x} RUST=0x{:02x}\n      C   [{}..{}] = {:02x?}\n      RUST[{}..{}] = {:02x?}",
                    i,
                    a.image.len(),
                    a.image[i],
                    b.image[i],
                    lo,
                    hi,
                    &a.image[lo..hi],
                    lo,
                    hi,
                    &b.image[lo..hi]
                ),
            );
        }
    }
    ("unknown".to_string(), "CompileOut != but every field matched".to_string())
}

/// A Rust-side `panic!` inside the cdylib unwinds out of an `extern "C"` function and
/// therefore ABORTS the whole process, which would take every other row's test down
/// with it. So every row's body runs in a child copy of this test binary (selected by
/// the row id, which is the prefix of the test function name): a crashing row fails
/// loudly, with its stdout/stderr forwarded, without hiding the other rows.
/// No check is relaxed — the child runs exactly the same assertions.
fn is_isolated_child(row: &str) -> bool {
    std::env::var("PCRE2_ISOLATE").as_deref() == Ok(row)
}

fn run_isolated_child(row: &str) {
    let filter = format!("{}_", row.to_lowercase());
    let exe = std::env::current_exe().expect("current_exe");
    let out = std::process::Command::new(exe)
        .args(["--nocapture", "--test-threads=1", &filter])
        .env("PCRE2_ISOLATE", row)
        .output()
        .expect("spawn child test process");
    print!("{}", String::from_utf8_lossy(&out.stdout));
    eprint!("{}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        stdout.contains("1 passed"),
        "{}: the child process (filter {:?}) did not report a passing test: {:?}\n\
         a signal status here means the library ABORTED (Rust panic / SIGSEGV) on one \
         of this row's inputs — see the child output above",
        row,
        filter,
        out.status
    );
    assert!(out.status.success(), "{}: child process failed: {:?}", row, out.status);
}

macro_rules! isolate {
    ($row:literal) => {
        if !is_isolated_child($row) {
            run_isolated_child($row);
            return;
        }
    };
}

fn trace_on() -> bool {
    std::env::var("PCRE2_TRACE").is_ok()
}

/// PCRE2 documents that passing a pattern that is not valid UTF together with
/// PCRE2_UTF **and** PCRE2_NO_UTF_CHECK is UNDEFINED BEHAVIOUR, and the C library
/// really does die on it: `pcre2_compile_8(b"\xff\xfe", 2, PCRE2_LITERAL|PCRE2_UTF|
/// PCRE2_NO_UTF_CHECK, ..)` SEGFAULTS in `c_src/build/libpcre2.so` (verified
/// separately) while the Rust build panics in `compile_branch.rs:2696`. There is no
/// ground truth to compare against for such inputs, so — and only for them — the
/// NO_UTF_CHECK bit is dropped, which keeps the invalid-UTF patterns in the suite with
/// the validity check enabled. Every other option combination is passed through as-is.
fn utf_safe_opts(pat: &[u8], plen: usize, opts: u32) -> u32 {
    if opts & PCRE2_NO_UTF_CHECK == 0 || opts & (PCRE2_UTF | PCRE2_MATCH_INVALID_UTF) == 0 {
        return opts;
    }
    let eff: &[u8] = if plen == PCRE2_ZERO_TERMINATED {
        let n = pat.iter().position(|&b| b == 0).unwrap_or(pat.len());
        &pat[..n]
    } else {
        &pat[..plen.min(pat.len())]
    };
    if std::str::from_utf8(eff).is_ok() {
        opts
    } else {
        opts & !PCRE2_NO_UTF_CHECK
    }
}

/// Divergence accumulator for one row: keeps counts + the first example per kind so
/// one run reports EVERY distinct divergence rather than stopping at the first.
struct D {
    row: &'static str,
    cases: usize,
    compiled: usize,
    failed: usize,
    errs: BTreeMap<i32, usize>,
    trace: bool,
    diffs: BTreeMap<String, (usize, String)>,
}

impl D {
    fn new(row: &'static str) -> D {
        println!("[{}] running", row);
        D {
            row,
            cases: 0,
            compiled: 0,
            failed: 0,
            errs: BTreeMap::new(),
            trace: trace_on(),
            diffs: BTreeMap::new(),
        }
    }

    fn record(&mut self, kind: String, detail: String) {
        let e = self.diffs.entry(kind).or_insert((0, String::new()));
        e.0 += 1;
        if e.1.is_empty() {
            e.1 = detail;
        }
    }

    fn add(&mut self, label: &str, pat: &[u8], plen: usize, opts: u32, a: &CompileOut, b: &CompileOut) {
        self.cases += 1;
        if a.ok {
            self.compiled += 1;
        } else {
            self.failed += 1;
            *self.errs.entry(a.errorcode).or_insert(0) += 1;
        }
        if a != b {
            let (kind, detail) = describe(a, b);
            self.record(
                kind,
                format!(
                    "{}\n      pattern = \"{}\"\n      plen = {:#x} options = 0x{:08x}\n      {}",
                    label,
                    esc(pat),
                    plen,
                    opts,
                    detail
                ),
            );
        }
    }

    /// Compile `pat` in both libraries and compare everything observable.
    unsafe fn probe(
        &mut self,
        label: &str,
        pat: &[u8],
        plen: usize,
        opts: u32,
        ccc: Ptr,
        rcc: Ptr,
    ) -> (CompileOut, CompileOut) {
        let (c, r) = both();
        let opts = utf_safe_opts(pat, plen, opts);
        if self.trace {
            eprintln!(
                "TRACE {} {} pattern=\"{}\" plen={:#x} opts=0x{:08x}",
                self.row,
                label,
                esc(pat),
                plen,
                opts
            );
        }
        let a = c.compile_probe(pat, plen, opts, ccc);
        let b = r.compile_probe(pat, plen, opts, rcc);
        self.add(label, pat, plen, opts, &a, &b);
        (a, b)
    }

    /// Same, with an explicit (possibly NULL) pattern pointer.
    unsafe fn probe_ptr(
        &mut self,
        label: &str,
        pat: Sptr,
        plen: usize,
        opts: u32,
        ccc: Ptr,
        rcc: Ptr,
    ) {
        let (c, r) = both();
        let a = raw_probe(c, pat, plen, opts, ccc);
        let b = raw_probe(r, pat, plen, opts, rcc);
        self.add(label, b"<raw ptr>", plen, opts, &a, &b);
    }

    fn finish(self) {
        let mut top: Vec<(i32, usize)> = self.errs.iter().map(|(k, v)| (*k, *v)).collect();
        top.sort_by_key(|(_, v)| std::cmp::Reverse(*v));
        top.truncate(10);
        println!(
            "[{}] {} comparison(s) ({} compiled OK, {} rejected by both); {} distinct compile \
             error codes, most frequent: {:?}",
            self.row,
            self.cases,
            self.compiled,
            self.failed,
            self.errs.len(),
            top
        );
        assert!(self.cases > 0, "{}: no cases ran", self.row);
        if !self.diffs.is_empty() {
            let mut msg = format!(
                "{}: {} DIVERGENCE KIND(S) over {} comparisons\n",
                self.row,
                self.diffs.len(),
                self.cases
            );
            for (kind, (count, example)) in &self.diffs {
                msg.push_str(&format!("\n  [{}] x{}\n      {}\n", kind, count, example));
            }
            panic!("{}", msg);
        }
    }
}

/// `Api::compile_probe` for a raw (possibly NULL) pattern pointer.
/// Collects exactly the same fields as the harness helper.
unsafe fn raw_probe(api: &Api, pat: Sptr, plen: usize, opts: u32, cc: Ptr) -> CompileOut {
    let mut ec: i32 = 0;
    let mut eo: usize = 0;
    let code = (api.pcre2_compile_8)(pat, plen, opts, &mut ec, &mut eo, cc);
    if code.is_null() {
        return CompileOut { ok: false, errorcode: ec, erroroffset: eo, info: vec![], image: vec![] };
    }
    let out = snapshot(api, code, ec, eo);
    (api.pcre2_code_free_8)(code);
    out
}

/// Everything observable about an already-compiled code block (does NOT free it).
unsafe fn snapshot(api: &Api, code: Ptr, ec: i32, eo: usize) -> CompileOut {
    let mut info = vec![];
    for &(k, w) in INFO_SCALARS {
        let mut buf = [0u8; 8];
        let rc = (api.pcre2_pattern_info_8)(code, k, buf.as_mut_ptr() as Ptr);
        let v = if w == 4 {
            u32::from_ne_bytes([buf[0], buf[1], buf[2], buf[3]]) as u64
        } else {
            u64::from_ne_bytes(buf)
        };
        info.push((k, rc, if rc == 0 { v } else { 0 }));
    }
    let mut ncount: u32 = 0;
    let mut nsize: u32 = 0;
    (api.pcre2_pattern_info_8)(code, PCRE2_INFO_NAMECOUNT, &mut ncount as *mut u32 as Ptr);
    (api.pcre2_pattern_info_8)(code, PCRE2_INFO_NAMEENTRYSIZE, &mut nsize as *mut u32 as Ptr);
    let mut ntab: *const u8 = ptr::null();
    (api.pcre2_pattern_info_8)(code, PCRE2_INFO_NAMETABLE, &mut ntab as *mut *const u8 as Ptr);
    let mut image = vec![];
    if !ntab.is_null() {
        image.extend_from_slice(std::slice::from_raw_parts(ntab, (ncount * nsize) as usize));
    }
    let mut bm: *const u8 = ptr::null();
    if (api.pcre2_pattern_info_8)(code, PCRE2_INFO_FIRSTBITMAP, &mut bm as *mut *const u8 as Ptr)
        == 0
        && !bm.is_null()
    {
        image.extend_from_slice(std::slice::from_raw_parts(bm, 32));
    }
    let mut sbytes: *mut u8 = ptr::null_mut();
    let mut ssize: usize = 0;
    let codes = [code];
    let rc = (api.pcre2_serialize_encode_8)(
        codes.as_ptr() as *const Ptr,
        1,
        &mut sbytes,
        &mut ssize,
        ptr::null_mut(),
    );
    if rc == 1 && !sbytes.is_null() {
        image.extend_from_slice(std::slice::from_raw_parts(sbytes, ssize));
        (api.pcre2_serialize_free_8)(sbytes);
    } else {
        image.extend_from_slice(b"SERIALIZE_FAIL");
        image.extend_from_slice(&rc.to_ne_bytes());
    }
    CompileOut { ok: true, errorcode: ec, erroroffset: eo, info, image }
}

// ============================================================ compile contexts

/// A pair of compile contexts, one per library, kept in lock-step.
struct Cc {
    c: Ptr,
    r: Ptr,
}

impl Cc {
    unsafe fn new() -> Cc {
        let (c, r) = both();
        let cc = Cc {
            c: (c.pcre2_compile_context_create_8)(ptr::null_mut()),
            r: (r.pcre2_compile_context_create_8)(ptr::null_mut()),
        };
        assert!(!cc.c.is_null() && !cc.r.is_null(), "compile_context_create returned NULL");
        cc
    }
    unsafe fn extra(&self, v: u32) {
        let (c, r) = both();
        assert_eq!(
            (c.pcre2_set_compile_extra_options_8)(self.c, v),
            (r.pcre2_set_compile_extra_options_8)(self.r, v),
            "set_compile_extra_options(0x{:x}) rc differs",
            v
        );
    }
    unsafe fn newline(&self, v: u32) {
        let (c, r) = both();
        assert_eq!(
            (c.pcre2_set_newline_8)(self.c, v),
            (r.pcre2_set_newline_8)(self.r, v),
            "set_newline({}) rc differs",
            v
        );
    }
    unsafe fn bsr(&self, v: u32) {
        let (c, r) = both();
        assert_eq!(
            (c.pcre2_set_bsr_8)(self.c, v),
            (r.pcre2_set_bsr_8)(self.r, v),
            "set_bsr({}) rc differs",
            v
        );
    }
    unsafe fn max_pattern_length(&self, v: usize) {
        let (c, r) = both();
        assert_eq!(
            (c.pcre2_set_max_pattern_length_8)(self.c, v),
            (r.pcre2_set_max_pattern_length_8)(self.r, v)
        );
    }
    unsafe fn max_pattern_compiled_length(&self, v: usize) {
        let (c, r) = both();
        assert_eq!(
            (c.pcre2_set_max_pattern_compiled_length_8)(self.c, v),
            (r.pcre2_set_max_pattern_compiled_length_8)(self.r, v)
        );
    }
    unsafe fn max_varlookbehind(&self, v: u32) {
        let (c, r) = both();
        assert_eq!(
            (c.pcre2_set_max_varlookbehind_8)(self.c, v),
            (r.pcre2_set_max_varlookbehind_8)(self.r, v)
        );
    }
    unsafe fn parens_nest_limit(&self, v: u32) {
        let (c, r) = both();
        assert_eq!(
            (c.pcre2_set_parens_nest_limit_8)(self.c, v),
            (r.pcre2_set_parens_nest_limit_8)(self.r, v)
        );
    }
    unsafe fn optimize(&self, v: u32) {
        let (c, r) = both();
        assert_eq!(
            (c.pcre2_set_optimize_8)(self.c, v),
            (r.pcre2_set_optimize_8)(self.r, v),
            "set_optimize({}) rc differs",
            v
        );
    }
    unsafe fn free(self) {
        let (c, r) = both();
        (c.pcre2_compile_context_free_8)(self.c);
        (r.pcre2_compile_context_free_8)(self.r);
    }
}

// ========================================================= random pattern gen

const T_LIT: &[&[u8]] = &[
    b"a", b"b", b"Z", b"q", b"0", b"7", b"-", b"_", b"~", b" ", b"\t", b"\n", b"\r", b"#", b"=",
    b"%", b"@", b"/", b"<", b">", b"&", b"!", b",", b";", b":", b"\x00", b"\x1f", b"\x7f", b"\x80",
    b"\xa9", b"\xbf", b"\xc3\xa9", b"\xc4\xb0", b"\xc4\xb1", b"\xe2\x84\xaa", b"\xe2\x82\xac",
    b"\xf0\x9f\x98\x80", b"\xff", b"\xfe", b"\xed\xa0\x80",
];

const T_ESC: &[&[u8]] = &[
    b"\\d", b"\\w", b"\\s", b"\\D", b"\\W", b"\\S", b"\\h", b"\\v", b"\\H", b"\\V", b"\\R", b"\\X",
    b"\\N", b"\\b", b"\\B", b"\\A", b"\\Z", b"\\z", b"\\G", b"\\K", b"\\Qa*b\\E", b"\\Q\\E",
    b"\\p{L}", b"\\p{Greek}", b"\\P{Nd}", b"\\pL", b"\\x{41}", b"\\x{1F600}", b"\\x{D800}",
    b"\\x41", b"\\x", b"\\o{101}", b"\\o{}", b"\\N{U+0041}", b"\\c@", b"\\cA", b"\\c", b"\\e",
    b"\\a", b"\\f", b"\\v0", b"\\777", b"\\0", b"\\00", b"\\8", b"\\9", b"\\u0041", b"\\u{41}",
    b"\\U", b"\\.", b"\\\\", b".", b"^", b"$", b"\\C",
];

const T_CLASS: &[&[u8]] = &[
    b"[a-z]", b"[^\\x00-\\x1f]", b"[[:alpha:]]", b"[[:^digit:]]", b"[\\p{L}\\d]", b"[]", b"[^]",
    b"[]]", b"[abc]", b"[^a-c0-9]", b"[\\x{100}-\\x{200}]", b"[\\Qa-z\\E]", b"[a-z--q]",
    b"[[a-z]&&[^q]]", b"[[a-z]~~[a-c]]", b"[\\d\\s\\w]", b"[^\\x{100}]", b"[\\x00-\\x{10FFFF}]",
    b"[a\\-z]", b"[.-/]", b"[[.a.]]", b"[[=a=]]",
];

const T_QUANT: &[&[u8]] = &[
    b"*", b"+", b"?", b"*?", b"+?", b"??", b"*+", b"++", b"?+", b"{0,3}", b"{2,}", b"{3}",
    b"{0,0}", b"{1,4}", b"{2,1}", b"{,3}", b"{}", b"{65536}",
];

/// `{1,65535}` is generated separately and rarely: applied to a GROUP the compiler
/// replicates the group until MAX_PATTERN_SIZE is hit, which is slow.
const T_QUANT_BIG: &[&[u8]] = &[b"{1,65535}", b"{65535}", b"{65534,65535}"];

const T_BACKREF: &[&[u8]] =
    &[b"\\1", b"\\2", b"\\g1", b"\\g{-1}", b"\\g{1}", b"\\g{+1}", b"\\k<n>", b"\\k'n'", b"\\k{n}", b"(?P=n)", b"\\g{n}"];

const T_RECURSE: &[&[u8]] =
    &[b"(?R)", b"(?1)", b"(?2)", b"(?&n)", b"(?P>n)", b"(?+1)", b"(?-1)", b"(?0)"];

const T_COND: &[&[u8]] = &[
    b"(?(1)a|b)", b"(?(<n>)a)", b"(?('n')a)", b"(?(n)a)", b"(?(R)a)", b"(?(R1)a)", b"(?(R&n)a)",
    b"(?(DEFINE)(?<x>a))", b"(?(VERSION>=10.0)a|b)", b"(?(VERSION<10.0)a|b)", b"(?(?=a)b|c)",
    b"(?(?<=a)b|c)", b"(?(?!a)b|c)",
];

const T_VERB: &[&[u8]] = &[
    b"(*MARK:m)", b"(*:m)", b"(*SKIP)", b"(*SKIP:s)", b"(*THEN)", b"(*THEN:t)", b"(*PRUNE)",
    b"(*PRUNE:p)", b"(*COMMIT)", b"(*COMMIT:c)", b"(*ACCEPT)", b"(*FAIL)", b"(*F)",
];

const T_CALLOUT: &[&[u8]] = &[
    b"(?C)", b"(?C1)", b"(?C255)", b"(?C\"str\")", b"(?C{x})", b"(?C'y')", b"(?C`z`)", b"(?C%p%)",
    b"(?C#h#)", b"(?C$d$)", b"(?C^c^)", b"(?C{})", b"(?C{{}})",
];

const T_INLINE: &[&[u8]] = &[
    b"(?i)", b"(?-i)", b"(?x)", b"(?s)", b"(?m)", b"(?J)", b"(?U)", b"(?n)", b"(?aT)", b"(?aD)",
    b"(?aS)", b"(?aW)", b"(?aP)", b"(?a)", b"(?xx)", b"(?^i)", b"(?^)", b"(?imnsxrUJ)", b"(?-imsx)",
    b"(?#c)", b"(?# unterminated? no )",
];

const T_OPEN: &[&[u8]] = &[
    b"(", b"(?:", b"(?|", b"(?<n>", b"(?'m'", b"(?P<p>", b"(?>", b"(?=", b"(?!", b"(?<=", b"(?<!",
    b"(*atomic:", b"(*script_run:", b"(*sr:", b"(*asr:", b"(*scs:(1)", b"(*pla:", b"(*plb:",
    b"(*nla:", b"(*nlb:", b"(*napla:", b"(*naplb:", b"(?i:", b"(?-i:", b"(?x:", b"(?*", b"(?<*",
    b"(?(?=a)",
];

/// Every defined public compile option bit (PUBLIC_COMPILE_OPTIONS, pcre2_compile.c:693)
/// except PCRE2_LITERAL, which is exercised on its own in C080.
const RAND_OPTS: &[u32] = &[
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
    PCRE2_MATCH_INVALID_UTF,
    PCRE2_ALT_EXTENDED_CLASS,
];

/// Every defined extra option bit (PUBLIC_COMPILE_EXTRA_OPTIONS, pcre2_compile.c:706).
const RAND_XOPTS: &[u32] = &[
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

fn gen_item(rng: &mut Rng, out: &mut Vec<u8>, depth: u32) {
    if out.len() > 400 {
        out.extend_from_slice(b"a");
        return;
    }
    let k = rng.below(100);
    match k {
        0..=27 => out.extend_from_slice(rng.pick(T_LIT)),
        28..=47 => out.extend_from_slice(rng.pick(T_ESC)),
        48..=57 => out.extend_from_slice(rng.pick(T_CLASS)),
        58..=61 => out.extend_from_slice(rng.pick(T_BACKREF)),
        62..=64 => out.extend_from_slice(rng.pick(T_RECURSE)),
        65..=67 => out.extend_from_slice(rng.pick(T_COND)),
        68..=70 => out.extend_from_slice(rng.pick(T_VERB)),
        71..=73 => out.extend_from_slice(rng.pick(T_CALLOUT)),
        74..=77 => out.extend_from_slice(rng.pick(T_INLINE)),
        _ => {
            if depth == 0 {
                out.extend_from_slice(rng.pick(T_LIT));
            } else {
                out.extend_from_slice(rng.pick(T_OPEN));
                gen_seq(rng, out, depth - 1);
                // 1 in 40 groups is left unterminated on purpose (ERR14 shapes)
                if rng.below(40) != 0 {
                    out.push(b')');
                }
            }
        }
    }
    let q = rng.below(100);
    if q < 25 {
        out.extend_from_slice(rng.pick(T_QUANT));
    } else if q < 26 {
        out.extend_from_slice(rng.pick(T_QUANT_BIG));
    }
}

fn gen_seq(rng: &mut Rng, out: &mut Vec<u8>, depth: u32) {
    let nalt = 1 + rng.below(4);
    for a in 0..nalt {
        if a > 0 {
            out.push(b'|');
        }
        let n = rng.below(5);
        for _ in 0..n {
            gen_item(rng, out, depth);
        }
    }
}

fn gen_pattern(rng: &mut Rng) -> Vec<u8> {
    let mut v = Vec::with_capacity(64);
    let depth = rng.below(6);
    gen_seq(rng, &mut v, depth);
    v
}

fn subset(rng: &mut Rng, pool: &[u32], one_in: u32) -> u32 {
    let mut o = 0u32;
    for &b in pool {
        if rng.below(one_in) == 0 {
            o |= b;
        }
    }
    o
}

fn nfuzz() -> usize {
    std::env::var("FUZZ_N").ok().and_then(|s| s.parse().ok()).unwrap_or(200_000)
}

/// The random-pattern engine used by every option-family row.
///
/// `force` / `forcex` name the option / extra-option bits owned by the row: each one
/// is randomly forced ON or OFF for every generated pattern so the row's branches are
/// hit from both sides, while all the other option bits, the newline convention and
/// the BSR setting float randomly.
unsafe fn fuzz(d: &mut D, cc: &Cc, seed: u64, n: usize, force: &[u32], forcex: &[u32]) {
    let mut rng = Rng::new(seed);
    for i in 0..n {
        let pat = gen_pattern(&mut rng);
        let mut o = subset(&mut rng, RAND_OPTS, 10);
        for &b in force {
            if rng.below(2) == 0 {
                o |= b;
            } else {
                o &= !b;
            }
        }
        let mut x = subset(&mut rng, RAND_XOPTS, 12);
        for &b in forcex {
            if rng.below(2) == 0 {
                x |= b;
            } else {
                x &= !b;
            }
        }
        cc.extra(x);
        cc.newline(1 + rng.below(6));
        cc.bsr(1 + rng.below(2));
        let label = format!("fuzz #{} extra=0x{:08x}", i, x);
        if rng.below(8) == 0 {
            // zero-terminated form (patterns may contain embedded NULs, which both
            // libraries must truncate at identically)
            let mut z = pat.clone();
            z.push(0);
            d.probe(&label, &z, PCRE2_ZERO_TERMINATED, o, cc.c, cc.r);
        } else {
            d.probe(&label, &pat, pat.len(), o, cc.c, cc.r);
        }
    }
    cc.extra(0);
    cc.newline(PCRE2_NEWLINE_LF);
    cc.bsr(PCRE2_BSR_UNICODE);
}

/// Cross product: every extra-option value x every option value x every pattern.
unsafe fn matrix(d: &mut D, cc: &Cc, pats: &[&[u8]], opts: &[u32], extras: &[u32]) {
    for &x in extras {
        cc.extra(x);
        for &o in opts {
            for p in pats {
                d.probe(&format!("extra=0x{:08x}", x), p, p.len(), o, cc.c, cc.r);
            }
        }
    }
    cc.extra(0);
}

// ============================================================== C078

/// C078 — the NULL / length argument checks and the PCRE2_ZERO_TERMINATED strlen
/// conversion, all of which happen before any parsing (pcre2_compile.c:10339-10402).
#[test]
fn c078_null_and_length_arguments() {
    let (c, r) = both();
    isolate!("C078");
    let mut d = D::new("C078");
    unsafe {
        // harness sanity check: the probe really does capture the compiled image and
        // every scalar info key, so an equal comparison is not vacuous
        {
            let (c, _) = both();
            let a = c.compile_probe(b"abc", 3, 0, ptr::null_mut());
            assert!(a.ok, "C078: \"abc\" must compile");
            assert_eq!(a.info.len(), INFO_SCALARS.len());
            assert!(
                a.image.len() > 1100,
                "C078: serialized image is only {} bytes - compile_probe is not capturing the compiled pattern",
                a.image.len()
            );
            // ... and that a single differing image byte is detected and localised
            let mut b = a.clone();
            let mid = b.image.len() / 2;
            b.image[mid] ^= 0xff;
            assert_ne!(a, b);
            let (kind, detail) = describe(&a, &b);
            assert_eq!(kind, "image-bytes", "describe() missed an image difference: {}", detail);
            assert!(detail.contains(&format!("at {} of", mid)), "wrong offset reported: {}", detail);
            // and a differing info value
            let mut b2 = a.clone();
            b2.info[0].2 ^= 1;
            assert_eq!(describe(&a, &b2).0, "info:ALLOPTIONS");
        }
        // pattern == NULL with patlen 0 -> compiles the empty pattern
        d.probe_ptr("pattern=NULL patlen=0", ptr::null(), 0, 0, ptr::null_mut(), ptr::null_mut());
        // pattern == NULL with a non-zero length -> ERR16
        for &n in &[1usize, 2, 100, PCRE2_ZERO_TERMINATED] {
            d.probe_ptr(
                &format!("pattern=NULL patlen={:#x}", n),
                ptr::null(),
                n,
                0,
                ptr::null_mut(),
                ptr::null_mut(),
            );
        }
        // empty pattern, both length forms
        d.probe("empty explicit len", b"\0", 0, 0, ptr::null_mut(), ptr::null_mut());
        d.probe("empty zero-terminated", b"\0", PCRE2_ZERO_TERMINATED, 0, ptr::null_mut(), ptr::null_mut());

        // zero-terminated vs explicit length, incl. embedded NULs
        let pats: &[&[u8]] = &[
            b"abc\0",
            b"a\0b\0",
            b"\0abc\0",
            b"a(b\0)c\0",
            b"[a\0b]\0",
            b"a\\0b\0",
            b"(?<n\0>a)\0",
        ];
        for p in pats {
            d.probe("zero-terminated", p, PCRE2_ZERO_TERMINATED, 0, ptr::null_mut(), ptr::null_mut());
            for n in 0..p.len() {
                d.probe(&format!("explicit len {}", n), p, n, 0, ptr::null_mut(), ptr::null_mut());
            }
        }

        // errorptr == NULL: returns NULL, and writes 0 through erroroffset if given
        for pat in [b"a".as_ref(), b"a(".as_ref()] {
            for pass_eo in [false, true] {
                let mut eo_c: usize = 0xdead;
                let mut eo_r: usize = 0xdead;
                let pc = (c.pcre2_compile_8)(
                    pat.as_ptr(),
                    pat.len(),
                    0,
                    ptr::null_mut(),
                    if pass_eo { &mut eo_c } else { ptr::null_mut() },
                    ptr::null_mut(),
                );
                let pr = (r.pcre2_compile_8)(
                    pat.as_ptr(),
                    pat.len(),
                    0,
                    ptr::null_mut(),
                    if pass_eo { &mut eo_r } else { ptr::null_mut() },
                    ptr::null_mut(),
                );
                d.cases += 1;
                if !pc.is_null() || !pr.is_null() {
                    d.record(
                        "errorptr-NULL-returned-code".to_string(),
                        format!("pattern {:?}: C null={} RUST null={}", esc(pat), pc.is_null(), pr.is_null()),
                    );
                    if !pc.is_null() {
                        (c.pcre2_code_free_8)(pc);
                    }
                    if !pr.is_null() {
                        (r.pcre2_code_free_8)(pr);
                    }
                }
                if eo_c != eo_r {
                    d.record(
                        "errorptr-NULL-erroroffset".to_string(),
                        format!("pattern {:?}: erroroffset C={:#x} RUST={:#x}", esc(pat), eo_c, eo_r),
                    );
                }
            }
        }

        // erroroffset == NULL: ERR120
        for pat in [b"a".as_ref(), b"a(".as_ref(), b"".as_ref()] {
            let mut ec_c: i32 = -999;
            let mut ec_r: i32 = -999;
            let pc = (c.pcre2_compile_8)(
                pat.as_ptr(),
                pat.len(),
                0,
                &mut ec_c,
                ptr::null_mut(),
                ptr::null_mut(),
            );
            let pr = (r.pcre2_compile_8)(
                pat.as_ptr(),
                pat.len(),
                0,
                &mut ec_r,
                ptr::null_mut(),
                ptr::null_mut(),
            );
            d.cases += 1;
            if !pc.is_null() || !pr.is_null() || ec_c != ec_r {
                d.record(
                    "erroroffset-NULL".to_string(),
                    format!(
                        "pattern {:?}: C (null={} ec={}) RUST (null={} ec={})",
                        esc(pat),
                        pc.is_null(),
                        ec_c,
                        pr.is_null(),
                        ec_r
                    ),
                );
            }
            if !pc.is_null() {
                (c.pcre2_code_free_8)(pc);
            }
            if !pr.is_null() {
                (r.pcre2_code_free_8)(pr);
            }
        }

        // random patterns compiled at every truncation length
        let mut rng = Rng::new(0x078);
        for i in 0..(nfuzz() / 8).max(60) {
            let pat = gen_pattern(&mut rng);
            let n = rng.below(pat.len() as u32 + 1) as usize;
            d.probe(&format!("rand trunc #{}", i), &pat, n, 0, ptr::null_mut(), ptr::null_mut());
            let mut z = pat.clone();
            z.push(0);
            d.probe(&format!("rand zt #{}", i), &z, PCRE2_ZERO_TERMINATED, 0, ptr::null_mut(), ptr::null_mut());
        }
    }
    d.finish();
}

// ============================================================== C079

/// C079 — undefined public option bits and undefined extra-option bits give ERR17
/// (pcre2_compile.c:10376-10382).
#[test]
fn c079_undefined_option_bits() {
    isolate!("C079");
    let mut d = D::new("C079");
    unsafe {
        let cc = Cc::new();
        let pats: &[&[u8]] = &[b"a", b"", b"(a)(?<n>b)\\1", b"[a-z]+", b"a(", b"\\p{L}"];
        // every single option bit on its own, plus a few multi-bit combinations
        let mut opts: Vec<u32> = (0..32).map(|b| 1u32 << b).collect();
        opts.push(0);
        opts.push(0x1000_0000);
        opts.push(0x1000_0000 | PCRE2_CASELESS);
        opts.push(0xffff_ffff);
        opts.push(!(PCRE2_LITERAL));
        for &o in &opts {
            cc.extra(0);
            for p in pats {
                d.probe("opt bit", p, p.len(), o, cc.c, cc.r);
            }
        }
        // every single extra-option bit on its own
        for b in 0..32 {
            let x = 1u32 << b;
            cc.extra(x);
            for p in pats {
                d.probe(&format!("extra bit 0x{:08x}", x), p, p.len(), 0, cc.c, cc.r);
            }
        }
        for &x in &[0u32, 0xffff_ffff, 0x0002_0000, 0xffff_0000] {
            cc.extra(x);
            for p in pats {
                d.probe(&format!("extra 0x{:08x}", x), p, p.len(), 0, cc.c, cc.r);
            }
        }
        // random: random single undefined bit added to a random legal option set
        let mut rng = Rng::new(0x079);
        for i in 0..(nfuzz() / 8).max(60) {
            let pat = gen_pattern(&mut rng);
            let bad = 1u32 << rng.below(32);
            let o = subset(&mut rng, RAND_OPTS, 10) | bad;
            let x = subset(&mut rng, RAND_XOPTS, 12) | (1u32 << rng.below(32));
            cc.extra(x);
            d.probe(&format!("rand #{} bad=0x{:08x}", i, bad), &pat, pat.len(), o, cc.c, cc.r);
        }
        cc.extra(0);
        cc.free();
    }
    d.finish();
}

// ============================================================== C080

/// C080 — PCRE2_LITERAL: the allowed option subset (PUBLIC_LITERAL_COMPILE_OPTIONS,
/// ERR92) and the separate literal loop in the parser (pcre2_compile.c:3185).
#[test]
fn c080_literal() {
    isolate!("C080");
    let mut d = D::new("C080");
    unsafe {
        let cc = Cc::new();
        let pats: &[&[u8]] = &[
            b"", b"a", b"a.b", b"a*b", b"(a)", b"[a-z]", b"\\d", b"(*UTF)", b"a|b", b"^a$",
            b"a\\Kb", b"(?i)a", b"\\", b"[", b")", b"a{2,3}", b"\xc3\xa9", b"\xff\xfe",
            b"(?<n>x)\\1", b"a#b\nc", b"a b",
        ];
        // allowed companions (pcre2_compile.c:689-692)
        let allowed = [
            PCRE2_ANCHORED,
            PCRE2_AUTO_CALLOUT,
            PCRE2_CASELESS,
            PCRE2_ENDANCHORED,
            PCRE2_FIRSTLINE,
            PCRE2_MATCH_INVALID_UTF,
            PCRE2_NO_START_OPTIMIZE,
            PCRE2_NO_UTF_CHECK,
            PCRE2_USE_OFFSET_LIMIT,
            PCRE2_UTF,
        ];
        let disallowed = [
            PCRE2_MULTILINE,
            PCRE2_DOTALL,
            PCRE2_EXTENDED,
            PCRE2_EXTENDED_MORE,
            PCRE2_DUPNAMES,
            PCRE2_UNGREEDY,
            PCRE2_UCP,
            PCRE2_ALT_BSUX,
            PCRE2_ALLOW_EMPTY_CLASS,
            PCRE2_NO_AUTO_CAPTURE,
            PCRE2_ALT_EXTENDED_CLASS,
            PCRE2_NEVER_BACKSLASH_C,
            PCRE2_ALT_VERBNAMES,
            PCRE2_ALT_CIRCUMFLEX,
            PCRE2_DOLLAR_ENDONLY,
            PCRE2_MATCH_UNSET_BACKREF,
            PCRE2_NEVER_UCP,
            PCRE2_NEVER_UTF,
            PCRE2_NO_AUTO_POSSESS,
            PCRE2_NO_DOTSTAR_ANCHOR,
        ];
        let mut optsets: Vec<u32> = vec![PCRE2_LITERAL];
        for &o in allowed.iter() {
            optsets.push(PCRE2_LITERAL | o);
        }
        for &o in disallowed.iter() {
            optsets.push(PCRE2_LITERAL | o);
        }
        optsets.push(PCRE2_LITERAL | allowed.iter().fold(0, |a, b| a | b));
        // allowed / disallowed extra options (PUBLIC_LITERAL_COMPILE_EXTRA_OPTIONS 702-704)
        let extras = [
            0,
            PCRE2_EXTRA_MATCH_LINE,
            PCRE2_EXTRA_MATCH_WORD,
            PCRE2_EXTRA_CASELESS_RESTRICT,
            PCRE2_EXTRA_TURKISH_CASING,
            PCRE2_EXTRA_MATCH_LINE | PCRE2_EXTRA_MATCH_WORD,
            PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES,
            PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL,
            PCRE2_EXTRA_ESCAPED_CR_IS_LF,
            PCRE2_EXTRA_ALT_BSUX,
            PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK,
            PCRE2_EXTRA_ASCII_BSD,
            PCRE2_EXTRA_ASCII_POSIX,
            PCRE2_EXTRA_PYTHON_OCTAL,
            PCRE2_EXTRA_NO_BS0,
            PCRE2_EXTRA_NEVER_CALLOUT,
        ];
        matrix(&mut d, &cc, pats, &optsets, &extras);

        // random literal patterns (arbitrary bytes must all be taken literally)
        let mut rng = Rng::new(0x080);
        for i in 0..nfuzz() {
            let pat = gen_pattern(&mut rng);
            let mut o = PCRE2_LITERAL;
            for &b in allowed.iter() {
                if rng.below(4) == 0 {
                    o |= b;
                }
            }
            if rng.below(6) == 0 {
                o |= *rng.pick(&disallowed);
            }
            let x = *rng.pick(&extras);
            cc.extra(x);
            cc.newline(1 + rng.below(6));
            d.probe(&format!("rand literal #{} extra=0x{:x}", i, x), &pat, pat.len(), o, cc.c, cc.r);
        }
        cc.extra(0);
        cc.newline(PCRE2_NEWLINE_LF);
        cc.free();
    }
    d.finish();
}

// ============================================================== C081

/// C081 — PCRE2_CASELESS x PCRE2_UTF x PCRE2_UCP x PCRE2_EXTRA_CASELESS_RESTRICT:
/// OP_CHARI/OP_NOTI/OP_REFI selection, the other-case additions to class bitmaps,
/// ucd_caseless_sets and the 8-bit non-UTF UCP path (pcre2_compile.c:6375-6449 etc).
#[test]
fn c081_caseless() {
    isolate!("C081");
    let mut d = D::new("C081");
    unsafe {
        let cc = Cc::new();
        let pats: &[&[u8]] = &[
            b"a",
            b"A",
            b"K",
            b"\\x{212A}",
            b"s",
            b"S",
            b"\\x{17F}",
            b"\\x{130}",
            b"\\x{131}",
            b"i",
            b"I",
            b"[a-z]",
            b"[A-Z]",
            b"[\\x{100}-\\x{200}]",
            b"(a)\\1",
            b"(?<n>a)\\k<n>",
            b"ab",
            b"aB",
            b"Ab",
            b"aa",
            b"(?=a)ab",
            b"a*a",
            b"[^a]",
            b"[^\\x{212A}]",
            b"\xc3\xa9",
            b"\xc3\x89",
            b"\\x{E9}",
            b"\\x{C9}",
            b"\xff",
            b"\\x{FF}",
            b"\\x{178}",
            b"\xe2\x84\xaa",
            b"\\x{1E9E}",
            b"\\x{DF}",
            b"(?i)a",
            b"(?-i)a",
            b"(?i:a)b",
            b"\\x{345}",
            b"\\x{3A9}",
            b"[\\x{130}\\x{131}]",
        ];
        let base = [0u32, PCRE2_CASELESS];
        let uni = [0u32, PCRE2_UTF, PCRE2_UCP, PCRE2_UTF | PCRE2_UCP];
        let mut optsets = vec![];
        for &b in base.iter() {
            for &u in uni.iter() {
                optsets.push(b | u);
            }
        }
        matrix(&mut d, &cc, pats, &optsets, &[0, PCRE2_EXTRA_CASELESS_RESTRICT]);
        fuzz(
            &mut d,
            &cc,
            0x081,
            nfuzz(),
            &[PCRE2_CASELESS, PCRE2_UTF, PCRE2_UCP],
            &[PCRE2_EXTRA_CASELESS_RESTRICT],
        );
        cc.free();
    }
    d.finish();
}

// ============================================================== C082

/// C082 — PCRE2_EXTRA_TURKISH_CASING: needs UTF in the 8-bit build (ERR105), is
/// mutually exclusive with CASELESS_RESTRICT (ERR106) (pcre2_compile.c:10649-10673).
#[test]
fn c082_turkish_casing() {
    isolate!("C082");
    let mut d = D::new("C082");
    unsafe {
        let cc = Cc::new();
        let pats: &[&[u8]] = &[
            b"(?i)i",
            b"(?i)I",
            b"(?i)\\x{130}",
            b"(?i)\\x{131}",
            b"(?i)(i)\\1",
            b"(?i)[i]",
            b"(?i)[I\\x{130}]",
            b"i",
            b"(*TURKISH_CASING)(?i)i",
            b"(*TURKISH_CASING)(*UTF)(?i)i",
            b"(*UTF)(*TURKISH_CASING)(?i)\\x{131}",
            b"(*TURKISH_CASING)",
            b"(?i)\xc4\xb0",
            b"(?i)\xc4\xb1",
        ];
        let optsets = [
            0u32,
            PCRE2_UCP,
            PCRE2_UTF,
            PCRE2_UTF | PCRE2_UCP,
            PCRE2_CASELESS,
            PCRE2_CASELESS | PCRE2_UTF,
            PCRE2_CASELESS | PCRE2_UCP,
            PCRE2_CASELESS | PCRE2_UTF | PCRE2_UCP,
        ];
        let extras = [
            0,
            PCRE2_EXTRA_TURKISH_CASING,
            PCRE2_EXTRA_TURKISH_CASING | PCRE2_EXTRA_CASELESS_RESTRICT,
            PCRE2_EXTRA_CASELESS_RESTRICT,
        ];
        matrix(&mut d, &cc, pats, &optsets, &extras);
        fuzz(
            &mut d,
            &cc,
            0x082,
            nfuzz() / 2,
            &[PCRE2_UTF, PCRE2_CASELESS],
            &[PCRE2_EXTRA_TURKISH_CASING, PCRE2_EXTRA_CASELESS_RESTRICT],
        );
        cc.free();
    }
    d.finish();
}

// ============================================================== C083

/// C083 — PCRE2_UTF / PCRE2_NO_UTF_CHECK / PCRE2_NEVER_UTF and the (*UTF) / (*UTF8)
/// start-of-pattern settings against valid and invalid UTF-8 (pcre2_compile.c:10607-10632).
#[test]
fn c083_utf_validity() {
    isolate!("C083");
    let mut d = D::new("C083");
    unsafe {
        let cc = Cc::new();
        let pats: &[&[u8]] = &[
            b"abc",
            b"\xc3\xa9",
            b"\xe2\x82\xac",
            b"\xf0\x9f\x98\x80",
            b"a\xc3\xa9b",
            // invalid UTF-8 shapes
            b"\x80",
            b"\xbf",
            b"\xfe",
            b"\xff",
            b"\xc0\x80",
            b"\xc1\xbf",
            b"\xc3",
            b"\xe0\x80\x80",
            b"\xe0\xa0",
            b"\xed\xa0\x80",
            b"\xed\xbf\xbf",
            b"\xf0\x80\x80\x80",
            b"\xf4\x90\x80\x80",
            b"\xf5\x80\x80\x80",
            b"\xf8\x88\x80\x80\x80",
            b"\xfc\x84\x80\x80\x80\x80",
            b"\xc3\x28",
            b"a\xf0\x9f\x98",
            b"[\xff]",
            b"(?<\xc3\xa9>a)",
            // (*UTF) / (*UTF8) interaction
            b"(*UTF)abc",
            b"(*UTF8)abc",
            b"(*UTF)\xff",
            b"(*UTF8)\xff",
            b"(*UTF)(*UCP)\xff",
            b"(*UTF)a\xc3\xa9",
            b"(*UCP)(*UTF)\x80",
        ];
        let optsets = [
            0u32,
            PCRE2_UTF,
            PCRE2_NO_UTF_CHECK,
            PCRE2_UTF | PCRE2_NO_UTF_CHECK,
            PCRE2_NEVER_UTF,
            PCRE2_NEVER_UTF | PCRE2_UTF,
            PCRE2_MATCH_INVALID_UTF,
            PCRE2_MATCH_INVALID_UTF | PCRE2_NO_UTF_CHECK,
            PCRE2_UTF | PCRE2_UCP,
        ];
        matrix(&mut d, &cc, pats, &optsets, &[0]);
        // random byte strings, not just generated patterns: pure UTF validation paths
        let mut rng = Rng::new(0x083);
        for i in 0..nfuzz() {
            let n = rng.below(12) as usize;
            let mut pat: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
            if rng.below(2) == 0 {
                pat = gen_pattern(&mut rng);
                let k = rng.below(4);
                for _ in 0..k {
                    pat.push(rng.byte());
                }
            }
            let o = *rng.pick(&optsets) | subset(&mut rng, RAND_OPTS, 16);
            d.probe(&format!("rand bytes #{}", i), &pat, pat.len(), o, cc.c, cc.r);
        }
        fuzz(&mut d, &cc, 0x1083, nfuzz() / 2, &[PCRE2_UTF, PCRE2_NEVER_UTF, PCRE2_NO_UTF_CHECK], &[]);
        cc.free();
    }
    d.finish();
}

// ============================================================== C084

/// C084 — PCRE2_UCP / PCRE2_NEVER_UCP / (*UCP), every PCRE2_EXTRA_ASCII_* bit and the
/// (?aX) inline forms (pcre2_compile.c:2889-2931,4089-4101,5070-5101,8262-8372).
#[test]
fn c084_ucp_and_ascii_bits() {
    isolate!("C084");
    let mut d = D::new("C084");
    unsafe {
        let cc = Cc::new();
        let pats: &[&[u8]] = &[
            b"\\d", b"\\D", b"\\s", b"\\S", b"\\w", b"\\W", b"\\b", b"\\B", b"\\h", b"\\v", b"\\R",
            b"[[:alpha:]]", b"[[:digit:]]", b"[[:space:]]", b"[[:word:]]", b"[[:punct:]]",
            b"[[:^alpha:]]", b"[\\d]", b"[\\D]", b"[\\s]", b"[\\S]", b"[\\w]", b"[\\W]",
            b"[\\d\\w]", b"[^\\d]", b"\\w+", b"\\d{2,3}", b"(?aD)\\d", b"(?aS)\\s", b"(?aW)\\w",
            b"(?aP)[[:alpha:]]", b"(?aT)\\d", b"(?a)\\d\\s\\w", b"(?aD:\\d)\\d",
            b"(*UCP)\\d", b"(*UCP)[[:alpha:]]", b"(*UCP)(*UTF)\\w",
            b"\\p{L}", b"\\X", b"[[:alnum:]]",
        ];
        let optsets = [
            0u32,
            PCRE2_UCP,
            PCRE2_UTF,
            PCRE2_UCP | PCRE2_UTF,
            PCRE2_NEVER_UCP,
            PCRE2_NEVER_UCP | PCRE2_UCP,
            PCRE2_CASELESS | PCRE2_UCP,
        ];
        let each = [
            0,
            PCRE2_EXTRA_ASCII_BSD,
            PCRE2_EXTRA_ASCII_BSS,
            PCRE2_EXTRA_ASCII_BSW,
            PCRE2_EXTRA_ASCII_POSIX,
            PCRE2_EXTRA_ASCII_DIGIT,
            PCRE2_EXTRA_ASCII_BSD
                | PCRE2_EXTRA_ASCII_BSS
                | PCRE2_EXTRA_ASCII_BSW
                | PCRE2_EXTRA_ASCII_POSIX
                | PCRE2_EXTRA_ASCII_DIGIT,
            PCRE2_EXTRA_ASCII_POSIX | PCRE2_EXTRA_ASCII_DIGIT,
        ];
        matrix(&mut d, &cc, pats, &optsets, &each);
        fuzz(
            &mut d,
            &cc,
            0x084,
            nfuzz(),
            &[PCRE2_UCP, PCRE2_NEVER_UCP, PCRE2_UTF],
            &[
                PCRE2_EXTRA_ASCII_BSD,
                PCRE2_EXTRA_ASCII_BSS,
                PCRE2_EXTRA_ASCII_BSW,
                PCRE2_EXTRA_ASCII_POSIX,
                PCRE2_EXTRA_ASCII_DIGIT,
            ],
        );
        cc.free();
    }
    d.finish();
}

// ============================================================== C085

/// C085 — PCRE2_DOTALL / MULTILINE / DOLLAR_ENDONLY / ALT_CIRCUMFLEX and the inline
/// (?s) (?m) (?-s) (?-m) (?^) forms (pcre2_compile.c:6258-6281).
#[test]
fn c085_dot_and_anchors() {
    isolate!("C085");
    let mut d = D::new("C085");
    unsafe {
        let cc = Cc::new();
        let pats: &[&[u8]] = &[
            b".", b".*", b".+", b".?", b"^a", b"a$", b"^a$", b"a\\Z", b"a\\z", b"^", b"$", b"a.b",
            b"(?s).", b"(?-s).", b"(?m)^a", b"(?-m)^a", b"(?^).", b"(?^m)^a", b"(?s:.)(.)",
            b"^.*$", b"\\A.*\\z", b"[^\\n]", b".*?", b"(?s).*", b"a$\\n", b"$\\n^", b"^\\n$",
        ];
        let mut optsets = vec![];
        for i in 0..16u32 {
            let mut o = 0;
            if i & 1 != 0 {
                o |= PCRE2_DOTALL;
            }
            if i & 2 != 0 {
                o |= PCRE2_MULTILINE;
            }
            if i & 4 != 0 {
                o |= PCRE2_DOLLAR_ENDONLY;
            }
            if i & 8 != 0 {
                o |= PCRE2_ALT_CIRCUMFLEX;
            }
            optsets.push(o);
        }
        // each newline convention, since ^/$/. depend on it
        for nl in 1..=6 {
            cc.newline(nl);
            for &o in &optsets {
                for p in pats {
                    d.probe(&format!("newline={}", nl), p, p.len(), o, cc.c, cc.r);
                }
            }
        }
        cc.newline(PCRE2_NEWLINE_LF);
        fuzz(
            &mut d,
            &cc,
            0x085,
            nfuzz(),
            &[PCRE2_DOTALL, PCRE2_MULTILINE, PCRE2_DOLLAR_ENDONLY, PCRE2_ALT_CIRCUMFLEX],
            &[],
        );
        cc.free();
    }
    d.finish();
}

// ============================================================== C086

/// C086 — PCRE2_EXTENDED / PCRE2_EXTENDED_MORE and (?x) (?xx) (?-x) (?^x)
/// (pcre2_compile.c:3220-3222,3325-3335,4004,4202,5119-5139).
#[test]
fn c086_extended() {
    isolate!("C086");
    let mut d = D::new("C086");
    unsafe {
        let cc = Cc::new();
        let pats: &[&[u8]] = &[
            b"a b",
            b"a\tb",
            b"a # comment\nb",
            b"a # comment",
            b"[a b]",
            b"[a\tb]",
            b"a\\ b",
            b"(?# comment)a",
            b"a(?# unterminated",
            b"a b c",
            b"a\x0bb",
            b"a\x0cb",
            b"a\rb",
            b"a\nb",
            b"(?x)a b",
            b"(?xx)[a b]",
            b"(?x)(?xx)[a b]",
            b"(?x)(?xx)(?-xx)[a b]",
            b"(?-x)a b",
            b"(?^x)a b",
            b"(?x:a b)a b",
            b"(?xx:[a b])[a b]",
            b"a{2, 3}",
            b"[ ]",
            b"[\t]",
            b"\\x{20}",
            b"(?x)a#c\nb",
            b"(?x)\\Qa b\\E",
        ];
        let optsets = [
            0u32,
            PCRE2_EXTENDED,
            PCRE2_EXTENDED_MORE,
            PCRE2_EXTENDED | PCRE2_EXTENDED_MORE,
            PCRE2_EXTENDED | PCRE2_ALT_VERBNAMES,
        ];
        matrix(&mut d, &cc, pats, &optsets, &[0]);
        fuzz(&mut d, &cc, 0x086, nfuzz(), &[PCRE2_EXTENDED, PCRE2_EXTENDED_MORE], &[]);
        cc.free();
    }
    d.finish();
}

// ============================================================== C087

/// C087 — PCRE2_UNGREEDY and (?U) / (?-U) (pcre2_compile.c:6118,6576).
#[test]
fn c087_ungreedy() {
    isolate!("C087");
    let mut d = D::new("C087");
    unsafe {
        let cc = Cc::new();
        let pats: &[&[u8]] = &[
            b"a*", b"a*?", b"a+", b"a+?", b"a?", b"a??", b"a{2,5}", b"a{2,5}?", b"(a|b)*",
            b"[ab]*", b"a*+", b"a{2,5}+", b".*", b"(?U)a*", b"(?-U)a*", b"(?U:a*)a*", b"\\d+",
            b"(?:ab)*", b"\\X*", b"(a)\\1*", b"[\\x{100}]*", b"a{0,}", b"a{1,}?",
        ];
        matrix(&mut d, &cc, pats, &[0, PCRE2_UNGREEDY, PCRE2_UNGREEDY | PCRE2_CASELESS], &[0]);
        fuzz(&mut d, &cc, 0x087, nfuzz(), &[PCRE2_UNGREEDY], &[]);
        cc.free();
    }
    d.finish();
}

// ============================================================== C088

/// C088 — PCRE2_NO_AUTO_CAPTURE and (?n) / (?-n) (pcre2_compile.c:4733,5111).
#[test]
fn c088_no_auto_capture() {
    isolate!("C088");
    let mut d = D::new("C088");
    unsafe {
        let cc = Cc::new();
        let pats: &[&[u8]] = &[
            b"(a)", b"(a)(b)", b"(?<n>a)", b"(a)\\1", b"(?:a)", b"(a)(?1)", b"(?n)(a)\\1",
            b"(?n)(a)(?<n>b)", b"(?-n)(a)\\1", b"(?n:(a))\\1", b"(a)(?n)(b)\\1", b"(?n)(?|(a)|(b))",
            b"((a))", b"(?n)((a))", b"(a)(b)(c)\\3", b"(?n)(a)(?&n)", b"(?n)(?<n>a)(?&n)",
        ];
        matrix(&mut d, &cc, pats, &[0, PCRE2_NO_AUTO_CAPTURE, PCRE2_NO_AUTO_CAPTURE | PCRE2_DUPNAMES], &[0]);
        fuzz(&mut d, &cc, 0x088, nfuzz(), &[PCRE2_NO_AUTO_CAPTURE], &[]);
        cc.free();
    }
    d.finish();
}

// ============================================================== C089

/// C089 — PCRE2_DUPNAMES, (?J), PCRE2_INFO_JCHANGED, MAX_NAME_SIZE and MAX_NAME_COUNT
/// (pcre2_compile.c:5105,5736).
#[test]
fn c089_dupnames() {
    isolate!("C089");
    let mut d = D::new("C089");
    unsafe {
        let cc = Cc::new();
        let pats: &[&[u8]] = &[
            b"(?<n>a)(?<n>b)",
            b"(?<n>a)|(?<n>b)",
            b"(?<n>a)(?<m>b)",
            b"(?<n>a)\\k<n>",
            b"(?J)(?<n>a)(?<n>b)",
            b"(?J:(?<n>a)(?<n>b))",
            b"(?<n>a)(?<n>b)(?<n>c)",
            b"(?J)(?<n>a)(?<n>b)\\k<n>",
            b"(?-J)(?<n>a)(?<n>b)",
            b"(?J)(?<n>a)(?J-J)(?<n>b)",
            b"(?<n>a)(?'n'b)",
            b"(?P<n>a)(?P<n>b)",
            b"(?J)(?<n>a)(?<n>b)(?(<n>)x)",
        ];
        matrix(&mut d, &cc, pats, &[0, PCRE2_DUPNAMES, PCRE2_DUPNAMES | PCRE2_NO_AUTO_CAPTURE], &[0]);

        // MAX_NAME_SIZE boundary: 128 units OK, 129 -> ERR48
        for n in [1usize, 32, MAX_NAME_SIZE - 1, MAX_NAME_SIZE, MAX_NAME_SIZE + 1, MAX_NAME_SIZE + 2] {
            let name = "x".repeat(n);
            for pat in [
                format!("(?<{}>a)", name),
                format!("(?<{}>a)\\k<{}>", name, name),
                format!("(?'{}'a)", name),
                format!("(?P<{}>a)", name),
            ] {
                d.probe(&format!("name len {}", n), pat.as_bytes(), pat.len(), 0, cc.c, cc.r);
            }
        }
        // MAX_NAME_COUNT boundary (10000). NOTE: each named group also costs code units,
        // so the compiled-size limit (ERR20) may be reached first — both libraries must
        // agree on WHICH error comes out.
        for n in [9usize, 999, 9999, 10000, 10001] {
            let mut p = String::new();
            for i in 0..n {
                p.push_str(&format!("(?<n{}>)", i));
            }
            d.probe(&format!("{} distinct names", n), p.as_bytes(), p.len(), 0, cc.c, cc.r);
        }
        fuzz(&mut d, &cc, 0x089, nfuzz(), &[PCRE2_DUPNAMES], &[]);
        cc.free();
    }
    d.finish();
}

// ============================================================== C090

/// C090 — PCRE2_ALLOW_EMPTY_CLASS and PCRE2_ALT_EXTENDED_CLASS
/// (pcre2_compile.c:4215,6286,3944-3962).
#[test]
fn c090_empty_and_extended_classes() {
    isolate!("C090");
    let mut d = D::new("C090");
    unsafe {
        let cc = Cc::new();
        let mut pats: Vec<Vec<u8>> = vec![
            b"[]".to_vec(),
            b"[]]".to_vec(),
            b"[^]".to_vec(),
            b"[^]]".to_vec(),
            b"[]a]".to_vec(),
            b"[^]a]".to_vec(),
            b"[]*".to_vec(),
            b"[[a-z][0-9]]".to_vec(),
            b"[[a-z]&&[^q]]".to_vec(),
            b"[a-z--q]".to_vec(),
            b"[[a-z]~~[a-c]]".to_vec(),
            b"[[a-z]&&[b-y]&&[^q]]".to_vec(),
            b"[[a-z]".to_vec(),
            b"[[a-z]&&]".to_vec(),
            b"[&&]".to_vec(),
            b"[a&&b]".to_vec(),
            b"[\\d&&[:alpha:]]".to_vec(),
            b"(?[[a-z]-[q]])".to_vec(),
            b"[^[a-z]&&[^q]]".to_vec(),
            b"[[^a]&&[^b]]".to_vec(),
        ];
        // nesting depth beyond the ECLASS limit
        for depth in [3usize, 10, 20, 32, 33, 64, 100, 200] {
            let mut s = Vec::new();
            for _ in 0..depth {
                s.extend_from_slice(b"[");
            }
            s.extend_from_slice(b"a");
            for _ in 0..depth {
                s.extend_from_slice(b"]");
            }
            pats.push(s);
        }
        let refs: Vec<&[u8]> = pats.iter().map(|v| v.as_slice()).collect();
        matrix(
            &mut d,
            &cc,
            &refs,
            &[
                0,
                PCRE2_ALLOW_EMPTY_CLASS,
                PCRE2_ALT_EXTENDED_CLASS,
                PCRE2_ALLOW_EMPTY_CLASS | PCRE2_ALT_EXTENDED_CLASS,
                PCRE2_ALT_EXTENDED_CLASS | PCRE2_UTF,
                PCRE2_ALT_EXTENDED_CLASS | PCRE2_UCP,
                PCRE2_ALT_EXTENDED_CLASS | PCRE2_CASELESS,
            ],
            &[0],
        );
        fuzz(&mut d, &cc, 0x090, nfuzz(), &[PCRE2_ALLOW_EMPTY_CLASS, PCRE2_ALT_EXTENDED_CLASS], &[]);
        cc.free();
    }
    d.finish();
}

// ============================================================== C091

/// C091 — PCRE2_ALT_BSUX, PCRE2_EXTRA_ALT_BSUX and
/// PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES (pcre2_compile.c:1496,1639-1712,2012-2093).
#[test]
fn c091_alt_bsux() {
    isolate!("C091");
    let mut d = D::new("C091");
    unsafe {
        let cc = Cc::new();
        let pats: &[&[u8]] = &[
            b"\\u0041", b"\\u{41}", b"\\x41", b"\\x{41}", b"\\U", b"\\u", b"\\uD800", b"\\x{D800}",
            b"\\u{110000}", b"\\0", b"\\8", b"\\uDFFF", b"\\x{DFFF}", b"\\x{DC00}", b"\\u00", b"\\u004",
            b"\\uZZZZ", b"\\u{}", b"\\u{0}", b"\\u{10FFFF}", b"\\u{110000}x", b"[\\u0041]",
            b"[\\x{D800}]", b"\\x{}", b"\\x{ }", b"\\xg", b"\\x{110000}", b"\\U0041", b"\\u{D800}",
        ];
        let optsets = [0u32, PCRE2_ALT_BSUX, PCRE2_UTF, PCRE2_ALT_BSUX | PCRE2_UTF];
        let extras = [
            0,
            PCRE2_EXTRA_ALT_BSUX,
            PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES,
            PCRE2_EXTRA_ALT_BSUX | PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES,
        ];
        matrix(&mut d, &cc, pats, &optsets, &extras);
        fuzz(
            &mut d,
            &cc,
            0x091,
            nfuzz(),
            &[PCRE2_ALT_BSUX, PCRE2_UTF],
            &[PCRE2_EXTRA_ALT_BSUX, PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES],
        );
        cc.free();
    }
    d.finish();
}

// ============================================================== C092

/// C092 — PCRE2_EXTRA_PYTHON_OCTAL / NO_BS0 / BAD_ESCAPE_IS_LITERAL / ESCAPED_CR_IS_LF
/// (pcre2_compile.c:1530,1854-1962,3600,4468).
#[test]
fn c092_octal_and_bad_escapes() {
    isolate!("C092");
    let mut d = D::new("C092");
    unsafe {
        let cc = Cc::new();
        let pats: &[&[u8]] = &[
            b"\\0", b"\\00", b"\\000", b"\\0777", b"\\8", b"\\9", b"\\o{101}", b"\\o", b"\\o{}",
            b"\\o{400}", b"\\g", b"\\j", b"[\\z]", b"\\\r", b"[\\\r]", b"a\\\rb", b"\\1", b"\\12",
            b"\\123", b"\\0123", b"(a)\\1", b"(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)\\10", b"\\o{0}",
            b"\\o{777}", b"\\o{7777777777}", b"[\\0]", b"[\\8]", b"[\\o{101}]", b"\\N{U+41}",
            b"\\N{name}", b"\\k", b"\\y", b"[\\y]", b"\\l", b"\\L", b"\\E", b"\\_",
        ];
        let extras = [
            0,
            PCRE2_EXTRA_PYTHON_OCTAL,
            PCRE2_EXTRA_NO_BS0,
            PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL,
            PCRE2_EXTRA_ESCAPED_CR_IS_LF,
            PCRE2_EXTRA_PYTHON_OCTAL | PCRE2_EXTRA_NO_BS0,
            PCRE2_EXTRA_PYTHON_OCTAL | PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL,
            PCRE2_EXTRA_NO_BS0 | PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL,
            PCRE2_EXTRA_ESCAPED_CR_IS_LF | PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL,
        ];
        matrix(&mut d, &cc, pats, &[0, PCRE2_UTF, PCRE2_ALT_BSUX], &extras);
        fuzz(
            &mut d,
            &cc,
            0x092,
            nfuzz(),
            &[],
            &[
                PCRE2_EXTRA_PYTHON_OCTAL,
                PCRE2_EXTRA_NO_BS0,
                PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL,
                PCRE2_EXTRA_ESCAPED_CR_IS_LF,
            ],
        );
        cc.free();
    }
    d.finish();
}

// ============================================================== C093

/// C093 — PCRE2_NEVER_BACKSLASH_C (ERR83) and PCRE2_INFO_HASBACKSLASHC
/// (pcre2_compile.c:3664).
#[test]
fn c093_backslash_c() {
    isolate!("C093");
    let mut d = D::new("C093");
    unsafe {
        let cc = Cc::new();
        let pats: &[&[u8]] = &[
            b"\\ca", b"\\cA", b"\\c[", b"\\c]", b"\\c", b"a\\c", b"\\c\x80", b"\\c\xff",
            b"\\c\n", b"(?<=\\cA)x", b"[\\cA]", b"\\C", b"a\\cAb", b"\\c0", b"\\c?", b"\\c;",
            b"(?=\\cA)", b"(?i)\\cA", b"\\cA*", b"abc",
        ];
        matrix(
            &mut d,
            &cc,
            pats,
            &[
                0,
                PCRE2_NEVER_BACKSLASH_C,
                PCRE2_NEVER_BACKSLASH_C | PCRE2_CASELESS,
                PCRE2_UTF,
                PCRE2_NEVER_BACKSLASH_C | PCRE2_UTF,
            ],
            &[0],
        );
        fuzz(&mut d, &cc, 0x093, nfuzz(), &[PCRE2_NEVER_BACKSLASH_C], &[]);
        cc.free();
    }
    d.finish();
}

// ============================================================== C094

/// C094 — PCRE2_ALT_VERBNAMES and the MAX_MARK=255 limit
/// (pcre2_compile.c:3323-3384, pcre2_intmodedep.h:233).
#[test]
fn c094_verb_names() {
    isolate!("C094");
    let mut d = D::new("C094");
    unsafe {
        let cc = Cc::new();
        let mut pats: Vec<Vec<u8>> = vec![
            b"(*MARK:a b)".to_vec(),
            b"(*MARK:a\tb)".to_vec(),
            b"(*MARK:a\\x41b)".to_vec(),
            b"(*MARK:a\\tb)".to_vec(),
            b"(*MARK:a\\nb)".to_vec(),
            b"(*MARK:)".to_vec(),
            b"(*MARK)".to_vec(),
            b"(*:)".to_vec(),
            b"(*ACCEPT:x)".to_vec(),
            b"(*THEN:a)".to_vec(),
            b"(*SKIP:a)".to_vec(),
            b"(*PRUNE:a)".to_vec(),
            b"(*COMMIT:a)".to_vec(),
            b"(*NOSUCHVERB)".to_vec(),
            b"(*MARK:a#b\nc)".to_vec(),
            b"(*MARK:a\\Qb\\Ec)".to_vec(),
            b"(*MARK:a)b".to_vec(),
            b"(*MARK:\\)".to_vec(),
            b"(*MARK:a".to_vec(),
            b"(*F:x)".to_vec(),
            b"(*FAIL:x)".to_vec(),
        ];
        for n in [1usize, 254, MAX_MARK, MAX_MARK + 1, MAX_MARK + 2] {
            pats.push(format!("(*MARK:{})", "m".repeat(n)).into_bytes());
            pats.push(format!("(*:{})", "m".repeat(n)).into_bytes());
            pats.push(format!("(*THEN:{})", "m".repeat(n)).into_bytes());
        }
        let refs: Vec<&[u8]> = pats.iter().map(|v| v.as_slice()).collect();
        matrix(
            &mut d,
            &cc,
            &refs,
            &[
                0,
                PCRE2_ALT_VERBNAMES,
                PCRE2_EXTENDED,
                PCRE2_EXTENDED | PCRE2_ALT_VERBNAMES,
                PCRE2_EXTENDED_MORE | PCRE2_ALT_VERBNAMES,
                PCRE2_ALT_VERBNAMES | PCRE2_UTF,
            ],
            &[0],
        );
        fuzz(&mut d, &cc, 0x094, nfuzz(), &[PCRE2_ALT_VERBNAMES, PCRE2_EXTENDED], &[]);
        cc.free();
    }
    d.finish();
}

// ============================================================== C095

/// C095 — PCRE2_AUTO_CALLOUT and PCRE2_EXTRA_NEVER_CALLOUT (ERR103)
/// (pcre2_compile.c:2985-3008,3143,5288,5313,10739-10741).
#[test]
fn c095_callout_options() {
    isolate!("C095");
    let mut d = D::new("C095");
    unsafe {
        let cc = Cc::new();
        let pats: &[&[u8]] = &[
            b"", b"abc", b"a(b)c", b"a|b", b"[abc]", b"a{2,3}", b"(?C1)a", b"(?C{x})a", b"(?C)",
            b"a*b+c?", b"(?:a)(?=b)", b"(?<n>a)\\k<n>", b"\\d+", b"(?i)a", b"(*MARK:m)a",
            b"a(?C255)b", b"[\\x{100}]", b"\\p{L}+", b"(?>a)", b"(?(1)a|b)(x)",
        ];
        matrix(
            &mut d,
            &cc,
            pats,
            &[0, PCRE2_AUTO_CALLOUT, PCRE2_AUTO_CALLOUT | PCRE2_UTF, PCRE2_AUTO_CALLOUT | PCRE2_EXTENDED],
            &[0, PCRE2_EXTRA_NEVER_CALLOUT],
        );
        fuzz(&mut d, &cc, 0x095, nfuzz(), &[PCRE2_AUTO_CALLOUT], &[PCRE2_EXTRA_NEVER_CALLOUT]);
        cc.free();
    }
    d.finish();
}

// ============================================================== C096

/// C096 — PCRE2_EXTRA_MATCH_WORD / PCRE2_EXTRA_MATCH_LINE (LINE wins) and each with
/// PCRE2_LITERAL (pcre2_compile.c:3166-3175,5894-5903,10733-10736).
#[test]
fn c096_match_word_and_line() {
    isolate!("C096");
    let mut d = D::new("C096");
    unsafe {
        let cc = Cc::new();
        let pats: &[&[u8]] = &[
            b"abc", b"a|b", b"", b"^a", b"a$", b"^a$", b".*", b"(a)(b)", b"a.b", b"\\d+",
            b"(?<n>a)", b"a)b", b"[a-z]", b"^(a|b)$", b"\\bx\\b",
        ];
        let extras = [
            0,
            PCRE2_EXTRA_MATCH_WORD,
            PCRE2_EXTRA_MATCH_LINE,
            PCRE2_EXTRA_MATCH_WORD | PCRE2_EXTRA_MATCH_LINE,
        ];
        matrix(
            &mut d,
            &cc,
            pats,
            &[0, PCRE2_LITERAL, PCRE2_MULTILINE, PCRE2_ANCHORED, PCRE2_ENDANCHORED, PCRE2_UTF, PCRE2_CASELESS],
            &extras,
        );
        fuzz(&mut d, &cc, 0x096, nfuzz(), &[], &[PCRE2_EXTRA_MATCH_WORD, PCRE2_EXTRA_MATCH_LINE]);
        cc.free();
    }
    d.finish();
}

// ============================================================== C097

/// C097 — PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK (\K in a lookaround, ERR7 without it)
/// (pcre2_compile.c:8339).
#[test]
fn c097_lookaround_bsk() {
    isolate!("C097");
    let mut d = D::new("C097");
    unsafe {
        let cc = Cc::new();
        let pats: &[&[u8]] = &[
            b"(?=a\\K)b", b"(?<=a\\K)b", b"(?!a\\K)b", b"(?<!a\\K)b", b"a\\Kb",
            b"(?=(a\\K))(?1)", b"(?:a\\K)b", b"(?>a\\K)b", b"(*atomic:a\\K)", b"(*pla:a\\K)",
            b"(*nla:a\\K)", b"(?(?=a\\K)b|c)", b"(?=a(?:b\\K))", b"\\K", b"(?=\\K)",
            b"(?<=\\K)", b"(?*a\\K)", b"(?<*a\\K)", b"(*scs:(1)a\\K)(x)",
        ];
        matrix(
            &mut d,
            &cc,
            pats,
            &[0, PCRE2_UTF, PCRE2_ANCHORED],
            &[0, PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK],
        );
        fuzz(&mut d, &cc, 0x097, nfuzz(), &[], &[PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK]);
        cc.free();
    }
    d.finish();
}

// ============================================================== C098

/// C098 — PCRE2_MATCH_UNSET_BACKREF: changes the compile-time minimum-length reasoning
/// (pcre2_compile.c:9765,9806, pcre2_study.c:467-557).
#[test]
fn c098_match_unset_backref() {
    isolate!("C098");
    let mut d = D::new("C098");
    unsafe {
        let cc = Cc::new();
        let pats: &[&[u8]] = &[
            b"(a)?\\1b",
            b"(a)|\\1",
            b"(?<n>a)?\\k<n>b",
            b"(a)(b)?\\2",
            b"\\1(a)",
            b"(a)\\2(b)",
            b"(?:(a)|b)\\1",
            b"(?J)(?<n>a)|(?<n>b)\\k<n>",
            b"(?J)(?<n>a)(?<n>b)\\k<n>",
            b"(a)\\1",
            b"(a)?\\1",
            b"((a)|b)\\2",
            b"(?(1)x)(a)",
            b"(a)?(?(1)\\1|b)",
            b"\\g{-1}(a)",
            b"(a)(?P=n)",
        ];
        matrix(
            &mut d,
            &cc,
            pats,
            &[
                0,
                PCRE2_MATCH_UNSET_BACKREF,
                PCRE2_MATCH_UNSET_BACKREF | PCRE2_DUPNAMES,
                PCRE2_MATCH_UNSET_BACKREF | PCRE2_CASELESS,
                PCRE2_DUPNAMES,
            ],
            &[0],
        );
        fuzz(&mut d, &cc, 0x098, nfuzz(), &[PCRE2_MATCH_UNSET_BACKREF], &[]);
        cc.free();
    }
    d.finish();
}

// ============================================================== C099

/// C099 — PCRE2_ANCHORED / PCRE2_ENDANCHORED and the auto-anchoring path with
/// PCRE2_OPTIM_DOTSTAR_ANCHOR on and off (pcre2_compile.c:11106-11113,11186-11190).
#[test]
fn c099_anchoring() {
    isolate!("C099");
    let mut d = D::new("C099");
    unsafe {
        let cc = Cc::new();
        let pats: &[&[u8]] = &[
            b"abc", b"^abc", b"\\Aabc", b".*abc", b".*abc$", b".*?abc", b"(?>.*)abc",
            b".*(*PRUNE)abc", b".*(*SKIP)abc", b".*(*THEN)abc", b".*(*COMMIT)abc",
            b".*(*ACCEPT)abc", b"abc$", b"a|^b", b"(?:^a|^b)", b"^a|^b", b"(^a)|(^b)",
            b".*\\Kabc", b"(.*)abc", b"(?i).*abc", b"^", b"$", b".*", b"[^\\n]*abc",
            b"\\A.*abc", b"(?m)^abc", b"(?s).*abc", b"(?s:.*)abc", b"(*NO_DOTSTAR_ANCHOR).*abc",
        ];
        let optsets = [
            0u32,
            PCRE2_ANCHORED,
            PCRE2_ENDANCHORED,
            PCRE2_ANCHORED | PCRE2_ENDANCHORED,
            PCRE2_DOTALL,
            PCRE2_DOTALL | PCRE2_ANCHORED,
            PCRE2_DOTALL | PCRE2_MULTILINE,
            PCRE2_NO_DOTSTAR_ANCHOR,
            PCRE2_NO_DOTSTAR_ANCHOR | PCRE2_DOTALL,
        ];
        for &dir in &[
            PCRE2_OPTIMIZATION_FULL,
            PCRE2_DOTSTAR_ANCHOR_OFF,
            PCRE2_DOTSTAR_ANCHOR_D,
            PCRE2_OPTIMIZATION_NONE,
            PCRE2_START_OPTIMIZE_OFF,
            PCRE2_START_OPTIMIZE_D,
            PCRE2_AUTO_POSSESS_OFF,
            PCRE2_AUTO_POSSESS_D,
        ] {
            cc.optimize(dir);
            for &o in &optsets {
                for p in pats {
                    d.probe(&format!("optimize={}", dir), p, p.len(), o, cc.c, cc.r);
                }
            }
        }
        // bad directive values must be rejected identically
        for &dir in &[2u32, 63, 70, 71, 1000, 0xffff_ffff] {
            cc.optimize(dir);
        }
        cc.optimize(PCRE2_OPTIMIZATION_FULL);
        fuzz(&mut d, &cc, 0x099, nfuzz(), &[PCRE2_ANCHORED, PCRE2_ENDANCHORED, PCRE2_NO_DOTSTAR_ANCHOR, PCRE2_DOTALL], &[]);
        cc.free();
    }
    d.finish();
}

// ============================================================== C100

/// C100 — PCRE2_FIRSTLINE / PCRE2_NO_START_OPTIMIZE (and PCRE2_OPTIM_START_OPTIMIZE)
/// against every newline convention: the whole first-code-unit / start-bitmap /
/// required-code-unit / minlength block (pcre2_compile.c:11122-11250).
#[test]
fn c100_start_optimize() {
    isolate!("C100");
    let mut d = D::new("C100");
    unsafe {
        let cc = Cc::new();
        let pats: &[&[u8]] = &[
            b"abc", b"[ab]c", b"\\d+", b"^abc", b"(word|WORD)", b"a*a", b"(?=a)ab", b"\\Aabc",
            b"(?i)abc", b"(?i)[ab]c", b"\\R", b"\\Rabc", b"a\\nb", b"a\\r\\nb", b"(?m)^abc",
            b"\\bword\\b", b"(?:ab|cd)ef", b"[^a]b", b"\\p{L}x", b"(a)(b)\\2", b"x{3,}y",
            b"(?U)a*b", b"abc|abd", b"\\Kabc", b"(*NO_START_OPT)abc", b"(?s).*x",
        ];
        let optsets = [
            0u32,
            PCRE2_FIRSTLINE,
            PCRE2_NO_START_OPTIMIZE,
            PCRE2_FIRSTLINE | PCRE2_NO_START_OPTIMIZE,
            PCRE2_FIRSTLINE | PCRE2_MULTILINE,
            PCRE2_CASELESS | PCRE2_FIRSTLINE,
            PCRE2_UTF | PCRE2_FIRSTLINE,
        ];
        for nl in 1..=6 {
            cc.newline(nl);
            for &dir in &[PCRE2_OPTIMIZATION_FULL, PCRE2_START_OPTIMIZE_OFF, PCRE2_OPTIMIZATION_NONE] {
                cc.optimize(dir);
                for &o in &optsets {
                    for p in pats {
                        d.probe(&format!("newline={} optimize={}", nl, dir), p, p.len(), o, cc.c, cc.r);
                    }
                }
            }
        }
        cc.newline(PCRE2_NEWLINE_LF);
        cc.optimize(PCRE2_OPTIMIZATION_FULL);
        fuzz(&mut d, &cc, 0x100, nfuzz(), &[PCRE2_FIRSTLINE, PCRE2_NO_START_OPTIMIZE], &[]);
        // and the same random patterns with the start optimization switched off in the
        // compile context rather than through the option bit
        cc.optimize(PCRE2_START_OPTIMIZE_OFF);
        fuzz(&mut d, &cc, 0x1100, nfuzz() / 2, &[PCRE2_FIRSTLINE], &[]);
        cc.optimize(PCRE2_OPTIMIZATION_FULL);
        cc.free();
    }
    d.finish();
}

// ============================================================== C101

/// C101 — pcre2_set_max_pattern_length: the `patlen > max_pattern_length` check after
/// the PCRE2_ZERO_TERMINATED strlen conversion, ERR88 (pcre2_compile.c:10399-10401).
#[test]
fn c101_max_pattern_length() {
    isolate!("C101");
    let mut d = D::new("C101");
    unsafe {
        let cc = Cc::new();
        let pats: &[&[u8]] = &[b"", b"a", b"abc", b"(a)(?<n>b)\\1[x-z]", b"a{2,3}(?:b|c)*"];
        for p in pats {
            let n = p.len();
            let mut lens: Vec<usize> = vec![0, 1, PCRE2_UNSET];
            if n > 0 {
                lens.push(n - 1);
            }
            lens.push(n);
            lens.push(n + 1);
            lens.push(n + 100);
            for &lim in &lens {
                cc.max_pattern_length(lim);
                d.probe(&format!("max_pattern_length={:#x} explicit", lim), p, n, 0, cc.c, cc.r);
                let mut z = p.to_vec();
                z.push(0);
                d.probe(
                    &format!("max_pattern_length={:#x} zero-term", lim),
                    &z,
                    PCRE2_ZERO_TERMINATED,
                    0,
                    cc.c,
                    cc.r,
                );
                // an embedded NUL makes the two length forms disagree
                let mut e = p.to_vec();
                e.push(0);
                e.extend_from_slice(b"xyzzy");
                e.push(0);
                d.probe(
                    &format!("max_pattern_length={:#x} embedded-NUL zero-term", lim),
                    &e,
                    PCRE2_ZERO_TERMINATED,
                    0,
                    cc.c,
                    cc.r,
                );
                d.probe(
                    &format!("max_pattern_length={:#x} embedded-NUL explicit", lim),
                    &e,
                    e.len() - 1,
                    0,
                    cc.c,
                    cc.r,
                );
            }
        }
        // random patterns against a random limit near their length
        let mut rng = Rng::new(0x101);
        for i in 0..(nfuzz() / 4).max(60) {
            let pat = gen_pattern(&mut rng);
            let delta = rng.below(5) as i64 - 2;
            let lim = (pat.len() as i64 + delta).max(0) as usize;
            cc.max_pattern_length(lim);
            d.probe(&format!("rand #{} lim={}", i, lim), &pat, pat.len(), 0, cc.c, cc.r);
        }
        cc.max_pattern_length(PCRE2_UNSET);
        cc.free();
    }
    d.finish();
}

// ============================================================== C102

/// C102 — pcre2_set_max_pattern_compiled_length: the limit is compared against the
/// name-table + char-list + code size in bytes, before the pcre2_real_code header is
/// added, giving ERR101 (pcre2_compile.c:10855-10877). The limit is swept one byte at a
/// time across the whole plausible range so the exact flip point must agree.
#[test]
fn c102_max_pattern_compiled_length() {
    isolate!("C102");
    let mut d = D::new("C102");
    unsafe {
        let cc = Cc::new();
        // (pattern, options) — the char-list / XCLASS shapes need PCRE2_UTF in the
        // 8-bit build so that code points above 0xff are legal at all
        let pats: &[(&[u8], u32)] = &[
            (b"a", 0),
            (b"abc", 0),
            (b"(?<name>a)(?<other>b)\\k<name>", 0),
            (b"[\\x{100}-\\x{200}\\x{300}-\\x{400}]", PCRE2_UTF),
            (b"[\\p{L}\\p{Greek}\\d]", 0),
            (b"[\\p{L}\\p{Greek}\\d]", PCRE2_UTF),
            (b"(?<n>a)[\\x{500}-\\x{600}]\\k<n>(?:x|y){2,5}", PCRE2_UTF),
            (b"(?<n>a)(?<m>b)(?<o>c)[a-f0-9]+\\k<n>", 0),
        ];
        for &(p, opts) in pats {
            cc.max_pattern_compiled_length(PCRE2_UNSET);
            let (a, _) = d.probe("unlimited", p, p.len(), opts, cc.c, cc.r);
            let size = a
                .info
                .iter()
                .find(|(k, _, _)| *k == PCRE2_INFO_SIZE)
                .map(|(_, _, v)| *v as usize)
                .unwrap_or(0);
            assert!(size > 0, "C102: no PCRE2_INFO_SIZE for {:?} opts=0x{:x}", esc(p), opts);
            let lo = size.saturating_sub(200);
            for lim in lo..=(size + 8) {
                cc.max_pattern_compiled_length(lim);
                d.probe(&format!("limit={} (INFO_SIZE={})", lim, size), p, p.len(), opts, cc.c, cc.r);
            }
            for &lim in &[0usize, 1, 2, 4, 8, 16, 32, PCRE2_UNSET] {
                cc.max_pattern_compiled_length(lim);
                d.probe(&format!("limit={:#x}", lim), p, p.len(), opts, cc.c, cc.r);
            }
        }
        // random patterns with a random limit
        let mut rng = Rng::new(0x102);
        for i in 0..(nfuzz() / 4).max(60) {
            let pat = gen_pattern(&mut rng);
            let lim = rng.below(400) as usize;
            cc.max_pattern_compiled_length(lim);
            d.probe(&format!("rand #{} lim={}", i, lim), &pat, pat.len(), 0, cc.c, cc.r);
        }
        cc.max_pattern_compiled_length(PCRE2_UNSET);
        cc.free();
    }
    d.finish();
}

// ============================================================== C103

/// C103 — pcre2_set_parens_nest_limit: `nest_depth > parens_nest_limit` gives ERR19
/// during the parse (pcre2_compile.c:3211-3240).
#[test]
fn c103_parens_nest_limit() {
    isolate!("C103");
    let mut d = D::new("C103");
    unsafe {
        let cc = Cc::new();
        let kinds: &[(&str, &str, &str)] = &[
            ("capture", "(", ")"),
            ("non-capture", "(?:", ")"),
            ("lookahead", "(?=", ")"),
            ("lookbehind-na", "(?<*", ")"),
            ("atomic", "(?>", ")"),
            ("branch-reset", "(?|", ")"),
            ("named", "(?<n>", ")"),
            ("options", "(?i:", ")"),
        ];
        for &lim in &[0u32, 1, 2, 3, 248, 249, 250, 251, 252, u32::MAX] {
            cc.parens_nest_limit(lim);
            for &(kind, open, close) in kinds {
                for n in [0usize, 1, 2, 3, 249, 250, 251, 252] {
                    // deeply nested named groups repeat the same name; keep to the
                    // shapes that are legal so the depth check is what is observed
                    if kind == "named" && n > 1 {
                        continue;
                    }
                    let mut s = String::new();
                    for _ in 0..n {
                        s.push_str(open);
                    }
                    s.push('a');
                    for _ in 0..n {
                        s.push_str(close);
                    }
                    d.probe(
                        &format!("limit={} {} depth={}", lim, kind, n),
                        s.as_bytes(),
                        s.len(),
                        0,
                        cc.c,
                        cc.r,
                    );
                }
            }
        }
        // mixed nesting, random depth, random limit
        let mut rng = Rng::new(0x103);
        for i in 0..(nfuzz() / 8).max(60) {
            let depth = rng.below(300) as usize;
            let mut s = Vec::new();
            let mut closers: Vec<&[u8]> = vec![];
            for _ in 0..depth {
                let o = *rng.pick(&[
                    b"(".as_ref(),
                    b"(?:".as_ref(),
                    b"(?=".as_ref(),
                    b"(?>".as_ref(),
                    b"(?|".as_ref(),
                    b"(?i:".as_ref(),
                    b"(*atomic:".as_ref(),
                ]);
                s.extend_from_slice(o);
                closers.push(b")");
            }
            s.push(b'a');
            for _ in closers {
                s.push(b')');
            }
            let lim = *rng.pick(&[0u32, 1, 10, 100, 249, 250, 251, 300, u32::MAX]);
            cc.parens_nest_limit(lim);
            d.probe(&format!("rand #{} depth={} lim={}", i, depth, lim), &s, s.len(), 0, cc.c, cc.r);
        }
        cc.parens_nest_limit(250);
        cc.free();
    }
    d.finish();
}

// ============================================================== C104

/// C104 — pcre2_set_max_varlookbehind (ERR100) and the hard 65535 cap (ERR87)
/// (pcre2_compile.c:10066-10068).
#[test]
fn c104_max_varlookbehind() {
    isolate!("C104");
    let mut d = D::new("C104");
    unsafe {
        let cc = Cc::new();
        let mut pats: Vec<Vec<u8>> = vec![
            b"(?<=a|bb|ccc)x".to_vec(),
            b"(?<=(a|bb)(c|dd))x".to_vec(),
            b"(?<=abc)x".to_vec(),
            b"(?<=a{3})x".to_vec(),
            b"(?<!a|bb)x".to_vec(),
            b"(?<*a|bb)x".to_vec(),
            b"(*plb:a|bb)x".to_vec(),
            b"(*nlb:a|bb)x".to_vec(),
            b"(?<=\\d+)x".to_vec(),
            b"(?<=a*)x".to_vec(),
            b"(?<=(?<=a)b)x".to_vec(),
            b"(?<=a{65535})x".to_vec(),
            b"(?<=a{65536})x".to_vec(),
            b"(?<=a{0,65535})x".to_vec(),
            b"(?<=a{0,65536})x".to_vec(),
        ];
        for n in [0usize, 1, 2, 253, 254, 255, 256, 257] {
            pats.push(format!("(?<=a{{0,{}}})x", n).into_bytes());
            pats.push(format!("(?<=a{{{},{}}})x", n, n + 1).into_bytes());
            pats.push(format!("(?<={})x", "a".repeat(n)).into_bytes());
        }
        let refs: Vec<&[u8]> = pats.iter().map(|v| v.as_slice()).collect();
        for &lim in &[0u32, 1, 2, 253, 254, 255, 256, 65535, 65536, u32::MAX] {
            cc.max_varlookbehind(lim);
            for p in &refs {
                d.probe(&format!("max_varlookbehind={}", lim), p, p.len(), 0, cc.c, cc.r);
            }
        }
        cc.max_varlookbehind(255);
        // random patterns wrapped in a lookbehind, random limit
        let mut rng = Rng::new(0x104);
        for i in 0..(nfuzz() / 4).max(60) {
            let inner = gen_pattern(&mut rng);
            let mut s = Vec::new();
            s.extend_from_slice(*rng.pick(&[
                b"(?<=".as_ref(),
                b"(?<!".as_ref(),
                b"(?<*".as_ref(),
                b"(*nlb:".as_ref(),
            ]));
            s.extend_from_slice(&inner);
            s.extend_from_slice(b")x");
            let lim = *rng.pick(&[0u32, 1, 5, 254, 255, 256, 65535, u32::MAX]);
            cc.max_varlookbehind(lim);
            d.probe(&format!("rand #{} lim={}", i, lim), &s, s.len(), 0, cc.c, cc.r);
        }
        cc.max_varlookbehind(255);
        cc.free();
    }
    d.finish();
}

// ============================================================== C105

struct GuardState {
    threshold: u32,
    seen: Vec<u32>,
}

unsafe extern "C" fn guard_cb(depth: u32, data: Ptr) -> i32 {
    let st = &mut *(data as *mut GuardState);
    st.seen.push(depth);
    if st.threshold != 0 && depth >= st.threshold {
        1
    } else {
        0
    }
}

unsafe extern "C" fn guard_zero(_depth: u32, _data: Ptr) -> i32 {
    0
}

/// C105 — pcre2_set_compile_recursion_guard: the guard is called with cb->parens_depth
/// at each group entry and a non-zero return gives ERR33 (pcre2_compile.c:8598-8601).
/// The sequence of depths the guard is called with is compared too.
#[test]
fn c105_compile_recursion_guard() {
    isolate!("C105");
    let mut d = D::new("C105");
    unsafe {
        let (c, r) = both();
        let cc = Cc::new();
        let mut pats: Vec<Vec<u8>> = vec![
            b"a".to_vec(),
            b"(a)".to_vec(),
            b"(a)(b)".to_vec(),
            b"((((a))))".to_vec(),
            b"(?:(?:(?:a)))".to_vec(),
            b"(?=(a))(b)".to_vec(),
            b"(a(b(c(d))))".to_vec(),
        ];
        for n in [0usize, 1, 5, 249, 250] {
            let mut s = String::new();
            for _ in 0..n {
                s.push('(');
            }
            s.push('a');
            for _ in 0..n {
                s.push(')');
            }
            pats.push(s.into_bytes());
        }
        for &thr in &[0u32, 1, 2, 5, 250, 251, 1000] {
            for p in &pats {
                let mut sc = GuardState { threshold: thr, seen: vec![] };
                let mut sr = GuardState { threshold: thr, seen: vec![] };
                assert_eq!(
                    (c.pcre2_set_compile_recursion_guard_8)(
                        cc.c,
                        Some(guard_cb),
                        &mut sc as *mut GuardState as Ptr
                    ),
                    (r.pcre2_set_compile_recursion_guard_8)(
                        cc.r,
                        Some(guard_cb),
                        &mut sr as *mut GuardState as Ptr
                    )
                );
                d.probe(&format!("guard threshold={}", thr), p, p.len(), 0, cc.c, cc.r);
                if sc.seen != sr.seen {
                    d.record(
                        "guard-depth-sequence".to_string(),
                        format!(
                            "pattern \"{}\" threshold {}: C depths {:?} vs RUST depths {:?}",
                            esc(p),
                            thr,
                            sc.seen,
                            sr.seen
                        ),
                    );
                }
            }
        }
        // guard that always returns 0, and no guard at all
        for p in &pats {
            assert_eq!(
                (c.pcre2_set_compile_recursion_guard_8)(cc.c, Some(guard_zero), ptr::null_mut()),
                (r.pcre2_set_compile_recursion_guard_8)(cc.r, Some(guard_zero), ptr::null_mut())
            );
            d.probe("guard always-0", p, p.len(), 0, cc.c, cc.r);
            assert_eq!(
                (c.pcre2_set_compile_recursion_guard_8)(cc.c, None, ptr::null_mut()),
                (r.pcre2_set_compile_recursion_guard_8)(cc.r, None, ptr::null_mut())
            );
            d.probe("guard NULL", p, p.len(), 0, cc.c, cc.r);
        }
        // random patterns with a random threshold, comparing the depth sequences
        let mut rng = Rng::new(0x105);
        for i in 0..(nfuzz() / 4).max(60) {
            let pat = gen_pattern(&mut rng);
            let thr = *rng.pick(&[0u32, 1, 2, 3, 8, 100]);
            let mut sc = GuardState { threshold: thr, seen: vec![] };
            let mut sr = GuardState { threshold: thr, seen: vec![] };
            (c.pcre2_set_compile_recursion_guard_8)(
                cc.c,
                Some(guard_cb),
                &mut sc as *mut GuardState as Ptr,
            );
            (r.pcre2_set_compile_recursion_guard_8)(
                cc.r,
                Some(guard_cb),
                &mut sr as *mut GuardState as Ptr,
            );
            d.probe(&format!("rand #{} threshold={}", i, thr), &pat, pat.len(), 0, cc.c, cc.r);
            if sc.seen != sr.seen {
                d.record(
                    "guard-depth-sequence".to_string(),
                    format!(
                        "pattern \"{}\" threshold {}: C depths {:?} vs RUST depths {:?}",
                        esc(&pat),
                        thr,
                        sc.seen,
                        sr.seen
                    ),
                );
            }
        }
        (c.pcre2_set_compile_recursion_guard_8)(cc.c, None, ptr::null_mut());
        (r.pcre2_set_compile_recursion_guard_8)(cc.r, None, ptr::null_mut());
        cc.free();
    }
    d.finish();
}

// ============================================================== C106

/// C106 — the parsed-pattern buffer: on the stack up to PARSED_PATTERN_DEFAULT_SIZE
/// (1024) uint32_t, otherwise malloc'd (pcre2_compile.c:10326,10728-10752).
#[test]
fn c106_parsed_pattern_heap() {
    isolate!("C106");
    let mut d = D::new("C106");
    unsafe {
        let cc = Cc::new();
        let n = PARSED_PATTERN_DEFAULT_SIZE;
        let sizes = [
            1usize,
            n / 8,
            n / 5 - 2,
            n / 5 - 1,
            n / 5,
            n / 5 + 1,
            n - 4,
            n - 3,
            n - 2,
            n - 1,
            n,
            n + 1,
            n + 2,
            n + 8,
            2 * n,
            4 * n,
        ];
        for &sz in &sizes {
            let lits = "a".repeat(sz);
            let groups = "(?:a)".repeat(sz / 4);
            let named = (0..(sz / 40).max(1)).map(|i| format!("(?<n{}>a)", i)).collect::<String>();
            for (what, p) in [("literals", &lits), ("groups", &groups), ("named", &named)] {
                for &o in &[0u32, PCRE2_AUTO_CALLOUT, PCRE2_EXTENDED, PCRE2_UTF, PCRE2_CASELESS] {
                    d.probe(
                        &format!("{} n={} opts=0x{:x}", what, sz, o),
                        p.as_bytes(),
                        p.len(),
                        o,
                        cc.c,
                        cc.r,
                    );
                }
            }
        }
        // long random patterns (well past the stack buffer) with random options
        let mut rng = Rng::new(0x106);
        for i in 0..(nfuzz() / 20).max(50) {
            let mut pat = Vec::new();
            while pat.len() < 1200 {
                let p = gen_pattern(&mut rng);
                pat.extend_from_slice(&p);
                pat.push(b'|');
            }
            pat.pop();
            let o = subset(&mut rng, RAND_OPTS, 12);
            d.probe(&format!("rand long #{} len={}", i, pat.len()), &pat, pat.len(), o, cc.c, cc.r);
        }
        cc.free();
    }
    d.finish();
}

// ============================================================== C107

/// C107 — the compile workspace (COMPILE_WORK_SIZE = 6000 code units), the LINK_SIZE=2
/// boundaries and MAX_PATTERN_SIZE = 65536 code units (ERR20)
/// (pcre2_compile.c:136,150,6167-6207,10841-10848).
#[test]
fn c107_workspace_and_size_limits() {
    isolate!("C107");
    let mut d = D::new("C107");
    unsafe {
        let cc = Cc::new();
        // "a"*n compiles to about 2n code units: sweep both boundaries
        for &n in &[
            10usize, 2990, 2999, 3000, 3001, 3010, 5990, 6000, 6010, 32000, 32700, 32760, 32764,
            32766, 32767, 32768, 32769, 32800, 40000,
        ] {
            let p = "a".repeat(n);
            d.probe(&format!("\"a\"x{}", n), p.as_bytes(), p.len(), 0, cc.c, cc.r);
        }
        // groups: each "(?:a)" and "(a)" costs several code units and a bracket offset
        for &n in &[10usize, 1000, 1200, 1500, 6000, 8000, 9000, 10000, 12000] {
            let p1 = "(?:a)".repeat(n);
            d.probe(&format!("\"(?:a)\"x{}", n), p1.as_bytes(), p1.len(), 0, cc.c, cc.r);
            let p2 = "(a)".repeat(n);
            d.probe(&format!("\"(a)\"x{}", n), p2.as_bytes(), p2.len(), 0, cc.c, cc.r);
        }
        // a single group whose contents are huge, so the LINK_SIZE=2 bracket offset is
        // pushed towards saturation
        for &n in &[3000usize, 16000, 30000, 32000, 32800] {
            let p = format!("({})", "a".repeat(n));
            d.probe(&format!("(a x{})", n), p.as_bytes(), p.len(), 0, cc.c, cc.r);
            let q = format!("(?:{}|b)", "a".repeat(n));
            d.probe(&format!("(?:a x{}|b)", n), q.as_bytes(), q.len(), 0, cc.c, cc.r);
        }
        // classes with thousands of ranges (XCLASS / char-list encodings)
        for &n in &[10usize, 100, 1000, 2000, 4000, 8000] {
            let mut s = String::from("[");
            for i in 0..n {
                s.push_str(&format!("\\x{{{:X}}}-\\x{{{:X}}}", 0x100 + 4 * i, 0x101 + 4 * i));
            }
            s.push(']');
            for &o in &[0u32, PCRE2_UTF, PCRE2_CASELESS] {
                d.probe(&format!("class {} ranges opts=0x{:x}", n, o), s.as_bytes(), s.len(), o, cc.c, cc.r);
            }
        }
        // quantified groups: replication is what actually overruns the workspace
        for &n in &[100usize, 1000, 2000, 3000] {
            let p = format!("(?:{}){{2}}", "a".repeat(n));
            d.probe(&format!("(?:a x{}){{2}}", n), p.as_bytes(), p.len(), 0, cc.c, cc.r);
            let q = format!("(?:a){{{}}}", n);
            d.probe(&format!("(?:a){{{}}}", n), q.as_bytes(), q.len(), 0, cc.c, cc.r);
        }
        // deeply concatenated random material, driven past the workspace size
        let mut rng = Rng::new(0x107);
        for i in 0..(nfuzz() / 100).max(20) {
            let mut pat = Vec::new();
            let target = 2000 + rng.below(6000) as usize;
            while pat.len() < target {
                pat.extend_from_slice(&gen_pattern(&mut rng));
            }
            let o = subset(&mut rng, RAND_OPTS, 14);
            d.probe(&format!("rand huge #{} len={}", i, pat.len()), &pat, pat.len(), o, cc.c, cc.r);
        }
        cc.free();
    }
    d.finish();
}

// ============================================================== C108

/// C108 — capture counts across the interesting boundaries and the derived
/// CAPTURECOUNT / FRAMESIZE / NAMECOUNT / NAMEENTRYSIZE (pcre2_pattern_info.c:136-166).
/// NOTE: above roughly 8000 groups the compiled form exceeds MAX_PATTERN_SIZE, so the
/// 65535/65536 group cases come out as a size error — both libraries must produce the
/// SAME error for them.
#[test]
fn c108_capture_counts() {
    isolate!("C108");
    let mut d = D::new("C108");
    unsafe {
        let cc = Cc::new();
        for &n in &[0usize, 1, 2, 3, 62, 63, 64, 65, 66, 99, 100, 101, 1000, 8000, 65535, 65536] {
            let p1 = "(a)".repeat(n);
            d.probe(&format!("{} plain captures", n), p1.as_bytes(), p1.len(), 0, cc.c, cc.r);
            let p2 = "()".repeat(n);
            d.probe(&format!("{} empty captures", n), p2.as_bytes(), p2.len(), 0, cc.c, cc.r);
            if n <= 1000 {
                let p3 = (0..n).map(|i| format!("(?<n{}>a)", i)).collect::<String>();
                d.probe(&format!("{} named captures", n), p3.as_bytes(), p3.len(), 0, cc.c, cc.r);
                let p4 = format!("{}{}", "(".repeat(n), ")".repeat(n));
                d.probe(&format!("{} nested captures", n), p4.as_bytes(), p4.len(), 0, cc.c, cc.r);
                let p5 = format!("{}a{}\\{}", "(".repeat(n), ")".repeat(n), n.max(1));
                d.probe(&format!("{} nested + backref", n), p5.as_bytes(), p5.len(), 0, cc.c, cc.r);
            }
            for &o in &[PCRE2_NO_AUTO_CAPTURE, PCRE2_DUPNAMES, PCRE2_UTF] {
                if n <= 100 {
                    d.probe(&format!("{} captures opts=0x{:x}", n, o), p1.as_bytes(), p1.len(), o, cc.c, cc.r);
                }
            }
        }
        fuzz(&mut d, &cc, 0x108, nfuzz() / 2, &[PCRE2_NO_AUTO_CAPTURE, PCRE2_DUPNAMES], &[]);
        cc.free();
    }
    d.finish();
}

// ============================================================== C109

/// C109 — every named-group and name-reference syntax, the MAX_NAME_SIZE /
/// MAX_NAME_COUNT limits and the first+last-character hash used by the name table
/// (pcre2_compile_cgroup.c:58-99, pcre2_compile.c:5736).
#[test]
fn c109_named_groups() {
    isolate!("C109");
    let mut d = D::new("C109");
    unsafe {
        let cc = Cc::new();
        let pats: &[&[u8]] = &[
            b"(?<n>a)",
            b"(?'n'a)",
            b"(?P<n>a)",
            b"(?<n>a)\\k<n>",
            b"(?<n>a)\\k'n'",
            b"(?<n>a)\\k{n}",
            b"(?<n>a)(?P=n)",
            b"(?<n>a)\\g{n}",
            b"(?<n>a)(?&n)",
            b"(?<n>a)(?P>n)",
            b"\\k<n>",
            b"\\k'n'",
            b"\\k{n}",
            b"(?P=n)",
            b"\\g{n}",
            b"(?&n)",
            b"(?P>n)",
            b"(?<1n>a)",
            b"(?<>a)",
            b"(?<n->a)",
            b"(?<n n>a)",
            b"(?<n.>a)",
            b"(?<\xc3\xa9>a)",
            b"(?<n>a)(?<m>b)\\k<m>\\k<n>",
            // hash collisions: same first+last character
            b"(?<abc>a)(?<axc>b)\\k<abc>\\k<axc>",
            b"(?<ac>a)(?<abbbbc>b)\\k<ac>\\k<abbbbc>",
            b"(?<aa>1)(?<aba>2)(?<abba>3)\\k<aa>\\k<aba>\\k<abba>",
            b"(?<a>1)(?<b>2)\\k<a>\\k<b>",
            b"(?J)(?<n>a)(?<n>b)\\k<n>",
            b"(?<n>a)(?(<n>)x|y)",
            b"(?<n>a)(?('n')x|y)",
            b"(?<n>a)(?(R&n)x|y)",
            b"(?<n>(?&n)?a)",
        ];
        matrix(&mut d, &cc, pats, &[0, PCRE2_DUPNAMES, PCRE2_UTF, PCRE2_NO_AUTO_CAPTURE], &[0]);

        // MAX_NAME_SIZE = 128
        for n in [1usize, 127, 128, 129, 130, 200] {
            let nm = "n".repeat(n);
            for pat in [
                format!("(?<{}>a)", nm),
                format!("(?<{}>a)\\k<{}>", nm, nm),
                format!("(?<{}>a)(?&{})", nm, nm),
                format!("\\k<{}>", nm),
            ] {
                d.probe(&format!("name size {}", n), pat.as_bytes(), pat.len(), 0, cc.c, cc.r);
            }
        }
        // MAX_NAME_COUNT = 10000 (see the C089 note about ERR20 arriving first)
        for n in [99usize, 9999, 10000, 10001] {
            let p = (0..n).map(|i| format!("(?<nm{}>)", i)).collect::<String>();
            d.probe(&format!("{} names", n), p.as_bytes(), p.len(), 0, cc.c, cc.r);
        }
        // random named-group soup
        let mut rng = Rng::new(0x109);
        let names: &[&str] = &["n", "m", "abc", "axc", "aa", "aba", "x1", "N", "nn", "n_m"];
        for i in 0..(nfuzz() / 2).max(60) {
            let mut s = String::new();
            let k = 1 + rng.below(5);
            for _ in 0..k {
                let nm = *rng.pick(names);
                match rng.below(6) {
                    0 => s.push_str(&format!("(?<{}>a)", nm)),
                    1 => s.push_str(&format!("(?'{}'b)", nm)),
                    2 => s.push_str(&format!("(?P<{}>c)", nm)),
                    3 => s.push_str(&format!("\\k<{}>", nm)),
                    4 => s.push_str(&format!("(?&{})", nm)),
                    _ => s.push_str(&format!("(?({})x|y)", nm)),
                }
            }
            let o = if rng.below(2) == 0 { PCRE2_DUPNAMES } else { 0 };
            d.probe(&format!("rand names #{}", i), s.as_bytes(), s.len(), o, cc.c, cc.r);
        }
        cc.free();
    }
    d.finish();
}

// ============================================================== C110

/// C110 — every group-reference syntax and the relative-number resolution against
/// cb->bracount (pcre2_compile.c:1882-1902,4807).
#[test]
fn c110_group_references() {
    isolate!("C110");
    let mut d = D::new("C110");
    unsafe {
        let cc = Cc::new();
        let pats: &[&[u8]] = &[
            b"(a)\\1", b"(a)\\g1", b"(a)\\g{1}", b"(a)\\g{-1}", b"(a)\\g{+1}", b"(a)\\g{0}",
            b"(a)(?1)", b"(a)(?-1)", b"(a)(?+1)", b"(?R)", b"(?0)", b"(?2)(a)(b)", b"(?3)(a)(b)",
            b"\\1", b"(a)\\2", b"(a)(?2)", b"(?1)(a)", b"((?1))", b"(a(?1))", b"(?1)((?R))",
            b"(a)(?1)*", b"(a)\\1{2,3}", b"(?:(a)\\1)", b"(a)(b)\\g{-2}", b"(a)(b)\\g{-1}",
            b"(a)(b)(c)\\g{-3}", b"(a)(b)(c)\\g{-4}", b"(a)\\g<1>", b"(a)\\g'1'", b"(a)\\g{ 1 }",
            b"(a)\\g", b"(a)\\g{}", b"(a)\\g{-}", b"(a)\\g{+}", b"(a)\\g{99}", b"(?99)",
            b"(a)(?+2)(b)", b"(?-0)", b"(?+0)", b"(a)\\g{+2}(b)",
        ];
        matrix(
            &mut d,
            &cc,
            pats,
            &[0, PCRE2_NO_AUTO_CAPTURE, PCRE2_DUPNAMES, PCRE2_CASELESS, PCRE2_MATCH_UNSET_BACKREF],
            &[0],
        );
        // random reference soup over a random number of groups
        let mut rng = Rng::new(0x110);
        for i in 0..(nfuzz() / 2).max(60) {
            let ngroups = rng.below(5);
            let mut s = String::new();
            for _ in 0..ngroups {
                s.push_str("(a)");
            }
            let k = 1 + rng.below(4);
            for _ in 0..k {
                let num = rng.below(6);
                match rng.below(8) {
                    0 => s.push_str(&format!("\\{}", num)),
                    1 => s.push_str(&format!("\\g{}", num)),
                    2 => s.push_str(&format!("\\g{{{}}}", num)),
                    3 => s.push_str(&format!("\\g{{-{}}}", num)),
                    4 => s.push_str(&format!("\\g{{+{}}}", num)),
                    5 => s.push_str(&format!("(?{})", num)),
                    6 => s.push_str(&format!("(?-{})", num)),
                    _ => s.push_str(&format!("(?+{})", num)),
                }
            }
            d.probe(&format!("rand refs #{}", i), s.as_bytes(), s.len(), 0, cc.c, cc.r);
        }
        cc.free();
    }
    d.finish();
}

// ============================================================== C111

/// C111 — quantifier boundaries and the OP_EXACT/OP_UPTO/... encodings with IMM2_SIZE=2
/// (pcre2_compile.c:1433,6118-6207).
#[test]
fn c111_quantifiers() {
    isolate!("C111");
    let mut d = D::new("C111");
    unsafe {
        let cc = Cc::new();
        let quants: &[&str] = &[
            "", "*", "+", "?", "*?", "+?", "??", "*+", "++", "?+", "{0,0}", "{0}", "{1}", "{0,1}",
            "{1,1}", "{2}", "{2,3}", "{2,1}", "{,3}", "{3,}", "{}", "{ }", "{2,", "{65533}",
            "{65534}", "{65535}", "{65536}", "{0,65535}", "{0,65536}", "{65535,65535}", "**", "*?+",
            "{2}{3}", "{1}?",
        ];
        // one atom per compiled form
        let atoms: &[(&str, u32)] = &[
            ("a", 0),
            ("[ab]", 0),
            ("[^ab]", 0),
            ("[\\x{100}]", PCRE2_UTF),
            ("[[a-z]&&[^q]]", PCRE2_ALT_EXTENDED_CLASS),
            ("(?:a)", 0),
            ("(a)", 0),
            ("(a)\\1", 0),
            ("\\X", 0),
            ("\\d", 0),
            ("\\p{L}", 0),
            (".", 0),
            ("\\R", 0),
            ("(?:)", 0),
            ("(?=a)", 0),
            ("(?>a)", 0),
            ("\\x{100}", PCRE2_UTF),
            ("(?i)a", 0),
            ("\\Qab\\E", 0),
            ("(?C1)a", 0),
        ];
        for &(atom, extra_opt) in atoms {
            for q in quants {
                let p = format!("{}{}", atom, q);
                for &o in &[extra_opt, extra_opt | PCRE2_UNGREEDY, extra_opt | PCRE2_CASELESS] {
                    d.probe(&format!("atom={} quant={}", atom, q), p.as_bytes(), p.len(), o, cc.c, cc.r);
                }
            }
        }
        // {n,m} on a group replicates it: check the boundary where that overflows
        for &(n, m) in &[(1usize, 2usize), (2, 4), (100, 200), (1000, 1001), (0, 4000), (2, 32000)] {
            let p = format!("(?:ab){{{},{}}}", n, m);
            d.probe(&format!("group {{{},{}}}", n, m), p.as_bytes(), p.len(), 0, cc.c, cc.r);
        }
        fuzz(&mut d, &cc, 0x111, nfuzz(), &[PCRE2_UNGREEDY, PCRE2_NO_AUTO_POSSESS], &[]);
        cc.free();
    }
    d.finish();
}

// ============================================================== C112

/// C112 — every compiled class form: OP_CLASS, OP_NCLASS, OP_XCLASS (legacy items and
/// XCL_LIST), OP_ECLASS, OP_ALLANY and the ECLASS collapse
/// (pcre2_compile_class.c:1069-1861,2213-2760).
#[test]
fn c112_class_forms() {
    isolate!("C112");
    let mut d = D::new("C112");
    unsafe {
        let cc = Cc::new();
        let mut pats: Vec<Vec<u8>> = vec![
            b"[abc]".to_vec(),
            b"[^abc]".to_vec(),
            b"[\\x{100}]".to_vec(),
            b"[^\\x{100}]".to_vec(),
            b"[[:alpha:]]".to_vec(),
            b"[[:^alpha:]]".to_vec(),
            b"[\\p{L}]".to_vec(),
            b"[\\P{L}]".to_vec(),
            b"[\\p{Han}]".to_vec(),
            b"[\\p{Bidi_Control}]".to_vec(),
            b"[\\d\\s\\w]".to_vec(),
            b"[a-\\x{10FFFF}]".to_vec(),
            b"[[a-z]&&[^q]]".to_vec(),
            b"(?[[a-z]-[q]])".to_vec(),
            b"[^\\x00-\\x{10FFFF}]".to_vec(),
            b"[\\x00-\\x{10FFFF}]".to_vec(),
            b"[\\x00-\\xff]".to_vec(),
            b"[^\\x00-\\xff]".to_vec(),
            b"[a]".to_vec(),
            b"[^a]".to_vec(),
            b"[\\x{80}-\\x{ff}]".to_vec(),
            b"[a-b-c]".to_vec(),
            b"[\\d-z]".to_vec(),
            b"[z-a]".to_vec(),
            b"[\\x{100}-\\x{10FFFF}]".to_vec(),
            b"[^\\p{L}\\p{N}]".to_vec(),
            b"[\\p{L}&&[^\\p{Greek}]]".to_vec(),
            b"[\\Qab\\E-c]".to_vec(),
        ];
        // enough disjoint ranges to force the XCL_LIST encoding
        for &n in &[2usize, 4, 8, 16, 32, 64, 128, 300] {
            let mut s = String::from("[");
            for i in 0..n {
                s.push_str(&format!("\\x{{{:X}}}-\\x{{{:X}}}", 0x100 + 8 * i, 0x104 + 8 * i));
            }
            s.push(']');
            pats.push(s.into_bytes());
            let mut t = String::from("[^");
            for i in 0..n {
                t.push_str(&format!("\\x{{{:X}}}-\\x{{{:X}}}", 0x100 + 8 * i, 0x104 + 8 * i));
            }
            t.push(']');
            pats.push(t.into_bytes());
        }
        let refs: Vec<&[u8]> = pats.iter().map(|v| v.as_slice()).collect();
        matrix(
            &mut d,
            &cc,
            &refs,
            &[
                0,
                PCRE2_UTF,
                PCRE2_UCP,
                PCRE2_UTF | PCRE2_UCP,
                PCRE2_ALT_EXTENDED_CLASS,
                PCRE2_ALT_EXTENDED_CLASS | PCRE2_UTF,
                PCRE2_CASELESS | PCRE2_UTF,
                PCRE2_CASELESS,
                PCRE2_UTF | PCRE2_ALLOW_EMPTY_CLASS,
            ],
            &[0, PCRE2_EXTRA_CASELESS_RESTRICT],
        );
        // random classes
        let mut rng = Rng::new(0x112);
        for i in 0..nfuzz() {
            let mut s = Vec::new();
            s.push(b'[');
            if rng.below(4) == 0 {
                s.push(b'^');
            }
            let k = 1 + rng.below(6);
            for _ in 0..k {
                match rng.below(10) {
                    0..=3 => s.extend_from_slice(rng.pick(T_LIT)),
                    4..=5 => s.extend_from_slice(rng.pick(T_ESC)),
                    6 => s.extend_from_slice(rng.pick(T_CLASS)),
                    7 => {
                        s.extend_from_slice(rng.pick(T_LIT));
                        s.push(b'-');
                        s.extend_from_slice(rng.pick(T_LIT));
                    }
                    8 => s.extend_from_slice(*rng.pick(&[
                        b"&&".as_ref(),
                        b"--".as_ref(),
                        b"~~".as_ref(),
                        b"||".as_ref(),
                    ])),
                    _ => s.extend_from_slice(&format!("\\x{{{:X}}}", rng.below(0x11000)).into_bytes()),
                }
            }
            s.push(b']');
            if rng.below(3) == 0 {
                s.extend_from_slice(rng.pick(T_QUANT));
            }
            let o = subset(&mut rng, RAND_OPTS, 10);
            d.probe(&format!("rand class #{}", i), &s, s.len(), o, cc.c, cc.r);
        }
        cc.free();
    }
    d.finish();
}

// ============================================================== C113

/// C113 — every POSIX class name, its negation and the bad shapes
/// (pcre2_compile.c:623-660,3935-3942,4089-4101).
#[test]
fn c113_posix_classes() {
    isolate!("C113");
    let mut d = D::new("C113");
    unsafe {
        let cc = Cc::new();
        let names = [
            "alpha", "lower", "upper", "alnum", "ascii", "blank", "cntrl", "digit", "graph",
            "print", "punct", "space", "word", "xdigit",
        ];
        let mut pats: Vec<Vec<u8>> = vec![
            b"[[:nosuch:]]".to_vec(),
            b"[[:alpha:]".to_vec(),
            b"[:alpha:]".to_vec(),
            b"[[.a.]]".to_vec(),
            b"[[=a=]]".to_vec(),
            b"[[:]]".to_vec(),
            b"[[::]]".to_vec(),
            b"[[:^:]]".to_vec(),
            b"[[:alpha]]".to_vec(),
            b"[[:ALPHA:]]".to_vec(),
            b"[[:alpha:][:digit:]]".to_vec(),
            b"[^[:alpha:]]".to_vec(),
            b"[[:word:]-[:digit:]]".to_vec(),
        ];
        for n in names {
            pats.push(format!("[[:{}:]]", n).into_bytes());
            pats.push(format!("[[:^{}:]]", n).into_bytes());
            pats.push(format!("[^[:{}:]]", n).into_bytes());
            pats.push(format!("[a[:{}:]z]", n).into_bytes());
        }
        let refs: Vec<&[u8]> = pats.iter().map(|v| v.as_slice()).collect();
        matrix(
            &mut d,
            &cc,
            &refs,
            &[0, PCRE2_UCP, PCRE2_UTF, PCRE2_UTF | PCRE2_UCP, PCRE2_CASELESS, PCRE2_ALT_EXTENDED_CLASS],
            &[0, PCRE2_EXTRA_ASCII_POSIX, PCRE2_EXTRA_ASCII_DIGIT, PCRE2_EXTRA_ASCII_POSIX | PCRE2_EXTRA_ASCII_DIGIT],
        );
        cc.free();
    }
    d.finish();
}

// ============================================================== C114

/// C114 — every \p / \P property shape, the name normalisation and the _pcre2_utt
/// binary search (pcre2_compile.c:2411-2431).
#[test]
fn c114_properties() {
    isolate!("C114");
    let mut d = D::new("C114");
    unsafe {
        let cc = Cc::new();
        let pats: &[&[u8]] = &[
            b"\\p{L}", b"\\P{L}", b"\\p{Lu}", b"\\p{^Lu}", b"\\pL", b"\\PL", b"\\p{Latin}",
            b"\\p{Script=Latin}", b"\\p{Script_Extensions=Latin}", b"\\p{scx:Latin}",
            b"\\p{sc=Latin}", b"\\p{Any}", b"\\p{Assigned}", b"\\p{Bidi_Class=L}", b"\\p{bc=L}",
            b"\\p{Bidi_Control}", b"\\p{Xan}", b"\\p{Xps}", b"\\p{Xsp}", b"\\p{Xuc}", b"\\p{Xwd}",
            b"\\p", b"\\p{}", b"\\p{NoSuchProperty}", b"\\p{ Greek }", b"\\p{G_r_e_e_k}",
            b"\\p{greek}", b"\\p{GREEK}", b"\\p{Greek-}", b"\\p{Common}", b"\\p{Zzzz}",
            b"\\p{Cn}", b"\\p{C}", b"\\p{N}", b"\\p{Nd}", b"\\p{Nl}", b"\\p{No}",
            b"\\p{Han}", b"\\p{Hani}", b"\\p{sc=Han}", b"\\p{scx=Han}",
            b"\\p{Bidi_Mirrored}", b"\\p{bidim}", b"\\p{Cased}", b"\\p{Case_Ignorable}",
            b"\\p{Changes_When_Lowercased}", b"\\p{ID_Start}", b"\\p{ID_Continue}",
            b"\\p{Emoji}", b"\\p{Emoji_Presentation}", b"\\p{Grapheme_Base}",
            b"\\p{Default_Ignorable_Code_Point}", b"\\p{Alphabetic}", b"\\p{White_Space}",
            b"\\p{gc=L}", b"\\p{general_category=L}", b"\\p{=L}", b"\\p{L=}", b"\\p{L=L}",
            b"[\\p{L}]", b"[\\P{L}]", b"[\\p{^L}]", b"\\p{L}+", b"(?i)\\p{Lu}",
            b"\\p{Lu}\\p{Ll}\\p{Lt}\\p{Lm}\\p{Lo}", b"\\P{Any}", b"\\p{Unknown}",
            b"\\p{scx:Greek}\\p{sc:Greek}", b"\\pC", b"\\p{Cc}", b"\\p{Cf}", b"\\p{Cs}",
            b"\\p{Co}", b"\\p{Zs}", b"\\p{Zl}", b"\\p{Zp}", b"\\p{Pd}", b"\\p{Ps}", b"\\p{Pe}",
            b"\\p{Pi}", b"\\p{Pf}", b"\\p{Pc}", b"\\p{Po}", b"\\p{Sm}", b"\\p{Sc}", b"\\p{Sk}",
            b"\\p{So}", b"\\p{Mn}", b"\\p{Mc}", b"\\p{Me}",
        ];
        matrix(
            &mut d,
            &cc,
            pats,
            &[0, PCRE2_UTF, PCRE2_UCP, PCRE2_UTF | PCRE2_UCP, PCRE2_CASELESS | PCRE2_UTF],
            &[0, PCRE2_EXTRA_CASELESS_RESTRICT],
        );
        // random property names: mostly junk, sometimes real
        let mut rng = Rng::new(0x114);
        let real: &[&str] = &[
            "L", "Lu", "Ll", "N", "Nd", "Greek", "Latin", "Han", "Any", "Assigned", "Xan", "Xwd",
            "Script=Greek", "scx=Greek", "bc=L", "Bidi_Control", "Cased", "Emoji",
        ];
        for i in 0..(nfuzz() / 2).max(60) {
            let name: String = if rng.below(2) == 0 {
                real[rng.below(real.len() as u32) as usize].to_string()
            } else {
                let n = rng.below(8);
                (0..n)
                    .map(|_| *rng.pick(&['a', 'Z', '_', '=', ':', '-', '0', '{', ' ', 'L']))
                    .collect()
            };
            let neg = rng.below(2) == 0;
            let s = format!("\\{}{{{}}}", if neg { 'P' } else { 'p' }, name);
            let o = subset(&mut rng, RAND_OPTS, 12);
            d.probe(&format!("rand prop #{}", i), s.as_bytes(), s.len(), o, cc.c, cc.r);
        }
        cc.free();
    }
    d.finish();
}

// ============================================================== C115

/// C115 — every assertion shape and the alasnames/alasmeta table
/// (pcre2_compile.c:565-610,4762,4883).
#[test]
fn c115_assertions() {
    isolate!("C115");
    let mut d = D::new("C115");
    unsafe {
        let cc = Cc::new();
        let pats: &[&[u8]] = &[
            b"(?=a)",
            b"(?!a)",
            b"(?<=a)",
            b"(?<!a)",
            b"(?*a)",
            b"(*pla:a)",
            b"(?<*a)",
            b"(*plb:a)",
            b"(*nla:a)",
            b"(*nlb:a)",
            b"(*napla:a)",
            b"(*naplb:a)",
            b"(*positive_lookahead:a)",
            b"(*positive_lookbehind:a)",
            b"(*non_atomic_positive_lookahead:a)",
            b"(*non_atomic_positive_lookbehind:a)",
            b"(*negative_lookahead:a)",
            b"(*negative_lookbehind:a)",
            b"(*atomic:a)",
            b"(*sr:\\w+)",
            b"(*asr:\\w+)",
            b"(*script_run:\\w+)",
            b"(*atomic_script_run:\\w+)",
            b"(*scs:(1)a)(x)",
            b"(*scan_substring:(1)a)(x)",
            b"(*nosuchassertion:a)",
            b"(*pla)",
            b"(*pla:)",
            b"(*PLA:a)",
            b"(*scs:a)",
            b"(*scs:(0)a)",
            b"(*scs:(1)(2)a)(x)(y)",
            b"(*scs:(<n>)a)(?<n>x)",
            b"(*sr:)",
            b"(?=)",
            b"(?!)",
            b"(?<=)",
            b"(?<!)",
            b"(?=a)(?!b)(?<=c)(?<!d)",
            b"(?=(?=a))",
            b"(?<=(?<=a))",
            b"(*atomic:(*atomic:a))",
            b"(*sr:(*asr:a))",
            b"(?=a)*",
            b"(?=a)+",
            b"(*plb:a|bb)",
        ];
        matrix(
            &mut d,
            &cc,
            pats,
            &[0, PCRE2_UTF, PCRE2_UCP, PCRE2_CASELESS, PCRE2_AUTO_CALLOUT, PCRE2_NO_AUTO_CAPTURE],
            &[0],
        );
        // random assertion nesting
        let mut rng = Rng::new(0x115);
        let opens: &[&str] = &[
            "(?=", "(?!", "(?<=", "(?<!", "(?*", "(?<*", "(*pla:", "(*plb:", "(*nla:", "(*nlb:",
            "(*napla:", "(*naplb:", "(*atomic:", "(*sr:", "(*asr:", "(*script_run:",
            "(*atomic_script_run:", "(*positive_lookahead:", "(*negative_lookbehind:",
            "(*scs:(1)",
        ];
        for i in 0..nfuzz() {
            let mut s = String::new();
            let k = 1 + rng.below(3);
            for _ in 0..k {
                s.push_str(rng.pick(opens));
            }
            let inner = gen_pattern(&mut rng);
            s.push_str(&String::from_utf8_lossy(&inner).replace('\u{FFFD}', "x"));
            for _ in 0..k {
                s.push(')');
            }
            s.push_str("(x)");
            let o = subset(&mut rng, RAND_OPTS, 12);
            d.probe(&format!("rand assert #{}", i), s.as_bytes(), s.len(), o, cc.c, cc.r);
        }
        cc.free();
    }
    d.finish();
}

// ============================================================== C116

/// C116 — lookbehind length analysis: OP_REVERSE vs OP_VREVERSE and the separate
/// failure modes (pcre2_compile.c:10000-10068).
#[test]
fn c116_lookbehinds() {
    isolate!("C116");
    let mut d = D::new("C116");
    unsafe {
        let cc = Cc::new();
        let mut pats: Vec<Vec<u8>> = vec![
            b"(?<=abc)x".to_vec(),
            b"(?<=ab|cd)x".to_vec(),
            b"(?<=a|bb)x".to_vec(),
            b"(?<=a{2,5})x".to_vec(),
            b"(?<=\\d+)x".to_vec(),
            b"(?<=(a)\\1)x".to_vec(),
            b"(?<=\\C)x".to_vec(),
            b"(?<=a\\C)x".to_vec(),
            b"(?<=^)x".to_vec(),
            b"(?<!)x".to_vec(),
            b"(?<=)x".to_vec(),
            b"(?<=(?<=a)b)x".to_vec(),
            b"(?<=a(?=b))x".to_vec(),
            b"(?<=(?:a|bb)c)x".to_vec(),
            b"(?<=a*)x".to_vec(),
            b"(?<=a?)x".to_vec(),
            b"(?<=\\X)x".to_vec(),
            b"(?<=\\R)x".to_vec(),
            b"(?<=(?R))x".to_vec(),
            b"(?<=(?1))(a)x".to_vec(),
            b"(?<=\\K)x".to_vec(),
            b"(?<=(*ACCEPT))x".to_vec(),
            b"(?<=[a-z])x".to_vec(),
            b"(?<=\\p{L})x".to_vec(),
            b"(?<=a|b|c|dd|eee)x".to_vec(),
            b"(?<*a|bb)x".to_vec(),
            b"(*nlb:a|bbb)x".to_vec(),
            b"(?<=\\x{100})x".to_vec(),
            b"(?<=(?|a|bb))x".to_vec(),
            b"(?<=a{0,255})x".to_vec(),
        ];
        for n in [1usize, 254, 255, 256, 65534, 65535, 65536] {
            pats.push(format!("(?<=a{{{}}})x", n).into_bytes());
            pats.push(format!("(?<=a{{0,{}}})x", n).into_bytes());
        }
        for n in [1usize, 100, 255, 256] {
            pats.push(format!("(?<={})x", "a".repeat(n)).into_bytes());
        }
        let refs: Vec<&[u8]> = pats.iter().map(|v| v.as_slice()).collect();
        matrix(
            &mut d,
            &cc,
            &refs,
            &[0, PCRE2_UTF, PCRE2_UCP, PCRE2_CASELESS, PCRE2_MULTILINE, PCRE2_AUTO_CALLOUT],
            &[0, PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK],
        );
        // random lookbehind bodies
        let mut rng = Rng::new(0x116);
        for i in 0..nfuzz() {
            let inner = gen_pattern(&mut rng);
            let mut s = Vec::new();
            s.extend_from_slice(*rng.pick(&[
                b"(?<=".as_ref(),
                b"(?<!".as_ref(),
                b"(?<*".as_ref(),
                b"(*nlb:".as_ref(),
                b"(*naplb:".as_ref(),
            ]));
            s.extend_from_slice(&inner);
            s.extend_from_slice(b")x");
            let o = subset(&mut rng, RAND_OPTS, 12);
            d.probe(&format!("rand lookbehind #{}", i), &s, s.len(), o, cc.c, cc.r);
        }
        cc.free();
    }
    d.finish();
}

// ============================================================== C117

/// C117 — every conditional-group condition kind: OP_CREF, OP_DNCREF, OP_RREF,
/// OP_DNRREF, OP_FALSE, OP_TRUE and inline assertions with GF_CONDASSERT.
#[test]
fn c117_conditionals() {
    isolate!("C117");
    let mut d = D::new("C117");
    unsafe {
        let cc = Cc::new();
        let pats: &[&[u8]] = &[
            b"(?(1)a|b)",
            b"(a)(?(1)a|b)",
            b"(?(1)a|b)(a)",
            b"(?(<n>)a|b)",
            b"(?<n>x)(?(<n>)a|b)",
            b"(?('n')a|b)",
            b"(?<n>x)(?('n')a|b)",
            b"(?(n)a|b)",
            b"(?<n>x)(?(n)a|b)",
            b"(?(R)a|b)",
            b"(?(R1)a|b)",
            b"(a)(?(R1)a|b)",
            b"(?(R&n)a|b)",
            b"(?<n>x)(?(R&n)a|b)",
            b"(?(DEFINE)(?<x>a))(?&x)",
            b"(?(DEFINE)(?<x>a))",
            b"(?(DEFINE)a|b)",
            b"(?(VERSION>=10.0)a|b)",
            b"(?(VERSION<10.0)a|b)",
            b"(?(VERSION=10.48)a|b)",
            b"(?(VERSION>=99.0)a|b)",
            b"(?(VERSION>=x)a|b)",
            b"(?(VERSION)a|b)",
            b"(?(?=a)b|c)",
            b"(?(?<=a)b|c)",
            b"(?(?!a)b|c)",
            b"(?(?<!a)b|c)",
            b"(?(1)a|b|c)",
            b"(?()a)",
            b"(?(0)a|b)",
            b"(?(99)a|b)",
            b"(?(-1)a|b)(a)",
            b"(a)(?(-1)a|b)",
            b"(a)(?(+1)a|b)(b)",
            b"(?(1)a)",
            b"(?(1))",
            b"(?(?=a))",
            b"(?(1)(?(2)x|y)|z)(a)(b)",
            b"(?<n>a)(?J)(?<n>b)(?(<n>)x|y)",
            b"(?(R)(?R)|a)",
            b"(?(assert)a|b)",
            b"(?(?C1)a|b)",
        ];
        matrix(
            &mut d,
            &cc,
            pats,
            &[0, PCRE2_DUPNAMES, PCRE2_NO_AUTO_CAPTURE, PCRE2_UTF, PCRE2_AUTO_CALLOUT],
            &[0],
        );
        // random conditions
        let mut rng = Rng::new(0x117);
        for i in 0..nfuzz() {
            let cond = *rng.pick(&[
                "1", "2", "0", "-1", "+1", "<n>", "'n'", "n", "R", "R1", "R&n", "DEFINE",
                "VERSION>=10.0", "VERSION<10.0", "?=a", "?!a", "?<=a", "?<!a", "", "99",
            ]);
            let body = gen_pattern(&mut rng);
            let mut s = format!("(?({})", cond);
            s.push_str(&String::from_utf8_lossy(&body).replace('\u{FFFD}', "x"));
            if rng.below(2) == 0 {
                s.push_str("|b");
            }
            s.push(')');
            if rng.below(2) == 0 {
                s.push_str("(?<n>a)(b)");
            }
            let o = subset(&mut rng, RAND_OPTS, 12);
            d.probe(&format!("rand cond #{}", i), s.as_bytes(), s.len(), o, cc.c, cc.r);
        }
        cc.free();
    }
    d.finish();
}

// ============================================================== C118

/// C118 — numeric and string callouts, every delimiter pair from
/// _pcre2_callout_start_delims / _end_delims (pcre2_compile.c:5288-5330,
/// pcre2_tables.c:84-92).
#[test]
fn c118_callouts() {
    isolate!("C118");
    let mut d = D::new("C118");
    unsafe {
        let cc = Cc::new();
        let mut pats: Vec<Vec<u8>> = vec![
            b"(?C)".to_vec(),
            b"(?C0)".to_vec(),
            b"(?C1)".to_vec(),
            b"(?C255)".to_vec(),
            b"(?C256)".to_vec(),
            b"(?C1000)".to_vec(),
            b"(?C{x})".to_vec(),
            b"(?C'x'".to_vec(),
            b"(?C{})".to_vec(),
            b"(?C".to_vec(),
            b"(?C1".to_vec(),
            b"(?C{x}".to_vec(),
            b"(?C*x*)".to_vec(),
            b"(?C(x))".to_vec(),
            b"(?C[x])".to_vec(),
            b"(?C<x>)".to_vec(),
            b"(?C1x)".to_vec(),
            b"(?C-1)".to_vec(),
            b"(?C 1)".to_vec(),
            b"(?C{x}y)".to_vec(),
            b"a(?C1)b(?C2)c".to_vec(),
            b"(?C1)(?C2)(?C3)".to_vec(),
        ];
        // the nine start delimiters with their matching ends
        let delims: &[(char, char)] = &[
            ('`', '`'),
            ('\'', '\''),
            ('"', '"'),
            ('^', '^'),
            ('%', '%'),
            ('#', '#'),
            ('$', '$'),
            ('{', '}'),
        ];
        for &(s, e) in delims {
            pats.push(format!("(?C{}str{})", s, e).into_bytes());
            pats.push(format!("(?C{}{})", s, e).into_bytes());
            // doubled delimiter inside the string means a literal delimiter
            pats.push(format!("(?C{}a{}{}b{})", s, s, s, e).into_bytes());
            pats.push(format!("(?C{}a{}", s, e).into_bytes());
            pats.push(format!("(?C{}unterminated", s).into_bytes());
            pats.push(format!("(?C{}a\nb{})", s, e).into_bytes());
        }
        // a very long callout string
        for n in [10usize, 1000, 60000, 70000] {
            pats.push(format!("(?C{{{}}})", "x".repeat(n)).into_bytes());
        }
        let refs: Vec<&[u8]> = pats.iter().map(|v| v.as_slice()).collect();
        matrix(
            &mut d,
            &cc,
            &refs,
            &[0, PCRE2_AUTO_CALLOUT, PCRE2_UTF, PCRE2_EXTENDED],
            &[0, PCRE2_EXTRA_NEVER_CALLOUT],
        );
        // random callouts
        let mut rng = Rng::new(0x118);
        for i in 0..(nfuzz() / 2).max(60) {
            let mut s = String::new();
            let k = 1 + rng.below(3);
            for _ in 0..k {
                match rng.below(3) {
                    0 => s.push_str(&format!("(?C{})", rng.below(300))),
                    1 => {
                        let (o, c) = delims[rng.below(delims.len() as u32) as usize];
                        let body: String = (0..rng.below(6))
                            .map(|_| *rng.pick(&['a', 'b', '}', '{', '\'', '"', '`', '^', '%', '#', '$', '\\']))
                            .collect();
                        s.push_str(&format!("(?C{}{}{})", o, body, c));
                    }
                    _ => s.push('a'),
                }
            }
            let o = subset(&mut rng, RAND_OPTS, 12);
            d.probe(&format!("rand callout #{}", i), s.as_bytes(), s.len(), o, cc.c, cc.r);
        }
        cc.free();
    }
    d.finish();
}

// ============================================================== C119

/// C119 — every verb, with and without an argument, and in every kind of enclosing
/// group (pcre2_compile.c:527-560).
#[test]
fn c119_verbs() {
    isolate!("C119");
    let mut d = D::new("C119");
    unsafe {
        let cc = Cc::new();
        let verbs: &[&str] = &[
            "(*MARK:a)", "(*:a)", "(*ACCEPT)", "(*ACCEPT:a)", "(*F)", "(*FAIL)", "(*FAIL:a)",
            "(*COMMIT)", "(*COMMIT:a)", "(*PRUNE)", "(*PRUNE:a)", "(*SKIP)", "(*SKIP:a)",
            "(*THEN)", "(*THEN:a)", "(*MARK)", "(*:)",
        ];
        let wrappers: &[(&str, &str)] = &[
            ("", ""),
            ("(", ")"),
            ("(?:", ")"),
            ("(?>", ")"),
            ("(?=", ")"),
            ("(?!", ")"),
            ("(?<=a", ")"),
            ("(?<!a", ")"),
            ("(*atomic:", ")"),
            ("(*sr:", ")"),
            ("(?|", ")"),
            ("(?i:", ")"),
            ("(?(1)", ")(a)"),
        ];
        for v in verbs {
            for &(o, c) in wrappers {
                let p = format!("{}{}{}", o, v, c);
                d.probe("verb", p.as_bytes(), p.len(), 0, cc.c, cc.r);
                let q = format!("a{}{}{}b", o, v, c);
                d.probe("verb in context", q.as_bytes(), q.len(), 0, cc.c, cc.r);
            }
            // inside a recursed group
            let r = format!("(a{})(?1)", v);
            d.probe("verb in recursed group", r.as_bytes(), r.len(), 0, cc.c, cc.r);
            // quantified
            let s = format!("(?:{})*", v);
            d.probe("quantified verb", s.as_bytes(), s.len(), 0, cc.c, cc.r);
        }
        for &o in &[PCRE2_ALT_VERBNAMES, PCRE2_AUTO_CALLOUT, PCRE2_EXTENDED, PCRE2_UTF] {
            for v in verbs {
                d.probe(&format!("opts=0x{:x}", o), v.as_bytes(), v.len(), o, cc.c, cc.r);
            }
        }
        fuzz(&mut d, &cc, 0x119, nfuzz(), &[PCRE2_ALT_VERBNAMES], &[]);
        cc.free();
    }
    d.finish();
}

// ============================================================== C120

/// C120 — the whole pso_list start-of-pattern scan (pcre2_compile.c:741-763,
/// 10497-10597): each setting alone, in combination and preceded by another.
#[test]
fn c120_start_of_pattern_settings() {
    isolate!("C120");
    let mut d = D::new("C120");
    unsafe {
        let cc = Cc::new();
        let psos: &[&str] = &[
            "(*UTF)",
            "(*UTF8)",
            "(*UCP)",
            "(*NOTEMPTY)",
            "(*NOTEMPTY_ATSTART)",
            "(*NO_AUTO_POSSESS)",
            "(*NO_DOTSTAR_ANCHOR)",
            "(*NO_JIT)",
            "(*NO_START_OPT)",
            "(*CASELESS_RESTRICT)",
            "(*TURKISH_CASING)",
            "(*LIMIT_HEAP=1000)",
            "(*LIMIT_MATCH=1000)",
            "(*LIMIT_DEPTH=1000)",
            "(*LIMIT_RECURSION=1000)",
            "(*CR)",
            "(*LF)",
            "(*CRLF)",
            "(*ANY)",
            "(*NUL)",
            "(*ANYCRLF)",
            "(*BSR_ANYCRLF)",
            "(*BSR_UNICODE)",
        ];
        let odd: &[&str] = &[
            "(*",
            "(*)",
            "(*FOO)",
            "(*UTF",
            "(*utf)",
            "(*LIMIT_MATCH=)",
            "(*LIMIT_MATCH=x)",
            "(*LIMIT_MATCH=99999999999999999999)",
            "(*LIMIT_MATCH=4294967295)",
            "(*LIMIT_MATCH=4294967296)",
            "(*LIMIT_HEAP=0)",
            "(*LIMIT_DEPTH=0)",
            "(*LIMIT_RECURSION=)",
            "(*UTF)(*",
            "(*NUL)(*CR)",
            "(*ANY)(*ANYCRLF)",
            "(*BSR_UNICODE)(*BSR_ANYCRLF)",
            "(*UTF)x(*UCP)",
            "(*MARK:x)(*UTF)",
        ];
        // each alone (and with a trailing 'a' so there is something to compile)
        for s in psos.iter().chain(odd.iter()) {
            for suffix in ["", "a", "\\d", "\u{e9}"] {
                let p = format!("{}{}", s, suffix);
                for &o in &[0u32, PCRE2_UTF, PCRE2_NEVER_UTF, PCRE2_NEVER_UCP, PCRE2_CASELESS] {
                    d.probe("pso alone", p.as_bytes(), p.len(), o, cc.c, cc.r);
                }
            }
        }
        // every ordered pair
        for a in psos {
            for b in psos {
                let p = format!("{}{}x", a, b);
                d.probe("pso pair", p.as_bytes(), p.len(), 0, cc.c, cc.r);
            }
        }
        // all of them at once (in table order and reversed)
        let all: String = psos.concat() + "x";
        d.probe("all psos", all.as_bytes(), all.len(), 0, cc.c, cc.r);
        let mut rev: Vec<&str> = psos.to_vec();
        rev.reverse();
        let allr: String = rev.concat() + "x";
        d.probe("all psos reversed", allr.as_bytes(), allr.len(), 0, cc.c, cc.r);
        // random sequences of settings in front of random patterns
        let mut rng = Rng::new(0x120);
        for i in 0..(nfuzz() / 2).max(60) {
            let k = rng.below(5);
            let mut s = String::new();
            for _ in 0..k {
                s.push_str(if rng.below(8) == 0 { rng.pick(odd) } else { rng.pick(psos) });
            }
            let body = gen_pattern(&mut rng);
            s.push_str(&String::from_utf8_lossy(&body).replace('\u{FFFD}', "x"));
            let o = subset(&mut rng, RAND_OPTS, 14);
            d.probe(&format!("rand pso #{}", i), s.as_bytes(), s.len(), o, cc.c, cc.r);
        }
        cc.free();
    }
    d.finish();
}

// ============================================================== C121

/// C121 — the inline option parser: the ^ reset, the hyphen-once rule, the five
/// two-character 'a' sequences, the doubled-x rule and option restoration on group exit
/// (pcre2_compile.c:5036-5140).
#[test]
fn c121_inline_options() {
    isolate!("C121");
    let mut d = D::new("C121");
    unsafe {
        let cc = Cc::new();
        let pats: &[&[u8]] = &[
            b"(?i)a", b"(?i:a)", b"(?-i)a", b"(?i-m)a", b"(?^)a", b"(?^i)a", b"(?imnsxrUJ)a",
            b"(?xx)a", b"(?aD)a", b"(?aP)a", b"(?aS)a", b"(?aT)a", b"(?aW)a", b"(?a)a",
            b"(?i-)a", b"(?q)a", b"(?i", b"(?", b"(?)a", b"(?-)a", b"(?--i)a", b"(?i-m-s)a",
            b"(?^-i)a", b"(?^^)a", b"(?aa)a", b"(?aX)a", b"(?a-D)a", b"(?ax)a",
            b"(?i)(?-i)a", b"(?i:(?-i:a))a", b"((?i)a)a", b"(?:(?i)a)a", b"(?i)a(?-i)a",
            b"(?x)(?xx)a b(?-xx)a b", b"(?J)(?<n>a)(?<n>b)", b"(?n)(a)", b"(?U)a*",
            b"(?r)a", b"(?-r)a", b"(?i-mnsxrUJ)a", b"(?^imnsxrUJ)a", b"(?^aD)a",
            b"(?i)[a]", b"(?i)\\p{L}", b"(?-)a", b"(? i)a", b"(?i )a", b"(?i-x:a)b",
            b"(?|(?i)a|b)", b"(?>(?i)a)b", b"(?=(?i)a)b", b"(?i)(?J)(?<n>a)(?<n>b)",
        ];
        matrix(
            &mut d,
            &cc,
            pats,
            &[
                0,
                PCRE2_CASELESS,
                PCRE2_EXTENDED,
                PCRE2_EXTENDED_MORE,
                PCRE2_MULTILINE,
                PCRE2_DOTALL,
                PCRE2_UCP,
                PCRE2_UTF,
                PCRE2_DUPNAMES,
                PCRE2_NO_AUTO_CAPTURE,
                PCRE2_UNGREEDY,
            ],
            &[0, PCRE2_EXTRA_CASELESS_RESTRICT],
        );
        // random inline option strings
        let mut rng = Rng::new(0x121);
        let letters = ['i', 'm', 'n', 's', 'x', 'r', 'U', 'J', 'a', 'D', 'P', 'S', 'T', 'W', '^', '-', 'q', ':'];
        for i in 0..nfuzz() {
            let k = 1 + rng.below(5);
            let body: String = (0..k).map(|_| *rng.pick(&letters)).collect();
            let mut s = format!("(?{})", body);
            if rng.below(2) == 0 {
                s = format!("(?{}:a)", body);
            }
            s.push_str("a b");
            let o = subset(&mut rng, RAND_OPTS, 12);
            d.probe(&format!("rand inline #{} (?{})", i, body), s.as_bytes(), s.len(), o, cc.c, cc.r);
        }
        cc.free();
    }
    d.finish();
}

// ============================================================== C122

/// C122 — branch reset (?| : group renumbering across alternatives, the name table and
/// backreference resolution.
#[test]
fn c122_branch_reset() {
    isolate!("C122");
    let mut d = D::new("C122");
    unsafe {
        let cc = Cc::new();
        let mut pats: Vec<Vec<u8>> = vec![
            b"(?|(a)|(b))".to_vec(),
            b"(?|(a)(b)|(c))".to_vec(),
            b"(?|(a)|(b)(c))".to_vec(),
            b"(?|(?<n>a)|(?<n>b))".to_vec(),
            b"(?|(a))\\1".to_vec(),
            b"(?|(a)|(b))\\1".to_vec(),
            b"(?|(a)(b)|(c))\\2".to_vec(),
            b"(x)(?|(a)|(b))\\1\\2".to_vec(),
            b"(?|a|b)".to_vec(),
            b"(?|)".to_vec(),
            b"(?|(?<n>a)|(?<m>b))".to_vec(),
            b"(?|(a)|(?<n>b))".to_vec(),
            b"(?|(?|(a)|(b))|(c))".to_vec(),
            b"(?|(a)|(b))(?|(c)|(d))\\1\\2".to_vec(),
            b"(?|(a)|(b))(?1)".to_vec(),
            b"(?|(?<n>a)|(?<n>b))(?&n)".to_vec(),
            b"(?|(a)|(b))(?(1)x|y)".to_vec(),
        ];
        for depth in [2usize, 10, 100, 190, 199, 200, 201, 250, 251] {
            let mut s = String::new();
            for _ in 0..depth {
                s.push_str("(?|");
            }
            s.push('a');
            for _ in 0..depth {
                s.push(')');
            }
            pats.push(s.into_bytes());
        }
        let refs: Vec<&[u8]> = pats.iter().map(|v| v.as_slice()).collect();
        matrix(
            &mut d,
            &cc,
            &refs,
            &[0, PCRE2_DUPNAMES, PCRE2_NO_AUTO_CAPTURE, PCRE2_UTF, PCRE2_MATCH_UNSET_BACKREF],
            &[0],
        );
        // random branch-reset groups with random capture/backref mixes
        let mut rng = Rng::new(0x122);
        for i in 0..nfuzz() {
            let nalt = 1 + rng.below(4);
            let mut s = String::from("(?|");
            for a in 0..nalt {
                if a > 0 {
                    s.push('|');
                }
                let ng = rng.below(4);
                for g in 0..ng {
                    match rng.below(3) {
                        0 => s.push_str("(a)"),
                        1 => s.push_str(&format!("(?<n{}>b)", g)),
                        _ => s.push_str("(?:c)"),
                    }
                }
            }
            s.push(')');
            let k = rng.below(3);
            for _ in 0..k {
                s.push_str(&format!("\\{}", 1 + rng.below(4)));
            }
            let o = if rng.below(2) == 0 { PCRE2_DUPNAMES } else { 0 };
            d.probe(&format!("rand branch-reset #{}", i), s.as_bytes(), s.len(), o, cc.c, cc.r);
        }
        cc.free();
    }
    d.finish();
}

// ============================================================== C123

/// Compare two already-compiled code blocks (one per library) in full.
unsafe fn cmp_codes(d: &mut D, label: &str, code_c: Ptr, code_r: Ptr) {
    let (c, r) = both();
    if code_c.is_null() || code_r.is_null() {
        d.cases += 1;
        if code_c.is_null() != code_r.is_null() {
            d.record(
                "null-mismatch".to_string(),
                format!("{}: C null={} RUST null={}", label, code_c.is_null(), code_r.is_null()),
            );
        }
        return;
    }
    let a = snapshot(c, code_c, 0, 0);
    let b = snapshot(r, code_r, 0, 0);
    d.add(label, b"<compiled code block>", 0, 0, &a, &b);
}

/// C123 — pcre2_code_copy / pcre2_code_copy_with_tables with the default tables, with a
/// pcre2_maketables result installed in the compile context, and on a code that has
/// PCRE2_DEREF_TABLES set (obtained by serialize/deserialize)
/// (pcre2_compile.c:1131-1195, pcre2_internal.h:520).
#[test]
fn c123_code_copy() {
    isolate!("C123");
    let mut d = D::new("C123");
    unsafe {
        let (c, r) = both();

        // pcre2_maketables must produce byte-identical tables in both libraries
        let mut tlen: u32 = 0;
        assert_eq!(
            (c.pcre2_config_8)(PCRE2_CONFIG_TABLES_LENGTH, &mut tlen as *mut u32 as Ptr),
            (r.pcre2_config_8)(PCRE2_CONFIG_TABLES_LENGTH, &mut tlen as *mut u32 as Ptr)
        );
        assert!(tlen > 0);
        let tab_c = (c.pcre2_maketables_8)(ptr::null_mut());
        let tab_r = (r.pcre2_maketables_8)(ptr::null_mut());
        assert!(!tab_c.is_null() && !tab_r.is_null());
        d.cases += 1;
        let sc = std::slice::from_raw_parts(tab_c, tlen as usize);
        let sr = std::slice::from_raw_parts(tab_r, tlen as usize);
        if sc != sr {
            let i = sc.iter().zip(sr.iter()).position(|(x, y)| x != y).unwrap();
            d.record(
                "maketables".to_string(),
                format!("maketables differ at byte {}: C=0x{:02x} RUST=0x{:02x}", i, sc[i], sr[i]),
            );
        }

        let pats: &[(&[u8], u32)] = &[
            (b"a", 0),
            (b"(?<name>a)(?<other>b)\\k<name>[a-f0-9]+", 0),
            (b"(?<n>x)[\\x{100}-\\x{200}\\x{2000}-\\x{3000}]\\k<n>", PCRE2_UTF),
            (b"(?i)[[:alpha:]]\\d{2,5}(?:x|y)", 0),
            (b"\\p{Greek}+(?<g>\\w)\\k<g>", PCRE2_UTF | PCRE2_UCP),
        ];

        for &(p, opts) in pats {
            for use_tables in [false, true] {
                let ccx = Cc::new();
                if use_tables {
                    assert_eq!(
                        (c.pcre2_set_character_tables_8)(ccx.c, tab_c),
                        (r.pcre2_set_character_tables_8)(ccx.r, tab_r)
                    );
                }
                let mut ec = 0i32;
                let mut eo = 0usize;
                let code_c =
                    (c.pcre2_compile_8)(p.as_ptr(), p.len(), opts, &mut ec, &mut eo, ccx.c);
                let code_r =
                    (r.pcre2_compile_8)(p.as_ptr(), p.len(), opts, &mut ec, &mut eo, ccx.r);
                let label = format!("pattern \"{}\" tables={}", esc(p), use_tables);
                cmp_codes(&mut d, &format!("{} original", label), code_c, code_r);
                assert!(!code_c.is_null() && !code_r.is_null(), "C123: {} failed to compile", label);

                let cp_c = (c.pcre2_code_copy_8)(code_c);
                let cp_r = (r.pcre2_code_copy_8)(code_r);
                cmp_codes(&mut d, &format!("{} code_copy", label), cp_c, cp_r);

                let wt_c = (c.pcre2_code_copy_with_tables_8)(code_c);
                let wt_r = (r.pcre2_code_copy_with_tables_8)(code_r);
                cmp_codes(&mut d, &format!("{} code_copy_with_tables", label), wt_c, wt_r);

                // a copy of a copy, and a with_tables copy of a plain copy
                let cp2_c = (c.pcre2_code_copy_8)(cp_c);
                let cp2_r = (r.pcre2_code_copy_8)(cp_r);
                cmp_codes(&mut d, &format!("{} copy of copy", label), cp2_c, cp2_r);
                let wt2_c = (c.pcre2_code_copy_with_tables_8)(wt_c);
                let wt2_r = (r.pcre2_code_copy_with_tables_8)(wt_r);
                cmp_codes(&mut d, &format!("{} with_tables of with_tables", label), wt2_c, wt2_r);

                // free the ORIGINAL first: the copies must still be fully usable
                (c.pcre2_code_free_8)(code_c);
                (r.pcre2_code_free_8)(code_r);
                cmp_codes(&mut d, &format!("{} code_copy after original freed", label), cp_c, cp_r);
                cmp_codes(
                    &mut d,
                    &format!("{} with_tables after original freed", label),
                    wt_c,
                    wt_r,
                );
                (c.pcre2_code_free_8)(cp_c);
                (r.pcre2_code_free_8)(cp_r);
                cmp_codes(&mut d, &format!("{} copy2 after copy freed", label), cp2_c, cp2_r);
                (c.pcre2_code_free_8)(wt_c);
                (r.pcre2_code_free_8)(wt_r);
                cmp_codes(&mut d, &format!("{} wt2 after wt freed", label), wt2_c, wt2_r);
                (c.pcre2_code_free_8)(cp2_c);
                (r.pcre2_code_free_8)(cp2_r);
                (c.pcre2_code_free_8)(wt2_c);
                (r.pcre2_code_free_8)(wt2_r);
                ccx.free();
            }

            // a code with PCRE2_DEREF_TABLES set: serialize then deserialize, then copy
            let mut ec = 0i32;
            let mut eo = 0usize;
            let code_c = (c.pcre2_compile_8)(p.as_ptr(), p.len(), opts, &mut ec, &mut eo, ptr::null_mut());
            let code_r = (r.pcre2_compile_8)(p.as_ptr(), p.len(), opts, &mut ec, &mut eo, ptr::null_mut());
            assert!(!code_c.is_null() && !code_r.is_null());
            let mut by_c: *mut u8 = ptr::null_mut();
            let mut by_r: *mut u8 = ptr::null_mut();
            let mut sz_c = 0usize;
            let mut sz_r = 0usize;
            let list_c = [code_c];
            let list_r = [code_r];
            let rc_c = (c.pcre2_serialize_encode_8)(
                list_c.as_ptr() as *const Ptr,
                1,
                &mut by_c,
                &mut sz_c,
                ptr::null_mut(),
            );
            let rc_r = (r.pcre2_serialize_encode_8)(
                list_r.as_ptr() as *const Ptr,
                1,
                &mut by_r,
                &mut sz_r,
                ptr::null_mut(),
            );
            assert_eq!(rc_c, rc_r);
            assert_eq!(sz_c, sz_r);
            let mut dec_c: [Ptr; 1] = [ptr::null_mut()];
            let mut dec_r: [Ptr; 1] = [ptr::null_mut()];
            let dc = (c.pcre2_serialize_decode_8)(dec_c.as_mut_ptr(), 1, by_c, ptr::null_mut());
            let dr = (r.pcre2_serialize_decode_8)(dec_r.as_mut_ptr(), 1, by_r, ptr::null_mut());
            assert_eq!(dc, dr);
            cmp_codes(&mut d, "deserialized (DEREF_TABLES)", dec_c[0], dec_r[0]);
            let dcp_c = (c.pcre2_code_copy_8)(dec_c[0]);
            let dcp_r = (r.pcre2_code_copy_8)(dec_r[0]);
            cmp_codes(&mut d, "code_copy of DEREF_TABLES code", dcp_c, dcp_r);
            let dwt_c = (c.pcre2_code_copy_with_tables_8)(dec_c[0]);
            let dwt_r = (r.pcre2_code_copy_with_tables_8)(dec_r[0]);
            cmp_codes(&mut d, "code_copy_with_tables of DEREF_TABLES code", dwt_c, dwt_r);
            // free the deserialized original (drops one table reference) and use the copy
            (c.pcre2_code_free_8)(dec_c[0]);
            (r.pcre2_code_free_8)(dec_r[0]);
            cmp_codes(&mut d, "DEREF_TABLES copy after original freed", dcp_c, dcp_r);
            cmp_codes(&mut d, "DEREF_TABLES with_tables after original freed", dwt_c, dwt_r);
            (c.pcre2_code_free_8)(dcp_c);
            (r.pcre2_code_free_8)(dcp_r);
            (c.pcre2_code_free_8)(dwt_c);
            (r.pcre2_code_free_8)(dwt_r);
            (c.pcre2_serialize_free_8)(by_c);
            (r.pcre2_serialize_free_8)(by_r);
            (c.pcre2_code_free_8)(code_c);
            (r.pcre2_code_free_8)(code_r);
        }

        // NULL arguments
        d.cases += 1;
        let n1 = (c.pcre2_code_copy_8)(ptr::null_mut());
        let n2 = (r.pcre2_code_copy_8)(ptr::null_mut());
        let n3 = (c.pcre2_code_copy_with_tables_8)(ptr::null_mut());
        let n4 = (r.pcre2_code_copy_with_tables_8)(ptr::null_mut());
        if !(n1.is_null() && n2.is_null() && n3.is_null() && n4.is_null()) {
            d.record(
                "copy-NULL".to_string(),
                format!(
                    "code_copy(NULL)/copy_with_tables(NULL) must be NULL: {} {} {} {}",
                    n1.is_null(),
                    n2.is_null(),
                    n3.is_null(),
                    n4.is_null()
                ),
            );
        }
        // pcre2_code_free(NULL) is a no-op in both
        (c.pcre2_code_free_8)(ptr::null_mut());
        (r.pcre2_code_free_8)(ptr::null_mut());

        // random patterns: compile, copy both ways, compare, free in a random order
        let ccx = Cc::new();
        let mut rng = Rng::new(0x123);
        for i in 0..(nfuzz() / 8).max(60) {
            let pat = gen_pattern(&mut rng);
            let opts = subset(&mut rng, RAND_OPTS, 14);
            let opts = utf_safe_opts(&pat, pat.len(), opts);
            let use_tables = rng.below(2) == 0;
            if use_tables {
                (c.pcre2_set_character_tables_8)(ccx.c, tab_c);
                (r.pcre2_set_character_tables_8)(ccx.r, tab_r);
            } else {
                (c.pcre2_set_character_tables_8)(ccx.c, ptr::null());
                (r.pcre2_set_character_tables_8)(ccx.r, ptr::null());
            }
            let mut ec = 0i32;
            let mut eo = 0usize;
            let a = (c.pcre2_compile_8)(pat.as_ptr(), pat.len(), opts, &mut ec, &mut eo, ccx.c);
            let b = (r.pcre2_compile_8)(pat.as_ptr(), pat.len(), opts, &mut ec, &mut eo, ccx.r);
            if a.is_null() || b.is_null() {
                d.cases += 1;
                if a.is_null() != b.is_null() {
                    d.record(
                        "rand-compile-mismatch".to_string(),
                        format!("pattern \"{}\" opts 0x{:x}", esc(&pat), opts),
                    );
                }
                if !a.is_null() {
                    (c.pcre2_code_free_8)(a);
                }
                if !b.is_null() {
                    (r.pcre2_code_free_8)(b);
                }
                continue;
            }
            let (x, y) = if rng.below(2) == 0 {
                ((c.pcre2_code_copy_8)(a), (r.pcre2_code_copy_8)(b))
            } else {
                ((c.pcre2_code_copy_with_tables_8)(a), (r.pcre2_code_copy_with_tables_8)(b))
            };
            cmp_codes(&mut d, &format!("rand copy #{} \"{}\"", i, esc(&pat)), x, y);
            if rng.below(2) == 0 {
                (c.pcre2_code_free_8)(a);
                (r.pcre2_code_free_8)(b);
                cmp_codes(&mut d, &format!("rand copy #{} after original freed", i), x, y);
                (c.pcre2_code_free_8)(x);
                (r.pcre2_code_free_8)(y);
            } else {
                (c.pcre2_code_free_8)(x);
                (r.pcre2_code_free_8)(y);
                (c.pcre2_code_free_8)(a);
                (r.pcre2_code_free_8)(b);
            }
        }
        ccx.free();
        (c.pcre2_maketables_free_8)(ptr::null_mut(), tab_c);
        (r.pcre2_maketables_free_8)(ptr::null_mut(), tab_r);
    }
    d.finish();
}

// ====================================================== C124 / C125 helpers

/// A private allocator so the "non-NULL general context" path is exercised. Both
/// libraries call these same functions; every block is released through `my_free`.
unsafe extern "C" fn my_malloc(size: usize, _data: Ptr) -> Ptr {
    let total = size + 16;
    let layout = std::alloc::Layout::from_size_align(total, 16).unwrap();
    let p = std::alloc::alloc(layout);
    if p.is_null() {
        return ptr::null_mut();
    }
    *(p as *mut usize) = total;
    p.add(16) as Ptr
}

unsafe extern "C" fn my_free(p: Ptr, _data: Ptr) {
    if p.is_null() {
        return;
    }
    let base = (p as *mut u8).sub(16);
    let total = *(base as *mut usize);
    let layout = std::alloc::Layout::from_size_align(total, 16).unwrap();
    std::alloc::dealloc(base, layout);
}

/// `pcre2_general_context_create` allocates the context itself with the supplied
/// malloc, so a malloc that fails from the start cannot produce a context at all.
/// This one starts working and is switched off with `FAIL_MALLOC` once both contexts
/// exist. (Only C124/C125 use it and each runs in its own process.)
static FAIL_MALLOC: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

unsafe extern "C" fn failing_malloc(size: usize, data: Ptr) -> Ptr {
    if FAIL_MALLOC.load(std::sync::atomic::Ordering::SeqCst) {
        ptr::null_mut()
    } else {
        my_malloc(size, data)
    }
}

/// The defined part of a match data block after a match call.
///
/// * the ovector is only defined for the first `2*rc` slots on success (`rc > 0`),
///   for every slot when `rc == 0` (ovector too small) and for the first pair on a
///   partial match; the rest is uninitialised memory in BOTH libraries;
/// * `startchar` is always set by `pcre2_match` (pcre2_match.c:7136) but only on a
///   match by `pcre2_dfa_match` (pcre2_dfa_match.c:4058);
/// * `mark` is always set by both (pcre2_match.c:8169,8211, pcre2_dfa_match.c:3690).
#[derive(Debug, PartialEq, Eq)]
struct MState {
    rc: i32,
    ovector: Vec<usize>,
    startchar: Option<usize>,
    mark: Option<Vec<u8>>,
}

unsafe fn mstate(api: &Api, md: Ptr, rc: i32, dfa: bool) -> MState {
    let count = (api.pcre2_get_ovector_count_8)(md) as usize;
    let ov = (api.pcre2_get_ovector_pointer_8)(md);
    let n = if rc > 0 {
        (2 * rc as usize).min(2 * count)
    } else if rc == 0 {
        2 * count
    } else if rc == PCRE2_ERROR_PARTIAL {
        (2).min(2 * count)
    } else {
        0
    };
    MState {
        rc,
        ovector: (0..n).map(|i| *ov.add(i)).collect(),
        startchar: if !dfa || rc >= 0 {
            Some((api.pcre2_get_startchar_8)(md))
        } else {
            None
        },
        mark: read_mark(api, md),
    }
}

/// Everything observable about a fresh match data block. `startchar` and `mark` are
/// deliberately NOT read before a match: pcre2_match_data.c:53-68 leaves them
/// uninitialised, so their pre-match values are not defined behaviour to compare.
#[derive(Debug, PartialEq, Eq)]
struct MdOut {
    ovector_count: u32,
    size: usize,
    heapframes_size: usize,
    ovector_offset: usize,
}

unsafe fn md_out(api: &Api, md: Ptr) -> MdOut {
    MdOut {
        ovector_count: (api.pcre2_get_ovector_count_8)(md),
        size: (api.pcre2_get_match_data_size_8)(md),
        heapframes_size: (api.pcre2_get_match_data_heapframes_size_8)(md),
        ovector_offset: (api.pcre2_get_ovector_pointer_8)(md) as usize - md as usize,
    }
}

// ============================================================== C124

/// C124 — pcre2_match_data_create with gcontext NULL and non-NULL, the [1, UINT16_MAX]
/// clamp on oveccount and the block size, plus the getters before and after a match
/// (pcre2_match_data.c:53-68,141-170).
#[test]
fn c124_match_data_create() {
    isolate!("C124");
    let mut d = D::new("C124");
    unsafe {
        let (c, r) = both();
        let gc_c = (c.pcre2_general_context_create_8)(Some(my_malloc), Some(my_free), ptr::null_mut());
        let gc_r = (r.pcre2_general_context_create_8)(Some(my_malloc), Some(my_free), ptr::null_mut());
        assert!(!gc_c.is_null() && !gc_r.is_null());

        for &n in &[
            0u32, 1, 2, 3, 4, 63, 64, 65, 100, 1000, 65534, 65535, 65536, 65537, 100000,
            0xffff_fffe, 0xffff_ffff,
        ] {
            for (what, g_c, g_r) in [
                ("gcontext=NULL", ptr::null_mut(), ptr::null_mut()),
                ("gcontext=custom", gc_c, gc_r),
            ] {
                let md_c = (c.pcre2_match_data_create_8)(n, g_c);
                let md_r = (r.pcre2_match_data_create_8)(n, g_r);
                d.cases += 1;
                assert!(!md_c.is_null() && !md_r.is_null(), "C124: create({}) returned NULL", n);
                let a = md_out(c, md_c);
                let b = md_out(r, md_r);
                if a != b {
                    d.record(
                        "match_data_create".to_string(),
                        format!("oveccount={} {}: C {:?} vs RUST {:?}", n, what, a, b),
                    );
                }
                // the ovector is writable for exactly 2*oveccount slots
                let ov_c = (c.pcre2_get_ovector_pointer_8)(md_c);
                let ov_r = (r.pcre2_get_ovector_pointer_8)(md_r);
                let slots = (a.ovector_count as usize) * 2;
                for i in 0..slots {
                    *ov_c.add(i) = 0x5a5a_0000 + i;
                    *ov_r.add(i) = 0x5a5a_0000 + i;
                }
                let rd_c: Vec<usize> = (0..slots).map(|i| *ov_c.add(i)).collect();
                let rd_r: Vec<usize> = (0..slots).map(|i| *ov_r.add(i)).collect();
                if rd_c != rd_r {
                    d.record("ovector-write".to_string(), format!("oveccount={} {}", n, what));
                }
                (c.pcre2_match_data_free_8)(md_c);
                (r.pcre2_match_data_free_8)(md_r);
            }
        }

        // a general context whose malloc always fails
        let bad_c = (c.pcre2_general_context_create_8)(Some(failing_malloc), Some(my_free), ptr::null_mut());
        let bad_r = (r.pcre2_general_context_create_8)(Some(failing_malloc), Some(my_free), ptr::null_mut());
        assert!(!bad_c.is_null() && !bad_r.is_null());
        FAIL_MALLOC.store(true, std::sync::atomic::Ordering::SeqCst);
        for &n in &[0u32, 1, 100, 65535] {
            let md_c = (c.pcre2_match_data_create_8)(n, bad_c);
            let md_r = (r.pcre2_match_data_create_8)(n, bad_r);
            d.cases += 1;
            if !(md_c.is_null() && md_r.is_null()) {
                d.record(
                    "failing-malloc".to_string(),
                    format!(
                        "create({}) with a failing malloc: C null={} RUST null={}",
                        n,
                        md_c.is_null(),
                        md_r.is_null()
                    ),
                );
                if !md_c.is_null() {
                    (c.pcre2_match_data_free_8)(md_c);
                }
                if !md_r.is_null() {
                    (r.pcre2_match_data_free_8)(md_r);
                }
            }
        }
        FAIL_MALLOC.store(false, std::sync::atomic::Ordering::SeqCst);
        (c.pcre2_general_context_free_8)(bad_c);
        (r.pcre2_general_context_free_8)(bad_r);

        // the getters BEFORE and AFTER a successful, a failed and a partial match
        let pats: &[(&[u8], &[u8])] = &[
            (b"(a)(b)?", b"xxab"),
            (b"a(?<n>b)c", b"zabc"),
            (b"(*MARK:m)abc", b"abc"),
            (b"(a)(b)(c)(d)", b"abcd"),
            (b"^nomatch$", b"abc"),
            (b"abcdef", b"abc"),
            (b"(?<n>a)(*MARK:q)(b)", b"ab"),
        ];
        for &(p, subj) in pats {
            let mut ec = 0i32;
            let mut eo = 0usize;
            let code_c = (c.pcre2_compile_8)(p.as_ptr(), p.len(), 0, &mut ec, &mut eo, ptr::null_mut());
            let code_r = (r.pcre2_compile_8)(p.as_ptr(), p.len(), 0, &mut ec, &mut eo, ptr::null_mut());
            assert!(!code_c.is_null() && !code_r.is_null());
            for &n in &[0u32, 1, 2, 3, 64, 65, 1000] {
                for &mopts in &[0u32, PCRE2_PARTIAL_SOFT, PCRE2_PARTIAL_HARD, PCRE2_NOTEMPTY] {
                    let md_c = (c.pcre2_match_data_create_8)(n, ptr::null_mut());
                    let md_r = (r.pcre2_match_data_create_8)(n, ptr::null_mut());
                    d.cases += 1;
                    let a0 = md_out(c, md_c);
                    let b0 = md_out(r, md_r);
                    if a0 != b0 {
                        d.record(
                            "pre-match getters".to_string(),
                            format!("oveccount={} pattern \"{}\": C {:?} vs RUST {:?}", n, esc(p), a0, b0),
                        );
                    }
                    let rc_c = (c.pcre2_match_8)(
                        code_c,
                        subj.as_ptr(),
                        subj.len(),
                        0,
                        mopts,
                        md_c,
                        ptr::null_mut(),
                    );
                    let rc_r = (r.pcre2_match_8)(
                        code_r,
                        subj.as_ptr(),
                        subj.len(),
                        0,
                        mopts,
                        md_r,
                        ptr::null_mut(),
                    );
                    let ma = mstate(c, md_c, rc_c, false);
                    let mb = mstate(r, md_r, rc_r, false);
                    if ma != mb {
                        d.record(
                            "post-match state".to_string(),
                            format!(
                                "pattern \"{}\" subject \"{}\" oveccount={} mopts=0x{:x}:\n      C    {:?}\n      RUST {:?}",
                                esc(p),
                                esc(subj),
                                n,
                                mopts,
                                ma,
                                mb
                            ),
                        );
                    }
                    let a1 = md_out(c, md_c);
                    let b1 = md_out(r, md_r);
                    if a1 != b1 {
                        d.record(
                            "post-match getters".to_string(),
                            format!(
                                "pattern \"{}\" oveccount={} mopts=0x{:x}: C {:?} vs RUST {:?}",
                                esc(p),
                                n,
                                mopts,
                                a1,
                                b1
                            ),
                        );
                    }
                    (c.pcre2_match_data_free_8)(md_c);
                    (r.pcre2_match_data_free_8)(md_r);
                }
            }
            (c.pcre2_code_free_8)(code_c);
            (r.pcre2_code_free_8)(code_r);
        }

        // random oveccounts
        let mut rng = Rng::new(0x124);
        for _ in 0..(nfuzz() / 8).max(60) {
            let n = match rng.below(4) {
                0 => rng.below(4),
                1 => rng.below(200),
                2 => 65530 + rng.below(10),
                _ => rng.next_u64() as u32,
            };
            let use_gc = rng.below(2) == 0;
            let md_c = (c.pcre2_match_data_create_8)(n, if use_gc { gc_c } else { ptr::null_mut() });
            let md_r = (r.pcre2_match_data_create_8)(n, if use_gc { gc_r } else { ptr::null_mut() });
            d.cases += 1;
            assert!(!md_c.is_null() && !md_r.is_null());
            let a = md_out(c, md_c);
            let b = md_out(r, md_r);
            if a != b {
                d.record(
                    "match_data_create".to_string(),
                    format!("rand oveccount={}: C {:?} vs RUST {:?}", n, a, b),
                );
            }
            (c.pcre2_match_data_free_8)(md_c);
            (r.pcre2_match_data_free_8)(md_r);
        }

        (c.pcre2_general_context_free_8)(gc_c);
        (r.pcre2_general_context_free_8)(gc_r);
        // free(NULL) must be a no-op in both
        (c.pcre2_match_data_free_8)(ptr::null_mut());
        (r.pcre2_match_data_free_8)(ptr::null_mut());
    }
    d.finish();
}

// ============================================================== C125

/// C125 — pcre2_match_data_create_from_pattern (oveccount = top_bracket+1, inheriting
/// the UINT16_MAX clamp; the code doubles as the general context when gcontext is NULL)
/// and pcre2_get_match_data_heapframes_size, which stays 0 until pcre2_match allocates
/// frames and stays 0 for pcre2_dfa_match (pcre2_match_data.c:80-88,178-182).
#[test]
fn c125_match_data_from_pattern() {
    isolate!("C125");
    let mut d = D::new("C125");
    unsafe {
        let (c, r) = both();
        let gc_c = (c.pcre2_general_context_create_8)(Some(my_malloc), Some(my_free), ptr::null_mut());
        let gc_r = (r.pcre2_general_context_create_8)(Some(my_malloc), Some(my_free), ptr::null_mut());

        // code == NULL must give NULL in both
        d.cases += 1;
        let z1 = (c.pcre2_match_data_create_from_pattern_8)(ptr::null_mut(), ptr::null_mut());
        let z2 = (r.pcre2_match_data_create_from_pattern_8)(ptr::null_mut(), ptr::null_mut());
        let z3 = (c.pcre2_match_data_create_from_pattern_8)(ptr::null_mut(), gc_c);
        let z4 = (r.pcre2_match_data_create_from_pattern_8)(ptr::null_mut(), gc_r);
        if !(z1.is_null() && z2.is_null() && z3.is_null() && z4.is_null()) {
            d.record(
                "from_pattern(NULL)".to_string(),
                format!("{} {} {} {}", z1.is_null(), z2.is_null(), z3.is_null(), z4.is_null()),
            );
        }

        // patterns with 0, 1, 64, 100 (and more) capture groups
        let mut pats: Vec<(String, Vec<u8>)> = vec![
            ("abc".to_string(), b"xxabc".to_vec()),
            ("(a)".to_string(), b"a".to_vec()),
            ("(?<n>a)(b)".to_string(), b"ab".to_vec()),
            ("(*MARK:m)(a)|(b)".to_string(), b"b".to_vec()),
        ];
        for &n in &[0usize, 1, 2, 63, 64, 65, 100, 101, 1000] {
            let mut p = String::new();
            for _ in 0..n {
                p.push_str("(a)");
            }
            p.push('z');
            pats.push((p, "aaaz".repeat(1).into_bytes()));
        }
        for (p, subj) in &pats {
            let mut ec = 0i32;
            let mut eo = 0usize;
            let code_c =
                (c.pcre2_compile_8)(p.as_ptr(), p.len(), 0, &mut ec, &mut eo, ptr::null_mut());
            let code_r =
                (r.pcre2_compile_8)(p.as_ptr(), p.len(), 0, &mut ec, &mut eo, ptr::null_mut());
            assert!(
                !code_c.is_null() && !code_r.is_null(),
                "C125: pattern {:?} did not compile (ec={})",
                p,
                ec
            );
            for (what, g_c, g_r) in [
                ("gcontext=NULL", ptr::null_mut(), ptr::null_mut()),
                ("gcontext=custom", gc_c, gc_r),
            ] {
                let md_c = (c.pcre2_match_data_create_from_pattern_8)(code_c, g_c);
                let md_r = (r.pcre2_match_data_create_from_pattern_8)(code_r, g_r);
                d.cases += 1;
                assert!(!md_c.is_null() && !md_r.is_null());
                let a0 = md_out(c, md_c);
                let b0 = md_out(r, md_r);
                if a0 != b0 {
                    d.record(
                        "from_pattern getters".to_string(),
                        format!("pattern {:?} {}: C {:?} vs RUST {:?}", p, what, a0, b0),
                    );
                }
                // successful / failed / partial match, then the getters again
                for &(mopts, sub) in &[
                    (0u32, subj.as_slice()),
                    (0u32, b"".as_ref()),
                    (PCRE2_PARTIAL_SOFT, b"a".as_ref()),
                    (PCRE2_PARTIAL_HARD, b"a".as_ref()),
                    (PCRE2_NOTEMPTY_ATSTART, subj.as_slice()),
                ] {
                    let rc_c = (c.pcre2_match_8)(
                        code_c,
                        sub.as_ptr(),
                        sub.len(),
                        0,
                        mopts,
                        md_c,
                        ptr::null_mut(),
                    );
                    let rc_r = (r.pcre2_match_8)(
                        code_r,
                        sub.as_ptr(),
                        sub.len(),
                        0,
                        mopts,
                        md_r,
                        ptr::null_mut(),
                    );
                    let ma = mstate(c, md_c, rc_c, false);
                    let mb = mstate(r, md_r, rc_r, false);
                    d.cases += 1;
                    if ma != mb {
                        d.record(
                            "post-match state".to_string(),
                            format!(
                                "pattern {:?} subject \"{}\" mopts=0x{:x}:\n      C    {:?}\n      RUST {:?}",
                                p,
                                esc(sub),
                                mopts,
                                ma,
                                mb
                            ),
                        );
                    }
                    let a1 = md_out(c, md_c);
                    let b1 = md_out(r, md_r);
                    if a1 != b1 {
                        d.record(
                            "post-match getters".to_string(),
                            format!(
                                "pattern {:?} mopts=0x{:x} {}: C {:?} vs RUST {:?}",
                                p, mopts, what, a1, b1
                            ),
                        );
                    }
                }
                // heapframes_size stays 0 for pcre2_dfa_match
                let md2_c = (c.pcre2_match_data_create_from_pattern_8)(code_c, g_c);
                let md2_r = (r.pcre2_match_data_create_from_pattern_8)(code_r, g_r);
                let mut ws_c = [0i32; 128];
                let mut ws_r = [0i32; 128];
                let rc_c = (c.pcre2_dfa_match_8)(
                    code_c,
                    subj.as_ptr(),
                    subj.len(),
                    0,
                    0,
                    md2_c,
                    ptr::null_mut(),
                    ws_c.as_mut_ptr(),
                    128,
                );
                let rc_r = (r.pcre2_dfa_match_8)(
                    code_r,
                    subj.as_ptr(),
                    subj.len(),
                    0,
                    0,
                    md2_r,
                    ptr::null_mut(),
                    ws_r.as_mut_ptr(),
                    128,
                );
                d.cases += 1;
                let ma = mstate(c, md2_c, rc_c, true);
                let mb = mstate(r, md2_r, rc_r, true);
                if ma != mb {
                    d.record(
                        "dfa post-match state".to_string(),
                        format!("pattern {:?}:\n      C    {:?}\n      RUST {:?}", p, ma, mb),
                    );
                }
                let a2 = md_out(c, md2_c);
                let b2 = md_out(r, md2_r);
                if a2 != b2 || a2.heapframes_size != 0 || b2.heapframes_size != 0 {
                    d.record(
                        "dfa heapframes".to_string(),
                        format!(
                            "pattern {:?} after dfa_match: C {:?} vs RUST {:?} (both must have heapframes_size 0)",
                            p, a2, b2
                        ),
                    );
                }
                (c.pcre2_match_data_free_8)(md2_c);
                (r.pcre2_match_data_free_8)(md2_r);
                (c.pcre2_match_data_free_8)(md_c);
                (r.pcre2_match_data_free_8)(md_r);
            }
            (c.pcre2_code_free_8)(code_c);
            (r.pcre2_code_free_8)(code_r);
        }

        // random patterns: from_pattern must agree on the derived ovector size
        let mut rng = Rng::new(0x125);
        for _ in 0..(nfuzz() / 8).max(60) {
            let pat = gen_pattern(&mut rng);
            let opts = utf_safe_opts(&pat, pat.len(), subset(&mut rng, RAND_OPTS, 14));
            let mut ec = 0i32;
            let mut eo = 0usize;
            let a = (c.pcre2_compile_8)(pat.as_ptr(), pat.len(), opts, &mut ec, &mut eo, ptr::null_mut());
            let b = (r.pcre2_compile_8)(pat.as_ptr(), pat.len(), opts, &mut ec, &mut eo, ptr::null_mut());
            if a.is_null() || b.is_null() {
                if !a.is_null() {
                    (c.pcre2_code_free_8)(a);
                }
                if !b.is_null() {
                    (r.pcre2_code_free_8)(b);
                }
                continue;
            }
            let md_c = (c.pcre2_match_data_create_from_pattern_8)(a, ptr::null_mut());
            let md_r = (r.pcre2_match_data_create_from_pattern_8)(b, ptr::null_mut());
            d.cases += 1;
            let x = md_out(c, md_c);
            let y = md_out(r, md_r);
            if x != y {
                d.record(
                    "from_pattern getters".to_string(),
                    format!("rand pattern \"{}\" opts 0x{:x}: C {:?} vs RUST {:?}", esc(&pat), opts, x, y),
                );
            }
            (c.pcre2_match_data_free_8)(md_c);
            (r.pcre2_match_data_free_8)(md_r);
            (c.pcre2_code_free_8)(a);
            (r.pcre2_code_free_8)(b);
        }

        (c.pcre2_general_context_free_8)(gc_c);
        (r.pcre2_general_context_free_8)(gc_r);
    }
    d.finish();
}
