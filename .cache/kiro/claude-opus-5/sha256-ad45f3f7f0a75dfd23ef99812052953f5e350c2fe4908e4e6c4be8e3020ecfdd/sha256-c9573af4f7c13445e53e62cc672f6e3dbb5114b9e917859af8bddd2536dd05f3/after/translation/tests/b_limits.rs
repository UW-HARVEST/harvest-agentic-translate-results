//! Boundary sweeps around the compile-time / runtime limits, and around the
//! control-character escaping in `jsrepr.c`.
//!
//! These exist because a mutation sweep (`mutants.sh`) showed that testing a
//! limit only far past its threshold does not pin the threshold down: the
//! *exact* value at which the C flips from accept to reject has to be compared.
mod common;
use common::*;
use std::os::raw::{c_char, c_int};

/// `JS_ASTLIMIT` is 400 nested expressions (`jsparse.c` INCREC).
#[test]
fn ast_limit_boundary() {
    let mut srcs = Vec::new();
    for n in 1..=420 {
        srcs.push(format!("{}1{}", "(".repeat(n), ")".repeat(n)));
    }
    diff_eval_batch("nested parenthesis depth", &srcs, 0);
    diff_eval_batch("nested parenthesis depth (strict)", &srcs, JS_STRICT);

    // The same limit is reached through other nesting shapes.
    let mut srcs = Vec::new();
    for n in 390..=415 {
        srcs.push(format!("{}1{}", "-".repeat(n), ""));
        srcs.push(format!("{}1{}", "!".repeat(n), ""));
        srcs.push(format!("{}1{}", "typeof ".repeat(n), ""));
        srcs.push(format!("[{}1{}]", "[".repeat(n), "]".repeat(n)));
        srcs.push(format!("({}1{})", "{a:".repeat(n).replace("1", ""), "}".repeat(n)));
        srcs.push(format!("1{}", "+1".repeat(n)));
        srcs.push(format!("a{}", ".b".repeat(n)));
        srcs.push(format!("f{}", "()".repeat(n)));
    }
    diff_eval_batch("nested expression shapes", &srcs, 0);
}

/// `JS_ASTLIMIT` also guards statement nesting.
#[test]
fn statement_nesting_boundary() {
    let mut srcs = Vec::new();
    for n in 390..=412 {
        srcs.push(format!("{}1{}", "if(1){".repeat(n), "}".repeat(n)));
        srcs.push(format!("{}{}", "while(0){".repeat(n), "}".repeat(n)));
        srcs.push(format!("{}{}", "for(;;){break;".repeat(n), "}".repeat(n)));
        srcs.push(format!("{}{}", "{".repeat(n), "}".repeat(n)));
    }
    diff_eval_batch("nested statement depth", &srcs, 0);
}

/// `JS_TRYLIMIT` is 64.  Two distinct checks share it:
///  * `js_pushtry` (jsrun.c) — reached by entering nested `try` blocks;
///  * `js_ptry` (jsstate.c) — reached when `js_ploadstring` / `js_dostring` is
///    called while 64 try frames are already live.
#[test]
fn try_limit_boundary_from_script() {
    let mut srcs = Vec::new();
    for n in 1..=80 {
        let mut s = String::new();
        for _ in 0..n {
            s.push_str("try{");
        }
        s.push_str("var d=0");
        for i in 0..n {
            s.push_str(&format!("}}catch(e{i}){{d=d+1}}"));
        }
        srcs.push(format!("(function(){{ {s}; return 'ok'+d }})()"));
    }
    diff_eval_batch("nested try depth", &srcs, 0);

    // ... and with a throw at the bottom so every handler actually runs.
    let mut srcs = Vec::new();
    for n in 1..=80 {
        let mut s = String::new();
        for _ in 0..n {
            s.push_str("try{");
        }
        s.push_str("throw 0");
        for i in 0..n {
            s.push_str(&format!("}}catch(e{i}){{throw e{i}+1}}"));
        }
        srcs.push(format!("try{{{s}}}catch(e){{'caught:'+e}}"));
    }
    diff_eval_batch("nested try depth with throw", &srcs, 0);

    // finally blocks push try frames too
    let mut srcs = Vec::new();
    for n in 1..=80 {
        let mut s = String::new();
        for _ in 0..n {
            s.push_str("try{");
        }
        s.push_str("1");
        for _ in 0..n {
            s.push_str("}finally{}");
        }
        srcs.push(format!("try{{{s}}}catch(e){{'caught:'+e}}"));
    }
    diff_eval_batch("nested try/finally depth", &srcs, 0);
}

