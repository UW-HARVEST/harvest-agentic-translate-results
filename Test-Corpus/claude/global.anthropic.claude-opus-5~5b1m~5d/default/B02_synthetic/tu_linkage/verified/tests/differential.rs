//! Differential tests: run the original C program (`c_src`) and the Rust
//! translation as *subprocesses* and require byte-identical stdout, byte
//! identical stderr and an identical exit status for every input.
//!
//! Nothing here links against the translation as a library; both programs are
//! driven exactly the way a shell would drive them.

#![cfg(unix)]

use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// harness
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
struct Out {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// raw `wait(2)` status, so a death by signal is compared too
    raw_status: i32,
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the C executable, building it with cmake on first use if needed.
fn c_binary() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = workspace_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");
        if !exe.exists() {
            std::fs::create_dir_all(&build).expect("cannot create c_src/build");
            let cfg = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .status()
                .expect("failed to run `cmake ..` (is cmake installed?)");
            assert!(cfg.success(), "cmake configuration of c_src failed");
            let bld = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build)
                .status()
                .expect("failed to run `cmake --build .`");
            assert!(bld.success(), "cmake build of c_src failed");
        }
        assert!(
            exe.exists(),
            "C reference binary {} is missing; build it with \
             `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`",
            exe.display()
        );
        exe
    })
}

/// Path to the Rust executable built by cargo for this integration test.
fn rust_binary() -> &'static Path {
    static R_BIN: OnceLock<PathBuf> = OnceLock::new();
    R_BIN.get_or_init(|| PathBuf::from(env!("CARGO_BIN_EXE_driver")))
}

/// Runs `exe` with the given raw-byte arguments and stdin, with `argv[0]`
/// forced to the literal `driver` for *both* programs.  main.c prints
/// `argv[0]` in its usage text, so without this the two executables could only
/// ever differ by their own path.
fn run(exe: &Path, args: &[Vec<u8>], stdin_data: &[u8]) -> Out {
    let os_args: Vec<OsString> = args
        .iter()
        .map(|a| OsStr::from_bytes(a).to_os_string())
        .collect();

    let mut child = Command::new(exe)
        .arg0("driver")
        .args(&os_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));

    {
        let mut si = child.stdin.take().expect("piped stdin");
        // The child may exit without reading stdin (e.g. `--help`); a broken
        // pipe here is not a test failure.
        let _ = si.write_all(stdin_data);
        let _ = si.flush();
    }

    let out = child.wait_with_output().expect("wait_with_output");
    Out {
        stdout: out.stdout,
        stderr: out.stderr,
        raw_status: out.status.into_raw(),
    }
}

/// Runs `exe`, reads only the first few bytes of its stdout and then closes the
/// pipe, so the program keeps writing into a pipe with no reader.  Returns the
/// raw wait status.  (The C program is killed by SIGPIPE here; only the status
/// is comparable, since how much output made it into the 64 KiB pipe buffer
/// before the reader went away is not part of either program's contract.)
fn run_with_stdout_closed_early(exe: &Path, args: &[Vec<u8>]) -> i32 {
    use std::io::Read;

    let os_args: Vec<OsString> = args
        .iter()
        .map(|a| OsStr::from_bytes(a).to_os_string())
        .collect();
    let mut child = Command::new(exe)
        .arg0("driver")
        .args(&os_args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));
    {
        let mut so = child.stdout.take().expect("piped stdout");
        let mut buf = [0u8; 4];
        let _ = so.read(&mut buf);
        // `so` is dropped here: the read end of the pipe is closed.
    }
    child.wait().expect("wait").into_raw()
}

