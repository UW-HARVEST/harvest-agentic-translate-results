// Differential tests: run the original C program and the Rust translation as
// subprocesses with identical argv/stdin and require byte-identical stdout,
// byte-identical stderr and the same exit status.
//
// Nothing here links against the Rust crate as a library: both programs are
// driven exactly the way a shell would drive them.
//
// `usage()` in main.c prints argv[0], so both binaries are copied to a
// directory named `driver` inside two sibling directories and invoked as
// `./driver` with the matching working directory.  That makes argv[0] the same
// string for both processes.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

struct Harness {
    c_dir: PathBuf,
    rs_dir: PathBuf,
}

#[derive(PartialEq, Eq)]
struct Output {
    status: Option<i32>,
    signal: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl std::fmt::Debug for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "status={:?} signal={:?}\n  stdout={:?}\n  stderr={:?}",
            self.status,
            self.signal,
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr)
        )
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().unwrap().to_path_buf()
}

/// Builds the C program (out of source, so `c_src/` is never written to unless
/// it already contains a build directory) and returns the path to the binary.
fn c_binary() -> PathBuf {
    if let Some(p) = std::env::var_os("C_DRIVER") {
        return PathBuf::from(p);
    }
    let root = workspace_root();
    let prebuilt = root.join("c_src/build/driver");
    if prebuilt.is_file() {
        return prebuilt;
    }
    let build_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/c_build");
    std::fs::create_dir_all(&build_dir).expect("create c build dir");
    let cmake = Command::new("cmake")
        .arg("-S")
        .arg(root.join("c_src"))
        .arg("-B")
        .arg(&build_dir)
        .output()
        .expect("run cmake (is cmake installed?)");
    assert!(
        cmake.status.success(),
        "cmake configure failed:\n{}\n{}",
        String::from_utf8_lossy(&cmake.stdout),
        String::from_utf8_lossy(&cmake.stderr)
    );
    let build = Command::new("cmake")
        .arg("--build")
        .arg(&build_dir)
        .output()
        .expect("run cmake --build");
    assert!(
        build.status.success(),
        "cmake --build failed:\n{}\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    build_dir.join("driver")
}

fn harness() -> &'static Harness {
    static H: OnceLock<Harness> = OnceLock::new();
    H.get_or_init(|| {
        let stage = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/diffrun");
        let c_dir = stage.join("c");
        let rs_dir = stage.join("rs");
        std::fs::create_dir_all(&c_dir).expect("mkdir c");
        std::fs::create_dir_all(&rs_dir).expect("mkdir rs");

        let c_src_bin = c_binary();
        let rs_src_bin = PathBuf::from(env!("CARGO_BIN_EXE_driver"));
        copy_exe(&c_src_bin, &c_dir.join("driver"));
        copy_exe(&rs_src_bin, &rs_dir.join("driver"));
        Harness { c_dir, rs_dir }
    })
}