/// `js_ptry` in jsstate.c: call `js_ploadstring` from a host function invoked at
/// a controlled `try` nesting depth and compare the return code at every depth.
#[test]
fn try_limit_boundary_from_ploadstring() {
    let p = pair();
    for depth in 0..80usize {
        let mut outs = Vec::new();
        for (which, api) in [(0usize, &p.c), (1usize, &p.r)] {
            let tramp: unsafe extern "C-unwind" fn(JS) =
                if which == 0 { probe_load_c } else { probe_load_r };
            unsafe {
                let _ = take_reports();
                let j = (api.js_newstate)(None, std::ptr::null_mut(), 0);
                (api.js_setreport)(j, Some(report_cb));
                (api.js_newcfunction)(j, Some(tramp), c"probe".as_ptr(), 0);
                (api.js_setglobal)(j, c"probe".as_ptr());
                let mut s = String::new();
                for _ in 0..depth {
                    s.push_str("try{");
                }
                s.push_str("probe()");
                for _ in 0..depth {
                    s.push_str("}catch(e){throw e}");
                }
                let fname = cs("[string]");
                let csrc = cs(&format!("try{{{s}}}catch(e){{'caught:'+String(e)}}"));
                let mut rc = (api.js_ploadstring)(j, fname.as_ptr(), csrc.as_ptr());
                if rc == 0 {
                    (api.js_pushundefined)(j);
                    if (api.js_pcall)(j, 0) != 0 {
                        rc = 2;
                    }
                } else {
                    rc = 1;
                }
                let fb = cs("<throw>");
                let v = rstr((api.js_trystring)(j, -1, fb.as_ptr()));
                (api.js_freestate)(j);
                outs.push((rc, v, take_reports(), PROBE.with(|x| x.take())));
            }
        }
        assert_eq!(outs[0], outs[1], "js_ptry at try depth {depth}");
    }
}

thread_local! {
    static PROBE: std::cell::Cell<i32> = const { std::cell::Cell::new(0) };
}

unsafe fn probe_load(api: &Api, j: JS) {
    unsafe {
        let fname = cs("[inner]");
        let src = cs("1+1");
        let rc = (api.js_ploadstring)(j, fname.as_ptr(), src.as_ptr());
        PROBE.with(|x| x.set(rc));
        if rc == 0 {
            (api.js_pushundefined)(j);
            (api.js_pcall)(j, 0);
        }
    }
}

unsafe extern "C-unwind" fn probe_load_c(j: JS) {
    unsafe { probe_load(&pair().c, j) }
}
unsafe extern "C-unwind" fn probe_load_r(j: JS) {
    unsafe { probe_load(&pair().r, j) }
}

/// `JS_STRLIMIT` is `1<<28`.  The rejection is `n > JS_STRLIMIT`, so the
/// threshold has to be probed at exactly `JS_STRLIMIT + 1`.
const JS_STRLIMIT: usize = 1 << 28;

