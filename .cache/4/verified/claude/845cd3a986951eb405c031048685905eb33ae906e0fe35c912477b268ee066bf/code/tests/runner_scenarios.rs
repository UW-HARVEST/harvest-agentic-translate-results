//! Phase B/C — scenarios that need a *fresh process* per run: everything that
//! consumes stdin (`interactive_tokenizer`, `CONFIGS.md` row C39), the virgin
//! "analyzer not initialized" state and the flush-at-exit behaviour.
//!
//! `examples/ffi_runner.rs` loads exactly one library with `libloading` and
//! replays a command script against it; the same runner binary and the same
//! script/stdin are used for the C and the Rust `.so`, so any difference in the
//! captured stdout/stderr/exit status comes from the library.

mod common;

use common::*;

fn script_for_stdin(cmds: &[&str]) -> String {
    let mut s = String::new();
    for c in cmds {
        s.push_str(c);
        s.push('\n');
    }
    s
}

// ---------------------------------------------------------------------------
// C39: interactive_tokenizer over real stdin
// ---------------------------------------------------------------------------

#[test]
fn c39_interactive_tokenizer_stdin_shapes() {
    let script = script_for_stdin(&["init", "interactive"]);

    let mut cases: Vec<(&str, Vec<u8>)> = vec![
        ("empty stdin", vec![]),
        ("blank line first", b"\n".to_vec()),
        ("blank then text", b"\nint x;\n".to_vec()),
        ("one line", b"int x = 1;\n".to_vec()),
        ("one line no newline", b"int x = 1;".to_vec()),
        (
            "three lines",
            b"int a = 1;\nif (a) { return \"s\"; }\n// done\n\n".to_vec(),
        ),
        ("only spaces", b"    \n".to_vec()),
        ("crlf", b"a\r\nb\r\n\r\n".to_vec()),
        ("high bytes", b"caf\xc3\xa9 \xff\x80\n".to_vec()),
        ("nul byte in line", b"ab\0cd\n".to_vec()),
        ("unterminated string", b"\"abc\n".to_vec()),
        ("open comment", b"/* abc\n".to_vec()),
    ];

    // exactly around the 100-token truncation limit
    for n in [99usize, 100, 101, 102, 250] {
        let mut v = Vec::new();
        for i in 0..n {
            v.extend_from_slice(format!("t{} ", i).as_bytes());
        }
        v.push(b'\n');
        cases.push(("many tokens", v));
    }

    // a line longer than fgets' 256-byte buffer
    let mut long_line = vec![b'a'; 300];
    long_line.push(b'\n');
    cases.push(("300-byte line", long_line));
    let mut long_line = vec![b'z'; 600];
    long_line.extend_from_slice(b" tail\n");
    cases.push(("600-byte line", long_line));

    // more than MAX_INPUT_SIZE (4096) bytes of input -> strncat saturation
    let mut big = Vec::new();
    while big.len() < 5000 {
        big.extend_from_slice(b"word1 word2 word3 word4 word5 word6 word7 word8\n");
    }
    big.extend_from_slice(b"\n");
    cases.push(("5000 bytes", big));
    let mut huge = Vec::new();
    while huge.len() < 9000 {
        huge.extend_from_slice(b"0123456789abcdef ");
    }
    huge.extend_from_slice(b"\n\n");
    cases.push(("9000 bytes", huge));

    for (name, stdin) in &cases {
        println!("interactive: {} ({} bytes)", name, stdin.len());
        diff_runner(&script, stdin);
    }
}

#[test]
fn c39b_interactive_tokenizer_random_stdin() {
    let script = script_for_stdin(&["init", "interactive"]);
    let mut rng = Rng::new(0xC39);
    for _ in 0..25 {
        let mut payload = Vec::new();
        let lines = rng.range(1, 6);
        for _ in 0..lines {
            payload.extend_from_slice(&random_source(&mut rng, 15));
            payload.push(b'\n');
        }
        if rng.chance(2) {
            payload.push(b'\n');
        }
        diff_runner(&script, &payload);
    }
}

#[test]
fn c39c_interactive_tokenizer_twice() {
    // two consecutive calls share the stdin buffer state
    let script = script_for_stdin(&["init", "interactive", "interactive", "stats"]);
    let cases: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"a\n\nb\n\n".to_vec(),
        b"first line\n\nsecond line\nthird\n\n".to_vec(),
        {
            let mut v = Vec::new();
            while v.len() < 5000 {
                v.extend_from_slice(b"tok ");
            }
            v.extend_from_slice(b"\n\nmore tokens here\n\n");
            v
        },
    ];
    for stdin in &cases {
        diff_runner(&script, stdin);
    }
}

// ---------------------------------------------------------------------------
// E34: interactive_tokenizer whose ops.load_text fails
// ---------------------------------------------------------------------------