fn copy_exe(from: &Path, to: &Path) {
    // A running binary cannot be overwritten in place; remove first.
    let _ = std::fs::remove_file(to);
    std::fs::copy(from, to).unwrap_or_else(|e| panic!("copy {:?} -> {:?}: {e}", from, to));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(to, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

fn run_in<S: AsRef<std::ffi::OsStr>>(dir: &Path, args: &[S], stdin_data: &[u8]) -> Output {
    let mut child = Command::new("./driver")
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn ./driver in {:?}: {e}", dir));
    {
        let mut si = child.stdin.take().unwrap();
        // The program may exit without reading stdin; ignore EPIPE.
        let _ = si.write_all(stdin_data);
        let _ = si.flush();
    }
    let out = child.wait_with_output().expect("wait");
    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        out.status.signal()
    };
    #[cfg(not(unix))]
    let signal = None;
    Output {
        status: out.status.code(),
        signal,
        stdout: out.stdout,
        stderr: out.stderr,
    }
}

#[track_caller]
fn check_io<S: AsRef<std::ffi::OsStr>>(args: &[S], stdin_data: &[u8]) {
    let h = harness();
    let c = run_in(&h.c_dir, args, stdin_data);
    let r = run_in(&h.rs_dir, args, stdin_data);
    assert!(
        c == r,
        "mismatch for args={:?} stdin={:?}\n C : {:?}\n RS: {:?}",
        args.iter().map(|a| a.as_ref()).collect::<Vec<_>>(),
        String::from_utf8_lossy(stdin_data),
        c,
        r
    );
}

#[track_caller]
fn check(args: &[&str]) {
    check_io(args, b"");
}

/// Same as `check`, but the arguments are raw bytes (possibly not UTF-8).
#[cfg(unix)]
#[track_caller]
fn check_bytes(args: &[&[u8]], stdin_data: &[u8]) {
    use std::os::unix::ffi::OsStrExt;
    let owned: Vec<std::ffi::OsString> = args
        .iter()
        .map(|a| std::ffi::OsStr::from_bytes(a).to_os_string())
        .collect();
    check_io(&owned, stdin_data);
}

#[track_caller]
fn check_prog(prog: &[i32]) {
    let owned: Vec<String> = prog.iter().map(|v| v.to_string()).collect();
    let args: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
    check(&args);
}

// ---------------------------------------------------------------------------
// argv handling / main.c
// ---------------------------------------------------------------------------

#[test]
fn no_arguments_is_no_program() {
    // code.len == 0 -> "no program" on stderr, exit 2
    check(&[]);
}

#[test]
fn help_flag() {
    check(&["--help"]);
    // --help wins as soon as it is seen, even after other arguments
    check(&["1", "--help", "2"]);
    check(&["--help", "--help"]);
    check(&["--stdin", "--help"]);
    // ...but arguments before it are still parsed (and can print "skip")
    check(&["zzz", "--help"]);
}

#[test]
fn unparsable_arguments_are_skipped() {
    check(&["abc"]);
    check(&["12x"]);
    check(&["x12"]);
    check(&["--verbose"]);
    check(&["-"]);
    check(&["+"]);
    check(&["0x10"]);
    check(&["3.5"]);
    check(&["1 2"]);
    check(&["abc", "def"]); // all skipped -> no program, exit 2
    check(&["abc", "0", "5"]);
}

#[test]
fn empty_string_argument_parses_as_zero() {
    // strtol("") leaves *e == '\0', so the guard accepts it and pushes 0.
    check(&[""]);
    check(&["", ""]);
}

#[test]
fn whitespace_and_sign_forms() {
    check(&["  42  "]); // trailing space -> *e != 0 -> skipped
    check(&["  42"]);
    check(&["\t7"]);
    check(&["\u{b}7"]); // vertical tab is strtol whitespace
    check(&["\u{c}7"]); // form feed
    check(&["+5"]);
    check(&["-0"]);
    check(&["007"]);
    check(&["-000000009"]);
}

#[test]
fn integer_narrowing_and_saturation() {
    check(&["2147483647"]);
    check(&["2147483648"]); // (int) narrowing wraps
    check(&["-2147483648"]);
    check(&["-2147483649"]);
    check(&["4294967296"]); // -> 0
    check(&["4294967301"]); // -> 5
    check(&["9223372036854775807"]);
    check(&["9223372036854775808"]); // LONG_MAX saturation
    check(&["99999999999999999999"]);
    check(&["-99999999999999999999"]);
}

// ---------------------------------------------------------------------------
// stdin handling / read_stdin
// ---------------------------------------------------------------------------

#[test]
fn stdin_empty_is_no_program() {
    check_io(&["--stdin"], b"");
    check_io(&["--stdin"], b"\n");
    check_io(&["--stdin"], b"   \t \r\n");
}

#[test]
fn stdin_basic_tokenising() {
    check_io(&["--stdin"], b"0 5\n");
    check_io(&["--stdin"], b"0 5");            // no trailing newline
    check_io(&["--stdin"], b"0\t5\r\n3\n");    // tabs and CRLF
    check_io(&["--stdin"], b"0\n5\n3\n4\n");   // one per line
    check_io(&["--stdin"], b"  0   5  \n");    // runs of separators
    check_io(&["--stdin"], b"0 5 zz 3\n");     // unparsable token dropped silently
    check_io(&["--stdin"], b"0x10 010 +3 -0 5.5\n");
    check_io(&["--stdin"], b"\x0b42\n");       // vertical tab inside a token
    check_io(&["--stdin"], b"2147483648 -2147483649\n");
}

#[test]
fn stdin_combined_with_argv() {
    check_io(&["0", "5", "--stdin"], b"1 10\n");
    check_io(&["--stdin", "0", "5"], b"1 10\n"); // argv is parsed first regardless
    check_io(&["--stdin", "--stdin"], b"3\n");
    check_io(&["zz", "--stdin"], b"3\n");
}

#[test]
fn stdin_embedded_nul_truncates_the_line() {
    // The tokenizer walks buf as a C string, so a NUL hides the rest of that
    // fgets chunk (but not later lines).
    check_io(&["--stdin"], b"1\x002 3\n4 5\n");
    check_io(&["--stdin"], b"\x000 5\n0 3\n");
}

#[test]
fn stdin_long_lines_are_split_by_fgets() {
    // fgets reads at most 4095 bytes, which can cut a number in half.
    let mut long = Vec::new();
    for _ in 0..3000 {
        long.extend_from_slice(b"3 ");
    }
    long.push(b'\n');
    check_io(&["--stdin"], &long);

    // Force a split in the middle of a token: pad to 4093 bytes then a long
    // number, so the digits land in two different fgets chunks.
    let mut split = vec![b'0'; 0];
    split.extend(std::iter::repeat(b' ').take(4090));
    split.extend_from_slice(b"123456789\n");
    check_io(&["--stdin"], &split);

    let mut nonl = std::iter::repeat(b'7').take(5000).collect::<Vec<u8>>();
    nonl.push(b'\n');
    check_io(&["--stdin"], &nonl);
}

// ---------------------------------------------------------------------------
// engine.c: every opcode and every early return
// ---------------------------------------------------------------------------

#[test]
fn single_opcode_programs() {
    for op in -3..=13 {
        check_prog(&[op]);
    }
}

#[test]
fn error_return_paths() {
    check_prog(&[0]); // rc 1  : PUSH with no immediate
    check_prog(&[1]); // rc 2  : ADD with empty stack
    check_prog(&[0, 5, 1]); // rc 2  : ADD with one element
    check_prog(&[2]); // rc 3  : MUL with empty stack
    check_prog(&[0, 5, 2]); // rc 3  : MUL with one element
    check_prog(&[4]); // rc 4  : POP with empty stack
    check_prog(&[6]); // rc 5  : JMPIF without k
    check_prog(&[6, 1]); // rc 6  : JMPIF with empty stack
    check_prog(&[0, 1, 6, 5]); // rc 7  : jump past the end
    check_prog(&[0, 1, 6, -1]); // rc 7  : negative k becomes huge
    check_prog(&[7]); // rc 8  : LOOP without count
    check_prog(&[7, 2]); // rc 9  : LOOP with nothing to run
    check_prog(&[9]); // rc 10 : STREAM without m
    check_prog(&[9, -1]); // rc 11 : negative m
    check_prog(&[9, 3]); // rc 11 : m greater than stack length
    check_prog(&[11]); // rc 99 : unknown opcode
    check_prog(&[12]);
    check_prog(&[-1]);
    check_prog(&[i32::MIN]);
    check_prog(&[i32::MAX]);
}

#[test]
fn halt_and_trailing_code() {
    check_prog(&[10]);
    check_prog(&[0, 5, 10]);
    check_prog(&[0, 5, 10, 1]); // code after HALT never runs
    check_prog(&[0, 5, 10, 99]);
}

#[test]
fn push_add_mul_dup_pop() {
    check_prog(&[0, 2, 0, 3, 1]);
    check_prog(&[0, 2, 0, 3, 2]);
    check_prog(&[0, -2, 0, 3, 2]);
    check_prog(&[0, 2147483647, 0, 2, 1]); // signed overflow in the C add
    check_prog(&[0, 2147483647, 0, 2, 2]); // and in the multiply
    check_prog(&[0, -2147483648, 0, -1, 2]);
    check_prog(&[3]); // DUP of an empty stack pushes the default 0
    check_prog(&[3, 3, 3]);
    check_prog(&[0, 7, 3, 3, 4, 4]);
    check_prog(&[0, 7, 4, 4]); // second POP fails -> rc 4
}

#[test]
fn classify_opcodes_cover_every_trace_bucket() {
    // op 5 traces 5..9 depending on the bucket, op 8 always traces 13.
    for x in -5..40 {
        check_prog(&[0, x, 5]);
        check_prog(&[0, x, 8]);
        check_prog(&[0, x, 5, 5]);
        check_prog(&[0, x, 8, 8]);
    }
    check_prog(&[5]); // empty stack -> peek default 0
    check_prog(&[8]);
    check_prog(&[0, i32::MIN, 5]);
    check_prog(&[0, i32::MAX, 5]);
    check_prog(&[0, i32::MIN, 8]);
    check_prog(&[0, i32::MAX, 8]);
}

#[test]
fn conditional_jumps() {
    check_prog(&[0, 0, 6, 1, 0, 4]); // cond == 0 -> no jump
    check_prog(&[0, 1, 6, 1, 0, 4]); // cond != 0 -> skip one word
    check_prog(&[0, 1, 6, 0]); // k == 0
    check_prog(&[0, 1, 6, 2, 0, 4]); // jump exactly to the end
    check_prog(&[0, 1, 6, 3, 0, 4]); // one past the end -> rc 7
    check_prog(&[0, -1, 6, 1, 3]);
    check_prog(&[0, 1, 6, 1, 10, 3]);
}

#[test]
fn loop_opcode() {
    check_prog(&[7, 0, 3]); // times == 0 -> body never runs
    check_prog(&[7, 1, 3]);
    check_prog(&[7, 3, 3]);
    check_prog(&[7, -1, 3]); // negative count -> body never runs
    check_prog(&[7, 2, 0]); // inner PUSH lacks its immediate -> trace 12
    check_prog(&[7, 2, 1]); // inner ADD fails -> trace 12
    check_prog(&[7, 2, 7]); // nested LOOP fails immediately
    check_prog(&[7, 2, 9]); // nested STREAM lacks m
    check_prog(&[7, 2, 10]); // inner HALT succeeds every iteration
    check_prog(&[7, 2, 99]); // unknown opcode -> rc 99 -> trace 12
    check_prog(&[7, 4, 5]);
    check_prog(&[7, 4, 8]);
    check_prog(&[0, 5, 7, 3, 3, 4]); // execution resumes at saved_ip + 1
    check_prog(&[7, 100, 3]);
}

#[test]
fn stream_opcode() {
    check_prog(&[9, 0]); // m == 0
    check_prog(&[0, 5, 9, 1]); // m == stack length, second pop round fails
    check_prog(&[0, 5, 0, 6, 9, 2]);
    check_prog(&[0, 5, 0, 6, 0, 7, 9, 2]); // second round of pops succeeds
    check_prog(&[0, 1, 0, 2, 0, 3, 0, 4, 9, 4]);
    check_prog(&[0, -3, 0, -4, 9, 2]);
    check_prog(&[0, 10, 0, 20, 0, 30, 9, 3]);
    check_prog(&[0, 0, 9, 1, 9, 1]);
    check_prog(&[9, 1]); // m > len (len 0) -> rc 11
}

#[test]
fn state_is_shared_between_calls() {
    // a.c/b.c keep static state, so repeated classify calls must drift in the
    // same way in both programs.
    let mut prog = vec![0, 3];
    for _ in 0..40 {
        prog.push(5);
    }
    check_prog(&prog);

    let mut prog2 = vec![0, 7];
    for _ in 0..30 {
        prog2.push(8);
    }
    check_prog(&prog2);

    let mut prog3 = Vec::new();
    for i in 0..12 {
        prog3.push(0);
        prog3.push(i * 7 - 3);
        prog3.push(5);
        prog3.push(9);
        prog3.push(2);
    }
    check_prog(&prog3);
}

#[test]
fn long_trace_wraps_through_the_alphabet_mask() {
    let mut prog = Vec::new();
    for _ in 0..300 {
        prog.push(3);
    }
    check_prog(&prog);

    let mut prog = vec![0, 9];
    for _ in 0..200 {
        prog.push(5);
    }
    check_prog(&prog);
}

// ---------------------------------------------------------------------------
// Phase C: paths the first round of testing did not reach
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn non_utf8_arguments_are_echoed_verbatim() {
    // fprintf("skip '%s'") writes the raw bytes of argv[i]; the Rust version
    // must not lossily re-encode them.
    check_bytes(&[b"\xff\xfe"], b"");
    check_bytes(&[b"1\xff"], b"");
    check_bytes(&[b"\x80\x80", b"5"], b"");
    check_bytes(&[b"--stdin\xff"], b"");
    check_bytes(&[b"-\xc3"], b"");
    check_bytes(&[b"--stdin"], b"\xff\xfe 5 3\n");
}

#[test]
fn fgets_chunk_boundary_sweep() {
    // A token that straddles the 4095-byte fgets cut is parsed as two separate
    // tokens (and each half may or may not parse).
    for pad in 4088..4100 {
        let mut data: Vec<u8> = std::iter::repeat(b' ').take(pad).collect();
        data.extend_from_slice(b"12345\n0 5\n");
        check_io(&["--stdin"], &data);
    }
    // Exactly one full chunk with no newline at all.
    let data: Vec<u8> = std::iter::repeat(b'4').take(4095).collect();
    check_io(&["--stdin"], &data);
    let data: Vec<u8> = std::iter::repeat(b'4').take(4096).collect();
    check_io(&["--stdin"], &data);
}

#[test]
fn nested_loop_body_covers_every_opcode() {
    for inner in -2..=12 {
        for times in [0, 1, 2, 3, 5] {
            check_prog(&[7, times, inner]);
            check_prog(&[0, 4, 0, 9, 7, times, inner]);
            check_prog(&[0, 4, 0, 9, 7, times, inner, 1, 9, 2]);
        }
    }
}

#[test]
fn stream_over_many_stack_depths() {
    for depth in 0..9 {
        let mut prog = Vec::new();
        for i in 0..depth {
            prog.push(0);
            prog.push(i * 3 - 4);
        }
        for m in 0..7 {
            let mut p1 = prog.clone();
            p1.extend_from_slice(&[9, m]);
            check_prog(&p1);
            let mut p2 = prog.clone();
            p2.extend_from_slice(&[9, m, 9, 1]);
            check_prog(&p2);
        }
    }
}

#[test]
fn jump_offset_sweep() {
    for k in -3..=7 {
        for cond in [0, 1, -1, 2] {
            check_prog(&[0, cond, 6, k, 3, 3, 4, 10]);
            check_prog(&[0, cond, 6, k]);
        }
    }
}

#[test]
fn many_arguments() {
    let progs: Vec<Vec<i32>> = vec![
        std::iter::repeat(3).take(500).collect(),
        std::iter::repeat([0, 5]).take(300).flatten().collect(),
        std::iter::repeat(5).take(200).collect(),
        std::iter::repeat(8).take(200).collect(),
    ];
    for p in progs.iter() {
        check_prog(p);
    }
}

#[test]
fn stdin_is_ignored_without_the_flag() {
    check_io(&["3"], b"9 9 9\n");
    check_io::<&str>(&[], b"9 9 9\n");
    check_io(&["--help"], b"9 9 9\n");
}

// ---------------------------------------------------------------------------
// Broad deterministic sweep
// ---------------------------------------------------------------------------

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 11
    }
    fn range(&mut self, lo: i64, hi: i64) -> i64 {
        lo + (self.next() % ((hi - lo + 1) as u64)) as i64
    }
}