#[test]
fn string_limit_boundary() {
    // js_pushlstring only compares `n` against the limit, so the boundary can
    // be probed without a huge buffer.
    for n in [
        JS_STRLIMIT as c_int,
        JS_STRLIMIT as c_int + 1,
        JS_STRLIMIT as c_int + 2,
    ] {
        // n == JS_STRLIMIT would proceed to copy n bytes out of a 3-byte buffer
        // (undefined behaviour in the C), so only the rejecting side is probed.
        if n <= JS_STRLIMIT as c_int {
            continue;
        }
        diff_protected(&format!("js_pushlstring n={n}"), 0, || {
            move |api: &Api, j: JS| unsafe {
                (api.js_pushlstring)(j, c"abc".as_ptr(), n);
                log("unreached");
            }
        });
    }

    // js_pushstring / js_intern use strlen(), so they need a real buffer whose
    // length is exactly JS_STRLIMIT + 1 (256 MiB, allocated once).
    let buf: Vec<c_char> = {
        let mut v = vec![b'a' as c_char; JS_STRLIMIT + 1];
        v.push(0);
        v
    };
    let buf = std::sync::Arc::new(buf);
    {
        let b = buf.clone();
        diff_protected("js_pushstring at JS_STRLIMIT+1", 0, move || {
            let b = b.clone();
            move |api: &Api, j: JS| unsafe {
                (api.js_pushstring)(j, b.as_ptr());
                log("unreached");
            }
        });
    }
    {
        let b = buf.clone();
        diff_protected("js_intern at JS_STRLIMIT+1", 0, move || {
            let b = b.clone();
            move |api: &Api, j: JS| unsafe {
                let s = (api.js_intern)(j, b.as_ptr());
                log(format!("unreached {}", s as usize));
            }
        });
    }
}

/// `JS_ARRAYLIMIT` is `1<<26`; `jsR_setproperty` rejects `newlen > limit`.
#[test]
fn array_limit_boundary() {
    let limit: i64 = 1 << 26;
    let mut srcs = Vec::new();
    for d in [-2i64, -1, 0, 1, 2] {
        srcs.push(format!("try{{ var a=[]; a.length={}; String(a.length) }}catch(e){{'E:'+e}}", limit + d));
        srcs.push(format!("try{{ var a=[]; a[{}]=1; String(a.length) }}catch(e){{'E:'+e}}", limit + d));
        srcs.push(format!("try{{ String(new Array({}).length) }}catch(e){{'E:'+e}}", limit + d));
    }
    // and around the uint32 array-index boundary
    for v in [
        "4294967293", "4294967294", "4294967295", "4294967296", "2147483647", "2147483648",
        "-1", "0", "1", "1.5",
    ] {
        srcs.push(format!("try{{ var a=[]; a.length={v}; String(a.length) }}catch(e){{'E:'+e}}"));
        srcs.push(format!("try{{ var a=[]; a[{v}]=1; String(a.length)+':'+String(a[{v}]) }}catch(e){{'E:'+e}}"));
        srcs.push(format!("try{{ String(new Array({v}).length) }}catch(e){{'E:'+e}}"));
    }
    diff_eval_batch("array length boundaries", &srcs, 0);
}

/// `jsrepr.c` escapes control characters with `if (c < ' ')`; the threshold has
/// to be probed at 0x1f / 0x20 exactly.
#[test]
fn repr_control_characters() {
    let mut srcs = Vec::new();
    for c in 0u32..=0x22 {
        if c == 0 {
            continue; // a NUL cannot be embedded in a C source string
        }
        srcs.push(format!("JSON.stringify('a\\u{c:04x}b')"));
        srcs.push(format!("'a\\u{c:04x}b'.length"));
        srcs.push(format!("escape('a\\u{c:04x}b')"));
        srcs.push(format!("encodeURIComponent('a\\u{c:04x}b')"));
    }
    for c in [0x7eu32, 0x7f, 0x80, 0xa0, 0xff, 0x100, 0x2028, 0x2029, 0xd7ff, 0xe000, 0xfffd, 0xffff] {
        srcs.push(format!("JSON.stringify('a\\u{c:04x}b')"));
        srcs.push(format!("escape('a\\u{c:04x}b')"));
        srcs.push(format!("encodeURIComponent('a\\u{c:04x}b')"));
    }
    diff_eval_batch("control characters in JSON/escape", &srcs, 0);

    // js_repr / js_torepr over the same alphabet
    let p = pair();
    for c in 1u32..=0x22 {
        let setup = format!("'x\\u{c:04x}y'");
        let mut outs = Vec::new();
        for (which, api) in [(0usize, &p.c), (1usize, &p.r)] {
            let setup = setup.clone();
            let out = run_protected(api, which, 0, move |api: &Api, j: JS| unsafe {
                let fname = cs("[string]");
                let src = cs(&format!("({setup})"));
                if (api.js_ploadstring)(j, fname.as_ptr(), src.as_ptr()) == 0 {
                    (api.js_pushundefined)(j);
                    (api.js_pcall)(j, 0);
                }
                log(format!("torepr={:?}", rstr((api.js_torepr)(j, -1))));
                (api.js_repr)(j, -1);
                log(describe(api, j, -1));
            });
            outs.push(out);
        }
        assert_eq!(outs[0], outs[1], "repr of control char {c:#04x}");
    }
    // objects and arrays containing control characters, plus key escaping
    let mut srcs = Vec::new();
    for c in 1u32..=0x22 {
        srcs.push(format!("JSON.stringify({{'k\\u{c:04x}':'v\\u{c:04x}'}})"));
        srcs.push(format!("JSON.stringify(['a\\u{c:04x}'])"));
        srcs.push(format!("JSON.stringify({{a:'\\u{c:04x}'}},null,2)"));
    }
    diff_eval_batch("control characters in objects", &srcs, 0);
}

