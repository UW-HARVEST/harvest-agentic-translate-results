//! Differential-test helper: loads ONE shared library (the C build or the Rust
//! build) with `libloading` and replays a script of C-API calls against it.
//!
//! It exists so that the stdin-consuming and process-state-sensitive entry
//! points (`interactive_tokenizer`, the "analyzer not initialized" state, the
//! `stdout` flush at exit) can be compared in a *fresh process* per scenario.
//!
//! Usage: `ffi_runner <library.so> <script-file>`; the library's stdout is left
//! untouched on fd 1, everything the runner itself reports goes to stderr.
//!
//! Script: one command per line, arguments hex-encoded (`load 6162` = "ab").
//!
//!   init | initnull | initstub
//!   load <hex> | analyze <hex> | find <hex> | readfile <hex>
//!   menu | dist | score | stats | next | peek | reset | flush
//!   result <w> <n> <k> <o> <c> <s> <l> <ch>
//!   interactive | interactive_badload | main

#[path = "../tests/common/mod.rs"]
mod common;

use common::*;
use std::ffi::{c_char, c_int};
use std::io::Write;

fn hex(s: &str) -> Vec<u8> {
    let b = s.as_bytes();
    assert!(b.len() % 2 == 0, "odd hex string {:?}", s);
    (0..b.len() / 2)
        .map(|i| {
            let hi = (b[2 * i] as char).to_digit(16).expect("hex") as u8;
            let lo = (b[2 * i + 1] as char).to_digit(16).expect("hex") as u8;
            (hi << 4) | lo
        })
        .collect()
}

extern "C" {
    #[link_name = "exit"]
    fn libc_exit(code: c_int) -> !;
}

static mut STUB_LOAD_RC: c_int = -1;

extern "C" fn stub_next() -> CToken {
    let mut t = CToken::zeroed();
    t.ttype = TOKEN_EOF;
    t
}

extern "C" fn stub_reset() {}

extern "C" fn stub_load(_t: *const c_char) -> c_int {
    unsafe { STUB_LOAD_RC }
}

extern "C" fn stub_stats(l: *mut usize, t: *mut usize, c: *mut usize) {
    unsafe {
        if !l.is_null() {
            *l = 0;
        }
        if !t.is_null() {
            *t = 0;
        }
        if !c.is_null() {
            *c = 0;
        }
    }
}

fn stub_ops() -> COps {
    COps {
        next_token: Some(stub_next),
        peek_token: Some(stub_next),
        reset: Some(stub_reset),
        load_text: Some(stub_load),
        get_stats: Some(stub_stats),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    assert!(
        args.len() == 3,
        "usage: ffi_runner <library.so> <script-file>"
    );
    let api = Api::load("lib", std::path::Path::new(&args[1]));
    let script = std::fs::read_to_string(&args[2]).expect("read script");

    let mut err = std::io::stderr();

    for line in script.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut it = line.split_whitespace();
        let cmd = it.next().unwrap();
        match cmd {
            "init" => (api.analyzer_init)((api.get_tokenizer_ops)()),
            "initnull" => (api.analyzer_init)(COps::null()),
            "initstub" => (api.analyzer_init)(stub_ops()),
            "load" => {
                let arg = cstring(&hex(it.next().unwrap_or("")));
                let rc = (api.tokenizer_load_text)(arg.as_ptr() as *const c_char);
                let _ = writeln!(err, "load={}", rc);
            }
            "loadnull" => {
                let rc = (api.tokenizer_load_text)(std::ptr::null());
                let _ = writeln!(err, "load=null:{}", rc);
            }
            "analyze" => {
                let arg = cstring(&hex(it.next().unwrap_or("")));
                let r = (api.analyze_text)(arg.as_ptr() as *const c_char);
                let _ = writeln!(err, "analyze={:?}", r);
            }
            "analyzenull" => {
                let r = (api.analyze_text)(std::ptr::null());
                let _ = writeln!(err, "analyze=null:{:?}", r);
            }
            "find" => {
                let arg = cstring(&hex(it.next().unwrap_or("")));
                (api.find_patterns)(arg.as_ptr() as *const c_char);
            }
            "findnull" => (api.find_patterns)(std::ptr::null()),
            "readfile" => {
                let arg = cstring(&hex(it.next().unwrap_or("")));
                let p = (api.read_file)(arg.as_ptr() as *const c_char);
                if p.is_null() {
                    let _ = writeln!(err, "readfile=NULL");
                } else {
                    let mut bytes = Vec::new();
                    let mut i = 0isize;
                    unsafe {
                        while *p.offset(i) != 0 {
                            bytes.push(*p.offset(i) as u8);
                            i += 1;
                        }
                    }
                    let _ = writeln!(err, "readfile={} {}", bytes.len(), show(&bytes));
                    c_free(p);
                }
            }
            "menu" => (api.print_menu)(),
            "dist" => (api.print_token_distribution)(),
            "score" => {
                let _ = writeln!(err, "score={}", (api.calculate_complexity_score)());
            }
            "stats" => {
                let (l, t, c) = api.stats();
                let _ = writeln!(err, "stats={} {} {}", l, t, c);
            }
            "next" => {
                let _ = writeln!(err, "next={:?}", api.next());
            }
            "peek" => {
                let _ = writeln!(err, "peek={:?}", api.peek());
            }
            "reset" => (api.tokenizer_reset)(),
            "flush" => api.flush(),
            "result" => {
                let n: Vec<usize> = it.map(|x| x.parse().expect("number")).collect();
                assert_eq!(n.len(), 8, "result needs 8 numbers");
                (api.print_analysis_result)(CResult {
                    word_count: n[0],
                    number_count: n[1],
                    keyword_count: n[2],
                    operator_count: n[3],
                    comment_count: n[4],
                    string_count: n[5],
                    line_count: n[6],
                    char_count: n[7],
                });
            }
            "exitnow" => {
                // leave whatever is buffered to the library's flush-at-exit
                // handling (libc's for the C build, `atexit` for the Rust one)
                let _ = err.flush();
                unsafe { libc_exit(0) };
            }
            "interactive" => (api.interactive_tokenizer)((api.get_tokenizer_ops)()),
            "interactive_null" => (api.interactive_tokenizer)(COps::null()),
            "interactive_badload" => {
                unsafe { STUB_LOAD_RC = -1 };
                (api.interactive_tokenizer)(stub_ops());
            }
            other => panic!("unknown command {:?}", other),
        }
        let _ = err.flush();
    }

    // Drain whatever the library still holds buffered, exactly like the
    // process-exit flush of the C build.
    api.flush();
}