#[test]
fn exhaustive_two_word_programs() {
    let alphabet: [i32; 14] = [-1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 99];
    for &a in alphabet.iter() {
        for &b in alphabet.iter() {
            check_prog(&[a, b]);
        }
    }
}

#[test]
fn random_programs() {
    let mut rng = Lcg(0x1234_5678_9abc_def0);
    for _ in 0..500 {
        let n = rng.range(1, 10) as usize;
        let mut prog: Vec<i32> = Vec::with_capacity(n);
        for _ in 0..n {
            let mut v = match rng.range(0, 9) {
                0 | 1 => rng.range(i32::MIN as i64, i32::MAX as i64) as i32,
                2 | 3 => rng.range(-25, 25) as i32,
                _ => rng.range(-2, 12) as i32,
            };
            // A huge LOOP count makes *both* programs run for hours (the C
            // program just as much as the Rust one), so keep the iteration
            // count small; `loop_opcode` covers the LOOP paths explicitly.
            if prog.last() == Some(&7) {
                v = rng.range(-2, 6) as i32;
            }
            prog.push(v);
        }
        check_prog(&prog);
    }
}

#[test]
fn random_programs_via_stdin() {
    let mut rng = Lcg(0xdead_beef_0bad_f00d);
    for _ in 0..150 {
        let n = rng.range(1, 14) as usize;
        let mut words: Vec<String> = Vec::new();
        let mut prev = 0i32;
        for _ in 0..n {
            let mut v = match rng.range(0, 5) {
                0 => rng.range(i32::MIN as i64, i32::MAX as i64) as i32,
                1 => rng.range(-25, 25) as i32,
                _ => rng.range(-2, 12) as i32,
            };
            if prev == 7 {
                v = rng.range(-2, 6) as i32; // keep LOOP counts small (see above)
            }
            prev = v;
            words.push(v.to_string());
        }
        let sep = match rng.range(0, 3) {
            0 => " ",
            1 => "\n",
            2 => "\t",
            _ => "\r\n",
        };
        let mut data = words.join(sep).into_bytes();
        data.push(b'\n');
        check_io(&["--stdin"], &data);
    }
}