/// `REG_MAXREC` (4096), `REG_MAXPROG` (32<<10), `REG_MAXCLASS` (128),
/// `REG_MAXSPAN` (64 runes = 32 ranges) and `REG_MAXSUB` (16) boundaries.
#[test]
fn regexp_limit_boundaries() {
    let p = pair();
    let mut pats: Vec<String> = Vec::new();
    // REG_MAXREC through the P_CAT chain length
    for n in [4093usize, 4094, 4095, 4096, 4097, 4098] {
        pats.push("a".repeat(n));
    }
    // REG_MAXCLASS
    for n in [126usize, 127, 128, 129, 130] {
        pats.push("[a-b]".repeat(n));
    }
    // REG_MAXSUB
    for n in [14usize, 15, 16, 17, 18] {
        pats.push("(a)".repeat(n));
    }
    // REG_MAXSPAN: 64 runes == 32 disjoint ranges
    for n in [30usize, 31, 32, 33, 34] {
        let mut s = String::from("[");
        for i in 0..n {
            s.push_str(&format!("\\u{:04x}-\\u{:04x}", 0x100 + i * 4, 0x100 + i * 4 + 1));
        }
        s.push(']');
        pats.push(s);
    }
    // back-reference numbering boundary
    for n in 0..20 {
        pats.push(format!("{}\\{}", "(a)".repeat(3), n));
    }
    // quantifier count boundaries
    for n in ["0", "1", "254", "255", "256", "65535", "65536", "2147483647", "4294967295", "4294967296"] {
        pats.push(format!("a{{{n}}}"));
        pats.push(format!("a{{{n},}}"));
        pats.push(format!("a{{0,{n}}}"));
    }
    for pat in pats {
        let mut outs = Vec::new();
        for api in [&p.c, &p.r] {
            unsafe {
                let cp = cbuf(pat.as_bytes());
                let mut ep: *const c_char = std::ptr::null();
                let prog = (api.js_regcomp)(cp.as_ptr(), 0, &mut ep);
                let mut r = format!("null={} err={}", prog.is_null(), rstr(ep));
                if !prog.is_null() {
                    let subj = cbuf(b"aaaaaaaaaaaaaaaaaaaaaab");
                    let mut sub = Resub::default();
                    let rc = (api.js_regexec)(prog, subj.as_ptr(), &mut sub, 0);
                    r.push_str(&format!(" exec={rc} nsub={}", sub.nsub));
                    if rc == 0 {
                        let base = subj.as_ptr() as isize;
                        for i in 0..(sub.nsub.clamp(0, 16) as usize) {
                            let sp = sub.sub[i].sp;
                            let e = sub.sub[i].ep;
                            r.push_str(&format!(
                                " [{}]={},{}",
                                i,
                                if sp.is_null() { -1 } else { sp as isize - base },
                                if e.is_null() { -1 } else { e as isize - base }
                            ));
                        }
                    }
                    (api.js_regfree)(prog);
                }
                outs.push(r);
            }
        }
        assert_eq!(
            outs[0], outs[1],
            "regexp limit boundary for pattern of length {}",
            pat.len()
        );
    }
}