fn show(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\t' => s.push_str("\\t"),
            b'\r' => s.push_str("\\r"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    s
}

fn describe(args: &[Vec<u8>], stdin_data: &[u8]) -> String {
    let a: Vec<String> = args.iter().map(|x| format!("'{}'", show(x))).collect();
    let mut st = show(stdin_data);
    if st.len() > 200 {
        st.truncate(200);
        st.push_str("...<truncated>");
    }
    format!("args=[{}] stdin=\"{}\"", a.join(", "), st)
}

/// The core assertion: same stdout, same stderr, same exit status.
#[track_caller]
fn assert_same(label: &str, args: &[Vec<u8>], stdin_data: &[u8]) {
    let c = run(c_binary(), args, stdin_data);
    let r = run(rust_binary(), args, stdin_data);
    let ctx = describe(args, stdin_data);

    assert_eq!(
        show(&c.stdout),
        show(&r.stdout),
        "[{label}] stdout differs for {ctx}"
    );
    assert_eq!(
        c.stdout, r.stdout,
        "[{label}] stdout bytes differ for {ctx}"
    );
    assert_eq!(
        show(&c.stderr),
        show(&r.stderr),
        "[{label}] stderr differs for {ctx}"
    );
    assert_eq!(
        c.stderr, r.stderr,
        "[{label}] stderr bytes differ for {ctx}"
    );
    assert_eq!(
        c.raw_status, r.raw_status,
        "[{label}] exit status differs for {ctx} (C={:?} rust={:?}, stdout C={:?})",
        c.raw_status,
        r.raw_status,
        show(&c.stdout)
    );
}

// convenience constructors -------------------------------------------------

fn a(strs: &[&str]) -> Vec<Vec<u8>> {
    strs.iter().map(|s| s.as_bytes().to_vec()).collect()
}

fn ints(v: &[i32]) -> Vec<Vec<u8>> {
    v.iter().map(|x| x.to_string().into_bytes()).collect()
}

/// Bytecode program given as ints, run purely from the command line.
#[track_caller]
fn prog(label: &str, v: &[i32]) {
    assert_same(label, &ints(v), b"");
}

/// Same program, but fed through `--stdin` (one token per line so that no
/// `fgets` chunk boundary interferes).
#[track_caller]
fn prog_stdin(label: &str, v: &[i32]) {
    let mut s = Vec::new();
    for x in v {
        s.extend_from_slice(x.to_string().as_bytes());
        s.push(b'\n');
    }
    assert_same(label, &a(&["--stdin"]), &s);
}

// ---------------------------------------------------------------------------
// Phase B: the input classes main.c / engine.c branch on
// ---------------------------------------------------------------------------

#[test]
fn empty_input_no_program() {
    // `if (code.len==0) { fprintf(stderr,"no program\n"); return 2; }`
    assert_same("no args", &[], b"");
    assert_same("only --stdin, empty stdin", &a(&["--stdin"]), b"");
    assert_same("only --stdin, blank lines", &a(&["--stdin"]), b"\n\n\n");
    assert_same("only --stdin, whitespace", &a(&["--stdin"]), b" \t \r\n  \n");
    assert_same("all args skipped", &a(&["abc", "x"]), b"");
    assert_same("stdin ignored without flag", &[], b"0 7\n");
}

#[test]
fn single_item_programs() {
    // one bytecode: every opcode on its own, i.e. every early `return` of
    // run_engine that a 1-instruction program can reach
    prog("op0 missing imm -> rc 1", &[0]);
    prog("op1 underflow -> rc 2", &[1]);
    prog("op2 underflow -> rc 3", &[2]);
    prog("op3 dup of empty stack (peek default 0)", &[3]);
    prog("op4 pop of empty stack -> rc 4", &[4]);
    prog("op5 classify empty stack (peek default 0)", &[5]);
    prog("op6 missing k -> rc 5", &[6]);
    prog("op7 missing times -> rc 8", &[7]);
    prog("op8 classify empty stack", &[8]);
    prog("op9 missing m -> rc 10", &[9]);
    prog("op10 halt -> rc 0", &[10]);
    prog("op11 invalid -> rc 99", &[11]);
    prog("negative op -> rc 99", &[-1]);
    prog("INT_MAX op -> rc 99", &[2147483647]);
    prog("INT_MIN op -> rc 99", &[-2147483648]);
}

#[test]
fn help_flag() {
    // usage() prints argv[0]; run() forces argv[0]="driver" for both programs
    assert_same("--help alone", &a(&["--help"]), b"");
    assert_same("--help before a bytecode", &a(&["--help", "5"]), b"");
    assert_same("--help after a bytecode", &a(&["5", "--help"]), b"");
    assert_same("--help after a skip", &a(&["abc", "--help"]), b"");
    assert_same("--help with --stdin", &a(&["--stdin", "--help"]), b"0 7\n");
    assert_same("--help twice", &a(&["--help", "--help"]), b"");
}

#[test]
fn argument_parsing_strtol_quirks() {
    // `strtol` + `if (e && *e=='\0')`: full-string parses are pushed, anything
    // else produces `skip '...'` on stderr.
    for s in [
        "",            // empty arg: strtol converts nothing but *e=='\0' -> pushes 0
        " ",           // whitespace only -> skip
        "  ",          //
        "\t",          //
        "-",           // sign only -> skip
        "+",           //
        "--",          //
        "0x10",        // base 10 stops at 'x' -> skip
        "1e3",         //
        "12abc",       //
        "abc",         //
        ".5",          //
        "5.0",         //
        "1,2",         //
        "+5",          // accepted
        "-0",          //
        " 12",         // leading whitespace is skipped by strtol -> accepted
        "\t9",         //
        "\n5",         //
        "\r5",         //
        "\u{b}5",      // \v is isspace() too
        "\u{c}5",      // \f
        "12 ",         // trailing junk -> skip
        "5\t",         //
        "5\n",         //
        "5\r",         //
        "007",         // leading zeros
        "0000000000",  //
        "--STDIN",     // not the flag -> strtol -> skip
        "--Stdin",     //
        "-stdin",      //
        "2147483647",  // INT_MAX
        "2147483648",  // (int) truncation -> INT_MIN
        "-2147483648", //
        "-2147483649", //
        "4294967296",  // truncates to 0
        "4294967297",  // truncates to 1
        "9223372036854775807",  // LONG_MAX
        "9223372036854775808",  // strtol saturates -> (int)LONG_MAX == -1
        "-9223372036854775808", // LONG_MIN
        "-9223372036854775809", // saturates -> (int)LONG_MIN == 0
        "99999999999999999999999999999999",
        "-99999999999999999999999999999999",
        "0000000000000000000000000000005",
    ] {
        assert_same("strtol arg", &a(&[s]), b"");
        // ... and the same token as part of a real program
        assert_same("strtol arg in program", &a(&["0", s, "5"]), b"");
    }
}

#[test]
fn argument_parsing_non_utf8() {
    // argv is raw bytes; `skip '...'` must reproduce them verbatim
    for bad in [
        vec![0xffu8, 0xfe],
        vec![0xc3],
        vec![b'5', 0xff],
        vec![0xff, b'5'],
        vec![0xd9, 0xa1, 0xd9, 0xa2], // Arabic-Indic digits: not [0-9]
        vec![b'c', b'a', b'f', 0xe9],
    ] {
        assert_same("non-utf8 arg", &[bad.clone()], b"");
        assert_same(
            "non-utf8 arg inside program",
            &[b"0".to_vec(), bad, b"5".to_vec()],
            b"",
        );
    }
}

#[test]
fn stdin_tokenising() {
    // read_stdin(): fgets + manual tokenising on ' ', '\t', '\n', '\r'
    for s in [
        &b"0 7\n"[..],
        b"0 7",           // no trailing newline
        b" 0\t7 \r\n",    // every separator
        b"0\n7\n",        // one per line
        b"0\r\n7\r\n",    // CRLF
        b"0 7\n\n\n",     // blank lines
        b"\n\n0 7",       //
        b"0 7 abc def 5", // unparsable tokens are silently dropped (no `skip`)
        b"abc 0 7",
        b"   0   7   ",
        b"0\t\t\t7",
        b"9999999999999999999999 0 7", // saturating token
        b"0 -7 5",
        b"+0 +7",
        b"0 7 --stdin",  // flags are not special on stdin
        b"0 7 --help",   //
        b"\x0b5 0 7",    // \v: token separator? no - strtol skips it
        b"5\x0b 0 7",    // trailing \v -> token dropped
        b"0\x007\n",     // embedded NUL: rest of the fgets chunk is dropped
        b"\x00",         // NUL only -> empty program
        b"0\x007\n0 5\n" ,// ... but the next line is still read
        b"0 5 5 5 10\n",
    ] {
        assert_same("stdin tokens", &a(&["--stdin"]), s);
        assert_same("stdin tokens after args", &a(&["0", "3", "--stdin"]), s);
        assert_same("stdin flag repeated", &a(&["--stdin", "--stdin"]), s);
    }
}

#[test]
fn stdin_fgets_chunk_boundary() {
    // fgets() reads at most 4095 bytes, so a long line is handed to the
    // tokeniser in pieces and a number can be cut in half.  Here the boundary
    // falls inside "1234": the C program sees the tokens ... 0 1 | 234 5.
    let mut line = Vec::new();
    for _ in 0..1023 {
        line.extend_from_slice(b"0 3 ");
    }
    line.extend_from_slice(b"0 ");
    assert_eq!(line.len(), 4094);
    line.extend_from_slice(b"1234 5\n");
    assert_same("4095-byte fgets split inside a number", &a(&["--stdin"]), &line);

    // boundary exactly on a separator / exactly on the newline
    for pad in [4093usize, 4094, 4095, 4096, 4097] {
        let mut l = vec![b'0'; 1];
        l.push(b' ');
        while l.len() < pad {
            l.push(b'3');
            l.push(b' ');
        }
        l.truncate(pad);
        l.extend_from_slice(b"\n0 5\n");
        assert_same("fgets boundary", &a(&["--stdin"]), &l);
    }

    // several over-long lines in a row
    let mut multi = Vec::new();
    for _ in 0..3 {
        for _ in 0..2050 {
            multi.extend_from_slice(b"1 ");
        }
        multi.extend_from_slice(b"98765\n");
    }
    assert_same("multiple long lines", &a(&["--stdin"]), &multi);
}

#[test]
fn opcode_push_add_mul_dup_pop() {
    prog("push", &[0, 42]);
    prog("push negative", &[0, -42]);
    prog("push INT_MIN", &[0, -2147483648]);
    prog("add", &[0, 2, 0, 40, 1]);
    prog("add wraps", &[0, 2147483647, 0, 1, 1]);
    prog("add underflow after one push", &[0, 5, 1]);
    prog("mul", &[0, 6, 0, 7, 2]);
    prog("mul wraps", &[0, 2147483647, 0, 2147483647, 2]);
    prog("mul underflow after one push", &[0, 5, 2]);
    prog("dup then add", &[0, 21, 3, 1]);
    prog("pop", &[0, 1, 4]);
    prog("pop twice, second underflows", &[0, 1, 4, 4]);
    prog("halt stops execution", &[0, 1, 10, 0, 2]);
    prog("invalid op ends run", &[0, 1, 12]);
}

#[test]
fn opcode_classify_buckets() {
    // op5 pushes classify() and traces 5/6/7/8/9 depending on the bucket;
    // op8 always traces 13.  Sweep enough values to hit every bucket of all
    // three implementations.
    for v in [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 27, 31, 32, 63,
        64, 127, 128, 255, 256, 1000, 1001, -1, -2, -3, -7, -8, -9, -10, -127, -128, -255, -256,
        2147483647, 2147483646, -2147483648, -2147483647, 65536, 1000000,
    ] {
        prog("op5 bucket", &[0, v, 5]);
        prog("op8 classify", &[0, v, 8]);
        prog("op5 twice (static state advances)", &[0, v, 5, 5]);
        prog("op8 then op5", &[0, v, 8, 5]);
    }
    prog("op5 on empty stack uses peek default 0", &[5]);
    prog("op8 on empty stack uses peek default 0", &[8]);
    prog("long classify chain", &[0, 3, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5]);
    prog("long op8 chain", &[0, 3, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8]);
}

#[test]
fn opcode_jump() {
    // case 6: missing operand (rc 5), empty stack (rc 6), out-of-range jump
    // (rc 7), taken and untaken branches.
    prog("jump missing k", &[6]);
    prog("jump empty stack", &[6, 1]);
    prog("jump not taken", &[0, 0, 6, 2, 0, 9, 10]);
    prog("jump taken k=0", &[0, 1, 6, 0, 10]);
    prog("jump taken k=1", &[0, 1, 6, 1, 11, 10]);
    prog("jump taken k=2", &[0, 1, 6, 2, 11, 11, 10]);
    prog("jump taken to exactly the end", &[0, 1, 6, 1, 11]);
    prog("jump one past the end -> rc 7", &[0, 1, 6, 2, 11]);
    prog("jump far past the end -> rc 7", &[0, 1, 6, 100, 11]);
    prog("negative k -> (size_t) is huge -> rc 7", &[0, 1, 6, -1, 11]);
    prog("negative k, INT_MIN", &[0, 1, 6, -2147483648, 11]);
    prog("cond false ignores bad k", &[0, 0, 6, -1, 10]);
    prog("cond false, k past end", &[0, 0, 6, 999, 10]);
    prog("jump with negative nonzero cond", &[0, -5, 6, 1, 11, 10]);
}

#[test]
fn opcode_loop() {
    // case 7: missing times (rc 8), missing body (rc 9), the recursive
    // single-instruction execution, and the "inner returned nonzero" path
    // (trace 12).
    prog("loop missing times", &[7]);
    prog("loop missing body", &[7, 3]);
    prog("loop times=0", &[0, 5, 7, 0, 3, 10]);
    prog("loop times negative", &[0, 5, 7, -1, 3, 10]);
    prog("loop times INT_MIN", &[0, 5, 7, -2147483648, 3]);
    for times in [1, 2, 3, 4, 7] {
        for inner in [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, -1] {
            prog("loop body", &[0, 5, 7, times, inner]);
            prog("loop body then more code", &[0, 5, 7, times, inner, 5, 10]);
        }
    }
    prog("loop over halt", &[0, 5, 7, 3, 10, 5]);
    prog("nested loop tokens", &[0, 5, 7, 2, 7, 2, 3]);
    prog("loop after loop", &[0, 5, 7, 2, 3, 7, 2, 3]);
}

#[test]
fn opcode_stream() {
    // case 9: missing m (rc 10), m<0 (rc 11), m>stack.len (rc 11), and the
    // double-pop loop which pops up to 2*m values.
    prog("stream missing m", &[9]);
    prog("stream m negative", &[0, 1, 9, -1]);
    prog("stream m INT_MIN", &[0, 1, 9, -2147483648]);
    prog("stream m > stack", &[0, 1, 9, 2]);
    prog("stream m huge", &[0, 1, 9, 1000000]);
    prog("stream m=0 on empty stack", &[9, 0]);

    // stack depth S vs m: exercises the second (partially failing) pop loop
    let pushes = [7, 9, 10, 255, -1, 4, 100, 3, 12, -256];
    for s in 0..=8usize {
        let mut base = Vec::new();
        for i in 0..s {
            base.push(0);
            base.push(pushes[i % pushes.len()]);
        }
        for m in 0..=(s + 2) as i32 {
            for tail in [&[][..], &[9, 1][..], &[3, 5][..], &[5, 8][..]] {
                let mut p = base.clone();
                p.push(9);
                p.push(m);
                p.extend_from_slice(tail);
                prog("stream depth/m matrix", &p);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Phase C: wide sweeps over the arithmetic-heavy leaf functions
// ---------------------------------------------------------------------------

/// Values that exercise `target`/`call_a_once`/`call_b_once` boundaries.
fn sweep_values() -> Vec<i32> {
    let mut v: Vec<i32> = (-300..=300).collect();
    v.extend_from_slice(&[
        511, 512, 1023, 1024, 4095, 4096, 65535, 65536, 1 << 20, 1 << 24, 1 << 30,
        (1 << 30) + 1, 2147483646, 2147483647, -65536, -1000000, 1000000, 999999, 12345, 54321,
        -2147483647, -2147483648, 0x55, 0x7f, 0x1f, 0x2222, 17, -17,
    ]);
    v
}

/// One process, thousands of `classify` calls: each value is classified and
/// folded into a running sum, so any single differing result changes stdout.
#[test]
fn classify_sweep_folded() {
    for op in [5i32, 8] {
        let mut p = vec![0, 0];
        for v in sweep_values() {
            p.extend_from_slice(&[0, v, op, 1, 1]);
        }
        prog("classify sweep (sum fold)", &p);

        let mut q = vec![0, 1];
        for v in sweep_values() {
            q.extend_from_slice(&[0, v, op, 2, 1]);
        }
        prog("classify sweep (mul fold)", &q);
    }
}

/// Same idea for `process_stream` with m = 1..4.
#[test]
fn stream_sweep_folded() {
    let vals = sweep_values();

    let mut p1 = vec![0, 0];
    for v in &vals {
        p1.extend_from_slice(&[0, *v, 3, 9, 1, 1]);
    }
    prog("stream sweep m=1", &p1);

    let mut p2 = vec![0, 0];
    for v in &vals {
        p2.extend_from_slice(&[0, *v, 3, 9, 2]);
    }
    prog("stream sweep m=2", &p2);

    let mut p3 = vec![0, 0];
    for v in &vals {
        p3.extend_from_slice(&[0, *v, 3, 3, 9, 3]);
    }
    prog("stream sweep m=3", &p3);

    let mut p4 = vec![0, 0];
    for v in &vals {
        p4.extend_from_slice(&[0, *v, 3, 3, 3, 9, 4]);
    }
    prog("stream sweep m=4", &p4);
}

#[test]
fn exhaustive_short_programs() {
    // every 1- and 2-token program over all opcodes plus an invalid one
    let alpha: [i32; 13] = [-1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
    for &x in &alpha {
        prog("len1", &[x]);
        for &y in &alpha {
            prog("len2", &[x, y]);
        }
    }
}

#[test]
fn exhaustive_three_token_programs() {
    let alpha: [i32; 11] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    for &x in &alpha {
        for &y in &alpha {
            for &z in &alpha {
                prog("len3", &[x, y, z]);
            }
        }
    }
}

#[test]
fn exhaustive_four_token_programs() {
    // every 4-token program over the operand-taking opcodes plus halt, which
    // is where the interesting interactions live (push/jump/loop/stream all
    // consume the next token)
    let alpha: [i32; 7] = [0, 1, 3, 5, 6, 7, 9];
    for &w in &alpha {
        for &x in &alpha {
            for &y in &alpha {
                for &z in &alpha {
                    prog("len4", &[w, x, y, z]);
                }
            }
        }
    }
}

/// Deterministic pseudo-random programs (no external crates), so failures are
/// reproducible.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 11
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

#[test]
fn randomised_programs() {
    let pool: [i32; 24] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, -1, -2, 12, 99, 255, -255, 100, -100, 12345,
        2147483647, -2147483648, 65536,
    ];
    let mut rng = Lcg(0x2024_0828_dead_beef);
    for _ in 0..400 {
        let len = 1 + rng.below(14) as usize;
        let mut p = Vec::with_capacity(len);
        for _ in 0..len {
            if rng.below(100) < 80 {
                p.push(pool[rng.below(pool.len() as u64) as usize]);
            } else {
                p.push(rng.next() as i32);
            }
        }
        prog("random program", &p);
    }
}

/// Long programs built so that they never hit an error path: they run to the
/// last instruction, so `STEPS` and a several-hundred-character `TRACE` depend
/// on every intermediate stack value, every `classify` result and every
/// `process_stream` result.  This is the deepest end-to-end check.
#[test]
fn deep_valid_programs() {
    let imms: [i32; 13] = [
        0, 1, 2, 3, 7, 9, 10, -1, -7, 255, 12345, -2147483648, 2147483647,
    ];
    for seed in 0..24u64 {
        let mut rng = Lcg(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ 0xdead_1234);
        let mut p: Vec<i32> = Vec::new();
        let mut depth: i64 = 0;
        for _ in 0..400 {
            // kinds always legal: 0=push 1=dup 2=op5 3=op8 4=loop
            // kinds needing depth>=1: 5=pop 6=jump(k=0) 7=stream
            // kinds needing depth>=2: 8=add 9=mul
            let limit = if depth >= 2 {
                10
            } else if depth >= 1 {
                8
            } else {
                5
            };
            match rng.below(limit) {
                0 => {
                    p.push(0);
                    p.push(imms[rng.below(imms.len() as u64) as usize]);
                    depth += 1;
                }
                1 => {
                    p.push(3);
                    depth += 1;
                }
                2 => {
                    p.push(5);
                    depth += 1;
                }
                3 => {
                    p.push(8);
                    depth += 1;
                }
                4 => {
                    let times = rng.below(5) as i32;
                    p.extend_from_slice(&[7, times, 3]); // body: dup
                    depth += times as i64;
                }
                5 => {
                    p.push(4);
                    depth -= 1;
                }
                6 => {
                    p.extend_from_slice(&[6, 0]); // jump of 0: always in range
                    depth -= 1;
                }
                7 => {
                    let m = rng.below(depth as u64 + 1) as i64;
                    p.push(9);
                    p.push(m as i32);
                    depth = depth - std::cmp::min(2 * m, depth) + 1;
                }
                8 => {
                    p.push(1);
                    depth -= 1;
                }
                _ => {
                    p.push(2);
                    depth -= 1;
                }
            }
        }
        prog("deep valid program", &p);
    }
}

#[test]
fn randomised_programs_via_stdin() {
    let pool: [i32; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, -1, 3, 42, -42];
    let mut rng = Lcg(0x5eed_1234_5678_9abc);
    for _ in 0..150 {
        let len = 1 + rng.below(12) as usize;
        let mut p = Vec::with_capacity(len);
        for _ in 0..len {
            p.push(pool[rng.below(pool.len() as u64) as usize]);
        }
        prog_stdin("random program via stdin", &p);
        // ... and split between argv and stdin
        let split = p.len() / 2;
        let mut args = ints(&p[..split]);
        args.push(b"--stdin".to_vec());
        let mut si = Vec::new();
        for x in &p[split..] {
            si.extend_from_slice(x.to_string().as_bytes());
            si.push(b' ');
        }
        si.push(b'\n');
        assert_same("args + stdin program", &args, &si);
    }
}

/// Thousands of pseudo-random values (full i32 range) folded through
/// classify / process_stream in a single process each.  This is the widest
/// check of a.c / b.c / lib.c arithmetic: one differing result anywhere
/// changes the printed fold.
#[test]
fn wide_random_value_sweeps() {
    let mut rng = Lcg(0x1357_9bdf_0246_8ace);
    let mut vals: Vec<i32> = (0..4000).map(|_| rng.next() as i32).collect();
    vals.extend((0..4000).map(|_| (rng.below(4001) as i32) - 2000));

    let patterns: [&[i32]; 5] = [
        &[5, 1, 1],    // classify (op5), sum fold
        &[8, 1, 1],    // classify (op8), sum fold
        &[8, 2, 1],    // classify, multiply fold
        &[3, 9, 1, 1], // process_stream, m=1
        &[3, 9, 2],    // process_stream, m=2
    ];
    for pat in patterns {
        let mut p = vec![0, 0];
        for v in &vals {
            p.push(0);
            p.push(*v);
            p.extend_from_slice(pat);
        }
        prog_stdin("wide random sweep", &p);
    }
}

/// `vm->steps` is an `int`; the op-7 loop drives it far past a plain program's
/// step count.  (The full wrap past INT_MAX needs ~2^31 iterations and takes
/// minutes, so this stops at 5 million - enough to prove the recursive
/// single-instruction execution and the counter stay in step.)
#[test]
fn loop_step_counter_at_scale() {
    prog("5M iterations over halt", &[0, 5, 7, 5_000_000, 10]);
    prog("1M iterations, code after the loop", &[0, 5, 7, 1_000_000, 10, 5, 8, 10]);
}

/// A C program is killed by SIGPIPE when it writes to a pipe nobody reads any
/// more; the Rust runtime ignores SIGPIPE by default, so the translation has to
/// restore the default disposition or it would exit 0 where the C dies with
/// signal 13.
#[test]
fn stdout_pipe_closed_early_dies_the_same_way() {
    for args in [
        &["0", "1", "7", "70000", "3"][..],
        &["0", "1", "7", "300000", "3"][..],
        &["0", "1", "7", "500000", "3"][..],
    ] {
        let argv = a(args);
        let c = run_with_stdout_closed_early(c_binary(), &argv);
        let r = run_with_stdout_closed_early(rust_binary(), &argv);
        assert_eq!(
            c, r,
            "exit status differs when stdout is closed early for {args:?} \
             (C raw status {c}, rust raw status {r})"
        );
    }
}

#[test]
fn large_inputs() {
    // long argv
    let mut big: Vec<i32> = Vec::new();
    for _ in 0..10000 {
        big.push(0);
        big.push(3);
    }
    prog("20000 argv bytecodes", &big);

    // long stdin, one token per line
    prog_stdin("10000 stdin bytecodes", &big[..10000]);

    // deep stack then a big stream
    let mut deep: Vec<i32> = Vec::new();
    for i in 0..600 {
        deep.push(0);
        deep.push(i % 37);
    }
    deep.extend_from_slice(&[9, 600]);
    prog("600-deep stream", &deep);
    let mut deep2 = deep.clone();
    deep2.truncate(deep2.len() - 2);
    deep2.extend_from_slice(&[9, 300, 5, 8]);
    prog("300 of 600 stream", &deep2);

    // `int tmp[m]` in engine.c case 9 is a VLA: a very large m allocates a
    // large frame in C where the Rust uses the heap.
    let mut huge: Vec<i32> = Vec::with_capacity(200_002);
    for i in 0..100_000 {
        huge.push(0);
        huge.push(i % 29);
    }
    huge.extend_from_slice(&[9, 100_000]);
    prog_stdin("100k-element VLA stream", &huge);
}
