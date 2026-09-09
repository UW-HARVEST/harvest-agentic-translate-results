//! Differential tests for CONFIGS.md rows 57, 58, 89, 90, 91, 92, 93, 94.
//!
//! * 57/58: the whole `dtest/js/*.js` corpus through `js_dostring` on a
//!   non-strict / strict state that has the `dtest/driver.c` globals installed.
//! * 89:    the two real driver binaries, compared byte for byte.
//! * 90-94: Date / JSON / Math / String / Array built-ins driven from script
//!   source, with deterministic (fixed seed, fixed TZ) inputs.
#![allow(non_snake_case, dead_code, unused_unsafe, unused_imports)]

mod common;
use common::*;

use std::ffi::{c_char, c_int, c_void, CString};
use std::path::PathBuf;
use std::process::Command;
use std::ptr::null_mut;

/* ------------------------------------------------------------------ libc -- */

extern "C" {
    fn putchar(c: c_int) -> c_int;
    fn fputs(s: *const c_char, f: *mut c_void) -> c_int;
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn setvbuf(f: *mut c_void, buf: *mut c_char, mode: c_int, size: usize) -> c_int;
    fn tzset();
    static mut stdout: *mut c_void;
    static mut stderr: *mut c_void;
}

const _IONBF: c_int = 2;

/* ------------------------------------------------- driver-like callbacks -- */

/// The Api currently in use; the `js_CFunction` callbacks below have no other
/// way of reaching it.  Set at the top of every `diff` closure (i.e. inside the
/// forked child), so the two runs never see each other's pointer.
static mut CUR: *const Api = std::ptr::null();

unsafe extern "C" fn jsB_print(J: JS) {
    let api = &*CUR;
    let top = (api.js_gettop)(J);
    let mut i = 1;
    while i < top {
        let s = (api.js_tostring)(J, i);
        if i > 1 {
            putchar(' ' as c_int);
        }
        fputs(s, stdout);
        i += 1;
    }
    putchar('\n' as c_int);
    (api.js_pushundefined)(J);
}

unsafe extern "C" fn jsB_repr(J: JS) {
    let api = &*CUR;
    fputs((api.js_torepr)(J, 1), stdout);
    putchar('\n' as c_int);
    (api.js_pushundefined)(J);
}

unsafe extern "C" fn jsB_gc(J: JS) {
    let api = &*CUR;
    (api.js_gc)(J, 1);
    (api.js_pushundefined)(J);
}

unsafe extern "C" fn myreport(_J: JS, message: *const c_char) {
    printf(cs("[report] %s\n").as_ptr(), message);
}

unsafe extern "C" fn mypanic(_J: JS) {
    printf(cs("[panic]\n").as_ptr());
}

/* js_newcfunctionx() stores the `name` pointer without copying it (see
 * c_src/src/jsvalue.c:493), so the names must outlive the state -- a temporary
 * CString would leave a dangling pointer that js_torepr later prints. */
const N_PRINT: &[u8] = b"print\0";
const N_REPR: &[u8] = b"repr\0";
const N_GC: &[u8] = b"gc\0";

fn nm(b: &'static [u8]) -> *const c_char {
    b.as_ptr() as *const c_char
}

/// Mirror of `setup()` in dtest/driver.c.
unsafe fn setup(api: &Api, J: JS) {
    (api.js_setreport)(J, Some(myreport));
    (api.js_atpanic)(J, Some(mypanic));
    (api.js_newcfunction)(J, Some(jsB_print), nm(N_PRINT), 1);
    (api.js_setglobal)(J, nm(N_PRINT));
    (api.js_newcfunction)(J, Some(jsB_repr), nm(N_REPR), 1);
    (api.js_setglobal)(J, nm(N_REPR));
    (api.js_newcfunction)(J, Some(jsB_gc), nm(N_GC), 0);
    (api.js_setglobal)(J, nm(N_GC));
}

/// Unbuffered stdout/stderr, exactly like the driver, so that any interleaving
/// of library output and test output is reproducible.
unsafe fn unbuffer() {
    setvbuf(stdout, null_mut(), _IONBF, 0);
    setvbuf(stderr, null_mut(), _IONBF, 0);
}

unsafe fn boot(api: &Api, flags: c_int) -> JS {
    CUR = api as *const Api;
    unbuffer();
    tzset();
    let J = newstate(api, flags);
    setup(api, J);
    J
}

unsafe fn finish(api: &Api, J: JS, rc: c_int) {
    printf(cs("dostring returned %d\n").as_ptr(), rc);
    (api.js_gc)(J, 1);
    (api.js_freestate)(J);
    p_line("done");
}

unsafe fn push_num_array(api: &Api, J: JS, name: &str, vals: &[f64]) {
    (api.js_newarray)(J);
    for (i, v) in vals.iter().enumerate() {
        (api.js_pushnumber)(J, *v);
        (api.js_setindex)(J, -2, i as c_int);
    }
    (api.js_setglobal)(J, cs(name).as_ptr());
}

unsafe fn push_str_array(api: &Api, J: JS, name: &str, vals: &[CString]) {
    (api.js_newarray)(J);
    for (i, v) in vals.iter().enumerate() {
        (api.js_pushstring)(J, v.as_ptr());
        (api.js_setindex)(J, -2, i as c_int);
    }
    (api.js_setglobal)(J, cs(name).as_ptr());
}

fn set_tz(tz: &str) {
    std::env::set_var("TZ", tz);
    unsafe { tzset() };
}

fn root_dir() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
}

fn dtest_dir() -> PathBuf {
    root_dir().join("dtest")
}

/* ================================================================ 57 / 58 == */

/// All `dtest/js/*.js` files, sorted, read at runtime.
fn corpus() -> Vec<(String, CString)> {
    let dir = dtest_dir().join("js");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {}", dir.display(), e))
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".js"))
        .collect();
    names.sort();
    assert!(names.len() > 50, "corpus looks too small: {}", names.len());
    names
        .into_iter()
        .map(|n| {
            let mut b = std::fs::read(dir.join(&n)).unwrap();
            b.retain(|c| *c != 0); /* C strings cannot carry NUL */
            (n, CString::new(b).unwrap())
        })
        .collect()
}

/// `common::diff`, plus the `0x...` -> `PTR` normalisation that dtest/run.sh
/// applies.  `js_torepr` of a Function prints the raw object address
/// (`[Function 0x1cabcb90, f, [string]:1]`, see dtest/js/t13_trap.js), which is
/// process specific, so a byte exact comparison of it would be meaningless.
/// Everything else is compared byte for byte, exit status included.
fn diff_norm<F: Fn(&Api)>(tag: &str, f: F) {
    let l = libs();
    let (oc, sc) = capture(&format!("c_{}", tag), || f(&l.c));
    let (or, sr) = capture(&format!("r_{}", tag), || f(&l.r));
    let (nc, nr) = (norm_hex(&oc), norm_hex(&or));
    if nc != nr || sc != sr {
        let mut msg = format!(
            "DIVERGENCE in `{}`\n  C status: {}\n  R status: {}\n",
            tag,
            decode_status(sc),
            decode_status(sr)
        );
        msg += &line_diffs("stdout+stderr", &nc, &nr, 12);
        if nc == nr {
            msg += "  output identical, exit status differs\n";
        }
        panic!("{}", msg);
    }
}

fn run_corpus(tagprefix: &str, flags: c_int) {
    set_tz("UTC"); /* deterministic local time for t07/t15 */
    let files = corpus();
    for (name, src) in &files {
        diff_norm(&format!("{}_{}", tagprefix, name), |api| unsafe {
            let J = boot(api, flags);
            let rc = (api.js_dostring)(J, src.as_ptr());
            finish(api, J, rc);
        });
    }
}

#[test]
fn cfg57_corpus_nonstrict() {
    run_corpus("c57", 0);
}

#[test]
fn cfg58_corpus_strict() {
    run_corpus("c58", JS_STRICT);
}

/* ===================================================================== 89 == */

/// `sed -E 's/0x[0-9a-f]+/PTR/g'`, as in dtest/run.sh.
/// Drop glibc `assert()` diagnostics. The C library is built with assertions
/// live, so a failing assert prints
///   `<prog>: <abs path>/jsdtoa.c:387: minus: Assertion `x.f >= y.f' failed.`
/// whose program name and absolute C source path no translation can reproduce.
/// The SIGABRT exit status and all other bytes are still compared.
fn strip_assert(b: &[u8]) -> Vec<u8> {
    let s = String::from_utf8_lossy(b);
    let mut out = String::new();
    for line in s.split_inclusive('\n') {
        if line.contains(": Assertion `") && line.contains("failed.") {
            continue;
        }
        out.push_str(line);
    }
    out.into_bytes()
}

