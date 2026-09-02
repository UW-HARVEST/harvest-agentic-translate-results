//! Phase B rows 40b / 78: output the C library emits outside the normal return
//! path — `js_gc(J,1)` statistics (routed through `js_report`), the `js_trap`
//! stack dump and `jsS_dumpstrings` (both `printf` to stdout).
//!
//! The `js_trap` / `jsS_dumpstrings` tests redirect the process-wide fd 1, so
//! this binary MUST run with `--test-threads=1` (see `run_all.sh`).
mod common;
use common::*;
use std::os::raw::c_int;

fn digits_masked(s: &str) -> Vec<String> {
    s.lines()
        .map(|l| {
            l.chars()
                .map(|c| if c.is_ascii_digit() { '#' } else { c })
                .collect()
        })
        .collect()
}

/// Replace `0x…` heap addresses with a placeholder: the two libraries
/// necessarily get different addresses from `malloc`.
fn addr_masked(s: &str) -> String {
    let mut out = String::new();
    let b: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < b.len() {
        if b[i] == '0' && i + 1 < b.len() && b[i + 1] == 'x' {
            out.push_str("0xADDR");
            i += 2;
            while i < b.len() && b[i].is_ascii_hexdigit() {
                i += 1;
            }
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    out
}

/// `js_gc(J, report)` sends its statistics line through `js_report`, NOT to
/// stdout, so it is captured with the report callback.
fn gc_reports(api: &Api, script: Option<&str>, gcs: usize, report: c_int) -> Vec<String> {
    unsafe {
        let _ = take_reports();
        let j = (api.js_newstate)(None, std::ptr::null_mut(), 0);
        (api.js_setreport)(j, Some(report_cb));
        if let Some(src) = script {
            let csrc = cs(src);
            let fname = cs("[string]");
            if (api.js_ploadstring)(j, fname.as_ptr(), csrc.as_ptr()) == 0 {
                (api.js_pushundefined)(j);
                (api.js_pcall)(j, 0);
            }
            (api.js_pop)(j, 1);
        }
        for _ in 0..gcs {
            (api.js_gc)(j, report);
        }
        (api.js_freestate)(j);
        take_reports()
    }
}

#[test]
fn row40b_gc_report_text() {
    let p = pair();
    let script = "var a=[]; for (var i=0;i<50;++i) a.push({x:i}); a=null;";
    let a = gc_reports(&p.c, Some(script), 1, 1);
    let b = gc_reports(&p.r, Some(script), 1, 1);
    assert!(
        !a.is_empty() && a[0].contains("garbage collected"),
        "the C GC report must be captured, got {a:?}"
    );
    assert_eq!(a, b, "js_gc(J,1) report text");
}

#[test]
fn row40c_gc_report_exact_counts_fresh_state() {
    // On a freshly booted state the object graph is fully determined by the
    // built-ins, so every count must match exactly.
    let p = pair();
    let a = gc_reports(&p.c, None, 3, 1);
    let b = gc_reports(&p.r, None, 3, 1);
    assert_eq!(a.len(), 3, "expected one report per js_gc call, got {a:?}");
    assert_eq!(a, b, "js_gc(J,1) exact report on a fresh state");
}

#[test]
fn row40d_gc_report_flag_domain() {
    // `report` is a plain int: 0 means silent, everything else prints.
    let p = pair();
    for report in [0 as c_int, 1, 2, -1, i32::MAX, i32::MIN] {
        let a = gc_reports(&p.c, Some("var o={};"), 2, report);
        let b = gc_reports(&p.r, Some("var o={};"), 2, report);
        assert_eq!(a, b, "js_gc report={report}");
        if report == 0 {
            assert!(a.is_empty(), "report=0 must be silent, got {a:?}");
        } else {
            assert_eq!(a.len(), 2, "report={report} must print, got {a:?}");
        }
    }
}

#[test]
fn row40e_gc_report_after_varied_workloads() {
    let p = pair();
    for script in [
        "var a=[]; for(var i=0;i<200;++i) a.push({x:i}); a=null;",
        "var s=''; for(var i=0;i<200;++i) s+=String(i); s=null;",
        "var f=function(){return 1}; f=null;",
        "var r=/a(b)c/g; r.exec('abc'); r=null;",
        "var o={}; for(var i=0;i<100;++i) o['k'+i]={n:i}; o=null;",
        "(function(){ var o={}; o.self=o; })();",
        "try { throw new Error('x') } catch(e) {}",
        "var a=[1,2,3]; a[100]=1; a=null;",
    ] {
        let a = gc_reports(&p.c, Some(script), 2, 1);
        let b = gc_reports(&p.r, Some(script), 2, 1);
        assert_eq!(a.len(), 2, "expected 2 reports for {script:?}");
        assert_eq!(a, b, "js_gc report after {script:?}");
    }
}

#[test]
fn row78_trap_dump() {
    let p = pair();
    for pc in [0 as c_int, 1, -1, 12345] {
        let mut outs = Vec::new();
        for api in [&p.c, &p.r] {
            let (_, out) = capture_stdout(|| unsafe {
                let j = (api.js_newstate)(None, std::ptr::null_mut(), 0);
                // build a non-trivial stack before dumping
                (api.js_pushnumber)(j, 1.5);
                let s = cs("hello");
                (api.js_pushstring)(j, s.as_ptr());
                (api.js_pushboolean)(j, 1);
                (api.js_pushnull)(j);
                (api.js_pushundefined)(j);
                (api.js_newarray)(j);
                (api.js_newobject)(j);
                (api.js_pushglobal)(j);
                (api.js_trap)(j, pc);
                (api.js_freestate)(j);
            });
            assert!(
                out.contains("stack {"),
                "js_trap must print to stdout, got {out:?}"
            );
            outs.push(addr_masked(&out));
        }
        assert_eq!(outs[0], outs[1], "js_trap(J,{pc}) dump");
    }
}

#[test]
fn row78b_trap_inside_script() {
    let p = pair();
    let mut outs = Vec::new();
    for api in [&p.c, &p.r] {
        let (_, out) = capture_stdout(|| unsafe {
            let j = (api.js_newstate)(None, std::ptr::null_mut(), 0);
            let src = cs("function f(a,b){ debugger; return a+b } f(1,2)");
            let fname = cs("[string]");
            if (api.js_ploadstring)(j, fname.as_ptr(), src.as_ptr()) == 0 {
                (api.js_pushundefined)(j);
                (api.js_pcall)(j, 0);
            }
            (api.js_freestate)(j);
        });
        assert!(
            out.contains("stack trace:"),
            "the debugger statement must dump a stack trace, got {out:?}"
        );
        outs.push(addr_masked(&out));
    }
    assert_eq!(outs[0], outs[1], "debugger statement -> js_trap output");
}

#[test]
fn dumpstrings() {
    let p = pair();
    let mut outs = Vec::new();
    for api in [&p.c, &p.r] {
        let (_, out) = capture_stdout(|| unsafe {
            let j = (api.js_newstate)(None, std::ptr::null_mut(), 0);
            for s in ["alpha", "beta", "gamma", "a", "", "zzzz", "\u{e9}"] {
                let cstr = cs(s);
                (api.js_intern)(j, cstr.as_ptr());
            }
            (api.jsS_dumpstrings)(j);
            (api.js_freestate)(j);
        });
        assert!(
            !out.trim().is_empty(),
            "jsS_dumpstrings must print something"
        );
        outs.push(digits_masked(&out));
    }
    assert_eq!(outs[0], outs[1], "jsS_dumpstrings output");
}