/// `js_setlimit` thresholds, swept one step at a time so the exact budget at
/// which the C flips is pinned down.
#[test]
fn limit_sweep_fine() {
    let p = pair();
    for src in [
        "var i=0; while(i<50) ++i; i",
        "function f(n){return n?f(n-1):0} f(20)",
        "[1,2,3].map(function(x){return x*2}).join(',')",
        "JSON.stringify({a:[1,2,3]})",
    ] {
        for runlimit in 1..80 {
            let mut outs = Vec::new();
            for api in [&p.c, &p.r] {
                unsafe {
                    let j = (api.js_newstate)(None, std::ptr::null_mut(), 0);
                    (api.js_setreport)(j, Some(report_cb));
                    let _ = take_reports();
                    (api.js_setlimit)(j, runlimit, 0);
                    let fname = cs("[string]");
                    let csrc = cs(src);
                    let mut rc = (api.js_ploadstring)(j, fname.as_ptr(), csrc.as_ptr());
                    if rc == 0 {
                        (api.js_pushundefined)(j);
                        if (api.js_pcall)(j, 0) != 0 {
                            rc = 2;
                        }
                    } else {
                        rc = 1;
                    }
                    let fb = cs("<throw>");
                    let v = rstr((api.js_trystring)(j, -1, fb.as_ptr()));
                    (api.js_freestate)(j);
                    outs.push((rc, v, take_reports()));
                }
            }
            assert_eq!(outs[0], outs[1], "runlimit={runlimit} src={src:?}");
        }
        for memlimit in (1..4000).step_by(37) {
            let mut outs = Vec::new();
            for api in [&p.c, &p.r] {
                unsafe {
                    let j = (api.js_newstate)(None, std::ptr::null_mut(), 0);
                    (api.js_setreport)(j, Some(report_cb));
                    let _ = take_reports();
                    (api.js_setlimit)(j, 0, memlimit);
                    let fname = cs("[string]");
                    let csrc = cs(src);
                    let mut rc = (api.js_ploadstring)(j, fname.as_ptr(), csrc.as_ptr());
                    if rc == 0 {
                        (api.js_pushundefined)(j);
                        if (api.js_pcall)(j, 0) != 0 {
                            rc = 2;
                        }
                    } else {
                        rc = 1;
                    }
                    let fb = cs("<throw>");
                    let v = rstr((api.js_trystring)(j, -1, fb.as_ptr()));
                    (api.js_freestate)(j);
                    outs.push((rc, v, take_reports()));
                }
            }
            assert_eq!(outs[0], outs[1], "memlimit={memlimit} src={src:?}");
        }
    }
}

/// Call-stack / environment-stack depth (`JS_ENVLIMIT` = 1024,
/// `JS_STACKSIZE` = 4096) swept one frame at a time.
#[test]
fn call_depth_boundary() {
    let mut srcs = Vec::new();
    for n in [1usize, 100, 500, 1000, 1020, 1021, 1022, 1023, 1024, 1025, 1030, 2000] {
        srcs.push(format!(
            "try{{ (function f(n){{ return n<=0 ? 0 : 1+f(n-1) }})({n}) }}catch(e){{'E:'+String(e)}}"
        ));
    }
    for n in [1usize, 1000, 4090, 4095, 4096, 4097, 5000] {
        srcs.push(format!(
            "try{{ Math.max({}) }}catch(e){{'E:'+String(e)}}",
            (0..n).map(|i| i.to_string()).collect::<Vec<_>>().join(",")
        ));
        srcs.push(format!(
            "try{{ [{}].length }}catch(e){{'E:'+String(e)}}",
            (0..n).map(|i| i.to_string()).collect::<Vec<_>>().join(",")
        ));
    }
    diff_eval_batch("call/stack depth", &srcs, 0);
}