fn norm_hex(b: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0usize;
    while i < b.len() {
        if b[i] == b'0' && i + 1 < b.len() && b[i + 1] == b'x' {
            let mut j = i + 2;
            while j < b.len() && (b[j].is_ascii_digit() || (b[j] >= b'a' && b[j] <= b'f')) {
                j += 1;
            }
            if j > i + 2 {
                out.extend_from_slice(b"PTR");
                i = j;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    out
}

fn status_str(st: &std::process::ExitStatus) -> String {
    use std::os::unix::process::ExitStatusExt;
    match st.code() {
        Some(c) => format!("exit {}", c),
        None => format!("signal {}", st.signal().unwrap_or(-1)),
    }
}

fn line_diffs(what: &str, a: &[u8], b: &[u8], max: usize) -> String {
    let sa = String::from_utf8_lossy(a);
    let sb = String::from_utf8_lossy(b);
    let al: Vec<&str> = sa.lines().collect();
    let bl: Vec<&str> = sb.lines().collect();
    let mut msg = String::new();
    let mut shown = 0;
    for i in 0..al.len().max(bl.len()) {
        let x = al.get(i).copied().unwrap_or("<missing>");
        let y = bl.get(i).copied().unwrap_or("<missing>");
        if x != y {
            msg += &format!("    {} line {}:\n      C: {}\n      R: {}\n", what, i + 1, x, y);
            shown += 1;
            if shown >= max {
                msg += "    ...\n";
                break;
            }
        }
    }
    msg
}

fn build_driver(dtest: &PathBuf, out: &str, lib: &str, rpath: &str) {
    let rp = format!("-Wl,-rpath,{}", rpath);
    let st = Command::new("gcc")
        .current_dir(dtest)
        .args([
            "-O1",
            "-I../c_src/include",
            "-o",
            out,
            "driver.c",
            lib,
            "-lm",
            rp.as_str(),
        ])
        .output()
        .unwrap_or_else(|e| panic!("cannot run gcc: {}", e));
    assert!(
        st.status.success(),
        "building {} failed:\n{}\n{}",
        out,
        String::from_utf8_lossy(&st.stdout),
        String::from_utf8_lossy(&st.stderr)
    );
}

#[test]
fn cfg89_drivers() {
    set_tz("UTC");
    let dtest = dtest_dir();

    /* Always rebuild both drivers so they match the current libraries. */
    build_driver(
        &dtest,
        "driver_c",
        "../c_src/build/libmujs.so",
        "$ORIGIN/../c_src/build",
    );
    build_driver(
        &dtest,
        "driver_r",
        "../translation/target/release/libmujs.so",
        "$ORIGIN/../translation/target/release",
    );

    let mut fails: Vec<String> = Vec::new();
    let mut runs = 0usize;

    /* IMPORTANT: the C libmujs.so has SONAME "libmujs.so", so driver_c records
     * a plain NEEDED "libmujs.so" that the loader resolves through
     * LD_LIBRARY_PATH *before* the RUNPATH baked in by -Wl,-rpath.  cargo test
     * puts target/release on LD_LIBRARY_PATH, which would silently make
     * driver_c load the *Rust* library and turn every comparison below into a
     * tautology.  Pin LD_LIBRARY_PATH per process to prevent that. */
    let cdir = root_dir().join("c_src/build");
    let rdir = root_dir().join("translation/target/release");

    let compare = |args: &[&str], fails: &mut Vec<String>, runs: &mut usize| {
        let oc = Command::new("./driver_c")
            .current_dir(&dtest)
            .env("LD_LIBRARY_PATH", &cdir)
            .args(args)
            .output()
            .expect("run driver_c");
        let or = Command::new("./driver_r")
            .current_dir(&dtest)
            .env("LD_LIBRARY_PATH", &rdir)
            .args(args)
            .output()
            .expect("run driver_r");
        *runs += 1;
        let (co, ro) = (norm_hex(&oc.stdout), norm_hex(&or.stdout));
        let (ce, re) = (
            strip_assert(&norm_hex(&oc.stderr)),
            strip_assert(&norm_hex(&or.stderr)),
        );
        let (cs_, rs_) = (status_str(&oc.status), status_str(&or.status));
        if co != ro || ce != re || cs_ != rs_ {
            let mut m = format!("  driver {:?}\n    C status: {}\n    R status: {}\n", args, cs_, rs_);
            if co != ro {
                m += &line_diffs("stdout", &co, &ro, 8);
            }
            if ce != re {
                m += &line_diffs("stderr", &ce, &re, 8);
            }
            fails.push(m);
        }
    };

    for m in ["-api", "-lowlevel", "-regexp", "-ctx"] {
        compare(&[m], &mut fails, &mut runs);
    }

    let picks = [
        "js/t01_numbers.js",
        "js/t06_json.js",
        "js/t07_date.js",
        "js/t11_misc.js",
        "js/t12_limits.js",
        "js/t13_trap.js",
        "js/t16_syntax.js",
        "js/f000_refuzz.js",
        "js/g000_progfuzz.js",
    ];
    let extras: [&[&str]; 6] = [
        &[],
        &["strict"],
        &["dumpstrings"],
        &["limit"],
        &["memlimit"],
        &["strict", "dumpstrings"],
    ];
    for f in picks {
        for ex in extras.iter() {
            let mut a: Vec<&str> = vec![f];
            a.extend_from_slice(ex);
            compare(&a, &mut fails, &mut runs);
        }
    }

    /* dtest/run.sh takes ~30 s on this machine, well under the 300 s budget,
     * so it is executed as part of this test.  (If it ever exceeds the limit
     * `timeout` kills it with status 124 and we skip it instead of failing.) */
    let rs = Command::new("timeout")
        .current_dir(&dtest)
        .env_remove("LD_LIBRARY_PATH") /* see the note above */
        .args(["300", "bash", "run.sh"])
        .output()
        .expect("run dtest/run.sh");
    let rout = String::from_utf8_lossy(&rs.stdout).into_owned();
    if rs.status.code() == Some(124) {
        eprintln!("note: dtest/run.sh exceeded 300 s and was skipped");
    } else if !rs.status.success() {
        /* run.sh folds stderr into stdout (2>&1), so the unmatchable glibc
         * assert diagnostic shows up as a diff there. Accept a failure only if
         * EVERY differing line is such an assert message. */
        let mut m = String::from("  bash dtest/run.sh reported failures:\n");
        let mut real = false;
        for l in rout.lines() {
            let is_diffline = l.starts_with('<') || l.starts_with('>');
            let is_assert = l.contains(": Assertion `") && l.contains("failed.");
            if is_diffline && !is_assert {
                real = true;
            }
            if l.contains("EXIT DIFF") {
                real = true;
            }
            if l.contains("DIFF") || is_diffline || l.starts_with("---") {
                m += &format!("    {}\n", l);
            }
        }
        if real {
            fails.push(m);
        } else {
            eprintln!("note: dtest/run.sh differences are glibc assert diagnostics only");
        }
    }

    if !fails.is_empty() {
        panic!(
            "DIVERGENCE in cfg89_drivers ({} driver comparisons run):\n{}",
            runs,
            fails.join("")
        );
    }
}

/* ===================================================================== 90 == */

const JS_DATE: &str = r##"
function T(f) { try { return f(); } catch (e) { return "throw:" + e.name + ":" + e.message; } }
function S(x) { return "" + x; }

var MS = [0,1,-1,999,1000,-1000,59999,60000,3599999,3600000,86399999,86400000,-86400000,
  1000000000,-1000000000,1234567890123,-1234567890123,
  8640000000000000,-8640000000000000,8640000000000001,-8640000000000001,
  NaN,Infinity,-Infinity,0.5,-0.5,-0,1.5,-1.5,
  946684800000,951782400000,2147483647000,-2208988800000,
  1e21,-1e21,1e13,253402300799999,-62167219200000,
  1483228800000,1451606400000,-86400001,4102444800000,
  -1e13,68400000,1234567890,7258118400000];

var GET = ["getTime","valueOf","getFullYear","getMonth","getDate","getDay","getHours",
  "getMinutes","getSeconds","getMilliseconds","getTimezoneOffset",
  "getUTCFullYear","getUTCMonth","getUTCDate","getUTCDay","getUTCHours",
  "getUTCMinutes","getUTCSeconds","getUTCMilliseconds"];
var FMT = ["toString","toDateString","toTimeString","toLocaleString","toLocaleDateString",
  "toLocaleTimeString","toUTCString","toISOString","toJSON"];

function dumpdate(tag, d) {
  var i, s = tag;
  for (i = 0; i < GET.length; ++i)
    s += " " + GET[i] + "=" + S(T(function () { return d[GET[i]](); }));
  print(s);
  for (i = 0; i < FMT.length; ++i)
    print("  " + FMT[i] + " -> " + S(T(function () { return d[FMT[i]](); })));
}

print("== constructed from ms ==");
for (var mi = 0; mi < MS.length; ++mi)
  dumpdate("ms[" + mi + "]=" + S(MS[mi]), new Date(MS[mi]));

print("== constructor argument shapes ==");
dumpdate("c0", new Date(0));
dumpdate("c1", new Date(2000, 0));
dumpdate("c2", new Date(2000, 0, 1));
dumpdate("c3", new Date(2000, 11, 31, 23, 59, 59, 999));
dumpdate("c4", new Date(1970, 0, 1, 0, 0, 0, 0));
dumpdate("c5", new Date(99, 0, 1));
dumpdate("c6", new Date(NaN, 0));
dumpdate("c7", new Date(2000, 13, 1));
dumpdate("c8", new Date(2000, 0, 0));
dumpdate("c9", new Date(2000, 0, -1));
dumpdate("c10", new Date("2000-01-01T00:00:00Z"));
dumpdate("c11", new Date("garbage"));
dumpdate("c12", new Date(null));
dumpdate("c13", new Date(true));
dumpdate("c14", new Date(undefined));
dumpdate("c15", new Date("1970-01-01"));
dumpdate("c16", new Date(1970, 0, 1, 24, 60, 60, 1000));
dumpdate("c17", new Date(-1, 0, 1));
dumpdate("c18", new Date(1e21, 0, 1));
dumpdate("c19", new Date("2016-02-29T12:34:56.789+05:30"));
print("no-arg typeof: " + (typeof new Date()) + " " + (new Date() instanceof Date));
print("Date() typeof: " + (typeof Date()));
print("Date.now typeof: " + (typeof Date.now()) + " positive=" + (Date.now() > 1e12));
print("Date.length=" + Date.length + " UTC.length=" + Date.UTC.length + " parse.length=" + Date.parse.length);

print("== Date.UTC ==");
print("UTC()=" + S(T(function () { return Date.UTC(); })));
print("UTC(2000)=" + S(Date.UTC(2000)));
print("UTC(2000,0)=" + S(Date.UTC(2000, 0)));
print("UTC(2000,1,29)=" + S(Date.UTC(2000, 1, 29)));
print("UTC(2000,11,31,23,59,59,999)=" + S(Date.UTC(2000, 11, 31, 23, 59, 59, 999)));
print("UTC(1970,0,1,0,0,0,0)=" + S(Date.UTC(1970, 0, 1, 0, 0, 0, 0)));
print("UTC(70,0,1)=" + S(Date.UTC(70, 0, 1)));
print("UTC(99,0,1)=" + S(Date.UTC(99, 0, 1)));
print("UTC(NaN,0)=" + S(Date.UTC(NaN, 0)));
print("UTC(2000,13,1)=" + S(Date.UTC(2000, 13, 1)));
print("UTC(2000,0,0)=" + S(Date.UTC(2000, 0, 0)));
print("UTC(2000,0,-1)=" + S(Date.UTC(2000, 0, -1)));
print("UTC(2000,-1,1)=" + S(Date.UTC(2000, -1, 1)));
print("UTC(1e6,0,1)=" + S(Date.UTC(1000000, 0, 1)));
print("UTC('2000','1','2')=" + S(Date.UTC("2000", "1", "2")));
print("UTC(Infinity,0)=" + S(Date.UTC(Infinity, 0)));
print("UTC(2000,0,1,0,0,0,-1)=" + S(Date.UTC(2000, 0, 1, 0, 0, 0, -1)));

print("== Date.parse ==");
var PARSE = ["1970-01-01T00:00:00Z", "1970-01-01T00:00:00.000Z", "1970-01-01",
  "2000-01-01T00:00:00", "2000-01-01T00:00:00+01:00", "2000-01-01T00:00:00-05:30",
  "2016-02-29T23:59:59.999Z", "0000-01-01T00:00:00Z", "+275760-09-13T00:00:00.000Z",
  "-000001-01-01T00:00:00Z", "1999-12-31T23:59:60Z", "2000-13-01T00:00:00Z",
  "2000-01-32T00:00:00Z", "2000-01-01T24:00:00Z", "2000-01-01T00:60:00Z",
  "Thu Jan 01 1970 00:00:00 GMT+0000", "Thu, 01 Jan 1970 00:00:00 GMT",
  "Mon, 25 Dec 1995 13:30:00 GMT", "Mon, 25 Dec 1995 13:30:00 +0430",
  "Dec 25 1995", "December 25, 1995", "25 Dec 1995", "1995/12/25",
  "12/25/1995", "1995-12-25 13:30:00", "Wed Mar 02 2016", "Tue Feb 30 2016",
  "", " ", "garbage", "T", "Z", "1970", "1970-01", "197", "1e3",
  "1970-01-01T00:00:00.1Z", "1970-01-01T00:00:00.123456Z",
  "1970-01-01t00:00:00z", "1970-01-01T00:00Z", "1970-01-01 00:00:00Z",
  "  1970-01-01T00:00:00Z  ", "9999-12-31T23:59:59.999Z",
  "1970-01-01T00:00:00+24:00", "1970-01-01T00:00:00+0100", "1970-01-01T00:00:00GMT"];
for (var pi = 0; pi < PARSE.length; ++pi) {
  var pv = Date.parse(PARSE[pi]);
  print("parse(" + JSON.stringify(PARSE[pi]) + ")=" + S(pv) +
    " round=" + S(T(function () { return new Date(pv).toISOString(); })));
}
print("parse(no args)=" + S(Date.parse()));
print("parse(0)=" + S(Date.parse(0)) + " parse(null)=" + S(Date.parse(null)));

print("== setters ==");
var SET = [
  ["setTime", [[0], [1000000000000], [NaN], [], [-1], [8640000000000001], ["123"]]],
  ["setMilliseconds", [[0], [500], [-1], [1000], [NaN], []]],
  ["setUTCMilliseconds", [[0], [500], [-1], [1000], [NaN], []]],
  ["setSeconds", [[0], [59], [60], [-1], [30, 500], [NaN, NaN], []]],
  ["setUTCSeconds", [[0], [59], [60], [-1], [30, 500], []]],
  ["setMinutes", [[0], [59], [90], [-5], [10, 20], [10, 20, 30], []]],
  ["setUTCMinutes", [[0], [59], [90], [-5], [10, 20, 30], []]],
  ["setHours", [[0], [23], [24], [-1], [1, 2], [1, 2, 3], [1, 2, 3, 4], []]],
  ["setUTCHours", [[0], [23], [24], [-1], [1, 2, 3, 4], []]],
  ["setDate", [[1], [31], [32], [0], [-1], [NaN], []]],
  ["setUTCDate", [[1], [31], [32], [0], [-1], []]],
  ["setMonth", [[0], [11], [12], [-1], [5, 15], []]],
  ["setUTCMonth", [[0], [11], [12], [-1], [5, 15], []]],
  ["setFullYear", [[2000], [1899], [0], [-1], [2000, 5], [2000, 5, 20], []]],
  ["setUTCFullYear", [[2000], [1899], [0], [-1], [2000, 5, 20], []]]
];
var BASES = [0, 1234567890123, -1234567890123, 8640000000000000, NaN];
for (var bi = 0; bi < BASES.length; ++bi) {
  for (var si = 0; si < SET.length; ++si) {
    var nm = SET[si][0], lists = SET[si][1];
    for (var li = 0; li < lists.length; ++li) {
      var ar = lists[li];
      var d = new Date(BASES[bi]);
      var r = T(function () { return d[nm].apply(d, ar); });
      print("base=" + S(BASES[bi]) + " " + nm + "(" + ar.join(",") + ") -> " + S(r) +
        " time=" + S(d.getTime()) + " iso=" + S(T(function () { return d.toISOString(); })) +
        " str=" + S(T(function () { return d.toString(); })));
    }
  }
}

print("== arithmetic and comparison ==");
var A = new Date(0), B = new Date(86400000), C = new Date(NaN), D = new Date(-1);
print("B-A=" + S(B - A) + " A-B=" + S(A - B) + " C-A=" + S(C - A) + " D-A=" + S(D - A));
print("B>A=" + (B > A) + " A>B=" + (A > B) + " C>A=" + (C > A) + " C<A=" + (C < A) + " C==C=" + (C == C));
print("+A=" + S(+A) + " +B=" + S(+B) + " +C=" + S(+C));
print("A==B=" + (A == B) + " A===A=" + (A === A) + " A==0=" + (A == 0));
print("A+B=" + (A + B));
print("A*1=" + S(A * 1) + " B/2=" + S(B / 2) + " B%7=" + S(B % 7));
print("String(A)=" + String(A) + " Number(A)=" + S(Number(A)));
var sorted = [B, A, D, C].sort(function (x, y) { return x - y; });
print("sorted times=" + sorted.map(function (x) { return S(x.getTime()); }).join(","));
print("JSON=" + JSON.stringify({ a: A, b: B, c: T(function () { return JSON.stringify(C); }) }));
print("toJSON of invalid: " + S(T(function () { return C.toJSON(); })));
print("proto call on non-date: " + S(T(function () { return Date.prototype.getTime.call({}); })));
print("proto call on number: " + S(T(function () { return Date.prototype.toString.call(5); })));
print("valueOf/getTime same: " + (A.valueOf() === A.getTime()));
"##;

#[test]
fn cfg90_date() {
    /* fixed timezone, set before the first library call, never "now" */
    set_tz("UTC");
    for tz in ["UTC", "America/New_York", "Asia/Kolkata"] {
        set_tz(tz);
        let src = cs(JS_DATE);
        diff(&format!("c90_date_{}", tz), |api| unsafe {
            let J = boot(api, 0);
            let rc = (api.js_dostring)(J, src.as_ptr());
            finish(api, J, rc);
        });
    }
}

/* ===================================================================== 91 == */

const JS_JSON: &str = r##"
function T(f) { try { return f(); } catch (e) { return "throw:" + e.name + ":" + e.message; } }
function S(x) { return "" + x; }

print("== stringify of value kinds ==");
var VALS = [0, -0, 1, -1, 0.5, 1e21, 1e-7, 1/3, 1e308, 5e-324, NaN, Infinity, -Infinity,
  true, false, null, undefined, "", "a", "a\"b", "a\\b", "\u0000\u0001\u001f",
  "\n\t\r\b\f", "é", "中文", "😀", "\ud800", "\udfff",
  "\u2028\u2029", [], [1, 2, 3], [1, [2, [3, [4]]]], [undefined, null, NaN],
  {}, { a: 1 }, { a: { b: { c: { d: 1 } } } }, { a: undefined, b: function () {}, c: 1 },
  function () {}, new Date(0), new Number(5), new String("s"), new Boolean(true),
  [1, "two", true, null, { x: [1, 2] }], { "": 1, " ": 2, "\n": 3 }];
for (var i = 0; i < VALS.length; ++i) {
  print("v[" + i + "] typeof=" + (typeof VALS[i]) + " -> " + S(T(function () { return JSON.stringify(VALS[i]); })));
  print("   indent2 -> " + S(T(function () { return JSON.stringify(VALS[i], null, 2); })));
  print("   roundtrip -> " + S(T(function () { return JSON.stringify(JSON.parse(JSON.stringify(VALS[i]))); })));
}

print("== indent as number 0..12 and as string ==");
var DEEP = { a: 1, b: [1, 2, { c: 3, d: [4, 5] }], e: { f: { g: "h" } }, i: null };
for (var n = 0; n <= 12; ++n)
  print("indent " + n + ":\n" + JSON.stringify(DEEP, null, n));
var ISTR = ["", " ", "\t", "..", "0123456789ab", "0123456789abcdef", "\n", "é"];
for (var k = 0; k < ISTR.length; ++k)
  print("indent " + JSON.stringify(ISTR[k]) + ":\n" + JSON.stringify(DEEP, null, ISTR[k]));
print("indent -1:\n" + JSON.stringify(DEEP, null, -1));
print("indent 100:\n" + JSON.stringify(DEEP, null, 100));
print("indent NaN:\n" + JSON.stringify(DEEP, null, NaN));
print("indent new Number(3):\n" + JSON.stringify(DEEP, null, new Number(3)));
print("indent new String('--'):\n" + JSON.stringify(DEEP, null, new String("--")));

print("== replacer ==");
print(JSON.stringify(DEEP, function (k, v) { return k === "c" ? undefined : v; }));
print(JSON.stringify(DEEP, function (k, v) { return typeof v === "number" ? v * 2 : v; }));
print(JSON.stringify(DEEP, function (k, v) { print("  visit " + JSON.stringify(k) + " " + (typeof v)); return v; }));
print(JSON.stringify(DEEP, function (k, v) { return typeof v === "object" && v !== null ? v : String(v); }));
print(JSON.stringify(DEEP, ["a", "b", "e", "f"]));
print(JSON.stringify(DEEP, []));
print(JSON.stringify(DEEP, ["a", "a", "nope"], 1));
print(JSON.stringify([1, 2, 3], ["0", "1"]));
print(JSON.stringify(DEEP, [new String("a"), new Number(0)]));
print("replacer throws: " + S(T(function () { return JSON.stringify(DEEP, function () { throw new Error("rep"); }); })));
print("toJSON: " + JSON.stringify({ x: { toJSON: function (k) { return "TJ:" + k; } }, y: 1 }));
print("toJSON in array: " + JSON.stringify([{ toJSON: function () { return 42; } }]));

print("== parse and reviver ==");
var GOOD = ['0', '-0', '1', '-1', '1.5', '1e3', '1E-3', '-1.5e+3', '"a"', '""',
  '"\\u00e9"', '"\\ud83d\\ude00"', '"\\ud800"', '"\\n\\t\\r\\b\\f\\/\\\\\\""',
  '"\\u0000"', 'true', 'false', 'null', '[]', '{}', '[1,2,3]', '{"a":1}',
  '{"a":{"b":[1,2,{"c":null}]}}', '[[[[[1]]]]]', ' \t\r\n{ "a" : [ 1 , 2 ] } \r\n',
  '1e308', '1e309', '-1e309', '5e-324', '1e-400', '123456789012345678901234567890',
  '{"a":1,"a":2}', '[1,2,[3,[4,[5,[6]]]]]', '"\\u0041\\u0301"'];
for (var g = 0; g < GOOD.length; ++g) {
  print("parse(" + GOOD[g] + ") -> " + S(T(function () { return JSON.stringify(JSON.parse(GOOD[g])); })));
  print("   revived -> " + S(T(function () {
    return JSON.stringify(JSON.parse(GOOD[g], function (k, v) {
      return typeof v === "number" ? v + 1 : v;
    }));
  })));
}
print("reviver visit order:");
JSON.parse('{"a":[1,{"b":2}],"c":3}', function (k, v) { print("  rev " + JSON.stringify(k) + " " + (typeof v) + " " + S(v)); return v; });
print("reviver deleting: " + JSON.stringify(JSON.parse('{"a":1,"b":2}', function (k, v) { return k === "a" ? undefined : v; })));
print("reviver throws: " + S(T(function () { return JSON.parse('{"a":1}', function () { throw new Error("rev"); }); })));

print("== invalid input ==");
var BAD = ['', ' ', '{', '}', '[', ']', '[1,', '{"a":}', '{a:1}', "'x'", '1 2', '01',
  '+1', '.5', '5.', '1e', 'NaN', 'Infinity', '-Infinity', 'undefined', 'tru', 'nul',
  '[1,2,]', '{"a":1,}', '"\\x41"', '"unterminated', '["a" "b"]', '{"a" 1}', '"\\u12"',
  '-', '--1', '1.2.3', '0x10', '{"a":1}}', '[]]', '"\\q"', '"a\nb"', '[,]', '[1,,2]',
  '{,}', '{"a"}', 'é', '[1 2]', '{"a":1 "b":2}'];
for (var b = 0; b < BAD.length; ++b)
  print("bad " + JSON.stringify(BAD[b]) + " -> " + S(T(function () { return JSON.stringify(JSON.parse(BAD[b])); })));

print("== cyclic ==");
var c1 = {}; c1.self = c1;
print("obj cycle: " + S(T(function () { return JSON.stringify(c1); })));
var c2 = []; c2.push(c2);
print("arr cycle: " + S(T(function () { return JSON.stringify(c2); })));
var c3 = { a: { b: {} } }; c3.a.b.up = c3.a;
print("deep cycle: " + S(T(function () { return JSON.stringify(c3); })));
var c4 = { x: 1 }; var c5 = { a: c4, b: c4 };
print("shared (not cyclic): " + S(T(function () { return JSON.stringify(c5); })));
print("cycle with indent: " + S(T(function () { return JSON.stringify(c1, null, 2); })));

print("== random documents ==");
for (var d = 0; d < DOCS.length; ++d) {
  var doc = DOCS[d];
  print("doc " + d + " len=" + doc.length);
  var v = T(function () { return JSON.parse(doc); });
  if (typeof v === "string" && v.substring(0, 6) === "throw:") { print("  " + v); continue; }
  print("  s0=" + S(T(function () { return JSON.stringify(v); })));
  print("  s2=" + S(T(function () { return JSON.stringify(v, null, 2); })));
  print("  st=" + S(T(function () { return JSON.stringify(v, null, "\t"); })));
  print("  rr=" + S(T(function () { return JSON.stringify(JSON.parse(JSON.stringify(v))); })));
  print("  rv=" + S(T(function () {
    return JSON.stringify(JSON.parse(doc, function (k, v2) {
      if (typeof v2 === "number") return -v2;
      if (typeof v2 === "string") return v2.length;
      return v2;
    }));
  })));
}
"##;

fn json_num(rng: &mut Rng) -> String {
    match rng.range(11) {
        0 => "0".to_string(),
        1 => "-0".to_string(),
        2 => format!("{}", rng.next_u32()),
        3 => format!("-{}", rng.next_u32()),
        4 => format!("{}.{}", rng.range(1000), rng.range(1000000)),
        5 => format!("{}e{}", 1 + rng.range(99), rng.range(41) as i32 - 20),
        6 => "1e308".to_string(),
        7 => "5e-324".to_string(),
        8 => "123456789012345678901234567890".to_string(),
        9 => format!("-{}.{}e-{}", rng.range(10), rng.range(1000), rng.range(30)),
        _ => format!("{}", rng.range(20) as i32 - 10),
    }
}

fn json_str(rng: &mut Rng) -> String {
    const P: [&str; 14] = [
        "",
        "a",
        "abc",
        "hello world",
        "\\u00e9",
        "\\ud83d\\ude00",
        "\\n\\t\\r\\b\\f",
        "\\\\",
        "\\\"",
        "\\/",
        "\u{e9}\u{4e2d}\u{6587}",
        "\\u0041\\u0301",
        "0123456789",
        " ",
    ];
    let mut s = String::from("\"");
    let n = rng.range(4);
    for _ in 0..n {
        s.push_str(P[rng.range(P.len() as u32) as usize]);
    }
    s.push('"');
    s
}

fn json_doc(rng: &mut Rng, depth: u32, out: &mut String) {
    let kinds = if depth >= 3 { 6 } else { 8 };
    match rng.range(kinds) {
        0 => out.push_str("null"),
        1 => out.push_str("true"),
        2 => out.push_str("false"),
        3 | 4 => out.push_str(&json_num(rng)),
        5 => out.push_str(&json_str(rng)),
        6 => {
            out.push('[');
            let n = rng.range(5);
            for i in 0..n {
                if i > 0 {
                    out.push(',');
                }
                json_doc(rng, depth + 1, out);
            }
            out.push(']');
        }
        _ => {
            out.push('{');
            let n = rng.range(5);
            for i in 0..n {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&json_str(rng));
                out.push(':');
                json_doc(rng, depth + 1, out);
            }
            out.push('}');
        }
    }
}

#[test]
fn cfg91_json() {
    set_tz("UTC");
    let mut rng = Rng::new(0x5eed_1234_5678_9abc);
    let mut docs: Vec<CString> = Vec::new();
    for _ in 0..200 {
        let mut s = String::new();
        json_doc(&mut rng, 0, &mut s);
        docs.push(CString::new(s).unwrap());
    }
    let src = cs(JS_JSON);
    diff("c91_json", |api| unsafe {
        let J = boot(api, 0);
        push_str_array(api, J, "DOCS", &docs);
        let rc = (api.js_dostring)(J, src.as_ptr());
        finish(api, J, rc);
    });
}

/* ===================================================================== 92 == */

const JS_MATH: &str = r##"
function T(f) { try { return f(); } catch (e) { return "throw:" + e.name + ":" + e.message; } }
/* N() uses the library's own number->string conversion */
function N(x) { return "" + x; }

var UN = ["abs","acos","asin","atan","ceil","cos","exp","floor","log","round","sin","sqrt","tan"];
var BI = ["atan2","pow","max","min"];

var F = [0, -0, 1, -1, 0.5, -0.5, Infinity, -Infinity, NaN,
  1e308, -1e308, 5e-324, -5e-324, 1e-323, 2, -2, 3.5, -3.5, 0.1, -0.1,
  1e21, -1e21, 1e-7, Math.PI, Math.E, 1/3, -1/3,
  2147483647, -2147483648, 4294967296, 9007199254740992, 9007199254740993,
  0.49999999999999994, 1.5, 2.5, -1.5, -2.5, 4.5, -4.5, 1e16 + 1];
var G = [0, -0, 1, -1, 0.5, -0.5, Infinity, -Infinity, NaN, 2, -2, 1e308, 5e-324, 3, 0.1, 1/3];

print("== constants ==");
print("E=" + N(Math.E) + " LN10=" + N(Math.LN10) + " LN2=" + N(Math.LN2) +
  " LOG2E=" + N(Math.LOG2E) + " LOG10E=" + N(Math.LOG10E) + " PI=" + N(Math.PI) +
  " SQRT1_2=" + N(Math.SQRT1_2) + " SQRT2=" + N(Math.SQRT2));
for (var u = 0; u < UN.length; ++u) print("length " + UN[u] + "=" + Math[UN[u]].length);
for (var b0 = 0; b0 < BI.length; ++b0) print("length " + BI[b0] + "=" + Math[BI[b0]].length);

print("== unary over fixed values ==");
for (var i = 0; i < F.length; ++i)
  for (var u2 = 0; u2 < UN.length; ++u2)
    print(UN[u2] + "(" + N(F[i]) + ")=" + N(Math[UN[u2]](F[i])));

print("== unary over random values ==");
for (var r = 0; r < V.length; ++r) {
  var line = "r[" + r + "] x=" + N(V[r]);
  for (var u3 = 0; u3 < UN.length; ++u3)
    line += " " + UN[u3] + "=" + N(Math[UN[u3]](V[r]));
  print(line);
}

print("== binary over fixed values ==");
for (var x = 0; x < G.length; ++x)
  for (var y = 0; y < G.length; ++y)
    print("atan2(" + N(G[x]) + "," + N(G[y]) + ")=" + N(Math.atan2(G[x], G[y])) +
      " pow=" + N(Math.pow(G[x], G[y])) +
      " max=" + N(Math.max(G[x], G[y])) +
      " min=" + N(Math.min(G[x], G[y])));

print("== binary over random pairs ==");
for (var p = 0; p + 1 < V.length; p += 2)
  print("pair " + p + " atan2=" + N(Math.atan2(V[p], V[p + 1])) +
    " pow=" + N(Math.pow(V[p], V[p + 1])) +
    " max=" + N(Math.max(V[p], V[p + 1])) +
    " min=" + N(Math.min(V[p], V[p + 1])));

print("== max/min arities ==");
print("max()=" + N(Math.max()) + " min()=" + N(Math.min()));
print("max(1)=" + N(Math.max(1)) + " min(1)=" + N(Math.min(1)));
print("max(-0,0)=" + N(Math.max(-0, 0)) + " min(-0,0)=" + N(Math.min(-0, 0)));
print("max(0,-0)=" + N(Math.max(0, -0)) + " min(0,-0)=" + N(Math.min(0, -0)));
print("max(1,2)=" + N(Math.max(1, 2)) + " min(1,2)=" + N(Math.min(1, 2)));
print("max(NaN,1)=" + N(Math.max(NaN, 1)) + " min(NaN,1)=" + N(Math.min(NaN, 1)));
print("max(1,NaN)=" + N(Math.max(1, NaN)) + " min(1,NaN)=" + N(Math.min(1, NaN)));
print("max(1,2,3,4,5)=" + N(Math.max(1, 2, 3, 4, 5)) + " min(1,2,3,4,5)=" + N(Math.min(1, 2, 3, 4, 5)));
print("max(1,NaN,3,-Infinity,5)=" + N(Math.max(1, NaN, 3, -Infinity, 5)));
print("min(1,NaN,3,-Infinity,5)=" + N(Math.min(1, NaN, 3, -Infinity, 5)));
print("max('3',true,null,undefined,[])=" + N(Math.max("3", true, null, undefined, [])));
print("min('3',true,null,[2])=" + N(Math.min("3", true, null, [2])));
print("max.apply(null,V.slice(0,50))=" + N(Math.max.apply(null, V.slice(0, 50))));
print("min.apply(null,V.slice(0,50))=" + N(Math.min.apply(null, V.slice(0, 50))));

print("== coercion of arguments ==");
var CO = ["1", "  2  ", "0x10", "", "abc", true, false, null, undefined, [], [3], {}, new Number(4)];
for (var c = 0; c < CO.length; ++c)
  print("abs(" + JSON.stringify(CO[c]) + ")=" + N(Math.abs(CO[c])) +
    " floor=" + N(Math.floor(CO[c])) + " sqrt=" + N(Math.sqrt(CO[c])));
print("no-arg: abs=" + N(Math.abs()) + " floor=" + N(Math.floor()) + " pow=" + N(Math.pow()) +
  " atan2=" + N(Math.atan2()) + " round=" + N(Math.round()));

print("== random (range only, plus values which are deterministic here) ==");
var allin = true, seen = [];
for (var q = 0; q < 100; ++q) {
  var rv = Math.random();
  if (!(rv >= 0 && rv < 1)) allin = false;
  if (q < 10) seen.push(N(rv));
}
print("random in [0,1): " + allin);
print("first ten: " + seen.join(" "));
"##;

#[test]
fn cfg92_math() {
    set_tz("UTC");
    let mut rng = Rng::new(0x1234_5678_9abc_def0);
    let mut v: Vec<f64> = Vec::new();
    for i in 0..500 {
        /* mixture of "nice" doubles and full bit patterns */
        if i % 5 == 4 {
            let b = rng.bits_f64();
            v.push(b);
        } else {
            v.push(rng.nice_f64());
        }
    }
    let src = cs(JS_MATH);
    diff("c92_math", |api| unsafe {
        let J = boot(api, 0);
        push_num_array(api, J, "V", &v);
        let rc = (api.js_dostring)(J, src.as_ptr());
        finish(api, J, rc);
    });
}

/* ===================================================================== 93 == */

const JS_STRING: &str = r##"
function T(f) { try { return f(); } catch (e) { return "throw:" + e.name + ":" + e.message; } }
function S(x) { return "" + x; }
function Q(x) { return T(function () { return JSON.stringify(x); }); }

var IDX = [-1000, -2, -1, 0, 1, 2, 3, 5, 100, 1.7, NaN, "1", undefined];

/* mujs omits trailing non-participating capture groups, so the replacement
 * function must inspect `arguments` rather than named parameters. */
function repdump() {
  var a = [];
  for (var i = 0; i < arguments.length; ++i) a.push((typeof arguments[i]) + ":" + S(arguments[i]));
  return "{" + arguments.length + ":" + a.join("|") + "}";
}

function strtests(tag, s) {
  var i;
  print(tag + " q=" + Q(s) + " len=" + s.length);
  var o = "";
  for (i = 0; i < IDX.length; ++i)
    o += " [" + S(IDX[i]) + "]=" + Q(s.charAt(IDX[i])) + "/" + S(s.charCodeAt(IDX[i]));
  print("  chars" + o);
  print("  upper=" + Q(s.toUpperCase()) + " lower=" + Q(s.toLowerCase()));
  print("  trim=" + Q(s.trim()) + " tlen=" + s.trim().length);
  print("  slice=" + Q(s.slice(1)) + " " + Q(s.slice(-2)) + " " + Q(s.slice(1, -1)) + " " +
    Q(s.slice(-5, -1)) + " " + Q(s.slice(100, 200)) + " " + Q(s.slice(2, 1)) + " " + Q(s.slice()));
  print("  substring=" + Q(s.substring(1)) + " " + Q(s.substring(-3)) + " " + Q(s.substring(1, -1)) +
    " " + Q(s.substring(3, 1)) + " " + Q(s.substring(0, 100)) + " " + Q(s.substring(NaN, 2)));
  /* substr is Annex B and absent from mujs: the (identical) throw is the result */
  print("  substr=" + S(T(function () {
    return Q(s.substr(1)) + " " + Q(s.substr(-2)) + " " + Q(s.substr(1, 2)) + " " +
      Q(s.substr(-3, 2)) + " " + Q(s.substr(0, -1)) + " " + Q(s.substr(100, 5)) + " " + Q(s.substr());
  })));
  print("  indexOf=" + s.indexOf("a") + " " + s.indexOf("a", 2) + " " + s.indexOf("a", -5) + " " +
    s.indexOf("") + " " + s.indexOf("", 3) + " " + s.indexOf("ab") + " " + s.indexOf("zz"));
  print("  lastIndexOf=" + s.lastIndexOf("a") + " " + s.lastIndexOf("a", 2) + " " +
    s.lastIndexOf("") + " " + s.lastIndexOf("ab") + " " + s.lastIndexOf("a", NaN));
  print("  split0=" + Q(s.split("")) + " splita=" + Q(s.split("a")));
  print("  splitre=" + Q(s.split(/[aeiou]/)) + " splitre2=" + Q(s.split(/(a)|(b)/)));
  print("  splitlim=" + Q(s.split("", 3)) + " " + Q(s.split(/a/, 2)) + " " + Q(s.split("a", 0)) +
    " " + Q(s.split(undefined)) + " " + Q(s.split(s)));
  print("  repl$=" + Q(s.replace("a", "[$&|$1|$$|$'|$`]")));
  print("  replre=" + Q(s.replace(/(a)(b)?/g, "<$&:$1:$2:$$:$`:$':$0:$99>")));
  print("  replfn=" + Q(s.replace(/[ab]/g, repdump)));
  print("  replfn2=" + Q(s.replace(/(a)(b)?/, repdump)));
  print("  replfn3=" + Q(s.replace("a", repdump)));
  print("  match=" + Q(s.match(/[a-z]+/g)) + " " + Q(s.match(/(a)(b)/)) + " " + Q(s.match("a")));
  print("  search=" + s.search(/b/) + " " + s.search(/zzz/) + " " + s.search("a") + " " + s.search(/^/));
  print("  concat=" + Q(s.concat("-", 1, null, undefined)) + " " + Q(s.concat()));
  print("  localeCompare=" + s.localeCompare("abc") + " " + s.localeCompare(s) + " " +
    s.localeCompare("") + " " + s.localeCompare("é"));
  print("  charCodeAt sweep=" + (function () {
    var a = [];
    for (var j = 0; j < s.length; ++j) a.push(s.charCodeAt(j));
    return a.join(",");
  })());
}

var FIX = ["", "a", "abc", "ABC", "aBcDeF", "hello world", " \t\n trim me \r\n ",
  "ß", "ﬀ", "İ", "ı", "Σςσ", "éÉ",
  "中文", "😀", "abcabcabc", "aaa", "  ", "x,y,,z,", "a.b.c",
  "The quick brown fox", "Å", "12345", "-0", "NaN", " nbsp ",
  "ab", "ba", "aab", "b", "ßstraßeß", "MASSE masse Maße",
  "Ισως", "i̇", "Ǆǅǆ", "a\u0000b"];
print("== fixed strings ==");
for (var f = 0; f < FIX.length; ++f) strtests("fix[" + f + "]", FIX[f]);

print("== case mapping specials ==");
var CM = ["ß", "ﬀ", "ﬃ", "İ", "ı", "Σ", "σ", "ς",
  "ͅ", "ẞ", "ΐ", "և", "ŉ", "ǰ", "ẖ",
  "ςς", "aς", "ςa", "Ϊ́"];
for (var m = 0; m < CM.length; ++m)
  print("cm[" + m + "] " + Q(CM[m]) + " len=" + CM[m].length +
    " upper=" + Q(CM[m].toUpperCase()) + " ulen=" + CM[m].toUpperCase().length +
    " lower=" + Q(CM[m].toLowerCase()) + " llen=" + CM[m].toLowerCase().length);

print("== fromCharCode ==");
var CC = [[], [0], [65], [65, 66, 67], [0xffff], [0x10000], [0x1f600], [-1], [65.9], [NaN],
  [1e10], [0xd83d, 0xde00], [0x110000], [0x7f], [0x80], [0x7ff], [0x800], [0xfffd],
  ["65"], [true], [null], [undefined], [65, -1, 66]];
for (var cc = 0; cc < CC.length; ++cc)
  print("fromCharCode(" + CC[cc].join(",") + ")=" +
    Q(T(function () { return String.fromCharCode.apply(String, CC[cc]); })) + " len=" +
    S(T(function () { return String.fromCharCode.apply(String, CC[cc]).length; })));
print("fromCharCode.length=" + String.fromCharCode.length);

print("== replace corner cases ==");
var R0 = "abcabc";
function PT(label, f) { print(label + " -> " + S(T(f))); }
PT("r1", function () { return Q(R0.replace("b", "$&$&")); });
PT("r2", function () { return Q(R0.replace("b", "$`")); });
PT("r3", function () { return Q(R0.replace("b", "$'")); });
PT("r4", function () { return Q(R0.replace("b", "$$")); });
PT("r5", function () { return Q(R0.replace("b", "$1")); });
PT("r6", function () { return Q(R0.replace("b", "$")); });
PT("r7", function () { return Q(R0.replace(/b/g, "X")); });
PT("r8", function () { return Q(R0.replace(/(b)/g, "[$1]")); });
PT("r9", function () { return Q(R0.replace(/(a)(b)(c)/g, "$3$2$1")); });
PT("r10", function () { return Q(R0.replace(/x/g, "Y")); });
PT("r11", function () { return Q(R0.replace(/()/g, "-")); });
PT("r12", function () { return Q(R0.replace(/c?/g, "*")); });
PT("r13", function () { return Q("".replace(/^/, "S")); });
PT("r14", function () { return Q(R0.replace(/b/, function () { return "$&"; })); });
PT("r15", function () { return Q(R0.replace(/(a)|(z)/g, repdump)); });
PT("r16", function () { return Q(R0.replace(/b/g, undefined)); });
PT("r17", function () { return Q(R0.replace(/b/g, 5)); });
PT("r18", function () { return Q(R0.replace(/b/g, null)); });
PT("r19", function () { return Q(R0.replace()); });
PT("r20", function () { return Q(R0.replace(/(a)(b)(c)(a)(b)(c)/, "$6$5$4$3$2$1")); });
print("== split corner cases ==");
PT("s1", function () { return Q("".split("")); });
PT("s2", function () { return Q("".split("a")); });
PT("s3", function () { return Q("".split(/a/)); });
PT("s4", function () { return Q("abc".split(/(?:)/)); });
PT("s5", function () { return Q("a,b,c".split(",", -1)); });
PT("s6", function () { return Q("a,b,c".split(",", 1e10)); });
PT("s7", function () { return Q("a,b,c".split(",", NaN)); });
PT("s8", function () { return Q("A<B>C".split(/(<)(>)?/)); });
PT("s9", function () { return Q("ab".split(/a*?/)); });
PT("s10", function () { return Q("ab".split(/a*/)); });
PT("s11", function () { return Q("a,b".split()); });
PT("s12", function () { return Q("abc".split(/b/g)); });
print("== length with non-ascii ==");
var LN = ["é", "中", "😀", "aéb", "ééé",
  "߿", "ࠀ", "￿", "ჿ", "abcé中😀"];
for (var l = 0; l < LN.length; ++l)
  print("len " + Q(LN[l]) + " = " + LN[l].length + " cc0=" + LN[l].charCodeAt(0) +
    " up=" + Q(LN[l].toUpperCase()) + " sl=" + Q(LN[l].slice(1)));
print("== random strings ==");
for (var rr = 0; rr < R.length; ++rr) strtests("rnd[" + rr + "]", R[rr]);
"##;

fn rand_string(rng: &mut Rng) -> String {
    const ASCII: &[u8] = b"abcABC 019,.;-_\t\n\"'\\/$&`*?+[]{}()|^";
    const UTF: [&str; 12] = [
        "\u{e9}",
        "\u{df}",
        "\u{4e2d}",
        "\u{6587}",
        "\u{1f600}",
        "\u{fb00}",
        "\u{130}",
        "\u{131}",
        "\u{3a3}",
        "\u{3c2}",
        "\u{a0}",
        "\u{2028}",
    ];
    let n = rng.range(18);
    let mut s = String::new();
    for _ in 0..n {
        if rng.range(5) == 0 {
            s.push_str(UTF[rng.range(UTF.len() as u32) as usize]);
        } else {
            s.push(ASCII[rng.range(ASCII.len() as u32) as usize] as char);
        }
    }
    s
}

#[test]
fn cfg93_string() {
    set_tz("UTC");
    let mut rng = Rng::new(0x0bad_c0de_dead_beef);
    let mut r: Vec<CString> = Vec::new();
    while r.len() < 200 {
        let s = rand_string(&mut rng);
        if let Ok(c) = CString::new(s) {
            r.push(c);
        }
    }
    let src = cs(JS_STRING);
    diff("c93_string", |api| unsafe {
        let J = boot(api, 0);
        push_str_array(api, J, "R", &r);
        let rc = (api.js_dostring)(J, src.as_ptr());
        finish(api, J, rc);
    });
}

/* ===================================================================== 94 == */

const JS_ARRAY: &str = r##"
function T(f) { try { return f(); } catch (e) { return "throw:" + e.name + ":" + e.message; } }
function S(x) { return "" + x; }

/* full description of an array: elements, holes, length and own keys */
function A(a) {
  var i, s = [];
  for (i = 0; i < a.length; ++i)
    s.push(i in a ? (typeof a[i]) + ":" + S(a[i]) : "<hole>");
  var k = [];
  for (var p in a) k.push(p);
  return "[" + s.join(",") + "] len=" + a.length + " keys=" + k.join("|");
}
function C(a) { return a.slice ? a.slice(0) : a; }
function mk(src) { return eval(src); }

function cmpnum(x, y) { return x - y; }
function cmpdesc(x, y) { return y - x; }
function cmpstr(x, y) { return String(x) < String(y) ? -1 : String(x) > String(y) ? 1 : 0; }
function cmpzero() { return 0; }
function cmpbad() { return NaN; }

function arraytests(tag, a) {
  print(tag + " " + A(a));
  print("  sort()=" + S(T(function () { return A(C(a).sort()); })));
  print("  sort(num)=" + S(T(function () { return A(C(a).sort(cmpnum)); })));
  print("  sort(desc)=" + S(T(function () { return A(C(a).sort(cmpdesc)); })));
  print("  sort(str)=" + S(T(function () { return A(C(a).sort(cmpstr)); })));
  print("  sort(zero)=" + S(T(function () { return A(C(a).sort(cmpzero)); })));
  print("  sort(nan)=" + S(T(function () { return A(C(a).sort(cmpbad)); })));
  print("  join=" + S(T(function () { return C(a).join(); })) +
    " join-=" + S(T(function () { return C(a).join("-"); })) +
    " join''=" + S(T(function () { return C(a).join(""); })) +
    " joinNull=" + S(T(function () { return C(a).join(null); })));
  print("  toString=" + S(T(function () { return C(a).toString(); })));
  print("  reverse=" + S(T(function () { return A(C(a).reverse()); })));
  print("  slice=" + S(T(function () { return A(C(a).slice(1)); })) +
    " " + S(T(function () { return A(C(a).slice(-2)); })) +
    " " + S(T(function () { return A(C(a).slice(1, -1)); })) +
    " " + S(T(function () { return A(C(a).slice(5, 2)); })));
  print("  concat=" + S(T(function () { return A(C(a).concat([9, 10], 11, "x")); })));
  print("  indexOf=" + S(T(function () { return C(a).indexOf(1) + "/" + C(a).indexOf(undefined) +
    "/" + C(a).indexOf("a") + "/" + C(a).indexOf(1, 2) + "/" + C(a).lastIndexOf(1) +
    "/" + C(a).lastIndexOf(1, -2); })));
  var sp = [[0, 0], [0, 1], [1, 2], [-1, 1], [-3, 2], [0, 100], [2], [], [1, 0, "X"],
    [-2, 1, "Y", "Z"], [100, 1, "W"], [0, -1, "V"], [NaN, NaN, "U"]];
  for (var i = 0; i < sp.length; ++i) {
    var b = C(a);
    var got = T(function () { return A(b.splice.apply(b, sp[i])); });
    print("  splice(" + sp[i].join(",") + ") removed=" + S(got) + " left=" + A(b));
  }
  var st = C(a);
  print("  push=" + S(T(function () { return st.push(7, 8); })) + " -> " + A(st));
  print("  pop=" + S(T(function () { return S(st.pop()); })) + " -> " + A(st));
  print("  shift=" + S(T(function () { return S(st.shift()); })) + " -> " + A(st));
  print("  unshift=" + S(T(function () { return st.unshift("u", "v"); })) + " -> " + A(st));
  print("  push()=" + S(T(function () { return st.push(); })) + " -> " + A(st));
  print("  every=" + S(T(function () { return C(a).every(function (x) { return x !== undefined; }); })) +
    " some=" + S(T(function () { return C(a).some(function (x) { return typeof x === "number"; }); })));
  var fe = [];
  T(function () { C(a).forEach(function (x, i2) { fe.push(i2 + ":" + S(x)); }); });
  print("  forEach=" + fe.join(","));
  print("  map=" + S(T(function () { return A(C(a).map(function (x) { return typeof x; })); })));
  print("  filter=" + S(T(function () { return A(C(a).filter(function (x) { return !!x; })); })));
  print("  reduce=" + S(T(function () { return S(C(a).reduce(function (p, x) { return S(p) + "/" + S(x); })); })));
  print("  reduce0=" + S(T(function () { return S(C(a).reduce(function (p, x) { return S(p) + "/" + S(x); }, "I")); })));
  print("  reduceRight=" + S(T(function () { return S(C(a).reduceRight(function (p, x) { return S(p) + "/" + S(x); })); })));
  print("  reduceRight0=" + S(T(function () { return S(C(a).reduceRight(function (p, x) { return S(p) + "/" + S(x); }, "I")); })));
}

var FIX = [
  "[]", "[1]", "[1,2,3]", "[3,1,2]", "[10,9,80,7]", "[,]", "[,,]", "[1,,3]",
  "[undefined,1,undefined]", "[null,undefined,0,'',false,NaN]",
  "['b','a','c']", "['10','9','80']", "[1,'1',true,null,undefined,{},[]]",
  "[NaN,Infinity,-Infinity,0,-0]", "[{a:1},{a:2}]", "[[3],[1],[2]]",
  "[1,2,3,4,5,6,7,8,9,10]", "[5,4,3,2,1]", "[1,1,1,1]",
  "[2147483647,-2147483648,0.5,-0.5]", "['\\u00e9','a','\\u4e2d']",
  "[true,false,true]", "[function(){},1]", "[new Date(0),new Date(1)]"
];
print("== fixed arrays ==");
for (var f = 0; f < FIX.length; ++f) arraytests("fix[" + f + "] " + FIX[f], mk(FIX[f]));

print("== sparse / flat transitions ==");
function transition(n) {
  var a = [];
  var i;
  for (i = 0; i < n; ++i) a[i] = i;
  print("flat n=" + n + " " + A(a));
  a[n + 1] = "afterhole";
  print("  hole at " + n + ": " + A(a));
  a.named = "N";
  print("  named: " + A(a) + " named=" + a.named);
  a.length = n;
  print("  truncated: " + A(a));
  a.length = n + 4;
  print("  extended: " + A(a));
  delete a[0];
  print("  deleted 0: " + A(a));
  a[0] = "back";
  print("  set 0: " + A(a));
  print("  sort: " + A(a.sort()));
  print("  join: " + a.join("+"));
  print("  splice: " + A(a.splice(1, 2, "s")) + " left " + A(a));
  print("  concat: " + A(a.concat([1, , 2])));
}
transition(0);
transition(1);
transition(3);
transition(8);
transition(17);

print("== length games ==");
var la = [1, 2, 3];
print(S(T(function () { la.length = -1; return A(la); })));
print(S(T(function () { la.length = 4294967296; return A(la); })));
print(S(T(function () { la.length = "2"; return A(la); })));
print(S(T(function () { la.length = 1.5; return A(la); })));
print("array from string index: " + A(mk("['a','b']")));
var na = []; na[4294967294] = "big";
print("huge index: len=" + na.length + " val=" + na[4294967294] + " hasIndex=" + (4294967294 in na));
var xa = []; xa["01"] = "notindex"; xa[1] = "index";
print("non-canonical index: " + A(xa) + " xa['01']=" + xa["01"]);

print("== sort stability / comparator side effects ==");
var so = [{ k: 1, i: "a" }, { k: 1, i: "b" }, { k: 0, i: "c" }, { k: 1, i: "d" }, { k: 0, i: "e" }];
print(so.sort(function (x, y) { return x.k - y.k; }).map(function (x) { return x.k + x.i; }).join(","));
var mut = [3, 1, 2];
print(S(T(function () { return A(mut.sort(function (x, y) { mut.push(99); return x - y; })); })));
print("sort non-callable: " + S(T(function () { return [1, 2].sort(5); })));
print("sort throwing comparator: " + S(T(function () { return [3, 1, 2].sort(function () { throw new Error("cmp"); }); })));

print("== generic / prototype methods ==");
print("isArray: " + [Array.isArray([]), Array.isArray({}), Array.isArray("x")].join(","));
print("Array(3)=" + A(Array(3)) + " Array(1,2)=" + A(Array(1, 2)) + " Array('3')=" + A(Array("3")));
print("Array(-1): " + S(T(function () { return A(Array(-1)); })));
print("join on arraylike: " + S(T(function () { return Array.prototype.join.call({ length: 3, 0: "a", 2: "c" }, "-"); })));
print("slice on string: " + S(T(function () { return A(Array.prototype.slice.call("abc")); })));
print("push on arraylike: " + S(T(function () { var o = { length: 1, 0: "z" }; Array.prototype.push.call(o, "q"); return o.length + "/" + o[1]; })));
print("concat spreadable: " + A([1].concat([2, [3, 4]], 5)));

print("== random arrays ==");
for (var r = 0; r < SRCS.length; ++r) arraytests("rnd[" + r + "] " + SRCS[r], mk(SRCS[r]));
"##;

fn rand_array_src(rng: &mut Rng) -> String {
    let n = rng.range(13);
    let mut s = String::from("[");
    for i in 0..n {
        if i > 0 {
            s.push(',');
        }
        match rng.range(13) {
            0 => s.push_str(&format!("{}", rng.range(100))),
            1 => s.push_str(&format!("-{}", rng.range(100))),
            2 => s.push_str(&format!("{}.{}", rng.range(10), rng.range(1000))),
            3 => s.push_str(&format!("\"{}\"", ["a", "b", "abc", "", "10", "9", "\\u00e9"][rng.range(7) as usize])),
            4 => s.push_str("null"),
            5 => s.push_str("undefined"),
            6 => s.push_str(if rng.range(2) == 0 { "true" } else { "false" }),
            7 => { /* hole */ }
            8 => s.push_str(if rng.range(2) == 0 { "[]" } else { "[1,2]" }),
            9 => s.push_str(if rng.range(2) == 0 { "{}" } else { "{a:1}" }),
            10 => s.push_str(["NaN", "Infinity", "-Infinity", "-0"][rng.range(4) as usize]),
            11 => s.push_str(&format!("\"{}\"", rng.range(100))),
            _ => s.push_str(&format!("{}e{}", 1 + rng.range(9), rng.range(20))),
        }
    }
    s.push(']');
    s
}

#[test]
fn cfg94_array() {
    set_tz("UTC");
    let mut rng = Rng::new(0x00c0_ffee_1234_5678);
    let mut srcs: Vec<CString> = Vec::new();
    for _ in 0..100 {
        srcs.push(CString::new(rand_array_src(&mut rng)).unwrap());
    }
    let src = cs(JS_ARRAY);
    diff("c94_array", |api| unsafe {
        let J = boot(api, 0);
        push_str_array(api, J, "SRCS", &srcs);
        let rc = (api.js_dostring)(J, src.as_ptr());
        finish(api, J, rc);
    });
}