#[test]
fn e34_interactive_tokenizer_load_failure() {
    let script = script_for_stdin(&["init", "interactive_badload"]);
    for stdin in [&b""[..], &b"\n"[..], &b"some text\n\n"[..]] {
        diff_runner(&script, stdin);
    }
}

// ---------------------------------------------------------------------------
// Virgin-process scenarios
// ---------------------------------------------------------------------------

#[test]
fn virgin_process_scenarios() {
    // analyze/find/dist/score before analyzer_init
    diff_runner("analyze 616263\n", b"");
    diff_runner("analyzenull\n", b"");
    diff_runner("find 61\n", b"");
    diff_runner("findnull\n", b"");
    diff_runner("dist\n", b"");
    diff_runner("score\n", b"");
    diff_runner("stats\n", b"");
    diff_runner("next\npeek\nnext\nstats\n", b"");
    diff_runner("reset\nnext\nstats\n", b"");
    diff_runner("loadnull\nanalyzenull\nfindnull\ndist\nscore\n", b"");
    // interactive before init still works (it does not consult `initialized`)
    diff_runner("interactive\n", b"int x;\n\n");
    // full run in a virgin process
    diff_runner(
        "init\nload 696e7420783b\nnext\nnext\nstats\nanalyze 696e7420613b\ndist\nscore\nfind 61\nmenu\nresult 1 2 3 4 5 6 7 8\n",
        b"",
    );
}

/// `analyzer_init` with an all-NULL `tokenizer_ops_t` and then an entry point
/// that dispatches through it: the C build calls a NULL function pointer, so
/// both builds must die the same way and with the same (buffered-and-lost)
/// output.
#[test]
fn null_ops_dispatch_dies_identically() {
    for script in [
        "initnull\nanalyze 616263\n",
        "initnull\nanalyzenull\n",
        "initnull\nfind 61\n",
        "initnull\nfind \n",
        "initnull\nmenu\nfind 61\n",
        "interactive_null\n",
        "init\ninteractive_null\n",
        "init\nanalyze 6120622063\ninitnull\nfind 61\n",
    ] {
        let run = diff_runner(script, b"some stdin\n\n");
        assert_eq!(
            run.signal,
            Some(11),
            "expected SIGSEGV from the NULL ops dispatch of\n{}\nstdout: {}\nstderr: {}",
            script,
            show(&run.stdout),
            show(&run.stderr)
        );
    }
}

#[test]
fn null_ops_installed_then_pure_functions() {
    // analyzer_init with an all-NULL tokenizer_ops_t: the entry points that do
    // not dispatch through the ops must still behave identically.
    diff_runner("initnull\ndist\nscore\nmenu\nresult 0 0 0 0 0 0 0 0\n", b"");
    diff_runner("init\nanalyze 696620612b62\ninitnull\ndist\nscore\n", b"");
}

// ---------------------------------------------------------------------------
// stdout flush at process exit
// ---------------------------------------------------------------------------

#[test]
fn flush_at_exit() {
    // No explicit flush: the buffered output must still appear, in the same
    // order, when the process exits.
    diff_runner("menu\nexitnow\n", b"");
    diff_runner("init\nanalyze 696e742061203d20313b\ndist\nmenu\nexitnow\n", b"");
    // more than one stdio buffer's worth of output
    let mut s = String::new();
    s.push_str("init\n");
    for _ in 0..40 {
        s.push_str("menu\n");
    }
    s.push_str("exitnow\n");
    diff_runner(&s, b"");
}

// ---------------------------------------------------------------------------
// Mixed scripts (randomized)
// ---------------------------------------------------------------------------

#[test]
fn random_runner_scripts() {
    let mut rng = Rng::new(0xBEEF);
    for _ in 0..40 {
        let mut s = String::new();
        let n = rng.range(1, 12);
        for _ in 0..n {
            match rng.below(12) {
                0 => s.push_str("init\n"),
                1 => s.push_str("initnull\n"),
                2 => {
                    let t = random_source(&mut rng, 10);
                    s.push_str(&format!("load {}\n", to_hex(&t)));
                }
                3 => {
                    let t = random_source(&mut rng, 10);
                    s.push_str(&format!("analyze {}\n", to_hex(&t)));
                }
                4 => {
                    let t = random_soup(&mut rng, 4);
                    s.push_str(&format!("find {}\n", to_hex(&t)));
                }
                5 => s.push_str("dist\n"),
                6 => s.push_str("score\n"),
                7 => s.push_str("stats\n"),
                8 => s.push_str("next\n"),
                9 => s.push_str("peek\n"),
                10 => s.push_str("reset\n"),
                _ => s.push_str("menu\n"),
            }
        }
        if rng.chance(3) {
            s.push_str("exitnow\n");
        }
        diff_runner(&s, b"");
    }
}
